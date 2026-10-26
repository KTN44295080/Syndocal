//! C0 conformance examples: fake owners exercise the shared signatures only.
//! These tests do not implement or certify GPU cleanup, history, or native output.
use super::*;
use protocol::{VideoOutputKind, VideoRenderInputId};
use std::cell::Cell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

fn context() -> GpuContextId {
    GpuContextId {
        owner_id: 7,
        device_generation: 3,
    }
}
fn frame_id() -> NativeFrameId {
    NativeFrameId {
        output_id: 11,
        project_epoch: 19,
        request_sequence: 23,
        geometry_generation: 5,
    }
}

#[test]
fn c1_native_contract_authoritative_history_scope_excludes_request_sequence() {
    let authoritative = frame_id();
    let scope = authoritative.history_scope();
    for request_sequence in [0, 1, authoritative.request_sequence, u64::MAX] {
        let later = NativeFrameId {
            request_sequence,
            ..authoritative
        };
        assert!(scope.matches(later));
        assert_eq!(later.history_scope(), scope);
        assert_eq!(scope.at_request(request_sequence), later);
    }
    // The scope is captured before inspecting incoming results; a foreign
    // result cannot replace the authoritative output/epoch/geometry identity.
    let changes: &[fn(&mut NativeFrameId)] = &[
        |value| value.output_id += 1,
        |value| value.project_epoch += 1,
        |value| value.geometry_generation += 1,
    ];
    for change in changes {
        let mut foreign = authoritative;
        change(&mut foreign);
        assert!(!scope.matches(foreign));
        assert_ne!(foreign.history_scope(), scope);
        assert_eq!(
            scope.at_request(authoritative.request_sequence),
            authoritative
        );
    }
    let later = scope.at_request(authoritative.request_sequence + 1);
    assert_ne!(
        later, authoritative,
        "a history scope is not a frame receipt"
    );
}

fn entry_id() -> HistoryEntryId {
    HistoryEntryId {
        input: VideoRenderInputKey {
            project_render_epoch: 19,
            input_id: VideoRenderInputId(31),
        },
        queue_epoch: 37,
        sequence: 41,
    }
}
fn clocks() -> FrozenChainClocks {
    FrozenChainClocks {
        frame_index: u32::MAX,
        time_seconds_bits: (-0.0_f32).to_bits(),
        time_delta_seconds_bits: 0x7fc0_0123,
        date_bits: [
            2026.0_f32.to_bits(),
            10.0_f32.to_bits(),
            26.0_f32.to_bits(),
            0x7fc0_0456,
        ],
    }
}
fn layer_identity() -> LayerResultIdentity {
    LayerResultIdentity {
        context: context(),
        frame: frame_id(),
        entry: entry_id(),
        layer_id: 2,
        source_generation: 43,
        content_sequence: 47,
        pts_ms: 33,
        duration_ms: 33,
        width: 2,
        height: 1,
        format: VideoPixelFormat::Rgba8,
        origin: LayerOrigin::Executed(clocks()),
    }
}
fn pixels(layer_id: u64) -> VideoFrame {
    VideoFrame {
        layer_id,
        width: 2,
        height: 1,
        pts_ms: 33,
        duration_ms: 33,
        format: VideoPixelFormat::Rgba8,
        data: vec![1, 2, 3, 255, 4, 5, 6, 255],
    }
}
fn materialization() -> MaterializationDescriptor {
    MaterializationDescriptor {
        layer: layer_identity(),
        allocation: GpuAllocationId {
            context: context(),
            sequence: 53,
        },
        completed_submission: GpuSubmissionId {
            context: context(),
            sequence: 59,
        },
    }
}
fn cache_descriptor() -> CacheCommitDescriptor {
    CacheCommitDescriptor {
        key: VideoOutputLastValidKey {
            project_render_epoch: 19,
            output_id: 11,
            width: 2,
            height: 1,
            composition_id: 61,
            kind: VideoOutputKind::Display,
            fullscreen: false,
            monitor_id: Some(1),
            endpoint_name: None,
        },
        snapshot: SnapshotIdentity {
            frame: frame_id(),
            allocation: GpuAllocationId {
                context: context(),
                sequence: 67,
            },
            completed_submission: GpuSubmissionId {
                context: context(),
                sequence: 71,
            },
            pts_ms: 33,
            duration_ms: 33,
            width: 2,
            height: 1,
            format: VideoPixelFormat::Rgba8,
        },
        fence: CachePublicationFence {
            request_sequence: 23,
            blackout_generation: 73,
            configuration_generation: 79,
        },
    }
}
fn fault(kind: NativeGpuFaultKind, phase: GpuPhase) -> NativeGpuFault {
    NativeGpuFault {
        context: context(),
        frame: Some(frame_id()),
        phase,
        kind,
        message: "opaque diagnostic: not a dispatch key".into(),
    }
}

// A deliberately non-Clone resource. The count observes actual Rust ownership.
#[derive(Debug)]
struct OwnedProbe(Rc<Cell<usize>>);
impl Drop for OwnedProbe {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}
fn probe() -> (OwnedProbe, Rc<Cell<usize>>) {
    let drops = Rc::new(Cell::new(0));
    (OwnedProbe(Rc::clone(&drops)), drops)
}

// A consumer-side fixture checks metadata before accepting an affine pair.
// It performs no GPU validation; for_test is the only proof producer here.
fn accept_exact(
    exact: ExactCpuLayer,
    expected: &MaterializationDescriptor,
) -> Result<VideoFrame, ExactCpuLayer> {
    let actual = exact.frame();
    let layer = &expected.layer;
    let matches = exact.proof().descriptor() == expected
        && actual.layer_id == layer.layer_id
        && actual.width == layer.width
        && actual.height == layer.height
        && actual.pts_ms == layer.pts_ms
        && actual.duration_ms == layer.duration_ms
        && actual.format == layer.format
        && actual.data.len() == (u64::from(layer.width) * u64::from(layer.height) * 4) as usize;
    if matches {
        Ok(exact.into_parts().0)
    } else {
        Err(exact)
    }
}

#[test]
fn native_contract_materialization_rejects_cross_bound_identity() {
    let expected = materialization();
    let changes: &[fn(&mut LayerResultIdentity)] = &[
        |v| v.context.owner_id += 1,
        |v| v.context.device_generation += 1,
        |v| v.frame.output_id += 1,
        |v| v.frame.project_epoch += 1,
        |v| v.frame.request_sequence += 1,
        |v| v.frame.geometry_generation += 1,
        |v| v.entry.input.project_render_epoch += 1,
        |v| v.entry.input.input_id.0 += 1,
        |v| v.entry.queue_epoch += 1,
        |v| v.entry.sequence += 1,
        |v| v.layer_id += 1,
        |v| v.source_generation += 1,
        |v| v.content_sequence += 1,
        |v| v.pts_ms += 1,
        |v| v.duration_ms += 1,
        |v| v.width += 1,
        |v| v.height += 1,
        |v| v.format = VideoPixelFormat::Bgra8,
        |v| v.origin = LayerOrigin::ImportedCpu,
    ];
    for change in changes {
        let mut wrong = expected;
        change(&mut wrong.layer);
        let frame = pixels(2);
        let pointer = frame.data.as_ptr();
        let exact = ExactCpuLayer::for_test(frame, MaterializationProof::for_test(wrong));
        let rejected = accept_exact(exact, &expected).expect_err("cross-bound proof must reject");
        assert_eq!(
            rejected.frame().data.as_ptr(),
            pointer,
            "rejection retains the original pair"
        );
        assert_eq!(rejected.proof().descriptor(), &wrong);
    }
    let allocation_changes: &[fn(&mut MaterializationDescriptor)] = &[
        |v| v.allocation.context.owner_id += 1,
        |v| v.allocation.context.device_generation += 1,
        |v| v.allocation.sequence += 1,
        |v| v.completed_submission.context.owner_id += 1,
        |v| v.completed_submission.context.device_generation += 1,
        |v| v.completed_submission.sequence += 1,
    ];
    for change in allocation_changes {
        let mut wrong = expected;
        change(&mut wrong);
        let exact = ExactCpuLayer::for_test(pixels(2), MaterializationProof::for_test(wrong));
        assert!(accept_exact(exact, &expected).is_err());
    }
    let frame = pixels(2);
    let pointer = frame.data.as_ptr();
    let exact = ExactCpuLayer::for_test(frame, MaterializationProof::for_test(expected));
    let accepted = accept_exact(exact, &expected).unwrap();
    assert_eq!(
        accepted.data.as_ptr(),
        pointer,
        "acceptance moves pixels, not a clone"
    );
}

#[test]
fn native_contract_cpu_materialization_rejects_mismatched_frame_metadata() {
    let changes: &[fn(&mut VideoFrame)] = &[
        |v| v.layer_id += 1,
        |v| v.width += 1,
        |v| v.height += 1,
        |v| v.pts_ms += 1,
        |v| v.duration_ms += 1,
        |v| v.format = VideoPixelFormat::Bgra8,
        |v| {
            v.data.pop();
        },
    ];
    for change in changes {
        let expected = materialization();
        let mut frame = pixels(2);
        change(&mut frame);
        let exact = ExactCpuLayer::for_test(frame, MaterializationProof::for_test(expected));
        assert!(accept_exact(exact, &expected).is_err());
    }
}

#[test]
fn native_contract_frozen_clocks_keep_bits_and_do_not_identify_history_by_pts() {
    let first = layer_identity();
    let mut repeated_pts = first;
    repeated_pts.frame.request_sequence += 1;
    repeated_pts.entry.sequence += 1;
    let mut next_clocks = clocks();
    next_clocks.frame_index = next_clocks.frame_index.wrapping_add(1);
    repeated_pts.origin = LayerOrigin::Executed(next_clocks);
    assert_eq!(repeated_pts.pts_ms, first.pts_ms);
    assert_ne!(
        repeated_pts.entry, first.entry,
        "same PTS is a new canonical result"
    );
    assert_ne!(repeated_pts.frame, first.frame);
    assert_eq!(next_clocks.frame_index, 0);
    assert_eq!(next_clocks.time_seconds_bits, 0x8000_0000);
    assert_eq!(next_clocks.time_delta_seconds_bits, 0x7fc0_0123);
    assert_eq!(next_clocks.date_bits, clocks().date_bits);
    let mut imported = first;
    imported.origin = LayerOrigin::ImportedCpu;
    assert!(matches!(imported.origin, LayerOrigin::ImportedCpu));
    assert_ne!(
        imported, first,
        "CPU import carries no invented executed clock"
    );
}

struct MockPending {
    resource: Option<OwnedProbe>,
    terminal: bool,
}
struct MockCompletion {
    script: VecDeque<CompletionProgress<(), NativeGpuFault>>,
    state: GpuOwnerState,
    retained: HashMap<RetirementId, OwnedProbe>,
}
impl MockCompletion {
    fn new(script: Vec<CompletionProgress<(), NativeGpuFault>>) -> Self {
        Self {
            script: script.into(),
            state: GpuOwnerState::Active,
            retained: HashMap::new(),
        }
    }
    fn reap(&mut self, id: RetirementId) {
        assert!(self.retained.remove(&id).is_some());
        self.state = GpuOwnerState::Reaped;
    }
}
impl GpuCompletionPort for MockCompletion {
    type Pending = MockPending;
    type Completed = OwnedProbe;
    type Retiring = RetirementId;
    fn poll_completion(
        &mut self,
        pending: &mut MockPending,
        cancel: &dyn VideoRenderCancellation,
    ) -> CompletionProgress<OwnedProbe, NativeGpuFault> {
        assert!(!pending.terminal, "terminal poll cannot be repeated");
        let step = if cancel.is_cancelled() {
            CompletionProgress::Cancelled
        } else {
            self.script
                .pop_front()
                .expect("bounded scripted completion")
        };
        match step {
            CompletionProgress::Pending => CompletionProgress::Pending,
            CompletionProgress::Cancelled => {
                pending.terminal = true;
                self.state = GpuOwnerState::Retiring;
                CompletionProgress::Cancelled
            }
            CompletionProgress::Complete(Ok(())) => {
                pending.terminal = true;
                CompletionProgress::Complete(Ok(pending.resource.take().unwrap()))
            }
            CompletionProgress::Complete(Err(error)) => {
                pending.terminal = true;
                self.state = match error.kind {
                    NativeGpuFaultKind::DeviceLost => GpuOwnerState::DeviceLostRetained,
                    NativeGpuFaultKind::ScopeDrain => GpuOwnerState::PoisonedRetained,
                    _ => GpuOwnerState::Retiring,
                };
                CompletionProgress::Complete(Err(error))
            }
        }
    }
    fn cancel_and_retire(&mut self, mut pending: MockPending) -> RetirementId {
        let id = RetirementId {
            context: context(),
            sequence: 83,
        };
        assert!(self
            .retained
            .insert(id, pending.resource.take().unwrap())
            .is_none());
        if self.state == GpuOwnerState::Active {
            self.state = GpuOwnerState::Retiring;
        }
        id
    }
}

#[test]
fn native_contract_cancel_keeps_the_owner_until_explicit_retirement() {
    let (resource, drops) = probe();
    let mut pending = MockPending {
        resource: Some(resource),
        terminal: false,
    };
    let mut port = MockCompletion::new(vec![CompletionProgress::Pending]);
    assert!(matches!(
        port.poll_completion(&mut pending, &|| false),
        CompletionProgress::Pending
    ));
    assert_eq!(port.state, GpuOwnerState::Active);
    assert!(matches!(
        port.poll_completion(&mut pending, &|| true),
        CompletionProgress::Cancelled
    ));
    assert_eq!(drops.get(), 0);
    let id = port.cancel_and_retire(pending);
    let failure = MaterializationFailure::Cancelled {
        retirement: Some(id),
    };
    assert!(
        matches!(&failure, MaterializationFailure::Cancelled { retirement: Some(actual) } if *actual == id)
    );
    drop(failure);
    assert_eq!(drops.get(), 0, "a retirement ID is not the strong owner");
    assert!(!port.retained.contains_key(&RetirementId {
        context: GpuContextId {
            device_generation: 4,
            ..context()
        },
        ..id
    }));
    port.reap(id);
    assert_eq!(port.state, GpuOwnerState::Reaped);
    assert_eq!(drops.get(), 1);
}

#[test]
fn native_contract_completion_moves_a_nonclone_payload_once() {
    let (resource, drops) = probe();
    let mut pending = MockPending {
        resource: Some(resource),
        terminal: false,
    };
    let mut port = MockCompletion::new(vec![CompletionProgress::Complete(Ok(()))]);
    let CompletionProgress::Complete(Ok(completed)) = port.poll_completion(&mut pending, &|| false)
    else {
        panic!("expected completion");
    };
    assert!(pending.resource.is_none());
    drop(pending);
    assert_eq!(drops.get(), 0);
    drop(completed);
    assert_eq!(drops.get(), 1);
}

#[test]
fn native_contract_faults_preserve_typed_phase_and_retained_owner() {
    let kinds = [
        NativeGpuFaultKind::DeviceLost,
        NativeGpuFaultKind::OutOfMemory,
        NativeGpuFaultKind::Validation,
        NativeGpuFaultKind::Submission,
        NativeGpuFaultKind::Map,
        NativeGpuFaultKind::SnapshotCopy,
        NativeGpuFaultKind::ScopeDrain,
        NativeGpuFaultKind::HistoricalMaterialization { entry: entry_id() },
        NativeGpuFaultKind::IdentityMismatch,
        NativeGpuFaultKind::StateMismatch,
        NativeGpuFaultKind::SequenceExhausted,
    ];
    let phases = [
        GpuPhase::Initialize,
        GpuPhase::IsfBatch,
        GpuPhase::HistoricalReplay,
        GpuPhase::CompositionSnapshot,
        GpuPhase::MappingPresent,
        GpuPhase::Retirement,
    ];
    for (i, kind) in kinds.into_iter().enumerate() {
        let phase = phases[i % phases.len()];
        let (resource, drops) = probe();
        let mut pending = MockPending {
            resource: Some(resource),
            terminal: false,
        };
        let mut port =
            MockCompletion::new(vec![CompletionProgress::Complete(Err(fault(kind, phase)))]);
        let CompletionProgress::Complete(Err(error)) =
            port.poll_completion(&mut pending, &|| false)
        else {
            panic!("fault became success");
        };
        assert_eq!(
            error,
            fault(kind, phase),
            "typed metadata, not message dispatch"
        );
        let expected_state = match kind {
            NativeGpuFaultKind::DeviceLost => GpuOwnerState::DeviceLostRetained,
            NativeGpuFaultKind::ScopeDrain => GpuOwnerState::PoisonedRetained,
            _ => GpuOwnerState::Retiring,
        };
        assert_eq!(port.state, expected_state);
        let retirement = port.cancel_and_retire(pending);
        let failure = MaterializationFailure::Fault {
            fault: error,
            retirement: Some(retirement),
        };
        let MaterializationFailure::Fault {
            fault: returned,
            retirement: returned_id,
        } = failure
        else {
            panic!("fault was reclassified as cancellation");
        };
        assert_eq!(returned, fault(kind, phase));
        assert_eq!(returned_id, Some(retirement));
        assert_eq!(drops.get(), 0);
        port.reap(retirement);
        assert_eq!(drops.get(), 1);
    }
}

#[test]
fn native_contract_declines_return_owned_input_without_completion_evidence() {
    let reasons = [
        UnsupportedNativeFx::EmptyChain,
        UnsupportedNativeFx::UnsupportedJobCount,
        UnsupportedNativeFx::NonLayerScope,
        UnsupportedNativeFx::ClipOrTransitionExecution,
        UnsupportedNativeFx::InterLayerDependency,
        UnsupportedNativeFx::Callback,
        UnsupportedNativeFx::PersistentTarget,
        UnsupportedNativeFx::AuxiliaryResource,
        UnsupportedNativeFx::Multipass,
        UnsupportedNativeFx::CustomTarget,
    ];
    let mut declines: Vec<_> = reasons
        .into_iter()
        .map(|reason| NativeGpuDecline::UnsupportedChain { reason })
        .collect();
    declines.extend([
        NativeGpuDecline::UnsupportedComposition,
        NativeGpuDecline::UnsupportedFormat,
        NativeGpuDecline::UnsupportedExtent,
        NativeGpuDecline::UnknownUploadPitch,
        NativeGpuDecline::BudgetExceeded {
            live: 500,
            requested: 20,
            limit: 512,
        },
        NativeGpuDecline::QuoteExpired,
    ]);
    // A preflight adapter returns the same owned input with a typed decline.
    fn preflight(
        input: OwnedProbe,
        reason: NativeGpuDecline,
    ) -> Result<OwnedProbe, (NativeGpuDecline, OwnedProbe)> {
        Err((reason, input))
    }
    for reason in declines {
        let (input, drops) = probe();
        let (returned_reason, returned_input) = preflight(input, reason).unwrap_err();
        assert_eq!(returned_reason, reason);
        assert_eq!(drops.get(), 0, "CPU fallback still owns its original input");
        drop(returned_input);
        assert_eq!(drops.get(), 1);
    }
}

struct MockReady {
    resource: OwnedProbe,
    retry_spent: bool,
}
enum MockPermit {
    Allowed,
    Rejected(PresentRejection),
    Fault,
}
struct MockPresentReceipt {
    frame: NativeFrameId,
    submission: GpuSubmissionId,
}
#[derive(Default)]
struct MockPresentSink {
    retained: Option<OwnedProbe>,
    deliveries: usize,
}
impl NativePresentSink for MockPresentSink {
    type Ready = MockReady;
    type Permit = MockPermit;
    type Receipt = MockPresentReceipt;
    fn present_once(
        &mut self,
        mut ready: MockReady,
        permit: MockPermit,
    ) -> Result<PresentOutcome<MockReady, MockPresentReceipt>, NativeGpuFault> {
        match permit {
            MockPermit::Rejected(reason) => {
                let retry =
                    if reason == PresentRejection::SurfaceLostOrOutdated && !ready.retry_spent {
                        ready.retry_spent = true;
                        Some(ready)
                    } else {
                        None
                    };
                Ok(PresentOutcome::NotPresented { reason, retry })
            }
            MockPermit::Fault => {
                self.retained = Some(ready.resource);
                Err(fault(
                    NativeGpuFaultKind::Validation,
                    GpuPhase::MappingPresent,
                ))
            }
            MockPermit::Allowed => {
                self.retained = Some(ready.resource);
                self.deliveries += 1;
                Ok(PresentOutcome::Presented(MockPresentReceipt {
                    frame: frame_id(),
                    submission: GpuSubmissionId {
                        context: context(),
                        sequence: 89,
                    },
                }))
            }
        }
    }
}

#[test]
fn native_contract_present_rejections_never_count_as_deliveries_or_reset_retry() {
    for reason in [
        PresentRejection::Cancelled,
        PresentRejection::Revoked,
        PresentRejection::StaleGeometry,
        PresentRejection::StaleMapping,
        PresentRejection::SurfaceTimeout,
        PresentRejection::SurfaceLostOrOutdated,
        PresentRejection::Suspended,
    ] {
        let (resource, drops) = probe();
        let mut sink = MockPresentSink::default();
        let outcome = sink
            .present_once(
                MockReady {
                    resource,
                    retry_spent: false,
                },
                MockPermit::Rejected(reason),
            )
            .unwrap();
        let PresentOutcome::NotPresented {
            reason: actual,
            retry,
        } = outcome
        else {
            panic!("rejection became a delivery");
        };
        assert_eq!(actual, reason);
        if let Some(ready) = retry {
            assert_eq!(reason, PresentRejection::SurfaceLostOrOutdated);
            assert_eq!(drops.get(), 0);
            let second = sink
                .present_once(ready, MockPermit::Rejected(reason))
                .unwrap();
            assert!(matches!(
                second,
                PresentOutcome::NotPresented { retry: None, .. }
            ));
        }
        assert_eq!(sink.deliveries, 0);
        assert_eq!(drops.get(), 1);
    }
}

#[test]
fn native_contract_present_receipt_does_not_release_submission_resources() {
    let (resource, drops) = probe();
    let mut sink = MockPresentSink::default();
    let outcome = sink
        .present_once(
            MockReady {
                resource,
                retry_spent: false,
            },
            MockPermit::Allowed,
        )
        .unwrap();
    let PresentOutcome::Presented(receipt) = outcome else {
        panic!("expected delivery");
    };
    assert_eq!(receipt.frame, frame_id());
    assert_eq!(
        receipt.submission,
        GpuSubmissionId {
            context: context(),
            sequence: 89
        }
    );
    assert_eq!(sink.deliveries, 1);
    assert_eq!(drops.get(), 0);
    drop(sink.retained.take());
    assert_eq!(drops.get(), 1);
    let (resource, drops) = probe();
    let error = match sink.present_once(
        MockReady {
            resource,
            retry_spent: false,
        },
        MockPermit::Fault,
    ) {
        Err(error) => error,
        Ok(_) => panic!("physical fault became success"),
    };
    assert_eq!(
        error,
        fault(NativeGpuFaultKind::Validation, GpuPhase::MappingPresent)
    );
    assert_eq!(sink.deliveries, 1);
    assert_eq!(drops.get(), 0);
    drop(sink.retained.take());
    assert_eq!(drops.get(), 1);
}

// Bound to simulated validated snapshot evidence before the trait call.
// This is a conformance example, not the production output/cache state machine.
struct MockCache {
    expected: CacheCommitDescriptor,
    current: Option<(VideoOutputLastValidKey, VideoFrame)>,
    cancelled: bool,
}
impl MockCache {
    fn new() -> Self {
        let expected = cache_descriptor();
        let mut previous = pixels(0);
        previous.data.fill(9);
        Self {
            current: Some((expected.key.clone(), previous)),
            expected,
            cancelled: false,
        }
    }
}
impl CpuSnapshotCommit for MockCache {
    fn commit(
        &mut self,
        key: &VideoOutputLastValidKey,
        frame: NativeFrameId,
        snapshot: VideoFrame,
        fence: CachePublicationFence,
    ) -> Result<CacheCommitReceipt, SnapshotCommitRejection> {
        let expected = &self.expected;
        if self.cancelled {
            return Err(SnapshotCommitRejection::Cancelled);
        }
        if frame.project_epoch != expected.snapshot.frame.project_epoch {
            return Err(SnapshotCommitRejection::StaleEpoch);
        }
        if frame.request_sequence != expected.snapshot.frame.request_sequence
            || fence.request_sequence != expected.fence.request_sequence
        {
            return Err(SnapshotCommitRejection::StaleRequest);
        }
        if fence.blackout_generation != expected.fence.blackout_generation {
            return Err(SnapshotCommitRejection::BlackoutFence);
        }
        if fence.configuration_generation != expected.fence.configuration_generation {
            return Err(SnapshotCommitRejection::ConfigurationChanged);
        }
        if key != &expected.key {
            return Err(SnapshotCommitRejection::KeyMismatch);
        }
        let image = expected.snapshot;
        if frame != image.frame
            || snapshot.layer_id != 0
            || snapshot.width != image.width
            || snapshot.height != image.height
            || snapshot.pts_ms != image.pts_ms
            || snapshot.duration_ms != image.duration_ms
            || snapshot.format != image.format
            || snapshot.data.len()
                != (u64::from(image.width) * u64::from(image.height) * 4) as usize
        {
            return Err(SnapshotCommitRejection::SnapshotIdentityMismatch);
        }
        self.current = Some((key.clone(), snapshot));
        Ok(CacheCommitReceipt::for_test(expected.clone()))
    }
    fn invalidate_output(&mut self, output: VideoOutputId) {
        if self
            .current
            .as_ref()
            .is_some_and(|(key, _)| key.output_id == output)
        {
            self.current = None;
        }
        if self.expected.key.output_id == output {
            self.expected.fence.blackout_generation += 1;
        }
    }
}

#[test]
fn native_contract_cache_full_key_rejections_do_not_mutate_previous_pixels() {
    let changes: &[fn(&mut VideoOutputLastValidKey)] = &[
        |v| v.project_render_epoch += 1,
        |v| v.output_id += 1,
        |v| v.width += 1,
        |v| v.height += 1,
        |v| v.composition_id += 1,
        |v| v.kind = VideoOutputKind::NdiSender,
        |v| v.fullscreen = !v.fullscreen,
        |v| v.monitor_id = None,
        |v| v.endpoint_name = Some("different endpoint".into()),
    ];
    for change in changes {
        let mut cache = MockCache::new();
        let expected = cache.expected.clone();
        let old_pointer = cache.current.as_ref().unwrap().1.data.as_ptr();
        let mut wrong = expected.key.clone();
        change(&mut wrong);
        assert_eq!(
            cache
                .commit(&wrong, expected.snapshot.frame, pixels(0), expected.fence)
                .unwrap_err(),
            SnapshotCommitRejection::KeyMismatch
        );
        let (key, frame) = cache.current.as_ref().unwrap();
        assert_eq!(key, &expected.key);
        assert_eq!(frame.data.as_ptr(), old_pointer);
        assert_eq!(frame.data, vec![9; 8]);
    }
}

#[test]
fn native_contract_cache_fences_and_snapshot_identity_reject_without_mutation() {
    type Mutation =
        fn(&mut MockCache, &mut NativeFrameId, &mut VideoFrame, &mut CachePublicationFence);
    let cases: &[(SnapshotCommitRejection, Mutation)] = &[
        (SnapshotCommitRejection::Cancelled, |c, _, _, _| {
            c.cancelled = true
        }),
        (SnapshotCommitRejection::StaleEpoch, |_, f, _, _| {
            f.project_epoch += 1
        }),
        (SnapshotCommitRejection::StaleRequest, |_, f, _, _| {
            f.request_sequence += 1
        }),
        (SnapshotCommitRejection::StaleRequest, |_, _, _, p| {
            p.request_sequence += 1
        }),
        (SnapshotCommitRejection::BlackoutFence, |_, _, _, p| {
            p.blackout_generation += 1
        }),
        (
            SnapshotCommitRejection::ConfigurationChanged,
            |_, _, _, p| p.configuration_generation += 1,
        ),
        (
            SnapshotCommitRejection::SnapshotIdentityMismatch,
            |_, f, _, _| f.output_id += 1,
        ),
        (
            SnapshotCommitRejection::SnapshotIdentityMismatch,
            |_, f, _, _| f.geometry_generation += 1,
        ),
        (
            SnapshotCommitRejection::SnapshotIdentityMismatch,
            |_, _, p, _| p.width += 1,
        ),
        (
            SnapshotCommitRejection::SnapshotIdentityMismatch,
            |_, _, p, _| p.height += 1,
        ),
        (
            SnapshotCommitRejection::SnapshotIdentityMismatch,
            |_, _, p, _| p.pts_ms += 1,
        ),
        (
            SnapshotCommitRejection::SnapshotIdentityMismatch,
            |_, _, p, _| p.duration_ms += 1,
        ),
        (
            SnapshotCommitRejection::SnapshotIdentityMismatch,
            |_, _, p, _| p.format = VideoPixelFormat::Bgra8,
        ),
        (
            SnapshotCommitRejection::SnapshotIdentityMismatch,
            |_, _, p, _| {
                p.data.pop();
            },
        ),
    ];
    for (reason, change) in cases {
        let mut cache = MockCache::new();
        let expected = cache.expected.clone();
        let old_pointer = cache.current.as_ref().unwrap().1.data.as_ptr();
        let (mut frame, mut pixels, mut fence) =
            (expected.snapshot.frame, pixels(0), expected.fence);
        change(&mut cache, &mut frame, &mut pixels, &mut fence);
        assert_eq!(
            cache
                .commit(&expected.key, frame, pixels, fence)
                .unwrap_err(),
            *reason
        );
        assert_eq!(cache.current.as_ref().unwrap().1.data.as_ptr(), old_pointer);
        assert_eq!(cache.current.as_ref().unwrap().1.data, vec![9; 8]);
    }
}

#[test]
fn native_contract_cache_commit_moves_snapshot_and_invalidation_fences_late_completion() {
    let mut cache = MockCache::new();
    let expected = cache.expected.clone();
    let frame = pixels(0);
    let pointer = frame.data.as_ptr();
    let receipt = cache
        .commit(
            &expected.key,
            expected.snapshot.frame,
            frame,
            expected.fence,
        )
        .unwrap();
    assert_eq!(cache.current.as_ref().unwrap().1.data.as_ptr(), pointer);
    assert_eq!(receipt.descriptor(), &expected);
    // The bound allocation/submission are retained exactly; CPU pixels do not mint them.
    assert_eq!(
        receipt.descriptor().snapshot.allocation,
        expected.snapshot.allocation
    );
    assert_eq!(
        receipt.descriptor().snapshot.completed_submission,
        expected.snapshot.completed_submission
    );
    let mut different = expected.clone();
    different.snapshot.allocation.context.device_generation += 1;
    assert_ne!(receipt.descriptor(), &different);
    different = expected.clone();
    different.snapshot.completed_submission.sequence += 1;
    assert_ne!(receipt.descriptor(), &different);
    cache.invalidate_output(expected.key.output_id + 1);
    assert_eq!(cache.current.as_ref().unwrap().1.data.as_ptr(), pointer);
    cache.invalidate_output(expected.key.output_id);
    assert!(cache.current.is_none());
    assert_eq!(
        cache
            .commit(
                &expected.key,
                expected.snapshot.frame,
                pixels(0),
                expected.fence
            )
            .unwrap_err(),
        SnapshotCommitRejection::BlackoutFence
    );
    assert!(
        cache.current.is_none(),
        "late completion cannot resurrect invalidated cache"
    );
}
