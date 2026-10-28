//! Physical client extent of a native output window, read on its render
//! thread without a main-thread round trip.
//!
//! `tauri::Window::inner_size` is a synchronous request to the main event
//! loop (tauri-runtime-wry `window_getter!`): the calling thread blocks until
//! the main thread has finished whatever WebView, IPC or window work it is
//! doing. The resident Display tick read it up to three times (tick start,
//! BeforeAcquire, BeforePhysicalPresent), so every main-thread stall landed on
//! the frame deadline (C4R7 live: 3.2 ms mean, 13 ms worst case per tick).
//!
//! On Windows the event loop answers with exactly `GetClientRect(hwnd)` (tao
//! `Window::inner_size`), which any thread may call without sending a
//! message. The HWND is resolved once on the cold start path and every read
//! re-proves it is still a window of this process and of the thread that
//! created it, so a destroyed (and possibly reused) handle fails closed
//! instead of reporting another window's size. Other platforms keep the
//! event-loop query.
use std::sync::Arc;

pub(crate) struct NativeWindowExtent {
    #[cfg(not(windows))]
    window: Arc<tauri::Window>,
    #[cfg(windows)]
    client: win32::ClientExtentReader,
}

impl NativeWindowExtent {
    /// Cold path: one event-loop query for the native handle.
    pub(crate) fn new(window: &Arc<tauri::Window>) -> Result<Self, String> {
        #[cfg(windows)]
        {
            let hwnd = window
                .hwnd()
                .map_err(|error| format!("Native output window HWND query failed: {error}"))?;
            let client = win32::ClientExtentReader::new(hwnd.0 as isize)?;
            Ok(Self { client })
        }
        #[cfg(not(windows))]
        {
            Ok(Self {
                window: Arc::clone(window),
            })
        }
    }

    /// Physical client size in pixels, identical to `inner_size`.
    pub(crate) fn physical_size(&self) -> Result<(u32, u32), String> {
        #[cfg(windows)]
        {
            self.client.physical_size()
        }
        #[cfg(not(windows))]
        {
            self.window
                .inner_size()
                .map(|size| (size.width, size.height))
                .map_err(|error| error.to_string())
        }
    }
}

#[cfg(windows)]
mod win32 {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, GetWindowThreadProcessId};

    /// The owning process/thread pair captured with the handle. Window
    /// handles are only reused after destruction, and a reused handle cannot
    /// keep both this process and the original creating thread unless that
    /// thread recreated it; such a window is still never a foreign one.
    pub(super) struct ClientExtentReader {
        hwnd: isize,
        process: u32,
        thread: u32,
    }

    fn owner(hwnd: isize) -> (u32, u32) {
        let mut process = 0_u32;
        // SAFETY: a plain query on a window handle value; an invalid handle
        // returns thread 0 and leaves the out value untouched.
        let thread = unsafe {
            GetWindowThreadProcessId(HWND(hwnd as *mut core::ffi::c_void), Some(&mut process))
        };
        (process, thread)
    }

    impl ClientExtentReader {
        pub(super) fn new(hwnd: isize) -> Result<Self, String> {
            let (process, thread) = owner(hwnd);
            if hwnd == 0 || thread == 0 || process != std::process::id() {
                return Err("Native output window handle is not a window of this process".into());
            }
            Ok(Self {
                hwnd,
                process,
                thread,
            })
        }

        pub(super) fn physical_size(&self) -> Result<(u32, u32), String> {
            if owner(self.hwnd) != (self.process, self.thread) {
                return Err("Native output window no longer exists".into());
            }
            let mut rect = RECT::default();
            // SAFETY: `rect` is a valid out pointer; GetClientRect reads the
            // window's stored client rectangle and sends no message.
            unsafe { GetClientRect(HWND(self.hwnd as *mut core::ffi::c_void), &mut rect) }
                .map_err(|error| format!("Native output window client rectangle: {error}"))?;
            // tao: (right - left, bottom - top); the client origin is (0, 0).
            let width = u32::try_from(rect.right - rect.left)
                .map_err(|_| "Native output window has a negative client width".to_string())?;
            let height = u32::try_from(rect.bottom - rect.top)
                .map_err(|_| "Native output window has a negative client height".to_string())?;
            Ok((width, height))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use windows::core::w;
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, SetWindowPos, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER,
            WINDOW_EX_STYLE, WS_POPUP,
        };

        fn popup(width: i32, height: i32) -> HWND {
            // SAFETY: a hidden, never-shown popup of the predefined STATIC
            // class owned by this test thread.
            unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("STATIC"),
                    w!("c4r7 extent"),
                    WS_POPUP,
                    0,
                    0,
                    width,
                    height,
                    None,
                    None,
                    None,
                    None,
                )
            }
            .expect("test popup window")
        }

        #[test]
        fn native_display_window_extent_reads_off_thread_and_fails_closed() {
            let hwnd = popup(321, 123);
            let reader = ClientExtentReader::new(hwnd.0 as isize).unwrap();
            assert_eq!(reader.physical_size().unwrap(), (321, 123));
            // The render thread is never the window's thread; no message is
            // sent, so this works while the creating thread does not pump.
            let raw = hwnd.0 as isize;
            let off_thread = std::thread::spawn(move || {
                let reader = ClientExtentReader::new(raw).unwrap();
                reader.physical_size()
            })
            .join()
            .unwrap();
            assert_eq!(off_thread.unwrap(), (321, 123));
            // A resize is observed by the next read, exactly as inner_size.
            // SAFETY: resizing the test-owned popup without activation.
            unsafe {
                SetWindowPos(
                    hwnd,
                    None,
                    0,
                    0,
                    640,
                    360,
                    SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                )
            }
            .unwrap();
            assert_eq!(reader.physical_size().unwrap(), (640, 360));
            // SAFETY: destroying the test-owned popup on its creating thread.
            unsafe { DestroyWindow(hwnd) }.unwrap();
            assert!(reader.physical_size().is_err());
            assert!(ClientExtentReader::new(raw).is_err());
            assert!(ClientExtentReader::new(0).is_err());
        }
    }
}
