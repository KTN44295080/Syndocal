//! Ordinary-fixture steady state on a real headless broker.
//!
//! One to three file layers, each with an executing ISF chain, a Composition
//! and a Display output. Every tick decodes a new provider frame. These tests
//! never construct a surface, Ready token or Presented receipt: the prepared
//! frame is the real facade result and is discarded exactly as a revoked tick
//! would discard it.
use super::present::ResidentDisplayContent;
use super::*;
use crate::native_gpu_budget::ChargeKind;
use crate::native_gpu_contract::GpuOwnerState;
use crate::{
    VideoEffectRenderContext, VideoFrame, VideoFrameProviderError, VideoPixelFormat,
    VideoRuntimeConfig,
};
use protocol::{VideoLayerTransitionRuntimeSnapshot, VideoSnapshot};
use std::time::{Duration, Instant};

const OUTPUT: u64 = 101;
const EPOCH: u64 = 7;
const EXTENT: (u32, u32) = (64, 40);
const TICKS: u64 = 40;
/// The C4 ordinary fixture: one top ISF pass plus two stacked passes.
const STAGES: usize = 3;

#[derive(Default)]
struct Provider {
    reads: usize,
}

impl VideoFrameProvider for Provider {
    fn retain_layers(&mut self, _: &[protocol::VideoLayerId]) {}

    fn frame_for_layer(
        &mut self,
        layer: &protocol::VideoLayerSummary,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoFrameProviderError> {
        self.reads += 1;
        let shade = (layer.state.position_ms / 16 % 200) as u8;
        Ok(VideoFrame {
            layer_id: layer.id,
            width,
            height,
            pts_ms: layer.state.position_ms,
            duration_ms: 16,
            format: VideoPixelFormat::Rgba8,
            data: [shade, 34, layer.id as u8 * 40, 255].repeat(width as usize * height as usize),
        })
    }

    fn layer_frame_inputs(
        &self,
        layer: &protocol::VideoLayerSummary,
        width: u32,
        height: u32,
    ) -> Vec<crate::VideoRenderInput> {
        vec![crate::legacy_layer_input(layer, width, height, None)]
    }
}

struct Permit;
impl NativeResidentPresentationAuthority for Permit {
    fn revalidate(&self, _: NativeResidentPresentBoundary) -> NativeResidentPresentDecision {
        NativeResidentPresentDecision::Permit
    }
}

fn session_with_compositor(extent: (u32, u32)) -> NativeResidentDisplaySession {
    let factory = NativeGpuFactory::new();
    let owner = factory
        .create_headless(OUTPUT, NativeGpuRequirements::default())
        .expect("steady-state tests require an actual headless adapter");
    let mut session = NativeResidentDisplaySession {
        output_id: OUTPUT,
        factory,
        owner: ResidentOwner(Some(owner)),
        presenter: None,
        compositor: None,
        cache: SharedNativeOutputCache::new(OUTPUT),
        geometry_generation: 0,
        reserved_geometry: None,
        history: None,
        terminal_fault: None,
        delivery: crate::native_gpu_output::NativeDeliveryCounter::default(),
        retired_ready: Vec::new(),
        retirements: Vec::new(),
        unsubmitted: Vec::new(),
        recovered_history: None,
        request_sequence: 0,
        history_configuration: None,
        input_bindings: Vec::new(),
        next_input_id: 1,
        history_host: None,
        follow_last_valid: None,
    };
    session.compositor = Some(
        NativeGpuCompositor::from_shared(&mut session.owner)
            .expect("the shared compositor initializes on the headless owner"),
    );
    // Headless sessions have no surface to configure; reserve and confirm the
    // exact geometry identity used by preparation.
    session.reserve_geometry(extent.0, extent.1).unwrap();
    session.reserved_geometry = None;
    session
}

fn scene(layers: u64, extent: (u32, u32)) -> VideoSnapshot {
    let source = crate::isf_native_gpu::tests::IDENTITY;
    let shader = crate::prepare_isf_shader(source).unwrap();
    let mut effect =
        crate::isf_effect_from_prepared("steady fixture".into(), source, None, &shader);
    let stage: protocol::VideoIsfEffectStageSummary =
        serde_json::from_value(serde_json::to_value(&effect).unwrap()).unwrap();
    effect.stack = vec![stage; STAGES - 1];
    VideoSnapshot {
        layers: (1..=layers)
            .map(|id| protocol::VideoLayerSummary {
                id,
                label: format!("steady layer {id}"),
                source: protocol::VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some(format!("{id}.mp4")),
                    name: None,
                    codec: None,
                    metadata: None,
                },
                media_asset_id: None,
                blend_mode: protocol::VideoBlendMode::Normal,
                state: protocol::VideoLayerState {
                    opacity: 0.7,
                    ..protocol::VideoLayerState::default()
                },
                isf_effect: Some(effect.clone()),
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
            })
            .collect(),
        media_assets: Vec::new(),
        effect_chains: Vec::new(),
        effect_presets: Vec::new(),
        layer_groups: Vec::new(),
        transition_buses: Vec::new(),
        compositions: vec![protocol::CompositionSummary {
            id: 70,
            label: "steady".into(),
            layer_ids: (1..=layers).collect(),
            timeline_layer_ids: Vec::new(),
            output_ids: vec![OUTPUT],
        }],
        outputs: vec![protocol::VideoOutputSummary {
            id: OUTPUT,
            label: "headless steady".into(),
            kind: protocol::VideoOutputKind::Display,
            enabled: true,
            composition_id: 70,
            fullscreen: false,
            monitor_id: Some(0),
            monitor_identity: None,
            width: extent.0,
            height: extent.1,
            endpoint_name: None,
            opacity: 1.0,
            blackout: false,
            mapping: protocol::VideoOutputMapping::default(),
        }],
        mapping_presets: Vec::new(),
        auto_vj: protocol::AutoVjSnapshot::default(),
        master_opacity: 1.0,
        blackout: false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Path {
    Resident,
    Cpu,
    HardBlackout,
    Rejected,
}

fn prepare_with(
    session: &mut NativeResidentDisplaySession,
    renderer: &mut VideoPreviewRenderer<Provider>,
    snapshot: &VideoSnapshot,
    extent: (u32, u32),
    authority: &impl NativeResidentPresentationAuthority,
) -> Result<NativeResidentDisplayFrame, NativeResidentPrepareError> {
    let clips = protocol::VideoClipRuntimeSnapshot::default();
    session.prepare_output(
        renderer,
        snapshot,
        VideoEffectRenderContext {
            clip_runtime: &clips,
            project_render_epoch: EPOCH,
            audio_frame: None,
        },
        &VideoLayerTransitionRuntimeSnapshot::default(),
        extent,
        &|| false,
        authority,
    )
}

fn tick(
    session: &mut NativeResidentDisplaySession,
    renderer: &mut VideoPreviewRenderer<Provider>,
    snapshot: &VideoSnapshot,
    extent: (u32, u32),
) -> Path {
    let frame = prepare_with(session, renderer, snapshot, extent, &Permit)
        .unwrap_or_else(|error| panic!("ordinary steady tick failed: {error}"));
    path_of(frame)
}

fn path_of(frame: NativeResidentDisplayFrame) -> Path {
    match frame.content {
        ResidentDisplayContent::Resident(_) => Path::Resident,
        ResidentDisplayContent::Cpu(_) => Path::Cpu,
        ResidentDisplayContent::HardBlackout(_) => Path::HardBlackout,
        ResidentDisplayContent::Rejected => Path::Rejected,
    }
}

fn budget_line(session: &NativeResidentDisplaySession, extent: (u32, u32)) -> (u64, String) {
    let snapshot = session.owner.budget_snapshot();
    let frame = u64::from(extent.0) * u64::from(extent.1) * 4;
    let kinds = [
        ("IsfScratch", ChargeKind::IsfScratch),
        ("ResidentLayer", ChargeKind::ResidentLayer),
        ("Composition", ChargeKind::CompositionBuffer),
        ("Mapping", ChargeKind::MappingBuffer),
        ("SnapshotReadback", ChargeKind::SnapshotReadback),
        ("UploadStaging", ChargeKind::QueueUploadStaging),
        ("ActiveCpuInput", ChargeKind::ActiveCpuInput),
        ("CpuSnapshot", ChargeKind::CpuSnapshot),
        ("CpuLastValid", ChargeKind::CpuLastValid),
        ("CachedEffect", ChargeKind::CachedEffectResource),
    ];
    let detail = kinds
        .iter()
        .filter_map(|(name, kind)| {
            let entry = snapshot.for_kind(*kind);
            let bytes = entry.active_bytes + entry.retired_bytes + entry.reserved_bytes;
            (bytes != 0).then(|| format!("{name}={:.2}P", bytes as f64 / frame as f64))
        })
        .collect::<Vec<_>>()
        .join(" ");
    (
        snapshot.managed_bytes(),
        format!(
            "live={:.2}P high_water={:.2}P allocations={} reservations={} [{detail}]",
            snapshot.managed_bytes() as f64 / frame as f64,
            snapshot.high_water_bytes as f64 / frame as f64,
            snapshot.allocation_count,
            snapshot.reservation_count
        ),
    )
}

fn stop_and_reap(
    session: &mut NativeResidentDisplaySession,
    renderer: &mut VideoPreviewRenderer<Provider>,
) {
    session.begin_stop_with_renderer(renderer).unwrap();
    let started = Instant::now();
    while !session.poll_stop().unwrap() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "steady fixture Stop failed to reap"
        );
        std::thread::yield_now();
    }
    assert_eq!(session.owner.state(), GpuOwnerState::Reaped);
    assert_eq!(session.owner.budget_snapshot().managed_bytes(), 0);
    assert_eq!(session.owner.budget_snapshot().reservation_count, 0);
}

fn run_ordinary_fixture(layers: u64, extent: (u32, u32), ticks: u64) -> (Vec<Path>, Vec<u64>) {
    let mut session = session_with_compositor(extent);
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        Provider::default(),
    );
    let mut snapshot = scene(layers, extent);
    let mut paths = Vec::new();
    let mut live = Vec::new();
    for index in 0..ticks {
        for layer in &mut snapshot.layers {
            layer.state.position_ms = index * 16;
        }
        paths.push(tick(&mut session, &mut renderer, &snapshot, extent));
        let (bytes, line) = budget_line(&session, extent);
        println!(
            "layers={layers} tick={index} path={:?} {line}",
            paths[index as usize]
        );
        live.push(bytes);
    }
    assert_eq!(renderer.frame_provider.reads, (ticks * layers) as usize);
    stop_and_reap(&mut session, &mut renderer);
    (paths, live)
}

#[test]
fn native_gpu_output_display_ordinary_fixture_stays_resident_with_steady_budget() {
    let runs = (1..=3)
        .map(|layers| (layers, run_ordinary_fixture(layers, EXTENT, TICKS)))
        .collect::<Vec<_>>();
    for (layers, (paths, live)) in runs {
        assert!(
            paths.iter().all(|path| *path == Path::Resident),
            "{layers}-layer ordinary fixture left the resident path: {paths:?}"
        );
        // After the eight-entry history window fills, retained charges must be
        // a steady state rather than growing with the number of presents.
        let settled = &live[16..];
        assert!(
            settled.iter().all(|bytes| *bytes == settled[0]),
            "{layers}-layer managed bytes are not steady: {live:?}"
        );
    }
}

/// QA diagnostic at the C4 physical extent. Run explicitly with --ignored.
#[test]
#[ignore = "QA diagnostic: allocates 2560x1600 frames for three layers"]
fn native_gpu_output_display_ordinary_fixture_budget_trace_at_c4_extent() {
    for layers in [1, 3] {
        let (paths, live) = run_ordinary_fixture(layers, (2560, 1600), 24);
        println!("layers={layers} paths={paths:?} live={live:?}");
    }
}

/// Revokes every presentation-authority probe from call `from` onwards. Stop
/// is never requested, so every rejection here is a presentation revocation
/// (an accepted blackout, an ownership edit or a configuration change).
struct RevokeFrom {
    calls: std::cell::Cell<usize>,
    from: usize,
}

impl NativeResidentPresentationAuthority for RevokeFrom {
    fn revalidate(&self, _: NativeResidentPresentBoundary) -> NativeResidentPresentDecision {
        let call = self.calls.get() + 1;
        self.calls.set(call);
        if call >= self.from {
            NativeResidentPresentDecision::Revoked
        } else {
            NativeResidentPresentDecision::Permit
        }
    }
}

fn assert_healthy(session: &NativeResidentDisplaySession, context: &str) {
    assert!(
        session.terminal_fault.is_none(),
        "{context}: terminal fault"
    );
    assert_eq!(session.owner.state(), GpuOwnerState::Active, "{context}");
    assert!(
        !session.owner.has_unfinished_logical_frame(),
        "{context}: unfinished logical frame"
    );
    assert!(
        session.retirements.is_empty(),
        "{context}: retained retirement"
    );
    assert!(
        session.unsubmitted.is_empty(),
        "{context}: unsubmitted reservation"
    );
    assert!(
        session.recovered_history.is_none(),
        "{context}: recovered history"
    );
}

/// C4 blackout regression. A revocation observed at any preparation boundary
/// of an ordinary tick (before decode, during acquisition, after submission,
/// while polling completion) must not retire the shared owner: the tick is
/// typed Revoked without required retirement and the very next tick runs.
/// Hard blackout is the payload-less canonical content; restore resumes the
/// resident path on the same owner.
#[test]
fn native_gpu_output_display_revocation_at_every_boundary_keeps_owner_then_blackout_restores() {
    for layers in [1, 3] {
        let mut session = session_with_compositor(EXTENT);
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            Provider::default(),
        );
        let mut snapshot = scene(layers, EXTENT);
        let mut position = 0;
        let mut advance = |snapshot: &mut VideoSnapshot| {
            position += 16;
            for layer in &mut snapshot.layers {
                layer.state.position_ms = position;
            }
        };
        // Warm the history, then count the probes of one ordinary tick.
        for _ in 0..3 {
            advance(&mut snapshot);
            assert_eq!(
                tick(&mut session, &mut renderer, &snapshot, EXTENT),
                Path::Resident
            );
        }
        advance(&mut snapshot);
        let counting = RevokeFrom {
            calls: std::cell::Cell::new(0),
            from: usize::MAX,
        };
        let frame = prepare_with(&mut session, &mut renderer, &snapshot, EXTENT, &counting)
            .expect("counting tick permits every probe");
        assert_eq!(path_of(frame), Path::Resident);
        let probes = counting.calls.get();
        assert!(
            probes >= 3,
            "{layers}-layer tick probed authority {probes} times"
        );
        for from in 1..=probes {
            advance(&mut snapshot);
            let revoking = RevokeFrom {
                calls: std::cell::Cell::new(0),
                from,
            };
            let result = prepare_with(&mut session, &mut renderer, &snapshot, EXTENT, &revoking);
            assert!(
                matches!(
                    result,
                    Err(NativeResidentPrepareError::Revoked {
                        retirement_required: false
                    })
                ),
                "{layers} layers, revocation from probe {from}/{probes}: {:?}",
                result.as_ref().err()
            );
            assert_healthy(&session, &format!("{layers} layers, probe {from}"));
            advance(&mut snapshot);
            assert_eq!(
                tick(&mut session, &mut renderer, &snapshot, EXTENT),
                Path::Resident,
                "{layers} layers: the tick after revocation {from} must run"
            );
        }
        // Accepted hard blackout: payload-less content, no decode or pixels.
        let reads = renderer.frame_provider.reads;
        snapshot.blackout = true;
        advance(&mut snapshot);
        assert_eq!(
            tick(&mut session, &mut renderer, &snapshot, EXTENT),
            Path::HardBlackout
        );
        assert_eq!(
            renderer.frame_provider.reads, reads,
            "blackout decodes nothing"
        );
        assert_healthy(&session, "after blackout");
        snapshot.blackout = false;
        advance(&mut snapshot);
        assert_eq!(
            tick(&mut session, &mut renderer, &snapshot, EXTENT),
            Path::Resident,
            "{layers} layers: restore resumes the resident path"
        );
        assert_healthy(&session, "after restore");
        stop_and_reap(&mut session, &mut renderer);
    }
}
