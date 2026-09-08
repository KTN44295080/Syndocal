//! Pure principal, grant, and prepared-consent authority decisions.
//!
//! This module is deliberately transport- and UI-neutral.  It does not mint
//! credentials, persist secrets, or claim that a caller is the desktop UI.
//! The trusted adapter supplies a freshly issued principal incarnation and a
//! locally approved consent id; this core only records the bounded state and
//! rejects stale, revoked, or incorrectly bound decisions.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::control_plane::OperationRisk;
use crate::control_plane_registry_v2::AdapterKind;

const MAX_PRINCIPAL_BYTES: usize = 128;
const MAX_OPERATION_BYTES: usize = 128;
const MAX_PROJECT_BYTES: usize = 256;
const MAX_CONSENT_ID_BYTES: usize = 128;
const MAX_CONSENTS: usize = 256;
const MAX_GRANTS_PER_PRINCIPAL: usize = 128;
pub const MAX_PREPARED_CONSENT_TTL_MS: u64 = 15_000;

fn bounded_ascii(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_bytes
        && value
            .bytes()
            .all(|byte| byte.is_ascii() && !byte.is_ascii_whitespace())
}

/// Stable identity of an external control-plane principal.  This is not a
/// credential and must not be used as one on a wire.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentPrincipalId(String);

impl AgentPrincipalId {
    pub fn new(value: impl Into<String>) -> Result<Self, AgentAuthorityError> {
        let value = value.into();
        if !bounded_ascii(&value, MAX_PRINCIPAL_BYTES) {
            return Err(AgentAuthorityError::InvalidPrincipal);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The narrow capability vocabulary understood by the authority core.
/// Capability names are intentionally separate from transport credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCapability {
    Read,
    Runtime,
    Live,
    Authored,
    Output,
    File,
    SafetyBlackoutEngage,
}

impl AgentCapability {
    fn permitted_in_safe_mode(self) -> bool {
        matches!(self, Self::Read | Self::Runtime)
    }

    fn matches_risk(self, risk: OperationRisk) -> bool {
        matches!(
            (self, risk),
            (Self::Read, OperationRisk::R0)
                | (Self::Runtime, OperationRisk::R1)
                | (Self::Live, OperationRisk::R2)
                | (Self::Authored, OperationRisk::R3)
                | (Self::Output, OperationRisk::R4)
                | (Self::File, OperationRisk::R5)
                | (Self::SafetyBlackoutEngage, OperationRisk::S0)
        )
    }
}

/// A grant is exact-operation scoped in this first core.  A future operation
/// family grant must be introduced as an explicit, separately reviewed
/// policy; prefix matching is intentionally not implicit here.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentGrant {
    pub adapter: AdapterKind,
    pub capability: AgentCapability,
    pub operation_id: String,
    #[serde(default)]
    pub project_id: Option<String>,
}

impl AgentGrant {
    pub fn new(
        adapter: AdapterKind,
        capability: AgentCapability,
        operation_id: impl Into<String>,
        project_id: Option<String>,
    ) -> Result<Self, AgentAuthorityError> {
        let operation_id = operation_id.into();
        if !bounded_ascii(&operation_id, MAX_OPERATION_BYTES) {
            return Err(AgentAuthorityError::InvalidOperation);
        }
        if project_id
            .as_deref()
            .is_some_and(|value| !bounded_ascii(value, MAX_PROJECT_BYTES))
        {
            return Err(AgentAuthorityError::InvalidProject);
        }
        Ok(Self {
            adapter,
            capability,
            operation_id,
            project_id,
        })
    }
}

/// All facts that a high-risk consent must bind.  The argument fingerprint is
/// supplied by the canonical operation registry; this module never hashes or
/// rewrites an adapter payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRequestContext {
    pub principal: AgentPrincipalId,
    pub principal_incarnation: u64,
    pub adapter: AdapterKind,
    pub operation_id: String,
    pub capability: AgentCapability,
    pub risk: OperationRisk,
    pub owner_incarnation: u64,
    pub canonical_arguments_fingerprint: [u8; 32],
    pub project_id: Option<String>,
    pub project_generation: u64,
    pub output_generation: u64,
}

impl AgentRequestContext {
    pub fn validate(&self) -> Result<(), AgentAuthorityError> {
        if self.principal_incarnation == 0 {
            return Err(AgentAuthorityError::InvalidIncarnation);
        }
        if !bounded_ascii(&self.operation_id, MAX_OPERATION_BYTES) {
            return Err(AgentAuthorityError::InvalidOperation);
        }
        if self
            .project_id
            .as_deref()
            .is_some_and(|value| !bounded_ascii(value, MAX_PROJECT_BYTES))
        {
            return Err(AgentAuthorityError::InvalidProject);
        }
        if !self.capability.matches_risk(self.risk) {
            return Err(AgentAuthorityError::RiskCapabilityMismatch);
        }
        if matches!(self.risk, OperationRisk::R4 | OperationRisk::R5) && self.owner_incarnation == 0
        {
            return Err(AgentAuthorityError::InvalidOwnerIncarnation);
        }
        Ok(())
    }

    fn matches_grant(&self, grant: &AgentGrant) -> bool {
        grant.adapter == self.adapter
            && grant.capability == self.capability
            && grant.operation_id == self.operation_id
            && grant.project_id == self.project_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentAuthorization {
    pub authority_generation: u64,
    pub principal_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrincipalMode {
    Safe,
    Promoted,
}

#[derive(Debug, Clone)]
struct PrincipalState {
    incarnation: u64,
    generation: u64,
    mode: PrincipalMode,
    revoked: bool,
    grants: BTreeSet<AgentGrant>,
}

#[derive(Debug, Clone)]
struct PreparedConsent {
    context: AgentRequestContext,
    authority_generation: u64,
    principal_generation: u64,
    expires_at_ms: u64,
}

/// Bounded authority state.  The service does not own sockets, credentials,
/// UI dialogs, project state, or physical output.  Those remain in their
/// respective adapters and call this core at their final admission boundary.
#[derive(Debug, Default)]
pub struct AgentAuthority {
    authority_generation: u64,
    kill_switch_active: bool,
    principals: BTreeMap<AgentPrincipalId, PrincipalState>,
    consents: BTreeMap<String, PreparedConsent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentAuthorityError {
    InvalidPrincipal,
    InvalidOperation,
    InvalidProject,
    InvalidIncarnation,
    InvalidOwnerIncarnation,
    InvalidConsentId,
    InvalidConsentTtl,
    RiskCapabilityMismatch,
    AlreadyPaired,
    UnknownPrincipal,
    PrincipalIncarnationStale,
    PrincipalRevoked,
    KillSwitchActive,
    SafeModeDenied,
    MissingGrant,
    GrantCapacity,
    ConsentCapacity,
    ConsentRequired,
    ConsentUnknown,
    ConsentExpired,
    ConsentReplayed,
    ConsentPrincipalMismatch,
    ConsentOwnerMismatch,
    ConsentOperationMismatch,
    ConsentArgumentsMismatch,
    ConsentProjectMismatch,
    ConsentGenerationMismatch,
    AuthorityGenerationStale,
    GenerationOverflow,
}

impl AgentAuthority {
    pub fn pair_external(
        &mut self,
        principal: AgentPrincipalId,
        incarnation: u64,
    ) -> Result<(), AgentAuthorityError> {
        if self.kill_switch_active {
            return Err(AgentAuthorityError::KillSwitchActive);
        }
        if incarnation == 0 {
            return Err(AgentAuthorityError::InvalidIncarnation);
        }
        if let Some(existing) = self.principals.get(&principal) {
            if !existing.revoked {
                return Err(AgentAuthorityError::AlreadyPaired);
            }
            if incarnation <= existing.incarnation {
                return Err(AgentAuthorityError::PrincipalIncarnationStale);
            }
        }
        let generation = self
            .principals
            .get(&principal)
            .map(|existing| next_generation(existing.generation))
            .transpose()?
            .unwrap_or(1);
        self.principals.insert(
            principal,
            PrincipalState {
                incarnation,
                generation,
                mode: PrincipalMode::Safe,
                revoked: false,
                grants: BTreeSet::new(),
            },
        );
        Ok(())
    }

    pub fn promote(
        &mut self,
        principal: &AgentPrincipalId,
        incarnation: u64,
    ) -> Result<AgentAuthorization, AgentAuthorityError> {
        let state = self.active_state_mut(principal, incarnation)?;
        let next = next_generation(state.generation)?;
        state.mode = PrincipalMode::Promoted;
        state.generation = next;
        self.current_authorization(principal)
    }

    pub fn grant(
        &mut self,
        principal: &AgentPrincipalId,
        incarnation: u64,
        grant: AgentGrant,
    ) -> Result<AgentAuthorization, AgentAuthorityError> {
        let state = self.active_state_mut(principal, incarnation)?;
        if state.grants.len() >= MAX_GRANTS_PER_PRINCIPAL && !state.grants.contains(&grant) {
            return Err(AgentAuthorityError::GrantCapacity);
        }
        if state.mode == PrincipalMode::Safe && !grant.capability.permitted_in_safe_mode() {
            return Err(AgentAuthorityError::SafeModeDenied);
        }
        let next = next_generation(state.generation)?;
        state.grants.insert(grant);
        state.generation = next;
        self.current_authorization(principal)
    }

    pub fn revoke(
        &mut self,
        principal: &AgentPrincipalId,
        incarnation: u64,
    ) -> Result<(), AgentAuthorityError> {
        let state = self.active_state_mut(principal, incarnation)?;
        let next = next_generation(state.generation)?;
        state.revoked = true;
        state.generation = next;
        self.remove_consents_for(principal);
        Ok(())
    }

    /// Immediately revokes every external principal and invalidates all
    /// outstanding consent.  It does not call or alter any local safety path.
    pub fn kill_switch(&mut self) -> Result<(), AgentAuthorityError> {
        let next_authority_generation = next_generation(self.authority_generation)?;
        let next_principal_generations = self
            .principals
            .iter()
            .map(|(principal, state)| {
                next_generation(state.generation).map(|generation| (principal.clone(), generation))
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.authority_generation = next_authority_generation;
        self.kill_switch_active = true;
        for ((principal, state), (expected_principal, generation)) in
            self.principals.iter_mut().zip(next_principal_generations)
        {
            debug_assert_eq!(principal, &expected_principal);
            state.revoked = true;
            state.grants.clear();
            state.generation = generation;
        }
        self.consents.clear();
        Ok(())
    }

    /// Clearing the switch only reopens the admission gate; it does not
    /// resurrect revoked principals or grants.  Re-pairing is required.
    pub fn clear_kill_switch(&mut self) -> Result<(), AgentAuthorityError> {
        let next = next_generation(self.authority_generation)?;
        self.authority_generation = next;
        self.kill_switch_active = false;
        Ok(())
    }

    pub fn authorize(
        &self,
        context: &AgentRequestContext,
    ) -> Result<AgentAuthorization, AgentAuthorityError> {
        self.check_grant(context)?;
        if matches!(context.risk, OperationRisk::R4 | OperationRisk::R5) {
            return Err(AgentAuthorityError::ConsentRequired);
        }
        self.current_authorization(&context.principal)
    }

    pub fn prepare_consent(
        &mut self,
        consent_id: impl Into<String>,
        context: AgentRequestContext,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<(), AgentAuthorityError> {
        context.validate()?;
        if !matches!(context.risk, OperationRisk::R4 | OperationRisk::R5) {
            return Err(AgentAuthorityError::ConsentRequired);
        }
        self.check_grant(&context)?;
        if ttl_ms == 0 || ttl_ms > MAX_PREPARED_CONSENT_TTL_MS {
            return Err(AgentAuthorityError::InvalidConsentTtl);
        }
        let consent_id = consent_id.into();
        if !bounded_ascii(&consent_id, MAX_CONSENT_ID_BYTES) {
            return Err(AgentAuthorityError::InvalidConsentId);
        }
        if self.consents.len() >= MAX_CONSENTS || self.consents.contains_key(&consent_id) {
            return Err(AgentAuthorityError::ConsentCapacity);
        }
        let expires_at_ms = now_ms
            .checked_add(ttl_ms)
            .ok_or(AgentAuthorityError::GenerationOverflow)?;
        let principal_generation = self
            .principals
            .get(&context.principal)
            .ok_or(AgentAuthorityError::UnknownPrincipal)?
            .generation;
        self.consents.insert(
            consent_id,
            PreparedConsent {
                context,
                authority_generation: self.authority_generation,
                principal_generation,
                expires_at_ms,
            },
        );
        Ok(())
    }

    pub fn authorize_with_consent(
        &mut self,
        consent_id: &str,
        context: &AgentRequestContext,
        now_ms: u64,
    ) -> Result<AgentAuthorization, AgentAuthorityError> {
        self.check_grant(context)?;
        if !matches!(context.risk, OperationRisk::R4 | OperationRisk::R5) {
            return Err(AgentAuthorityError::ConsentRequired);
        }
        let prepared = self
            .consents
            .get(consent_id)
            .ok_or(AgentAuthorityError::ConsentUnknown)?;
        if now_ms >= prepared.expires_at_ms {
            self.consents.remove(consent_id);
            return Err(AgentAuthorityError::ConsentExpired);
        }
        if prepared.authority_generation != self.authority_generation {
            return Err(AgentAuthorityError::AuthorityGenerationStale);
        }
        let current_generation = self
            .principals
            .get(&context.principal)
            .ok_or(AgentAuthorityError::UnknownPrincipal)?
            .generation;
        if prepared.principal_generation != current_generation {
            return Err(AgentAuthorityError::PrincipalIncarnationStale);
        }
        compare_consent_context(&prepared.context, context)?;
        self.consents.remove(consent_id);
        self.current_authorization(&context.principal)
    }

    /// A terminal completion must carry the admission generations it observed.
    /// Revocation, kill-switch, or any later promotion/grant change rejects the
    /// old result before it can be published as current authority.
    pub fn validate_authorization(
        &self,
        principal: &AgentPrincipalId,
        principal_incarnation: u64,
        authorization: AgentAuthorization,
    ) -> Result<(), AgentAuthorityError> {
        if self.kill_switch_active {
            return Err(AgentAuthorityError::KillSwitchActive);
        }
        if authorization.authority_generation != self.authority_generation {
            return Err(AgentAuthorityError::AuthorityGenerationStale);
        }
        let state = self
            .principals
            .get(principal)
            .ok_or(AgentAuthorityError::UnknownPrincipal)?;
        if state.revoked {
            return Err(AgentAuthorityError::PrincipalRevoked);
        }
        if state.incarnation != principal_incarnation {
            return Err(AgentAuthorityError::PrincipalIncarnationStale);
        }
        if state.generation != authorization.principal_generation {
            return Err(AgentAuthorityError::PrincipalIncarnationStale);
        }
        Ok(())
    }

    pub fn is_kill_switch_active(&self) -> bool {
        self.kill_switch_active
    }

    fn active_state_mut(
        &mut self,
        principal: &AgentPrincipalId,
        incarnation: u64,
    ) -> Result<&mut PrincipalState, AgentAuthorityError> {
        if self.kill_switch_active {
            return Err(AgentAuthorityError::KillSwitchActive);
        }
        let state = self
            .principals
            .get_mut(principal)
            .ok_or(AgentAuthorityError::UnknownPrincipal)?;
        if state.revoked {
            return Err(AgentAuthorityError::PrincipalRevoked);
        }
        if state.incarnation != incarnation {
            return Err(AgentAuthorityError::PrincipalIncarnationStale);
        }
        Ok(state)
    }

    fn check_grant(&self, context: &AgentRequestContext) -> Result<(), AgentAuthorityError> {
        context.validate()?;
        if self.kill_switch_active {
            return Err(AgentAuthorityError::KillSwitchActive);
        }
        let state = self
            .principals
            .get(&context.principal)
            .ok_or(AgentAuthorityError::UnknownPrincipal)?;
        if state.revoked {
            return Err(AgentAuthorityError::PrincipalRevoked);
        }
        if state.incarnation != context.principal_incarnation {
            return Err(AgentAuthorityError::PrincipalIncarnationStale);
        }
        if state.mode == PrincipalMode::Safe && !context.capability.permitted_in_safe_mode() {
            return Err(AgentAuthorityError::SafeModeDenied);
        }
        if !state
            .grants
            .iter()
            .any(|grant| context.matches_grant(grant))
        {
            return Err(AgentAuthorityError::MissingGrant);
        }
        Ok(())
    }

    fn current_authorization(
        &self,
        principal: &AgentPrincipalId,
    ) -> Result<AgentAuthorization, AgentAuthorityError> {
        let state = self
            .principals
            .get(principal)
            .ok_or(AgentAuthorityError::UnknownPrincipal)?;
        Ok(AgentAuthorization {
            authority_generation: self.authority_generation,
            principal_generation: state.generation,
        })
    }

    fn remove_consents_for(&mut self, principal: &AgentPrincipalId) {
        self.consents
            .retain(|_, consent| &consent.context.principal != principal);
    }
}

fn compare_consent_context(
    prepared: &AgentRequestContext,
    current: &AgentRequestContext,
) -> Result<(), AgentAuthorityError> {
    if prepared.context_principal() != current.context_principal() {
        return Err(AgentAuthorityError::ConsentPrincipalMismatch);
    }
    if prepared.owner_incarnation != current.owner_incarnation {
        return Err(AgentAuthorityError::ConsentOwnerMismatch);
    }
    if prepared.operation_id != current.operation_id
        || prepared.adapter != current.adapter
        || prepared.capability != current.capability
        || prepared.risk != current.risk
    {
        return Err(AgentAuthorityError::ConsentOperationMismatch);
    }
    if prepared.canonical_arguments_fingerprint != current.canonical_arguments_fingerprint {
        return Err(AgentAuthorityError::ConsentArgumentsMismatch);
    }
    if prepared.project_id != current.project_id {
        return Err(AgentAuthorityError::ConsentProjectMismatch);
    }
    if prepared.project_generation != current.project_generation
        || prepared.output_generation != current.output_generation
    {
        return Err(AgentAuthorityError::ConsentGenerationMismatch);
    }
    Ok(())
}

impl AgentRequestContext {
    fn context_principal(&self) -> (&AgentPrincipalId, u64) {
        (&self.principal, self.principal_incarnation)
    }
}

fn next_generation(value: u64) -> Result<u64, AgentAuthorityError> {
    value
        .checked_add(1)
        .ok_or(AgentAuthorityError::GenerationOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FINGERPRINT: [u8; 32] = [0x11; 32];

    fn principal(value: &str) -> AgentPrincipalId {
        AgentPrincipalId::new(value).unwrap()
    }

    fn grant(capability: AgentCapability, operation_id: &str) -> AgentGrant {
        AgentGrant::new(
            AdapterKind::ExternalMcp,
            capability,
            operation_id,
            Some("project-a".to_string()),
        )
        .unwrap()
    }

    fn context(
        principal: &AgentPrincipalId,
        capability: AgentCapability,
        risk: OperationRisk,
    ) -> AgentRequestContext {
        AgentRequestContext {
            principal: principal.clone(),
            principal_incarnation: 7,
            adapter: AdapterKind::ExternalMcp,
            operation_id: "syndocal.output.blackout.release.v2".to_string(),
            capability,
            risk,
            owner_incarnation: 19,
            canonical_arguments_fingerprint: FINGERPRINT,
            project_id: Some("project-a".to_string()),
            project_generation: 4,
            output_generation: 9,
        }
    }

    #[test]
    fn paired_principal_starts_safe_and_allows_only_explicit_read_runtime_grants() {
        let id = principal("client-a");
        let mut authority = AgentAuthority::default();
        authority.pair_external(id.clone(), 7).unwrap();
        authority
            .grant(
                &id,
                7,
                AgentGrant::new(
                    AdapterKind::ExternalMcp,
                    AgentCapability::Read,
                    "syndocal.query.project.authority.v1",
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        authority
            .grant(
                &id,
                7,
                AgentGrant::new(
                    AdapterKind::ExternalMcp,
                    AgentCapability::Runtime,
                    "syndocal.runtime.timeline.transport.set_playing.v1",
                    None,
                )
                .unwrap(),
            )
            .unwrap();

        let mut read = context(&id, AgentCapability::Read, OperationRisk::R0);
        read.operation_id = "syndocal.query.project.authority.v1".to_string();
        read.project_id = None;
        assert!(authority.authorize(&read).is_ok());

        let mut runtime = context(&id, AgentCapability::Runtime, OperationRisk::R1);
        runtime.operation_id = "syndocal.runtime.timeline.transport.set_playing.v1".to_string();
        runtime.project_id = None;
        assert!(authority.authorize(&runtime).is_ok());

        assert_eq!(
            authority.authorize(&context(&id, AgentCapability::Output, OperationRisk::R4)),
            Err(AgentAuthorityError::SafeModeDenied)
        );
    }

    #[test]
    fn promotion_does_not_grant_operations_and_exact_grant_is_required() {
        let id = principal("client-a");
        let mut authority = AgentAuthority::default();
        authority.pair_external(id.clone(), 7).unwrap();
        authority.promote(&id, 7).unwrap();
        let output = context(&id, AgentCapability::Output, OperationRisk::R4);
        assert_eq!(
            authority.authorize(&output),
            Err(AgentAuthorityError::MissingGrant)
        );
        authority
            .grant(&id, 7, grant(AgentCapability::Output, &output.operation_id))
            .unwrap();
        assert_eq!(
            authority.authorize(&output),
            Err(AgentAuthorityError::ConsentRequired)
        );
    }

    #[test]
    fn consent_binds_operation_owner_arguments_project_and_generations_and_is_single_use() {
        let id = principal("client-a");
        let mut authority = AgentAuthority::default();
        authority.pair_external(id.clone(), 7).unwrap();
        authority.promote(&id, 7).unwrap();
        let original = context(&id, AgentCapability::Output, OperationRisk::R4);
        authority
            .grant(
                &id,
                7,
                grant(AgentCapability::Output, &original.operation_id),
            )
            .unwrap();
        authority
            .prepare_consent("consent-1", original.clone(), 100, 100)
            .unwrap();

        let mut stale = original.clone();
        stale.output_generation += 1;
        assert_eq!(
            authority.authorize_with_consent("consent-1", &stale, 150),
            Err(AgentAuthorityError::ConsentGenerationMismatch)
        );
        assert!(authority
            .authorize_with_consent("consent-1", &original, 150)
            .is_ok());
        assert_eq!(
            authority.authorize_with_consent("consent-1", &original, 150),
            Err(AgentAuthorityError::ConsentUnknown)
        );
    }

    #[test]
    fn wrong_principal_owner_operation_and_arguments_are_rejected_without_consuming_consent() {
        let id = principal("client-a");
        let other = principal("client-b");
        let mut authority = AgentAuthority::default();
        authority.pair_external(id.clone(), 7).unwrap();
        authority.pair_external(other.clone(), 8).unwrap();
        authority.promote(&id, 7).unwrap();
        authority.promote(&other, 8).unwrap();
        let original = context(&id, AgentCapability::Output, OperationRisk::R4);
        authority
            .grant(
                &id,
                7,
                grant(AgentCapability::Output, &original.operation_id),
            )
            .unwrap();
        let mut other_context = original.clone();
        other_context.principal = other;
        other_context.principal_incarnation = 8;
        authority
            .grant(
                &other_context.principal,
                8,
                grant(AgentCapability::Output, &other_context.operation_id),
            )
            .unwrap();
        authority
            .prepare_consent("consent-1", original.clone(), 100, 100)
            .unwrap();

        assert_eq!(
            authority.authorize_with_consent("consent-1", &other_context, 101),
            Err(AgentAuthorityError::ConsentPrincipalMismatch)
        );
        let mut wrong_args = original.clone();
        wrong_args.canonical_arguments_fingerprint[0] ^= 1;
        assert_eq!(
            authority.authorize_with_consent("consent-1", &wrong_args, 101),
            Err(AgentAuthorityError::ConsentArgumentsMismatch)
        );
        let mut wrong_owner = original.clone();
        wrong_owner.owner_incarnation += 1;
        assert_eq!(
            authority.authorize_with_consent("consent-1", &wrong_owner, 101),
            Err(AgentAuthorityError::ConsentOwnerMismatch)
        );
        assert!(authority
            .authorize_with_consent("consent-1", &original, 101)
            .is_ok());
    }

    #[test]
    fn expiry_revoke_and_kill_switch_reject_old_or_pending_results() {
        let id = principal("client-a");
        let mut authority = AgentAuthority::default();
        authority.pair_external(id.clone(), 7).unwrap();
        authority.promote(&id, 7).unwrap();
        let original = context(&id, AgentCapability::Output, OperationRisk::R4);
        authority
            .grant(
                &id,
                7,
                grant(AgentCapability::Output, &original.operation_id),
            )
            .unwrap();
        authority
            .prepare_consent("expired", original.clone(), 100, 1)
            .unwrap();
        assert_eq!(
            authority.authorize_with_consent("expired", &original, 101),
            Err(AgentAuthorityError::ConsentExpired)
        );

        let authorization = authority.authorize(&AgentRequestContext {
            risk: OperationRisk::R0,
            capability: AgentCapability::Read,
            operation_id: "syndocal.query.project.authority.v1".to_string(),
            project_id: None,
            owner_incarnation: 0,
            ..original.clone()
        });
        assert_eq!(authorization, Err(AgentAuthorityError::MissingGrant));

        authority
            .prepare_consent("revoked", original.clone(), 200, 100)
            .unwrap();
        authority.revoke(&id, 7).unwrap();
        assert_eq!(
            authority.authorize_with_consent("revoked", &original, 201),
            Err(AgentAuthorityError::PrincipalRevoked)
        );
        assert_eq!(
            authority.authorize(&original),
            Err(AgentAuthorityError::PrincipalRevoked)
        );

        authority.clear_kill_switch().unwrap();
        authority.pair_external(principal("client-b"), 1).unwrap();
        authority.kill_switch().unwrap();
        assert!(authority.is_kill_switch_active());
        assert_eq!(
            authority.authorize(&original),
            Err(AgentAuthorityError::KillSwitchActive)
        );
    }

    #[test]
    fn old_authorization_is_rejected_after_grant_change_and_wrong_incarnation() {
        let id = principal("client-a");
        let mut authority = AgentAuthority::default();
        authority.pair_external(id.clone(), 7).unwrap();
        let read = AgentRequestContext {
            risk: OperationRisk::R0,
            capability: AgentCapability::Read,
            operation_id: "syndocal.query.project.authority.v1".to_string(),
            project_id: None,
            owner_incarnation: 0,
            ..context(&id, AgentCapability::Read, OperationRisk::R0)
        };
        authority
            .grant(
                &id,
                7,
                AgentGrant::new(
                    AdapterKind::ExternalMcp,
                    AgentCapability::Read,
                    &read.operation_id,
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        let authorization = authority.authorize(&read).unwrap();
        authority
            .grant(
                &id,
                7,
                AgentGrant::new(
                    AdapterKind::ExternalMcp,
                    AgentCapability::Runtime,
                    "syndocal.runtime.timeline.transport.set_playing.v1",
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(
            authority.validate_authorization(&id, 7, authorization),
            Err(AgentAuthorityError::PrincipalIncarnationStale)
        );
        assert_eq!(
            authority.authorize(&AgentRequestContext {
                principal_incarnation: 6,
                ..read
            }),
            Err(AgentAuthorityError::PrincipalIncarnationStale)
        );
    }

    #[test]
    fn re_pairing_advances_principal_generation_and_invalidates_old_authorization() {
        let id = principal("client-a");
        let mut authority = AgentAuthority::default();
        authority.pair_external(id.clone(), 7).unwrap();
        let read = AgentRequestContext {
            risk: OperationRisk::R0,
            capability: AgentCapability::Read,
            operation_id: "syndocal.query.project.authority.v1".to_string(),
            project_id: None,
            owner_incarnation: 0,
            ..context(&id, AgentCapability::Read, OperationRisk::R0)
        };
        authority
            .grant(
                &id,
                7,
                AgentGrant::new(
                    AdapterKind::ExternalMcp,
                    AgentCapability::Read,
                    &read.operation_id,
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        let old_authorization = authority.authorize(&read).unwrap();
        authority.revoke(&id, 7).unwrap();
        authority.pair_external(id.clone(), 8).unwrap();
        assert_eq!(
            authority.validate_authorization(&id, 8, old_authorization),
            Err(AgentAuthorityError::PrincipalIncarnationStale)
        );
        assert_eq!(
            authority.authorize(&AgentRequestContext {
                principal_incarnation: 8,
                ..read
            }),
            Err(AgentAuthorityError::MissingGrant)
        );
    }

    #[test]
    fn pairing_is_closed_while_kill_switch_is_active_until_repair_after_clear() {
        let id = principal("client-a");
        let other = principal("client-b");
        let mut authority = AgentAuthority::default();
        authority.pair_external(id, 7).unwrap();
        authority.kill_switch().unwrap();
        assert_eq!(
            authority.pair_external(other.clone(), 1),
            Err(AgentAuthorityError::KillSwitchActive)
        );
        authority.clear_kill_switch().unwrap();
        authority.pair_external(other, 1).unwrap();
    }

    #[test]
    fn generation_overflow_rejects_without_partial_state_change() {
        let id = principal("client-a");
        let mut authority = AgentAuthority::default();
        authority.pair_external(id.clone(), 7).unwrap();
        authority.authority_generation = u64::MAX;
        assert_eq!(
            authority.clear_kill_switch(),
            Err(AgentAuthorityError::GenerationOverflow)
        );
        assert!(!authority.is_kill_switch_active());

        authority.authority_generation = 0;
        let state = authority.principals.get_mut(&id).unwrap();
        state.generation = u64::MAX;
        let before = state.clone();
        assert_eq!(
            authority.promote(&id, 7),
            Err(AgentAuthorityError::GenerationOverflow)
        );
        assert_eq!(
            authority.principals.get(&id).unwrap().generation,
            before.generation
        );
        assert_eq!(authority.principals.get(&id).unwrap().mode, before.mode);
        assert_eq!(
            authority.kill_switch(),
            Err(AgentAuthorityError::GenerationOverflow)
        );
        assert!(!authority.is_kill_switch_active());
        assert!(!authority.principals.get(&id).unwrap().revoked);
    }
}
