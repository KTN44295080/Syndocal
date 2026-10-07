//! Explicit-target adapters over the durable project publication service.
//! Caller identity and authorization are derived from the immutable native dispatch.
use super::{
    agent_bridge::{AgentBridge, AgentBridgeDispatch},
    native_adapter_error::NativeAdapterError,
    AppState, ProjectCoordinator, ProjectPublicationRequestV1,
    ProjectPublicationSurfaceV1 as Surface, ProjectPublicationTargetPolicyV1 as TargetPolicy,
};
use protocol::control_plane_command::MAX_SAFE_JAVASCRIPT_INTEGER as MAX;
use protocol::control_plane_file::{
    self as wire, ProjectBackupAuthorityRequestV1, ProjectFileAuthorityRequestV1,
    ProjectFileAuthorityV1, ProjectFileBackupSummaryV1, ProjectFilePhaseV1, ProjectFileRequestV1,
    ProjectFileStatusV1,
};
use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Component, Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, WebviewWindow};

const MAX_TARGET_BYTES: u64 = 128 * 1024 * 1024;
const MAX_AUDIT: usize = 65_536;
#[derive(Default)]
pub(crate) struct ProjectFileControlPlaneState {
    admission: Mutex<Admission>,
    backup_targets: Mutex<BackupTargets>,
}
#[derive(Default)]
struct BackupTargets {
    issued: HashMap<(String, String), Instant>,
}
impl BackupTargets {
    fn issue(
        &mut self,
        origin: &str,
        request: &ProjectFileRequestV1,
        now: Instant,
    ) -> Result<(), String> {
        self.issued
            .retain(|_, at| now.saturating_duration_since(*at) < Duration::from_secs(300));
        let key = (origin.into(), request_hash(request)?);
        if !self.issued.contains_key(&key) && self.issued.len() >= 256 {
            return Err("project_file_backup_authority_capacity".into());
        }
        self.issued.insert(key, now);
        Ok(())
    }
    fn validate(
        &self,
        origin: &str,
        request: &ProjectFileRequestV1,
        now: Instant,
    ) -> Result<(), String> {
        self.issued
            .get(&(origin.into(), request_hash(request)?))
            .filter(|at| now.saturating_duration_since(**at) < Duration::from_secs(300))
            .map(|_| ())
            .ok_or_else(|| "project_file_guard_backup_authority_expired_or_unknown".into())
    }
}
#[derive(Default)]
struct Admission {
    callers: HashMap<String, (f64, Instant)>,
    audit: Vec<super::diagnostic_audit::Row>,
    reserved: usize,
}
impl ProjectFileControlPlaneState {
    pub(crate) fn diagnostic_audit(&self,before:Option<u64>)->Result<super::diagnostic_audit::Page,String> {
        use super::diagnostic_audit::page;
        let inner=super::diagnostic_audit_capture::read_lock(&self.admission,"file")?;
        let page=page("project_file",inner.audit.iter(),before,|r|r.sequence,|r,_|r.clone())?;
        if page.records.iter().any(|row|row.phase=="invalid") {
            return Err("diagnostic_audit_file_receipt_invalid; preserve the original operation receipt and report the invalid typed result".into());
        }
        Ok(page)
    }
    pub(super) fn admitted<T: super::project_file_audit::AuditResult>(
        &self,
        caller: &str,
        operation: &str,
        request_id: u64,
        request: &impl serde::Serialize,
        run: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        let now = Instant::now();
        let shape = request_hash(request)?;
        {
            let mut inner = self
                .admission
                .lock()
                .map_err(|_| "project_file_admission_poisoned")?;
            inner
                .callers
                .retain(|_, (_, at)| now.saturating_duration_since(*at) < Duration::from_secs(900));
            if inner.audit.len() + inner.reserved + 2 > MAX_AUDIT
                || (!inner.callers.contains_key(caller) && inner.callers.len() >= 256)
            {
                return Err("project_file_admission_capacity".into());
            }
            let (tokens, at) = inner.callers.entry(caller.into()).or_insert((8.0, now));
            *tokens = (*tokens + now.saturating_duration_since(*at).as_secs_f64() * 4.0).min(8.0);
            *at = now;
            if *tokens < 1.0 {
                return Err("project_file_rate_busy".into());
            }
            *tokens -= 1.0;
            inner.reserved += 1;
            let mut row=super::diagnostic_audit::Row::new(inner.audit.len() as u64+1,"admitted",Some(caller),Some(operation),Some(request_id));
            row.shape_sha256=Some(shape.clone());
            inner.audit.push(row);
        }
        let result = run();
        let mut inner = self
            .admission
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        inner.reserved -= 1;
        let mut row=super::diagnostic_audit::Row::new(inner.audit.len() as u64+1,"terminal",Some(caller),Some(operation),Some(request_id));
        row.shape_sha256=Some(shape);
        match &result {
            Ok(value)=>match value.audit_outcome(operation,request_id) {
                Ok(outcome)=>{
                    row.succeeded=outcome.succeeded;
                    row.outcome_sha256=Some(super::diagnostic_audit::identity_hash(outcome.code));
                    row.generation_before=outcome.generation_before;row.generation_after=outcome.generation_after;
                },
                // Audit validation never changes a possibly committed result.
                // The invalid row instead blocks diagnostic publication.
                Err(_)=>row.phase="invalid",
            },
            // A failed call can follow an irreversible effect (for example,
            // deletion succeeded but its terminal journal flush failed).
            // Only a typed receipt can establish the actual effect outcome.
            Err(_)=>{row.succeeded=None;row.outcome_sha256=Some(super::diagnostic_audit::identity_hash("native_error"));},
        }
        inner.audit.push(row);
        result
    }
}

#[derive(Clone)]
struct Context {
    app: AppHandle,
    dispatch: Option<AgentBridgeDispatch>,
    state_address: usize,
    window_label: String,
    owner_id: String,
    owner_incarnation: u64,
    origin: String,
    request: ProjectFileRequestV1,
}
thread_local! { static CONTEXT: RefCell<Option<Context>> = const { RefCell::new(None) }; }

fn surface(id: &str) -> Result<Surface, String> {
    match id {
        wire::SAVE_ID => Ok(Surface::Save),
        wire::SAVE_AS_ID => Ok(Surface::SaveAs),
        wire::TEMPLATE_ID => Ok(Surface::UserTemplate),
        wire::BACKUP_ID => Ok(Surface::Backup),
        _ => Err("project_file_operation_invalid".into()),
    }
}
fn request_hash(request: &impl serde::Serialize) -> Result<String, String> {
    serde_json::to_vec(request)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|_| "project_file_request_encoding_failed".into())
}
fn identity(
    state: &AppState,
    label: &str,
    caller: &str,
    operation: &str,
) -> Result<(String, String, u64), String> {
    let owner = super::current_project_transaction_owner_for_window(state, label)?;
    let incarnation = super::project_transaction_owner_binding_for_window(state, label, &owner)?;
    let bytes = serde_json::to_vec(&(label, incarnation, caller, operation))
        .map_err(|_| "project_file_identity_invalid")?;
    Ok((
        format!("mcp-file-{:x}", Sha256::digest(bytes)),
        owner,
        incarnation,
    ))
}
fn legacy_request(
    request: &ProjectFileRequestV1,
    origin: &str,
    owner: &str,
) -> Result<ProjectPublicationRequestV1, String> {
    request.validate().map_err(str::to_string)?;
    let surface = surface(&request.operation_id)?;
    Ok(ProjectPublicationRequestV1 {
        schema_version: 1,
        origin_id: origin.into(),
        owner_id: owner.into(),
        request_id: request.request_id,
        surface,
        expected_project_epoch: request.expected_fence.project_epoch,
        expected_project_revision: request.expected_fence.project_revision,
        expected_checkpoint_hash: request.expected_fence.project_checkpoint_hash.clone(),
        mapping_authority_hash: request.expected_fence.project_checkpoint_hash.clone(),
        source_path: None,
        reason: Some(format!("mcp-file-{}", request_hash(request)?)),
        target_policy: match surface {
            Surface::Save => TargetPolicy::CurrentOrDialog,
            Surface::Backup => TargetPolicy::ManagedUnique,
            _ => TargetPolicy::Dialog,
        },
    })
}

fn target_key(id: &str, destination: &str) -> Result<PathBuf, String> {
    let target = Path::new(destination);
    super::validate_project_publication_target_path_v1(surface(id)?, target)?;
    if target
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        return Err("project_file_target_parent_traversal".into());
    }
    super::normalized_recovery_target_key(target)
}
/// Bounded, streaming target observation; directories and links fail closed.
fn target_digest(target: &Path) -> Result<Option<String>, String> {
    let metadata = match fs::symlink_metadata(target) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("project_file_target_metadata: {error}")),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > MAX_TARGET_BYTES
    {
        return Err("project_file_target_not_bounded_regular_file".into());
    }
    let mut file =
        fs::File::open(target).map_err(|error| format!("project_file_target_open: {error}"))?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut count = 0u64;
    loop {
        let size = file
            .read(&mut buffer)
            .map_err(|error| format!("project_file_target_read: {error}"))?;
        if size == 0 {
            break;
        }
        count += size as u64;
        if count > MAX_TARGET_BYTES {
            return Err("project_file_target_size_changed".into());
        }
        hash.update(&buffer[..size]);
    }
    if count != metadata.len()
        || file
            .seek(SeekFrom::End(0))
            .map_err(|_| "project_file_target_seek")?
            != count
    {
        return Err("project_file_target_size_changed".into());
    }
    Ok(Some(format!("{:x}", hash.finalize())))
}
fn authorize(context: &Context, state: &AppState) -> Result<(), String> {
    if context.state_address != std::ptr::from_ref(state).addr() {
        return Err("project_file_guard_state_mismatch".into());
    }
    if let Some(dispatch) = &context.dispatch {
        context
            .app
            .state::<AgentBridge>()
            .authority("main")?
            .authorize_bridge_request(
                &dispatch.principal_id,
                dispatch.principal_incarnation,
                &dispatch.method,
                &dispatch.params, &dispatch.request_id,)
            .map_err(|_| "project_file_guard_authorization_revoked")?;
    }
    Ok(())
}
fn current_authority(
    context: &Context,
    state: &AppState,
    coordinator: &ProjectCoordinator,
) -> Result<(), String> {
    authorize(context, state)?;
    let request = &context.request;
    // Query capture takes query -> coordinator locks. The issued-session
    // check therefore belongs before publication locks in execute(), while
    // this final CAS verifies the live server-owned window incarnation.
    if super::project_transaction_owner_binding_for_window(
        state,
        &context.window_label,
        &context.owner_id,
    )? != context.owner_incarnation
    {
        return Err("project_file_guard_owner_incarnation_changed".into());
    }
    let fence = &request.expected_fence;
    if (
        coordinator.epoch,
        coordinator.revision,
        coordinator.checkpoint_hash.as_str(),
        coordinator.publication_generation,
    ) != (
        fence.project_epoch,
        fence.project_revision,
        fence.project_checkpoint_hash.as_str(),
        fence.project_publication_generation,
    ) || coordinator.path_generation != request.expected_path_generation
        || coordinator.authority_disposition_generation != request.expected_disposition_generation
    {
        return Err("project_file_guard_stale_fence".into());
    }
    if [
        coordinator.path_generation,
        coordinator.authority_disposition_generation,
        coordinator.recovery_authority_serial,
    ]
    .iter()
    .any(|value| *value >= MAX)
    {
        return Err("project_file_guard_generation_exhausted".into());
    }
    Ok(())
}
fn current(
    context: &Context,
    state: &AppState,
    coordinator: &ProjectCoordinator,
) -> Result<(), String> {
    current_authority(context, state, coordinator)?;
    let request = &context.request;
    if request.operation_id == wire::BACKUP_ID {
        context
            .app
            .state::<ProjectFileControlPlaneState>()
            .backup_targets
            .lock()
            .map_err(|_| "project_file_backup_authority_poisoned")?
            .validate(&context.origin, request, Instant::now())?;
    }
    let key = if request.operation_id == wire::BACKUP_ID {
        super::project_file_managed_backup::key(&context.app, &request.destination)?
    } else {
        target_key(&request.operation_id, &request.destination)?
    };
    if key != Path::new(&request.destination) {
        return Err("project_file_guard_noncanonical_target".into());
    }
    if request.operation_id == wire::SAVE_ID
        && coordinator
            .ancillary
            .current_project_path
            .as_ref()
            .map(|path| super::normalized_recovery_target_key(path))
            .transpose()?
            .as_deref()
            != Some(key.as_path())
    {
        return Err("project_file_guard_save_current_path_mismatch".into());
    }
    if target_digest(&key)? != request.expected_target_sha256 {
        return Err("project_file_guard_target_changed".into());
    }
    Ok(())
}

/// Runs under the existing admission/coordinator locks before a new reservation or resume.
pub(super) fn validate_begin(
    state: &AppState,
    coordinator: &mut ProjectCoordinator,
) -> Result<(), String> {
    CONTEXT.with(|slot| {
        if let Some(context) = slot.borrow().as_ref() {
            super::reconcile_project_checkpoint_for_coordinator(state, coordinator)?;
            current(context, state, coordinator)?;
        }
        Ok(())
    })
}
pub(super) fn explicit_target(state: &AppState) -> Result<Option<PathBuf>, String> {
    CONTEXT.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|context| {
                authorize(context, state)?;
                Ok(PathBuf::from(&context.request.destination))
            })
            .transpose()
    })
}
pub(super) fn explicit_backup_target(
    state: &AppState,
    directory: &Path,
) -> Result<Option<PathBuf>, String> {
    CONTEXT.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|context| {
                authorize(context, state)?;
                if context.request.operation_id != wire::BACKUP_ID {
                    return Err("project_file_guard_backup_context_mismatch".into());
                }
                super::project_file_managed_backup::legacy_target(
                    directory,
                    &context.request.destination,
                )
            })
            .transpose()
    })
}
pub(super) fn create_backup_directory(state: &AppState, directory: &Path) -> Result<(), String> {
    CONTEXT.with(|slot| {
        if let Some(context) = slot.borrow().as_ref() {
            if context.request.operation_id != wire::BACKUP_ID {
                return Err("project_file_guard_backup_context_mismatch".into());
            }
            let _publication = state
                .project_save_publication
                .lock()
                .map_err(|_| "project_file_publication_poisoned")?;
            let _admission = super::lock_project_external_command_admission(state)?;
            let mut coordinator = super::lock_project_coordinator(state)?;
            super::reconcile_project_checkpoint_for_coordinator(state, &mut coordinator)?;
            current(context, state, &coordinator)?;
            fs::create_dir_all(directory)
                .map_err(|error| format!("Unable to create project backup directory: {error}"))
        } else {
            fs::create_dir_all(directory)
                .map_err(|error| format!("Unable to create project backup directory: {error}"))
        }
    })
}
pub(super) fn backup_metadata(
    state: &AppState,
    ticket: &super::ProjectSaveTicket,
    source: Option<String>,
    reason: String,
) -> Result<(Option<String>, String), String> {
    CONTEXT.with(|slot| {
        let borrowed = slot.borrow();
        let Some(context) = borrowed.as_ref() else {
            return Ok((source, reason));
        };
        authorize(context, state)?;
        if context.request.operation_id != wire::BACKUP_ID {
            return Err("project_file_guard_backup_context_mismatch".into());
        }
        Ok((
            ticket
                .current_project_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            "MCP backup".into(),
        ))
    })
}
/// Retention is a separate post-publication effect: the new target now exists,
/// while principal/owner/project/path authority must remain current per deletion.
pub(super) fn validate_backup_retention(
    state: &AppState,
    coordinator: &ProjectCoordinator,
) -> Result<(), String> {
    CONTEXT.with(|slot| {
        if let Some(context) = slot.borrow().as_ref() {
            if context.request.operation_id != wire::BACKUP_ID {
                return Err("project_file_guard_backup_context_mismatch".into());
            }
            current_authority(context, state, coordinator)?;
        }
        Ok(())
    })
}
pub(super) fn publish_backup(
    state: &AppState,
    coordinator: &ProjectCoordinator,
    temp: &Path,
    target: &Path,
) -> Result<(), String> {
    if CONTEXT.with(|slot| slot.borrow().is_some()) {
        publish(state, coordinator, temp, target)
    } else {
        super::publish_new_file_atomically(temp, target)
    }
}
/// The final authorization/target fence is adjacent to the real filesystem effect.
pub(super) fn publish(
    state: &AppState,
    coordinator: &ProjectCoordinator,
    temp: &Path,
    target: &Path,
) -> Result<(), String> {
    CONTEXT.with(|slot| {
        let borrowed = slot.borrow();
        let Some(context) = borrowed.as_ref() else {
            return super::publish_prepared_project_save_if_current(temp, target, true);
        };
        current(context, state, coordinator)
            .map_err(|error| format!("project_file_guard_final: {error}"))?;
        let actual_target = if context.request.operation_id == wire::BACKUP_ID {
            super::normalized_recovery_target_key(target)?
        } else {
            target.to_path_buf()
        };
        if actual_target != Path::new(&context.request.destination) {
            return Err("project_file_guard_destination_mismatch".into());
        }
        if context.request.expected_target_sha256.is_none() {
            // Reuse the existing no-replace publication primitive (Windows
            // MoveFileExW with WRITE_THROUGH, same-directory link elsewhere).
            // A target created after observation cannot be overwritten.
            publish_missing_target(temp, target)
        } else {
            super::replace_file_atomically(temp, target)
        }
    })
}

fn publish_missing_target(temp: &Path, target: &Path) -> Result<(), String> {
    // A no-replace rejection is not evidence that we published the artifact.
    // Keep it out of template hash-only success reconciliation, even when a
    // racing writer created exactly the same prepared bytes.
    super::publish_new_file_atomically(temp, target)
        .map_err(|error| format!("project_file_guard_no_replace: {error}"))
}

fn stored_status(
    state: &AppState,
    request: &ProjectFileRequestV1,
    native: &ProjectPublicationRequestV1,
) -> Result<ProjectFileStatusV1, String> {
    let _publication = state
        .project_save_publication
        .lock()
        .map_err(|_| "project_file_publication_poisoned")?;
    let app = super::project_swap_app_handle(state)?;
    let durable = super::load_project_recovery_authority_state_from_path(
        &super::project_recovery_authority_state_path(&app)?,
    )?;
    let shape = super::project_publication_shape_hash_v1(native)?;
    let mut status = ProjectFileStatusV1 {
        schema_version: 1,
        request: request.clone(),
        phase: ProjectFilePhaseV1::Missing,
        target_path: None,
        artifact_sha256: None,
        recovery_authority_serial: 0,
        saved_project_epoch: request.expected_fence.project_epoch,
        saved_project_revision: request.expected_fence.project_revision,
        saved_checkpoint_hash: request.expected_fence.project_checkpoint_hash.clone(),
        error: None,
        warning: None,
        backup: None,
    };
    if let Some(terminal) = durable
        .publication_journal
        .terminals
        .iter()
        .find(|value| super::project_publication_request_key_matches(native, &value.request))
    {
        if terminal.shape_hash != shape {
            return Err("project_file_request_shape_conflict".into());
        }
        use super::ProjectPublicationTerminalOutcomeV1 as Outcome;
        status.phase = match terminal.outcome {
            Outcome::Succeeded => ProjectFilePhaseV1::Succeeded,
            Outcome::Cancelled => ProjectFilePhaseV1::Cancelled,
            Outcome::Abandoned => ProjectFilePhaseV1::Abandoned,
            Outcome::Failed => ProjectFilePhaseV1::Failed,
            Outcome::Indeterminate => ProjectFilePhaseV1::Indeterminate,
        };
        status.target_path = terminal.target_path.clone();
        status.artifact_sha256 = terminal.artifact_digest.clone();
        status.recovery_authority_serial = terminal.recovery_authority_serial;
        status.saved_project_epoch = terminal.project_epoch;
        status.saved_project_revision = terminal.project_revision;
        status.saved_checkpoint_hash = terminal.checkpoint_hash.clone();
        status.error = terminal.error.clone();
        status.warning = terminal.warning.clone();
        if let Some(backup) = terminal.backup.as_ref() {
            if [backup.id, backup.created_at_unix_ms, backup.bytes]
                .iter()
                .any(|value| *value > MAX)
            {
                return Err("project_file_backup_receipt_integer_unsafe".into());
            }
            status.backup = Some(ProjectFileBackupSummaryV1 {
                id: backup.id,
                created_at_unix_ms: backup.created_at_unix_ms,
                source_path: backup.source_path.clone(),
                reason: backup.reason.clone(),
                bytes: backup.bytes,
            });
        }
    } else if let Some(pending) = durable
        .publication_journal
        .pending
        .iter()
        .find(|value| super::project_publication_request_key_matches(native, &value.request))
    {
        if pending.shape_hash != shape {
            return Err("project_file_request_shape_conflict".into());
        }
        use super::ProjectPublicationPendingPhaseV1 as Phase;
        status.phase = match pending.phase {
            Phase::Reserved => ProjectFilePhaseV1::Reserved,
            Phase::Selecting => ProjectFilePhaseV1::Selecting,
            Phase::Selected => ProjectFilePhaseV1::Selected,
            Phase::Prepared => ProjectFilePhaseV1::Prepared,
        };
        status.target_path = pending
            .target_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned());
        status.artifact_sha256 = pending.prepared_digest.clone();
        status.recovery_authority_serial = pending.recovery_authority_serial_before;
        status.error = pending.indeterminate_error.clone();
    } else if let Some(origin) = durable
        .publication_journal
        .origins
        .iter()
        .find(|value| value.origin_id == native.origin_id)
    {
        if origin.acknowledged_request_id == native.request_id {
            if origin.acknowledged_shape_hash.as_deref() != Some(&shape) {
                return Err("project_file_request_shape_conflict".into());
            }
            status.phase = ProjectFilePhaseV1::Acknowledged;
        } else if native.request_id <= origin.high_water_request_id {
            return Err("project_file_request_retired".into());
        }
    }
    if status.recovery_authority_serial > MAX {
        return Err("project_file_receipt_integer_unsafe".into());
    }
    Ok(status)
}

fn authority(
    app: &AppHandle,
    window: &WebviewWindow,
    caller: &str,
    request: ProjectFileAuthorityRequestV1,
) -> Result<ProjectFileAuthorityV1, NativeAdapterError> {
    request.validate().map_err(str::to_string)?;
    let state = app.state::<AppState>();
    let (origin, _, _) = identity(&state, window.label(), caller, &request.operation_id)?;
    let destination = if request.operation_id == wire::BACKUP_ID {
        super::project_file_managed_backup::key(app, &request.destination)?
    } else {
        target_key(&request.operation_id, &request.destination)?
    };
    let target_sha256 = target_digest(&destination)?;
    if request.operation_id == wire::BACKUP_ID && target_sha256.is_some() {
        return Err("project_file_backup_candidate_changed".into());
    }
    let fence = app
        .state::<super::ControlPlaneQueryState>()
        .issue_project_mutation_fence_for_window(window.label(), &state)
        .map_err(NativeAdapterError::from)?;
    let _publication = state
        .project_save_publication
        .lock()
        .map_err(|_| "project_file_publication_poisoned")?;
    let _admission = super::lock_project_external_command_admission(&state)?;
    let mut coordinator = super::lock_project_coordinator(&state)?;
    super::reconcile_project_checkpoint_for_coordinator(&state, &mut coordinator)?;
    if (
        coordinator.epoch,
        coordinator.revision,
        coordinator.checkpoint_hash.as_str(),
        coordinator.publication_generation,
    ) != (
        fence.project_epoch,
        fence.project_revision,
        fence.project_checkpoint_hash.as_str(),
        fence.project_publication_generation,
    ) {
        return Err("project_file_authority_changed_during_query".into());
    }
    let durable = super::load_project_recovery_authority_state_from_path(
        &super::project_recovery_authority_state_path(app)?,
    )?;
    let next_request_id = durable
        .publication_journal
        .origins
        .iter()
        .find(|value| value.origin_id == origin)
        .map_or(Ok(1), |value| {
            value
                .high_water_request_id
                .checked_add(1)
                .filter(|id| *id <= MAX)
                .ok_or("project_file_request_sequence_exhausted")
        })?;
    if coordinator.path_generation > MAX || coordinator.authority_disposition_generation > MAX {
        return Err("project_file_authority_integer_unsafe".into());
    }
    Ok(ProjectFileAuthorityV1 {
        schema_version: 1,
        fence,
        path_generation: coordinator.path_generation,
        disposition_generation: coordinator.authority_disposition_generation,
        next_request_id,
        current_project_path: coordinator
            .ancillary
            .current_project_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        destination: destination.to_string_lossy().into_owned(),
        target_sha256,
    })
}

fn backup_authority(
    app: &AppHandle,
    window: &WebviewWindow,
    caller: &str,
    request: ProjectBackupAuthorityRequestV1,
) -> Result<ProjectFileAuthorityV1, NativeAdapterError> {
    request.validate().map_err(str::to_string)?;
    let destination = super::project_file_managed_backup::observe(app)?;
    let result = authority(
        app,
        window,
        caller,
        ProjectFileAuthorityRequestV1 {
            schema_version: 1,
            operation_id: wire::BACKUP_ID.into(),
            destination: destination.to_string_lossy().into_owned(),
        },
    )?;
    let (origin, _, _) = identity(
        &app.state::<AppState>(),
        window.label(),
        caller,
        wire::BACKUP_ID,
    )?;
    let issued = ProjectFileRequestV1 {
        schema_version: 1,
        operation_id: wire::BACKUP_ID.into(),
        request_id: result.next_request_id,
        expected_fence: result.fence.clone(),
        expected_path_generation: result.path_generation,
        expected_disposition_generation: result.disposition_generation,
        destination: result.destination.clone(),
        expected_target_sha256: None,
    };
    app.state::<ProjectFileControlPlaneState>()
        .backup_targets
        .lock()
        .map_err(|_| "project_file_backup_authority_poisoned")?
        .issue(&origin, &issued, Instant::now())?;
    Ok(result)
}

fn with_context<T>(context: Context, run: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    CONTEXT.with(|slot| {
        if slot.borrow().is_some() {
            return Err("project_file_nested_context".into());
        }
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                CONTEXT.with(|slot| *slot.borrow_mut() = None);
            }
        }
        *slot.borrow_mut() = Some(context);
        let _reset = Reset;
        run()
    })
}
fn execute(
    app: &AppHandle,
    window: &WebviewWindow,
    caller: &str,
    dispatch: Option<AgentBridgeDispatch>,
    operation: &str,
    request: ProjectFileRequestV1,
) -> Result<ProjectFileStatusV1, String> {
    request.validate().map_err(str::to_string)?;
    if wire::publication_id(operation) && operation != request.operation_id {
        return Err("project_file_operation_mismatch".into());
    }
    let state = app.state::<AppState>();
    let (origin, owner, incarnation) =
        identity(&state, window.label(), caller, &request.operation_id)?;
    let native = legacy_request(&request, &origin, &owner)?;
    if operation == wire::STATUS_ID {
        return stored_status(&state, &request, &native);
    }
    let context = Context {
        app: app.clone(),
        dispatch,
        state_address: std::ptr::from_ref(&*state).addr(),
        window_label: window.label().into(),
        owner_id: owner.clone(),
        owner_incarnation: incarnation,
        origin: origin.clone(),
        request: request.clone(),
    };
    app.state::<ProjectFileControlPlaneState>()
        .admitted(&origin, operation, request.request_id, &request, || {
            with_context(context.clone(), || {
                authorize(&context, &state)?;
                let existing = stored_status(&state, &request, &native)?;
                if operation == wire::ACK_ID {
                    let _publication = state
                        .project_save_publication
                        .lock()
                        .map_err(|_| "project_file_publication_poisoned")?;
                    let _admission = super::lock_project_recovery_maintenance_admission(&state)?;
                    let coordinator = super::lock_project_coordinator(&state)?;
                    super::ensure_project_publication_mutation_owner_allowed(
                        &state,
                        &coordinator,
                        window.label(),
                        &owner,
                    )?;
                    authorize(&context, &state)?;
                    super::ensure_project_publication_receipt_not_claimed(
                        &state,
                        &native,
                        &super::project_publication_shape_hash_v1(&native)?,
                    )?;
                    let path = super::project_recovery_authority_state_path(app)?;
                    let mut durable =
                        super::load_project_recovery_authority_state_from_path(&path)?;
                    if super::acknowledge_project_publication_receipt_in_state_v1(
                        &mut durable,
                        &native,
                        &super::project_publication_shape_hash_v1(&native)?,
                    )? {
                        super::persist_project_recovery_authority_state_to_path(&path, &durable)?;
                    }
                    drop(coordinator);
                    drop(_admission);
                    drop(_publication);
                } else if wire::publication_id(operation) {
                    if !matches!(
                        existing.phase,
                        ProjectFilePhaseV1::Missing
                            | ProjectFilePhaseV1::Reserved
                            | ProjectFilePhaseV1::Selected
                    ) {
                        return Ok(existing);
                    }
                    app.state::<super::ControlPlaneQueryState>()
                        .validate_project_mutation_fence_window(
                            window.label(),
                            &request.expected_fence,
                            incarnation,
                        )
                        .map_err(|_| "project_file_guard_session_fence")?;
                    if operation == wire::BACKUP_ID {
                        super::save_project_backup_v1(
                            window.clone(),
                            app.clone(),
                            app.state::<AppState>(),
                            native.clone(),
                        )?;
                    } else {
                        super::execute_project_save_publication_v1(
                            window,
                            &state,
                            native.clone(),
                            surface(operation)?,
                        )?;
                    }
                } else {
                    return Err("project_file_operation_invalid".into());
                }
                stored_status(&state, &request, &native)
            })
        })
}

pub(crate) fn execute_local(
    app: &AppHandle,
    window: &WebviewWindow,
    operation: &str,
    request: Value,
) -> Result<Value, NativeAdapterError> {
    if operation == wire::BACKUP_LIST_ID {
        return serde_json::to_value(super::project_backup_listing::list(app,
            serde_json::from_value(request).map_err(|_| "project_file_request_invalid")?,
        )?).map_err(|_| "project_file_response_invalid".into());
    }
    if operation == wire::BACKUP_INSPECT_ID {
        return serde_json::to_value(super::project_backup_inspection::inspect(
            app,
            serde_json::from_value(request).map_err(|_| "project_file_request_invalid")?,
        )?).map_err(|_| "project_file_response_invalid".into());
    }
    if operation == wire::BACKUP_AUTHORITY_ID {
        return serde_json::to_value(backup_authority(
            app,
            window,
            "local",
            serde_json::from_value(request).map_err(|_| "project_file_request_invalid")?,
        )?)
        .map_err(|_| "project_file_response_invalid".into());
    }
    if operation == wire::AUTHORITY_ID {
        return serde_json::to_value(authority(
            app,
            window,
            "local",
            serde_json::from_value(request).map_err(|_| "project_file_request_invalid")?,
        )?)
        .map_err(|_| "project_file_response_invalid".into());
    }
    let request: ProjectFileRequestV1 =
        serde_json::from_value(request).map_err(|_| "project_file_request_invalid")?;
    request.validate().map_err(str::to_string)?;
    if operation != wire::STATUS_ID
        && !matches!(
            MessageDialog::new()
                .set_level(MessageLevel::Warning)
                .set_title("Confirm project file operation")
                .set_description("Publish or acknowledge this project file operation?")
                .set_buttons(MessageButtons::YesNo)
                .set_parent(window)
                .show(),
            MessageDialogResult::Yes
        )
    {
        return Err("project_file_local_confirmation_cancelled".into());
    }
    serde_json::to_value(execute(app, window, "local", None, operation, request)?)
        .map_err(|_| "project_file_response_invalid".into())
}
pub(crate) fn execute_external(
    app: &AppHandle,
    window: &WebviewWindow,
    dispatch: &AgentBridgeDispatch,
) -> Result<Value, NativeAdapterError> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Ingress {
        request: Value,
    }
    let operation = dispatch
        .params
        .get("operationId")
        .and_then(Value::as_str)
        .ok_or("project_file_operation_invalid")?;
    let ingress: Ingress = serde_json::from_value(
        dispatch
            .params
            .get("request")
            .cloned()
            .ok_or("project_file_request_invalid")?,
    )
    .map_err(|_| "project_file_request_invalid")?;
    app.state::<AgentBridge>()
        .authority("main")?
        .authorize_bridge_request(
            &dispatch.principal_id,
            dispatch.principal_incarnation,
            &dispatch.method,
            &dispatch.params, &dispatch.request_id,)?;
    let caller = format!(
        "external:{}:{}",
        dispatch.principal_id, dispatch.principal_incarnation
    );
    if operation == wire::BACKUP_LIST_ID {
        let listed = super::project_backup_listing::list(app,
            serde_json::from_value(ingress.request).map_err(|_| "project_file_request_invalid")?,
        )?;
        app.state::<AgentBridge>().authority("main")?.authorize_bridge_request(
            &dispatch.principal_id, dispatch.principal_incarnation, &dispatch.method, &dispatch.params, &dispatch.request_id,)?;
        return serde_json::to_value(listed).map_err(|_| "project_file_response_invalid".into());
    }
    if operation == wire::BACKUP_INSPECT_ID {
        let inspected = super::project_backup_inspection::inspect(
            app,
            serde_json::from_value(ingress.request).map_err(|_| "project_file_request_invalid")?,
        )?;
        app.state::<AgentBridge>().authority("main")?.authorize_bridge_request(
            &dispatch.principal_id, dispatch.principal_incarnation, &dispatch.method, &dispatch.params, &dispatch.request_id,)?;
        return serde_json::to_value(inspected).map_err(|_| "project_file_response_invalid".into());
    }
    if operation == wire::BACKUP_AUTHORITY_ID {
        return serde_json::to_value(backup_authority(
            app,
            window,
            &caller,
            serde_json::from_value(ingress.request).map_err(|_| "project_file_request_invalid")?,
        )?)
        .map_err(|_| "project_file_response_invalid".into());
    }
    if operation == wire::AUTHORITY_ID {
        return serde_json::to_value(authority(
            app,
            window,
            &caller,
            serde_json::from_value(ingress.request).map_err(|_| "project_file_request_invalid")?,
        )?)
        .map_err(|_| "project_file_response_invalid".into());
    }
    serde_json::to_value(execute(
        app,
        window,
        &caller,
        Some(dispatch.clone()),
        operation,
        serde_json::from_value(ingress.request).map_err(|_| "project_file_request_invalid")?,
    )?)
    .map_err(|_| "project_file_response_invalid".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn deletion_audit_fixture() -> wire::ProjectBackupDeleteReceiptV1 {
        let artifact_sha256="b".repeat(64);
        wire::ProjectBackupDeleteReceiptV1 {schema_version:1,
            request:wire::ProjectBackupDeleteRequestV1 {schema_version:1,operation_id:wire::BACKUP_DELETE_ID.into(),
                request_id:1,expected_fence:request().expected_fence,backup_id:7,expected_artifact_sha256:artifact_sha256.clone()},
            deleted_backup:wire::ProjectBackupInspectionV1 {schema_version:1,
                backup:wire::ProjectFileBackupSummaryV1 {id:7,created_at_unix_ms:123,source_path:Some("C:/PRIVATE_PATH.sdc".into()),
                    reason:"PRIVATE_LABEL".into(),bytes:128},artifact_sha256,restore_source_path:None}}
    }
    fn management_audit_fixture() -> protocol::control_plane_backup_management::ManagementReceiptV1 {
        use protocol::control_plane_backup_management as management;
        let record_id="c".repeat(64);
        management::ManagementReceiptV1 {schema_version:1,
            request:management::ManagementRequestV1 {schema_version:1,operation_id:management::MANAGE_ID.into(),request_id:1,
                expected_fence:request().expected_fence,expected_generation:4,expected_journal_sha256:"d".repeat(64),
                action:management::ManagementActionV1::Acknowledge {record_ids:vec![record_id.clone()]}},
            generation_before:4,generation_after:5,storage_version_before:2,affected_record_ids:vec![record_id],observed_artifact:None}
    }
    #[test]
    fn diagnostic_audit_backup_delete_success_without_publication_phase_is_exportable() {
        let receipt=deletion_audit_fixture();receipt.validate().unwrap();
        let state=ProjectFileControlPlaneState::default();
        state.admitted("PRIVATE_CALLER",wire::BACKUP_DELETE_ID,1,&receipt.request,||Ok(receipt.clone())).unwrap();
        let page=serde_json::to_value(state.diagnostic_audit(None).unwrap()).unwrap();
        assert_eq!(page["records"][1]["phase"],"terminal");
        assert_eq!(page["records"][1]["succeeded"],true);
        assert!(!page.to_string().contains("PRIVATE"));
    }
    #[test]
    fn diagnostic_audit_management_success_without_publication_phase_is_exportable() {
        let receipt=management_audit_fixture();receipt.validate().unwrap();
        let state=ProjectFileControlPlaneState::default();
        state.admitted("PRIVATE_CALLER",protocol::control_plane_backup_management::MANAGE_ID,1,&receipt.request,||Ok(receipt.clone())).unwrap();
        let page=serde_json::to_value(state.diagnostic_audit(None).unwrap()).unwrap();
        assert_eq!(page["records"][1]["phase"],"terminal");
        assert_eq!(page["records"][1]["succeeded"],true);
        assert_eq!(page["records"][1]["generation_before"],4);
        assert_eq!(page["records"][1]["generation_after"],5);
    }
    #[test]
    fn diagnostic_audit_file_projection_preserves_original_private_results() {
        let state=ProjectFileControlPlaneState::default();
        let mut private=publication_audit_fixture(ProjectFilePhaseV1::Succeeded);
        private.target_path=Some("C:/PRIVATE_PATH.sdc".into());private.error=Some("PRIVATE_CREDENTIAL".repeat(131_072));
        let returned=state.admitted("PRIVATE_CALLER",wire::SAVE_AS_ID,1,&request(),||Ok(private.clone())).unwrap();
        assert_eq!(returned.target_path,private.target_path);assert_eq!(returned.error,private.error);
        let failed:Result<ProjectFileStatusV1,String>=state.admitted("PRIVATE_CALLER",wire::SAVE_AS_ID,2,&request(),||Err("PRIVATE_ERROR_PATH".into()));
        assert!(failed.is_err());
        let original=state.admission.lock().unwrap().audit.clone();
        let first=serde_json::to_value(state.diagnostic_audit(None).unwrap()).unwrap();
        assert_eq!(first,serde_json::to_value(state.diagnostic_audit(None).unwrap()).unwrap());
        assert_eq!(first["retained_count"],4);assert_eq!(first["records"][1]["succeeded"],true);
        assert_eq!(first["records"][3]["succeeded"],Value::Null);
        assert!(!first.to_string().contains("PRIVATE"));
        assert_eq!(state.admission.lock().unwrap().audit,original);
        let retained=serde_json::to_string(&original).unwrap();
        assert!(!retained.contains("PRIVATE"));assert!(!retained.contains("result"));assert!(retained.len()<4096);
    }
    #[test]
    fn diagnostic_audit_file_status_success_does_not_claim_unknown_or_failed_publication() {
        use ProjectFilePhaseV1::*;
        for (phase,code,succeeded) in [(Succeeded,"succeeded",Some(true)),(Acknowledged,"acknowledged",Some(true)),
            (Cancelled,"cancelled",Some(false)),(Abandoned,"abandoned",Some(false)),(Failed,"failed",Some(false)),
            (Reserved,"reserved",None),(Selecting,"selecting",None),(Selected,"selected",None),
            (Prepared,"prepared",None),(Indeterminate,"indeterminate",None),(Missing,"missing",None)] {
            let state=ProjectFileControlPlaneState::default();
            state.admitted("PRIVATE_CALLER",wire::SAVE_AS_ID,1,&request(),||Ok(publication_audit_fixture(phase))).unwrap();
            let page=serde_json::to_value(state.diagnostic_audit(None).unwrap()).unwrap();
            assert_eq!(page["records"][1]["succeeded"],serde_json::to_value(succeeded).unwrap(),"{code}");
            assert_eq!(page["records"][1]["outcome_sha256"],super::super::diagnostic_audit::identity_hash(code));
            assert!(!page.to_string().contains("PRIVATE"));
        }
    }
    fn publication_audit_fixture(phase:ProjectFilePhaseV1)->ProjectFileStatusV1 {
        ProjectFileStatusV1 {schema_version:1,request:request(),phase,target_path:None,artifact_sha256:None,
            recovery_authority_serial:0,saved_project_epoch:3,saved_project_revision:4,saved_checkpoint_hash:"a".repeat(64),
            error:None,warning:None,backup:None}
    }
    #[test]
    fn diagnostic_audit_never_serializes_or_retains_the_terminal_result_body() {
        struct NonSerializable {private_body:Vec<u8>}
        impl super::super::project_file_audit::AuditResult for NonSerializable {
            fn audit_outcome(&self,_:&str,_:u64)->Result<super::super::project_file_audit::Outcome,&'static str> {
                Ok(super::super::project_file_audit::Outcome {code:"succeeded",succeeded:Some(true),generation_before:None,generation_after:None})
            }
        }
        let state=ProjectFileControlPlaneState::default();
        let result=state.admitted("PRIVATE_CALLER",wire::SAVE_AS_ID,1,&request(),||Ok(NonSerializable {private_body:vec![7;2*1024*1024]})).unwrap();
        assert_eq!(result.private_body.len(),2*1024*1024);
        let audit=serde_json::to_string(&state.admission.lock().unwrap().audit).unwrap();
        assert!(audit.len()<2048);assert!(!audit.contains("PRIVATE"));
        assert_eq!(state.diagnostic_audit(None).unwrap().records[1].succeeded,Some(true));
    }
    #[test]
    fn diagnostic_audit_invalid_typed_receipts_block_export_without_rewriting_results() {
        let state=ProjectFileControlPlaneState::default();
        let mut receipt=deletion_audit_fixture();receipt.deleted_backup.artifact_sha256="f".repeat(64);
        let returned=state.admitted("PRIVATE_CALLER",wire::BACKUP_DELETE_ID,1,&receipt.request,||Ok(receipt.clone())).unwrap();
        assert_eq!(returned.deleted_backup.artifact_sha256,receipt.deleted_backup.artifact_sha256);
        assert!(state.diagnostic_audit(None).unwrap_err().contains("file_receipt_invalid"));
        let state=ProjectFileControlPlaneState::default();
        let mut receipt=management_audit_fixture();receipt.generation_after=receipt.generation_before;
        let returned=state.admitted("PRIVATE_CALLER",protocol::control_plane_backup_management::MANAGE_ID,1,&receipt.request,||Ok(receipt.clone())).unwrap();
        assert_eq!(returned.generation_after,receipt.generation_after);
        assert!(state.diagnostic_audit(None).unwrap_err().contains("file_receipt_invalid"));
        use super::super::project_file_audit::AuditResult;
        assert!(deletion_audit_fixture().audit_outcome(wire::SAVE_AS_ID,1).is_err());
        assert!(management_audit_fixture().audit_outcome(protocol::control_plane_backup_management::MANAGE_ID,2).is_err());
        assert!(publication_audit_fixture(ProjectFilePhaseV1::Succeeded).audit_outcome(wire::SAVE_AS_ID,2).is_err());
    }
    #[test]
    fn diagnostic_audit_indeterminate_error_never_claims_an_irreversible_effect_failed() {
        let state=ProjectFileControlPlaneState::default();
        let receipt=deletion_audit_fixture();
        let error="project_backup_delete_indeterminate; artifact deleted but terminal publication failed; PRIVATE_PATH";
        let result:Result<wire::ProjectBackupDeleteReceiptV1,String>=state.admitted("PRIVATE_CALLER",wire::BACKUP_DELETE_ID,1,
            &receipt.request,||Err(error.into()));
        assert_eq!(result.unwrap_err(),error);
        let page=state.diagnostic_audit(None).unwrap();
        assert_eq!(page.records[1].phase,"terminal");assert_eq!(page.records[1].succeeded,None);
        assert_eq!(page.records[1].outcome_sha256,Some(super::super::diagnostic_audit::identity_hash("native_error")));
        assert!(!serde_json::to_string(&page).unwrap().contains("PRIVATE"));
    }
    fn request() -> ProjectFileRequestV1 {
        ProjectFileRequestV1 {
            schema_version: 1,
            operation_id: wire::SAVE_AS_ID.into(),
            request_id: 1,
            expected_fence: protocol::control_plane_command::ProjectMutationFenceV1 {
                process_incarnation: 1,
                session_incarnation: 2,
                project_epoch: 3,
                project_revision: 4,
                project_checkpoint_hash: "a".repeat(64),
                project_publication_generation: 5,
            },
            expected_path_generation: 6,
            expected_disposition_generation: 7,
            destination: "C:/private/日本語.sdc".into(),
            expected_target_sha256: None,
        }
    }
    #[test]
    fn project_file_durable_shape_binds_full_fence_and_target() {
        let original = request();
        let native = legacy_request(&original, "server-origin", "server-owner").unwrap();
        let shape = super::super::project_publication_shape_hash_v1(&native).unwrap();
        for index in 0..8 {
            let mut changed = original.clone();
            match index {
                0 => changed.expected_fence.process_incarnation += 1,
                1 => changed.expected_fence.session_incarnation += 1,
                2 => changed.expected_fence.project_publication_generation += 1,
                3 => changed.expected_path_generation += 1,
                4 => changed.expected_disposition_generation += 1,
                5 => changed.destination.push_str(".sdc"),
                6 => changed.expected_target_sha256 = Some("b".repeat(64)),
                _ => changed.operation_id = wire::SAVE_ID.into(),
            }
            let other = legacy_request(&changed, "server-origin", "server-owner").unwrap();
            assert_eq!(native.origin_id, other.origin_id);
            assert_eq!(native.request_id, other.request_id);
            assert_ne!(
                shape,
                super::super::project_publication_shape_hash_v1(&other).unwrap()
            );
        }
        assert_eq!(native.owner_id, "server-owner");
        assert_eq!(
            native.mapping_authority_hash,
            original.expected_fence.project_checkpoint_hash
        );
    }
    #[test]
    fn project_file_target_rejects_relative_traversal_and_wrong_surface_extension() {
        for target in [
            "relative.sdc",
            "C:/private/../show.sdc",
            "C:/private/wrong.zip",
        ] {
            assert!(target_key(wire::SAVE_AS_ID, target).is_err());
        }
        assert!(target_key(wire::TEMPLATE_ID, "C:/private/show.sdc").is_err());
    }
    #[test]
    fn project_file_bounded_target_observation_and_no_clobber() {
        let directory = std::env::temp_dir().join(format!(
            "syndocal-project-file-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let target = directory.join("日本語.sdc");
        let temp = directory.join("staging.tmp");
        assert_eq!(target_digest(&target).unwrap(), None);
        fs::write(&temp, b"prepared").unwrap();
        fs::write(&target, b"racing writer").unwrap();
        assert!(publish_missing_target(&temp, &target)
            .unwrap_err()
            .starts_with("project_file_guard_"));
        assert_eq!(fs::read(&target).unwrap(), b"racing writer");
        assert_eq!(
            target_digest(&target).unwrap(),
            Some(format!("{:x}", Sha256::digest(b"racing writer")))
        );
        assert!(target_digest(&directory).is_err());
        fs::write(&target, b"prepared").unwrap();
        assert!(publish_missing_target(&temp, &target)
            .unwrap_err()
            .starts_with("project_file_guard_"));
        assert_eq!(fs::read(&temp).unwrap(), b"prepared");
        assert_eq!(fs::read(&target).unwrap(), b"prepared");
        fs::remove_file(&target).unwrap();
        publish_missing_target(&temp, &target).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"prepared");
        assert!(!temp.exists());
        fs::remove_file(target).unwrap();
        fs::remove_dir(directory).unwrap();
    }
    #[test]
    fn project_file_backup_authority_binds_origin_full_request_expiry_and_capacity() {
        let now = Instant::now();
        let mut authority = BackupTargets::default();
        let mut original = request();
        original.operation_id = wire::BACKUP_ID.into();
        authority.issue("actual-origin", &original, now).unwrap();
        authority.validate("actual-origin", &original, now).unwrap();
        assert!(authority.validate("forged-origin", &original, now).is_err());
        for field in 0..3 {
            let mut changed = original.clone();
            match field {
                0 => changed.destination.push('0'),
                1 => changed.request_id += 1,
                _ => changed.expected_fence.project_publication_generation += 1,
            }
            assert!(authority.validate("actual-origin", &changed, now).is_err());
        }
        assert!(authority
            .validate("actual-origin", &original, now + Duration::from_secs(300))
            .is_err());
        for index in 1..256 {
            authority
                .issue(&format!("origin-{index}"), &original, now)
                .unwrap();
        }
        assert!(authority.issue("overflow", &original, now).is_err());
        authority
            .issue("fresh", &original, now + Duration::from_secs(300))
            .unwrap();
        assert_eq!(authority.issued.len(), 1);
    }
}
