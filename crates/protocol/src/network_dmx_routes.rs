//! Bounded network-only route payload. USB device identity remains host-local.
use crate::{DmxOutputConfig, DmxOutputProtocol};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkDmxProtocol {
    ArtNet,
    Sacn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkDmxRoute {
    pub enabled: bool,
    pub protocol: NetworkDmxProtocol,
    pub target_ip: String,
    pub port: u16,
    pub universe: u16,
}
impl NetworkDmxRoute {
    pub fn validate(&self) -> Result<(), String> {
        let ip = if self.protocol == NetworkDmxProtocol::Sacn && self.target_ip == "multicast" {
            std::net::Ipv4Addr::new(239, 255, (self.universe >> 8) as u8, self.universe as u8)
        } else {
            self.target_ip.parse::<std::net::Ipv4Addr>().map_err(|_| {
                "DMX destination must be an IPv4 address or sACN multicast".to_string()
            })?
        };
        if ip.is_unspecified() || self.port == 0 {
            return Err("DMX destination and port must be explicit".into());
        }
        match self.protocol {
            NetworkDmxProtocol::ArtNet if self.universe > 32767 => {
                Err("Art-Net universe must be 0–32767".into())
            }
            NetworkDmxProtocol::Sacn if self.universe == 0 || self.universe > 63999 => {
                Err("sACN universe must be 1–63999".into())
            }
            _ => Ok(()),
        }
    }
    pub fn config(&self) -> DmxOutputConfig {
        DmxOutputConfig {
            enabled: self.enabled,
            protocol: match self.protocol {
                NetworkDmxProtocol::ArtNet => DmxOutputProtocol::ArtNet,
                NetworkDmxProtocol::Sacn => DmxOutputProtocol::Sacn,
            },
            target_ip: self.target_ip.clone(),
            port: self.port,
            universe: self.universe,
            serial_port: String::new(),
            serial_baud_rate: 57600,
        }
    }
}
pub fn validate_network_dmx_routes(routes: &[NetworkDmxRoute]) -> Result<(), String> {
    if routes.len() > 256 {
        return Err("At most 256 network routes are supported".into());
    }
    let mut keys = std::collections::HashSet::new();
    for route in routes {
        route.validate()?;
        let ip = if route.protocol == NetworkDmxProtocol::Sacn && route.target_ip == "multicast" {
            std::net::Ipv4Addr::new(239, 255, (route.universe >> 8) as u8, route.universe as u8)
        } else {
            route
                .target_ip
                .parse::<std::net::Ipv4Addr>()
                .map_err(|e| e.to_string())?
        };
        if route.enabled && !keys.insert((route.protocol as u8, ip, route.port, route.universe)) {
            return Err("Duplicate enabled DMX destination and universe".into());
        }
    }
    Ok(())
}
/// Validate authored additional routes without admitting a serial-device alias.
pub fn validate_network_dmx_configs(configs: &[DmxOutputConfig]) -> Result<(), String> {
    let routes = configs
        .iter()
        .map(|config| {
            let protocol = match config.protocol {
                DmxOutputProtocol::ArtNet => NetworkDmxProtocol::ArtNet,
                DmxOutputProtocol::Sacn => NetworkDmxProtocol::Sacn,
                _ => return Err("Additional routes must be Art-Net or sACN".to_string()),
            };
            if !config.serial_port.is_empty() {
                return Err("Network routes cannot carry a serial device alias".to_string());
            }
            Ok(NetworkDmxRoute {
                enabled: config.enabled,
                protocol,
                target_ip: config.target_ip.clone(),
                port: config.port,
                universe: config.universe,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    validate_network_dmx_routes(&routes)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn route(universe: u16) -> NetworkDmxRoute {
        NetworkDmxRoute {
            enabled: true,
            protocol: NetworkDmxProtocol::ArtNet,
            target_ip: "127.0.0.1".into(),
            port: 16454,
            universe,
        }
    }
    #[test]
    fn multiple_universes_and_mirrors_are_distinct() {
        let first = route(0);
        let second = route(1);
        let mut mirror = first.clone();
        mirror.port += 1;
        assert!(validate_network_dmx_routes(&[first.clone(), second, mirror]).is_ok());
        let mut multicast = route(7);
        multicast.protocol = NetworkDmxProtocol::Sacn;
        multicast.target_ip = "multicast".into();
        let mut explicit = multicast.clone();
        explicit.target_ip = "239.255.0.7".into();
        assert!(validate_network_dmx_routes(&[multicast.clone()]).is_ok());
        assert!(validate_network_dmx_routes(&[multicast, explicit]).is_err());
        assert!(validate_network_dmx_routes(&[first.clone(), first]).is_err());
    }
    #[test]
    fn invalid_routes_cannot_reach_sender_construction() {
        let mut r = route(1);
        r.target_ip = "host-name".into();
        assert!(r.validate().is_err());
        r.target_ip = "0.0.0.0".into();
        assert!(r.validate().is_err());
        r.target_ip = "127.0.0.1".into();
        r.port = 0;
        assert!(r.validate().is_err());
        r.port = 6454;
        r.universe = 0;
        assert!(validate_network_dmx_routes(&[r.clone()]).is_ok());
        r.protocol = NetworkDmxProtocol::Sacn;
        assert!(r.validate().is_err());
        r.universe = 64000;
        assert!(r.validate().is_err());
        r.protocol = NetworkDmxProtocol::ArtNet;
        r.universe = 32768;
        assert!(r.validate().is_err());
        assert!(validate_network_dmx_routes(&vec![route(1); 256]).is_err());
        assert!(serde_json::from_str::<NetworkDmxRoute>(r#"{"enabled":true,"protocol":"EnttecOpenDmx","target_ip":"127.0.0.1","port":6454,"universe":0}"#).is_err());
    }
}

/// Compatibility primary for an empty complete network list. Not an authored
/// route: unspecified IP and port zero are rejected by route validation.
/// Ordinary disabled legacy U0 loopback routes never match this sentinel.
pub fn empty_network_dmx_output() -> DmxOutputConfig {
    DmxOutputConfig {
        enabled: false,
        protocol: DmxOutputProtocol::ArtNet,
        target_ip: "0.0.0.0".into(),
        port: 0,
        universe: 0,
        serial_port: String::new(),
        serial_baud_rate: 57600,
    }
}
pub fn is_empty_network_dmx_output(config: &DmxOutputConfig) -> bool {
    config == &empty_network_dmx_output()
}

#[cfg(test)]
mod complete_table_tests {
    use super::*;
    #[test]
    fn complete_table_supports_256_routes_and_explicit_loopback() {
        let routes = (0..256)
            .map(|universe| NetworkDmxRoute {
                enabled: true,
                protocol: NetworkDmxProtocol::ArtNet,
                target_ip: "127.0.0.1".into(),
                port: 6454,
                universe,
            })
            .collect::<Vec<_>>();
        assert!(validate_network_dmx_routes(&routes).is_ok());
        let mut overflow = routes;
        overflow.push(NetworkDmxRoute {
            universe: 256,
            ..overflow[0].clone()
        });
        assert!(validate_network_dmx_routes(&overflow).is_err());
        assert!(is_empty_network_dmx_output(&empty_network_dmx_output()));
        assert!(!is_empty_network_dmx_output(&DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        }));
    }
}
