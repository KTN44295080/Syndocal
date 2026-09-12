//! Backend-owned principal, credential, grant, and consent service.
//!
//! The protocol crate owns the pure decisions.  This module owns the local
//! lifecycle around those decisions: pairing challenges, OS-protected
//! credentials, and the bounded administrative surface that a future desktop
//! UI and sidecar adapter will call.  It deliberately does not accept a
//! network address, PID, process name, or descriptor token as identity.

use protocol::agent_authority::{
    AgentAuthority, AgentAuthorityError, AgentAuthorization, AgentGrant, AgentPrincipalId,
    AgentRequestContext,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Mutex,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

const MAX_CHALLENGES: usize = 32;
const PAIRING_TTL: Duration = Duration::from_secs(60);
const CREDENTIAL_BYTES: usize = 32;
const CREDENTIAL_TARGET_PREFIX: &str = "Syndocal/AgentAuthority/v1/";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingChallenge {
    pub challenge_id: String,
    pub principal_id: String,
    pub challenge: String,
    pub expires_in_ms: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingApproval {
    pub principal_id: String,
    pub principal_incarnation: u64,
    pub credential: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrincipalSummary {
    pub principal_id: String,
    pub principal_incarnation: u64,
    pub mode: &'static str,
    pub revoked: bool,
    pub grants: Vec<AgentGrant>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthorityStatus {
    pub kill_switch_active: bool,
    pub principals: Vec<PrincipalSummary>,
}

#[derive(Debug, Clone)]
struct PrincipalRecord {
    principal: AgentPrincipalId,
    incarnation: u64,
    mode: PrincipalMode,
    revoked: bool,
    grants: BTreeSet<AgentGrant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrincipalMode {
    Safe,
    Promoted,
}

#[derive(Debug, Clone)]
struct PendingPairing {
    principal: AgentPrincipalId,
    challenge: String,
    expires_at: Instant,
}

#[derive(Debug)]
struct Inner {
    authority: AgentAuthority,
    principals: BTreeMap<AgentPrincipalId, PrincipalRecord>,
    pending: BTreeMap<String, PendingPairing>,
    kill_switch_active: bool,
}

/// The OS credential store is intentionally hidden behind this small type so
/// no secret-bearing representation can leak into descriptors, logs, or
/// project state. Tests use an in-memory store and never touch the real user
/// credential vault.
#[derive(Debug, Default)]
struct CredentialStore {
    #[cfg(test)]
    memory: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl CredentialStore {
    fn target(principal: &AgentPrincipalId) -> String {
        let digest = Sha256::digest(principal.as_str().as_bytes());
        format!("{CREDENTIAL_TARGET_PREFIX}{digest:x}",)
    }

    fn put(&self, principal: &AgentPrincipalId, credential: &[u8]) -> Result<(), String> {
        if credential.len() != CREDENTIAL_BYTES {
            return Err("agent_credential_size_invalid".to_string());
        }
        #[cfg(test)]
        {
            self.memory
                .lock()
                .map_err(|_| "agent_credential_store_poisoned".to_string())?
                .insert(Self::target(principal), credential.to_vec());
            return Ok(());
        }
        #[cfg(all(windows, not(test)))]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows::Win32::{
                Foundation::FILETIME,
                Security::Credentials::{
                    CredWriteW, CREDENTIALW, CRED_FLAGS, CRED_PERSIST_LOCAL_MACHINE,
                    CRED_TYPE_GENERIC,
                },
            };
            let target = Self::target(principal);
            let target: Vec<u16> = std::ffi::OsStr::new(&target)
                .encode_wide()
                .chain(Some(0))
                .collect();
            let credential = CREDENTIALW {
                Flags: CRED_FLAGS(0),
                Type: CRED_TYPE_GENERIC,
                TargetName: windows::core::PWSTR(target.as_ptr() as *mut u16),
                Comment: windows::core::PWSTR::null(),
                LastWritten: FILETIME::default(),
                CredentialBlobSize: credential.len() as u32,
                CredentialBlob: credential.as_ptr() as *mut u8,
                Persist: CRED_PERSIST_LOCAL_MACHINE,
                AttributeCount: 0,
                Attributes: std::ptr::null_mut(),
                TargetAlias: windows::core::PWSTR::null(),
                UserName: windows::core::PWSTR::null(),
            };
            unsafe { CredWriteW(&credential, 0) }
                .map_err(|_| "agent_credential_store_write_failed".to_string())?;
            return Ok(());
        }
        #[cfg(all(not(test), not(windows)))]
        {
            let _ = principal;
            Err("agent_credential_store_unavailable".to_string())
        }
    }

    fn get(&self, principal: &AgentPrincipalId) -> Result<Option<Zeroizing<Vec<u8>>>, String> {
        #[cfg(test)]
        {
            return Ok(self
                .memory
                .lock()
                .map_err(|_| "agent_credential_store_poisoned".to_string())?
                .get(&Self::target(principal))
                .cloned()
                .map(Zeroizing::new));
        }
        #[cfg(all(windows, not(test)))]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows::{
                core::PCWSTR,
                Win32::Security::Credentials::{
                    CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
                },
            };
            let target = Self::target(principal);
            let target: Vec<u16> = std::ffi::OsStr::new(&target)
                .encode_wide()
                .chain(Some(0))
                .collect();
            let mut raw = std::ptr::null_mut::<CREDENTIALW>();
            let result =
                unsafe { CredReadW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None, &mut raw) };
            if let Err(error) = result {
                if error.code().0 as u32 == 1168 {
                    return Ok(None);
                }
                return Err("agent_credential_store_read_failed".to_string());
            }
            if raw.is_null() {
                return Err("agent_credential_store_empty".to_string());
            }
            let value = unsafe {
                let credential = &*raw;
                if credential.CredentialBlob.is_null()
                    || credential.CredentialBlobSize as usize != CREDENTIAL_BYTES
                {
                    CredFree(raw.cast());
                    return Err("agent_credential_store_invalid".to_string());
                }
                std::slice::from_raw_parts(
                    credential.CredentialBlob,
                    credential.CredentialBlobSize as usize,
                )
                .to_vec()
            };
            unsafe { CredFree(raw.cast()) };
            return Ok(Some(Zeroizing::new(value)));
        }
        #[cfg(all(not(test), not(windows)))]
        {
            let _ = principal;
            Ok(None)
        }
    }

    fn remove(&self, principal: &AgentPrincipalId) -> Result<(), String> {
        #[cfg(test)]
        {
            self.memory
                .lock()
                .map_err(|_| "agent_credential_store_poisoned".to_string())?
                .remove(&Self::target(principal));
            return Ok(());
        }
        #[cfg(all(windows, not(test)))]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows::{
                core::PCWSTR,
                Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC},
            };
            let target = Self::target(principal);
            let target: Vec<u16> = std::ffi::OsStr::new(&target)
                .encode_wide()
                .chain(Some(0))
                .collect();
            let result = unsafe { CredDeleteW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None) };
            if let Err(error) = result {
                // ERROR_NOT_FOUND is an idempotent delete; every other error
                // is retained because revocation must not become ambiguous.
                if error.code().0 as u32 != 1168 {
                    return Err("agent_credential_store_delete_failed".to_string());
                }
            }
            return Ok(());
        }
        #[cfg(all(not(test), not(windows)))]
        {
            let _ = principal;
            Err("agent_credential_store_unavailable".to_string())
        }
    }
}

pub(crate) struct AgentAuthorityService {
    inner: Mutex<Inner>,
    credentials: CredentialStore,
}

impl std::fmt::Debug for AgentAuthorityService {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentAuthorityService")
            .field("inner", &self.inner)
            .field("credentials", &"REDACTED")
            .finish()
    }
}

impl Default for AgentAuthorityService {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentAuthorityService {
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                authority: AgentAuthority::default(),
                principals: BTreeMap::new(),
                pending: BTreeMap::new(),
                kill_switch_active: false,
            }),
            credentials: CredentialStore::default(),
        }
    }

    pub(crate) fn begin_pairing(&self, principal_id: &str) -> Result<PairingChallenge, String> {
        let principal = AgentPrincipalId::new(principal_id.to_string())
            .map_err(|error| authority_error(error).to_string())?;
        let mut inner = self.lock()?;
        if inner.kill_switch_active {
            return Err(authority_error(AgentAuthorityError::KillSwitchActive).to_string());
        }
        if inner
            .principals
            .get(&principal)
            .is_some_and(|record| !record.revoked)
        {
            return Err(authority_error(AgentAuthorityError::AlreadyPaired).to_string());
        }
        if inner.pending.len() >= MAX_CHALLENGES {
            return Err("agent_pairing_capacity".to_string());
        }
        let challenge_id = random_hex(16)?;
        let challenge = random_hex(32)?;
        inner.pending.insert(
            challenge_id.clone(),
            PendingPairing {
                principal: principal.clone(),
                challenge: challenge.clone(),
                expires_at: Instant::now() + PAIRING_TTL,
            },
        );
        Ok(PairingChallenge {
            challenge_id,
            principal_id: principal.as_str().to_string(),
            challenge,
            expires_in_ms: PAIRING_TTL.as_millis() as u64,
        })
    }

    pub(crate) fn approve_pairing(
        &self,
        challenge_id: &str,
        challenge: &str,
    ) -> Result<PairingApproval, String> {
        let mut inner = self.lock()?;
        let pending = inner
            .pending
            .remove(challenge_id)
            .ok_or_else(|| "agent_pairing_unknown".to_string())?;
        if pending.expires_at <= Instant::now() {
            return Err("agent_pairing_expired".to_string());
        }
        if !constant_time_text_eq(&pending.challenge, challenge) {
            return Err("agent_pairing_challenge_mismatch".to_string());
        }
        if inner.kill_switch_active {
            return Err(authority_error(AgentAuthorityError::KillSwitchActive).to_string());
        }
        if inner
            .principals
            .get(&pending.principal)
            .is_some_and(|record| !record.revoked)
        {
            return Err(authority_error(AgentAuthorityError::AlreadyPaired).to_string());
        }
        let previous_incarnation = inner
            .principals
            .get(&pending.principal)
            .map(|record| record.incarnation)
            .unwrap_or(0);
        let incarnation = previous_incarnation
            .checked_add(1)
            .filter(|value| *value != 0)
            .ok_or_else(|| authority_error(AgentAuthorityError::GenerationOverflow).to_string())?;
        let secret = random_bytes(CREDENTIAL_BYTES)?;
        self.credentials.put(&pending.principal, &secret)?;
        if let Err(error) = inner
            .authority
            .pair_external(pending.principal.clone(), incarnation)
        {
            let _ = self.credentials.remove(&pending.principal);
            return Err(authority_error(error).to_string());
        }
        inner.principals.insert(
            pending.principal.clone(),
            PrincipalRecord {
                principal: pending.principal.clone(),
                incarnation,
                mode: PrincipalMode::Safe,
                revoked: false,
                grants: BTreeSet::new(),
            },
        );
        Ok(PairingApproval {
            principal_id: pending.principal.as_str().to_string(),
            principal_incarnation: incarnation,
            credential: hex_encode(&secret),
        })
    }

    pub(crate) fn authenticate(
        &self,
        principal_id: &str,
        incarnation: u64,
        credential: &str,
    ) -> Result<(), String> {
        let principal = AgentPrincipalId::new(principal_id.to_string())
            .map_err(|error| authority_error(error).to_string())?;
        let inner = self.lock()?;
        if inner.kill_switch_active {
            return Err(authority_error(AgentAuthorityError::KillSwitchActive).to_string());
        }
        let record = inner
            .principals
            .get(&principal)
            .ok_or_else(|| authority_error(AgentAuthorityError::UnknownPrincipal).to_string())?;
        if record.revoked {
            return Err(authority_error(AgentAuthorityError::PrincipalRevoked).to_string());
        }
        if record.incarnation != incarnation {
            return Err(
                authority_error(AgentAuthorityError::PrincipalIncarnationStale).to_string(),
            );
        }
        let expected = self
            .credentials
            .get(&principal)?
            .ok_or_else(|| "agent_credential_missing".to_string())?;
        let supplied = hex_decode(credential)?;
        if !constant_time_bytes_eq(&expected, &supplied) {
            return Err("agent_credential_invalid".to_string());
        }
        Ok(())
    }

    pub(crate) fn promote(
        &self,
        principal_id: &str,
        incarnation: u64,
    ) -> Result<AgentAuthorization, String> {
        let principal = self.principal(principal_id)?;
        let mut inner = self.lock()?;
        let authorization = inner
            .authority
            .promote(&principal, incarnation)
            .map_err(|error| authority_error(error).to_string())?;
        inner
            .principals
            .get_mut(&principal)
            .ok_or_else(|| "agent_principal_state_missing".to_string())?
            .mode = PrincipalMode::Promoted;
        Ok(authorization)
    }

    pub(crate) fn grant(
        &self,
        principal_id: &str,
        incarnation: u64,
        grant: AgentGrant,
    ) -> Result<AgentAuthorization, String> {
        let principal = self.principal(principal_id)?;
        let mut inner = self.lock()?;
        let authorization = inner
            .authority
            .grant(&principal, incarnation, grant.clone())
            .map_err(|error| authority_error(error).to_string())?;
        inner
            .principals
            .get_mut(&principal)
            .ok_or_else(|| "agent_principal_state_missing".to_string())?
            .grants
            .insert(grant);
        Ok(authorization)
    }

    pub(crate) fn revoke(&self, principal_id: &str, incarnation: u64) -> Result<(), String> {
        let principal = self.principal(principal_id)?;
        let mut inner = self.lock()?;
        inner
            .authority
            .revoke(&principal, incarnation)
            .map_err(|error| authority_error(error).to_string())?;
        inner
            .principals
            .get_mut(&principal)
            .ok_or_else(|| "agent_principal_state_missing".to_string())?
            .revoked = true;
        if let Some(record) = inner.principals.get_mut(&principal) {
            record.mode = PrincipalMode::Safe;
            record.grants.clear();
        }
        self.credentials.remove(&principal)
    }

    pub(crate) fn kill_switch(&self) -> Result<(), String> {
        let mut inner = self.lock()?;
        inner
            .authority
            .kill_switch()
            .map_err(|error| authority_error(error).to_string())?;
        inner.kill_switch_active = true;
        for record in inner.principals.values_mut() {
            record.revoked = true;
            record.grants.clear();
        }
        let principals = inner.principals.keys().cloned().collect::<Vec<_>>();
        drop(inner);
        for principal in principals {
            self.credentials.remove(&principal)?;
        }
        Ok(())
    }

    pub(crate) fn clear_kill_switch(&self) -> Result<(), String> {
        let mut inner = self.lock()?;
        inner
            .authority
            .clear_kill_switch()
            .map_err(|error| authority_error(error).to_string())?;
        inner.kill_switch_active = false;
        Ok(())
    }

    pub(crate) fn summaries(&self) -> Result<Vec<PrincipalSummary>, String> {
        let inner = self.lock()?;
        Ok(inner
            .principals
            .values()
            .map(|record| PrincipalSummary {
                principal_id: record.principal.as_str().to_string(),
                principal_incarnation: record.incarnation,
                mode: match record.mode {
                    PrincipalMode::Safe => "safe",
                    PrincipalMode::Promoted => "promoted",
                },
                revoked: record.revoked,
                grants: record.grants.iter().cloned().collect(),
            })
            .collect())
    }

    pub(crate) fn status(&self) -> Result<AuthorityStatus, String> {
        Ok(AuthorityStatus {
            kill_switch_active: self.is_kill_switch_active()?,
            principals: self.summaries()?,
        })
    }

    pub(crate) fn is_kill_switch_active(&self) -> Result<bool, String> {
        Ok(self.lock()?.kill_switch_active)
    }

    pub(crate) fn prepare_consent(
        &self,
        consent_id: String,
        context: AgentRequestContext,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<(), String> {
        self.lock()?
            .authority
            .prepare_consent(consent_id, context, now_ms, ttl_ms)
            .map_err(|error| authority_error(error).to_string())
    }

    pub(crate) fn authorize_with_consent(
        &self,
        consent_id: &str,
        context: &AgentRequestContext,
        now_ms: u64,
    ) -> Result<AgentAuthorization, String> {
        self.lock()?
            .authority
            .authorize_with_consent(consent_id, context, now_ms)
            .map_err(|error| authority_error(error).to_string())
    }

    fn principal(&self, principal_id: &str) -> Result<AgentPrincipalId, String> {
        let principal = AgentPrincipalId::new(principal_id.to_string())
            .map_err(|error| authority_error(error).to_string())?;
        if !self.lock()?.principals.contains_key(&principal) {
            return Err(authority_error(AgentAuthorityError::UnknownPrincipal).to_string());
        }
        Ok(principal)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Inner>, String> {
        self.inner
            .lock()
            .map_err(|_| "agent_authority_state_poisoned".to_string())
    }
}

fn random_bytes(length: usize) -> Result<Zeroizing<Vec<u8>>, String> {
    let mut bytes = Zeroizing::new(vec![0u8; length]);
    getrandom::getrandom(&mut bytes).map_err(|_| "secure_random_failed".to_string())?;
    Ok(bytes)
}

fn random_hex(bytes: usize) -> Result<String, String> {
    let bytes = random_bytes(bytes)?;
    Ok(hex_encode(&bytes))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hex_decode(value: &str) -> Result<Vec<u8>, String> {
    if value.len() != CREDENTIAL_BYTES * 2 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("agent_credential_encoding_invalid".to_string());
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|chunk| {
            let high = (chunk[0] as char)
                .to_digit(16)
                .ok_or_else(|| "agent_credential_encoding_invalid".to_string())?;
            let low = (chunk[1] as char)
                .to_digit(16)
                .ok_or_else(|| "agent_credential_encoding_invalid".to_string())?;
            Ok(((high << 4) | low) as u8)
        })
        .collect()
}

fn constant_time_text_eq(expected: &str, supplied: &str) -> bool {
    constant_time_bytes_eq(expected.as_bytes(), supplied.as_bytes())
}

fn constant_time_bytes_eq(expected: &[u8], supplied: &[u8]) -> bool {
    let mismatch = expected
        .iter()
        .zip(supplied.iter())
        .fold(0u8, |value, (left, right)| value | (left ^ right));
    expected.len() == supplied.len() && mismatch == 0
}

fn authority_error(error: AgentAuthorityError) -> &'static str {
    match error {
        AgentAuthorityError::InvalidPrincipal => "agent_invalid_principal",
        AgentAuthorityError::InvalidOperation => "agent_invalid_operation",
        AgentAuthorityError::InvalidProject => "agent_invalid_project",
        AgentAuthorityError::InvalidIncarnation => "agent_invalid_incarnation",
        AgentAuthorityError::InvalidOwnerIncarnation => "agent_invalid_owner_incarnation",
        AgentAuthorityError::InvalidConsentId => "agent_invalid_consent_id",
        AgentAuthorityError::InvalidConsentTtl => "agent_invalid_consent_ttl",
        AgentAuthorityError::RiskCapabilityMismatch => "agent_risk_capability_mismatch",
        AgentAuthorityError::AlreadyPaired => "agent_already_paired",
        AgentAuthorityError::UnknownPrincipal => "agent_unknown_principal",
        AgentAuthorityError::PrincipalIncarnationStale => "agent_principal_incarnation_stale",
        AgentAuthorityError::PrincipalRevoked => "agent_principal_revoked",
        AgentAuthorityError::KillSwitchActive => "agent_kill_switch_active",
        AgentAuthorityError::SafeModeDenied => "agent_safe_mode_denied",
        AgentAuthorityError::MissingGrant => "agent_missing_grant",
        AgentAuthorityError::GrantCapacity => "agent_grant_capacity",
        AgentAuthorityError::ConsentCapacity => "agent_consent_capacity",
        AgentAuthorityError::ConsentRequired => "agent_consent_required",
        AgentAuthorityError::ConsentUnknown => "agent_consent_unknown",
        AgentAuthorityError::ConsentExpired => "agent_consent_expired",
        AgentAuthorityError::ConsentReplayed => "agent_consent_replayed",
        AgentAuthorityError::ConsentPrincipalMismatch => "agent_consent_principal_mismatch",
        AgentAuthorityError::ConsentOwnerMismatch => "agent_consent_owner_mismatch",
        AgentAuthorityError::ConsentOperationMismatch => "agent_consent_operation_mismatch",
        AgentAuthorityError::ConsentArgumentsMismatch => "agent_consent_arguments_mismatch",
        AgentAuthorityError::ConsentProjectMismatch => "agent_consent_project_mismatch",
        AgentAuthorityError::ConsentGenerationMismatch => "agent_consent_generation_mismatch",
        AgentAuthorityError::AuthorityGenerationStale => "agent_authority_generation_stale",
        AgentAuthorityError::GenerationOverflow => "agent_generation_overflow",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::agent_authority::{AgentCapability, AgentGrant};
    use protocol::{control_plane::OperationRisk, control_plane_registry_v2::AdapterKind};

    fn service() -> AgentAuthorityService {
        AgentAuthorityService::new()
    }

    #[test]
    fn pairing_returns_one_credential_and_safe_mode_is_visible() {
        let service = service();
        let challenge = service.begin_pairing("client-a").unwrap();
        let approval = service
            .approve_pairing(&challenge.challenge_id, &challenge.challenge)
            .unwrap();
        assert_eq!(approval.principal_id, "client-a");
        assert_eq!(approval.credential.len(), CREDENTIAL_BYTES * 2);
        assert_eq!(service.summaries().unwrap()[0].mode, "safe");
        assert!(service
            .authenticate(
                "client-a",
                approval.principal_incarnation,
                &approval.credential
            )
            .is_ok());
        assert!(service
            .authenticate("client-a", approval.principal_incarnation, &"0".repeat(64))
            .is_err());
    }

    #[test]
    fn wrong_or_replayed_pairing_challenge_fails_closed() {
        let service = service();
        let challenge = service.begin_pairing("client-a").unwrap();
        assert_eq!(
            service.approve_pairing(&challenge.challenge_id, "wrong"),
            Err("agent_pairing_challenge_mismatch".to_string())
        );
        assert_eq!(
            service.approve_pairing(&challenge.challenge_id, &challenge.challenge),
            Err("agent_pairing_unknown".to_string())
        );
    }

    #[test]
    fn explicit_grants_do_not_escape_safe_mode() {
        let service = service();
        let challenge = service.begin_pairing("client-a").unwrap();
        let approval = service
            .approve_pairing(&challenge.challenge_id, &challenge.challenge)
            .unwrap();
        let grant = AgentGrant::new(
            AdapterKind::ExternalMcp,
            AgentCapability::Output,
            "syndocal.output.test.v1",
            None,
        )
        .unwrap();
        assert_eq!(
            service.grant("client-a", approval.principal_incarnation, grant.clone()),
            Err("agent_safe_mode_denied".to_string())
        );
        service
            .promote("client-a", approval.principal_incarnation)
            .unwrap();
        assert!(service
            .grant("client-a", approval.principal_incarnation, grant)
            .is_ok());
    }

    #[test]
    fn revoke_removes_credential_and_kill_switch_closes_pairing() {
        let service = service();
        let challenge = service.begin_pairing("client-a").unwrap();
        let approval = service
            .approve_pairing(&challenge.challenge_id, &challenge.challenge)
            .unwrap();
        service
            .revoke("client-a", approval.principal_incarnation)
            .unwrap();
        assert!(service
            .authenticate(
                "client-a",
                approval.principal_incarnation,
                &approval.credential
            )
            .is_err());
        service.kill_switch().unwrap();
        assert!(service.is_kill_switch_active().unwrap());
        assert!(service.begin_pairing("client-b").is_err());
        service.clear_kill_switch().unwrap();
        assert!(!service.is_kill_switch_active().unwrap());
    }

    #[test]
    fn re_pairing_advances_incarnation_after_revoke() {
        let service = service();
        let first = service.begin_pairing("client-a").unwrap();
        let first = service
            .approve_pairing(&first.challenge_id, &first.challenge)
            .unwrap();
        service
            .revoke("client-a", first.principal_incarnation)
            .unwrap();
        let second = service.begin_pairing("client-a").unwrap();
        let second = service
            .approve_pairing(&second.challenge_id, &second.challenge)
            .unwrap();
        assert!(second.principal_incarnation > first.principal_incarnation);
        assert!(service
            .authenticate("client-a", first.principal_incarnation, &first.credential)
            .is_err());
    }

    #[test]
    fn consent_delegates_exact_binding_to_protocol_authority() {
        let service = service();
        let challenge = service.begin_pairing("client-a").unwrap();
        let approval = service
            .approve_pairing(&challenge.challenge_id, &challenge.challenge)
            .unwrap();
        service
            .promote("client-a", approval.principal_incarnation)
            .unwrap();
        let grant = AgentGrant::new(
            AdapterKind::ExternalMcp,
            AgentCapability::Output,
            "syndocal.output.test.v1",
            Some("project-a".to_string()),
        )
        .unwrap();
        service
            .grant("client-a", approval.principal_incarnation, grant)
            .unwrap();
        let context = AgentRequestContext {
            principal: AgentPrincipalId::new("client-a").unwrap(),
            principal_incarnation: approval.principal_incarnation,
            adapter: AdapterKind::ExternalMcp,
            operation_id: "syndocal.output.test.v1".to_string(),
            capability: AgentCapability::Output,
            risk: OperationRisk::R4,
            owner_incarnation: 1,
            canonical_arguments_fingerprint: [7; 32],
            project_id: Some("project-a".to_string()),
            project_generation: 2,
            output_generation: 3,
        };
        service
            .prepare_consent("consent-a".to_string(), context.clone(), 100, 1_000)
            .unwrap();
        assert!(service
            .authorize_with_consent("consent-a", &context, 500)
            .is_ok());
        assert!(service
            .authorize_with_consent("consent-a", &context, 500)
            .is_err());
    }
}
