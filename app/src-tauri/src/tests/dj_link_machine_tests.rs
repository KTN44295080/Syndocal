use super::*;
use std::{
    path::PathBuf,
    process,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering as AtomicOrdering},
        Mutex,
    },
};

static TEMP_DIR_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = TEMP_DIR_SEQUENCE.fetch_add(1, AtomicOrdering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "syndocal-djlm-{}-{}-{label}-{unique}",
            process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        Self(dir)
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

fn sample_token(byte: u8) -> DjLinkToken {
    DjLinkToken::from_bytes(&[byte; DJ_LINK_TOKEN_LEN]).expect("non-zero token")
}

fn fully_populated_settings() -> DjLinkMachineSettingsV2 {
    DjLinkMachineSettingsV2 {
        version: DJ_LINK_MACHINE_SETTINGS_VERSION,
        revision: 7,
        transaction_state: DjLinkMachineTransactionState::Idle,
        credential_generation: Some(3),
        credential_generation_high_water: 3,
        adapter_guid: Some("0123abcd-5678-90ef-abcd-ef0123456789".to_string()),
        network_guid: Some("ffffffff-0000-aaaa-bbbb-ccccddddeeee".to_string()),
        bind_ip: Some("192.168.7.9".to_string()),
        bind_port: Some(49152),
        auto_start_armed: true,
        disarm_cleanup_pending: false,
        rollback: None,
    }
    .validated()
    .expect("populated settings validate")
}

// -- token type ----------------------------------------------------------

#[test]
fn token_rejects_wrong_length_and_zero_token() {
    assert!(DjLinkToken::from_bytes(&[1; 31]).is_none());
    assert!(DjLinkToken::from_bytes(&[1; 33]).is_none());
    assert!(DjLinkToken::from_bytes(&[0; DJ_LINK_TOKEN_LEN]).is_none());
    assert!(DjLinkToken::from_bytes(&[1; DJ_LINK_TOKEN_LEN]).is_some());
}

#[test]
fn token_debug_is_redacted_everywhere() {
    let record = DjLinkTokenRecord {
        generation: 9,
        token: sample_token(0xA5),
    };
    let token_debug = format!("{:?}", token_debug_probe(&record));
    let record_debug = format!("{record:?}");
    assert!(!token_debug.contains("165") && !token_debug.contains("a5"));
    assert!(!record_debug.contains("A5") && !record_debug.contains("165"));
    assert!(token_debug.contains("REDACTED"));
    assert!(record_debug.contains("REDACTED"));
}

fn token_debug_probe(record: &DjLinkTokenRecord) -> &DjLinkToken {
    &record.token
}

#[test]
fn credential_readback_debug_never_renders_secret_bytes() {
    let readback = CredentialReadback {
        blob: Zeroizing::new(vec![0x41_u8; DJ_LINK_TOKEN_BLOB_LEN]),
        credential_type: ReadbackType::Generic,
        machine_persisted: true,
        target_matches: true,
    };
    for rendered in [format!("{readback:?}"), format!("{readback:#?}")] {
        // Raw byte renderings ('A' filler), hex ("414141..."), and base64
        // ("QUFB..." for "AAA") of the secret blob must all be absent.
        assert!(!rendered.contains("AAAA"), "raw bytes leaked: {rendered}");
        assert!(
            !rendered.to_ascii_lowercase().contains("414141"),
            "hex leaked: {rendered}"
        );
        assert!(!rendered.contains("QUFB"), "base64 leaked: {rendered}");
        assert!(rendered.contains("blob_len"), "{rendered}");
        assert!(
            rendered.contains("REDACTED") || rendered.contains(".."),
            "{rendered}"
        );
    }
}

#[test]
fn platform_store_is_send_sync_for_tauri_state() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PlatformDjLinkCredentialStore>();
}

#[test]
fn token_equality_is_value_based() {
    assert_eq!(sample_token(1), sample_token(1));
    assert_ne!(sample_token(1), sample_token(2));
}

#[test]
fn token_drop_wipes_backing_bytes() {
    let mut token = std::mem::ManuallyDrop::new(sample_token(0x5A));
    let pointer = token.expose().as_ptr();
    // SAFETY: ManuallyDrop guarantees this destructor runs exactly once,
    // in place, so `pointer` stays valid for the wipe assertion below.
    let inner: &mut DjLinkToken = &mut token;
    unsafe { std::ptr::drop_in_place(inner) };
    let remaining = unsafe { std::slice::from_raw_parts(pointer, DJ_LINK_TOKEN_LEN) };
    assert!(
        remaining.iter().all(|byte| *byte == 0),
        "token bytes must be wiped on drop, found {:?}",
        remaining
    );
    std::mem::forget(token);
}

// -- strict blob codec ---------------------------------------------------

#[test]
fn blob_roundtrip_has_exact_layout() {
    let token = sample_token(0xC3);
    let blob = encode_dj_link_token_blob(&token, 0x0102_0304_0506_0708).expect("encode");
    assert_eq!(blob.len(), DJ_LINK_TOKEN_BLOB_LEN);
    assert_eq!(blob.len(), 50);
    assert_eq!(&blob[..8], &DJ_LINK_TOKEN_BLOB_MAGIC);
    assert_eq!(&blob[8..10], &DJ_LINK_TOKEN_BLOB_VERSION.to_le_bytes());
    assert_eq!(&blob[10..18], &0x0102_0304_0506_0708_u64.to_le_bytes());
    assert_eq!(&blob[18..], token.expose());
    let decoded = decode_dj_link_token_blob(&blob).expect("decode");
    assert_eq!(decoded.generation, 0x0102_0304_0506_0708);
    assert_eq!(decoded.token, token);
}

#[test]
fn blob_decode_rejects_bad_magic_version_lengths_and_values() {
    let token = sample_token(0x77);
    let blob = encode_dj_link_token_blob(&token, 42).expect("encode");

    let mut bad_magic = blob.to_vec();
    bad_magic[3] ^= 0xFF;
    assert_eq!(
        decode_dj_link_token_blob(&bad_magic),
        Err(DjLinkCredentialError::BlobEncode(
            DjLinkTokenBlobError::BadMagic
        ))
    );

    for version in [0_u16, 2, u16::MAX] {
        let mut bad_version = blob.to_vec();
        bad_version[8..10].copy_from_slice(&version.to_le_bytes());
        assert_eq!(
            decode_dj_link_token_blob(&bad_version),
            Err(DjLinkCredentialError::BlobEncode(
                DjLinkTokenBlobError::UnsupportedVersion(version)
            ))
        );
    }

    for length in 0..DJ_LINK_TOKEN_BLOB_LEN {
        let truncated = &blob[..length];
        let expected = if truncated.is_empty() {
            DjLinkTokenBlobError::Empty
        } else {
            DjLinkTokenBlobError::BadLength {
                expected: DJ_LINK_TOKEN_BLOB_LEN,
                found: length,
            }
        };
        assert_eq!(
            decode_dj_link_token_blob(truncated),
            Err(DjLinkCredentialError::BlobEncode(expected)),
            "length {length}"
        );
    }

    let mut trailing = blob.to_vec();
    trailing.push(0);
    assert_eq!(
        decode_dj_link_token_blob(&trailing),
        Err(DjLinkCredentialError::BlobEncode(
            DjLinkTokenBlobError::TooLong
        ))
    );

    let mut zero_generation = blob.to_vec();
    zero_generation[10..18].fill(0);
    assert_eq!(
        decode_dj_link_token_blob(&zero_generation),
        Err(DjLinkCredentialError::BlobEncode(
            DjLinkTokenBlobError::InvalidGeneration
        ))
    );

    let mut zero_token = blob.to_vec();
    zero_token[18..].fill(0);
    assert_eq!(
        decode_dj_link_token_blob(&zero_token),
        Err(DjLinkCredentialError::BlobEncode(
            DjLinkTokenBlobError::InvalidToken
        ))
    );

    assert_eq!(
        decode_dj_link_token_blob(&[]),
        Err(DjLinkCredentialError::BlobEncode(
            DjLinkTokenBlobError::Empty
        ))
    );
    assert!(decode_dj_link_token_blob(&vec![0_u8; 6000]).is_err());
}

#[test]
fn blob_encode_rejects_zero_generation_and_zero_token() {
    let token = sample_token(1);
    assert_eq!(
        encode_dj_link_token_blob(&token, 0),
        Err(DjLinkCredentialError::BlobEncode(
            DjLinkTokenBlobError::InvalidGeneration
        ))
    );
    let zero = DjLinkToken([0; DJ_LINK_TOKEN_LEN]);
    assert_eq!(
        encode_dj_link_token_blob(&zero, 1),
        Err(DjLinkCredentialError::BlobEncode(
            DjLinkTokenBlobError::InvalidToken
        ))
    );
}

// -- secret-safe errors --------------------------------------------------

#[test]
fn credential_errors_never_expose_the_secret() {
    let sentinel = sample_token(0xEE);
    let hex_sentinel = "eeeeee";
    let messages = [
        format!("{}", DjLinkCredentialError::PlatformUnsupported),
        format!("{}", DjLinkCredentialError::TargetNameInvalid),
        format!("{}", DjLinkCredentialError::BlobTooLarge),
        format!("{}", DjLinkCredentialError::NotConfigured),
        format!("{}", DjLinkCredentialError::VerifyMismatch),
        format!("{}", DjLinkCredentialError::PersistenceDowngraded),
        format!("{:?}", DjLinkCredentialError::WriteFailed { code: -1 }),
        format!("{:?}", DjLinkCredentialError::ReadFailed { code: -1 }),
        format!("{:?}", DjLinkCredentialError::RevokeFailed { code: -1 }),
        format!(
            "{:?}",
            DjLinkCredentialError::BlobEncode(DjLinkTokenBlobError::BadMagic)
        ),
    ];
    for message in messages {
        assert!(!message.to_ascii_lowercase().contains(hex_sentinel));
    }
    // A real failing flow must not leak the secret through its error path.
    let tampering = MemoryCredentialBackend::default();
    tampering.tamper_readback.store(true, Ordering::Relaxed);
    let store = store_over(tampering);
    let failure = format!("{:?}", store.save_token(4, &sentinel));
    assert!(!failure.to_ascii_lowercase().contains(hex_sentinel));
}

// -- credential orchestration over the memory backend --------------------

#[derive(Default)]
struct MemoryCredentialBackend {
    // Mutex/atomics instead of RefCell/Cell so the backend satisfies the
    // Send + Sync supertraits of DjLinkCredentialBackend.
    stored: Mutex<Option<Vec<u8>>>,
    fail_write: AtomicBool,
    fail_delete: AtomicBool,
    vanish_after_write: AtomicBool,
    tamper_readback: AtomicBool,
    downgrade_persistence: AtomicBool,
    foreign_type: AtomicBool,
    foreign_target: AtomicBool,
    /// Number of upcoming reads that report `ReadFailed` before succeeding
    /// again; lets tests observe post-save revocation through the store.
    reads_to_fail: AtomicUsize,
}

impl Clone for MemoryCredentialBackend {
    fn clone(&self) -> Self {
        let flag = |value: &AtomicBool| value.load(Ordering::Relaxed);
        Self {
            stored: Mutex::new(self.stored.lock().expect("stored mutex").clone()),
            fail_write: AtomicBool::new(flag(&self.fail_write)),
            fail_delete: AtomicBool::new(flag(&self.fail_delete)),
            vanish_after_write: AtomicBool::new(flag(&self.vanish_after_write)),
            tamper_readback: AtomicBool::new(flag(&self.tamper_readback)),
            downgrade_persistence: AtomicBool::new(flag(&self.downgrade_persistence)),
            foreign_type: AtomicBool::new(flag(&self.foreign_type)),
            foreign_target: AtomicBool::new(flag(&self.foreign_target)),
            reads_to_fail: AtomicUsize::new(self.reads_to_fail.load(Ordering::Relaxed)),
        }
    }
}

impl fmt::Debug for MemoryCredentialBackend {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("MemoryCredentialBackend").finish()
    }
}

impl MemoryCredentialBackend {
    fn readback_for(&self, blob: &[u8]) -> CredentialReadback {
        let mut copy = blob.to_vec();
        if self.tamper_readback.load(Ordering::Relaxed) {
            let last = copy.len() - 1;
            copy[last] ^= 0xFF;
        }
        CredentialReadback {
            blob: Zeroizing::new(copy),
            credential_type: if self.foreign_type.load(Ordering::Relaxed) {
                ReadbackType::Foreign
            } else {
                ReadbackType::Generic
            },
            machine_persisted: !self.downgrade_persistence.load(Ordering::Relaxed),
            target_matches: !self.foreign_target.load(Ordering::Relaxed),
        }
    }
}

impl DjLinkCredentialBackend for MemoryCredentialBackend {
    fn write_credential(&self, blob: &[u8]) -> Result<(), DjLinkCredentialError> {
        if self.fail_write.load(Ordering::Relaxed) {
            return Err(DjLinkCredentialError::WriteFailed { code: -1 });
        }
        *self.stored.lock().expect("stored mutex") = Some(blob.to_vec());
        Ok(())
    }

    fn read_credential(&self) -> Result<Option<CredentialReadback>, DjLinkCredentialError> {
        if self.vanish_after_write.load(Ordering::Relaxed) {
            return Ok(None);
        }
        if self.reads_to_fail.load(Ordering::Relaxed) > 0 {
            self.reads_to_fail.fetch_sub(1, Ordering::Relaxed);
            return Err(DjLinkCredentialError::ReadFailed { code: -1 });
        }
        let stored = self.stored.lock().expect("stored mutex");
        match stored.as_deref() {
            None => Ok(None),
            Some(blob) => Ok(Some(self.readback_for(blob))),
        }
    }

    fn delete_credential(&self) -> Result<(), DjLinkCredentialError> {
        if self.fail_delete.load(Ordering::Relaxed) {
            return Err(DjLinkCredentialError::RevokeFailed { code: -1 });
        }
        *self.stored.lock().expect("stored mutex") = None;
        Ok(())
    }
}

fn store_over(backend: MemoryCredentialBackend) -> PlatformDjLinkCredentialStore {
    PlatformDjLinkCredentialStore {
        backend: Some(Box::new(backend)),
        rollback_backend: Some(Box::new(MemoryCredentialBackend::default())),
    }
}

#[test]
fn save_then_load_roundtrips_through_trait() {
    let store = store_over(MemoryCredentialBackend::default());
    assert_eq!(
        store.persistence_scope(),
        DjLinkCredentialPersistenceScope::MachineLocal
    );
    assert_eq!(store.load_token(), Ok(None));
    store.save_token(11, &sample_token(0x11)).expect("save");
    let loaded = store.load_token().expect("load").expect("present");
    assert_eq!(loaded.generation, 11);
    assert_eq!(loaded.token, sample_token(0x11));
    store.revoke_token().expect("revoke");
    assert_eq!(store.load_token(), Ok(None));
    store.revoke_token().expect("revoke is idempotent");
}

#[test]
fn save_revokes_on_tampered_readback() {
    let backend = MemoryCredentialBackend::default();
    backend.tamper_readback.store(true, Ordering::Relaxed);
    let store = store_over(backend);
    assert_eq!(
        store.save_token(5, &sample_token(0x55)),
        Err(DjLinkCredentialError::VerifyMismatch)
    );
    assert_eq!(store.load_token(), Ok(None), "failed save must not linger");
}

#[test]
fn save_fails_closed_when_persistence_was_downgraded() {
    let backend = MemoryCredentialBackend::default();
    backend.downgrade_persistence.store(true, Ordering::Relaxed);
    let store = store_over(backend);
    assert_eq!(
        store.save_token(6, &sample_token(0x66)),
        Err(DjLinkCredentialError::PersistenceDowngraded)
    );
    assert_eq!(store.load_token(), Ok(None), "downgraded write was revoked");
}

#[test]
fn save_fails_when_written_credential_vanishes() {
    let backend = MemoryCredentialBackend::default();
    backend.vanish_after_write.store(true, Ordering::Relaxed);
    let store = store_over(backend);
    assert_eq!(
        store.save_token(7, &sample_token(0x77)),
        Err(DjLinkCredentialError::VerifyMismatch)
    );
}

#[test]
fn save_readback_failure_revokes_fresh_write_and_preserves_original_error() {
    // Exactly one failing read: the save's verification read. The
    // subsequent load therefore succeeds and proves the revocation.
    let backend = MemoryCredentialBackend::default();
    backend.reads_to_fail.store(1, Ordering::Relaxed);
    let store = store_over(backend);
    assert_eq!(
        store.save_token(9, &sample_token(0x99)),
        Err(DjLinkCredentialError::ReadFailed { code: -1 }),
        "the original read error must survive the best-effort revoke verbatim"
    );
    assert_eq!(
        store.load_token(),
        Ok(None),
        "a failed save must not leave the freshly written credential behind"
    );
}

#[test]
fn save_propagates_write_failure_without_side_effects() {
    let backend = MemoryCredentialBackend::default();
    backend.fail_write.store(true, Ordering::Relaxed);
    let store = store_over(backend);
    assert_eq!(
        store.save_token(8, &sample_token(0x88)),
        Err(DjLinkCredentialError::WriteFailed { code: -1 })
    );
}

#[test]
fn load_blocks_foreign_type_target_and_downgrade() {
    let foreign_type = MemoryCredentialBackend::default();
    foreign_type.foreign_type.store(true, Ordering::Relaxed);
    *foreign_type.stored.lock().expect("stored mutex") =
        Some(vec![0x41_u8; DJ_LINK_TOKEN_BLOB_LEN]);
    assert_eq!(
        store_over(foreign_type).load_token(),
        Err(DjLinkCredentialError::VerifyMismatch)
    );

    let foreign_target = MemoryCredentialBackend::default();
    foreign_target.foreign_target.store(true, Ordering::Relaxed);
    *foreign_target.stored.lock().expect("stored mutex") =
        Some(vec![0x41_u8; DJ_LINK_TOKEN_BLOB_LEN]);
    assert_eq!(
        store_over(foreign_target).load_token(),
        Err(DjLinkCredentialError::VerifyMismatch)
    );

    let downgraded = MemoryCredentialBackend::default();
    downgraded
        .downgrade_persistence
        .store(true, Ordering::Relaxed);
    *downgraded.stored.lock().expect("stored mutex") = Some(vec![0_u8; DJ_LINK_TOKEN_BLOB_LEN]);
    assert_eq!(
        store_over(downgraded.clone()).load_token(),
        Err(DjLinkCredentialError::PersistenceDowngraded)
    );
    assert!(
        downgraded.stored.lock().expect("stored mutex").is_some(),
        "load must not delete a downgraded credential"
    );
}

#[test]
fn revoke_propagates_delete_failure() {
    let backend = MemoryCredentialBackend::default();
    backend.fail_delete.store(true, Ordering::Relaxed);
    let store = store_over(backend);
    assert_eq!(
        store.revoke_token(),
        Err(DjLinkCredentialError::RevokeFailed { code: -1 })
    );
}

#[test]
fn unsupported_platform_fails_closed() {
    let store = PlatformDjLinkCredentialStore {
        backend: None,
        rollback_backend: None,
    };
    assert_eq!(
        store.persistence_scope(),
        DjLinkCredentialPersistenceScope::Unsupported
    );
    assert_eq!(
        store.save_token(1, &sample_token(1)),
        Err(DjLinkCredentialError::PlatformUnsupported)
    );
    assert_eq!(
        store.load_token(),
        Err(DjLinkCredentialError::PlatformUnsupported)
    );
    assert_eq!(
        store.revoke_token(),
        Err(DjLinkCredentialError::PlatformUnsupported)
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn native_platform_store_is_unsupported_off_windows() {
    assert!(!dj_link_machine_credential_persistence_supported());
    assert!(matches!(
        PlatformDjLinkCredentialStore::new(),
        Err(DjLinkCredentialError::PlatformUnsupported)
    ));
}

#[cfg(target_os = "windows")]
#[test]
fn windows_reports_machine_local_capability() {
    assert!(dj_link_machine_credential_persistence_supported());
    assert_eq!(
        PlatformDjLinkCredentialStore::new()
            .expect("windows supports credential manager")
            .persistence_scope(),
        DjLinkCredentialPersistenceScope::MachineLocal
    );
    assert_eq!(
        DJ_LINK_CREDENTIAL_TARGET,
        "jp.seraf.ktn.syndocal/dj-link/v1"
    );
}

// -- settings validation -------------------------------------------------

#[test]
fn settings_default_validates_and_serializes_exact_key_set() {
    let settings = DjLinkMachineSettingsV2::default()
        .validated()
        .expect("valid");
    let object = serde_json::to_value(&settings)
        .expect("serialize")
        .as_object()
        .expect("object")
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        object,
        vec![
            "adapter_guid",
            "auto_start_armed",
            "bind_ip",
            "bind_port",
            "credential_generation",
            "credential_generation_high_water",
            "disarm_cleanup_pending",
            "network_guid",
            "revision",
            "rollback",
            "transaction_state",
            "version"
        ]
    );
}

#[test]
fn v2_without_disarm_cleanup_marker_is_accepted_as_false() {
    // This V2 authority was introduced before cleanup intent existed.
    // The absent non-secret marker exactly means no interrupted disarm;
    // retain those local V2 files instead of silently classifying them as
    // corrupt or widening the retired-version boundary.
    let dir = TempDir::new("v2-no-cleanup-marker");
    let path = dir.path().join("machine-settings.json");
    let mut value =
        serde_json::to_value(DjLinkMachineSettingsV2::default()).expect("serialize V2 settings");
    value
        .as_object_mut()
        .expect("settings object")
        .remove("disarm_cleanup_pending");
    fs::write(
        &path,
        serde_json::to_vec(&value).expect("encode V2 settings"),
    )
    .expect("write V2 settings");
    match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => {
            assert!(!settings.disarm_cleanup_pending);
        }
        other => panic!("expected compatible V2 settings, got {other:?}"),
    }
}

#[test]
fn v2_without_generation_watermark_derives_the_present_authority_bound() {
    // The marker is a V2 additive clean extension. A previously written
    // V2 authority still contains an exact non-secret lower bound: its
    // accepted generation. Materialize it rather than reopening g1.
    let dir = TempDir::new("v2-no-generation-watermark");
    let path = dir.path().join("machine-settings.json");
    let mut value =
        serde_json::to_value(fully_populated_settings()).expect("serialize V2 settings");
    value
        .as_object_mut()
        .expect("settings object")
        .remove("credential_generation_high_water");
    fs::write(
        &path,
        serde_json::to_vec(&value).expect("encode V2 settings"),
    )
    .expect("write V2 settings");
    match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::Loaded(settings) => {
            assert_eq!(settings.credential_generation, Some(3));
            assert_eq!(settings.credential_generation_high_water, 3);
        }
        other => panic!("expected compatible V2 settings, got {other:?}"),
    }
}

#[test]
fn disarm_cleanup_marker_rejects_mixed_transaction_journals() {
    let stable = fully_populated_settings();
    let mut hostile = stable
        .begin_prepare_commit_with_rollback(
            stable.revision,
            4,
            DjLinkMachineRollback::from_stable(&stable).expect("stable preimage"),
        )
        .expect("prepared journal");
    hostile.disarm_cleanup_pending = true;
    assert_eq!(
        hostile.validated(),
        Err(DjLinkMachineSettingsError::DisarmCleanupInconsistent)
    );
}

#[test]
fn settings_metadata_never_contains_secret_material() {
    let settings = fully_populated_settings();
    let serialized = serde_json::to_string(&settings).expect("serialize");
    for banned in ["token", "pin", "blob", "secret", "hash"] {
        assert!(
            !serialized.to_ascii_lowercase().contains(banned),
            "metadata leaked banned key fragment {banned}: {serialized}"
        );
    }
}

#[test]
fn settings_validation_matrix() {
    let base = fully_populated_settings();

    let mut future = base.clone();
    future.version = DJ_LINK_MACHINE_SETTINGS_VERSION + 1;
    assert_eq!(
        future.clone().validated(),
        Err(DjLinkMachineSettingsError::UnsupportedVersion {
            found: DJ_LINK_MACHINE_SETTINGS_VERSION + 1
        })
    );

    for revision in [0_u64, u64::MAX] {
        let mut bad = base.clone();
        bad.revision = revision;
        assert_eq!(
            bad.clone().validated(),
            Err(DjLinkMachineSettingsError::InvalidRevision)
        );
    }

    let mut zero_gen = base.clone();
    zero_gen.credential_generation = Some(0);
    assert_eq!(
        zero_gen.clone().validated(),
        Err(DjLinkMachineSettingsError::InvalidGeneration)
    );

    let mut stale_high_water = base.clone();
    stale_high_water.credential_generation_high_water = 2;
    assert_eq!(
        stale_high_water.validated(),
        Err(DjLinkMachineSettingsError::GenerationHighWaterInconsistent)
    );

    for guid in [
        "",
        "0123ABCD-5678-90EF-ABCD-EF0123456789",
        "0123abcd567890efabcdef0123456789",
        "0123abcd-5678-90ef-abcd-ef012345678",
        "g123abcd-5678-90ef-abcd-ef0123456789",
    ] {
        let mut bad_guid = base.clone();
        bad_guid.adapter_guid = Some(guid.to_string());
        assert_eq!(
            bad_guid.clone().validated(),
            Err(DjLinkMachineSettingsError::InvalidAdapterGuid),
            "guid {guid:?}"
        );
    }

    for ip in [
        "",
        " 192.168.7.9",
        "192.168.7.9 ",
        "192.168.007.9",
        "not-an-ip",
        "2001:db8::1:0:0:0:1",
    ] {
        let mut bad_ip = base.clone();
        bad_ip.bind_ip = Some(ip.to_string());
        assert_eq!(
            bad_ip.clone().validated(),
            Err(DjLinkMachineSettingsError::InvalidBindIp),
            "ip {ip:?}"
        );
    }

    let mut port_zero = base.clone();
    port_zero.bind_port = Some(0);
    assert_eq!(
        port_zero.clone().validated(),
        Err(DjLinkMachineSettingsError::InvalidBindPort)
    );

    let mut inconsistent = DjLinkMachineSettingsV2 {
        transaction_state: DjLinkMachineTransactionState::PrepareCommit,
        ..base.clone()
    };
    inconsistent.credential_generation = None;
    assert_eq!(
        inconsistent.clone().validated(),
        Err(DjLinkMachineSettingsError::TransactionStateInconsistent)
    );
}

// -- trust identity pairing, arming, NIL, and canonical formats ----------

const SAMPLE_ADAPTER_GUID: &str = "0123abcd-5678-90ef-abcd-ef0123456789";
const SAMPLE_NETWORK_GUID: &str = "ffffffff-0000-aaaa-bbbb-ccccddddeeee";
const NIL_UUID: &str = "00000000-0000-0000-0000-000000000000";

#[test]
fn guid_pair_must_be_complete_even_when_disarmed() {
    let half_adapter = DjLinkMachineSettingsV2 {
        adapter_guid: Some(SAMPLE_ADAPTER_GUID.to_string()),
        network_guid: None,
        ..DjLinkMachineSettingsV2::default()
    };
    assert_eq!(
        half_adapter.clone().validated(),
        Err(DjLinkMachineSettingsError::GuidPairIncomplete)
    );
    let half_network = DjLinkMachineSettingsV2 {
        adapter_guid: None,
        network_guid: Some(SAMPLE_NETWORK_GUID.to_string()),
        auto_start_armed: false,
        ..DjLinkMachineSettingsV2::default()
    };
    assert_eq!(
        half_network.clone().validated(),
        Err(DjLinkMachineSettingsError::GuidPairIncomplete),
        "half-bound settings are rejected even when disarmed"
    );
    for mut armed_half in [half_adapter.clone(), half_network.clone()] {
        armed_half.auto_start_armed = true;
        assert_eq!(
            armed_half.validated(),
            Err(DjLinkMachineSettingsError::GuidPairIncomplete),
            "arming cannot legitimize a half-bound pair"
        );
    }

    // The journal preserves pairing validation across transitions.
    assert_eq!(
        half_adapter.begin_prepare_commit(half_adapter.revision, 4),
        Err(DjLinkMachineSettingsError::GuidPairIncomplete)
    );
    // Runtime recovery only accepts a fully validated rollback preimage.
    assert!(half_network.validated().is_err());
}

#[test]
fn auto_start_arming_requires_complete_identity_pair() {
    let unarmed_empty = DjLinkMachineSettingsV2::default();
    assert_eq!(unarmed_empty.auto_start_armed, false);
    let mut armed_empty = unarmed_empty.clone();
    armed_empty.auto_start_armed = true;
    assert_eq!(
        armed_empty.clone().validated(),
        Err(DjLinkMachineSettingsError::ArmedWithoutGuidPair)
    );

    let mut armed_pair = fully_populated_settings();
    armed_pair.auto_start_armed = true;
    assert!(
        armed_pair.clone().validated().is_ok(),
        "armed settings with the complete pair are valid"
    );
    let mut disarmed_pair = armed_pair.clone();
    disarmed_pair.auto_start_armed = false;
    assert!(disarmed_pair.validated().is_ok());

    // Arming survives journal phases only while the pair stays complete.
    let prepared = armed_pair
        .begin_prepare_commit(armed_pair.revision, 9)
        .expect("pair present");
    assert_eq!(prepared.auto_start_armed, true);
    let committing = prepared.stage_committing(prepared.revision).expect("stage");
    let finished = committing
        .finish_commit(committing.revision)
        .expect("finish");
    let revalidated = finished.clone().validated();
    assert_eq!(
        revalidated,
        Ok(finished),
        "finished journal state still validates"
    );
}

#[test]
fn nil_and_noncanonical_identity_fields_are_rejected_with_exact_variants() {
    let base = fully_populated_settings();
    for adapter in [
        NIL_UUID,
        SAMPLE_ADAPTER_GUID.to_uppercase().as_str(),
        "0123abcd567890efabcdef0123456789",
        "0123abcd-5678-90ef-abcd-ef012345678",
        "{0123abcd-5678-90ef-abcd-ef0123456789}",
        "",
    ] {
        let mut bad = base.clone();
        bad.adapter_guid = Some(adapter.to_string());
        assert_eq!(
            bad.clone().validated(),
            Err(DjLinkMachineSettingsError::InvalidAdapterGuid),
            "adapter {adapter:?}"
        );
    }
    for network in [
        NIL_UUID,
        SAMPLE_NETWORK_GUID.to_uppercase().as_str(),
        "ffffffff0000aaaabbbbccccddddeeee",
        "ffffffff-0000-aaaa-bbbb-ccccddddeee",
        "ffffffff-0000-aaaa-bbbb-ccccddddeeeg",
        "",
    ] {
        let mut bad = base.clone();
        bad.network_guid = Some(network.to_string());
        assert_eq!(
            bad.clone().validated(),
            Err(DjLinkMachineSettingsError::InvalidNetworkGuid),
            "network {network:?}"
        );
    }
    // Positive control: a second distinct canonical pair validates.
    let alternate = DjLinkMachineSettingsV2 {
        adapter_guid: Some(NIL_UUID.replacen('0', "a", 1).as_str().to_string()),
        network_guid: Some(SAMPLE_NETWORK_GUID.to_string()),
        ..fully_populated_settings()
    };
    assert!(alternate.validated().is_ok());
}

#[test]
fn persisted_half_bound_identity_blocks_load_and_persist() {
    let dir = TempDir::new("half-bound");
    let path = dir.path().join("half.json");
    let body = format!(
        r#"{{"version":{DJ_LINK_MACHINE_SETTINGS_VERSION},"revision":2,"transaction_state":"idle","adapter_guid":"{SAMPLE_ADAPTER_GUID}","auto_start_armed":false}}"#
    );
    fs::write(&path, &body).expect("seed");
    match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::BlockedCorrupt { detail } => {
            assert!(
                detail.contains("together") || detail.contains("GUID"),
                "detail should name the pairing rule: {detail}"
            );
        }
        other => panic!("expected corrupt block for half-bound identity, got {other:?}"),
    }
    assert_eq!(
        fs::read(&path).expect("after"),
        body.as_bytes(),
        "file untouched"
    );

    let persist_target = dir.path().join("never.json");
    let half = DjLinkMachineSettingsV2 {
        adapter_guid: Some(SAMPLE_ADAPTER_GUID.to_string()),
        network_guid: None,
        ..DjLinkMachineSettingsV2::default()
    };
    assert!(persist_dj_link_machine_settings_to_path_with(
        &persist_target,
        &half,
        |_t, _target| Ok(())
    )
    .is_err());
    assert!(
        !persist_target.exists(),
        "half-bound settings never persist"
    );
}

// -- CAS and transaction journal ----------------------------------------

#[test]
fn transaction_journal_happy_path_bumps_each_phase() {
    let initial = fully_populated_settings();
    let prepared = initial
        .begin_prepare_commit(initial.revision, 12)
        .expect("begin");
    assert_eq!(
        prepared.transaction_state,
        DjLinkMachineTransactionState::PrepareCommit
    );
    assert_eq!(prepared.revision, initial.revision + 1);
    assert_eq!(prepared.credential_generation, Some(12));

    let committing = prepared.stage_committing(prepared.revision).expect("stage");
    assert_eq!(
        committing.transaction_state,
        DjLinkMachineTransactionState::Committing
    );
    assert_eq!(committing.revision, prepared.revision + 1);

    let finished = committing
        .finish_commit(committing.revision)
        .expect("finish");
    assert_eq!(
        finished.transaction_state,
        DjLinkMachineTransactionState::Idle
    );
    assert_eq!(finished.revision, committing.revision + 1);
    assert_eq!(finished.credential_generation, Some(12));
}

#[test]
fn transaction_journal_enforces_cas_and_transitions() {
    let initial = fully_populated_settings();

    assert_eq!(
        initial.begin_prepare_commit(initial.revision + 1, 2),
        Err(DjLinkMachineSettingsError::CasMismatch {
            expected: initial.revision + 1,
            found: initial.revision,
        })
    );

    let prepared = initial
        .begin_prepare_commit(initial.revision, 2)
        .expect("begin");
    assert_eq!(
        prepared.finish_commit(prepared.revision),
        Err(DjLinkMachineSettingsError::InvalidTransition {
            from: "committing",
            to: "idle"
        })
    );
    assert_eq!(
        initial.stage_committing(initial.revision),
        Err(DjLinkMachineSettingsError::InvalidTransition {
            from: "prepare_commit",
            to: "committing"
        })
    );

    let aborted = prepared
        .abort_transaction(prepared.revision)
        .expect("abort");
    assert_eq!(
        aborted.transaction_state,
        DjLinkMachineTransactionState::Idle
    );
    assert_eq!(aborted.revision, prepared.revision + 1);
    assert_eq!(
        aborted.abort_transaction(aborted.revision),
        Err(DjLinkMachineSettingsError::InvalidTransition {
            from: "prepare_commit",
            to: "idle"
        })
    );
}

#[test]
fn transaction_journal_detects_revision_exhaustion() {
    let mut exhausted = fully_populated_settings();
    exhausted.revision = u64::MAX - 1;
    assert_eq!(
        exhausted.begin_prepare_commit(exhausted.revision, 3),
        Err(DjLinkMachineSettingsError::CasExhausted)
    );
}

// -- bounded, atomic settings I/O -----------------------------------------

#[test]
fn settings_io_missing_defaults_roundtrip_and_atomic_replace() {
    let dir = TempDir::new("io-roundtrip");
    let path = dj_link_machine_settings_path(dir.path());
    assert_eq!(
        load_dj_link_machine_settings_from_path(&path),
        DjLinkMachineSettingsLoadOutcome::MissingDefaults(DjLinkMachineSettingsV2::default())
    );
    let settings = fully_populated_settings();
    persist_dj_link_machine_settings_to_path_with(&path, &settings, |temporary, target| {
        let bytes = fs::read(temporary).expect("temp readable");
        assert!(!target.exists());
        fs::write(target, bytes).expect("replace");
        Ok(())
    })
    .expect("persist");
    assert_eq!(
        load_dj_link_machine_settings_from_path(&path),
        DjLinkMachineSettingsLoadOutcome::Loaded(settings)
    );
}

#[test]
fn settings_io_blocks_corrupt_future_unknown_and_oversize_while_preserving_bytes() {
    let dir = TempDir::new("io-blocked");

    let corrupt_path = dir.path().join("corrupt.json");
    fs::write(&corrupt_path, "{not json").expect("seed");
    let before = fs::read(&corrupt_path).expect("before");
    match load_dj_link_machine_settings_from_path(&corrupt_path) {
        DjLinkMachineSettingsLoadOutcome::BlockedCorrupt { .. } => {}
        other => panic!("expected corrupt block, got {other:?}"),
    }
    assert_eq!(fs::read(&corrupt_path).expect("after"), before);

    let future_path = dir.path().join("future.json");
    let future_body =
        br#"{"version":3,"revision":1,"transaction_state":"idle","auto_start_armed":false}"#;
    fs::write(&future_path, future_body).expect("seed");
    match load_dj_link_machine_settings_from_path(&future_path) {
        DjLinkMachineSettingsLoadOutcome::BlockedFutureVersion { found_version } => {
            assert_eq!(found_version, 3);
        }
        other => panic!("expected future block, got {other:?}"),
    }
    assert_eq!(fs::read(&future_path).expect("after"), future_body);

    let unknown_path = dir.path().join("unknown.json");
    let unknown_body = br#"{"version":2,"revision":1,"transaction_state":"idle","auto_start_armed":false,"web_remote_pin":"123456"}"#;
    fs::write(&unknown_path, unknown_body).expect("seed");
    match load_dj_link_machine_settings_from_path(&unknown_path) {
        DjLinkMachineSettingsLoadOutcome::BlockedCorrupt { .. } => {}
        other => panic!("expected unknown-field block, got {other:?}"),
    }
    assert_eq!(fs::read(&unknown_path).expect("after"), unknown_body);

    let oversize_path = dir.path().join("oversize.json");
    fs::write(
        &oversize_path,
        vec![b' '; (MAX_DJ_LINK_MACHINE_SETTINGS_BYTES + 1) as usize],
    )
    .expect("seed");
    match load_dj_link_machine_settings_from_path(&oversize_path) {
        DjLinkMachineSettingsLoadOutcome::BlockedCorrupt { detail } => {
            assert!(detail.contains("byte"), "detail: {detail}");
        }
        other => panic!("expected oversize block, got {other:?}"),
    }
    assert_eq!(
        fs::read(&oversize_path).expect("after").len(),
        (MAX_DJ_LINK_MACHINE_SETTINGS_BYTES + 1) as usize
    );
}

#[test]
fn settings_io_classifies_versions_above_u32_max_as_future_not_corrupt() {
    let dir = TempDir::new("huge-version");
    let path = dir.path().join("huge-version.json");
    let body = br#"{"version":4294967296,"revision":1,"transaction_state":"idle","auto_start_armed":false}"#;
    fs::write(&path, body).expect("seed");
    match load_dj_link_machine_settings_from_path(&path) {
        DjLinkMachineSettingsLoadOutcome::BlockedFutureVersion { found_version } => {
            assert_eq!(found_version, u64::from(u32::MAX) + 1);
        }
        other => panic!("expected future block, got {other:?}"),
    }
    assert_eq!(fs::read(&path).expect("after"), body.to_vec());
}

#[test]
fn settings_persist_rejects_invalid_settings_and_leaves_target_absent() {
    let dir = TempDir::new("persist-invalid");
    let path = dj_link_machine_settings_path(dir.path());
    let mut future = DjLinkMachineSettingsV2::default();
    future.version = 99;
    assert!(
        persist_dj_link_machine_settings_to_path_with(&path, &future, |_t, _target| Ok(()))
            .is_err()
    );
    assert!(!path.exists());

    let settings = fully_populated_settings();
    assert!(persist_dj_link_machine_settings_to_path_with(
        &path,
        &settings,
        |_temporary, _target| Err(String::from("replace failed")),
    )
    .is_err());
    let stray = fs::read_dir(dir.path())
        .expect("dir")
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"));
    assert!(
        !stray,
        "temporary files must be cleaned up on replace failure"
    );
}
