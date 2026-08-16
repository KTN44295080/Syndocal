from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


security = Path("app/src-tauri/src/control_plane_security.rs")

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
    "removed challenge device cannot resume on handle reuse",
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
        inner.removed_devices.insert(device, now);
        let invalidates_active = inner.active.as_ref().is_some_and(|record| {
            record.progress_device == Some(device) || record.matched_device == Some(device)
        });
        if invalidates_active {
            if let Some(record) = inner.active.take() {
                push_tombstone(&mut inner, record.consent_token, now);
            }
        }
    }
}
''',
    "device removal immediately invalidates active physical consent",
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
    '''            let mut source = INPUT_MESSAGE_SOURCE::default();
            if GetCurrentInputMessageSource(&mut source).is_err()
                || source.originId != IMO_HARDWARE
                || source.deviceType != IMDT_KEYBOARD
            {
''',
    '''            // `IMO_HARDWARE` is defense in depth, not our sole proof:
            // Windows may classify input inserted by a UIAccess=true process
            // as hardware-origin. The authoritative evidence below is a real
            // WM_INPUT keyboard record with a non-null hDevice that is present
            // in GetRawInputDeviceList; Microsoft documents RDP devices as
            // absent from that raw-device list.
            let mut source = INPUT_MESSAGE_SOURCE::default();
            if GetCurrentInputMessageSource(&mut source).is_err()
                || source.originId != IMO_HARDWARE
                || source.deviceType != IMDT_KEYBOARD
            {
''',
    "document layered physical-input proof",
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
