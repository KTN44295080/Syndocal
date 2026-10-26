//! I1 output policy tests: scripted completion, real cache policy, no GPU or window.
use super::*;
use protocol::VideoOutputKind;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::rc::Rc;

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
    Bind,
    Commit,
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
type CorruptCompleted = fn(&mut CompletedNativeOutput<OwnerTicket>);

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
    corruption: Option<CorruptCompleted>,
    retirement_steps: VecDeque<Result<bool, NativeGpuFaultKind>>,
    request: Option<NativeOutputRequest>,
    owners: HashMap<u64, OwnerProbe>,
    next_owner: u64,
    drops: Rc<Cell<usize>>,
    submitted_pixels: Option<*const u8>,
    cpu_calls: usize,
    submits: usize,
    polls: usize,
}
impl FakeCompletionPort {
    fn new(stop: Stop, events: Events) -> Self {
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
            corruption: None,
            retirement_steps: VecDeque::new(),
            request: None,
            owners: HashMap::new(),
            next_owner: 1,
            drops: Rc::new(Cell::new(0)),
            submitted_pixels: None,
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
                let id = pending.0;
                pending.0 = 0;
                let image = snapshot(&request, id);
                let frame = pixels(&request);
                self.submitted_pixels = Some(frame.data.as_ptr());
                let mut completed = CompletedNativeOutput::for_test(
                    OwnerTicket(id),
                    image,
                    image,
                    frame,
                    Ok(OutputPhaseIdentity {
                        frame: request.frame,
                        submission: GpuSubmissionId {
                            context: request.context,
                            sequence: 11,
                        },
                        phase: GpuPhase::IsfBatch,
                    }),
                    Ok(OutputPhaseIdentity {
                        frame: request.frame,
                        submission: image.completed_submission,
                        phase: GpuPhase::CompositionSnapshot,
                    }),
                    true,
                );
                if let Some(corrupt) = self.corruption {
                    corrupt(&mut completed);
                }
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
        self.retain(completed.gpu)
    }
    fn retire_ready(&mut self, ready: ReadyNativeGpuOutput<Self::Gpu>) -> Self::Retiring {
        self.retain(ready.gpu)
    }
    fn poll_retirement(&mut self, retiring: &mut Self::Retiring) -> Result<bool, NativeGpuFault> {
        assert!(self.owners.contains_key(&retiring.0));
        match self.retirement_steps.pop_front().unwrap_or(Ok(true)) {
            Ok(true) => {
                self.events.borrow_mut().push(Event::Reap);
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
        let published = self.core.commit_bound(
            self.binding.take().expect("validated binding"),
            key,
            frame,
            pixels,
            fence,
            &self.stop,
        )?;
        let mut descriptor = published.descriptor().clone();
        if self.foreign_receipt {
            descriptor.snapshot.allocation.sequence += 1;
        }
        if matches!(self.hook, CommitHook::StopAfterPublish) {
            self.stop.set();
        }
        Ok(CacheCommitReceipt::for_test(descriptor))
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
    fn new() -> Self {
        let stop = Stop::default();
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut request = request();
        let mut core = NativeOutputCache::new(request.frame.output_id);
        request.fence = core
            .begin_request(request.key.clone(), request.frame)
            .unwrap();
        Self {
            request,
            driver: FakeCompletionPort::new(stop.clone(), Rc::clone(&events)),
            cache: FakeCache {
                core,
                binding: None,
                stop: stop.clone(),
                hook: CommitHook::None,
                foreign_receipt: false,
                bind_rejection: None,
                stop_in_bind: false,
                commits: 0,
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
    let mut h = Harness::new();
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
        let mut h = Harness::new();
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
    let mut h = Harness::new();
    h.driver.mode = PreflightMode::Fail;
    assert_eq!(
        drive_to_rejection(&mut h),
        Cause::Fault(NativeGpuFaultKind::OutOfMemory)
    );
    assert_eq!((h.driver.cpu_calls, h.driver.submits), (0, 0));
    let mut h = Harness::new();
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
        let mut h = Harness::new();
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
    for mode in [
        PreflightMode::Admit,
        PreflightMode::Decline,
        PreflightMode::Fail,
    ] {
        let mut h = Harness::new();
        h.driver.mode = mode;
        h.driver.stop_in_preflight = true;
        assert_eq!(drive_to_rejection(&mut h), Cause::Cancelled);
        assert_eq!((h.driver.cpu_calls, h.driver.submits), (0, 0));
    }
    for failure in [false, true] {
        let mut h = Harness::new();
        h.driver.stop_in_submit = true;
        h.driver.fail_submit = failure;
        assert_eq!(drive_to_rejection(&mut h), Cause::Cancelled);
        assert_eq!((h.driver.cpu_calls, h.driver.owners.len()), (0, 1));
    }
    let mut h = Harness::new();
    h.driver.stop_in_poll = true;
    assert_eq!(drive_to_rejection(&mut h), Cause::Cancelled);
    assert_eq!(h.cache.commits, 0);
    let mut h = Harness::new();
    h.request.key.width = 0;
    h.stop.set();
    assert_eq!(drive_to_rejection(&mut h), Cause::Cancelled);
    assert!(h.events.borrow().is_empty());
}

#[test]
fn native_output_live_stop_authority_and_mapping_are_checked_at_every_boundary() {
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
            let mut h = Harness::new();
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
    let mut h = Harness::new();
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

    let mut h = Harness::new();
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
    let cases: &[(&str, CorruptCompleted, NativeGpuFaultKind)] = &[
        (
            "ISF fault with successful composition",
            |c| {
                c.isf = Err(gpu_fault(
                    &request(),
                    GpuPhase::IsfBatch,
                    NativeGpuFaultKind::Validation,
                ));
            },
            NativeGpuFaultKind::Validation,
        ),
        (
            "composition fault",
            |c| {
                c.composition = Err(gpu_fault(
                    &request(),
                    GpuPhase::CompositionSnapshot,
                    NativeGpuFaultKind::OutOfMemory,
                ));
            },
            NativeGpuFaultKind::OutOfMemory,
        ),
        (
            "wrong ISF frame",
            |c| c.isf.as_mut().unwrap().frame.request_sequence += 1,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "wrong ISF context",
            |c| c.isf.as_mut().unwrap().submission.context.device_generation += 1,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "wrong phase",
            |c| c.composition.as_mut().unwrap().phase = GpuPhase::MappingPresent,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "submission order",
            |c| c.isf.as_mut().unwrap().submission.sequence = 13,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "snapshot submission",
            |c| c.snapshot_identity.completed_submission.sequence += 1,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "copy/unmap incomplete",
            |c| c.copied_unmapped = false,
            NativeGpuFaultKind::SnapshotCopy,
        ),
        (
            "GPU source allocation",
            |c| c.gpu_identity.allocation.sequence += 1,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "snapshot frame",
            |c| c.snapshot_identity.frame.request_sequence += 1,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "snapshot context",
            |c| c.snapshot_identity.allocation.context.owner_id += 1,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "snapshot dimensions",
            |c| c.snapshot_identity.width += 1,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "snapshot PTS",
            |c| c.snapshot_identity.pts_ms += 1,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "snapshot duration",
            |c| c.snapshot_identity.duration_ms += 1,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "snapshot format",
            |c| c.snapshot_identity.format = VideoPixelFormat::Bgra8,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "pixel layer",
            |c| c.pixels.as_mut().unwrap().layer_id = 1,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "pixel width",
            |c| c.pixels.as_mut().unwrap().width += 1,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "pixel height",
            |c| c.pixels.as_mut().unwrap().height += 1,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "pixel PTS",
            |c| c.pixels.as_mut().unwrap().pts_ms += 1,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "pixel duration",
            |c| c.pixels.as_mut().unwrap().duration_ms += 1,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "pixel format",
            |c| c.pixels.as_mut().unwrap().format = VideoPixelFormat::Bgra8,
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "short packed pixels",
            |c| {
                c.pixels.as_mut().unwrap().data.pop();
            },
            NativeGpuFaultKind::IdentityMismatch,
        ),
        (
            "missing owned pixels",
            |c| c.pixels = None,
            NativeGpuFaultKind::IdentityMismatch,
        ),
    ];
    for &(label, corrupt, expected) in cases {
        let mut h = Harness::new();
        h.driver.corruption = Some(corrupt);
        assert_eq!(
            drive_to_rejection(&mut h),
            Cause::Fault(expected),
            "{label}"
        );
        assert_eq!((h.cache.commits, h.driver.cpu_calls), (0, 0), "{label}");
        assert!(h.cache.core.last_valid(&h.request.key).is_none());
        assert_eq!((h.driver.owners.len(), h.driver.drops.get()), (1, 0));
    }
}

#[test]
fn native_output_cache_binding_and_live_commit_fences_cannot_mint_ready() {
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
        let mut h = Harness::new();
        h.cache.hook = hook;
        assert_eq!(drive_to_rejection(&mut h), expected);
        assert_eq!(
            h.cache.core.last_valid(&h.request.key).is_some(),
            matches!(hook, CommitHook::StopAfterPublish)
        );
        assert_eq!(h.driver.cpu_calls, 0);
    }
    let mut h = Harness::new();
    h.cache.foreign_receipt = true;
    assert_eq!(
        drive_to_rejection(&mut h),
        Cause::Fault(NativeGpuFaultKind::IdentityMismatch)
    );
    assert_eq!(h.cache.commits, 1);
    assert!(h.cache.core.last_valid(&h.request.key).is_some());
    let mut h = Harness::new();
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
        let mut h = Harness::new();
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
    let mut h = Harness::new();
    let ready = h.ready();
    assert_eq!(ready.payload().0, 1);
    assert_eq!(ready.snapshot_identity(), snapshot(&h.request, 1));
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
    assert_eq!(Some(cached.data.as_ptr()), h.driver.submitted_pixels);
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
    assert!(position(Event::Poll) < position(Event::Commit));
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
    for mode in [
        SinkMode::TimeoutRetry,
        SinkMode::NoRetry,
        SinkMode::Fault,
        SinkMode::ForeignRetry,
    ] {
        let mut h = Harness::new();
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
    let mut h = Harness::new();
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
    for mode in [
        SinkMode::Lost,
        SinkMode::TimeoutRetry,
        SinkMode::NoRetry,
        SinkMode::Fault,
        SinkMode::Present,
    ] {
        let mut h = Harness::new();
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
    for second in [SinkMode::Present, SinkMode::Lost, SinkMode::ResetRetry] {
        let mut h = Harness::new();
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
        let mut h = Harness::new();
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
    let mut h = Harness::new();
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
    let mut h = Harness::new();
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
        let mut h = Harness::new();
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
    let mut h = Harness::new();
    let mut wrong = pixels(&h.request);
    wrong.data.push(0);
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
        let mut h = Harness::new();
        change(&mut h.request);
        assert_eq!(
            drive_to_rejection(&mut h),
            Cause::Fault(NativeGpuFaultKind::IdentityMismatch)
        );
        assert_eq!((h.driver.submits, h.driver.cpu_calls), (0, 0));
    }
    let mut h = Harness::new();
    h.request.key.width = 0;
    assert_eq!(
        drive_to_rejection(&mut h),
        Cause::Authority(PresentRejection::Suspended)
    );
    let mut h = Harness::new();
    h.request.mapping.black_level = 0.5;
    assert_eq!(
        drive_to_rejection(&mut h),
        Cause::Authority(PresentRejection::StaleMapping)
    );

    let mut h = Harness::new();
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
