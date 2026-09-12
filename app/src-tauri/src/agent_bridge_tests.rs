use super::*;
fn id(value: usize) -> String {
    format!("00000000-0000-4000-8000-{value:012x}")
}
fn command(method: &str) -> Command {
    let params = match method {
        "fixtures.list" => serde_json::json!({}),
        "runtime.get" | "control_plane.get_capabilities" | "recording.get_status" => {
            serde_json::json!({})
        }
        "fixtures.get" => serde_json::json!({"fixtureId": 1}),
        "output.set_video_blackout" => serde_json::json!({
            "enabled": true,
            "expectedProject": {
                "project_epoch": 0,
                "project_revision": 0,
                "checkpoint_hash": "a".repeat(64)
            }
        }),
        "control_plane.execute" => serde_json::json!({
            "operationId": "syndocal.query.control_plane.capabilities.v1",
            "request": {}
        }),
        _ => {
            serde_json::json!({"fixtureId":1,"position":{"x":1,"y":2,"z":3},"rotation":{"pitch":0,"yaw":0,"roll":0},"expectedProject":{"project_epoch":0,"project_revision":0,"checkpoint_hash":"a".repeat(64)}})
        }
    };
    Request {
        token: "token".to_string(),
        request_id: id(1),
        method: method.to_string(),
        params,
        auth: None,
    }
    .command()
    .unwrap()
}

fn hex_bytes(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|chunk| {
            let high = (chunk[0] as char).to_digit(16).unwrap();
            let low = (chunk[1] as char).to_digit(16).unwrap();
            ((high << 4) | low) as u8
        })
        .collect()
}

fn proof(key: &str, message: &str) -> String {
    use sha2::{Digest, Sha256};
    let key = hex_bytes(key);
    let mut normalized = [0u8; 64];
    normalized[..key.len()].copy_from_slice(&key);
    let mut inner_pad = [0x36u8; 64];
    let mut outer_pad = [0x5cu8; 64];
    for index in 0..64 {
        inner_pad[index] ^= normalized[index];
        outer_pad[index] ^= normalized[index];
    }
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(message.as_bytes());
    let inner_digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_digest);
    outer
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn agent_bridge_wire_auth_methods_and_bounds_are_strict() {
    assert!(token_matches("abcdef", "abcdef"));
    for invalid in ["abcde", "abcdefx", "abcdeg", ""] {
        assert!(!token_matches("abcdef", invalid));
    }
    assert!(main_only("main").is_ok());
    for label in ["", "video-output-1", "timeline", "Main"] {
        assert!(main_only(label).is_err());
    }
    let value =
        serde_json::json!({"token":"token","requestId":id(1),"method":"fixtures.list","params":{}});
    let mut extra = value.clone();
    extra["owner"] = serde_json::json!("main");
    assert!(serde_json::from_value::<Request>(extra).is_err());
    for (field, content) in [
        ("method", serde_json::json!("output.enable")),
        ("params", serde_json::json!({"extra":true})),
        ("requestId", serde_json::json!("not-a-uuid")),
    ] {
        let mut invalid = value.clone();
        invalid[field] = content;
        assert!(serde_json::from_value::<Request>(invalid)
            .unwrap()
            .command()
            .is_err());
    }
    for fixture_id in [0u64, 9_007_199_254_740_992] {
        let value = serde_json::json!({"token":"token","requestId":id(1),"method":"fixtures.get","params":{"fixtureId":fixture_id}});
        assert!(serde_json::from_value::<Request>(value)
            .unwrap()
            .command()
            .is_err());
    }
    let mut runtime = value.clone();
    runtime["method"] = serde_json::json!("runtime.get");
    assert!(matches!(
        serde_json::from_value::<Request>(runtime.clone())
            .unwrap()
            .command()
            .unwrap(),
        Command::RuntimeGet(_)
    ));
    runtime["params"] = serde_json::json!({"enableOutputs": true});
    assert!(serde_json::from_value::<Request>(runtime)
        .unwrap()
        .command()
        .is_err());

    let mut capabilities = value.clone();
    capabilities["method"] = serde_json::json!("control_plane.get_capabilities");
    capabilities["params"] = serde_json::json!({});
    assert!(matches!(
        serde_json::from_value::<Request>(capabilities)
            .unwrap()
            .command()
            .unwrap(),
        Command::ControlPlaneCapabilities(_)
    ));

    let mut recording = value.clone();
    recording["method"] = serde_json::json!("recording.get_status");
    recording["params"] = serde_json::json!({});
    assert!(matches!(
        serde_json::from_value::<Request>(recording)
            .unwrap()
            .command()
            .unwrap(),
        Command::RecordingStatus(_)
    ));

    let mut canonical = value.clone();
    canonical["method"] = serde_json::json!("control_plane.execute");
    canonical["params"] = serde_json::json!({
        "operationId": "syndocal.query.control_plane.capabilities.v1",
        "request": {}
    });
    assert!(matches!(
        serde_json::from_value::<Request>(canonical.clone())
            .unwrap()
            .command()
            .unwrap(),
        Command::ControlPlaneExecute(_)
    ));
    canonical["params"]["operationId"] = serde_json::json!("syndocal.query.not_reviewed.v1");
    assert_eq!(
        serde_json::from_value::<Request>(canonical)
            .unwrap()
            .command()
            .unwrap_err(),
        "invalid_canonical_operation"
    );

    let video = serde_json::json!({
        "token": "token",
        "requestId": id(1),
        "method": "output.set_video_blackout",
        "params": {
            "enabled": true,
            "expectedProject": {
                "project_epoch": 0,
                "project_revision": 0,
                "checkpoint_hash": "a".repeat(64)
            }
        }
    });
    assert!(matches!(
        serde_json::from_value::<Request>(video.clone())
            .unwrap()
            .command()
            .unwrap(),
        Command::SetVideoBlackout(_)
    ));
    for invalid in [
        serde_json::json!({"enabled": "true", "expectedProject": {"project_epoch": 0, "project_revision": 0, "checkpoint_hash": "a".repeat(64)}}),
        serde_json::json!({"enabled": true, "expectedProject": {"project_epoch": 9_007_199_254_740_992u64, "project_revision": 0, "checkpoint_hash": "a".repeat(64)}}),
        serde_json::json!({"enabled": true, "expectedProject": {"project_epoch": 0, "project_revision": 0, "checkpoint_hash": "A".repeat(64)}}),
        serde_json::json!({"enabled": true, "expectedProject": {"project_epoch": 0, "project_revision": 0, "checkpoint_hash": "a".repeat(63)}}),
        serde_json::json!({"enabled": true, "expectedProject": {"project_epoch": 0, "project_revision": 0, "checkpoint_hash": "a".repeat(64)}, "extra": false}),
    ] {
        let mut candidate = video.clone();
        candidate["params"] = invalid;
        assert!(serde_json::from_value::<Request>(candidate)
            .unwrap()
            .command()
            .is_err());
    }
}

#[test]
fn agent_bridge_process_requires_nonce_proof_and_exact_external_grant() {
    let directory = std::env::temp_dir().join(format!(
        "syndocal-agent-auth-test-{}",
        storage::random_hex(12).unwrap()
    ));
    std::fs::create_dir(&directory).unwrap();
    let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured_for_emit = std::sync::Arc::clone(&captured);
    let service = authority::AgentAuthorityService::new();
    let pairing = service.begin_pairing("client-a").unwrap();
    let approval = service
        .approve_pairing(&pairing.challenge_id, &pairing.challenge)
        .unwrap();
    service
        .grant(
            "client-a",
            approval.principal_incarnation,
            protocol::agent_authority::AgentGrant::new(
                protocol::control_plane_registry_v2::AdapterKind::ExternalMcp,
                protocol::agent_authority::AgentCapability::Read,
                "syndocal.query.agent_bridge.fixtures.list.v1",
                None,
            )
            .unwrap(),
        )
        .unwrap();
    let lock = storage::instance_lock(&directory).unwrap();
    let inner = Inner {
        token: "bridge-token".to_string(),
        session_nonce: "a".repeat(64),
        authority: service,
        ledger: Mutex::new(ledger::Ledger::new(None).unwrap()),
        auth_nonces: Mutex::new(std::collections::BTreeSet::new()),
        emit: Box::new(move |dispatch| {
            captured_for_emit.lock().unwrap().push(dispatch.clone());
            Ok(())
        }),
        connections: AtomicUsize::new(0),
        _lock: lock,
        descriptor_path: directory.join("missing-descriptor.json"),
    };
    inner.ledger.lock().unwrap().register().unwrap();
    let request_id = id(91);
    let client_nonce = "b".repeat(64);
    let method = "fixtures.list";
    let auth = wire::Auth {
        principal_id: "client-a".to_string(),
        principal_incarnation: approval.principal_incarnation,
        client_nonce: client_nonce.clone(),
        proof: proof(
            &approval.credential,
            &authority::auth_proof_message(
                &inner.session_nonce,
                &client_nonce,
                &request_id,
                method,
            ),
        ),
    };
    let request = Request {
        token: inner.token.clone(),
        request_id: request_id.clone(),
        method: method.to_string(),
        params: serde_json::json!({}),
        auth: Some(auth.clone()),
    };
    let missing_auth = serde_json::to_vec(&Request {
        auth: None,
        ..request.clone()
    })
    .unwrap();
    assert_eq!(
        process(&inner, &missing_auth).error.as_deref(),
        Some("agent_authentication_required")
    );
    let mut invalid_auth = request.clone();
    invalid_auth.auth.as_mut().unwrap().proof = "0".repeat(64);
    assert_eq!(
        process(&inner, &serde_json::to_vec(&invalid_auth).unwrap())
            .error
            .as_deref(),
        Some("agent_auth_proof_invalid")
    );
    let response = process(&inner, &serde_json::to_vec(&request).unwrap());
    assert_eq!(response.status, "pending");
    assert_eq!(captured.lock().unwrap()[0].principal_id, "client-a");
    assert_eq!(
        process(&inner, &serde_json::to_vec(&request).unwrap())
            .error
            .as_deref(),
        Some("agent_auth_nonce_replayed")
    );
    drop(inner);
    std::fs::remove_dir_all(&directory).unwrap();
}
#[test]
fn agent_bridge_claim_is_exact_once_and_replay_never_dispatches_twice() {
    let mut ledger = ledger::Ledger::new(None).unwrap();
    let command = command("fixtures.set_transform");
    assert_eq!(
        ledger.begin(&id(1), &command).unwrap().0.error.as_deref(),
        Some("not_available")
    );
    let generation = ledger.register().unwrap();
    let (pending, dispatch) = ledger.begin(&id(1), &command).unwrap();
    assert_eq!(pending.status, "pending");
    assert!(dispatch.is_some());
    assert!(ledger
        .complete(generation, &id(1), serde_json::json!({}))
        .is_err());
    assert!(ledger.claim(generation + 1, &id(1)).is_err());
    assert!(ledger.claim(generation, &id(2)).is_err());
    assert!(ledger.claim(generation, &id(1)).is_ok());
    assert!(ledger.claim(generation, &id(1)).is_err());
    assert!(ledger.begin(&id(1), &command).unwrap().1.is_none());
    ledger
        .complete(generation, &id(1), serde_json::json!({"ok":true}))
        .unwrap();
    let (terminal, redispatch) = ledger.begin(&id(1), &command).unwrap();
    assert_eq!(terminal.status, "completed");
    assert!(redispatch.is_none());
    assert_eq!(
        ledger
            .begin(&id(1), &self::command("fixtures.get"))
            .unwrap()
            .0
            .error
            .as_deref(),
        Some("request_conflict")
    );
}
#[test]
fn agent_bridge_video_blackout_is_a_deduplicated_mutation() {
    let mut ledger = ledger::Ledger::new(None).unwrap();
    let generation = ledger.register().unwrap();
    let command = command("output.set_video_blackout");
    let (pending, dispatch) = ledger.begin(&id(1), &command).unwrap();
    assert_eq!(pending.status, "pending");
    let dispatch = dispatch.unwrap();
    assert_eq!(dispatch.method, "output.set_video_blackout");
    assert_eq!(
        dispatch.params,
        serde_json::json!({
            "enabled": true,
            "expectedProject": {
                "project_epoch": 0,
                "project_revision": 0,
                "checkpoint_hash": "a".repeat(64)
            }
        })
    );
    ledger.claim(generation, &id(1)).unwrap();
    ledger
        .complete(generation, &id(1), serde_json::json!({"ok": true}))
        .unwrap();
    let (completed, redispatch) = ledger.begin(&id(1), &command).unwrap();
    assert_eq!(completed.status, "completed");
    assert!(redispatch.is_none());
    let changed = match command.clone() {
        Command::SetVideoBlackout(mut value) => {
            value.enabled = false;
            Command::SetVideoBlackout(value)
        }
        _ => unreachable!(),
    };
    let (conflict, redispatch) = ledger.begin(&id(1), &changed).unwrap();
    assert_eq!(conflict.error.as_deref(), Some("request_conflict"));
    assert!(redispatch.is_none());
}
#[test]
fn agent_bridge_video_blackout_mutation_survives_reload_without_redispatch() {
    let directory = std::env::temp_dir().join(format!(
        "syndocal-agent-video-test-{}",
        storage::random_hex(12).unwrap()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("ledger.json");
    let command = command("output.set_video_blackout");
    {
        let mut ledger = ledger::Ledger::new(Some(path.clone())).unwrap();
        ledger.register().unwrap();
        let (pending, dispatch) = ledger.begin(&id(1), &command).unwrap();
        assert_eq!(pending.status, "pending");
        assert!(dispatch.is_some());
    }
    let mut restarted = ledger::Ledger::new(Some(path.clone())).unwrap();
    restarted.register().unwrap();
    let (unknown, redispatch) = restarted.begin(&id(1), &command).unwrap();
    assert_eq!(unknown.status, "unknown");
    assert!(redispatch.is_none());
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn agent_bridge_canonical_reads_are_not_persisted_mutations_but_writes_are() {
    let directory = std::env::temp_dir().join(format!(
        "syndocal-agent-canonical-test-{}",
        storage::random_hex(12).unwrap()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("ledger.json");
    let mut ledger = ledger::Ledger::new(Some(path.clone())).unwrap();
    let generation = ledger.register().unwrap();
    let read = command("control_plane.execute");
    let (pending, dispatch) = ledger.begin(&id(1), &read).unwrap();
    assert_eq!(pending.status, "pending");
    assert!(dispatch.is_some());
    ledger.claim(generation, &id(1)).unwrap();
    ledger
        .complete(generation, &id(1), serde_json::json!({"ok": true}))
        .unwrap();
    let write = Command::ControlPlaneExecute(wire::CanonicalOperation {
        operation_id: "syndocal.output.enable.v2".to_string(),
        request: serde_json::json!({"request_id": id(2)}),
    });
    ledger.begin(&id(2), &write).unwrap();
    drop(ledger);
    let mut restarted = ledger::Ledger::new(Some(path.clone())).unwrap();
    restarted.register().unwrap();
    let (read_again, read_dispatch) = restarted.begin(&id(1), &read).unwrap();
    assert_eq!(read_again.status, "pending");
    assert!(read_dispatch.is_some());
    let (write_again, write_dispatch) = restarted.begin(&id(2), &write).unwrap();
    assert_eq!(write_again.status, "unknown");
    assert!(write_dispatch.is_none());
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
}
#[test]
fn agent_bridge_reload_fences_late_completion_and_does_not_replay() {
    let mut ledger = ledger::Ledger::new(None).unwrap();
    let generation = ledger.register().unwrap();
    let command = command("fixtures.set_transform");
    ledger.begin(&id(1), &command).unwrap();
    ledger.claim(generation, &id(1)).unwrap();
    let newer = ledger.register().unwrap();
    assert!(newer > generation);
    assert_eq!(ledger.status(&id(1)).status, "unknown");
    assert!(ledger
        .complete(generation, &id(1), serde_json::json!({"ok":true}))
        .is_err());
    assert!(ledger
        .complete(newer, &id(1), serde_json::json!({"ok":true}))
        .is_err());
    assert!(ledger.begin(&id(1), &command).unwrap().1.is_none());
}
#[test]
fn agent_bridge_capacity_preserves_inflight_and_mutation_tombstones() {
    let mut ledger = ledger::Ledger::new(None).unwrap();
    let generation = ledger.register().unwrap();
    let command = command("fixtures.set_transform");
    for value in 1..=64 {
        ledger.begin(&id(value), &command).unwrap();
    }
    assert_eq!(
        ledger.begin(&id(65), &command).unwrap_err(),
        "inflight_capacity"
    );
    for value in 1..=64 {
        assert_eq!(ledger.status(&id(value)).status, "pending");
    }
    ledger.claim(generation, &id(1)).unwrap();
    ledger
        .complete(generation, &id(1), serde_json::json!({}))
        .unwrap();
    ledger.begin(&id(65), &command).unwrap();
    let (old, redispatch) = ledger.begin(&id(1), &command).unwrap();
    assert_eq!(old.status, "unknown");
    assert!(redispatch.is_none());
}
#[test]
fn agent_bridge_oversize_result_cannot_be_reported_completed() {
    let mut ledger = ledger::Ledger::new(None).unwrap();
    let generation = ledger.register().unwrap();
    ledger
        .begin(&id(1), &self::command("fixtures.get"))
        .unwrap();
    ledger.claim(generation, &id(1)).unwrap();
    assert_eq!(
        ledger.complete(
            generation,
            &id(1),
            serde_json::json!("x".repeat(wire::MAX_RESULT_BYTES))
        ),
        Err("result_oversize".to_string())
    );
    assert_eq!(ledger.status(&id(1)).status, "pending");
}
#[test]
fn agent_bridge_restart_retains_mutation_identity_without_automatic_replay() {
    let directory = std::env::temp_dir().join(format!(
        "syndocal-agent-test-{}",
        storage::random_hex(12).unwrap()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("ledger.json");
    {
        let mut ledger = ledger::Ledger::new(Some(path.clone())).unwrap();
        ledger.register().unwrap();
        ledger
            .begin(&id(1), &command("fixtures.set_transform"))
            .unwrap();
        assert!(path.exists());
    }
    let mut restarted = ledger::Ledger::new(Some(path.clone())).unwrap();
    restarted.register().unwrap();
    assert_eq!(restarted.status(&id(1)).status, "unknown");
    assert!(restarted
        .begin(&id(1), &command("fixtures.set_transform"))
        .unwrap()
        .1
        .is_none());
    drop(restarted);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
}
