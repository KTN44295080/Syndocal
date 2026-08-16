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
    // Once a challenge is Ready, removing its matched device permanently
    // invalidates that token. A later input event with a recycled OS handle
    // must not clear the removal tombstone and resurrect the proof.
    if inner
        .active
        .as_ref()
        .is_some_and(|record| record.matched_device.is_some())
    {
        return;
    }
    inner.removed_devices.remove(&device);
    let Some(record) = inner.active.as_mut() else {
        return;
    };
    if record.expires_at <= now {
        return;
    }
''',
    "ready device removal cannot resurrect on handle reuse",
)
replace_exact(
    security,
    '''    if record.expires_at <= now {
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
    '''    if record.expires_at <= now {
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
    '''    if let Ok(mut inner) = inner.lock() {
        prune_security_inner(&mut inner, now);
        inner.removed_devices.insert(device, now);
    }
''',
    '''    if let Ok(mut inner) = inner.lock() {
        prune_security_inner(&mut inner, now);
        if let Some(record) = inner.active.as_mut() {
            if record.matched_device.is_none() && record.progress_device == Some(device) {
                record.progress = 0;
                record.progress_device = None;
            }
        }
        inner.removed_devices.insert(device, now);
    }
''',
    "reset incomplete challenge when its keyboard disappears",
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
        // Simulate a later input event whose OS handle has been recycled to the
        // same numeric value. The already-ready challenge must stay invalid.
        state.observe_physical_digit_for_test(1, 0x1111);
        assert_eq!(
            state.consume_consent(&authority, &prepared.consent_token),
            Err(ConsentConsumeError::DeviceRemoved)
        );
    }

    #[test]
    fn prepared_consent_rejects_other_r4_operations() {
''',
    "single-device physical consent focused test",
)
