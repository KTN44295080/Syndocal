//! One prepared renderer invocation, consumed by one native or brokered CPU path.
//!
//! The continuation borrows the renderer's only ISF runtime. Inputs, DATE, audio,
//! keys and preparation faults are frozen before planning; no executor is called
//! while preparing or querying the prospective plan. Accepted work cannot be
//! converted to a CPU retry after its affine batch has been passed to execution.
#[cfg(test)]
use super::PreparedNativeCpuInvocation;
use super::{
    NativeBatchAdmission, NativeBatchRestrictions, NativeCpuRenderFailure,
    NativeProspectiveIdentityError, PreparedNativeBatch, PreparedNativeInputs, PreparedNativeLayer,
};
use crate::isf_chain_execution::PreparedChain;
use crate::isf_runtime::{IsfGpuRuntime, StackBatch};
use crate::native_gpu_budget::{BudgetQuote, FrameReservation, NativeAllocationPlan};
use crate::native_gpu_context::NativeGpuOwner;
use crate::native_gpu_contract::{
    GpuContextId, GpuPhase, LayerResultIdentity, NativeFrameId, NativeGpuDecline, NativeGpuFault,
    NativeGpuFaultKind, RetirementId,
};
use crate::native_gpu_result::NativeCpuResults;
use crate::{
    check_runtime_cancellation, resolved_chain_executes, ResolvedVideoEffectChain, VideoFrame,
    VideoFrameProvider, VideoPreviewRenderer, VideoRenderCancellation, VideoRuntimeError,
};
use std::{borrow::Cow, sync::Arc};

/// Decoded input and its authored chain. This is not an executed history entry.
pub(crate) struct NativeLayerPreparation {
    pub(crate) input: Arc<VideoFrame>,
    pub(crate) chain: ResolvedVideoEffectChain,
    pub(crate) entry: crate::native_gpu_contract::HistoryEntryId,
    pub(crate) source_generation: u64,
    pub(crate) content_sequence: u64,
}

#[derive(Debug)]
pub(crate) enum NativePreparationFailure {
    Runtime(VideoRuntimeError),
    Fault(NativeGpuFault),
}

impl std::fmt::Display for NativePreparationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runtime(error) => {
                write!(formatter, "native renderer preparation failed: {error:?}")
            }
            Self::Fault(fault) => write!(
                formatter,
                "native GPU preparation failed ({:?} during {:?}, context {:?}, frame {:?}): {}",
                fault.kind, fault.phase, fault.context, fault.frame, fault.message,
            ),
        }
    }
}

/// Non-Clone and tied to the renderer's one runtime for the complete preflight.
pub(crate) struct NativePreparedContinuation<'runtime> {
    runtime: &'runtime mut IsfGpuRuntime,
    inputs: PreparedNativeInputs,
}

pub(crate) enum NativeContinuationAdmission<'runtime> {
    Accepted(NativeAdmittedContinuation<'runtime>),
    Declined {
        reason: NativeGpuDecline,
        continuation: NativePreparedContinuation<'runtime>,
    },
}

pub(crate) struct NativeAdmittedContinuation<'runtime> {
    runtime: &'runtime mut IsfGpuRuntime,
    batch: PreparedNativeBatch,
}

pub(crate) struct NativeCpuContinuation<'runtime> {
    runtime: &'runtime mut IsfGpuRuntime,
    inputs: PreparedNativeInputs,
}

/// Only failures proven to be unexecuted return a continuation. Terminal
/// cancellation/fault variants carry the broker's exact retirement identity.
pub(crate) enum NativeCpuContinuationFailure<'runtime> {
    Admission {
        fault: NativeGpuFault,
        reservation: FrameReservation,
        continuation: NativeCpuContinuation<'runtime>,
    },
    Declined {
        reason: NativeGpuDecline,
        continuation: NativeCpuContinuation<'runtime>,
    },
    Cancelled {
        retirement: Option<RetirementId>,
    },
    Fault {
        fault: NativeGpuFault,
        retirement: Option<RetirementId>,
    },
}

impl std::fmt::Debug for NativeCpuContinuationFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admission {
                fault, reservation, ..
            } => formatter
                .debug_struct("Admission")
                .field("fault", fault)
                .field("reservation", reservation)
                .finish_non_exhaustive(),
            Self::Declined { reason, .. } => formatter
                .debug_struct("Declined")
                .field("reason", reason)
                .finish_non_exhaustive(),
            Self::Cancelled { retirement } => formatter
                .debug_struct("Cancelled")
                .field("retirement", retirement)
                .finish(),
            Self::Fault { fault, retirement } => formatter
                .debug_struct("Fault")
                .field("fault", fault)
                .field("retirement", retirement)
                .finish(),
        }
    }
}

impl<'runtime> NativePreparedContinuation<'runtime> {
    pub(crate) fn allocation_plan(&self) -> Result<&NativeAllocationPlan, NativeGpuDecline> {
        self.inputs.allocation_plan()
    }

    pub(crate) fn prospective_identities(
        &self,
    ) -> Result<StackBatch<LayerResultIdentity>, NativeProspectiveIdentityError> {
        self.inputs.prospective_identities(self.runtime)
    }

    pub(crate) fn admit(self, quote: &BudgetQuote) -> NativeContinuationAdmission<'runtime> {
        match self.runtime.admit_native_batch(self.inputs, quote) {
            NativeBatchAdmission::Accepted(batch) => {
                NativeContinuationAdmission::Accepted(NativeAdmittedContinuation {
                    runtime: self.runtime,
                    batch,
                })
            }
            NativeBatchAdmission::Declined { reason, prepared } => {
                NativeContinuationAdmission::Declined {
                    reason,
                    continuation: Self {
                        runtime: self.runtime,
                        inputs: prepared,
                    },
                }
            }
        }
    }

    /// Pre-execution fallback consumes the original preparation. In particular,
    /// it does not call the renderer, decode, resolve controls or prepare FX again.
    pub(crate) fn into_cpu(self) -> Result<NativeCpuContinuation<'runtime>, NativeGpuFault> {
        let inputs = self.runtime.prepare_native_cpu_fallback(self.inputs)?;
        Ok(NativeCpuContinuation {
            runtime: self.runtime,
            inputs,
        })
    }
}

impl<'runtime> NativeAdmittedContinuation<'runtime> {
    /// I's existing C3 submit/complete owner can consume this batch with the same
    /// runtime. The callback's exact success/failure owner is returned unchanged.
    /// No preparation or CPU continuation is returned once the batch is consumed.
    pub(crate) fn with_runtime<T>(
        self,
        execute: impl FnOnce(&mut IsfGpuRuntime, PreparedNativeBatch) -> T,
    ) -> T {
        execute(self.runtime, self.batch)
    }

    /// Quote admission itself has not executed. If reservation or an authority
    /// check declines before execution, return exactly that unexecuted work.
    pub(crate) fn return_unexecuted(self) -> NativePreparedContinuation<'runtime> {
        NativePreparedContinuation {
            runtime: self.runtime,
            inputs: self.batch.inputs,
        }
    }
}

impl<'runtime> NativeCpuContinuation<'runtime> {
    /// Finish an output's frozen-history handover before this unexecuted CPU
    /// continuation runs. The same prepared inputs remain affine; callers
    /// refresh the allocation plan after the shared runtime's caches change.
    pub(crate) fn with_runtime_before_execution<T>(
        &mut self,
        operation: impl FnOnce(&mut IsfGpuRuntime) -> T,
    ) -> T {
        operation(self.runtime)
    }

    pub(crate) fn allocation_plan(&self) -> Result<&NativeAllocationPlan, NativeGpuDecline> {
        self.inputs.allocation_plan()
    }

    /// Requote after retirement/cache eviction without preparing another tick.
    pub(crate) fn refresh(self) -> Result<Self, NativeGpuFault> {
        let inputs = self.runtime.prepare_native_cpu_fallback(self.inputs)?;
        Ok(Self {
            runtime: self.runtime,
            inputs,
        })
    }

    pub(crate) fn run(
        self,
        owner: &mut NativeGpuOwner,
        reservation: FrameReservation,
        cancel: &dyn VideoRenderCancellation,
    ) -> Result<NativeCpuResults, NativeCpuContinuationFailure<'runtime>> {
        let Self { runtime, inputs } = self;
        match runtime.render_native_cpu_fallback(owner, inputs, reservation, cancel) {
            Ok(outcomes) => Ok(outcomes),
            Err(NativeCpuRenderFailure::Admission {
                fault,
                prepared,
                reservation,
            }) => Err(NativeCpuContinuationFailure::Admission {
                fault,
                reservation,
                continuation: Self {
                    runtime,
                    inputs: prepared,
                },
            }),
            Err(NativeCpuRenderFailure::Declined { reason, prepared }) => {
                Err(NativeCpuContinuationFailure::Declined {
                    reason,
                    continuation: Self {
                        runtime,
                        inputs: prepared,
                    },
                })
            }
            Err(NativeCpuRenderFailure::Cancelled { retirement }) => {
                Err(NativeCpuContinuationFailure::Cancelled { retirement })
            }
            Err(NativeCpuRenderFailure::Fault { fault, retirement }) => {
                Err(NativeCpuContinuationFailure::Fault { fault, retirement })
            }
        }
    }
}

impl<P: VideoFrameProvider> VideoPreviewRenderer<P> {
    fn require_native_continuation_runtime(
        &self,
        frame: NativeFrameId,
    ) -> Result<(), NativePreparationFailure> {
        match self.isf_runtime.as_ref() {
            Some(Ok(runtime)) if runtime.native_context().is_some() => Ok(()),
            _ => Err(NativePreparationFailure::Fault(NativeGpuFault {
                context: GpuContextId {
                    owner_id: frame.output_id,
                    device_generation: 0,
                },
                frame: Some(frame),
                phase: GpuPhase::IsfBatch,
                kind: NativeGpuFaultKind::StateMismatch,
                message: "native continuation requires the renderer's existing shared ISF runtime"
                    .into(),
            })),
        }
    }

    fn prepare_continuation_chain(
        &mut self,
        chain: ResolvedVideoEffectChain,
        layer_id: protocol::VideoLayerId,
        key: Option<protocol::VideoRenderInputKey>,
        cancellation: Option<&dyn VideoRenderCancellation>,
    ) -> Result<PreparedChain<'static>, VideoRuntimeError> {
        check_runtime_cancellation(cancellation)?;
        if resolved_chain_executes(&chain) {
            self.prepare_effect_chain(Cow::Owned(chain), layer_id, key, cancellation)
        } else {
            Ok(PreparedChain {
                source: Cow::Owned(chain),
                stages: Vec::new(),
                keys: Vec::new(),
                faults: Vec::new(),
            })
        }
    }

    /// Keep the existing canonical CPU preparation seam and signature. It owns
    /// prepared shader/image inputs, but creates no runtime or clock reservation.
    #[cfg(test)]
    pub(crate) fn prepare_native_cpu_invocation(
        &mut self,
        chain: ResolvedVideoEffectChain,
        input: Arc<VideoFrame>,
        render_input_key: Option<protocol::VideoRenderInputKey>,
        cancellation: Option<&dyn VideoRenderCancellation>,
    ) -> Result<PreparedNativeCpuInvocation, VideoRuntimeError> {
        let prepared =
            self.prepare_continuation_chain(chain, input.layer_id, render_input_key, cancellation)?;
        Ok(PreparedNativeCpuInvocation::new(
            input,
            prepared,
            self.isf_frame_inputs.date,
            self.isf_audio_frame.clone(),
        ))
    }

    /// Call after I's existing decode/input/history preparation, before any FX
    /// execution. Do not call the complete CPU renderer first. All layer keys,
    /// DATE and audio are captured once; the borrow then prevents another tick
    /// from changing this renderer's runtime while I quotes/plans/declines.
    pub(crate) fn prepare_native_layer_continuation(
        &mut self,
        frame: NativeFrameId,
        layers: StackBatch<NativeLayerPreparation>,
        restrictions: NativeBatchRestrictions,
        cancellation: Option<&dyn VideoRenderCancellation>,
    ) -> Result<NativePreparedContinuation<'_>, NativePreparationFailure> {
        check_runtime_cancellation(cancellation).map_err(NativePreparationFailure::Runtime)?;
        self.require_native_continuation_runtime(frame)?;
        let date = self.isf_frame_inputs.date;
        let audio = self.isf_audio_frame.clone();
        let mut jobs = StackBatch::Empty;
        for layer in layers {
            let prepared = self
                .prepare_continuation_chain(
                    layer.chain,
                    layer.input.layer_id,
                    Some(layer.entry.input),
                    cancellation,
                )
                .map_err(NativePreparationFailure::Runtime)?;
            jobs.push(PreparedNativeLayer {
                input: layer.input,
                chain: prepared,
                entry: layer.entry,
                source_generation: layer.source_generation,
                content_sequence: layer.content_sequence,
                date,
            });
        }
        check_runtime_cancellation(cancellation).map_err(NativePreparationFailure::Runtime)?;
        let runtime = self
            .isf_runtime
            .as_mut()
            .expect("checked runtime")
            .as_mut()
            .expect("checked shared runtime");
        let mut inputs = runtime
            .prepare_native_inputs(frame, jobs, restrictions)
            .map_err(NativePreparationFailure::Fault)?;
        // CPU fallback must use the renderer's frozen audio, not whatever audio
        // happened to be installed on the runtime by an earlier invocation.
        inputs.audio = audio;
        Ok(NativePreparedContinuation { runtime, inputs })
    }

    /// Group/output/resourceful work keeps its original canonical scope. This
    /// path never invents Layer history identities or enters ownerless CPU APIs.
    #[cfg(test)]
    pub(crate) fn prepare_native_cpu_continuation(
        &mut self,
        frame: NativeFrameId,
        chain: ResolvedVideoEffectChain,
        input: Arc<VideoFrame>,
        render_input_key: Option<protocol::VideoRenderInputKey>,
        cancellation: Option<&dyn VideoRenderCancellation>,
    ) -> Result<NativeCpuContinuation<'_>, NativePreparationFailure> {
        check_runtime_cancellation(cancellation).map_err(NativePreparationFailure::Runtime)?;
        self.require_native_continuation_runtime(frame)?;
        let invocation = self
            .prepare_native_cpu_invocation(chain, input, render_input_key, cancellation)
            .map_err(NativePreparationFailure::Runtime)?;
        let runtime = self
            .isf_runtime
            .as_mut()
            .expect("checked runtime")
            .as_mut()
            .expect("checked shared runtime");
        let inputs = runtime
            .prepare_native_cpu_inputs(frame, invocation)
            .map_err(NativePreparationFailure::Fault)?;
        Ok(NativeCpuContinuation { runtime, inputs })
    }
}

#[cfg(test)]
#[path = "isf_native_continuation_tests.rs"]
mod tests;
