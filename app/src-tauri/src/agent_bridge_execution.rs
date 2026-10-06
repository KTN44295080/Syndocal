//! Native execution of authenticated, immutable external MCP requests.
//! No renderer argument can select a different operation or skip admission.
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tauri::Manager;

use super::native_adapter_error::NativeAdapterError;
use super::{agent_bridge::AgentBridge, AppState, ControlPlaneQueryState};
use protocol::control_plane_command::OutputControlCommandRequestV2;
use protocol::control_plane_project::{
    ProjectReplacementRequestV1, ProjectReplacementResponseV1, PROJECT_BACKUP_RESTORE_OPERATION_ID,
    PROJECT_NEW_OPERATION_ID, PROJECT_OPEN_OPERATION_ID,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectReplacementIngress {
    request: ProjectReplacementRequestV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BackupDeleteIngress {
    request: protocol::control_plane_file::ProjectBackupDeleteRequestV1,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BackupJournalIngress {
    request: protocol::control_plane_backup_management::JournalQueryRequestV1,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BackupManagementIngress {
    request: protocol::control_plane_backup_management::ManagementRequestV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OutputIngress {
    request: OutputControlCommandRequestV2,
}

use super::diagnostic_audit::ExportRequest as DiagnosticExport;

// An observational capture may outlive the original grant. Never publish or
// retry when the exact immutable request is no longer authorized afterwards.
fn diagnostic_capture_authorize_publish<T, R>(
    capture: impl FnOnce() -> Result<T, String>,
    authorize: impl FnOnce() -> Result<(), String>,
    publish: impl FnOnce(T) -> Result<R, String>,
) -> Result<R, String> {
    let captured = capture()?;
    authorize()?;
    publish(captured)
}

pub(crate) fn execute(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
    renderer_generation: u64,
    request_id: &str,
) -> Result<Value, NativeAdapterError> {
    let bridge = app.state::<AgentBridge>();
    let dispatch =
        bridge.start_native_execution(window.label(), renderer_generation, request_id)?;
    let state = app.state::<AppState>();
    match dispatch.method.as_str() {
        "control_plane.execute" => {
            let operation_id = dispatch
                .params
                .get("operationId")
                .and_then(Value::as_str)
                .ok_or("agent_bridge_operation_invalid")?;
            if operation_id == protocol::control_plane_backup_management::QUERY_ID {
                let ingress: BackupJournalIngress = serde_json::from_value(
                    dispatch
                        .params
                        .get("request")
                        .cloned()
                        .ok_or("agent_bridge_arguments_invalid")?,
                )
                .map_err(|_| "agent_bridge_arguments_invalid")?;
                let result = serde_json::to_value(
                    super::project_backup_deletion_management::query_external(
                        app,
                        window,
                        &dispatch,
                        ingress.request,
                    )?,
                )
                .map_err(|_| "project_backup_delete_journal_response_invalid")?;
                return Ok(json!({"ok":true,"operation_id":operation_id,"result":result}));
            }
            if matches!(
                operation_id,
                protocol::control_plane_backup_management::MANAGE_ID
                    | protocol::control_plane_backup_management::STATUS_ID
            ) {
                let ingress: BackupManagementIngress = serde_json::from_value(
                    dispatch
                        .params
                        .get("request")
                        .cloned()
                        .ok_or("agent_bridge_arguments_invalid")?,
                )
                .map_err(|_| "agent_bridge_arguments_invalid")?;
                let result =
                    if operation_id == protocol::control_plane_backup_management::STATUS_ID {
                        serde_json::to_value(
                            super::project_backup_deletion_management::status_external(
                                app,
                                window,
                                &dispatch,
                                ingress.request,
                            )?,
                        )
                    } else {
                        serde_json::to_value(
                            super::project_backup_deletion_management::execute_external(
                                app,
                                window,
                                &dispatch,
                                ingress.request,
                            )?,
                        )
                    }
                    .map_err(|_| "project_backup_delete_management_response_invalid")?;
                return Ok(json!({"ok":true,"operation_id":operation_id,"result":result}));
            }
            if matches!(
                operation_id,
                protocol::control_plane_file::BACKUP_DELETE_ID
                    | protocol::control_plane_file::BACKUP_DELETE_STATUS_ID
            ) {
                let ingress: BackupDeleteIngress = serde_json::from_value(
                    dispatch
                        .params
                        .get("request")
                        .cloned()
                        .ok_or("agent_bridge_arguments_invalid")?,
                )
                .map_err(|_| "agent_bridge_arguments_invalid")?;
                let result =
                    if operation_id == protocol::control_plane_file::BACKUP_DELETE_STATUS_ID {
                        serde_json::to_value(super::project_backup_deletion::status_external(
                            app,
                            window,
                            &dispatch,
                            ingress.request,
                        )?)
                    } else {
                        serde_json::to_value(super::project_backup_deletion::execute_external(
                            app,
                            window,
                            &dispatch,
                            ingress.request,
                        )?)
                    }
                    .map_err(|_| "project_backup_delete_response_invalid")?;
                return Ok(json!({"ok": true, "operation_id":operation_id, "result":result}));
            }
            if matches!(
                operation_id,
                protocol::control_plane_file::SAVE_ID
                    | protocol::control_plane_file::SAVE_AS_ID
                    | protocol::control_plane_file::TEMPLATE_ID
                    | protocol::control_plane_file::AUTHORITY_ID
                    | protocol::control_plane_file::STATUS_ID
                    | protocol::control_plane_file::ACK_ID
                    | protocol::control_plane_file::BACKUP_ID
                    | protocol::control_plane_file::BACKUP_AUTHORITY_ID
                    | protocol::control_plane_file::BACKUP_INSPECT_ID
                    | protocol::control_plane_file::BACKUP_LIST_ID
            ) {
                let result =
                    super::project_file_control_plane::execute_external(app, window, &dispatch)?;
                return Ok(json!({"ok": true, "operation_id":operation_id, "result":result}));
            }
            if matches!(
                operation_id,
                PROJECT_NEW_OPERATION_ID
                    | PROJECT_OPEN_OPERATION_ID
                    | PROJECT_BACKUP_RESTORE_OPERATION_ID
            ) {
                let ingress: ProjectReplacementIngress = serde_json::from_value(
                    dispatch
                        .params
                        .get("request")
                        .cloned()
                        .ok_or("agent_bridge_arguments_invalid")?,
                )
                .map_err(|_| "agent_bridge_arguments_invalid")?;
                if ingress.request.operation_id != operation_id {
                    return Err("agent_bridge_operation_identity_mismatch".into());
                }
                let result = super::project_replacement_control_plane::execute_external(
                    app,
                    window,
                    &dispatch,
                    ingress.request,
                )?;
                let ok = matches!(&result, ProjectReplacementResponseV1::Receipt(_));
                let result =
                    serde_json::to_value(&result).map_err(|_| "agent_project_response_invalid")?;
                return Ok(json!({"ok": ok, "operation_id": operation_id, "result": result}));
            }
            if !operation_id.starts_with("syndocal.output.") {
                return Err("native_operation_not_supported".into());
            }
            let ingress: OutputIngress = serde_json::from_value(
                dispatch
                    .params
                    .get("request")
                    .cloned()
                    .ok_or("agent_bridge_arguments_invalid")?,
            )
            .map_err(|_| "agent_bridge_arguments_invalid")?;
            if ingress.request.operation_id != operation_id
                || ingress.request.action.operation_id() != operation_id
            {
                return Err("agent_bridge_operation_identity_mismatch".into());
            }
            let query_state = app.state::<ControlPlaneQueryState>();
            let result = super::control_plane_runtime::execute_external_output_control(
                app,
                window,
                &state,
                &query_state,
                ingress.request,
            );
            let ok = matches!(
                &result,
                protocol::control_plane_command::OutputControlResponseV2::Receipt(_)
            );
            // A domain serialization failure is an unconfirmed mutation,
            // never a json! panic or permission to automatically replay it.
            let result =
                serde_json::to_value(&result).map_err(|_| "agent_output_response_invalid")?;
            Ok(json!({"ok": ok, "operation_id": operation_id, "result": result}))
        }
        "diagnostics.export" => {
            let request: DiagnosticExport = serde_json::from_value(dispatch.params.clone())
                .map_err(|_| "agent_bridge_arguments_invalid")?;
            let destination = std::path::PathBuf::from(request.destination);
            super::diagnostic_package_publication::require_new_target(&destination)?;
            diagnostic_capture_authorize_publish(
                ||super::capture_diagnostic_package(app,&state,&request.audit_before.unwrap_or_default(),request.expected_process_incarnation),
                ||app.state::<AgentBridge>().authority("main")?.authorize_bridge_request(
                    &dispatch.principal_id,dispatch.principal_incarnation,&dispatch.method,&dispatch.params),
                |(_,bytes,audit)| {
                    let sha256=format!("{:x}",Sha256::digest(&bytes));let size=bytes.len();
                    super::diagnostic_package_publication::publish_new_diagnostic_package(&destination,&bytes)
                        .map_err(|error|error.to_string())?;
                    Ok(json!({"ok":true,"operation_id":"syndocal.diagnostics.export.v1", "destination":destination,
                        "sha256":sha256,"bytes":size,"format_version":2,"audit":audit}))
                },
            ).map_err(Into::into)
        }
        _ => Err("native_operation_not_supported".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostic_capture_revocation_blocks_publication_without_replaying_capture() {
        use std::cell::Cell;
        let grant = Cell::new(true);
        let captures = Cell::new(0);
        let publications = Cell::new(0);
        let result = diagnostic_capture_authorize_publish(
            || {
                captures.set(captures.get() + 1);
                grant.set(false);
                Ok(vec![1, 2, 3])
            },
            || {
                if grant.get() {
                    Ok(())
                } else {
                    Err("agent_principal_revoked".into())
                }
            },
            |_| {
                publications.set(publications.get() + 1);
                Ok(())
            },
        );
        assert_eq!(result, Err("agent_principal_revoked".into()));
        assert_eq!(captures.get(), 1);
        assert_eq!(publications.get(), 0);
        let calls = std::cell::RefCell::new(Vec::new());
        let successful = diagnostic_capture_authorize_publish(
            || {
                calls.borrow_mut().push("capture");
                Ok(vec![7])
            },
            || {
                calls.borrow_mut().push("authorize");
                Ok(())
            },
            |bytes| {
                calls.borrow_mut().push("publish");
                Ok(bytes)
            },
        )
        .unwrap();
        assert_eq!(successful, vec![7]);
        assert_eq!(*calls.borrow(), ["capture", "authorize", "publish"]);
    }
}
