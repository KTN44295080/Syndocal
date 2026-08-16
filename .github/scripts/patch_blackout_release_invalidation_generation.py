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
    '''struct SecurityInner {
    active: Option<ConsentRecord>,
    tombstones: VecDeque<ConsentTombstone>,
    removed_devices: HashMap<usize, Instant>,
}
''',
    '''struct SecurityInner {
    active: Option<ConsentRecord>,
    tombstones: VecDeque<ConsentTombstone>,
    removed_devices: HashMap<usize, Instant>,
    invalidation_generation: u64,
}
''',
    "security invalidation generation",
)

replace_exact(
    security,
    '''    pub(crate) fn prepare_consent(
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
        if let Some(previous) = inner.active.take() {
            push_tombstone(&mut inner, previous.consent_token, now);
        }
        inner.active = Some(ConsentRecord {
            binding,
            challenge_id: challenge_id.clone(),
            consent_token: consent_token.clone(),
            display_code,
            progress: 0,
            progress_device: None,
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
''',
    '''    pub(crate) fn invalidation_generation(&self) -> Result<u64, String> {
        self.inner
            .lock()
            .map(|inner| inner.invalidation_generation)
            .map_err(|_| "Consent state lock was poisoned".to_string())
    }

    pub(crate) fn prepare_consent(
        &self,
        binding: ConsentAuthorityBinding,
    ) -> Result<PreparedConsentChallenge, String> {
        let generation = self.invalidation_generation()?;
        self.prepare_consent_at_generation(binding, generation)
    }

    pub(crate) fn prepare_consent_at_generation(
        &self,
        binding: ConsentAuthorityBinding,
        expected_invalidation_generation: u64,
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
        if inner.invalidation_generation != expected_invalidation_generation {
            return Err("Consent authority was invalidated before preparation completed".to_string());
        }
        prune_security_inner(&mut inner, now);
        if let Some(previous) = inner.active.take() {
            push_tombstone(&mut inner, previous.consent_token, now);
        }
        inner.active = Some(ConsentRecord {
            binding,
            challenge_id: challenge_id.clone(),
            consent_token: consent_token.clone(),
            display_code,
            progress: 0,
            progress_device: None,
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
''',
    "linearizable consent preparation generation",
)

replace_exact(
    security,
    '''fn prune_security_inner(inner: &mut SecurityInner, now: Instant) {
''',
    '''fn invalidate_security_generation(inner: &mut SecurityInner) {
    inner.invalidation_generation = inner.invalidation_generation.wrapping_add(1);
}

fn prune_security_inner(inner: &mut SecurityInner, now: Instant) {
''',
    "security generation invalidation helper",
)

# Principal retirement is a revocation boundary even if the retiring principal
# had no active challenge at the exact lock acquisition instant.
replace_exact(
    security,
    '''        if let Ok(mut inner) = self.inner.lock() {
            prune_security_inner(&mut inner, now);
            if inner
                .active
                .as_ref()
                .is_some_and(|record| record.binding.caller.principal == principal)
''',
    '''        if let Ok(mut inner) = self.inner.lock() {
            invalidate_security_generation(&mut inner);
            prune_security_inner(&mut inner, now);
            if inner
                .active
                .as_ref()
                .is_some_and(|record| record.binding.caller.principal == principal)
''',
    "principal retirement bumps consent generation",
)
replace_exact(
    security,
    '''    pub(crate) fn retire_all(&self) {
        let now = Instant::now();
        if let Ok(mut inner) = self.inner.lock() {
            prune_security_inner(&mut inner, now);
''',
    '''    pub(crate) fn retire_all(&self) {
        let now = Instant::now();
        if let Ok(mut inner) = self.inner.lock() {
            invalidate_security_generation(&mut inner);
            prune_security_inner(&mut inner, now);
''',
    "global retirement bumps consent generation",
)
replace_exact(
    security,
    '''    if let Ok(mut inner) = inner.lock() {
        prune_security_inner(&mut inner, now);
        if let Some(record) = inner.active.take() {
            push_tombstone(&mut inner, record.consent_token, now);
        }
    }
}

fn random_token_128() -> Result<String, String> {
''',
    '''    if let Ok(mut inner) = inner.lock() {
        invalidate_security_generation(&mut inner);
        prune_security_inner(&mut inner, now);
        if let Some(record) = inner.active.take() {
            push_tombstone(&mut inner, record.consent_token, now);
        }
    }
}

fn random_token_128() -> Result<String, String> {
''',
    "desktop/remote session transition bumps consent generation",
)

# Capture the generation before project/output/operator preflight. Any
# revocation/Standby/project replacement/Full Lock/session transition which
# races that preflight changes the generation and makes token minting fail.
replace_exact(
    runtime,
    '''    request
        .validate()
        .map_err(|_| "Output consent request is invalid".to_string())?;
    let binding = capture_binding(state, window.label())
''',
    '''    request
        .validate()
        .map_err(|_| "Output consent request is invalid".to_string())?;
    let consent_invalidation_generation =
        state.control_plane_security.invalidation_generation()?;
    let binding = capture_binding(state, window.label())
''',
    "capture consent invalidation generation before preflight",
)
replace_exact(
    runtime,
    '''    let prepared = state.control_plane_security.prepare_consent(authority)?;
''',
    '''    let prepared = state
        .control_plane_security
        .prepare_consent_at_generation(authority, consent_invalidation_generation)?;
''',
    "mint consent only if no invalidation raced preflight",
)

# Extend the already-gated project replacement test with the generation-race
# proof: a prepare begun before revocation cannot mint afterward.
replace_exact(
    security,
    '''        let prepared = state.prepare_consent(authority.clone()).unwrap();
        enter_code(&state, &prepared.display_code, 0x7777);
        state.retire_all();
        assert_eq!(
            state.consume_consent(&authority, &prepared.consent_token),
            Err(ConsentConsumeError::Replayed)
        );
''',
    '''        let prepared = state.prepare_consent(authority.clone()).unwrap();
        enter_code(&state, &prepared.display_code, 0x7777);
        let preflight_generation = state.invalidation_generation().unwrap();
        state.retire_all();
        assert_eq!(
            state.consume_consent(&authority, &prepared.consent_token),
            Err(ConsentConsumeError::Replayed)
        );
        assert!(state
            .prepare_consent_at_generation(authority.clone(), preflight_generation)
            .is_err());
''',
    "project replacement generation race focused proof",
)
