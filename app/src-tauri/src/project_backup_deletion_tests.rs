#[cfg(windows)]
mod project_backup_deletion_contract_tests {
    use super::*;
    use crate::project_backup_deletion::{execute_core, BackupDeletionControlPlaneState};
    use crate::project_file_control_plane::ProjectFileControlPlaneState;
    use protocol::control_plane_file::{ProjectBackupDeleteRequestV1 as Request, BACKUP_DELETE_ID};
    use sha2::{Digest,Sha256};
    const LABEL: &str = "media-asset-a6";
    fn authority(state:&AppState)->serde_json::Value {
        let coordinator=state.project_coordinator.lock().unwrap();
        let mut value=serde_json::to_value(project_authority_bundle_from_coordinator(state,&coordinator)).unwrap();
        value.as_object_mut().unwrap().remove("snapshot");
        value
    }
    fn fixture(label:&str) -> (PathBuf, Vec<u8>) {
        let root=unique_test_directory(label);
        fs::create_dir(&root).unwrap();
        let envelope=ProjectBackupEnvelope {version:PROJECT_BACKUP_VERSION,app:APP_NAME.into(),id:1,
            created_at_unix_ms:1,source_path:None,reason:"delete test".into(),
            project:serde_json::from_str(PHASE1_SAMPLE_PROJECT_JSON).unwrap(),midi_mappings:Vec::new(),
            osc_mappings:Vec::new(),dmx_mappings:Vec::new(),dj_track_triggers:Vec::new()};
        let bytes=crate::project_backup_json::project_backup_json_bytes(&envelope).unwrap();
        fs::write(root.join("backup-1.json"),&bytes).unwrap();(root,bytes)
    }
    fn request(query:&ControlPlaneQueryState,state:&AppState,bytes:&[u8],id:u64)->Request {
        Request {schema_version:1,operation_id:BACKUP_DELETE_ID.into(),request_id:id,backup_id:1,
            expected_fence:query.issue_project_mutation_fence_for_window(LABEL,state).unwrap(),
            expected_artifact_sha256:format!("{:x}",Sha256::digest(bytes))}
    }
    #[test]
    fn project_backup_deletion_exact_receipt_replay_never_deletes_recreated_artifact() {
        let harness=MediaAssetA6CommandHarness::new();let query=ControlPlaneQueryState::new().unwrap();
        let control=BackupDeletionControlPlaneState::default();let admission=ProjectFileControlPlaneState::default();
        let (root,bytes)=fixture("backup-delete-replay");let journal=root.join("journal.json");
        let request=request(&query,&harness.state,&bytes,1);
        let before=authority(&harness.state);
        let result=execute_core(&harness.state,&query,&control,&admission,LABEL,"one",request.clone(),
            &root,&journal,||Ok(()),||true).unwrap();
        assert!(!root.join("backup-1.json").exists());assert!(!journal.exists());
        assert_eq!(before,authority(&harness.state));
        fs::write(root.join("backup-1.json"),b"recreated untouched").unwrap();
        let replay=execute_core(&harness.state,&query,&control,&admission,LABEL,"one",request.clone(),
            &root,&journal,||Ok(()),||panic!("Replay cannot confirm")).unwrap();
        assert_eq!(serde_json::to_value(result).unwrap(),serde_json::to_value(replay).unwrap());
        assert_eq!(fs::read(root.join("backup-1.json")).unwrap(),b"recreated untouched");
        let mut changed=request;changed.expected_artifact_sha256="d".repeat(64);
        assert!(execute_core(&harness.state,&query,&control,&admission,LABEL,"one",changed,
            &root,&journal,||Ok(()),||panic!("Changed shape cannot confirm")).unwrap_err().contains("different shape"));
        fs::remove_file(root.join("backup-1.json")).unwrap();fs::remove_dir(root).unwrap();
    }
    #[test]
    fn project_backup_deletion_wrong_hash_writer_unissued_stale_and_final_revocation_preserve_bytes() {
        let harness=MediaAssetA6CommandHarness::new();let query=ControlPlaneQueryState::new().unwrap();
        let control=BackupDeletionControlPlaneState::default();let admission=ProjectFileControlPlaneState::default();
        let (root,bytes)=fixture("backup-delete-reject");let journal=root.join("journal.json");
        let mut wrong=request(&query,&harness.state,&bytes,1);wrong.expected_artifact_sha256="f".repeat(64);
        assert!(execute_core(&harness.state,&query,&control,&admission,LABEL,"one",wrong,
            &root,&journal,||Ok(()),||true).unwrap_err().contains("artifact_changed"));
        let writer=fs::OpenOptions::new().write(true).open(root.join("backup-1.json")).unwrap();
        assert!(execute_core(&harness.state,&query,&control,&admission,LABEL,"one",request(&query,&harness.state,&bytes,2),
            &root,&journal,||Ok(()),||true).unwrap_err().contains("delete_open"));drop(writer);
        let mut forged=request(&query,&harness.state,&bytes,3);forged.expected_fence.session_incarnation+=1;
        assert!(execute_core(&harness.state,&query,&control,&admission,LABEL,"one",forged,
            &root,&journal,||Ok(()),||panic!("Unissued cannot confirm")).unwrap_err().contains("unissued_fence"));
        let calls=std::cell::Cell::new(0);
        assert!(execute_core(&harness.state,&query,&control,&admission,LABEL,"one",request(&query,&harness.state,&bytes,4),
            &root,&journal,||{calls.set(calls.get()+1);if calls.get()>1 {Err("revoked at final effect".into())}else{Ok(())}},
            ||true).unwrap_err().contains("revoked at final effect"));
        let stale=request(&query,&harness.state,&bytes,5);
        assert!(execute_core(&harness.state,&query,&control,&admission,LABEL,"one",stale,
            &root,&journal,||Ok(()),||{harness.state.project_coordinator.lock().unwrap().publication_generation+=1;true})
            .unwrap_err().contains("stale_fence"));
        assert_eq!(fs::read(root.join("backup-1.json")).unwrap(),bytes);assert!(!journal.exists());
        fs::remove_file(root.join("backup-1.json")).unwrap();fs::remove_dir(root).unwrap();
    }
    #[test]
    fn project_backup_deletion_retired_query_session_during_preparation_cannot_delete() {
        let harness=MediaAssetA6CommandHarness::new();let query=ControlPlaneQueryState::new().unwrap();
        let control=BackupDeletionControlPlaneState::default();let admission=ProjectFileControlPlaneState::default();
        let (root,bytes)=fixture("backup-delete-session");let journal=root.join("journal.json");
        let request=request(&query,&harness.state,&bytes,1);
        assert!(execute_core(&harness.state,&query,&control,&admission,LABEL,"one",request,
            &root,&journal,||Ok(()),||{query.retire_window(LABEL).unwrap();true})
            .unwrap_err().contains("scope_retired_during_preparation"));
        assert_eq!(fs::read(root.join("backup-1.json")).unwrap(),bytes);assert!(!journal.exists());
        fs::remove_file(root.join("backup-1.json")).unwrap();fs::remove_dir(root).unwrap();
    }
    #[test]
    fn project_backup_deletion_update_claim_and_normalized_pending_receipt_protect_artifact() {
        let harness=MediaAssetA6CommandHarness::new();let query=ControlPlaneQueryState::new().unwrap();
        let control=BackupDeletionControlPlaneState::default();let admission=ProjectFileControlPlaneState::default();
        let (root,bytes)=fixture("backup-delete-reference");let journal=root.join("journal.json");
        *harness.state.application_update_publication_claim.lock().unwrap()=Some(ApplicationUpdatePublicationClaim {
            origin_id:"test".into(),request_id:1,shape_hash:"a".repeat(64),backup_id:1});
        assert!(execute_core(&harness.state,&query,&control,&admission,LABEL,"one",request(&query,&harness.state,&bytes,1),
            &root,&journal,||Ok(()),||true).unwrap_err().contains("still using"));
        *harness.state.application_update_publication_claim.lock().unwrap()=None;
        let publication=project_publication_request_for_test(ProjectPublicationSurfaceV1::Backup,"pending-delete",1,
            None,Some("autosave".into()));
        let shape=project_publication_shape_hash_v1(&publication).unwrap();
        let target=normalized_recovery_target_key(&root.join("backup-1.json")).unwrap();
        assert_ne!(target,root.join("backup-1.json"),"Windows fixture must exercise normalized spelling");
        let durable=PersistedProjectRecoveryAuthorityState {version:PROJECT_RECOVERY_AUTHORITY_STATE_VERSION,
            serial:3,last_transition:ProjectRecoveryAuthorityTransition::ProjectPublication,pending_clean_save:None,
            publication_journal:PersistedProjectPublicationJournalV1 {
                origins:vec![PersistedProjectPublicationOriginV1 {origin_id:publication.origin_id.clone(),
                    high_water_request_id:1,acknowledged_request_id:0,acknowledged_shape_hash:None}],
                terminals:Vec::new(),latest_reservation_generation:1,
                pending:vec![PersistedProjectPublicationPendingV1 {request:publication,shape_hash:shape,
                    surface:ProjectPublicationSurfaceV1::Backup,project_epoch:7,project_revision:11,
                    checkpoint_hash:"a".repeat(64),path_generation:0,authority_disposition_generation:0,
                    recovery_authority_serial_before:3,expected_recovery_authority_serial_after:None,
                    reservation_generation:1,source_path:None,reason:Some("autosave".into()),
                    phase:ProjectPublicationPendingPhaseV1::Selected,target_path:Some(target),
                    staging_path:None,prepared_digest:None,indeterminate_error:None}]}};
        persist_project_recovery_authority_state_to_path(&journal,&durable).unwrap();
        let journal_bytes=fs::read(&journal).unwrap();
        assert!(execute_core(&harness.state,&query,&control,&admission,LABEL,"one",request(&query,&harness.state,&bytes,2),
            &root,&journal,||Ok(()),||true).unwrap_err().contains("unacknowledged"));
        assert_eq!(fs::read(&journal).unwrap(),journal_bytes);assert_eq!(fs::read(root.join("backup-1.json")).unwrap(),bytes);
        fs::remove_file(root.join("backup-1.json")).unwrap();fs::remove_file(journal).unwrap();fs::remove_dir(root).unwrap();
    }
}
