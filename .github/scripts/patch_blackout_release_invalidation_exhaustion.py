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
    '''fn invalidate_security_generation(inner: &mut SecurityInner) {
    inner.invalidation_generation = inner.invalidation_generation.wrapping_add(1);
}
''',
    '''fn invalidate_security_generation(inner: &mut SecurityInner) {
    inner.invalidation_generation = inner
        .invalidation_generation
        .checked_add(1)
        .unwrap_or(u64::MAX);
}
''',
    "non-wrapping consent invalidation generation",
)

replace_exact(
    security,
    '''        if inner.invalidation_generation != expected_invalidation_generation {
            return Err("Consent authority was invalidated before preparation completed".to_string());
        }
''',
    '''        if inner.invalidation_generation == u64::MAX {
            return Err("Consent invalidation generation is exhausted".to_string());
        }
        if inner.invalidation_generation != expected_invalidation_generation {
            return Err("Consent authority was invalidated before preparation completed".to_string());
        }
''',
    "fail closed when consent invalidation generation is exhausted",
)

replace_exact(
    security,
    '''        assert!(state
            .prepare_consent_at_generation(authority.clone(), preflight_generation)
            .is_err());
''',
    '''        assert!(state
            .prepare_consent_at_generation(authority.clone(), preflight_generation)
            .is_err());

        {
            let mut inner = state.inner.lock().unwrap();
            inner.invalidation_generation = u64::MAX;
        }
        assert!(state
            .prepare_consent_at_generation(authority.clone(), u64::MAX)
            .is_err());
''',
    "focused exhausted invalidation generation proof",
)
