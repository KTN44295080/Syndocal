//! Route-exit regressions on real headless owners. No surface or receipt is fabricated.
use super::*;

fn at(ring: &mut RingFixture, position: u64) -> NativeResidentDisplayFrame {
    for layer in &mut ring.snapshot.layers {
        layer.state.position_ms = position;
    }
    prepare_with(
        &mut ring.session,
        &mut ring.renderer,
        &ring.snapshot,
        EXTENT,
        &Permit,
    )
    .unwrap_or_else(|error| panic!("tick at {position} failed: {error}"))
}

fn pass(ring: &RingFixture) -> u64 {
    let Some(ResidentHistoryState::Active(active)) = &ring.session.history else {
        panic!("resident history absent")
    };
    active.session.history(0).unwrap().scope().queue_epoch
}

fn follow(ring: &mut RingFixture) -> NativeResidentDisplayFrame {
    let clips = protocol::VideoClipRuntimeSnapshot::default();
    ring.session
        .prepare_follow_output(
            &mut ring.renderer,
            &ring.snapshot,
            &ring.snapshot,
            VideoEffectRenderContext {
                clip_runtime: &clips,
                project_render_epoch: EPOCH,
                audio_frame: None,
            },
            &VideoLayerTransitionRuntimeSnapshot::default(),
            protocol::VideoClipTakeKind::Crossfade,
            protocol::VideoLayerTransitionCurve::Linear,
            500,
            protocol::TimelineId(55),
            1,
            None,
            EXTENT,
            &|| false,
            &Permit,
        )
        .unwrap_or_else(|error| panic!("same-key Follow failed: {error}"))
}

fn handover_pressure(ring: &mut RingFixture) -> crate::native_gpu_budget::FrameReservation {
    let frame = NativeFrameId {
        output_id: OUTPUT,
        project_epoch: EPOCH,
        request_sequence: 900,
        geometry_generation: ring.session.geometry_generation(),
    };
    let mut plan = crate::native_gpu_budget::NativeAllocationPlan::new(frame);
    let pixel_bytes = u64::from(EXTENT.0) * u64::from(EXTENT.1) * 4;
    plan.add_new(
        ChargeKind::SerialCpuAdmission,
        crate::native_gpu_budget::NATIVE_MANAGED_LIMIT - 32 * pixel_bytes,
    )
    .unwrap();
    let quote = ring
        .session
        .owner
        .quote(&plan)
        .expect("steady fixture fits inside the 32P envelope");
    ring.session.owner.reserve(quote).unwrap()
}

#[test]
fn native_gpu_output_display_task87_cpu_import_selection_preserves_pass_and_host_ledger() {
    let mut ring = RingFixture::new(3, EXTENT);
    ring.renderer.frame_provider.pts_grid = Some(33);
    ring.warm(EXTENT);
    // The exact same source/output/key first takes the canonical CPU route.
    // A disabled ISF chain still leaves the canonical post-FX queue populated.
    let effects = ring
        .snapshot
        .layers
        .iter_mut()
        .map(|layer| layer.isf_effect.take())
        .collect::<Vec<_>>();
    for index in 0..12 {
        assert_eq!(path_of(at(&mut ring, index * 16)), Path::Cpu);
    }
    for (layer, effect) in ring.snapshot.layers.iter_mut().zip(effects) {
        layer.isf_effect = effect;
    }
    let host = ring.session.history_host.as_ref().unwrap().clone();
    // Select incoming first, establishing a resident pass; then seek back one
    // 30 fps source interval. At 208 ms incoming is 231 ms and the canonical
    // at-or-before entry is imported CPU post-FX at 198 ms.
    assert_eq!(path_of(at(&mut ring, 231)), Path::Resident);
    let original_pass = pass(&ring);
    let mut saw_cpu = false;
    for position in [208, 224, 240, 256] {
        let frame = at(&mut ring, position);
        if position == 208 {
            let ResidentDisplayContent::Cpu(cpu) = &frame.content else {
                panic!("imported CPU history must enter its canonical continuation, pass {original_pass}");
            };
            assert_eq!(
                cpu.owner.frame.pts_ms, 198,
                "imported at-or-before pixels were discarded"
            );
        }
        let path = path_of(frame);
        assert!(matches!(path, Path::Resident | Path::InFlight | Path::Cpu));
        saw_cpu |= path == Path::Cpu;
        assert!(
            ring.session.retiring_histories.is_empty(),
            "CPU-backed canonical selection parked imported owners at {position}"
        );
        if let Some(current) = &ring.session.history_host {
            assert_eq!(
                host.snapshot().unique_bytes,
                current.snapshot().unique_bytes,
                "import detached the retained host ledger"
            );
        }
        assert!(
            ring.session.owner.budget_snapshot().managed_bytes()
                <= crate::native_gpu_budget::NATIVE_MANAGED_LIMIT
        );
    }
    assert!(
        saw_cpu,
        "canonical CPU-backed history selection must exercise continuation"
    );
    stop_and_reap(&mut ring.session, &mut ring.renderer);
}

// Build a real retired high-PTS pass, using the same first-at-or-after decoder
// semantics as the existing loop-wrap regression, and stop at its first tick.
fn with_retired_pass() -> RingFixture {
    let mut ring = RingFixture::new(3, EXTENT);
    ring.renderer.frame_provider.pts_grid = Some(33);
    for index in 0..150 {
        drop(at(&mut ring, index * 85 % 1200));
        if !ring.session.retiring_histories.is_empty() {
            return ring;
        }
    }
    panic!("fixture never produced a genuinely retired history pass")
}

#[test]
fn native_gpu_output_display_task87_retired_histories_drain_on_cpu_follow_and_geometry_exits() {
    for exit in 0..3 {
        let mut ring = with_retired_pass();
        assert!(!ring.session.retiring_histories.is_empty());
        match exit {
            0 => {
                for layer in &mut ring.snapshot.layers {
                    layer.isf_effect = None;
                }
                assert_eq!(path_of(at(&mut ring, 1201)), Path::Cpu);
            }
            1 => {
                assert_eq!(path_of(follow(&mut ring)), Path::Cpu);
            }
            _ => {
                ring.session.reserve_geometry(48, 30).unwrap();
            }
        }
        assert!(
            ring.session.retiring_histories.is_empty(),
            "exit {exit} retained old history owners"
        );
        stop_and_reap(&mut ring.session, &mut ring.renderer);
    }
}

#[test]
fn native_gpu_output_display_task87_recycled_snapshot_and_idle_pools_release_on_cpu_and_follow() {
    for use_follow in [false, true] {
        let mut ring = RingFixture::new(3, EXTENT);
        for _ in 0..12 {
            drop(ring.prepare(EXTENT));
        }
        assert!(ring.session.cache.recycled_commits() > 0);
        assert!(ring.session.owner.pooled_resource_count() > 0);
        let key = ring
            .session
            .cache
            .last_valid_with_owner()
            .unwrap()
            .0
            .clone();
        // The 32P pressure isolates the CPU history handover. Follow composes
        // two sides beyond that envelope, so it runs without pressure and
        // asserts the release directly.
        let pressure = (!use_follow).then(|| handover_pressure(&mut ring));
        for layer in &mut ring.snapshot.layers {
            layer.isf_effect = None;
        }
        let frame = if use_follow {
            follow(&mut ring)
        } else {
            at(&mut ring, 208)
        };
        let ResidentDisplayContent::Cpu(cpu) = &frame.content else {
            panic!("exit must return actual CPU output")
        };
        assert_eq!(cpu.request.key, key, "route exit changed the request key");
        drop(frame);
        assert!(
            ring.session.cache.take_recycled_cpu_snapshot().is_none(),
            "idle LastValid buffer survived route exit"
        );
        assert_eq!(
            ring.session.owner.pooled_resource_count(),
            0,
            "idle resident pools survived CPU handover"
        );
        if let Some(pressure) = pressure {
            ring.session
                .owner
                .discard_unsubmitted_reservation(pressure)
                .unwrap();
        }
        stop_and_reap(&mut ring.session, &mut ring.renderer);
    }
}

#[test]
fn native_gpu_output_display_task87_abandon_releases_only_the_idle_recycled_charge() {
    let mut ring = RingFixture::new(3, EXTENT);
    ring.renderer.frame_provider.pts_grid = Some(33);
    for index in 0..12 {
        drop(at(&mut ring, index * 16));
    }
    assert!(ring.session.cache.recycled_commits() > 0);
    let key = ring
        .session
        .cache
        .last_valid_with_owner()
        .unwrap()
        .0
        .clone();
    let before = ring
        .session
        .owner
        .budget_snapshot()
        .for_kind(ChargeKind::CpuSnapshot)
        .active_bytes;
    // The steady ring usually holds the recycled buffer in its submitted
    // snapshot; only an idle one may leave the ledger here.
    let idle = ring.session.cache.has_recycled_cpu_snapshot();
    let pending = ring.session.has_pipelined_output();
    assert!(pending, "abandon must retain a real submitted owner");
    ring.session.abandon_pipelined_output();
    assert!(ring.session.has_pipelined_output());
    // A dropped charge leaves the ledger at the owner's next reap.
    ring.session.owner.poll_retirement().unwrap();
    let after = ring
        .session
        .owner
        .budget_snapshot()
        .for_kind(ChargeKind::CpuSnapshot)
        .active_bytes;
    let pixel_bytes = u64::from(EXTENT.0) * u64::from(EXTENT.1) * 4;
    assert_eq!(
        before - after,
        if idle { pixel_bytes } else { 0 },
        "idle recycled pixels, rather than valid or pending snapshots, must leave the ledger"
    );
    assert!(ring.session.cache.take_recycled_cpu_snapshot().is_none());
    for layer in &mut ring.snapshot.layers {
        layer.isf_effect = None;
    }
    let frame = at(&mut ring, 208);
    let ResidentDisplayContent::Cpu(cpu) = &frame.content else {
        panic!("abandoned resident tick must drain into canonical CPU output");
    };
    assert_eq!(cpu.request.key, key);
    drop(frame);
    stop_and_reap(&mut ring.session, &mut ring.renderer);
}

#[test]
fn native_gpu_output_display_task87_stage_a_fixed_clock_recovery_releases_idle_storage() {
    let mut ring = RingFixture::new(3, EXTENT);
    for _ in 0..12 {
        drop(ring.prepare(EXTENT));
    }
    let key = ring
        .session
        .cache
        .last_valid_with_owner()
        .unwrap()
        .0
        .clone();
    let recoveries = ring.session.path_evidence().recoveries;
    // As in isf_native_gpu_middle_stage_failure_preserves_neighbors_fault_identity_and_clock,
    // poison one genuinely prepared shader, then let G encode and recover it.
    // The unique source prevents reuse of an already compiled GPU pipeline.
    let effect = ring.snapshot.layers[1].isf_effect.as_mut().unwrap();
    effect.source = format!(
        "{}\n// task87 genuine Stage-A validation failure",
        effect.source
    )
    .into();
    let source = effect.source.clone();
    drop(ring.renderer.cached_isf_shader(&source, None, &[]).unwrap());
    let cached = ring
        .renderer
        .isf_shader_cache
        .get_mut(&(source, None, Vec::new()))
        .unwrap();
    let shader = cached.value.as_mut().unwrap();
    Arc::make_mut(shader).wgsl = "not valid WGSL".into();
    // Require a genuinely pending failed batch. A synchronous recovery cannot
    // stand in for Stage A; if the eager poll already recovered it this setup
    // must be refined, rather than silently passing a different scenario.
    drop(ring.prepare(EXTENT));
    assert_eq!(
        ring.session.path_evidence().recoveries,
        recoveries,
        "fault resolved synchronously; Stage-A scenario was not established"
    );
    assert!(
        ring.session.has_pipelined_output(),
        "faulty batch must remain in flight"
    );
    let pressure = handover_pressure(&mut ring);
    // The next tick first resolves the faulty predecessor in Stage A, then
    // uses the CPU route, preserving the physical request key.
    for layer in &mut ring.snapshot.layers {
        layer.isf_effect = None;
    }
    let frame = ring.prepare(EXTENT);
    let ResidentDisplayContent::Cpu(cpu) = &frame.content else {
        panic!("the tick after Stage-A recovery must return canonical CPU output");
    };
    assert_eq!(cpu.request.key, key, "recovery changed the request key");
    drop(frame);
    assert!(ring.session.path_evidence().recoveries > recoveries);
    assert!(
        ring.session.cache.take_recycled_cpu_snapshot().is_none(),
        "Stage A kept recycled LastValid"
    );
    assert_eq!(
        ring.session.owner.pooled_resource_count(),
        0,
        "Stage A kept idle GPU pools"
    );
    assert!(ring.session.retiring_histories.is_empty());
    ring.session
        .owner
        .discard_unsubmitted_reservation(pressure)
        .unwrap();
    stop_and_reap(&mut ring.session, &mut ring.renderer);
}

#[test]
fn native_gpu_output_display_task87_repeated_seek_passes_never_fault_and_stay_bounded() {
    let mut ring = RingFixture::new(3, EXTENT);
    ring.renderer.frame_provider.pts_grid = Some(33);
    drop(at(&mut ring, 0));
    // Every tick must publish (`at` panics on any output fault). Each pass
    // owns its host ledger; retiring passes drain one entry per input per
    // tick, so their total stays within a second history window.
    let mut previous = pass(&ring);
    let mut changed = 0;
    for index in 1..150 {
        drop(at(&mut ring, index * 85 % 1200));
        let current = pass(&ring);
        changed += usize::from(current != previous);
        previous = current;
        let host = ring.session.history_host.as_ref().unwrap().snapshot();
        assert!(host.unique_bytes <= host.limit);
        let retiring: usize = ring
            .session
            .retiring_histories
            .iter()
            .map(|history| history.metadata().len())
            .sum();
        assert!(
            retiring <= 3 * crate::frame_history_policy::NATIVE_HISTORY_CAPACITY,
            "retiring passes accumulated {retiring} entries at tick {index}"
        );
    }
    assert!(changed >= 2, "repeated passes were not exercised");
    stop_and_reap(&mut ring.session, &mut ring.renderer);
}
