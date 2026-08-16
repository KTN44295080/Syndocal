from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


security = Path("app/src-tauri/src/control_plane_security.rs")
main = Path("app/src-tauri/src/main.rs")

replace_exact(
    security,
    '''    pub(crate) fn retire_principal(&self, principal: &str) {
        let now = Instant::now();
        if let Ok(mut inner) = self.inner.lock() {
            prune_security_inner(&mut inner, now);
            if inner
                .active
                .as_ref()
                .is_some_and(|record| record.binding.caller.principal == principal)
            {
                if let Some(record) = inner.active.take() {
                    push_tombstone(&mut inner, record.consent_token, now);
                }
            }
        }
    }
''',
    '''    pub(crate) fn retire_principal(&self, principal: &str) {
        let now = Instant::now();
        if let Ok(mut inner) = self.inner.lock() {
            prune_security_inner(&mut inner, now);
            if inner
                .active
                .as_ref()
                .is_some_and(|record| record.binding.caller.principal == principal)
            {
                if let Some(record) = inner.active.take() {
                    push_tombstone(&mut inner, record.consent_token, now);
                }
            }
        }
    }

    pub(crate) fn retire_all(&self) {
        let now = Instant::now();
        if let Ok(mut inner) = self.inner.lock() {
            prune_security_inner(&mut inner, now);
            if let Some(record) = inner.active.take() {
                push_tombstone(&mut inner, record.consent_token, now);
            }
        }
    }
''',
    "global consent retirement",
)

replace_exact(
    security,
    '''    #[test]
    fn desktop_session_lock_invalidates_ready_release_consent() {
''',
    '''    #[test]
    fn project_authority_replacement_invalidates_ready_release_consent() {
        let state = ControlPlaneSecurityState::default();
        let authority = binding(OUTPUT_BLACKOUT_RELEASE_OPERATION_ID, 9);
        let prepared = state.prepare_consent(authority.clone()).unwrap();
        enter_code(&state, &prepared.display_code, 0x7777);
        state.retire_all();
        assert_eq!(
            state.consume_consent(&authority, &prepared.consent_token),
            Err(ConsentConsumeError::Replayed)
        );
    }

    #[test]
    fn desktop_session_lock_invalidates_ready_release_consent() {
''',
    "project replacement consent invalidation test",
)

replace_exact(
    main,
    '''    replacement_result.authority = Some(authority.clone());
    if let Ok(app) = project_swap_app_handle(state) {
''',
    '''    replacement_result.authority = Some(authority.clone());
    state.control_plane_security.retire_all();
    if let Ok(app) = project_swap_app_handle(state) {
''',
    "Undo/Redo project authority replacement invalidates consent",
)

replace_exact(
    main,
    '''    // An emit fault cannot invalidate an already-acknowledged publication.
    // The periodic authority refresh remains a recovery path, while this
''',
    '''    if coordinator_effect != ProjectReplacementCoordinatorEffect::RuntimeSanitize {
        state.control_plane_security.retire_all();
    }
    // An emit fault cannot invalidate an already-acknowledged publication.
    // The periodic authority refresh remains a recovery path, while this
''',
    "committed project replacement invalidates consent",
)

# Full Lock is an explicit operator safety boundary. A challenge prepared before
# entering Full Lock must not become usable again merely because the project is
# unlocked within the token TTL. Retire it at the lock transition; Partial Lock
# remains compatible with R4 and therefore does not revoke consent.
replace_exact(
    main,
    '''    let session = project_operator_session_for_policy(&mut sessions, &coordinator, &owner_id)
        .ok_or_else(|| "Configure an Operator policy before locking this project".to_string())?;
    session.unlocked = false;
    Ok(session.policy.lock_mode)
}
''',
    '''    let session = project_operator_session_for_policy(&mut sessions, &coordinator, &owner_id)
        .ok_or_else(|| "Configure an Operator policy before locking this project".to_string())?;
    let lock_mode = session.policy.lock_mode;
    session.unlocked = false;
    drop(sessions);
    if lock_mode == OperatorLockMode::Full {
        state.control_plane_security.retire_all();
    }
    Ok(lock_mode)
}
''',
    "Full Lock invalidates prepared R4 consent",
)
