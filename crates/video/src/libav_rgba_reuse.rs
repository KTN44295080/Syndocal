//! Exclusive hand-back of a displaced decoder frame's packed allocation.
//!
//! Resident replay history pins eight generations of immutable inputs. Keep
//! at most that many displaced candidates and retry them at cache replacement,
//! without waiting. Only a sole owner can return its pixel allocation.
use std::sync::Arc;

use super::{VideoFrame, VideoPixelFormat};

#[derive(Debug)]
pub(super) struct ReturnedRgba {
    extent: (u32, u32),
    data: Vec<u8>,
}

impl ReturnedRgba {
    #[cfg(test)]
    pub(super) fn reclaim(frame: Arc<VideoFrame>, extent: (u32, u32)) -> Option<Self> {
        Self::reclaim_or_return(frame, extent).ok()
    }

    fn reclaim_or_return(
        frame: Arc<VideoFrame>,
        extent: (u32, u32),
    ) -> Result<Self, Arc<VideoFrame>> {
        if !compatible(&frame, extent) {
            return Err(frame);
        }
        let frame = Arc::try_unwrap(frame)?;
        Ok(Self {
            extent,
            data: frame.data,
        })
    }

    pub(super) fn from_buffer(data: Vec<u8>, extent: (u32, u32)) -> Self {
        Self { extent, data }
    }

    pub(super) fn into_buffer(self, extent: (u32, u32), len: usize) -> Option<Vec<u8>> {
        (self.extent == extent && self.data.len() == len && self.data.capacity() == len)
            .then_some(self.data)
    }
}

fn compatible(frame: &VideoFrame, extent: (u32, u32)) -> bool {
    let Some(len) = packed_len(extent) else {
        return false;
    };
    frame.format == VideoPixelFormat::Rgba8
        && (frame.width, frame.height) == extent
        && frame.data.len() == len
        && frame.data.capacity() == len
}

/// The resident replay horizon is eight inputs. Reap before inserting the
/// current displacement, after the prior output's history retirement. Extra
/// external readers may outlive this bound; overflow simply releases its Arc.
#[derive(Clone, Default)]
pub(super) struct RetiredRgba {
    frames: [Option<Arc<VideoFrame>>; 8],
}

impl RetiredRgba {
    pub(super) fn retire(
        &mut self,
        frame: Arc<VideoFrame>,
        extent: (u32, u32),
    ) -> Option<ReturnedRgba> {
        let mut returned = None;
        for slot in &mut self.frames {
            if slot
                .as_ref()
                .is_some_and(|frame| !compatible(frame, extent))
            {
                *slot = None;
            }
            if returned.is_none()
                && slot
                    .as_ref()
                    .is_some_and(|frame| Arc::strong_count(frame) == 1)
            {
                match ReturnedRgba::reclaim_or_return(slot.take().unwrap(), extent) {
                    Ok(buffer) => returned = Some(buffer),
                    Err(frame) => *slot = Some(frame),
                }
            }
        }
        if !compatible(&frame, extent)
            || self
                .frames
                .iter()
                .flatten()
                .any(|old| Arc::ptr_eq(old, &frame))
        {
            return returned;
        }
        // Prefer an immediately exclusive current frame if no older candidate
        // was ready; retain shared frames only within the fixed return horizon.
        let frame = if returned.is_none() {
            match ReturnedRgba::reclaim_or_return(frame, extent) {
                Ok(buffer) => return Some(buffer),
                Err(frame) => frame,
            }
        } else {
            frame
        };
        if let Some(slot) = self.frames.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(frame);
        }
        returned
    }
}

fn packed_len((width, height): (u32, u32)) -> Option<usize> {
    (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)
}

/// A compatible returned buffer already has the exact initialized length.
/// The scaler or row-copy path overwrites every packed output byte.
pub(super) fn destination(
    spare: &mut Option<ReturnedRgba>,
    extent: (u32, u32),
    len: usize,
) -> Vec<u8> {
    spare
        .take()
        .and_then(|spare| spare.into_buffer(extent, len))
        .unwrap_or_else(|| vec![0; len])
}
