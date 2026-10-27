use super::*;

#[test]
fn native_gpu_fatal_phase_receipt_retains_generation_even_with_completion_marker() {
    for kind in [
        NativeGpuFaultKind::DeviceLost,
        NativeGpuFaultKind::ScopeDrain,
    ] {
        let mut core = scripted();
        let reservation = reserve(&mut core, 1, 32);
        let id = core.begin(frame(1), reservation).unwrap();
        let weak = retained(&mut core, id, 32);
        core.frames[0].open_failure = Some(DriverFailure {
            kind: NativeGpuFaultKind::Validation,
            message: "earlier encode error must not mask failed cleanup".into(),
        });
        phase(
            &mut core,
            id,
            GpuPhase::IsfBatch,
            Err(DriverFailure {
                kind,
                message: "fatal phase".into(),
            }),
        );
        core.driver.completed = 1;
        core.driver.scope_ready = true;
        let fault = core.progress().unwrap_err();
        assert_eq!(fault.kind, kind);
        assert_eq!(fault.phase, GpuPhase::IsfBatch);
        assert!(weak.upgrade().is_some());
        assert_eq!(core.frames.len(), 1);
        assert_eq!(
            core.budget
                .snapshot()
                .for_kind(ChargeKind::IsfScratch)
                .quarantined_bytes,
            32
        );
        assert!(core.retirement().is_err());
    }
}

#[test]
fn native_gpu_map_failure_cannot_publish_or_release_a_dependent_phase_early() {
    let mut core = scripted();
    let reservation = reserve(&mut core, 1, 64);
    let id = core.begin(frame(1), reservation).unwrap();
    let weak = retained(&mut core, id, 64);
    let isf = phase(&mut core, id, GpuPhase::IsfBatch, Ok(()));
    let snapshot = phase(
        &mut core,
        id,
        GpuPhase::CompositionSnapshot,
        Err(DriverFailure {
            kind: NativeGpuFaultKind::Map,
            message: "injected map failure".into(),
        }),
    );
    core.driver.scope_ready = true;
    core.driver.completed = isf.submission().sequence;
    core.progress().unwrap();
    assert!(!core.ready(0));
    assert!(weak.upgrade().is_some());
    core.driver.completed = snapshot.submission().sequence;
    core.progress().unwrap();
    let mut resolved = core.finish_record(0).unwrap();
    assert!(resolved.take_phase(isf).unwrap().validation().is_ok());
    let fault = resolved
        .take_phase(snapshot)
        .unwrap()
        .into_validation()
        .unwrap_err();
    assert_eq!(fault.kind, NativeGpuFaultKind::Map);
    assert_eq!(fault.phase, GpuPhase::CompositionSnapshot);
    assert!(weak.upgrade().is_none());
    assert_eq!(core.budget.snapshot().managed_bytes(), 0);
}

#[test]
fn native_gpu_transaction_sequence_exhaustion_refunds_unexecuted_reservation() {
    let mut core = scripted();
    core.next_transaction = u64::MAX;
    let reservation = reserve(&mut core, 1, 32);
    let fault = core.begin(frame(1), reservation).unwrap_err();
    assert_eq!(fault.kind, NativeGpuFaultKind::SequenceExhausted);
    assert_eq!(core.budget.snapshot().managed_bytes(), 0);
    assert_eq!(core.budget.snapshot().reservation_count, 0);
    assert!(core.frames.is_empty());
    assert_eq!(core.driver.polls, 0);
}

#[test]
fn native_gpu_cancelled_history_read_does_not_retire_successful_history_allocation() {
    let mut core = scripted();
    let mut history_reservation = reserve(&mut core, 1, 32);
    let allocation = core.budget.next_allocation_id().unwrap();
    let charge = core
        .budget
        .bind_allocation(
            &mut history_reservation,
            allocation,
            ChargeKind::IsfScratch,
            32,
        )
        .unwrap();
    core.budget.cancel_reservation(history_reservation).unwrap();
    let history = Arc::new(ChargedBytes {
        _bytes: vec![0; 32],
        charge,
    });
    let mut plan = NativeAllocationPlan::new(frame(2));
    plan.add_existing(allocation);
    let quote = core.budget.quote(&plan).unwrap();
    let reservation = core.budget.reserve(quote).unwrap();
    let id = core.begin(frame(2), reservation).unwrap();
    core.frames[0]
        .resources
        .push(Arc::clone(&history) as Arc<dyn Any + Send + Sync>);
    core.frames[0].allocations.push(allocation);
    let receipt = phase(&mut core, id, GpuPhase::CompositionSnapshot, Ok(()));
    core.retire(id).unwrap();
    assert_eq!(
        core.budget
            .snapshot()
            .for_kind(ChargeKind::IsfScratch)
            .active_bytes,
        32
    );
    core.driver.completed = receipt.submission().sequence;
    core.driver.scope_ready = true;
    core.retirement().unwrap();
    assert!(core.frames.is_empty());
    assert!(
        core.budget.quote(&plan).is_ok(),
        "cancelled reader must not invalidate historical resident"
    );
    drop(history);
    core.reap().unwrap();
    assert_eq!(core.budget.snapshot().managed_bytes(), 0);
}

fn frame(sequence: u64) -> NativeFrameId {
    NativeFrameId {
        output_id: 11,
        project_epoch: 7,
        request_sequence: sequence,
        geometry_generation: 2,
    }
}
fn context() -> GpuContextId {
    GpuContextId {
        owner_id: 11,
        device_generation: 3,
    }
}

#[derive(Default)]
struct ScriptedGpuDriver {
    completed: u64,
    scope_ready: bool,
    polls: usize,
    failure: Option<DriverFailure>,
}
impl CompletionDriver for ScriptedGpuDriver {
    type Ticket = u64;
    type Scopes = Option<Result<(), DriverFailure>>;
    fn poll(&mut self) -> Result<(), DriverFailure> {
        self.polls += 1;
        match self.failure.take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
    fn complete(&self, ticket: &u64) -> bool {
        *ticket <= self.completed
    }
    fn scopes(&mut self, scopes: &mut Self::Scopes) -> Option<Result<(), DriverFailure>> {
        if self.scope_ready {
            scopes.take()
        } else {
            None
        }
    }
}

fn scripted() -> BrokerCore<ScriptedGpuDriver> {
    BrokerCore::new(ScriptedGpuDriver::default(), context())
}

fn reserve(
    core: &mut BrokerCore<ScriptedGpuDriver>,
    sequence: u64,
    bytes: u64,
) -> FrameReservation {
    let mut plan = NativeAllocationPlan::new(frame(sequence));
    if bytes != 0 {
        plan.add_new(ChargeKind::IsfScratch, bytes).unwrap();
    }
    let quote = core.budget.quote(&plan).unwrap();
    core.budget.reserve(quote).unwrap()
}

fn phase(
    core: &mut BrokerCore<ScriptedGpuDriver>,
    id: RetirementId,
    phase: GpuPhase,
    result: Result<(), DriverFailure>,
) -> PendingPhaseReceipt {
    core.open(id, phase).unwrap();
    let sequence = core.submission_id(frame(1), phase).unwrap();
    core.register(id, sequence, phase, sequence.sequence, Some(result))
        .unwrap()
}

struct ChargedBytes {
    _bytes: Vec<u8>,
    charge: AllocationCharge,
}

fn retained(
    core: &mut BrokerCore<ScriptedGpuDriver>,
    id: RetirementId,
    bytes: u64,
) -> std::sync::Weak<ChargedBytes> {
    let index = core.index(id).unwrap();
    let allocation = core.budget.next_allocation_id().unwrap();
    let charge = core
        .budget
        .bind_allocation(
            core.frames[index].reservation.as_mut().unwrap(),
            allocation,
            ChargeKind::IsfScratch,
            bytes,
        )
        .unwrap();
    let owner = Arc::new(ChargedBytes {
        _bytes: vec![0; bytes as usize],
        charge,
    });
    let weak = Arc::downgrade(&owner);
    core.frames[index].allocations.push(owner.charge.id());
    core.frames[index].owned_allocations.push(owner.charge.id());
    core.frames[index].resources.push(owner);
    weak
}

#[test]
fn native_gpu_broker_two_phases_are_independent_and_do_not_wait_between_submits() {
    let mut core = scripted();
    let reservation = reserve(&mut core, 1, 64);
    let id = core.begin(frame(1), reservation).unwrap();
    let weak = retained(&mut core, id, 64);
    let isf = phase(&mut core, id, GpuPhase::IsfBatch, Ok(()));
    let compose = phase(
        &mut core,
        id,
        GpuPhase::CompositionSnapshot,
        Err(DriverFailure {
            kind: NativeGpuFaultKind::Validation,
            message: "compose-only".into(),
        }),
    );
    assert_eq!(core.driver.polls, 0);
    assert!(weak.upgrade().is_some());
    assert_eq!(core.budget.snapshot().managed_bytes(), 64);
    core.driver.scope_ready = true;
    core.driver.completed = isf.submission().sequence;
    core.progress().unwrap();
    assert!(!core.ready(0));
    assert!(
        weak.upgrade().is_some(),
        "dependent composition still owns input"
    );
    core.driver.completed = compose.submission().sequence;
    core.progress().unwrap();
    assert!(core.ready(0));
    let mut results = core.finish_record(0).unwrap();
    let isf = results.take_phase(isf).unwrap();
    assert!(isf.validation().is_ok());
    assert_eq!(isf.context(), context());
    assert_eq!(isf.frame(), frame(1));
    assert_eq!(isf.phase(), GpuPhase::IsfBatch);
    assert_eq!(isf.transaction(), id);
    assert_eq!(isf.submission().sequence, 1);
    let compose = results
        .take_phase(compose)
        .unwrap()
        .into_validation()
        .unwrap_err();
    assert_eq!(compose.phase, GpuPhase::CompositionSnapshot);
    assert_eq!(compose.kind, NativeGpuFaultKind::Validation);
    assert!(weak.upgrade().is_none());
    assert_eq!(core.budget.snapshot().managed_bytes(), 0);
}

#[test]
fn native_gpu_recorded_encode_failure_still_drains_pending_scopes_and_owners() {
    let mut core = scripted();
    let reservation = reserve(&mut core, 1, 32);
    let id = core.begin(frame(1), reservation).unwrap();
    let weak = retained(&mut core, id, 32);
    core.open(id, GpuPhase::IsfBatch).unwrap();
    core.frames[0].open_failure = Some(DriverFailure {
        kind: NativeGpuFaultKind::Validation,
        message: "middle stage".into(),
    });
    let submission = core.submission_id(frame(1), GpuPhase::IsfBatch).unwrap();
    let receipt = core
        .register(
            id,
            submission,
            GpuPhase::IsfBatch,
            submission.sequence,
            Some(Ok(())),
        )
        .unwrap();
    core.driver.completed = submission.sequence;
    core.progress().unwrap();
    assert!(
        !core.ready(0),
        "recorded fault does not prove pop completion"
    );
    assert!(weak.upgrade().is_some());
    core.driver.scope_ready = true;
    core.progress().unwrap();
    let mut resolved = core.finish_record(0).unwrap();
    assert_eq!(
        resolved
            .take_phase(receipt)
            .unwrap()
            .into_validation()
            .unwrap_err()
            .message,
        "middle stage"
    );
    assert!(weak.upgrade().is_none());
}

#[test]
fn native_gpu_drop_pending_is_cancel_not_resource_release_and_stop_reaps() {
    let mut core = scripted();
    let reservation = reserve(&mut core, 1, 64);
    let id = core.begin(frame(1), reservation).unwrap();
    let weak = retained(&mut core, id, 64);
    phase(&mut core, id, GpuPhase::IsfBatch, Ok(()));
    drop(PendingGpuTransaction {
        id,
        marker: Arc::clone(&core.frames[0].marker),
        terminal: false,
    });
    assert_eq!(
        core.frames[0].marker.load(Ordering::Acquire),
        CANCELLED_TOKEN
    );
    let stop = core.stop();
    assert_eq!(stop.context(), context());
    assert_eq!(
        core.retirement().unwrap(),
        RetirementProgress::Pending {
            transactions: 1,
            allocations: 1
        }
    );
    assert!(weak.upgrade().is_some());
    assert_eq!(core.budget.snapshot().managed_bytes(), 64);
    let reservation = reserve(&mut core, 2, 1);
    assert!(core.begin(frame(2), reservation).is_err());
    assert_eq!(
        core.budget.snapshot().managed_bytes(),
        64,
        "failed admission refunds only new unused bytes"
    );
    core.driver.scope_ready = true;
    core.driver.completed = 1;
    assert_eq!(core.retirement().unwrap(), RetirementProgress::Reaped);
    assert_eq!(core.state, GpuOwnerState::Reaped);
    assert!(weak.upgrade().is_none());
}

#[test]
fn native_gpu_device_loss_and_drain_failure_quarantine_exact_live_owners() {
    for kind in [
        NativeGpuFaultKind::DeviceLost,
        NativeGpuFaultKind::ScopeDrain,
    ] {
        let mut core = scripted();
        let reservation = reserve(&mut core, 1, 64);
        let id = core.begin(frame(1), reservation).unwrap();
        let weak = retained(&mut core, id, 64);
        phase(&mut core, id, GpuPhase::IsfBatch, Ok(()));
        core.driver.failure = Some(DriverFailure {
            kind,
            message: "injected".into(),
        });
        assert_eq!(core.progress().unwrap_err().kind, kind);
        assert!(weak.upgrade().is_some());
        assert_eq!(
            core.budget
                .snapshot()
                .for_kind(ChargeKind::IsfScratch)
                .quarantined_bytes,
            64
        );
        core.driver.completed = u64::MAX;
        core.driver.scope_ready = true;
        assert!(core.retirement().is_err());
        assert!(
            weak.upgrade().is_some(),
            "later fake completion cannot unpoison a generation"
        );
        assert_eq!(core.budget.snapshot().managed_bytes(), 64);
    }
}

#[test]
fn native_gpu_generation_phase_and_sequence_mismatches_fail_closed() {
    let mut core = scripted();
    let reservation = reserve(&mut core, 1, 0);
    let id = core.begin(frame(1), reservation).unwrap();
    let old = RetirementId {
        context: GpuContextId {
            device_generation: 2,
            ..context()
        },
        ..id
    };
    assert_eq!(
        core.open(old, GpuPhase::IsfBatch).unwrap_err().kind,
        NativeGpuFaultKind::IdentityMismatch
    );
    core.open(id, GpuPhase::IsfBatch).unwrap();
    assert_eq!(
        core.open(id, GpuPhase::CompositionSnapshot)
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::StateMismatch
    );
    core.next_submission = u64::MAX;
    assert_eq!(
        core.submission_id(frame(1), GpuPhase::IsfBatch)
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::SequenceExhausted
    );
    assert_eq!(core.driver.polls, 0);
    assert!(core.frames[0].phases.is_empty());
}

#[test]
fn native_gpu_limits_negotiate_optional_features_without_weakening_required_alignment() {
    let supported = wgpu::Features::FLOAT32_FILTERABLE;
    let requirements = NativeGpuRequirements::default();
    assert_eq!(
        negotiated_features(&requirements, supported, &wgpu::Limits::default()).unwrap(),
        supported
    );
    let required = NativeGpuRequirements {
        required_features: wgpu::Features::TEXTURE_COMPRESSION_BC,
        ..NativeGpuRequirements::default()
    };
    assert!(matches!(
        negotiated_features(&required, supported, &wgpu::Limits::default()),
        Err(ContextCreateError::RequiredFeature(_))
    ));
    let mut too_large = NativeGpuRequirements::default();
    too_large.required_limits.max_texture_dimension_2d = u32::MAX;
    assert!(matches!(
        negotiated_features(&too_large, supported, &wgpu::Limits::default()),
        Err(ContextCreateError::RequiredLimit)
    ));
    let mut alignment = NativeGpuRequirements::default();
    alignment
        .required_limits
        .min_uniform_buffer_offset_alignment = 1;
    assert!(matches!(
        negotiated_features(&alignment, supported, &wgpu::Limits::default()),
        Err(ContextCreateError::RequiredLimit)
    ));
    for error in [
        ContextCreateError::Instance("foreign".into()),
        ContextCreateError::Surface("window".into()),
        ContextCreateError::Adapter("adapter".into()),
        ContextCreateError::Device("device".into()),
        ContextCreateError::RequiredLimit,
        ContextCreateError::SequenceExhausted,
    ] {
        assert!(!error.to_string().is_empty());
    }
}

fn headless(owner: u64) -> NativeGpuOwner {
    NativeGpuFactory::new()
        .create_headless(owner, NativeGpuRequirements::default())
        .expect("real adapter required")
}

fn finish(
    owner: &mut NativeGpuOwner,
    pending: &mut PendingGpuTransaction,
) -> ResolvedGpuTransaction {
    let started = Instant::now();
    loop {
        match owner.poll_transaction(pending, &|| false) {
            CompletionProgress::Pending if started.elapsed() < Duration::from_secs(5) => {
                std::thread::yield_now()
            }
            CompletionProgress::Complete(Ok(result)) => return result,
            CompletionProgress::Complete(Err(error)) => panic!("broker completion: {error:?}"),
            CompletionProgress::Pending => panic!("real GPU did not complete within 5s"),
            CompletionProgress::Cancelled => panic!("unexpected cancellation"),
        }
    }
}

#[test]
fn native_gpu_real_headless_scope_pop_isolates_isf_and_composition_without_intermediate_poll() {
    let mut owner = headless(73);
    let context = Arc::clone(owner.context());
    assert_eq!(context.id(), owner.capabilities().context);
    assert_eq!(
        context.device().features(),
        owner.capabilities().enabled_features
    );
    assert_eq!(
        context.adapter().get_info().backend,
        owner.capabilities().adapter_info.backend
    );
    assert_eq!(context.device().limits(), owner.capabilities().limits);
    assert!(owner.capabilities().upload_row_pitch.is_some());
    assert_eq!(Arc::strong_count(context.instance()), 1);
    let quote = owner.quote(&NativeAllocationPlan::new(frame(1))).unwrap();
    let reservation = owner.reserve(quote).unwrap();
    let mut txn = owner.begin_frame(frame(1), reservation).unwrap();
    let isf = txn.begin_phase(GpuPhase::IsfBatch).unwrap();
    assert!(Arc::ptr_eq(isf.context(), &context));
    let encoder = isf
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let encoded = isf.finish(vec![encoder.finish()]);
    let isf_receipt = txn.submit_phase(encoded).unwrap();
    assert_eq!(isf_receipt.phase(), GpuPhase::IsfBatch);
    assert_eq!(isf_receipt.frame(), frame(1));
    let compose = txn.begin_phase(GpuPhase::CompositionSnapshot).unwrap();
    // MAP_READ with STORAGE is invalid without MAPPABLE_PRIMARY_BUFFERS.
    let bad = compose.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("injected composition validation"),
        size: 16,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });
    let encoded = compose.finish(Vec::new());
    let compose_receipt = txn.submit_phase(encoded).unwrap();
    let mut pending = txn.finish();
    assert!(owner.core.frames[0]
        .phases
        .iter()
        .all(|phase| !phase.complete));
    let mut resolved = finish(&mut owner, &mut pending);
    assert!(resolved
        .take_phase(isf_receipt)
        .unwrap()
        .validation()
        .is_ok());
    assert_eq!(
        resolved
            .take_phase(compose_receipt)
            .unwrap()
            .into_validation()
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::Validation
    );
    assert!(matches!(
        owner.poll_transaction(&mut pending, &|| false),
        CompletionProgress::Complete(Err(NativeGpuFault {
            kind: NativeGpuFaultKind::StateMismatch,
            ..
        }))
    ));
    drop(bad);
    owner.begin_stop();
    assert_eq!(owner.poll_retirement().unwrap(), RetirementProgress::Reaped);
}

#[test]
fn native_gpu_real_cancel_after_write_retains_until_cleanup_and_unbound_reservation_refunds() {
    let mut owner = headless(74);
    let mut plan = NativeAllocationPlan::new(frame(1));
    plan.add_new(ChargeKind::IsfScratch, 16).unwrap();
    plan.add_new(ChargeKind::QueueUploadStaging, 16).unwrap();
    plan.add_new(ChargeKind::CpuSnapshot, 128).unwrap();
    plan.add_new(ChargeKind::CachedEffectResource, 8).unwrap();
    let quote = owner.quote(&plan).unwrap();
    let reservation = owner.reserve(quote).unwrap();
    let mut txn = owner.begin_frame(frame(1), reservation).unwrap();
    assert_eq!(txn.frame(), frame(1));
    let transaction = txn.id();
    let mut phase = txn.begin_phase(GpuPhase::IsfBatch).unwrap();
    let charge = phase.bind_allocation(ChargeKind::IsfScratch, 16).unwrap();
    let staging = phase
        .bind_allocation(ChargeKind::QueueUploadStaging, 16)
        .unwrap();
    struct BufferOwner {
        buffer: wgpu::Buffer,
        _charge: AllocationCharge,
        _staging: AllocationCharge,
    }
    let ids = [charge.id(), staging.id()];
    let resource = Arc::new(BufferOwner {
        buffer: phase.device().create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 16,
            usage: wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }),
        _charge: charge,
        _staging: staging,
    });
    let weak = Arc::downgrade(&resource);
    phase.retain_resource(Arc::clone(&resource), &ids).unwrap();
    // Creation can fail after admission/binding but before a concrete owner is
    // installed. The broker's pin must keep that charge through cleanup too.
    let failed_creation = phase.bind_allocation(ChargeKind::CpuSnapshot, 128).unwrap();
    drop(failed_creation);
    phase.mark_queue_write();
    phase.queue().write_buffer(&resource.buffer, 0, &[1; 16]);
    phase.record_failure(NativeGpuFaultKind::Validation, "after write");
    drop(resource);
    drop(phase);
    let retired = txn.cancel();
    assert_eq!(retired.id(), transaction);
    assert!(weak.upgrade().is_some());
    assert_eq!(owner.budget_snapshot().managed_bytes(), 168);
    assert!(owner.core.frames[0].open.is_none());
    assert_eq!(
        owner.core.frames[0].phases.len(),
        1,
        "one cleanup submit drains pending write"
    );
    owner.begin_stop();
    let deadline = Instant::now();
    loop {
        match owner.poll_retirement().unwrap() {
            RetirementProgress::Reaped => break,
            RetirementProgress::Pending { .. } if deadline.elapsed() < Duration::from_secs(5) => {
                std::thread::yield_now()
            }
            _ => panic!("cancel cleanup did not finish"),
        }
    }
    assert!(weak.upgrade().is_none());
    assert_eq!(owner.budget_snapshot().managed_bytes(), 0);
}

#[test]
fn native_gpu_real_initialize_and_send_handoff_share_exact_device() {
    fn assert_send<T: Send>() {}
    assert_send::<NativeGpuOwner>();
    let factory = NativeGpuFactory::new();
    let mut a = factory
        .create_headless(75, NativeGpuRequirements::default())
        .unwrap();
    let b = factory
        .create_headless(75, NativeGpuRequirements::default())
        .unwrap();
    assert_ne!(
        a.context().id(),
        b.context().id(),
        "same owner and adapter name do not imply same device"
    );
    assert!(Arc::ptr_eq(factory.instance(), a.context().instance()));
    let shader = a.initialize(|device, _queue| device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("native GPU initialize scope"),
        source: wgpu::ShaderSource::Wgsl("@vertex fn main() -> @builtin(position) vec4<f32> { return vec4<f32>(0.0, 0.0, 0.0, 1.0); }".into()),
    })).unwrap();
    assert_eq!(a.state(), GpuOwnerState::Active);
    drop(shader);
    a.begin_stop();
    assert_eq!(a.poll_retirement().unwrap(), RetirementProgress::Reaped);
}

#[test]
fn native_gpu_result_receipt_is_affine_and_foreign_transaction_rejected() {
    let mut core = scripted();
    let reservation = reserve(&mut core, 1, 0);
    let id = core.begin(frame(1), reservation).unwrap();
    let receipt = phase(&mut core, id, GpuPhase::HistoricalReplay, Ok(()));
    core.driver.completed = 1;
    core.driver.scope_ready = true;
    core.progress().unwrap();
    let mut result = core.finish_record(0).unwrap();
    let foreign = PendingPhaseReceipt {
        transaction: RetirementId {
            sequence: 999,
            ..id
        },
        frame: receipt.frame,
        phase: receipt.phase,
        submission: receipt.submission,
    };
    assert_eq!(
        result.take_phase(foreign).err().unwrap().kind,
        NativeGpuFaultKind::IdentityMismatch
    );
    assert!(result.take_phase(receipt).unwrap().validation().is_ok());
    assert!(result.phases.is_empty());
}

#[test]
fn native_gpu_cancel_port_keeps_registry_when_caller_consumes_ticket() {
    let mut owner = headless(76);
    let mut subplan = NativeAllocationPlan::new(frame(1));
    subplan.add_new(ChargeKind::CpuSnapshot, 16).unwrap();
    subplan.add_new(ChargeKind::MappingBuffer, 16).unwrap();
    let mut plan = NativeAllocationPlan::new(frame(1));
    plan.merge(&subplan).unwrap();
    let quote = owner.quote(&plan).unwrap();
    let reservation = owner.reserve(quote).unwrap();
    let mut txn = owner.begin_frame(frame(1), reservation).unwrap();
    assert!(txn.validate_plan(&plan).is_ok());
    let mut unrelated = NativeAllocationPlan::new(frame(1));
    unrelated.add_new(ChargeKind::CpuSnapshot, 16).unwrap();
    unrelated.add_new(ChargeKind::MappingBuffer, 16).unwrap();
    assert!(
        txn.validate_plan(&unrelated).is_err(),
        "same extent/bytes do not establish plan identity"
    );
    let mut child = txn.split_reservation(&subplan).unwrap();
    let mut phase = txn.begin_phase(GpuPhase::IsfBatch).unwrap();
    let charge = phase
        .bind_reserved_allocation(&mut child, ChargeKind::CpuSnapshot, 16)
        .unwrap();
    let id = charge.id();
    let pixels = Arc::new(ChargedBytes {
        _bytes: vec![0; 16],
        charge,
    });
    phase.retain_resource(pixels, &[id]).unwrap();
    drop(child); // unused child bytes remain authoritative until whole-frame reap
    let encoded = phase.finish(Vec::new());
    txn.submit_phase(encoded).unwrap();
    let mut pending = txn.finish();
    let cancellation_reads = std::cell::Cell::new(0);
    let cancel_after_poll = || {
        let reads = cancellation_reads.get() + 1;
        cancellation_reads.set(reads);
        reads >= 2
    };
    assert!(matches!(
        GpuCompletionPort::poll_completion(&mut owner, &mut pending, &cancel_after_poll),
        CompletionProgress::Cancelled
    ));
    assert_eq!(
        cancellation_reads.get(),
        2,
        "cancel is rechecked after callbacks"
    );
    let retiring = GpuCompletionPort::cancel_and_retire(&mut owner, pending);
    assert_eq!(retiring.id().context, owner.context().id());
    assert_eq!(owner.core.frames.len(), 1);
    owner.begin_stop();
    let started = Instant::now();
    loop {
        match owner.poll_retirement().unwrap() {
            RetirementProgress::Reaped => break,
            RetirementProgress::Pending { .. } if started.elapsed() < Duration::from_secs(5) => {
                std::thread::yield_now()
            }
            _ => panic!("retirement timeout"),
        }
    }
    assert_eq!(owner.budget_snapshot().reservation_count, 0);
    assert_eq!(owner.budget_snapshot().managed_bytes(), 0);
}

// The scripted driver makes GPU progress controllable. These tests distinguish
// a released logical slot from a completed/reaped present submission.
#[test]
fn native_gpu_g3_sealed_present_admits_one_next_isf_without_poll_or_early_release() {
    let mut core = scripted();
    let reservation = reserve(&mut core, 1, 32);
    let old = core.begin(frame(1), reservation).unwrap();
    let old_owner = retained(&mut core, old, 32);
    let present = phase(&mut core, old, GpuPhase::MappingPresent, Ok(()));

    let premature = reserve(&mut core, 2, 64);
    assert_eq!(
        core.begin(frame(2), premature).unwrap_err().kind,
        NativeGpuFaultKind::StateMismatch,
        "submit alone must not release the logical slot"
    );
    assert_eq!(core.budget.snapshot().managed_bytes(), 32);
    core.seal(core.index(old).unwrap());
    assert_eq!(core.frames[0].stage, FrameRecordStage::PresentRetirement);
    assert_eq!(core.frames[0].marker.load(Ordering::Acquire), ACTIVE_TOKEN);

    let next_reservation = reserve(&mut core, 2, 64);
    let next = core.begin(frame(2), next_reservation).unwrap();
    let next_owner = retained(&mut core, next, 64);
    let isf = phase(&mut core, next, GpuPhase::IsfBatch, Ok(()));
    core.seal(core.index(next).unwrap());
    assert_eq!(core.driver.polls, 0);
    assert_eq!(core.driver.completed, 0);
    assert_eq!(core.frames.len(), 2);
    assert_eq!(isf.submission().sequence, present.submission().sequence + 1);
    assert_ne!(
        core.frames[core.index(old).unwrap()].owned_allocations[0],
        core.frames[core.index(next).unwrap()].owned_allocations[0],
        "pending-present capacity cannot become the next allocation"
    );
    assert!(old_owner.upgrade().is_some());
    assert!(next_owner.upgrade().is_some());
    assert_eq!(core.budget.snapshot().managed_bytes(), 96);

    let speculative = reserve(&mut core, 3, 1);
    assert_eq!(
        core.begin(frame(3), speculative).unwrap_err().kind,
        NativeGpuFaultKind::StateMismatch
    );
    assert_eq!(core.budget.snapshot().managed_bytes(), 96);

    core.driver.scope_ready = true;
    core.driver.completed = isf.submission().sequence;
    core.progress().unwrap();
    // Resolve N+1 first: Vec removal must leave the old receipt and owners valid.
    let mut resolved_next = core.finish_record(core.index(next).unwrap()).unwrap();
    let foreign_receipt = PendingPhaseReceipt {
        transaction: old,
        frame: frame(1),
        phase: GpuPhase::MappingPresent,
        submission: present.submission(),
    };
    assert_eq!(
        resolved_next
            .take_phase(foreign_receipt)
            .err()
            .unwrap()
            .kind,
        NativeGpuFaultKind::IdentityMismatch
    );
    assert!(resolved_next.take_phase(isf).unwrap().validation().is_ok());
    assert!(next_owner.upgrade().is_none());
    assert!(old_owner.upgrade().is_some());
    assert_eq!(core.frames.len(), 1);
    assert_eq!(core.frames[0].id, old);
    assert_eq!(core.budget.snapshot().managed_bytes(), 32);
    let mut resolved_old = core.finish_record(core.index(old).unwrap()).unwrap();
    assert!(resolved_old
        .take_phase(present)
        .unwrap()
        .validation()
        .is_ok());
    assert!(old_owner.upgrade().is_none());
    assert_eq!(core.budget.snapshot().managed_bytes(), 0);
}

#[test]
fn native_gpu_g3_only_a_separate_single_present_phase_releases_the_logical_slot() {
    let phase_sets: &[&[GpuPhase]] = &[
        &[],
        &[GpuPhase::Initialize],
        &[GpuPhase::IsfBatch],
        &[GpuPhase::CompositionSnapshot],
        &[GpuPhase::HistoricalReplay],
        &[GpuPhase::Retirement],
        &[GpuPhase::IsfBatch, GpuPhase::MappingPresent],
        &[GpuPhase::CompositionSnapshot, GpuPhase::MappingPresent],
        &[GpuPhase::MappingPresent, GpuPhase::MappingPresent],
    ];
    for phases in phase_sets {
        let mut core = scripted();
        let reservation = reserve(&mut core, 1, 0);
        let id = core.begin(frame(1), reservation).unwrap();
        for gpu_phase in *phases {
            phase(&mut core, id, *gpu_phase, Ok(()));
        }
        core.seal(core.index(id).unwrap());
        assert_eq!(
            core.frames[0].stage,
            FrameRecordStage::Sealed,
            "phase set {phases:?}"
        );
        assert!(core.has_logical_frame());
        assert_eq!(
            core.open(id, GpuPhase::IsfBatch).unwrap_err().kind,
            NativeGpuFaultKind::StateMismatch,
            "a sealed transaction cannot reopen"
        );
        let reservation = reserve(&mut core, 2, 1);
        assert_eq!(
            core.begin(frame(2), reservation).unwrap_err().kind,
            NativeGpuFaultKind::StateMismatch,
            "phase set {phases:?}"
        );
        assert_eq!(core.driver.polls, 0);
        assert_eq!(core.budget.snapshot().managed_bytes(), 0);
    }
}

#[test]
fn native_gpu_g3_unfinished_cancelled_or_faulted_present_stays_in_the_logical_slot() {
    for condition in [
        "open",
        "queued-write",
        "cancelled",
        "encode-fault",
        "scope-fault",
        "stopping",
    ] {
        let mut core = scripted();
        let reservation = reserve(&mut core, 1, 32);
        let id = core.begin(frame(1), reservation).unwrap();
        let resource = retained(&mut core, id, 32);
        phase(&mut core, id, GpuPhase::MappingPresent, Ok(()));
        match condition {
            "open" => core.open(id, GpuPhase::CompositionSnapshot).unwrap(),
            "queued-write" => core.frames[0].queued_write = true,
            "cancelled" => core.retire(id).unwrap(),
            "encode-fault" => {
                core.frames[0].phases[0].execution_failure = Some(DriverFailure {
                    kind: NativeGpuFaultKind::Validation,
                    message: "known mapping encode failure".into(),
                });
            }
            "scope-fault" => {
                core.frames[0].phases[0].scope_result = Some(Err(DriverFailure {
                    kind: NativeGpuFaultKind::Validation,
                    message: "already resolved invalid mapping scope".into(),
                }));
            }
            "stopping" => {
                core.stop();
            }
            _ => unreachable!(),
        }
        core.seal(core.index(id).unwrap());
        assert_eq!(
            core.frames[0].stage,
            FrameRecordStage::Sealed,
            "{condition}"
        );
        let reservation = reserve(&mut core, 2, 1);
        assert_eq!(
            core.begin(frame(2), reservation).unwrap_err().kind,
            NativeGpuFaultKind::StateMismatch,
            "{condition}"
        );
        assert!(resource.upgrade().is_some(), "{condition}");
        assert_eq!(core.budget.snapshot().managed_bytes(), 32);
        assert_eq!(core.driver.polls, 0);
    }
}

#[test]
fn native_gpu_g3_second_present_is_rejected_before_scope_or_submission() {
    let mut core = scripted();
    let reservation = reserve(&mut core, 1, 0);
    let old = core.begin(frame(1), reservation).unwrap();
    phase(&mut core, old, GpuPhase::MappingPresent, Ok(()));
    core.seal(core.index(old).unwrap());
    assert_eq!(
        core.open(old, GpuPhase::MappingPresent).unwrap_err().kind,
        NativeGpuFaultKind::StateMismatch,
        "the detached record cannot be reused as an encoder"
    );
    let reservation = reserve(&mut core, 2, 0);
    let next = core.begin(frame(2), reservation).unwrap();
    let submission_before = core.next_submission;
    assert_eq!(
        core.open(next, GpuPhase::MappingPresent).unwrap_err().kind,
        NativeGpuFaultKind::StateMismatch
    );
    let index = core.index(next).unwrap();
    assert!(core.frames[index].open.is_none());
    assert!(core.frames[index].phases.is_empty());
    assert!(!core.frames[index].queued_write);
    assert_eq!(core.next_submission, submission_before);
    assert_eq!(core.driver.polls, 0);
    assert_eq!(core.frames.len(), 2);
    assert_eq!(core.budget.snapshot().managed_bytes(), 0);
    // No capacity is charged here: a byte budget alone cannot bound this queue.
    phase(&mut core, next, GpuPhase::IsfBatch, Ok(()));
    assert_eq!(core.next_submission, submission_before + 1);
}

#[test]
fn native_gpu_g3_stop_retains_present_and_next_frame_until_scopes_and_external_owner_release() {
    let mut core = scripted();
    let reservation = reserve(&mut core, 1, 32);
    let old = core.begin(frame(1), reservation).unwrap();
    let old_weak = retained(&mut core, old, 32);
    let external = old_weak.upgrade().unwrap();
    let present = phase(&mut core, old, GpuPhase::MappingPresent, Ok(()));
    core.seal(core.index(old).unwrap());
    let reservation = reserve(&mut core, 2, 64);
    let next = core.begin(frame(2), reservation).unwrap();
    let next_weak = retained(&mut core, next, 64);
    let isf = phase(&mut core, next, GpuPhase::IsfBatch, Ok(()));
    core.seal(core.index(next).unwrap());

    core.stop();
    assert_eq!(core.state, GpuOwnerState::Retiring);
    assert!(core
        .frames
        .iter()
        .all(|record| record.marker.load(Ordering::Acquire) == CANCELLED_TOKEN));
    assert_eq!(
        core.retirement().unwrap(),
        RetirementProgress::Pending {
            transactions: 2,
            allocations: 2
        }
    );
    core.driver.completed = present.submission().sequence;
    assert_eq!(
        core.retirement().unwrap(),
        RetirementProgress::Pending {
            transactions: 2,
            allocations: 2
        },
        "GPU completion is insufficient while scope pop is pending"
    );
    assert!(old_weak.upgrade().is_some());
    assert!(next_weak.upgrade().is_some());
    assert_eq!(core.budget.snapshot().managed_bytes(), 96);
    core.driver.scope_ready = true;
    assert_eq!(
        core.retirement().unwrap(),
        RetirementProgress::Pending {
            transactions: 1,
            allocations: 2
        }
    );
    assert_eq!(core.frames[0].id, next);
    core.driver.completed = isf.submission().sequence;
    assert_eq!(
        core.retirement().unwrap(),
        RetirementProgress::Pending {
            transactions: 0,
            allocations: 1
        }
    );
    assert!(next_weak.upgrade().is_none());
    assert!(old_weak.upgrade().is_some());
    assert_eq!(core.budget.snapshot().managed_bytes(), 32);
    assert_eq!(
        core.budget
            .snapshot()
            .for_kind(ChargeKind::IsfScratch)
            .retired_bytes,
        32
    );
    drop(external);
    assert_eq!(core.retirement().unwrap(), RetirementProgress::Reaped);
    assert!(old_weak.upgrade().is_none());
    assert_eq!(core.budget.snapshot().managed_bytes(), 0);
    assert_eq!(core.budget.snapshot().reservation_count, 0);
}

#[test]
fn native_gpu_g3_loss_or_poison_quarantines_both_records_despite_late_completion() {
    for kind in [
        NativeGpuFaultKind::DeviceLost,
        NativeGpuFaultKind::ScopeDrain,
    ] {
        for via_scope in [false, true] {
            let mut core = scripted();
            let reservation = reserve(&mut core, 1, 32);
            let old = core.begin(frame(1), reservation).unwrap();
            let old_weak = retained(&mut core, old, 32);
            let result = if via_scope {
                Err(DriverFailure {
                    kind,
                    message: "late fatal present scope".into(),
                })
            } else {
                Ok(())
            };
            phase(&mut core, old, GpuPhase::MappingPresent, result);
            core.seal(core.index(old).unwrap());
            let reservation = reserve(&mut core, 2, 64);
            let next = core.begin(frame(2), reservation).unwrap();
            let next_weak = retained(&mut core, next, 64);
            phase(&mut core, next, GpuPhase::IsfBatch, Ok(()));
            core.seal(core.index(next).unwrap());
            let unexecuted = reserve(&mut core, 3, 1);
            if !via_scope {
                core.driver.failure = Some(DriverFailure {
                    kind,
                    message: "lost device or failed driver drain".into(),
                });
            }
            core.driver.completed = u64::MAX;
            core.driver.scope_ready = true;
            let fault = core.progress().unwrap_err();
            assert_eq!(fault.kind, kind);
            assert_eq!(
                core.state,
                if kind == NativeGpuFaultKind::DeviceLost {
                    GpuOwnerState::DeviceLostRetained
                } else {
                    GpuOwnerState::PoisonedRetained
                }
            );
            assert!(old_weak.upgrade().is_some());
            assert!(next_weak.upgrade().is_some());
            assert_eq!(core.frames.len(), 2);
            assert_eq!(
                core.budget
                    .snapshot()
                    .for_kind(ChargeKind::IsfScratch)
                    .quarantined_bytes,
                96
            );
            assert_eq!(
                core.begin(frame(3), unexecuted).unwrap_err().kind,
                NativeGpuFaultKind::StateMismatch
            );
            assert_eq!(core.budget.snapshot().managed_bytes(), 96);
            assert!(matches!(
                core.budget.quote(&NativeAllocationPlan::new(frame(4))),
                Err(NativeGpuDecline::QuoteExpired)
            ));
            core.stop();
            assert!(core.retirement().is_err());
            assert!(core.retirement().is_err());
            assert_eq!(core.frames.len(), 2);
            assert!(old_weak.upgrade().is_some());
            assert!(next_weak.upgrade().is_some());
            assert_eq!(core.budget.snapshot().managed_bytes(), 96);
        }
    }
}

#[test]
fn native_gpu_g3_exact_512mib_includes_present_reservations_and_shared_read_owners() {
    use crate::native_gpu_budget::NATIVE_MANAGED_LIMIT;
    let mut core = scripted();
    let present_bytes = 64 * 1024 * 1024;
    let unused_present_bytes = 16;
    let mut plan = NativeAllocationPlan::new(frame(1));
    plan.add_new(ChargeKind::MappingBuffer, present_bytes)
        .unwrap();
    plan.add_new(ChargeKind::QueueUploadStaging, unused_present_bytes)
        .unwrap();
    let quote = core.budget.quote(&plan).unwrap();
    let reservation = core.budget.reserve(quote).unwrap();
    let old = core.begin(frame(1), reservation).unwrap();
    let old_index = core.index(old).unwrap();
    let old_allocation = core.budget.next_allocation_id().unwrap();
    // A modeled ledger oracle: these capacity charges do not allocate 512 MiB
    // of physical texture memory. Real GPU overlap is tested separately below.
    let old_charge = Arc::new(
        core.budget
            .bind_allocation(
                core.frames[old_index].reservation.as_mut().unwrap(),
                old_allocation,
                ChargeKind::MappingBuffer,
                present_bytes,
            )
            .unwrap(),
    );
    let old_weak = Arc::downgrade(&old_charge);
    core.frames[old_index].allocations.push(old_allocation);
    core.frames[old_index]
        .owned_allocations
        .push(old_allocation);
    core.frames[old_index]
        .resources
        .push(Arc::clone(&old_charge) as Arc<dyn Any + Send + Sync>);
    let present = phase(&mut core, old, GpuPhase::MappingPresent, Ok(()));
    core.seal(core.index(old).unwrap());
    assert_eq!(core.budget.snapshot().allocated_bytes, present_bytes);
    assert_eq!(core.budget.snapshot().reserved_bytes, unused_present_bytes);

    let next_bytes = NATIVE_MANAGED_LIMIT - present_bytes - unused_present_bytes;
    let mut next_plan = NativeAllocationPlan::new(frame(2));
    next_plan.add_existing(old_allocation);
    next_plan.add_existing(old_allocation);
    next_plan
        .add_new(ChargeKind::IsfScratch, next_bytes)
        .unwrap();
    let quote = core.budget.quote(&next_plan).unwrap();
    let reservation = core.budget.reserve(quote).unwrap();
    let next = core.begin(frame(2), reservation).unwrap();
    let next_index = core.index(next).unwrap();
    let next_allocation = core.budget.next_allocation_id().unwrap();
    assert_ne!(old_allocation, next_allocation);
    let next_charge = Arc::new(
        core.budget
            .bind_allocation(
                core.frames[next_index].reservation.as_mut().unwrap(),
                next_allocation,
                ChargeKind::IsfScratch,
                next_bytes,
            )
            .unwrap(),
    );
    core.frames[next_index]
        .allocations
        .extend([old_allocation, next_allocation]);
    core.frames[next_index]
        .owned_allocations
        .push(next_allocation);
    core.frames[next_index].resources.push(next_charge);
    core.frames[next_index].resources.push(old_charge);
    let isf = phase(&mut core, next, GpuPhase::IsfBatch, Ok(()));
    core.seal(core.index(next).unwrap());
    assert_eq!(core.budget.snapshot().managed_bytes(), NATIVE_MANAGED_LIMIT);
    assert_eq!(core.budget.snapshot().allocation_count, 2);
    let mut overflow = NativeAllocationPlan::new(frame(3));
    overflow.add_new(ChargeKind::CpuSnapshot, 1).unwrap();
    assert!(matches!(
        core.budget.quote(&overflow),
        Err(NativeGpuDecline::BudgetExceeded { live, requested: 1, limit })
            if live == NATIVE_MANAGED_LIMIT && limit == NATIVE_MANAGED_LIMIT
    ));

    core.driver.scope_ready = true;
    core.driver.completed = present.submission().sequence;
    core.progress().unwrap();
    let mut resolved_old = core.finish_record(core.index(old).unwrap()).unwrap();
    assert!(resolved_old
        .take_phase(present)
        .unwrap()
        .validation()
        .is_ok());
    assert!(
        old_weak.upgrade().is_some(),
        "the next submission still reads the alias"
    );
    assert!(core.budget.try_reap(old_allocation).unwrap().is_none());
    assert_eq!(
        core.budget.snapshot().managed_bytes(),
        NATIVE_MANAGED_LIMIT - unused_present_bytes,
        "only unused present reservation capacity is returned at old completion"
    );
    core.driver.completed = isf.submission().sequence;
    core.progress().unwrap();
    let mut resolved_next = core.finish_record(core.index(next).unwrap()).unwrap();
    assert!(resolved_next.take_phase(isf).unwrap().validation().is_ok());
    assert!(old_weak.upgrade().is_none());
    assert_eq!(core.budget.snapshot().managed_bytes(), 0);
    assert_eq!(
        core.budget.snapshot().high_water_bytes,
        NATIVE_MANAGED_LIMIT
    );
}

#[test]
fn native_gpu_g3_dropped_present_token_retires_only_its_own_record() {
    let mut core = scripted();
    let reservation = reserve(&mut core, 1, 32);
    let old = core.begin(frame(1), reservation).unwrap();
    let old_weak = retained(&mut core, old, 32);
    let present = phase(&mut core, old, GpuPhase::MappingPresent, Ok(()));
    core.seal(core.index(old).unwrap());
    let token = PendingGpuTransaction {
        id: old,
        marker: Arc::clone(&core.frames[core.index(old).unwrap()].marker),
        terminal: false,
    };
    let reservation = reserve(&mut core, 2, 64);
    let next = core.begin(frame(2), reservation).unwrap();
    let next_weak = retained(&mut core, next, 64);
    let isf = phase(&mut core, next, GpuPhase::IsfBatch, Ok(()));
    core.seal(core.index(next).unwrap());
    drop(token);
    core.driver.scope_ready = true;
    core.driver.completed = present.submission().sequence;
    assert_eq!(
        core.retirement().unwrap(),
        RetirementProgress::Pending {
            transactions: 1,
            allocations: 1
        }
    );
    assert!(old_weak.upgrade().is_none());
    assert!(next_weak.upgrade().is_some());
    assert_eq!(core.frames[0].id, next);
    assert_eq!(core.frames[0].marker.load(Ordering::Acquire), ACTIVE_TOKEN);
    assert_eq!(core.budget.snapshot().managed_bytes(), 64);
    core.driver.completed = isf.submission().sequence;
    core.progress().unwrap();
    let mut resolved = core.finish_record(core.index(next).unwrap()).unwrap();
    assert!(resolved.take_phase(isf).unwrap().validation().is_ok());
    assert!(next_weak.upgrade().is_none());
    core.stop();
    assert_eq!(core.retirement().unwrap(), RetirementProgress::Reaped);
}

struct G3PresentBuffer {
    buffer: wgpu::Buffer,
    _charge: AllocationCharge,
    _staging: AllocationCharge,
}

fn g3_real_present_phase(
    owner: &mut NativeGpuOwner,
    frame: NativeFrameId,
) -> (
    PendingPhaseReceipt,
    PendingGpuTransaction,
    std::sync::Weak<G3PresentBuffer>,
) {
    let mut plan = NativeAllocationPlan::new(frame);
    plan.add_new(ChargeKind::MappingBuffer, 16).unwrap();
    plan.add_new(ChargeKind::QueueUploadStaging, 16).unwrap();
    let quote = owner.quote(&plan).unwrap();
    let reservation = owner.reserve(quote).unwrap();
    let mut txn = owner.begin_frame(frame, reservation).unwrap();
    let mut phase = txn.begin_phase(GpuPhase::MappingPresent).unwrap();
    let charge = phase
        .bind_allocation(ChargeKind::MappingBuffer, 16)
        .unwrap();
    let staging = phase
        .bind_allocation(ChargeKind::QueueUploadStaging, 16)
        .unwrap();
    let allocations = [charge.id(), staging.id()];
    let buffer = Arc::new(G3PresentBuffer {
        buffer: phase.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("G3 retained present phase buffer"),
            size: 16,
            usage: wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }),
        _charge: charge,
        _staging: staging,
    });
    let weak = Arc::downgrade(&buffer);
    phase
        .retain_resource(Arc::clone(&buffer), &allocations)
        .unwrap();
    phase.mark_queue_write();
    phase.queue().write_buffer(&buffer.buffer, 0, &[0x51; 16]);
    drop(buffer);
    let encoded = phase.finish(Vec::new());
    let receipt = txn.submit_phase(encoded).unwrap();
    (receipt, txn.finish(), weak)
}

#[test]
fn native_gpu_g3_real_batch3_encodes_behind_an_unpolled_present_phase() {
    use crate::isf_native_gpu::tests as isf;
    use crate::native_gpu_result::NativeIsfCompletion;
    let mut fixture = isf::gpu();
    let (present, mut old_pending, old_weak) =
        g3_real_present_phase(&mut fixture.owner, isf::request(1));
    let old = old_pending.id;
    let old_allocations = fixture.owner.core.frames[fixture.owner.core.index(old).unwrap()]
        .owned_allocations
        .clone();
    let first_clock = fixture.runtime.native_frame_index();
    let (encoded, next_pending) = isf::encode(&mut fixture, 2, isf::identity_jobs(5, 3));
    let next = next_pending.id;
    assert_eq!(fixture.runtime.native_frame_index(), first_clock + 3);
    assert_eq!(fixture.owner.core.frames.len(), 2);
    let old_record = &fixture.owner.core.frames[fixture.owner.core.index(old).unwrap()];
    assert_eq!(old_record.stage, FrameRecordStage::PresentRetirement);
    assert!(
        !old_record.phases[0].complete,
        "no broker completion poll has run"
    );
    assert!(old_weak.upgrade().is_some());
    let next_record = &fixture.owner.core.frames[fixture.owner.core.index(next).unwrap()];
    assert_eq!(next_record.stage, FrameRecordStage::Sealed);
    assert_eq!(next_record.phases.len(), 1);
    assert_eq!(next_record.phases[0].phase, GpuPhase::IsfBatch);
    assert_eq!(
        next_record.phases[0].id.sequence,
        present.submission().sequence + 1
    );
    assert!(!next_record.phases[0].complete);
    assert!(!next_record.owned_allocations.is_empty());
    assert!(next_record
        .owned_allocations
        .iter()
        .all(|id| !old_allocations.contains(id)));
    let before = fixture.owner.budget_snapshot();
    let quote = fixture
        .owner
        .quote(&NativeAllocationPlan::new(isf::request(3)))
        .unwrap();
    let reservation = fixture.owner.reserve(quote).unwrap();
    assert!(matches!(
        fixture.owner.begin_frame(isf::request(3), reservation),
        Err(NativeGpuFault {
            kind: NativeGpuFaultKind::StateMismatch,
            ..
        })
    ));
    assert_eq!(fixture.runtime.native_frame_index(), first_clock + 3);
    assert_eq!(
        fixture.owner.budget_snapshot().managed_bytes(),
        before.managed_bytes()
    );

    let mut resolved_old = finish(&mut fixture.owner, &mut old_pending);
    assert!(resolved_old
        .take_phase(present)
        .unwrap()
        .validation()
        .is_ok());
    assert!(old_weak.upgrade().is_none());
    assert_eq!(fixture.owner.core.frames.len(), 1);
    assert_eq!(fixture.owner.core.frames[0].id, next);
    match isf::complete(&mut fixture, encoded, next_pending) {
        NativeIsfCompletion::Validated { layers, .. } => {
            assert_eq!(layers.len(), 3);
            drop(layers);
        }
        _ => panic!("real next-frame Batch3 did not validate"),
    }
    assert_eq!(fixture.runtime.native_frame_index(), first_clock + 3);
    drop(fixture.runtime);
    isf::stop(&mut fixture.owner);
}

#[test]
fn native_gpu_g3_serial_cpu_admission_and_unused_refund_ignore_only_present_retirement() {
    let mut owner = headless(77);
    let (present, mut old_pending, old_weak) = g3_real_present_phase(&mut owner, frame(1));
    let retained_bytes = owner.budget_snapshot().managed_bytes();
    let mut plan = NativeAllocationPlan::new(frame(2));
    plan.add_new(ChargeKind::CpuSnapshot, 64).unwrap();
    let quote = owner.quote(&plan).unwrap();
    let reservation = owner.reserve(quote).unwrap();
    owner
        .discard_unsubmitted_reservation(reservation)
        .unwrap_or_else(|(fault, _)| panic!("unused refund behind present: {fault:?}"));
    assert_eq!(owner.budget_snapshot().managed_bytes(), retained_bytes);
    let quote = owner.quote(&plan).unwrap();
    let reservation = owner.reserve(quote).unwrap();
    owner
        .admit_serial_cpu_plan(reservation, &plan)
        .unwrap_or_else(|(fault, _)| panic!("serial admission behind present: {fault:?}"));
    assert_eq!(owner.budget_snapshot().managed_bytes(), retained_bytes);
    assert!(old_weak.upgrade().is_some());

    let quote = owner.quote(&NativeAllocationPlan::new(frame(3))).unwrap();
    let reservation = owner.reserve(quote).unwrap();
    let mut txn = owner.begin_frame(frame(3), reservation).unwrap();
    let phase = txn.begin_phase(GpuPhase::IsfBatch).unwrap();
    let encoded = phase.finish(Vec::new());
    let isf = txn.submit_phase(encoded).unwrap();
    let mut next_pending = txn.finish();
    let quote = owner.quote(&plan).unwrap();
    let reservation = owner.reserve(quote).unwrap();
    let (fault, refund) = match owner.discard_unsubmitted_reservation(reservation) {
        Err(rejected) => rejected,
        Ok(()) => panic!("logical pending frame must still block unused refund"),
    };
    assert_eq!(fault.kind, NativeGpuFaultKind::StateMismatch);
    let quote = owner.quote(&plan).unwrap();
    let reservation = owner.reserve(quote).unwrap();
    let (fault, envelope) = match owner.admit_serial_cpu_plan(reservation, &plan) {
        Err(rejected) => rejected,
        Ok(()) => panic!("logical pending frame must still block serial admission"),
    };
    assert_eq!(fault.kind, NativeGpuFaultKind::StateMismatch);
    assert_eq!(
        owner.budget_snapshot().managed_bytes(),
        retained_bytes + 128
    );

    let mut resolved_next = finish(&mut owner, &mut next_pending);
    assert!(resolved_next.take_phase(isf).unwrap().validation().is_ok());
    owner
        .discard_unsubmitted_reservation(refund)
        .unwrap_or_else(|(fault, _)| panic!("refund after logical completion: {fault:?}"));
    owner
        .admit_serial_cpu_plan(envelope, &plan)
        .unwrap_or_else(|(fault, _)| {
            panic!("serial admission after logical completion: {fault:?}")
        });
    assert_eq!(owner.budget_snapshot().managed_bytes(), retained_bytes);
    assert!(old_weak.upgrade().is_some());
    let mut resolved_old = finish(&mut owner, &mut old_pending);
    assert!(resolved_old
        .take_phase(present)
        .unwrap()
        .validation()
        .is_ok());
    assert!(old_weak.upgrade().is_none());
    owner.begin_stop();
    assert_eq!(owner.poll_retirement().unwrap(), RetirementProgress::Reaped);
}

fn g4_replacement(generation: u64) -> BrokerCore<ScriptedGpuDriver> {
    BrokerCore::new(
        ScriptedGpuDriver::default(),
        GpuContextId {
            device_generation: generation,
            ..context()
        },
    )
}

#[test]
fn native_gpu_g4_loss_or_poison_rearm_retains_both_g3_records_and_old_reservations() {
    for kind in [
        NativeGpuFaultKind::DeviceLost,
        NativeGpuFaultKind::ScopeDrain,
    ] {
        let mut core = scripted();
        let mut plan = NativeAllocationPlan::new(frame(1));
        plan.add_new(ChargeKind::IsfScratch, 32).unwrap();
        plan.add_new(ChargeKind::CpuSnapshot, 16).unwrap();
        let reservation = core
            .budget
            .reserve(core.budget.quote(&plan).unwrap())
            .unwrap();
        let present_id = core.begin(frame(1), reservation).unwrap();
        let present_owner = retained(&mut core, present_id, 32);
        phase(&mut core, present_id, GpuPhase::MappingPresent, Ok(()));
        core.seal(core.index(present_id).unwrap());
        let reservation = reserve(&mut core, 2, 64);
        let next_id = core.begin(frame(2), reservation).unwrap();
        let next_owner = retained(&mut core, next_id, 64);
        phase(&mut core, next_id, GpuPhase::IsfBatch, Ok(()));
        core.seal(core.index(next_id).unwrap());
        core.driver.failure = Some(DriverFailure {
            kind,
            message: "old generation failure".into(),
        });
        assert_eq!(core.progress().unwrap_err().kind, kind);
        assert_eq!(core.budget.snapshot().managed_bytes(), 112);
        let old_context = core.context;
        core.rearm_with_replacement(g4_replacement(old_context.device_generation + 7))
            .unwrap_or_else(|(fault, _)| panic!("retained re-arm: {fault:?}"));
        assert_eq!(core.state, GpuOwnerState::Active);
        assert!(core.frames.is_empty());
        assert_eq!(core.retained_generations.len(), 1);
        assert_eq!(core.retained_generations[0].frames.len(), 2);
        assert!(present_owner.upgrade().is_some());
        assert!(next_owner.upgrade().is_some());
        assert_eq!(core.budget.snapshot().allocated_bytes, 96);
        assert_eq!(core.budget.snapshot().reserved_bytes, 16);
        assert_eq!(
            core.budget
                .snapshot()
                .for_kind(ChargeKind::IsfScratch)
                .quarantined_bytes,
            96
        );
        assert_eq!(
            core.index(present_id).unwrap_err().kind,
            NativeGpuFaultKind::IdentityMismatch
        );
        assert_eq!(
            core.retained_generations[0].frames[0]
                .marker
                .load(Ordering::Acquire),
            CANCELLED_TOKEN
        );

        core.retained_generations[0]._driver.completed = u64::MAX;
        core.retained_generations[0]._driver.scope_ready = true;
        let old_polls = core.retained_generations[0]._driver.polls;
        let reservation = reserve(&mut core, 3, 32);
        let current = core.begin(frame(3), reservation).unwrap();
        let current_owner = retained(&mut core, current, 32);
        let receipt = phase(&mut core, current, GpuPhase::IsfBatch, Ok(()));
        assert_eq!(receipt.submission().context, core.context);
        assert_eq!(receipt.submission().sequence, 1);
        core.driver.completed = 1;
        core.driver.scope_ready = true;
        core.progress().unwrap();
        let mut resolved = core.finish_record(core.index(current).unwrap()).unwrap();
        assert!(resolved.take_phase(receipt).unwrap().validation().is_ok());
        assert!(current_owner.upgrade().is_none());
        assert_eq!(core.budget.snapshot().managed_bytes(), 112);
        assert_eq!(
            core.retained_generations[0]._driver.polls, old_polls,
            "old failed driver must not resume execution or prove a new success"
        );
        core.stop();
        assert_eq!(
            core.retirement().unwrap(),
            RetirementProgress::Pending {
                transactions: 2,
                allocations: 2
            }
        );
        assert!(present_owner.upgrade().is_some());
        assert!(next_owner.upgrade().is_some());
    }
}

#[test]
fn native_gpu_g4_drained_generation_keeps_external_owner_until_its_last_release() {
    let mut core = scripted();
    let reservation = reserve(&mut core, 1, 32);
    let id = core.begin(frame(1), reservation).unwrap();
    let weak = retained(&mut core, id, 32);
    let external = weak.upgrade().unwrap();
    let allocation = external.charge.id();
    let receipt = phase(&mut core, id, GpuPhase::IsfBatch, Ok(()));
    core.driver.completed = receipt.submission().sequence;
    core.driver.scope_ready = true;
    core.progress().unwrap();
    let mut resolved = core.finish_record(core.index(id).unwrap()).unwrap();
    resolved
        .take_phase(receipt)
        .unwrap()
        .into_validation()
        .unwrap();
    core.stop();
    assert_eq!(
        core.retirement().unwrap(),
        RetirementProgress::Pending {
            transactions: 0,
            allocations: 1
        }
    );
    core.rearm_with_replacement(g4_replacement(context().device_generation + 9))
        .unwrap_or_else(|(fault, _)| panic!("drained re-arm: {fault:?}"));
    assert_eq!(core.retained_generations.len(), 1);
    assert_eq!(core.budget.snapshot().managed_bytes(), 32);
    let mut alias = NativeAllocationPlan::new(frame(2));
    alias.add_existing(allocation);
    assert_eq!(
        core.budget.quote(&alias).unwrap_err(),
        NativeGpuDecline::QuoteExpired
    );
    core.progress().unwrap();
    assert!(weak.upgrade().is_some());
    drop(external);
    core.progress().unwrap();
    assert!(weak.upgrade().is_none());
    assert!(core.retained_generations.is_empty());
    assert_eq!(core.budget.snapshot().managed_bytes(), 0);
    core.stop();
    assert_eq!(core.retirement().unwrap(), RetirementProgress::Reaped);
}

#[test]
fn native_gpu_g4_rearm_rejects_active_pending_foreign_and_used_bootstraps_intact() {
    let mut core = scripted();
    let before = core.budget.snapshot();
    let (fault, replacement) = core
        .rearm_with_replacement(g4_replacement(9))
        .err()
        .unwrap();
    assert_eq!(fault.kind, NativeGpuFaultKind::StateMismatch);
    assert!(replacement.is_pristine());
    assert_eq!(core.budget.snapshot(), before);

    let reservation = reserve(&mut core, 1, 0);
    let id = core.begin(frame(1), reservation).unwrap();
    phase(&mut core, id, GpuPhase::Initialize, Ok(()));
    core.stop();
    let before = core.budget.snapshot();
    let (fault, replacement) = core.rearm_with_replacement(replacement).err().unwrap();
    assert_eq!(fault.kind, NativeGpuFaultKind::StateMismatch);
    assert!(replacement.is_pristine());
    assert_eq!(core.budget.snapshot(), before);
    core.driver.completed = 1;
    core.driver.scope_ready = true;
    assert_eq!(core.retirement().unwrap(), RetirementProgress::Reaped);

    let mut used = replacement;
    let reservation = reserve(&mut used, 1, 0);
    used.budget.cancel_reservation(reservation).unwrap();
    let used_snapshot = used.budget.snapshot();
    let (fault, used) = core.rearm_with_replacement(used).err().unwrap();
    assert_eq!(fault.kind, NativeGpuFaultKind::StateMismatch);
    assert_eq!(used.budget.snapshot(), used_snapshot);
    let before = core.budget.snapshot();
    for replacement_context in [
        GpuContextId {
            owner_id: context().owner_id + 1,
            device_generation: 9,
        },
        context(),
    ] {
        let replacement = BrokerCore::new(ScriptedGpuDriver::default(), replacement_context);
        let (fault, replacement) = core.rearm_with_replacement(replacement).err().unwrap();
        assert_eq!(fault.kind, NativeGpuFaultKind::IdentityMismatch);
        assert!(replacement.is_pristine());
        assert_eq!(replacement.context, replacement_context);
        assert_eq!(core.budget.snapshot(), before);
    }
}

#[test]
fn native_gpu_g4_retained_generation_bound_includes_zero_byte_failed_scopes() {
    let mut core = scripted();
    for sequence in 0..=MAX_RETAINED_DEVICE_GENERATIONS {
        let reservation = reserve(&mut core, sequence as u64 + 1, 0);
        let id = core.begin(frame(sequence as u64 + 1), reservation).unwrap();
        phase(&mut core, id, GpuPhase::Initialize, Ok(()));
        core.driver.failure = Some(DriverFailure {
            kind: NativeGpuFaultKind::ScopeDrain,
            message: "unproved zero-byte scope".into(),
        });
        core.progress().unwrap_err();
        let next = g4_replacement(core.context.device_generation + 1);
        if sequence == MAX_RETAINED_DEVICE_GENERATIONS {
            let before = core.budget.snapshot();
            let (fault, next) = core.rearm_with_replacement(next).err().unwrap();
            assert_eq!(fault.kind, NativeGpuFaultKind::StateMismatch);
            assert!(next.is_pristine());
            assert_eq!(
                core.retained_generations.len(),
                MAX_RETAINED_DEVICE_GENERATIONS
            );
            assert_eq!(core.frames.len(), 1);
            assert_eq!(core.budget.snapshot(), before);
        } else {
            core.rearm_with_replacement(next)
                .unwrap_or_else(|(fault, _)| panic!("bounded re-arm: {fault:?}"));
            assert_eq!(core.retained_generations.len(), sequence + 1);
        }
    }
    assert_eq!(core.budget.snapshot().managed_bytes(), 0);
    assert_eq!(
        core.budget.snapshot().reservation_count,
        MAX_RETAINED_DEVICE_GENERATIONS + 1
    );
    core.stop();
    assert!(
        core.retirement().is_err(),
        "zero-byte unresolved scopes are not a successful Stop"
    );
}

struct G4LifetimeDriver {
    _live: Arc<()>,
}
impl CompletionDriver for G4LifetimeDriver {
    type Ticket = ();
    type Scopes = ();
    fn poll(&mut self) -> Result<(), DriverFailure> {
        Ok(())
    }
    fn complete(&self, _: &()) -> bool {
        false
    }
    fn scopes(&mut self, _: &mut ()) -> Option<Result<(), DriverFailure>> {
        None
    }
}

#[test]
fn native_gpu_g4_owner_drop_retains_ledger_and_drivers_without_wait_or_false_reap() {
    let live = Arc::new(());
    let clean = BrokerCore::new(
        G4LifetimeDriver {
            _live: Arc::clone(&live),
        },
        context(),
    );
    let weak = Arc::downgrade(&live);
    drop(live);
    drop(OwnedBrokerCore::new(clean));
    assert!(weak.upgrade().is_none(), "an empty owner releases normally");

    let live = Arc::new(());
    let weak = Arc::downgrade(&live);
    let mut core = BrokerCore::new(G4LifetimeDriver { _live: live }, context());
    let mut plan = NativeAllocationPlan::new(frame(1));
    plan.add_new(ChargeKind::CpuLastValid, 16).unwrap();
    let mut reservation = core
        .budget
        .reserve(core.budget.quote(&plan).unwrap())
        .unwrap();
    let id = core.budget.next_allocation_id().unwrap();
    let external_charge = core
        .budget
        .bind_allocation(&mut reservation, id, ChargeKind::CpuLastValid, 16)
        .unwrap();
    core.budget.cancel_reservation(reservation).unwrap();
    assert!(
        !core.can_release(),
        "a final external charge is part of retirement"
    );
    drop(OwnedBrokerCore::new(core));
    assert!(weak.upgrade().is_some());
    drop(external_charge);
    assert!(
        weak.upgrade().is_some(),
        "abandoned ownership cannot invent a reap after a later external drop"
    );

    let old_live = Arc::new(());
    let old_weak = Arc::downgrade(&old_live);
    let mut core = BrokerCore::new(G4LifetimeDriver { _live: old_live }, context());
    let plan = NativeAllocationPlan::new(frame(2));
    let reservation = core
        .budget
        .reserve(core.budget.quote(&plan).unwrap())
        .unwrap();
    let txn = core.begin(frame(2), reservation).unwrap();
    core.open(txn, GpuPhase::Initialize).unwrap();
    let submission = core.submission_id(frame(2), GpuPhase::Initialize).unwrap();
    core.register(txn, submission, GpuPhase::Initialize, (), ())
        .unwrap();
    core.state = GpuOwnerState::DeviceLostRetained;
    core.budget.quarantine();
    let new_live = Arc::new(());
    let new_weak = Arc::downgrade(&new_live);
    let replacement = BrokerCore::new(
        G4LifetimeDriver { _live: new_live },
        GpuContextId {
            device_generation: 9,
            ..context()
        },
    );
    core.rearm_with_replacement(replacement)
        .unwrap_or_else(|(fault, _)| panic!("owner retention test: {fault:?}"));
    assert_eq!(core.retained_generations.len(), 1);
    assert!(
        !core.can_release(),
        "an unresolved zero-byte phase still owns its driver and scopes"
    );
    drop(OwnedBrokerCore::new(core));
    assert!(old_weak.upgrade().is_some());
    assert!(new_weak.upgrade().is_some());
}

#[test]
fn native_gpu_g4_real_rearm_uses_original_ledger_and_drains_new_device_independently() {
    let factory = NativeGpuFactory::new();
    let mut owner = factory
        .create_headless(11, NativeGpuRequirements::default())
        .unwrap();
    let (receipt, mut pending, weak) = g3_real_present_phase(&mut owner, frame(1));
    let external = weak.upgrade().unwrap();
    let old_context = owner.context().id();
    finish(&mut owner, &mut pending)
        .take_phase(receipt)
        .unwrap()
        .into_validation()
        .unwrap();
    owner.begin_stop();
    assert_eq!(
        owner.poll_retirement().unwrap(),
        RetirementProgress::Pending {
            transactions: 0,
            allocations: 2
        }
    );
    let old_bytes = owner.budget_snapshot().managed_bytes();
    let replacement = factory
        .create_headless(11, NativeGpuRequirements::default())
        .unwrap();
    let next_context = replacement.context().id();
    owner
        .rearm_with_replacement(replacement)
        .unwrap_or_else(|(fault, _)| panic!("real re-arm: {fault:?}"));
    assert_eq!(owner.context().id(), next_context);
    assert!(next_context.device_generation > old_context.device_generation);
    assert_eq!(owner.budget_snapshot().managed_bytes(), old_bytes);
    assert_eq!(owner.core.retained_generations.len(), 1);
    let (receipt, mut pending, current_weak) = g3_real_present_phase(&mut owner, frame(2));
    assert_eq!(owner.budget_snapshot().managed_bytes(), old_bytes * 2);
    assert_ne!(
        external._charge.id(),
        current_weak.upgrade().unwrap()._charge.id()
    );
    finish(&mut owner, &mut pending)
        .take_phase(receipt)
        .unwrap()
        .into_validation()
        .unwrap();
    assert!(current_weak.upgrade().is_none());
    assert!(weak.upgrade().is_some());
    assert_eq!(owner.budget_snapshot().managed_bytes(), old_bytes);
    drop(external);
    owner.poll_retirement().unwrap();
    assert!(weak.upgrade().is_none());
    assert!(owner.core.retained_generations.is_empty());
    assert_eq!(owner.budget_snapshot().managed_bytes(), 0);
    owner.begin_stop();
    assert_eq!(owner.poll_retirement().unwrap(), RetirementProgress::Reaped);
}

#[test]
fn native_gpu_g4_real_rearm_rejects_foreign_exposed_quoted_and_initialized_replacements() {
    let factory = NativeGpuFactory::new();
    let mut owner = factory
        .create_headless(11, NativeGpuRequirements::default())
        .unwrap();
    owner.begin_stop();
    assert_eq!(owner.poll_retirement().unwrap(), RetirementProgress::Reaped);
    let original = owner.context().id();
    let snapshot = owner.budget_snapshot();

    let foreign = NativeGpuFactory::new()
        .create_headless(11, NativeGpuRequirements::default())
        .unwrap();
    let foreign_context = foreign.context().id();
    let (fault, foreign) = owner.rearm_with_replacement(foreign).err().unwrap();
    assert_eq!(fault.kind, NativeGpuFaultKind::IdentityMismatch);
    assert_eq!(foreign.context().id(), foreign_context);
    drop(foreign);

    let wrong_owner = factory
        .create_headless(12, NativeGpuRequirements::default())
        .unwrap();
    let (fault, wrong_owner) = owner.rearm_with_replacement(wrong_owner).err().unwrap();
    assert_eq!(fault.kind, NativeGpuFaultKind::IdentityMismatch);
    drop(wrong_owner);

    let replacement = factory
        .create_headless(11, NativeGpuRequirements::default())
        .unwrap();
    let exposed = Arc::clone(replacement.context());
    let (fault, replacement) = owner.rearm_with_replacement(replacement).err().unwrap();
    assert_eq!(fault.kind, NativeGpuFaultKind::StateMismatch);
    assert!(Arc::ptr_eq(&exposed, replacement.context()));
    drop(exposed);
    let quote = replacement
        .quote(&NativeAllocationPlan::new(frame(1)))
        .unwrap();
    let (fault, mut replacement) = owner.rearm_with_replacement(replacement).err().unwrap();
    assert_eq!(fault.kind, NativeGpuFaultKind::StateMismatch);
    drop(quote);
    replacement.initialize(|_, _| ()).unwrap();
    let (fault, mut replacement) = owner.rearm_with_replacement(replacement).err().unwrap();
    assert_eq!(fault.kind, NativeGpuFaultKind::StateMismatch);
    replacement.begin_stop();
    assert_eq!(
        replacement.poll_retirement().unwrap(),
        RetirementProgress::Reaped
    );
    drop(replacement);
    assert_eq!(owner.context().id(), original);
    assert_eq!(owner.budget_snapshot(), snapshot);

    let replacement = factory
        .create_headless(11, NativeGpuRequirements::default())
        .unwrap();
    owner
        .rearm_with_replacement(replacement)
        .unwrap_or_else(|(fault, _)| panic!("fresh replacement after rejections: {fault:?}"));
    assert!(owner.core.retained_generations.is_empty());
    owner.begin_stop();
    assert_eq!(owner.poll_retirement().unwrap(), RetirementProgress::Reaped);
}

fn g4_finish_reserved_phase(
    owner: &mut NativeGpuOwner,
    identity: NativeFrameId,
    reservation: FrameReservation,
) {
    let mut transaction = owner.begin_frame(identity, reservation).unwrap();
    let phase = transaction.begin_phase(GpuPhase::IsfBatch).unwrap();
    let encoder = phase
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let encoded = phase.finish(vec![encoder.finish()]);
    let receipt = transaction.submit_phase(encoded).unwrap();
    let mut pending = transaction.finish();
    finish(owner, &mut pending)
        .take_phase(receipt)
        .unwrap()
        .into_validation()
        .unwrap();
}

#[test]
fn native_gpu_g4_real_late_cancel_of_retained_pending_keeps_new_owner_and_quote_healthy() {
    for kind in [
        NativeGpuFaultKind::DeviceLost,
        NativeGpuFaultKind::ScopeDrain,
    ] {
        let factory = NativeGpuFactory::new();
        let mut owner = factory
            .create_headless(11, NativeGpuRequirements::default())
            .unwrap();
        let (old_receipt, old_pending, old_weak) = g3_real_present_phase(&mut owner, frame(1));
        let old_id = old_pending.id;
        let old_marker = Arc::clone(&old_pending.marker);
        if kind == NativeGpuFaultKind::DeviceLost {
            owner.context.lost.store(true, Ordering::Release);
        } else {
            owner.context.uncaptured.store(true, Ordering::Release);
        }
        let replacement = factory
            .create_headless(11, NativeGpuRequirements::default())
            .unwrap();
        owner
            .rearm_with_replacement(replacement)
            .unwrap_or_else(|(fault, _)| panic!("late-cancel re-arm: {fault:?}"));
        let current_context = owner.context().id();
        let old_snapshot = owner.budget_snapshot();
        assert_eq!(old_snapshot.managed_bytes(), 32);
        assert_eq!(old_snapshot.allocation_count, 2);
        assert_eq!(owner.core.retained_generations.len(), 1);
        let old_record = &owner.core.retained_generations[0].frames[0];
        let old_allocations = old_record.owned_allocations.clone();
        let old_resource_count = old_record.resources.len();
        assert!(old_record.reservation.is_some());
        assert_eq!(old_record.phases[0].id, old_receipt.submission());
        assert!(!old_record.phases[0].complete);
        assert!(old_record.phases[0].scope_result.is_none());

        let (receipt, mut current_pending, current_weak) =
            g3_real_present_phase(&mut owner, frame(2));
        assert_eq!(
            receipt.submission().sequence,
            old_receipt.submission().sequence
        );
        assert_ne!(
            receipt.submission().context,
            old_receipt.submission().context
        );
        let plan = NativeAllocationPlan::new(frame(3));
        let quote = owner.quote(&plan).unwrap();
        let before_cancel = owner.budget_snapshot();
        let retiring = GpuCompletionPort::cancel_and_retire(&mut owner, old_pending);
        assert_eq!(retiring.id(), old_id);
        assert_eq!(owner.context().id(), current_context);
        assert_eq!(owner.state(), GpuOwnerState::Active);
        assert_eq!(owner.budget_snapshot(), before_cancel);
        assert_eq!(old_marker.load(Ordering::Acquire), CANCELLED_TOKEN);
        assert_eq!(current_pending.marker.load(Ordering::Acquire), ACTIVE_TOKEN);
        assert_eq!(owner.core.retained_generations.len(), 1);
        assert_eq!(owner.core.retained_generations[0].frames.len(), 1);
        let old_record = &owner.core.retained_generations[0].frames[0];
        assert_eq!(old_record.id, old_id);
        assert_eq!(old_record.owned_allocations, old_allocations);
        assert_eq!(old_record.resources.len(), old_resource_count);
        assert!(old_record.reservation.is_some());
        assert!(!old_record.phases[0].complete);
        assert!(old_record.phases[0].scope_result.is_none());
        assert!(old_weak.upgrade().is_some());

        let reservation = owner
            .reserve(quote)
            .expect("old cleanup must not expire new quote");
        finish(&mut owner, &mut current_pending)
            .take_phase(receipt)
            .unwrap()
            .into_validation()
            .unwrap();
        assert!(current_weak.upgrade().is_none());
        g4_finish_reserved_phase(&mut owner, frame(3), reservation);
        assert_eq!(owner.state(), GpuOwnerState::Active);
        assert_eq!(
            owner.budget_snapshot().managed_bytes(),
            old_snapshot.managed_bytes()
        );
        assert_eq!(
            owner.budget_snapshot().allocation_count,
            old_snapshot.allocation_count
        );
        let old_record = &owner.core.retained_generations[0].frames[0];
        assert_eq!(old_record.phases[0].id, old_receipt.submission());
        assert!(!old_record.phases[0].complete);
        assert!(old_record.phases[0].scope_result.is_none());
        assert_eq!(old_marker.load(Ordering::Acquire), CANCELLED_TOKEN);
        owner.begin_stop();
        assert_eq!(
            owner.poll_retirement().unwrap(),
            RetirementProgress::Pending {
                transactions: 1,
                allocations: 2,
            }
        );
        assert!(old_weak.upgrade().is_some());
        assert_eq!(owner.budget_snapshot().managed_bytes(), 32);
    }
}

#[test]
fn native_gpu_g4_real_late_cancel_after_healthy_stop_does_not_reap_new_generation() {
    for retain_external in [false, true] {
        let factory = NativeGpuFactory::new();
        let mut owner = factory
            .create_headless(11, NativeGpuRequirements::default())
            .unwrap();
        let (_receipt, old_pending, old_weak) = g3_real_present_phase(&mut owner, frame(1));
        let old_id = old_pending.id;
        let old_marker = Arc::clone(&old_pending.marker);
        let external = retain_external.then(|| old_weak.upgrade().unwrap());
        owner.begin_stop();
        let started = Instant::now();
        let progress = loop {
            let progress = owner.poll_retirement().unwrap();
            if owner.core.frames.is_empty() {
                break progress;
            }
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "old Stop did not drain"
            );
            std::thread::yield_now();
        };
        assert_eq!(
            progress,
            if retain_external {
                RetirementProgress::Pending {
                    transactions: 0,
                    allocations: 2,
                }
            } else {
                RetirementProgress::Reaped
            }
        );
        assert_eq!(old_marker.load(Ordering::Acquire), TERMINAL_TOKEN);
        assert!(
            !old_pending.terminal,
            "the caller never polled this affine token"
        );
        let replacement = factory
            .create_headless(11, NativeGpuRequirements::default())
            .unwrap();
        owner
            .rearm_with_replacement(replacement)
            .unwrap_or_else(|(fault, _)| panic!("drained late-cancel re-arm: {fault:?}"));
        assert_eq!(
            owner.core.retained_generations.len(),
            usize::from(retain_external)
        );
        assert!(owner
            .core
            .retained_generations
            .iter()
            .all(|generation| generation.frames.is_empty()));
        let (receipt, mut current_pending, current_weak) =
            g3_real_present_phase(&mut owner, frame(2));
        let quote = owner.quote(&NativeAllocationPlan::new(frame(3))).unwrap();
        let before_cancel = owner.budget_snapshot();
        let retiring = owner.cancel_and_retire(old_pending);
        assert_eq!(retiring.id(), old_id);
        assert_eq!(owner.state(), GpuOwnerState::Active);
        assert_eq!(owner.budget_snapshot(), before_cancel);
        assert_eq!(old_marker.load(Ordering::Acquire), TERMINAL_TOKEN);
        assert_eq!(current_pending.marker.load(Ordering::Acquire), ACTIVE_TOKEN);
        let reservation = owner
            .reserve(quote)
            .expect("terminal cleanup must preserve new quote");
        finish(&mut owner, &mut current_pending)
            .take_phase(receipt)
            .unwrap()
            .into_validation()
            .unwrap();
        assert!(current_weak.upgrade().is_none());
        g4_finish_reserved_phase(&mut owner, frame(3), reservation);
        assert_eq!(owner.state(), GpuOwnerState::Active);
        drop(external);
        assert_eq!(
            owner.poll_retirement().unwrap(),
            RetirementProgress::Pending {
                transactions: 0,
                allocations: 0,
            }
        );
        assert!(old_weak.upgrade().is_none());
        assert!(owner.core.retained_generations.is_empty());
        assert_eq!(owner.budget_snapshot().managed_bytes(), 0);
        owner.begin_stop();
        assert_eq!(owner.poll_retirement().unwrap(), RetirementProgress::Reaped);
    }
}

#[test]
fn native_gpu_g4_late_pending_cleanup_rejects_foreign_terminal_and_unregistered_aliases() {
    let mut core = scripted();
    let reservation = reserve(&mut core, 1, 32);
    let old_id = core.begin(frame(1), reservation).unwrap();
    let old_weak = retained(&mut core, old_id, 32);
    phase(&mut core, old_id, GpuPhase::IsfBatch, Ok(()));
    let old_marker = Arc::clone(&core.frames[0].marker);
    let authentic = PendingGpuTransaction {
        id: old_id,
        marker: Arc::clone(&old_marker),
        terminal: false,
    };
    core.driver.failure = Some(DriverFailure {
        kind: NativeGpuFaultKind::DeviceLost,
        message: "old pending session".into(),
    });
    core.progress().unwrap_err();
    core.rearm_with_replacement(g4_replacement(context().device_generation + 4))
        .unwrap_or_else(|(fault, _)| panic!("identity re-arm: {fault:?}"));

    // Numeric equality, including a genuine terminal marker, cannot substitute
    // for the originating session capability.
    let mut foreign = scripted();
    let reservation = reserve(&mut foreign, 1, 0);
    let foreign_id = foreign.begin(frame(1), reservation).unwrap();
    assert_eq!(foreign_id, old_id);
    let foreign_pending = PendingGpuTransaction {
        id: foreign_id,
        marker: Arc::clone(&foreign.frames[0].marker),
        terminal: false,
    };
    foreign.finish_record(0).unwrap();
    assert_eq!(
        foreign_pending.marker.load(Ordering::Acquire),
        TERMINAL_TOKEN
    );
    let unregistered = RetirementId {
        sequence: old_id.sequence + 1,
        ..old_id
    };
    let aliases = [
        foreign_pending,
        PendingGpuTransaction {
            id: unregistered,
            marker: Arc::clone(&old_marker),
            terminal: false,
        },
        PendingGpuTransaction {
            id: old_id,
            marker: Arc::new(TransactionMarker {
                session: Arc::clone(&core.session),
                id: old_id,
                state: AtomicU8::new(CANCELLED_TOKEN),
            }),
            terminal: true,
        },
        PendingGpuTransaction {
            id: unregistered,
            marker: Arc::new(TransactionMarker {
                session: Arc::clone(&core.session),
                id: unregistered,
                state: AtomicU8::new(ACTIVE_TOKEN),
            }),
            terminal: false,
        },
    ];
    let snapshot = core.budget.snapshot();
    let quote = core
        .budget
        .quote(&NativeAllocationPlan::new(frame(2)))
        .unwrap();
    for alias in aliases {
        assert_eq!(
            core.retire_pending(&alias).unwrap_err().kind,
            NativeGpuFaultKind::IdentityMismatch
        );
        assert_eq!(core.state, GpuOwnerState::Active);
        assert_eq!(core.budget.snapshot(), snapshot);
        assert_eq!(core.driver.polls, 0);
        assert_eq!(core.retained_generations.len(), 1);
        assert_eq!(core.retained_generations[0].frames.len(), 1);
        assert!(old_weak.upgrade().is_some());
    }
    core.retire_pending(&authentic).unwrap();
    assert_eq!(old_marker.load(Ordering::Acquire), CANCELLED_TOKEN);
    let reservation = core.budget.reserve(quote).unwrap();
    let current = core.begin(frame(2), reservation).unwrap();
    let receipt = phase(&mut core, current, GpuPhase::IsfBatch, Ok(()));
    core.driver.completed = receipt.submission().sequence;
    core.driver.scope_ready = true;
    core.progress().unwrap();
    core.finish_record(core.index(current).unwrap())
        .unwrap()
        .take_phase(receipt)
        .unwrap()
        .into_validation()
        .unwrap();
    assert_eq!(core.state, GpuOwnerState::Active);
    assert_eq!(core.budget.snapshot().managed_bytes(), 32);
    assert!(old_weak.upgrade().is_some());
}
