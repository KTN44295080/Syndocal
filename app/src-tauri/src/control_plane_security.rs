//! Backend-owned human-presence and prepared-consent service.
//!
//! The release path deliberately accepts evidence only from a Windows Raw
//! Input message whose packet is a keyboard packet and whose non-null device
//! handle is present in the current raw-input device catalog. DOM events,
//! accelerator callbacks, posted messages and API traffic never enter this
//! module's evidence path. The packet/device checks are intentional: the
//! higher-level input-message source API is not the trust boundary for a
//! `WM_INPUT` packet and can reject valid Raw Input keyboard messages.
//! This is an in-process presence proof, not an attestation against a
//! privileged signed UIAccess injector or a driver-level virtual device that
//! can produce kernel-attributed Raw Input; those threats are outside this
//! boundary. Ordinary `SendInput`, `PostMessage`, DOM events and API traffic
//! are rejected by the packet/device checks below.

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

// R4 physical confirmation is intentionally short-lived. The local bridge
// is not the AI4 grant/presence service; this bound only prevents a prepared
// challenge from remaining usable while a user or project state changes.
const CONSENT_CHALLENGE_TTL: Duration = Duration::from_secs(15);
// Once the physical sequence is complete, keep the Ready record for a small
// internal handoff window so the status poll can reach the one-shot consume
// path even when the original pending-input deadline is at the boundary.
const CONSENT_READY_HANDOFF_TTL: Duration = Duration::from_secs(5);
const CONSENT_TOMBSTONE_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_CONSENT_RECORDS: usize = 64;
const CHALLENGE_DIGITS: usize = 6;
const MAX_RAW_INPUT_PACKET_BYTES: u32 = 4 * 1024;

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
    #[cfg(any(target_os = "windows", test))]
    display_code: [u8; CHALLENGE_DIGITS],
    #[cfg(any(target_os = "windows", test))]
    progress: usize,
    #[cfg(any(target_os = "windows", test))]
    bound_device: Option<usize>,
    matched_device: Option<usize>,
    ready_expires_at: Option<Instant>,
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
        // A removal invalidates the active challenge, but a newly prepared
        // challenge starts a new device-observation generation. The next Raw
        // Input packet must still carry a currently enumerated handle.
        inner.removed_devices.clear();
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
            #[cfg(any(target_os = "windows", test))]
            display_code,
            #[cfg(any(target_os = "windows", test))]
            progress: 0,
            #[cfg(any(target_os = "windows", test))]
            bound_device: None,
            matched_device: None,
            ready_expires_at: None,
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
        if consent_record_deadline(record) <= now {
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
        if consent_record_deadline(record) <= now {
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
    // Keep an expired active record until its next explicit prepare replaces
    // it. Status/consume can then report `Expired` instead of losing the
    // challenge identity and misreporting an ordinary timeout as `Missing`.
    // A replacement or successful consume still moves the token to the
    // bounded tombstone set, preserving one-shot/replay protection.
    inner.tombstones.retain(|entry| entry.expires_at > now);
    inner
        .removed_devices
        .retain(|_, removed_at| now.saturating_duration_since(*removed_at) < CONSENT_TOMBSTONE_TTL);
}

fn consent_record_deadline(record: &ConsentRecord) -> Instant {
    record.ready_expires_at.unwrap_or(record.expires_at)
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

#[cfg(any(target_os = "windows", test))]
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
    if inner.removed_devices.contains_key(&device) {
        return;
    }
    let Some(record) = inner.active.as_mut() else {
        return;
    };
    if consent_record_deadline(record) <= now || record.matched_device.is_some() {
        return;
    }
    if let Some(bound_device) = record.bound_device {
        if bound_device != device {
            return;
        }
    } else {
        record.bound_device = Some(device);
    }
    if record.display_code[record.progress] == digit {
        record.progress += 1;
        if record.progress == CHALLENGE_DIGITS {
            record.matched_device = Some(device);
            // A code entered early remains usable only until the original
            // challenge expiry.  A code completed at the boundary receives
            // one bounded five-second handoff.  The public deadline is
            // monotonic so the renderer can distinguish this one transition
            // from an attacker-controlled repeated extension.
            let ready_expires_at = (now + CONSENT_READY_HANDOFF_TTL).max(record.expires_at);
            record.ready_expires_at = Some(ready_expires_at);
            // Publish the same one-shot handoff deadline that status and
            // consume enforce. Keeping the public expiry at the original
            // pending-input deadline would make a valid Ready response look
            // stale to the renderer and discard the handoff window.
            record.expires_at_unix_ms = record.expires_at_unix_ms.max(
                current_unix_ms().saturating_add(CONSENT_READY_HANDOFF_TTL.as_millis() as u64),
            );
        }
    } else {
        record.progress = usize::from(record.display_code[0] == digit);
    }
}

#[cfg(any(target_os = "windows", test))]
fn record_device_removal(inner: &Arc<Mutex<SecurityInner>>, device: usize, now: Instant) {
    if device == 0 {
        return;
    }
    if let Ok(mut inner) = inner.lock() {
        prune_security_inner(&mut inner, now);
        // Only a device that has already contributed to this challenge can
        // invalidate it. Ignoring unrelated removal notifications keeps this
        // map bounded even on systems with noisy device-change traffic.
        let relevant = inner.active.as_ref().is_some_and(|record| {
            record.bound_device == Some(device) || record.matched_device == Some(device)
        });
        if relevant {
            inner.removed_devices.insert(device, now);
        }
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
            Input::{GetRawInputData, RAWINPUT, RAWINPUTHEADER, RID_INPUT},
            WindowsAndMessaging::{
                DefWindowProcW, DestroyWindow, GetWindowLongPtrW, PostQuitMessage,
                SetWindowLongPtrW, CREATESTRUCTW, GIDC_REMOVAL, GWLP_USERDATA, WM_APP, WM_DESTROY,
                WM_INPUT, WM_INPUT_DEVICE_CHANGE, WM_NCCREATE, WM_NCDESTROY,
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
            // Do not use GetCurrentInputMessageSource as a second provenance
            // check here: its IMO_HARDWARE value is not a documented
            // WM_INPUT trust boundary and can include UIAccess injection.
            // The accepted evidence is the bounded system WM_INPUT packet,
            // its keyboard type, and its currently enumerated device handle.
            if !raw_input_wparam_is_acceptable(wparam.0) {
                return DefWindowProcW(hwnd, message, wparam, lparam);
            }
            if !inner
                .lock()
                .map(|state| state.active.is_some())
                .unwrap_or(false)
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
            if !raw_input_size_is_acceptable(size, size_of::<RAWINPUT>() as u32) {
                return DefWindowProcW(hwnd, message, wparam, lparam);
            }
            // RAWINPUT contains a pointer-sized device handle. Keep the
            // allocation typed so the cast below is aligned on every Windows
            // target; a Vec<u8>/Vec<usize> does not document that contract.
            let elements = (size as usize).div_ceil(size_of::<RAWINPUT>());
            let mut buffer = vec![RAWINPUT::default(); elements];
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
            let raw = &buffer[0];
            let device = raw.header.hDevice.0 as usize;
            if !raw_keyboard_device_is_acceptable(
                raw.header.dwType,
                device,
                raw_keyboard_device_is_enumerated(device),
            ) {
                return DefWindowProcW(hwnd, message, wparam, lparam);
            }
            let keyboard = raw.data.keyboard;
            let digit = raw_keyboard_digit(keyboard.Message, keyboard.VKey);
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

#[cfg(any(target_os = "windows", test))]
fn raw_keyboard_device_is_acceptable(raw_type: u32, device: usize, enumerated: bool) -> bool {
    raw_type == RIM_TYPEKEYBOARD_VALUE && device != 0 && enumerated
}

#[cfg(any(target_os = "windows", test))]
fn raw_input_size_is_acceptable(size: u32, minimum: u32) -> bool {
    size >= minimum && size <= MAX_RAW_INPUT_PACKET_BYTES
}

#[cfg(any(target_os = "windows", test))]
fn raw_input_wparam_is_acceptable(wparam: usize) -> bool {
    wparam == RAW_RIM_INPUT || wparam == RAW_RIM_INPUTSINK
}

#[cfg(any(target_os = "windows", test))]
fn raw_keyboard_digit(message: u32, vkey: u16) -> Option<u8> {
    if message != RAW_WM_KEYDOWN && message != RAW_WM_SYSKEYDOWN {
        return None;
    }
    match vkey {
        0x30..=0x39 => Some((vkey - 0x30) as u8),
        0x60..=0x69 => Some((vkey - 0x60) as u8),
        _ => None,
    }
}

#[cfg(any(target_os = "windows", test))]
const RIM_TYPEKEYBOARD_VALUE: u32 = 1;

#[cfg(any(target_os = "windows", test))]
const RAW_RIM_INPUT: usize = 0;

#[cfg(any(target_os = "windows", test))]
const RAW_RIM_INPUTSINK: usize = 1;

#[cfg(any(target_os = "windows", test))]
const RAW_WM_KEYDOWN: u32 = 0x0100;

#[cfg(any(target_os = "windows", test))]
const RAW_WM_SYSKEYDOWN: u32 = 0x0104;

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

        let mut wrong_action = authority.clone();
        wrong_action.operation_id = "syndocal.output.blackout.release.v1".to_string();
        assert_eq!(
            state.consume_consent(&wrong_action, &prepared.consent_token),
            Err(ConsentConsumeError::WrongBinding)
        );
        let mut wrong_argument = authority.clone();
        wrong_argument.argument_fingerprint = "33".repeat(32);
        assert_eq!(
            state.consume_consent(&wrong_argument, &prepared.consent_token),
            Err(ConsentConsumeError::WrongBinding)
        );
        let mut wrong_owner = authority.clone();
        wrong_owner.caller.owner_incarnation += 1;
        assert_eq!(
            state.consume_consent(&wrong_owner, &prepared.consent_token),
            Err(ConsentConsumeError::WrongBinding)
        );

        enter_code(&state, &prepared.display_code, 0x1234);
        assert_eq!(
            state
                .consent_status(&authority.caller, &prepared.challenge_id)
                .unwrap()
                .state,
            PreparedConsentState::Ready
        );
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

    #[test]
    fn ready_handoff_survives_original_deadline_until_one_shot_consume() {
        let state = ControlPlaneSecurityState::default();
        let authority = binding("syndocal.output.arm.v1", 9);
        let prepared = state.prepare_consent(authority.clone()).unwrap();
        enter_code(&state, &prepared.display_code, 0x9999);
        {
            let mut inner = state.inner.lock().unwrap();
            let record = inner.active.as_mut().unwrap();
            assert!(record.matched_device.is_some());
            record.expires_at = Instant::now() - Duration::from_secs(1);
        }

        assert_eq!(
            state
                .consent_status(&authority.caller, &prepared.challenge_id)
                .unwrap()
                .state,
            PreparedConsentState::Ready
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
    fn boundary_ready_publishes_one_bounded_monotonic_handoff_deadline() {
        let state = ControlPlaneSecurityState::default();
        let authority = binding("syndocal.output.arm.v1", 9);
        let prepared = state.prepare_consent(authority.clone()).unwrap();
        let code = prepared
            .display_code
            .bytes()
            .map(|digit| digit - b'0')
            .collect::<Vec<_>>();
        let boundary = Instant::now();
        let pending_public_deadline = current_unix_ms().saturating_add(10);
        {
            let mut inner = state.inner.lock().unwrap();
            let record = inner.active.as_mut().unwrap();
            record.expires_at = boundary + Duration::from_millis(10);
            record.expires_at_unix_ms = pending_public_deadline;
        }
        for digit in &code[..CHALLENGE_DIGITS - 1] {
            record_physical_digit(&state.inner, *digit, 0x4242, boundary);
        }
        record_physical_digit(
            &state.inner,
            code[CHALLENGE_DIGITS - 1],
            0x4242,
            boundary + Duration::from_millis(9),
        );

        let status = state
            .consent_status(&authority.caller, &prepared.challenge_id)
            .unwrap();
        assert_eq!(status.state, PreparedConsentState::Ready);
        assert!(status.expires_at_unix_ms >= pending_public_deadline);
        assert!(
            status.expires_at_unix_ms
                <= current_unix_ms()
                    .saturating_add(CONSENT_READY_HANDOFF_TTL.as_millis() as u64)
                    .saturating_add(100),
            "Ready deadline exceeded its single bounded handoff"
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
    fn expired_ready_handoff_is_fail_closed_until_replacement() {
        let state = ControlPlaneSecurityState::default();
        let authority = binding("syndocal.output.arm.v1", 9);
        let prepared = state.prepare_consent(authority.clone()).unwrap();
        enter_code(&state, &prepared.display_code, 0x9999);
        {
            let mut inner = state.inner.lock().unwrap();
            inner.active.as_mut().unwrap().ready_expires_at =
                Some(Instant::now() - Duration::from_secs(1));
        }

        assert_eq!(
            state.consent_status(&authority.caller, &prepared.challenge_id),
            Err(ConsentConsumeError::Expired)
        );
        assert_eq!(
            state.consume_consent(&authority, &prepared.consent_token),
            Err(ConsentConsumeError::Expired)
        );
        let _replacement = state.prepare_consent(authority.clone()).unwrap();
        assert_eq!(
            state.consume_consent(&authority, &prepared.consent_token),
            Err(ConsentConsumeError::Replayed)
        );
    }

    #[test]
    fn expired_pending_challenge_is_reported_and_cannot_be_consumed() {
        let state = ControlPlaneSecurityState::default();
        let authority = binding("syndocal.output.arm.v1", 9);
        let prepared = state.prepare_consent(authority.clone()).unwrap();
        {
            let mut inner = state.inner.lock().unwrap();
            inner.active.as_mut().unwrap().expires_at = Instant::now() - Duration::from_secs(1);
        }

        assert_eq!(
            state.consent_status(&authority.caller, &prepared.challenge_id),
            Err(ConsentConsumeError::Expired)
        );
        assert_eq!(
            state.consume_consent(&authority, &prepared.consent_token),
            Err(ConsentConsumeError::Expired)
        );
        let _replacement = state.prepare_consent(authority.clone()).unwrap();
        assert_eq!(
            state.consume_consent(&authority, &prepared.consent_token),
            Err(ConsentConsumeError::Replayed)
        );
    }

    #[test]
    fn raw_keyboard_digit_accepts_top_row_and_numpad_keydowns_only() {
        assert_eq!(raw_keyboard_digit(RAW_WM_KEYDOWN, 0x30), Some(0));
        assert_eq!(raw_keyboard_digit(RAW_WM_KEYDOWN, 0x39), Some(9));
        assert_eq!(raw_keyboard_digit(RAW_WM_SYSKEYDOWN, 0x60), Some(0));
        assert_eq!(raw_keyboard_digit(RAW_WM_SYSKEYDOWN, 0x69), Some(9));
        assert_eq!(raw_keyboard_digit(0x0101, 0x31), None);
        assert_eq!(raw_keyboard_digit(0x0105, 0x61), None);
        assert_eq!(raw_keyboard_digit(RAW_WM_KEYDOWN, 0x41), None);
        assert_eq!(raw_keyboard_digit(RAW_WM_KEYDOWN, 0x6a), None);
    }

    #[test]
    fn raw_keyboard_device_acceptance_is_fail_closed() {
        assert!(raw_keyboard_device_is_acceptable(
            RIM_TYPEKEYBOARD_VALUE,
            0x1234,
            true
        ));
        assert!(!raw_keyboard_device_is_acceptable(
            RIM_TYPEKEYBOARD_VALUE,
            0,
            true
        ));
        assert!(!raw_keyboard_device_is_acceptable(
            RIM_TYPEKEYBOARD_VALUE,
            0x1234,
            false
        ));
        assert!(!raw_keyboard_device_is_acceptable(2, 0x1234, true));
    }

    #[test]
    fn raw_input_size_is_bounded_and_requires_a_complete_header() {
        assert!(!raw_input_size_is_acceptable(47, 48));
        assert!(raw_input_size_is_acceptable(48, 48));
        assert!(raw_input_size_is_acceptable(MAX_RAW_INPUT_PACKET_BYTES, 48));
        assert!(!raw_input_size_is_acceptable(
            MAX_RAW_INPUT_PACKET_BYTES + 1,
            48
        ));
    }

    #[test]
    fn raw_input_wparam_accepts_only_system_raw_input_values() {
        assert!(raw_input_wparam_is_acceptable(RAW_RIM_INPUT));
        assert!(raw_input_wparam_is_acceptable(RAW_RIM_INPUTSINK));
        assert!(!raw_input_wparam_is_acceptable(2));
        assert!(!raw_input_wparam_is_acceptable(usize::MAX));
    }

    #[test]
    fn physical_sequence_stays_bound_to_first_device_and_removal_is_sticky() {
        let state = ControlPlaneSecurityState::default();
        let authority = binding("syndocal.output.arm.v1", 9);
        let prepared = state.prepare_consent(authority.clone()).unwrap();
        let device = 0x1234;
        let other_device = 0x5678;

        for (index, digit) in prepared.display_code.bytes().enumerate() {
            state.observe_physical_digit_for_test(
                digit - b'0',
                if index == CHALLENGE_DIGITS - 1 {
                    other_device
                } else {
                    device
                },
            );
        }
        assert_eq!(
            state
                .consent_status(&authority.caller, &prepared.challenge_id)
                .unwrap()
                .state,
            PreparedConsentState::PendingPhysicalInput
        );

        state.remove_physical_device_for_test(device);
        for digit in prepared.display_code.bytes() {
            state.observe_physical_digit_for_test(digit - b'0', device);
        }
        assert_eq!(
            state
                .consent_status(&authority.caller, &prepared.challenge_id)
                .unwrap()
                .state,
            PreparedConsentState::PendingPhysicalInput
        );

        let fresh = state.prepare_consent(authority.clone()).unwrap();
        enter_code(&state, &fresh.display_code, device);
        assert_eq!(
            state
                .consent_status(&authority.caller, &fresh.challenge_id)
                .unwrap()
                .state,
            PreparedConsentState::Ready
        );
    }

    #[test]
    fn unrelated_device_removals_are_ignored_and_bound_removal_blocks_input() {
        let state = ControlPlaneSecurityState::default();
        let authority = binding("syndocal.output.arm.v1", 9);
        let prepared = state.prepare_consent(authority.clone()).unwrap();
        let bound_device = 0x1234;

        for device in 0x10000..0x10000 + 10_000 {
            state.remove_physical_device_for_test(device);
        }
        assert!(state.inner.lock().unwrap().removed_devices.is_empty());

        let first_digit = prepared.display_code.as_bytes()[0] - b'0';
        state.observe_physical_digit_for_test(first_digit, bound_device);
        for device in 0x20000..0x20000 + 10_000 {
            state.remove_physical_device_for_test(device);
        }
        assert!(state.inner.lock().unwrap().removed_devices.is_empty());

        state.remove_physical_device_for_test(bound_device);
        assert_eq!(state.inner.lock().unwrap().removed_devices.len(), 1);
        for digit in prepared.display_code.bytes().skip(1) {
            state.observe_physical_digit_for_test(digit - b'0', bound_device);
        }
        assert_eq!(
            state
                .consent_status(&authority.caller, &prepared.challenge_id)
                .unwrap()
                .state,
            PreparedConsentState::PendingPhysicalInput
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "requires an interactive Windows input desktop"]
    fn send_input_does_not_satisfy_physical_confirmation() {
        use std::mem::size_of;
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
        };

        let state = ControlPlaneSecurityState::default();
        state.start_physical_input_monitor().unwrap();
        let authority = binding("syndocal.output.arm.v1", 9);
        let prepared = state.prepare_consent(authority.clone()).unwrap();
        for digit in prepared.display_code.bytes() {
            let key = VIRTUAL_KEY(u16::from(digit - b'0') + 0x30);
            let inputs = [
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: key,
                            ..Default::default()
                        },
                    },
                },
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: key,
                            dwFlags: KEYEVENTF_KEYUP,
                            ..Default::default()
                        },
                    },
                },
            ];
            let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
            assert_eq!(sent, inputs.len() as u32);
        }
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(
            state
                .consent_status(&authority.caller, &prepared.challenge_id)
                .unwrap()
                .state,
            PreparedConsentState::PendingPhysicalInput
        );
    }
}
