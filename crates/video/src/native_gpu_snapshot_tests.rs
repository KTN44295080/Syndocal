//! Scripted policy tests only: no GPU completion or native-window evidence.
use super::*;
use crate::native_gpu_contract::{
    CachePublicationFence, GpuAllocationId, GpuContextId, GpuSubmissionId, NativeFrameId,
};
use protocol::VideoOutputKind;
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

type Events = Rc<RefCell<Vec<&'static str>>>;

thread_local! {
    /// Snapshot copies whose destination was a recycled LastValid buffer.
    pub(crate) static RECYCLED_COPIES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

fn descriptor() -> CacheCommitDescriptor {
    let context = GpuContextId {
        owner_id: 1,
        device_generation: 2,
    };
    CacheCommitDescriptor {
        key: crate::VideoOutputLastValidKey {
            project_render_epoch: 3,
            output_id: 4,
            width: 2,
            height: 2,
            composition_id: 5,
            kind: VideoOutputKind::Display,
            fullscreen: false,
            monitor_id: Some(6),
            endpoint_name: None,
        },
        snapshot: SnapshotIdentity {
            frame: NativeFrameId {
                output_id: 4,
                project_epoch: 3,
                request_sequence: 7,
                geometry_generation: 8,
            },
            allocation: GpuAllocationId {
                context,
                sequence: 9,
            },
            completed_submission: GpuSubmissionId {
                context,
                sequence: 10,
            },
            pts_ms: 33,
            duration_ms: 34,
            width: 2,
            height: 2,
            format: VideoPixelFormat::Rgba8,
        },
        fence: CachePublicationFence {
            request_sequence: 7,
            blackout_generation: 11,
            configuration_generation: 12,
        },
    }
}

struct FakeLease(Events);
impl Drop for FakeLease {
    fn drop(&mut self) {
        self.0.borrow_mut().push("drop-gpu");
    }
}

struct FakeView<'a> {
    bytes: &'a [u8],
    events: Events,
}
impl AsRef<[u8]> for FakeView<'_> {
    fn as_ref(&self) -> &[u8] {
        self.bytes
    }
}
impl Drop for FakeView<'_> {
    fn drop(&mut self) {
        self.events.borrow_mut().push("drop-view");
    }
}

struct ScriptedMapper {
    script: VecDeque<CompletionProgress<SnapshotPhaseReceipt, NativeGpuFault>>,
    bytes: Vec<u8>,
    events: Events,
    view_error: bool,
    unmap_error: bool,
    identity: SnapshotIdentity,
}
impl SnapshotMapPort for ScriptedMapper {
    type View<'a> = FakeView<'a>;

    fn poll_phase_and_map(&mut self) -> CompletionProgress<SnapshotPhaseReceipt, NativeGpuFault> {
        self.events.borrow_mut().push("poll");
        self.script
            .pop_front()
            .expect("unexpected extra mapper poll")
    }
    fn mapped_view(&mut self) -> Result<Self::View<'_>, NativeGpuFault> {
        self.events.borrow_mut().push("view");
        if self.view_error {
            return Err(fault(
                &self.identity,
                NativeGpuFaultKind::Map,
                "scripted view error",
            ));
        }
        Ok(FakeView {
            bytes: &self.bytes,
            events: self.events.clone(),
        })
    }
    fn unmap(&mut self) -> Result<(), NativeGpuFault> {
        self.events.borrow_mut().push("unmap");
        if self.unmap_error {
            return Err(fault(
                &self.identity,
                NativeGpuFaultKind::ScopeDrain,
                "scripted unmap failure",
            ));
        }
        Ok(())
    }
}
impl Drop for ScriptedMapper {
    fn drop(&mut self) {
        self.events.borrow_mut().push("drop-mapper");
    }
}

fn pending(
    descriptor: &CacheCommitDescriptor,
    script: Vec<CompletionProgress<SnapshotPhaseReceipt, NativeGpuFault>>,
) -> (PendingPremappingSnapshot<FakeLease, ScriptedMapper>, Events) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mapper = ScriptedMapper {
        script: script.into(),
        // Two packed rows with four padding bytes between them.
        bytes: vec![
            1, 2, 3, 4, 5, 6, 7, 8, 99, 99, 99, 99, 9, 10, 11, 12, 13, 14, 15, 16, 99, 99, 99, 99,
        ],
        events: events.clone(),
        view_error: false,
        unmap_error: false,
        identity: descriptor.snapshot,
    };
    (
        NativeGpuSnapshotter::begin(SnapshotEncoding {
            descriptor: descriptor.clone(),
            gpu: FakeLease(events.clone()),
            mapper,
            readback_row_pitch: 12,
        }),
        events,
    )
}

fn success(identity: SnapshotIdentity) -> CompletionProgress<SnapshotPhaseReceipt, NativeGpuFault> {
    CompletionProgress::Complete(Ok(SnapshotPhaseReceipt::for_test(identity)))
}
fn completed(
    pending: &mut PendingPremappingSnapshot<FakeLease, ScriptedMapper>,
    descriptor: &CacheCommitDescriptor,
) -> CompletedPremappingSnapshot<FakeLease> {
    match NativeGpuSnapshotter::poll(pending, descriptor, &|| false) {
        SnapshotProgress::Complete(completed) => completed,
        _ => panic!("snapshot did not complete"),
    }
}

struct FakeCache {
    expected: CacheCommitDescriptor,
    image: Option<VideoFrame>,
    commits: usize,
    rejection: Option<SnapshotCommitRejection>,
    foreign_receipt: bool,
}
impl FakeCache {
    fn new(descriptor: &CacheCommitDescriptor) -> Self {
        Self {
            expected: descriptor.clone(),
            image: None,
            commits: 0,
            rejection: None,
            foreign_receipt: false,
        }
    }
}
impl CpuSnapshotCommit for FakeCache {
    fn commit(
        &mut self,
        key: &crate::VideoOutputLastValidKey,
        frame: NativeFrameId,
        snapshot: VideoFrame,
        fence: CachePublicationFence,
    ) -> Result<CacheCommitReceipt, SnapshotCommitRejection> {
        if let Some(reason) = self.rejection {
            return Err(reason);
        }
        assert_eq!(key, &self.expected.key);
        assert_eq!(frame, self.expected.snapshot.frame);
        assert_eq!(fence, self.expected.fence);
        if self.foreign_receipt {
            let mut wrong = self.expected.clone();
            wrong.snapshot.allocation.sequence += 1;
            return Ok(CacheCommitReceipt::for_test(wrong));
        }
        self.commits += 1;
        self.image = Some(snapshot);
        Ok(CacheCommitReceipt::for_test(self.expected.clone()))
    }
    fn invalidate_output(&mut self, output: u64) {
        if output == self.expected.key.output_id {
            self.expected.fence.blackout_generation += 1;
            self.image = None;
        }
    }
}

#[test]
fn snapshot_policy_pending_copy_unmap_cache_move_then_ready_input() {
    let expected = descriptor();
    let (mut pending, events) = pending(
        &expected,
        vec![CompletionProgress::Pending, success(expected.snapshot)],
    );
    let mut surface_acquires = 0;
    assert!(matches!(
        NativeGpuSnapshotter::poll(&mut pending, &expected, &|| false),
        SnapshotProgress::Pending
    ));
    assert_eq!(surface_acquires, 0);
    let output = completed(&mut pending, &expected);
    assert_eq!(
        &*events.borrow(),
        &["poll", "poll", "view", "drop-view", "unmap"]
    );
    assert_eq!(surface_acquires, 0);
    let pointer = output.pixels.data.as_ptr();
    let mut cache = FakeCache::new(&expected);
    let bound = output
        .commit(&mut cache, &expected, &|| false)
        .unwrap_or_else(|_| panic!("commit failed"));
    assert_eq!(cache.commits, 1);
    let pixels = cache.image.as_ref().unwrap();
    assert_eq!(
        pixels.data.as_ptr(),
        pointer,
        "cache commit moved the same Vec"
    );
    assert_eq!(pixels.data, (1u8..=16).collect::<Vec<_>>());
    assert_eq!(pixels.layer_id, 0);
    assert_eq!((pixels.pts_ms, pixels.duration_ms), (33, 34));
    // Only this cache-bound type may cross the fake I Ready boundary.
    let (gpu, receipt) = bound.into_parts();
    assert_eq!(receipt.descriptor(), &expected);
    surface_acquires += 1;
    assert_eq!(surface_acquires, 1);
    let (no_gpu, mapper) = pending.into_retirement().into_parts();
    assert!(no_gpu.is_none());
    assert!(!events.borrow().contains(&"drop-gpu"));
    drop(mapper);
    assert_eq!(
        cache.image.as_ref().unwrap().data,
        (1u8..=16).collect::<Vec<_>>()
    );
    drop(gpu);
    assert!(events.borrow().contains(&"drop-gpu"));
}

/// C4R12 stage B: a recycled packed buffer of exactly the packed size is
/// the copy destination (no new allocation), fully overwritten, and gives the
/// same pixels as a fresh allocation; any other capacity is not used.
#[test]
fn snapshot_policy_copies_into_a_recycled_buffer_of_exact_packed_capacity() {
    let expected = descriptor();
    let (mut fresh, _) = pending(&expected, vec![success(expected.snapshot)]);
    let fresh = completed(&mut fresh, &expected).pixels;
    assert_eq!(fresh.data, (1u8..=16).collect::<Vec<_>>());

    let mut recycled = Vec::with_capacity(16);
    recycled.extend_from_slice(&[0xEE; 16]);
    let recycled_pointer = recycled.as_ptr();
    let (mut reused, _) = pending(&expected, vec![success(expected.snapshot)]);
    reused.packed = Some(recycled);
    let reused = completed(&mut reused, &expected).pixels;
    assert_eq!(reused, fresh, "recycled destination changes no pixel");
    assert_eq!(reused.data.as_ptr(), recycled_pointer, "no new allocation");
    assert_eq!(reused.data.capacity(), 16);

    for wrong in [Vec::with_capacity(15), Vec::with_capacity(64)] {
        let wrong_pointer = wrong.as_ptr();
        let (mut other, _) = pending(&expected, vec![success(expected.snapshot)]);
        other.packed = Some(wrong);
        let other = completed(&mut other, &expected).pixels;
        assert_eq!(other, fresh);
        assert_ne!(other.data.as_ptr(), wrong_pointer);
        assert_eq!(other.data.capacity(), 16);
    }
}

#[test]
fn snapshot_policy_faults_and_cancel_keep_pending_owners_until_retirement() {
    let expected = descriptor();
    for kind in [
        NativeGpuFaultKind::Map,
        NativeGpuFaultKind::Validation,
        NativeGpuFaultKind::OutOfMemory,
        NativeGpuFaultKind::ScopeDrain,
    ] {
        let failure = fault(&expected.snapshot, kind, "scripted completion failure");
        let (mut pending, events) =
            pending(&expected, vec![CompletionProgress::Complete(Err(failure))]);
        match NativeGpuSnapshotter::poll(&mut pending, &expected, &|| false) {
            SnapshotProgress::Failed(error) => assert_eq!(error.kind, kind),
            _ => panic!("failure became completion"),
        }
        assert_eq!(&*events.borrow(), &["poll"]);
        let retirement = pending.into_retirement();
        assert!(!events.borrow().contains(&"drop-gpu"));
        drop(retirement);
        assert!(events.borrow().contains(&"drop-gpu"));
        assert!(events.borrow().contains(&"drop-mapper"));
    }
    for owner_cancel in [true, false] {
        let (mut pending, events) = pending(&expected, vec![CompletionProgress::Cancelled]);
        assert!(matches!(
            NativeGpuSnapshotter::poll(&mut pending, &expected, &|| owner_cancel),
            SnapshotProgress::Cancelled
        ));
        assert!(!events.borrow().contains(&"drop-gpu"));
        let before = events.borrow().len();
        assert!(matches!(
            NativeGpuSnapshotter::poll(&mut pending, &expected, &|| false),
            SnapshotProgress::Failed(NativeGpuFault {
                kind: NativeGpuFaultKind::StateMismatch,
                ..
            })
        ));
        assert_eq!(events.borrow().len(), before);
        drop(pending.into_retirement());
    }
}

#[test]
fn snapshot_policy_late_foreign_receipts_never_copy_or_publish() {
    let expected = descriptor();
    let mutations: &[fn(&mut SnapshotIdentity)] = &[
        |v| v.frame.request_sequence += 1,
        |v| v.frame.geometry_generation += 1,
        |v| v.frame.project_epoch += 1,
        |v| v.allocation.sequence += 1,
        |v| v.allocation.context.device_generation += 1,
        |v| v.completed_submission.sequence += 1,
        |v| v.pts_ms += 1,
        |v| v.duration_ms += 1,
        |v| v.width += 1,
        |v| v.format = VideoPixelFormat::Bgra8,
    ];
    for mutate in mutations {
        let mut foreign = expected.snapshot;
        mutate(&mut foreign);
        let (mut pending, events) = pending(
            &expected,
            vec![success(foreign), success(expected.snapshot)],
        );
        assert!(matches!(
            NativeGpuSnapshotter::poll(&mut pending, &expected, &|| false),
            SnapshotProgress::Failed(NativeGpuFault {
                kind: NativeGpuFaultKind::IdentityMismatch,
                ..
            })
        ));
        assert_eq!(&*events.borrow(), &["poll"]);
        assert!(matches!(
            NativeGpuSnapshotter::poll(&mut pending, &expected, &|| false),
            SnapshotProgress::Failed(NativeGpuFault {
                kind: NativeGpuFaultKind::StateMismatch,
                ..
            })
        ));
        assert_eq!(
            pending.mapper.script.len(),
            1,
            "late callback was not consumed"
        );
        drop(pending.into_retirement());
    }
}

#[test]
fn snapshot_policy_key_fences_reject_before_mapper_and_before_cache() {
    let expected = descriptor();
    let cases: &[(SnapshotCommitRejection, fn(&mut CacheCommitDescriptor))] = &[
        (SnapshotCommitRejection::StaleEpoch, |v| {
            v.snapshot.frame.project_epoch += 1
        }),
        (SnapshotCommitRejection::StaleRequest, |v| {
            v.snapshot.frame.request_sequence += 1
        }),
        (SnapshotCommitRejection::StaleRequest, |v| {
            v.fence.request_sequence += 1
        }),
        (SnapshotCommitRejection::BlackoutFence, |v| {
            v.fence.blackout_generation += 1
        }),
        (SnapshotCommitRejection::ConfigurationChanged, |v| {
            v.fence.configuration_generation += 1
        }),
        (SnapshotCommitRejection::KeyMismatch, |v| {
            v.key.composition_id += 1
        }),
        (SnapshotCommitRejection::KeyMismatch, |v| {
            v.key.output_id += 1
        }),
        (SnapshotCommitRejection::KeyMismatch, |v| {
            v.key.endpoint_name = Some("other".into())
        }),
        (SnapshotCommitRejection::KeyMismatch, |v| {
            v.key.monitor_id = None
        }),
        (SnapshotCommitRejection::KeyMismatch, |v| {
            v.key.fullscreen = true
        }),
        (SnapshotCommitRejection::KeyMismatch, |v| {
            v.key.kind = VideoOutputKind::NdiSender
        }),
        (SnapshotCommitRejection::SnapshotIdentityMismatch, |v| {
            v.snapshot.allocation.sequence += 1
        }),
    ];
    for (reason, mutate) in cases {
        let mut current = expected.clone();
        mutate(&mut current);
        let (mut first, events) = pending(&expected, vec![success(expected.snapshot)]);
        assert!(
            matches!(NativeGpuSnapshotter::poll(&mut first, &current, &|| false), SnapshotProgress::Rejected(actual) if actual == *reason)
        );
        assert!(events.borrow().is_empty());
        drop(first.into_retirement());
        let (mut second, _) = pending(&expected, vec![success(expected.snapshot)]);
        let output = completed(&mut second, &expected);
        let mut cache = FakeCache::new(&expected);
        let rejected = match output.commit(&mut cache, &current, &|| false) {
            Err(error) => error,
            Ok(_) => panic!("stale completed snapshot committed"),
        };
        assert_eq!(rejected.reason, *reason);
        assert_eq!(cache.commits, 0);
        drop(rejected.gpu);
        drop(second.into_retirement());
    }
}

#[test]
fn snapshot_policy_copy_error_drops_view_then_unmaps_without_cache() {
    let expected = descriptor();
    let (mut pending, events) = pending(&expected, vec![success(expected.snapshot)]);
    pending.mapper.bytes.truncate(15);
    assert!(matches!(
        NativeGpuSnapshotter::poll(&mut pending, &expected, &|| false),
        SnapshotProgress::Failed(NativeGpuFault {
            kind: NativeGpuFaultKind::SnapshotCopy,
            ..
        })
    ));
    assert_eq!(&*events.borrow(), &["poll", "view", "drop-view", "unmap"]);
    assert!(!events.borrow().contains(&"drop-gpu"));
    drop(pending.into_retirement());
}

#[test]
fn snapshot_policy_view_or_unmap_failure_has_no_completed_pixels() {
    let expected = descriptor();
    for view_error in [true, false] {
        let (mut pending, events) = pending(&expected, vec![success(expected.snapshot)]);
        pending.mapper.view_error = view_error;
        pending.mapper.unmap_error = !view_error;
        let expected_kind = if view_error {
            NativeGpuFaultKind::Map
        } else {
            NativeGpuFaultKind::ScopeDrain
        };
        assert!(
            matches!(NativeGpuSnapshotter::poll(&mut pending, &expected, &|| false), SnapshotProgress::Failed(NativeGpuFault { kind, .. }) if kind == expected_kind)
        );
        if view_error {
            assert_eq!(&*events.borrow(), &["poll", "view", "unmap"]);
        } else {
            assert_eq!(&*events.borrow(), &["poll", "view", "drop-view", "unmap"]);
        }
        assert!(!events.borrow().contains(&"drop-gpu"));
        drop(pending.into_retirement());
    }
}

#[test]
fn snapshot_policy_invalid_encoding_never_calls_mapper() {
    let expected = descriptor();
    let mutations: &[fn(&mut PendingPremappingSnapshot<FakeLease, ScriptedMapper>)] = &[
        |v| v.readback_row_pitch = 7,
        |v| v.readback_row_pitch = u64::MAX,
        |v| v.descriptor.snapshot.width = 0,
        |v| v.descriptor.snapshot.format = VideoPixelFormat::Bgra8,
        |v| v.descriptor.snapshot.completed_submission.context.owner_id += 1,
        |v| v.descriptor.key.project_render_epoch += 1,
        |v| v.descriptor.key.output_id += 1,
        |v| v.descriptor.fence.request_sequence += 1,
    ];
    for mutate in mutations {
        let (mut pending, events) = pending(&expected, vec![success(expected.snapshot)]);
        mutate(&mut pending);
        let current = pending.descriptor.clone();
        assert!(matches!(
            NativeGpuSnapshotter::poll(&mut pending, &current, &|| false),
            SnapshotProgress::Failed(_)
        ));
        assert!(events.borrow().is_empty());
        drop(pending.into_retirement());
    }
}

#[test]
fn snapshot_policy_cache_rejection_or_foreign_receipt_cannot_create_ready_input() {
    let expected = descriptor();
    for reason in [
        Some(SnapshotCommitRejection::Cancelled),
        Some(SnapshotCommitRejection::ConfigurationChanged),
        None,
    ] {
        let (mut pending, events) = pending(&expected, vec![success(expected.snapshot)]);
        let output = completed(&mut pending, &expected);
        let mut cache = FakeCache::new(&expected);
        cache.rejection = reason;
        cache.foreign_receipt = reason.is_none();
        let rejected = match output.commit(&mut cache, &expected, &|| false) {
            Err(error) => error,
            Ok(_) => panic!("foreign or rejected cache receipt became Ready input"),
        };
        assert_eq!(
            rejected.reason,
            reason.unwrap_or(SnapshotCommitRejection::SnapshotIdentityMismatch)
        );
        assert_eq!(cache.commits, 0);
        assert!(!events.borrow().contains(&"drop-gpu"));
        drop(rejected.gpu);
        drop(pending.into_retirement());
    }
}

#[test]
fn snapshot_policy_cancel_at_cache_boundary_and_blackout_prevent_publication() {
    let expected = descriptor();
    for cancelled in [true, false] {
        let (mut pending, _) = pending(&expected, vec![success(expected.snapshot)]);
        let output = completed(&mut pending, &expected);
        let mut cache = FakeCache::new(&expected);
        if !cancelled {
            cache.invalidate_output(expected.key.output_id);
        }
        let current = cache.expected.clone();
        let rejected = match output.commit(&mut cache, &current, &|| cancelled) {
            Err(error) => error,
            Ok(_) => panic!("revoked snapshot committed"),
        };
        assert_eq!(
            rejected.reason,
            if cancelled {
                SnapshotCommitRejection::Cancelled
            } else {
                SnapshotCommitRejection::BlackoutFence
            }
        );
        assert_eq!(cache.commits, 0);
        assert!(cache.image.is_none());
        drop(rejected.gpu);
        drop(pending.into_retirement());
    }
}

#[test]
fn snapshot_policy_cancel_during_copy_unmap_keeps_source_for_retirement() {
    let expected = descriptor();
    let (mut pending, events) = pending(&expected, vec![success(expected.snapshot)]);
    let checks = std::cell::Cell::new(0);
    let cancel = || {
        checks.set(checks.get() + 1);
        checks.get() > 1
    };
    assert!(matches!(
        NativeGpuSnapshotter::poll(&mut pending, &expected, &cancel),
        SnapshotProgress::Cancelled
    ));
    assert_eq!(checks.get(), 2);
    assert_eq!(&*events.borrow(), &["poll", "view", "drop-view", "unmap"]);
    assert!(!events.borrow().contains(&"drop-gpu"));
    drop(pending.into_retirement());
    assert!(events.borrow().contains(&"drop-gpu"));
}

// Real shared-context adapter tests: G's headless owner, G's real ISF
// candidates/leases, P's composition phase and the real readback map. These
// are headless correctness/ownership evidence, not native-window or timing.
use crate::isf_native_gpu::await_transaction;
use crate::isf_native_gpu::tests::{gpu, Fixture};
use crate::native_gpu_composition::tests::{
    headless, identity, identity_batch, layer, plan, render, submit_shared_frame, validate_isf,
    SharedFrame,
};
use crate::native_gpu_composition::{NativeGpuCompositor, SelectedTextureKind};
use crate::native_gpu_context::RetirementProgress;
use crate::CompositionPlan;
use protocol::{Transform2D, VideoBlendMode};
use std::time::{Duration, Instant};

const REAL_PTS_MS: u64 = 1_000;
const REAL_DURATION_MS: u64 = 33;

fn start_snapshot(
    encoded: crate::native_gpu_composition::EncodedPremappingSnapshot,
    frame: NativeFrameId,
    extent: (u32, u32),
) -> SharedPremappingSnapshot {
    let key = crate::VideoOutputLastValidKey {
        project_render_epoch: frame.project_epoch,
        output_id: frame.output_id,
        width: extent.0,
        height: extent.1,
        composition_id: 1,
        kind: VideoOutputKind::Display,
        fullscreen: true,
        monitor_id: Some(1),
        endpoint_name: None,
    };
    let fence = CachePublicationFence {
        request_sequence: frame.request_sequence,
        blackout_generation: 4,
        configuration_generation: 5,
    };
    match NativeGpuSnapshotter::begin_shared(encoded, key, fence, REAL_PTS_MS, REAL_DURATION_MS) {
        Ok(snapshot) => snapshot,
        Err(failure) => panic!("snapshot begin rejected: {:?}", failure.fault),
    }
}

fn finish_snapshot(
    fixture: &mut Fixture,
    pending: &mut SharedPremappingSnapshot,
) -> CompletedPremappingSnapshot<NativeSnapshotGpu> {
    let current = pending.descriptor().clone();
    let started = Instant::now();
    loop {
        match NativeGpuSnapshotter::poll_shared(pending, &mut fixture.owner, &current, &|| false) {
            SnapshotProgress::Pending => {
                assert!(
                    started.elapsed() < Duration::from_secs(5),
                    "real snapshot map did not complete"
                );
                std::thread::yield_now();
            }
            SnapshotProgress::Complete(completed) => return completed,
            SnapshotProgress::Failed(error) => panic!("real snapshot failed: {error:?}"),
            SnapshotProgress::Rejected(reason) => panic!("real snapshot rejected: {reason:?}"),
            SnapshotProgress::Cancelled => panic!("real snapshot unexpectedly cancelled"),
        }
    }
}

fn opaque_plans() -> [crate::CompositionLayerPlan; 3] {
    [
        layer(1, VideoBlendMode::Normal, 1.0, Transform2D::default()),
        layer(
            2,
            VideoBlendMode::Add,
            1.0,
            Transform2D {
                x: 0.07,
                y: -0.035,
                scale_x: 0.83,
                scale_y: 0.6,
                crop_left: 0.13,
                crop_bottom: 0.25,
                ..Default::default()
            },
        ),
        layer(
            3,
            VideoBlendMode::Difference,
            1.0,
            Transform2D {
                crop_left: 0.2,
                crop_right: 0.1,
                ..Default::default()
            },
        ),
    ]
}

fn opaque_pixel(layer: u64, index: usize) -> [u8; 4] {
    let value = (index as u8)
        .wrapping_mul(29)
        .wrapping_add(layer as u8 * 71);
    [value, value.wrapping_mul(3), 255 - value, 255]
}

fn wait_for_allocations(fixture: &mut Fixture, expected: usize) {
    // These assertions isolate outstanding snapshot/lease owners. Idle pool
    // storage now deliberately survives a tick, so release that bank first.
    fixture.owner.release_resource_pools();
    let started = Instant::now();
    loop {
        match fixture.owner.poll_retirement().unwrap() {
            RetirementProgress::Pending {
                transactions: 0,
                allocations,
            } if allocations == expected => return,
            progress => assert!(
                started.elapsed() < Duration::from_secs(5),
                "allocations were not reaped to {expected}: {progress:?}"
            ),
        }
        std::thread::yield_now();
    }
}

fn reap_after_stop(fixture: &mut Fixture) {
    let started = Instant::now();
    loop {
        match fixture.owner.poll_retirement().unwrap() {
            RetirementProgress::Reaped => break,
            progress => assert!(
                started.elapsed() < Duration::from_secs(5),
                "owner was not reaped: {progress:?}"
            ),
        }
        std::thread::yield_now();
    }
    assert_eq!(fixture.owner.budget_snapshot().managed_bytes(), 0);
}

#[test]
fn real_shared_isf_then_composition_snapshot_bytes_equal_cpu_and_gpu_oracles() {
    let mut fixture = gpu();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    eprintln!(
        "REAL_ADAPTER shared composition/snapshot: {:?}",
        fixture.owner.capabilities().adapter_info
    );
    let size = (41, 29);
    let pixels = u64::from(size.0 * size.1 * 4);
    let plans = opaque_plans();
    let batch = identity_batch(size, 3, opaque_pixel);
    let SharedFrame {
        frame,
        isf,
        pending,
        encoded,
        candidate_inputs,
    } = submit_shared_frame(&mut fixture, &compositor, 1, batch, &[], &plans, false);
    let encoded = encoded.unwrap();
    let submission = encoded.submission();
    // A key/fence for another request is rejected before any map starts and
    // returns every encoded owner intact.
    let stale_key = crate::VideoOutputLastValidKey {
        project_render_epoch: frame.project_epoch,
        output_id: frame.output_id,
        width: size.0,
        height: size.1,
        composition_id: 1,
        kind: VideoOutputKind::Display,
        fullscreen: true,
        monitor_id: Some(1),
        endpoint_name: None,
    };
    let stale_fence = CachePublicationFence {
        request_sequence: frame.request_sequence + 1,
        blackout_generation: 4,
        configuration_generation: 5,
    };
    let encoded = match NativeGpuSnapshotter::begin_shared(
        encoded,
        stale_key,
        stale_fence,
        REAL_PTS_MS,
        REAL_DURATION_MS,
    ) {
        Ok(_) => panic!("a stale fence must not start the snapshot map"),
        Err(failure) => {
            assert_eq!(failure.fault.kind, NativeGpuFaultKind::IdentityMismatch);
            failure.encoded
        }
    };
    assert_eq!(encoded.submission(), submission);
    let mut snapshot = start_snapshot(encoded, frame, size);
    let mut resolved = await_transaction(&mut fixture.owner, pending, &|| false).unwrap();
    let layers = validate_isf(&mut fixture, isf, &mut resolved);
    assert_eq!(layers.len(), 3);
    snapshot.resolve_shared_phase(&mut resolved).unwrap();
    let completed = finish_snapshot(&mut fixture, &mut snapshot);
    let image = completed.pixels();
    assert_eq!(
        (
            image.width,
            image.height,
            image.format,
            image.pts_ms,
            image.duration_ms
        ),
        (
            size.0,
            size.1,
            VideoPixelFormat::Rgba8,
            REAL_PTS_MS,
            REAL_DURATION_MS
        )
    );
    assert_eq!(completed.phase.identity.completed_submission, submission);
    assert_eq!(completed.phase.identity, completed.descriptor.snapshot);
    // Opaque inputs, opacity 1 and integer canonical blends: exact CPU oracle.
    let cpu = crate::composite_frames_rgba8(
        &CompositionPlan {
            composition_id: 1,
            label: "P50 CPU oracle".into(),
            output_ids: vec![],
            master_opacity: 1.0,
            blackout: false,
            layers: plans.to_vec(),
        },
        &candidate_inputs,
        size.0,
        size.1,
    )
    .unwrap();
    assert_eq!(
        image.data, cpu.data,
        "snapshot bytes must equal the CPU oracle"
    );
    let gpu_oracle = headless().expect("the shared test already proved an adapter exists");
    let ids = [identity(1, size), identity(2, size), identity(3, size)];
    let oracle_plan = plan(
        &plans,
        &ids,
        &[SelectedTextureKind::Validated; 3],
        size,
        [0; 4],
    )
    .unwrap();
    assert_eq!(
        image.data,
        render(&gpu_oracle, &oracle_plan, &candidate_inputs, false),
        "snapshot bytes must equal the existing buffer-source GPU oracle"
    );
    assert!(image.data.len() as u64 <= completed.gpu.cpu_capacity());

    // The CPU copy moves into the (simulated) LastValid cache without a clone.
    let expected = snapshot.descriptor().clone();
    let pointer = image.data.as_ptr();
    let mut cache = FakeCache::new(&expected);
    let bound = completed
        .commit(&mut cache, &expected, &|| false)
        .unwrap_or_else(|_| panic!("matching snapshot commit failed"));
    assert_eq!(cache.image.as_ref().unwrap().data.as_ptr(), pointer);
    let (gpu_owner, receipt) = bound.into_parts();
    assert_eq!(receipt.descriptor(), &expected);
    assert_eq!(
        gpu_owner.premapping().allocation_id(),
        expected.snapshot.allocation
    );
    assert_eq!(gpu_owner.cpu_capacity(), pixels);

    // Only the premapping target and the CPU snapshot charge outlive the frame.
    drop(snapshot);
    drop(layers);
    drop(resolved);
    wait_for_allocations(&mut fixture, 2);
    assert_eq!(fixture.owner.budget_snapshot().allocated_bytes, 2 * pixels);
    drop(gpu_owner);
    wait_for_allocations(&mut fixture, 0);
    fixture.owner.begin_stop();
    reap_after_stop(&mut fixture);
}

#[test]
fn real_shared_history_lease_and_candidates_match_gpu_oracle_and_cpu_tolerance() {
    let mut fixture = gpu();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let size = (37, 23);
    // Frame 1 produces validated history; only layer 1's lease is retained.
    let first = submit_shared_frame(
        &mut fixture,
        &compositor,
        1,
        identity_batch(size, 3, opaque_pixel),
        &[],
        &opaque_plans(),
        false,
    );
    let mut first_snapshot = start_snapshot(first.encoded.unwrap(), first.frame, size);
    let mut resolved = await_transaction(&mut fixture.owner, first.pending, &|| false).unwrap();
    let mut layers = validate_isf(&mut fixture, first.isf, &mut resolved).into_iter();
    first_snapshot.resolve_shared_phase(&mut resolved).unwrap();
    drop(finish_snapshot(&mut fixture, &mut first_snapshot));
    drop(first_snapshot);
    let history = layers.next().unwrap();
    drop(layers);
    let history_input = Arc::clone(&first.candidate_inputs[0]);

    // Frame 2 composes that lease under two semi-transparent new candidates.
    let plans = [
        layer(1, VideoBlendMode::Normal, 1.0, Transform2D::default()),
        layer(
            2,
            VideoBlendMode::Multiply,
            0.37,
            Transform2D {
                x: 1.2,
                scale_y: 0.6,
                ..Default::default()
            },
        ),
        layer(
            3,
            VideoBlendMode::SoftLight,
            0.7,
            Transform2D {
                crop_left: 0.3,
                scale_x: 0.8,
                ..Default::default()
            },
        ),
    ];
    let second = submit_shared_frame(
        &mut fixture,
        &compositor,
        2,
        identity_batch(size, 2, |layer, index| {
            let value = (index as u8)
                .wrapping_mul(13)
                .wrapping_add(layer as u8 * 17);
            [value, 255 - value, value / 2, value.wrapping_mul(7)]
        }),
        &[&history.lease],
        &plans,
        false,
    );
    let mut snapshot = start_snapshot(second.encoded.unwrap(), second.frame, size);
    let mut resolved = await_transaction(&mut fixture.owner, second.pending, &|| false).unwrap();
    let layers = validate_isf(&mut fixture, second.isf, &mut resolved);
    snapshot.resolve_shared_phase(&mut resolved).unwrap();
    let completed = finish_snapshot(&mut fixture, &mut snapshot);
    let mut inputs = vec![history_input];
    inputs.extend(second.candidate_inputs.iter().cloned());
    assert!(inputs[2].data.chunks_exact(4).any(|pixel| pixel[3] < 255));
    let gpu_oracle = headless().expect("the shared test already proved an adapter exists");
    let ids = [identity(1, size), identity(2, size), identity(3, size)];
    let oracle_plan = plan(
        &plans,
        &ids,
        &[SelectedTextureKind::Validated; 3],
        size,
        [0; 4],
    )
    .unwrap();
    assert_eq!(
        completed.pixels().data,
        render(&gpu_oracle, &oracle_plan, &inputs, false),
        "validated history + candidates must equal the existing GPU oracle"
    );
    let cpu = crate::composite_frames_rgba8(
        &CompositionPlan {
            composition_id: 1,
            label: "P50 CPU oracle".into(),
            output_ids: vec![],
            master_opacity: 1.0,
            blackout: false,
            layers: plans.to_vec(),
        },
        &inputs,
        size.0,
        size.1,
    )
    .unwrap();
    // Existing CPU contract: one rounding level per fractional blend.
    for (index, (actual, expected)) in completed.pixels().data.iter().zip(&cpu.data).enumerate() {
        assert!(
            actual.abs_diff(*expected) <= 1,
            "CPU oracle channel {index}: GPU={actual} CPU={expected}"
        );
    }
    drop(completed);
    drop(snapshot);
    drop(layers);
    drop(resolved);
    drop(history);
    wait_for_allocations(&mut fixture, 0);
    fixture.owner.begin_stop();
    reap_after_stop(&mut fixture);
}

#[test]
fn real_shared_snapshot_map_error_keeps_owners_and_never_mints_a_receipt() {
    let mut fixture = gpu();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let size = (23, 17);
    let pixels = u64::from(size.0 * size.1 * 4);
    let shared = submit_shared_frame(
        &mut fixture,
        &compositor,
        1,
        identity_batch(size, 3, opaque_pixel),
        &[],
        &opaque_plans(),
        false,
    );
    let mut snapshot = start_snapshot(shared.encoded.unwrap(), shared.frame, size);
    // Abort the real pending map before any device poll; wgpu delivers a real
    // MapAborted callback error to G's map guard.
    snapshot.mapper.readback.buffer().unmap();
    let mut resolved = await_transaction(&mut fixture.owner, shared.pending, &|| false).unwrap();
    let layers = validate_isf(&mut fixture, shared.isf, &mut resolved);
    assert_eq!(layers.len(), 3, "ISF result is independent of the map");
    snapshot.resolve_shared_phase(&mut resolved).unwrap();
    let current = snapshot.descriptor().clone();
    match NativeGpuSnapshotter::poll_shared(&mut snapshot, &mut fixture.owner, &current, &|| false)
    {
        SnapshotProgress::Failed(error) => assert_eq!(error.kind, NativeGpuFaultKind::Map),
        _ => panic!("aborted map must fail the snapshot"),
    }
    assert!(!snapshot.mapper.mapped, "no receipt and no owned mapping");
    match NativeGpuSnapshotter::poll_shared(&mut snapshot, &mut fixture.owner, &current, &|| false)
    {
        SnapshotProgress::Failed(error) => {
            assert_eq!(error.kind, NativeGpuFaultKind::StateMismatch)
        }
        _ => panic!("terminal snapshot must not be repolled into success"),
    }
    drop(layers);
    drop(resolved);
    // Premapping target, readback and CPU charge stay with the failed owner.
    wait_for_allocations(&mut fixture, 3);
    assert_eq!(fixture.owner.budget_snapshot().allocated_bytes, 3 * pixels);
    let (gpu_owner, mapper) = snapshot.into_retirement().into_parts();
    assert!(gpu_owner.is_some());
    drop(mapper);
    drop(gpu_owner);
    wait_for_allocations(&mut fixture, 0);
    assert_eq!(
        fixture.owner.state(),
        crate::native_gpu_contract::GpuOwnerState::Active,
        "a failed snapshot map does not poison the shared owner"
    );
    fixture.owner.begin_stop();
    reap_after_stop(&mut fixture);
}

#[test]
fn real_shared_snapshot_cancel_then_late_callback_and_foreign_phase_never_publish() {
    let mut fixture = gpu();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let size = (23, 17);
    // Frame 1 is cancelled before its map completes.
    let shared = submit_shared_frame(
        &mut fixture,
        &compositor,
        1,
        identity_batch(size, 3, opaque_pixel),
        &[],
        &opaque_plans(),
        false,
    );
    let mut cancelled = start_snapshot(shared.encoded.unwrap(), shared.frame, size);
    let current = cancelled.descriptor().clone();
    assert!(matches!(
        NativeGpuSnapshotter::poll_shared(&mut cancelled, &mut fixture.owner, &current, &|| true),
        SnapshotProgress::Cancelled
    ));
    let mut resolved = await_transaction(&mut fixture.owner, shared.pending, &|| false).unwrap();
    drop(validate_isf(&mut fixture, shared.isf, &mut resolved));
    assert_eq!(
        cancelled
            .resolve_shared_phase(&mut resolved)
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::StateMismatch
    );
    // The real callback arrives late; it cannot publish through the cancelled owner.
    let started = Instant::now();
    while cancelled.mapper.map.try_complete().unwrap().is_none() {
        assert!(started.elapsed() < Duration::from_secs(5));
        fixture.owner.poll_retirement().unwrap();
        std::thread::yield_now();
    }
    assert!(matches!(
        NativeGpuSnapshotter::poll_shared(&mut cancelled, &mut fixture.owner, &current, &|| false),
        SnapshotProgress::Failed(NativeGpuFault {
            kind: NativeGpuFaultKind::StateMismatch,
            ..
        })
    ));
    assert!(!cancelled.mapper.mapped);
    drop(resolved);

    // A snapshot can only adopt its own transaction's P phase: frame 2's
    // receipt presented to frame 3's resolved transaction is rejected by G.
    let stale_frame = submit_shared_frame(
        &mut fixture,
        &compositor,
        2,
        identity_batch(size, 3, opaque_pixel),
        &[],
        &opaque_plans(),
        false,
    );
    let mut stale = start_snapshot(stale_frame.encoded.unwrap(), stale_frame.frame, size);
    let mut stale_resolved =
        await_transaction(&mut fixture.owner, stale_frame.pending, &|| false).unwrap();
    drop(validate_isf(
        &mut fixture,
        stale_frame.isf,
        &mut stale_resolved,
    ));
    let next = submit_shared_frame(
        &mut fixture,
        &compositor,
        3,
        identity_batch(size, 3, opaque_pixel),
        &[],
        &opaque_plans(),
        false,
    );
    let mut fresh = start_snapshot(next.encoded.unwrap(), next.frame, size);
    let mut resolved = await_transaction(&mut fixture.owner, next.pending, &|| false).unwrap();
    drop(validate_isf(&mut fixture, next.isf, &mut resolved));
    assert_eq!(
        stale.resolve_shared_phase(&mut resolved).unwrap_err().kind,
        NativeGpuFaultKind::IdentityMismatch
    );
    let stale_current = stale.descriptor().clone();
    assert!(matches!(
        NativeGpuSnapshotter::poll_shared(&mut stale, &mut fixture.owner, &stale_current, &|| {
            false
        }),
        SnapshotProgress::Failed(NativeGpuFault {
            kind: NativeGpuFaultKind::StateMismatch,
            ..
        })
    ));
    assert!(!stale.mapper.mapped);
    fresh.resolve_shared_phase(&mut resolved).unwrap();
    let completed = finish_snapshot(&mut fixture, &mut fresh);
    assert_eq!(completed.descriptor.snapshot.frame, next.frame);
    drop(completed);
    drop(resolved);
    drop(stale_resolved);
    drop(fresh);
    drop(stale.into_retirement());
    drop(cancelled.into_retirement());
    wait_for_allocations(&mut fixture, 0);
    assert_eq!(
        fixture.owner.state(),
        crate::native_gpu_contract::GpuOwnerState::Active
    );
    fixture.owner.begin_stop();
    reap_after_stop(&mut fixture);
}

#[test]
fn real_shared_stop_before_completion_retains_owners_until_retired() {
    let mut fixture = gpu();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let size = (23, 17);
    let pixels = u64::from(size.0 * size.1 * 4);
    let shared = submit_shared_frame(
        &mut fixture,
        &compositor,
        1,
        identity_batch(size, 3, opaque_pixel),
        &[],
        &opaque_plans(),
        false,
    );
    let mut snapshot = start_snapshot(shared.encoded.unwrap(), shared.frame, size);
    let mut pending = shared.pending;
    let token = fixture.owner.begin_stop();
    assert_eq!(token.context(), fixture.owner.context().id());
    assert!(matches!(
        fixture.owner.poll_transaction(&mut pending, &|| false),
        CompletionProgress::Cancelled
    ));
    let retiring_transaction = fixture.owner.cancel_and_retire(pending);
    let current = snapshot.descriptor().clone();
    assert!(matches!(
        NativeGpuSnapshotter::poll_shared(&mut snapshot, &mut fixture.owner, &current, &|| false),
        SnapshotProgress::Cancelled
    ));
    assert!(!snapshot.mapper.mapped);
    let retiring = snapshot.into_retirement();
    drop(shared.isf);
    // The broker drains the cancelled frame; P's retiring owner still holds
    // the premapping target, the readback and the CPU charge.
    wait_for_allocations(&mut fixture, 3);
    assert_eq!(fixture.owner.budget_snapshot().allocated_bytes, 3 * pixels);
    assert_eq!(retiring_transaction.id().context, token.context());
    drop(retiring);
    reap_after_stop(&mut fixture);
}
