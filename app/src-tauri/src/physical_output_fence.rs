//! Shared physical-output FC-28 authority fence and Timeline Follow
//! settlement machinery for the Spout and NDI transports.
//!
//! This source is included once per transport parent module via
//! `#[path = "physical_output_fence.rs"] mod physical_output_fence;`, so both
//! backends compile the exact same fence, watchdog, and settlement logic from
//! this one file without a `main.rs` module declaration. Backend-specific
//! differences (SDK kind, sender identity labels) enter only as parameters,
//! and every backend keeps its own authority-capture call sites so the
//! per-backend suites prove identical behavior through the identical code.
//!
//! FC-28 authority fence: the engine-issued presentation configuration token
//! is bound to the paired snapshot through `video_presentation_sample` (the
//! token is loaded strictly before the snapshot read), so every published
//! presentation mutation after capture — including content-restoring
//! mutate/revert sequences that deep equality cannot see — is rejected by an
//! exact token comparison immediately before the SDK boundary. Deep video
//! content equality is deliberately not rechecked here: ordinary playback
//! advances playhead/transition truth inside `snapshot.video` on every engine
//! tick without touching the token, so a deep comparison would mass-revoke
//! normal playback. Ownership epoch/generation/state/role/reasons/error, the
//! safety-blackout latch authority, and the full route/kind/endpoint/
//! mapping/enablement/dimension identity of the bound output are rechecked in
//! the same pre-dispatch pass. The sender name remains part of this identity
//! because an output ID may not be rebound to a different physical resource
//! under an already-created sender.
//!
//! Bounded settlement: every continuously non-admitted Follow generation
//! (Retry, Revoked, NotApplicable, or stuck ownership admission) expires into
//! a visible Fault through the engine settlement API instead of extending a
//! Timeline Follow hold indefinitely.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use engine::{
    EngineHandle, OutputOwnershipTeardownLease, SafetyBlackoutAuthority,
    TimelineFollowVideoRenderSnapshot, VideoPresentationSample,
};
use protocol::{
    OutputOwnershipState, TimelineFollowSettlementAck, TimelineFollowSettlementAckResult,
    TimelineFollowSettlementConsumerId, TimelineFollowSettlementDomain, VideoOutputId,
    VideoOutputKind,
};

pub(super) const TIMELINE_FOLLOW_SETTLEMENT_ACK_TTL: Duration = Duration::from_secs(1);
pub(super) const TIMELINE_FOLLOW_SETTLEMENT_ACK_RETRY: Duration = Duration::from_millis(25);
pub(super) const TIMELINE_FOLLOW_TERMINAL_RECEIPT_RETRIES: u8 = 3;

/// Upper bound on how long one Follow generation may remain continuously
/// unresolved (retried, revoked, or otherwise unadmitted) in one output
/// worker. Expiry is a visible Fault through the engine settlement API; it
/// never invents success and never silently drops the hold.
pub(super) const TIMELINE_FOLLOW_SETTLEMENT_DEADLINE: Duration = Duration::from_secs(2);

/// Upper bound on one frame's wait for video output ownership admission.
/// Expiry faults visibly instead of spinning until the route is stopped.
pub(super) const VIDEO_OUTPUT_ADMISSION_DEADLINE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct TimelineFollowOutputFrameKey {
    pub(crate) epoch: u64,
    pub(crate) generation: u64,
    pub(crate) output_id: VideoOutputId,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

/// Engine settlement identity intentionally excludes presentation size.
/// A resize must invalidate the cached frame without producing another
/// terminal result for the same Follow output consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TimelineFollowOutputAckKey {
    epoch: u64,
    generation: u64,
    output_id: VideoOutputId,
}

impl TimelineFollowOutputFrameKey {
    fn ack_key(self) -> TimelineFollowOutputAckKey {
        TimelineFollowOutputAckKey {
            epoch: self.epoch,
            generation: self.generation,
            output_id: self.output_id,
        }
    }

    pub(super) fn settlement_ack(
        self,
        result: TimelineFollowSettlementAckResult,
    ) -> TimelineFollowSettlementAck {
        TimelineFollowSettlementAck {
            epoch: self.epoch,
            generation: self.generation,
            domain: TimelineFollowSettlementDomain::Video,
            consumer_id: TimelineFollowSettlementConsumerId::VideoOutput {
                output_id: self.output_id,
            },
            result,
        }
    }
}

#[derive(Debug, Clone)]
struct PendingTimelineFollowSettlementAck {
    pub(super) ack: TimelineFollowSettlementAck,
    retry_not_before: Instant,
}

/// Continuous non-admission of exactly one Follow generation in one worker.
/// A new generation or an admitted send resets the clock.
#[derive(Debug, Clone)]
struct UnresolvedFollowSettlement {
    ack_key: TimelineFollowOutputAckKey,
    since: Instant,
    last_reason: String,
}

/// Renderer-local continuity, acknowledgement, and bounded-settlement state
/// for one production output worker. A publication error retains the exact
/// first result so a reply-loss retry can never change Fault into Applied for
/// the same consumer.
#[derive(Debug, Default)]
pub(super) struct TimelineFollowOutputState {
    active_ack_key: Option<TimelineFollowOutputAckKey>,
    active_frame_key: Option<TimelineFollowOutputFrameKey>,
    last_valid: Option<(TimelineFollowOutputFrameKey, video::VideoFrame)>,
    pending_ack: Option<PendingTimelineFollowSettlementAck>,
    published_ack: Option<TimelineFollowSettlementAck>,
    unresolved: Option<UnresolvedFollowSettlement>,
    #[cfg(test)]
    settlement_deadline_override: Option<Duration>,
}

impl TimelineFollowOutputState {
    fn settlement_deadline(&self) -> Duration {
        #[cfg(test)]
        if let Some(deadline) = self.settlement_deadline_override {
            return deadline;
        }
        TIMELINE_FOLLOW_SETTLEMENT_DEADLINE
    }

    /// Generation/output changes fence stale ACK retries; size changes only
    /// retire last-valid continuity. The first physical terminal result for a
    /// single ACK identity remains exact through a lost engine reply. A new
    /// ACK identity also restarts bounded-settlement tracking.
    pub(super) fn observe_key(&mut self, key: TimelineFollowOutputFrameKey) {
        let ack_key = key.ack_key();
        if self.active_ack_key != Some(ack_key) {
            self.active_ack_key = Some(ack_key);
            self.pending_ack = None;
            self.published_ack = None;
            self.unresolved = None;
        }
        if self.active_frame_key != Some(key) {
            self.active_frame_key = Some(key);
            self.last_valid = None;
        }
    }

    pub(super) fn last_valid(
        &self,
        key: TimelineFollowOutputFrameKey,
    ) -> Option<&video::VideoFrame> {
        self.last_valid
            .as_ref()
            .and_then(|(cached_key, frame)| (*cached_key == key).then_some(frame))
    }

    pub(super) fn update_last_valid(
        &mut self,
        key: TimelineFollowOutputFrameKey,
        frame: video::VideoFrame,
    ) {
        self.observe_key(key);
        self.last_valid = Some((key, frame));
    }

    pub(super) fn queue_ack(
        &mut self,
        key: TimelineFollowOutputFrameKey,
        result: TimelineFollowSettlementAckResult,
        now: Instant,
    ) {
        self.observe_key(key);
        let ack = key.settlement_ack(result);
        if self.published_for(key) || self.pending_ack.is_some() {
            return;
        }
        self.pending_ack = Some(PendingTimelineFollowSettlementAck {
            ack,
            retry_not_before: now,
        });
    }

    pub(super) fn try_publish_ack(
        &mut self,
        now: Instant,
        mut publish: impl FnMut(&TimelineFollowSettlementAck) -> Result<(), String>,
    ) -> Result<bool, String> {
        let Some(pending) = self.pending_ack.as_mut() else {
            return Ok(false);
        };
        if now < pending.retry_not_before {
            return Ok(false);
        }
        let ack = pending.ack.clone();
        match publish(&ack) {
            Ok(()) => {
                self.published_ack = Some(ack);
                self.pending_ack = None;
                Ok(true)
            }
            Err(error) => {
                pending.retry_not_before = now + TIMELINE_FOLLOW_SETTLEMENT_ACK_RETRY;
                Err(error)
            }
        }
    }

    pub(super) fn published_for(&self, key: TimelineFollowOutputFrameKey) -> bool {
        let ack_key = key.ack_key();
        self.published_ack.as_ref().is_some_and(|ack| {
            ack.epoch == ack_key.epoch
                && ack.generation == ack_key.generation
                && ack.consumer_id
                    == (TimelineFollowSettlementConsumerId::VideoOutput {
                        output_id: ack_key.output_id,
                    })
        })
    }

    /// Record that the tracked Follow generation reached its settled physical
    /// outcome (an admitted send). Clears the unresolved clock.
    pub(super) fn note_follow_admitted(&mut self) {
        self.unresolved = None;
    }

    /// Advance the bounded-settlement clock for one more non-admitted
    /// iteration of `key`. Returns true exactly when the settlement deadline
    /// expires for this generation; the caller must then fault visibly and
    /// stop admitting further silent retries.
    pub(super) fn tick_follow_unresolved(
        &mut self,
        key: TimelineFollowOutputFrameKey,
        now: Instant,
        reason: &str,
    ) -> bool {
        let ack_key = key.ack_key();
        let deadline = self.settlement_deadline();
        match self.unresolved.as_mut() {
            Some(unresolved) if unresolved.ack_key == ack_key => {
                unresolved.last_reason = reason.to_string();
                if now.duration_since(unresolved.since) >= deadline {
                    self.unresolved = None;
                    return true;
                }
                false
            }
            _ => {
                self.unresolved = Some(UnresolvedFollowSettlement {
                    ack_key,
                    since: now,
                    last_reason: reason.to_string(),
                });
                false
            }
        }
    }

    #[cfg(test)]
    pub(super) fn set_settlement_deadline(&mut self, deadline: Duration) {
        self.settlement_deadline_override = Some(deadline);
    }

    #[cfg(test)]
    pub(super) fn pending_ack_result(&self) -> Option<&TimelineFollowSettlementAckResult> {
        self.pending_ack.as_ref().map(|pending| &pending.ack.result)
    }
}

#[derive(Debug)]
pub(super) enum TimelineFollowOutputRenderDecision {
    Frame {
        key: TimelineFollowOutputFrameKey,
        frame: video::VideoFrame,
        result: TimelineFollowSettlementAckResult,
        follow_identity: Option<TimelineFollowActiveIdentity>,
    },
    NotApplicable {
        key: TimelineFollowOutputFrameKey,
        reason: String,
    },
    /// The local Follow sample was superseded before an engine-issued
    /// presentation token could exist. Do not acknowledge or fence it: the
    /// next loop must capture a new authoritative sample. The optional frame
    /// key feeds the bounded-settlement watchdog while the same generation
    /// keeps failing to admit.
    Retry {
        reason: String,
        key: Option<TimelineFollowOutputFrameKey>,
    },
}

/// Runtime-only identity for the active Follow presentation. A new progress
/// sample is fine; retirement or a new generation is not the same send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TimelineFollowActiveIdentity {
    pub(crate) epoch: u64,
    pub(crate) generation: u64,
    pub(crate) source_timeline_id: protocol::TimelineId,
    pub(crate) target_timeline_id: protocol::TimelineId,
}

pub(super) fn timeline_follow_active_identity(
    follow: &TimelineFollowVideoRenderSnapshot,
) -> TimelineFollowActiveIdentity {
    TimelineFollowActiveIdentity {
        epoch: follow.epoch,
        generation: follow.generation,
        source_timeline_id: follow.source_timeline_id,
        target_timeline_id: follow.target_timeline_id,
    }
}

pub(super) fn validate_timeline_follow_active_identity(
    backend_label: &str,
    expected: TimelineFollowActiveIdentity,
    current: Option<&TimelineFollowVideoRenderSnapshot>,
) -> Result<(), String> {
    let current = current.ok_or_else(|| {
        format!(
            "{backend_label} Timeline Follow {}:{} was retired before physical send",
            expected.epoch, expected.generation
        )
    })?;
    let actual = timeline_follow_active_identity(current);
    if actual != expected {
        return Err(format!(
            "{backend_label} Timeline Follow identity changed before physical send: expected {}:{} {}->{}, actual {}:{} {}->{}",
            expected.epoch,
            expected.generation,
            expected.source_timeline_id.0,
            expected.target_timeline_id.0,
            actual.epoch,
            actual.generation,
            actual.source_timeline_id.0,
            actual.target_timeline_id.0,
        ));
    }
    Ok(())
}

fn revalidate_timeline_follow_active_identity(
    engine: &EngineHandle,
    backend_label: &str,
    expected: TimelineFollowActiveIdentity,
) -> Result<(), String> {
    validate_timeline_follow_active_identity(
        backend_label,
        expected,
        engine.timeline_follow_video_render_snapshot().as_ref(),
    )
}

fn timeline_follow_output_config(
    snapshot: &protocol::VideoSnapshot,
    output_id: VideoOutputId,
) -> Result<&protocol::VideoOutputSummary, String> {
    snapshot
        .outputs
        .iter()
        .find(|output| output.id == output_id)
        .ok_or_else(|| format!("Timeline Follow output {output_id} was not found"))
}

/// The ownership permit fences ownership rotation while it is held. Sampling the
/// ready owner after acquisition therefore rejects a stale Follow snapshot
/// before either physical send or Applied acknowledgement.
#[cfg(test)]
pub(super) fn timeline_follow_epoch_matches_ready_owner(
    engine: &EngineHandle,
    key: TimelineFollowOutputFrameKey,
) -> bool {
    let ownership = engine.output_ownership_status();
    ownership.video_allowed
        && ownership.state == OutputOwnershipState::Ready
        && ownership.epoch == key.epoch
}

/// Comparable local observations used to admit one CPU-backed output render.
/// The owner, rendered project image, output route, mapping, safety bits, and
/// the runtime safety-blackout latch authority are captured as one authority
/// and rechecked immediately before the SDK boundary. See the module docs for
/// why the presentation token — not deep video equality — is the content
/// mutation fence.
///
/// There is intentionally no "epoch zero is probably okay" or first-output
/// compatibility path. Zero is an ordinary comparable epoch; a missing or
/// changed identity is never sendable.
#[derive(Debug, Clone)]
pub(super) struct OutputPresentationAuthority {
    ownership: protocol::OutputOwnershipStatus,
    project_blackout: bool,
    blackout_authority: SafetyBlackoutAuthority,
    output: protocol::VideoOutputSummary,
    route_endpoint_name: String,
    presentation_config_token: u64,
}

pub(super) fn capture_output_presentation_authority(
    backend_label: &str,
    expected_kind: VideoOutputKind,
    sample: &VideoPresentationSample,
    ownership: protocol::OutputOwnershipStatus,
    blackout_authority: SafetyBlackoutAuthority,
    output_id: VideoOutputId,
    route_endpoint_name: &str,
) -> Result<OutputPresentationAuthority, String> {
    if ownership.state != OutputOwnershipState::Ready || !ownership.video_allowed {
        return Err(format!(
            "{backend_label} output {output_id} is not currently owned for video presentation"
        ));
    }
    let snapshot = &sample.snapshot;
    let output = snapshot
        .video
        .outputs
        .iter()
        .find(|output| output.id == output_id)
        .cloned()
        .ok_or_else(|| format!("{backend_label} output {output_id} was removed before render"))?;
    if !output.enabled {
        return Err(format!("{backend_label} output {output_id} is disabled"));
    }
    if output.kind != expected_kind {
        return Err(format!(
            "{backend_label} output {output_id} route resource kind is not {expected_kind:?}"
        ));
    }
    if output.endpoint_name.as_deref() != Some(route_endpoint_name) {
        return Err(format!(
            "{backend_label} output {output_id} endpoint does not match bound sender '{route_endpoint_name}'"
        ));
    }
    if output.width == 0 || output.height == 0 {
        return Err(format!(
            "{backend_label} output {output_id} has an invalid zero-area {}x{} contract",
            output.width, output.height
        ));
    }
    Ok(OutputPresentationAuthority {
        ownership,
        project_blackout: snapshot.blackout,
        blackout_authority,
        output,
        route_endpoint_name: route_endpoint_name.to_string(),
        presentation_config_token: sample.config_token,
    })
}

impl OutputPresentationAuthority {
    pub(super) fn ownership_epoch(&self) -> u64 {
        self.ownership.epoch
    }

    #[cfg(test)]
    pub(super) fn ownership_status(&self) -> protocol::OutputOwnershipStatus {
        self.ownership.clone()
    }

    pub(super) fn project_blackout(&self) -> bool {
        self.project_blackout
    }

    #[cfg(test)]
    pub(super) fn blackout_authority(&self) -> SafetyBlackoutAuthority {
        self.blackout_authority
    }

    #[cfg(test)]
    pub(super) fn presentation_config_token(&self) -> u64 {
        self.presentation_config_token
    }

    pub(super) fn output_summary(&self) -> &protocol::VideoOutputSummary {
        &self.output
    }

    #[cfg(test)]
    pub(super) fn override_output_mapping_for_test(
        &mut self,
        mapping: protocol::VideoOutputMapping,
    ) {
        self.output.mapping = mapping;
    }
}

pub(super) fn revalidate_output_presentation_authority(
    engine: &EngineHandle,
    backend_label: &str,
    authority: &OutputPresentationAuthority,
) -> Result<(), String> {
    // Exact ownership role/state/generation/epoch/reasons/error continuity.
    let ownership = engine.output_ownership_status();
    if ownership != authority.ownership {
        return Err(format!(
            "{backend_label} output {} authority changed before send",
            authority.output.id
        ));
    }
    if ownership.state != OutputOwnershipState::Ready || !ownership.video_allowed {
        return Err(format!(
            "{backend_label} output {} is no longer owned for video presentation",
            authority.output.id
        ));
    }
    let sample = engine.video_presentation_sample();
    let current = &sample.snapshot;
    // Safety blackout latch: visible bit plus engaged/epoch/generation
    // authority.
    if current.blackout != authority.project_blackout {
        return Err(format!(
            "{backend_label} output {} project safety blackout changed before send",
            authority.output.id
        ));
    }
    if engine.safety_blackout_authority() != authority.blackout_authority {
        return Err(format!(
            "{backend_label} output {} project safety blackout authority changed before send",
            authority.output.id
        ));
    }
    // Exact route/kind/endpoint and output identity/mapping/enabled/
    // dimension continuity for the bound output.
    let output = current
        .video
        .outputs
        .iter()
        .find(|output| output.id == authority.output.id)
        .ok_or_else(|| {
            format!(
                "{backend_label} output {} was removed before send",
                authority.output.id
            )
        })?;
    if output.kind != authority.output.kind
        || output.endpoint_name.as_deref() != Some(authority.route_endpoint_name.as_str())
    {
        return Err(format!(
            "{backend_label} output {} bound route resource identity changed before send",
            authority.output.id
        ));
    }
    if output != &authority.output || !output.enabled {
        return Err(format!(
            "{backend_label} output {} route, mapping, or enablement changed before send",
            authority.output.id
        ));
    }
    // Presentation-content mutation fence: every published presentation
    // mutation — including a mutate/revert pair that restores deep equality
    // exactly — advanced this engine-issued token. Ordinary per-tick
    // playback progress never advances it, so live position/transition
    // changes are admitted without mass revocation.
    if sample.config_token != authority.presentation_config_token {
        return Err(format!(
            "{backend_label} output {} presentation authority token advanced before send (captured {}, current {})",
            authority.output.id,
            authority.presentation_config_token,
            sample.config_token
        ));
    }
    Ok(())
}

pub(super) fn materialize_output_artistic_result(
    result: video::VideoOutputArtisticRenderResult,
    authority: &OutputPresentationAuthority,
) -> Result<video::VideoFrame, String> {
    let contract = video::VideoOutputPresentationContract::new(
        authority.ownership.epoch,
        authority.output.id,
        authority.output.width,
        authority.output.height,
        &authority.output.mapping,
    );
    video::materialize_output_artistic_rgba8_for_transport(result, &contract)
        .map(video::VideoOutputRgba8Presentation::into_frame)
        .map_err(|error| {
            format!(
                "Physical output {} presentation rejected: {error:?}",
                authority.output.id
            )
        })
}

pub(super) fn validate_follow_output_authority(
    backend_label: &str,
    follow: &TimelineFollowVideoRenderSnapshot,
    authority: &OutputPresentationAuthority,
) -> Result<(), String> {
    for (label, video) in [
        ("outgoing", &follow.outgoing_video),
        ("incoming", &follow.incoming_video),
    ] {
        let output = timeline_follow_output_config(video, authority.output.id)?;
        if output != &authority.output {
            return Err(format!(
                "{backend_label} Timeline Follow {label} output {} does not match the current project authority",
                authority.output.id
            ));
        }
    }
    Ok(())
}

#[derive(Debug)]
pub(super) enum PhysicalOutputSendError {
    /// The transport observed a changed authority before calling the SDK.
    /// This is a revoke/re-render condition, never a physical-send fault.
    Revoked(String),
    /// The SDK call itself was attempted and failed.
    Sdk(String),
}

impl std::fmt::Display for PhysicalOutputSendError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Revoked(error) | Self::Sdk(error) => formatter.write_str(error),
        }
    }
}

pub(super) fn send_frame_if_authorized<T>(
    backend_label: &str,
    engine: &EngineHandle,
    authority: &OutputPresentationAuthority,
    follow_identity: Option<TimelineFollowActiveIdentity>,
    send: impl FnOnce() -> Result<T, String>,
) -> Result<T, PhysicalOutputSendError> {
    // FC-28 fail-close authority fence. The full revalidation compares the
    // captured ownership epoch/generation, safety-blackout authority, and
    // complete mapping/output identity against a fresh engine presentation
    // sample, ending with an exact presentation-token comparison. The final
    // statement before the SDK boundary is one more bare token load: every
    // published presentation mutation revokes the frame with zero SDK sends,
    // and only an unchanged token admits exactly one physical commit. A
    // mutation published entirely inside the residual single-load window
    // after that final load remains an engine-API TOCTOU boundary; the engine
    // exposes no lease that holds these authorities across an external SDK
    // call.
    revalidate_output_presentation_authority(engine, backend_label, authority)
        .map_err(PhysicalOutputSendError::Revoked)?;
    if let Some(follow_identity) = follow_identity {
        revalidate_timeline_follow_active_identity(engine, backend_label, follow_identity)
            .map_err(PhysicalOutputSendError::Revoked)?;
    }
    if engine.video_presentation_config_token() != authority.presentation_config_token {
        return Err(PhysicalOutputSendError::Revoked(format!(
            "{backend_label} output {} presentation authority token advanced between revalidation and the SDK send (captured {}, current {})",
            authority.output.id,
            authority.presentation_config_token,
            engine.video_presentation_config_token()
        )));
    }
    send().map_err(PhysicalOutputSendError::Sdk)
}

/// Safety state is current-project authority, not historical Follow input.
/// Do not decode/render either historical side when safety blackout is set;
/// retain payload-less blackout until canonical transport materialization.
pub(super) fn render_timeline_follow_hard_blackout_output(
    follow: &TimelineFollowVideoRenderSnapshot,
    authority: &OutputPresentationAuthority,
) -> Result<TimelineFollowOutputRenderDecision, String> {
    if follow.epoch != authority.ownership.epoch {
        return Err(format!(
            "Timeline Follow {}:{} ownership epoch {} does not match current output authority {}",
            follow.epoch, follow.generation, follow.epoch, authority.ownership.epoch
        ));
    }
    // Current project safety is authoritative. In particular, do not read
    // historical output configuration or decode either Follow side merely to
    // discover that an older snapshot was visible: this payload is defined
    // exclusively by the current admitted route contract.
    let key = TimelineFollowOutputFrameKey {
        epoch: follow.epoch,
        generation: follow.generation,
        output_id: authority.output.id,
        width: authority.output.width,
        height: authority.output.height,
    };
    let artistic = video::VideoOutputArtisticRenderResult {
        payload: video::VideoOutputArtisticPayload::HardBlackout,
        output_mapping: authority.output.mapping.clone(),
        output_mapping_identity: video::VideoOutputMappingIdentity::from_mapping(
            &authority.output.mapping,
        ),
        evidence: video::VideoOutputRenderEvidence {
            project_render_epoch: authority.ownership.epoch,
            output_id: authority.output.id,
            freshness: video::VideoOutputRenderFreshness::Fresh,
            error: None,
        },
    };
    Ok(TimelineFollowOutputRenderDecision::Frame {
        key,
        frame: materialize_output_artistic_result(artistic, authority)?,
        result: TimelineFollowSettlementAckResult::Applied,
        follow_identity: Some(timeline_follow_active_identity(follow)),
    })
}

/// Render the exact engine-owned Follow inputs through each snapshot's full
/// output path, then apply the one canonical Follow Transition chain. The
/// returned settlement result is intentionally independent from physical send;
/// callers may publish Applied only after their SDK send succeeds.
pub(super) fn render_timeline_follow_output<P: video::VideoFrameProvider>(
    renderer: &mut video::VideoPreviewRenderer<P>,
    engine_snapshot: &protocol::EngineSnapshot,
    follow: &TimelineFollowVideoRenderSnapshot,
    output_id: VideoOutputId,
    state: &mut TimelineFollowOutputState,
) -> Result<TimelineFollowOutputRenderDecision, String> {
    let outgoing_output = timeline_follow_output_config(&follow.outgoing_video, output_id)?;
    let incoming_output = timeline_follow_output_config(&follow.incoming_video, output_id)?;
    if outgoing_output.width != incoming_output.width
        || outgoing_output.height != incoming_output.height
    {
        return Err(format!(
            "Timeline Follow output {output_id} dimensions diverged: outgoing {}x{}, incoming {}x{}",
            outgoing_output.width,
            outgoing_output.height,
            incoming_output.width,
            incoming_output.height
        ));
    }
    let key = TimelineFollowOutputFrameKey {
        epoch: follow.epoch,
        generation: follow.generation,
        output_id,
        width: outgoing_output.width,
        height: outgoing_output.height,
    };
    state.observe_key(key);
    if !outgoing_output.enabled || !incoming_output.enabled {
        return Ok(TimelineFollowOutputRenderDecision::NotApplicable {
            key,
            reason: format!("Timeline Follow output {output_id} is disabled"),
        });
    }
    if video::VideoOutputMappingIdentity::from_mapping(&outgoing_output.mapping)
        != video::VideoOutputMappingIdentity::from_mapping(&incoming_output.mapping)
    {
        return Err(format!(
            "Timeline Follow output {output_id} mapping diverged between outgoing and incoming snapshots"
        ));
    }

    let context = video::VideoEffectRenderContext {
        clip_runtime: &engine_snapshot.video_clip_runtime,
        project_render_epoch: follow.epoch,
    };
    let outgoing = renderer
        .prepare_output_artistic_render_result_preview_with_effects_and_transitions(
            &follow.outgoing_video,
            context,
            &engine_snapshot.video_transition_runtime,
            output_id,
            outgoing_output.width,
            outgoing_output.height,
        )
        .map_err(|error| format!("Timeline Follow outgoing output render failed: {error:?}"))?;
    let incoming = renderer
        .prepare_output_artistic_render_result_preview_with_effects_and_transitions(
            &follow.incoming_video,
            context,
            &engine_snapshot.video_transition_runtime,
            output_id,
            incoming_output.width,
            incoming_output.height,
        )
        .map_err(|error| format!("Timeline Follow incoming output render failed: {error:?}"))?;

    let output_contract = video::VideoOutputPresentationContract::new(
        follow.epoch,
        output_id,
        outgoing_output.width,
        outgoing_output.height,
        &outgoing_output.mapping,
    );
    let outgoing_is_blackout = outgoing.is_hard_blackout();
    let incoming_is_blackout = incoming.is_hard_blackout();
    output_contract
        .admit(&outgoing)
        .map_err(|error| format!("Timeline Follow outgoing output is not sendable: {error:?}"))?;
    output_contract
        .admit(&incoming)
        .map_err(|error| format!("Timeline Follow incoming output is not sendable: {error:?}"))?;

    let opaque_black = || -> Result<video::VideoFrame, String> {
        let len = (outgoing_output.width as usize)
            .checked_mul(outgoing_output.height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| {
                format!(
                    "Timeline Follow output {output_id} frame byte length overflow for {}x{}",
                    outgoing_output.width, outgoing_output.height
                )
            })?;
        let mut data = vec![0_u8; len];
        for pixel in data.chunks_exact_mut(4) {
            pixel[3] = u8::MAX;
        }
        Ok(video::VideoFrame {
            layer_id: 0,
            width: outgoing_output.width,
            height: outgoing_output.height,
            pts_ms: 0,
            duration_ms: 0,
            format: video::VideoPixelFormat::Rgba8,
            data,
        })
    };
    let outgoing_frame = match outgoing.payload {
        video::VideoOutputArtisticPayload::Frame(frame) => frame,
        video::VideoOutputArtisticPayload::HardBlackout => opaque_black()?,
    };
    let incoming_frame = match incoming.payload {
        video::VideoOutputArtisticPayload::Frame(frame) => frame,
        video::VideoOutputArtisticPayload::HardBlackout => opaque_black()?,
    };

    let transition_scope = protocol::VideoEffectScope::Transition {
        owner: protocol::VideoTransitionEffectOwner::TimelineFollow {
            source_timeline_id: follow.source_timeline_id,
        },
    };
    let transition_chain = follow
        .transition_effect_chain
        .as_ref()
        .map(|chain| {
            if chain.scope != transition_scope {
                return Err(format!(
                    "Timeline Follow transition chain {:?} has the wrong owner",
                    chain.id
                ));
            }
            let mut catalog = protocol::VideoSnapshot::default();
            catalog.effect_chains.push(chain.clone());
            video::resolve_video_effect_chain(&catalog, &transition_scope)
                .map_err(|fault| {
                    format!(
                        "Timeline Follow transition chain is invalid: {}",
                        fault.message
                    )
                })?
                .ok_or_else(|| "Timeline Follow transition chain did not resolve".to_string())
        })
        .transpose()?;

    let combined = renderer
        .render_follow_output_transition_rgba8(video::VideoFollowOutputTransitionRequest {
            outgoing: &outgoing_frame,
            incoming: &incoming_frame,
            kind: follow.kind,
            curve: follow.curve,
            progress_millis: follow.progress_millis,
            source_timeline_id: follow.source_timeline_id,
            transition_chain: transition_chain.as_ref(),
            last_valid: state.last_valid(key),
        })
        .map_err(|error| format!("Timeline Follow output combine failed: {error:?}"))?;
    if combined.evidence.freshness != video::VideoOutputRenderFreshness::Fresh {
        return Err(combined.evidence.error.clone().unwrap_or_else(|| {
            format!(
                "Timeline Follow transition used {:?} render evidence",
                combined.evidence.freshness
            )
        }));
    }
    let artistic = video::VideoOutputArtisticRenderResult {
        payload: if outgoing_is_blackout && incoming_is_blackout {
            video::VideoOutputArtisticPayload::HardBlackout
        } else {
            video::VideoOutputArtisticPayload::Frame(combined.frame.clone())
        },
        output_mapping: outgoing_output.mapping.clone(),
        output_mapping_identity: video::VideoOutputMappingIdentity::from_mapping(
            &outgoing_output.mapping,
        ),
        evidence: video::VideoOutputRenderEvidence {
            project_render_epoch: follow.epoch,
            output_id,
            freshness: video::VideoOutputRenderFreshness::Fresh,
            error: None,
        },
    };
    let presentation =
        video::materialize_output_artistic_rgba8_for_transport(artistic, &output_contract)
            .map_err(|error| {
                format!("Timeline Follow output {output_id} presentation rejected: {error:?}")
            })?;
    if !presentation.is_hard_blackout() {
        state.update_last_valid(key, combined.frame.clone());
    }
    Ok(TimelineFollowOutputRenderDecision::Frame {
        key,
        frame: presentation.into_frame(),
        result: TimelineFollowSettlementAckResult::Applied,
        follow_identity: Some(timeline_follow_active_identity(follow)),
    })
}

pub(super) fn timeline_follow_output_key(
    follow: &TimelineFollowVideoRenderSnapshot,
    output_id: VideoOutputId,
) -> Result<TimelineFollowOutputFrameKey, String> {
    let output = follow
        .outgoing_video
        .outputs
        .iter()
        .find(|output| output.id == output_id)
        .ok_or_else(|| format!("Timeline Follow output {output_id} was not found"))?;
    Ok(TimelineFollowOutputFrameKey {
        epoch: follow.epoch,
        generation: follow.generation,
        output_id,
        width: output.width,
        height: output.height,
    })
}

pub(super) fn timeline_follow_result_after_physical_send(
    rendered: TimelineFollowSettlementAckResult,
    send_error: Option<&str>,
    backend: &str,
    output_id: VideoOutputId,
) -> TimelineFollowSettlementAckResult {
    match send_error {
        Some(error) => TimelineFollowSettlementAckResult::Fault {
            fault: format!("{backend} output route {output_id} send failed: {error}"),
        },
        None => rendered,
    }
}

pub(super) fn publish_timeline_follow_output_result(
    engine: &EngineHandle,
    state: &mut TimelineFollowOutputState,
    key: TimelineFollowOutputFrameKey,
    result: TimelineFollowSettlementAckResult,
) -> Result<bool, String> {
    let now = Instant::now();
    state.queue_ack(key, result, now);
    state.try_publish_ack(now, |ack| {
        engine.acknowledge_timeline_follow_settlement_published(
            ack.clone(),
            Instant::now() + TIMELINE_FOLLOW_SETTLEMENT_ACK_TTL,
        )
    })
}

pub(super) fn publish_timeline_follow_output_result_until_resolved(
    engine: &EngineHandle,
    state: &mut TimelineFollowOutputState,
    key: TimelineFollowOutputFrameKey,
    result: TimelineFollowSettlementAckResult,
    should_stop: impl FnMut() -> bool,
) {
    publish_timeline_follow_output_result_until_resolved_with(
        engine,
        state,
        key,
        result,
        should_stop,
        |ack| {
            engine.acknowledge_timeline_follow_settlement_published(
                ack.clone(),
                Instant::now() + TIMELINE_FOLLOW_SETTLEMENT_ACK_TTL,
            )
        },
    );
}

/// An issued acknowledgement may have settled Follow even if the caller lost
/// its reply. Replays are exact and bounded after that terminal transition.
pub(super) fn publish_timeline_follow_output_result_until_resolved_with(
    engine: &EngineHandle,
    state: &mut TimelineFollowOutputState,
    key: TimelineFollowOutputFrameKey,
    result: TimelineFollowSettlementAckResult,
    mut should_stop: impl FnMut() -> bool,
    mut publish: impl FnMut(&TimelineFollowSettlementAck) -> Result<(), String>,
) {
    let mut issued_publish = false;
    let mut terminal_receipt_retries = 0;
    loop {
        let now = Instant::now();
        state.queue_ack(key, result.clone(), now);
        let _ = state.try_publish_ack(now, |ack| {
            issued_publish = true;
            publish(ack)
        });
        if state.published_for(key) || should_stop() {
            return;
        }
        if engine.output_ownership_status().epoch != key.epoch {
            return;
        }
        match engine.timeline_follow_video_render_snapshot() {
            Some(follow) if follow.generation == key.generation => {}
            Some(_) => return,
            None if issued_publish
                && terminal_receipt_retries < TIMELINE_FOLLOW_TERMINAL_RECEIPT_RETRIES =>
            {
                terminal_receipt_retries += 1;
            }
            None => return,
        }
        std::thread::sleep(TIMELINE_FOLLOW_SETTLEMENT_ACK_RETRY);
    }
}

/// Close output admission before an acknowledgement retry can block. The
/// caller retains the returned lease through its physical teardown path.
pub(super) fn fence_timeline_follow_fault_before_ack(
    engine: &EngineHandle,
    state: &mut TimelineFollowOutputState,
    key: TimelineFollowOutputFrameKey,
    fault: String,
) -> (OutputOwnershipTeardownLease, TimelineFollowOutputFrameKey) {
    let failure_lease = engine.begin_output_ownership_failure_fence(fault.clone());
    // Failure fencing advances ownership epoch; engine acknowledgement must
    // use that epoch while retaining the original Follow generation/output.
    let failed_key = TimelineFollowOutputFrameKey {
        epoch: engine.output_ownership_status().epoch,
        ..key
    };
    state.queue_ack(
        failed_key,
        TimelineFollowSettlementAckResult::Fault { fault },
        Instant::now(),
    );
    (failure_lease, failed_key)
}

/// Convert one expired bounded-settlement deadline into the exact visible
/// outcome: an ownership failure fence plus the queued Fault acknowledgement.
/// The returned fault string must become the worker's terminal error so the
/// expiry is never silently dropped.
pub(super) fn fence_timeline_follow_settlement_deadline(
    engine: &EngineHandle,
    state: &mut TimelineFollowOutputState,
    key: TimelineFollowOutputFrameKey,
    reason: &str,
) -> (
    OutputOwnershipTeardownLease,
    TimelineFollowOutputFrameKey,
    String,
) {
    let fault = format!(
        "Timeline Follow {}:{} output {} bounded settlement deadline ({:?}) exceeded: {}",
        key.epoch,
        key.generation,
        key.output_id,
        state.settlement_deadline(),
        reason
    );
    let (failure_lease, failed_key) =
        fence_timeline_follow_fault_before_ack(engine, state, key, fault.clone());
    (failure_lease, failed_key, fault)
}

#[derive(Default)]
pub(super) struct StartupFailureFenceState {
    epoch: Option<u64>,
    lease: Option<OutputOwnershipTeardownLease>,
}

pub(super) type StartupFailureLeaseSlot = Arc<Mutex<StartupFailureFenceState>>;

pub(super) fn ensure_startup_failure_fence(
    engine: &EngineHandle,
    slot: &StartupFailureLeaseSlot,
    reason: impl Into<String>,
) -> u64 {
    let mut state = match slot.lock() {
        Ok(state) => state,
        Err(poisoned) => poisoned.into_inner(),
    };
    if state.epoch.is_none() {
        state.lease = Some(engine.begin_output_ownership_failure_fence(reason));
        state.epoch = Some(engine.output_ownership_status().epoch);
    }
    state.epoch.expect("startup failure fence epoch must exist")
}

pub(super) fn take_startup_failure_lease(
    slot: &StartupFailureLeaseSlot,
) -> Option<OutputOwnershipTeardownLease> {
    match slot.lock() {
        Ok(mut state) => state.lease.take(),
        Err(poisoned) => poisoned.into_inner().lease.take(),
    }
}

pub(super) fn fence_timeline_follow_fault_before_ack_in_startup(
    engine: &EngineHandle,
    state: &mut TimelineFollowOutputState,
    key: TimelineFollowOutputFrameKey,
    fault: String,
    slot: &StartupFailureLeaseSlot,
) -> TimelineFollowOutputFrameKey {
    let failed_key = TimelineFollowOutputFrameKey {
        epoch: ensure_startup_failure_fence(engine, slot, fault.clone()),
        ..key
    };
    state.queue_ack(
        failed_key,
        TimelineFollowSettlementAckResult::Fault { fault },
        Instant::now(),
    );
    failed_key
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::EngineCommand;

    #[test]
    fn constants_are_ordered_and_bounded() {
        assert!(TIMELINE_FOLLOW_SETTLEMENT_ACK_RETRY < TIMELINE_FOLLOW_SETTLEMENT_ACK_TTL);
        assert!(TIMELINE_FOLLOW_SETTLEMENT_DEADLINE > TIMELINE_FOLLOW_SETTLEMENT_ACK_TTL);
        assert!(VIDEO_OUTPUT_ADMISSION_DEADLINE >= TIMELINE_FOLLOW_SETTLEMENT_ACK_TTL);
        assert!(TIMELINE_FOLLOW_TERMINAL_RECEIPT_RETRIES > 0);
    }

    #[test]
    fn watchdog_expiry_is_bounded_visible_and_single_shot_per_generation() {
        let key = TimelineFollowOutputFrameKey {
            epoch: 3,
            generation: 9,
            output_id: 12,
            width: 4,
            height: 2,
        };
        let next_generation = TimelineFollowOutputFrameKey {
            generation: 10,
            ..key
        };
        let start = Instant::now();
        let mut state = TimelineFollowOutputState::default();
        state.set_settlement_deadline(Duration::from_millis(30));

        assert!(
            !state.tick_follow_unresolved(key, start, "first retry"),
            "the first observation only arms the clock"
        );
        assert!(
            !state.tick_follow_unresolved(key, start + Duration::from_millis(29), "still retrying"),
            "before the deadline the worker keeps its bounded retry policy"
        );
        assert!(
            state.tick_follow_unresolved(key, start + Duration::from_millis(31), "stuck"),
            "the deadline must expire visibly"
        );
        assert!(
            !state.tick_follow_unresolved(key, start + Duration::from_millis(32), "after fire"),
            "expiry fires once; the caller faults and stops instead of refiring"
        );

        // A new generation restarts the clock instead of inheriting expiry.
        assert!(
            !state.tick_follow_unresolved(
                next_generation,
                start + Duration::from_millis(40),
                "new generation"
            ),
            "a successor generation must get its own bounded window"
        );

        // An admitted send clears tracking entirely; the next failure arms a
        // fresh full window rather than expiring immediately.
        state.note_follow_admitted();
        assert!(!state.tick_follow_unresolved(
            next_generation,
            start + Duration::from_millis(80),
            "fresh window after admitted send"
        ));
    }

    #[test]
    fn retry_decision_carries_its_watchdog_frame_key() {
        let key = TimelineFollowOutputFrameKey {
            epoch: 5,
            generation: 7,
            output_id: 13,
            width: 8,
            height: 4,
        };
        let decision = TimelineFollowOutputRenderDecision::Retry {
            reason: "superseded sample".to_string(),
            key: Some(key),
        };
        match decision {
            TimelineFollowOutputRenderDecision::Retry {
                reason,
                key: Some(watchdog_key),
            } => {
                assert_eq!(watchdog_key, key);
                assert_eq!(reason, "superseded sample");
            }
            _ => panic!("the Retry decision must preserve its watchdog key"),
        }
    }

    /// One live settling Follow presenter with a Hold fault policy. The long
    /// follow duration keeps the presenter settled-pending for the whole test
    /// so deadline behavior is observable without racing completion.
    fn settling_follow_engine(
        output_id: VideoOutputId,
        source_timeline_id: protocol::TimelineId,
        target_timeline_id: protocol::TimelineId,
        audio_clip_id: u64,
        follow_duration_ms: u64,
    ) -> (EngineHandle, TimelineFollowOutputFrameKey) {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let transition = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        transition.complete().unwrap();
        let output = protocol::VideoOutputSummary {
            id: output_id,
            label: "Shared Fence Output".to_string(),
            kind: protocol::VideoOutputKind::Display,
            enabled: true,
            composition_id: 1,
            fullscreen: false,
            monitor_id: None,
            monitor_identity: None,
            width: 4,
            height: 2,
            endpoint_name: None,
            opacity: 1.0,
            blackout: false,
            mapping: protocol::VideoOutputMapping::default(),
        };
        engine.send(EngineCommand::AddVideoOutput(output)).unwrap();

        let audio_layer = protocol::TimelineLayerSummary {
            id: 2,
            label: "Audio".to_string(),
            order: 0,
            muted: false,
            locked: false,
            solo: false,
            expanded: false,
            kind: protocol::TimelineLayerKind::Audio,
        };
        let mut source = protocol::TimelineSnapshot {
            id: source_timeline_id,
            label: "Shared fence Follow source".to_string(),
            layers: vec![audio_layer.clone()],
            audio_clips: vec![protocol::TimelineAudioClipSummary {
                id: audio_clip_id,
                layer_id: 2,
                media_asset_id: None,
                path: format!("memory://shared-fence-{audio_clip_id}.wav"),
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 100,
                gain: 1.0,
                fade_in_ms: 0,
                fade_out_ms: 0,
                output_bus: protocol::TimelineAudioOutputBus::Program,
            }],
            duration_ms: 100,
            ..protocol::TimelineSnapshot::default()
        };
        source.follow = Some(protocol::TimelineFollowSummary {
            enabled: true,
            next_timeline_id: target_timeline_id,
            duration: protocol::VideoClipTakeDuration::milliseconds(follow_duration_ms),
            curve: protocol::VideoLayerTransitionCurve::Linear,
            video_kind: protocol::VideoClipTakeKind::Crossfade,
            lighting_policy: protocol::TimelineFollowLightingPolicy::LinearMerge,
            destination_bpm: None,
            preroll_ms: 0,
            trans_cadence_bars: 4,
            trans_target_measures: Vec::new(),
            hold_first_destination_measure: false,
            fault_policy: protocol::TimelineFollowFaultPolicy::Hold,
        });
        let target = protocol::TimelineSnapshot {
            id: target_timeline_id,
            label: "Shared fence Follow target".to_string(),
            layers: vec![audio_layer],
            duration_ms: 100,
            ..protocol::TimelineSnapshot::default()
        };
        engine
            .apply_timeline_bank_published(vec![source, target], source_timeline_id, false)
            .unwrap();
        engine.send(EngineCommand::SeekTimeline(90)).unwrap();
        engine
            .send(EngineCommand::SetTimelinePlaying(true))
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(2);
        let follow = loop {
            if let Some(follow) = engine.timeline_follow_video_render_snapshot() {
                if matches!(
                    engine.timeline_follow_runtime_status(follow.epoch).status,
                    protocol::TimelineFollowRuntimeStatus::Settling
                ) {
                    break follow;
                }
            }
            assert!(
                Instant::now() < deadline,
                "shared fence Follow never entered settlement"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        let key = timeline_follow_output_key(&follow, output_id).unwrap();
        assert_eq!(key.epoch, engine.output_ownership_status().epoch);
        (engine, key)
    }

    #[test]
    fn settlement_deadline_faults_the_hold_visibly_through_the_engine() {
        let output_id = 96_401;
        let (engine, key) = settling_follow_engine(
            output_id,
            protocol::TimelineId(96_402),
            protocol::TimelineId(96_403),
            96_404,
            10,
        );
        let mut state = TimelineFollowOutputState::default();
        state.set_settlement_deadline(Duration::from_millis(50));
        assert!(
            !state.tick_follow_unresolved(key, Instant::now(), "injected persistent retry"),
            "arming tick must not fire"
        );
        std::thread::sleep(Duration::from_millis(60));
        assert!(
            state.tick_follow_unresolved(key, Instant::now(), "injected persistent retry"),
            "an unadmitted generation beyond the deadline must expire"
        );

        let (failure_lease, failed_key, fault) = fence_timeline_follow_settlement_deadline(
            &engine,
            &mut state,
            key,
            "injected persistent retry",
        );
        assert!(
            fault.contains("bounded settlement deadline"),
            "the expiry must name its cause: {fault}"
        );
        assert_ne!(failed_key.epoch, key.epoch);
        publish_timeline_follow_output_result_until_resolved(
            &engine,
            &mut state,
            failed_key,
            TimelineFollowSettlementAckResult::Fault { fault },
            || false,
        );
        assert!(state.published_for(failed_key));
        let runtime = engine.snapshot().timeline.follow_runtime;
        assert_eq!(
            runtime.status,
            protocol::TimelineFollowRuntimeStatus::Held,
            "the expired hold must resolve into a visible Held outcome"
        );
        assert!(matches!(
            runtime.outcome,
            Some(protocol::TimelineFollowOutcome::Held)
        ));
        drop(failure_lease);
    }

    #[test]
    fn single_shot_publish_queues_and_publishes_one_exact_ack() {
        let output_id = 96_405;
        let (engine, key) = settling_follow_engine(
            output_id,
            protocol::TimelineId(96_406),
            protocol::TimelineId(96_407),
            96_408,
            10,
        );
        let mut state = TimelineFollowOutputState::default();
        let published = publish_timeline_follow_output_result(
            &engine,
            &mut state,
            key,
            TimelineFollowSettlementAckResult::Applied,
        )
        .expect("a live presenter accepts the exact video Applied acknowledgement");
        assert!(published);
        assert!(state.published_for(key));
        assert!(state.pending_ack.is_none());
        assert_eq!(
            engine.snapshot().timeline.follow_runtime.status,
            protocol::TimelineFollowRuntimeStatus::Settling,
            "one consumer acknowledgement never invents overall success"
        );
    }
}
