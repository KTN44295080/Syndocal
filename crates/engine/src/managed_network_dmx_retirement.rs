use super::*;

/// Internal engine policy; no serialization or caller-supplied destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedDmxRetirementScope {
    ExactShow,
    ConfiguredNetwork,
}

impl EngineRuntime {
    pub(super) fn apply_managed_dmx_retirement_for_scope(
        &mut self,
        scope: ManagedDmxRetirementScope,
        safety_epoch: u64,
        safety_generation: u64,
        failure_epoch: u64,
        operation: &ManagedShowDmxFailStopOperationInner,
    ) -> Result<ManagedShowDmxFailStopReceipt, ManagedShowDmxFailStopError> {
        if scope == ManagedDmxRetirementScope::ExactShow
            || !self.managed_show_dmx_has_pristine_absent_usb()
        {
            return self.apply_managed_show_dmx_fail_stop(
                safety_epoch,
                safety_generation,
                failure_epoch,
                operation,
            );
        }
        self.apply_managed_network_dmx_retirement(
            safety_epoch,
            safety_generation,
            failure_epoch,
            operation,
        )
    }

    fn apply_managed_network_dmx_retirement(
        &mut self,
        safety_epoch: u64,
        safety_generation: u64,
        failure_epoch: u64,
        operation: &ManagedShowDmxFailStopOperationInner,
    ) -> Result<ManagedShowDmxFailStopReceipt, ManagedShowDmxFailStopError> {
        // The engine is the sole route owner. Hold the S0 enqueue gate through
        // exact admission and zero sends so a queued release cannot overtake
        // this one consumed physical boundary. No persistent sender is created.
        let telemetry = Arc::clone(&self.shared_telemetry);
        let _safety_gate = telemetry.safety_blackout_enqueue_gate.lock().map_err(|_| {
            if operation.begin_commit() {
                ManagedShowDmxFailStopError::in_doubt(
                    "Managed network retirement safety gate was poisoned",
                )
            } else {
                ManagedShowDmxFailStopError::cancelled(
                    "Managed network retirement cancelled before commit",
                )
            }
        })?;
        if telemetry
            .has_pending_safety_blackout_enqueue()
            .map_err(|error| {
                if operation.begin_commit() {
                    ManagedShowDmxFailStopError::in_doubt(error)
                } else {
                    ManagedShowDmxFailStopError::cancelled(
                        "Managed network retirement cancelled before commit",
                    )
                }
            })?
        {
            return Err(ManagedShowDmxFailStopError::rejected(
                "Managed network retirement was superseded by queued S0 authority",
            ));
        }
        match self.managed_show_dmx_fail_stop_authority_preflight(
            safety_epoch,
            safety_generation,
            failure_epoch,
        ) {
            ManagedShowDmxFailStopAuthorityCheck::Exact => {}
            ManagedShowDmxFailStopAuthorityCheck::RejectedBeforeCommit(error) => {
                return Err(ManagedShowDmxFailStopError::rejected(error))
            }
            ManagedShowDmxFailStopAuthorityCheck::InDoubtAfterCommit(error) => {
                if !operation.begin_commit() {
                    return Err(ManagedShowDmxFailStopError::cancelled(
                        "Managed network retirement cancelled before commit",
                    ));
                }
                return Err(ManagedShowDmxFailStopError::in_doubt(error));
            }
        }
        if !operation.begin_commit() {
            return Err(ManagedShowDmxFailStopError::cancelled(
                "Managed network retirement cancelled before commit",
            ));
        }
        self.clear_managed_show_dmx_fail_stop_completion();
        // Fail closed for serial configurations: the exact show USB path owns
        // its verified device, zero-write receipt and bounded worker shutdown.
        let is_network = |config: &DmxOutputConfig| {
            !config.enabled
                || matches!(
                    config.protocol,
                    DmxOutputProtocol::ArtNet | DmxOutputProtocol::Sacn
                )
        };
        if !is_network(&self.output)
            || !self
                .additional_dmx_outputs
                .iter()
                .all(|route| is_network(&route.config))
        {
            return Err(ManagedShowDmxFailStopError::in_doubt("Managed configured retirement requires network routes or the dedicated exact show USB path"));
        }
        let mut errors = Vec::new();
        retire_network_sender(&mut self.dmx_sender, &self.output, &mut errors);
        self.dmx_sender_recovery = DmxRouteRecovery::default();
        for route in &mut self.additional_dmx_outputs {
            retire_network_sender(&mut route.sender, &route.config, &mut errors);
            route.recovery = DmxRouteRecovery::default();
        }
        self.output_ownership_role = MachineOutputRole::Standby;
        if !errors.is_empty() {
            return Err(ManagedShowDmxFailStopError::in_doubt(errors.join("; ")));
        }
        if !self.managed_show_dmx_terminal_absent()
            || !self.managed_show_dmx_has_pristine_absent_usb()
            || !matches!(
                self.managed_show_dmx_fail_stop_authority_preflight(
                    safety_epoch,
                    safety_generation,
                    failure_epoch
                ),
                ManagedShowDmxFailStopAuthorityCheck::Exact
            )
        {
            return Err(ManagedShowDmxFailStopError::in_doubt("Managed network retirement lost its terminal sender-absent or exact S0/failure proof"));
        }
        let serial = telemetry
            .try_show_serial_dmx_route_status_snapshot()
            .map_err(ManagedShowDmxFailStopError::in_doubt)?;
        // The compatibility operation carries internal terminal authority;
        // its network API consumes this result without exposing the exact-show
        // local datagram/USB receipt. Never project a show zero-send claim.
        Ok(ManagedShowDmxFailStopReceipt {
            safety: SafetyBlackoutAuthority {
                engaged: true,
                epoch: safety_epoch,
                generation: safety_generation,
            },
            failure_epoch,
            artnet_zero_accepted: false,
            serial_status_revision: serial.revision,
        })
    }
}

fn retire_network_sender(
    sender: &mut Option<DmxSender>,
    config: &DmxOutputConfig,
    errors: &mut Vec<String>,
) {
    let retained = match sender.take() {
        Some(retained) => Ok(retained),
        None if !config.enabled => return,
        // The earlier all-deny/S0 publication can already have dropped a
        // sender. Its current authored endpoint still needs one zero datagram.
        // This transport is never installed, armed, retried or given a live
        // frame; it is dropped within this consumed stop operation.
        None => match config.protocol {
            DmxOutputProtocol::ArtNet => ArtNetSender::new(&config.target_ip, config.port)
                .map(DmxSender::ArtNet)
                .map_err(|error| error.to_string()),
            DmxOutputProtocol::Sacn => SacnSender::new(&config.target_ip, config.port)
                .map(DmxSender::Sacn)
                .map_err(|error| error.to_string()),
            _ => Err("Managed network retirement cannot create a serial transport".into()),
        },
    };
    let mut retained = match retained {
        Ok(retained) => retained,
        Err(error) => {
            errors.push(format!(
                "Managed network retirement zero-only transport failed: {error}"
            ));
            return;
        }
    };
    let expected_size = match (&retained, &config.protocol, config.enabled) {
        (DmxSender::ArtNet(_), DmxOutputProtocol::ArtNet, true) => Some(530),
        (DmxSender::Sacn(_), DmxOutputProtocol::Sacn, true) => Some(io::sacn::SACN_PACKET_LEN),
        #[cfg(test)]
        (
            DmxSender::TestExactArtNetRoute | DmxSender::TestFailingArtNetSend,
            DmxOutputProtocol::ArtNet,
            true,
        ) => Some(0),
        _ => None,
    };
    let Some(expected_size) = expected_size else {
        errors.push("Managed network retirement found a sender/configuration type mismatch".into());
        return;
    };
    match retained.send_dmx_frame(config.universe, &[0; 512]) {
        Ok(size) if size == expected_size => {}
        Ok(size) => errors.push(format!(
            "Managed network retirement zero send wrote {size} of {expected_size} bytes"
        )),
        Err(error) => errors.push(format!(
            "Managed network retirement zero send failed: {error}"
        )),
    }
    // Drop the network transport, including on an ambiguous send. The failure
    // fence and S0 remain engaged; no persistent sender/reconnect is installed.
}

#[cfg(test)]
#[path = "managed_network_dmx_retirement_tests.rs"]
mod tests;
