//! Network route publication owns candidate sockets until snapshot acknowledgement.
use super::*;
use protocol::network_dmx_routes::{NetworkDmxRoute, validate_network_dmx_routes};

#[derive(Clone)]
pub(super) struct NetworkDmxRoutesRollback {
    // Publication rollback clones its record. Physical senders have one owner.
    prior: Arc<Mutex<Option<(RuntimeDmxOutput, Vec<RuntimeDmxOutput>)>>>,
    generation: u64,
    last_error: Option<String>,
}
impl NetworkDmxRoutesRollback {
    pub(super) fn restore(self, runtime: &mut EngineRuntime) {
        let prior = self
            .prior
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .take();
        if let Some((primary, additional)) = prior {
            runtime.output = primary.config;
            runtime.dmx_sender = primary.sender;
            runtime.dmx_sender_recovery = primary.recovery;
            runtime.additional_dmx_outputs = additional;
            runtime.dmx_route_configuration_generation = self.generation;
            runtime.last_error = self.last_error;
        }
    }
}
impl EngineHandle {
    pub fn set_network_dmx_routes_published(
        &self,
        expected_outputs: Vec<DmxOutputConfig>,
        routes: Vec<NetworkDmxRoute>,
        expected_safety_epoch: u64,
        expected_safety_generation: u64,
        expires_at: Instant,
    ) -> Result<(), String> {
        let (ack, receiver) = mpsc::sync_channel(1);
        self.send(EngineCommand::SetNetworkDmxRoutesPublished {
            expected_outputs,
            routes,
            expected_safety_epoch,
            expected_safety_generation,
            expires_at,
            ack,
        })
        .map_err(|e| e.to_string())?;
        receiver
            .recv_timeout(Duration::from_secs(3))
            .map_err(|e| format!("Network DMX route publication was not acknowledged: {e}"))?
    }
}
impl EngineRuntime {
    pub(super) fn apply_network_dmx_routes(
        &mut self,
        expected_outputs: &[DmxOutputConfig],
        routes: &[NetworkDmxRoute],
        expected_safety_epoch: u64,
        expected_safety_generation: u64,
        expires_at: Instant,
    ) -> Result<NetworkDmxRoutesRollback, String> {
        validate_network_dmx_routes(routes)?;
        let current = std::iter::once(self.output.clone())
            .chain(self.additional_dmx_outputs.iter().map(|r| r.config.clone()))
            .collect::<Vec<_>>();
        if current != expected_outputs {
            return Err("DMX routes changed before publication".into());
        }
        if current.iter().any(|r| {
            !matches!(
                r.protocol,
                DmxOutputProtocol::ArtNet | DmxOutputProtocol::Sacn
            )
        }) {
            return Err(
                "Legacy serial output must be retired before network route replacement".into(),
            );
        }
        if !self.configured_dmx_legacy_usb_absence_verified() {
            return Err(
                "Retire the legacy U0 USB mirror before replacing its network route".into(),
            );
        }
        let ownership = self.output_ownership_gate.status();
        if ownership.state != OutputOwnershipState::Ready
            || ownership.effective_role != MachineOutputRole::Both
            || ownership.desired_role != MachineOutputRole::Both
            || !ownership.lighting_allowed
        {
            return Err(
                "Network DMX route configuration requires active Both output authority".into(),
            );
        }
        // Build every candidate before modifying A. Opening UDP sockets sends no frames.
        let mut candidates = Vec::with_capacity(routes.len());
        for route in routes {
            let (candidate, error) = RuntimeDmxOutput::new_with_error_and_lighting_allowed(
                route.config(),
                true,
                &self.shared_telemetry.open_dmx_safety_write_gate,
            );
            if let Some(error) = error {
                return Err(format!("Network DMX route initialization failed: {error}"));
            }
            candidates.push(candidate);
        }
        let shared = Arc::clone(&self.shared_telemetry);
        let _enqueue = shared
            .safety_blackout_enqueue_gate
            .lock()
            .map_err(|_| "Safety enqueue gate poisoned")?;
        let safety = shared
            .safety_blackout
            .lock()
            .map_err(|_| "Safety authority poisoned")?;
        if Instant::now() > expires_at
            || shared.has_pending_safety_blackout_enqueue()?
            || safety.epoch != expected_safety_epoch
            || safety.generation != expected_safety_generation
        {
            return Err(
                "Network DMX route configuration expired or safety authority changed".into(),
            );
        }
        let generation = self.dmx_route_configuration_generation;
        let last_error = self.last_error.clone();
        let primary = if candidates.is_empty() {
            RuntimeDmxOutput::new_with_lighting_allowed(
                protocol::network_dmx_routes::empty_network_dmx_output(),
                false,
                &self.shared_telemetry.open_dmx_safety_write_gate,
            )
        } else {
            candidates.remove(0)
        };
        let prior_primary = RuntimeDmxOutput {
            config: std::mem::replace(&mut self.output, primary.config),
            sender: std::mem::replace(&mut self.dmx_sender, primary.sender),
            recovery: std::mem::replace(&mut self.dmx_sender_recovery, primary.recovery),
        };
        let prior = (
            prior_primary,
            std::mem::replace(&mut self.additional_dmx_outputs, candidates),
        );
        self.bump_dmx_route_configuration_generation();
        self.last_error = None;
        Ok(NetworkDmxRoutesRollback {
            prior: Arc::new(Mutex::new(Some(prior))),
            generation,
            last_error,
        })
    }
}
