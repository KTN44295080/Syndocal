//! Backend-owned human-presence and prepared-consent service.
//!
//! The release path deliberately accepts evidence only from a Windows Raw
//! Input message whose source is reported as hardware and whose non-null
//! device handle is present in the current raw-input device catalog. DOM
//! events, accelerator callbacks, posted messages and API traffic never enter
//! this module's evidence path.

use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use getrandom::getrandom;

const CONSENT_CHALLENGE_TTL: Duration = Duration::from_secs(30);
const CONSENT_TOMBSTONE_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_CONSENT_RECORDS: usize = 64;
const CHALLENGE_DIGITS: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ConsentCallerBinding {
    pub principal: String,
    pub window_label: String,
    pub owner_incarnation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConsentAuthorityBinding {
    pub caller: ConsentCallerBinding,
    pub operation_id: String,
    pub argument_fingerprint: String,
    pub project_epoch: u64,
    pub project_revision: u64,
    pub project_checkpoint_hash: String,
    pub project_publication_generation: u64,
    pub output_epoch: u64,
    pub output_generation: u64,
    pub safety_blackout_epoch: u64,
    pub safety_blackout_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedConsentChallenge {
    pub challenge_id: String,
    pub consent_token: String,
    pub display_code: String,
    pub expires_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreparedConsentState {
    PendingPhysicalInput,
    Ready,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedConsentStatus {
    pub state: PreparedConsentState,
    pub expires_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConsentConsumeError {
    Missing,
    Expired,
    WrongBinding,
    PhysicalInputPending,
    DeviceRemoved,
    Replayed,
    Internal,
}

#[derive(Debug, Clone)]
struct ConsentRecord {
    binding: ConsentAuthorityBinding,
    challenge_id: String,
    consent_token: String,
    display_code: [u8; CHALLENGE_DIGITS],
    progress: usize,
    matched_device: Option<usize>,
    expires_at: Instant,
    expires_at_unix_ms: u64,
}

#[derive(Debug, Clone)]
struct ConsentTombstone {
    consent_token: String,
    expires_at: Instant,
}

#[derive(Debug, Default)]
struct SecurityInner {
    active: Option<ConsentRecord>,
    tombstones: VecDeque<ConsentTombstone>,
    removed_devices: HashMap<usize, Instant>,
}

pub(crate) struct ControlPlaneSecurityState {
    inner: Arc<Mutex<SecurityInner>>,
    monitor: Mutex<Option<PhysicalInputMonitor>>,
    monitor_active: AtomicBool,
}

impl Default for ControlPlaneSecurityState {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SecurityInner::default())),
            monitor: Mutex::new(None),
            monitor_active: AtomicBool::new(false),
        }
    }
}

impl ControlPlaneSecurityState {
    pub(crate) fn start_physical_input_monitor(&self) -> Result<(), String> {
        let mut monitor = self
            .monitor
            .lock()
            .map_err(|_| "Physical-input monitor lock was poisoned".to_string())?;
        if monitor.is_some() {
            return Ok(());
        }
        *monitor = Some(PhysicalInputMonitor::start(Arc::clone(&self.inner))?);
        self.monitor_active.store(true, Ordering::Release);
        Ok(())
    }

    pub(crate) fn prepare_consent(
        &self,
        binding: ConsentAuthorityBinding,
    ) -> Result<PreparedConsentChallenge, String> {
        #[cfg(not(test))]
        if !self.monitor_active.load(Ordering::Acquire) {
            return Err("Physical Raw Input confirmation is unavailable".to_string());
        }
        validate_binding(&binding)?;
        let now = Instant::now();
        let expires_at = now + CONSENT_CHALLENGE_TTL;
        let expires_at_unix_ms = current_unix_ms()
            .checked_add(CONSENT_CHALLENGE_TTL.as_millis() as u64)
            .ok_or_else(|| "Consent expiry is unavailable".to_string())?;
        let challenge_id = random_token_128()?;
        let consent_token = random_token_128()?;
        let display_code = random_decimal_code()?;
        let display_code_text = display_code
            .iter()
            .map(|digit| char::from(b'0' + *digit))
            .collect::<String>();

        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "Consent state lock was poisoned".to_string())?;
        prune_security_inner(&mut inner, now);
        // There is exactly one physical confirmation challenge process-wide.
        // Starting a newer dialog invalidates the older dialog rather than
        // allowing one physical key sequence to approve multiple operations.
        if let Some(previous) = inner.active.take() {
            push_tombstone(&mut inner, previous.consent_token, now);
        }
        inner.active = Some(ConsentRecord {
            binding,
            challenge_id: challenge_id.clone(),
            consent_token: consent_token.clone(),
            display_code,
            progress: 0,
            matched_device: None,
            expires_at,
            expires_at_unix_ms,
        });
        Ok(PreparedConsentChallenge {
            challenge_id,
            consent_token,
            display_code: display_code_text,
            expires_at_unix_ms,
        })
    }

    pub(crate) fn consent_status(
        &self,
        caller: &ConsentCallerBinding,
        challenge_id: &str,
    ) -> Result<PreparedConsentStatus, ConsentConsumeError> {
        let now = Instant::now();
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| ConsentConsumeError::Internal)?;
        prune_security_inner(&mut inner, now);
        let Some(record) = inner.active.as_ref() else {
            return Err(ConsentConsumeError::Missing);
        };
        if record.challenge_id != challenge_id || &record.binding.caller != caller {
            return Err(ConsentConsumeError::WrongBinding);
        }
        if record.expires_at <= now {
            return Err(ConsentConsumeError::Expired);
        }
        let state = if record.matched_device.is_some() {
            PreparedConsentState::Ready
        } else {
            PreparedConsentState::PendingPhysicalInput
        };
        Ok(PreparedConsentStatus {
            state,
            expires_at_unix_ms: record.expires_at_unix_ms,
        })
    }

    pub(crate) fn consume_consent(
        &self,
        binding: &ConsentAuthorityBinding,
        consent_token: &str,
    ) -> Result<(), ConsentConsumeError> {
        let now = Instant::now();
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| ConsentConsumeError::Internal)?;
        prune_security_inner(&mut inner, now);
        if inner
            .tombstones
            .iter()
            .any(|entry| entry.consent_token == consent_token)
        {
            return Err(ConsentConsumeError::Replayed);
        }
        let Some(record) = inner.active.as_ref() else {
            return Err(ConsentConsumeError::Missing);
        };
        if record.expires_at <= now {
            return Err(ConsentConsumeError::Expired);
        }
        if &record.binding != binding || record.consent_token != consent_token {
            return Err(ConsentConsumeError::WrongBinding);
        }
        let Some(device) = record.matched_device else {
            return Err(ConsentConsumeError::PhysicalInputPending);
        };
        if inner.removed_devices.contains_key(&device) {
            return Err(ConsentConsumeError::DeviceRemoved);
        }
        let consumed = inner.active.take().ok_or(ConsentConsumeError::Missing)?;
        push_tombstone(&mut inner, consumed.consent_token, now);
        Ok(())
    }

    pub(crate) fn retire_caller(&self, caller: &ConsentCallerBinding) {
        let now = Instant::now();
        if let Ok(mut inner) = self.inner.lock() {
            prune_security_inner(&mut inner, now);
            if inner
                .active
                .as_ref()
                .is_some_and(|record| &record.binding.caller == caller)
            {
                if let Some(record) = inner.active.take() {
                    push_tombstone(&mut inner, record.consent_token, now);
                }
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn observe_physical_digit_for_test(&self, digit: u8, device: usize) {
        record_physical_digit(&self.inner, digit, device, Instant::now());
    }

    #[cfg(test)]
    pub(crate) fn remove_physical_device_for_test(&self, device: usize) {
        record_device_removal(&self.inner, device, Instant::now());
    }
}

fn validate_binding(binding: &ConsentAuthorityBinding) -> Result<(), String> {
    if binding.caller.principal.is_empty()
        || binding.caller.window_label.is_empty()
        || binding.caller.owner_incarnation == 0
        || binding.operation_id.is_empty()
        || binding.argument_fingerprint.len() != 64
        || !binding
            .argument_fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || binding.project_checkpoint_hash.len() != 64
        || !binding
            .project_checkpoint_hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || binding.output_epoch == 0
        || binding.output_generation == 0
        || binding.safety_blackout_epoch == 0
        || binding.safety_blackout_generation == 0
    {
        return Err("Consent authority binding is invalid".to_string());
    }
    Ok(())
}

fn prune_security_inner(inner: &mut SecurityInner, now: Instant) {
    if inner
        .active
        .as_ref()
        .is_some_and(|record| record.expires_at <= now)
    {
        if let Some(record) = inner.active.take() {
            push_tombstone(inner, record.consent_token, now);
        }
    }
    inner.tombstones.retain(|entry| entry.expires_at > now);
    inner
        .removed_devices
        .retain(|_, removed_at| now.saturating_duration_since(*removed_at) < CONSENT_TOMBSTONE_TTL);
}

fn push_tombstone(inner: &mut SecurityInner, consent_token: String, now: Instant) {
    while inner.tombstones.len() >= MAX_CONSENT_RECORDS {
        inner.tombstones.pop_front();
    }
    inner.tombstones.push_back(ConsentTombstone {
        consent_token,
        expires_at: now + CONSENT_TOMBSTONE_TTL,
    });
}

fn record_physical_digit(
    inner: &Arc<Mutex<SecurityInner>>,
    digit: u8,
    device: usize,
    now: Instant,
) {
    if digit > 9 || device == 0 {
        return;
    }
    let Ok(mut inner) = inner.lock() else {
        return;
    };
    prune_security_inner(&mut inner, now);
    inner.removed_devices.remove(&device);
    let Some(record) = inner.active.as_mut() else {
        return;
    };
    if record.expires_at <= now || record.matched_device.is_some() {
        return;
    }
    if record.display_code[record.progress] == digit {
        record.progress += 1;
        if record.progress == CHALLENGE_DIGITS {
            record.matched_device = Some(device);
        }
    } else {
        record.progress = usize::from(record.display_code[0] == digit);
    }
}

fn record_device_removal(inner: &Arc<Mutex<SecurityInner>>, device: usize, now: Instant) {
    if device == 0 {
        return;
    }
    if let Ok(mut inner) = inner.lock() {
        prune_security_inner(&mut inner, now);
        inner.removed_devices.insert(device, now);
    }
}

fn random_token_128() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    getrandom(&mut bytes).map_err(|error| format!("OS randomness failed: {error}"))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn random_decimal_code() -> Result<[u8; CHALLENGE_DIGITS], String> {
    let mut random = [0u8; CHALLENGE_DIGITS];
    getrandom(&mut random).map_err(|error| format!("OS randomness failed: {error}"))?;
    Ok(random.map(|value| value % 10))
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

#[cfg(target_os = "windows")]
struct PhysicalInputMonitor {
    hwnd: isize,
    worker: Option<std::thread::JoinHandle<()>>,
}

#[cfg(not(target_os = "windows"))]
struct PhysicalInputMonitor;

#[cfg(target_os = "windows")]
impl PhysicalInputMonitor {
    fn start(inner: Arc<Mutex<SecurityInner>>) -> Result<Self, String> {
        use std::sync::mpsc::sync_channel;
        let (ready_tx, ready_rx) = sync_channel(1);
        let worker = std::thread::Builder::new()
            .name("syndocal-raw-input-consent".to_string())
            .spawn(move || raw_input_thread(inner, ready_tx))
            .map_err(|error| format!("Unable to start Raw Input monitor: {error}"))?;
        let hwnd = ready_rx
            .recv()
            .map_err(|_| "Raw Input monitor exited during startup".to_string())??;
        Ok(Self {
            hwnd,
            worker: Some(worker),
        })
    }
}

#[cfg(not(target_os = "windows"))]
impl PhysicalInputMonitor {
    fn start(_inner: Arc<Mutex<SecurityInner>>) -> Result<Self, String> {
        Err("Physical Raw Input confirmation is available only on Windows".to_string())
    }
}

#[cfg(target_os = "windows")]
impl Drop for PhysicalInputMonitor {
    fn drop(&mut self) {
        use windows::Win32::{
            Foundation::{HWND, LPARAM, WPARAM},
            UI::WindowsAndMessaging::{PostMessageW, WM_APP},
        };
        unsafe {
            let _ = PostMessageW(
                Some(HWND(self.hwnd as *mut _)),
                WM_APP + 0x51,
                WPARAM(0),
                LPARAM(0),
            );
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(target_os = "windows")]
fn raw_input_thread(
    inner: Arc<Mutex<SecurityInner>>,
    ready: std::sync::mpsc::SyncSender<Result<isize, String>>,
) {
    use std::{ffi::c_void, mem::size_of};
    use windows::{
        core::w,
        Win32::UI::{
            Input::{RegisterRawInputDevices, RAWINPUTDEVICE, RIDEV_DEVNOTIFY, RIDEV_INPUTSINK},
            WindowsAndMessaging::{
                CreateWindowExW, DispatchMessageW, GetMessageW, RegisterClassW, TranslateMessage,
                HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSW,
            },
        },
    };
    unsafe {
        let class = WNDCLASSW {
            lpfnWndProc: Some(raw_input_window_proc),
            lpszClassName: w!("SyndocalRawInputConsentWindowV1"),
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            let _ = ready.send(Err("Unable to register Raw Input window class".to_string()));
            return;
        }
        let context = Box::into_raw(Box::new(inner));
        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("SyndocalRawInputConsentWindowV1"),
            w!(""),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            None,
            Some(context.cast::<c_void>()),
        ) {
            Ok(hwnd) => hwnd,
            Err(error) => {
                drop(Box::from_raw(context));
                let _ = ready.send(Err(format!("Unable to create Raw Input window: {error}")));
                return;
            }
        };
        let devices = [RAWINPUTDEVICE {
            usUsagePage: 0x01,
            usUsage: 0x06,
            dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
            hwndTarget: hwnd,
        }];
        if let Err(error) = RegisterRawInputDevices(&devices, size_of::<RAWINPUTDEVICE>() as u32) {
            let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
            let _ = ready.send(Err(format!(
                "Unable to register physical keyboard input: {error}"
            )));
            return;
        }
        let _ = ready.send(Ok(hwnd.0 as isize));
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn raw_input_window_proc(
    hwnd: windows::Win32::Foundation::HWND,
    message: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use std::{ffi::c_void, mem::size_of, sync::Arc};
    use windows::Win32::{
        Foundation::{HANDLE, LRESULT},
        UI::{
            Input::{
                GetCurrentInputMessageSource, GetRawInputData, IMDT_KEYBOARD, IMO_HARDWARE,
                INPUT_MESSAGE_SOURCE, RAWINPUT, RAWINPUTHEADER, RID_INPUT, RIM_TYPEKEYBOARD,
            },
            WindowsAndMessaging::{
                DefWindowProcW, DestroyWindow, GetWindowLongPtrW, PostQuitMessage,
                SetWindowLongPtrW, CREATESTRUCTW, GIDC_REMOVAL, GWLP_USERDATA, WM_APP, WM_DESTROY,
                WM_INPUT, WM_INPUT_DEVICE_CHANGE, WM_KEYDOWN, WM_NCCREATE, WM_NCDESTROY,
                WM_SYSKEYDOWN,
            },
        },
    };

    if message == WM_NCCREATE {
        let create = lparam.0 as *const CREATESTRUCTW;
        if create.is_null() {
            return LRESULT(0);
        }
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*create).lpCreateParams as isize);
        return DefWindowProcW(hwnd, message, wparam, lparam);
    }
    let context = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Arc<Mutex<SecurityInner>>;
    match message {
        WM_INPUT if !context.is_null() => {
            let inner = &*context;
            if !inner
                .lock()
                .map(|state| state.active.is_some())
                .unwrap_or(false)
            {
                return DefWindowProcW(hwnd, message, wparam, lparam);
            }
            let mut source = INPUT_MESSAGE_SOURCE::default();
            if GetCurrentInputMessageSource(&mut source).is_err()
                || source.originId != IMO_HARDWARE
                || source.deviceType != IMDT_KEYBOARD
            {
                return DefWindowProcW(hwnd, message, wparam, lparam);
            }
            let mut size = 0u32;
            let raw_handle = windows::Win32::UI::Input::HRAWINPUT(lparam.0 as *mut c_void);
            GetRawInputData(
                raw_handle,
                RID_INPUT,
                None,
                &mut size,
                size_of::<RAWINPUTHEADER>() as u32,
            );
            if size < size_of::<RAWINPUT>() as u32 {
                return DefWindowProcW(hwnd, message, wparam, lparam);
            }
            let words = (size as usize).div_ceil(size_of::<usize>());
            let mut buffer = vec![0usize; words];
            let copied = GetRawInputData(
                raw_handle,
                RID_INPUT,
                Some(buffer.as_mut_ptr().cast()),
                &mut size,
                size_of::<RAWINPUTHEADER>() as u32,
            );
            if copied == u32::MAX || copied != size {
                return DefWindowProcW(hwnd, message, wparam, lparam);
            }
            let raw = &*(buffer.as_ptr().cast::<RAWINPUT>());
            if raw.header.dwType != RIM_TYPEKEYBOARD.0 || raw.header.hDevice.0.is_null() {
                return DefWindowProcW(hwnd, message, wparam, lparam);
            }
            let device = raw.header.hDevice.0 as usize;
            if !raw_keyboard_device_is_enumerated(device) {
                return DefWindowProcW(hwnd, message, wparam, lparam);
            }
            let keyboard = raw.data.keyboard;
            if keyboard.Message != WM_KEYDOWN && keyboard.Message != WM_SYSKEYDOWN {
                return DefWindowProcW(hwnd, message, wparam, lparam);
            }
            let digit = match keyboard.VKey {
                0x30..=0x39 => Some((keyboard.VKey - 0x30) as u8),
                0x60..=0x69 => Some((keyboard.VKey - 0x60) as u8),
                _ => None,
            };
            if let Some(digit) = digit {
                record_physical_digit(inner, digit, device, Instant::now());
            }
            DefWindowProcW(hwnd, message, wparam, lparam)
        }
        WM_INPUT_DEVICE_CHANGE if !context.is_null() && wparam.0 as u32 == GIDC_REMOVAL => {
            let device = HANDLE(lparam.0 as *mut c_void).0 as usize;
            record_device_removal(&*context, device, Instant::now());
            DefWindowProcW(hwnd, message, wparam, lparam)
        }
        value if value == WM_APP + 0x51 => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            if !context.is_null() {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                drop(Box::from_raw(context));
            }
            DefWindowProcW(hwnd, message, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

#[cfg(target_os = "windows")]
unsafe fn raw_keyboard_device_is_enumerated(device: usize) -> bool {
    use std::mem::size_of;
    use windows::Win32::UI::Input::{GetRawInputDeviceList, RAWINPUTDEVICELIST, RIM_TYPEKEYBOARD};
    let mut count = 0u32;
    let first = GetRawInputDeviceList(None, &mut count, size_of::<RAWINPUTDEVICELIST>() as u32);
    if first == u32::MAX || count == 0 || count > 4_096 {
        return false;
    }
    let mut entries = vec![RAWINPUTDEVICELIST::default(); count as usize];
    let copied = GetRawInputDeviceList(
        Some(entries.as_mut_ptr()),
        &mut count,
        size_of::<RAWINPUTDEVICELIST>() as u32,
    );
    copied != u32::MAX
        && entries
            .into_iter()
            .take(copied as usize)
            .any(|entry| entry.dwType == RIM_TYPEKEYBOARD && entry.hDevice.0 as usize == device)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(operation: &str, output_generation: u64) -> ConsentAuthorityBinding {
        ConsentAuthorityBinding {
            caller: ConsentCallerBinding {
                principal: "desktop:main:1".to_string(),
                window_label: "main".to_string(),
                owner_incarnation: 1,
            },
            operation_id: operation.to_string(),
            argument_fingerprint: "11".repeat(32),
            project_epoch: 2,
            project_revision: 3,
            project_checkpoint_hash: "22".repeat(32),
            project_publication_generation: 4,
            output_epoch: 5,
            output_generation,
            safety_blackout_epoch: 6,
            safety_blackout_generation: 7,
        }
    }

    fn enter_code(state: &ControlPlaneSecurityState, code: &str, device: usize) {
        for digit in code.bytes() {
            state.observe_physical_digit_for_test(digit - b'0', device);
        }
    }

    #[test]
    fn prepared_consent_is_physical_single_use_and_exactly_bound() {
        let state = ControlPlaneSecurityState::default();
        let authority = binding("syndocal.output.arm.v1", 9);
        let prepared = state.prepare_consent(authority.clone()).unwrap();
        assert_eq!(prepared.display_code.len(), CHALLENGE_DIGITS);
        assert_eq!(
            state
                .consent_status(&authority.caller, &prepared.challenge_id)
                .unwrap()
                .state,
            PreparedConsentState::PendingPhysicalInput
        );
        assert_eq!(
            state.consume_consent(&authority, &prepared.consent_token),
            Err(ConsentConsumeError::PhysicalInputPending)
        );
        enter_code(&state, &prepared.display_code, 0x1234);
        assert_eq!(
            state
                .consent_status(&authority.caller, &prepared.challenge_id)
                .unwrap()
                .state,
            PreparedConsentState::Ready
        );
        let mut wrong = authority.clone();
        wrong.output_generation += 1;
        assert_eq!(
            state.consume_consent(&wrong, &prepared.consent_token),
            Err(ConsentConsumeError::WrongBinding)
        );
        assert_eq!(
            state.consume_consent(&authority, &prepared.consent_token),
            Ok(())
        );
        assert_eq!(
            state.consume_consent(&authority, &prepared.consent_token),
            Err(ConsentConsumeError::Replayed)
        );
    }

    #[test]
    fn new_challenge_retires_old_and_device_removal_invalidates_ready_token() {
        let state = ControlPlaneSecurityState::default();
        let authority = binding("syndocal.output.arm.v1", 9);
        let first = state.prepare_consent(authority.clone()).unwrap();
        let second = state.prepare_consent(authority.clone()).unwrap();
        assert_eq!(
            state.consume_consent(&authority, &first.consent_token),
            Err(ConsentConsumeError::Replayed)
        );
        enter_code(&state, &second.display_code, 0x7777);
        state.remove_physical_device_for_test(0x7777);
        assert_eq!(
            state.consume_consent(&authority, &second.consent_token),
            Err(ConsentConsumeError::DeviceRemoved)
        );
    }
}
