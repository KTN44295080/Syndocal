mod project_backup_restoration_contract_tests {
    use super::*;
    use crate::project_replacement_control_plane::{
        execute_core, prepare, ProjectReplacementControlPlaneState,
    };
    use protocol::control_plane_project::{
        ProjectReplacementActionV1 as Action, ProjectReplacementErrorV1 as Error,
        ProjectReplacementRequestV1 as Request, ProjectReplacementResponseV1 as Response,
    };
    use sha2::{Digest, Sha256};

    fn backup() -> ProjectBackupEnvelope {
        let project: ProjectFile = serde_json::from_str(PHASE1_SAMPLE_PROJECT_JSON).unwrap();
        let common = serde_json::json!({"action":"LightingMaster", "fixture_id":null,
            "attribute":null,"group_id":null,"cue_id":null,"layer_id":null,"output_id":null,
            "video_param":null,"cue_point_index":null,"duration_ms":null,"low":0.0,"high":1.0});
        let mut midi = common.clone();
        midi["channel"] = serde_json::json!(0);
        midi["message"] = serde_json::json!("ControlChange");
        midi["number"] = serde_json::json!(7);
        let mut osc = common.clone();
        osc["address"] = serde_json::json!("/restore/master");
        let mut dmx = common;
        dmx["universe"] = serde_json::json!(0);
        dmx["channel"] = serde_json::json!(12);
        ProjectBackupEnvelope { version: PROJECT_BACKUP_VERSION, app: APP_NAME.into(), id: 7,
            created_at_unix_ms: 97, source_path: Some("C:/公演/元ファイル.sdc".into()),
            reason: "restore proof".into(),
            dj_track_triggers: vec![serde_json::from_value(serde_json::json!({"id":"restore-dj",
                "selector":{"contentId":"restore-proof"},"timelineId":project.snapshot.timeline.id})).unwrap()],
            project, midi_mappings: vec![serde_json::from_value(midi).unwrap()],
            osc_mappings: vec![serde_json::from_value(osc).unwrap()],
            dmx_mappings: vec![serde_json::from_value(dmx).unwrap()] }
    }
    fn request(
        query: &ControlPlaneQueryState,
        state: &AppState,
        bytes: &[u8],
        source: Option<String>,
    ) -> Request {
        let action = Action::RestoreBackup {
            backup_id: 7,
            expected_file_sha256: format!("{:x}", Sha256::digest(bytes)),
            expected_source_path: source,
        };
        Request {
            schema_version: 1,
            operation_id: action.operation_id().into(),
            request_id: 7,
            expected_fence: query
                .issue_project_mutation_fence_for_window("media-asset-a6", state)
                .unwrap(),
            action,
        }
    }
    #[test]
    fn project_backup_restoration_prepares_full_mappings_exact_bytes_and_unsaved_source_projection()
    {
        let harness = MediaAssetA6CommandHarness::new();
        let query = ControlPlaneQueryState::new().unwrap();
        let root = unique_test_directory("backup-restore-image");
        std::fs::create_dir_all(&root).unwrap();
        let file = project_backup_path(&root, 7);
        for source in [
            Some("C:/公演/元ファイル.sdc".to_string()),
            None,
            Some("C:/template.json".to_string()),
        ] {
            let mut backup = backup();
            backup.source_path = source.clone();
            let projection = source.filter(|source| is_syndocal_project_path(Path::new(source)));
            let mut bytes = project_backup_json::project_backup_json_bytes(&backup).unwrap();
            bytes.extend_from_slice(b" \r\n\t");
            std::fs::write(&file, &bytes).unwrap();
            let prepared = prepare(
                &request(&query, &harness.state, &bytes, projection.clone()),
                Some(&root),
            )
            .unwrap();
            assert_eq!(
                prepared.authority_disposition,
                ProjectAuthorityDisposition::UnsavedReplacement
            );
            assert_eq!(prepared.result.current_project_path, projection);
            assert_eq!(prepared.mappings.midi_mappings, backup.midi_mappings);
            assert_eq!(prepared.mappings.osc_mappings, backup.osc_mappings);
            assert_eq!(prepared.mappings.dmx_mappings, backup.dmx_mappings);
            assert_eq!(
                prepared.mappings.dj_track_triggers,
                backup.dj_track_triggers
            );
            let expected = serde_json::to_value(prepared.snapshot.clone()).unwrap();
            harness
                .state
                .engine
                .set_output_ownership_role(MachineOutputRole::Standby)
                .unwrap();
            request_project_snapshot_publication(&harness.state.engine, prepared.snapshot).unwrap();
            let actual = serde_json::to_value(project_snapshot_for_save(
                harness.state.engine.persistence_snapshot().unwrap(),
            ))
            .unwrap();
            assert_eq!(
                actual, expected,
                "Restore preparation and acknowledged persistence must agree"
            );
            assert_eq!(
                std::fs::read(&file).unwrap(),
                bytes,
                "Restore cannot rewrite backup bytes"
            );
            let output = harness.state.engine.output_ownership_status();
            assert!(!output.lighting_allowed && !output.video_allowed);
        }
        std::fs::remove_file(&file).unwrap();
        std::fs::remove_dir(&root).unwrap();
    }
    #[test]
    fn project_backup_restoration_rejects_changed_invalid_or_missing_before_confirmation_and_publication(
    ) {
        let harness = MediaAssetA6CommandHarness::new();
        let query = ControlPlaneQueryState::new().unwrap();
        let root = unique_test_directory("backup-restore-rejections");
        std::fs::create_dir_all(&root).unwrap();
        let file = project_backup_path(&root, 7);
        let backup = backup();
        let bytes = project_backup_json::project_backup_json_bytes(&backup).unwrap();
        let initial = request(&query, &harness.state, &bytes, backup.source_path.clone());
        let before = query
            .issue_project_mutation_fence_for_window("media-asset-a6", &harness.state)
            .unwrap();
        for (contents, action, code) in [
            (
                Some(bytes.clone()),
                Action::RestoreBackup {
                    backup_id: 7,
                    expected_file_sha256: "a".repeat(64),
                    expected_source_path: backup.source_path.clone(),
                },
                Error::FileChanged,
            ),
            (
                Some(bytes.clone()),
                Action::RestoreBackup {
                    backup_id: 7,
                    expected_file_sha256: format!("{:x}", Sha256::digest(&bytes)),
                    expected_source_path: None,
                },
                Error::FileChanged,
            ),
            (
                Some(b"{}".to_vec()),
                initial.action.clone(),
                Error::InvalidProject,
            ),
            (None, initial.action.clone(), Error::InvalidProject),
        ] {
            if let Some(contents) = &contents {
                std::fs::write(&file, contents).unwrap();
            } else {
                std::fs::remove_file(&file).unwrap();
            }
            let original = Request {
                action,
                ..initial.clone()
            };
            let response = execute_core(
                &harness.state,
                &query,
                &ProjectReplacementControlPlaneState::default(),
                "media-asset-a6",
                "local",
                original,
                Some(&root),
                || panic!("Invalid backup cannot confirm"),
                |_| panic!("Invalid backup cannot retire or publish"),
            );
            assert_eq!(
                response,
                Response::Rejected {
                    request_id: 7,
                    code
                }
            );
            let after = query
                .issue_project_mutation_fence_for_window("media-asset-a6", &harness.state)
                .unwrap();
            assert_eq!(after.project_epoch, before.project_epoch);
            assert_eq!(after.project_revision, before.project_revision);
            assert_eq!(
                after.project_checkpoint_hash,
                before.project_checkpoint_hash
            );
            assert_eq!(
                after.project_publication_generation,
                before.project_publication_generation
            );
            if let Some(contents) = contents {
                assert_eq!(std::fs::read(&file).unwrap(), contents);
            }
        }
        std::fs::remove_dir(&root).unwrap();
    }

    #[test]
    fn project_backup_restoration_preflights_locks_and_exhausted_generation_before_file_read() {
        for mode in [
            Some(OperatorLockMode::Full),
            Some(OperatorLockMode::Partial),
            None,
        ] {
            let harness = MediaAssetA6CommandHarness::new();
            let query = ControlPlaneQueryState::new().unwrap();
            let original = request(&query, &harness.state, b"absent backup", None);
            let mut coordinator = harness.state.project_coordinator.lock().unwrap();
            if let Some(mode) = mode {
                coordinator.ancillary.operator_policy = Some(OperatorPolicy {
                    lock_mode: mode,
                    lock_on_load: true,
                    credential: protocol::OperatorCredentialVerifier {
                        scheme: OPERATOR_CREDENTIAL_SCHEME.into(),
                        iterations: OPERATOR_CREDENTIAL_MIN_ITERATIONS,
                        salt_b64: base64::engine::general_purpose::STANDARD.encode([1_u8; 16]),
                        verifier_b64: base64::engine::general_purpose::STANDARD.encode([2_u8; 32]),
                    },
                });
            } else {
                coordinator.recovery_authority_serial =
                    protocol::control_plane_command::MAX_SAFE_JAVASCRIPT_INTEGER;
            }
            drop(coordinator);
            // Missing root would be InvalidProject if preparation/read preceded preflight.
            assert_eq!(
                execute_core(
                    &harness.state,
                    &query,
                    &ProjectReplacementControlPlaneState::default(),
                    "media-asset-a6",
                    "local",
                    original,
                    None,
                    || panic!("Preflight rejection cannot confirm"),
                    |_| panic!("Preflight rejection cannot publish")
                ),
                Response::Rejected {
                    request_id: 7,
                    code: if mode.is_some() {
                        Error::Forbidden
                    } else {
                        Error::Overloaded
                    }
                }
            );
        }
    }
}
