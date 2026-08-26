//! Machine-local DJ-Link persistence primitives.
//!
//! Two isolated stores live here:
//!
//! - [`DjLinkCredentialStore`] persists ONLY the raw 32-byte DJ-Link token in
//!   the Windows Credential Manager under the fixed target
//!   [`DJ_LINK_CREDENTIAL_TARGET`] as a `CRED_TYPE_GENERIC` /
//!   `CRED_PERSIST_LOCAL_MACHINE` credential wrapped in a strict
//!   magic/version/generation blob. The six-digit Web Remote PIN is process
//!   local by contract and is never persisted by this module.
//! - [`DjLinkMachineSettingsV2`] persists non-secret metadata only (revision,
//!   transaction state, credential generation, NIC/network GUIDs, bind
//!   endpoint, auto-start arming). No token, token hash, PIN, or blob ever
//!   enters the settings file, which is enforced structurally by the field
//!   set and by tests.
//!
//! Writer contract: a single Syndocal process owns writes to both stores
//! (single-instance enforcement lives at the app layer). Two same-user
//! writers would race around this journal's CAS guarantees, so concurrent
//! external mutation of the fixed credential target is out of contract;
//! readers always validate what they read and fail closed on surprises.
//!
//! Scope guards for this tranche: no NIC enumeration, no startup/listener/UI
//! integration, and no network I/O. Loaders never mutate or delete corrupt or
//! future-version files; they block and preserve them.

use crate::dj_link_network::is_canonical_nonnil_uuid;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fmt,
    fs::{self},
    io::{Read, Write},
    net::IpAddr,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use zeroize::{Zeroize, Zeroizing};

/// Fixed Windows Credential Manager target for the Syndocal DJ-Link token.
pub const DJ_LINK_CREDENTIAL_TARGET: &str = "jp.seraf.ktn.syndocal/dj-link/v1";
/// Separate machine-local Credential Manager target used only while an arm or
/// rotation transaction is active. It holds the previously accepted token so
/// a crash or a failed final settings write can restore the authority without
/// ever placing secret material in the settings journal.
pub const DJ_LINK_CREDENTIAL_ROLLBACK_TARGET: &str = "jp.seraf.ktn.syndocal/dj-link/v1/rollback";
/// Raw DJ-Link token length in bytes.
pub const DJ_LINK_TOKEN_LEN: usize = 32;
const DJ_LINK_TOKEN_BLOB_MAGIC: [u8; 8] = *b"SYNDJLK!";
/// Blob schema version; readers reject any other value.
pub const DJ_LINK_TOKEN_BLOB_VERSION: u16 = 1;
const DJ_LINK_GENERATION_OFFSET: usize = DJ_LINK_TOKEN_BLOB_MAGIC.len() + 2;
/// magic(8) + version u16 LE(2) + generation u64 LE(8) + token(32).
pub const DJ_LINK_TOKEN_BLOB_LEN: usize = DJ_LINK_GENERATION_OFFSET + 8 + DJ_LINK_TOKEN_LEN;
/// Windows Credential Manager target-name ceiling (`CRED_MAX_STRING_LENGTH`).
pub const MAX_DJ_LINK_CREDENTIAL_TARGET_CHARS: usize = 256;
/// Windows Credential Manager generic-blob ceiling (`CRED_MAX_CREDENTIAL_BLOB_SIZE`).
pub const MAX_DJ_LINK_TOKEN_BLOB_BYTES: usize = 2560;
/// Fixed settings file name below the machine-local data directory.
pub const DJ_LINK_MACHINE_SETTINGS_FILE: &str = "dj-link-machine-settings.json";
/// Bounded settings-file read limit; larger files are corrupt by definition.
pub const MAX_DJ_LINK_MACHINE_SETTINGS_BYTES: u64 = 4 * 1024;
/// Current settings schema version; readers reject newer schemas fail-closed.
pub const DJ_LINK_MACHINE_SETTINGS_VERSION: u32 = 2;

static SETTINGS_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

// ---------------------------------------------------------------------------
// Secret-bearing token type
// ---------------------------------------------------------------------------

/// The raw 32-byte DJ-Link token. Debug output is redacted, equality is
/// constant time, and the backing bytes are wiped on drop.
#[derive(Clone)]
pub struct DjLinkToken([u8; DJ_LINK_TOKEN_LEN]);

impl DjLinkToken {
    /// Rejects lengths other than [`DJ_LINK_TOKEN_LEN`] and all-zero tokens.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != DJ_LINK_TOKEN_LEN || bytes.iter().all(|byte| *byte == 0) {
            return None;
        }
        let mut array = [0_u8; DJ_LINK_TOKEN_LEN];
        array.copy_from_slice(bytes);
        Some(Self(array))
    }

    /// Draw a new nonzero token from the operating-system CSPRNG.  The raw
    /// bytes remain inside the zeroizing token wrapper until the caller
    /// deliberately injects its encoded wire form into a live listener.
    pub fn generate() -> Result<Self, DjLinkCredentialError> {
        let mut bytes = Zeroizing::new([0_u8; DJ_LINK_TOKEN_LEN]);
        getrandom::getrandom(&mut *bytes).map_err(|error| DjLinkCredentialError::RandomFailed {
            detail: error.to_string(),
        })?;
        Self::from_bytes(&*bytes).ok_or(DjLinkCredentialError::RandomFailed {
            detail: "OS CSPRNG returned an all-zero DJ-Link token".to_string(),
        })
    }

    /// Explicit accessor marking every place the secret material is exposed.
    pub fn expose(&self) -> &[u8; DJ_LINK_TOKEN_LEN] {
        &self.0
    }
}

impl PartialEq for DjLinkToken {
    fn eq(&self, other: &Self) -> bool {
        constant_time_eq(&self.0, &other.0)
    }
}

impl Eq for DjLinkToken {}

impl fmt::Debug for DjLinkToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DjLinkToken(REDACTED)")
    }
}

impl Drop for DjLinkToken {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// A decoded credential: the strict blob generation plus its token.
pub struct DjLinkTokenRecord {
    pub generation: u64,
    pub token: DjLinkToken,
}

impl fmt::Debug for DjLinkTokenRecord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DjLinkTokenRecord")
            .field("generation", &self.generation)
            .field("token", &"REDACTED")
            .finish()
    }
}

impl PartialEq for DjLinkTokenRecord {
    fn eq(&self, other: &Self) -> bool {
        self.generation == other.generation && self.token == other.token
    }
}

impl Eq for DjLinkTokenRecord {}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0_u8;
    for (a, b) in left.iter().zip(right.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

// ---------------------------------------------------------------------------
// Strict token blob codec
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DjLinkTokenBlobError {
    Empty,
    TooLong,
    BadMagic,
    UnsupportedVersion(u16),
    BadLength { expected: usize, found: usize },
    InvalidGeneration,
    InvalidToken,
}

impl fmt::Display for DjLinkTokenBlobError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("token blob is empty"),
            Self::TooLong => write!(
                formatter,
                "token blob exceeds {} bytes",
                DJ_LINK_TOKEN_BLOB_LEN
            ),
            Self::BadMagic => formatter.write_str("token blob magic mismatch"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "token blob version {version} is unsupported")
            }
            Self::BadLength { expected, found } => {
                write!(
                    formatter,
                    "token blob length {found} does not match {expected}"
                )
            }
            Self::InvalidGeneration => formatter.write_str("token blob generation must be nonzero"),
            Self::InvalidToken => formatter.write_str("token blob carries an all-zero token"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DjLinkCredentialError {
    PlatformUnsupported,
    TargetNameInvalid,
    BlobTooLarge,
    /// The credential under the fixed target exists but its blob is missing
    /// or oversized foreign data. Distinct from [`DjLinkCredentialError::
    /// VerifyMismatch`] so stored corruption is never mislabeled as a failed
    /// verification of our own write.
    StoredBlobInvalid,
    BlobEncode(DjLinkTokenBlobError),
    #[cfg(test)]
    NotConfigured,
    WriteFailed {
        code: i32,
    },
    ReadFailed {
        code: i32,
    },
    RevokeFailed {
        code: i32,
    },
    VerifyMismatch,
    PersistenceDowngraded,
    RandomFailed {
        detail: String,
    },
}

impl fmt::Display for DjLinkCredentialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlatformUnsupported => {
                formatter.write_str("machine-local DJ-Link credentials are unsupported here")
            }
            Self::TargetNameInvalid => formatter.write_str("credential target name is invalid"),
            Self::BlobTooLarge => formatter.write_str("credential blob exceeds the platform limit"),
            Self::StoredBlobInvalid => {
                formatter.write_str("stored DJ-Link credential blob is unusable")
            }
            Self::BlobEncode(error) => write!(formatter, "credential blob rejected: {error}"),
            #[cfg(test)]
            Self::NotConfigured => formatter.write_str("no DJ-Link credential is configured"),
            Self::WriteFailed { code } => {
                write!(formatter, "credential write failed (code {code})")
            }
            Self::ReadFailed { code } => write!(formatter, "credential read failed (code {code})"),
            Self::RevokeFailed { code } => {
                write!(formatter, "credential revoke failed (code {code})")
            }
            Self::VerifyMismatch => {
                formatter.write_str("stored credential did not verify against what was written")
            }
            Self::PersistenceDowngraded => {
                formatter.write_str("credential was not persisted machine-local as required")
            }
            Self::RandomFailed { detail } => {
                write!(formatter, "DJ-Link token generation failed: {detail}")
            }
        }
    }
}

/// Encodes the strict machine-local blob: magic, LE version, LE nonzero
/// generation, raw token. The buffer is zeroized on drop.
pub fn encode_dj_link_token_blob(
    token: &DjLinkToken,
    generation: u64,
) -> Result<Zeroizing<Vec<u8>>, DjLinkCredentialError> {
    if generation == 0 {
        return Err(DjLinkCredentialError::BlobEncode(
            DjLinkTokenBlobError::InvalidGeneration,
        ));
    }
    if token.expose().iter().all(|byte| *byte == 0) {
        return Err(DjLinkCredentialError::BlobEncode(
            DjLinkTokenBlobError::InvalidToken,
        ));
    }
    let mut blob = Zeroizing::new(Vec::with_capacity(DJ_LINK_TOKEN_BLOB_LEN));
    blob.extend_from_slice(&DJ_LINK_TOKEN_BLOB_MAGIC);
    blob.extend_from_slice(&DJ_LINK_TOKEN_BLOB_VERSION.to_le_bytes());
    blob.extend_from_slice(&generation.to_le_bytes());
    blob.extend_from_slice(token.expose());
    debug_assert_eq!(blob.len(), DJ_LINK_TOKEN_BLOB_LEN);
    Ok(blob)
}

/// Strictly decodes a machine-local blob, rejecting bad magic, unsupported
/// versions, wrong lengths, zero generations, and zero tokens.
pub fn decode_dj_link_token_blob(blob: &[u8]) -> Result<DjLinkTokenRecord, DjLinkCredentialError> {
    if blob.is_empty() {
        return Err(DjLinkCredentialError::BlobEncode(
            DjLinkTokenBlobError::Empty,
        ));
    }
    if blob.len() > DJ_LINK_TOKEN_BLOB_LEN {
        return Err(DjLinkCredentialError::BlobEncode(
            DjLinkTokenBlobError::TooLong,
        ));
    }
    if blob.len() != DJ_LINK_TOKEN_BLOB_LEN {
        return Err(DjLinkCredentialError::BlobEncode(
            DjLinkTokenBlobError::BadLength {
                expected: DJ_LINK_TOKEN_BLOB_LEN,
                found: blob.len(),
            },
        ));
    }
    if !constant_time_eq(
        &blob[..DJ_LINK_TOKEN_BLOB_MAGIC.len()],
        &DJ_LINK_TOKEN_BLOB_MAGIC,
    ) {
        return Err(DjLinkCredentialError::BlobEncode(
            DjLinkTokenBlobError::BadMagic,
        ));
    }
    let version = u16::from_le_bytes(
        blob[DJ_LINK_TOKEN_BLOB_MAGIC.len()..DJ_LINK_GENERATION_OFFSET]
            .try_into()
            .expect("two-byte slice"),
    );
    if version != DJ_LINK_TOKEN_BLOB_VERSION {
        return Err(DjLinkCredentialError::BlobEncode(
            DjLinkTokenBlobError::UnsupportedVersion(version),
        ));
    }
    let generation = u64::from_le_bytes(
        blob[DJ_LINK_GENERATION_OFFSET..DJ_LINK_GENERATION_OFFSET + 8]
            .try_into()
            .expect("eight-byte slice"),
    );
    if generation == 0 {
        return Err(DjLinkCredentialError::BlobEncode(
            DjLinkTokenBlobError::InvalidGeneration,
        ));
    }
    let token = DjLinkToken::from_bytes(&blob[DJ_LINK_GENERATION_OFFSET + 8..]).ok_or(
        DjLinkCredentialError::BlobEncode(DjLinkTokenBlobError::InvalidToken),
    )?;
    Ok(DjLinkTokenRecord { generation, token })
}

// ---------------------------------------------------------------------------
// Credential store trait and platform implementations
// ---------------------------------------------------------------------------

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DjLinkCredentialPersistenceScope {
    /// Windows Credential Manager with `CRED_PERSIST_LOCAL_MACHINE`: the
    /// secret is bound to this user account on this machine only. It survives
    /// reboot and logoff, is invisible to other local users, and roaming
    /// profiles or sync services never carry it to other machines.
    MachineLocal,
    /// No persistent machine-local storage exists on this platform.
    Unsupported,
}

/// Fail-closed platform capability query for machine-local persistence.
pub fn dj_link_machine_credential_persistence_supported() -> bool {
    #[cfg(target_os = "windows")]
    {
        true
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

pub trait DjLinkCredentialStore {
    #[cfg(test)]
    fn persistence_scope(&self) -> DjLinkCredentialPersistenceScope;
    fn save_token(&self, generation: u64, token: &DjLinkToken)
        -> Result<(), DjLinkCredentialError>;
    fn load_token(&self) -> Result<Option<DjLinkTokenRecord>, DjLinkCredentialError>;
    fn revoke_token(&self) -> Result<(), DjLinkCredentialError>;
    /// Saves the previously accepted primary credential for an in-flight
    /// settings transaction. The rollback target is still Windows Credential
    /// Manager; no secret ever enters the JSON journal.
    fn save_rollback_token(
        &self,
        generation: u64,
        token: &DjLinkToken,
    ) -> Result<(), DjLinkCredentialError>;
    fn load_rollback_token(&self) -> Result<Option<DjLinkTokenRecord>, DjLinkCredentialError>;
    fn revoke_rollback_token(&self) -> Result<(), DjLinkCredentialError>;
}

#[derive(Debug)]
pub struct PlatformDjLinkCredentialStore {
    backend: Option<Box<dyn DjLinkCredentialBackend>>,
    rollback_backend: Option<Box<dyn DjLinkCredentialBackend>>,
}

impl PlatformDjLinkCredentialStore {
    pub fn new() -> Result<Self, DjLinkCredentialError> {
        if !dj_link_machine_credential_persistence_supported() {
            return Err(DjLinkCredentialError::PlatformUnsupported);
        }
        #[cfg(target_os = "windows")]
        {
            Ok(Self {
                backend: Some(Box::new(win_credentials::WindowsCredentialBackend::new(
                    DJ_LINK_CREDENTIAL_TARGET,
                ))),
                rollback_backend: Some(Box::new(win_credentials::WindowsCredentialBackend::new(
                    DJ_LINK_CREDENTIAL_ROLLBACK_TARGET,
                ))),
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(DjLinkCredentialError::PlatformUnsupported)
        }
    }
}

impl DjLinkCredentialStore for PlatformDjLinkCredentialStore {
    #[cfg(test)]
    fn persistence_scope(&self) -> DjLinkCredentialPersistenceScope {
        if self.backend.is_some() {
            DjLinkCredentialPersistenceScope::MachineLocal
        } else {
            DjLinkCredentialPersistenceScope::Unsupported
        }
    }

    fn save_token(
        &self,
        generation: u64,
        token: &DjLinkToken,
    ) -> Result<(), DjLinkCredentialError> {
        let backend = self
            .backend
            .as_deref()
            .ok_or(DjLinkCredentialError::PlatformUnsupported)?;
        save_token_via_backend(backend, generation, token)
    }

    fn load_token(&self) -> Result<Option<DjLinkTokenRecord>, DjLinkCredentialError> {
        let backend = self
            .backend
            .as_deref()
            .ok_or(DjLinkCredentialError::PlatformUnsupported)?;
        load_token_via_backend(backend)
    }

    fn revoke_token(&self) -> Result<(), DjLinkCredentialError> {
        let backend = self
            .backend
            .as_deref()
            .ok_or(DjLinkCredentialError::PlatformUnsupported)?;
        backend.delete_credential()
    }

    fn save_rollback_token(
        &self,
        generation: u64,
        token: &DjLinkToken,
    ) -> Result<(), DjLinkCredentialError> {
        let backend = self
            .rollback_backend
            .as_deref()
            .ok_or(DjLinkCredentialError::PlatformUnsupported)?;
        save_token_via_backend(backend, generation, token)
    }

    fn load_rollback_token(&self) -> Result<Option<DjLinkTokenRecord>, DjLinkCredentialError> {
        let backend = self
            .rollback_backend
            .as_deref()
            .ok_or(DjLinkCredentialError::PlatformUnsupported)?;
        load_token_via_backend(backend)
    }

    fn revoke_rollback_token(&self) -> Result<(), DjLinkCredentialError> {
        let backend = self
            .rollback_backend
            .as_deref()
            .ok_or(DjLinkCredentialError::PlatformUnsupported)?;
        backend.delete_credential()
    }
}

pub fn platform_dj_link_credential_store(
) -> Result<Box<dyn DjLinkCredentialStore>, DjLinkCredentialError> {
    Ok(Box::new(PlatformDjLinkCredentialStore::new()?))
}

// Compile-time proof that the store stays Send + Sync (safe for Tauri
// managed state); this holds only while the boxed backend trait carries
// both auto traits.
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PlatformDjLinkCredentialStore>();
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadbackType {
    Generic,
    Foreign,
}

pub(crate) struct CredentialReadback {
    blob: Zeroizing<Vec<u8>>,
    credential_type: ReadbackType,
    machine_persisted: bool,
    target_matches: bool,
}

/// Manual redacted debug: renders lengths and classification flags only, so
/// no raw token/blob byte (nor any hex or base64 rendering of them) can ever
/// reach logs through a derived `Debug`.
impl fmt::Debug for CredentialReadback {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CredentialReadback")
            .field("blob_len", &self.blob.len())
            .field("credential_type", &self.credential_type)
            .field("machine_persisted", &self.machine_persisted)
            .field("target_matches", &self.target_matches)
            .finish_non_exhaustive()
    }
}

/// Backend contract for the fixed-target credential store. `Send + Sync` is
/// required so [`PlatformDjLinkCredentialStore`] can live in Tauri managed
/// state, which demands both auto traits.
trait DjLinkCredentialBackend: fmt::Debug + Send + Sync {
    fn write_credential(&self, blob: &[u8]) -> Result<(), DjLinkCredentialError>;
    /// Returns `Ok(None)` when the fixed target holds no credential.
    fn read_credential(&self) -> Result<Option<CredentialReadback>, DjLinkCredentialError>;
    /// Idempotent: deleting an absent credential succeeds.
    fn delete_credential(&self) -> Result<(), DjLinkCredentialError>;
}

fn validate_save_preconditions(blob_len: usize) -> Result<(), DjLinkCredentialError> {
    if DJ_LINK_CREDENTIAL_TARGET.is_empty()
        || DJ_LINK_CREDENTIAL_TARGET.chars().count() > MAX_DJ_LINK_CREDENTIAL_TARGET_CHARS
    {
        return Err(DjLinkCredentialError::TargetNameInvalid);
    }
    if blob_len > MAX_DJ_LINK_TOKEN_BLOB_BYTES {
        return Err(DjLinkCredentialError::BlobTooLarge);
    }
    Ok(())
}

fn save_token_via_backend(
    backend: &dyn DjLinkCredentialBackend,
    generation: u64,
    token: &DjLinkToken,
) -> Result<(), DjLinkCredentialError> {
    let blob = encode_dj_link_token_blob(token, generation)?;
    validate_save_preconditions(blob.len())?;
    backend.write_credential(&blob)?;
    let readback = match backend.read_credential() {
        Ok(readback) => readback,
        Err(error) => {
            // A successful write followed by a failed read-back would leave
            // our freshly written secret in the store. Perform one bounded,
            // best-effort revoke (single attempt, result ignored) so a failed
            // save never lingers, and propagate the ORIGINAL read error
            // verbatim. No secret material was obtained here, so there is
            // nothing extra to zeroize on this path.
            let _ = backend.delete_credential();
            return Err(error);
        }
    };
    match readback {
        None => {
            let _ = backend.delete_credential();
            Err(DjLinkCredentialError::VerifyMismatch)
        }
        Some(readback) => {
            let verified = verify_credential_readback(&blob, &readback);
            if verified.is_err() {
                let _ = backend.delete_credential();
            }
            verified
        }
    }
}

fn load_token_via_backend(
    backend: &dyn DjLinkCredentialBackend,
) -> Result<Option<DjLinkTokenRecord>, DjLinkCredentialError> {
    match backend.read_credential()? {
        None => Ok(None),
        Some(readback) => {
            if readback.credential_type != ReadbackType::Generic || !readback.target_matches {
                return Err(DjLinkCredentialError::VerifyMismatch);
            }
            if !readback.machine_persisted {
                // Leave a downgraded credential in place: removing evidence of
                // platform policy changes helps nobody; fail closed instead.
                return Err(DjLinkCredentialError::PersistenceDowngraded);
            }
            decode_dj_link_token_blob(&readback.blob).map(Some)
        }
    }
}

fn verify_credential_readback(
    expected_blob: &[u8],
    readback: &CredentialReadback,
) -> Result<(), DjLinkCredentialError> {
    if readback.credential_type != ReadbackType::Generic || !readback.target_matches {
        return Err(DjLinkCredentialError::VerifyMismatch);
    }
    // Session-persistence capability check: the credential manager must report
    // machine-local persistence, otherwise roaming policies downgraded the
    // write and the token would silently leave this machine's scope.
    if !readback.machine_persisted {
        return Err(DjLinkCredentialError::PersistenceDowngraded);
    }
    if !constant_time_eq(&readback.blob, expected_blob) {
        return Err(DjLinkCredentialError::VerifyMismatch);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
mod win_credentials {
    //! Thin Win32 bindings layer. Every buffer handed back by the OS is
    //! wiped before `CredFree`; no secret material survives this module.

    use super::{
        CredentialReadback, DjLinkCredentialBackend, DjLinkCredentialError, ReadbackType, Zeroize,
        Zeroizing, MAX_DJ_LINK_CREDENTIAL_TARGET_CHARS,
    };
    use windows::core::{HRESULT, PCWSTR, PWSTR};
    use windows::Win32::Foundation::ERROR_NOT_FOUND;
    use windows::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_MAX_CREDENTIAL_BLOB_SIZE,
        CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
    };

    #[derive(Debug)]
    pub(super) struct WindowsCredentialBackend {
        target: &'static str,
    }

    impl WindowsCredentialBackend {
        pub(super) const fn new(target: &'static str) -> Self {
            Self { target }
        }

        fn target_wide(&self) -> Vec<u16> {
            self.target
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect()
        }
    }

    fn is_not_found(error: &windows::core::Error) -> bool {
        error.code() == HRESULT::from_win32(ERROR_NOT_FOUND.0)
    }

    fn error_code(error: &windows::core::Error) -> i32 {
        error.code().0
    }

    fn pwstr_matches(ptr: PWSTR, expected_with_nul: &[u16]) -> bool {
        if ptr.0.is_null() {
            return false;
        }
        let expected = &expected_with_nul[..expected_with_nul.len() - 1];
        let max_chars = MAX_DJ_LINK_CREDENTIAL_TARGET_CHARS;
        // SAFETY: TargetName returned by CredReadW lives inside the OS-owned
        // credential buffer until CredFree runs. The scan dereferences only
        // indices 0..max_chars and fails closed when no nul terminator shows
        // up within the platform target-name ceiling instead of reading past
        // the bounded window.
        unsafe {
            let mut length = 0_usize;
            while length < max_chars && *ptr.0.add(length) != 0 {
                length += 1;
            }
            if length == max_chars {
                // Unterminated or boundary-length target: treat as foreign.
                return false;
            }
            let actual = std::slice::from_raw_parts(ptr.0, length);
            actual == expected
        }
    }

    impl DjLinkCredentialBackend for WindowsCredentialBackend {
        fn write_credential(&self, blob: &[u8]) -> Result<(), DjLinkCredentialError> {
            let mut target = self.target_wide();
            let mut credential = CREDENTIALW::default();
            credential.Type = CRED_TYPE_GENERIC;
            credential.TargetName = PWSTR(target.as_mut_ptr());
            credential.Comment = PWSTR::null();
            credential.CredentialBlobSize = blob.len() as u32;
            credential.CredentialBlob = blob.as_ptr() as *mut u8;
            credential.Persist = CRED_PERSIST_LOCAL_MACHINE;
            // CRED_PERSIST_LOCAL_MACHINE stores per-user, machine-local: only
            // this Windows user on this machine can read the secret back, and
            // it is never roamed. Writes assume the single-writer contract
            // documented in the module header.
            credential.AttributeCount = 0;
            credential.Attributes = std::ptr::null_mut();
            credential.TargetAlias = PWSTR::null();
            credential.UserName = PWSTR::null();
            // SAFETY: credential and its buffers outlive the CredWriteW call.
            unsafe { CredWriteW(&credential, 0) }.map_err(|error| {
                DjLinkCredentialError::WriteFailed {
                    code: error_code(&error),
                }
            })
        }

        fn read_credential(&self) -> Result<Option<CredentialReadback>, DjLinkCredentialError> {
            let target = self.target_wide();
            let mut pointer: *mut CREDENTIALW = std::ptr::null_mut();
            // SAFETY: pointer receives an OS-allocated CREDENTIALW that we own
            // until CredFree; target stays alive for the duration of the call.
            if let Err(error) = unsafe {
                CredReadW(
                    PCWSTR(target.as_ptr()),
                    CRED_TYPE_GENERIC,
                    None,
                    &mut pointer,
                )
            } {
                return if is_not_found(&error) {
                    Ok(None)
                } else {
                    Err(DjLinkCredentialError::ReadFailed {
                        code: error_code(&error),
                    })
                };
            }
            // SAFETY: CredReadW reported success, so pointer is valid.
            let credential = unsafe { &*pointer };
            let blob_size = credential.CredentialBlobSize as usize;
            let credential_type = if credential.Type == CRED_TYPE_GENERIC {
                ReadbackType::Generic
            } else {
                ReadbackType::Foreign
            };
            let machine_persisted = credential.Persist == CRED_PERSIST_LOCAL_MACHINE;
            let target_matches = pwstr_matches(credential.TargetName, &target);
            let mut blob = Zeroizing::new(Vec::new());
            if blob_size > 0 {
                if credential.CredentialBlob.is_null() {
                    // The OS reported a non-empty blob without a buffer:
                    // refuse before any slice construction instead of risking
                    // a null-pointer span.
                    // SAFETY: pointer was allocated by CredReadW; no wipe is
                    // possible without a buffer.
                    unsafe { CredFree(pointer.cast()) };
                    return Err(DjLinkCredentialError::StoredBlobInvalid);
                }
                // An oversized blob cannot be ours: classify it precisely as
                // unusable stored data instead of mislabeling foreign garbage
                // as a verification mismatch.
                if blob_size > CRED_MAX_CREDENTIAL_BLOB_SIZE as usize {
                    // SAFETY: wiping then freeing the OS-owned buffer.
                    unsafe {
                        std::slice::from_raw_parts_mut(credential.CredentialBlob, blob_size)
                            .zeroize();
                        CredFree(pointer.cast());
                    }
                    return Err(DjLinkCredentialError::StoredBlobInvalid);
                }
                // SAFETY: CredentialBlob spans exactly CredentialBlobSize bytes.
                let source =
                    unsafe { std::slice::from_raw_parts(credential.CredentialBlob, blob_size) };
                blob.extend_from_slice(source);
                // Wipe the OS-owned copy before releasing it with CredFree.
                // SAFETY: same span as above, still owned until CredFree.
                unsafe {
                    std::slice::from_raw_parts_mut(credential.CredentialBlob, blob_size).zeroize();
                }
            }
            // SAFETY: pointer was allocated by CredReadW and is released once.
            unsafe { CredFree(pointer.cast()) };
            Ok(Some(CredentialReadback {
                blob,
                credential_type,
                machine_persisted,
                target_matches,
            }))
        }

        fn delete_credential(&self) -> Result<(), DjLinkCredentialError> {
            let target = self.target_wide();
            // SAFETY: target outlives the CredDeleteW call.
            if let Err(error) =
                unsafe { CredDeleteW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None) }
            {
                if !is_not_found(&error) {
                    return Err(DjLinkCredentialError::RevokeFailed {
                        code: error_code(&error),
                    });
                }
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Machine-local non-secret settings metadata
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DjLinkMachineTransactionState {
    /// Stable committed state; all values authoritative.
    Idle,
    /// Update journal written; the credential store mutation may or may not
    /// have happened. Values are staged intent only; recovery must restore
    /// the separately retained old authority.
    PrepareCommit,
    /// A new credential may have been written, but it is not authoritative
    /// until the final `Idle` settings persist succeeds. Recovery restores
    /// the old authority from the rollback credential/preimage.
    Committing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DjLinkMachineSettingsV2 {
    pub version: u32,
    /// Compare-and-swap revision; strictly monotonic across persisted writes.
    pub revision: u64,
    pub transaction_state: DjLinkMachineTransactionState,
    /// Generation recorded in the persisted token blob; `None` when no
    /// credential is configured. Never carries token, hash, PIN, or blob.
    pub credential_generation: Option<u64>,
    /// Non-secret reservation watermark. It survives a rolled-back journal
    /// and a disarm so a staged generation is never reused.
    #[serde(default)]
    pub credential_generation_high_water: u64,
    pub adapter_guid: Option<String>,
    pub network_guid: Option<String>,
    pub bind_ip: Option<String>,
    pub bind_port: Option<u16>,
    pub auto_start_armed: bool,
    /// A disarm reached durable intent but Credential Manager deletion still
    /// needs retrying. It is non-secret and prevents credential restoration
    /// until both primary and rollback records have been removed.
    #[serde(default)]
    pub disarm_cleanup_pending: bool,
    /// Non-secret last-known-good metadata retained only while an active
    /// journal is recoverable. The corresponding token is held separately in
    /// `DJ_LINK_CREDENTIAL_ROLLBACK_TARGET`.
    pub rollback: Option<DjLinkMachineRollback>,
}

/// The prior stable non-secret settings snapshot for an active transaction.
/// It deliberately excludes all credential bytes and derivatives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DjLinkMachineRollback {
    pub credential_generation: Option<u64>,
    pub adapter_guid: Option<String>,
    pub network_guid: Option<String>,
    pub bind_ip: Option<String>,
    pub bind_port: Option<u16>,
    pub auto_start_armed: bool,
}

impl Default for DjLinkMachineSettingsV2 {
    fn default() -> Self {
        Self {
            version: DJ_LINK_MACHINE_SETTINGS_VERSION,
            revision: 1,
            transaction_state: DjLinkMachineTransactionState::Idle,
            credential_generation: None,
            credential_generation_high_water: 0,
            adapter_guid: None,
            network_guid: None,
            bind_ip: None,
            bind_port: None,
            auto_start_armed: false,
            disarm_cleanup_pending: false,
            rollback: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DjLinkMachineSettingsError {
    UnsupportedVersion {
        found: u32,
    },
    InvalidRevision,
    InvalidGeneration,
    GenerationHighWaterInconsistent,
    InvalidAdapterGuid,
    InvalidNetworkGuid,
    /// Exactly one of adapter/network GUID is present; identity is pair-only.
    GuidPairIncomplete,
    /// `auto_start_armed` was set without the complete identity pair.
    ArmedWithoutGuidPair,
    /// Credential cleanup intent is only valid for a disarmed idle authority.
    DisarmCleanupInconsistent,
    InvalidBindIp,
    InvalidBindPort,
    TransactionStateInconsistent,
    TransactionRollbackInconsistent,
    CasMismatch {
        expected: u64,
        found: u64,
    },
    CasExhausted,
    InvalidTransition {
        from: &'static str,
        to: &'static str,
    },
}

impl fmt::Display for DjLinkMachineSettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion { found } => {
                write!(
                    formatter,
                    "DJ-Link machine settings version {found} is unsupported; \
                     the file was left untouched"
                )
            }
            Self::InvalidRevision => {
                formatter.write_str("DJ-Link machine settings revision is reserved")
            }
            Self::InvalidGeneration => {
                formatter.write_str("DJ-Link credential generation must be nonzero")
            }
            Self::GenerationHighWaterInconsistent => formatter.write_str(
                "DJ-Link credential generation high-water mark is below persisted authority",
            ),
            Self::InvalidAdapterGuid => formatter.write_str(
                "adapter GUID must be a canonical lowercase hyphenated UUID and not nil",
            ),
            Self::InvalidNetworkGuid => formatter.write_str(
                "network GUID must be a canonical lowercase hyphenated UUID and not nil",
            ),
            Self::GuidPairIncomplete => formatter
                .write_str("adapter GUID and network GUID must be stored together or not at all"),
            Self::ArmedWithoutGuidPair => formatter
                .write_str("auto-start arming requires the complete adapter/network GUID pair"),
            Self::DisarmCleanupInconsistent => formatter.write_str(
                "credential cleanup pending authority must be idle, disarmed, and free of a rollback journal",
            ),
            Self::InvalidBindIp => formatter.write_str("bind IP is not a canonical IP address"),
            Self::InvalidBindPort => formatter.write_str("bind port must be nonzero"),
            Self::TransactionStateInconsistent => {
                formatter.write_str("active DJ-Link transaction requires a credential generation")
            }
            Self::TransactionRollbackInconsistent => formatter.write_str(
                "active DJ-Link transaction requires non-secret rollback metadata and idle settings must not retain it",
            ),
            Self::CasMismatch { expected, found } => {
                write!(
                    formatter,
                    "DJ-Link machine settings CAS mismatch: expected revision {expected}, found {found}"
                )
            }
            Self::CasExhausted => {
                formatter.write_str("DJ-Link machine settings revision space is exhausted")
            }
            Self::InvalidTransition { from, to } => {
                write!(
                    formatter,
                    "illegal DJ-Link transaction state; the journal requires {from} -> {to}"
                )
            }
        }
    }
}

impl DjLinkMachineSettingsV2 {
    pub fn validated(mut self) -> Result<Self, DjLinkMachineSettingsError> {
        if self.version != DJ_LINK_MACHINE_SETTINGS_VERSION {
            return Err(DjLinkMachineSettingsError::UnsupportedVersion {
                found: self.version,
            });
        }
        if self.revision == 0 || self.revision == u64::MAX {
            return Err(DjLinkMachineSettingsError::InvalidRevision);
        }
        if self.credential_generation == Some(0) {
            return Err(DjLinkMachineSettingsError::InvalidGeneration);
        }
        // Pre-watermark V2 metadata has an exact non-secret lower bound in
        // its present authority. Materialize that bound in memory; the next
        // settings write records it explicitly.
        if self.credential_generation_high_water == 0 {
            self.credential_generation_high_water = self
                .credential_generation
                .into_iter()
                .chain(
                    self.rollback
                        .as_ref()
                        .and_then(|rollback| rollback.credential_generation),
                )
                .max()
                .unwrap_or(0);
        }
        if self
            .credential_generation
            .is_some_and(|generation| generation > self.credential_generation_high_water)
            || self.rollback.as_ref().is_some_and(|rollback| {
                rollback
                    .credential_generation
                    .is_some_and(|generation| generation > self.credential_generation_high_water)
            })
        {
            return Err(DjLinkMachineSettingsError::GenerationHighWaterInconsistent);
        }
        // Trust identity pairing: adapter and network GUIDs are stored
        // together or not at all, in every persisted valid state, disarmed
        // included. Half-bound settings can never be persisted or loaded.
        match (self.adapter_guid.as_deref(), self.network_guid.as_deref()) {
            (Some(adapter), Some(network)) => {
                if !is_canonical_nonnil_uuid(adapter) {
                    return Err(DjLinkMachineSettingsError::InvalidAdapterGuid);
                }
                if !is_canonical_nonnil_uuid(network) {
                    return Err(DjLinkMachineSettingsError::InvalidNetworkGuid);
                }
            }
            (None, None) => {}
            (Some(_), None) | (None, Some(_)) => {
                return Err(DjLinkMachineSettingsError::GuidPairIncomplete)
            }
        }
        if self.auto_start_armed && self.adapter_guid.is_none() {
            return Err(DjLinkMachineSettingsError::ArmedWithoutGuidPair);
        }
        if self.disarm_cleanup_pending
            && (self.auto_start_armed
                || self.transaction_state != DjLinkMachineTransactionState::Idle
                || self.rollback.is_some())
        {
            return Err(DjLinkMachineSettingsError::DisarmCleanupInconsistent);
        }
        if let Some(ip) = self.bind_ip.as_deref() {
            validate_canonical_ip(ip)?;
        }
        if self.bind_port == Some(0) {
            return Err(DjLinkMachineSettingsError::InvalidBindPort);
        }
        match self.transaction_state {
            DjLinkMachineTransactionState::Idle => {
                if self.rollback.is_some() {
                    return Err(DjLinkMachineSettingsError::TransactionRollbackInconsistent);
                }
            }
            DjLinkMachineTransactionState::PrepareCommit
            | DjLinkMachineTransactionState::Committing => {
                if self.credential_generation.is_none() {
                    return Err(DjLinkMachineSettingsError::TransactionStateInconsistent);
                }
                let rollback = self
                    .rollback
                    .as_ref()
                    .ok_or(DjLinkMachineSettingsError::TransactionRollbackInconsistent)?;
                rollback.validated()?;
            }
        }
        Ok(self)
    }

    fn clone_with_state(
        &self,
        revision: u64,
        state: DjLinkMachineTransactionState,
    ) -> Result<Self, DjLinkMachineSettingsError> {
        let mut next = self.clone();
        next.revision = revision;
        next.transaction_state = state;
        next.validated()
    }

    fn check_cas_and_edge(
        &self,
        expected_revision: u64,
        legal_from: DjLinkMachineTransactionState,
        edge_to: &'static str,
    ) -> Result<u64, DjLinkMachineSettingsError> {
        if self.revision != expected_revision {
            return Err(DjLinkMachineSettingsError::CasMismatch {
                expected: expected_revision,
                found: self.revision,
            });
        }
        if self.transaction_state != legal_from {
            return Err(DjLinkMachineSettingsError::InvalidTransition {
                from: transaction_state_label(legal_from),
                to: edge_to,
            });
        }
        next_revision(self.revision)
    }

    /// Journal phase 1: `Idle -> PrepareCommit`, staging the credential
    /// generation the caller intends to materialize in the credential store.
    #[cfg(test)]
    pub fn begin_prepare_commit(
        &self,
        expected_revision: u64,
        staged_credential_generation: u64,
    ) -> Result<Self, DjLinkMachineSettingsError> {
        let rollback = DjLinkMachineRollback::from_stable(self)?;
        self.begin_prepare_commit_with_rollback(
            expected_revision,
            staged_credential_generation,
            rollback,
        )
    }

    /// Starts a journal whose staged settings differ from the accepted stable
    /// settings. The caller must provide that older, validated preimage.
    pub fn begin_prepare_commit_with_rollback(
        &self,
        expected_revision: u64,
        staged_credential_generation: u64,
        rollback: DjLinkMachineRollback,
    ) -> Result<Self, DjLinkMachineSettingsError> {
        let revision = self.check_cas_and_edge(
            expected_revision,
            DjLinkMachineTransactionState::Idle,
            "prepare_commit",
        )?;
        let mut next = self.clone();
        next.revision = revision;
        next.transaction_state = DjLinkMachineTransactionState::PrepareCommit;
        next.credential_generation = Some(staged_credential_generation);
        next.credential_generation_high_water = next
            .credential_generation_high_water
            .max(staged_credential_generation);
        next.rollback = Some(rollback);
        next.validated()
    }

    /// Journal phase 2: `PrepareCommit -> Committing` after the credential
    /// store mutation actually completed.
    pub fn stage_committing(
        &self,
        expected_revision: u64,
    ) -> Result<Self, DjLinkMachineSettingsError> {
        let revision = self.check_cas_and_edge(
            expected_revision,
            DjLinkMachineTransactionState::PrepareCommit,
            "committing",
        )?;
        self.clone_with_state(revision, DjLinkMachineTransactionState::Committing)
    }

    /// Journal phase 3: `Committing -> Idle`, making the values authoritative.
    pub fn finish_commit(
        &self,
        expected_revision: u64,
    ) -> Result<Self, DjLinkMachineSettingsError> {
        let revision = self.check_cas_and_edge(
            expected_revision,
            DjLinkMachineTransactionState::Committing,
            "idle",
        )?;
        let mut next = self.clone();
        next.revision = revision;
        next.transaction_state = DjLinkMachineTransactionState::Idle;
        next.rollback = None;
        next.validated()
    }

    /// Restores the prior non-secret settings from an active journal. The
    /// caller must independently restore/verify its associated rollback
    /// credential before persisting this state.
    pub fn restore_rollback(
        &self,
        expected_revision: u64,
    ) -> Result<Self, DjLinkMachineSettingsError> {
        let revision = self.check_active_transaction_cas(expected_revision)?;
        let rollback = self
            .rollback
            .clone()
            .ok_or(DjLinkMachineSettingsError::TransactionRollbackInconsistent)?;
        let mut restored = rollback.into_idle_settings(revision);
        restored.version = self.version;
        restored.credential_generation_high_water = self.credential_generation_high_water;
        restored.validated()
    }

    #[cfg(test)]
    pub fn abort_transaction(
        &self,
        expected_revision: u64,
    ) -> Result<Self, DjLinkMachineSettingsError> {
        self.restore_rollback(expected_revision)
    }

    fn check_active_transaction_cas(
        &self,
        expected_revision: u64,
    ) -> Result<u64, DjLinkMachineSettingsError> {
        if self.revision != expected_revision {
            return Err(DjLinkMachineSettingsError::CasMismatch {
                expected: expected_revision,
                found: self.revision,
            });
        }
        match self.transaction_state {
            DjLinkMachineTransactionState::PrepareCommit
            | DjLinkMachineTransactionState::Committing => {}
            DjLinkMachineTransactionState::Idle => {
                return Err(DjLinkMachineSettingsError::InvalidTransition {
                    from: "prepare_commit",
                    to: "idle",
                })
            }
        }
        next_revision(self.revision)
    }
}

impl DjLinkMachineRollback {
    pub fn from_stable(
        settings: &DjLinkMachineSettingsV2,
    ) -> Result<Self, DjLinkMachineSettingsError> {
        let stable = settings.clone().validated()?;
        if stable.transaction_state != DjLinkMachineTransactionState::Idle {
            return Err(DjLinkMachineSettingsError::InvalidTransition {
                from: transaction_state_label(stable.transaction_state),
                to: "rollback_snapshot",
            });
        }
        Ok(Self {
            credential_generation: stable.credential_generation,
            adapter_guid: stable.adapter_guid,
            network_guid: stable.network_guid,
            bind_ip: stable.bind_ip,
            bind_port: stable.bind_port,
            auto_start_armed: stable.auto_start_armed,
        })
    }

    fn validated(&self) -> Result<(), DjLinkMachineSettingsError> {
        self.clone().into_idle_settings(1).validated().map(|_| ())
    }

    fn into_idle_settings(self, revision: u64) -> DjLinkMachineSettingsV2 {
        DjLinkMachineSettingsV2 {
            version: DJ_LINK_MACHINE_SETTINGS_VERSION,
            revision,
            transaction_state: DjLinkMachineTransactionState::Idle,
            credential_generation: self.credential_generation,
            credential_generation_high_water: 0,
            adapter_guid: self.adapter_guid,
            network_guid: self.network_guid,
            bind_ip: self.bind_ip,
            bind_port: self.bind_port,
            auto_start_armed: self.auto_start_armed,
            disarm_cleanup_pending: false,
            rollback: None,
        }
    }
}

fn transaction_state_label(state: DjLinkMachineTransactionState) -> &'static str {
    match state {
        DjLinkMachineTransactionState::Idle => "idle",
        DjLinkMachineTransactionState::PrepareCommit => "prepare_commit",
        DjLinkMachineTransactionState::Committing => "committing",
    }
}

fn next_revision(revision: u64) -> Result<u64, DjLinkMachineSettingsError> {
    revision
        .checked_add(1)
        .filter(|next| *next != u64::MAX)
        .ok_or(DjLinkMachineSettingsError::CasExhausted)
}

fn validate_canonical_ip(value: &str) -> Result<(), DjLinkMachineSettingsError> {
    if value.is_empty()
        || value.len() > 45
        || value.trim() != value
        || value.chars().any(|c| c.is_control())
    {
        return Err(DjLinkMachineSettingsError::InvalidBindIp);
    }
    let parsed: IpAddr = value
        .parse()
        .map_err(|_| DjLinkMachineSettingsError::InvalidBindIp)?;
    if parsed.to_string() != value {
        return Err(DjLinkMachineSettingsError::InvalidBindIp);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Bounded, atomic settings I/O (timeline_cue_audio pattern)
// ---------------------------------------------------------------------------

pub fn dj_link_machine_settings_path(local_data_dir: &Path) -> PathBuf {
    local_data_dir.join(DJ_LINK_MACHINE_SETTINGS_FILE)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DjLinkMachineSettingsLoadOutcome {
    Loaded(DjLinkMachineSettingsV2),
    MissingDefaults(DjLinkMachineSettingsV2),
    /// Corrupt, unreadable, or oversized. The file on disk is never modified.
    BlockedCorrupt {
        detail: String,
    },
    /// A newer schema wrote this file. Blocked fail-closed; never modified.
    /// The declared version is carried as `u64` so values above `u32::MAX`
    /// are classified as future-version evidence, not corruption.
    BlockedFutureVersion {
        found_version: u64,
    },
    /// A known retired schema cannot safely recover an active journal.
    BlockedRetiredVersion {
        found_version: u64,
    },
}

pub fn load_dj_link_machine_settings_from_path(path: &Path) -> DjLinkMachineSettingsLoadOutcome {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return DjLinkMachineSettingsLoadOutcome::MissingDefaults(
                DjLinkMachineSettingsV2::default(),
            )
        }
        Err(error) => {
            return DjLinkMachineSettingsLoadOutcome::BlockedCorrupt {
                detail: format!("unable to read {}: {error}", path.display()),
            }
        }
    };
    let declared_len = match file.metadata() {
        Ok(metadata) => metadata.len(),
        Err(error) => {
            return DjLinkMachineSettingsLoadOutcome::BlockedCorrupt {
                detail: format!("unable to inspect {}: {error}", path.display()),
            }
        }
    };
    if declared_len > MAX_DJ_LINK_MACHINE_SETTINGS_BYTES {
        return DjLinkMachineSettingsLoadOutcome::BlockedCorrupt {
            detail: format!(
                "{} exceeds the {} byte safety limit",
                path.display(),
                MAX_DJ_LINK_MACHINE_SETTINGS_BYTES
            ),
        };
    }
    let read_limit = MAX_DJ_LINK_MACHINE_SETTINGS_BYTES.saturating_add(1);
    let mut bytes = Vec::with_capacity(usize::try_from(declared_len).unwrap_or(0));
    if let Err(error) = file.take(read_limit).read_to_end(&mut bytes) {
        return DjLinkMachineSettingsLoadOutcome::BlockedCorrupt {
            detail: format!("unable to read {}: {error}", path.display()),
        };
    }
    if bytes.len() as u64 > MAX_DJ_LINK_MACHINE_SETTINGS_BYTES {
        return DjLinkMachineSettingsLoadOutcome::BlockedCorrupt {
            detail: format!(
                "{} grew beyond the {} byte safety limit",
                path.display(),
                MAX_DJ_LINK_MACHINE_SETTINGS_BYTES
            ),
        };
    }
    match serde_json::from_slice::<DjLinkMachineSettingsV2>(&bytes)
        .map_err(|error| error.to_string())
        .and_then(|settings| settings.validated().map_err(|error| error.to_string()))
    {
        Ok(settings) => DjLinkMachineSettingsLoadOutcome::Loaded(settings),
        Err(detail) => classify_blocked_load(&bytes, detail),
    }
}

fn classify_blocked_load(bytes: &[u8], detail: String) -> DjLinkMachineSettingsLoadOutcome {
    let found_version = serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|value| value.get("version").and_then(serde_json::Value::as_u64))
        .filter(|version| *version != u64::from(DJ_LINK_MACHINE_SETTINGS_VERSION));
    match found_version {
        Some(1) => DjLinkMachineSettingsLoadOutcome::BlockedRetiredVersion { found_version: 1 },
        Some(version) => DjLinkMachineSettingsLoadOutcome::BlockedFutureVersion {
            found_version: version,
        },
        None => DjLinkMachineSettingsLoadOutcome::BlockedCorrupt { detail },
    }
}

pub fn persist_dj_link_machine_settings_to_path(
    path: &Path,
    settings: &DjLinkMachineSettingsV2,
) -> Result<(), String> {
    persist_dj_link_machine_settings_to_path_with(path, settings, |temporary, target| {
        super::replace_file_atomically(temporary, target)
    })
}

pub fn persist_dj_link_machine_settings_to_path_with(
    path: &Path,
    settings: &DjLinkMachineSettingsV2,
    replace: impl FnOnce(&Path, &Path) -> Result<(), String>,
) -> Result<(), String> {
    let settings = settings
        .clone()
        .validated()
        .map_err(|error| format!("Unable to persist DJ-Link machine settings: {error}"))?;
    let bytes = serde_json::to_vec(&settings)
        .map_err(|error| format!("Unable to encode DJ-Link machine settings: {error}"))?;
    if bytes.len() as u64 > MAX_DJ_LINK_MACHINE_SETTINGS_BYTES {
        return Err(format!(
            "DJ-Link machine settings exceed the {} byte safety limit",
            MAX_DJ_LINK_MACHINE_SETTINGS_BYTES
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        format!(
            "DJ-Link machine settings path has no parent: {}",
            path.display()
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Unable to create DJ-Link machine settings directory {}: {error}",
            parent.display()
        )
    })?;
    let file_name = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or(DJ_LINK_MACHINE_SETTINGS_FILE);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!(
        ".{file_name}.{}.{}.{}.tmp",
        std::process::id(),
        nonce,
        SETTINGS_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| {
                format!(
                    "Unable to create temporary DJ-Link machine settings {}: {error}",
                    temporary.display()
                )
            })?;
        file.write_all(&bytes).map_err(|error| {
            format!(
                "Unable to write temporary DJ-Link machine settings {}: {error}",
                temporary.display()
            )
        })?;
        file.sync_all().map_err(|error| {
            format!(
                "Unable to flush temporary DJ-Link machine settings {}: {error}",
                temporary.display()
            )
        })?;
        drop(file);
        replace(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

// ---------------------------------------------------------------------------
// Deterministic tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
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
        assert_eq!(
            PlatformDjLinkCredentialStore::new(),
            Err(DjLinkCredentialError::PlatformUnsupported)
        );
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
        let mut value = serde_json::to_value(DjLinkMachineSettingsV2::default())
            .expect("serialize V2 settings");
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
        assert!(persist_dj_link_machine_settings_to_path_with(
            &path,
            &future,
            |_t, _target| Ok(())
        )
        .is_err());
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
}
