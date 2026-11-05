//! Thread-affine macOS Syphon/Metal owner; no frame is submitted during creation.

#![cfg(all(target_os = "macos", feature = "syphon"))]

use std::ffi::{c_char, c_void, CStr, CString};
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::rc::Rc;

const ERROR_CAPACITY: usize = 512;
const METADATA_CAPACITY: usize = 4096;
const SERVER_LIST_CAPACITY: usize = 128 * 1024;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SyphonServer {
    pub name: String,
    pub app_name: String,
    pub uuid: String,
}

unsafe extern "C" {
    fn syndocal_syphon_list_servers(
        json: *mut c_char,
        capacity: usize,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn syndocal_syphon_client_create(
        endpoint: *const c_char,
        error: *mut c_char,
        capacity: usize,
    ) -> *mut c_void;
    fn syndocal_syphon_client_try_receive_rgba(
        handle: *mut c_void,
        width: *mut u32,
        height: *mut u32,
        rgba: *mut u8,
        rgba_capacity: usize,
        error: *mut c_char,
        error_capacity: usize,
    ) -> i32;
    fn syndocal_syphon_client_close(
        handle: *mut c_void,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
    #[cfg(test)]
    fn syndocal_syphon_test_client_create(
        server: *mut c_void,
        error: *mut c_char,
        capacity: usize,
    ) -> *mut c_void;
    #[cfg(test)]
    fn syndocal_syphon_test_wait_frame(handle: *mut c_void) -> i32;
    #[cfg(test)]
    fn syndocal_syphon_test_lifecycle(server: *mut c_void, client: *mut c_void) -> i32;
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

pub(crate) fn list_servers() -> Result<Vec<SyphonServer>, String> {
    let mut json = vec![0 as c_char; SERVER_LIST_CAPACITY];
    let mut error = [0; ERROR_CAPACITY];
    // Framework objects remain on this thread; only bounded JSON crosses FFI.
    let result = unsafe {
        syndocal_syphon_list_servers(
            json.as_mut_ptr(),
            json.len(),
            error.as_mut_ptr(),
            error.len(),
        )
    };
    if result != 1 {
        return Err(native_error(&error, "Syphon server discovery failed"));
    }
    let json = unsafe { CStr::from_ptr(json.as_ptr()) }.to_bytes();
    let servers: Vec<SyphonServer> = serde_json::from_slice(json)
        .map_err(|error| format!("Invalid Syphon server list: {error}"))?;
    if servers.len() > 64 {
        return Err("Syphon server list exceeds 64 servers".into());
    }
    let mut identities = std::collections::HashSet::new();
    for server in &servers {
        if !identities.insert(&server.uuid) {
            return Err(
                "Syphon publishers have duplicate identities; close them and refresh".into(),
            );
        }
    }
    Ok(servers)
}

/// Construct, consume, and retire on its owner thread. No detached polling or
/// frame storage allocation occurs here once the caller's buffer is sized.
pub(crate) struct SyphonReceiver {
    handle: Option<NonNull<c_void>>,
    _thread_affinity: PhantomData<Rc<()>>,
}

impl SyphonReceiver {
    pub(crate) fn new(endpoint: &str) -> Result<Self, String> {
        if endpoint.len() > 4096 {
            return Err(
                "Syphon server identity exceeds 4096 bytes; refresh and select again".into(),
            );
        }
        let identity: SyphonServer = serde_json::from_str(endpoint).map_err(|_| {
            "Invalid Syphon server identity; refresh and select a discovered server"
        })?;
        if identity.uuid.is_empty()
            || identity.uuid.len() > 1024
            || identity.app_name.is_empty()
            || identity.name.len() > 1024
            || identity.app_name.len() > 1024
            || [&identity.uuid, &identity.name, &identity.app_name]
                .iter()
                .any(|value| value.chars().any(char::is_control))
        {
            return Err(
                "Invalid Syphon server identity; refresh and select a discovered server".into(),
            );
        }
        let endpoint = CString::new(endpoint).map_err(|_| "Syphon server identity contains NUL")?;
        let mut error = [0; ERROR_CAPACITY];
        let handle = unsafe {
            syndocal_syphon_client_create(endpoint.as_ptr(), error.as_mut_ptr(), error.len())
        };
        Ok(Self {
            handle: Some(
                NonNull::new(handle)
                    .ok_or_else(|| native_error(&error, "Could not initialize Syphon input"))?,
            ),
            _thread_affinity: PhantomData,
        })
    }

    pub(crate) fn receive_rgba(
        &mut self,
        rgba: &mut Vec<u8>,
    ) -> Result<Option<(u32, u32)>, String> {
        let handle = self
            .handle
            .ok_or_else(|| "Syphon input is closed".to_owned())?;
        let mut width = 0;
        let mut height = 0;
        let mut error = [0; ERROR_CAPACITY];
        // A resize response retains the exact acquired texture natively. At
        // most one retry is needed; stable dimensions reuse both allocations.
        for attempt in 0..2 {
            let result = unsafe {
                syndocal_syphon_client_try_receive_rgba(
                    handle.as_ptr(),
                    &mut width,
                    &mut height,
                    rgba.as_mut_ptr(),
                    rgba.len(),
                    error.as_mut_ptr(),
                    error.len(),
                )
            };
            match result {
                0 => return Ok(None),
                1 | 2 => {
                    if !(1..=8192).contains(&width) || !(1..=8192).contains(&height) {
                        return Err("Syphon input returned invalid dimensions".into());
                    }
                    let length = (width as usize)
                        .checked_mul(height as usize)
                        .and_then(|pixels| pixels.checked_mul(4))
                        .ok_or_else(|| "Syphon input frame size overflow".to_owned())?;
                    if result == 1 {
                        if length > rgba.len() {
                            return Err("Syphon input exceeded its frame buffer".into());
                        }
                        rgba.truncate(length);
                        return Ok(Some((width, height)));
                    }
                    if attempt == 0 {
                        rgba.resize(length, 0);
                    }
                }
                _ => return Err(native_error(&error, "Syphon input receive failed")),
            }
        }
        Err("Syphon input dimensions changed during its bounded receive".into())
    }

    /// On failure the retained native owner remains valid for explicit retry.
    pub(crate) fn close(&mut self) -> Result<(), String> {
        let Some(handle) = self.handle else {
            return Ok(());
        };
        let mut error = [0; ERROR_CAPACITY];
        if unsafe { syndocal_syphon_client_close(handle.as_ptr(), error.as_mut_ptr(), error.len()) }
            == 1
        {
            self.handle = None;
            Ok(())
        } else {
            Err(native_error(&error, "Syphon input shutdown failed"))
        }
    }
}

impl Drop for SyphonReceiver {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            // The runtime normally retains this owner on explicit close failure.
            // An unwind must not release an object with unproven GPU retirement.
            eprintln!("Syphon input could not be destroyed: {error}");
        }
    }
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
    fn syphon_native_client_loopback_preserves_dimensions_rgba_orientation_and_closes() {
        let mut sender = SyphonSender::new("Syndocal-CI-Client-Loopback").unwrap();
        let mut error = [0; ERROR_CAPACITY];
        // Use the actual serverDescription directly: discovery announcements
        // are asynchronous and are separately exercised by the friend kit.
        let handle = unsafe {
            syndocal_syphon_test_client_create(
                sender.handle.unwrap().as_ptr(),
                error.as_mut_ptr(),
                error.len(),
            )
        };
        let mut receiver = SyphonReceiver {
            handle: Some(
                NonNull::new(handle).expect(&native_error(&error, "Loopback client failed")),
            ),
            _thread_affinity: PhantomData,
        };
        assert_eq!(
            unsafe {
                syndocal_syphon_test_lifecycle(
                    sender.handle.unwrap().as_ptr(),
                    receiver.handle.unwrap().as_ptr(),
                )
            },
            1
        );
        // Red/green top row, blue/white bottom row. Exact bytes detect BGRA
        // swaps, vertical flips, alpha loss, and stale/empty texture reads.
        let rgba = [
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];
        sender.send_rgba(2, 2, &rgba).unwrap();
        // Wait only on this in-process client's new-frame notification, never
        // poll an external publisher or rely on an arbitrary sleep.
        assert_eq!(unsafe { syndocal_syphon_test_wait_frame(handle) }, 1);
        let mut received = Vec::new();
        assert_eq!(receiver.receive_rgba(&mut received).unwrap(), Some((2, 2)));
        assert_eq!(received, rgba);
        assert_eq!(receiver.receive_rgba(&mut received).unwrap(), None);
        let capacity = received.capacity();
        let address = received.as_ptr();
        let next = [1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, 10, 11, 12, 255];
        sender.send_rgba(2, 2, &next).unwrap();
        assert_eq!(unsafe { syndocal_syphon_test_wait_frame(handle) }, 1);
        assert_eq!(receiver.receive_rgba(&mut received).unwrap(), Some((2, 2)));
        assert_eq!(received, next);
        assert_eq!(received.capacity(), capacity);
        assert_eq!(received.as_ptr(), address);
        receiver.close().unwrap();
        receiver.close().unwrap();
        assert!(receiver.receive_rgba(&mut received).is_err());
        sender.close().unwrap();
        assert_eq!(
            unsafe { syndocal_syphon_test_lifecycle(std::ptr::null_mut(), std::ptr::null_mut()) },
            1,
            "stop + release must leave neither the native server nor client alive"
        );
    }

    #[test]
    fn syphon_native_client_rejects_invalid_identity_before_ffi() {
        for endpoint in [
            "old free-form server name",
            r#"{"uuid":"","name":"test","app_name":"test"}"#,
            r#"{"uuid":"test","name":"test","app_name":"test","future":1}"#,
            r#"{"uuid":"test","name":"test\n","app_name":"test"}"#,
        ] {
            assert!(SyphonReceiver::new(endpoint).is_err(), "{endpoint}");
        }
    }

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
