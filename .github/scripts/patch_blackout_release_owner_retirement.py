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
    '''    pub(crate) fn retire_caller(&self, caller: &ConsentCallerBinding) {
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
''',
    "principal-scoped consent retirement",
)

replace_exact(
    security,
    '''    #[test]
    fn prepared_consent_rejects_other_r4_operations() {
''',
    '''    #[test]
    fn renderer_principal_retirement_invalidates_ready_release_consent() {
        let state = ControlPlaneSecurityState::default();
        let authority = binding(OUTPUT_BLACKOUT_RELEASE_OPERATION_ID, 9);
        let prepared = state.prepare_consent(authority.clone()).unwrap();
        enter_code(&state, &prepared.display_code, 0x8888);
        state.retire_principal(&authority.caller.principal);
        assert_eq!(
            state.consume_consent(&authority, &prepared.consent_token),
            Err(ConsentConsumeError::Replayed)
        );
    }

    #[test]
    fn prepared_consent_rejects_other_r4_operations() {
''',
    "principal retirement focused test",
)

text = main.read_text(encoding="utf-8")
old = '''            state.authored_control_plane.retire_principal(retired_owner);
            state.runtime_control_plane.retire_principal(retired_owner);
'''
new = '''            state.authored_control_plane.retire_principal(retired_owner);
            state.runtime_control_plane.retire_principal(retired_owner);
            state.control_plane_security.retire_principal(retired_owner);
'''
count = text.count(old)
if count != 2:
    raise RuntimeError(f"indented renderer retirement seams: expected 2 matches, found {count}")
text = text.replace(old, new)
old = '''        state.authored_control_plane.retire_principal(retired_owner);
        state.runtime_control_plane.retire_principal(retired_owner);
'''
new = '''        state.authored_control_plane.retire_principal(retired_owner);
        state.runtime_control_plane.retire_principal(retired_owner);
        state.control_plane_security.retire_principal(retired_owner);
'''
count = text.count(old)
if count != 2:
    raise RuntimeError(f"renderer retirement seams: expected 2 matches, found {count}")
main.write_text(text.replace(old, new), encoding="utf-8")
