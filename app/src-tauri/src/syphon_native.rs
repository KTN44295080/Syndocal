//! Thread-affine macOS Syphon/Metal owner; no frame is submitted during creation.

#![cfg(all(target_os = "macos", feature = "syphon"))]

use std::ffi::{c_char, c_void, CStr, CString};
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::rc::Rc;

const ERROR_CAPACITY: usize = 512;

unsafe extern "C" {
    fn syndocal_syphon_create(
        name: *const c_char,
        error: *mut c_char,
        capacity: usize,
    ) -> *mut c_void;
    fn syndocal_syphon_send_rgba(
        handle: *mut c_void,
        width: u32,
        height: u32,
        rgba: *const u8,
        length: usize,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
    fn syndocal_syphon_close(handle: *mut c_void, error: *mut c_char, capacity: usize) -> i32;
}

pub(crate) struct SyphonSender {
    handle: Option<NonNull<c_void>>,
    // The owning worker creates, publishes, drains and destroys on one thread.
    _thread_affinity: PhantomData<Rc<()>>,
}

fn native_error(buffer: &[c_char; ERROR_CAPACITY], fallback: &str) -> String {
    // The fixed buffer starts zeroed and the C ABI always uses bounded snprintf.
    let detail = unsafe { CStr::from_ptr(buffer.as_ptr()) }.to_string_lossy();
    if detail.is_empty() {
        fallback.to_owned()
    } else {
        detail.into_owned()
    }
}

impl SyphonSender {
    pub(crate) fn new(name: &str) -> Result<Self, String> {
        if name.trim().is_empty() || name.len() > 255 || name.chars().any(char::is_control) {
            return Err(
                "Syphon server name must contain 1..255 UTF-8 bytes and no control characters"
                    .to_owned(),
            );
        }
        let name = CString::new(name).map_err(|_| "Syphon server name contains NUL".to_owned())?;
        let mut error = [0; ERROR_CAPACITY];
        // The native side copies the name and returns an independently owned
        // retained object; the pointer never leaves this thread-affine wrapper.
        let handle =
            unsafe { syndocal_syphon_create(name.as_ptr(), error.as_mut_ptr(), error.len()) };
        Ok(Self {
            handle: Some(
                NonNull::new(handle)
                    .ok_or_else(|| native_error(&error, "Could not initialize Syphon output"))?,
            ),
            _thread_affinity: PhantomData,
        })
    }

    pub(crate) fn send_rgba(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
        let handle = self
            .handle
            .ok_or_else(|| "Syphon output is closed".to_owned())?;
        if !(1..=8192).contains(&width)
            || !(1..=8192).contains(&height)
            || (width as usize)
                .checked_mul(height as usize)
                .and_then(|n| n.checked_mul(4))
                != Some(rgba.len())
        {
            return Err(
                "Syphon frame must contain tightly packed RGBA8 pixels (1..8192 per side)"
                    .to_owned(),
            );
        }
        let mut error = [0; ERROR_CAPACITY];
        // Native send copies the pixels and waits for GPU execution and Syphon's
        // completion handler before returning. The borrowed slice can then die.
        let sent = unsafe {
            syndocal_syphon_send_rgba(
                handle.as_ptr(),
                width,
                height,
                rgba.as_ptr(),
                rgba.len(),
                error.as_mut_ptr(),
                error.len(),
            )
        };
        if sent == 1 {
            Ok(())
        } else {
            Err(native_error(&error, "Syphon frame publication failed"))
        }
    }

    /// A failed close leaves this owner valid. The transport must retain its
    /// teardown lease and this owner until close succeeds; it cannot report idle.
    pub(crate) fn close(&mut self) -> Result<(), String> {
        let Some(handle) = self.handle else {
            return Ok(());
        };
        let mut error = [0; ERROR_CAPACITY];
        // Native code consumes the retained object only on successful shutdown.
        let closed =
            unsafe { syndocal_syphon_close(handle.as_ptr(), error.as_mut_ptr(), error.len()) };
        if closed == 1 {
            self.handle = None;
            Ok(())
        } else {
            Err(native_error(&error, "Syphon output shutdown failed"))
        }
    }
}

impl Drop for SyphonSender {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            // Normal transport teardown calls close explicitly and retains the
            // owner on failure. This is an unwind fallback: do not claim success
            // or free an object which may still own a submitted command buffer.
            eprintln!("Syphon output could not be destroyed: {error}");
        }
    }
}
