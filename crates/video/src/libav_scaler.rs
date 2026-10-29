//! Owned libswscale conversion of a decoded frame to packed RGBA (C4R12).
//!
//! The conversion writes straight into the output `VideoFrame`'s own buffer:
//! there is no intermediate FFmpeg frame and no per-frame full-size copy.
//! Output-sized conversions on the decode pool run on libswscale's own slice
//! threads, so one source frame no longer occupies a single core for most of a
//! source frame interval. Both changes are bit-exact with the previous
//! `sws_getContext` + `sws_scale` + row-copy path: same scaler, flags and
//! formats, one output row per slice row. In place is used only where the
//! packed row is a multiple of an FFmpeg frame's 64-byte line alignment, so a
//! SIMD output loop that rounds a row up to its vector width never reaches the
//! next row (which another slice thread may be writing); every other width
//! converts into an FFmpeg frame and is copied row by row, as before.

use std::ptr::{null_mut, NonNull};

use ffmpeg::{ffi, format::Pixel, software::scaling::flag::Flags, util::frame::video::Video};
use ffmpeg_next as ffmpeg;

/// Outputs at least this large are converted on `SLICE_THREADS` threads when
/// the scaler is created on a decode-pool worker; smaller conversions
/// (thumbnails, tests) stay single-threaded and own no worker threads.
pub(super) const SLICE_THREAD_MIN_OUTPUT_PIXELS: u64 = 1024 * 1024;
/// Slice threads for an output-sized conversion. Measured on the C4 fixture
/// (1920x1080 NV12 to 2560x1600 RGBA, C4R12 `libav_scaler_tests` bench).
pub(super) const SLICE_THREADS: i32 = 4;
/// libswscale's SIMD paths need 16-byte aligned planes; an FFmpeg-allocated
/// frame always provides that.
const DESTINATION_ALIGNMENT: usize = 16;
/// In-place rows must be whole multiples of an FFmpeg frame's line alignment
/// (`av_frame_get_buffer` pads lines to 32/64 bytes): a packed RGBA row of a
/// multiple of 16 pixels is exactly such a line, with no padding to overrun.
const IN_PLACE_ROW_BYTES: usize = 64;

/// Slice threads for a scaler created on the current thread.
///
/// Only the process-lifetime decode pool's workers get slice threads. A
/// scaler is cached per thread and freed in that thread's TLS destructor;
/// freeing a threaded context joins its workers, and on Windows TLS
/// destructors run under the loader lock, which an exiting worker thread also
/// needs, so a join there deadlocks (a render, Display or test thread that
/// exits). The static decode pool is never dropped, so its workers never run
/// those destructors; every other thread keeps single-threaded contexts, which
/// own no threads and free without joining.
pub(super) fn slice_threads(target_width: u32, target_height: u32) -> i32 {
    let on_decode_pool =
        super::libav_decode_pool().is_some_and(|pool| pool.current_thread_index().is_some());
    if on_decode_pool
        && u64::from(target_width) * u64::from(target_height) >= SLICE_THREAD_MIN_OUTPUT_PIXELS
    {
        SLICE_THREADS
    } else {
        1
    }
}

/// One initialized libswscale context for a fixed source/target geometry.
/// It owns its native pointer (and its slice threads) and is intentionally
/// neither `Send` nor `Sync`, like `ffmpeg_next`'s scaling context.
pub(super) struct RgbaScaler {
    context: NonNull<ffi::SwsContext>,
    source_format: Pixel,
    source_width: u32,
    source_height: u32,
    target_width: u32,
    target_height: u32,
}

impl RgbaScaler {
    /// The `sws_getContext(source, RGBA target, BILINEAR)` conversion, plus
    /// the slice-thread count, initialized through the option API.
    pub(super) fn new(
        source_format: Pixel,
        source_width: u32,
        source_height: u32,
        target_width: u32,
        target_height: u32,
    ) -> Result<Self, ffmpeg::Error> {
        Self::with_threads(
            source_format,
            source_width,
            source_height,
            target_width,
            target_height,
            slice_threads(target_width, target_height),
        )
    }

    fn with_threads(
        source_format: Pixel,
        source_width: u32,
        source_height: u32,
        target_width: u32,
        target_height: u32,
        threads: i32,
    ) -> Result<Self, ffmpeg::Error> {
        // SAFETY: allocation has no preconditions; a null result is handled.
        let context = NonNull::new(unsafe { ffi::sws_alloc_context() })
            .ok_or(ffmpeg::Error::Other { errno: ffi::ENOMEM })?;
        // From here `Drop` frees the context on every exit.
        let scaler = Self {
            context,
            source_format,
            source_width,
            source_height,
            target_width,
            target_height,
        };
        let options: [(&std::ffi::CStr, i64); 8] = [
            (c"srcw", i64::from(source_width)),
            (c"srch", i64::from(source_height)),
            (
                c"src_format",
                ffi::AVPixelFormat::from(source_format) as i64,
            ),
            (c"dstw", i64::from(target_width)),
            (c"dsth", i64::from(target_height)),
            (c"dst_format", ffi::AVPixelFormat::AV_PIX_FMT_RGBA as i64),
            (c"sws_flags", i64::from(Flags::BILINEAR.bits())),
            (c"threads", i64::from(threads)),
        ];
        for (name, value) in options {
            // SAFETY: the context is live and its AVClass names these options.
            let result =
                unsafe { ffi::av_opt_set_int(context.as_ptr().cast(), name.as_ptr(), value, 0) };
            if result < 0 {
                return Err(ffmpeg::Error::from(result));
            }
        }
        // SAFETY: the context is live and fully configured; no filters.
        let result = unsafe { ffi::sws_init_context(context.as_ptr(), null_mut(), null_mut()) };
        if result < 0 {
            return Err(ffmpeg::Error::from(result));
        }
        Ok(scaler)
    }

    /// Slice threads libswscale actually started for this context.
    #[cfg(test)]
    pub(super) fn threads(&self) -> i64 {
        let mut value = 0_i64;
        // SAFETY: the context is live; "threads" is an integer option.
        let result = unsafe {
            ffi::av_opt_get_int(
                self.context.as_ptr().cast(),
                c"threads".as_ptr(),
                0,
                &mut value,
            )
        };
        assert!(result >= 0, "threads option must be readable");
        value
    }

    fn check_source(&self, source: &Video) -> Result<(), ffmpeg::Error> {
        if source.format() != self.source_format
            || source.width() != self.source_width
            || source.height() != self.source_height
        {
            return Err(ffmpeg::Error::InputChanged);
        }
        Ok(())
    }

    fn packed_row_bytes(&self) -> usize {
        self.target_width as usize * 4
    }

    /// Whether a tightly packed RGBA row of the target width is exactly an
    /// FFmpeg-allocated frame's line (no padding a SIMD row loop could use).
    pub(super) fn packed_rows_aligned(&self) -> bool {
        self.packed_row_bytes() % IN_PLACE_ROW_BYTES == 0
    }

    /// Whether `destination` can receive the conversion in place with the
    /// alignment an FFmpeg-allocated frame would have.
    pub(super) fn writes_in_place(&self, destination: &[u8]) -> bool {
        self.packed_rows_aligned() && (destination.as_ptr() as usize) % DESTINATION_ALIGNMENT == 0
    }

    /// Convert `source` into the packed, tightly strided RGBA `destination`.
    /// The caller checks `writes_in_place` first.
    pub(super) fn run_into(
        &mut self,
        source: &Video,
        destination: &mut [u8],
    ) -> Result<(), ffmpeg::Error> {
        self.check_source(source)?;
        let row = self.packed_row_bytes();
        if destination.len() != row * self.target_height as usize
            || !self.writes_in_place(destination)
        {
            return Err(ffmpeg::Error::InvalidData);
        }
        let mut target = Video::empty();
        // SAFETY: `target` is a live, empty AVFrame. Its only buffer reference
        // wraps `destination`, which outlives this call; the no-op free
        // callback never releases Rust-owned memory. libswscale releases its
        // own reference in `sws_frame_end` (called by `sws_scale_frame`, and
        // again below on failure) before this function returns, and dropping
        // `target` releases the last one.
        let data = destination.as_mut_ptr();
        unsafe {
            let buffer = ffi::av_buffer_create(
                data,
                destination.len(),
                Some(borrowed_destination_free),
                null_mut(),
                0,
            );
            if buffer.is_null() {
                return Err(ffmpeg::Error::Other { errno: ffi::ENOMEM });
            }
            let frame = target.as_mut_ptr();
            (*frame).buf[0] = buffer;
            (*frame).data[0] = data;
            (*frame).linesize[0] = row as i32;
            (*frame).width = self.target_width as i32;
            (*frame).height = self.target_height as i32;
            (*frame).format = ffi::AVPixelFormat::AV_PIX_FMT_RGBA as i32;
        }
        self.run_frame(source, &mut target)
    }

    /// Convert `source` into an FFmpeg-allocated RGBA frame (the fallback for
    /// a destination that cannot be written in place).
    pub(super) fn run(&mut self, source: &Video, target: &mut Video) -> Result<(), ffmpeg::Error> {
        self.check_source(source)?;
        // SAFETY: `target` is a live AVFrame; this only reads its data pointer.
        if !unsafe { target.is_empty() } {
            return Err(ffmpeg::Error::InvalidData);
        }
        self.run_frame(source, target)
    }

    fn run_frame(&mut self, source: &Video, target: &mut Video) -> Result<(), ffmpeg::Error> {
        // SAFETY: the context was initialized for exactly this source
        // geometry/format (checked) and an RGBA target of its output size;
        // both frames are live AVFrames.
        let result = unsafe {
            ffi::sws_scale_frame(self.context.as_ptr(), target.as_mut_ptr(), source.as_ptr())
        };
        if result < 0 {
            // A failed start can leave the context holding frame references;
            // release them now so nothing outlives the destination borrow.
            // SAFETY: valid on a live context after any frame start attempt.
            unsafe { ffi::sws_frame_end(self.context.as_ptr()) };
            return Err(ffmpeg::Error::from(result));
        }
        Ok(())
    }
}

impl Drop for RgbaScaler {
    fn drop(&mut self) {
        // SAFETY: the context is owned, live and freed exactly once; this
        // also joins its slice threads.
        unsafe { ffi::sws_freeContext(self.context.as_ptr()) };
    }
}

unsafe extern "C" fn borrowed_destination_free(_opaque: *mut std::ffi::c_void, _data: *mut u8) {}

#[cfg(test)]
#[path = "libav_scaler_tests.rs"]
mod tests;
