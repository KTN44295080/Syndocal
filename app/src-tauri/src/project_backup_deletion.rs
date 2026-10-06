//! Exact managed artifact deletion. Project identity and the durable backup
//! guards remain locked through the same-handle filesystem effect.
use std::{path::Path, time::Instant};
use protocol::control_plane_file::{ProjectBackupDeleteRequestV1 as Request,
    ProjectBackupDeleteReceiptV1 as Receipt, ProjectBackupInspectionV1, BACKUP_DELETE_ID};
use sha2::{Digest, Sha256};
use tauri::{Manager, AppHandle, WebviewWindow};
use super::{AppState, ControlPlaneQueryState, agent_bridge::AgentBridgeDispatch,
    authored_control_plane::{AuthoredControlPlaneState, GenericAuthoredMutationReceiptKey},
    project_file_control_plane::ProjectFileControlPlaneState};

#[derive(Default)]
pub(crate) struct BackupDeletionControlPlaneState {
    receipts: AuthoredControlPlaneState,
}

struct Prepared {
    #[cfg(windows)]
    file: std::fs::File,
    inspection: ProjectBackupInspectionV1,
}
#[cfg(windows)]
fn prepare(directory: &Path, request: &Request) -> Result<Prepared, String> {
    use std::{fs, os::windows::fs::OpenOptionsExt};
    use windows::Win32::{Foundation::GENERIC_READ,
        Storage::FileSystem::{DELETE, FILE_FLAG_OPEN_REPARSE_POINT}};
    let path = super::project_file_managed_backup::path_for_id(directory, request.backup_id)?;
    let metadata = fs::symlink_metadata(&path).map_err(|e| format!("project_backup_delete_metadata: {e}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("project_backup_delete_not_regular_file".into());
    }
    // No other read/write/delete handle may coexist with this observation.
    // No truncation, creation, rename or deletion occurs during preparation.
    let file = fs::OpenOptions::new().read(true).access_mode(GENERIC_READ.0 | DELETE.0)
        .share_mode(0).custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(&path).map_err(|e| format!("project_backup_delete_open: {e}"))?;
    let inspection = super::project_backup_inspection::read_opened(&path, &file,
        super::project_backup_json::PROJECT_BACKUP_MAX_BYTES)?.inspection();
    if inspection.artifact_sha256 != request.expected_artifact_sha256 {
        return Err("project_backup_delete_artifact_changed".into());
    }
    Ok(Prepared { file, inspection })
}
#[cfg(not(windows))]
fn prepare(_directory: &Path, _request: &Request) -> Result<Prepared, String> {
    // A pathname hash/unlink sequence cannot establish the same-handle contract.
    Err("project_backup_delete_same_handle_platform_unsupported".into())
}
#[cfg(windows)]
fn commit(prepared: Prepared) -> Result<ProjectBackupInspectionV1, String> {
    use std::{ffi::c_void, mem::size_of, os::windows::io::AsRawHandle};
    use windows::Win32::{Foundation::HANDLE, Storage::FileSystem::{
        SetFileInformationByHandle, FileDispositionInfo, FILE_DISPOSITION_INFO}};
    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    // The exclusive original handle is the one whose bytes were decoded/hash
    // checked. Close it before releasing publication/admission/coordinator locks.
    unsafe { SetFileInformationByHandle(HANDLE(prepared.file.as_raw_handle()),
        FileDispositionInfo, (&disposition as *const FILE_DISPOSITION_INFO).cast::<c_void>(),
        size_of::<FILE_DISPOSITION_INFO>() as u32) }
        .map_err(|e| format!("project_backup_delete_disposition: {e}"))?;
    drop(prepared.file);
    Ok(prepared.inspection)
}
#[cfg(not(windows))]
fn commit(_prepared: Prepared) -> Result<ProjectBackupInspectionV1, String> {
    Err("project_backup_delete_same_handle_platform_unsupported".into())
}

pub(super) fn execute_core(
    state: &AppState, query: &ControlPlaneQueryState, control: &BackupDeletionControlPlaneState,
    admission: &ProjectFileControlPlaneState, label: &str, caller: &str, request: Request,
    directory: &Path, journal: &Path, authorize: impl Fn() -> Result<(), String>,
    confirm: impl FnOnce() -> bool,
) -> Result<Receipt, String> {
    request.validate().map_err(str::to_string)?;
    authorize()?;
    let owner = super::current_project_transaction_owner_for_window(state, label)?;
    let incarnation = super::project_transaction_owner_binding_for_window(state, label, &owner)?;
    let shape = format!("{:x}", Sha256::digest(serde_json::to_vec(&request)
        .map_err(|_| "project_backup_delete_request_encoding")?));
    let caller = format!("{label}:{incarnation}:{caller}");
    let key = GenericAuthoredMutationReceiptKey {
        operation_id: BACKUP_DELETE_ID.into(),
        request_id: format!("backup-delete:{caller}:{}", request.request_id),
        window_label: label.into(), owner_id: owner.clone(), owner_incarnation: incarnation,
        start_epoch: 0, start_revision: 0, start_checkpoint_hash: String::new(),
    };
    // A retained exact terminal fact precedes fresh-fence/file checks. Replays
    // cannot delete a newly created file, or adopt another caller/owner's result.
    control.receipts.generic_terminal_single_flight(key, shape, Instant::now(), || {
        query.validate_project_mutation_fence_window(label, &request.expected_fence, incarnation)
            .map_err(|_| "project_backup_delete_unissued_fence")?;
        admission.admitted(&caller, BACKUP_DELETE_ID, request.request_id, &request, || {
            if !confirm() { return Err("project_backup_delete_cancelled".into()); }
            let prepared = prepare(directory, &request)?;
            // Query -> coordinator is the established lock order. Recheck the
            // issued process/window session after slow preparation, before
            // acquiring publication/admission/coordinator locks.
            query.validate_project_mutation_fence_window(label, &request.expected_fence, incarnation)
                .map_err(|_| "project_backup_delete_scope_retired_during_preparation")?;
            let _publication = state.project_save_publication.lock()
                .map_err(|_| "project_backup_delete_publication_poisoned")?;
            let _admission = super::lock_project_external_command_admission(state)?;
            let mut coordinator = super::lock_project_coordinator(state)?;
            super::reconcile_project_checkpoint_for_coordinator(state, &mut coordinator)?;
            if super::project_transaction_owner_binding_for_window(state, label, &owner)? != incarnation {
                return Err("project_backup_delete_owner_changed".into());
            }
            let fence = &request.expected_fence;
            if (coordinator.epoch, coordinator.revision, coordinator.checkpoint_hash.as_str(),
                coordinator.publication_generation) != (fence.project_epoch, fence.project_revision,
                fence.project_checkpoint_hash.as_str(), fence.project_publication_generation) {
                return Err("project_backup_delete_stale_fence".into());
            }
            super::ensure_no_pending_project_transaction(&coordinator)?;
            super::ensure_project_operator_authoritative_mutation_allowed(state, &coordinator, &owner)?;
            super::ensure_project_backup_not_claimed(state, request.backup_id)?;
            super::ensure_project_backup_not_durably_referenced_v1(journal, directory, request.backup_id)?;
            let receipt = Receipt { schema_version: 1, request: request.clone(),
                deleted_backup: prepared.inspection.clone() };
            receipt.validate().map_err(str::to_string)?;
            authorize()?;
            commit(prepared)?;
            Ok(receipt)
        })
    })
}

fn execute(app: &AppHandle, window: &WebviewWindow, caller: &str, request: Request,
    authorize: impl Fn() -> Result<(), String>, confirm: impl FnOnce() -> bool,
) -> Result<Receipt, String> {
    execute_core(&app.state::<AppState>(), &app.state::<ControlPlaneQueryState>(),
        &app.state::<BackupDeletionControlPlaneState>(), &app.state::<ProjectFileControlPlaneState>(),
        window.label(), caller, request,
        &super::app_data_subdirectory_path(app, super::PROJECT_BACKUP_DIRECTORY)?,
        &super::project_recovery_authority_state_path(app)?, authorize, confirm)
}

pub(crate) fn execute_local(app: &AppHandle, window: &WebviewWindow, request: Request) -> Result<Receipt, String> {
    use rfd::{MessageDialog, MessageButtons, MessageDialogResult, MessageLevel};
    let description = format!("Delete managed backup {} with its verified artifact identity?", request.backup_id);
    execute(app, window, "local", request, || Ok(()), || matches!(
        MessageDialog::new().set_level(MessageLevel::Warning).set_title("Delete project backup")
        .set_description(description).set_buttons(MessageButtons::YesNo).set_parent(window).show(),
        MessageDialogResult::Yes))
}
pub(crate) fn execute_external(app: &AppHandle, window: &WebviewWindow,
    dispatch: &AgentBridgeDispatch, request: Request,
) -> Result<Receipt, String> {
    if dispatch.method != "control_plane.execute"
        || dispatch.params.get("operationId").and_then(|v| v.as_str()) != Some(BACKUP_DELETE_ID) {
        return Err("project_backup_delete_immutable_operation_mismatch".into());
    }
    let immutable: Request = serde_json::from_value(dispatch.params["request"]["request"].clone())
        .map_err(|_| "project_backup_delete_immutable_request_invalid")?;
    if immutable != request { return Err("project_backup_delete_immutable_request_mismatch".into()); }
    let caller = format!("external:{}:{}", dispatch.principal_id, dispatch.principal_incarnation);
    execute(app, window, &caller, request, || app.state::<super::agent_bridge::AgentBridge>()
        .authority("main")?.authorize_bridge_request(&dispatch.principal_id, dispatch.principal_incarnation,
            &dispatch.method, &dispatch.params), || true)
}
