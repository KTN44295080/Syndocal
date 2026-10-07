//! Exercises the shared production command body with the real disarmed Engine.
use super::*;
use std::cell::Cell;
use protocol::control_plane_command::{SetTimelinePlayingRuntimePayloadV1,TimelineLoopRuntimeActionV1};

#[test]
fn external_transport_final_authorization_denial_prevents_publication_and_replay_reentry() {
    let harness=crate::tests::MediaAssetA6CommandHarness::new();
    let state=&harness.state;let query=ControlPlaneQueryState::new().unwrap();let label="media-asset-a6";
    let project=issue_runtime_authority_project_fence(state,&query,label).unwrap();
    let source=state.engine.timeline_transport_authority();
    let fence=TimelineTransportRuntimeFenceV1 {project,domain:TIMELINE_TRANSPORT_RUNTIME_DOMAIN_V1.into(),
        source_runtime_epoch:source.epoch,source_runtime_generation:source.generation};
    let authority_id=random_authority_id().unwrap();
    state.runtime_control_plane.issue_authority(capture_binding(state,label).unwrap(),fence.clone(),authority_id.clone(),Instant::now()).unwrap();
    let request=RuntimeCommandRequestV1 {operation_id:TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID.into(),
        authority_id,request_id:1,expected_fence:fence,payload:SetTimelinePlayingRuntimePayloadV1 {playing:true}};
    let before=state.engine.control_plane_runtime_snapshot();let output=state.engine.output_ownership_status();
    let calls=Cell::new(0);let authorize=||{calls.set(calls.get()+1);if calls.get()<3 {Ok(())}else{Err(RuntimeCommandErrorCodeV1::Forbidden)}};
    let response=set_timeline_transport_playing_authorized(label,state,&query,request.clone(),&authorize);
    assert_eq!(calls.get(),3,"denial occurs at the final coordinator boundary, after both admission checks");
    assert!(matches!(response,RuntimeCommandResponseV1::Rejected(ref r) if r.error.code==RuntimeCommandErrorCodeV1::Forbidden));
    assert_eq!(state.engine.control_plane_runtime_snapshot(),before);assert_eq!(state.engine.output_ownership_status(),output);
    let replay=set_timeline_transport_playing_authorized(label,state,&query,request,&||Ok(()));
    assert_eq!(replay,response,"a retained denied domain request cannot later publish under a renewed grant");
    assert_eq!(state.engine.control_plane_runtime_snapshot(),before);
}

#[test]
fn external_loop_final_authorization_denial_prevents_capability_consumption_and_publication() {
    let harness=crate::tests::MediaAssetA6CommandHarness::new();
    let state=&harness.state;let query=ControlPlaneQueryState::new().unwrap();let label="media-asset-a6";
    let project=issue_runtime_authority_project_fence(state,&query,label).unwrap();let source=state.engine.control_plane_runtime_snapshot();
    let fence=TimelineLoopRuntimeFenceV1 {project,domain:TIMELINE_LOOP_RUNTIME_DOMAIN_V1.into(),
        source_runtime_epoch:source.timeline_transport_epoch,source_runtime_generation:source.timeline_transport_generation,
        source_loop_generation:source.timeline_loop_generation,source_follow_generation:source.timeline_follow_generation};
    let authority_id=random_authority_id().unwrap();
    state.runtime_control_plane.issue_timeline_loop_authority(capture_binding(state,label).unwrap(),fence.clone(),authority_id.clone(),Instant::now()).unwrap();
    let request=TimelineLoopRuntimeRequestV1 {operation_id:TIMELINE_LOOP_RUNTIME_OPERATION_ID.into(),authority_id,request_id:1,
        expected_fence:fence,action:TimelineLoopRuntimeActionV1::SetEnabled {enabled:true}};
    let calls=Cell::new(0);let authorize=||{calls.set(calls.get()+1);if calls.get()<3 {Ok(())}else{Err(RuntimeCommandErrorCodeV1::Forbidden)}};
    let response=commit_timeline_loop_runtime_authorized(label,state,&query,request.clone(),&authorize);
    assert_eq!(calls.get(),3);assert!(matches!(response,TimelineLoopRuntimeResponseV1::Rejected(ref r) if r.error.code==RuntimeCommandErrorCodeV1::Forbidden));
    assert_eq!(state.engine.control_plane_runtime_snapshot(),source);
    assert_eq!(commit_timeline_loop_runtime_authorized(label,state,&query,request,&||Ok(())),response);
    assert_eq!(state.engine.control_plane_runtime_snapshot(),source);
}
