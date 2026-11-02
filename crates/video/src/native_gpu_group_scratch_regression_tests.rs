//! Real-decoder, production-presenter budget regression. Offscreen target only:
//! this proves residency/budget, never physical scanout or native-window FPS.
use super::*;
use crate::gpu_surface::native::{
    AdmittedPhysicalGeometry, NativePresentTarget, TargetAcquireError,
};
use crate::native_gpu_output::{NativePresentPermit, NativePresentSink, PresentOutcome};

struct Offscreen(wgpu::Texture);
impl NativePresentTarget for Offscreen {
    type Frame = wgpu::Texture;
    fn format(&self) -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba8Unorm
    }
    fn configure(&mut self, _: &wgpu::Device, _: (u32, u32)) {}
    fn acquire(&mut self) -> Result<Self::Frame, TargetAcquireError> {
        Ok(self.0.clone())
    }
    fn view(&self, frame: &Self::Frame) -> wgpu::TextureView {
        frame.create_view(&Default::default())
    }
    fn present(&mut self, _: Self::Frame) {}
    fn discard(&mut self, _: Self::Frame) {}
}

#[test]
#[ignore = "QA real C4 media, C5 groupfx fixture and 2560x1600 production presenter"]
fn native_gpu_group_fx_production_presenter_scratch_budget_regression() {
    let media = std::env::var("SYNDOCAL_QA_C4_MEDIA").expect("C4 H264 media required");
    let fixture = std::env::var("SYNDOCAL_QA_GROUPFX_FIXTURE").expect("C5 groupfx .sdc required");
    let project: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fixture).unwrap()).unwrap();
    let mut snapshot: VideoSnapshot =
        serde_json::from_value(project["snapshot"]["video"].clone()).unwrap();
    assert_eq!(snapshot.layers.len(), 3);
    assert_eq!(snapshot.layer_groups.len(), 1);
    let group = &snapshot.layer_groups[0];
    assert_eq!(group.layer_ids, [1, 2, 3]);
    assert_eq!(group.blend_mode, protocol::VideoBlendMode::Screen);
    assert_eq!(group.transform.as_ref().unwrap().rotation_deg, 10.0);
    // Preserve authored FX/group geometry; remove only slot/asset indirection,
    // exactly as in the task95 causal reproduction. Real decoder receives the
    // requested 2560x1600 extent instead of a forced 1920x1080 test-provider input.
    for layer in &mut snapshot.layers {
        layer.source.path = Some(media.clone());
        layer.clip_slots.clear();
        layer.default_clip_slot_id = None;
        layer.media_asset_id = None;
    }
    snapshot.media_assets.clear();
    // Use this harness's output identity; authored pixel policy is unchanged.
    for output in &mut snapshot.outputs {
        output.id = OUTPUT;
    }
    for composition in &mut snapshot.compositions {
        composition.output_ids = vec![OUTPUT];
    }
    let mut session = session();
    let mut renderer = renderer();
    let image = session
        .owner
        .context()
        .device()
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("Group FX regression offscreen surface"),
            size: wgpu::Extent3d {
                width: EXTENT.0,
                height: EXTENT.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
    let mut presenter = SharedNativePresenter::new(Offscreen(image), &mut session.owner).unwrap();
    presenter
        .configure_admitted(
            AdmittedPhysicalGeometry {
                extent: EXTENT,
                geometry_generation: session.geometry_generation,
            },
            &mut session.owner,
        )
        .unwrap();
    let clips = protocol::VideoClipRuntimeSnapshot::default();
    let transitions = VideoLayerTransitionRuntimeSnapshot::default();
    let started = std::time::Instant::now();
    let mut live_high = 0;
    let mut resident_ticks = 0;
    for tick in 0..TICKS {
        presenter.poll_retirements(&mut session.owner).unwrap();
        for layer in &mut snapshot.layers {
            layer.state.position_ms = tick * 1000 / 60;
        }
        let frame = session
            .prepare_output(
                &mut renderer,
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &clips,
                    project_render_epoch: EPOCH,
                    audio_frame: None,
                },
                &transitions,
                EXTENT,
                &|| false,
                &Permit,
            )
            .unwrap_or_else(|error| panic!("group FX tick {tick}: {error}"));
        match frame.content {
            super::super::present::ResidentDisplayContent::Resident(ready) => {
                resident_ticks += 1;
                let authority = super::super::present::ResidentAuthority(&Permit);
                let mut sink = presenter.sink(&mut session.owner).unwrap();
                assert!(matches!(
                    sink.present_once(ready, NativePresentPermit::new(&authority, &|| false))
                        .unwrap(),
                    PresentOutcome::Presented(_)
                ));
            }
            // Both matched task95 probes can be in flight during startup.
            // Every tick after the same 30-tick warmup must be resident.
            super::super::present::ResidentDisplayContent::InFlight if tick < 30 => {}
            _ => panic!("group FX tick {tick} left steady resident delivery"),
        }
        let budget = session.owner.budget_snapshot();
        live_high = live_high.max(budget.allocated_bytes);
        println!(
            "group FX tick={tick} live={} ledger_high={}",
            budget.allocated_bytes, budget.high_water_bytes
        );
        let due = started + std::time::Duration::from_nanos((tick + 1) * (1_000_000_000 / 60));
        if let Some(left) = due.checked_duration_since(std::time::Instant::now()) {
            std::thread::sleep(left);
        }
    }
    let evidence = session.path_evidence();
    assert!(resident_ticks >= TICKS - 30);
    assert_eq!(evidence.cpu_declines, 0);
    assert_eq!(evidence.budget_declines, 0);
    assert_eq!(evidence.gpu_batch_ticks, TICKS);
    assert!(
        session.owner.budget_snapshot().high_water_bytes
            <= crate::native_gpu_budget::NATIVE_MANAGED_LIMIT
    );
    println!(
        "group FX live high water={live_high} ledger={:?} evidence={evidence:?}",
        session.owner.budget_snapshot()
    );
    release_shared_presenter_for_stop(&mut presenter, &mut session.owner).unwrap();
    session.begin_stop_with_renderer(&mut renderer).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !session.poll_stop().unwrap() {
        assert!(
            std::time::Instant::now() < deadline,
            "regression Stop did not drain"
        );
        std::thread::yield_now();
    }
}
