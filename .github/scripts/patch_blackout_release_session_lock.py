from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


cargo = Path("app/src-tauri/Cargo.toml")
security = Path("app/src-tauri/src/control_plane_security.rs")

replace_exact(
    cargo,
    'windows = { version = "=0.61.3", features = ["Win32_Foundation", "Win32_Storage_FileSystem", "Win32_System_IO", "Win32_UI_Input", "Win32_UI_Input_KeyboardAndMouse", "Win32_UI_WindowsAndMessaging"] }',
    'windows = { version = "=0.61.3", features = ["Win32_Foundation", "Win32_Storage_FileSystem", "Win32_System_IO", "Win32_System_RemoteDesktop", "Win32_UI_Input", "Win32_UI_Input_KeyboardAndMouse", "Win32_UI_WindowsAndMessaging"] }',
    "Windows session notification feature",
)

# The monitor's liveness flag must be owned by the worker, not optimistically
# latched by the caller after startup. If the Raw Input/WTS message loop exits,
# R4 preparation immediately becomes unavailable and any outstanding token is
# retired before the worker returns.
replace_exact(
    security,
    "    monitor_active: AtomicBool,\n",
    "    monitor_active: Arc<AtomicBool>,\n",
    "share physical monitor liveness with worker",
)
replace_exact(
    security,
    "            monitor_active: AtomicBool::new(false),\n",
    "            monitor_active: Arc::new(AtomicBool::new(false)),\n",
    "initialize shared physical monitor liveness",
)
replace_exact(
    security,
    '''        *monitor = Some(PhysicalInputMonitor::start(Arc::clone(&self.inner))?);
        self.monitor_active.store(true, Ordering::Release);
        Ok(())
''',
    '''        *monitor = Some(PhysicalInputMonitor::start(
            Arc::clone(&self.inner),
            Arc::clone(&self.monitor_active),
        )?);
        Ok(())
''',
    "worker owns physical monitor active transition",
)
replace_exact(
    security,
    '''    fn start(inner: Arc<Mutex<SecurityInner>>) -> Result<Self, String> {
''',
    '''    fn start(
        inner: Arc<Mutex<SecurityInner>>,
        monitor_active: Arc<AtomicBool>,
    ) -> Result<Self, String> {
''',
    "Windows physical monitor receives liveness flag",
)
replace_exact(
    security,
    '''            .spawn(move || raw_input_thread(inner, ready_tx))
''',
    '''            .spawn(move || raw_input_thread(inner, monitor_active, ready_tx))
''',
    "Raw Input worker receives liveness flag",
)
replace_exact(
    security,
    '''    fn start(_inner: Arc<Mutex<SecurityInner>>) -> Result<Self, String> {
''',
    '''    fn start(
        _inner: Arc<Mutex<SecurityInner>>,
        _monitor_active: Arc<AtomicBool>,
    ) -> Result<Self, String> {
''',
    "non-Windows physical monitor signature stays fail-closed",
)
replace_exact(
    security,
    '''fn raw_input_thread(
    inner: Arc<Mutex<SecurityInner>>,
    ready: std::sync::mpsc::SyncSender<Result<isize, String>>,
) {
''',
    '''fn raw_input_thread(
    inner: Arc<Mutex<SecurityInner>>,
    monitor_active: Arc<AtomicBool>,
    ready: std::sync::mpsc::SyncSender<Result<isize, String>>,
) {
''',
    "Raw Input thread owns physical monitor liveness",
)
replace_exact(
    security,
    '''        let context = Box::into_raw(Box::new(inner));
''',
    '''        let monitor_inner = Arc::clone(&inner);
        let context = Box::into_raw(Box::new(inner));
''',
    "retain consent state for worker-exit revocation",
)

replace_exact(
    security,
    '''fn record_device_removal(inner: &Arc<Mutex<SecurityInner>>, device: usize, now: Instant) {
    if device == 0 {
        return;
    }
    if let Ok(mut inner) = inner.lock() {
        prune_security_inner(&mut inner, now);
        let invalidates_active = inner.active.as_ref().is_some_and(|record| {
            record.progress_device == Some(device) || record.matched_device == Some(device)
        });
        if !invalidates_active {
            return;
        }
        // Record only the device which is actually bound to the active
        // challenge. Unrelated or synthetic removal notifications cannot grow
        // retained security state for the full tombstone TTL.
        inner.removed_devices.insert(device, now);
        if let Some(record) = inner.active.take() {
            push_tombstone(&mut inner, record.consent_token, now);
        }
    }
}
''',
    '''fn record_device_removal(inner: &Arc<Mutex<SecurityInner>>, device: usize, now: Instant) {
    if device == 0 {
        return;
    }
    if let Ok(mut inner) = inner.lock() {
        prune_security_inner(&mut inner, now);
        let invalidates_active = inner.active.as_ref().is_some_and(|record| {
            record.progress_device == Some(device) || record.matched_device == Some(device)
        });
        if !invalidates_active {
            return;
        }
        // Record only the device which is actually bound to the active
        // challenge. Unrelated or synthetic removal notifications cannot grow
        // retained security state for the full tombstone TTL.
        inner.removed_devices.insert(device, now);
        if let Some(record) = inner.active.take() {
            push_tombstone(&mut inner, record.consent_token, now);
        }
    }
}

fn record_desktop_session_security_transition(
    inner: &Arc<Mutex<SecurityInner>>,
    now: Instant,
) {
    if let Ok(mut inner) = inner.lock() {
        prune_security_inner(&mut inner, now);
        if let Some(record) = inner.active.take() {
            push_tombstone(&mut inner, record.consent_token, now);
        }
    }
}
''',
    "desktop session transition consent invalidation helper",
)

replace_exact(
    security,
    '''    use windows::{
        core::w,
        Win32::UI::{
            Input::{RegisterRawInputDevices, RAWINPUTDEVICE, RIDEV_DEVNOTIFY, RIDEV_INPUTSINK},
            WindowsAndMessaging::{
                CreateWindowExW, DispatchMessageW, GetMessageW, RegisterClassW, TranslateMessage,
                HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSW,
            },
        },
    };
''',
    '''    use windows::{
        core::w,
        Win32::{
            System::RemoteDesktop::{
                WTSRegisterSessionNotification, WTSUnRegisterSessionNotification,
                NOTIFY_FOR_THIS_SESSION,
            },
            UI::{
                Input::{
                    RegisterRawInputDevices, RAWINPUTDEVICE, RIDEV_DEVNOTIFY, RIDEV_INPUTSINK,
                },
                WindowsAndMessaging::{
                    CreateWindowExW, DispatchMessageW, GetMessageW, RegisterClassW,
                    TranslateMessage, HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSW,
                },
            },
        },
    };
''',
    "Raw Input thread imports session notifications",
)

replace_exact(
    security,
    '''        let devices = [RAWINPUTDEVICE {
            usUsagePage: 0x01,
''',
    '''        // Microsoft documents an early-startup RPC_S_INVALID_BINDING race
        // before the Remote Desktop Services dependencies are ready. Retry for
        // a short bounded interval, then fail closed: R4 Release must never run
        // without desktop lock/remote-session invalidation being active.
        let session_notification_deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) {
                Ok(()) => break,
                Err(_) if Instant::now() < session_notification_deadline => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(error) => {
                    let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
                    let _ = ready.send(Err(format!(
                        "Unable to register desktop session-security notifications: {error}"
                    )));
                    return;
                }
            }
        }
        let devices = [RAWINPUTDEVICE {
            usUsagePage: 0x01,
''',
    "register session notifications before Raw Input",
)

replace_exact(
    security,
    '''        if let Err(error) = RegisterRawInputDevices(&devices, size_of::<RAWINPUTDEVICE>() as u32) {
            let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
''',
    '''        if let Err(error) = RegisterRawInputDevices(&devices, size_of::<RAWINPUTDEVICE>() as u32) {
            let _ = WTSUnRegisterSessionNotification(hwnd);
            let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
''',
    "unregister session notifications on Raw Input setup failure",
)
replace_exact(
    security,
    '''        let _ = ready.send(Ok(hwnd.0 as isize));
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
''',
    '''        monitor_active.store(true, Ordering::Release);
        if ready.send(Ok(hwnd.0 as isize)).is_err() {
            monitor_active.store(false, Ordering::Release);
            let _ = WTSUnRegisterSessionNotification(hwnd);
            let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
            return;
        }
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        monitor_active.store(false, Ordering::Release);
        record_desktop_session_security_transition(&monitor_inner, Instant::now());
''',
    "Raw Input worker exit fails closed and revokes consent",
)

replace_exact(
    security,
    '''    use windows::Win32::{
        Foundation::{HANDLE, LRESULT},
        UI::{
''',
    '''    use windows::Win32::{
        Foundation::{HANDLE, LRESULT},
        System::RemoteDesktop::WTSUnRegisterSessionNotification,
        UI::{
''',
    "window proc imports session unregister",
)
replace_exact(
    security,
    '''                WM_INPUT, WM_INPUT_DEVICE_CHANGE, WM_KEYDOWN, WM_NCCREATE, WM_NCDESTROY,
                WM_SYSKEYDOWN,
''',
    '''                WM_INPUT, WM_INPUT_DEVICE_CHANGE, WM_KEYDOWN, WM_NCCREATE, WM_NCDESTROY,
                WM_SYSKEYDOWN, WM_WTSSESSION_CHANGE, WTS_CONSOLE_DISCONNECT,
                WTS_REMOTE_CONNECT, WTS_REMOTE_DISCONNECT, WTS_SESSION_LOCK,
                WTS_SESSION_REMOTE_CONTROL,
''',
    "window proc imports typed session-security constants",
)

replace_exact(
    security,
    '''        WM_INPUT_DEVICE_CHANGE if !context.is_null() && wparam.0 as u32 == GIDC_REMOVAL => {
            let device = HANDLE(lparam.0 as *mut c_void).0 as usize;
            record_device_removal(&*context, device, Instant::now());
            DefWindowProcW(hwnd, message, wparam, lparam)
        }
        value if value == WM_APP + 0x51 => {
''',
    '''        WM_INPUT_DEVICE_CHANGE if !context.is_null() && wparam.0 as u32 == GIDC_REMOVAL => {
            let device = HANDLE(lparam.0 as *mut c_void).0 as usize;
            record_device_removal(&*context, device, Instant::now());
            DefWindowProcW(hwnd, message, wparam, lparam)
        }
        WM_WTSSESSION_CHANGE
            if !context.is_null()
                && [
                    WTS_SESSION_LOCK as usize,
                    WTS_CONSOLE_DISCONNECT as usize,
                    WTS_REMOTE_CONNECT as usize,
                    WTS_REMOTE_DISCONNECT as usize,
                    WTS_SESSION_REMOTE_CONTROL as usize,
                ]
                .contains(&wparam.0) =>
        {
            // A challenge confirmed on the local console cannot survive a
            // transition into/out of a remote-control session. RDP input is
            // already excluded by Raw Input device enumeration, but this also
            // prevents a still-live 15-second local token being consumed after
            // session topology changes.
            record_desktop_session_security_transition(&*context, Instant::now());
            DefWindowProcW(hwnd, message, wparam, lparam)
        }
        value if value == WM_APP + 0x51 => {
''',
    "handle desktop lock and remote session transitions",
)

replace_exact(
    security,
    '''        WM_NCDESTROY => {
            if !context.is_null() {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
''',
    '''        WM_NCDESTROY => {
            let _ = WTSUnRegisterSessionNotification(hwnd);
            if !context.is_null() {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
''',
    "unregister session notifications at hidden-window teardown",
)

replace_exact(
    security,
    '''    #[test]
    fn renderer_principal_retirement_invalidates_ready_release_consent() {
''',
    '''    #[test]
    fn desktop_session_lock_invalidates_ready_release_consent() {
        let state = ControlPlaneSecurityState::default();
        let authority = binding(OUTPUT_BLACKOUT_RELEASE_OPERATION_ID, 9);
        let prepared = state.prepare_consent(authority.clone()).unwrap();
        enter_code(&state, &prepared.display_code, 0x9999);
        record_desktop_session_security_transition(&state.inner, Instant::now());
        assert_eq!(
            state.consume_consent(&authority, &prepared.consent_token),
            Err(ConsentConsumeError::Replayed)
        );
    }

    #[test]
    fn renderer_principal_retirement_invalidates_ready_release_consent() {
''',
    "desktop session transition focused test",
)
