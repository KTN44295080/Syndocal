//! Typed Timeline execution from the immutable authenticated broker dispatch.
//! The GUI and this adapter share domain fences, receipts and publication code.
use protocol::control_plane_command::{
    RuntimeCommandErrorCodeV1, RuntimeCommandRequestV1, RuntimeCommandResponseV1,
    TimelineLoopRuntimeRequestV1, TimelineLoopRuntimeResponseV1,
    TimelineFollowAbortRuntimeRequestV1, TimelineFollowAbortRuntimeResponseV1,
    TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID, TIMELINE_LOOP_RUNTIME_OPERATION_ID,
    TIMELINE_FOLLOW_ABORT_OPERATION_ID,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::Manager;
use super::{agent_bridge::{AgentBridge, AgentBridgeDispatch}, native_adapter_error::NativeAdapterError,
    AppState, ControlPlaneQueryState};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ingress<T> { request: T }

pub(crate) fn supports(operation: &str) -> bool {
    matches!(operation, TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID
        | TIMELINE_LOOP_RUNTIME_OPERATION_ID | TIMELINE_FOLLOW_ABORT_OPERATION_ID)
}

fn decode<T: serde::de::DeserializeOwned>(dispatch: &AgentBridgeDispatch) -> Result<T, NativeAdapterError> {
    let ingress: Ingress<T> = serde_json::from_value(dispatch.params.get("request").cloned()
        .ok_or("agent_bridge_arguments_invalid")?).map_err(|_| "agent_bridge_arguments_invalid")?;
    Ok(ingress.request)
}

fn result<T: Serialize>(operation: &str, value: &T, accepted: bool) -> Result<Value, NativeAdapterError> {
    let value=serde_json::to_value(value).map_err(|_| "agent_runtime_response_invalid")?;
    Ok(json!({"ok":accepted,"operation_id":operation,"result":value}))
}

pub(crate) fn execute(app: &tauri::AppHandle, window: &tauri::WebviewWindow,
    dispatch: &AgentBridgeDispatch) -> Result<Value, NativeAdapterError> {
    let operation=dispatch.params.get("operationId").and_then(Value::as_str)
        .ok_or("agent_bridge_operation_invalid")?;
    if dispatch.method!="control_plane.execute" || !supports(operation) {
        return Err("native_operation_not_supported".into());
    }
    let state=app.state::<AppState>();let query=app.state::<ControlPlaneQueryState>();
    let authorize=|| app.state::<AgentBridge>().authority("main")
        .and_then(|authority|authority.authorize_bridge_request(&dispatch.principal_id,
            dispatch.principal_incarnation,&dispatch.method,&dispatch.params,&dispatch.request_id))
        .map_err(|code|if matches!(code.as_str(),"agent_authority_state_poisoned"|"agent_credential_store_poisoned") {RuntimeCommandErrorCodeV1::Internal}
            else {RuntimeCommandErrorCodeV1::Forbidden});
    match operation {
        TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID => {
            let request: RuntimeCommandRequestV1=decode(dispatch)?;
            if request.operation_id!=operation {return Err("agent_bridge_operation_identity_mismatch".into());}
            let response=super::control_plane_runtime::set_timeline_transport_playing_authorized(
                window.label(),&state,&query,request,&authorize);
            result(operation,&response,matches!(&response,RuntimeCommandResponseV1::Receipt(_)))
        }
        TIMELINE_LOOP_RUNTIME_OPERATION_ID => {
            let request: TimelineLoopRuntimeRequestV1=decode(dispatch)?;
            if request.operation_id!=operation {return Err("agent_bridge_operation_identity_mismatch".into());}
            let response=super::control_plane_runtime::commit_timeline_loop_runtime_authorized(
                window.label(),&state,&query,request,&authorize);
            result(operation,&response,matches!(&response,TimelineLoopRuntimeResponseV1::Receipt(_)))
        }
        TIMELINE_FOLLOW_ABORT_OPERATION_ID => {
            let request: TimelineFollowAbortRuntimeRequestV1=decode(dispatch)?;
            if request.operation_id!=operation {return Err("agent_bridge_operation_identity_mismatch".into());}
            let response=super::control_plane_runtime::abort_timeline_follow_runtime_authorized(
                window.label(),&state,&query,request,&authorize);
            result(operation,&response,matches!(&response,TimelineFollowAbortRuntimeResponseV1::Receipt(_)))
        }
        _ => Err("native_operation_not_supported".into()),
    }
}
