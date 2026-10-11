use super::*;

fn runtime() -> EngineRuntime {
    let mut runtime = EngineRuntime::new(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    for (id, address) in [(1, 1), (2, 4), (3, 20)] {
        runtime.apply_command(EngineCommand::PatchFixture {
            fixture_id: id,
            request: sample_patch_request(&format!("F{id}"), address),
            profile: sample_profile(),
        });
    }
    runtime
}
fn change(id: u64, from: u16, to: u16) -> FixturePatchChange {
    FixturePatchChange {
        fixture_id: id,
        expected_universe: 0,
        expected_address: from,
        expected_footprint: 3,
        label: format!("F{id}"),
        universe: 0,
        address: to,
    }
}
#[test]
fn fixture_repatch_moves_across_own_old_ranges_and_preserves_values() {
    let mut runtime = runtime();
    runtime.apply_command(EngineCommand::SetAttribute {
        fixture_id: 1,
        attribute: "Dimmer".into(),
        value: 42_000,
    });
    let values = runtime.values.clone();
    fixture_repatch::apply(
        &mut runtime.fixtures,
        vec![change(1, 1, 4), change(2, 4, 7)],
    )
    .unwrap();
    assert_eq!(
        runtime
            .fixtures
            .iter()
            .map(|f| f.request.address)
            .collect::<Vec<_>>(),
        vec![4, 7, 20]
    );
    assert_eq!(runtime.values, values);
}
#[test]
fn fixture_repatch_invalid_last_member_never_moves_first_member() {
    for changes in [
        vec![change(1, 1, 4), change(2, 4, 20)],
        vec![change(1, 1, 4), change(2, 4, 512)],
        vec![change(1, 1, 4), change(2, 9, 7)],
        vec![change(1, 1, 4), change(1, 1, 7)],
        vec![change(1, 1, 4), change(9, 4, 7)],
        vec![change(1, 1, 4), change(2, 4, 5)],
    ] {
        let mut runtime = runtime();
        let before = runtime.build_persistence_snapshot();
        assert!(fixture_repatch::apply(&mut runtime.fixtures, changes).is_err());
        assert_eq!(
            runtime.build_persistence_snapshot().fixtures,
            before.fixtures
        );
    }
}
#[test]
fn fixture_repatch_publication_failure_and_expiry_restore_a() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    engine
        .patch_fixtures_published(vec![
            FixturePatchCandidate {
                fixture_id: 1,
                request: sample_patch_request("F1", 1),
                profile: sample_profile(),
            },
            FixturePatchCandidate {
                fixture_id: 2,
                request: sample_patch_request("F2", 4),
                profile: sample_profile(),
            },
        ])
        .unwrap();
    let before = engine.persistence_snapshot().unwrap();
    engine.force_next_pending_publication_failure_for_tests();
    assert!(engine
        .set_fixture_patches_published_with_timeout(
            vec![change(1, 1, 4), change(2, 4, 7)],
            Duration::from_secs(2)
        )
        .is_err());
    assert_eq!(
        engine.persistence_snapshot().unwrap().fixtures,
        before.fixtures
    );
    assert!(engine
        .set_fixture_patches_published_with_timeout(vec![change(1, 1, 4)], Duration::ZERO)
        .is_err());
    assert_eq!(
        engine.persistence_snapshot().unwrap().fixtures,
        before.fixtures
    );
    engine
        .set_fixture_patches_published_with_timeout(
            vec![change(1, 1, 4), change(2, 4, 7)],
            Duration::from_secs(2),
        )
        .unwrap();
    assert_eq!(
        engine
            .snapshot()
            .fixtures
            .iter()
            .map(|f| f.address)
            .collect::<Vec<_>>(),
        vec![4, 7]
    );
}
