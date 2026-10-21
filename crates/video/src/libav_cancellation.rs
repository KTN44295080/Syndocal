//! Decoder-owned cancellation and FFmpeg interrupt userdata ownership.
use super::VideoDecodeError;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Clone)]
pub(super) struct StopState(Arc<StopInner>);

struct StopInner {
    retiring: AtomicBool,
    external: Option<Arc<AtomicBool>>,
}

impl StopState {
    pub(super) fn new(external: Option<Arc<AtomicBool>>) -> Self {
        Self(Arc::new(StopInner {
            retiring: AtomicBool::new(false),
            external,
        }))
    }

    pub(super) fn cancel(&self) {
        self.0.retiring.store(true, Ordering::Release);
    }

    pub(super) fn is_cancelled(&self) -> bool {
        self.0.retiring.load(Ordering::Acquire)
            || self
                .0
                .external
                .as_ref()
                .is_some_and(|stop| stop.load(Ordering::Acquire))
    }

    pub(super) fn check(&self) -> Result<(), VideoDecodeError> {
        if self.is_cancelled() {
            Err(VideoDecodeError::Cancelled)
        } else {
            Ok(())
        }
    }
}

#[cfg(feature = "libav")]
mod input {
    use super::super::ffmpeg;
    use super::*;
    use std::{
        ffi::{c_int, c_void, CString},
        ops::{Deref, DerefMut},
        path::Path,
        ptr,
    };

    /// Fields drop in declaration order: close the AVFormatContext before
    /// freeing its interrupt userdata. The Box address stays stable when the
    /// session moves between the decoder and its bounded decode pool.
    pub(in super::super) struct InterruptInput {
        input: ffmpeg::format::context::Input,
        _interrupt: Box<StopState>,
    }

    unsafe extern "C" fn interrupted(opaque: *mut c_void) -> c_int {
        // FFmpeg calls only while the input context is live. Atomic loads
        // cannot panic or call back into the engine / decoder locks.
        let stop = unsafe { &*opaque.cast::<StopState>() };
        c_int::from(stop.is_cancelled())
    }

    impl InterruptInput {
        pub(in super::super) fn open(path: &Path, stop: &StopState) -> Result<Self, ffmpeg::Error> {
            let path = path.to_str().ok_or(ffmpeg::Error::InvalidData)?;
            let path = CString::new(path).map_err(|_| ffmpeg::Error::InvalidData)?;
            let mut interrupt = Box::new(stop.clone());
            // Own the allocation before installing its pointer. Every error
            // closes/frees the native context while this Box is still alive.
            unsafe {
                let mut context = ffmpeg::ffi::avformat_alloc_context();
                if context.is_null() {
                    return Err(ffmpeg::Error::Bug);
                }
                (*context).interrupt_callback = ffmpeg::ffi::AVIOInterruptCB {
                    callback: Some(interrupted),
                    opaque: (&mut *interrupt as *mut StopState).cast(),
                };
                let opened = ffmpeg::ffi::avformat_open_input(
                    &mut context,
                    path.as_ptr(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                );
                if opened < 0 {
                    // FFmpeg normally frees and nulls this pointer on error.
                    // Close a retained context too, without leaking userdata.
                    if !context.is_null() {
                        ffmpeg::ffi::avformat_close_input(&mut context);
                    }
                    return Err(ffmpeg::Error::from(opened));
                }
                let info = ffmpeg::ffi::avformat_find_stream_info(context, ptr::null_mut());
                if info < 0 {
                    ffmpeg::ffi::avformat_close_input(&mut context);
                    return Err(ffmpeg::Error::from(info));
                }
                Ok(Self {
                    input: ffmpeg::format::context::Input::wrap(context),
                    _interrupt: interrupt,
                })
            }
        }
    }

    impl Deref for InterruptInput {
        type Target = ffmpeg::format::context::Input;
        fn deref(&self) -> &Self::Target {
            &self.input
        }
    }

    impl DerefMut for InterruptInput {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.input
        }
    }
}

#[cfg(feature = "libav")]
pub(super) use input::InterruptInput;
