//! Explicit management of bounded deletion facts. This module never deletes a
//! backup and never turns an unresolved original effect into a success claim.
use std::{fs, io::Read, path::Path, time::Instant};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager, WebviewWindow};
use protocol::control_plane_backup_management::{self as wire, ArtifactObservationV1,
    ManagementRequestV1 as Request, ManagementReceiptV1 as Receipt,
    JournalQueryRequestV1, JournalViewV1};
use super::{AppState, ControlPlaneQueryState, agent_bridge::AgentBridgeDispatch,
    authored_control_plane::GenericAuthoredMutationReceiptKey,
    project_backup_deletion::BackupDeletionControlPlaneState,
    project_file_control_plane::ProjectFileControlPlaneState,
    native_adapter_error::NativeAdapterError, project_backup_deletion_journal as durable};

struct Identity { owner:String, incarnation:u64, caller:String, origin:String }
fn identity(state:&AppState,label:&str,caller:&str,process:u64)->Result<Identity,String> {
    let owner=super::current_project_transaction_owner_for_window(state,label)?;
    let incarnation=super::project_transaction_owner_binding_for_window(state,label,&owner)?;
    let caller=format!("{label}:{incarnation}:{caller}");
    let origin=durable::origin(&format!("deletion-management-v1:{process}:{caller}"),&owner)?;
    Ok(Identity {owner,incarnation,caller,origin})
}
fn validate_owner(state:&AppState,label:&str,actor:&Identity)->Result<(),String> {
    if super::project_transaction_owner_binding_for_window(state,label,&actor.owner)?!=actor.incarnation {
        return Err("project_backup_delete_management_owner_changed".into());
    }
    Ok(())
}
fn query_lock(state:&AppState)->Result<std::sync::MutexGuard<'_,()>,NativeAdapterError> {
    use std::sync::TryLockError;
    use protocol::control_plane_query::{QueryError,QueryErrorCode};
    match state.project_save_publication.try_lock() {
        Ok(lock)=>Ok(lock),
        Err(TryLockError::WouldBlock)=>Err(QueryError::from_code(QueryErrorCode::Overloaded).into()),
        Err(TryLockError::Poisoned(_))=>Err(QueryError::from_code(QueryErrorCode::Internal).into()),
    }
}
pub(super) fn query_core(state:&AppState,query:&ControlPlaneQueryState,label:&str,caller:&str,
    request:JournalQueryRequestV1,recovery:&Path,authorize:impl Fn()->Result<(),String>)
    ->Result<JournalViewV1,NativeAdapterError> {
    request.validate().map_err(str::to_string)?;authorize()?;
    let actor=identity(state,label,caller,query.process_incarnation())?;
    let _publication=query_lock(state)?;
    let observation=durable::observe(&durable::path_for_recovery_journal(recovery))?;
    let value=observation.journal.view(&request,&observation.sha256,query.process_incarnation())?;
    validate_owner(state,label,&actor)?;authorize()?;Ok(value)
}
pub(super) fn status_core(state:&AppState,query:&ControlPlaneQueryState,label:&str,caller:&str,
    request:Request,recovery:&Path,authorize:impl Fn()->Result<(),String>)
    ->Result<Option<Receipt>,NativeAdapterError> {
    request.validate().map_err(str::to_string)?;authorize()?;
    let actor=identity(state,label,caller,query.process_incarnation())?;
    let _publication=query_lock(state)?;
    let value=durable::load(&durable::path_for_recovery_journal(recovery))?.management_status(&actor.origin,&request)?;
    validate_owner(state,label,&actor)?;authorize()?;Ok(value)
}

struct PreparedObservation { value:ArtifactObservationV1, _file:Option<fs::File> }
fn regular(metadata:&fs::Metadata)->bool {
    #[cfg(windows)]
    {use std::os::windows::fs::MetadataExt;
     metadata.is_file()&&!metadata.file_type().is_symlink()
        &&metadata.file_attributes()&windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT.0==0}
    #[cfg(not(windows))]
    {metadata.is_file()&&!metadata.file_type().is_symlink()}
}
fn prepare_observation(directory:&Path,id:u64)->Result<PreparedObservation,String> {
    let path=super::project_file_managed_backup::path_for_id(directory,id)?;
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind()==std::io::ErrorKind::NotFound=>return Ok(PreparedObservation {value:ArtifactObservationV1::Missing {},_file:None}),
        Err(error)=>return Err(format!("project_backup_delete_management_artifact_metadata: {error}")),
        Ok(metadata) if !regular(&metadata)=>return Err("project_backup_delete_management_artifact_not_regular".into()),
        Ok(_)=>{},
    }
    let mut options=fs::OpenOptions::new();options.read(true);
    #[cfg(windows)]
    {use std::os::windows::fs::OpenOptionsExt;
     options.share_mode(0).custom_flags(windows::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT.0);}
    let file=options.open(&path).map_err(|error|format!("project_backup_delete_management_artifact_open: {error}"))?;
    let metadata=file.metadata().map_err(|error|error.to_string())?;
    if !regular(&metadata)||metadata.len()>128*1024*1024 {return Err("project_backup_delete_management_artifact_not_bounded_regular".into());}
    // Hash a bounded streaming read; no decoded project/frame copy is needed
    // to preserve an artifact, including a damaged but regular managed leaf.
    let mut reader=(&file).take(128*1024*1024+1);let mut buffer=[0u8;64*1024];
    let mut hash=Sha256::new();let mut bytes=0;
    loop {let count=reader.read(&mut buffer).map_err(|error|error.to_string())?;if count==0{break;}
        bytes+=count as u64;hash.update(&buffer[..count]);}
    if bytes!=metadata.len()||file.metadata().map_err(|error|error.to_string())?.len()!=bytes {
        return Err("project_backup_delete_management_artifact_size_changed".into());
    }
    Ok(PreparedObservation {value:ArtifactObservationV1::Present {sha256:format!("{hash:x}",hash=hash.finalize())},_file:Some(file)})
}

pub(super) fn execute_core(state:&AppState,query:&ControlPlaneQueryState,
    control:&BackupDeletionControlPlaneState,admission:&ProjectFileControlPlaneState,
    label:&str,caller:&str,request:Request,directory:&Path,recovery:&Path,
    authorize:impl Fn()->Result<(),String>,confirm:impl FnOnce()->bool)->Result<Receipt,String> {
    request.validate().map_err(str::to_string)?;authorize()?;
    let actor=identity(state,label,caller,query.process_incarnation())?;
    let path=durable::path_for_recovery_journal(recovery);
    let initial={
        let _publication=state.project_save_publication.lock().map_err(|_|"project_backup_delete_management_publication_poisoned")?;
        let observation=durable::observe(&path)?;
        if let Some(receipt)=observation.journal.management_status(&actor.origin,&request)? {
            validate_owner(state,label,&actor)?;authorize()?;return Ok(receipt);
        }
        validate_owner(state,label,&actor)?;observation
    };
    let key=GenericAuthoredMutationReceiptKey {operation_id:wire::MANAGE_ID.into(),
        request_id:format!("backup-delete-management:{}:{}:{}",query.process_incarnation(),actor.caller,request.request_id),
        window_label:label.into(),owner_id:actor.owner.clone(),owner_incarnation:actor.incarnation,
        start_epoch:0,start_revision:0,start_checkpoint_hash:String::new()};
    let shape=format!("{:x}",Sha256::digest(serde_json::to_vec(&request).map_err(|_|"project_backup_delete_management_encoding")?));
    control.receipts.generic_terminal_single_flight(key,shape,Instant::now(),||{
        query.validate_project_mutation_fence_window(label,&request.expected_fence,actor.incarnation)
            .map_err(|_|"project_backup_delete_management_unissued_fence")?;
        admission.admitted(&actor.caller,wire::MANAGE_ID,request.request_id,&request,||{
            if initial.sha256!=request.expected_journal_sha256||initial.journal.generation()!=request.expected_generation {
                return Err("project_backup_delete_management_snapshot_changed".into());
            }
            if !confirm() {return Err("project_backup_delete_management_cancelled".into());}
            let backup_id=initial.journal.release_backup_id(&request.action)?;
            let prepared=backup_id.map(|id|prepare_observation(directory,id)).transpose()?;
            query.validate_project_mutation_fence_window(label,&request.expected_fence,actor.incarnation)
                .map_err(|_|"project_backup_delete_management_scope_retired_during_preparation")?;
            let _publication=state.project_save_publication.lock().map_err(|_|"project_backup_delete_management_publication_poisoned")?;
            let _admission=super::lock_project_external_command_admission(state)?;
            let mut coordinator=super::lock_project_coordinator(state)?;
            super::reconcile_project_checkpoint_for_coordinator(state,&mut coordinator)?;
            validate_owner(state,label,&actor)?;
            let fence=&request.expected_fence;
            if (coordinator.epoch,coordinator.revision,coordinator.checkpoint_hash.as_str(),coordinator.publication_generation)
                !=(fence.project_epoch,fence.project_revision,fence.project_checkpoint_hash.as_str(),fence.project_publication_generation) {
                return Err("project_backup_delete_management_stale_fence".into());
            }
            super::ensure_no_pending_project_transaction(&coordinator)?;
            super::ensure_project_operator_authoritative_mutation_allowed(state,&coordinator,&actor.owner)?;
            let current=durable::observe(&path)?;
            if current.sha256!=request.expected_journal_sha256||current.journal.generation()!=request.expected_generation {
                return Err("project_backup_delete_management_snapshot_changed".into());
            }
            if let Some(id)=backup_id {
                super::ensure_project_backup_not_claimed(state,id)?;
                if matches!(prepared.as_ref().map(|value|&value.value),Some(ArtifactObservationV1::Missing {})) {
                    // Absence is observational. Even an external OS race after
                    // this check cannot delete/overwrite any current bytes.
                    let final_observation=prepare_observation(directory,id)?;
                    if final_observation.value!=(ArtifactObservationV1::Missing {}) {
                        return Err("project_backup_delete_management_artifact_changed".into());
                    }
                }
            }
            let (next,receipt)=current.journal.managed(&actor.origin,&request,
                prepared.as_ref().map(|value|value.value.clone()),query.process_incarnation())?;
            authorize()?;
            durable::persist(&path,&next).map_err(|error|super::input_diagnostic::bounded_diagnostic(
                format_args!("project_backup_delete_management_indeterminate; query the original management request; do not repeat intent: {error}")))?;
            drop(prepared);Ok(receipt)
        })
    })
}

fn caller(dispatch:&AgentBridgeDispatch)->String {format!("external:{}:{}",dispatch.principal_id,dispatch.principal_incarnation)}
fn authorize(app:&AppHandle,dispatch:&AgentBridgeDispatch)->Result<(),String> {
    app.state::<super::agent_bridge::AgentBridge>().authority("main")?.authorize_bridge_request(
        &dispatch.principal_id,dispatch.principal_incarnation,&dispatch.method,&dispatch.params, &dispatch.request_id,)
}
fn immutable<T:serde::de::DeserializeOwned+PartialEq>(dispatch:&AgentBridgeDispatch,id:&str,request:&T)->Result<(),String> {
    if dispatch.method!="control_plane.execute"||dispatch.params.get("operationId").and_then(serde_json::Value::as_str)!=Some(id) {
        return Err("project_backup_delete_management_immutable_operation_mismatch".into());
    }
    let native:T=serde_json::from_value(dispatch.params["request"]["request"].clone())
        .map_err(|_|"project_backup_delete_management_immutable_request_invalid")?;
    if &native!=request {return Err("project_backup_delete_management_immutable_request_mismatch".into());}
    Ok(())
}
pub(crate) fn query_external(app:&AppHandle,window:&WebviewWindow,dispatch:&AgentBridgeDispatch,request:JournalQueryRequestV1)
    ->Result<JournalViewV1,NativeAdapterError> {
    immutable(dispatch,wire::QUERY_ID,&request)?;
    query_core(&app.state::<AppState>(),&app.state::<ControlPlaneQueryState>(),window.label(),&caller(dispatch),request,
        &super::project_recovery_authority_state_path(app)?,||authorize(app,dispatch))
}
pub(crate) fn query_local(app:&AppHandle,window:&WebviewWindow,request:JournalQueryRequestV1)->Result<JournalViewV1,NativeAdapterError> {
    query_core(&app.state::<AppState>(),&app.state::<ControlPlaneQueryState>(),window.label(),"local",request,
        &super::project_recovery_authority_state_path(app)?,||Ok(()))
}
pub(crate) fn status_external(app:&AppHandle,window:&WebviewWindow,dispatch:&AgentBridgeDispatch,request:Request)
    ->Result<Option<Receipt>,NativeAdapterError> {
    immutable(dispatch,wire::STATUS_ID,&request)?;
    status_core(&app.state::<AppState>(),&app.state::<ControlPlaneQueryState>(),window.label(),&caller(dispatch),request,
        &super::project_recovery_authority_state_path(app)?,||authorize(app,dispatch))
}
pub(crate) fn status_local(app:&AppHandle,window:&WebviewWindow,request:Request)->Result<Option<Receipt>,NativeAdapterError> {
    status_core(&app.state::<AppState>(),&app.state::<ControlPlaneQueryState>(),window.label(),"local",request,
        &super::project_recovery_authority_state_path(app)?,||Ok(()))
}
pub(crate) fn execute_external(app:&AppHandle,window:&WebviewWindow,dispatch:&AgentBridgeDispatch,request:Request)->Result<Receipt,String> {
    immutable(dispatch,wire::MANAGE_ID,&request)?;
    execute_core(&app.state::<AppState>(),&app.state::<ControlPlaneQueryState>(),&app.state::<BackupDeletionControlPlaneState>(),
        &app.state::<ProjectFileControlPlaneState>(),window.label(),&caller(dispatch),request,
        &super::app_data_subdirectory_path(app,super::PROJECT_BACKUP_DIRECTORY)?,&super::project_recovery_authority_state_path(app)?,
        ||authorize(app,dispatch),||true)
}
pub(crate) fn execute_local(app:&AppHandle,window:&WebviewWindow,request:Request)->Result<Receipt,String> {
    use rfd::{MessageDialog,MessageButtons,MessageDialogResult,MessageLevel};
    execute_core(&app.state::<AppState>(),&app.state::<ControlPlaneQueryState>(),&app.state::<BackupDeletionControlPlaneState>(),
        &app.state::<ProjectFileControlPlaneState>(),window.label(),"local",request,
        &super::app_data_subdirectory_path(app,super::PROJECT_BACKUP_DIRECTORY)?,&super::project_recovery_authority_state_path(app)?,||Ok(()),
        ||matches!(MessageDialog::new().set_level(MessageLevel::Warning).set_title("Manage deletion journal")
            .set_description("Apply the exact observed journal management action? Current backup bytes are preserved.")
            .set_buttons(MessageButtons::YesNo).set_parent(window).show(),MessageDialogResult::Yes))
}
