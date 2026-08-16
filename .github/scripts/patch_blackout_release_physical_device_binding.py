from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


security = Path("app/src-tauri/src/control_plane_security.rs")
runtime = Path("app/src-tauri/src/control_plane_runtime.rs")

replace_exact(
    security,
    '''pub(crate) struct ConsentAuthorityBinding {
    pub caller: ConsentCallerBinding,
    pub operation_id: String,
''',
    '''pub(crate) struct ConsentAuthorityBinding {
    pub caller: ConsentCallerBinding,
    /// Native HWND of the renderer which displayed this exact challenge.
    /// Stored only in backend consent state; it is not an external protocol field.
    pub foreground_window: usize,
    pub operation_id: String,
''',
    "consent authority binds challenge renderer HWND",
)
replace_exact(
    security,
    '''    display_code: [u8; CHALLENGE_DIGITS],
    progress: usize,
    matched_device: Option<usize>,
''',
    '''    display_code: [u8; CHALLENGE_DIGITS],
    progress: usize,
    progress_device: Option<usize>,
    matched_device: Option<usize>,
''',
    "consent record physical progress device",
)
replace_exact(
    security,
    '''            display_code,
            progress: 0,
            matched_device: None,
''',
    '''            display_code,
            progress: 0,
            progress_device: None,
            matched_device: None,
''',
    "initialize consent physical progress device",
)
replace_exact(
    security,
    '''        || binding.caller.owner_incarnation == 0
        || binding.operation_id != OUTPUT_BLACKOUT_RELEASE_OPERATION_ID
''',
    '''        || binding.caller.owner_incarnation == 0
        || binding.foreground_window == 0
        || binding.operation_id != OUTPUT_BLACKOUT_RELEASE_OPERATION_ID
''',
    "consent authority requires challenge renderer HWND",
)
replace_exact(
    security,
    '''        if inner.removed_devices.contains_key(&device) {
            return Err(ConsentConsumeError::DeviceRemoved);
        }
        let consumed = inner.active.take().ok_or(ConsentConsumeError::Missing)?;
''',
    '''        if inner.removed_devices.contains_key(&device) {
            return Err(ConsentConsumeError::DeviceRemoved);
        }
        #[cfg(all(target_os = "windows", not(test)))]
        if unsafe { !raw_keyboard_device_is_enumerated(device) } {
            // A WM_INPUT_DEVICE_CHANGE removal notification is useful but not
            // the sole liveness proof. Re-enumerate at the consume boundary so
            // a missed notification cannot leave a physically absent keyboard
            // capable of authorizing an energizing R4 command.
            return Err(ConsentConsumeError::DeviceRemoved);
        }
        let consumed = inner.active.take().ok_or(ConsentConsumeError::Missing)?;
''',
    "re-enumerate matched keyboard at consent consume",
)
replace_exact(
    security,
    '''    prune_security_inner(&mut inner, now);
    inner.removed_devices.remove(&device);
    let Some(record) = inner.active.as_mut() else {
        return;
    };
    if record.expires_at <= now || record.matched_device.is_some() {
        return;
    }
''',
    '''    prune_security_inner(&mut inner, now);
    #[cfg(all(target_os = "windows", not(test)))]
    if !inner.active.as_ref().is_some_and(|record| {
        foreground_window_matches(record.binding.foreground_window)
    }) {
        // RIDEV_INPUTSINK lets the hidden Raw Input window receive records even
        // when Syndocal is not foreground. Only the renderer which displayed
        // this exact challenge may contribute confirmation digits.
        return;
    }
    // A device which disappeared after starting or completing this exact
    // challenge may never resume it, even if Windows later reuses the same
    // numeric Raw Input handle. A fresh challenge may bind a freshly observed
    // device, but an outstanding challenge keeps its original device binding.
    if inner.active.as_ref().is_some_and(|record| {
        (record.progress_device == Some(device) || record.matched_device == Some(device))
            && inner.removed_devices.contains_key(&device)
    }) {
        return;
    }
    inner.removed_devices.remove(&device);
    let Some(record) = inner.active.as_mut() else {
        return;
    };
    if record.expires_at <= now || record.matched_device.is_some() {
        return;
    }
''',
    "bind physical digits to active challenge renderer and device",
)
replace_exact(
    security,
    '''    if record.expires_at <= now || record.matched_device.is_some() {
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
''',
    '''    if record.expires_at <= now || record.matched_device.is_some() {
        return;
    }
    if record
        .progress_device
        .is_some_and(|progress_device| progress_device != device)
    {
        return;
    }
    if record.display_code[record.progress] == digit {
        if record.progress == 0 {
            record.progress_device = Some(device);
        }
        record.progress += 1;
        if record.progress == CHALLENGE_DIGITS {
            record.matched_device = Some(device);
        }
    } else {
        record.progress = usize::from(record.display_code[0] == digit);
        record.progress_device = (record.progress == 1).then_some(device);
    }
''',
    "require one physical device for complete challenge",
)
replace_exact(
    security,
    '''fn record_device_removal(inner: &Arc<Mutex<SecurityInner>>, device: usize, now: Instant) {
''',
    '''#[cfg(all(target_os = "windows", not(test)))]
fn foreground_window_matches(expected_window: usize) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    unsafe {
        let foreground = GetForegroundWindow();
        !foreground.0.is_null() && foreground.0 as usize == expected_window
    }
}

fn record_device_removal(inner: &Arc<Mutex<SecurityInner>>, device: usize, now: Instant) {
''',
    "foreground renderer HWND proof helper",
)
replace_exact(
    security,
    '''fn record_device_removal(inner: &Arc<Mutex<SecurityInner>>, device: usize, now: Instant) {
    if device == 0 {
        return;
    }
    if let Ok(mut inner) = inner.lock() {
        prune_security_inner(&mut inner, now);
        inner.removed_devices.insert(device, now);
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
''',
    "device removal immediately invalidates only bound physical consent",
)
replace_exact(
    security,
    '''        state.remove_physical_device_for_test(0x7777);
        assert_eq!(
            state.consume_consent(&authority, &second.consent_token),
            Err(ConsentConsumeError::DeviceRemoved)
        );
''',
    '''        state.remove_physical_device_for_test(0x7777);
        assert_eq!(
            state.consume_consent(&authority, &second.consent_token),
            Err(ConsentConsumeError::Replayed)
        );
''',
    "existing device-removal test expects immediate revocation",
)
replace_exact(
    security,
    '''            // `IMO_HARDWARE` is defense in depth, not our sole proof:
            // the authoritative evidence below is a real WM_INPUT keyboard
            // record with a non-null hDevice that is present in the current
            // GetRawInputDeviceList enumeration.
            let mut source = INPUT_MESSAGE_SOURCE::default();
''',
    '''            // `IMO_HARDWARE` is defense in depth, not our sole proof:
            // the authoritative evidence below is a real WM_INPUT keyboard
            // record with a non-null hDevice that is present in the current
            // GetRawInputDeviceList enumeration. Foreground renderer binding
            // is checked atomically with the active challenge in record_physical_digit.
            let mut source = INPUT_MESSAGE_SOURCE::default();
''',
    "document layered physical-input and foreground proof",
)
replace_exact(
    security,
    '''            caller: ConsentCallerBinding {
                principal: "desktop:main:1".to_string(),
                window_label: "main".to_string(),
                owner_incarnation: 1,
            },
            operation_id: operation.to_string(),
''',
    '''            caller: ConsentCallerBinding {
                principal: "desktop:main:1".to_string(),
                window_label: "main".to_string(),
                owner_incarnation: 1,
            },
            foreground_window: 0x1234,
            operation_id: operation.to_string(),
''',
    "test consent binding includes renderer HWND",
)
replace_exact(
    security,
    '''    #[test]
    fn prepared_consent_rejects_other_r4_operations() {
''',
    '''    #[test]
    fn prepared_consent_requires_one_enumerated_physical_device_for_the_whole_code() {
        let state = ControlPlaneSecurityState::default();
        let authority = binding(OUTPUT_BLACKOUT_RELEASE_OPERATION_ID, 9);
        let prepared = state.prepare_consent(authority.clone()).unwrap();
        let digits = prepared
            .display_code
            .bytes()
            .map(|digit| digit - b'0')
            .collect::<Vec<_>>();
        for digit in &digits[..CHALLENGE_DIGITS - 1] {
            state.observe_physical_digit_for_test(*digit, 0x1111);
        }
        state.observe_physical_digit_for_test(digits[CHALLENGE_DIGITS - 1], 0x2222);
        assert_eq!(
            state
                .consent_status(&authority.caller, &prepared.challenge_id)
                .unwrap()
                .state,
            PreparedConsentState::PendingPhysicalInput
        );
        state.observe_physical_digit_for_test(digits[CHALLENGE_DIGITS - 1], 0x1111);
        assert_eq!(
            state
                .consent_status(&authority.caller, &prepared.challenge_id)
                .unwrap()
                .state,
            PreparedConsentState::Ready
        );
        // Unrelated removal notifications are ignored and do not accumulate
        // retained device state.
        state.remove_physical_device_for_test(0x2222);
        assert!(!state.inner.lock().unwrap().removed_devices.contains_key(&0x2222));
        state.remove_physical_device_for_test(0x1111);
        state.observe_physical_digit_for_test(1, 0x1111);
        assert_eq!(
            state.consent_status(&authority.caller, &prepared.challenge_id),
            Err(ConsentConsumeError::Missing)
        );
        assert_eq!(
            state.consume_consent(&authority, &prepared.consent_token),
            Err(ConsentConsumeError::Replayed)
        );

        let partial_state = ControlPlaneSecurityState::default();
        let partial = partial_state.prepare_consent(authority.clone()).unwrap();
        let partial_digits = partial
            .display_code
            .bytes()
            .map(|digit| digit - b'0')
            .collect::<Vec<_>>();
        partial_state.observe_physical_digit_for_test(partial_digits[0], 0x3333);
        partial_state.remove_physical_device_for_test(0x3333);
        for digit in &partial_digits[1..] {
            partial_state.observe_physical_digit_for_test(*digit, 0x3333);
            partial_state.observe_physical_digit_for_test(*digit, 0x4444);
        }
        assert_eq!(
            partial_state.consent_status(&authority.caller, &partial.challenge_id),
            Err(ConsentConsumeError::Missing)
        );
        assert_eq!(
            partial_state.consume_consent(&authority, &partial.consent_token),
            Err(ConsentConsumeError::Replayed)
        );
    }

    #[test]
    fn prepared_consent_rejects_other_r4_operations() {
''',
    "single-device physical consent focused test",
)

# Bind the prepared consent to the exact native renderer window that displayed
# the challenge. The same HWND is rebuilt for execution and participates in the
# backend-only ConsentAuthorityBinding equality check.
replace_exact(
    runtime,
    '''    let binding = capture_binding(state, window.label())
        .map_err(|_| "Output consent caller is not registered".to_string())?;
    query_state
''',
    '''    let binding = capture_binding(state, window.label())
        .map_err(|_| "Output consent caller is not registered".to_string())?;
    let foreground_window = blackout_release_native_window_handle(window)?;
    query_state
''',
    "capture native renderer HWND before Release consent preflight",
)
replace_exact(
    runtime,
    '''    let authority = ConsentAuthorityBinding {
        caller: ConsentCallerBinding {
            principal: binding.principal,
            window_label: binding.window_label,
            owner_incarnation: binding.owner_incarnation,
        },
        operation_id: request.action.operation_id().to_string(),
''',
    '''    let authority = ConsentAuthorityBinding {
        caller: ConsentCallerBinding {
            principal: binding.principal,
            window_label: binding.window_label,
            owner_incarnation: binding.owner_incarnation,
        },
        foreground_window,
        operation_id: request.action.operation_id().to_string(),
''',
    "prepared Release consent stores native renderer HWND",
)
replace_exact(
    runtime,
    '''    let binding = match capture_binding(state, window.label()) {
        Ok(binding) => binding,
        Err(_) => return output_control_rejection(&request, OutputControlErrorCodeV1::Forbidden),
    };
    let key = output_control_receipt_key(&request, &binding);
''',
    '''    let binding = match capture_binding(state, window.label()) {
        Ok(binding) => binding,
        Err(_) => return output_control_rejection(&request, OutputControlErrorCodeV1::Forbidden),
    };
    let foreground_window = match blackout_release_native_window_handle(window) {
        Ok(window) => window,
        Err(_) => return output_control_rejection(&request, OutputControlErrorCodeV1::Internal),
    };
    let key = output_control_receipt_key(&request, &binding);
''',
    "execution rebuilds native renderer HWND binding",
)
replace_exact(
    runtime,
    '''    let consent_binding = ConsentAuthorityBinding {
        caller: ConsentCallerBinding {
            principal: binding.principal.clone(),
            window_label: binding.window_label.clone(),
            owner_incarnation: binding.owner_incarnation,
        },
        operation_id: request.operation_id.clone(),
''',
    '''    let consent_binding = ConsentAuthorityBinding {
        caller: ConsentCallerBinding {
            principal: binding.principal.clone(),
            window_label: binding.window_label.clone(),
            owner_incarnation: binding.owner_incarnation,
        },
        foreground_window,
        operation_id: request.operation_id.clone(),
''',
    "execution consent binding includes native renderer HWND",
)
replace_exact(
    runtime,
    '''fn exact_output_control_fence_matches(
    state: &AppState,
''',
    '''fn blackout_release_native_window_handle(window: &WebviewWindow) -> Result<usize, String> {
    #[cfg(target_os = "windows")]
    {
        let hwnd = window
            .hwnd()
            .map_err(|error| format!("Blackout Release renderer HWND is unavailable: {error}"))?;
        let value = hwnd.0 as usize;
        if value == 0 {
            Err("Blackout Release renderer HWND is unavailable".to_string())
        } else {
            Ok(value)
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;
        Err("Blackout Release physical consent is available only on Windows".to_string())
    }
}

fn exact_output_control_fence_matches(
    state: &AppState,
''',
    "native Release renderer HWND helper",
)
