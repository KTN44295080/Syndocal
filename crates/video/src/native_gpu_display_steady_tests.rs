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
        ring: Default::default(),
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
    InFlight,
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
        ResidentDisplayContent::InFlight => Path::InFlight,
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
        // ADR-01 section 7.2 snapshot ring: the first frame is synchronous
        // (presentable), the second tick fills the pipeline with zero presents,
        // and every later tick returns its resident predecessor.
        assert_eq!(paths[0], Path::Resident, "{layers}-layer first frame");
        assert_eq!(paths[1], Path::InFlight, "{layers}-layer pipeline fill");
        assert!(
            paths[2..].iter().all(|path| *path == Path::Resident),
            "{layers}-layer ordinary fixture left the resident path: {paths:?}"
        );
        // After the eight-entry history window fills, retained charges must be
        // a steady state rather than growing with the number of presents.
        // Between ticks one ring output is in flight; its transient ISF owners
        // are reaped whenever its transaction happens to complete, so a sample
        // may be lower but never higher than the steady in-flight envelope.
        let settled = &live[16..];
        let envelope = *settled.iter().max().unwrap();
        assert_eq!(
            live[16..24].iter().max(),
            live[24..].iter().max(),
            "{layers}-layer managed bytes grow: {live:?}"
        );
        assert!(
            settled.iter().filter(|bytes| **bytes == envelope).count() * 2 > settled.len(),
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
        // Warm the history (synchronous first frame, snapshot-ring fill, one
        // pipelined tick), then count the probes of one ordinary tick.
        for expected in [Path::Resident, Path::InFlight, Path::Resident] {
            advance(&mut snapshot);
            assert_eq!(
                tick(&mut session, &mut renderer, &snapshot, EXTENT),
                expected
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

/// Request sequence and PTS of a returned resident output.
fn resident_identity(frame: &NativeResidentDisplayFrame) -> Option<(u64, u64)> {
    match &frame.content {
        ResidentDisplayContent::Resident(ready) => {
            Some((ready.request.frame.request_sequence, ready.request.pts_ms))
        }
        _ => None,
    }
}

fn readback_bytes(session: &NativeResidentDisplaySession) -> u64 {
    let snapshot = session.owner.budget_snapshot();
    let entry = snapshot.for_kind(ChargeKind::SnapshotReadback);
    entry.active_bytes + entry.retired_bytes + entry.reserved_bytes
}

struct RingFixture {
    session: NativeResidentDisplaySession,
    renderer: VideoPreviewRenderer<Provider>,
    snapshot: VideoSnapshot,
    position: u64,
}

impl RingFixture {
    fn new(layers: u64, extent: (u32, u32)) -> Self {
        Self {
            session: session_with_compositor(extent),
            renderer: VideoPreviewRenderer::with_frame_provider(
                VideoRuntimeConfig::default(),
                Provider::default(),
            ),
            snapshot: scene(layers, extent),
            position: 0,
        }
    }

    fn prepare(&mut self, extent: (u32, u32)) -> NativeResidentDisplayFrame {
        self.position += 16;
        for layer in &mut self.snapshot.layers {
            layer.state.position_ms = self.position;
        }
        prepare_with(
            &mut self.session,
            &mut self.renderer,
            &self.snapshot,
            extent,
            &Permit,
        )
        .unwrap_or_else(|error| panic!("snapshot ring tick failed: {error}"))
    }

    /// Synchronous first frame, pipeline fill, one pipelined tick.
    fn warm(&mut self, extent: (u32, u32)) {
        for expected in [Path::Resident, Path::InFlight, Path::Resident] {
            let frame = self.prepare(extent);
            assert_eq!(path_of(frame), expected);
        }
        assert!(self.session.ring.owns_in_flight());
        assert!(self.session.owner.has_unfinished_logical_frame());
    }
}

/// Ordering proof: tick N returns output N-1 exactly once, in logical order,
/// and only after its snapshot was published as the CPU LastValid (the cache
/// holds that very request's pixels when the Fresh Ready is returned). The
/// ring charges exactly two snapshot readbacks, steady over every frame, and
/// Stop with a slot in flight reaps every owner.
#[test]
fn native_gpu_output_display_snapshot_ring_returns_each_output_once_after_its_last_valid() {
    for layers in [1, 3] {
        let mut ring = RingFixture::new(layers, EXTENT);
        let frame_bytes = u64::from(EXTENT.0) * u64::from(EXTENT.1) * 4;
        let first = ring.prepare(EXTENT);
        assert_eq!(
            resident_identity(&first),
            Some((1, 16)),
            "synchronous first frame"
        );
        drop(first);
        let fill = ring.prepare(EXTENT);
        assert_eq!(
            path_of(fill),
            Path::InFlight,
            "pipeline fill: zero presents"
        );
        for tick in 3..=20_u64 {
            let frame = ring.prepare(EXTENT);
            let (sequence, pts) = resident_identity(&frame)
                .unwrap_or_else(|| panic!("{layers} layers: tick {tick} returned no output"));
            assert_eq!(
                (sequence, pts),
                (tick - 1, (tick - 1) * 16),
                "{layers} layers: tick {tick} returns its predecessor"
            );
            assert_eq!(
                frame.evidence().freshness,
                crate::VideoOutputRenderFreshness::Fresh
            );
            let ResidentDisplayContent::Resident(ready) = &frame.content else {
                unreachable!()
            };
            let cached = ring
                .session
                .cache
                .last_valid(&ready.request.key)
                .expect("LastValid published before the Fresh Ready");
            assert_eq!((cached.pts_ms, cached.width), (pts, EXTENT.0));
            assert!(
                ring.session.ring.owns_in_flight(),
                "output {tick} is in flight"
            );
            assert!(ring.session.ring.free_readbacks() <= 1);
            assert_eq!(
                readback_bytes(&ring.session),
                2 * frame_bytes,
                "{layers} layers: the ring charges exactly two readbacks at tick {tick}"
            );
        }
        assert_eq!(ring.session.ring.discarded, 0);
        assert!(ring.session.ring.armed());
        stop_and_reap(&mut ring.session, &mut ring.renderer);
        assert!(!ring.session.ring.owns_in_flight());
    }
}

/// Hard blackout with output N in flight: N completes (history committed),
/// its snapshot is fenced and discarded with zero presents, the blackout is
/// the payload-less canonical content with no decode, and restore resumes
/// synchronously before the ring re-fills.
#[test]
fn native_gpu_output_display_snapshot_ring_blackout_with_a_slot_in_flight_discards_then_restores() {
    for layers in [1, 3] {
        let mut ring = RingFixture::new(layers, EXTENT);
        ring.warm(EXTENT);
        let reads = ring.renderer.frame_provider.reads;
        ring.snapshot.blackout = true;
        let frame = ring.prepare(EXTENT);
        assert_eq!(path_of(frame), Path::HardBlackout);
        assert_eq!(
            ring.renderer.frame_provider.reads, reads,
            "blackout decodes nothing"
        );
        assert_eq!(
            ring.session.ring.discarded, 1,
            "the in-flight output was discarded"
        );
        assert!(!ring.session.ring.owns_in_flight());
        assert!(!ring.session.ring.armed());
        assert_healthy(&ring.session, "blackout with a slot in flight");
        ring.snapshot.blackout = false;
        for expected in [Path::Resident, Path::InFlight, Path::Resident] {
            let frame = ring.prepare(EXTENT);
            assert_eq!(path_of(frame), expected, "{layers} layers restore");
        }
        stop_and_reap(&mut ring.session, &mut ring.renderer);
    }
}

/// Resize with output N in flight: N's old-geometry snapshot is discarded,
/// the new geometry starts synchronously, the old readbacks are dropped and
/// the ring reallocates exactly two readbacks at the new extent.
#[test]
fn native_gpu_output_display_snapshot_ring_resize_discards_old_geometry_and_reallocates() {
    let resized = (48, 30);
    let old_bytes = u64::from(EXTENT.0) * u64::from(EXTENT.1) * 4;
    let new_bytes = u64::from(resized.0) * u64::from(resized.1) * 4;
    let mut ring = RingFixture::new(2, EXTENT);
    ring.warm(EXTENT);
    assert_eq!(readback_bytes(&ring.session), 2 * old_bytes);
    let generation = ring.session.geometry_generation();
    ring.session.reserve_geometry(resized.0, resized.1).unwrap();
    ring.session.reserved_geometry = None;
    assert_eq!(ring.session.geometry_generation(), generation + 1);
    ring.snapshot = scene(2, resized);
    let mut path = path_of(ring.prepare(resized));
    assert_eq!(
        ring.session.ring.discarded, 1,
        "old-geometry output discarded"
    );
    // Old-extent canonical history entries keep the next ticks on CPU until
    // they leave the eight-entry window; the ring stays disarmed meanwhile.
    let mut ticks = 1;
    while path != Path::Resident {
        assert_eq!(
            path,
            Path::Cpu,
            "only canonical CPU output precedes resident"
        );
        assert!(!ring.session.ring.armed() && !ring.session.ring.owns_in_flight());
        path = path_of(ring.prepare(resized));
        ticks += 1;
        assert!(
            ticks <= 12,
            "resized output never returned to the resident path"
        );
    }
    // The first resident tick at the new geometry is synchronous.
    for expected in [Path::InFlight, Path::Resident, Path::Resident] {
        let frame = ring.prepare(resized);
        assert_eq!(path_of(frame), expected);
    }
    assert_eq!(
        readback_bytes(&ring.session),
        2 * new_bytes,
        "old readbacks reaped; two new-extent readbacks charged"
    );
    stop_and_reap(&mut ring.session, &mut ring.renderer);
}

/// Device loss with output N in flight fails closed: the next tick reports a
/// physical fault, N's complete owner moves into the session's retirement
/// (no ring slot keeps it), the generation is retained and nothing is reaped
/// or presented as a fallback.
#[test]
fn native_gpu_output_display_snapshot_ring_device_loss_with_a_slot_in_flight_fails_closed() {
    let mut ring = RingFixture::new(1, EXTENT);
    ring.warm(EXTENT);
    ring.session.owner.mark_device_lost_for_tests();
    ring.position += 16;
    for layer in &mut ring.snapshot.layers {
        layer.state.position_ms = ring.position;
    }
    let result = prepare_with(
        &mut ring.session,
        &mut ring.renderer,
        &ring.snapshot,
        EXTENT,
        &Permit,
    );
    assert!(
        matches!(result, Err(NativeResidentPrepareError::Fault(_))),
        "device loss is a fault, not a control interruption: {:?}",
        result.as_ref().err()
    );
    assert!(!ring.session.ring.owns_in_flight());
    assert_eq!(ring.session.retirements.len(), 1);
    assert!(ring.session.terminal_fault.is_some());
    assert_eq!(
        ring.session.owner.state(),
        GpuOwnerState::DeviceLostRetained
    );
    assert!(ring.session.owner.budget_snapshot().managed_bytes() > 0);
    ring.session
        .begin_stop_with_renderer(&mut ring.renderer)
        .unwrap();
    assert!(
        !matches!(ring.session.poll_stop(), Ok(true)),
        "a lost generation is retained, never reported Reaped"
    );
}

/// A caller that ends a tick without a session preparation (an app-level
/// revocation or settlement) abandons the in-flight output: the next tick
/// drains it with zero presents and renders its own output synchronously.
#[test]
fn native_gpu_output_display_snapshot_ring_abandoned_output_is_never_published() {
    let mut ring = RingFixture::new(1, EXTENT);
    ring.warm(EXTENT);
    assert!(ring.session.has_pipelined_output());
    ring.session.abandon_pipelined_output();
    let frame = ring.prepare(EXTENT);
    assert!(!frame.prepared_by_previous_tick());
    assert_eq!(
        resident_identity(&frame),
        Some((4, ring.position)),
        "the tick after an abandoned output presents its own frame"
    );
    drop(frame);
    assert_eq!(ring.session.ring.discarded, 1);
    assert!(!ring.session.has_pipelined_output());
    let fill = ring.prepare(EXTENT);
    assert_eq!(path_of(fill), Path::InFlight);
    let next = ring.prepare(EXTENT);
    assert!(next.prepared_by_previous_tick());
    // A held Ready keeps its premapping charge; Stop reaps after it drops.
    drop(next);
    stop_and_reap(&mut ring.session, &mut ring.renderer);
}
