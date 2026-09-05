//! Internal render-time sampling. No serialized or authored state is changed.
use super::*;

const VIDEO_SAMPLE_PUBLICATION_MAX_AGE: Duration = DMX_TICK_INTERVAL.saturating_mul(2);
const VIDEO_SAMPLE_DELIVERY_MAX_AGE: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoSamplingMode {
    ContinuousRootTimeline,
    Canonical(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoRenderSampleValidation { Current, DeadlineExpired, SemanticStale }

#[derive(Debug, Clone, PartialEq)]
pub struct VideoRenderSampleFence {
    generation: u64,
    output_epoch: u64,
    deadline: Option<Instant>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoRenderSample {
    pub project_render_epoch: u64,
    pub bpm: f32,
    pub video: VideoSnapshot,
    pub clip_runtime: VideoClipRuntimeSnapshot,
    pub transition_runtime: VideoLayerTransitionRuntimeSnapshot,
    pub mode: VideoSamplingMode,
    pub fence: VideoRenderSampleFence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Projection {
    layer_id: VideoLayerId,
    clip_id: TimelineVideoClipId,
    asset_id: MediaAssetId,
    source_ms: u64,
}

struct Window {
    captured_at: Instant,
    fractional_ns: u32,
    capture_deadline: Instant,
    semantic_deadline: Instant,
    projections: Vec<Projection>,
}

pub(super) struct VideoSamplingPublication {
    generation: u64,
    exhausted: bool,
    identity: Option<(TimelineId, u64, u64, u64)>,
    mode: VideoSamplingMode,
    window: Option<Window>,
}

impl Default for VideoSamplingPublication {
    fn default() -> Self {
        Self { generation: 0, exhausted: false, identity: None,
            mode: VideoSamplingMode::Canonical("not-yet-published"), window: None }
    }
}

impl EngineRuntime {
    /// Called only while the corresponding EngineSnapshot write guard is held.
    /// Reader order is also snapshot -> sampling. A failed/rolled-back command
    /// never calls this hook and therefore does not revoke a valid sample.
    pub(super) fn publish_video_sampling(&self, snapshot: &EngineSnapshot, tick: bool) {
        let (mode, window) = if tick { self.video_sampling_window(snapshot) }
            else { (VideoSamplingMode::Canonical("command-publication"), None) };
        let identity = Some((self.timeline_id, self.timeline_transport_epoch,
            self.timeline_transport_generation, self.output_ownership_gate.status().epoch));
        // A poisoned sidecar remains poisoned. Readers reject it; publishing
        // the ordinary engine snapshot must never revive its sampling permit.
        let Ok(mut published) = self.shared_telemetry.video_render_sampling.write() else { return; };
        let changed_projection = match (&published.window, &window) {
            (Some(old), Some(new)) => old.projections.iter().map(|p| (p.layer_id, p.clip_id, p.asset_id))
                .ne(new.projections.iter().map(|p| (p.layer_id, p.clip_id, p.asset_id)))
                || self.last_tick >= old.capture_deadline,
            (None, None) => false,
            _ => true,
        };
        if !tick || self.video_presentation_config_dirty || identity != published.identity || mode != published.mode || changed_projection {
            match published.generation.checked_add(1) {
                Some(next) => published.generation = next,
                None => published.exhausted = true,
            }
        }
        published.identity = identity;
        published.mode = mode;
        published.window = window;
    }

    fn video_sampling_window(&self, snapshot: &EngineSnapshot) -> (VideoSamplingMode, Option<Window>) {
        let discrete = |reason| (VideoSamplingMode::Canonical(reason), None);
        if !self.timeline_playing { return discrete("paused"); }
        if self.timeline_count_in_until.is_some() { return discrete("count-in"); }
        if self.timeline_external_sync_source.is_some() { return discrete("external-sync"); }
        if self.timeline_loop_runtime.status != TimelineLoopRuntimeStatus::Disabled { return discrete("root-loop"); }
        if self.timeline_follow_transition.is_some() || self.timeline_follow_transport.is_some() {
            return discrete("follow-transition");
        }
        if self.auto_vj.status.armed || snapshot.video_clip_runtime.layers.iter().any(|layer|
            layer.transition.is_some() || layer.pending_launch.is_some() || layer.queued_slot_id.is_some()) {
            return discrete("independent-video-launch");
        }
        // The initial contract advances plain root clip source time only.
        // Lighting-only child transports are permitted; video-affecting state
        // requires a future time-evaluated effect/automation sampling contract.
        if !self.timeline_video_automations.is_empty()
            || self.child_transports.iter().chain(&self.direct_child_transports).chain(&self.nested_child_transports)
                .any(|child| !child.video_automations.is_empty())
            || !snapshot.video.effect_chains.is_empty()
            || !snapshot.video_transition_runtime.buses.is_empty()
            || self.effects.iter().any(|effect| match &effect.kind {
                RuntimeEffectKind::Lfo(effect) => !effect.request.video_targets.is_empty(),
                RuntimeEffectKind::PositionWave(effect) => !effect.video_targets.is_empty(),
                _ => false,
            })
            || self.cues.iter().any(|cue| !cue.video_targets.is_empty() || !cue.video_output_targets.is_empty()) {
            return discrete("video-modulation");
        }
        if !self.video_layer_fades.is_empty() || !self.video_output_fades.is_empty() {
            return discrete("video-fade");
        }
        let projections = self.timeline_video_projection_layers(self.timeline_id, &self.timeline_video_clips,
            &self.timeline_layers, self.timeline_position_ms, true);
        if projections.is_empty() { return discrete("no-explicit-root-video"); }
        let projected_ids: HashSet<_> = projections.iter().map(|p| p.layer.id).collect();
        if snapshot.video.layers.iter().any(|layer| layer.state.enabled && layer.state.playing && !projected_ids.contains(&layer.id)) {
            return discrete("independent-video-transport");
        }
        let position = self.timeline_position_ms;
        let mut horizon = self.timeline_duration_ms();
        // Every root cue boundary is conservative, including a jump's source
        // end. Future overlapping clips revoke the old winner at their start.
        for event in &self.timeline_events {
            for boundary in [event.time_ms, timeline_event_end_ms(event)] {
                if boundary > position { horizon = horizon.min(boundary); }
            }
        }
        if let Some(follow) = self.timeline_follow.as_ref().filter(|follow| follow.enabled) {
            let admission = self.timeline_duration_ms().saturating_sub(follow.preroll_ms);
            if admission <= position { return discrete("follow-admission"); }
            horizon = horizon.min(admission);
        }
        for clip in &self.timeline_video_clips {
            for boundary in [clip.start_ms, clip.start_ms.saturating_add(clip.duration_ms)] {
                if boundary > position { horizon = horizon.min(boundary); }
            }
        }
        let mut rows = Vec::with_capacity(projections.len());
        for projection in projections {
            let Some(clip) = self.timeline_video_clips.iter().find(|clip| clip.id == projection.clip_id) else {
                return discrete("projection-mismatch");
            };
            if clip.fade_in_ms != 0 || clip.fade_out_ms != 0 { return discrete("clip-fade"); }
            let Some(layer) = snapshot.video.layers.iter().find(|layer| layer.id == projection.layer.id) else {
                return discrete("projection-mismatch");
            };
            if layer.media_asset_id != Some(clip.media_asset_id) || layer.state.position_ms != projection.layer.state.position_ms {
                return discrete("projection-mismatch");
            }
            if layer.source.kind != VideoSourceKind::File { return discrete("non-file-source"); }
            let Some(duration) = layer.source.metadata.as_ref().and_then(|metadata| metadata.duration_ms) else {
                return discrete("source-duration-unavailable");
            };
            if layer.state.position_ms >= duration { return discrete("source-ended"); }
            horizon = horizon.min(position.saturating_add(duration - layer.state.position_ms));
            rows.push(Projection { layer_id: layer.id, clip_id: clip.id, asset_id: clip.media_asset_id, source_ms: layer.state.position_ms });
        }
        let fractional_ns = self.timeline_tick_remainder.filter(|carry| carry.timeline_id == self.timeline_id
            && carry.transport_epoch == self.timeline_transport_epoch && carry.transport_generation == self.timeline_transport_generation
            && carry.position_ms == position).map_or(0, |carry| carry.nanoseconds);
        let remaining_ns = u128::from(horizon.saturating_sub(position)) * 1_000_000;
        if remaining_ns <= u128::from(fractional_ns) { return discrete("at-boundary"); }
        let boundary_duration = Duration::from_millis(horizon.saturating_sub(position))
            .checked_sub(Duration::from_nanos(u64::from(fractional_ns)))
            .expect("positive semantic interval checked above");
        let Some(boundary_deadline) = self.last_tick.checked_add(boundary_duration) else {
            return discrete("clock-overflow");
        };
        let Some(freshness_deadline) = self.last_tick.checked_add(VIDEO_SAMPLE_PUBLICATION_MAX_AGE) else {
            return discrete("clock-overflow");
        };
        let capture_deadline = boundary_deadline.min(freshness_deadline);
        (VideoSamplingMode::ContinuousRootTimeline, Some(Window { captured_at: self.last_tick, fractional_ns,
            capture_deadline, semantic_deadline: boundary_deadline, projections: rows }))
    }
}

impl EngineHandle {
    /// None means the last admitted interval expired: wait for a canonical tick.
    /// Errors revoke old displayed frames. Canonical mode never extrapolates.
    pub fn capture_video_render_sample(&self) -> Result<Option<VideoRenderSample>, String> {
        let epoch = self.output_ownership_status().epoch;
        let snapshot = self.snapshot.read().map_err(|_| "Engine snapshot lock was poisoned")?;
        let publication = self.shared_telemetry.video_render_sampling.read().map_err(|_| "Video sample lock was poisoned")?;
        publication.capture(&snapshot, epoch, Instant::now())
    }

    pub fn validate_video_render_sample(&self, fence: &VideoRenderSampleFence) -> Result<VideoRenderSampleValidation, String> {
        let _snapshot = self.snapshot.read().map_err(|_| "Engine snapshot lock was poisoned")?;
        let publication = self.shared_telemetry.video_render_sampling.read().map_err(|_| "Video sample lock was poisoned")?;
        Ok(publication.validate(fence, self.output_ownership_status().epoch, Instant::now()))
    }
}

impl VideoSamplingPublication {
    /// Prepared under the reserved sidecar guard, before an irreversible receipt.
    /// Canonical samples bind the live output epoch when captured; they have no
    /// interpolation anchor that could belong to an earlier output owner.
    pub(super) fn prepare_command_publication(&self) -> Self {
        Self {
            generation: self.generation.saturating_add(1),
            exhausted: self.exhausted || self.generation == u64::MAX,
            identity: None,
            mode: VideoSamplingMode::Canonical("command-publication"),
            window: None,
        }
    }
    fn validate(&self, fence: &VideoRenderSampleFence, epoch: u64, now: Instant) -> VideoRenderSampleValidation {
        if self.exhausted || self.generation != fence.generation || fence.output_epoch != epoch {
            VideoRenderSampleValidation::SemanticStale
        } else if fence.deadline.is_some_and(|deadline| now >= deadline) {
            VideoRenderSampleValidation::DeadlineExpired
        } else { VideoRenderSampleValidation::Current }
    }

    fn capture(&self, snapshot: &EngineSnapshot, epoch: u64, now: Instant) -> Result<Option<VideoRenderSample>, String> {
        if self.exhausted { return Err("Video sample generation exhausted".into()); }
        if self.identity.is_some_and(|(_, _, _, output_epoch)| output_epoch != epoch) {
            return Err("Video sample output ownership changed".into());
        }
        if self.window.as_ref().is_some_and(|window| now >= window.capture_deadline) { return Ok(None); }
        // Capture freshness bounds extrapolation from the canonical tick. Once
        // captured, decoding may finish later, but never beyond a real Timeline
        // boundary or the bounded delivery age. Publications still revoke the
        // fence immediately when its semantic generation or output epoch changes.
        let deadline = self.window.as_ref().map(|window| {
            now.checked_add(VIDEO_SAMPLE_DELIVERY_MAX_AGE)
                .map(|delivery| delivery.min(window.semantic_deadline))
                .ok_or("Video sample delivery clock overflow")
        }).transpose()?;
        let mut video = snapshot.video.clone();
        if let Some(window) = &self.window {
            // Windows checked_duration_since tolerates a backwards QPC epsilon.
            // This contract rejects any future anchor, including that interval.
            if now < window.captured_at { return Err("Video sample clock preceded its publication".into()); }
            let elapsed = now.checked_duration_since(window.captured_at).ok_or("Video sample clock preceded its publication")?;
            let delta_ms = (elapsed.as_nanos() + u128::from(window.fractional_ns)) / 1_000_000;
            let delta_ms = u64::try_from(delta_ms).map_err(|_| "Video sample time overflow")?;
            for row in &window.projections {
                let layer = video.layers.iter_mut().find(|layer| layer.id == row.layer_id).ok_or("Video sample projection disappeared")?;
                layer.state.position_ms = row.source_ms.checked_add(delta_ms).ok_or("Video sample source time overflow")?;
            }
        }
        Ok(Some(VideoRenderSample { project_render_epoch: epoch, bpm: snapshot.clock.bpm, video,
            clip_runtime: snapshot.video_clip_runtime.clone(), transition_runtime: snapshot.video_transition_runtime.clone(),
            mode: self.mode, fence: VideoRenderSampleFence { generation: self.generation, output_epoch: epoch,
                deadline } }))
    }
}

#[cfg(test)]
#[path = "video_render_sample_tests.rs"]
mod tests;
