//! Program monitor sampling is issued and fenced by the engine. Expired windows
//! return None so the caller waits for a canonical tick without decoding.
use engine::{EngineHandle, VideoRenderSample};

pub(super) fn capture_video_monitor_snapshot(
    engine: &EngineHandle,
) -> Result<Option<VideoRenderSample>, String> {
    engine.capture_video_render_sample()
}
