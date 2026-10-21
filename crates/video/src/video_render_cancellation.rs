use super::{VideoDecodeError, VideoFrameProviderError, VideoPreviewError, VideoRuntimeError};

impl VideoPreviewError {
    pub fn is_cancelled(&self) -> bool {
        match self {
            Self::Cancelled | Self::Runtime(VideoRuntimeError::Cancelled)
            | Self::Decode { error: VideoDecodeError::Cancelled, .. } => true,
            Self::RenderInput { error, .. } => matches!(error,
                VideoFrameProviderError::Cancelled
                | VideoFrameProviderError::Decode { error: VideoDecodeError::Cancelled, .. }),
            _ => false,
        }
    }
}

/// Cooperative cancellation supplied by owners that must stop a render
/// without detaching the renderer or abandoning decoder ownership.
pub trait VideoRenderCancellation {
    fn is_cancelled(&self) -> bool;
}

impl<F> VideoRenderCancellation for F
where
    F: Fn() -> bool,
{
    fn is_cancelled(&self) -> bool {
        self()
    }
}

pub(crate) fn check_render_cancellation(
    cancellation: Option<&dyn VideoRenderCancellation>,
) -> Result<(), VideoPreviewError> {
    if cancellation.is_some_and(VideoRenderCancellation::is_cancelled) {
        Err(VideoPreviewError::Cancelled)
    } else {
        Ok(())
    }
}

pub(crate) fn check_runtime_cancellation(
    cancellation: Option<&dyn VideoRenderCancellation>,
) -> Result<(), VideoRuntimeError> {
    if cancellation.is_some_and(VideoRenderCancellation::is_cancelled) {
        Err(VideoRuntimeError::Cancelled)
    } else {
        Ok(())
    }
}
