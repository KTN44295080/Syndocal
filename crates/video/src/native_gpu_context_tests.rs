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
