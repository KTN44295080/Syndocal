//! Application-owned show-ASIO output session.
//!
//! This owns one bridge session, two logical stereo Rodio mixers and exactly
//! one non-realtime drain worker. It deliberately has no CPAL/WASAPI fallback.

use crate::{
    asio_bridge_v3::{
        BridgeV3Fault, BridgeV3Module, BridgeV3Session, BridgeV3SessionPhase, BridgeV3StartError,
        OutputStartRequestV3,
    },
    asio_program_cue::{
        AsioPreflightIdentity, AsioPreflightState, MachineAsioOutputProfile, OutputProfileLock,
        PreflightPhase, PreflightSolo, PreflightStateError, PreflightTarget,
    },
    asio_program_cue_render::{
        preflight_now_ms, AsioRenderContext, PreflightRenderControl, ProgramCueMixers,
        ProgramCueRenderWorker, RenderFault, TransportGeneration,
    },
    asio_timeline_output::{AsioTimelineOutputSelector, TimelineOutputIdentity},
};
use engine::TimelineAudioLiveFence;
use protocol::TimelineAudioOutputBus;
use std::{
    ffi::c_void,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OutputCapabilitiesV3 {
    schema_version: u32,
    kind: String,
    abi_version: u32,
    backend: String,
    built: bool,
    driver: OutputCapabilityDriver,
    output: OutputCapabilityTuple,
    input: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OutputCapabilityDriver {
    id: String,
    name: String,
}
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OutputCapabilityTuple {
    channels: u32,
    native_format: String,
    sample_rates_hz: Vec<u32>,
    fixed_buffer_frames: OutputBufferConstraints,
}
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OutputBufferConstraints {
    min: u32,
    max: u32,
    preferred: u32,
    granularity: i32,
}

impl OutputCapabilitiesV3 {
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let parsed: Self = serde_json::from_str(text)
            .map_err(|error| format!("ASIO output capabilities JSON is invalid: {error}"))?;
        if parsed.schema_version != 3
            || parsed.kind != "capabilities"
            || parsed.abi_version != 3
            || parsed.backend != "asio-sdk-v3-rt"
            || !parsed.built
            || parsed.driver.id.trim() != parsed.driver.id
            || !parsed.driver.id.starts_with("asio:")
            || parsed.driver.name.trim().is_empty()
            || parsed.input.is_some()
                && parsed
                    .input
                    .as_ref()
                    .is_some_and(|value| !value.is_object())
            || parsed.output.channels == 0
            || !matches!(
                parsed.output.native_format.as_str(),
                "f32" | "i16" | "i24" | "i32" | "f64"
            )
            || !parsed.output.sample_rates_hz.contains(&48_000)
            || parsed.output.fixed_buffer_frames.min == 0
            || parsed.output.fixed_buffer_frames.min > parsed.output.fixed_buffer_frames.max
            || parsed.output.fixed_buffer_frames.preferred < parsed.output.fixed_buffer_frames.min
            || parsed.output.fixed_buffer_frames.preferred > parsed.output.fixed_buffer_frames.max
            || parsed.output.fixed_buffer_frames.granularity < -1
        {
            return Err(
                "ASIO output capabilities are not an exact usable v3 output tuple".to_owned(),
            );
        }
        Ok(parsed)
    }

    pub(crate) fn validate_profile(
        &self,
        profile: &MachineAsioOutputProfile,
    ) -> Result<(), String> {
        profile.validate().map_err(profile_error)?;
        if profile.driver_id() != self.driver.id
            || profile.sample_rate_hz() != 48_000
            || profile.device_output_channels() > self.output.channels
            || profile.native_format() != self.output.native_format
            || profile.fixed_buffer_frames() < self.output.fixed_buffer_frames.min
            || profile.fixed_buffer_frames() > self.output.fixed_buffer_frames.max
        {
            return Err(
                "ASIO output profile no longer matches freshly enumerated driver capabilities"
                    .to_owned(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AsioOutputLifecycle {
    Locked,
    Starting,
    Active,
    StoppedPendingClose(String),
    Fault(String),
    Closed,
}

/// Snapshot of the ephemeral preflight controls for the exact current ASIO
/// session/transport pair.  This is intentionally not persisted or attached
/// to the machine profile; UI handlers use it only to publish a successful
/// command result after the runtime has rechecked its active identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AsioPreflightStatus {
    identity: AsioPreflightIdentity,
    phase: PreflightPhase,
    live_playback_active: bool,
    test: Option<PreflightTarget>,
    solo: PreflightSolo,
}

impl AsioPreflightStatus {
    pub(crate) fn identity(self) -> AsioPreflightIdentity {
        self.identity
    }

    pub(crate) fn live_playback_active(self) -> bool {
        self.live_playback_active
    }

    pub(crate) fn test(self) -> Option<PreflightTarget> {
        self.test
    }

    pub(crate) fn solo(self) -> PreflightSolo {
        self.solo
    }
}

pub(crate) struct AsioOutputRuntime {
    lifecycle: AsioOutputLifecycle,
    timeline_audio_live_fence: TimelineAudioLiveFence,
    session: Option<BridgeV3Session>,
    context: Option<Arc<AsioRenderContext>>,
    session_generation: Option<u64>,
    preflight: Option<Arc<PreflightRenderControl>>,
    program_mixer: Option<rodio::mixer::Mixer>,
    cue_mixer: Option<rodio::mixer::Mixer>,
    worker_stop: Option<Arc<AtomicBool>>,
    worker: Option<JoinHandle<()>>,
}

impl Default for AsioOutputRuntime {
    fn default() -> Self {
        Self {
            lifecycle: AsioOutputLifecycle::Locked,
            timeline_audio_live_fence: TimelineAudioLiveFence::default(),
            session: None,
            context: None,
            session_generation: None,
            preflight: None,
            program_mixer: None,
            cue_mixer: None,
            worker_stop: None,
            worker: None,
        }
    }
}

impl AsioOutputRuntime {
    pub(crate) fn lifecycle(&self) -> &AsioOutputLifecycle {
        &self.lifecycle
    }

    /// Bind the engine-owned Timeline audio liveness authority before a
    /// native session is admitted. Cloning preserves one shared atomic word;
    /// copying only the current value would reopen the live/Test race.
    pub(crate) fn bind_timeline_audio_live_fence(
        &mut self,
        fence: &TimelineAudioLiveFence,
    ) -> Result<(), String> {
        if !matches!(
            self.lifecycle,
            AsioOutputLifecycle::Locked | AsioOutputLifecycle::Closed
        ) || self.session.is_some()
            || self.context.is_some()
            || self.preflight.is_some()
            || self.session_generation.is_some()
            || self.program_mixer.is_some()
            || self.cue_mixer.is_some()
            || self.worker_stop.is_some()
            || self.worker.is_some()
        {
            return Err(
                "Timeline audio live fence binding is only allowed before ASIO Start".to_owned(),
            );
        }
        self.timeline_audio_live_fence = fence.clone();
        Ok(())
    }

    /// Capture the exact active session/transport identity used by ephemeral
    /// preflight operations.  The identity is never inferred from a caller's
    /// transport-only value and is rejected once the render context faults.
    pub(crate) fn current_preflight_identity(&mut self) -> Result<AsioPreflightIdentity, String> {
        let (_, identity) = self.active_preflight_control()?;
        Ok(identity)
    }

    /// Query preflight state against the current active identity. `now_ms` is
    /// supplied by the caller so tests and UI command handlers can use an
    /// explicit clock domain without putting a clock read on the callback.
    pub(crate) fn preflight_status(&mut self, now_ms: u64) -> Result<AsioPreflightStatus, String> {
        let (_, identity) = self.active_preflight_control()?;
        self.preflight_status_for_identity(identity, now_ms)
    }

    /// Set one bounded test against the exact active session/transport pair.
    /// `PreflightTarget::Off` clears only the test; a non-Off target is
    /// mutually exclusive with an active solo through `AsioPreflightState`.
    pub(crate) fn set_preflight_test(
        &mut self,
        target: PreflightTarget,
        now_ms: u64,
        duration_ms: u64,
    ) -> Result<AsioPreflightStatus, String> {
        let identity = self.current_preflight_identity()?;
        self.set_preflight_test_for_identity(identity, target, now_ms, duration_ms)
    }

    pub(crate) fn set_preflight_test_for_identity(
        &mut self,
        expected: AsioPreflightIdentity,
        target: PreflightTarget,
        now_ms: u64,
        duration_ms: u64,
    ) -> Result<AsioPreflightStatus, String> {
        let (control, actual) = self.active_preflight_control()?;
        ensure_preflight_identity(expected, actual)?;
        {
            let mut state = control
                .state()
                .lock()
                .map_err(|_| self.fail_closed_preflight_state_lock(&control))?;
            let mut previous_signature = control.selection_signature();
            let expired = state
                .clear_if_expired(expected, now_ms)
                .map_err(preflight_error)?;
            if expired {
                self.advance_preflight_epoch(&control)?;
                control.publish_selection_kind(&state);
                previous_signature = 0;
            }
            let result = state.begin_test_with_mapper_and_live_fence(
                control.mapper(),
                target,
                expected,
                now_ms,
                duration_ms,
                Some(control.timeline_audio_live_fence()),
                control.live_drain_active(),
            );
            if let Err(error) = result {
                if state.selection_signature() != previous_signature {
                    self.apply_selection_transition(&control, &state, previous_signature)?;
                }
                return Err(preflight_error(error));
            }
            let selection_changed = if state.selection_signature() != previous_signature {
                self.apply_selection_transition(&control, &state, previous_signature)?;
                true
            } else {
                false
            };
            // Preserve the historical Off command's invalidation semantics,
            // including the fail-closed exhaustion gate, even when Test was
            // already clear (for example while Solo remains selected).
            if !selection_changed && target == PreflightTarget::Off {
                self.advance_preflight_epoch(&control)?;
            }
        }
        // A callback/event can fault after the state transaction.  Recheck
        // before reporting success so a stale UI reply cannot claim admission.
        self.poll_terminal_fault()?;
        self.preflight_status_for_identity(expected, now_ms)
    }

    pub(crate) fn set_preflight_solo(
        &mut self,
        mode: PreflightSolo,
        now_ms: u64,
    ) -> Result<AsioPreflightStatus, String> {
        let identity = self.current_preflight_identity()?;
        self.set_preflight_solo_for_identity(identity, mode, now_ms)
    }

    pub(crate) fn set_preflight_solo_for_identity(
        &mut self,
        expected: AsioPreflightIdentity,
        mode: PreflightSolo,
        now_ms: u64,
    ) -> Result<AsioPreflightStatus, String> {
        let (control, actual) = self.active_preflight_control()?;
        ensure_preflight_identity(expected, actual)?;
        {
            let mut state = control
                .state()
                .lock()
                .map_err(|_| self.fail_closed_preflight_state_lock(&control))?;
            let mut previous_signature = control.selection_signature();
            let expired = state
                .clear_if_expired(expected, now_ms)
                .map_err(preflight_error)?;
            if expired {
                self.advance_preflight_epoch(&control)?;
                control.publish_selection_kind(&state);
                previous_signature = 0;
            }
            let result = state.set_solo_with_live_fence(
                mode,
                expected,
                now_ms,
                Some(control.timeline_audio_live_fence()),
                control.live_drain_active(),
            );
            if let Err(error) = result {
                if state.selection_signature() != previous_signature {
                    self.apply_selection_transition(&control, &state, previous_signature)?;
                }
                return Err(preflight_error(error));
            }
            if state.selection_signature() != previous_signature {
                self.apply_selection_transition(&control, &state, previous_signature)?;
            }
        }
        self.poll_terminal_fault()?;
        self.preflight_status_for_identity(expected, now_ms)
    }

    /// Update the live-playback gate for the exact active identity. This is a
    /// runtime-facing seam for the Timeline owner; stopping playback is also
    /// permitted during its ordinary teardown before the preflight controls
    /// are discarded.
    pub(crate) fn set_preflight_live_playback(
        &mut self,
        active: bool,
    ) -> Result<AsioPreflightStatus, String> {
        let identity = self.current_preflight_identity()?;
        self.set_preflight_live_playback_for_identity(identity, active)
    }

    pub(crate) fn set_preflight_live_playback_for_identity(
        &mut self,
        expected: AsioPreflightIdentity,
        active: bool,
    ) -> Result<AsioPreflightStatus, String> {
        let (control, actual) = self.active_preflight_control()?;
        ensure_preflight_identity(expected, actual)?;
        {
            let mut state = control
                .state()
                .lock()
                .map_err(|_| self.fail_closed_preflight_state_lock(&control))?;
            state
                .set_live_playback_active(active, expected)
                .map_err(preflight_error)?;
        }
        self.poll_terminal_fault()?;
        self.preflight_status_for_identity(expected, preflight_now_ms())
    }

    /// Admit Timeline live playback for the exact active session/transport.
    ///
    /// Starting live playback may take ownership from an ephemeral Test or
    /// Solo selection, but only the two typed blocker errors from
    /// `set_live_playback_active(true, ...)` authorize that cleanup.  The
    /// Test fence is advanced before Test is cleared, and every cleanup/retry
    /// stays under this one state lock so the worker cannot render a new Test
    /// in between. Identity, lifecycle, lock, and all other non-terminal errors
    /// are returned without selection mutation. A pre-latched terminal render
    /// fault takes the existing fail-safe transition instead: it clears every
    /// preflight selection, fences queued Test output, and returns the fault.
    pub(crate) fn admit_timeline_live_playback_for_identity(
        &mut self,
        expected: AsioPreflightIdentity,
    ) -> Result<AsioPreflightStatus, String> {
        let (control, actual) = self.active_preflight_control()?;
        ensure_preflight_identity(expected, actual)?;
        let status = {
            let mut state = control
                .state()
                .lock()
                .map_err(|_| self.fail_closed_preflight_state_lock(&control))?;
            match state.set_live_playback_active(true, expected) {
                Ok(()) => {}
                Err(PreflightStateError::LivePlaybackBlockedByTest(_)) => {
                    self.advance_preflight_epoch(&control)?;
                    state
                        .begin_test_with_mapper(
                            control.mapper(),
                            PreflightTarget::Off,
                            expected,
                            0,
                            1,
                        )
                        .map_err(preflight_error)?;
                    control.publish_selection_kind(&state);
                    state
                        .set_live_playback_active(true, expected)
                        .map_err(preflight_error)?;
                }
                Err(PreflightStateError::LivePlaybackBlockedBySolo(_)) => {
                    let previous_signature = control.selection_signature();
                    state
                        .set_solo(PreflightSolo::None, expected, preflight_now_ms())
                        .map_err(preflight_error)?;
                    self.apply_selection_transition(&control, &state, previous_signature)?;
                    state
                        .set_live_playback_active(true, expected)
                        .map_err(preflight_error)?;
                }
                Err(error) => return Err(preflight_error(error)),
            }
            AsioPreflightStatus {
                identity: expected,
                phase: state.phase(),
                live_playback_active: state.live_playback_active(),
                test: state
                    .test_target(expected, preflight_now_ms())
                    .map_err(preflight_error)?,
                solo: state.solo(),
            }
        };
        self.poll_terminal_fault()?;
        Ok(status)
    }

    fn preflight_status_for_identity(
        &mut self,
        expected: AsioPreflightIdentity,
        now_ms: u64,
    ) -> Result<AsioPreflightStatus, String> {
        let (control, actual) = self.active_preflight_control()?;
        ensure_preflight_identity(expected, actual)?;
        let mut state = control
            .state()
            .lock()
            .map_err(|_| self.fail_closed_preflight_state_lock(&control))?;
        let expired = state
            .clear_if_expired(expected, now_ms)
            .map_err(preflight_error)?;
        if expired {
            self.advance_preflight_epoch(&control)?;
            control.publish_selection_kind(&state);
        }
        let test = state
            .test_target(expected, now_ms)
            .map_err(preflight_error)?;
        Ok(AsioPreflightStatus {
            identity: expected,
            phase: state.phase(),
            live_playback_active: state.live_playback_active(),
            test,
            solo: state.solo(),
        })
    }

    fn active_preflight_control(
        &mut self,
    ) -> Result<(Arc<PreflightRenderControl>, AsioPreflightIdentity), String> {
        self.poll_terminal_fault()?;
        if !matches!(self.lifecycle, AsioOutputLifecycle::Active) {
            return Err(format!(
                "ASIO preflight requires Active lifecycle (got {:?})",
                self.lifecycle
            ));
        }
        let context = self
            .context
            .as_ref()
            .ok_or_else(|| "ASIO preflight has no render context".to_owned())?;
        let session_generation = self
            .session_generation
            .ok_or_else(|| "ASIO preflight has no session generation".to_owned())?;
        let identity =
            AsioPreflightIdentity::new(session_generation, context.transport().current())
                .map_err(preflight_error)?;
        let control = self
            .preflight
            .as_ref()
            .cloned()
            .ok_or_else(|| "ASIO preflight control is unavailable".to_owned())?;
        Ok((control, identity))
    }

    fn advance_preflight_epoch(&self, control: &PreflightRenderControl) -> Result<(), String> {
        control.advance_selection_epoch().map_err(|fault| {
            self.latch_preflight_epoch_fault(fault);
            format!("ASIO preflight callback fence exhausted: {}", fault.code())
        })
    }

    fn apply_selection_transition(
        &self,
        control: &PreflightRenderControl,
        state: &AsioPreflightState,
        previous_signature: u8,
    ) -> Result<(), String> {
        control
            .publish_selection_transition(state, previous_signature)
            .map(|_| ())
            .map_err(|fault| {
                self.latch_preflight_epoch_fault(fault);
                format!("ASIO preflight callback fence exhausted: {}", fault.code())
            })
    }

    fn preflight_epoch_error(&self, fault: RenderFault) -> String {
        self.latch_preflight_epoch_fault(fault);
        format!("ASIO preflight callback fence exhausted: {}", fault.code())
    }

    fn latch_preflight_epoch_fault(&self, fault: RenderFault) {
        if let Some(context) = self.context.as_ref() {
            context.latch_worker_fault(fault);
        }
    }

    fn fail_closed_preflight_state_lock(&self, control: &PreflightRenderControl) -> String {
        if let Some(context) = self.context.as_ref() {
            context.latch_worker_fault(RenderFault::InvalidBlock);
        }
        let fence_error = control
            .invalidate_selection_without_state()
            .err()
            .map(|fault| self.preflight_epoch_error(fault));
        match fence_error {
            Some(fence_error) => {
                format!("ASIO preflight state lock was poisoned; {fence_error}")
            }
            None => "ASIO preflight state lock was poisoned".to_owned(),
        }
    }

    /// Observe the render context's permanent fault latch and synchronize the
    /// application-owned lifecycle with it.  This only observes existing
    /// atomics; it never clears a fault or starts a replacement output path.
    pub(crate) fn poll_terminal_fault(&mut self) -> Result<(), String> {
        let Some(fault) = self
            .context
            .as_ref()
            .and_then(|context| context.terminal_fault())
        else {
            return Ok(());
        };
        let reason = render_error(fault);
        self.clear_preflight_for_fault();
        if !matches!(self.lifecycle, AsioOutputLifecycle::StoppedPendingClose(_)) {
            self.lifecycle = AsioOutputLifecycle::Fault(reason.clone());
        }
        // Do not leave a stale handle available to code that still uses the
        // legacy accessors after a worker or callback fault.  The context and
        // bridge session remain owned until explicit Stop/Close drains them.
        self.program_mixer.take();
        self.cue_mixer.take();
        Err(reason)
    }

    /// Capture the exact session/transport/bus identity before asynchronous
    /// decoder work. No mixer escapes this runtime owner.
    pub(crate) fn current_output_identity(
        &mut self,
        bus: TimelineAudioOutputBus,
    ) -> Result<TimelineOutputIdentity, String> {
        self.poll_terminal_fault()?;
        if !matches!(self.lifecycle, AsioOutputLifecycle::Active) {
            return Err(format!(
                "show-ASIO output source admission requires Active lifecycle (got {:?})",
                self.lifecycle
            ));
        }
        let transport_generation = self
            .context
            .as_ref()
            .ok_or_else(|| "show-ASIO output source admission has no render context".to_owned())?
            .transport()
            .current();
        let session_generation = self.session_generation.ok_or_else(|| {
            "show-ASIO output source admission has no session generation".to_owned()
        })?;
        let identity = TimelineOutputIdentity::new(session_generation, transport_generation, bus)
            .map_err(|error| error.to_string())?;
        // A callback/event can latch concurrently with the first observation.
        // Recheck before publishing the identity.
        self.poll_terminal_fault()?;
        Ok(identity)
    }

    /// Attach one decoded source to its exact private PROGRAM/CUE mixer.
    /// Callers retain only a Sink control handle; raw mixers never cross this
    /// process owner. A transport rotation permanently ends the wrapped source.
    pub(crate) fn attach_source<S>(
        &mut self,
        expected: TimelineOutputIdentity,
        source: S,
        start_paused: bool,
    ) -> Result<rodio::Sink, String>
    where
        S: rodio::Source + Send + 'static,
    {
        let actual = self.current_output_identity(expected.bus())?;
        if actual != expected {
            return Err(format!(
                "show-ASIO output source identity changed before attachment: expected {expected:?}, got {actual:?}"
            ));
        }
        let context = self
            .context
            .as_ref()
            .ok_or_else(|| "show-ASIO output source admission has no render context".to_owned())?;
        let selector =
            AsioTimelineOutputSelector::from_identity(expected, Arc::clone(context.transport()))
                .map_err(|error| error.to_string())?;
        let guarded = selector
            .source_for(expected, source)
            .map_err(|error| error.to_string())?;
        let mixer = match expected.bus() {
            TimelineAudioOutputBus::Program => self.program_mixer.as_ref(),
            TimelineAudioOutputBus::Cue => self.cue_mixer.as_ref(),
        }
        .ok_or_else(|| {
            format!(
                "show-ASIO output source admission has no {:?} mixer",
                expected.bus()
            )
        })?;
        let sink = rodio::Sink::connect_new(mixer);
        if start_paused {
            sink.pause();
        }
        sink.append(guarded);

        match self.current_output_identity(expected.bus()) {
            Ok(current) if current == expected => Ok(sink),
            Ok(current) => {
                sink.stop();
                Err(format!(
                    "show-ASIO output source identity changed during attachment: expected {expected:?}, got {current:?}"
                ))
            }
            Err(error) => {
                sink.stop();
                Err(error)
            }
        }
    }

    /// The bridge owns this telemetry snapshot.  The app parses it before it
    /// ever crosses a command boundary, so diagnostics cannot become an
    /// accidental alternate configuration channel.
    pub(crate) fn telemetry_text(&self) -> Result<String, String> {
        self.session
            .as_ref()
            .ok_or_else(|| "show-ASIO output has no active session".to_owned())?
            .telemetry_text()
            .map_err(bridge_error)
    }

    pub(crate) fn start(
        &mut self,
        bridge: &BridgeV3Module,
        profile: MachineAsioOutputProfile,
        first_output_frame: u64,
        session_generation: u64,
    ) -> Result<(), String> {
        if !matches!(
            self.lifecycle,
            AsioOutputLifecycle::Locked | AsioOutputLifecycle::Closed
        ) {
            return Err(
                "show-ASIO output requires an explicit Stop/Close before another Start".to_owned(),
            );
        }
        self.lifecycle = AsioOutputLifecycle::Starting;
        let result = self.start_inner(bridge, profile, first_output_frame, session_generation);
        if let Err(error) = &result {
            // `start_inner` owns a successful bridge Start before attempting
            // worker creation. Its cleanup path may therefore leave a live
            // session in StoppedPendingClose, or may have completed Close and
            // reached Closed. Preserve either authoritative phase instead of
            // overwriting it with a generic startup fault.
            if !matches!(
                self.lifecycle,
                AsioOutputLifecycle::Closed | AsioOutputLifecycle::StoppedPendingClose(_)
            ) {
                self.lifecycle = AsioOutputLifecycle::Fault(error.clone());
            }
        }
        result
    }

    fn start_inner(
        &mut self,
        bridge: &BridgeV3Module,
        profile: MachineAsioOutputProfile,
        first: u64,
        session_generation: u64,
    ) -> Result<(), String> {
        let transport = Arc::new(TransportGeneration::new(1).map_err(render_error)?);
        let context = AsioRenderContext::new_with_live_fence(
            session_generation,
            &profile,
            4,
            transport,
            self.timeline_audio_live_fence.clone(),
        )
        .map_err(profile_error)?;
        let mut mixers = ProgramCueMixers::new(Arc::clone(&context), profile.clone(), first)
            .map_err(profile_error)?;
        // Start only after at least two complete device-width blocks exist.
        mixers.worker_mut().prefill(2).map_err(render_error)?;
        let request = OutputStartRequestV3::from_profile(&profile, first, session_generation)
            .map_err(bridge_error)?;
        let raw_context = Arc::as_ptr(&context).cast_mut().cast::<c_void>();
        let session = match bridge.start_output(
            &request,
            asio_output_callback,
            asio_event_callback,
            raw_context,
        ) {
            Ok(session) => session,
            Err(BridgeV3StartError::Bridge(error)) => return Err(bridge_error(error)),
            Err(BridgeV3StartError::Cleanup {
                start,
                cleanup,
                session,
            }) => {
                let session = *session;
                let error = format!(
                    "{}; bridge session cleanup failed and remains retryable: {}",
                    bridge_error(start),
                    bridge_error(cleanup)
                );
                self.install_failed_start_cleanup_session(
                    session,
                    Arc::clone(&context),
                    session_generation,
                    error.clone(),
                );
                return Err(error);
            }
        };
        self.install_started_session(
            session,
            context,
            session_generation,
            mixers,
            |worker, context, stop| {
                thread::Builder::new()
                    .name("syndocal-asio-program-cue-render".to_owned())
                    .spawn(move || {
                        render_worker_loop(worker, context, stop);
                    })
                    .map_err(|error| format!("ASIO render worker could not start: {error}"))
            },
        )
    }

    fn install_failed_start_cleanup_session(
        &mut self,
        session: BridgeV3Session,
        context: Arc<AsioRenderContext>,
        session_generation: u64,
        error: String,
    ) {
        if session.is_closed() {
            // Close consumed or terminally quarantined the native handle even
            // though its diagnostic was malformed. There is nothing left to
            // retry and the callback has been drained, so keep the runtime
            // restartable instead of stranding it in Fault without a session.
            self.clear_closed_resources();
            self.lifecycle = AsioOutputLifecycle::Closed;
            return;
        }
        let pending_close = matches!(session.phase(), BridgeV3SessionPhase::StoppedPendingClose);
        self.session = Some(session);
        self.context = Some(context);
        self.session_generation = Some(session_generation);
        self.lifecycle = if pending_close {
            AsioOutputLifecycle::StoppedPendingClose(error)
        } else {
            AsioOutputLifecycle::Fault(error)
        };
    }

    /// Transfer every resource needed for callback-safe teardown into the
    /// runtime immediately after bridge Start succeeds. This ordering is
    /// critical: a worker spawn error must still have an owned session and
    /// context available for Stop/Close, including a retryable Close failure.
    fn install_started_session<F>(
        &mut self,
        session: BridgeV3Session,
        context: Arc<AsioRenderContext>,
        session_generation: u64,
        mixers: ProgramCueMixers,
        spawn_worker: F,
    ) -> Result<(), String>
    where
        F: FnOnce(
            ProgramCueRenderWorker,
            Arc<AsioRenderContext>,
            Arc<AtomicBool>,
        ) -> Result<JoinHandle<()>, String>,
    {
        let preflight = mixers.preflight_control();
        let (program, cue, worker) = mixers.into_parts();
        let stop = Arc::new(AtomicBool::new(false));

        // Publish ownership before invoking the fallible worker spawn. The
        // callback context is now kept alive by `self.context` for all cleanup
        // outcomes, and the session cannot be dropped on a spawn error.
        self.session = Some(session);
        self.context = Some(Arc::clone(&context));
        self.session_generation = Some(session_generation);
        self.preflight = Some(preflight);
        self.program_mixer = Some(program);
        self.cue_mixer = Some(cue);
        self.worker_stop = Some(Arc::clone(&stop));

        let handle = match spawn_worker(worker, context, stop) {
            Ok(handle) => handle,
            Err(spawn_error) => {
                let cleanup = self.stop_and_close();
                return Err(match cleanup {
                    Ok(()) => format!(
                        "{spawn_error}; bridge session Stop/Close completed during worker cleanup"
                    ),
                    Err(cleanup_error) => format!(
                        "{spawn_error}; bridge session cleanup failed and remains retryable: {cleanup_error}"
                    ),
                });
            }
        };
        self.worker = Some(handle);
        self.lifecycle = AsioOutputLifecycle::Active;
        Ok(())
    }

    /// Ordinary transport changes are a generation barrier, not a device
    /// restart. Old queued blocks are retired before the caller publishes a
    /// seek/pause/loop/bus/project revision.
    pub(crate) fn rotate_transport_generation(&self) -> Result<u64, String> {
        if !matches!(self.lifecycle, AsioOutputLifecycle::Active) {
            return Err(format!(
                "show-ASIO transport rotation requires Active lifecycle (got {:?})",
                self.lifecycle
            ));
        }
        let context = self
            .context
            .as_ref()
            .ok_or_else(|| "show-ASIO output is not active".to_owned())?;
        if let Some(fault) = context.terminal_fault() {
            return Err(render_error(fault));
        }
        let session_generation = self
            .session_generation
            .ok_or_else(|| "show-ASIO output has no session generation".to_owned())?;
        let current_identity =
            AsioPreflightIdentity::new(session_generation, context.transport().current())
                .map_err(preflight_error)?;
        let next_transport_generation = current_identity
            .transport_generation()
            .checked_add(1)
            .ok_or_else(|| "ASIO preflight transport generation is exhausted".to_owned())?;
        let next_identity = AsioPreflightIdentity::new(
            current_identity.session_generation(),
            next_transport_generation,
        )
        .map_err(preflight_error)?;
        let preflight = self.preflight.as_ref().cloned();
        let mut preflight_state = match preflight.as_ref() {
            Some(control) => match control.state().lock() {
                Ok(state) => Some(state),
                Err(_) => return Err(self.fail_closed_preflight_state_lock(control)),
            },
            None => None,
        };
        context.retire_queued_blocks();
        if let Some(state) = preflight_state.as_mut() {
            state
                .clear_for_transport_rotation(current_identity, next_identity)
                .map_err(preflight_error)?;
            if let Some(control) = preflight.as_ref() {
                self.advance_preflight_epoch(control)?;
                control.publish_selection_kind(state);
            }
        }
        context.transport().rotate().map_err(render_error)
    }

    /// Explicit stop/close only. Successful bridge Stop drains callback
    /// readers, so the context can then be dropped safely.
    pub(crate) fn stop_and_close(&mut self) -> Result<(), String> {
        // Fence the callback context and queued Test/Solo blocks at request
        // entry. The worker may be joined, and the bridge may still execute
        // a final callback, so neither operation can precede this barrier.
        let preflight_error = self.clear_preflight_for_stop();
        if let Some(stop) = self.worker_stop.take() {
            stop.store(true, Ordering::Release);
        }
        let worker_error = self.worker.take().and_then(|worker| {
            worker
                .join()
                .err()
                .map(|_| "ASIO render worker panicked during Stop".to_owned())
        });
        let Some(mut session) = self.session.take() else {
            return Err(match (worker_error, preflight_error) {
                (Some(worker_error), Some(preflight_error)) => {
                    format!("{worker_error}; {preflight_error}")
                }
                (Some(worker_error), None) => worker_error,
                (None, Some(preflight_error)) => preflight_error,
                (None, None) => "show-ASIO output has no active session".to_owned(),
            });
        };

        // Keep the callback context alive through this exact Stop/Close
        // attempt, even when joining the application worker reported a panic.
        let bridge_result = session.stop_and_close().map_err(bridge_error);
        match bridge_result {
            Ok(()) => {
                self.clear_closed_resources();
                self.lifecycle = AsioOutputLifecycle::Closed;
                match (preflight_error, worker_error) {
                    (Some(preflight_error), Some(worker_error)) => {
                        Err(format!("{preflight_error}; {worker_error}"))
                    }
                    (Some(preflight_error), None) => Err(preflight_error),
                    (None, Some(worker_error)) => Err(worker_error),
                    (None, None) => Ok(()),
                }
            }
            Err(bridge_error) => {
                let error = match (worker_error, preflight_error) {
                    (Some(worker_error), Some(preflight_error)) => {
                        format!("{worker_error}; {preflight_error}; {bridge_error}")
                    }
                    (Some(worker_error), None) => format!("{worker_error}; {bridge_error}"),
                    (None, Some(preflight_error)) => {
                        format!("{preflight_error}; {bridge_error}")
                    }
                    (None, None) => bridge_error,
                };
                if session.is_closed() {
                    // A bridge may consume its native handle while returning
                    // a malformed diagnostic. Do not retain a closed pointer
                    // and turn a subsequent retry into an AlreadyClosed loop.
                    self.clear_closed_resources();
                    self.lifecycle = AsioOutputLifecycle::Closed;
                    return Err(error);
                }
                let pending_close =
                    matches!(session.phase(), BridgeV3SessionPhase::StoppedPendingClose);
                self.session = Some(session);
                // Mixers are no longer an admitted handoff once Stop has been
                // requested. Keep only the callback context and bridge
                // session needed for a safe Close retry.
                self.program_mixer.take();
                self.cue_mixer.take();
                self.lifecycle = if pending_close {
                    AsioOutputLifecycle::StoppedPendingClose(error.clone())
                } else {
                    AsioOutputLifecycle::Fault(error.clone())
                };
                Err(error)
            }
        }
    }

    fn clear_closed_resources(&mut self) {
        self.context.take();
        self.session_generation.take();
        self.preflight.take();
        self.program_mixer.take();
        self.cue_mixer.take();
        self.worker_stop.take();
        self.worker.take();
    }

    fn clear_preflight_for_stop(&mut self) -> Option<String> {
        // Make callback teardown terminal before inspecting any fallible
        // application state. A poisoned lock, missing identity, or worker
        // join must never leave a ready block admissible during Stop.
        if let Some(context) = self.context.as_ref() {
            context.latch_worker_fault(RenderFault::InvalidBlock);
        }
        let Some(control) = self.preflight.as_ref().cloned() else {
            return None;
        };
        let Some(context) = self.context.as_ref() else {
            return Some("ASIO preflight has no render context during Stop".to_owned());
        };
        let Some(session_generation) = self.session_generation else {
            return Some("ASIO preflight has no session generation during Stop".to_owned());
        };
        let identity =
            match AsioPreflightIdentity::new(session_generation, context.transport().current()) {
                Ok(identity) => identity,
                Err(error) => return Some(preflight_error(error)),
            };
        let result = match control.state().lock() {
            Ok(mut state) => {
                let result = state.clear_for_stop(identity);
                if result.is_ok() {
                    let epoch_result = control.advance_selection_epoch();
                    control.publish_selection_kind(&state);
                    if let Err(fault) = epoch_result {
                        return Some(self.preflight_epoch_error(fault));
                    }
                }
                result.err().map(preflight_error)
            }
            Err(_) => Some(self.fail_closed_preflight_state_lock(&control)),
        };
        result
    }

    fn clear_preflight_for_fault(&mut self) {
        let Some(control) = self.preflight.as_ref().cloned() else {
            return;
        };
        let Some(context) = self.context.as_ref() else {
            return;
        };
        if let Some(fault) = context.terminal_fault() {
            // The caller normally latched this first; retaining the original
            // fault here makes the terminal-before-state ordering explicit
            // for every fault cleanup entry.
            context.latch_worker_fault(fault);
        }
        let Some(session_generation) = self.session_generation else {
            return;
        };
        let Ok(identity) =
            AsioPreflightIdentity::new(session_generation, context.transport().current())
        else {
            return;
        };
        match control.state().lock() {
            Ok(mut state) => {
                if state.clear_for_fault(identity).is_err() {
                    let _ = control.invalidate_selection_without_state();
                    return;
                }
                // Keep the complete transition under one state guard. A
                // second lock after epoch publication left a poison or
                // interleaving window where stale kind/signature values could
                // remain callback-visible.
                let epoch_result = control.advance_selection_epoch();
                control.publish_selection_kind(&state);
                if let Err(fault) = epoch_result {
                    self.latch_preflight_epoch_fault(fault);
                }
            }
            Err(_) => {
                let _ = self.fail_closed_preflight_state_lock(&control);
            }
        };
    }
}

impl Drop for AsioOutputRuntime {
    fn drop(&mut self) {
        if self.session.is_none() {
            return;
        }

        // App teardown must never release the raw callback context ahead of a
        // terminal bridge Close. Make one synchronous best-effort teardown;
        // if the driver still refuses Stop/Close, deliberately retain the
        // session, DLL table and context for process lifetime. A bounded leak
        // on terminal teardown is safer than a callback use-after-free.
        let _ = self.stop_and_close();
        if self.session.is_some() {
            if let Some(session) = self.session.take() {
                std::mem::forget(session);
            }
            if let Some(context) = self.context.take() {
                std::mem::forget(context);
            }
            self.session_generation.take();
            self.program_mixer.take();
            self.cue_mixer.take();
            self.worker_stop.take();
            self.worker.take();
        }
    }
}

#[cfg(test)]
impl AsioOutputRuntime {
    /// Build an active in-memory owner for cross-module timeline/output tests.
    /// No bridge handle or worker thread is created; the returned mixer sources
    /// are the observers for the two private logical buses.
    pub(crate) fn active_in_memory(
        session_generation: u64,
        transport_generation: u64,
    ) -> Result<
        (
            Self,
            Arc<AsioRenderContext>,
            rodio::mixer::MixerSource,
            rodio::mixer::MixerSource,
        ),
        String,
    > {
        Self::active_in_memory_with_live_fence(
            session_generation,
            transport_generation,
            TimelineAudioLiveFence::default(),
        )
    }

    /// Build an active in-memory owner bound to the supplied engine liveness
    /// authority. This keeps command-level tests on the same shared fence as
    /// production without starting a native bridge session.
    pub(crate) fn active_in_memory_with_live_fence(
        session_generation: u64,
        transport_generation: u64,
        timeline_audio_live_fence: TimelineAudioLiveFence,
    ) -> Result<
        (
            Self,
            Arc<AsioRenderContext>,
            rodio::mixer::MixerSource,
            rodio::mixer::MixerSource,
        ),
        String,
    > {
        let profile = MachineAsioOutputProfile::for_test(4, 4, 1, 2, 3);
        let transport =
            Arc::new(TransportGeneration::new(transport_generation).map_err(render_error)?);
        let context = AsioRenderContext::new_with_live_fence(
            session_generation,
            &profile,
            4,
            transport,
            timeline_audio_live_fence.clone(),
        )
        .map_err(profile_error)?;
        let identity = AsioPreflightIdentity::new(session_generation, transport_generation)
            .map_err(preflight_error)?;
        let preflight = PreflightRenderControl::new(
            &profile,
            identity,
            Arc::clone(context.preflight_selection_epoch()),
            Arc::clone(context.preflight_selection_epoch_exhausted()),
            Arc::clone(context.preflight_selection_kind()),
            Arc::clone(context.preflight_selection_signature()),
            timeline_audio_live_fence.clone(),
            Arc::clone(context.live_drain_remaining()),
            Arc::clone(context.live_drain_in_progress()),
        )
        .map_err(profile_error)?;
        let (program_mixer, program_source) = rodio::mixer::mixer(2, 48_000);
        let (cue_mixer, cue_source) = rodio::mixer::mixer(2, 48_000);
        let runtime = Self {
            lifecycle: AsioOutputLifecycle::Active,
            timeline_audio_live_fence,
            session: None,
            context: Some(Arc::clone(&context)),
            session_generation: Some(session_generation),
            preflight: Some(preflight),
            program_mixer: Some(program_mixer),
            cue_mixer: Some(cue_mixer),
            worker_stop: None,
            worker: None,
        };
        Ok((runtime, context, program_source, cue_source))
    }

    fn active_in_memory_with_worker(
        session_generation: u64,
        transport_generation: u64,
    ) -> Result<(Self, Arc<AsioRenderContext>, ProgramCueRenderWorker), String> {
        let profile = MachineAsioOutputProfile::for_test(4, 4, 1, 2, 3);
        let transport =
            Arc::new(TransportGeneration::new(transport_generation).map_err(render_error)?);
        let context = AsioRenderContext::new(session_generation, &profile, 4, transport)
            .map_err(profile_error)?;
        let mixers =
            ProgramCueMixers::new(Arc::clone(&context), profile, 0).map_err(profile_error)?;
        let preflight = mixers.preflight_control();
        let (program_mixer, cue_mixer, worker) = mixers.into_parts();
        let runtime = Self {
            lifecycle: AsioOutputLifecycle::Active,
            timeline_audio_live_fence: context.timeline_audio_live_fence().clone(),
            session: None,
            context: Some(Arc::clone(&context)),
            session_generation: Some(session_generation),
            preflight: Some(preflight),
            program_mixer: Some(program_mixer),
            cue_mixer: Some(cue_mixer),
            worker_stop: None,
            worker: None,
        };
        Ok((runtime, context, worker))
    }
}

fn render_worker_loop(
    mut worker: ProgramCueRenderWorker,
    context: Arc<AsioRenderContext>,
    stop: Arc<AtomicBool>,
) {
    while !stop.load(Ordering::Acquire) {
        match worker.render_one_complete_block() {
            Ok(()) => {}
            Err(RenderFault::QueueUnderflow) => thread::sleep(Duration::from_millis(1)),
            Err(fault) => {
                context.latch_worker_fault(fault);
                break;
            }
        }
    }
}

unsafe extern "C" fn asio_output_callback(
    context: *mut c_void,
    output: *mut f32,
    channels: u32,
    frames: u32,
    first: u64,
    session: u64,
    render: u64,
) -> u32 {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if context.is_null() {
            return crate::asio_program_cue_render::CALLBACK_PANIC;
        }
        let context = unsafe { &*(context.cast::<AsioRenderContext>()) };
        unsafe { context.callback_copy(output, channels, frames, first, session, render) }
    }));
    match result {
        Ok(code) => code,
        Err(_) => crate::asio_program_cue_render::CALLBACK_PANIC,
    }
}

unsafe extern "C" fn asio_event_callback(
    context: *mut c_void,
    severity: u32,
    _kind: u32,
    _message: *const u8,
    _message_len: usize,
) {
    if severity < 2 || context.is_null() {
        return;
    }
    // No string decoding, lock, allocation or UI work is permitted here.
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        (&*(context.cast::<AsioRenderContext>())).latch_worker_fault(RenderFault::InvalidBlock)
    }));
}

fn profile_error(error: OutputProfileLock) -> String {
    error.to_string()
}
fn bridge_error(error: BridgeV3Fault) -> String {
    error.to_string()
}
fn render_error(error: RenderFault) -> String {
    error.code().to_owned()
}

fn preflight_error(error: PreflightStateError) -> String {
    error.to_string()
}

fn ensure_preflight_identity(
    expected: AsioPreflightIdentity,
    actual: AsioPreflightIdentity,
) -> Result<(), String> {
    if expected == actual {
        Ok(())
    } else {
        Err(preflight_error(PreflightStateError::IdentityMismatch {
            expected,
            actual,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active_runtime(
        session_generation: u64,
        transport_generation: u64,
    ) -> (
        AsioOutputRuntime,
        Arc<AsioRenderContext>,
        rodio::mixer::MixerSource,
        rodio::mixer::MixerSource,
    ) {
        AsioOutputRuntime::active_in_memory(session_generation, transport_generation).unwrap()
    }

    fn poisoned_stop_fences_queued_selection(test_selection: bool) {
        let (mut runtime, context, mut worker) =
            AsioOutputRuntime::active_in_memory_with_worker(95, 31).unwrap();
        let identity = runtime.current_preflight_identity().unwrap();
        if test_selection {
            runtime
                .set_preflight_test_for_identity(identity, PreflightTarget::ProgramLeft, 0, 30_000)
                .unwrap();
        } else {
            runtime
                .set_preflight_solo_for_identity(identity, PreflightSolo::ProgramOnly, 0)
                .unwrap();
        }
        worker.render_one_complete_block_at(0).unwrap();

        let control = runtime.preflight.as_ref().unwrap().clone();
        let poison = thread::spawn(move || {
            let _guard = control.state().lock().unwrap();
            panic!("injected poisoned preflight state for Stop fencing");
        });
        assert!(poison.join().is_err());

        let (session, bridge_control) = crate::asio_bridge_v3::test_session_with_failures(0, 0);
        runtime.session = Some(session);
        let epoch_before = context
            .preflight_selection_epoch()
            .load(std::sync::atomic::Ordering::Acquire);
        let error = runtime.stop_and_close().unwrap_err();
        assert!(error.contains("preflight state lock was poisoned"));
        assert_eq!(runtime.lifecycle(), &AsioOutputLifecycle::Closed);
        assert!(runtime.session.is_none());
        assert!(runtime.context.is_none());
        assert!(
            context
                .preflight_selection_epoch()
                .load(std::sync::atomic::Ordering::Acquire)
                > epoch_before
        );
        assert_eq!(
            context.preflight_selection_kind().load(Ordering::Acquire),
            0
        );
        assert_eq!(
            context
                .preflight_selection_signature()
                .load(Ordering::Acquire),
            0
        );
        assert_eq!(context.terminal_fault(), Some(RenderFault::InvalidBlock));

        let mut destination = [99.0_f32; 16];
        let result = unsafe { context.callback_copy(destination.as_mut_ptr(), 4, 4, 0, 95, 95) };
        assert_eq!(result, crate::asio_program_cue_render::CALLBACK_TERMINAL);
        assert_eq!(destination, [0.0; 16]);
        assert_eq!(
            bridge_control
                .stop_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1
        );
        assert_eq!(
            bridge_control
                .close_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1
        );
    }

    #[test]
    fn normal_state_is_not_reentered_implicitly_after_fault_or_close() {
        let mut runtime = AsioOutputRuntime::default();
        assert_eq!(runtime.lifecycle(), &AsioOutputLifecycle::Locked);
        assert!(runtime
            .current_output_identity(TimelineAudioOutputBus::Program)
            .is_err());
    }

    #[test]
    fn poisoned_stop_fences_queued_test_before_bridge_close() {
        poisoned_stop_fences_queued_selection(true);
    }

    #[test]
    fn poisoned_stop_fences_queued_solo_before_bridge_close() {
        poisoned_stop_fences_queued_selection(false);
    }

    fn stop_entry_fences_callback_during_join(test_selection: bool) {
        let (mut runtime, context, mut render_worker) =
            AsioOutputRuntime::active_in_memory_with_worker(96, 32).unwrap();
        let identity = runtime.current_preflight_identity().unwrap();
        if test_selection {
            runtime
                .set_preflight_test_for_identity(identity, PreflightTarget::ProgramLeft, 0, 30_000)
                .unwrap();
        } else {
            runtime
                .set_preflight_solo_for_identity(identity, PreflightSolo::ProgramOnly, 0)
                .unwrap();
        }
        render_worker.render_one_complete_block_at(0).unwrap();
        drop(render_worker);

        let stop = Arc::new(AtomicBool::new(false));
        let callback_started = Arc::new(AtomicBool::new(false));
        let callback_result = Arc::new(std::sync::atomic::AtomicU32::new(u32::MAX));
        let callback_silent = Arc::new(AtomicBool::new(false));
        let callback_stop = Arc::clone(&stop);
        let callback_context = Arc::clone(&context);
        let callback_started_for_thread = Arc::clone(&callback_started);
        let callback_result_for_thread = Arc::clone(&callback_result);
        let callback_silent_for_thread = Arc::clone(&callback_silent);
        let join_worker = thread::spawn(move || {
            while !callback_stop.load(Ordering::Acquire) {
                thread::yield_now();
            }
            callback_started_for_thread.store(true, Ordering::Release);
            let mut destination = [99.0_f32; 16];
            let result = unsafe {
                callback_context.callback_copy(destination.as_mut_ptr(), 4, 4, 0, 96, 96)
            };
            callback_result_for_thread.store(result, Ordering::Release);
            callback_silent_for_thread.store(
                destination.iter().all(|sample| *sample == 0.0),
                Ordering::Release,
            );
        });
        runtime.worker_stop = Some(stop);
        runtime.worker = Some(join_worker);
        let (session, bridge_control) = crate::asio_bridge_v3::test_session_with_failures(0, 0);
        runtime.session = Some(session);

        // stop_and_close must publish the terminal callback fence before it
        // releases this join barrier. The callback therefore cannot consume
        // the queued Test/Solo block, even though it runs during join.
        runtime.stop_and_close().unwrap();
        assert!(callback_started.load(Ordering::Acquire));
        assert_eq!(
            callback_result.load(Ordering::Acquire),
            crate::asio_program_cue_render::CALLBACK_TERMINAL
        );
        assert!(callback_silent.load(Ordering::Acquire));
        assert_eq!(
            bridge_control
                .stop_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1
        );
        assert_eq!(
            bridge_control
                .close_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1
        );
    }

    #[test]
    fn stop_entry_fences_queued_test_during_worker_join() {
        stop_entry_fences_callback_during_join(true);
    }

    #[test]
    fn stop_entry_fences_queued_solo_during_worker_join() {
        stop_entry_fences_callback_during_join(false);
    }

    fn poisoned_fault_fences_queued_selection(test_selection: bool) {
        let (mut runtime, context, mut render_worker) =
            AsioOutputRuntime::active_in_memory_with_worker(97, 33).unwrap();
        let identity = runtime.current_preflight_identity().unwrap();
        if test_selection {
            runtime
                .set_preflight_test_for_identity(identity, PreflightTarget::ProgramLeft, 0, 30_000)
                .unwrap();
        } else {
            runtime
                .set_preflight_solo_for_identity(identity, PreflightSolo::ProgramOnly, 0)
                .unwrap();
        }
        render_worker.render_one_complete_block_at(0).unwrap();

        let control = runtime.preflight.as_ref().unwrap().clone();
        let poisoned = std::panic::catch_unwind(move || {
            let _guard = control.state().lock().unwrap();
            panic!("injected poisoned preflight state for Fault fencing");
        });
        assert!(poisoned.is_err());
        context.latch_worker_fault(RenderFault::CallbackPanic);
        assert_eq!(
            runtime.poll_terminal_fault().unwrap_err(),
            "output_callback_panic"
        );
        assert_eq!(
            context.preflight_selection_kind().load(Ordering::Acquire),
            0
        );
        assert_eq!(
            context
                .preflight_selection_signature()
                .load(Ordering::Acquire),
            0
        );

        let mut destination = [99.0_f32; 16];
        assert_eq!(
            unsafe { context.callback_copy(destination.as_mut_ptr(), 4, 4, 0, 97, 97) },
            crate::asio_program_cue_render::CALLBACK_TERMINAL
        );
        assert_eq!(destination, [0.0; 16]);
    }

    fn poisoned_rotation_fences_queued_selection(test_selection: bool) {
        let (mut runtime, context, mut render_worker) =
            AsioOutputRuntime::active_in_memory_with_worker(98, 34).unwrap();
        let identity = runtime.current_preflight_identity().unwrap();
        if test_selection {
            runtime
                .set_preflight_test_for_identity(identity, PreflightTarget::ProgramLeft, 0, 30_000)
                .unwrap();
        } else {
            runtime
                .set_preflight_solo_for_identity(identity, PreflightSolo::ProgramOnly, 0)
                .unwrap();
        }
        render_worker.render_one_complete_block_at(0).unwrap();

        let control = runtime.preflight.as_ref().unwrap().clone();
        let poisoned = std::panic::catch_unwind(move || {
            let _guard = control.state().lock().unwrap();
            panic!("injected poisoned preflight state for rotation fencing");
        });
        assert!(poisoned.is_err());
        assert!(runtime.rotate_transport_generation().is_err());
        assert_eq!(
            context.preflight_selection_kind().load(Ordering::Acquire),
            0
        );
        assert_eq!(
            context
                .preflight_selection_signature()
                .load(Ordering::Acquire),
            0
        );

        let mut destination = [99.0_f32; 16];
        assert_eq!(
            unsafe { context.callback_copy(destination.as_mut_ptr(), 4, 4, 0, 98, 98) },
            crate::asio_program_cue_render::CALLBACK_TERMINAL
        );
        assert_eq!(destination, [0.0; 16]);
    }

    #[test]
    fn poisoned_fault_fences_queued_test_before_callback() {
        poisoned_fault_fences_queued_selection(true);
    }

    #[test]
    fn poisoned_fault_fences_queued_solo_before_callback() {
        poisoned_fault_fences_queued_selection(false);
    }

    #[test]
    fn poisoned_rotation_fences_queued_test_before_callback() {
        poisoned_rotation_fences_queued_selection(true);
    }

    #[test]
    fn poisoned_rotation_fences_queued_solo_before_callback() {
        poisoned_rotation_fences_queued_selection(false);
    }

    #[test]
    fn live_fence_binding_is_pre_start_only_and_preserves_shared_word() {
        let engine = engine::EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let fence = engine.timeline_audio_live_fence();
        let mut runtime = AsioOutputRuntime::default();
        runtime.bind_timeline_audio_live_fence(&fence).unwrap();
        for _ in 0..8 {
            let authority = engine.timeline_transport_authority();
            match engine.set_timeline_playing_published(
                authority.epoch,
                authority.generation,
                true,
                std::time::Instant::now() + std::time::Duration::from_secs(1),
            ) {
                Ok(_) => break,
                Err(error) if error.contains("stale") => std::thread::yield_now(),
                Err(error) => panic!("Timeline live test mutation failed: {error}"),
            }
        }
        assert!(fence.active());
        assert_eq!(runtime.timeline_audio_live_fence.word(), fence.word());

        for lifecycle in [
            AsioOutputLifecycle::Starting,
            AsioOutputLifecycle::Active,
            AsioOutputLifecycle::StoppedPendingClose("retry".to_owned()),
            AsioOutputLifecycle::Fault("latched".to_owned()),
        ] {
            runtime.lifecycle = lifecycle;
            assert!(runtime.bind_timeline_audio_live_fence(&fence).is_err());
        }

        runtime.lifecycle = AsioOutputLifecycle::Locked;
        runtime.session_generation = Some(1);
        assert!(runtime.bind_timeline_audio_live_fence(&fence).is_err());
        runtime.session_generation = None;
        runtime.lifecycle = AsioOutputLifecycle::Closed;
        runtime.bind_timeline_audio_live_fence(&fence).unwrap();
    }

    #[test]
    fn active_runtime_attaches_sources_to_exact_private_bus_and_generation() {
        let (mut runtime, _context, mut program_source, mut cue_source) = active_runtime(41, 7);
        let program_identity = runtime
            .current_output_identity(TimelineAudioOutputBus::Program)
            .unwrap();
        assert_eq!(program_identity.session_generation(), 41);
        assert_eq!(program_identity.transport_generation(), 7);
        let program_sink = runtime
            .attach_source(
                program_identity,
                rodio::buffer::SamplesBuffer::new(2, 48_000, vec![0.25_f32, 0.5_f32]),
                false,
            )
            .unwrap();
        assert_eq!(program_source.next(), Some(0.25));
        assert_eq!(cue_source.next(), None);
        let cue_identity = runtime
            .current_output_identity(TimelineAudioOutputBus::Cue)
            .unwrap();
        let cue_sink = runtime
            .attach_source(
                cue_identity,
                rodio::buffer::SamplesBuffer::new(2, 48_000, vec![0.75_f32, 1.0_f32]),
                false,
            )
            .unwrap();
        // The preceding empty read advanced the interleaved source by one
        // channel. Rodio holds a newly-added stereo source until the next
        // channel boundary instead of swapping L/R.
        assert_eq!(cue_source.next(), None);
        assert_eq!(cue_source.next(), Some(0.75));
        drop((program_sink, cue_sink));
    }

    #[test]
    fn source_admission_rejects_every_non_active_lifecycle_and_missing_context() {
        for lifecycle in [
            AsioOutputLifecycle::Locked,
            AsioOutputLifecycle::Starting,
            AsioOutputLifecycle::StoppedPendingClose("retry close".to_owned()),
            AsioOutputLifecycle::Fault("latched".to_owned()),
            AsioOutputLifecycle::Closed,
        ] {
            let mut runtime = AsioOutputRuntime::default();
            runtime.lifecycle = lifecycle;
            assert!(runtime
                .current_output_identity(TimelineAudioOutputBus::Program)
                .is_err());
        }

        let mut runtime = AsioOutputRuntime::default();
        runtime.lifecycle = AsioOutputLifecycle::Active;
        let error = runtime
            .current_output_identity(TimelineAudioOutputBus::Program)
            .unwrap_err();
        assert!(error.contains("no render context"));
    }

    #[test]
    fn output_identity_reflects_transport_rotation_without_mutating_prior_identity() {
        let (mut runtime, _context, _program_source, _cue_source) = active_runtime(52, 10);
        let before = runtime
            .current_output_identity(TimelineAudioOutputBus::Program)
            .unwrap();
        assert_eq!(runtime.rotate_transport_generation().unwrap(), 11);
        let after = runtime
            .current_output_identity(TimelineAudioOutputBus::Program)
            .unwrap();
        assert_eq!(before.session_generation(), 52);
        assert_eq!(before.transport_generation(), 10);
        assert_eq!(after.session_generation(), 52);
        assert_eq!(after.transport_generation(), 11);
        assert!(runtime
            .attach_source(
                before,
                rodio::buffer::SamplesBuffer::new(1, 48_000, vec![0.25_f32]),
                false,
            )
            .is_err());
    }

    #[test]
    fn transport_rotation_requires_active_fault_free_runtime() {
        let (mut runtime, context, _program_source, _cue_source) = active_runtime(54, 13);
        runtime.lifecycle = AsioOutputLifecycle::StoppedPendingClose("pending".to_owned());
        let error = runtime.rotate_transport_generation().unwrap_err();
        assert!(error.contains("Active lifecycle"));
        assert_eq!(context.transport().current(), 13);

        runtime.lifecycle = AsioOutputLifecycle::Active;
        context.latch_worker_fault(RenderFault::CallbackPanic);
        let error = runtime.rotate_transport_generation().unwrap_err();
        assert_eq!(error, "output_callback_panic");
        assert_eq!(context.transport().current(), 13);
    }

    #[test]
    fn worker_spawn_failure_cleans_up_started_session_and_retains_close_retry() {
        let profile = MachineAsioOutputProfile::for_test(4, 4, 1, 2, 3);
        let context = AsioRenderContext::new(
            75,
            &profile,
            4,
            Arc::new(TransportGeneration::new(1).unwrap()),
        )
        .unwrap();
        let mut mixers = ProgramCueMixers::new(Arc::clone(&context), profile, 0).unwrap();
        mixers.worker_mut().prefill(2).unwrap();
        let (session, control) = crate::asio_bridge_v3::test_session_with_failures(0, 1);
        let mut runtime = AsioOutputRuntime::default();

        let error = runtime
            .install_started_session(
                session,
                Arc::clone(&context),
                75,
                mixers,
                |_worker, _context, _stop| Err("injected worker spawn failure".to_owned()),
            )
            .unwrap_err();
        assert!(error.contains("injected worker spawn failure"));
        assert!(error.contains("remains retryable"));
        assert_eq!(
            control
                .stop_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1
        );
        assert_eq!(
            control
                .close_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1
        );
        assert!(matches!(
            runtime.lifecycle(),
            AsioOutputLifecycle::StoppedPendingClose(_)
        ));
        assert!(runtime.session.is_some());

        runtime.stop_and_close().unwrap();
        assert_eq!(
            control
                .stop_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1,
            "close retry after a spawn failure must not repeat Stop",
        );
        assert_eq!(
            control
                .close_calls
                .load(std::sync::atomic::Ordering::Acquire),
            2
        );
        assert!(runtime.session.is_none());
        assert_eq!(runtime.lifecycle(), &AsioOutputLifecycle::Closed);
    }

    #[test]
    fn worker_join_panic_still_attempts_bridge_stop_and_close() {
        let (session, control) = crate::asio_bridge_v3::test_session_with_failures(0, 1);
        let stop = Arc::new(AtomicBool::new(false));
        let worker = thread::spawn(|| panic!("injected render worker panic"));
        let mut runtime = AsioOutputRuntime {
            lifecycle: AsioOutputLifecycle::Active,
            timeline_audio_live_fence: TimelineAudioLiveFence::default(),
            session: Some(session),
            context: None,
            session_generation: Some(88),
            preflight: None,
            program_mixer: None,
            cue_mixer: None,
            worker_stop: Some(stop),
            worker: Some(worker),
        };

        let error = runtime.stop_and_close().unwrap_err();
        assert!(error.contains("worker panicked"));
        assert!(error.contains("close"));
        assert_eq!(
            control
                .stop_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1
        );
        assert_eq!(
            control
                .close_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1
        );
        assert!(runtime.session.is_some());
        assert!(matches!(
            runtime.lifecycle(),
            AsioOutputLifecycle::StoppedPendingClose(_)
        ));

        runtime.stop_and_close().unwrap();
        assert_eq!(
            control
                .stop_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1,
            "a worker join panic must not make a pending Close retry repeat Stop",
        );
        assert_eq!(
            control
                .close_calls
                .load(std::sync::atomic::Ordering::Acquire),
            2
        );
        assert!(runtime.session.is_none());
        assert_eq!(runtime.lifecycle(), &AsioOutputLifecycle::Closed);
    }

    #[test]
    fn failed_start_cleanup_with_terminally_closed_session_remains_restartable() {
        let profile = MachineAsioOutputProfile::for_test(4, 4, 1, 2, 3);
        let context = AsioRenderContext::new(
            91,
            &profile,
            4,
            Arc::new(TransportGeneration::new(1).unwrap()),
        )
        .unwrap();
        let (mut session, control) =
            crate::asio_bridge_v3::test_session_with_nonclearing_successful_close();
        let cleanup_error = session.stop_and_close().unwrap_err();
        assert!(session.is_closed());

        let mut runtime = AsioOutputRuntime::default();
        runtime.lifecycle = AsioOutputLifecycle::Starting;
        runtime.install_failed_start_cleanup_session(
            session,
            context,
            91,
            cleanup_error.to_string(),
        );

        assert_eq!(runtime.lifecycle(), &AsioOutputLifecycle::Closed);
        assert!(runtime.session.is_none());
        assert!(runtime.context.is_none());
        assert_eq!(
            control
                .close_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1,
            "a terminally consumed/quarantined cleanup handle must not be retried",
        );
    }

    #[test]
    fn ambiguous_successful_close_is_not_retained_for_retry() {
        let profile = MachineAsioOutputProfile::for_test(4, 4, 1, 2, 3);
        let context = AsioRenderContext::new(
            92,
            &profile,
            4,
            Arc::new(TransportGeneration::new(1).unwrap()),
        )
        .unwrap();
        let (session, control) =
            crate::asio_bridge_v3::test_session_with_nonclearing_successful_close();
        let mut runtime = AsioOutputRuntime {
            lifecycle: AsioOutputLifecycle::Active,
            timeline_audio_live_fence: TimelineAudioLiveFence::default(),
            session: Some(session),
            context: Some(context),
            session_generation: Some(92),
            preflight: None,
            program_mixer: None,
            cue_mixer: None,
            worker_stop: None,
            worker: None,
        };

        let error = runtime.stop_and_close().unwrap_err();
        assert!(error.contains("stale handle was quarantined"));
        assert_eq!(runtime.lifecycle(), &AsioOutputLifecycle::Closed);
        assert!(runtime.session.is_none());
        assert!(runtime.context.is_none());
        assert_eq!(
            control
                .close_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1,
        );
    }

    #[test]
    fn drop_retains_callback_context_when_terminal_stop_fails() {
        let profile = MachineAsioOutputProfile::for_test(4, 4, 1, 2, 3);
        let context = AsioRenderContext::new(
            93,
            &profile,
            4,
            Arc::new(TransportGeneration::new(1).unwrap()),
        )
        .unwrap();
        let weak_context = Arc::downgrade(&context);
        let (session, control) = crate::asio_bridge_v3::test_session_with_failures(1, 0);
        let runtime = AsioOutputRuntime {
            lifecycle: AsioOutputLifecycle::Active,
            timeline_audio_live_fence: TimelineAudioLiveFence::default(),
            session: Some(session),
            context: Some(Arc::clone(&context)),
            session_generation: Some(93),
            preflight: None,
            program_mixer: None,
            cue_mixer: None,
            worker_stop: None,
            worker: None,
        };
        drop(context);

        drop(runtime);

        assert_eq!(
            control
                .stop_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1
        );
        assert_eq!(
            control
                .close_calls
                .load(std::sync::atomic::Ordering::Acquire),
            0
        );
        assert!(
            weak_context.upgrade().is_some(),
            "callback context must be retained for process lifetime when Stop/Close is nonterminal",
        );
    }

    #[test]
    fn latched_terminal_fault_transitions_to_fault_and_removes_mixer_access() {
        let (mut runtime, context, _program_source, _cue_source) = active_runtime(63, 12);
        context.latch_worker_fault(RenderFault::CallbackPanic);

        let error = runtime.poll_terminal_fault().unwrap_err();
        assert_eq!(error, "output_callback_panic");
        assert_eq!(
            runtime.lifecycle(),
            &AsioOutputLifecycle::Fault("output_callback_panic".to_owned())
        );
        assert!(runtime
            .current_output_identity(TimelineAudioOutputBus::Program)
            .is_err());

        // The latch is permanent: a later poll/admission cannot auto-recover
        // the lifecycle or manufacture a normal-output fallback.
        assert_eq!(runtime.poll_terminal_fault().unwrap_err(), error);
        assert_eq!(
            runtime.lifecycle(),
            &AsioOutputLifecycle::Fault("output_callback_panic".to_owned())
        );
        assert!(runtime
            .current_output_identity(TimelineAudioOutputBus::Cue)
            .is_err());
    }

    #[test]
    fn preflight_runtime_commands_are_identity_gated_and_mutually_exclusive() {
        let mut inactive = AsioOutputRuntime::default();
        assert!(inactive.preflight_status(0).is_err());

        let (mut runtime, _context, _program_source, _cue_source) = active_runtime(64, 14);
        let identity = runtime.current_preflight_identity().unwrap();
        let test = runtime
            .set_preflight_test_for_identity(identity, PreflightTarget::ProgramLeft, 100, 1000)
            .unwrap();
        assert_eq!(test.identity(), identity);
        assert_eq!(test.test(), Some(PreflightTarget::ProgramLeft));
        assert_eq!(test.solo(), PreflightSolo::None);

        let error = runtime
            .set_preflight_solo_for_identity(identity, PreflightSolo::CueOnly, 101)
            .unwrap_err();
        assert!(error.contains("already active"));
        let unchanged = runtime.preflight_status(101).unwrap();
        assert_eq!(unchanged.test(), Some(PreflightTarget::ProgramLeft));
        assert_eq!(unchanged.solo(), PreflightSolo::None);

        // Off is a transaction that releases only the test gate.  Solo can
        // then be admitted, while a new test remains blocked by that solo.
        runtime
            .set_preflight_test_for_identity(identity, PreflightTarget::Off, 102, 1)
            .unwrap();
        let solo = runtime
            .set_preflight_solo_for_identity(identity, PreflightSolo::CueOnly, 103)
            .unwrap();
        assert_eq!(solo.test(), None);
        assert_eq!(solo.solo(), PreflightSolo::CueOnly);
        let error = runtime
            .set_preflight_test_for_identity(identity, PreflightTarget::Cue, 104, 1000)
            .unwrap_err();
        assert!(error.contains("blocked by CueOnly"));
        let stale = AsioPreflightIdentity::new(64, 13).unwrap();
        let error = runtime
            .set_preflight_solo_for_identity(stale, PreflightSolo::None, 105)
            .unwrap_err();
        assert!(error.contains("identity mismatch"));
    }

    #[test]
    fn timeline_live_admission_clears_test_blocker_before_retry() {
        let (mut runtime, context, _program_source, _cue_source) = active_runtime(69, 20);
        let identity = runtime.current_preflight_identity().unwrap();
        let now_ms = preflight_now_ms();
        runtime
            .set_preflight_test_for_identity(identity, PreflightTarget::Cue, now_ms, 30_000)
            .unwrap();
        assert_eq!(
            runtime.preflight_status(now_ms).unwrap().test(),
            Some(PreflightTarget::Cue)
        );
        let epoch_before = context.preflight_test_epoch().load(Ordering::Acquire);

        let status = runtime
            .admit_timeline_live_playback_for_identity(identity)
            .unwrap();
        assert_eq!(status.identity(), identity);
        assert!(status.live_playback_active());
        assert_eq!(status.test(), None);
        assert_eq!(status.solo(), PreflightSolo::None);
        assert_eq!(
            context.preflight_test_epoch().load(Ordering::Acquire),
            epoch_before + 1
        );
    }

    #[test]
    fn timeline_live_admission_clears_solo_blocker_only() {
        let (mut runtime, context, _program_source, _cue_source) = active_runtime(70, 21);
        let identity = runtime.current_preflight_identity().unwrap();
        let now_ms = preflight_now_ms();
        runtime
            .set_preflight_solo_for_identity(identity, PreflightSolo::ProgramOnly, now_ms)
            .unwrap();
        assert_eq!(
            runtime.preflight_status(now_ms).unwrap().solo(),
            PreflightSolo::ProgramOnly
        );
        let epoch_before = context.preflight_test_epoch().load(Ordering::Acquire);

        let status = runtime
            .admit_timeline_live_playback_for_identity(identity)
            .unwrap();
        assert_eq!(status.identity(), identity);
        assert!(status.live_playback_active());
        assert_eq!(status.test(), None);
        assert_eq!(status.solo(), PreflightSolo::None);
        assert_eq!(
            context.preflight_test_epoch().load(Ordering::Acquire),
            epoch_before + 1
        );
    }

    #[test]
    fn timeline_live_admission_non_blocker_keeps_selection_and_fence_unchanged() {
        let (mut runtime, context, _program_source, _cue_source) = active_runtime(71, 22);
        let identity = runtime.current_preflight_identity().unwrap();
        let now_ms = preflight_now_ms();
        runtime
            .set_preflight_test_for_identity(identity, PreflightTarget::ProgramLeft, now_ms, 30_000)
            .unwrap();
        let before = runtime
            .preflight
            .as_ref()
            .unwrap()
            .state()
            .lock()
            .unwrap()
            .clone();
        let epoch_before = context.preflight_test_epoch().load(Ordering::Acquire);
        let stale = AsioPreflightIdentity::new(71, 21).unwrap();

        let error = runtime
            .admit_timeline_live_playback_for_identity(stale)
            .unwrap_err();
        assert!(error.contains("identity mismatch"));
        assert_eq!(
            *runtime.preflight.as_ref().unwrap().state().lock().unwrap(),
            before
        );
        assert_eq!(
            context.preflight_test_epoch().load(Ordering::Acquire),
            epoch_before
        );
    }

    #[test]
    fn timeline_live_admission_fences_an_already_queued_test_block() {
        let (mut runtime, context, mut worker) =
            AsioOutputRuntime::active_in_memory_with_worker(72, 23).unwrap();
        let identity = runtime.current_preflight_identity().unwrap();
        let now_ms = preflight_now_ms();
        runtime
            .set_preflight_test_for_identity(identity, PreflightTarget::ProgramLeft, now_ms, 30_000)
            .unwrap();
        worker.render_one_complete_block_at(now_ms).unwrap();

        runtime
            .admit_timeline_live_playback_for_identity(identity)
            .unwrap();

        let mut destination = [1.0_f32; 16];
        let result = unsafe {
            context.callback_copy(
                destination.as_mut_ptr(),
                4,
                4,
                0,
                identity.session_generation(),
                identity.session_generation(),
            )
        };
        assert_eq!(result, crate::asio_program_cue_render::CALLBACK_ACCEPTED);
        assert_eq!(destination, [0.0; 16]);
    }

    #[test]
    fn timeline_live_admission_preserves_a_stopped_phase_error() {
        let (mut runtime, context, _program_source, _cue_source) = active_runtime(73, 24);
        let identity = runtime.current_preflight_identity().unwrap();
        let control = runtime.preflight.as_ref().unwrap().clone();
        control
            .state()
            .lock()
            .unwrap()
            .clear_for_stop(identity)
            .unwrap();
        let before = control.state().lock().unwrap().clone();
        let epoch_before = context.preflight_test_epoch().load(Ordering::Acquire);

        let error = runtime
            .admit_timeline_live_playback_for_identity(identity)
            .unwrap_err();
        assert!(error.contains("Stopped"));
        assert_eq!(*control.state().lock().unwrap(), before);
        assert_eq!(
            context.preflight_test_epoch().load(Ordering::Acquire),
            epoch_before
        );
    }

    #[test]
    fn timeline_live_admission_applies_the_existing_terminal_fault_transition() {
        let (mut runtime, context, _program_source, _cue_source) = active_runtime(74, 25);
        let identity = runtime.current_preflight_identity().unwrap();
        let now_ms = preflight_now_ms();
        runtime
            .set_preflight_test_for_identity(identity, PreflightTarget::Cue, now_ms, 30_000)
            .unwrap();
        let epoch_before = context.preflight_test_epoch().load(Ordering::Acquire);
        context.latch_worker_fault(RenderFault::CallbackPanic);

        let error = runtime
            .admit_timeline_live_playback_for_identity(identity)
            .unwrap_err();
        assert!(error.contains("callback_panic"));
        assert!(matches!(runtime.lifecycle(), AsioOutputLifecycle::Fault(_)));
        let state = runtime
            .preflight
            .as_ref()
            .unwrap()
            .state()
            .lock()
            .unwrap()
            .clone();
        let mut expected = crate::asio_program_cue::AsioPreflightState::from_identity(identity);
        expected.clear_for_fault(identity).unwrap();
        assert_eq!(state, expected);
        assert_eq!(state.phase(), PreflightPhase::Faulted);
        assert_eq!(state.solo(), PreflightSolo::None);
        assert!(!state.live_playback_active());
        assert_eq!(
            context.preflight_test_epoch().load(Ordering::Acquire),
            epoch_before + 1
        );
    }

    #[test]
    fn timeline_live_admission_fences_when_state_lock_is_poisoned() {
        let (mut runtime, context, _program_source, _cue_source) = active_runtime(75, 26);
        let control = runtime.preflight.as_ref().unwrap().clone();
        let before = control.state().lock().unwrap().clone();
        let epoch_before = context.preflight_test_epoch().load(Ordering::Acquire);
        let poison_control = control.clone();
        let poisoned = std::panic::catch_unwind(move || {
            let _guard = poison_control.state().lock().unwrap();
            panic!("poison preflight state for admission proof");
        });
        assert!(poisoned.is_err());
        let identity = runtime.current_preflight_identity().unwrap();

        let error = runtime
            .admit_timeline_live_playback_for_identity(identity)
            .unwrap_err();
        assert!(error.contains("lock was poisoned"));
        let state = control.state().lock().unwrap_err().into_inner().clone();
        assert_eq!(state, before);
        assert_eq!(
            context.preflight_test_epoch().load(Ordering::Acquire),
            epoch_before + 1
        );
        assert_eq!(
            context.preflight_selection_kind().load(Ordering::Acquire),
            0
        );
        assert_eq!(
            context
                .preflight_selection_signature()
                .load(Ordering::Acquire),
            0
        );
        assert_eq!(context.terminal_fault(), Some(RenderFault::InvalidBlock));
    }

    #[test]
    fn preflight_runtime_rotation_expires_old_identity_and_clears_selection() {
        let (mut runtime, _context, _program_source, _cue_source) = active_runtime(65, 15);
        let old = runtime.current_preflight_identity().unwrap();
        runtime
            .set_preflight_test_for_identity(old, PreflightTarget::Cue, 0, 1000)
            .unwrap();
        assert_eq!(runtime.rotate_transport_generation().unwrap(), 16);

        let current = runtime.current_preflight_identity().unwrap();
        assert_eq!(current.session_generation(), 65);
        assert_eq!(current.transport_generation(), 16);
        let status = runtime.preflight_status(1).unwrap();
        assert_eq!(status.identity(), current);
        assert_eq!(status.test(), None);
        assert_eq!(status.solo(), PreflightSolo::None);

        let error = runtime
            .set_preflight_test_for_identity(old, PreflightTarget::Cue, 2, 1000)
            .unwrap_err();
        assert!(error.contains("identity mismatch"));
    }

    #[test]
    fn preflight_epoch_exhaustion_propagates_error_and_latches_context() {
        let (mut runtime, context, _program_source, _cue_source) = active_runtime(68, 19);
        let identity = runtime.current_preflight_identity().unwrap();
        context
            .preflight_test_epoch()
            .store(u64::MAX, Ordering::Release);

        let error = runtime
            .set_preflight_test_for_identity(identity, PreflightTarget::Off, 0, 1)
            .unwrap_err();
        assert!(error.contains("preflight_epoch_exhausted"));
        assert_eq!(
            context.terminal_fault(),
            Some(RenderFault::PreflightEpochExhausted)
        );
        assert_eq!(
            runtime.poll_terminal_fault().unwrap_err(),
            "preflight_epoch_exhausted"
        );
    }

    #[test]
    fn preflight_runtime_live_playback_gate_and_stop_clear_are_fail_closed() {
        let (mut runtime, _context, _program_source, _cue_source) = active_runtime(66, 17);
        let control = runtime.preflight.as_ref().unwrap().clone();
        let identity = runtime.current_preflight_identity().unwrap();

        let active = runtime.set_preflight_live_playback(true).unwrap();
        assert!(active.live_playback_active());
        assert!(runtime
            .set_preflight_test(PreflightTarget::Cue, 0, 100)
            .unwrap_err()
            .contains("live show playback"));
        assert!(runtime
            .set_preflight_solo(PreflightSolo::ProgramOnly, 0)
            .unwrap_err()
            .contains("live show playback"));
        runtime.set_preflight_live_playback(false).unwrap();
        runtime
            .set_preflight_test(PreflightTarget::Cue, 10, 100)
            .unwrap();

        let (session, _session_control) = crate::asio_bridge_v3::test_session_with_failures(0, 0);
        runtime.session = Some(session);
        runtime.stop_and_close().unwrap();
        {
            let mut state = control.state().lock().unwrap();
            assert_eq!(state.identity(), identity);
            assert_eq!(state.phase(), PreflightPhase::Stopped);
            assert_eq!(state.solo(), PreflightSolo::None);
            assert_eq!(
                state.test_target(identity, 10).unwrap_err().to_string(),
                "ASIO preflight requires Active session 66 / transport 17; current phase is Stopped"
            );
        }
        assert_eq!(runtime.lifecycle(), &AsioOutputLifecycle::Closed);

        // A terminal context fault clears a fresh active state's selection and
        // gates every subsequent UI operation behind Fault lifecycle.
        let (mut faulted, context, _program_source, _cue_source) = active_runtime(67, 18);
        let fault_control = faulted.preflight.as_ref().unwrap().clone();
        faulted
            .set_preflight_solo(PreflightSolo::ProgramOnly, 20)
            .unwrap();
        context.latch_worker_fault(RenderFault::CallbackPanic);
        assert_eq!(
            faulted.poll_terminal_fault().unwrap_err(),
            "output_callback_panic"
        );
        let fault_state = fault_control.state().lock().unwrap();
        assert_eq!(fault_state.phase(), PreflightPhase::Faulted);
        assert_eq!(fault_state.solo(), PreflightSolo::None);
        drop(fault_state);
        assert!(faulted.preflight_status(20).is_err());
    }
}
