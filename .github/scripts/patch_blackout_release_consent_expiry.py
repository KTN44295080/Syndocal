from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


security = Path("app/src-tauri/src/control_plane_security.rs")
control = Path("app/src/components/BlackoutReleaseControl.tsx")

replace_exact(
    security,
    '''    pub(crate) fn consent_status(
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
''',
    '''    pub(crate) fn consent_status(
        &self,
        caller: &ConsentCallerBinding,
        challenge_id: &str,
    ) -> Result<PreparedConsentStatus, ConsentConsumeError> {
        self.consent_status_at(caller, challenge_id, Instant::now())
    }

    fn consent_status_at(
        &self,
        caller: &ConsentCallerBinding,
        challenge_id: &str,
        now: Instant,
    ) -> Result<PreparedConsentStatus, ConsentConsumeError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| ConsentConsumeError::Internal)?;
        let matching_expired = inner.active.as_ref().is_some_and(|record| {
            record.challenge_id == challenge_id
                && &record.binding.caller == caller
                && record.expires_at <= now
        });
        if matching_expired {
            if let Some(record) = inner.active.take() {
                push_tombstone(&mut inner, record.consent_token, now);
            }
            return Err(ConsentConsumeError::Expired);
        }
        prune_security_inner(&mut inner, now);
        let Some(record) = inner.active.as_ref() else {
            return Err(ConsentConsumeError::Missing);
        };
        if record.challenge_id != challenge_id || &record.binding.caller != caller {
            return Err(ConsentConsumeError::WrongBinding);
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
        self.consume_consent_at(binding, consent_token, Instant::now())
    }

    fn consume_consent_at(
        &self,
        binding: &ConsentAuthorityBinding,
        consent_token: &str,
        now: Instant,
    ) -> Result<(), ConsentConsumeError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| ConsentConsumeError::Internal)?;
        let matching_expired = inner.active.as_ref().is_some_and(|record| {
            &record.binding == binding
                && record.consent_token == consent_token
                && record.expires_at <= now
        });
        if matching_expired {
            if let Some(record) = inner.active.take() {
                push_tombstone(&mut inner, record.consent_token, now);
            }
            return Err(ConsentConsumeError::Expired);
        }
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
''',
    "typed monotonic consent expiry",
)

replace_exact(
    security,
    '''    #[test]
    fn new_challenge_retires_old_and_device_removal_invalidates_ready_token() {
''',
    '''    #[test]
    fn prepared_consent_expiry_is_monotonic_typed_and_then_tombstoned() {
        let status_state = ControlPlaneSecurityState::default();
        let authority = binding(OUTPUT_BLACKOUT_RELEASE_OPERATION_ID, 9);
        let prepared = status_state.prepare_consent(authority.clone()).unwrap();
        let expired_at = Instant::now() + CONSENT_CHALLENGE_TTL + Duration::from_millis(1);
        assert_eq!(
            status_state.consent_status_at(
                &authority.caller,
                &prepared.challenge_id,
                expired_at,
            ),
            Err(ConsentConsumeError::Expired)
        );
        assert_eq!(
            status_state.consume_consent_at(
                &authority,
                &prepared.consent_token,
                expired_at,
            ),
            Err(ConsentConsumeError::Replayed)
        );

        let consume_state = ControlPlaneSecurityState::default();
        let prepared = consume_state.prepare_consent(authority.clone()).unwrap();
        enter_code(&consume_state, &prepared.display_code, 0x4444);
        assert_eq!(
            consume_state.consume_consent_at(
                &authority,
                &prepared.consent_token,
                Instant::now() + CONSENT_CHALLENGE_TTL + Duration::from_millis(1),
            ),
            Err(ConsentConsumeError::Expired)
        );
    }

    #[test]
    fn new_challenge_retires_old_and_device_removal_invalidates_ready_token() {
''',
    "monotonic consent expiry focused tests",
)

replace_exact(
    control,
    '''      while (currentGeneration === generation) {
        if (Date.now() >= consent.challenge.expires_at_unix_ms) {
          throw new Error("Physical confirmation expired. Start a new blackout release request.");
        }
        const state = await releaseRuntime.consentStatus(consent);
''',
    '''      while (currentGeneration === generation) {
        // Expiry is a backend monotonic-clock decision. The wall-clock expiry
        // is presentation metadata only and must not decide command validity.
        const state = await releaseRuntime.consentStatus(consent);
''',
    "remove renderer wall-clock consent expiry decision",
)
