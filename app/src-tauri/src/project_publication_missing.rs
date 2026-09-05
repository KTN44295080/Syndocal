//! Retire an exact, never-reserved request from a retired renderer without
//! dispatching it or touching any project, backup, target or staging file.
use super::*;

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum MissingPublicationResolutionV1 {
    Terminal {
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
        status: ProjectPublicationStatusV1,
    },
    Acknowledged {
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
        request: ProjectPublicationRequestV1,
        #[serde(rename = "shapeHash")]
        shape_hash: String,
    },
}

pub(super) fn resolve_for_window(
    state: &AppState, window_label: &str, journal_path: &Path,
    request: ProjectPublicationRequestV1, current_owner_id: String,
) -> Result<MissingPublicationResolutionV1, String> {
    resolve_with_writer(state, window_label, journal_path, request, current_owner_id,
        persist_project_recovery_authority_state_to_path)
}

fn resolve_with_writer(
    state: &AppState, window_label: &str, journal_path: &Path,
    request: ProjectPublicationRequestV1, current_owner_id: String,
    persist: impl FnOnce(&Path, &PersistedProjectRecoveryAuthorityState) -> Result<(), String>,
) -> Result<MissingPublicationResolutionV1, String> {
    let request = normalize_project_publication_request_v1(request)?;
    let current_owner_id = normalize_project_transaction_owner_id(current_owner_id)?;
    let _publication = state.project_save_publication.lock()
        .map_err(|_| "Project publication lock was poisoned")?;
    let _admission = lock_project_external_command_admission(state)?;
    let coordinator = lock_project_coordinator(state)?;
    ensure_project_publication_mutation_owner_allowed(state, &coordinator, window_label, &current_owner_id)?;
    // Retain the registry guard through the durable write. A renderer cannot
    // become live again between our absence proof and sequence consumption.
    let owners = state.project_transaction_owners.lock()
        .map_err(|_| "Project transaction owner registry was poisoned")?;
    if owners.get(window_label) != Some(&current_owner_id) {
        return Err("Missing publication resolution lost the invoking renderer owner".into());
    }
    if owners.values().any(|owner| owner == &request.owner_id) {
        return Err("Missing publication resolution requires the previous renderer owner to be retired".into());
    }
    let durable = load_project_recovery_authority_state_from_path(journal_path)?;
    let (resolution, candidate) = resolve_journal(&durable, &request)?;
    if let Some(candidate) = candidate {
        persist(journal_path, &candidate)?;
    }
    Ok(resolution)
}

fn resolve_journal(
    durable: &PersistedProjectRecoveryAuthorityState,
    request: &ProjectPublicationRequestV1,
) -> Result<(MissingPublicationResolutionV1, Option<PersistedProjectRecoveryAuthorityState>), String> {
    validate_project_publication_journal_v1(&durable.publication_journal)?;
    let shape_hash = project_publication_shape_hash_v1(request)?;
    let journal = &durable.publication_journal;
    if let Some(terminal) = journal.terminals.iter()
        .find(|terminal| project_publication_request_key_matches(request, &terminal.request)) {
        if terminal.request != *request || terminal.shape_hash != shape_hash {
            return Err("Missing publication resolution found a conflicting terminal shape".into());
        }
        return Ok((MissingPublicationResolutionV1::Terminal {
            schema_version: 1, status: project_publication_terminal_status_v1(terminal, None),
        }, None));
    }
    if journal.pending.iter().any(|pending| pending.request.origin_id == request.origin_id)
        || journal.terminals.iter().any(|terminal| terminal.request.origin_id == request.origin_id) {
        return Err("Missing publication resolution found an unresolved request for this origin; query its receipt before continuing".into());
    }
    let origin = journal.origins.iter().find(|origin| origin.origin_id == request.origin_id);
    if let Some(origin) = origin {
        if origin.acknowledged_request_id == request.request_id {
            if origin.acknowledged_shape_hash.as_deref() != Some(shape_hash.as_str()) {
                return Err("Missing publication resolution acknowledgement shape mismatch".into());
            }
            return Ok((MissingPublicationResolutionV1::Acknowledged {
                schema_version: 1, request: request.clone(), shape_hash,
            }, None));
        }
    }
    let expected = match origin {
        Some(origin) => origin.high_water_request_id.checked_add(1)
            .filter(|value| *value <= VIDEO_CLIP_RUNTIME_GENERATION_MAX)
            .ok_or("Missing publication resolution request sequence is exhausted")?,
        None => 1,
    };
    if request.request_id != expected {
        return Err(format!("Missing publication resolution cannot prove this request was never reserved: expected {expected}, received {}. Keep the durable intent for recovery.", request.request_id));
    }
    if journal.pending.len().saturating_add(journal.terminals.len()) >= MAX_PROJECT_PUBLICATION_TERMINALS {
        return Err("Missing publication resolution receipt capacity is full; settle existing receipts first".into());
    }
    let terminal = PersistedProjectPublicationTerminalV1 {
        request: request.clone(), shape_hash, surface: request.surface,
        outcome: ProjectPublicationTerminalOutcomeV1::Abandoned,
        project_epoch: request.expected_project_epoch,
        project_revision: request.expected_project_revision,
        checkpoint_hash: request.expected_checkpoint_hash.clone(),
        target_path: None, artifact_digest: None, backup: None,
        recovery_authority_serial: durable.serial,
        error: None, warning: None,
    };
    let mut candidate = durable.clone();
    if let Some(origin) = candidate.publication_journal.origins.iter_mut()
        .find(|origin| origin.origin_id == request.origin_id) {
        origin.high_water_request_id = request.request_id;
    } else {
        candidate.publication_journal.origins.push(PersistedProjectPublicationOriginV1 {
            origin_id: request.origin_id.clone(), high_water_request_id: request.request_id,
            acknowledged_request_id: 0, acknowledged_shape_hash: None,
        });
    }
    candidate.publication_journal.terminals.push(terminal.clone());
    validate_project_publication_journal_v1(&candidate.publication_journal)?;
    Ok((MissingPublicationResolutionV1::Terminal {
        schema_version: 1, status: project_publication_terminal_status_v1(&terminal, None),
    }, Some(candidate)))
}

#[cfg(test)]
#[path = "project_publication_missing_tests.rs"]
mod tests;
