use crate::{EngineHandle, TimelineTransportAuthority};
use protocol::{
    EngineSnapshot, TimelineFollowRuntimeStatus, TimelineFollowRuntimeStatusSnapshot,
    TimelineFollowRuntimeSummary, TimelineLoopRuntimeStatus, VideoClipRuntimeSnapshot,
    VideoLayerTransitionBusSummary, VideoLayerTransitionRuntimeSnapshot, VideoOutputSummary,
};

/// The runtime-only projection required by the control-plane query adapter.
/// Keeping this projection under one snapshot read preserves the existing
/// publication consistency while avoiding a deep copy of authored/project
/// collections on every query capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlPlaneRuntimeSnapshot {
    pub timeline_transport_epoch: u64,
    pub timeline_transport_generation: u64,
    pub timeline_transport_active: bool,
    pub timeline_follow_generation: u64,
    pub timeline_follow_active: bool,
    pub timeline_loop_generation: u64,
    pub timeline_loop_active: bool,
    pub video_clip_slots_active: bool,
    pub video_transitions_active: bool,
}

impl EngineHandle {
    // Keep the blocking/public snapshot reader's poison behavior and derive
    // transport tokens from the same Timeline defaults as that public reader.
    fn read_snapshot_field<T>(&self, read: impl FnOnce(&EngineSnapshot) -> T) -> T {
        match self.snapshot.read() {
            Ok(snapshot) => read(&snapshot),
            Err(_) => read(&EngineSnapshot::default()),
        }
    }

    /// The exact engine-owned fence used by the local runtime control plane.
    /// It is non-persistent and becomes visible atomically with the snapshot.
    pub fn timeline_transport_generation(&self) -> u64 {
        self.read_snapshot_field(|snapshot| snapshot.timeline.transport_generation)
    }

    /// Capture the epoch and generation from one shared publication guard.
    pub fn timeline_transport_authority(&self) -> TimelineTransportAuthority {
        self.read_snapshot_field(|snapshot| TimelineTransportAuthority {
            epoch: snapshot.timeline.transport_epoch,
            generation: snapshot.timeline.transport_generation,
        })
    }

    /// Return public Follow status stamped with the caller's authority epoch.
    pub fn timeline_follow_runtime_status(
        &self,
        epoch: u64,
    ) -> TimelineFollowRuntimeStatusSnapshot {
        self.read_snapshot_field(|snapshot| {
            TimelineFollowRuntimeStatusSnapshot::from_runtime(
                epoch,
                &snapshot.timeline.follow_runtime,
            )
        })
    }

    /// Read runtime-only Follow state without cloning unrelated authored data.
    pub fn timeline_follow_runtime_summary(&self) -> TimelineFollowRuntimeSummary {
        self.read_snapshot_field(|snapshot| snapshot.timeline.follow_runtime.clone())
    }

    /// Read the runtime-only fields needed by the control-plane query source
    /// capture without cloning the public project snapshot.
    pub fn control_plane_runtime_snapshot(&self) -> ControlPlaneRuntimeSnapshot {
        self.read_snapshot_field(|snapshot| ControlPlaneRuntimeSnapshot {
            timeline_transport_epoch: snapshot.timeline.transport_epoch,
            timeline_transport_generation: snapshot.timeline.transport_generation,
            timeline_transport_active: snapshot.timeline.playing,
            timeline_follow_generation: snapshot.timeline.follow_runtime.generation,
            timeline_follow_active: snapshot.timeline.follow_runtime.status
                != TimelineFollowRuntimeStatus::Idle,
            timeline_loop_generation: snapshot.timeline.loop_runtime.generation,
            timeline_loop_active: snapshot.timeline.loop_runtime.status
                != TimelineLoopRuntimeStatus::Disabled,
            video_clip_slots_active: snapshot.video_clip_runtime.layers.iter().any(|layer| {
                layer.playing
                    || layer.active_slot_id.is_some()
                    || layer.pending_launch.is_some()
                    || layer.transition.is_some()
            }),
            video_transitions_active: !snapshot.video_transition_runtime.buses.is_empty(),
        })
    }

    /// Read Clip Slot transport from the latest complete engine publication.
    pub fn video_clip_runtime_snapshot(&self) -> VideoClipRuntimeSnapshot {
        self.read_snapshot_field(|snapshot| snapshot.video_clip_runtime.clone())
    }

    /// Read Layer Transition Bus state from the latest complete publication.
    pub fn video_layer_transition_runtime_snapshot(&self) -> VideoLayerTransitionRuntimeSnapshot {
        self.read_snapshot_field(|snapshot| snapshot.video_transition_runtime.clone())
    }

    /// Read public/rendered video output summaries without cloning unrelated
    /// project and runtime collections.
    pub fn video_outputs_snapshot(&self) -> Vec<VideoOutputSummary> {
        self.read_snapshot_field(|snapshot| snapshot.video.outputs.clone())
    }

    /// Read authored video transition-bus definitions without cloning the
    /// unrelated public project and runtime collections.
    pub fn video_transition_buses_snapshot(&self) -> Vec<VideoLayerTransitionBusSummary> {
        self.read_snapshot_field(|snapshot| snapshot.video.transition_buses.clone())
    }

    /// Read the published Timeline playing flag without cloning unrelated
    /// public project and runtime collections.
    pub fn timeline_playing(&self) -> bool {
        self.read_snapshot_field(|snapshot| snapshot.timeline.playing)
    }
}
