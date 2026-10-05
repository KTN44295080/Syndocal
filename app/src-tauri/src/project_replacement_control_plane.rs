//! Typed New/Open/backup restore share the replacement lifecycle and publication CAS.
use std::{
    cell::RefCell,
    collections::HashMap,
    path::Path,
    sync::Mutex,
    time::{Duration, Instant},
};

use protocol::control_plane_project::{
    ProjectReplacementActionV1, ProjectReplacementAuthorityV1, ProjectReplacementErrorV1 as Error,
    ProjectReplacementOutcomeV1, ProjectReplacementReceiptV1, ProjectReplacementRequestV1,
    ProjectReplacementResponseV1 as Response,
};
use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager, WebviewWindow};

use super::{
    agent_bridge::{AgentBridge, AgentBridgeDispatch},
    authored_control_plane::{AuthoredControlPlaneState, GenericAuthoredMutationReceiptKey},
    control_plane_query::ControlPlaneQueryState,
    AppState, PreparedProjectLoad,
};

const MAX_CALLERS: usize = 256;
const MAX_AUDIT_RECORDS: usize = 65_536;
const CALLER_TTL: Duration = Duration::from_secs(15 * 60);

#[derive(Serialize)]
struct AuditRecord {
    stage: &'static str,
    caller: String,
    operation_id: String,
    request_id: u64,
    shape_sha256: String,
    response: Option<Response>,
}

struct Bucket {
    tokens: f64,
    updated: Instant,
}

#[derive(Default)]
struct AdmissionState {
    callers: HashMap<String, Bucket>,
    audit: Vec<AuditRecord>,
    reserved_terminals: usize,
}

#[derive(Default)]
pub(crate) struct ProjectReplacementControlPlaneState {
    // Reuse the bounded exact terminal/single-flight implementation. This
    // namespace binds the full start fence in the shape, rather than in the
    // key, so changing the fence cannot reuse an already-consumed request ID.
    receipts: AuthoredControlPlaneState,
    admission: Mutex<AdmissionState>,
}

impl ProjectReplacementControlPlaneState {
    fn admit(
        &self,
        caller: &str,
        request: &ProjectReplacementRequestV1,
        shape: &str,
        now: Instant,
    ) -> Result<(), Error> {
        let mut inner = self.admission.lock().map_err(|_| Error::Internal)?;
        inner
            .callers
            .retain(|_, bucket| now.saturating_duration_since(bucket.updated) < CALLER_TTL);
        if inner.audit.len() + inner.reserved_terminals + 2 > MAX_AUDIT_RECORDS
            || (!inner.callers.contains_key(caller) && inner.callers.len() >= MAX_CALLERS)
        {
            return Err(Error::Overloaded);
        }
        let bucket = inner.callers.entry(caller.to_string()).or_insert(Bucket {
            tokens: 8.0,
            updated: now,
        });
        bucket.tokens = (bucket.tokens
            + now.saturating_duration_since(bucket.updated).as_secs_f64() * 4.0)
            .min(8.0);
        bucket.updated = now;
        if bucket.tokens < 1.0 {
            return Err(Error::Busy);
        }
        bucket.tokens -= 1.0;
        inner.reserved_terminals += 1;
        inner.audit.push(AuditRecord {
            stage: "admitted",
            caller: caller.into(),
            operation_id: request.operation_id.clone(),
            request_id: request.request_id,
            shape_sha256: shape.into(),
            response: None,
        });
        Ok(())
    }

    fn terminal(
        &self,
        caller: &str,
        request: &ProjectReplacementRequestV1,
        shape: &str,
        response: &Response,
    ) {
        let mut inner = self
            .admission
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.reserved_terminals -= 1;
        inner.audit.push(AuditRecord {
            stage: "terminal",
            caller: caller.into(),
            operation_id: request.operation_id.clone(),
            request_id: request.request_id,
            shape_sha256: shape.into(),
            response: Some(response.clone()),
        });
    }
}

#[derive(Clone)]
struct ExternalAuthorization {
    app: AppHandle,
    dispatch: AgentBridgeDispatch,
    state_address: usize,
}

thread_local! {
    static EXTERNAL_AUTHORIZATION: RefCell<Option<ExternalAuthorization>> = const { RefCell::new(None) };
}

/// Called by each existing replacement preflight, including the final CAS.
/// No renderer field selects this origin or supplies this immutable request.
pub(super) fn validate_external_authorization(state: &AppState) -> Result<(), String> {
    EXTERNAL_AUTHORIZATION.with(|slot| {
        let value = slot.borrow();
        let Some(context) = value.as_ref() else {
            return Ok(());
        };
        if context.state_address != std::ptr::from_ref(state).addr() {
            return Err("project_replacement_external_state_mismatch".into());
        }
        context
            .app
            .state::<AgentBridge>()
            .authority("main")?
            .authorize_bridge_request(
                &context.dispatch.principal_id,
                context.dispatch.principal_incarnation,
                &context.dispatch.method,
                &context.dispatch.params,
            )
            .map_err(|_| "project_replacement_external_authorization_revoked".into())
    })
}

fn rejection(request: &ProjectReplacementRequestV1, code: Error) -> Response {
    Response::Rejected {
        request_id: request.request_id,
        code,
    }
}

fn classify(error: &str) -> Error {
    if error.contains("different shape") || error.contains("retired") {
        Error::Conflict
    } else if error.contains("capacity") || error.contains("limit") || error.contains("exhausted") {
        Error::Overloaded
    } else if error.contains("authority changed")
        || error.contains("publication generation changed")
    {
        Error::StaleFence
    } else if error.contains("pending") || error.contains("busy") {
        Error::Busy
    } else if error.contains("Lock blocks")
        || error.contains("Full Lock")
        || error.contains("FullLock")
        || error.contains("authorization")
        || error.contains("owner")
        || error.contains("renderer incarnation")
    {
        Error::Forbidden
    } else {
        Error::PublicationFailed
    }
}

pub(super) fn prepare(
    request: &ProjectReplacementRequestV1,
    backup_directory: Option<&Path>,
) -> Result<PreparedProjectLoad, Error> {
    let mut prepared = match &request.action {
        ProjectReplacementActionV1::New {} => super::prepare_project_load(
            protocol::ProjectFile {
                version: super::PROJECT_FILE_VERSION,
                app: super::APP_NAME.into(),
                operator_policy: None,
                custom_profiles: Vec::new(),
                fixture_groups: Vec::new(),
                snapshot: protocol::EngineSnapshot::default(),
            },
            super::ProjectControlMappings::default(),
            "New project".into(),
            None,
        )
        .map_err(|_| Error::InvalidProject),
        ProjectReplacementActionV1::Open {
            path,
            expected_file_sha256,
        } => {
            let target = Path::new(path);
            if !target.is_absolute() {
                return Err(Error::InvalidRequest);
            }
            super::validate_project_open_path(target).map_err(|_| Error::InvalidProject)?;
            // Hash and parse the same bounded read. A changed file is rejected
            // before retiring output; there is no hash(A)/parse(B) reopen.
            let bytes = super::read_project_bytes(target).map_err(|_| Error::InvalidProject)?;
            if format!("{:x}", Sha256::digest(&bytes)) != *expected_file_sha256 {
                return Err(Error::FileChanged);
            }
            let value =
                super::parse_project_json_bytes(&bytes).map_err(|_| Error::InvalidProject)?;
            let (project, mappings) = super::project_and_control_mappings_from_value(value)
                .map_err(|_| Error::InvalidProject)?;
            super::prepare_project_load(project, mappings, path.clone(), Some(target))
                .map_err(|_| Error::InvalidProject)
        }
        ProjectReplacementActionV1::RestoreBackup {
            backup_id,
            expected_file_sha256,
            expected_source_path,
        } => super::project_backup_restoration::prepare(
            backup_directory.ok_or(Error::InvalidProject)?,
            *backup_id,
            expected_file_sha256,
            expected_source_path,
        ),
    }?;
    // Prepare through the real Engine load/persistence policy, with a private
    // deny-output runtime. In particular, an empty legacy Timeline bank must
    // be classified before the Engine normalizes it. This runs before output
    // retirement and hashes the image that publication will acknowledge.
    prepared.snapshot = super::project_snapshot_for_save(
        engine::prepare_project_snapshot_persistence(prepared.snapshot)
            .map_err(|_| Error::InvalidProject)?,
    );
    Ok(prepared)
}

pub(super) fn execute_core(
    state: &AppState,
    query: &ControlPlaneQueryState,
    control: &ProjectReplacementControlPlaneState,
    window_label: &str,
    caller: &str,
    request: ProjectReplacementRequestV1,
    backup_directory: Option<&Path>,
    confirm: impl FnOnce() -> bool,
    publish: impl FnOnce(PreparedProjectLoad) -> Result<ProjectReplacementAuthorityV1, Error>,
) -> Response {
    if request.validate().is_err() {
        return rejection(&request, Error::InvalidRequest);
    }
    let owner = match super::current_project_transaction_owner_for_window(state, window_label) {
        Ok(owner) => owner,
        Err(_) => return rejection(&request, Error::Forbidden),
    };
    let incarnation =
        match super::project_transaction_owner_binding_for_window(state, window_label, &owner) {
            Ok(value) => value,
            Err(_) => return rejection(&request, Error::Forbidden),
        };
    let bytes = match serde_json::to_vec(&request) {
        Ok(value) => value,
        Err(_) => return rejection(&request, Error::InvalidRequest),
    };
    let shape = format!("{:x}", Sha256::digest(&bytes));
    let caller = format!("{window_label}:{incarnation}:{caller}");
    let key = GenericAuthoredMutationReceiptKey {
        operation_id: request.operation_id.clone(),
        request_id: format!("project-replacement:{caller}:{}", request.request_id),
        window_label: window_label.into(),
        owner_id: owner.clone(),
        owner_incarnation: incarnation,
        // Stable identity for this dedicated cache namespace. Every E/R/H,
        // publication, process/session and Open-target field remains in shape.
        start_epoch: 0,
        start_revision: 0,
        start_checkpoint_hash: String::new(),
    };
    let result =
        control
            .receipts
            .generic_terminal_single_flight(key, shape.clone(), Instant::now(), || {
                if query
                    .validate_project_mutation_fence_window(
                        window_label,
                        &request.expected_fence,
                        incarnation,
                    )
                    .is_err()
                {
                    return Ok(rejection(&request, Error::Forbidden));
                }
                let fence = &request.expected_fence;
                let command = match &request.action {
                    ProjectReplacementActionV1::New {} => "new_project_control_plane_v1",
                    ProjectReplacementActionV1::Open { .. } => "open_project_control_plane_v1",
                    ProjectReplacementActionV1::RestoreBackup { .. } => {
                        "restore_project_backup_control_plane_v1"
                    }
                };
                let outcome = super::with_project_replacement_invocation_generation(
                    state,
                    window_label,
                    command,
                    owner,
                    (
                        Some(incarnation),
                        fence.project_epoch,
                        fence.project_revision,
                        fence.project_checkpoint_hash.clone(),
                    ),
                    Some(fence.project_publication_generation),
                    || {
                        super::preflight_project_replacement_invocation(state)?;
                        if let Err(code) = control.admit(&caller, &request, &shape, Instant::now())
                        {
                            return Ok(rejection(&request, code));
                        }
                        let outcome = (|| {
                            let prepared = prepare(&request, backup_directory)?;
                            if !confirm() {
                                return Ok(ProjectReplacementOutcomeV1::Cancelled);
                            }
                            super::preflight_project_replacement_invocation(state)
                                .map_err(|error| classify(&error))?;
                            publish(prepared).map(ProjectReplacementOutcomeV1::Applied)
                        })();
                        let response = match outcome {
                            Ok(outcome) => {
                                let receipt = ProjectReplacementReceiptV1 {
                                    request: request.clone(),
                                    outcome,
                                };
                                if receipt.validate().is_err() {
                                    rejection(&request, Error::PublicationFailed)
                                } else {
                                    Response::Receipt(receipt)
                                }
                            }
                            Err(code) => rejection(&request, code),
                        };
                        control.terminal(&caller, &request, &shape, &response);
                        Ok(response)
                    },
                );
                Ok(match outcome {
                    Ok(response) => response,
                    Err(error) => rejection(&request, classify(&error)),
                })
            });
    match result {
        Ok(response) => response,
        Err(error) => rejection(&request, classify(&error)),
    }
}

#[cfg(test)]
mod admission_tests {
    use super::*;
    use protocol::control_plane_command::ProjectMutationFenceV1;

    fn request(id: u64) -> ProjectReplacementRequestV1 {
        ProjectReplacementRequestV1 {
            schema_version: 1,
            operation_id: protocol::control_plane_project::PROJECT_NEW_OPERATION_ID.into(),
            request_id: id,
            expected_fence: ProjectMutationFenceV1 {
                process_incarnation: 1,
                session_incarnation: 2,
                project_epoch: 3,
                project_revision: 4,
                project_checkpoint_hash: "a".repeat(64),
                project_publication_generation: 5,
            },
            action: ProjectReplacementActionV1::New {},
        }
    }

    #[test]
    fn project_replacement_control_plane_rate_refills_and_keeps_terminal_audit_slots() {
        let control = ProjectReplacementControlPlaneState::default();
        let now = Instant::now();
        for id in 1..=8 {
            let request = request(id);
            control.admit("caller", &request, "shape", now).unwrap();
            control.terminal(
                "caller",
                &request,
                "shape",
                &rejection(&request, Error::InvalidProject),
            );
        }
        assert_eq!(
            control.admit("caller", &request(9), "shape", now),
            Err(Error::Busy)
        );
        control
            .admit(
                "caller",
                &request(9),
                "shape",
                now + Duration::from_millis(250),
            )
            .unwrap();
        assert_eq!(control.admission.lock().unwrap().audit.len(), 17);
        assert_eq!(control.admission.lock().unwrap().reserved_terminals, 1);
    }
}

fn publish(
    state: &AppState,
    prepared: PreparedProjectLoad,
) -> Result<ProjectReplacementAuthorityV1, Error> {
    let result = super::replace_prepared_project_snapshot(
        state,
        prepared,
        super::ProjectSnapshotReplacementScope::ExternalCaller,
        None,
    )
    .map_err(|error| classify(&error))?;
    // The committed bundle was captured under the publication coordinator.
    // Never reread a later project to manufacture this request's receipt.
    let authority = result.authority.ok_or(Error::PublicationFailed)?;
    Ok(ProjectReplacementAuthorityV1 {
        project_epoch: authority.project_epoch,
        project_revision: authority.project_revision,
        checkpoint_hash: authority.checkpoint_hash,
        publication_generation: authority.publication_generation,
        recovery_authority_serial: authority.recovery_authority_serial,
        current_project_path: authority.current_project_path,
    })
}

pub(crate) fn execute_local(
    app: &AppHandle,
    window: &WebviewWindow,
    request: ProjectReplacementRequestV1,
    operation_id: &str,
) -> Response {
    if request.operation_id != operation_id {
        return rejection(&request, Error::InvalidRequest);
    }
    let state = app.state::<AppState>();
    let backup_directory = match &request.action {
        ProjectReplacementActionV1::RestoreBackup { .. } => {
            match super::app_data_subdirectory_path(app, super::PROJECT_BACKUP_DIRECTORY) {
                Ok(path) => Some(path),
                Err(_) => return rejection(&request, Error::InvalidProject),
            }
        }
        _ => None,
    };
    execute_core(
        &state,
        &app.state::<ControlPlaneQueryState>(),
        &app.state::<ProjectReplacementControlPlaneState>(),
        window.label(),
        "local",
        request,
        backup_directory.as_deref(),
        || {
            matches!(MessageDialog::new().set_level(MessageLevel::Warning)
            .set_title("Confirm project replacement")
            .set_description("Replace the current project and stop its outputs? Save pending edits first.")
            .set_buttons(MessageButtons::YesNo).set_parent(window).show(), MessageDialogResult::Yes)
        },
        |prepared| publish(&state, prepared),
    )
}

pub(crate) fn execute_external(
    app: &AppHandle,
    window: &WebviewWindow,
    dispatch: &AgentBridgeDispatch,
    request: ProjectReplacementRequestV1,
) -> Result<Response, String> {
    if dispatch.method != "control_plane.execute"
        || dispatch.params.get("operationId").and_then(|v| v.as_str())
            != Some(request.operation_id.as_str())
    {
        return Err("project_replacement_immutable_operation_mismatch".into());
    }
    let state = app.state::<AppState>();
    let immutable: ProjectReplacementRequestV1 =
        serde_json::from_value(dispatch.params["request"]["request"].clone())
            .map_err(|_| "project_replacement_immutable_request_invalid")?;
    if immutable != request {
        return Err("project_replacement_immutable_request_mismatch".into());
    }
    EXTERNAL_AUTHORIZATION.with(|slot| {
        if slot.borrow().is_some() {
            return Err("project_replacement_nested_authorization".into());
        }
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                EXTERNAL_AUTHORIZATION.with(|slot| *slot.borrow_mut() = None);
            }
        }
        *slot.borrow_mut() = Some(ExternalAuthorization {
            app: app.clone(),
            dispatch: dispatch.clone(),
            state_address: std::ptr::from_ref(&*state).addr(),
        });
        let _reset = Reset;
        validate_external_authorization(&state)?;
        let backup_directory = if matches!(
            &request.action,
            ProjectReplacementActionV1::RestoreBackup { .. }
        ) {
            Some(super::app_data_subdirectory_path(
                app,
                super::PROJECT_BACKUP_DIRECTORY,
            )?)
        } else {
            None
        };
        let caller = format!(
            "external:{}:{}",
            dispatch.principal_id, dispatch.principal_incarnation
        );
        Ok(execute_core(
            &state,
            &app.state::<ControlPlaneQueryState>(),
            &app.state::<ProjectReplacementControlPlaneState>(),
            window.label(),
            &caller,
            request,
            backup_directory.as_deref(),
            || true,
            |prepared| publish(&state, prepared),
        ))
    })
}
