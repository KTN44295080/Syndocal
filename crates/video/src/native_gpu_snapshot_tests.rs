//! Scripted policy tests only: no GPU completion or native-window evidence.
use super::*;
use crate::native_gpu_contract::{
    CachePublicationFence, GpuAllocationId, GpuContextId, GpuSubmissionId, NativeFrameId,
};
use protocol::VideoOutputKind;
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

type Events = Rc<RefCell<Vec<&'static str>>>;

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
