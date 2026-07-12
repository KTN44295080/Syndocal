use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use protocol::{DmxInputConfig, DmxInputProtocol, DmxInputStatus, DmxMergeMode};
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum DmxInputEvent {
    Frame {
        universe: u16,
        values: Box<[u8; 512]>,
        merge_mode: DmxMergeMode,
    },
    SignalLost {
        universe: u16,
    },
}

#[derive(Debug, Error)]
pub enum DmxInputError {
    #[error("invalid DMX input bind address '{0}'")]
    InvalidBindAddress(String),
    #[error("failed to bind DMX input socket: {0}")]
    Bind(std::io::Error),
    #[error("failed to configure DMX input socket: {0}")]
    Configure(std::io::Error),
    #[error("failed to start DMX input thread: {0}")]
    Thread(std::io::Error),
}

pub struct DmxInput {
    stop: Arc<AtomicBool>,
    status: Arc<Mutex<DmxInputStatus>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl DmxInput {
    pub fn start<F>(config: DmxInputConfig, on_event: F) -> Result<Self, DmxInputError>
    where
        F: Fn(DmxInputEvent) + Send + Sync + 'static,
    {
        let bind_ip = config
            .bind_ip
            .trim()
            .parse::<Ipv4Addr>()
            .map_err(|_| DmxInputError::InvalidBindAddress(config.bind_ip.clone()))?;
        let socket = UdpSocket::bind(SocketAddr::from((bind_ip, config.port)))
            .map_err(DmxInputError::Bind)?;
        socket
            .set_read_timeout(Some(Duration::from_millis(100)))
            .map_err(DmxInputError::Configure)?;
        if config.protocol == DmxInputProtocol::Sacn {
            let multicast = Ipv4Addr::new(
                239,
                255,
                (config.universe >> 8) as u8,
                (config.universe & 0xff) as u8,
            );
            socket
                .join_multicast_v4(&multicast, &Ipv4Addr::UNSPECIFIED)
                .map_err(DmxInputError::Configure)?;
        }

        let stop = Arc::new(AtomicBool::new(false));
        let status = Arc::new(Mutex::new(DmxInputStatus {
            running: true,
            ..DmxInputStatus::default()
        }));
        let thread_stop = Arc::clone(&stop);
        let thread_status = Arc::clone(&status);
        let callback = Arc::new(on_event);
        let thread = thread::Builder::new()
            .name("syndocal-dmx-input".to_string())
            .spawn(move || {
                let mut buffer = [0u8; 1536];
                let mut last_packet_at: Option<Instant> = None;
                while !thread_stop.load(Ordering::Relaxed) {
                    match socket.recv_from(&mut buffer) {
                        Ok((received, source)) => {
                            let parsed = match config.protocol {
                                DmxInputProtocol::ArtNet => {
                                    crate::artnet::parse_art_dmx_packet(&buffer[..received])
                                        .map(|packet| (packet.universe, packet.data))
                                }
                                DmxInputProtocol::Sacn => {
                                    crate::sacn::parse_sacn_dmx_packet(&buffer[..received])
                                        .map(|packet| (packet.universe, packet.data))
                                }
                            };
                            let Some((universe, data)) = parsed else {
                                if let Ok(mut status) = thread_status.lock() {
                                    status.invalid_packets =
                                        status.invalid_packets.saturating_add(1);
                                }
                                continue;
                            };
                            if universe != config.universe {
                                continue;
                            }
                            let mut values = Box::new([0u8; 512]);
                            let length = data.len().min(values.len());
                            values[..length].copy_from_slice(&data[..length]);
                            last_packet_at = Some(Instant::now());
                            if let Ok(mut status) = thread_status.lock() {
                                status.signal_present = true;
                                status.packets_received = status.packets_received.saturating_add(1);
                                status.last_packet_unix_ms = Some(current_unix_ms());
                                status.source_address = Some(source.to_string());
                            }
                            callback(DmxInputEvent::Frame {
                                universe,
                                values,
                                merge_mode: config.merge_mode,
                            });
                        }
                        Err(error)
                            if matches!(
                                error.kind(),
                                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                            ) => {}
                        Err(_) => break,
                    }
                    if last_packet_at.is_some_and(|last| {
                        last.elapsed() >= Duration::from_millis(config.timeout_ms.max(100))
                    }) {
                        last_packet_at = None;
                        if let Ok(mut status) = thread_status.lock() {
                            status.signal_present = false;
                        }
                        callback(DmxInputEvent::SignalLost {
                            universe: config.universe,
                        });
                    }
                }
                callback(DmxInputEvent::SignalLost {
                    universe: config.universe,
                });
                if let Ok(mut status) = thread_status.lock() {
                    status.running = false;
                    status.signal_present = false;
                }
            })
            .map_err(DmxInputError::Thread)?;
        Ok(Self {
            stop,
            status,
            thread: Some(thread),
        })
    }

    pub fn status(&self) -> DmxInputStatus {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_default()
    }
}

impl Drop for DmxInput {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn artnet_input_receives_frames_and_emits_signal_loss() {
        let probe = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let (sender, receiver) = mpsc::channel();
        let input = DmxInput::start(
            DmxInputConfig {
                protocol: DmxInputProtocol::ArtNet,
                bind_ip: "127.0.0.1".to_string(),
                port,
                universe: 3,
                merge_mode: DmxMergeMode::Htp,
                timeout_ms: 120,
            },
            move |event| {
                let _ = sender.send(event);
            },
        )
        .unwrap();
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let mut frame = [0u8; 512];
        frame[0] = 200;
        frame[511] = 17;
        let packet = crate::artnet::build_art_dmx_packet_with_sequence(3, 1, &frame);
        socket.send_to(&packet, ("127.0.0.1", port)).unwrap();

        let event = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        match event {
            DmxInputEvent::Frame {
                universe,
                values,
                merge_mode,
            } => {
                assert_eq!(universe, 3);
                assert_eq!(values[0], 200);
                assert_eq!(values[511], 17);
                assert_eq!(merge_mode, DmxMergeMode::Htp);
            }
            _ => panic!("expected DMX frame"),
        }
        assert!(input.status().signal_present);
        assert_eq!(input.status().packets_received, 1);

        let lost = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(matches!(lost, DmxInputEvent::SignalLost { universe: 3 }));
        assert!(!input.status().signal_present);
    }
}
