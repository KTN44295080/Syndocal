//! Thread-affine macOS Syphon/Metal owner; no frame is submitted during creation.

#![cfg(all(target_os = "macos", feature = "syphon"))]

use std::ffi::{c_char, c_void, CStr, CString};
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::rc::Rc;

const ERROR_CAPACITY: usize = 512;
const METADATA_CAPACITY: usize = 4096;

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
    fn syndocal_syphon_metadata(
        handle: *mut c_void,
        json: *mut c_char,
        capacity: usize,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    #[cfg(test)]
    fn syndocal_syphon_verify_test_pattern(
        handle: *mut c_void,
        rgba: *const u8,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
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
    pub(crate) fn metadata(
        &self,
    ) -> Result<crate::syphon_transport::syphon_diagnostics::NativeMetadata, String> {
        native_metadata(self.handle.map_or(std::ptr::null_mut(), NonNull::as_ptr))
    }

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

#[cfg(not(test))]
pub(crate) fn unavailable_metadata(
) -> Result<crate::syphon_transport::syphon_diagnostics::NativeMetadata, String> {
    native_metadata(std::ptr::null_mut())
}

fn native_metadata(
    handle: *mut c_void,
) -> Result<crate::syphon_transport::syphon_diagnostics::NativeMetadata, String> {
    let mut json = [0 as c_char; METADATA_CAPACITY];
    let mut error = [0; ERROR_CAPACITY];
    // The native side supplies a terminated UTF-8 JSON document or an error.
    let result = unsafe {
        syndocal_syphon_metadata(
            handle,
            json.as_mut_ptr(),
            json.len(),
            error.as_mut_ptr(),
            error.len(),
        )
    };
    if result != 1 {
        return Err(native_error(&error, "Syphon metadata query failed"));
    }
    let json = unsafe { CStr::from_ptr(json.as_ptr()) }.to_bytes();
    serde_json::from_slice(json).map_err(|error| format!("Invalid Syphon metadata: {error}"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syphon_native_published_texture_preserves_rgba_and_orientation() {
        let mut sender = SyphonSender::new("Syndocal-CI-Readback-Test")
            .expect("CI must have a Metal device and bundled Syphon shader");
        let metadata = sender.metadata().unwrap();
        assert!(!metadata.macos_version.is_empty());
        assert!(metadata
            .metal_device_name
            .as_deref()
            .is_some_and(|name| !name.is_empty()));
        assert!(metadata.has_unified_memory.is_some());
        // Top-left red, top-right green, bottom-left blue, bottom-right white.
        let rgba = [
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];
        sender.send_rgba(2, 2, &rgba).unwrap();
        let mut error = [0; ERROR_CAPACITY];
        let verified = unsafe {
            syndocal_syphon_verify_test_pattern(
                sender.handle.unwrap().as_ptr(),
                rgba.as_ptr(),
                error.as_mut_ptr(),
                error.len(),
            )
        };
        assert_eq!(
            verified,
            1,
            "{}",
            native_error(&error, "Syphon published texture readback failed")
        );
        sender.close().unwrap();
        sender.close().unwrap();
        assert!(sender.send_rgba(2, 2, &rgba).is_err());
    }
}
