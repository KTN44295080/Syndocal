//! I2 output tests: real headless G receipts, P snapshot policy, and live cache fences.
//! The presenter stays scripted; no window, surface, or native output is created.
use super::*;
use crate::isf_native_gpu::await_transaction;
use crate::native_gpu_budget::NativeAllocationPlan;
use crate::native_gpu_context::{
    NativeGpuFactory, NativeGpuOwner, NativeGpuRequirements, ResolvedGpuPhase, RetirementProgress,
};
use crate::native_gpu_result::IsfValidatedReceipt;
use crate::native_gpu_snapshot::{
    NativeGpuSnapshotter, RetiringPremappingSnapshot, SnapshotEncoding, SnapshotMapPort,
    SnapshotPhaseReceipt, SnapshotProgress,
};
use protocol::VideoOutputKind;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Clone, Default)]
struct Stop(Rc<Cell<bool>>);
impl VideoRenderCancellation for Stop {
    fn is_cancelled(&self) -> bool {
        self.0.get()
    }
}
impl Stop {
    fn set(&self) {
        self.0.set(true);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Boundary(OutputBoundary),
    Preflight,
    Submit,
    Poll,
    Map,
    View,
    ViewDrop,
    Unmap,
    Bind,
    Commit,
    DiscardBinding,
    Cpu,
    Sink,
    Acquire,
    Present,
    Retire,
    Reap,
}
type Events = Rc<RefCell<Vec<Event>>>;

fn request() -> NativeOutputRequest {
    let mapping = VideoOutputMapping::default();
    NativeOutputRequest {
        context: GpuContextId {
            owner_id: 7,
            device_generation: 3,
        },
        frame: NativeFrameId {
            output_id: 11,
            project_epoch: 19,
            request_sequence: 23,
            geometry_generation: 5,
        },
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
        fence: CachePublicationFence {
            request_sequence: 23,
            blackout_generation: 0,
            configuration_generation: 0,
        },
        pts_ms: 33,
        duration_ms: 17,
        mapping_identity: VideoOutputMappingIdentity::from_mapping(&mapping),
        mapping,
    }
}

fn snapshot(request: &NativeOutputRequest, allocation: u64) -> SnapshotIdentity {
    SnapshotIdentity {
        frame: request.frame,
        allocation: GpuAllocationId {
            context: request.context,
            sequence: allocation,
        },
        completed_submission: GpuSubmissionId {
            context: request.context,
            sequence: 13,
        },
        pts_ms: request.pts_ms,
        duration_ms: request.duration_ms,
        width: request.key.width,
        height: request.key.height,
        format: VideoPixelFormat::Rgba8,
    }
}
fn pixels(request: &NativeOutputRequest) -> VideoFrame {
    VideoFrame {
        layer_id: 0,
        width: request.key.width,
        height: request.key.height,
        pts_ms: request.pts_ms,
        duration_ms: request.duration_ms,
        format: VideoPixelFormat::Rgba8,
        data: vec![1, 2, 3, 255, 4, 5, 6, 255],
    }
}
fn gpu_fault(
    request: &NativeOutputRequest,
    phase: GpuPhase,
    kind: NativeGpuFaultKind,
) -> NativeGpuFault {
    NativeGpuFault {
        context: request.context,
        frame: Some(request.frame),
        phase,
        kind,
        message: "scripted failure".into(),
    }
}

struct FakeAuthority {
    stop: Stop,
    cancel_at: Option<OutputBoundary>,
    reject_at: Option<(OutputBoundary, PresentRejection)>,
    events: Events,
}
impl NativeOutputAuthority for FakeAuthority {
    fn revalidate(
        &self,
        _: &NativeOutputRequest,
        boundary: OutputBoundary,
    ) -> Result<(), PresentRejection> {
        self.events.borrow_mut().push(Event::Boundary(boundary));
        if self.cancel_at == Some(boundary) {
            self.stop.set();
        }
        match self.reject_at {
            Some((at, reason)) if at == boundary => Err(reason),
            _ => Ok(()),
        }
    }
}

// Tickets are affine lookup handles. The registry, never a ticket's Drop, owns
// the simulated submitted allocation.
struct OwnerTicket(u64);
struct OwnerProbe(Rc<Cell<usize>>);
impl Drop for OwnerProbe {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}
#[derive(Clone, Copy)]
enum PreflightMode {
    Admit,
    Decline,
    Fail,
}
#[derive(Clone, Copy)]
enum CpuMode {
    Complete,
    Cancelled,
    Failed,
    StopThenComplete,
}
#[derive(Clone, Copy)]
enum PollStep {
    Pending,
    Cancelled,
    Fault(NativeGpuFaultKind),
    Complete,
}
#[derive(Clone, Copy, Default)]
enum PhaseScript {
    #[default]
    Matching,
    IsfFailure,
    CompositionFailure,
    ForeignIsfFrame,
    ForeignIsfContext,
    WrongCompositionPhase,
    ReversedSubmissions,
    SeparateTransactions,
}

type SharedGpu = Rc<RefCell<TestGpu>>;

struct TestGpu {
    owner: NativeGpuOwner,
}
impl TestGpu {
    fn new() -> SharedGpu {
        Rc::new(RefCell::new(Self {
            owner: NativeGpuFactory::new()
                .create_headless(7, NativeGpuRequirements::default())
                .expect("output integration tests require a headless adapter"),
        }))
    }

    fn resolve(
        &mut self,
        frame: NativeFrameId,
        phases: &[(GpuPhase, Option<NativeGpuFaultKind>)],
    ) -> Vec<ResolvedGpuPhase> {
        let quote = self.owner.quote(&NativeAllocationPlan::new(frame)).unwrap();
        let reservation = self.owner.reserve(quote).unwrap();
        let mut transaction = self.owner.begin_frame(frame, reservation).unwrap();
        let mut receipts = Vec::new();
        for &(kind, failure) in phases {
            let mut phase = transaction.begin_phase(kind).unwrap();
            if let Some(failure) = failure {
                phase.record_failure(failure, "injected output phase failure");
            }
            let encoder = phase
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let commands = phase.finish(vec![encoder.finish()]);
            receipts.push(transaction.submit_phase(commands).unwrap());
        }
        let pending = transaction.finish();
        let mut resolved = await_transaction(&mut self.owner, pending, &|| false).unwrap();
        receipts
            .into_iter()
            .map(|receipt| resolved.take_phase(receipt).unwrap())
            .collect()
    }

    fn pair(
        &mut self,
        frame: NativeFrameId,
        script: PhaseScript,
    ) -> Result<(IsfValidatedReceipt, ResolvedGpuPhase), NativeGpuFault> {
        let (isf, composition) = match script {
            PhaseScript::ForeignIsfFrame => {
                let foreign = NativeFrameId {
                    request_sequence: frame.request_sequence + 1,
                    ..frame
                };
                let isf = self
                    .resolve(foreign, &[(GpuPhase::IsfBatch, None)])
                    .remove(0);
                let composition = self
                    .resolve(frame, &[(GpuPhase::CompositionSnapshot, None)])
                    .remove(0);
                (isf, composition)
            }
            PhaseScript::ForeignIsfContext => {
                let other = TestGpu::new();
                let isf = other
                    .borrow_mut()
                    .resolve(frame, &[(GpuPhase::IsfBatch, None)])
                    .remove(0);
                let composition = self
                    .resolve(frame, &[(GpuPhase::CompositionSnapshot, None)])
                    .remove(0);
                drop(other);
                (isf, composition)
            }
            PhaseScript::SeparateTransactions => {
                let isf = self.resolve(frame, &[(GpuPhase::IsfBatch, None)]).remove(0);
                let composition = self
                    .resolve(frame, &[(GpuPhase::CompositionSnapshot, None)])
                    .remove(0);
                assert_eq!(isf.context(), composition.context());
                assert_eq!(isf.frame(), composition.frame());
                assert!(isf.submission().sequence < composition.submission().sequence);
                assert_ne!(isf.transaction(), composition.transaction());
                (isf, composition)
            }
            PhaseScript::ReversedSubmissions => {
                let mut phases = self.resolve(
                    frame,
                    &[
                        (GpuPhase::CompositionSnapshot, None),
                        (GpuPhase::IsfBatch, None),
                    ],
                );
                let composition = phases.remove(0);
                (phases.remove(0), composition)
            }
            _ => {
                let isf_failure = matches!(script, PhaseScript::IsfFailure)
                    .then_some(NativeGpuFaultKind::Validation);
                let composition_failure = matches!(script, PhaseScript::CompositionFailure)
                    .then_some(NativeGpuFaultKind::OutOfMemory);
                let composition_phase = if matches!(script, PhaseScript::WrongCompositionPhase) {
                    GpuPhase::MappingPresent
                } else {
                    GpuPhase::CompositionSnapshot
                };
                let mut phases = self.resolve(
                    frame,
                    &[
                        (GpuPhase::IsfBatch, isf_failure),
                        (composition_phase, composition_failure),
                    ],
                );
                let isf = phases.remove(0);
                (isf, phases.remove(0))
            }
        };
        Ok((IsfValidatedReceipt::from_phase(isf)?, composition))
    }
}
impl Drop for TestGpu {
    fn drop(&mut self) {
        self.owner.begin_stop();
        let started = Instant::now();
        loop {
            match self.owner.poll_retirement() {
                Ok(RetirementProgress::Reaped) => return,
                Ok(RetirementProgress::Pending { .. })
                    if started.elapsed() < Duration::from_secs(5) =>
                {
                    std::thread::yield_now();
                }
                result => {
                    if !std::thread::panicking() {
                        panic!("output fixture Stop/reap failed: {result:?}");
                    }
                    return;
                }
            }
        }
    }
}

#[derive(Clone, Copy, Default)]
enum MapScript {
    #[default]
    Complete,
    ForeignReceipt,
    MapFailure,
    ViewFailure,
    ShortCopy,
    UnmapFailure,
}

struct ScriptedMap {
    identity: SnapshotIdentity,
    bytes: Vec<u8>,
    events: Events,
    script: MapScript,
}
struct ScriptedView<'a> {
    bytes: &'a [u8],
    events: Events,
}
impl AsRef<[u8]> for ScriptedView<'_> {
    fn as_ref(&self) -> &[u8] {
        self.bytes
    }
}
impl Drop for ScriptedView<'_> {
    fn drop(&mut self) {
        self.events.borrow_mut().push(Event::ViewDrop);
    }
}
impl ScriptedMap {
    fn fault(&self, kind: NativeGpuFaultKind) -> NativeGpuFault {
        NativeGpuFault {
            context: self.identity.allocation.context,
            frame: Some(self.identity.frame),
            phase: GpuPhase::CompositionSnapshot,
            kind,
            message: "scripted snapshot mapper failure".into(),
        }
    }
}
impl SnapshotMapPort for ScriptedMap {
    type View<'a> = ScriptedView<'a>;
    fn poll_phase_and_map(&mut self) -> CompletionProgress<SnapshotPhaseReceipt, NativeGpuFault> {
        self.events.borrow_mut().push(Event::Map);
        if matches!(self.script, MapScript::MapFailure) {
            return CompletionProgress::Complete(Err(self.fault(NativeGpuFaultKind::Map)));
        }
        let mut identity = self.identity;
        if matches!(self.script, MapScript::ForeignReceipt) {
            identity.allocation.sequence += 1;
        }
        // P's real map adapter is not joined at this HEAD. Only this mapper
        // fixture uses its test receipt; G phase evidence above is always real.
        CompletionProgress::Complete(Ok(SnapshotPhaseReceipt::for_test(identity)))
    }
    fn mapped_view(&mut self) -> Result<Self::View<'_>, NativeGpuFault> {
        self.events.borrow_mut().push(Event::View);
        if matches!(self.script, MapScript::ViewFailure) {
            return Err(self.fault(NativeGpuFaultKind::SnapshotCopy));
        }
        Ok(ScriptedView {
            bytes: &self.bytes,
            events: Rc::clone(&self.events),
        })
    }
    fn unmap(&mut self) -> Result<(), NativeGpuFault> {
        self.events.borrow_mut().push(Event::Unmap);
        if matches!(self.script, MapScript::UnmapFailure) {
            return Err(self.fault(NativeGpuFaultKind::SnapshotCopy));
        }
        Ok(())
    }
}

type CorruptDescriptor = fn(&mut CacheCommitDescriptor);

struct FakeCompletionPort {
    stop: Stop,
    events: Events,
    mode: PreflightMode,
    cpu_mode: CpuMode,
    fail_submit: bool,
    stop_in_preflight: bool,
    stop_in_submit: bool,
    stop_in_poll: bool,
    steps: VecDeque<PollStep>,
    gpu: SharedGpu,
    phase_script: PhaseScript,
    map_script: MapScript,
    descriptor_mutation: Option<CorruptDescriptor>,
    snapshot_mutation: Option<CorruptDescriptor>,
    retirement_steps: VecDeque<Result<bool, NativeGpuFaultKind>>,
    request: Option<NativeOutputRequest>,
    owners: HashMap<u64, OwnerProbe>,
    next_owner: u64,
    drops: Rc<Cell<usize>>,
    completed_owner: Option<u64>,
    held_completed: HashMap<u64, CompletedNativeOutput<OwnerTicket>>,
    held_snapshots: HashMap<u64, RetiringPremappingSnapshot<OwnerTicket, ScriptedMap>>,
    completed_snapshot: Option<SnapshotIdentity>,
    cpu_calls: usize,
    submits: usize,
    polls: usize,
}
impl FakeCompletionPort {
    fn new(stop: Stop, events: Events, gpu: SharedGpu) -> Self {
        Self {
            stop,
            events,
            mode: PreflightMode::Admit,
            cpu_mode: CpuMode::Complete,
            fail_submit: false,
            stop_in_preflight: false,
            stop_in_submit: false,
            stop_in_poll: false,
            steps: VecDeque::from([PollStep::Complete]),
            gpu,
            phase_script: PhaseScript::Matching,
            map_script: MapScript::Complete,
            descriptor_mutation: None,
            snapshot_mutation: None,
            retirement_steps: VecDeque::new(),
            request: None,
            owners: HashMap::new(),
            next_owner: 1,
            drops: Rc::new(Cell::new(0)),
            completed_owner: None,
            held_completed: HashMap::new(),
            held_snapshots: HashMap::new(),
            completed_snapshot: None,
            cpu_calls: 0,
            submits: 0,
            polls: 0,
        }
    }
    fn register(&mut self) -> OwnerTicket {
        let id = self.next_owner;
        self.next_owner += 1;
        assert!(self
            .owners
            .insert(id, OwnerProbe(Rc::clone(&self.drops)))
            .is_none());
        OwnerTicket(id)
    }
    fn retain(&mut self, ticket: OwnerTicket) -> OwnerTicket {
        assert!(self.owners.contains_key(&ticket.0));
        self.events.borrow_mut().push(Event::Retire);
        ticket
    }
}
impl GpuCompletionPort for FakeCompletionPort {
    type Pending = OwnerTicket;
    type Completed = CompletedNativeOutput<OwnerTicket>;
    type Retiring = OwnerTicket;

    fn poll_completion(
        &mut self,
        pending: &mut Self::Pending,
        _: &dyn VideoRenderCancellation,
    ) -> CompletionProgress<Self::Completed, NativeGpuFault> {
        self.events.borrow_mut().push(Event::Poll);
        self.polls += 1;
        if self.stop_in_poll {
            self.stop.set();
        }
        let request = self.request.as_ref().expect("submitted request").clone();
        match self.steps.pop_front().expect("no repeated terminal poll") {
            PollStep::Pending => CompletionProgress::Pending,
            PollStep::Cancelled => CompletionProgress::Cancelled,
            PollStep::Fault(kind) => CompletionProgress::Complete(Err(gpu_fault(
                &request,
                GpuPhase::CompositionSnapshot,
                kind,
            ))),
            PollStep::Complete => {
                let (isf, composition) =
                    match self.gpu.borrow_mut().pair(request.frame, self.phase_script) {
                        Ok(phases) => phases,
                        Err(error) => return CompletionProgress::Complete(Err(error)),
                    };
                let id = pending.0;
                let mut image = snapshot(&request, id);
                image.completed_submission = composition.submission();
                self.completed_snapshot = Some(image);
                let mut descriptor = CacheCommitDescriptor {
                    key: request.key.clone(),
                    snapshot: image,
                    fence: request.fence,
                };
                let mut snapshot_descriptor = descriptor.clone();
                if let Some(mutate) = self.snapshot_mutation {
                    mutate(&mut snapshot_descriptor);
                }
                let mut bytes = pixels(&request).data;
                if matches!(self.map_script, MapScript::ShortCopy) {
                    bytes.pop();
                }
                let mapper = ScriptedMap {
                    identity: snapshot_descriptor.snapshot,
                    bytes,
                    events: Rc::clone(&self.events),
                    script: self.map_script,
                };
                pending.0 = 0;
                let mut snapshot_pending = NativeGpuSnapshotter::begin(SnapshotEncoding {
                    descriptor: snapshot_descriptor.clone(),
                    gpu: OwnerTicket(id),
                    mapper,
                    readback_row_pitch: u64::from(snapshot_descriptor.snapshot.width) * 4,
                });
                let snapshot = match NativeGpuSnapshotter::poll(
                    &mut snapshot_pending,
                    &snapshot_descriptor,
                    &|| false,
                ) {
                    SnapshotProgress::Complete(snapshot) => snapshot,
                    SnapshotProgress::Failed(error) => {
                        pending.0 = id;
                        assert!(self
                            .held_snapshots
                            .insert(id, snapshot_pending.into_retirement())
                            .is_none());
                        return CompletionProgress::Complete(Err(error));
                    }
                    _ => panic!("one-step mapper must complete or return its typed failure"),
                };
                if let Some(mutate) = self.descriptor_mutation {
                    mutate(&mut descriptor);
                }
                let completed =
                    CompletedNativeOutput::new(isf, composition, descriptor.clone(), snapshot);
                assert_eq!(completed.descriptor(), &descriptor);
                assert!(self.completed_owner.replace(id).is_none());
                CompletionProgress::Complete(Ok(completed))
            }
        }
    }
    fn cancel_and_retire(&mut self, pending: Self::Pending) -> Self::Retiring {
        self.retain(pending)
    }
}
impl NativeOutputDriver for FakeCompletionPort {
    type Inputs = Vec<u8>;
    type Admitted = Vec<u8>;
    type Gpu = OwnerTicket;
    type Cpu = Vec<u8>;

    fn preflight(
        &mut self,
        inputs: Self::Inputs,
        request: &NativeOutputRequest,
    ) -> NativePreflight<Self::Inputs, Self::Admitted> {
        self.events.borrow_mut().push(Event::Preflight);
        self.request = Some(request.clone());
        if self.stop_in_preflight {
            self.stop.set();
        }
        match self.mode {
            PreflightMode::Admit => NativePreflight::Admitted(inputs),
            PreflightMode::Decline => NativePreflight::Declined {
                inputs,
                reason: NativeGpuDecline::BudgetExceeded {
                    live: 10,
                    requested: 8,
                    limit: 16,
                },
            },
            PreflightMode::Fail => NativePreflight::Failed {
                inputs,
                fault: gpu_fault(
                    request,
                    GpuPhase::Initialize,
                    NativeGpuFaultKind::OutOfMemory,
                ),
            },
        }
    }
    fn submit(
        &mut self,
        admitted: Self::Admitted,
        request: &NativeOutputRequest,
    ) -> Result<Self::Pending, SubmissionFailure<Self::Retiring>> {
        assert_eq!(admitted, vec![42, 9]);
        self.events.borrow_mut().push(Event::Submit);
        self.submits += 1;
        let ticket = self.register();
        if self.stop_in_submit {
            self.stop.set();
        }
        if self.fail_submit {
            Err(SubmissionFailure {
                fault: gpu_fault(request, GpuPhase::IsfBatch, NativeGpuFaultKind::Submission),
                retirement: ticket,
            })
        } else {
            Ok(ticket)
        }
    }
    fn execute_cpu(
        &mut self,
        inputs: Self::Inputs,
        _: &dyn VideoRenderCancellation,
    ) -> CpuExecution<Self::Cpu> {
        self.events.borrow_mut().push(Event::Cpu);
        self.cpu_calls += 1;
        match self.cpu_mode {
            CpuMode::Complete => CpuExecution::Complete(inputs),
            CpuMode::Cancelled => CpuExecution::Cancelled,
            CpuMode::Failed => CpuExecution::Failed(gpu_fault(
                self.request.as_ref().unwrap(),
                GpuPhase::Initialize,
                NativeGpuFaultKind::Validation,
            )),
            CpuMode::StopThenComplete => {
                self.stop.set();
                CpuExecution::Complete(inputs)
            }
        }
    }
    fn retire_completed(&mut self, completed: CompletedNativeOutput<Self::Gpu>) -> Self::Retiring {
        // A corrupt descriptor cannot select another allocation's retirement.
        let id = self
            .completed_owner
            .take()
            .expect("original completed owner");
        assert!(self.held_completed.insert(id, completed).is_none());
        self.retain(OwnerTicket(id))
    }
    fn retire_ready(&mut self, ready: ReadyNativeGpuOutput<Self::Gpu>) -> Self::Retiring {
        self.retain(ready.gpu)
    }
    fn poll_retirement(&mut self, retiring: &mut Self::Retiring) -> Result<bool, NativeGpuFault> {
        assert!(self.owners.contains_key(&retiring.0));
        match self.retirement_steps.pop_front().unwrap_or(Ok(true)) {
            Ok(true) => {
                self.events.borrow_mut().push(Event::Reap);
                self.held_completed.remove(&retiring.0);
                self.held_snapshots.remove(&retiring.0);
                self.owners.remove(&retiring.0);
                Ok(true)
            }
            Ok(false) => Ok(false),
            Err(kind) => Err(gpu_fault(
                self.request.as_ref().unwrap(),
                GpuPhase::Retirement,
                kind,
            )),
        }
    }
}

#[derive(Clone, Copy)]
enum CommitHook {
    None,
    Stop,
    StopAndReject,
    Blackout,
    Configuration,
    StopAfterPublish,
}
struct FakeCache {
    core: NativeOutputCache,
    binding: Option<ValidatedSnapshotBinding>,
    stop: Stop,
    hook: CommitHook,
    foreign_receipt: bool,
    bind_rejection: Option<SnapshotCommitRejection>,
    stop_in_bind: bool,
    commits: usize,
    publications: usize,
    committed_pixels: Option<*const u8>,
    events: Events,
}
impl BoundSnapshotCache for FakeCache {
    fn bind_validated(
        &mut self,
        binding: ValidatedSnapshotBinding,
    ) -> Result<(), SnapshotCommitRejection> {
        self.events.borrow_mut().push(Event::Bind);
        if self.stop_in_bind {
            self.stop.set();
        }
        if let Some(error) = self.bind_rejection {
            return Err(error);
        }
        assert!(self.binding.replace(binding).is_none());
        Ok(())
    }
    fn discard_binding(&mut self) {
        self.binding = None;
        self.events.borrow_mut().push(Event::DiscardBinding);
    }
}
impl CpuSnapshotCommit for FakeCache {
    fn commit(
        &mut self,
        key: &VideoOutputLastValidKey,
        frame: NativeFrameId,
        pixels: VideoFrame,
        fence: CachePublicationFence,
    ) -> Result<CacheCommitReceipt, SnapshotCommitRejection> {
        self.events.borrow_mut().push(Event::Commit);
        self.commits += 1;
        match self.hook {
            CommitHook::Stop => self.stop.set(),
            CommitHook::StopAndReject => {
                self.stop.set();
                return Err(SnapshotCommitRejection::KeyMismatch);
            }
            CommitHook::Blackout => self.core.blackout().unwrap(),
            CommitHook::Configuration => self.core.invalidate_configuration().unwrap(),
            CommitHook::None | CommitHook::StopAfterPublish => {}
        }
        let binding = self.binding.take().expect("validated binding");
        if self.foreign_receipt {
            // Negative-only cache impersonator: no real publication occurred.
            let mut descriptor = binding.descriptor;
            descriptor.snapshot.allocation.sequence += 1;
            return Ok(CacheCommitReceipt::for_test(descriptor));
        }
        self.committed_pixels = Some(pixels.data.as_ptr());
        let published = self
            .core
            .commit_bound(binding, key, frame, pixels, fence, &self.stop)?;
        self.publications += 1;
        if matches!(self.hook, CommitHook::StopAfterPublish) {
            self.stop.set();
        }
        Ok(CacheCommitReceipt::from_publication(published))
    }
    fn invalidate_output(&mut self, output: VideoOutputId) {
        if output == self.core.output_id {
            self.core.blackout().unwrap();
        }
    }
}

#[derive(Clone, Copy)]
enum SinkMode {
    Present,
    Lost,
    TimeoutRetry,
    NoRetry,
    Fault,
    ForeignRetry,
    ResetRetry,
}
#[derive(Debug, PartialEq, Eq)]
struct FakeReceipt {
    frame: NativeFrameId,
    serial: usize,
}
struct FakePresentSink<'a> {
    mode: SinkMode,
    stop: Stop,
    stop_after_acquire: bool,
    stop_when_returning: bool,
    retry_mutation: Option<fn(&mut ReadyNativeGpuOutput<OwnerTicket>)>,
    events: Events,
    acquired: usize,
    presented: usize,
    retained: Vec<ReadyNativeGpuOutput<OwnerTicket>>,
    permit_lifetime: PhantomData<&'a FakeAuthority>,
}
impl<'a> FakePresentSink<'a> {
    fn new(h: &Harness, mode: SinkMode) -> Self {
        Self {
            mode,
            stop: h.stop.clone(),
            stop_after_acquire: false,
            stop_when_returning: false,
            retry_mutation: None,
            events: Rc::clone(&h.events),
            acquired: 0,
            presented: 0,
            retained: Vec::new(),
            permit_lifetime: PhantomData,
        }
    }
}
impl<'a> NativePresentSink for FakePresentSink<'a> {
    type Ready = ReadyNativeGpuOutput<OwnerTicket>;
    type Permit = NativePresentPermit<'a, FakeAuthority>;
    type Receipt = FakeReceipt;
    fn present_once(
        &mut self,
        mut ready: Self::Ready,
        permit: Self::Permit,
    ) -> Result<PresentOutcome<Self::Ready, Self::Receipt>, NativeGpuFault> {
        self.events.borrow_mut().push(Event::Sink);
        if let Err(reason) = permit.before_acquire(&ready) {
            return Ok(PresentOutcome::NotPresented {
                reason,
                retry: Some(ready),
            });
        }
        self.acquired += 1;
        self.events.borrow_mut().push(Event::Acquire);
        match self.mode {
            SinkMode::Lost | SinkMode::ResetRetry | SinkMode::ForeignRetry => {
                if let Some(mutate) = self.retry_mutation {
                    mutate(&mut ready);
                }
                if matches!(self.mode, SinkMode::ForeignRetry) {
                    ready.request.context.device_generation += 1;
                }
                if matches!(self.mode, SinkMode::ResetRetry) {
                    ready.retry_remaining = true;
                }
                if self.stop_when_returning {
                    self.stop.set();
                }
                return Ok(PresentOutcome::NotPresented {
                    reason: PresentRejection::SurfaceLostOrOutdated,
                    retry: Some(ready),
                });
            }
            SinkMode::TimeoutRetry => {
                if self.stop_when_returning {
                    self.stop.set();
                }
                return Ok(PresentOutcome::NotPresented {
                    reason: PresentRejection::SurfaceTimeout,
                    retry: Some(ready),
                });
            }
            _ => {}
        }
        if self.stop_after_acquire {
            self.stop.set();
        }
        if let Err(reason) = permit.before_present(&ready) {
            return Ok(PresentOutcome::NotPresented {
                reason,
                retry: Some(ready),
            });
        }
        let frame = ready.request.frame;
        match self.mode {
            SinkMode::Present => {
                self.presented += 1;
                self.events.borrow_mut().push(Event::Present);
                self.retained.push(ready);
                if self.stop_when_returning {
                    self.stop.set();
                }
                Ok(PresentOutcome::Presented(FakeReceipt {
                    frame,
                    serial: self.presented,
                }))
            }
            SinkMode::NoRetry => {
                self.retained.push(ready);
                if self.stop_when_returning {
                    self.stop.set();
                }
                Ok(PresentOutcome::NotPresented {
                    reason: PresentRejection::SurfaceTimeout,
                    retry: None,
                })
            }
            SinkMode::Fault => {
                let fault = gpu_fault(
                    &ready.request,
                    GpuPhase::MappingPresent,
                    NativeGpuFaultKind::DeviceLost,
                );
                self.retained.push(ready);
                if self.stop_when_returning {
                    self.stop.set();
                }
                Err(fault)
            }
            _ => unreachable!("retry cases returned before presentation"),
        }
    }
}

struct Harness {
    request: NativeOutputRequest,
    driver: FakeCompletionPort,
    cache: FakeCache,
    authority: FakeAuthority,
    stop: Stop,
    events: Events,
}
impl Harness {
    fn new(gpu: &SharedGpu) -> Self {
        let stop = Stop::default();
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut request = request();
        request.context = gpu.borrow().owner.context().id();
        let mut core = NativeOutputCache::new(request.frame.output_id);
        request.fence = core
            .begin_request(request.key.clone(), request.frame)
            .unwrap();
        Self {
            request,
            driver: FakeCompletionPort::new(stop.clone(), Rc::clone(&events), Rc::clone(gpu)),
            cache: FakeCache {
                core,
                binding: None,
                stop: stop.clone(),
                hook: CommitHook::None,
                foreign_receipt: false,
                bind_rejection: None,
                stop_in_bind: false,
                commits: 0,
                publications: 0,
                committed_pixels: None,
                events: Rc::clone(&events),
            },
            authority: FakeAuthority {
                stop: stop.clone(),
                cancel_at: None,
                reject_at: None,
                events: Rc::clone(&events),
            },
            stop,
            events,
        }
    }
    fn begin(&mut self) -> BegunNativeOutput<FakeCompletionPort> {
        PreparedNativeOutput::new(self.request.clone(), vec![42, 9]).begin(
            &mut self.driver,
            &self.authority,
            &self.stop,
        )
    }
    fn pending(&mut self) -> PendingNativeGpuOutput<OwnerTicket> {
        match self.begin() {
            BegunNativeOutput::Native(pending) => pending,
            _ => panic!("expected pending"),
        }
    }
    fn ready(&mut self) -> ReadyNativeGpuOutput<OwnerTicket> {
        let pending = self.pending();
        match pending.poll(
            &mut self.driver,
            &mut self.cache,
            &self.authority,
            &self.stop,
        ) {
            PolledNativeOutput::Ready(ready) => ready,
            _ => panic!("expected Ready"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Cause {
    Cancelled,
    Authority(PresentRejection),
    Cache(SnapshotCommitRejection),
    Fault(NativeGpuFaultKind),
}
fn cause(reason: &OutputRejection) -> Cause {
    match reason {
        OutputRejection::Cancelled => Cause::Cancelled,
        OutputRejection::Authority(reason) => Cause::Authority(*reason),
        OutputRejection::Cache(reason) => Cause::Cache(*reason),
        OutputRejection::Fault(error) => Cause::Fault(error.kind),
    }
}
fn drive_to_rejection(h: &mut Harness) -> Cause {
    let pending = match h.begin() {
        BegunNativeOutput::Rejected { reason, retained } => {
            match retained {
                UnsubmittedOutput::Inputs(inputs) | UnsubmittedOutput::Admitted(inputs) => {
                    assert_eq!(inputs, vec![42, 9])
                }
            }
            return cause(&reason);
        }
        BegunNativeOutput::Retiring(retiring) => return cause(retiring.reason()),
        BegunNativeOutput::Native(pending) => pending,
        BegunNativeOutput::Cpu { .. } => panic!("unexpected CPU route"),
    };
    let ready = match pending.poll(&mut h.driver, &mut h.cache, &h.authority, &h.stop) {
        PolledNativeOutput::Retiring(retiring) => return cause(retiring.reason()),
        PolledNativeOutput::Ready(ready) => ready,
        PolledNativeOutput::Pending(_) => panic!("unexpected pending"),
    };
    let mut sink = FakePresentSink::new(h, SinkMode::Present);
    let mut delivered = NativeDeliveryCounter::default();
    let result = present_native_output(
        ready,
        &mut h.driver,
        &mut sink,
        &h.authority,
        &h.stop,
        &mut delivered,
    );
    assert_eq!(sink.presented, 0);
    assert_eq!(delivered.delivered(), 0);
    match result {
        NativePresentation::Retiring(retiring) => cause(retiring.reason()),
        _ => panic!("expected retirement"),
    }
}

#[test]
fn native_output_preflight_decline_is_the_only_whole_tick_cpu_route() {
    let gpu = TestGpu::new();
    let mut h = Harness::new(&gpu);
    h.driver.mode = PreflightMode::Decline;
    match h.begin() {
        BegunNativeOutput::Cpu {
            reason,
            result: CpuExecution::Complete(result),
        } => {
            assert_eq!(
                reason,
                NativeGpuDecline::BudgetExceeded {
                    live: 10,
                    requested: 8,
                    limit: 16
                }
            );
            assert_eq!(result, vec![42, 9]);
        }
        _ => panic!("expected one CPU execution"),
    }
    assert_eq!(
        (
            h.driver.cpu_calls,
            h.driver.submits,
            h.driver.polls,
            h.cache.commits
        ),
        (1, 0, 0, 0)
    );
    for cpu_mode in [
        CpuMode::Cancelled,
        CpuMode::Failed,
        CpuMode::StopThenComplete,
    ] {
        let mut h = Harness::new(&gpu);
        h.driver.mode = PreflightMode::Decline;
        h.driver.cpu_mode = cpu_mode;
        match h.begin() {
            BegunNativeOutput::Cpu { result, .. } => match (cpu_mode, result) {
                (CpuMode::Cancelled | CpuMode::StopThenComplete, CpuExecution::Cancelled) => {}
                (CpuMode::Failed, CpuExecution::Failed(error)) => {
                    assert_eq!(error.kind, NativeGpuFaultKind::Validation)
                }
                _ => panic!("CPU cancellation/fault must survive"),
            },
            _ => panic!("expected CPU terminal outcome"),
        }
        assert_eq!((h.driver.cpu_calls, h.driver.submits), (1, 0));
    }
}

#[test]
fn native_output_admitted_failures_never_restart_on_cpu() {
    let gpu = TestGpu::new();
    let mut h = Harness::new(&gpu);
    h.driver.mode = PreflightMode::Fail;
    assert_eq!(
        drive_to_rejection(&mut h),
        Cause::Fault(NativeGpuFaultKind::OutOfMemory)
    );
    assert_eq!((h.driver.cpu_calls, h.driver.submits), (0, 0));
    let mut h = Harness::new(&gpu);
    h.driver.fail_submit = true;
    assert_eq!(
        drive_to_rejection(&mut h),
        Cause::Fault(NativeGpuFaultKind::Submission)
    );
    assert_eq!(
        (h.driver.cpu_calls, h.driver.submits, h.driver.owners.len()),
        (0, 1, 1)
    );
    for step in [
        PollStep::Cancelled,
        PollStep::Fault(NativeGpuFaultKind::Map),
    ] {
        let mut h = Harness::new(&gpu);
        h.driver.steps = VecDeque::from([step]);
        let expected = match step {
            PollStep::Cancelled => Cause::Cancelled,
            _ => Cause::Fault(NativeGpuFaultKind::Map),
        };
        assert_eq!(drive_to_rejection(&mut h), expected);
        assert_eq!(
            (h.driver.cpu_calls, h.cache.commits, h.driver.owners.len()),
            (0, 0, 1)
        );
    }
}

#[test]
fn native_output_stop_takes_priority_inside_preflight_submit_and_completion() {
    let gpu = TestGpu::new();
    for mode in [
        PreflightMode::Admit,
        PreflightMode::Decline,
        PreflightMode::Fail,
    ] {
        let mut h = Harness::new(&gpu);
        h.driver.mode = mode;
        h.driver.stop_in_preflight = true;
        assert_eq!(drive_to_rejection(&mut h), Cause::Cancelled);
        assert_eq!((h.driver.cpu_calls, h.driver.submits), (0, 0));
    }
    for failure in [false, true] {
        let mut h = Harness::new(&gpu);
        h.driver.stop_in_submit = true;
        h.driver.fail_submit = failure;
        assert_eq!(drive_to_rejection(&mut h), Cause::Cancelled);
        assert_eq!((h.driver.cpu_calls, h.driver.owners.len()), (0, 1));
    }
    let mut h = Harness::new(&gpu);
    h.driver.stop_in_poll = true;
    assert_eq!(drive_to_rejection(&mut h), Cause::Cancelled);
    assert_eq!(h.cache.commits, 0);
    let mut h = Harness::new(&gpu);
    h.request.key.width = 0;
    h.stop.set();
    assert_eq!(drive_to_rejection(&mut h), Cause::Cancelled);
    assert!(h.events.borrow().is_empty());
}

#[test]
fn native_output_live_stop_authority_and_mapping_are_checked_at_every_boundary() {
    let gpu = TestGpu::new();
    let boundaries = [
        OutputBoundary::BeforePreflight,
        OutputBoundary::BeforeCpuFallback,
        OutputBoundary::BeforeSubmit,
        OutputBoundary::AfterSubmit,
        OutputBoundary::BeforePoll,
        OutputBoundary::AfterPoll,
        OutputBoundary::PhasesResolved,
        OutputBoundary::SnapshotCopiedUnmapped,
        OutputBoundary::BeforeCacheCommit,
        OutputBoundary::AfterCacheCommit,
        OutputBoundary::BeforeReady,
        OutputBoundary::BeforePresentCall,
        OutputBoundary::BeforeAcquire,
        OutputBoundary::BeforePhysicalPresent,
    ];
    for boundary in boundaries {
        for expected in [
            Cause::Cancelled,
            Cause::Authority(PresentRejection::Revoked),
            Cause::Authority(PresentRejection::StaleMapping),
        ] {
            let mut h = Harness::new(&gpu);
            if boundary == OutputBoundary::BeforeCpuFallback {
                h.driver.mode = PreflightMode::Decline;
            }
            match expected {
                Cause::Cancelled => h.authority.cancel_at = Some(boundary),
                Cause::Authority(reason) => h.authority.reject_at = Some((boundary, reason)),
                _ => unreachable!(),
            }
            assert_eq!(
                drive_to_rejection(&mut h),
                expected,
                "boundary {boundary:?}"
            );
            assert_eq!(h.driver.cpu_calls, 0);
            assert!(h.events.borrow().contains(&Event::Boundary(boundary)));
            assert!(!h.events.borrow().contains(&Event::Present));
            let commit_precedes_boundary = matches!(
                boundary,
                OutputBoundary::AfterCacheCommit
                    | OutputBoundary::BeforeReady
                    | OutputBoundary::BeforePresentCall
                    | OutputBoundary::BeforeAcquire
                    | OutputBoundary::BeforePhysicalPresent
            );
            assert_eq!(
                h.cache.core.last_valid(&h.request.key).is_some(),
                commit_precedes_boundary
            );
        }
    }
}

#[test]
fn native_output_pending_has_no_cache_evidence_and_retirement_keeps_the_owner() {
    let gpu = TestGpu::new();
    let mut h = Harness::new(&gpu);
    h.driver.steps = VecDeque::from([PollStep::Pending, PollStep::Pending]);
    let pending = h.pending();
    let pending = match pending.poll(&mut h.driver, &mut h.cache, &h.authority, &h.stop) {
        PolledNativeOutput::Pending(pending) => pending,
        _ => panic!("pending became terminal"),
    };
    assert_eq!(h.cache.commits, 0);
    assert_eq!(h.driver.drops.get(), 0);
    h.stop.set();
    let mut retiring = match pending.poll(&mut h.driver, &mut h.cache, &h.authority, &h.stop) {
        PolledNativeOutput::Retiring(retiring) => retiring,
        _ => panic!("cancel must retire"),
    };
    assert_eq!(cause(retiring.reason()), Cause::Cancelled);
    h.driver.retirement_steps =
        VecDeque::from([Ok(false), Err(NativeGpuFaultKind::ScopeDrain), Ok(true)]);
    assert!(!retiring.poll(&mut h.driver).unwrap());
    assert!(!retiring.is_retired());
    assert_eq!(
        retiring.poll(&mut h.driver).unwrap_err().kind,
        NativeGpuFaultKind::ScopeDrain
    );
    assert!(!retiring.is_retired());
    assert_eq!((h.driver.owners.len(), h.driver.drops.get()), (1, 0));
    assert!(retiring.poll(&mut h.driver).unwrap());
    assert!(retiring.is_retired());
    assert!(retiring.poll(&mut h.driver).unwrap());
    assert_eq!((h.driver.owners.len(), h.driver.drops.get()), (0, 1));

    let mut h = Harness::new(&gpu);
    h.driver.fail_submit = true;
    let retiring = match h.begin() {
        BegunNativeOutput::Retiring(retiring) => retiring,
        _ => panic!("submission failure must retain"),
    };
    drop(retiring);
    assert_eq!(
        (h.driver.owners.len(), h.driver.drops.get()),
        (1, 0),
        "dropping lookup ticket must not imply reaping or Stop ACK"
    );
}

#[test]
fn native_output_phase_copy_and_pixel_identity_fail_closed() {
    let gpu = TestGpu::new();
    let phases = [
        (
            "ISF failure despite successful composition",
            PhaseScript::IsfFailure,
            NativeGpuFaultKind::Validation,
        ),
        (
            "composition failure",
            PhaseScript::CompositionFailure,
            NativeGpuFaultKind::OutOfMemory,
        ),
        (
            "foreign ISF frame",
            PhaseScript::ForeignIsfFrame,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "foreign ISF context",
            PhaseScript::ForeignIsfContext,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "wrong composition phase",
            PhaseScript::WrongCompositionPhase,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "reversed submission order",
            PhaseScript::ReversedSubmissions,
            NativeGpuFaultKind::IdentityMismatch,
        ),
    ];
    for (label, script, expected) in phases {
        let mut h = Harness::new(&gpu);
        h.driver.phase_script = script;
        assert_eq!(
            drive_to_rejection(&mut h),
            Cause::Fault(expected),
            "{label}"
        );
        assert_eq!(
            (h.cache.publications, h.driver.cpu_calls),
            (0, 0),
            "{label}"
        );
        assert!(h.cache.core.last_valid(&h.request.key).is_none());
        assert_eq!((h.driver.owners.len(), h.driver.drops.get()), (1, 0));
    }

    let descriptors: &[(&str, CorruptDescriptor)] = &[
        ("snapshot submission", |d| {
            d.snapshot.completed_submission.sequence += 1
        }),
        ("snapshot frame", |d| d.snapshot.frame.request_sequence += 1),
        ("snapshot context", |d| {
            d.snapshot.allocation.context.owner_id += 1
        }),
        ("snapshot width", |d| d.snapshot.width += 1),
        ("snapshot height", |d| d.snapshot.height += 1),
        ("snapshot PTS", |d| d.snapshot.pts_ms += 1),
        ("snapshot duration", |d| d.snapshot.duration_ms += 1),
        ("snapshot format", |d| {
            d.snapshot.format = VideoPixelFormat::Bgra8
        }),
    ];
    for &(label, mutate) in descriptors {
        let mut h = Harness::new(&gpu);
        h.driver.descriptor_mutation = Some(mutate);
        assert_eq!(
            drive_to_rejection(&mut h),
            Cause::Fault(NativeGpuFaultKind::IdentityMismatch),
            "{label}",
        );
        assert_eq!(
            (h.cache.publications, h.driver.cpu_calls),
            (0, 0),
            "{label}"
        );
        assert_eq!((h.driver.owners.len(), h.driver.drops.get()), (1, 0));
    }

    for (script, expected) in [
        (
            MapScript::ForeignReceipt,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (MapScript::MapFailure, NativeGpuFaultKind::Map),
        (MapScript::ViewFailure, NativeGpuFaultKind::SnapshotCopy),
        (MapScript::ShortCopy, NativeGpuFaultKind::SnapshotCopy),
        (MapScript::UnmapFailure, NativeGpuFaultKind::SnapshotCopy),
    ] {
        let mut h = Harness::new(&gpu);
        h.driver.map_script = script;
        assert_eq!(drive_to_rejection(&mut h), Cause::Fault(expected));
        assert_eq!((h.cache.publications, h.driver.cpu_calls), (0, 0));
        assert!(h.cache.core.last_valid(&h.request.key).is_none());
        assert_eq!(h.driver.held_snapshots.len(), 1);
        assert_eq!((h.driver.owners.len(), h.driver.drops.get()), (1, 0));
        if matches!(script, MapScript::ShortCopy | MapScript::UnmapFailure) {
            let events = h.events.borrow();
            let dropped = events.iter().position(|e| *e == Event::ViewDrop).unwrap();
            let unmapped = events.iter().position(|e| *e == Event::Unmap).unwrap();
            assert!(dropped < unmapped);
        }
    }
}

#[test]
fn native_output_cache_binding_and_live_commit_fences_cannot_mint_ready() {
    let gpu = TestGpu::new();
    for (hook, expected) in [
        (CommitHook::Stop, Cause::Cancelled),
        (CommitHook::StopAndReject, Cause::Cancelled),
        (
            CommitHook::Blackout,
            Cause::Cache(SnapshotCommitRejection::BlackoutFence),
        ),
        (
            CommitHook::Configuration,
            Cause::Cache(SnapshotCommitRejection::ConfigurationChanged),
        ),
        (CommitHook::StopAfterPublish, Cause::Cancelled),
    ] {
        let mut h = Harness::new(&gpu);
        h.cache.hook = hook;
        assert_eq!(drive_to_rejection(&mut h), expected);
        assert_eq!(
            h.cache.core.last_valid(&h.request.key).is_some(),
            matches!(hook, CommitHook::StopAfterPublish)
        );
        assert_eq!(h.driver.cpu_calls, 0);
    }
    let mut h = Harness::new(&gpu);
    h.cache.foreign_receipt = true;
    assert_eq!(
        drive_to_rejection(&mut h),
        Cause::Cache(SnapshotCommitRejection::SnapshotIdentityMismatch)
    );
    assert_eq!((h.cache.commits, h.cache.publications), (1, 0));
    assert!(h.cache.core.last_valid(&h.request.key).is_none());
    assert!(h.cache.binding.is_none());
    let mut h = Harness::new(&gpu);
    h.cache.bind_rejection = Some(SnapshotCommitRejection::SnapshotIdentityMismatch);
    assert_eq!(
        drive_to_rejection(&mut h),
        Cause::Cache(SnapshotCommitRejection::SnapshotIdentityMismatch)
    );
    assert_eq!(h.cache.commits, 0);
    for (reason, live_stop) in [
        (SnapshotCommitRejection::KeyMismatch, true),
        (SnapshotCommitRejection::Cancelled, false),
    ] {
        let mut h = Harness::new(&gpu);
        h.cache.stop_in_bind = live_stop;
        h.cache.bind_rejection = Some(reason);
        assert_eq!(drive_to_rejection(&mut h), Cause::Cancelled);
        assert_eq!(h.cache.commits, 0);
        assert!(h.cache.core.last_valid(&h.request.key).is_none());
        assert_eq!((h.driver.owners.len(), h.driver.drops.get()), (1, 0));
    }
}

#[test]
fn native_output_success_moves_pixels_then_presents_and_issues_delivery_evidence() {
    let gpu = TestGpu::new();
    let mut h = Harness::new(&gpu);
    let ready = h.ready();
    assert_eq!(ready.payload().0, 1);
    assert_eq!(Some(ready.snapshot_identity()), h.driver.completed_snapshot);
    assert_eq!(
        ready.cache_receipt().descriptor().snapshot,
        ready.snapshot_identity()
    );
    let evidence = ready.evidence();
    assert_eq!(evidence.freshness, VideoOutputRenderFreshness::Fresh);
    assert_eq!(evidence.output_id, h.request.frame.output_id);
    assert_eq!(evidence.project_render_epoch, h.request.frame.project_epoch);
    assert!(evidence.error.is_none());
    let cached = h.cache.core.last_valid(&h.request.key).unwrap();
    assert_eq!(Some(cached.data.as_ptr()), h.cache.committed_pixels);
    assert_eq!(h.cache.publications, 1);
    assert!(h.cache.binding.is_none());
    let mut owned = h.cache.core.owned_last_valid(&h.request.key).unwrap();
    assert_ne!(owned.data.as_ptr(), cached.data.as_ptr());
    owned.data[0] = 99;
    assert_eq!(h.cache.core.last_valid(&h.request.key).unwrap().data[0], 1);
    let mut sink = FakePresentSink::new(&h, SinkMode::Present);
    let mut counter = NativeDeliveryCounter::default();
    match present_native_output(
        ready,
        &mut h.driver,
        &mut sink,
        &h.authority,
        &h.stop,
        &mut counter,
    ) {
        NativePresentation::Presented(delivery) => {
            assert_eq!(delivery.frame(), h.request.frame);
            assert_eq!(
                delivery.receipt(),
                &FakeReceipt {
                    frame: h.request.frame,
                    serial: 1
                }
            );
        }
        _ => panic!("only physical Presented creates delivery/Follow token"),
    }
    assert_eq!(
        (counter.delivered(), sink.acquired, sink.presented),
        (1, 1, 1)
    );
    let events = h.events.borrow();
    let position = |event| events.iter().position(|actual| *actual == event).unwrap();
    assert!(position(Event::Poll) < position(Event::Map));
    assert!(position(Event::Map) < position(Event::View));
    assert!(position(Event::View) < position(Event::ViewDrop));
    assert!(position(Event::ViewDrop) < position(Event::Unmap));
    assert!(position(Event::Unmap) < position(Event::Bind));
    assert!(position(Event::Bind) < position(Event::Commit));
    assert!(position(Event::Commit) < position(Event::DiscardBinding));
    assert!(position(Event::Commit) < position(Event::Acquire));
    assert!(position(Event::Acquire) < position(Event::Present));
    drop(events);
    assert_eq!(
        (h.driver.owners.len(), h.driver.drops.get()),
        (1, 0),
        "present does not certify third submission completion"
    );
    let submitted = sink.retained.pop().unwrap();
    let mut ticket = h.driver.retire_ready(submitted);
    assert!(h.driver.poll_retirement(&mut ticket).unwrap());
    assert_eq!(h.driver.drops.get(), 1);
}

#[test]
fn native_output_present_errors_and_inner_stop_never_deliver() {
    let gpu = TestGpu::new();
    for mode in [
        SinkMode::TimeoutRetry,
        SinkMode::NoRetry,
        SinkMode::Fault,
        SinkMode::ForeignRetry,
    ] {
        let mut h = Harness::new(&gpu);
        let ready = h.ready();
        let mut sink = FakePresentSink::new(&h, mode);
        let mut counter = NativeDeliveryCounter::default();
        let result = present_native_output(
            ready,
            &mut h.driver,
            &mut sink,
            &h.authority,
            &h.stop,
            &mut counter,
        );
        match (mode, result) {
            (SinkMode::TimeoutRetry, NativePresentation::Retiring(r)) => assert_eq!(
                cause(r.reason()),
                Cause::Authority(PresentRejection::SurfaceTimeout)
            ),
            (SinkMode::NoRetry, NativePresentation::NotPresented(reason)) => {
                assert_eq!(reason, PresentRejection::SurfaceTimeout)
            }
            (SinkMode::Fault, NativePresentation::Failed(error)) => {
                assert_eq!(error.kind, NativeGpuFaultKind::DeviceLost)
            }
            (SinkMode::ForeignRetry, NativePresentation::Retiring(r)) => assert_eq!(
                cause(r.reason()),
                Cause::Fault(NativeGpuFaultKind::IdentityMismatch)
            ),
            _ => panic!("unexpected present outcome"),
        }
        assert_eq!(
            (counter.delivered(), sink.presented, h.driver.cpu_calls),
            (0, 0, 0)
        );
        assert_eq!((h.driver.owners.len(), h.driver.drops.get()), (1, 0));
        assert!(h.cache.core.last_valid(&h.request.key).is_some());
    }
    let mut h = Harness::new(&gpu);
    let ready = h.ready();
    let mut sink = FakePresentSink::new(&h, SinkMode::Present);
    sink.stop_after_acquire = true;
    let mut counter = NativeDeliveryCounter::default();
    match present_native_output(
        ready,
        &mut h.driver,
        &mut sink,
        &h.authority,
        &h.stop,
        &mut counter,
    ) {
        NativePresentation::Retiring(r) => assert_eq!(cause(r.reason()), Cause::Cancelled),
        _ => panic!("Stop inside sink must prevent present"),
    }
    assert_eq!(
        (counter.delivered(), sink.acquired, sink.presented),
        (0, 1, 0)
    );
}

#[test]
fn native_output_stop_on_sink_return_cancels_failures_but_counts_actual_present() {
    let gpu = TestGpu::new();
    for mode in [
        SinkMode::Lost,
        SinkMode::TimeoutRetry,
        SinkMode::NoRetry,
        SinkMode::Fault,
        SinkMode::Present,
    ] {
        let mut h = Harness::new(&gpu);
        let ready = h.ready();
        let mut sink = FakePresentSink::new(&h, mode);
        sink.stop_when_returning = true;
        let mut counter = NativeDeliveryCounter::default();
        let result = present_native_output(
            ready,
            &mut h.driver,
            &mut sink,
            &h.authority,
            &h.stop,
            &mut counter,
        );
        assert!(h.stop.is_cancelled());
        match (mode, result) {
            (SinkMode::Lost | SinkMode::TimeoutRetry, NativePresentation::Retiring(r)) => {
                assert_eq!(cause(r.reason()), Cause::Cancelled);
                assert!(!r.is_retired());
            }
            (SinkMode::NoRetry | SinkMode::Fault, NativePresentation::NotPresented(reason)) => {
                assert_eq!(reason, PresentRejection::Cancelled);
                assert_eq!(sink.retained.len(), 1);
            }
            (SinkMode::Present, NativePresentation::Presented(delivery)) => {
                assert_eq!(delivery.frame(), h.request.frame);
                assert_eq!(sink.retained.len(), 1);
            }
            _ => panic!("Stop must not return a retry or erase a completed physical present"),
        }
        let presented = u64::from(matches!(mode, SinkMode::Present));
        assert_eq!(counter.delivered(), presented);
        assert_eq!(sink.presented as u64, presented);
        assert_eq!(
            (
                h.driver.owners.len(),
                h.driver.drops.get(),
                h.driver.cpu_calls
            ),
            (1, 0, 0)
        );
        assert!(h.cache.core.last_valid(&h.request.key).is_some());
    }
}

#[test]
fn native_output_surface_retry_is_one_use_even_if_sink_resets_the_returned_flag() {
    let gpu = TestGpu::new();
    for second in [SinkMode::Present, SinkMode::Lost, SinkMode::ResetRetry] {
        let mut h = Harness::new(&gpu);
        let ready = h.ready();
        let mut sink = FakePresentSink::new(&h, SinkMode::Lost);
        let mut counter = NativeDeliveryCounter::default();
        let ready = match present_native_output(
            ready,
            &mut h.driver,
            &mut sink,
            &h.authority,
            &h.stop,
            &mut counter,
        ) {
            NativePresentation::Retry(ready) => ready,
            _ => panic!("first lost surface allows one retry"),
        };
        assert_eq!(counter.delivered(), 0);
        sink.mode = second;
        match present_native_output(
            ready,
            &mut h.driver,
            &mut sink,
            &h.authority,
            &h.stop,
            &mut counter,
        ) {
            NativePresentation::Presented(_) if matches!(second, SinkMode::Present) => {
                assert_eq!(counter.delivered(), 1)
            }
            NativePresentation::Retiring(r) if !matches!(second, SinkMode::Present) => {
                assert_eq!(
                    cause(r.reason()),
                    Cause::Authority(PresentRejection::SurfaceLostOrOutdated)
                );
                assert_eq!(counter.delivered(), 0);
            }
            _ => panic!("second loss must not renew retry"),
        }
        assert_eq!(
            (
                h.driver.submits,
                h.driver.polls,
                h.cache.commits,
                h.driver.cpu_calls
            ),
            (1, 1, 1, 0)
        );
    }
}

#[test]
fn native_output_surface_retry_cannot_replace_frozen_identity_or_cache_receipt() {
    let gpu = TestGpu::new();
    let mutations: &[fn(&mut ReadyNativeGpuOutput<OwnerTicket>)] = &[
        |r| r.request.frame.request_sequence += 1,
        |r| r.request.context.device_generation += 1,
        |r| r.snapshot.allocation.sequence += 1,
        |r| r.request.key.endpoint_name = Some("replacement".into()),
        |r| r.request.fence.configuration_generation += 1,
        |r| r.request.pts_ms += 1,
        |r| r.request.duration_ms += 1,
        |r| r.request.mapping.black_level = 0.5,
        |r| {
            let mut mapping = r.request.mapping;
            mapping.black_level = 0.5;
            r.request.mapping_identity = VideoOutputMappingIdentity::from_mapping(&mapping);
        },
        |r| {
            r.request.mapping.black_level = 0.5;
            r.request.mapping_identity =
                VideoOutputMappingIdentity::from_mapping(&r.request.mapping);
        },
        |r| {
            let mut descriptor = r.receipt.descriptor().clone();
            descriptor.snapshot.pts_ms += 1;
            r.receipt = CacheCommitReceipt::for_test(descriptor);
        },
        |r| {
            r.request.key.composition_id += 1;
            let mut descriptor = r.receipt.descriptor().clone();
            descriptor.key = r.request.key.clone();
            r.receipt = CacheCommitReceipt::for_test(descriptor);
        },
    ];
    for mutate in mutations {
        let mut h = Harness::new(&gpu);
        let ready = h.ready();
        let mut sink = FakePresentSink::new(&h, SinkMode::Lost);
        sink.retry_mutation = Some(*mutate);
        let mut counter = NativeDeliveryCounter::default();
        match present_native_output(
            ready,
            &mut h.driver,
            &mut sink,
            &h.authority,
            &h.stop,
            &mut counter,
        ) {
            NativePresentation::Retiring(retiring) => {
                assert_eq!(
                    cause(retiring.reason()),
                    Cause::Fault(NativeGpuFaultKind::IdentityMismatch)
                );
                let OutputRejection::Fault(error) = retiring.reason() else {
                    unreachable!()
                };
                assert_eq!(error.context, h.request.context);
                assert_eq!(error.frame, Some(h.request.frame));
                assert_eq!(error.phase, GpuPhase::MappingPresent);
            }
            _ => panic!("a substituted candidate must not earn another attempt"),
        }
        assert_eq!(
            (counter.delivered(), sink.presented, h.driver.cpu_calls),
            (0, 0, 0)
        );
        assert_eq!((h.driver.owners.len(), h.driver.drops.get()), (1, 0));
        assert!(h.cache.core.last_valid(&h.request.key).is_some());
    }
}

// These bindings model delayed/invalid callbacks deliberately. Normal execution
// obtains bindings only from completed output policy, not test proof builders.
fn delayed_binding(request: &NativeOutputRequest) -> ValidatedSnapshotBinding {
    ValidatedSnapshotBinding {
        descriptor: CacheCommitDescriptor {
            key: request.key.clone(),
            snapshot: snapshot(request, 1),
            fence: request.fence,
        },
    }
}
fn replay_cache_commit(
    cache: &mut NativeOutputCache,
    request: &NativeOutputRequest,
    stop: &Stop,
) -> Result<PublishedSnapshot, SnapshotCommitRejection> {
    cache.commit_bound(
        delayed_binding(request),
        &request.key,
        request.frame,
        pixels(request),
        request.fence,
        stop,
    )
}

#[test]
fn native_output_cache_rejects_duplicate_stale_blackout_and_configuration_callbacks() {
    let gpu = TestGpu::new();
    let mut h = Harness::new(&gpu);
    drop(h.ready());
    assert!(matches!(
        replay_cache_commit(&mut h.cache.core, &h.request, &h.stop),
        Err(SnapshotCommitRejection::StaleRequest)
    ));
    let mut next = h.request.clone();
    next.frame.request_sequence += 1;
    next.fence = h
        .cache
        .core
        .begin_request(next.key.clone(), next.frame)
        .unwrap();
    assert!(matches!(
        replay_cache_commit(&mut h.cache.core, &h.request, &h.stop),
        Err(SnapshotCommitRejection::StaleRequest)
    ));
    assert_eq!(
        h.cache.core.last_valid(&h.request.key).unwrap().data,
        pixels(&h.request).data
    );
    assert!(matches!(
        h.cache.core.begin_request(next.key.clone(), next.frame),
        Err(CacheFenceError::Rejected(
            SnapshotCommitRejection::StaleRequest
        ))
    ));

    h.cache.core.invalidate_configuration().unwrap();
    assert!(
        h.cache.core.last_valid(&h.request.key).is_some(),
        "same full key's premapping LastValid survives configuration revocation"
    );
    assert!(matches!(
        replay_cache_commit(&mut h.cache.core, &next, &h.stop),
        Err(SnapshotCommitRejection::ConfigurationChanged)
    ));
    next.frame.request_sequence += 1;
    next.fence = h
        .cache
        .core
        .begin_request(next.key.clone(), next.frame)
        .unwrap();
    h.cache.invalidate_output(next.frame.output_id + 1);
    assert!(h.cache.core.last_valid(&next.key).is_some());
    h.cache.invalidate_output(next.frame.output_id);
    assert!(h.cache.core.last_valid(&next.key).is_none());
    assert!(matches!(
        replay_cache_commit(&mut h.cache.core, &next, &h.stop),
        Err(SnapshotCommitRejection::BlackoutFence)
    ));
}

#[test]
fn native_output_cache_geometry_changes_keep_pixels_but_changed_keys_do_not_resurrect() {
    let gpu = TestGpu::new();
    let mut h = Harness::new(&gpu);
    drop(h.ready());
    let original_pixels = h
        .cache
        .core
        .last_valid(&h.request.key)
        .unwrap()
        .data
        .as_ptr();
    let mut next = h.request.clone();
    next.frame.request_sequence += 1;
    next.frame.geometry_generation += 1;
    next.fence = h
        .cache
        .core
        .begin_request(next.key.clone(), next.frame)
        .unwrap();
    assert!(next.fence.configuration_generation > h.request.fence.configuration_generation);
    assert_eq!(
        h.cache.core.last_valid(&next.key).unwrap().data.as_ptr(),
        original_pixels
    );
    assert!(matches!(
        replay_cache_commit(&mut h.cache.core, &h.request, &h.stop),
        Err(SnapshotCommitRejection::ConfigurationChanged),
    ));

    let original_key = next.key.clone();
    next.frame.request_sequence += 1;
    next.key.composition_id += 1;
    next.fence = h
        .cache
        .core
        .begin_request(next.key.clone(), next.frame)
        .unwrap();
    assert!(h.cache.core.last_valid(&original_key).is_none());
    assert!(h.cache.core.last_valid(&next.key).is_none());
    next.frame.request_sequence += 1;
    next.key = original_key;
    h.cache
        .core
        .begin_request(next.key.clone(), next.frame)
        .unwrap();
    assert!(
        h.cache.core.last_valid(&next.key).is_none(),
        "a previous key cannot resurrect an invalidated frame"
    );
}

#[test]
fn native_output_cache_uses_every_full_key_component_and_checks_bound_pixels() {
    let gpu = TestGpu::new();
    let changes: &[fn(&mut VideoOutputLastValidKey)] = &[
        |k| k.project_render_epoch += 1,
        |k| k.output_id += 1,
        |k| k.width += 1,
        |k| k.height += 1,
        |k| k.composition_id += 1,
        |k| k.kind = VideoOutputKind::NdiSender,
        |k| k.fullscreen = !k.fullscreen,
        |k| k.monitor_id = Some(2),
        |k| k.endpoint_name = Some("other".into()),
    ];
    for change in changes {
        let mut h = Harness::new(&gpu);
        drop(h.ready());
        let mut other_key = h.request.key.clone();
        change(&mut other_key);
        assert!(h.cache.core.last_valid(&other_key).is_none());
        let mut next = h.request.clone();
        next.frame.request_sequence += 1;
        next.fence = h
            .cache
            .core
            .begin_request(next.key.clone(), next.frame)
            .unwrap();
        let result = h.cache.core.commit_bound(
            delayed_binding(&next),
            &other_key,
            next.frame,
            pixels(&next),
            next.fence,
            &h.stop,
        );
        assert!(matches!(result, Err(SnapshotCommitRejection::KeyMismatch)));
        assert!(h.cache.core.last_valid(&h.request.key).is_some());
    }
    // The P completion owns its pixels privately now. Deliberately invalid
    // callbacks exercise the actual cache's final pixel binding checks instead
    // of fabricating an impossible completed snapshot.
    let pixel_changes: &[fn(&mut VideoFrame)] = &[
        |p| p.layer_id += 1,
        |p| p.width += 1,
        |p| p.height += 1,
        |p| p.pts_ms += 1,
        |p| p.duration_ms += 1,
        |p| p.format = VideoPixelFormat::Bgra8,
        |p| {
            p.data.pop();
        },
        |p| p.data.push(0),
        |p| p.data.clear(),
    ];
    for mutate in pixel_changes {
        let mut h = Harness::new(&gpu);
        let mut wrong = pixels(&h.request);
        mutate(&mut wrong);
        assert!(matches!(
            h.cache.core.commit_bound(
                delayed_binding(&h.request),
                &h.request.key,
                h.request.frame,
                wrong,
                h.request.fence,
                &h.stop,
            ),
            Err(SnapshotCommitRejection::SnapshotIdentityMismatch)
        ));
        assert!(h.cache.core.last_valid(&h.request.key).is_none());
    }
    let mut h = Harness::new(&gpu);
    let mut old_epoch = h.request.frame;
    old_epoch.project_epoch -= 1;
    assert!(matches!(
        h.cache.core.commit_bound(
            delayed_binding(&h.request),
            &h.request.key,
            old_epoch,
            pixels(&h.request),
            h.request.fence,
            &h.stop,
        ),
        Err(SnapshotCommitRejection::StaleEpoch)
    ));
    assert!(h.cache.core.last_valid(&h.request.key).is_none());
}

#[test]
fn native_output_invalid_request_and_sequence_overflow_fail_before_side_effects() {
    let gpu = TestGpu::new();
    let changes: &[fn(&mut NativeOutputRequest)] = &[
        |r| r.key.output_id += 1,
        |r| r.key.project_render_epoch += 1,
        |r| r.fence.request_sequence += 1,
        |r| {
            r.key.width = u32::MAX;
            r.key.height = u32::MAX;
        },
    ];
    for change in changes {
        let mut h = Harness::new(&gpu);
        change(&mut h.request);
        assert_eq!(
            drive_to_rejection(&mut h),
            Cause::Fault(NativeGpuFaultKind::IdentityMismatch)
        );
        assert_eq!((h.driver.submits, h.driver.cpu_calls), (0, 0));
    }
    let mut h = Harness::new(&gpu);
    h.request.key.width = 0;
    assert_eq!(
        drive_to_rejection(&mut h),
        Cause::Authority(PresentRejection::Suspended)
    );
    let mut h = Harness::new(&gpu);
    h.request.mapping.black_level = 0.5;
    assert_eq!(
        drive_to_rejection(&mut h),
        Cause::Authority(PresentRejection::StaleMapping)
    );

    let mut h = Harness::new(&gpu);
    let ready = h.ready();
    let mut sink = FakePresentSink::new(&h, SinkMode::Present);
    // Deliberate overflow fixture; no production access to counter mutation.
    let mut counter = NativeDeliveryCounter {
        delivered: u64::MAX,
    };
    match present_native_output(
        ready,
        &mut h.driver,
        &mut sink,
        &h.authority,
        &h.stop,
        &mut counter,
    ) {
        NativePresentation::Retiring(r) => assert_eq!(
            cause(r.reason()),
            Cause::Fault(NativeGpuFaultKind::SequenceExhausted)
        ),
        _ => panic!("overflow must not present"),
    }
    assert_eq!(
        (sink.acquired, sink.presented, counter.delivered()),
        (0, 0, u64::MAX)
    );
    // Deliberate generation exhaustion fixture; poisoned cache cannot resume.
    h.cache.core.fence.blackout_generation = u64::MAX;
    assert_eq!(
        h.cache.core.blackout(),
        Err(CacheFenceError::SequenceExhausted)
    );
    let mut next = h.request.frame;
    next.request_sequence += 1;
    assert_eq!(
        h.cache.core.begin_request(h.request.key.clone(), next),
        Err(CacheFenceError::SequenceExhausted)
    );
}

#[test]
fn native_output_real_receipts_require_one_authoritative_transaction() {
    let gpu = TestGpu::new();
    let mut h = Harness::new(&gpu);
    h.driver.phase_script = PhaseScript::SeparateTransactions;
    assert_eq!(
        drive_to_rejection(&mut h),
        Cause::Fault(NativeGpuFaultKind::IdentityMismatch),
    );
    assert_eq!(
        (h.cache.commits, h.cache.publications, h.driver.cpu_calls),
        (0, 0, 0)
    );
    assert!(h.cache.core.last_valid(&h.request.key).is_none());
    assert_eq!(h.driver.held_completed.len(), 1);
    assert!(!h.events.borrow().contains(&Event::Present));
}

#[test]
fn native_output_foreign_p_snapshot_never_publishes_and_retains_actual_owner() {
    let gpu = TestGpu::new();
    let cases: &[(CorruptDescriptor, SnapshotCommitRejection)] = &[
        (
            |d| d.snapshot.allocation.sequence += 17,
            SnapshotCommitRejection::SnapshotIdentityMismatch,
        ),
        (
            |d| d.snapshot.pts_ms += 1,
            SnapshotCommitRejection::SnapshotIdentityMismatch,
        ),
        (
            |d| d.key.composition_id += 1,
            SnapshotCommitRejection::KeyMismatch,
        ),
        (
            |d| d.fence.blackout_generation += 1,
            SnapshotCommitRejection::BlackoutFence,
        ),
        (
            |d| d.fence.configuration_generation += 1,
            SnapshotCommitRejection::ConfigurationChanged,
        ),
        (
            |d| {
                d.snapshot.frame.request_sequence += 1;
                d.fence.request_sequence += 1;
            },
            SnapshotCommitRejection::StaleRequest,
        ),
        (
            |d| {
                d.snapshot.frame.project_epoch += 1;
                d.key.project_render_epoch += 1;
            },
            SnapshotCommitRejection::StaleEpoch,
        ),
    ];
    for &(mutate, expected) in cases {
        let mut h = Harness::new(&gpu);
        h.driver.snapshot_mutation = Some(mutate);
        let pending = h.pending();
        let mut retiring = match pending.poll(&mut h.driver, &mut h.cache, &h.authority, &h.stop) {
            PolledNativeOutput::Retiring(retiring) => retiring,
            _ => panic!("foreign P snapshot cannot mint Ready"),
        };
        assert_eq!(cause(retiring.reason()), Cause::Cache(expected));
        assert_eq!(
            (h.cache.commits, h.cache.publications, h.driver.cpu_calls),
            (0, 0, 0)
        );
        assert!(h.events.borrow().contains(&Event::Bind));
        assert!(h.events.borrow().contains(&Event::DiscardBinding));
        assert!(h.cache.binding.is_none());
        assert!(h.cache.core.last_valid(&h.request.key).is_none());
        assert_eq!(
            (h.driver.held_completed.len(), h.driver.drops.get()),
            (1, 0)
        );
        h.driver.retirement_steps = VecDeque::from([Ok(false), Ok(true)]);
        assert!(!retiring.poll(&mut h.driver).unwrap());
        assert_eq!(
            (h.driver.held_completed.len(), h.driver.drops.get()),
            (1, 0)
        );
        assert!(retiring.poll(&mut h.driver).unwrap());
        assert_eq!(
            (h.driver.held_completed.len(), h.driver.drops.get()),
            (0, 1)
        );
    }
    // Corrupting I's claimed allocation also must not choose retirement's owner.
    let mut h = Harness::new(&gpu);
    h.driver.descriptor_mutation = Some(|d| d.snapshot.allocation.sequence += 1000);
    let pending = h.pending();
    let mut retiring = match pending.poll(&mut h.driver, &mut h.cache, &h.authority, &h.stop) {
        PolledNativeOutput::Retiring(retiring) => retiring,
        _ => panic!("a claimed foreign allocation cannot mint Ready"),
    };
    assert_eq!(
        cause(retiring.reason()),
        Cause::Cache(SnapshotCommitRejection::SnapshotIdentityMismatch)
    );
    assert_eq!((h.cache.publications, h.driver.drops.get()), (0, 0));
    assert!(h.driver.owners.contains_key(&1));
    assert!(retiring.poll(&mut h.driver).unwrap());
    assert_eq!((h.driver.owners.len(), h.driver.drops.get()), (0, 1));
}

#[test]
fn native_output_actual_cache_writer_joins_g_receipts_and_p_snapshot_into_ready() {
    let gpu = TestGpu::new();
    let mut h = Harness::new(&gpu);
    let mut cache = NativeOutputCache::new(h.request.frame.output_id);
    let fence = cache
        .begin_request(h.request.key.clone(), h.request.frame)
        .unwrap();
    assert_eq!(fence, h.request.fence);
    let pending = h.pending();
    let ready = {
        let mut writer = cache.writer(&h.stop);
        match pending.poll(&mut h.driver, &mut writer, &h.authority, &h.stop) {
            PolledNativeOutput::Ready(ready) => ready,
            _ => panic!("joined G/P completion and actual cache writer must produce Ready"),
        }
    };
    let image = ready.snapshot_identity();
    assert_eq!(Some(image), h.driver.completed_snapshot);
    assert_eq!(ready.cache_receipt().descriptor().snapshot, image);
    assert_eq!(ready.cache_receipt().descriptor().key, h.request.key);
    assert_eq!(ready.cache_receipt().descriptor().fence, fence);
    assert_eq!(
        ready.evidence().freshness,
        VideoOutputRenderFreshness::Fresh
    );
    let stored = cache.last_valid(&h.request.key).unwrap();
    assert_eq!(stored.data, pixels(&h.request).data);
    assert_eq!(
        (
            stored.width,
            stored.height,
            stored.pts_ms,
            stored.duration_ms
        ),
        (image.width, image.height, image.pts_ms, image.duration_ms)
    );
    assert_eq!(
        (h.cache.commits, h.cache.publications),
        (0, 0),
        "the FakeCache was not used in this integration"
    );
    assert!(!h.events.borrow().contains(&Event::Present));
    let mut retiring = h.driver.retire_ready(ready);
    assert!(h.driver.poll_retirement(&mut retiring).unwrap());
    assert_eq!(h.driver.drops.get(), 1);
}

#[test]
fn native_output_ready_rejects_unbound_snapshot_without_losing_its_owner() {
    let gpu = TestGpu::new();
    let mut h = Harness::new(&gpu);
    let mut pending = h.pending();
    let completed = match h.driver.poll_completion(&mut pending.pending, &h.stop) {
        CompletionProgress::Complete(Ok(completed)) => completed,
        _ => panic!("fixture must provide real G receipts and P copied snapshot"),
    };
    drop(pending);
    let descriptor = completed.descriptor().clone();
    let (reason, completed) = match completed.into_ready(h.request.clone()) {
        Err(rejected) => rejected,
        Ok(_) => panic!("copied snapshot without cache binding cannot become Ready"),
    };
    assert_eq!(
        cause(&reason),
        Cause::Fault(NativeGpuFaultKind::StateMismatch)
    );
    assert_eq!(completed.descriptor(), &descriptor);
    assert!(matches!(
        completed.snapshot.as_ref(),
        Some(OutputSnapshotOwnership::Copied(_))
    ));
    assert_eq!((h.cache.commits, h.cache.publications), (0, 0));
    assert!(h.cache.binding.is_none());
    assert!(h.cache.core.last_valid(&h.request.key).is_none());
    assert_eq!((h.driver.owners.len(), h.driver.drops.get()), (1, 0));

    let ticket = h.driver.retire_completed(completed);
    assert_eq!(ticket.0, descriptor.snapshot.allocation.sequence);
    let mut retiring = RetiringNativeOutput::new(reason, ticket);
    h.driver.retirement_steps = VecDeque::from([Ok(false), Ok(true)]);
    assert!(!retiring.poll(&mut h.driver).unwrap());
    assert_eq!(
        (h.driver.held_completed.len(), h.driver.drops.get()),
        (1, 0)
    );
    assert!(retiring.poll(&mut h.driver).unwrap());
    assert!(retiring.poll(&mut h.driver).unwrap());
    assert_eq!(
        (h.driver.held_completed.len(), h.driver.owners.len()),
        (0, 0)
    );
    assert_eq!(h.driver.drops.get(), 1);
}

fn begun(
    cache: &mut NativeOutputCache,
    base: &NativeOutputRequest,
    sequence: u64,
) -> NativeOutputRequest {
    let mut request = base.clone();
    request.frame.request_sequence = sequence;
    request.fence = cache
        .begin_request(request.key.clone(), request.frame)
        .unwrap();
    request
}

fn accepts(cache: &NativeOutputCache, request: &NativeOutputRequest) -> bool {
    cache.accepts_publication(&request.key, request.frame, request.fence)
}

/// ADR-01 section 7.2 snapshot ring: exactly one displaced, unpublished
/// request of the same key and fences stays publishable after the next
/// request began, and only in logical order. A newer publication, blackout
/// or a configuration change revokes it; nothing older is ever publishable.
#[test]
fn native_output_cache_snapshot_ring_publishes_one_displaced_request_in_logical_order() {
    let stop = Stop(Rc::new(Cell::new(false)));
    let base = request();
    let mut cache = NativeOutputCache::new(base.frame.output_id);
    let first = begun(&mut cache, &base, 1);
    let second = begun(&mut cache, &base, 2);
    assert!(accepts(&cache, &first) && accepts(&cache, &second));
    replay_cache_commit(&mut cache, &first, &stop).unwrap();
    assert!(cache.last_valid(&first.key).is_some());
    assert!(matches!(
        replay_cache_commit(&mut cache, &first, &stop),
        Err(SnapshotCommitRejection::StaleRequest)
    ));
    replay_cache_commit(&mut cache, &second, &stop).unwrap();

    // Two requests began: only the one displaced request is publishable.
    let third = begun(&mut cache, &base, 3);
    let fourth = begun(&mut cache, &base, 4);
    let fifth = begun(&mut cache, &base, 5);
    assert!(
        !accepts(&cache, &third),
        "nothing older than one displaced request"
    );
    assert!(matches!(
        replay_cache_commit(&mut cache, &third, &stop),
        Err(SnapshotCommitRejection::StaleRequest)
    ));
    assert!(accepts(&cache, &fourth) && accepts(&cache, &fifth));
    // A newer publication first revokes the displaced request (no reversal).
    replay_cache_commit(&mut cache, &fifth, &stop).unwrap();
    assert!(!accepts(&cache, &fourth));
    assert!(matches!(
        replay_cache_commit(&mut cache, &fourth, &stop),
        Err(SnapshotCommitRejection::StaleRequest)
    ));

    // Blackout fences the displaced request.
    let sixth = begun(&mut cache, &base, 6);
    let _seventh = begun(&mut cache, &base, 7);
    assert!(accepts(&cache, &sixth));
    cache.blackout().unwrap();
    assert!(matches!(
        replay_cache_commit(&mut cache, &sixth, &stop),
        Err(SnapshotCommitRejection::BlackoutFence)
    ));

    // A geometry (configuration) change leaves no displaced request.
    let eighth = begun(&mut cache, &base, 8);
    let mut resized = base.clone();
    resized.frame.geometry_generation += 1;
    let ninth = begun(&mut cache, &resized, 9);
    assert!(!accepts(&cache, &eighth));
    assert!(matches!(
        replay_cache_commit(&mut cache, &eighth, &stop),
        Err(SnapshotCommitRejection::ConfigurationChanged)
    ));
    assert!(accepts(&cache, &ninth));
    replay_cache_commit(&mut cache, &ninth, &stop).unwrap();
}
