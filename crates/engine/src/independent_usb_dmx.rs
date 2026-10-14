//! Machine-local Open DMX cables for universes other than the U0 Unity mirror.
//!
//! Each cable outputs one universe from its own worker behind its own device
//! gate. The S0 latch is shared with the U0 cable: S0, and every USB fault
//! (which engages S0), zeros every cable at its next frame. A cable needs no
//! Art-Net route. Like the U0 cable, nothing here is persisted in a project;
//! the device identity comes from the machine-local binding store.

use super::*;

/// At most this many USB-DMX cables besides the U0 Unity-mirror cable.
pub const MAX_INDEPENDENT_USB_DMX_ROUTES: usize = 3;

/// Highest universe a USB-DMX cable may output (the Art-Net range).
const MAX_INDEPENDENT_USB_DMX_UNIVERSE: u16 = 32_767;

const UNASSIGNED_UNIVERSE: u32 = u32::MAX;

const INDEPENDENT_ARTNET_DETAIL: &str =
    "Independent USB-DMX universe: no Art-Net mirror is required.";

/// Shared, thread-safe state of one cable slot. The gate and status outlive
/// any one worker so a fault and its zero-write holds stay attached to the
/// cable's universe until it is re-armed or explicitly stopped.
pub(super) struct IndependentUsbDmxSlotTelemetry {
    pub(super) gate: OpenDmxSafetyWriteGate,
    pub(super) status: ShowSerialDmxRouteStatusStore,
    universe: AtomicU32,
}

impl IndependentUsbDmxSlotTelemetry {
    pub(super) fn new(root_gate: &OpenDmxSafetyWriteGate) -> Self {
        Self {
            gate: root_gate.device_gate(),
            status: ShowSerialDmxRouteStatusStore::new(),
            universe: AtomicU32::new(UNASSIGNED_UNIVERSE),
        }
    }

    pub(super) fn universe(&self) -> Option<u16> {
        u16::try_from(self.universe.load(Ordering::Acquire)).ok()
    }

    fn assign(&self, universe: u16) {
        self.universe.store(u32::from(universe), Ordering::Release);
    }
}

/// One running cable worker. Runtime-only, like `RuntimeShowSerialDmxRoute`.
pub(super) struct RuntimeIndependentUsbDmxRoute {
    universe: u16,
    device: io::serial_dmx::VerifiedUsbSerialPortIdentity,
    sender: Option<DmxSender>,
    /// The queue state last published (`Some(live)`), so the show-cadence
    /// tick publishes, and allocates, only on a change.
    published_live: Option<bool>,
}

/// Status of one cable slot. `universe` is the universe the slot last served;
/// `None` means the slot was never used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndependentUsbDmxRouteStatusSnapshot {
    pub slot: usize,
    pub universe: Option<u16>,
    pub revision: u64,
    pub status: ShowSerialDmxRouteStatus,
}

fn independent_status(
    active: bool,
    zero_frame_queued: bool,
    zero_frame_physical_write_completed: bool,
    live_frame_queued: bool,
    worker_shutdown_completed: bool,
    faulted: bool,
    detail: String,
) -> ShowSerialDmxRouteStatus {
    ShowSerialDmxRouteStatus {
        active,
        zero_frame_queued,
        zero_frame_physical_write_completed,
        live_frame_queued,
        worker_shutdown_completed,
        faulted,
        artnet_mirror_live: false,
        artnet_mirror_detail: INDEPENDENT_ARTNET_DETAIL.to_string(),
        detail,
    }
}

/// The same physical cable, by any of its stable identities.
fn same_usb_dmx_device(
    left: &io::serial_dmx::VerifiedUsbSerialPortIdentity,
    right: &io::serial_dmx::VerifiedUsbSerialPortIdentity,
) -> bool {
    let same_instance = |left: &Option<String>, right: &Option<String>| matches!((left, right), (Some(left), Some(right)) if left.eq_ignore_ascii_case(right));
    left.port_name.eq_ignore_ascii_case(&right.port_name)
        || same_instance(
            &left.windows_device_instance_id,
            &right.windows_device_instance_id,
        )
        || same_instance(
            &left.macos_device_instance_id,
            &right.macos_device_instance_id,
        )
}

/// Never used, or stopped with a joined worker and no fault or incomplete
/// zero transaction: the only slot state that proves no worker can write.
fn slot_status_is_clean(status: &ShowSerialDmxRouteStatus) -> bool {
    !status.active
        && !status.live_frame_queued
        && status.worker_shutdown_completed
        && !status.faulted
        && !(status.zero_frame_queued && !status.zero_frame_physical_write_completed)
}

impl EngineSharedTelemetry {
    /// Coherent per-slot snapshots; a poisoned or exhausted slot status is an
    /// error, never a synthetic stopped state.
    pub(super) fn try_independent_usb_dmx_route_statuses(
        &self,
    ) -> Result<Vec<IndependentUsbDmxRouteStatusSnapshot>, String> {
        self.independent_usb_dmx
            .iter()
            .enumerate()
            .map(|(slot, telemetry)| {
                let snapshot = telemetry.status.try_snapshot()?;
                Ok(IndependentUsbDmxRouteStatusSnapshot {
                    slot,
                    universe: telemetry.universe(),
                    revision: snapshot.revision,
                    status: snapshot.status,
                })
            })
            .collect()
    }

    /// Any latched cable fault blocks an S0 release, as the U0 fault does.
    pub(super) fn any_independent_usb_dmx_route_faulted(&self) -> bool {
        self.independent_usb_dmx
            .iter()
            .any(|telemetry| telemetry.status.fail_closed_snapshot().status.faulted)
    }
}

impl EngineRuntime {
    fn independent_usb_dmx_slot_for_universe(&self, universe: u16) -> Option<usize> {
        (0..MAX_INDEPENDENT_USB_DMX_ROUTES).find(|slot| {
            self.shared_telemetry.independent_usb_dmx[*slot].universe() == Some(universe)
        })
    }

    /// A slot already bound to `universe` (keeping its fault history), else a
    /// never-used or cleanly stopped slot.
    fn independent_usb_dmx_slot_for_enable(&self, universe: u16) -> Result<usize, String> {
        if let Some(slot) = self.independent_usb_dmx_slot_for_universe(universe) {
            return Ok(slot);
        }
        (0..MAX_INDEPENDENT_USB_DMX_ROUTES)
            .find(|slot| {
                self.independent_usb_dmx_routes[*slot].is_none()
                    && self.shared_telemetry.independent_usb_dmx[*slot]
                        .status
                        .try_snapshot()
                        .is_ok_and(|snapshot| slot_status_is_clean(&snapshot.status))
            })
            .ok_or_else(|| {
                format!(
                    "USB-DMX supports at most {MAX_INDEPENDENT_USB_DMX_ROUTES} cables besides Universe 0; stop one first (a faulted cable stays reserved until it is re-armed or stopped)"
                )
            })
    }

    pub(super) fn apply_independent_usb_dmx_route_enable(
        &mut self,
        universe: u16,
        expected_device: &io::serial_dmx::VerifiedUsbSerialPortIdentity,
        expected_safety_epoch: u64,
        expected_safety_generation: u64,
    ) -> Result<(), String> {
        self.apply_independent_usb_dmx_route_enable_with_sender_factory(
            universe,
            expected_device,
            expected_safety_epoch,
            expected_safety_generation,
            create_verified_show_serial_dmx_sender_bounded,
        )
    }

    pub(super) fn apply_independent_usb_dmx_route_enable_with_sender_factory<F>(
        &mut self,
        universe: u16,
        expected_device: &io::serial_dmx::VerifiedUsbSerialPortIdentity,
        expected_safety_epoch: u64,
        expected_safety_generation: u64,
        create_sender: F,
    ) -> Result<(), String>
    where
        F: FnOnce(
            &io::serial_dmx::VerifiedUsbSerialPortIdentity,
            &OpenDmxSafetyWriteGate,
        ) -> Result<DmxSender, String>,
    {
        if universe == SHOW_SERIAL_DMX_UNIVERSE || universe > MAX_INDEPENDENT_USB_DMX_UNIVERSE {
            return Err(format!(
                "USB-DMX Universe {universe} is not available here: Universe 0 is the Unity-mirror cable, and others must be 1–{MAX_INDEPENDENT_USB_DMX_UNIVERSE}"
            ));
        }
        if !show_serial_dmx_device_identity_is_exact(expected_device) {
            return Err(
                "USB-DMX worker requires one exact machine-local USB/PnP identity".to_string(),
            );
        }
        if self
            .independent_usb_dmx_routes
            .iter()
            .flatten()
            .any(|route| route.universe == universe)
        {
            return Err(format!(
                "USB-DMX Universe {universe} is already active; stop it before another activation"
            ));
        }
        if self
            .independent_usb_dmx_routes
            .iter()
            .flatten()
            .any(|route| same_usb_dmx_device(&route.device, expected_device))
        {
            return Err(
                "This USB-DMX cable already drives another universe; one cable outputs one universe"
                    .to_string(),
            );
        }
        let slot = self.independent_usb_dmx_slot_for_enable(universe)?;
        let shared_telemetry = Arc::clone(&self.shared_telemetry);
        let slot_telemetry = &shared_telemetry.independent_usb_dmx[slot];
        slot_telemetry.assign(universe);
        // A bounded stop or fault timeout may have detached a worker that can
        // still finish a driver call; only status proving it joined admits a
        // replacement. A faulted cable re-arms only under both S0 latches.
        let prior_status = slot_telemetry.status.try_snapshot().map_err(|error| {
            format!(
                "USB-DMX Universe {universe} activation was rejected because its prior status is unavailable: {error}"
            )
        })?;
        let (safety_authority_engaged, physical_s0_latched) = if prior_status.status.faulted {
            let safety = shared_telemetry.try_safety_blackout_authority()?;
            let physical = slot_telemetry.gate.blackout_engaged()?;
            (safety.engaged, physical)
        } else {
            (false, false)
        };
        require_show_serial_dmx_enable_admission(
            &prior_status.status,
            safety_authority_engaged,
            physical_s0_latched,
        )
        .map_err(|error| format!("USB-DMX Universe {universe} activation was rejected: {error}"))?;
        let ownership = self.output_ownership_gate.status();
        if ownership.state != OutputOwnershipState::Ready
            || ownership.effective_role != MachineOutputRole::Both
            || ownership.desired_role != MachineOutputRole::Both
            || !ownership.lighting_allowed
        {
            return Err(
                "USB-DMX worker requires the active local Both lighting authority".to_string(),
            );
        }

        // Device revalidation happens inside the constructor; a missing,
        // changed or busy port leaves runtime state untouched.
        let mut sender = create_sender(expected_device, &slot_telemetry.gate)?;
        let safety_matches = match shared_telemetry.safety_blackout_enqueue_gate.lock() {
            Ok(_enqueue_gate) => {
                shared_telemetry
                    .try_safety_blackout_authority()
                    .is_ok_and(|safety| {
                        safety.epoch == expected_safety_epoch
                            && safety.generation == expected_safety_generation
                    })
            }
            Err(_) => false,
        };
        if !safety_matches {
            // The worker has written nothing yet; it must still join, or
            // the slot refuses a replacement until it does.
            if let Err(error) = sender.shutdown_open_dmx_bounded(SHOW_SERIAL_DMX_ZERO_WRITE_TIMEOUT)
            {
                self.safety_blackout_engaged = true;
                let _ = shared_telemetry.engage_safety_blackout_for_usb_device_fault(
                    SHOW_SERIAL_DMX_ZERO_WRITE_TIMEOUT,
                    &slot_telemetry.gate,
                );
                slot_telemetry.status.set(independent_status(
                    false,
                    false,
                    false,
                    false,
                    false,
                    true,
                    format!(
                        "USB-DMX worker for Universe {universe} was rejected before its first frame and did not complete bounded shutdown: {error}. S0 remains engaged."
                    ),
                ));
            }
            return Err(
                "USB-DMX worker requires the exact currently-observed S0 safety blackout authority"
                    .to_string(),
            );
        }
        // Keep this cable zero-only while its first real zero transaction is
        // confirmed, without touching any other cable.
        let _zero_write_hold = match slot_telemetry
            .gate
            .reserve_startup_zero_write_bounded(SHOW_SERIAL_DMX_ZERO_WRITE_TIMEOUT)
        {
            Ok(reservation) => reservation,
            Err(error) => {
                return Err(
                    self.fail_independent_usb_dmx_activation_after_worker_creation(
                        slot,
                        &mut sender,
                        false,
                        "could not reserve its initial physical S0 zero write",
                        error,
                    ),
                );
            }
        };
        let zero_receipt = match sender.open_dmx_zero_write_receipt() {
            Ok(receipt) => receipt,
            Err(error) => {
                return Err(
                    self.fail_independent_usb_dmx_activation_after_worker_creation(
                        slot,
                        &mut sender,
                        false,
                        "could not prepare its initial S0 blackout receipt",
                        error,
                    ),
                );
            }
        };
        if let Err(error) = sender.send_dmx_frame(universe, &[0u8; 512]) {
            return Err(
                self.fail_independent_usb_dmx_activation_after_worker_creation(
                    slot,
                    &mut sender,
                    false,
                    "could not queue its initial S0 blackout",
                    error,
                ),
            );
        }
        if let Err(error) =
            sender.wait_for_open_dmx_zero_write(zero_receipt, SHOW_SERIAL_DMX_ZERO_WRITE_TIMEOUT)
        {
            return Err(
                self.fail_independent_usb_dmx_activation_after_worker_creation(
                    slot,
                    &mut sender,
                    true,
                    "did not complete its initial physical S0 zero write",
                    error,
                ),
            );
        }
        self.independent_usb_dmx_routes[slot] = Some(RuntimeIndependentUsbDmxRoute {
            universe,
            device: expected_device.clone(),
            sender: Some(sender),
            published_live: None,
        });
        self.clear_managed_show_dmx_fail_stop_completion();
        slot_telemetry.status.set(independent_status(
            true,
            true,
            true,
            false,
            false,
            false,
            format!(
                "Open DMX worker for Universe {universe} started after its initial physical zero transaction. The existing safety blackout state is preserved. This is not fixture or wire-delivery verification."
            ),
        ));
        Ok(())
    }

    /// A worker exists here, so every activation failure consumes it through
    /// the bounded shutdown and leaves a sticky S0 fault on this cable.
    fn fail_independent_usb_dmx_activation_after_worker_creation(
        &mut self,
        slot: usize,
        sender: &mut DmxSender,
        zero_frame_queued: bool,
        stage: &str,
        error: String,
    ) -> String {
        let shared_telemetry = Arc::clone(&self.shared_telemetry);
        let slot_telemetry = &shared_telemetry.independent_usb_dmx[slot];
        let _ = shared_telemetry.engage_safety_blackout_for_usb_device_fault(
            SHOW_SERIAL_DMX_ZERO_WRITE_TIMEOUT,
            &slot_telemetry.gate,
        );
        let shutdown_error = sender
            .shutdown_open_dmx_bounded(SHOW_SERIAL_DMX_ZERO_WRITE_TIMEOUT)
            .err();
        self.safety_blackout_engaged = true;
        let universe = slot_telemetry
            .universe()
            .map_or_else(|| "?".to_string(), |universe| universe.to_string());
        let detail = format!(
            "USB-DMX worker for Universe {universe} {stage}: {error}. Initial S0 queue acceptance={zero_frame_queued}; physical zero completion=false; bounded shutdown fault={}. S0 remains engaged; a fixture may retain its last look, and USB electrical safety is unverified.",
            shutdown_error.as_deref().unwrap_or("none"),
        );
        slot_telemetry.status.set(independent_status(
            false,
            zero_frame_queued,
            false,
            false,
            shutdown_error.is_none(),
            true,
            detail.clone(),
        ));
        self.last_error = Some(detail.clone());
        detail
    }

    /// Stop one cable under the exact engaged S0: one confirmed zero
    /// transaction, then a bounded shutdown. A faulted cable whose worker has
    /// already joined is instead taken out of service, which lets S0 be
    /// released without that cable; its last physical zero stays unconfirmed.
    pub(super) fn apply_independent_usb_dmx_route_stop(
        &mut self,
        universe: u16,
        expected_safety_epoch: u64,
        expected_safety_generation: u64,
    ) -> Result<(), String> {
        let Some(slot) = self.independent_usb_dmx_slot_for_universe(universe) else {
            return Err(format!("USB-DMX Universe {universe} has no cable"));
        };
        let safety = self.shared_telemetry.safety_blackout_authority();
        if !safety.engaged
            || safety.epoch != expected_safety_epoch
            || safety.generation != expected_safety_generation
        {
            return Err(
                "USB-DMX stop requires the exact currently-engaged S0 safety blackout authority"
                    .to_string(),
            );
        }
        let shared_telemetry = Arc::clone(&self.shared_telemetry);
        let slot_telemetry = &shared_telemetry.independent_usb_dmx[slot];
        if self.independent_usb_dmx_routes[slot].is_none() {
            let status = slot_telemetry.status.try_snapshot()?.status;
            if status.faulted && status.worker_shutdown_completed && !status.active {
                // A clean stopped slot: no queued or pending zero remains, so
                // the cable (or another) can be armed again from zero.
                slot_telemetry.status.set(independent_status(
                    false,
                    false,
                    false,
                    false,
                    true,
                    false,
                    format!(
                        "USB-DMX Universe {universe} was taken out of service after a fault, under S0. Its last physical zero is {}; a fixture on that cable may retain its last look.",
                        if status.zero_frame_physical_write_completed { "confirmed" } else { "unconfirmed" }
                    ),
                ));
                return Ok(());
            }
            return Err(format!("USB-DMX Universe {universe} is not active"));
        }
        slot_telemetry.gate.latch_blackout();
        let _enqueue_gate = shared_telemetry
            .safety_blackout_enqueue_gate
            .lock()
            .map_err(|_| {
                "Safety blackout enqueue gate was poisoned before USB-DMX stop".to_string()
            })?;
        let mut route = self.independent_usb_dmx_routes[slot]
            .take()
            .expect("presence was checked");
        let mut sender = route
            .sender
            .take()
            .ok_or_else(|| "USB-DMX worker sender was unexpectedly absent".to_string())?;
        let _zero_write_hold = match slot_telemetry
            .gate
            .reserve_zero_write_bounded(SHOW_SERIAL_DMX_ZERO_WRITE_TIMEOUT)
        {
            Ok(reservation) => reservation,
            Err(gate_error) => {
                let shutdown_error = sender
                    .shutdown_open_dmx_bounded(SHOW_SERIAL_DMX_ZERO_WRITE_TIMEOUT)
                    .err();
                self.safety_blackout_engaged = true;
                let detail = format!(
                    "USB-DMX Universe {universe} stop could not acquire its physical S0 gate within the bounded interval: {gate_error}. No physical zero transaction was confirmed; a fixture may retain its last look. Engine S0 remains engaged. bounded shutdown fault={}",
                    shutdown_error.as_deref().unwrap_or("none"),
                );
                slot_telemetry.status.set(independent_status(
                    false,
                    false,
                    false,
                    false,
                    shutdown_error.is_none(),
                    true,
                    detail.clone(),
                ));
                self.last_error = Some(detail.clone());
                return Err(detail);
            }
        };
        let zero_receipt = sender.open_dmx_zero_write_receipt();
        let zero_queued = sender.send_dmx_frame(universe, &[0u8; 512]).is_ok();
        let zero_completion = match (zero_receipt, zero_queued) {
            (Ok(receipt), true) => {
                sender.wait_for_open_dmx_zero_write(receipt, SHOW_SERIAL_DMX_ZERO_WRITE_TIMEOUT)
            }
            (Err(error), _) => Err(error),
            (_, false) => Err("USB-DMX worker rejected its stop zero frame".to_string()),
        };
        let stop_result = sender.shutdown_open_dmx_bounded(SHOW_SERIAL_DMX_ZERO_WRITE_TIMEOUT);
        let worker_shutdown_completed = stop_result.is_ok();
        match (zero_completion, stop_result) {
            (Ok(()), Ok(())) => {
                slot_telemetry.status.set(independent_status(
                    false,
                    zero_queued,
                    true,
                    false,
                    true,
                    false,
                    format!(
                        "Open DMX worker for Universe {universe} stopped after an S0 zero BREAK/MAB/write_all/flush transaction completed. This is not fixture or wire-delivery verification."
                    ),
                ));
                Ok(())
            }
            (zero_completion, stop_result) => {
                self.safety_blackout_engaged = true;
                let zero_detail = zero_completion.err().unwrap_or_default();
                let stop_detail = stop_result.err().unwrap_or_default();
                let detail = format!(
                    "USB-DMX Universe {universe} stop faulted without a confirmed physical S0 zero completion; a fixture may retain its last look. Engine S0 remains engaged. zero receipt: {zero_detail}; bounded shutdown: {stop_detail}"
                );
                slot_telemetry.status.set(independent_status(
                    false,
                    zero_queued,
                    false,
                    false,
                    worker_shutdown_completed,
                    true,
                    detail.clone(),
                ));
                self.last_error = Some(detail.clone());
                Err(detail)
            }
        }
    }

    /// Any cable fault is an S0 transition for every route: the shared latch
    /// is set before any status or authority mutex, then this cable gets one
    /// bounded zero attempt and a bounded shutdown.
    pub(super) fn fault_independent_usb_dmx_route(&mut self, slot: usize, cause: &str) {
        self.clear_managed_show_dmx_fail_stop_completion();
        let shared_telemetry = Arc::clone(&self.shared_telemetry);
        let slot_telemetry = &shared_telemetry.independent_usb_dmx[slot];
        slot_telemetry.gate.latch_blackout();
        let universe = self.independent_usb_dmx_routes[slot]
            .as_ref()
            .map(|route| route.universe)
            .or_else(|| slot_telemetry.universe())
            .map_or_else(|| "?".to_string(), |universe| universe.to_string());
        slot_telemetry.status.set(independent_status(
            false,
            false,
            false,
            false,
            false,
            true,
            format!(
                "USB-DMX Universe {universe} fault is engaging S0: {cause}. Physical zero delivery is not yet confirmed."
            ),
        ));
        let safety_result = shared_telemetry.engage_safety_blackout_for_usb_device_fault(
            SHOW_SERIAL_DMX_ZERO_WRITE_TIMEOUT,
            &slot_telemetry.gate,
        );
        self.safety_blackout_engaged = true;
        let physical_gate_confirmed =
            matches!(&safety_result, Ok((_, authority, Ok(()))) if authority.engaged);
        let mut zero_queued = false;
        let mut zero_physical_write_completed = false;
        let mut zero_write_error = match &safety_result {
            Ok((_, _, Err(error))) => Some(format!(
                "physical S0 gate did not become available: {error}; no physical zero transaction was attempted"
            )),
            Err(error) => Some(format!(
                "S0 authority transition failed: {error}; no physical zero transaction was attempted"
            )),
            Ok((_, authority, Ok(()))) if !authority.engaged => Some(
                "S0 authority was not engaged; no physical zero transaction was attempted"
                    .to_string(),
            ),
            Ok(_) => None,
        };
        let mut shutdown_error = None;
        if let Some(mut route) = self.independent_usb_dmx_routes[slot].take() {
            if let Some(mut sender) = route.sender.take() {
                if physical_gate_confirmed {
                    let zero_result = (|| -> Result<(), String> {
                        let _enqueue_gate = shared_telemetry
                            .safety_blackout_enqueue_gate
                            .lock()
                            .map_err(|_| {
                                "Safety blackout enqueue gate was poisoned during USB-DMX fault"
                                    .to_string()
                            })?;
                        let _zero_write_hold = slot_telemetry
                            .gate
                            .reserve_zero_write_bounded(SHOW_SERIAL_DMX_ZERO_WRITE_TIMEOUT)
                            .map_err(|error| {
                                format!("USB-DMX fault could not reserve bounded S0 zero write: {error}")
                            })?;
                        let receipt = sender.open_dmx_zero_write_receipt()?;
                        sender.send_dmx_frame(route.universe, &[0u8; 512])?;
                        zero_queued = true;
                        sender.wait_for_open_dmx_zero_write(
                            receipt,
                            SHOW_SERIAL_DMX_ZERO_WRITE_TIMEOUT,
                        )?;
                        zero_physical_write_completed = true;
                        Ok(())
                    })();
                    if let Err(error) = zero_result {
                        zero_write_error = Some(error);
                    }
                }
                // Always consume the worker through the bounded path so a
                // wedge detaches instead of reaching an unbounded join.
                if let Err(error) =
                    sender.shutdown_open_dmx_bounded(SHOW_SERIAL_DMX_ZERO_WRITE_TIMEOUT)
                {
                    shutdown_error = Some(error);
                }
            }
        }
        let detail = format!(
            "USB-DMX Universe {universe} faulted: {cause}. zero queued={zero_queued}; physical zero completion={zero_physical_write_completed}; zero receipt fault={}; bounded shutdown fault={}. A fixture may retain its last look when physical zero completion is false; engine S0 remains engaged. USB electrical safety is unverified. No fixture or wire delivery was verified.",
            zero_write_error.as_deref().unwrap_or("none"),
            shutdown_error.as_deref().unwrap_or("none"),
        );
        self.last_error = Some(detail.clone());
        slot_telemetry.status.set(independent_status(
            false,
            zero_queued,
            zero_physical_write_completed,
            false,
            shutdown_error.is_none(),
            true,
            detail,
        ));
    }

    /// Fault every running cable (ownership loss, ambiguous retirement).
    pub(super) fn fault_all_independent_usb_dmx_routes(&mut self, cause: &str) {
        for slot in 0..MAX_INDEPENDENT_USB_DMX_ROUTES {
            if self.independent_usb_dmx_routes[slot].is_some() {
                self.fault_independent_usb_dmx_route(slot, cause);
            }
        }
    }

    /// Stop every running cable under the exact engaged S0, for a managed
    /// retirement. Each failure is reported; the caller treats any as InDoubt.
    pub(super) fn stop_all_independent_usb_dmx_routes(
        &mut self,
        expected_safety_epoch: u64,
        expected_safety_generation: u64,
    ) -> Vec<String> {
        let mut errors = Vec::new();
        for slot in 0..MAX_INDEPENDENT_USB_DMX_ROUTES {
            let Some(universe) = self.independent_usb_dmx_routes[slot]
                .as_ref()
                .map(|route| route.universe)
            else {
                continue;
            };
            if let Err(error) = self.apply_independent_usb_dmx_route_stop(
                universe,
                expected_safety_epoch,
                expected_safety_generation,
            ) {
                errors.push(format!("USB-DMX Universe {universe}: {error}"));
            }
        }
        errors
    }

    pub(super) fn independent_usb_dmx_routes_absent(&self) -> bool {
        self.independent_usb_dmx_routes.iter().all(Option::is_none)
    }

    /// Fail-stop terminal proof: no cable is running and every slot is
    /// never-used or cleanly stopped. A faulted or detached cable (whose
    /// worker may still be inside a driver call) is not "absent".
    pub(super) fn independent_usb_dmx_terminal_absent(&self) -> bool {
        self.independent_usb_dmx_routes_absent()
            && self
                .shared_telemetry
                .independent_usb_dmx
                .iter()
                .all(|slot| {
                    slot.status
                        .try_snapshot()
                        .is_ok_and(|snapshot| slot_status_is_clean(&snapshot.status))
                })
    }

    /// Copy each running cable's completed frame before the network loop
    /// drains `frames`. S0 selects zero.
    pub(super) fn capture_independent_usb_dmx_frames(
        &self,
        frames: &HashMap<u16, [u8; 512]>,
        zero_selected: bool,
        now: Instant,
    ) -> [[u8; 512]; MAX_INDEPENDENT_USB_DMX_ROUTES] {
        let mut captured = [[0u8; 512]; MAX_INDEPENDENT_USB_DMX_ROUTES];
        if zero_selected {
            return captured;
        }
        for (slot, route) in self.independent_usb_dmx_routes.iter().enumerate() {
            if let Some(route) = route {
                captured[slot] = frames
                    .get(&route.universe)
                    .copied()
                    .unwrap_or_else(|| self.render_dmx_frame_for_universe(route.universe, now));
            }
        }
        captured
    }

    /// Queue each running cable's frame at the safety boundary. Losing the
    /// lighting authority or a rejected frame faults that cable (and S0).
    pub(super) fn send_independent_usb_dmx_frames(
        &mut self,
        frames: &[[u8; 512]; MAX_INDEPENDENT_USB_DMX_ROUTES],
        lighting_allowed: bool,
        zero_selected: bool,
    ) {
        for slot in 0..MAX_INDEPENDENT_USB_DMX_ROUTES {
            if self.independent_usb_dmx_routes[slot].is_none() {
                continue;
            }
            if !lighting_allowed {
                self.fault_independent_usb_dmx_route(
                    slot,
                    "lighting output authority was lost while the cable was active",
                );
                continue;
            }
            let route = self.independent_usb_dmx_routes[slot]
                .as_mut()
                .expect("presence was checked");
            let universe = route.universe;
            let sent = send_dmx_frame_at_safety_boundary(
                &mut route.sender,
                universe,
                &frames[slot],
                &self.shared_telemetry,
            );
            match sent {
                Err(error) => self.fault_independent_usb_dmx_route(
                    slot,
                    &format!("Open DMX worker rejected a completed frame: {error}"),
                ),
                Ok(_) => {
                    let live = !zero_selected;
                    if route.published_live != Some(live) {
                        route.published_live = Some(live);
                        self.shared_telemetry.independent_usb_dmx[slot].status.set(
                            independent_status(
                                true,
                                !live,
                                true,
                                live,
                                false,
                                false,
                                if live {
                                    format!("Open DMX worker is active; Universe {universe} frames are queued at show cadence. Queue acceptance is not fixture or wire-delivery verification.")
                                } else {
                                    format!("Open DMX worker for Universe {universe} is active with the latest S0 zero frame queued. Queue acceptance is not fixture or wire-delivery verification.")
                                },
                            ),
                        );
                    }
                }
            }
        }
    }
}
