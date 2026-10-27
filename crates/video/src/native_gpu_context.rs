//! Single-owner native GPU device, phase broker and retirement registry.
//!
//! The immutable context may be shared; queue writes, scopes, submission and
//! completion belong to the mutable owner. Pending tokens are affine handles to
//! a registry which keeps strong resource ownership even when a caller drops one.
use std::any::Any;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, OnceLock};
use std::task::{Context, Poll, Wake, Waker};
use std::time::{Duration, Instant};

#[cfg(test)]
use crate::native_gpu_budget::NativeGpuBudgetSnapshot;
use crate::native_gpu_budget::{
    AllocationCharge, BudgetQuote, ChargeKind, FrameReservation, NativeAllocationPlan,
    NativeGpuBudget,
};
#[cfg(test)]
use crate::native_gpu_contract::GpuCompletionPort;
use crate::native_gpu_contract::{
    CompletionProgress, GpuAllocationId, GpuContextId, GpuOwnerState, GpuPhase, GpuSubmissionId,
    NativeFrameId, NativeGpuDecline, NativeGpuFault, NativeGpuFaultKind, RetirementId,
};
use crate::VideoRenderCancellation;

static NEXT_DEVICE_GENERATION: AtomicU64 = AtomicU64::new(1);
const ACTIVE_TOKEN: u8 = 0;
const CANCELLED_TOKEN: u8 = 1;
const TERMINAL_TOKEN: u8 = 2;
const MAX_UNCAPTURED_ERROR_CHARS: usize = 2_048;
// Failed drivers retain unmanaged handles as well as charged allocations.
// A byte limit alone cannot bound repeated zero-byte failed generations.
const MAX_RETAINED_DEVICE_GENERATIONS: usize = 8;

pub(crate) struct NativeGpuFactory {
    instance: Arc<wgpu::Instance>,
}

pub(crate) struct SurfaceAdapterTarget<'a> {
    pub instance: &'a Arc<wgpu::Instance>,
    pub surface: &'a wgpu::Surface<'static>,
}

pub(crate) struct NativeGpuRequirements {
    pub required_features: wgpu::Features,
    pub optional_features: wgpu::Features,
    pub required_limits: wgpu::Limits,
}

impl Default for NativeGpuRequirements {
    fn default() -> Self {
        Self {
            required_features: wgpu::Features::empty(),
            optional_features: wgpu::Features::TEXTURE_COMPRESSION_BC
                | wgpu::Features::FLOAT32_FILTERABLE,
            required_limits: wgpu::Limits::default(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct NativeGpuCapabilities {
    pub context: GpuContextId,
    #[cfg(test)]
    pub adapter_info: wgpu::AdapterInfo,
    #[cfg(test)]
    pub enabled_features: wgpu::Features,
    pub limits: wgpu::Limits,
    pub upload_row_pitch: Option<u64>,
}

#[derive(Debug)]
pub(crate) enum ContextCreateError {
    Instance(String),
    Surface(String),
    Adapter(String),
    RequiredFeature(wgpu::Features),
    RequiredLimit,
    Device(String),
    SequenceExhausted,
}

impl std::fmt::Display for ContextCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Instance(message) => write!(f, "native GPU instance: {message}"),
            Self::Surface(message) => write!(f, "native GPU surface: {message}"),
            Self::Adapter(message) => write!(f, "native GPU adapter: {message}"),
            Self::RequiredFeature(features) => {
                write!(f, "native GPU required features: {features:?}")
            }
            Self::RequiredLimit => write!(f, "native GPU required limits are unsupported"),
            Self::Device(message) => write!(f, "native GPU device: {message}"),
            Self::SequenceExhausted => write!(f, "native GPU device generation exhausted"),
        }
    }
}

pub(crate) struct NativeGpuContext {
    instance: Arc<wgpu::Instance>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    capabilities: NativeGpuCapabilities,
    lost: Arc<AtomicBool>,
    uncaptured: Arc<AtomicBool>,
    uncaptured_error: Arc<OnceLock<String>>,
}

impl NativeGpuContext {
    pub(crate) fn id(&self) -> GpuContextId {
        self.capabilities.context
    }

    /// Borrow handles only while an owner phase authorizes their use.
    pub(crate) fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub(crate) fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub(crate) fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    pub(crate) fn instance(&self) -> &Arc<wgpu::Instance> {
        &self.instance
    }

    pub(crate) fn capabilities(&self) -> &NativeGpuCapabilities {
        &self.capabilities
    }

    fn uncaptured_message(&self, prefix: &str) -> String {
        match self.uncaptured_error.get() {
            Some(error) => format!("{prefix}: {error}"),
            None => format!("{prefix}: diagnostic callback is still recording the error"),
        }
    }
}

fn negotiated_features(
    requirements: &NativeGpuRequirements,
    supported: wgpu::Features,
    limits: &wgpu::Limits,
) -> Result<wgpu::Features, ContextCreateError> {
    let missing = requirements.required_features - supported;
    if !missing.is_empty() {
        return Err(ContextCreateError::RequiredFeature(missing));
    }
    // wgpu checks min-alignment limits in the correct opposite direction.
    if !requirements.required_limits.check_limits(limits) {
        return Err(ContextCreateError::RequiredLimit);
    }
    Ok(requirements.required_features | (requirements.optional_features & supported))
}

impl NativeGpuFactory {
    pub(crate) fn new() -> Self {
        Self {
            instance: Arc::new(wgpu::Instance::new(&wgpu::InstanceDescriptor::default())),
        }
    }

    pub(crate) fn instance(&self) -> &Arc<wgpu::Instance> {
        &self.instance
    }

    pub(crate) fn create_for_surface(
        &self,
        target: SurfaceAdapterTarget<'_>,
        owner_id: u64,
        requirements: NativeGpuRequirements,
    ) -> Result<NativeGpuOwner, ContextCreateError> {
        if !Arc::ptr_eq(&self.instance, target.instance) {
            return Err(ContextCreateError::Instance(
                "surface belongs to another factory".into(),
            ));
        }
        self.create(Some(target.surface), owner_id, requirements)
    }

    #[cfg(test)]
    pub(crate) fn create_headless(
        &self,
        owner_id: u64,
        requirements: NativeGpuRequirements,
    ) -> Result<NativeGpuOwner, ContextCreateError> {
        self.create(None, owner_id, requirements)
    }

    fn create(
        &self,
        surface: Option<&wgpu::Surface<'static>>,
        owner_id: u64,
        requirements: NativeGpuRequirements,
    ) -> Result<NativeGpuOwner, ContextCreateError> {
        let adapter =
            pollster::block_on(self.instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: surface,
            }))
            .map_err(|error| ContextCreateError::Adapter(error.to_string()))?;
        let features = negotiated_features(&requirements, adapter.features(), &adapter.limits())?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Syndocal shared native GPU"),
            required_features: features,
            required_limits: requirements.required_limits,
            ..Default::default()
        }))
        .map_err(|error| ContextCreateError::Device(error.to_string()))?;
        let generation = NEXT_DEVICE_GENERATION
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| ContextCreateError::SequenceExhausted)?;
        let context_id = GpuContextId {
            owner_id,
            device_generation: generation,
        };
        let lost = Arc::new(AtomicBool::new(false));
        let lost_callback = Arc::clone(&lost);
        device.set_device_lost_callback(move |_, _| {
            lost_callback.store(true, Ordering::Release);
        });
        let uncaptured = Arc::new(AtomicBool::new(false));
        let uncaptured_callback = Arc::clone(&uncaptured);
        let uncaptured_error: Arc<OnceLock<String>> = Arc::new(OnceLock::new());
        let uncaptured_error_callback = Arc::clone(&uncaptured_error);
        device.on_uncaptured_error(Box::new(move |error| {
            uncaptured_callback.store(true, Ordering::Release);
            // The first cold-path failure is diagnostic evidence. Later errors
            // neither replace it nor grow the retained message without bound.
            uncaptured_error_callback.get_or_init(|| {
                error
                    .to_string()
                    .chars()
                    .take(MAX_UNCAPTURED_ERROR_CHARS)
                    .collect()
            });
        }));
        let capabilities = NativeGpuCapabilities {
            context: context_id,
            #[cfg(test)]
            adapter_info: adapter.get_info(),
            #[cfg(test)]
            enabled_features: device.features(),
            limits: device.limits(),
            upload_row_pitch: crate::isf_runtime::native_queue_upload_pitch(&adapter),
        };
        let context = Arc::new(NativeGpuContext {
            instance: Arc::clone(&self.instance),
            adapter,
            device,
            queue,
            capabilities,
            lost,
            uncaptured,
            uncaptured_error,
        });
        Ok(NativeGpuOwner {
            core: OwnedBrokerCore::new(BrokerCore::new(
                RealGpuDriver {
                    context: Arc::clone(&context),
                },
                context_id,
            )),
            context,
        })
    }
}

#[derive(Debug)]
struct DriverFailure {
    kind: NativeGpuFaultKind,
    message: String,
}

type ScopeFuture = Pin<Box<dyn Future<Output = Option<wgpu::Error>> + Send>>;

struct ScopeSet {
    futures: Vec<ScopeFuture>,
    failure: Option<DriverFailure>,
}

struct NoopWake;
impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

impl ScopeSet {
    /// Poll each newly-created pop future before allowing the next push. This
    /// executes the pop even if a backend defers its body until first poll.
    fn seal(device: &wgpu::Device) -> Self {
        let mut set = Self {
            futures: Vec::with_capacity(2),
            failure: None,
        };
        for _ in 0..2 {
            let device = device.clone();
            let mut future: ScopeFuture = Box::pin(async move { device.pop_error_scope().await });
            let waker = Waker::from(Arc::new(NoopWake));
            match future.as_mut().poll(&mut Context::from_waker(&waker)) {
                Poll::Ready(error) => set.accept_error(error),
                Poll::Pending => set.futures.push(future),
            }
        }
        set
    }

    fn accept_error(&mut self, error: Option<wgpu::Error>) {
        if let Some(error) = error {
            let kind = match error {
                wgpu::Error::OutOfMemory { .. } => NativeGpuFaultKind::OutOfMemory,
                wgpu::Error::Validation { .. } => NativeGpuFaultKind::Validation,
                _ => NativeGpuFaultKind::Submission,
            };
            self.failure.get_or_insert(DriverFailure {
                kind,
                message: error.to_string(),
            });
        }
    }

    fn poll(&mut self) -> Option<Result<(), DriverFailure>> {
        let waker = Waker::from(Arc::new(NoopWake));
        let mut index = 0;
        while index < self.futures.len() {
            match self.futures[index]
                .as_mut()
                .poll(&mut Context::from_waker(&waker))
            {
                Poll::Pending => index += 1,
                Poll::Ready(error) => {
                    drop(self.futures.swap_remove(index));
                    self.accept_error(error);
                }
            }
        }
        self.futures.is_empty().then(|| match self.failure.take() {
            Some(error) => Err(error),
            None => Ok(()),
        })
    }
}

trait CompletionDriver {
    type Ticket;
    type Scopes;
    fn poll(&mut self) -> Result<(), DriverFailure>;
    fn complete(&self, ticket: &Self::Ticket) -> bool;
    fn scopes(&mut self, scopes: &mut Self::Scopes) -> Option<Result<(), DriverFailure>>;
}

struct RealGpuDriver {
    context: Arc<NativeGpuContext>,
}

struct RealSubmission {
    completed: Arc<AtomicBool>,
}

impl CompletionDriver for RealGpuDriver {
    type Ticket = RealSubmission;
    type Scopes = ScopeSet;

    fn poll(&mut self) -> Result<(), DriverFailure> {
        if self.context.lost.load(Ordering::Acquire) {
            return Err(DriverFailure {
                kind: NativeGpuFaultKind::DeviceLost,
                message: "shared device lost".into(),
            });
        }
        if self.context.uncaptured.load(Ordering::Acquire) {
            return Err(DriverFailure {
                kind: NativeGpuFaultKind::ScopeDrain,
                message: self
                    .context
                    .uncaptured_message("uncaptured shared-device error"),
            });
        }
        self.context
            .device
            .poll(wgpu::PollType::Poll)
            .map_err(|error| DriverFailure {
                kind: NativeGpuFaultKind::ScopeDrain,
                message: error.to_string(),
            })?;
        // poll dispatches callbacks. Recheck before any successful receipt can
        // be minted from completion markers updated during that same call.
        if self.context.lost.load(Ordering::Acquire) {
            return Err(DriverFailure {
                kind: NativeGpuFaultKind::DeviceLost,
                message: "shared device lost during poll".into(),
            });
        }
        if self.context.uncaptured.load(Ordering::Acquire) {
            return Err(DriverFailure {
                kind: NativeGpuFaultKind::ScopeDrain,
                message: self
                    .context
                    .uncaptured_message("uncaptured shared-device error during poll"),
            });
        }
        Ok(())
    }

    fn complete(&self, ticket: &Self::Ticket) -> bool {
        ticket.completed.load(Ordering::Acquire)
    }

    fn scopes(&mut self, scopes: &mut Self::Scopes) -> Option<Result<(), DriverFailure>> {
        scopes.poll()
    }
}

struct PhaseRecord<D: CompletionDriver> {
    id: GpuSubmissionId,
    phase: GpuPhase,
    ticket: D::Ticket,
    scopes: D::Scopes,
    execution_failure: Option<DriverFailure>,
    scope_result: Option<Result<(), DriverFailure>>,
    complete: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameRecordStage {
    Recording,
    Sealed,
    // Only the final, separate mapping/present submission may release the
    // logical output slot. All strong owners and its reservation stay here.
    PresentRetirement,
}

/// The existing affine marker carries its private session capability and exact
/// transaction identity. Its terminal state is minted only after the registry
/// actually finishes that record, so a late token needs no session tombstone.
struct TransactionMarker {
    session: Arc<()>,
    id: RetirementId,
    state: AtomicU8,
}

impl Deref for TransactionMarker {
    type Target = AtomicU8;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

struct FrameRecord<D: CompletionDriver> {
    id: RetirementId,
    frame: NativeFrameId,
    stage: FrameRecordStage,
    /// Admitted beside one in-flight logical output. Such a record may open
    /// only a MappingPresent or Initialize phase and never carries ISF,
    /// composition/snapshot or replay work.
    auxiliary: bool,
    marker: Arc<TransactionMarker>,
    reservation: Option<FrameReservation>,
    resources: Vec<Arc<dyn Any + Send + Sync>>,
    allocations: Vec<GpuAllocationId>,
    owned_allocations: Vec<GpuAllocationId>,
    phases: Vec<PhaseRecord<D>>,
    open: Option<GpuPhase>,
    open_failure: Option<DriverFailure>,
    queued_write: bool,
}

/// Frozen after re-arm: no queue calls, scope polling or success publication may
/// originate from this generation. Its exact owners stay with the session.
struct RetainedGeneration<D: CompletionDriver> {
    context: GpuContextId,
    _driver: D,
    frames: Vec<FrameRecord<D>>,
}

struct BrokerCore<D: CompletionDriver> {
    driver: D,
    context: GpuContextId,
    session: Arc<()>,
    state: GpuOwnerState,
    budget: NativeGpuBudget,
    next_transaction: u64,
    next_submission: u64,
    frames: Vec<FrameRecord<D>>,
    retained_generations: Vec<RetainedGeneration<D>>,
}

impl<D: CompletionDriver> BrokerCore<D> {
    fn new(driver: D, context: GpuContextId) -> Self {
        Self {
            driver,
            context,
            session: Arc::new(()),
            state: GpuOwnerState::Active,
            budget: NativeGpuBudget::new(context),
            next_transaction: 1,
            next_submission: 1,
            frames: Vec::new(),
            retained_generations: Vec::new(),
        }
    }

    fn is_pristine(&self) -> bool {
        self.state == GpuOwnerState::Active
            && self.next_transaction == 1
            && self.next_submission == 1
            && self.frames.is_empty()
            && self.retained_generations.is_empty()
            && self.budget.is_pristine()
    }

    fn can_release(&self) -> bool {
        let budget = self.budget.snapshot();
        self.frames.is_empty()
            && self.retained_generations.is_empty()
            && budget.allocation_count == 0
            && budget.reservation_count == 0
    }

    /// Adopt only the device/driver from a fresh factory bootstrap. The empty
    /// bootstrap ledger is discarded; the original session ledger never moves
    /// or resets. Every fallible check precedes the ownership transfer.
    fn rearm_with_replacement(&mut self, replacement: Self) -> Result<(), (NativeGpuFault, Self)> {
        let valid = (|| {
            if !replacement.is_pristine() {
                return Err(self.fault(
                    None,
                    GpuPhase::Retirement,
                    NativeGpuFaultKind::StateMismatch,
                    "replacement device already owns work, quotes or capacity",
                ));
            }
            match self.state {
                GpuOwnerState::Active => {
                    return Err(self.fault(
                        None,
                        GpuPhase::Retirement,
                        NativeGpuFaultKind::StateMismatch,
                        "stop or retire the old generation before re-arm",
                    ));
                }
                GpuOwnerState::Retiring | GpuOwnerState::Reaped if !self.frames.is_empty() => {
                    return Err(self.fault(
                        None,
                        GpuPhase::Retirement,
                        NativeGpuFaultKind::StateMismatch,
                        "healthy old submissions and scopes must drain before re-arm",
                    ));
                }
                _ => {}
            }
            let retain = !self.frames.is_empty() || self.budget.retains_generation(self.context);
            if retain {
                if self.retained_generations.len() >= MAX_RETAINED_DEVICE_GENERATIONS {
                    return Err(self.fault(
                        None,
                        GpuPhase::Retirement,
                        NativeGpuFaultKind::StateMismatch,
                        "retained device generation limit reached; session owners remain held",
                    ));
                }
                self.retained_generations.try_reserve(1).map_err(|error| {
                    self.fault(
                        None,
                        GpuPhase::Retirement,
                        NativeGpuFaultKind::OutOfMemory,
                        format!("cannot retain old device generation: {error}"),
                    )
                })?;
            }
            self.budget.rearm_generation(replacement.context)?;
            Ok(retain)
        })();
        let retain = match valid {
            Ok(retain) => retain,
            Err(fault) => return Err((fault, replacement)),
        };
        let previous = std::mem::replace(&mut self.driver, replacement.driver);
        let frames = std::mem::take(&mut self.frames);
        for frame in &frames {
            frame.marker.store(CANCELLED_TOKEN, Ordering::Release);
        }
        if retain {
            self.retained_generations.push(RetainedGeneration {
                context: self.context,
                _driver: previous,
                frames,
            });
        }
        self.context = replacement.context;
        self.state = GpuOwnerState::Active;
        self.next_transaction = 1;
        self.next_submission = 1;
        Ok(())
    }

    fn fault(
        &self,
        frame: Option<NativeFrameId>,
        phase: GpuPhase,
        kind: NativeGpuFaultKind,
        message: impl Into<String>,
    ) -> NativeGpuFault {
        NativeGpuFault {
            context: self.context,
            frame,
            phase,
            kind,
            message: message.into(),
        }
    }

    fn active(&self) -> Result<(), NativeGpuFault> {
        if self.state != GpuOwnerState::Active {
            return Err(self.fault(
                None,
                GpuPhase::Retirement,
                NativeGpuFaultKind::StateMismatch,
                "native GPU owner is not active",
            ));
        }
        Ok(())
    }

    fn begin(
        &mut self,
        frame: NativeFrameId,
        reservation: FrameReservation,
    ) -> Result<RetirementId, NativeGpuFault> {
        self.admit(frame, reservation, false)
    }

    /// Present or cold-initialization work beside at most one in-flight
    /// logical output (ADR-01 section 7.2 snapshot ring). Only one auxiliary
    /// record may be unretired at a time, so the registry stays bounded at one
    /// logical transaction, one auxiliary transaction and one retiring present.
    fn begin_auxiliary(
        &mut self,
        frame: NativeFrameId,
        reservation: FrameReservation,
    ) -> Result<RetirementId, NativeGpuFault> {
        self.admit(frame, reservation, true)
    }

    fn admit(
        &mut self,
        frame: NativeFrameId,
        reservation: FrameReservation,
        auxiliary: bool,
    ) -> Result<RetirementId, NativeGpuFault> {
        let admission = self.active().and_then(|()| {
            let occupied = if auxiliary {
                // Queue order keeps the logical output ahead of this submission
                // only once that output has finished recording.
                self.frames.iter().any(|record| {
                    if record.auxiliary {
                        record.stage != FrameRecordStage::PresentRetirement
                    } else {
                        record.stage == FrameRecordStage::Recording
                    }
                })
            } else {
                self.has_logical_frame()
            };
            if occupied {
                return Err(self.fault(
                    Some(frame),
                    GpuPhase::Initialize,
                    NativeGpuFaultKind::StateMismatch,
                    if auxiliary {
                        "one auxiliary present/initialize transaction may be in flight"
                    } else {
                        "one logical output may be in flight"
                    },
                ));
            }
            self.budget.validate_frame_reservation(&reservation, frame)
        });
        if let Err(error) = admission {
            if reservation.context() == self.context {
                self.budget.cancel_reservation_tree(reservation)?;
            }
            return Err(error);
        }
        let Some(next) = self.next_transaction.checked_add(1) else {
            self.budget.cancel_reservation_tree(reservation)?;
            return Err(self.fault(
                Some(frame),
                GpuPhase::Initialize,
                NativeGpuFaultKind::SequenceExhausted,
                "transaction sequence exhausted",
            ));
        };
        let id = RetirementId {
            context: self.context,
            sequence: self.next_transaction,
        };
        self.next_transaction = next;
        self.frames.push(FrameRecord {
            id,
            frame,
            stage: FrameRecordStage::Recording,
            auxiliary,
            marker: Arc::new(TransactionMarker {
                session: Arc::clone(&self.session),
                id,
                state: AtomicU8::new(ACTIVE_TOKEN),
            }),
            reservation: Some(reservation),
            resources: Vec::new(),
            allocations: Vec::new(),
            owned_allocations: Vec::new(),
            phases: Vec::new(),
            open: None,
            open_failure: None,
            queued_write: false,
        });
        Ok(id)
    }

    fn has_logical_frame(&self) -> bool {
        self.frames
            .iter()
            .any(|frame| frame.stage != FrameRecordStage::PresentRetirement)
    }

    fn seal(&mut self, index: usize) {
        let frame = &mut self.frames[index];
        // Do not detach ISF, snapshot, replay, initialization or mixed work.
        // A submitted phase alone is insufficient: the transaction must be
        // finished, its scope stack closed, and no cancellation/fault known.
        // Scope completion and GPU completion still belong to this record.
        frame.stage = if self.state == GpuOwnerState::Active
            && frame.marker.load(Ordering::Acquire) == ACTIVE_TOKEN
            && frame.open.is_none()
            && !frame.queued_write
            && frame.open_failure.is_none()
            && frame.phases.len() == 1
            && frame.phases[0].phase == GpuPhase::MappingPresent
            && frame.phases[0].execution_failure.is_none()
            && !matches!(frame.phases[0].scope_result, Some(Err(_)))
        {
            FrameRecordStage::PresentRetirement
        } else {
            FrameRecordStage::Sealed
        };
    }

    fn index(&self, id: RetirementId) -> Result<usize, NativeGpuFault> {
        self.frames
            .iter()
            .position(|record| record.id == id)
            .ok_or_else(|| {
                self.fault(
                    None,
                    GpuPhase::Retirement,
                    NativeGpuFaultKind::IdentityMismatch,
                    "unknown or foreign transaction",
                )
            })
    }

    fn open(&mut self, id: RetirementId, phase: GpuPhase) -> Result<(), NativeGpuFault> {
        self.active()?;
        let index = self.index(id)?;
        if self.frames[index].stage != FrameRecordStage::Recording
            || self.frames[index].marker.load(Ordering::Acquire) != ACTIVE_TOKEN
            || self.frames[index].open.is_some()
        {
            return Err(self.fault(
                Some(self.frames[index].frame),
                phase,
                NativeGpuFaultKind::StateMismatch,
                "sealed or cancelled transaction, or unsealed phase",
            ));
        }
        if self.frames[index].auxiliary
            && !matches!(phase, GpuPhase::MappingPresent | GpuPhase::Initialize)
        {
            return Err(self.fault(
                Some(self.frames[index].frame),
                phase,
                NativeGpuFaultKind::StateMismatch,
                "an auxiliary transaction carries only present or initialization work",
            ));
        }
        // At most one present retirement plus one logical output may be live.
        // In particular, an alias-only present cannot grow an uncharged queue
        // or overwrite pooled mapping resources before their previous use.
        // Check before the caller pushes scopes, allocates or issues writes.
        if phase == GpuPhase::MappingPresent
            && self
                .frames
                .iter()
                .any(|frame| frame.stage == FrameRecordStage::PresentRetirement)
        {
            return Err(self.fault(
                Some(self.frames[index].frame),
                phase,
                NativeGpuFaultKind::StateMismatch,
                "previous present must retire before another mapping phase",
            ));
        }
        self.frames[index].open = Some(phase);
        Ok(())
    }

    fn submission_id(
        &mut self,
        frame: NativeFrameId,
        phase: GpuPhase,
    ) -> Result<GpuSubmissionId, NativeGpuFault> {
        let next = self.next_submission.checked_add(1).ok_or_else(|| {
            self.fault(
                Some(frame),
                phase,
                NativeGpuFaultKind::SequenceExhausted,
                "submission sequence exhausted",
            )
        })?;
        let id = GpuSubmissionId {
            context: self.context,
            sequence: self.next_submission,
        };
        self.next_submission = next;
        Ok(id)
    }

    fn register(
        &mut self,
        id: RetirementId,
        submission: GpuSubmissionId,
        phase: GpuPhase,
        ticket: D::Ticket,
        scopes: D::Scopes,
    ) -> Result<PendingPhaseReceipt, NativeGpuFault> {
        let index = self.index(id)?;
        if self.frames[index].open != Some(phase) || submission.context != self.context {
            return Err(self.fault(
                Some(self.frames[index].frame),
                phase,
                NativeGpuFaultKind::IdentityMismatch,
                "phase submit identity mismatch",
            ));
        }
        let frame = self.frames[index].frame;
        let failure = self.frames[index].open_failure.take();
        self.frames[index].phases.push(PhaseRecord {
            id: submission,
            phase,
            ticket,
            scopes,
            execution_failure: failure,
            scope_result: None,
            complete: false,
        });
        self.frames[index].open = None;
        self.frames[index].queued_write = false;
        for allocation in &self.frames[index].allocations {
            if let Err(error) = self.budget.record_use(*allocation, submission) {
                self.state = GpuOwnerState::PoisonedRetained;
                self.budget.quarantine();
                return Err(error);
            }
        }
        Ok(PendingPhaseReceipt {
            transaction: id,
            frame,
            phase,
            submission,
        })
    }

    fn progress(&mut self) -> Result<(), NativeGpuFault> {
        if matches!(
            self.state,
            GpuOwnerState::PoisonedRetained | GpuOwnerState::DeviceLostRetained
        ) {
            return Err(self.fault(
                None,
                GpuPhase::Retirement,
                NativeGpuFaultKind::ScopeDrain,
                "shared generation remains quarantined",
            ));
        }
        if let Err(error) = self.driver.poll() {
            self.state = if error.kind == NativeGpuFaultKind::DeviceLost {
                GpuOwnerState::DeviceLostRetained
            } else {
                GpuOwnerState::PoisonedRetained
            };
            self.budget.quarantine();
            for frame in &self.frames {
                for allocation in &frame.allocations {
                    self.budget.mark_retired(*allocation)?;
                }
            }
            return Err(self.fault(None, GpuPhase::Retirement, error.kind, error.message));
        }
        let mut fatal_phase = None;
        for frame in &mut self.frames {
            for phase in &mut frame.phases {
                if phase.scope_result.is_none() {
                    if let Some(result) = self.driver.scopes(&mut phase.scopes) {
                        phase.scope_result = Some(match result {
                            Err(error)
                                if matches!(
                                    error.kind,
                                    NativeGpuFaultKind::DeviceLost | NativeGpuFaultKind::ScopeDrain
                                ) =>
                            {
                                Err(error)
                            }
                            ordinary => match phase.execution_failure.take() {
                                Some(failure) => Err(failure),
                                None => ordinary,
                            },
                        });
                    }
                }
                if !phase.complete && self.driver.complete(&phase.ticket) {
                    self.budget.acknowledge_completed(phase.id)?;
                    phase.complete = true;
                }
                if let Some(Err(error)) = &phase.scope_result {
                    if matches!(
                        error.kind,
                        NativeGpuFaultKind::DeviceLost | NativeGpuFaultKind::ScopeDrain
                    ) {
                        fatal_phase.get_or_insert((
                            frame.frame,
                            phase.phase,
                            error.kind,
                            error.message.clone(),
                        ));
                    }
                }
            }
        }
        if let Some((frame, phase, kind, message)) = fatal_phase {
            self.state = if kind == NativeGpuFaultKind::DeviceLost {
                GpuOwnerState::DeviceLostRetained
            } else {
                GpuOwnerState::PoisonedRetained
            };
            self.budget.quarantine();
            return Err(self.fault(Some(frame), phase, kind, message));
        }
        self.reap()
    }

    fn reap(&mut self) -> Result<(), NativeGpuFault> {
        let ids: Vec<_> = self.budget.live_allocation_ids().collect();
        for id in ids {
            if let Some(receipt) = self.budget.try_reap(id)? {
                self.budget.release_reaped(receipt)?;
            }
        }
        self.retained_generations.retain(|generation| {
            !generation.frames.is_empty() || self.budget.retains_generation(generation.context)
        });
        Ok(())
    }

    fn ready(&self, index: usize) -> bool {
        let frame = &self.frames[index];
        frame.open.is_none()
            && frame
                .phases
                .iter()
                .all(|phase| phase.complete && phase.scope_result.is_some())
    }

    fn finish_record(&mut self, index: usize) -> Result<ResolvedGpuTransaction, NativeGpuFault> {
        let mut frame = self.frames.remove(index);
        if let Some(reservation) = frame.reservation.take() {
            self.budget.cancel_reservation_tree(reservation)?;
        }
        let results = frame
            .phases
            .into_iter()
            .map(|phase| {
                let result = phase.scope_result.expect("resolved phase");
                ResolvedGpuPhase {
                    transaction: frame.id,
                    frame: frame.frame,
                    phase: phase.phase,
                    submission: phase.id,
                    result: result.map_err(|error| NativeGpuFault {
                        context: self.context,
                        frame: Some(frame.frame),
                        phase: phase.phase,
                        kind: error.kind,
                        message: error.message,
                    }),
                }
            })
            .collect();
        // These strong owners are released only after all dependent submissions
        // completed, including phases which read an earlier candidate.
        drop(frame.resources);
        self.reap()?;
        frame.marker.store(TERMINAL_TOKEN, Ordering::Release);
        Ok(ResolvedGpuTransaction {
            id: frame.id,
            phases: results,
        })
    }

    fn retire(&mut self, id: RetirementId) -> Result<(), NativeGpuFault> {
        if let Some(frame) = self
            .retained_generations
            .iter()
            .flat_map(|generation| &generation.frames)
            .find(|frame| frame.id == id)
        {
            // Re-arm already froze and quarantined this exact record. A late
            // cancellation transfers no charge, polls no old driver and cannot
            // expire a current-generation quote by changing the ledger revision.
            frame.marker.store(CANCELLED_TOKEN, Ordering::Release);
            return Ok(());
        }
        let index = self.index(id)?;
        self.frames[index]
            .marker
            .store(CANCELLED_TOKEN, Ordering::Release);
        for allocation in &self.frames[index].owned_allocations {
            self.budget.mark_retired(*allocation)?;
        }
        Ok(())
    }

    fn retire_pending(&mut self, pending: &PendingGpuTransaction) -> Result<(), NativeGpuFault> {
        let own_marker =
            Arc::ptr_eq(&self.session, &pending.marker.session) && pending.id == pending.marker.id;
        if own_marker {
            let registered = self
                .frames
                .iter()
                .chain(
                    self.retained_generations
                        .iter()
                        .flat_map(|generation| &generation.frames),
                )
                .any(|frame| frame.id == pending.id && Arc::ptr_eq(&frame.marker, &pending.marker));
            if registered {
                return self.retire(pending.id);
            }
            // Healthy Stop may have drained this record while its affine token
            // was still outside the owner. The authentic marker proves only
            // that record's finished cleanup, never this generation's Reaped.
            if pending.marker.load(Ordering::Acquire) == TERMINAL_TOKEN {
                return Ok(());
            }
        }
        Err(self.fault(
            None,
            GpuPhase::Retirement,
            NativeGpuFaultKind::IdentityMismatch,
            "pending token has no matching session retirement record",
        ))
    }

    fn stop(&mut self) -> StopRetirement {
        if self.state == GpuOwnerState::Active {
            self.state = GpuOwnerState::Retiring;
        }
        for frame in &self.frames {
            frame.marker.store(CANCELLED_TOKEN, Ordering::Release);
            for id in &frame.owned_allocations {
                if self.budget.mark_retired(*id).is_err() {
                    self.state = GpuOwnerState::PoisonedRetained;
                    self.budget.quarantine();
                }
            }
        }
        StopRetirement {
            context: self.context,
        }
    }

    fn retirement(&mut self) -> Result<RetirementProgress, NativeGpuFault> {
        self.progress()?;
        let mut index = 0;
        while index < self.frames.len() {
            if self.frames[index].marker.load(Ordering::Acquire) == CANCELLED_TOKEN
                && self.ready(index)
            {
                drop(self.finish_record(index)?);
            } else {
                index += 1;
            }
        }
        self.reap()?;
        let live = self.budget.live_allocation_ids().count();
        let snapshot = self.budget.snapshot();
        if self.frames.is_empty()
            && self.retained_generations.is_empty()
            && live == 0
            && snapshot.reservation_count == 0
            && self.state == GpuOwnerState::Retiring
        {
            self.state = GpuOwnerState::Reaped;
            Ok(RetirementProgress::Reaped)
        } else {
            Ok(RetirementProgress::Pending {
                transactions: self.frames.len()
                    + self
                        .retained_generations
                        .iter()
                        .map(|generation| generation.frames.len())
                        .sum::<usize>(),
                allocations: live,
            })
        }
    }
}

/// An accidental owner drop is not completion evidence. Normal lifecycle code
/// keeps this owner and polls retirement; the last-resort abandonment path keeps
/// the complete driver, ledger and registry until process teardown, without a
/// blocking destructor, detached cleanup or an invented successful Stop ACK.
struct OwnedBrokerCore<D: CompletionDriver>(Option<BrokerCore<D>>);

impl<D: CompletionDriver> OwnedBrokerCore<D> {
    fn new(core: BrokerCore<D>) -> Self {
        Self(Some(core))
    }
}

impl<D: CompletionDriver> Deref for OwnedBrokerCore<D> {
    type Target = BrokerCore<D>;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("native GPU core is owned")
    }
}

impl<D: CompletionDriver> DerefMut for OwnedBrokerCore<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().expect("native GPU core is owned")
    }
}

impl<D: CompletionDriver> Drop for OwnedBrokerCore<D> {
    fn drop(&mut self) {
        if let Some(mut core) = self.0.take() {
            if !core.can_release() {
                core.stop();
                core.budget.quarantine();
                std::mem::forget(core);
            }
        }
    }
}

pub(crate) struct NativeGpuOwner {
    context: Arc<NativeGpuContext>,
    core: OwnedBrokerCore<RealGpuDriver>,
}

pub(crate) struct NativeGpuFrameTxn<'a> {
    owner: &'a mut NativeGpuOwner,
    id: RetirementId,
    finished: bool,
}

pub(crate) struct GpuPhaseEncoder<'a> {
    owner: &'a mut NativeGpuOwner,
    transaction: RetirementId,
    phase: GpuPhase,
    finished: bool,
}

pub(crate) struct EncodedGpuPhase {
    transaction: RetirementId,
    phase: GpuPhase,
    commands: Vec<wgpu::CommandBuffer>,
}

#[derive(Debug)]
pub(crate) struct PendingPhaseReceipt {
    transaction: RetirementId,
    frame: NativeFrameId,
    phase: GpuPhase,
    submission: GpuSubmissionId,
}

impl PendingPhaseReceipt {
    pub(crate) fn submission(&self) -> GpuSubmissionId {
        self.submission
    }
    #[cfg(test)]
    pub(crate) fn frame(&self) -> NativeFrameId {
        self.frame
    }
    #[cfg(test)]
    pub(crate) fn phase(&self) -> GpuPhase {
        self.phase
    }
}

pub(crate) struct PendingGpuTransaction {
    id: RetirementId,
    marker: Arc<TransactionMarker>,
    terminal: bool,
}

impl Drop for PendingGpuTransaction {
    fn drop(&mut self) {
        if !self.terminal {
            let _ = self.marker.compare_exchange(
                ACTIVE_TOKEN,
                CANCELLED_TOKEN,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
    }
}

pub(crate) struct RetiringGpuTransaction {
    id: RetirementId,
}

impl RetiringGpuTransaction {
    pub(crate) fn id(&self) -> RetirementId {
        self.id
    }
}

pub(crate) struct StopRetirement {
    context: GpuContextId,
}
impl StopRetirement {
    pub(crate) fn context(&self) -> GpuContextId {
        self.context
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RetirementProgress {
    Pending {
        transactions: usize,
        allocations: usize,
    },
    Reaped,
}

pub(crate) struct ResolvedGpuTransaction {
    id: RetirementId,
    phases: Vec<ResolvedGpuPhase>,
}

impl ResolvedGpuTransaction {
    pub(crate) fn take_phase(
        &mut self,
        receipt: PendingPhaseReceipt,
    ) -> Result<ResolvedGpuPhase, NativeGpuFault> {
        if self.id != receipt.transaction {
            return Err(NativeGpuFault {
                context: self.id.context,
                frame: Some(receipt.frame),
                phase: receipt.phase,
                kind: NativeGpuFaultKind::IdentityMismatch,
                message: "phase receipt is from another transaction".into(),
            });
        }
        let index = self
            .phases
            .iter()
            .position(|phase| {
                phase.submission == receipt.submission
                    && phase.phase == receipt.phase
                    && phase.frame == receipt.frame
            })
            .ok_or_else(|| NativeGpuFault {
                context: self.id.context,
                frame: Some(receipt.frame),
                phase: receipt.phase,
                kind: NativeGpuFaultKind::IdentityMismatch,
                message: "phase receipt already consumed or mismatched".into(),
            })?;
        Ok(self.phases.remove(index))
    }
}

pub(crate) struct ResolvedGpuPhase {
    transaction: RetirementId,
    frame: NativeFrameId,
    phase: GpuPhase,
    submission: GpuSubmissionId,
    result: Result<(), NativeGpuFault>,
}

impl ResolvedGpuPhase {
    pub(crate) fn context(&self) -> GpuContextId {
        self.submission.context
    }
    pub(crate) fn frame(&self) -> NativeFrameId {
        self.frame
    }
    pub(crate) fn phase(&self) -> GpuPhase {
        self.phase
    }
    pub(crate) fn submission(&self) -> GpuSubmissionId {
        self.submission
    }
    pub(crate) fn transaction(&self) -> RetirementId {
        self.transaction
    }
    pub(crate) fn validation(&self) -> Result<(), &NativeGpuFault> {
        self.result.as_ref().map(|_| ())
    }
    pub(crate) fn into_validation(self) -> Result<(), NativeGpuFault> {
        self.result
    }
}

impl NativeGpuOwner {
    /// Keep this output session's ledger and retained generations while adopting
    /// a newly-created, unused device from the same factory and output owner.
    /// I must call this before constructing replacement runtimes/presenters.
    /// Rejection returns the complete replacement owner; no capacity is refunded
    /// and no old result/history/clock is rerun or declared recovered here.
    pub(crate) fn rearm_with_replacement(
        &mut self,
        mut replacement: Self,
    ) -> Result<(), (NativeGpuFault, Self)> {
        if self.context.lost.load(Ordering::Acquire)
            || self.context.uncaptured.load(Ordering::Acquire)
        {
            // Record real callback evidence even if no completion poll ran.
            let _ = self.check_device();
        }
        let foreign = !Arc::ptr_eq(self.context.instance(), replacement.context.instance())
            || self.context.id().owner_id != replacement.context.id().owner_id;
        let exposed = Arc::strong_count(&replacement.context) != 2;
        let unhealthy = replacement.context.lost.load(Ordering::Acquire)
            || replacement.context.uncaptured.load(Ordering::Acquire);
        if foreign || exposed || unhealthy {
            let fault = self.core.fault(
                None,
                GpuPhase::Retirement,
                if foreign {
                    NativeGpuFaultKind::IdentityMismatch
                } else {
                    NativeGpuFaultKind::StateMismatch
                },
                "replacement must be healthy, private and from this output's factory",
            );
            return Err((fault, replacement));
        }
        let next = replacement
            .core
            .0
            .take()
            .expect("replacement core is owned");
        if let Err((fault, next)) = self.core.rearm_with_replacement(next) {
            replacement.core.0 = Some(next);
            return Err((fault, replacement));
        }
        self.context = Arc::clone(&replacement.context);
        Ok(())
    }

    pub(crate) fn context(&self) -> &Arc<NativeGpuContext> {
        &self.context
    }
    pub(crate) fn capabilities(&self) -> &NativeGpuCapabilities {
        self.context.capabilities()
    }
    pub(crate) fn state(&self) -> GpuOwnerState {
        if self.context.lost.load(Ordering::Acquire) {
            GpuOwnerState::DeviceLostRetained
        } else {
            self.core.state
        }
    }
    pub(crate) fn quote(
        &self,
        plan: &NativeAllocationPlan,
    ) -> Result<BudgetQuote, NativeGpuDecline> {
        self.core.budget.quote(plan)
    }
    pub(crate) fn reserve(
        &mut self,
        quote: BudgetQuote,
    ) -> Result<FrameReservation, NativeGpuDecline> {
        self.core.budget.reserve(quote)
    }
    #[cfg(test)]
    pub(crate) fn budget_snapshot(&self) -> NativeGpuBudgetSnapshot {
        self.core.budget.snapshot()
    }

    /// Validate and return a wholly unused serial CPU admission envelope.
    /// The caller keeps exclusive `&mut NativeGpuOwner` until all exact chunk
    /// transactions finish; no resource, clock, upload or FX has run yet.
    pub(crate) fn admit_serial_cpu_plan(
        &mut self,
        reservation: FrameReservation,
        plan: &NativeAllocationPlan,
    ) -> Result<(), (NativeGpuFault, FrameReservation)> {
        let valid = self
            .core
            .budget
            .validate_unused_exact_plan(&reservation, plan)
            .and_then(|()| self.check_device())
            .and_then(|()| {
                if self.core.has_logical_frame() {
                    return Err(self.core.fault(
                        Some(plan.frame()),
                        GpuPhase::IsfBatch,
                        NativeGpuFaultKind::StateMismatch,
                        "serial CPU admission requires a free logical output slot",
                    ));
                }
                Ok(())
            });
        // Release only a matching own root. On any failure return the token;
        // foreign/bound/child capacity is neither refunded nor orphaned.
        let valid = valid.and_then(|()| self.core.budget.refund_unused_root(&reservation));
        valid.map_err(|fault| (fault, reservation))
    }

    pub(crate) fn discard_unsubmitted_reservation(
        &mut self,
        reservation: FrameReservation,
    ) -> Result<(), (NativeGpuFault, FrameReservation)> {
        if self.core.has_logical_frame() {
            return Err((
                self.core.fault(
                    Some(reservation.frame()),
                    GpuPhase::IsfBatch,
                    NativeGpuFaultKind::StateMismatch,
                    "cannot discard a reservation while a logical frame is pending",
                ),
                reservation,
            ));
        }
        self.core
            .budget
            .refund_unused_root(&reservation)
            .map_err(|fault| (fault, reservation))
    }

    /// Return a wholly unused present/initialization root before its
    /// auxiliary admission. Unlike the logical refund this is permitted while a
    /// logical output is in flight: an unbound root shares no slot with it.
    pub(crate) fn discard_unsubmitted_auxiliary_reservation(
        &mut self,
        reservation: FrameReservation,
    ) -> Result<(), (NativeGpuFault, FrameReservation)> {
        self.core
            .budget
            .refund_unused_root(&reservation)
            .map_err(|fault| (fault, reservation))
    }

    pub(crate) fn begin_frame(
        &mut self,
        frame: NativeFrameId,
        reservation: FrameReservation,
    ) -> Result<NativeGpuFrameTxn<'_>, NativeGpuFault> {
        if let Err(error) = self.check_device() {
            if reservation.context() == self.core.context {
                self.core.budget.cancel_reservation_tree(reservation)?;
            }
            return Err(error);
        }
        let id = self.core.begin(frame, reservation)?;
        Ok(NativeGpuFrameTxn {
            owner: self,
            id,
            finished: false,
        })
    }

    /// Admit one MappingPresent or Initialize transaction while one logical
    /// output may still be executing. The physical present of output N then
    /// runs while output N+1 owns the logical slot; queue order keeps N+1's
    /// work ahead of this submission, so completion stays ordered.
    pub(crate) fn begin_auxiliary_frame(
        &mut self,
        frame: NativeFrameId,
        reservation: FrameReservation,
    ) -> Result<NativeGpuFrameTxn<'_>, NativeGpuFault> {
        if let Err(error) = self.check_device() {
            if reservation.context() == self.core.context {
                self.core.budget.cancel_reservation_tree(reservation)?;
            }
            return Err(error);
        }
        let id = self.core.begin_auxiliary(frame, reservation)?;
        Ok(NativeGpuFrameTxn {
            owner: self,
            id,
            finished: false,
        })
    }

    fn check_device(&mut self) -> Result<(), NativeGpuFault> {
        if self.context.lost.load(Ordering::Acquire) {
            self.core.state = GpuOwnerState::DeviceLostRetained;
            self.core.budget.quarantine();
            return Err(self.core.fault(
                None,
                GpuPhase::Retirement,
                NativeGpuFaultKind::DeviceLost,
                "shared GPU generation is lost",
            ));
        }
        if self.context.uncaptured.load(Ordering::Acquire) {
            self.core.state = GpuOwnerState::PoisonedRetained;
            self.core.budget.quarantine();
            return Err(self.core.fault(
                None,
                GpuPhase::Retirement,
                NativeGpuFaultKind::ScopeDrain,
                self.context
                    .uncaptured_message("shared GPU generation has an uncaptured error"),
            ));
        }
        self.core.active()
    }

    fn bind(
        &mut self,
        transaction: RetirementId,
        kind: ChargeKind,
        capacity: u64,
    ) -> Result<AllocationCharge, NativeGpuFault> {
        let core = &mut *self.core;
        let index = core.index(transaction)?;
        let id = core.budget.next_allocation_id()?;
        let reservation = core.frames[index]
            .reservation
            .as_mut()
            .expect("active reservation");
        let charge = core
            .budget
            .bind_allocation(reservation, id, kind, capacity)?;
        // Bind precedes creation. Even a creation error keeps the bound charge
        // alive until the phase scopes and issued work are safely completed.
        core.frames[index].resources.push(Arc::new(charge.pin()));
        core.frames[index].allocations.push(id);
        core.frames[index].owned_allocations.push(id);
        Ok(charge)
    }

    fn retain<T: Any + Send + Sync>(
        &mut self,
        transaction: RetirementId,
        resource: Arc<T>,
        allocations: &[GpuAllocationId],
    ) -> Result<(), NativeGpuFault> {
        let index = self.core.index(transaction)?;
        // Validate the complete list before publishing any alias into the
        // transaction. Old CPU records use the same bound-ledger proof as the
        // quote and reserved-plan checks; old GPU handles remain forbidden.
        for id in allocations {
            if !self.core.budget.can_retain_allocation(*id) {
                return Err(self.core.fault(
                    Some(self.core.frames[index].frame),
                    GpuPhase::Initialize,
                    NativeGpuFaultKind::IdentityMismatch,
                    "resource has no live compatible allocation in this session",
                ));
            }
        }
        for id in allocations {
            if !self.core.frames[index].allocations.contains(id) {
                self.core.frames[index].allocations.push(*id);
            }
        }
        self.core.frames[index].resources.push(resource);
        Ok(())
    }

    fn bind_reserved(
        &mut self,
        transaction: RetirementId,
        reservation: &mut FrameReservation,
        kind: ChargeKind,
        capacity: u64,
    ) -> Result<AllocationCharge, NativeGpuFault> {
        let index = self.core.index(transaction)?;
        if reservation.frame() != self.core.frames[index].frame
            || reservation.context() != self.core.context
        {
            return Err(self.core.fault(
                Some(self.core.frames[index].frame),
                GpuPhase::CompositionSnapshot,
                NativeGpuFaultKind::IdentityMismatch,
                "foreign sub-reservation",
            ));
        }
        self.core.budget.validate_subreservation(
            self.core.frames[index]
                .reservation
                .as_ref()
                .expect("active reservation"),
            reservation,
        )?;
        let id = self.core.budget.next_allocation_id()?;
        let charge = self
            .core
            .budget
            .bind_allocation(reservation, id, kind, capacity)?;
        self.core.frames[index]
            .resources
            .push(Arc::new(charge.pin()));
        self.core.frames[index].allocations.push(id);
        self.core.frames[index].owned_allocations.push(id);
        Ok(charge)
    }

    fn submit(&mut self, encoded: EncodedGpuPhase) -> Result<PendingPhaseReceipt, NativeGpuFault> {
        self.check_device()?;
        let index = self.core.index(encoded.transaction)?;
        if self.core.frames[index].open != Some(encoded.phase) {
            return Err(self.core.fault(
                Some(self.core.frames[index].frame),
                encoded.phase,
                NativeGpuFaultKind::StateMismatch,
                "phase is not the current open scope",
            ));
        }
        let frame = self.core.frames[index].frame;
        let submission = self.core.submission_id(frame, encoded.phase)?;
        self.context.queue.submit(encoded.commands);
        let completed = Arc::new(AtomicBool::new(false));
        let callback = Arc::clone(&completed);
        self.context
            .queue
            .on_submitted_work_done(move || callback.store(true, Ordering::Release));
        let scopes = ScopeSet::seal(&self.context.device);
        self.core.register(
            encoded.transaction,
            submission,
            encoded.phase,
            RealSubmission { completed },
            scopes,
        )
    }

    // Drop performs no wait. Any issued writes remain owned by this registry;
    // an empty submit makes them drainable under their original phase scopes.
    fn abandon_phase(&mut self, id: RetirementId) {
        if let Ok(index) = self.core.index(id) {
            if let Some(phase) = self.core.frames[index].open {
                if self
                    .submit(EncodedGpuPhase {
                        transaction: id,
                        phase,
                        commands: Vec::new(),
                    })
                    .is_err()
                {
                    self.core.state = GpuOwnerState::PoisonedRetained;
                    self.core.budget.quarantine();
                }
            }
            if self.core.retire(id).is_err() {
                self.core.state = GpuOwnerState::PoisonedRetained;
                self.core.budget.quarantine();
            }
        }
    }

    pub(crate) fn poll_transaction(
        &mut self,
        pending: &mut PendingGpuTransaction,
        cancel: &dyn VideoRenderCancellation,
    ) -> CompletionProgress<ResolvedGpuTransaction, NativeGpuFault> {
        if pending.terminal {
            return CompletionProgress::Complete(Err(self.core.fault(
                None,
                GpuPhase::Retirement,
                NativeGpuFaultKind::StateMismatch,
                "terminal transaction repolled",
            )));
        }
        let index = match self.core.index(pending.id) {
            Ok(index) => index,
            Err(error) => {
                pending.terminal = true;
                return CompletionProgress::Complete(Err(error));
            }
        };
        if cancel.is_cancelled() || pending.marker.load(Ordering::Acquire) != ACTIVE_TOKEN {
            pending.terminal = true;
            pending.marker.store(CANCELLED_TOKEN, Ordering::Release);
            return CompletionProgress::Cancelled;
        }
        if let Err(error) = self.core.progress() {
            pending.terminal = true;
            return CompletionProgress::Complete(Err(error));
        }
        if cancel.is_cancelled() || pending.marker.load(Ordering::Acquire) != ACTIVE_TOKEN {
            pending.terminal = true;
            pending.marker.store(CANCELLED_TOKEN, Ordering::Release);
            return CompletionProgress::Cancelled;
        }
        if !self.core.ready(index) {
            return CompletionProgress::Pending;
        }
        pending.terminal = true;
        CompletionProgress::Complete(self.core.finish_record(index))
    }

    pub(crate) fn cancel_and_retire(
        &mut self,
        mut pending: PendingGpuTransaction,
    ) -> RetiringGpuTransaction {
        if self.core.retire_pending(&pending).is_err() {
            self.core.state = GpuOwnerState::PoisonedRetained;
            self.core.budget.quarantine();
        }
        pending.terminal = true;
        RetiringGpuTransaction { id: pending.id }
    }

    pub(crate) fn begin_stop(&mut self) -> StopRetirement {
        self.core.stop()
    }

    /// Test-only device-loss injection: the next broker poll observes loss
    /// exactly as the real device-lost callback would report it.
    #[cfg(test)]
    pub(crate) fn mark_device_lost_for_tests(&self) {
        self.context.lost.store(true, Ordering::Release);
    }
    /// A revoked preparation may resume on the next tick only when no logical
    /// frame remains. A prior physical-present retirement is independently
    /// owned by P and does not block the next logical output.
    pub(crate) fn has_unfinished_logical_frame(&self) -> bool {
        self.core.has_logical_frame()
    }
    pub(crate) fn poll_retirement(&mut self) -> Result<RetirementProgress, NativeGpuFault> {
        self.core.retirement()
    }

    /// Constructor-only cold path for pipeline/layout/sampler driver resources.
    /// Managed textures, buffers and queue writes require a planned frame phase;
    /// they must never be created through this zero-byte initialization closure.
    /// A timeout keeps the resource and transaction
    /// in this owner; no detached cleanup and no success is manufactured.
    pub(crate) fn initialize<T: Any + Send + Sync>(
        &mut self,
        create: impl FnOnce(&wgpu::Device, &wgpu::Queue) -> T,
    ) -> Result<T, NativeGpuFault> {
        let frame = NativeFrameId {
            output_id: self.core.context.owner_id,
            project_epoch: 0,
            request_sequence: 0,
            geometry_generation: 0,
        };
        let plan = NativeAllocationPlan::new(frame);
        let quote = self.quote(&plan).map_err(|_| {
            self.core.fault(
                Some(frame),
                GpuPhase::Initialize,
                NativeGpuFaultKind::StateMismatch,
                "initialization reservation failed",
            )
        })?;
        let reservation = self.reserve(quote).map_err(|_| {
            self.core.fault(
                Some(frame),
                GpuPhase::Initialize,
                NativeGpuFaultKind::StateMismatch,
                "initialization reservation expired",
            )
        })?;
        let mut txn = self.begin_auxiliary_frame(frame, reservation)?;
        let mut phase = txn.begin_phase(GpuPhase::Initialize)?;
        let resource = Arc::new(create(phase.device(), phase.queue()));
        phase.retain_resource(Arc::clone(&resource), &[])?;
        let encoded = phase.finish(Vec::new());
        let receipt = txn.submit_phase(encoded)?;
        let mut pending = txn.finish();
        let started = Instant::now();
        loop {
            match self.poll_transaction(&mut pending, &|| false) {
                CompletionProgress::Pending if started.elapsed() < Duration::from_secs(5) => {
                    std::thread::yield_now()
                }
                CompletionProgress::Pending => {
                    self.cancel_and_retire(pending);
                    return Err(self.core.fault(
                        Some(frame),
                        GpuPhase::Initialize,
                        NativeGpuFaultKind::ScopeDrain,
                        "initialization completion timed out; owner retained",
                    ));
                }
                CompletionProgress::Complete(Ok(mut resolved)) => {
                    resolved.take_phase(receipt)?.into_validation()?;
                    return Arc::try_unwrap(resource).map_err(|_| {
                        self.core.fault(
                            Some(frame),
                            GpuPhase::Initialize,
                            NativeGpuFaultKind::StateMismatch,
                            "initialization owner still retained",
                        )
                    });
                }
                CompletionProgress::Complete(Err(error)) => return Err(error),
                CompletionProgress::Cancelled => {
                    return Err(self.core.fault(
                        Some(frame),
                        GpuPhase::Initialize,
                        NativeGpuFaultKind::StateMismatch,
                        "initialization cancelled",
                    ))
                }
            }
        }
    }
}

impl NativeGpuFrameTxn<'_> {
    pub(crate) fn context(&self) -> &Arc<NativeGpuContext> {
        self.owner.context()
    }
    pub(crate) fn frame(&self) -> NativeFrameId {
        self.owner.core.frames[self.owner.core.index(self.id).expect("live transaction")].frame
    }
    pub(crate) fn id(&self) -> RetirementId {
        self.id
    }
    pub(crate) fn validate_plan(&self, plan: &NativeAllocationPlan) -> Result<(), NativeGpuFault> {
        let index = self.owner.core.index(self.id)?;
        let reservation = self.owner.core.frames[index]
            .reservation
            .as_ref()
            .expect("active reservation");
        self.owner
            .core
            .budget
            .validate_reserved_plan(reservation, plan)
    }

    /// ISF root binding is permitted only after P has split its new allocation
    /// slots. Family-wide coverage alone cannot prevent kind/capacity theft.
    pub(crate) fn validate_root_plan(
        &self,
        plan: &NativeAllocationPlan,
    ) -> Result<(), NativeGpuFault> {
        let index = self.owner.core.index(self.id)?;
        let reservation = self.owner.core.frames[index]
            .reservation
            .as_ref()
            .expect("active reservation");
        self.owner.core.budget.validate_root_plan(reservation, plan)
    }

    pub(crate) fn bind_allocation(
        &mut self,
        kind: ChargeKind,
        capacity: u64,
    ) -> Result<AllocationCharge, NativeGpuFault> {
        self.owner.bind(self.id, kind, capacity)
    }
    pub(crate) fn retain_resource<T: Any + Send + Sync>(
        &mut self,
        resource: Arc<T>,
        allocations: &[GpuAllocationId],
    ) -> Result<(), NativeGpuFault> {
        self.owner.retain(self.id, resource, allocations)
    }
    pub(crate) fn split_reservation(
        &mut self,
        plan: &NativeAllocationPlan,
    ) -> Result<FrameReservation, NativeGpuFault> {
        let core = &mut *self.owner.core;
        let index = core.index(self.id)?;
        let reservation = core.frames[index]
            .reservation
            .as_mut()
            .expect("active reservation");
        core.budget.split_reservation(reservation, plan)
    }
    pub(crate) fn begin_phase(
        &mut self,
        phase: GpuPhase,
    ) -> Result<GpuPhaseEncoder<'_>, NativeGpuFault> {
        self.owner.check_device()?;
        self.owner.core.open(self.id, phase)?;
        self.owner
            .context
            .device
            .push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        self.owner
            .context
            .device
            .push_error_scope(wgpu::ErrorFilter::Validation);
        Ok(GpuPhaseEncoder {
            owner: self.owner,
            transaction: self.id,
            phase,
            finished: false,
        })
    }
    pub(crate) fn submit_phase(
        &mut self,
        encoded: EncodedGpuPhase,
    ) -> Result<PendingPhaseReceipt, NativeGpuFault> {
        if encoded.transaction != self.id {
            return Err(self.owner.core.fault(
                Some(self.frame()),
                encoded.phase,
                NativeGpuFaultKind::IdentityMismatch,
                "encoded phase belongs to another transaction",
            ));
        }
        self.owner.submit(encoded)
    }
    pub(crate) fn finish(mut self) -> PendingGpuTransaction {
        self.finished = true;
        let index = self.owner.core.index(self.id).expect("live transaction");
        if self.owner.core.frames[index].open.is_some() {
            self.owner.abandon_phase(self.id);
        }
        self.owner.core.seal(index);
        PendingGpuTransaction {
            id: self.id,
            marker: Arc::clone(&self.owner.core.frames[index].marker),
            terminal: false,
        }
    }
    pub(crate) fn cancel(mut self) -> RetiringGpuTransaction {
        self.owner.abandon_phase(self.id);
        self.finished = true;
        RetiringGpuTransaction { id: self.id }
    }
}

impl Drop for NativeGpuFrameTxn<'_> {
    fn drop(&mut self) {
        if !self.finished {
            self.owner.abandon_phase(self.id);
        }
    }
}

#[cfg(test)]
impl GpuCompletionPort for NativeGpuOwner {
    type Pending = PendingGpuTransaction;
    type Completed = ResolvedGpuTransaction;
    type Retiring = RetiringGpuTransaction;

    fn poll_completion(
        &mut self,
        pending: &mut Self::Pending,
        cancel: &dyn VideoRenderCancellation,
    ) -> CompletionProgress<Self::Completed, NativeGpuFault> {
        self.poll_transaction(pending, cancel)
    }

    fn cancel_and_retire(&mut self, pending: Self::Pending) -> Self::Retiring {
        NativeGpuOwner::cancel_and_retire(self, pending)
    }
}

impl GpuPhaseEncoder<'_> {
    pub(crate) fn context(&self) -> &Arc<NativeGpuContext> {
        self.owner.context()
    }
    pub(crate) fn device(&self) -> &wgpu::Device {
        &self.owner.context.device
    }
    pub(crate) fn queue(&self) -> &wgpu::Queue {
        &self.owner.context.queue
    }
    pub(crate) fn bind_allocation(
        &mut self,
        kind: ChargeKind,
        capacity: u64,
    ) -> Result<AllocationCharge, NativeGpuFault> {
        self.owner.bind(self.transaction, kind, capacity)
    }
    pub(crate) fn retain_resource<T: Any + Send + Sync>(
        &mut self,
        resource: Arc<T>,
        allocations: &[GpuAllocationId],
    ) -> Result<(), NativeGpuFault> {
        self.owner.retain(self.transaction, resource, allocations)
    }
    pub(crate) fn bind_reserved_allocation(
        &mut self,
        reservation: &mut FrameReservation,
        kind: ChargeKind,
        capacity: u64,
    ) -> Result<AllocationCharge, NativeGpuFault> {
        self.owner
            .bind_reserved(self.transaction, reservation, kind, capacity)
    }
    pub(crate) fn mark_queue_write(&mut self) {
        let index = self.owner.core.index(self.transaction).expect("live phase");
        self.owner.core.frames[index].queued_write = true;
    }
    pub(crate) fn record_failure(&mut self, kind: NativeGpuFaultKind, message: impl Into<String>) {
        let index = self.owner.core.index(self.transaction).expect("live phase");
        self.owner.core.frames[index]
            .open_failure
            .get_or_insert(DriverFailure {
                kind,
                message: message.into(),
            });
    }
    pub(crate) fn finish(mut self, commands: Vec<wgpu::CommandBuffer>) -> EncodedGpuPhase {
        self.finished = true;
        EncodedGpuPhase {
            transaction: self.transaction,
            phase: self.phase,
            commands,
        }
    }
}

impl Drop for GpuPhaseEncoder<'_> {
    fn drop(&mut self) {
        if !self.finished {
            self.owner.abandon_phase(self.transaction);
        }
    }
}

#[cfg(test)]
#[path = "native_gpu_context_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "native_gpu_context_cpu_rearm_tests.rs"]
mod cpu_rearm_tests;
