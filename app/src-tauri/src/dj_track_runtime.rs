//! Strict per-deck DJ Link admission.

use super::*;

pub(super) fn dispatch_active(
    payload: protocol::DjLinkTrackPayload,
    engine: &EngineHandle,
    runtime: &mut DjLinkRuntime,
    current_generation: u64,
    event_id: &str,
    sequence: u64,
) -> DjLinkDispatchOutcome {
    if payload.validate().is_err() {
        return dj_link_rejected("invalid_track_payload", current_generation);
    }
    if !dj_link_track_payload_is_current(&payload) {
        return dj_link_rejected("not_current_playing_track", current_generation);
    }
    let position_ms = match dj_link_estimated_position_ms(&payload) {
        Ok(position_ms) => position_ms,
        Err(code) => return dj_link_rejected(code, current_generation),
    };
    let mapping = match dj_track_selector::resolve_track_mapping(&runtime.mappings, &payload) {
        dj_track_selector::DjTrackMappingResolution::NoMapping => None,
        dj_track_selector::DjTrackMappingResolution::Unique(mapping) => Some(mapping),
        dj_track_selector::DjTrackMappingResolution::Ambiguous => {
            return dj_link_rejected("track_mapping_ambiguous", current_generation)
        }
    };
    let released_same_owner = runtime.track_active
        && runtime.released
        && runtime.track_deck_number == Some(payload.deck)
        && runtime.track_deck_id.as_deref() == Some(payload.deck_id.as_str())
        && runtime.play_session_id.as_deref() == Some(payload.play_session_id.as_str());
    if released_same_owner {
        // RELEASE retires Timeline control but not the admitted receipt's
        // identity/mapping fence. A same deck/session reannounce may only
        // observe that receipt; it cannot relaunch another track or mapping.
        let Some(mapping) = mapping.as_ref() else {
            return dj_link_rejected("released_track_reannounce_mismatch", current_generation);
        };
        let dedupe_key = format!(
            "{}:{}:{}:{}:{}",
            runtime.project_epoch,
            mapping.id,
            payload.deck,
            payload.deck_id,
            payload.play_session_id
        );
        if !dj_link_track_identity_matches_runtime(runtime, &payload)
            || runtime.active_dedupe_key.as_deref() != Some(dedupe_key.as_str())
        {
            return dj_link_rejected("released_track_reannounce_mismatch", current_generation);
        }
        // Release is terminal for this admitted play session. The no-op must
        // not depend on the expiring replay cache: once the exact owner,
        // identity, and mapping receipt match, a later ACTIVE can only observe
        // that terminal receipt and can never reach the engine again.
        return dj_link_accepted(current_generation);
    }
    let Some(mapping) = mapping else {
        if runtime.track_active
            && runtime.play_session_id.is_some()
            && runtime.timeline_id.is_some()
        {
            return DjLinkDispatchOutcome::NoMapping {
                state_generation: current_generation,
            };
        }
        runtime.unmapped_active_blocked = true;
        return DjLinkDispatchOutcome::NoMapping {
            state_generation: current_generation,
        };
    };
    if runtime.pending_operator_return_request_id.is_some() {
        if runtime.track_active
            || runtime.play_session_id.is_some()
            || runtime.pedal_owner.is_some()
            || runtime.release_event_id.is_some()
            || runtime.authoritative_state != protocol::DjLinkTimelineStateValue::Running
        {
            return dj_link_rejected("operator_return_runtime_invalid", current_generation);
        }
        let expected_timeline_id = mapping.timeline_id.0.to_string();
        if runtime.timeline_id.as_deref() != Some(expected_timeline_id.as_str()) {
            return dj_link_rejected("operator_return_mapping_mismatch", current_generation);
        }
        let Some(snapshot) = engine.try_snapshot() else {
            return DjLinkDispatchOutcome::Busy {
                code: "engine_snapshot_busy".to_string(),
                state_generation: current_generation,
            };
        };
        if !snapshot.timeline.playing || snapshot.timeline.id != mapping.timeline_id {
            return dj_link_rejected("operator_return_timeline_mismatch", current_generation);
        }
        let dedupe_key = format!(
            "{}:{}:{}:{}:{}",
            runtime.project_epoch,
            mapping.id,
            payload.deck,
            payload.deck_id,
            payload.play_session_id
        );
        if !runtime.seen_play_sessions.contains_key(&dedupe_key)
            && runtime.seen_play_sessions.len() >= DJ_LINK_DEDUPE_LIMIT
        {
            return dj_link_rejected("play_session_capacity", current_generation);
        }
        let next_generation = match dj_link_next_generation(runtime) {
            Ok(next) => next,
            Err(_) => return dj_link_rejected("state_generation_exhausted", current_generation),
        };
        let mut next_runtime = runtime.clone();
        next_runtime
            .seen_play_sessions
            .insert(dedupe_key.clone(), Instant::now());
        next_runtime.active_dedupe_key = Some(dedupe_key);
        next_runtime.track_active = true;
        next_runtime.playing = true;
        next_runtime.released = false;
        next_runtime.unmapped_active_blocked = false;
        next_runtime.track_content_id = payload.content_id;
        next_runtime.track_title = payload.title;
        next_runtime.track_artist = payload.artist;
        next_runtime.track_deck_id = Some(payload.deck_id);
        next_runtime.track_deck_number = Some(payload.deck);
        next_runtime.track_started_at = Some(payload.started_at);
        next_runtime.track_playing = payload.is_playing;
        next_runtime.track_bpm = payload.track_bpm;
        next_runtime.position_sec = Some(payload.position_at_send_sec);
        next_runtime.source_position_ms = Some(position_ms);
        next_runtime.position_revision = Some(payload.position_revision);
        next_runtime.play_session_id = Some(payload.play_session_id);
        next_runtime.pedal_owner = Some("dj".to_string());
        next_runtime.release_event_id = None;
        next_runtime.pending_operator_return_request_id = None;
        next_runtime.position_bars = dj_link_engine_position_bars(&snapshot);
        next_runtime.loop_active = !matches!(
            snapshot.timeline.loop_runtime.status,
            protocol::TimelineLoopRuntimeStatus::Disabled
        );
        next_runtime.last_event_id = Some(event_id.to_string());
        next_runtime.state_generation = next_generation;
        *runtime = next_runtime;
        return DjLinkDispatchOutcome::TimelineState {
            state_generation: next_generation,
            state: dj_link_timeline_state_from_snapshot(runtime, &snapshot, event_id, sequence),
        };
    }
    let dedupe_key = format!(
        "{}:{}:{}:{}:{}",
        runtime.project_epoch, mapping.id, payload.deck, payload.deck_id, payload.play_session_id
    );
    if runtime.track_active && !runtime.released {
        let same_owner = runtime.track_deck_number == Some(payload.deck)
            && runtime.track_deck_id.as_deref() == Some(payload.deck_id.as_str())
            && runtime.play_session_id.as_deref() == Some(payload.play_session_id.as_str())
            && runtime.active_dedupe_key.as_deref() == Some(dedupe_key.as_str());
        if !same_owner {
            return dj_link_rejected("track_owner_unreleased", current_generation);
        }
    }
    match dj_link_position_revision_disposition(
        runtime,
        &payload.play_session_id,
        payload.position_revision,
        position_ms,
    ) {
        Ok(DjLinkPositionRevisionDisposition::ExactDuplicate) => {
            return dj_link_accepted(current_generation)
        }
        Ok(DjLinkPositionRevisionDisposition::NewerOrNewSession) => {}
        Err(code) => return dj_link_rejected(code, current_generation),
    }
    let mut next_runtime = runtime.clone();
    next_runtime.purge_dedupe(Instant::now());
    next_runtime.track_active = true;
    next_runtime.playing = true;
    next_runtime.track_content_id = payload.content_id.clone();
    next_runtime.track_title = payload.title.clone();
    next_runtime.track_artist = payload.artist.clone();
    next_runtime.track_deck_id = Some(payload.deck_id.clone());
    next_runtime.track_deck_number = Some(payload.deck);
    next_runtime.track_started_at = Some(payload.started_at.clone());
    next_runtime.track_playing = payload.is_playing;
    next_runtime.track_bpm = payload.track_bpm;
    next_runtime.position_sec = Some(payload.position_at_send_sec);
    next_runtime.source_position_ms = Some(position_ms);
    next_runtime.position_revision = Some(payload.position_revision);
    next_runtime.play_session_id = Some(payload.play_session_id.clone());
    if next_runtime.seen_play_sessions.contains_key(&dedupe_key) {
        return dj_link_accepted(current_generation);
    }
    if next_runtime.seen_play_sessions.len() >= DJ_LINK_DEDUPE_LIMIT {
        return dj_link_rejected("play_session_capacity", current_generation);
    }
    let next = match dj_link_next_generation(&next_runtime) {
        Ok(next) => next,
        Err(_) => return dj_link_rejected("state_generation_exhausted", current_generation),
    };
    let snapshot = match engine
        .dj_link_start_timeline_at_with_canonical_snapshot(mapping.timeline_id, position_ms)
    {
        Ok(snapshot) => snapshot,
        Err(_) => return dj_link_rejected("engine_publication_rejected", current_generation),
    };
    next_runtime
        .seen_play_sessions
        .insert(dedupe_key.clone(), Instant::now());
    next_runtime.active_dedupe_key = Some(dedupe_key);
    next_runtime.released = false;
    next_runtime.unmapped_active_blocked = false;
    next_runtime.pedal_owner = Some("dj".to_string());
    next_runtime.release_event_id = None;
    next_runtime.authoritative_state = protocol::DjLinkTimelineStateValue::Running;
    next_runtime.timeline_id = Some(mapping.timeline_id.0.to_string());
    next_runtime.position_bars = dj_link_engine_position_bars(&snapshot);
    next_runtime.loop_division = None;
    next_runtime.loop_revision = None;
    next_runtime.last_loop_fallback_intent_id = None;
    next_runtime.loop_active = false;
    next_runtime.last_follow_rebase = None;
    next_runtime.last_event_id = Some(event_id.to_string());
    next_runtime.state_generation = next;
    *runtime = next_runtime;
    DjLinkDispatchOutcome::TimelineState {
        state_generation: next,
        state: dj_link_timeline_state_from_snapshot(runtime, &snapshot, event_id, sequence),
    }
}

pub(super) fn dispatch_sync(
    payload: protocol::DjLinkTrackPayload,
    engine: &EngineHandle,
    runtime: &mut DjLinkRuntime,
    current_generation: u64,
    event_id: &str,
    sequence: u64,
) -> DjLinkDispatchOutcome {
    if payload.validate().is_err() {
        return dj_link_rejected("invalid_track_sync_payload", current_generation);
    }
    if runtime.unmapped_active_blocked {
        return dj_link_rejected("dj_link_unmapped_active", current_generation);
    }
    if runtime.released {
        return dj_link_rejected("dj_link_released", current_generation);
    }
    if !dj_link_track_payload_is_current(&payload)
        || !runtime.track_active
        || runtime.track_deck_number != Some(payload.deck)
        || runtime.track_deck_id.as_deref() != Some(payload.deck_id.as_str())
        || runtime.play_session_id.as_deref() != Some(payload.play_session_id.as_str())
        || !dj_link_track_identity_matches_runtime(runtime, &payload)
    {
        return dj_link_rejected("track_sync_context_mismatch", current_generation);
    }
    let position_ms = match dj_link_estimated_position_ms(&payload) {
        Ok(position_ms) => position_ms,
        Err(code) => return dj_link_rejected(code, current_generation),
    };
    match dj_link_position_revision_disposition(
        runtime,
        &payload.play_session_id,
        payload.position_revision,
        position_ms,
    ) {
        Ok(DjLinkPositionRevisionDisposition::NewerOrNewSession) => {}
        Ok(DjLinkPositionRevisionDisposition::ExactDuplicate) => {
            return dj_link_accepted(current_generation)
        }
        Err(code) => return dj_link_rejected(code, current_generation),
    }
    let Some(timeline_id) = runtime
        .timeline_id
        .as_deref()
        .and_then(|value| value.parse::<u64>().ok())
        .map(TimelineId)
    else {
        return dj_link_rejected("invalid_timeline_id", current_generation);
    };
    let snapshot = match engine
        .dj_link_sync_timeline_position_with_canonical_snapshot(timeline_id, position_ms)
    {
        Ok(snapshot) => snapshot,
        Err(_) => return dj_link_rejected("engine_publication_rejected", current_generation),
    };
    runtime.track_content_id = payload.content_id;
    runtime.track_title = payload.title;
    runtime.track_artist = payload.artist;
    runtime.track_bpm = payload.track_bpm;
    runtime.track_playing = payload.is_playing;
    runtime.position_sec = Some(payload.position_at_send_sec);
    runtime.source_position_ms = Some(position_ms);
    runtime.position_revision = Some(payload.position_revision);
    runtime.track_started_at = Some(payload.started_at);
    runtime.position_bars = dj_link_engine_position_bars(&snapshot);
    runtime.last_event_id = Some(event_id.to_string());
    DjLinkDispatchOutcome::TimelineState {
        state_generation: current_generation,
        state: dj_link_timeline_state_from_snapshot(runtime, &snapshot, event_id, sequence),
    }
}

pub(super) fn owner_matches(
    runtime: &DjLinkRuntime,
    deck: u8,
    deck_id: &str,
    play_session_id: &str,
) -> bool {
    runtime.track_active
        && runtime.track_deck_number == Some(deck)
        && runtime.track_deck_id.as_deref() == Some(deck_id)
        && runtime.play_session_id.as_deref() == Some(play_session_id)
}

/// STATE_SYNC is diagnostic and never establishes ownership. While one
/// operator-return request is pending, a replacement Agent is allowed to
/// describe its current candidate tuple before the subsequent TRACK_ACTIVE;
/// that later message remains the sole admission path. Outside that bounded
/// window, an existing runtime owner must match exactly.
pub(super) fn state_sync_owner_context_rejection(
    runtime: &DjLinkRuntime,
    payload: &protocol::DjLinkTrackStateSyncPayload,
) -> Option<&'static str> {
    let (Some(deck), Some(deck_id), Some(play_session_id)) = (
        payload.owner_deck,
        payload.owner_deck_id.as_deref(),
        payload.active_play_session_id.as_deref(),
    ) else {
        return None;
    };
    if runtime.pending_operator_return_request_id.is_some() {
        return None;
    }
    let runtime_has_owner =
        runtime.track_active || runtime.timeline_id.is_some() || runtime.play_session_id.is_some();
    if runtime_has_owner && !owner_matches(runtime, deck, deck_id, play_session_id) {
        return Some("state_sync_owner_context_mismatch");
    }
    None
}

/// Rebase the released Stage 2 runtime to a destination only after the engine
/// has published the exact terminal Follow completion.  The release receipt
/// and pedal owner remain the original physical-session fence; the rebase
/// never creates a new ownership or accepts an abort/fault/stale image.
pub(super) fn rebase_released_follow_completion(
    runtime: &mut DjLinkRuntime,
    snapshot: &EngineSnapshot,
    expected_timeline_id: Option<&str>,
    expected_play_session_id: Option<&str>,
) -> bool {
    let follow = &snapshot.timeline.follow_runtime;
    let (Some(source_timeline_id), Some(target_timeline_id)) =
        (follow.source_timeline_id, follow.target_timeline_id)
    else {
        return false;
    };
    let source = source_timeline_id.0.to_string();
    let target = target_timeline_id.0.to_string();
    if !runtime.released
        || !runtime.track_active
        || runtime.authoritative_state != protocol::DjLinkTimelineStateValue::Running
        || runtime.pedal_owner.as_deref() != Some("timeline")
        || runtime
            .release_event_id
            .as_deref()
            .is_none_or(str::is_empty)
        || runtime.play_session_id.as_deref().is_none_or(str::is_empty)
        || follow.status != protocol::TimelineFollowRuntimeStatus::Idle
        || follow.outcome != Some(protocol::TimelineFollowOutcome::Completed)
        || follow.generation == 0
        || snapshot.timeline.id != target_timeline_id
        || runtime.timeline_id.as_deref() != Some(source.as_str())
    {
        return false;
    }
    if expected_timeline_id.is_some_and(|expected| expected != target)
        || expected_play_session_id
            .is_some_and(|expected| runtime.play_session_id.as_deref() != Some(expected))
    {
        return false;
    }
    let receipt = (follow.generation, source, target);
    if runtime.last_follow_rebase.as_ref() == Some(&receipt) {
        return false;
    }
    let Ok(next_generation) = dj_link_next_generation(runtime) else {
        return false;
    };
    runtime.timeline_id = Some(receipt.2.clone());
    runtime.position_bars = dj_link_engine_position_bars(snapshot);
    runtime.loop_active = !matches!(
        snapshot.timeline.loop_runtime.status,
        protocol::TimelineLoopRuntimeStatus::Disabled
    );
    runtime.last_follow_rebase = Some(receipt);
    runtime.state_generation = next_generation;
    true
}

/// Return the fail-closed reason for a post-release Timeline command, if the
/// runtime has not established the exact Stage 2 authority fence yet.
///
/// Stage 1 `DJ_RELEASE` is the handoff edge: it turns the DJ loop off and
/// gives the already-running Timeline clock back to Syndocal.  Beat jumps and
/// absolute loop requests are therefore only valid after that edge, while the
/// authoritative running snapshot, Timeline pedal owner, and release receipt
/// all remain present.  Keep the identity checks in this same helper so a
/// command can never pass the authority gate with a stale Timeline/session.
pub(super) fn stage2_authority_rejection(
    runtime: &DjLinkRuntime,
    timeline_id: &str,
    play_session_id: &str,
    snapshot: &EngineSnapshot,
) -> Option<&'static str> {
    if runtime.unmapped_active_blocked {
        return Some("dj_link_unmapped_active");
    }
    if !runtime.released {
        return Some("timeline_stage2_not_authorized");
    }
    if !runtime.track_active || runtime.timeline_id.is_none() {
        return Some("timeline_not_active");
    }
    if runtime.authoritative_state != protocol::DjLinkTimelineStateValue::Running {
        return Some("timeline_state_not_running");
    }
    if !snapshot.timeline.playing {
        return Some("timeline_not_playing");
    }
    if matches!(
        snapshot.timeline.follow_runtime.status,
        protocol::TimelineFollowRuntimeStatus::Transitioning
            | protocol::TimelineFollowRuntimeStatus::Settling
    ) {
        // A Follow completion is the only legal rebase boundary. Before it,
        // F13 must not turn off the source loop or relinquish its clock.
        return Some("timeline_follow_settling");
    }
    if snapshot.timeline.id.0.to_string() != timeline_id {
        return Some("timeline_identity_mismatch");
    }
    if runtime.pedal_owner.as_deref() != Some("timeline") {
        return Some("timeline_pedal_owner_mismatch");
    }
    if runtime
        .release_event_id
        .as_deref()
        .map(str::is_empty)
        .unwrap_or(true)
    {
        return Some("timeline_release_missing");
    }
    if runtime.timeline_id.as_deref() != Some(timeline_id) {
        return Some("timeline_identity_mismatch");
    }
    if runtime.play_session_id.as_deref() != Some(play_session_id) {
        return Some("timeline_play_session_mismatch");
    }
    None
}

/// Validate an absolute Stage 2 loop request against both authoritative
/// projections. A pedal edge is a state transition, not an idempotent setter:
/// a stale loop-off must never be admitted after the loop has already been
/// released, and app/engine disagreement fails closed before enqueue.
pub(super) fn timeline_loop_set_authority_rejection(
    runtime: &DjLinkRuntime,
    snapshot: &EngineSnapshot,
    requested_active: bool,
) -> Option<&'static str> {
    let engine_loop_active = !matches!(
        snapshot.timeline.loop_runtime.status,
        protocol::TimelineLoopRuntimeStatus::Disabled
    );
    if runtime.loop_active != engine_loop_active {
        return Some("timeline_loop_state_mismatch");
    }
    if requested_active == engine_loop_active {
        return Some("timeline_loop_state_unchanged");
    }
    None
}

/// F14 has no desired-state bit: it is valid only when both projections agree
/// that the current runtime loop is active. Bounds remain engine-owned and are
/// rechecked there immediately before the acknowledged mutation.
pub(super) fn timeline_loop_half_authority_rejection(
    runtime: &DjLinkRuntime,
    snapshot: &EngineSnapshot,
) -> Option<&'static str> {
    let engine_loop_active = !matches!(
        snapshot.timeline.loop_runtime.status,
        protocol::TimelineLoopRuntimeStatus::Disabled
    );
    if runtime.loop_active != engine_loop_active {
        return Some("timeline_loop_state_mismatch");
    }
    if !engine_loop_active {
        return Some("timeline_loop_inactive");
    }
    None
}

/// Return the fail-closed reason for a correlated DJ Link RELEASE before it
/// may enqueue the engine mutation.  A fresh RELEASE must still be owned by
/// the DJ and must describe the currently active/running play session.  An
/// already released runtime is a separate, terminal replay path: it may only
/// be observed when the prior handoff left the exact Timeline-owned receipt
/// state intact, and it never reaches the engine again.
pub(super) fn release_authority_rejection(
    runtime: &DjLinkRuntime,
    timeline_id: &str,
    play_session_id: &str,
) -> Option<&'static str> {
    if runtime.unmapped_active_blocked {
        return Some("dj_link_unmapped_active");
    }
    if !runtime.track_active || runtime.timeline_id.is_none() {
        return Some("timeline_not_active");
    }
    if runtime.authoritative_state != protocol::DjLinkTimelineStateValue::Running {
        return Some("timeline_state_not_running");
    }
    if runtime.timeline_id.as_deref() != Some(timeline_id) {
        return Some("release_context_mismatch");
    }
    if runtime.play_session_id.as_deref() != Some(play_session_id) {
        return Some("release_context_mismatch");
    }
    if runtime.released {
        if runtime.pedal_owner.as_deref() != Some("timeline") {
            return Some("timeline_pedal_owner_mismatch");
        }
        if runtime.loop_active {
            return Some("timeline_release_state_invalid");
        }
        if runtime
            .release_event_id
            .as_deref()
            .map(str::is_empty)
            .unwrap_or(true)
        {
            return Some("timeline_release_missing");
        }
    } else {
        if runtime.pedal_owner.as_deref() != Some("dj") {
            return Some("dj_link_pedal_owner_mismatch");
        }
        if runtime.release_event_id.is_some() {
            return Some("release_state_invalid");
        }
    }
    None
}

pub(super) fn dispatch_loop_state(
    payload: protocol::DjLinkTrackLoopStatePayload,
    engine: &EngineHandle,
    runtime: &mut DjLinkRuntime,
    current_generation: u64,
    event_id: &str,
) -> DjLinkDispatchOutcome {
    if payload.validate().is_err() {
        return dj_link_rejected("invalid_track_loop_state_payload", current_generation);
    }
    if runtime.unmapped_active_blocked {
        return dj_link_rejected("dj_link_unmapped_active", current_generation);
    }
    if runtime.released {
        return dj_link_rejected("dj_link_released", current_generation);
    }
    if !owner_matches(
        runtime,
        payload.deck,
        &payload.deck_id,
        &payload.play_session_id,
    ) {
        return dj_link_rejected("track_loop_context_mismatch", current_generation);
    }
    if !payload.loop_state.active && runtime.loop_division.is_none() {
        if let Some(current_revision) = runtime.loop_revision {
            if payload.loop_state.revision < current_revision {
                return dj_link_rejected("stale_loop_revision", current_generation);
            }
            if payload.loop_state.revision == current_revision {
                return if !runtime.loop_active {
                    dj_link_accepted(current_generation)
                } else {
                    dj_link_rejected("loop_revision_conflict", current_generation)
                };
            }
        }
        let next = match dj_link_next_generation(runtime) {
            Ok(next) => next,
            Err(_) => return dj_link_rejected("state_generation_exhausted", current_generation),
        };
        // An initial measured no-loop state is meaningful authority for the
        // first bounded F14 fallback, but requires no engine mutation.
        runtime.loop_revision = Some(payload.loop_state.revision);
        runtime.loop_active = false;
        runtime.last_event_id = Some(event_id.to_string());
        runtime.state_generation = next;
        return dj_link_accepted(next);
    }
    let division = match dj_link_measured_loop_division(&payload.loop_state) {
        Ok(Some(division)) => division,
        Ok(None) => match runtime.loop_division {
            Some(division) => division,
            None => {
                return dj_link_rejected("inactive_loop_without_active_context", current_generation)
            }
        },
        Err(code) => return dj_link_rejected(code, current_generation),
    };
    if let Some(current_revision) = runtime.loop_revision {
        if payload.loop_state.revision < current_revision {
            return dj_link_rejected("stale_loop_revision", current_generation);
        }
        if payload.loop_state.revision == current_revision {
            let exact_duplicate = runtime.loop_active == payload.loop_state.active
                && (!payload.loop_state.active || runtime.loop_division == Some(division));
            return if exact_duplicate {
                dj_link_accepted(current_generation)
            } else {
                dj_link_rejected("loop_revision_conflict", current_generation)
            };
        }
    }
    let next = match dj_link_next_generation(runtime) {
        Ok(next) => next,
        Err(_) => return dj_link_rejected("state_generation_exhausted", current_generation),
    };
    let Some(timeline_id) = runtime
        .timeline_id
        .as_deref()
        .and_then(|value| value.parse::<u64>().ok())
        .map(TimelineId)
    else {
        return dj_link_rejected("invalid_timeline_id", current_generation);
    };
    if engine
        .dj_link_set_timeline_loop_absolute(timeline_id, division, payload.loop_state.active)
        .is_err()
    {
        return dj_link_rejected("engine_publication_rejected", current_generation);
    }
    runtime.loop_division = Some(division);
    runtime.loop_revision = Some(payload.loop_state.revision);
    runtime.loop_active = payload.loop_state.active;
    runtime.last_event_id = Some(event_id.to_string());
    runtime.state_generation = next;
    dj_link_accepted(next)
}

pub(super) fn dispatch_loop_fallback(
    payload: protocol::DjLinkTrackLoopFallbackPayload,
    engine: &EngineHandle,
    runtime: &mut DjLinkRuntime,
    current_generation: u64,
    event_id: &str,
) -> DjLinkDispatchOutcome {
    if payload.validate().is_err() {
        return dj_link_rejected("invalid_track_loop_fallback_payload", current_generation);
    }
    if runtime.unmapped_active_blocked {
        return dj_link_rejected("dj_link_unmapped_active", current_generation);
    }
    if runtime.released {
        return dj_link_rejected("dj_link_released", current_generation);
    }
    if !owner_matches(
        runtime,
        payload.deck,
        &payload.deck_id,
        &payload.play_session_id,
    ) {
        return dj_link_rejected("track_loop_fallback_context_mismatch", current_generation);
    }
    if runtime
        .last_loop_fallback_intent_id
        .is_some_and(|last| payload.pedal_intent_id <= last)
    {
        return dj_link_rejected("loop_fallback_intent_not_new", current_generation);
    }
    if runtime.loop_revision != payload.base_measured_loop_revision {
        return dj_link_rejected("loop_fallback_base_revision_mismatch", current_generation);
    }
    let effective_base_loop_division = runtime
        .loop_active
        .then_some(runtime.loop_division)
        .flatten();
    if effective_base_loop_division != payload.base_loop_division {
        return dj_link_rejected("loop_fallback_base_division_mismatch", current_generation);
    }
    let division = match dj_loop_range::division_for_profile_length(payload.target_length_beats) {
        Ok(division) => division,
        Err(code) => return dj_link_rejected(code, current_generation),
    };
    let expected_division = payload
        .base_loop_division
        .map(|base| {
            base.saturating_add(1)
                .min((protocol::DJ_LINK_LOOP_PROFILE_LENGTH_BEATS.len() - 1) as u8)
        })
        .unwrap_or(0);
    if division != expected_division {
        return dj_link_rejected("loop_fallback_target_not_next", current_generation);
    }
    let next = match dj_link_next_generation(runtime) {
        Ok(next) => next,
        Err(_) => return dj_link_rejected("state_generation_exhausted", current_generation),
    };
    let Some(timeline_id) = runtime
        .timeline_id
        .as_deref()
        .and_then(|value| value.parse::<u64>().ok())
        .map(TimelineId)
    else {
        return dj_link_rejected("invalid_timeline_id", current_generation);
    };
    if runtime.loop_active && runtime.loop_division == Some(division) {
        runtime.last_loop_fallback_intent_id = Some(payload.pedal_intent_id);
        runtime.last_event_id = Some(event_id.to_string());
        runtime.state_generation = next;
        return dj_link_accepted(next);
    }
    if engine
        .dj_link_set_timeline_loop_absolute(timeline_id, division, true)
        .is_err()
    {
        return dj_link_rejected("engine_publication_rejected", current_generation);
    }
    runtime.loop_division = Some(division);
    runtime.loop_active = true;
    runtime.last_loop_fallback_intent_id = Some(payload.pedal_intent_id);
    runtime.last_event_id = Some(event_id.to_string());
    runtime.state_generation = next;
    dj_link_accepted(next)
}
