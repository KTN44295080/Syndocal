mod project_replacement_control_plane_contract_tests {
    use super::*;
    use crate::project_replacement_control_plane::{
        execute_core, prepare, ProjectReplacementControlPlaneState,
    };
    use protocol::control_plane_project::{
        ProjectReplacementActionV1 as Action, ProjectReplacementErrorV1 as Error,
        ProjectReplacementOutcomeV1 as Outcome, ProjectReplacementRequestV1 as Request,
        ProjectReplacementResponseV1 as Response,
    };

    fn request(query: &ControlPlaneQueryState, state: &AppState, action: Action) -> Request {
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
    fn project_replacement_control_plane_new_matches_acknowledged_engine_persistence() {
        let harness = MediaAssetA6CommandHarness::new();
        let query = ControlPlaneQueryState::new().unwrap();
        let prepared = prepare(&request(&query, &harness.state, Action::New {}), None).unwrap();
        // Compare the actual prepared/hash input, without sanitizing it here.
        // Runtime-only defaults must not be hidden from this assertion.
        let expected = serde_json::to_value(prepared.snapshot.clone()).unwrap();
        request_project_snapshot_publication(&harness.state.engine, prepared.snapshot).unwrap();
        let actual = serde_json::to_value(project_snapshot_for_save(
            harness.state.engine.persistence_snapshot().unwrap(),
        ))
        .unwrap();
        assert_eq!(
            actual, expected,
            "New receipt and acknowledged engine must name the same persistent image"
        );
    }

    #[test]
    fn project_replacement_control_plane_legacy_open_matches_acknowledged_engine_persistence() {
        assert_open_ack_persistence(
            &serde_json::to_vec(&ProjectFile {
                version: PROJECT_FILE_VERSION,
                app: APP_NAME.into(),
                operator_policy: None,
                custom_profiles: Vec::new(),
                fixture_groups: Vec::new(),
                snapshot: EngineSnapshot::default(),
            })
            .unwrap(),
            "legacy-default",
        );
        assert_open_ack_persistence(PHASE1_SAMPLE_PROJECT_JSON.as_bytes(), "legacy-phase1");
        assert_open_ack_persistence(
            include_bytes!("../../../qa/migration/authored-control-project.json"),
            "current-authored-controls",
        );
    }

    fn assert_open_ack_persistence(bytes: &[u8], label: &str) {
        use sha2::{Digest, Sha256};
        let harness = MediaAssetA6CommandHarness::new();
        let query = ControlPlaneQueryState::new().unwrap();
        let root = unique_test_directory("typed-legacy-project-open");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("legacy.sdc");
        // Keep the fixtures byte-exact, including enabled output declarations.
        // The real worker must deny capabilities before it receives that image.
        harness
            .state
            .engine
            .set_output_ownership_role(MachineOutputRole::Standby)
            .unwrap();
        let denied = harness.state.engine.output_ownership_status();
        assert_eq!(denied.effective_role, MachineOutputRole::Standby);
        assert!(!denied.lighting_allowed && !denied.video_allowed);
        std::fs::write(&path, bytes).unwrap();
        let original = request(
            &query,
            &harness.state,
            Action::Open {
                path: path.to_string_lossy().into(),
                expected_file_sha256: format!("{:x}", Sha256::digest(bytes)),
            },
        );
        let prepared = prepare(&original, None).unwrap();
        let expected = serde_json::to_value(prepared.snapshot.clone()).unwrap();
        request_project_snapshot_publication(&harness.state.engine, prepared.snapshot).unwrap();
        let after = harness.state.engine.output_ownership_status();
        assert!(!after.lighting_allowed && !after.video_allowed);
        let actual = serde_json::to_value(project_snapshot_for_save(
            harness.state.engine.persistence_snapshot().unwrap(),
        ))
        .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(&root).unwrap();
        assert_eq!(
            actual, expected,
            "{label}: Open receipt and acknowledged engine must name the same persistent image"
        );
    }

    #[test]
    fn project_replacement_control_plane_open_rejects_current_reference_conflict_before_publication(
    ) {
        use sha2::{Digest, Sha256};
        let harness = MediaAssetA6CommandHarness::new();
        let query = ControlPlaneQueryState::new().unwrap();
        let root = unique_test_directory("typed-invalid-current-open");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("invalid-current.sdc");
        let mut source: serde_json::Value = serde_json::from_str(include_str!(
            "../../../qa/migration/authored-control-project.json"
        ))
        .unwrap();
        source["snapshot"]["timeline_bank"][0]["label"] =
            serde_json::json!("Conflicting authored bank");
        let bytes = serde_json::to_vec(&source).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let control = ProjectReplacementControlPlaneState::default();
        let original = request(
            &query,
            &harness.state,
            Action::Open {
                path: path.to_string_lossy().into(),
                expected_file_sha256: format!("{:x}", Sha256::digest(&bytes)),
            },
        );
        assert_eq!(
            execute_core(
                &harness.state,
                &query,
                &control,
                "media-asset-a6",
                "local",
                original,
                None,
                || panic!("Invalid current-schema image cannot confirm"),
                |_| panic!("Invalid current-schema image cannot publish"),
            ),
            Response::Rejected {
                request_id: 7,
                code: Error::InvalidProject
            }
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(&root).unwrap();
    }

    #[test]
    fn project_replacement_control_plane_recovery_generation_exhaustion_is_preflighted() {
        let harness = MediaAssetA6CommandHarness::new();
        let query = ControlPlaneQueryState::new().unwrap();
        let control = ProjectReplacementControlPlaneState::default();
        let original = request(&query, &harness.state, Action::New {});
        harness
            .state
            .project_coordinator
            .lock()
            .unwrap()
            .recovery_authority_serial =
            protocol::control_plane_command::MAX_SAFE_JAVASCRIPT_INTEGER;
        assert_eq!(
            execute_core(
                &harness.state,
                &query,
                &control,
                "media-asset-a6",
                "local",
                original,
                None,
                || panic!("Exhausted generation cannot confirm"),
                |_| panic!("Exhausted generation cannot publish")
            ),
            Response::Rejected {
                request_id: 7,
                code: Error::Overloaded
            }
        );
    }

    #[test]
    fn project_replacement_control_plane_operator_lock_rejects_before_confirmation_or_publication()
    {
        for mode in [OperatorLockMode::Full, OperatorLockMode::Partial] {
            let harness = MediaAssetA6CommandHarness::new();
            let query = ControlPlaneQueryState::new().unwrap();
            let control = ProjectReplacementControlPlaneState::default();
            let original = request(&query, &harness.state, Action::New {});
            let policy = OperatorPolicy {
                lock_mode: mode,
                lock_on_load: true,
                credential: protocol::OperatorCredentialVerifier {
                    scheme: OPERATOR_CREDENTIAL_SCHEME.into(),
                    iterations: OPERATOR_CREDENTIAL_MIN_ITERATIONS,
                    salt_b64: base64::engine::general_purpose::STANDARD.encode([1_u8; 16]),
                    verifier_b64: base64::engine::general_purpose::STANDARD.encode([2_u8; 32]),
                },
            };
            validate_operator_policy(&policy).unwrap();
            harness
                .state
                .project_coordinator
                .lock()
                .unwrap()
                .ancillary
                .operator_policy = Some(policy);
            assert_eq!(
                execute_core(
                    &harness.state,
                    &query,
                    &control,
                    "media-asset-a6",
                    "local",
                    original,
                    None,
                    || panic!("Operator Lock cannot confirm"),
                    |_| panic!("Operator Lock cannot publish")
                ),
                Response::Rejected {
                    request_id: 7,
                    code: Error::Forbidden
                }
            );
        }
    }

    #[test]
    fn project_replacement_control_plane_registry_requires_r5_audit_receipt_and_local_confirmation()
    {
        use protocol::control_plane::{
            OperationAuditRequirement, OperationCapability, OperationRisk,
        };
        use protocol::control_plane_registry_v2::{
            AdapterPolicy, ConsentPolicy, RatePolicy, ReceiptPolicy,
        };
        let registry = control_plane::canonical_registry().unwrap();
        for operation_id in [
            protocol::control_plane_project::PROJECT_NEW_OPERATION_ID,
            protocol::control_plane_project::PROJECT_OPEN_OPERATION_ID,
            protocol::control_plane_project::PROJECT_BACKUP_RESTORE_OPERATION_ID,
        ] {
            let operation = registry
                .canonical_operations
                .iter()
                .find(|operation| operation.operation_id == operation_id)
                .unwrap();
            assert_eq!(operation.risk, OperationRisk::R5);
            assert_eq!(
                operation.adapter_policy,
                AdapterPolicy::LocalWindowProjectReplacement
            );
            assert_eq!(operation.derived_adapters.len(), 1);
            operation.validate().unwrap();
            for mutate in [
                |op: &mut protocol::control_plane_registry_v2::CanonicalOperationDescriptor| {
                    op.risk = OperationRisk::R4
                },
                |op: &mut protocol::control_plane_registry_v2::CanonicalOperationDescriptor| {
                    op.audit = OperationAuditRequirement::NotApplicable
                },
                |op: &mut protocol::control_plane_registry_v2::CanonicalOperationDescriptor| {
                    op.consent_policy = ConsentPolicy::LocalExplicitAction
                },
                |op: &mut protocol::control_plane_registry_v2::CanonicalOperationDescriptor| {
                    op.rate_policy = RatePolicy::FailClosed
                },
                |op: &mut protocol::control_plane_registry_v2::CanonicalOperationDescriptor| {
                    op.receipt_policy = ReceiptPolicy::FailClosed
                },
                |op: &mut protocol::control_plane_registry_v2::CanonicalOperationDescriptor| {
                    op.capabilities
                        .push(OperationCapability::AllowedDuringFullLock)
                },
            ] {
                let mut forged = operation.clone();
                mutate(&mut forged);
                assert!(forged.validate().is_err());
            }
        }
    }

    #[test]
    fn project_replacement_control_plane_cancel_is_replayed_without_confirmation_or_publication() {
        let harness = MediaAssetA6CommandHarness::new();
        let query = ControlPlaneQueryState::new().unwrap();
        let control = ProjectReplacementControlPlaneState::default();
        let original = request(&query, &harness.state, Action::New {});
        let response = execute_core(
            &harness.state,
            &query,
            &control,
            "media-asset-a6",
            "local",
            original.clone(),
            None,
            || false,
            |_| panic!("Cancellation must not retire/publish"),
        );
        assert!(
            matches!(&response, Response::Receipt(receipt) if receipt.outcome == Outcome::Cancelled)
        );
        // Retained replay precedes current authority and issued-fence checks.
        harness
            .state
            .project_coordinator
            .lock()
            .unwrap()
            .publication_generation += 1;
        let replay = execute_core(
            &harness.state,
            &query,
            &control,
            "media-asset-a6",
            "local",
            original.clone(),
            None,
            || panic!("A replay cannot confirm again"),
            |_| panic!("A replay cannot publish"),
        );
        assert_eq!(replay, response);
        let mut changed = original;
        changed.expected_fence.project_publication_generation += 1;
        assert_eq!(
            execute_core(
                &harness.state,
                &query,
                &control,
                "media-asset-a6",
                "local",
                changed,
                None,
                || panic!("Changed shape cannot confirm"),
                |_| panic!("Changed shape cannot publish")
            ),
            Response::Rejected {
                request_id: 7,
                code: Error::Conflict
            }
        );
    }

    #[test]
    fn project_replacement_control_plane_rejects_unissued_fence_and_does_not_promote_principal_to_owner(
    ) {
        let harness = MediaAssetA6CommandHarness::new();
        let query = ControlPlaneQueryState::new().unwrap();
        let control = ProjectReplacementControlPlaneState::default();
        let mut forged = request(&query, &harness.state, Action::New {});
        forged.expected_fence.session_incarnation += 1;
        assert_eq!(
            execute_core(
                &harness.state,
                &query,
                &control,
                "media-asset-a6",
                "external:agent:1",
                forged,
                None,
                || panic!("Unissued fence cannot confirm"),
                |_| panic!("Unissued fence cannot publish")
            ),
            Response::Rejected {
                request_id: 7,
                code: Error::Forbidden
            }
        );
        assert_eq!(
            harness
                .state
                .project_transaction_owners
                .lock()
                .unwrap()
                .get("media-asset-a6")
                .unwrap(),
            MEDIA_ASSET_A6_OWNER
        );
    }

    #[test]
    fn project_replacement_control_plane_rejects_publication_only_change_before_confirmation() {
        let harness = MediaAssetA6CommandHarness::new();
        let query = ControlPlaneQueryState::new().unwrap();
        let control = ProjectReplacementControlPlaneState::default();
        let original = request(&query, &harness.state, Action::New {});
        harness
            .state
            .project_coordinator
            .lock()
            .unwrap()
            .publication_generation += 1;
        assert_eq!(
            execute_core(
                &harness.state,
                &query,
                &control,
                "media-asset-a6",
                "local",
                original,
                None,
                || panic!("Stale fence cannot confirm"),
                |_| panic!("Stale fence cannot publish")
            ),
            Response::Rejected {
                request_id: 7,
                code: Error::StaleFence
            }
        );
    }

    #[test]
    fn project_replacement_control_plane_open_hashes_and_parses_the_same_bytes_without_writing_source(
    ) {
        use sha2::{Digest, Sha256};
        let harness = MediaAssetA6CommandHarness::new();
        let query = ControlPlaneQueryState::new().unwrap();
        let root = unique_test_directory("typed-project-open");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("日本語.sdc");
        let project = ProjectFile {
            version: PROJECT_FILE_VERSION,
            app: APP_NAME.into(),
            operator_policy: None,
            custom_profiles: Vec::new(),
            fixture_groups: Vec::new(),
            snapshot: EngineSnapshot::default(),
        };
        let bytes = serde_json::to_vec(&project).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let original = request(
            &query,
            &harness.state,
            Action::Open {
                path: path.to_string_lossy().into(),
                expected_file_sha256: format!("{:x}", Sha256::digest(&bytes)),
            },
        );
        assert_eq!(prepare(&original, None).unwrap().snapshot.clock.bpm, 120.0);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        std::fs::write(&path, b"{}").unwrap();
        assert!(matches!(prepare(&original, None), Err(Error::FileChanged)));
        let mut changed = original.clone();
        changed.action = Action::Open {
            path: path.to_string_lossy().into(),
            expected_file_sha256: format!("{:x}", Sha256::digest(b"{}")),
        };
        assert!(matches!(prepare(&changed, None), Err(Error::InvalidProject)));
        let mut relative = original;
        relative.action = Action::Open {
            path: "relative.sdc".into(),
            expected_file_sha256: "a".repeat(64),
        };
        assert!(matches!(prepare(&relative, None), Err(Error::InvalidRequest)));
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(&root).unwrap();
    }
}
