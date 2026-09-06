use crate::{EngineHandle, TimelineTransportAuthority};
use protocol::{
    EngineSnapshot, TimelineFollowRuntimeStatusSnapshot, TimelineFollowRuntimeSummary,
    VideoClipRuntimeSnapshot, VideoLayerTransitionRuntimeSnapshot, VideoOutputSummary,
};

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

    /// Read the published Timeline playing flag without cloning unrelated
    /// public project and runtime collections.
    pub fn timeline_playing(&self) -> bool {
        self.read_snapshot_field(|snapshot| snapshot.timeline.playing)
    }
}
