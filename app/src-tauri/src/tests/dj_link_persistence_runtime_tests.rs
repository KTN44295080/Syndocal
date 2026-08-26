use super::*;
use crate::dj_link_machine::{
    dj_link_machine_settings_path, persist_dj_link_machine_settings_to_path, DjLinkCredentialError,
    DjLinkCredentialPersistenceScope, DjLinkMachineRollback, DjLinkMachineSettingsLoadOutcome,
    DjLinkMachineSettingsV2, DjLinkMachineTransactionState, DjLinkToken, DjLinkTokenRecord,
    DJ_LINK_MACHINE_SETTINGS_VERSION, DJ_LINK_TOKEN_LEN,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

static TEMP_DIR_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let sequence = TEMP_DIR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "syndocal-dj-link-persistence-{}-{}-{label}-{sequence}",
            process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&path).expect("create test directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn token(byte: u8) -> DjLinkToken {
    DjLinkToken::from_bytes(&[byte; DJ_LINK_TOKEN_LEN]).expect("nonzero token")
}

fn clone_record(record: &DjLinkTokenRecord) -> DjLinkTokenRecord {
    DjLinkTokenRecord {
        generation: record.generation,
        token: record.token.clone(),
    }
}

#[derive(Default)]
struct FakeCredentialStore {
    primary: Mutex<Option<DjLinkTokenRecord>>,
    rollback: Mutex<Option<DjLinkTokenRecord>>,
    primary_calls: AtomicUsize,
    rollback_calls: AtomicUsize,
    fail_primary_write_after_store: AtomicBool,
    fail_primary_revoke: AtomicBool,
    fail_rollback_revoke: AtomicBool,
    fail_rollback_revoke_on_call: Mutex<Option<usize>>,
}

impl FakeCredentialStore {
    fn with_primary(generation: u64, token: DjLinkToken) -> Self {
        Self {
            primary: Mutex::new(Some(DjLinkTokenRecord { generation, token })),
            ..Self::default()
        }
    }

    fn primary_generation(&self) -> Option<u64> {
        self.primary
            .lock()
            .expect("primary mutex")
            .as_ref()
            .map(|record| record.generation)
    }

    fn rollback_generation(&self) -> Option<u64> {
        self.rollback
            .lock()
            .expect("rollback mutex")
            .as_ref()
            .map(|record| record.generation)
    }

    fn api_calls(&self) -> usize {
        self.primary_calls.load(Ordering::Relaxed) + self.rollback_calls.load(Ordering::Relaxed)
    }

    fn fail_rollback_revoke_on_call(&self, call: usize) {
        *self
            .fail_rollback_revoke_on_call
            .lock()
            .expect("rollback revoke failure mutex") = Some(call);
    }
}

impl DjLinkCredentialStore for FakeCredentialStore {
    fn persistence_scope(&self) -> DjLinkCredentialPersistenceScope {
        DjLinkCredentialPersistenceScope::MachineLocal
    }

    fn save_token(
        &self,
        generation: u64,
        token: &DjLinkToken,
    ) -> Result<(), DjLinkCredentialError> {
        self.primary_calls.fetch_add(1, Ordering::Relaxed);
        *self.primary.lock().expect("primary mutex") = Some(DjLinkTokenRecord {
            generation,
            token: token.clone(),
        });
        if self
            .fail_primary_write_after_store
            .swap(false, Ordering::Relaxed)
        {
            return Err(DjLinkCredentialError::WriteFailed { code: -1 });
        }
        Ok(())
    }

    fn load_token(&self) -> Result<Option<DjLinkTokenRecord>, DjLinkCredentialError> {
        self.primary_calls.fetch_add(1, Ordering::Relaxed);
        Ok(self
            .primary
            .lock()
            .expect("primary mutex")
            .as_ref()
            .map(clone_record))
    }

    fn revoke_token(&self) -> Result<(), DjLinkCredentialError> {
        self.primary_calls.fetch_add(1, Ordering::Relaxed);
        if self.fail_primary_revoke.swap(false, Ordering::Relaxed) {
            return Err(DjLinkCredentialError::RevokeFailed { code: -2 });
        }
        *self.primary.lock().expect("primary mutex") = None;
        Ok(())
    }

    fn save_rollback_token(
        &self,
        generation: u64,
        token: &DjLinkToken,
    ) -> Result<(), DjLinkCredentialError> {
        self.rollback_calls.fetch_add(1, Ordering::Relaxed);
        *self.rollback.lock().expect("rollback mutex") = Some(DjLinkTokenRecord {
            generation,
            token: token.clone(),
        });
        Ok(())
    }

    fn load_rollback_token(&self) -> Result<Option<DjLinkTokenRecord>, DjLinkCredentialError> {
        self.rollback_calls.fetch_add(1, Ordering::Relaxed);
        Ok(self
            .rollback
            .lock()
            .expect("rollback mutex")
            .as_ref()
            .map(clone_record))
    }

    fn revoke_rollback_token(&self) -> Result<(), DjLinkCredentialError> {
        let call = self.rollback_calls.fetch_add(1, Ordering::Relaxed) + 1;
        let fail_on_call = {
            let mut pending = self
                .fail_rollback_revoke_on_call
                .lock()
                .expect("rollback revoke failure mutex");
            match *pending {
                Some(target) if target == call => {
                    *pending = None;
                    true
                }
                _ => false,
            }
        };
        if fail_on_call || self.fail_rollback_revoke.swap(false, Ordering::Relaxed) {
            return Err(DjLinkCredentialError::RevokeFailed { code: -3 });
        }
        *self.rollback.lock().expect("rollback mutex") = None;
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum PersistFailure {
    BeforeWrite,
    AfterWrite,
}

struct FakeSettingsPersistence {
    path: PathBuf,
    calls: AtomicUsize,
    fail_once: Mutex<Option<(usize, PersistFailure)>>,
}

impl FakeSettingsPersistence {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            calls: AtomicUsize::new(0),
            fail_once: Mutex::new(None),
        }
    }

    fn fail_on(&self, call: usize, failure: PersistFailure) {
        *self.fail_once.lock().expect("failure mutex") = Some((call, failure));
    }
}

impl DjLinkSettingsPersistence for FakeSettingsPersistence {
    fn persist(&self, settings: &DjLinkMachineSettingsV2) -> Result<(), String> {
        let call = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
        let failure = {
            let mut pending = self.fail_once.lock().expect("failure mutex");
            match *pending {
                Some((target, failure)) if target == call => {
                    *pending = None;
                    Some((target, failure))
                }
                _ => None,
            }
        };
        if matches!(failure, Some((target, PersistFailure::BeforeWrite)) if target == call) {
            return Err(format!("injected settings failure before call {call}"));
        }
        persist_dj_link_machine_settings_to_path(&self.path, settings)?;
        if matches!(failure, Some((target, PersistFailure::AfterWrite)) if target == call) {
            return Err(format!("injected settings failure after call {call}"));
        }
        Ok(())
    }
}

struct FixedTokenGenerator {
    result: Mutex<Result<DjLinkToken, String>>,
}

impl FixedTokenGenerator {
    fn succeeds(byte: u8) -> Self {
        Self {
            result: Mutex::new(Ok(token(byte))),
        }
    }

    fn fails() -> Self {
        Self {
            result: Mutex::new(Err("injected token generation failure".to_string())),
        }
    }
}

impl DjLinkTokenGenerator for FixedTokenGenerator {
    fn generate(&self) -> Result<DjLinkToken, String> {
        self.result
            .lock()
            .expect("generator mutex")
            .as_ref()
            .map(Clone::clone)
            .map_err(Clone::clone)
    }
}

fn stable_settings() -> DjLinkMachineSettingsV2 {
    DjLinkMachineSettingsV2 {
        version: DJ_LINK_MACHINE_SETTINGS_VERSION,
        revision: 7,
        transaction_state: DjLinkMachineTransactionState::Idle,
        credential_generation: Some(7),
        credential_generation_high_water: 7,
        adapter_guid: Some("11111111-1111-1111-1111-111111111111".to_string()),
        network_guid: Some("22222222-2222-2222-2222-222222222222".to_string()),
        bind_ip: Some("192.0.2.10".to_string()),
        bind_port: Some(9100),
        auto_start_armed: true,
        disarm_cleanup_pending: false,
        rollback: None,
    }
    .validated()
    .expect("valid stable settings")
}

fn seeded_authority(path: PathBuf, settings: DjLinkMachineSettingsV2) -> DjLinkMachineAuthority {
    DjLinkMachineAuthority {
        settings_path: Some(path),
        settings,
        block_reason: None,
        restored_credential: true,
        credential_cleanup_pending: false,
        wire_token: None,
    }
}

fn assert_old_authority_restored(path: &Path, credentials: &FakeCredentialStore) {
    match load_dj_link_machine_settings_from_path(path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => {
            let accepted = stable_settings();
            assert_eq!(
                settings.transaction_state,
                DjLinkMachineTransactionState::Idle
            );
            assert!(settings.rollback.is_none());
            // A generator failure occurs before the transaction has any durable
            // work to compensate, so the accepted generation may remain exact.
            assert!(settings.revision >= accepted.revision);
            assert!(
                settings.credential_generation_high_water
                    >= accepted.credential_generation_high_water
            );
            assert_eq!(
                settings.credential_generation,
                accepted.credential_generation
            );
            assert_eq!(settings.adapter_guid, accepted.adapter_guid);
            assert_eq!(settings.network_guid, accepted.network_guid);
            assert_eq!(settings.bind_ip, accepted.bind_ip);
            assert_eq!(settings.bind_port, accepted.bind_port);
            assert_eq!(settings.auto_start_armed, accepted.auto_start_armed);
        }
        other => panic!("expected restored stable settings, got {other:?}"),
    }
    assert_eq!(credentials.primary_generation(), Some(7));
    assert_eq!(credentials.rollback_generation(), None);
}

#[test]
fn authority_debug_redacts_known_wire_token() {
    let sentinel = "known-dj-link-wire-token-must-not-log";
    let mut authority = DjLinkMachineAuthority::default();
    authority.wire_token = Some(sentinel.to_string());
    let rendered = format!("{authority:?}");
    assert!(
        !rendered.contains(sentinel),
        "authority debug leaked wire token: {rendered}"
    );
    assert!(
        rendered.contains("REDACTED"),
        "authority debug must show redaction: {rendered}"
    );
}

#[test]
fn status_is_non_secret_and_default_is_fail_closed() {
    let status = DjLinkMachineAuthority::default().status();
    assert!(!status.configured);
    assert!(!status.auto_start_armed);
    assert!(!status.credential_ready);
    assert!(status.block_reason.is_none());
    let encoded = serde_json::to_string(&status).expect("status serializes");
    assert!(!encoded.contains("token"));
}

#[test]
fn config_requires_explicit_arm_and_never_enables_web_remote() {
    let mut authority = DjLinkMachineAuthority::default();
    assert!(authority.dj_only_config().is_err());
    authority.settings = stable_settings();
    authority.restored_credential = true;
    authority.install_test_wire_token("0123456789abcdef0123456789abcdef");
    let config = authority.dj_only_config().expect("armed config");
    assert!(config.dj_link_enabled);
    assert!(!config.web_remote_enabled);
    assert_eq!(config.pairing_pin, "");
    assert_eq!(config.bind_ip, "192.0.2.10");
}

#[test]
fn generic_listener_preparation_uses_the_production_entrypoints_without_nlm() {
    let mut authority = DjLinkMachineAuthority::default();
    let mut config = RemoteControlConfig::default();
    assert!(authority.prepare_listener_config(&mut config).is_ok());
    assert!(config.dj_link_token.is_none());
}

#[test]
fn armed_authority_rejects_web_only_listener_but_allows_exact_dj_or_hybrid_listener() {
    let mut authority = DjLinkMachineAuthority::default();
    authority.settings = stable_settings();
    authority.restored_credential = true;
    authority.install_test_wire_token("0123456789abcdef0123456789abcdef");

    let mut web_only = RemoteControlConfig::default();
    assert!(authority
        .prepare_listener_config_with(&mut web_only, &|_, _, _| Ok(()))
        .is_err());
    assert!(authority.status().auto_start_armed);
    assert!(authority.status().credential_ready);

    let mut exact_dj = authority.dj_only_config().expect("exact DJ config");
    assert!(authority
        .prepare_listener_config_with(&mut exact_dj, &|_, _, _| Ok(()))
        .is_ok());
    let mut hybrid = exact_dj.clone();
    hybrid.web_remote_enabled = true;
    assert!(authority
        .prepare_listener_config_with(&mut hybrid, &|_, _, _| Ok(()))
        .is_ok());

    authority.settings.auto_start_armed = false;
    authority.restored_credential = false;
    assert!(authority
        .prepare_listener_config_with(&mut web_only, &|_, _, _| Ok(()))
        .is_ok());
}

#[test]
fn listener_validation_uses_the_production_policy_with_an_injected_fresh_observer() {
    let mut authority = DjLinkMachineAuthority::default();
    authority.settings = stable_settings();
    authority.restored_credential = true;
    authority.install_test_wire_token("0123456789abcdef0123456789abcdef");
    let config = authority.dj_only_config().expect("armed config");
    assert!(authority
        .validate_listener_config_with(&config, &|network, adapter, ip| {
            assert_eq!(network, "22222222-2222-2222-2222-222222222222");
            assert_eq!(adapter, "11111111-1111-1111-1111-111111111111");
            assert_eq!(ip, "192.0.2.10");
            Ok(())
        })
        .is_ok());

    let mut mismatched = config.clone();
    mismatched.port = 9101;
    assert!(authority
        .validate_listener_config_with(&mismatched, &|_, _, _| Ok(()))
        .is_err());
    assert_eq!(
        authority.block_reason.as_deref(),
        Some("listener_config_mismatch")
    );

    authority.block_reason = None;
    assert!(authority
        .validate_listener_config_with(&config, &|_, _, _| {
            Err("network_binding_stale".to_string())
        })
        .is_err());
    assert_eq!(
        authority.block_reason.as_deref(),
        Some("network_binding_stale")
    );
}

#[test]
fn protected_settings_disarm_preserves_bytes_and_skips_all_credential_calls() {
    for (label, body, expected_reason) in [
        (
            "corrupt",
            b"{bad settings".as_slice(),
            "corrupt_machine_settings",
        ),
        (
            "future",
            br#"{"version":3,"revision":1,"transaction_state":"idle","auto_start_armed":false}"#
                .as_slice(),
            "future_machine_settings_version",
        ),
        (
            "retired",
            br#"{"version":1,"revision":1,"transaction_state":"idle","auto_start_armed":false}"#
                .as_slice(),
            "retired_machine_settings_version",
        ),
    ] {
        let directory = TempDir::new(label);
        let path = dj_link_machine_settings_path(directory.path());
        fs::write(&path, body).expect("seed protected bytes");
        let before = fs::read(&path).expect("read before");
        let mut authority = DjLinkMachineAuthority::default();
        authority.initialize(directory.path());
        assert_eq!(authority.block_reason.as_deref(), Some(expected_reason));
        let persistence = FakeSettingsPersistence::new(path.clone());
        let credentials = FakeCredentialStore::default();
        assert!(authority
            .disarm_with_dependencies(&persistence, &credentials)
            .is_err());
        assert_eq!(fs::read(&path).expect("read after"), before, "{label}");
        assert_eq!(credentials.api_calls(), 0, "{label} must not touch CredMan");
        assert_eq!(
            persistence.calls.load(Ordering::Relaxed),
            0,
            "{label} must not write"
        );
    }
}

#[test]
fn disarm_final_settings_failure_clears_process_credential_truth() {
    let directory = TempDir::new("disarm-final-persist");
    let path = dj_link_machine_settings_path(directory.path());
    persist_dj_link_machine_settings_to_path(&path, &stable_settings()).expect("seed settings");
    let persistence = FakeSettingsPersistence::new(path.clone());
    persistence.fail_on(2, PersistFailure::BeforeWrite);
    let credentials = FakeCredentialStore::with_primary(7, token(0x11));
    let mut authority = seeded_authority(path.clone(), stable_settings());
    authority.wire_token = Some("must-clear-after-revoke".to_string());
    assert!(authority
        .disarm_with_dependencies(&persistence, &credentials)
        .is_err());
    let status = authority.status();
    assert!(!status.credential_ready);
    assert_eq!(
        status.block_reason.as_deref(),
        Some("disarm_cleanup_pending")
    );
    assert!(authority.wire_token.is_none());
    assert_eq!(credentials.primary_generation(), None);
}

#[test]
fn disarm_primary_revoke_failure_is_visibly_disarmed_without_injection() {
    let directory = TempDir::new("disarm-primary-revoke");
    let path = dj_link_machine_settings_path(directory.path());
    persist_dj_link_machine_settings_to_path(&path, &stable_settings()).expect("seed settings");
    let persistence = FakeSettingsPersistence::new(path.clone());
    let credentials = FakeCredentialStore::with_primary(7, token(0x11));
    credentials
        .fail_primary_revoke
        .store(true, Ordering::Relaxed);
    let mut authority = seeded_authority(path.clone(), stable_settings());
    authority.wire_token = Some("must-clear-after-revoke-error".to_string());
    assert!(authority
        .disarm_with_dependencies(&persistence, &credentials)
        .is_err());
    let status = authority.status();
    assert!(!status.credential_ready);
    assert_eq!(
        status.block_reason.as_deref(),
        Some("disarm_cleanup_pending")
    );
    assert!(authority.wire_token.is_none());
    assert_eq!(credentials.primary_generation(), Some(7));
    match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => {
            assert!(settings.disarm_cleanup_pending);
            assert!(!settings.auto_start_armed);
        }
        other => panic!("expected durable disarm cleanup intent, got {other:?}"),
    }
}

#[test]
fn disarm_rollback_revoke_failure_is_visible_and_keeps_listener_disarmed() {
    let directory = TempDir::new("disarm-rollback-revoke");
    let path = dj_link_machine_settings_path(directory.path());
    persist_dj_link_machine_settings_to_path(&path, &stable_settings()).expect("seed settings");
    let persistence = FakeSettingsPersistence::new(path.clone());
    let credentials = FakeCredentialStore::with_primary(7, token(0x11));
    credentials
        .save_rollback_token(7, &token(0x11))
        .expect("seed rollback credential");
    credentials
        .fail_rollback_revoke
        .store(true, Ordering::Relaxed);
    let mut authority = seeded_authority(path.clone(), stable_settings());
    authority.wire_token = Some("must-clear-after-rollback-error".to_string());
    assert!(authority
        .disarm_with_dependencies(&persistence, &credentials)
        .is_err());
    let status = authority.status();
    assert!(!status.credential_ready);
    assert!(status.credential_cleanup_pending);
    assert_eq!(
        status.block_reason.as_deref(),
        Some("disarm_cleanup_pending")
    );
    assert!(authority.wire_token.is_none());
    assert_eq!(credentials.primary_generation(), None);
    assert_eq!(credentials.rollback_generation(), Some(7));
    match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => {
            assert!(settings.disarm_cleanup_pending);
            assert!(!settings.auto_start_armed);
        }
        other => panic!("expected durable disarm cleanup intent, got {other:?}"),
    }
}

#[test]
fn restart_retries_primary_disarm_cleanup_before_any_credential_restore() {
    let directory = TempDir::new("restart-primary-disarm-cleanup");
    let path = dj_link_machine_settings_path(directory.path());
    persist_dj_link_machine_settings_to_path(&path, &stable_settings()).expect("seed settings");
    let persistence = FakeSettingsPersistence::new(path.clone());
    let credentials = FakeCredentialStore::with_primary(7, token(0x11));
    credentials
        .fail_primary_revoke
        .store(true, Ordering::Relaxed);
    let mut authority = seeded_authority(path.clone(), stable_settings());
    authority.install_test_wire_token("must-clear-before-restart");
    assert!(authority
        .disarm_with_dependencies(&persistence, &credentials)
        .is_err());
    assert_eq!(credentials.primary_generation(), Some(7));

    let interrupted = match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => settings,
        other => panic!("expected durable cleanup intent, got {other:?}"),
    };
    let mut restarted = seeded_authority(path.clone(), interrupted.clone());
    restarted.initialize_with_dependencies(interrupted, &persistence, &credentials);
    let status = restarted.status();
    assert!(!status.credential_ready);
    assert!(!status.auto_start_armed);
    assert!(status.block_reason.is_none());
    assert!(restarted.wire_token.is_none());
    assert_eq!(credentials.primary_generation(), None);
    assert_eq!(credentials.rollback_generation(), None);
    match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => {
            assert!(!settings.disarm_cleanup_pending);
            assert_eq!(settings.credential_generation, None);
        }
        other => panic!("expected completed disarm, got {other:?}"),
    }
}

#[test]
fn restart_retries_rollback_disarm_cleanup_before_any_credential_restore() {
    let directory = TempDir::new("restart-rollback-disarm-cleanup");
    let path = dj_link_machine_settings_path(directory.path());
    persist_dj_link_machine_settings_to_path(&path, &stable_settings()).expect("seed settings");
    let persistence = FakeSettingsPersistence::new(path.clone());
    let credentials = FakeCredentialStore::with_primary(7, token(0x11));
    credentials
        .save_rollback_token(7, &token(0x11))
        .expect("seed rollback credential");
    credentials
        .fail_rollback_revoke
        .store(true, Ordering::Relaxed);
    let mut authority = seeded_authority(path.clone(), stable_settings());
    authority.install_test_wire_token("must-clear-before-restart");
    assert!(authority
        .disarm_with_dependencies(&persistence, &credentials)
        .is_err());
    assert_eq!(credentials.primary_generation(), None);
    assert_eq!(credentials.rollback_generation(), Some(7));

    let interrupted = match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => settings,
        other => panic!("expected durable cleanup intent, got {other:?}"),
    };
    let mut restarted = seeded_authority(path.clone(), interrupted.clone());
    restarted.initialize_with_dependencies(interrupted, &persistence, &credentials);
    let status = restarted.status();
    assert!(!status.credential_ready);
    assert!(!status.auto_start_armed);
    assert!(status.block_reason.is_none());
    assert!(restarted.wire_token.is_none());
    assert_eq!(credentials.primary_generation(), None);
    assert_eq!(credentials.rollback_generation(), None);
    match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => {
            assert!(!settings.disarm_cleanup_pending);
            assert_eq!(settings.credential_generation, None);
        }
        other => panic!("expected completed disarm, got {other:?}"),
    }
}

#[test]
fn restart_keeps_a_failed_disarm_cleanup_as_the_stable_nonsecret_block_code() {
    let directory = TempDir::new("restart-disarm-cleanup-still-fails");
    let path = dj_link_machine_settings_path(directory.path());
    let mut cleanup_intent = stable_settings();
    cleanup_intent.revision = 8;
    cleanup_intent.auto_start_armed = false;
    cleanup_intent.disarm_cleanup_pending = true;
    persist_dj_link_machine_settings_to_path(&path, &cleanup_intent).expect("seed cleanup intent");
    let persistence = FakeSettingsPersistence::new(path.clone());
    let credentials = FakeCredentialStore::with_primary(7, token(0x11));
    credentials
        .fail_primary_revoke
        .store(true, Ordering::Relaxed);
    let mut restarted = seeded_authority(path, cleanup_intent.clone());
    restarted.initialize_with_dependencies(cleanup_intent, &persistence, &credentials);
    let status = restarted.status();
    assert_eq!(
        status.block_reason.as_deref(),
        Some("disarm_cleanup_pending")
    );
    assert!(status.credential_cleanup_pending);
    assert!(!status.credential_ready);
    assert!(restarted.wire_token.is_none());
    assert_eq!(credentials.primary_generation(), Some(7));
}

#[test]
fn unsupported_credential_platform_keeps_durable_disarm_cleanup_visible() {
    let mut cleanup_intent = stable_settings();
    cleanup_intent.revision = 8;
    cleanup_intent.auto_start_armed = false;
    cleanup_intent.disarm_cleanup_pending = true;
    let mut authority = DjLinkMachineAuthority::default();
    authority.initialize_without_credential_store(cleanup_intent);
    let status = authority.status();
    assert_eq!(
        status.block_reason.as_deref(),
        Some("disarm_cleanup_pending")
    );
    assert!(status.credential_cleanup_pending);
    assert!(!status.credential_ready);
    assert!(!status.auto_start_armed);
    assert!(authority.wire_token.is_none());
}

#[test]
fn after_rename_disarm_marker_error_fails_closed_and_restart_completes_cleanup() {
    let directory = TempDir::new("disarm-marker-after-rename");
    let path = dj_link_machine_settings_path(directory.path());
    persist_dj_link_machine_settings_to_path(&path, &stable_settings()).expect("seed settings");
    let persistence = FakeSettingsPersistence::new(path.clone());
    persistence.fail_on(1, PersistFailure::AfterWrite);
    let credentials = FakeCredentialStore::with_primary(7, token(0x11));
    let mut authority = seeded_authority(path.clone(), stable_settings());
    authority.install_test_wire_token("must-clear-after-marker-write-error");
    assert!(authority
        .disarm_with_dependencies(&persistence, &credentials)
        .is_err());
    let status = authority.status();
    assert_eq!(
        status.block_reason.as_deref(),
        Some("disarm_cleanup_pending")
    );
    assert!(status.credential_cleanup_pending);
    assert!(!status.credential_ready);
    assert!(authority.wire_token.is_none());
    assert!(
        authority.dj_only_config().is_err(),
        "same-process DJ start stays blocked"
    );
    assert_eq!(
        credentials.primary_generation(),
        Some(7),
        "no credential mutation before marker is known"
    );

    let interrupted = match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => settings,
        other => panic!("expected after-rename cleanup marker, got {other:?}"),
    };
    assert!(interrupted.disarm_cleanup_pending);
    let mut restarted = seeded_authority(path.clone(), interrupted.clone());
    restarted.initialize_with_dependencies(interrupted, &persistence, &credentials);
    assert!(restarted.status().block_reason.is_none());
    assert!(!restarted.status().credential_ready);
    assert!(restarted.wire_token.is_none());
    assert_eq!(credentials.primary_generation(), None);
}

#[test]
fn missing_primary_arms_at_next_persisted_generation_without_reusing_g1() {
    let directory = TempDir::new("missing-primary-next-generation");
    let path = dj_link_machine_settings_path(directory.path());
    persist_dj_link_machine_settings_to_path(&path, &stable_settings()).expect("seed settings");
    let persistence = FakeSettingsPersistence::new(path.clone());
    let credentials = FakeCredentialStore::default();
    let mut authority = seeded_authority(path.clone(), stable_settings());
    authority.restored_credential = false;
    authority.block_reason = Some("credential_missing".to_string());
    authority
        .arm_or_rotate_with_dependencies(
            None,
            &persistence,
            &credentials,
            &FixedTokenGenerator::succeeds(0x22),
            &|_, _, _| Ok(()),
        )
        .expect("rearm after missing primary");
    match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => {
            assert_eq!(settings.credential_generation, Some(8));
            assert_eq!(settings.credential_generation_high_water, 8);
            assert!(settings.auto_start_armed);
            assert!(!settings.disarm_cleanup_pending);
        }
        other => panic!("expected rearmed settings, got {other:?}"),
    }
    assert_eq!(credentials.primary_generation(), Some(8));
    assert_eq!(credentials.rollback_generation(), None);
}

#[test]
fn stable_rotate_returns_new_show_once_token_when_rollback_cleanup_fails() {
    let directory = TempDir::new("rotate-rollback-cleanup-show-once");
    let path = dj_link_machine_settings_path(directory.path());
    persist_dj_link_machine_settings_to_path(&path, &stable_settings()).expect("seed settings");
    let persistence = FakeSettingsPersistence::new(path.clone());
    let credentials = FakeCredentialStore::with_primary(7, token(0x11));
    // Rotation first clears an orphan (call 1), then writes the old rollback
    // record (call 2), and only after the durable Idle g8 commit clears that
    // rollback record (call 3). That cleanup may fail without losing the
    // committed authority or the one-time operator return.
    credentials.fail_rollback_revoke_on_call(3);
    let mut authority = seeded_authority(path.clone(), stable_settings());
    let returned = authority
        .arm_or_rotate_with_dependencies(
            None,
            &persistence,
            &credentials,
            &FixedTokenGenerator::succeeds(0x22),
            &|_, _, _| Ok(()),
        )
        .expect("cleanup failure after final commit remains nonfatal");
    assert_eq!(returned, wire_token(&token(0x22)));
    let status = authority.status();
    assert!(status.auto_start_armed);
    assert!(status.credential_ready);
    assert!(status.credential_cleanup_pending);
    assert!(status.block_reason.is_none());
    assert_eq!(credentials.primary_generation(), Some(8));
    assert_eq!(credentials.rollback_generation(), Some(7));

    let stable = match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => settings,
        other => panic!("expected durable final g8 settings, got {other:?}"),
    };
    assert_eq!(
        stable.transaction_state,
        DjLinkMachineTransactionState::Idle
    );
    assert_eq!(stable.credential_generation, Some(8));
    let mut restarted = seeded_authority(path, stable.clone());
    restarted.initialize_with_dependencies(stable, &persistence, &credentials);
    let status = restarted.status();
    assert!(status.auto_start_armed);
    assert!(status.credential_ready);
    assert!(!status.credential_cleanup_pending);
    assert!(status.block_reason.is_none());
    assert_eq!(credentials.primary_generation(), Some(8));
    assert_eq!(credentials.rollback_generation(), None);
}

#[test]
fn missing_primary_prepare_failure_reserves_generation_for_the_retry() {
    let directory = TempDir::new("missing-primary-prepare-failure");
    let path = dj_link_machine_settings_path(directory.path());
    persist_dj_link_machine_settings_to_path(&path, &stable_settings()).expect("seed settings");
    let persistence = FakeSettingsPersistence::new(path.clone());
    persistence.fail_on(1, PersistFailure::BeforeWrite);
    let credentials = FakeCredentialStore::default();
    let mut authority = seeded_authority(path.clone(), stable_settings());
    authority.restored_credential = false;
    authority.block_reason = Some("credential_missing".to_string());
    assert!(authority
        .arm_or_rotate_with_dependencies(
            None,
            &persistence,
            &credentials,
            &FixedTokenGenerator::succeeds(0x22),
            &|_, _, _| Ok(()),
        )
        .is_err());
    match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => {
            assert_eq!(settings.credential_generation, None);
            assert_eq!(settings.credential_generation_high_water, 8);
            assert!(!settings.auto_start_armed);
            assert!(!settings.disarm_cleanup_pending);
        }
        other => panic!("expected restored missing-primary preimage, got {other:?}"),
    }
    assert_eq!(credentials.primary_generation(), None);
    authority
        .arm_or_rotate_with_dependencies(
            None,
            &persistence,
            &credentials,
            &FixedTokenGenerator::succeeds(0x33),
            &|_, _, _| Ok(()),
        )
        .expect("retry after reserved generation");
    match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => {
            assert_eq!(settings.credential_generation, Some(9));
            assert_eq!(settings.credential_generation_high_water, 9);
        }
        other => panic!("expected g9 retry authority, got {other:?}"),
    }
    assert_eq!(credentials.primary_generation(), Some(9));
}

#[test]
fn missing_primary_committing_failure_reserves_generation_across_restart_retry() {
    let directory = TempDir::new("missing-primary-committing-failure");
    let path = dj_link_machine_settings_path(directory.path());
    persist_dj_link_machine_settings_to_path(&path, &stable_settings()).expect("seed settings");
    let persistence = FakeSettingsPersistence::new(path.clone());
    // The prepared g8 journal is durable; publishing Committing then fails.
    persistence.fail_on(2, PersistFailure::BeforeWrite);
    let credentials = FakeCredentialStore::default();
    let mut authority = seeded_authority(path.clone(), stable_settings());
    authority.restored_credential = false;
    authority.block_reason = Some("credential_missing".to_string());
    assert!(authority
        .arm_or_rotate_with_dependencies(
            None,
            &persistence,
            &credentials,
            &FixedTokenGenerator::succeeds(0x22),
            &|_, _, _| Ok(()),
        )
        .is_err());

    let restored = match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => settings,
        other => panic!("expected recovered missing-primary preimage, got {other:?}"),
    };
    assert_eq!(restored.credential_generation, None);
    assert_eq!(restored.credential_generation_high_water, 8);
    let mut restarted = seeded_authority(path.clone(), restored.clone());
    restarted.initialize_with_dependencies(restored, &persistence, &credentials);
    assert!(!restarted.status().credential_ready);
    restarted
        .arm_or_rotate_with_dependencies(
            None,
            &persistence,
            &credentials,
            &FixedTokenGenerator::succeeds(0x33),
            &|_, _, _| Ok(()),
        )
        .expect("retry after a durably reserved g8");
    match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => {
            assert_eq!(settings.credential_generation, Some(9));
            assert_eq!(settings.credential_generation_high_water, 9);
        }
        other => panic!("expected g9 retry authority, got {other:?}"),
    }
}

#[test]
fn missing_primary_final_after_rename_failure_reserves_generation_across_restart_retry() {
    let directory = TempDir::new("missing-primary-final-after-rename");
    let path = dj_link_machine_settings_path(directory.path());
    persist_dj_link_machine_settings_to_path(&path, &stable_settings()).expect("seed settings");
    let persistence = FakeSettingsPersistence::new(path.clone());
    // g8 reaches the final Idle write, but the writer reports after rename.
    // Compensation must retain g8 as reserved while restoring the missing
    // primary preimage, so restart cannot reuse g1 or g8.
    persistence.fail_on(3, PersistFailure::AfterWrite);
    let credentials = FakeCredentialStore::default();
    let mut authority = seeded_authority(path.clone(), stable_settings());
    authority.restored_credential = false;
    authority.block_reason = Some("credential_missing".to_string());
    assert!(authority
        .arm_or_rotate_with_dependencies(
            None,
            &persistence,
            &credentials,
            &FixedTokenGenerator::succeeds(0x22),
            &|_, _, _| Ok(()),
        )
        .is_err());

    let restored = match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => settings,
        other => panic!("expected recovered missing-primary preimage, got {other:?}"),
    };
    assert_eq!(restored.credential_generation, None);
    assert_eq!(restored.credential_generation_high_water, 8);
    assert_eq!(credentials.primary_generation(), None);
    let mut restarted = seeded_authority(path.clone(), restored.clone());
    restarted.initialize_with_dependencies(restored, &persistence, &credentials);
    restarted
        .arm_or_rotate_with_dependencies(
            None,
            &persistence,
            &credentials,
            &FixedTokenGenerator::succeeds(0x33),
            &|_, _, _| Ok(()),
        )
        .expect("retry after an ambiguous final g8 write");
    match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => {
            assert_eq!(settings.credential_generation, Some(9));
            assert_eq!(settings.credential_generation_high_water, 9);
        }
        other => panic!("expected g9 retry authority, got {other:?}"),
    }
}

#[test]
fn restart_discards_interrupted_missing_primary_rotation_without_promoting_the_new_token() {
    let directory = TempDir::new("missing-primary-restart");
    let path = dj_link_machine_settings_path(directory.path());
    let old = stable_settings();
    let mut rollback_settings = old.clone();
    rollback_settings.credential_generation = None;
    rollback_settings.auto_start_armed = false;
    let mut desired = old.clone();
    desired.credential_generation = None;
    desired.disarm_cleanup_pending = false;
    let prepared = desired
        .begin_prepare_commit_with_rollback(
            old.revision,
            8,
            DjLinkMachineRollback::from_stable(&rollback_settings)
                .expect("missing-primary preimage"),
        )
        .expect("prepared journal");
    persist_dj_link_machine_settings_to_path(&path, &prepared).expect("seed interrupted journal");
    let credentials = FakeCredentialStore::with_primary(8, token(0x22));
    let persistence = FakeSettingsPersistence::new(path.clone());
    let mut restarted = seeded_authority(path.clone(), prepared.clone());
    restarted.initialize_with_dependencies(prepared, &persistence, &credentials);
    assert!(restarted.status().block_reason.is_none());
    assert!(!restarted.status().credential_ready);
    assert!(restarted.wire_token.is_none());
    assert_eq!(credentials.primary_generation(), None);
    match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => {
            assert_eq!(settings.credential_generation, None);
            assert_eq!(settings.credential_generation_high_water, 8);
            assert!(!settings.auto_start_armed);
        }
        other => panic!("expected old missing-primary preimage, got {other:?}"),
    }
    restarted
        .arm_or_rotate_with_dependencies(
            None,
            &persistence,
            &credentials,
            &FixedTokenGenerator::succeeds(0x33),
            &|_, _, _| Ok(()),
        )
        .expect("restart retry after reserved g8");
    match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => {
            assert_eq!(settings.credential_generation, Some(9));
            assert_eq!(settings.credential_generation_high_water, 9);
        }
        other => panic!("expected g9 restart retry authority, got {other:?}"),
    }
}

#[test]
fn arm_pre_token_failure_preserves_accepted_authority() {
    let directory = TempDir::new("pre-token");
    let path = dj_link_machine_settings_path(directory.path());
    persist_dj_link_machine_settings_to_path(&path, &stable_settings()).expect("seed settings");
    let persistence = FakeSettingsPersistence::new(path.clone());
    let credentials = FakeCredentialStore::with_primary(7, token(0x11));
    let mut authority = seeded_authority(path.clone(), stable_settings());
    let accepted_wire = wire_token(&token(0x11));
    authority.install_test_wire_token(&accepted_wire);
    assert!(authority
        .arm_or_rotate_with_dependencies(
            None,
            &persistence,
            &credentials,
            &FixedTokenGenerator::fails(),
            &|_, _, _| Ok(()),
        )
        .is_err());
    assert_old_authority_restored(&path, &credentials);
    assert!(authority.status().credential_ready);
    assert_eq!(
        authority.wire_token.as_deref(),
        Some(accepted_wire.as_str())
    );
    assert_eq!(persistence.calls.load(Ordering::Relaxed), 0);
}

#[test]
fn arm_after_credential_write_failure_compensates_to_old_authority() {
    let directory = TempDir::new("credential-write");
    let path = dj_link_machine_settings_path(directory.path());
    persist_dj_link_machine_settings_to_path(&path, &stable_settings()).expect("seed settings");
    let persistence = FakeSettingsPersistence::new(path.clone());
    let credentials = FakeCredentialStore::with_primary(7, token(0x11));
    credentials
        .fail_primary_write_after_store
        .store(true, Ordering::Relaxed);
    let mut authority = seeded_authority(path.clone(), stable_settings());
    assert!(authority
        .arm_or_rotate_with_dependencies(
            None,
            &persistence,
            &credentials,
            &FixedTokenGenerator::succeeds(0x22),
            &|_, _, _| Ok(()),
        )
        .is_err());
    assert_old_authority_restored(&path, &credentials);
    let expected_wire = wire_token(&token(0x11));
    assert!(authority.status().credential_ready);
    assert_eq!(
        authority.wire_token.as_deref(),
        Some(expected_wire.as_str())
    );
}

#[test]
fn arm_committing_persist_failure_compensates_to_old_authority() {
    let directory = TempDir::new("committing-persist");
    let path = dj_link_machine_settings_path(directory.path());
    persist_dj_link_machine_settings_to_path(&path, &stable_settings()).expect("seed settings");
    let persistence = FakeSettingsPersistence::new(path.clone());
    persistence.fail_on(2, PersistFailure::BeforeWrite);
    let credentials = FakeCredentialStore::with_primary(7, token(0x11));
    let mut authority = seeded_authority(path.clone(), stable_settings());
    assert!(authority
        .arm_or_rotate_with_dependencies(
            None,
            &persistence,
            &credentials,
            &FixedTokenGenerator::succeeds(0x22),
            &|_, _, _| Ok(()),
        )
        .is_err());
    assert_old_authority_restored(&path, &credentials);
    let expected_wire = wire_token(&token(0x11));
    assert!(authority.status().credential_ready);
    assert_eq!(
        authority.wire_token.as_deref(),
        Some(expected_wire.as_str())
    );
}

#[test]
fn arm_final_persist_failure_compensates_even_if_writer_reports_after_rename() {
    let directory = TempDir::new("final-persist");
    let path = dj_link_machine_settings_path(directory.path());
    persist_dj_link_machine_settings_to_path(&path, &stable_settings()).expect("seed settings");
    let persistence = FakeSettingsPersistence::new(path.clone());
    persistence.fail_on(3, PersistFailure::AfterWrite);
    let credentials = FakeCredentialStore::with_primary(7, token(0x11));
    let mut authority = seeded_authority(path.clone(), stable_settings());
    assert!(authority
        .arm_or_rotate_with_dependencies(
            None,
            &persistence,
            &credentials,
            &FixedTokenGenerator::succeeds(0x22),
            &|_, _, _| Ok(()),
        )
        .is_err());
    assert_old_authority_restored(&path, &credentials);
    let expected_wire = wire_token(&token(0x11));
    assert!(authority.status().credential_ready);
    assert_eq!(
        authority.wire_token.as_deref(),
        Some(expected_wire.as_str())
    );
}

#[test]
fn restart_rolls_back_nonidle_new_primary_to_verified_old_authority() {
    let directory = TempDir::new("restart");
    let path = dj_link_machine_settings_path(directory.path());
    let stable = stable_settings();
    let mut desired = stable.clone();
    desired.credential_generation = Some(8);
    let prepared = desired
        .begin_prepare_commit_with_rollback(
            stable.revision,
            8,
            DjLinkMachineRollback::from_stable(&stable).expect("preimage"),
        )
        .expect("prepared");
    let committing = prepared
        .stage_committing(prepared.revision)
        .expect("committing");
    persist_dj_link_machine_settings_to_path(&path, &committing).expect("seed interrupted journal");
    let credentials = FakeCredentialStore::with_primary(8, token(0x22));
    credentials
        .save_rollback_token(7, &token(0x11))
        .expect("seed rollback token");
    let persistence = FakeSettingsPersistence::new(path.clone());
    let mut restarted = seeded_authority(path.clone(), committing);
    restarted.initialize_with_dependencies(
        match load_dj_link_machine_settings_from_path(&path) {
            DjLinkMachineSettingsLoadOutcome::Loaded(settings) => settings,
            other => panic!("expected interrupted journal, got {other:?}"),
        },
        &persistence,
        &credentials,
    );
    assert_old_authority_restored(&path, &credentials);
    assert!(restarted.block_reason.is_none());
    assert!(restarted.restored_credential);
}
