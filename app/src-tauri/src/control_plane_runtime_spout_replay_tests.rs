use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn spout_replay_public_request() -> protocol::control_plane_command::OutputControlCommandRequestV2 {
    protocol::control_plane_command::OutputControlCommandRequestV2 {
        operation_id:
            protocol::control_plane_command::OUTPUT_SHOW_SPOUT_OUTPUTS_ENABLE_OPERATION_ID
                .to_string(),
        request_id: 901,
        expected_fence: protocol::control_plane_command::OutputControlFenceV1 {
            process_incarnation: 11,
            session_incarnation: 12,
            project_epoch: 3,
            project_revision: 4,
            project_checkpoint_hash: "a".repeat(64),
            project_publication_generation: 5,
            output_epoch: 6,
            output_generation: 7,
            safety_blackout_epoch: 8,
            safety_blackout_generation: 9,
        },
        action: protocol::control_plane_command::OutputControlActionV2::EnableShowSpoutOutputs {
            lease: protocol::control_plane_command::OutputLeaseAuthorityV1 {
                lease_id: "lease-0000000000000001".to_string(),
                generation: 1,
            },
        },
    }
}

fn spout_replay_private_request_and_receipt(
    public_request: &protocol::control_plane_command::OutputControlCommandRequestV2,
) -> (
    crate::output_lease::OutputLeaseRequest,
    crate::output_lease::OutputLeaseRequestReceipt,
) {
    let owner = crate::output_lease::OutputLeaseOwner::new("local-ui", "main", 41, 7)
        .expect("Spout replay owner");
    let resources = crate::output_lease::OutputLeaseResources::new(&[
        crate::output_lease::OutputLeaseResource::Lighting,
        crate::output_lease::OutputLeaseResource::Video,
    ])
    .expect("Spout replay resources");
    let mut registry =
        crate::output_lease::OutputLeaseRegistry::fresh_process(41).expect("Spout replay registry");
    let acquire = crate::output_lease::OutputLeaseRequest::from_action(
        "local-ui",
        "spout-replay-fixture",
        1,
        crate::output_lease::OutputLeaseRequestAction::Acquire {
            owner: owner.clone(),
            resources: resources.clone(),
            project_identity: format!(
                "project_epoch:{}",
                public_request.expected_fence.project_epoch
            ),
            ttl_ms: 30_000,
        },
    )
    .expect("Spout replay fixture acquire request");
    let acquired = registry
        .submit_request(&acquire, 1)
        .expect("Spout replay fixture acquire receipt");
    let lease_id = acquired
        .lease_id
        .expect("Spout replay fixture lease identity");
    let generation = acquired
        .generation_after
        .expect("Spout replay fixture lease generation");
    let private_request = crate::output_lease::OutputLeaseRequest::from_action(
        "local-ui",
        crate::OUTPUT_LEASE_OUTPUT_CONTROL_DOMAIN,
        public_request.request_id,
        crate::output_lease::OutputLeaseRequestAction::AuthorizeOrdinary {
            lease_id,
            owner,
            expected_generation: generation,
            exact_resources: resources,
        },
    )
    .expect("Spout replay fixture authorization request");
    let receipt = registry
        .submit_request(&private_request, 2)
        .expect("Spout replay fixture authorization receipt");
    (private_request, receipt)
}

fn spout_replay_response(
    request: &protocol::control_plane_command::OutputControlCommandRequestV2,
    private_receipt: &crate::output_lease::OutputLeaseRequestReceipt,
) -> protocol::control_plane_command::OutputControlResponseV2 {
    let mut fence_after = request.expected_fence.clone();
    fence_after.project_revision += 1;
    fence_after.project_checkpoint_hash = "b".repeat(64);
    fence_after.project_publication_generation += 1;
    let lease_result =
        crate::control_plane_runtime::output_control_lease_result_from_registry_receipt(
            &request.action,
            private_receipt,
        )
        .expect("Spout replay fixture lease result");
    let response = protocol::control_plane_command::OutputControlResponseV2::Receipt(Box::new(
        protocol::control_plane_command::OutputControlReceiptV2 {
            operation_id: request.operation_id.clone(),
            request_id: request.request_id,
            shape_sha256: crate::control_plane_runtime::hex_sha256(
                &request
                    .canonical_shape_bytes()
                    .expect("Spout replay public shape"),
            ),
            argument_fingerprint: crate::control_plane_runtime::hex_sha256(
                &request
                    .argument_fingerprint_bytes()
                    .expect("Spout replay public arguments"),
            ),
            audit_sequence: 1,
            fence_before: request.expected_fence.clone(),
            fence_after,
            outcome: protocol::control_plane_command::OutputControlReceiptOutcomeV2::Applied,
            lease_result: Some(lease_result),
        },
    ));
    response.validate().expect("Spout replay response is valid");
    response
}

fn unique_spout_replay_journal_path() -> (PathBuf, PathBuf) {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after Unix epoch")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "syndocal-spout-managed-terminal-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).expect("create Spout replay journal directory");
    let path = directory.join(crate::OUTPUT_LEASE_DURABLE_RECEIPT_STATE_FILE);
    (directory, path)
}

fn spout_replay_receipt(
    response: &protocol::control_plane_command::OutputControlResponseV2,
) -> &protocol::control_plane_command::OutputControlReceiptV2 {
    match response {
        protocol::control_plane_command::OutputControlResponseV2::Receipt(receipt) => receipt,
        protocol::control_plane_command::OutputControlResponseV2::Rejected(_) => {
            unreachable!("Spout replay test response is a receipt")
        }
    }
}

#[test]
fn show_spout_managed_terminal_replays_exact_response_after_successor_fence_persisted() {
    let (directory, path) = unique_spout_replay_journal_path();
    let request = spout_replay_public_request();
    request.validate().expect("Spout replay public request");
    let (private_request, private_receipt) = spout_replay_private_request_and_receipt(&request);
    let response = spout_replay_response(&request, &private_receipt);
    let shape_sha256 = spout_replay_receipt(&response).shape_sha256.clone();
    let argument_fingerprint = spout_replay_receipt(&response).argument_fingerprint.clone();

    let mut journal = crate::OutputLeaseDurableReceiptJournal::in_memory();
    journal
        .install_path(path.clone())
        .expect("install Spout replay journal");
    assert_eq!(
        journal.prepare(&private_request),
        Ok(crate::OutputLeaseDurablePrepareResult::Fresh)
    );
    journal
        .record(&private_receipt)
        .expect("record canonical Spout lease receipt");
    journal
        .record_managed_exact_both_output_control_terminal(
            "local-ui",
            "main",
            &request,
            &shape_sha256,
            &argument_fingerprint,
            &response,
        )
        .expect("record exact Spout public terminal");

    let current_fence = spout_replay_receipt(&response).fence_after.clone();
    assert_ne!(
        current_fence, request.expected_fence,
        "the persisted project commit advances the current fence"
    );

    let mut restarted = crate::OutputLeaseDurableReceiptJournal::in_memory();
    restarted
        .install_path(path.clone())
        .expect("restart loads Spout public terminal");
    assert_eq!(
        restarted
            .lookup_managed_exact_both_output_control_terminal(
                "local-ui",
                "main",
                &request,
                &shape_sha256,
                &argument_fingerprint,
            )
            .expect("lookup persisted Spout public terminal"),
        Some(response.clone()),
        "retry returns the exact original terminal after the project fence advanced"
    );
    fs::remove_dir_all(directory).expect("remove Spout replay journal directory");
}

#[test]
fn show_spout_managed_terminal_rejects_invalid_fence_without_publishing() {
    let (directory, path) = unique_spout_replay_journal_path();
    let request = spout_replay_public_request();
    let (private_request, private_receipt) = spout_replay_private_request_and_receipt(&request);
    let response = spout_replay_response(&request, &private_receipt);
    let shape_sha256 = spout_replay_receipt(&response).shape_sha256.clone();
    let argument_fingerprint = spout_replay_receipt(&response).argument_fingerprint.clone();
    let mut invalid_response = response;
    let protocol::control_plane_command::OutputControlResponseV2::Receipt(receipt) =
        &mut invalid_response
    else {
        unreachable!("Spout test response is a receipt");
    };
    receipt.fence_after.output_generation += 1;

    let mut journal = crate::OutputLeaseDurableReceiptJournal::in_memory();
    journal
        .install_path(path)
        .expect("install Spout invalid-terminal journal");
    journal
        .prepare(&private_request)
        .expect("prepare Spout invalid-terminal private receipt");
    journal
        .record(&private_receipt)
        .expect("record Spout invalid-terminal private receipt");
    assert!(
        journal
            .record_managed_exact_both_output_control_terminal(
                "local-ui",
                "main",
                &request,
                &shape_sha256,
                &argument_fingerprint,
                &invalid_response,
            )
            .is_err(),
        "a physical/output fence change is rejected as an invalid Spout terminal"
    );
    assert_eq!(
        journal
            .lookup_managed_exact_both_output_control_terminal(
                "local-ui",
                "main",
                &request,
                &shape_sha256,
                &argument_fingerprint,
            )
            .expect("lookup rejected Spout terminal"),
        None,
        "invalid terminal rejection does not publish a replay record"
    );
    fs::remove_dir_all(directory).expect("remove Spout invalid-terminal directory");
}
