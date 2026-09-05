use super::*;
fn id(value: usize) -> String {
    format!("00000000-0000-4000-8000-{value:012x}")
}
fn command(method: &str) -> Command {
    let params = match method {
        "fixtures.list" => serde_json::json!({}),
        "fixtures.get" => serde_json::json!({"fixtureId": 1}),
        _ => {
            serde_json::json!({"fixtureId":1,"position":{"x":1,"y":2,"z":3},"rotation":{"pitch":0,"yaw":0,"roll":0},"expectedProject":{"project_epoch":0,"project_revision":0,"checkpoint_hash":"a".repeat(64)}})
        }
    };
    Request {
        token: "token".to_string(),
        request_id: id(1),
        method: method.to_string(),
        params,
    }
    .command()
    .unwrap()
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
    assert!(matches!(serde_json::from_value::<Request>(runtime.clone()).unwrap().command().unwrap(), Command::RuntimeGet(_)));
    runtime["params"] = serde_json::json!({"enableOutputs": true});
    assert!(serde_json::from_value::<Request>(runtime).unwrap().command().is_err());
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
