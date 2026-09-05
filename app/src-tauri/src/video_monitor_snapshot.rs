//! Owned Program-monitor inputs captured under one engine snapshot read lock.
//! This is deliberately not an incomplete EngineSnapshot: no lighting, cue-bank,
//! Timeline-bank or persistence payload is copied into the monitor worker.

use engine::EngineHandle;
use protocol::{
    EngineSnapshot, VideoClipRuntimeSnapshot, VideoLayerTransitionRuntimeSnapshot, VideoSnapshot,
};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct VideoMonitorSnapshot {
    pub(super) project_render_epoch: u64,
    pub(super) bpm: f32,
    pub(super) video: VideoSnapshot,
    pub(super) clip_runtime: VideoClipRuntimeSnapshot,
    pub(super) transition_runtime: VideoLayerTransitionRuntimeSnapshot,
}

fn project(snapshot: &EngineSnapshot, project_render_epoch: u64) -> VideoMonitorSnapshot {
    VideoMonitorSnapshot {
        project_render_epoch,
        bpm: snapshot.clock.bpm,
        video: snapshot.video.clone(),
        clip_runtime: snapshot.video_clip_runtime.clone(),
        transition_runtime: snapshot.video_transition_runtime.clone(),
    }
}

pub(super) fn capture_video_monitor_snapshot(
    engine: &EngineHandle,
) -> Result<VideoMonitorSnapshot, String> {
    // Preserve the existing ownership-before-snapshot capture order. The
    // caller must still reject a retired epoch before publishing its frame.
    let epoch = engine.output_ownership_status().epoch;
    engine.inspect_snapshot(|snapshot| project(snapshot, epoch))
}

#[cfg(test)]
mod tests {
    use super::{project, VideoMonitorSnapshot};
    use protocol::{
        EngineSnapshot, VideoClipLayerRuntimeSummary, VideoClipSlotId, VideoClipTakeDuration,
        VideoClipTakeKind, VideoLayerTransitionBusRuntimeSummary, VideoLayerTransitionCurve,
        VideoLayerTransitionTarget, VideoTransitionBusId,
    };

    #[test]
    fn monitor_projection_preserves_render_fields_and_omits_unrelated_state() {
        let mut source = EngineSnapshot::default();
        source.clock.bpm = 137.0;
        source.video.master_opacity = 0.375;
        source.video.blackout = true;
        source.video_clip_runtime.layers.push(VideoClipLayerRuntimeSummary {
            layer_id: 71,
            active_slot_id: Some(VideoClipSlotId(19)),
            playhead_ms: 1234,
            ..VideoClipLayerRuntimeSummary::default()
        });
        source.video_clip_runtime.timeline_video_projection_layer_ids = vec![71];
        let from = VideoLayerTransitionTarget::Layer { layer_id: 71 };
        source.video_transition_runtime.buses.push(VideoLayerTransitionBusRuntimeSummary {
            bus_id: VideoTransitionBusId(5),
            origin_from: from.clone(),
            from,
            to: VideoLayerTransitionTarget::Layer { layer_id: 72 },
            kind: VideoClipTakeKind::Custom,
            curve: VideoLayerTransitionCurve::EaseIn,
            elapsed_ms: 250,
            duration_ms: 1000,
            duration: VideoClipTakeDuration::milliseconds(1000),
            progress_millis: 250,
        });
        let projected = project(&source, 83);
        // Exhaustive construction also fixes the focused type's field boundary.
        assert_eq!(projected, VideoMonitorSnapshot {
            project_render_epoch: 83,
            bpm: 137.0,
            video: source.video.clone(),
            clip_runtime: source.video_clip_runtime.clone(),
            transition_runtime: source.video_transition_runtime.clone(),
        });
        source.playback_master = 0.25;
        source.blackout = !source.blackout;
        source.timeline.position_ms = 9999;
        assert_eq!(project(&source, 83), projected);
        source.video.master_opacity = 0.5;
        assert_eq!(projected.video.master_opacity, 0.375);
    }
}
