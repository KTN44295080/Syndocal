//! P's production adapter between G's native results and the pure history
//! (INTERFACE-01 section 5.2). Validated history entries are minted only from
//! actual affine G owners: a validated ISF layer with its batch receipt, or a
//! fixed-clock CPU recovery outcome with its charge. Nothing here fabricates a
//! receipt, proof or charge; rejected owners are returned intact.
use super::{
    CpuBackingOrigin, CpuHistoryBacking, Entry, HistoryBacking, HistoryError, HistoryHostBudget,
    HistoryHostFootprint, HistoryPhaseOutcome, HistoryResident, HostAllocation,
    MaterializedCpuLayer, MaterializedResident, NativeHistory, ValidatedHistoryEntries,
};
use crate::isf_native_gpu::MaterializedGpuLayer;
use crate::native_gpu_budget::AllocationCharge;
use crate::native_gpu_contract::{
    GpuAllocationId, GpuPhase, LayerOrigin, LayerResultIdentity, MaterializationDescriptor,
    NativeFrameId,
};
use crate::native_gpu_result::{
    CanonicalCpuLayerOutcome, CpuOutcomeProvenance, FrozenStatelessRecipe, GpuLayerLease,
    IsfValidatedReceipt, ValidatedCpuMaterialization, ValidatedLayerResult,
};
use crate::{VideoEffectStageFault, VideoFrame, VideoPixelFormat};
use std::sync::Arc;

/// Production history: G recipes, G leases, G CPU allocation charges.
pub(crate) type NativeGpuHistory =
    NativeHistory<FrozenStatelessRecipe, GpuLayerLease, AllocationCharge>;
pub(crate) type NativeGpuValidatedEntries =
    ValidatedHistoryEntries<FrozenStatelessRecipe, GpuLayerLease, AllocationCharge>;

impl HistoryResident for GpuLayerLease {
    fn identity(&self) -> &LayerResultIdentity {
        GpuLayerLease::identity(self)
    }
    fn allocation(&self) -> GpuAllocationId {
        self.allocation_id()
    }
}
impl HistoryHostFootprint for FrozenStatelessRecipe {
    /// The recipe pins its raw input frame for exact replay. While a frame is
    /// active G charges that input separately (ActiveCpuInput); the long-term
    /// retention is the host budget's, so it is not marked managed here.
    fn host_allocations(&self) -> Vec<HostAllocation> {
        vec![HostAllocation::shared_frame(&self.job().input, false)]
    }
}
/// G mints this only from a validated private-readback completion
/// (PendingCpuMaterialization::complete), with its CpuSnapshot charge.
impl MaterializedCpuLayer for ValidatedCpuMaterialization {
    type Charge = AllocationCharge;
    fn frame(&self) -> &VideoFrame {
        ValidatedCpuMaterialization::frame(self)
    }
    fn descriptor(&self) -> &MaterializationDescriptor {
        ValidatedCpuMaterialization::descriptor(self)
    }
    fn into_charged_parts(self) -> (VideoFrame, MaterializationDescriptor, AllocationCharge) {
        self.into_parts()
    }
}
/// G mints this only from a validated historical replay completion.
impl MaterializedResident for MaterializedGpuLayer {
    type Resident = GpuLayerLease;
    fn descriptor(&self) -> MaterializationDescriptor {
        MaterializedGpuLayer::descriptor(self)
    }
    fn resident(&self) -> &GpuLayerLease {
        &self.lease
    }
    fn into_resident(self) -> GpuLayerLease {
        self.into_parts().0
    }
}

/// The G owner offered for sealing, returned unchanged with the reason.
pub(crate) struct HistorySealFailure<O> {
    pub error: HistoryError,
    pub outcome: O,
}

impl NativeGpuValidatedEntries {
    /// Seal one validated ISF layer with the receipt of its own Batch3 phase.
    /// The lease, recipe and claimed identity must all be that receipt's
    /// frame/context; a layer of another frame (stale) or generation (foreign)
    /// or with a rewritten identity is rejected. The recipe's inputs are
    /// charged to the host budget before the entry exists. The composition
    /// outcome starts invalid; record it with `record_composition`.
    pub(crate) fn seal_isf_layer(
        receipt: &IsfValidatedReceipt,
        layer: ValidatedLayerResult,
        host: &HistoryHostBudget,
    ) -> Result<Self, HistorySealFailure<ValidatedLayerResult>> {
        let phase = receipt.phase();
        let identity = layer.identity;
        let matches = phase.phase() == GpuPhase::IsfBatch
            && phase.validation().is_ok()
            && identity == *layer.lease.identity()
            && identity == *layer.recipe.identity()
            && identity.context == phase.context()
            && layer.lease.allocation_id().context == phase.context()
            && identity.frame == phase.frame()
            && identity.format == VideoPixelFormat::Rgba8
            && matches!(identity.origin, LayerOrigin::Executed(_));
        if !matches {
            return Err(HistorySealFailure {
                error: HistoryError::IdentityMismatch,
                outcome: layer,
            });
        }
        let replay_host = match host.charge_all(layer.recipe.host_allocations()) {
            Ok(charges) => charges,
            Err(error) => {
                return Err(HistorySealFailure {
                    error,
                    outcome: layer,
                })
            }
        };
        let ValidatedLayerResult {
            identity,
            lease,
            recipe,
        } = layer;
        Ok(Self {
            frame: identity.frame,
            phase: HistoryPhaseOutcome::IsfValid {
                composition_valid: false,
            },
            entries: vec![Entry {
                identity,
                resource_identity: identity,
                backing: HistoryBacking::Replay(recipe),
                replay_host,
                resident: Some(Arc::new(lease)),
            }],
        })
    }

    /// Seal one of G's fixed-clock recovery outcomes for `frame` as CPU backing
    /// (never a recipe). Pixels and their G charge become one owner. The stage
    /// faults are returned to I, which applies the canonical publication and
    /// Error/LastValid rules before deciding to commit.
    pub(crate) fn seal_recovered_cpu(
        frame: NativeFrameId,
        outcome: CanonicalCpuLayerOutcome,
        host: &HistoryHostBudget,
    ) -> Result<(Self, Vec<VideoEffectStageFault>), HistorySealFailure<CanonicalCpuLayerOutcome>>
    {
        let identity = outcome.identity;
        let checked = if identity.frame != frame
            || outcome.provenance != CpuOutcomeProvenance::FixedClockRecovery
            || outcome.charge.id().context != identity.context
            || outcome.charge.capacity() < outcome.frame.data.len() as u64
        {
            Err(HistoryError::IdentityMismatch)
        } else {
            super::check_pixels(&outcome.frame, &identity)
                .and_then(|()| host.reserve(outcome.frame.data.capacity() as u64))
        };
        let reservation = match checked {
            Ok(reservation) => reservation,
            Err(error) => return Err(HistorySealFailure { error, outcome }),
        };
        let CanonicalCpuLayerOutcome {
            identity,
            frame: pixels,
            faults,
            provenance: _,
            charge,
        } = outcome;
        let pixels = Arc::new(pixels);
        let host = reservation.bind(&pixels, true);
        Ok((
            Self {
                frame: identity.frame,
                phase: HistoryPhaseOutcome::RecoveredCpu,
                entries: vec![Entry {
                    identity,
                    resource_identity: identity,
                    backing: HistoryBacking::Cpu(Arc::new(CpuHistoryBacking {
                        frame: pixels,
                        origin: CpuBackingOrigin::Recovered { charge },
                        host,
                    })),
                    replay_host: Vec::new(),
                    resident: None,
                }],
            },
            faults,
        ))
    }
}

#[cfg(test)]
#[path = "native_gpu_history_adapter_tests.rs"]
mod tests;
