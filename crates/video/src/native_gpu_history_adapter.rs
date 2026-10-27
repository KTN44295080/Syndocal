//! P's production adapter between G's native results and the pure history
//! (INTERFACE-01 section 5.2). Validated history entries are minted only from
//! actual affine G owners: a validated ISF layer with its batch receipt, or a
//! fixed-clock CPU recovery outcome with its charge. Nothing here fabricates a
//! receipt, proof or charge; rejected owners are returned intact.
#[cfg(test)]
use super::MaterializedResident;
use super::{
    CpuBackingOrigin, CpuHistoryBacking, Entry, HistoryBacking, HistoryError, HistoryHostBudget,
    HistoryHostFootprint, HistoryPhaseOutcome, HistoryResident, HostAllocation,
    MaterializedCpuLayer, NativeHistory, ValidatedHistoryEntries,
};
#[cfg(test)]
use crate::isf_native_gpu::MaterializedGpuLayer;
use crate::native_gpu_budget::{AllocationCharge, ChargeKind};
use crate::native_gpu_contract::{
    GpuAllocationId, GpuContextId, GpuPhase, HistoryEntryId, LayerOrigin, LayerResultIdentity,
    MaterializationDescriptor, NativeFrameId,
};
use crate::native_gpu_result::{
    CanonicalCpuLayerOutcome, CpuOutcomeProvenance, FrozenStatelessRecipe, GpuLayerLease,
    IsfValidatedReceipt, NativeCpuFrameOwner, ValidatedCpuMaterialization, ValidatedLayerResult,
};
use crate::{VideoEffectStageFault, VideoFrame, VideoPixelFormat};
use std::sync::Arc;

/// The complete owner accompanying a canonical pixel Arc. The adapter checks
/// exact Arc identity before admitting the pair; a matching byte buffer alone
/// is not evidence that this owner carries that allocation's charge.
#[derive(Clone)]
pub(crate) enum CanonicalCpuHistoryOwner {
    History(Arc<CpuHistoryBacking<AllocationCharge>>),
    Native(Arc<NativeCpuFrameOwner>),
}

#[derive(Clone)]
pub(crate) struct CanonicalCpuHistoryFrame {
    pub entry: HistoryEntryId,
    pub frame: Arc<VideoFrame>,
    pub owner: Option<CanonicalCpuHistoryOwner>,
}

#[must_use]
pub(crate) struct CanonicalCpuHistoryImportFailure {
    pub error: HistoryError,
    pub frames: Vec<CanonicalCpuHistoryFrame>,
}

fn canonical_cpu_charge(
    charge: &AllocationCharge,
    frame: &VideoFrame,
) -> Result<GpuAllocationId, HistoryError> {
    if !matches!(
        charge.kind(),
        ChargeKind::ActiveCpuInput | ChargeKind::CpuSnapshot | ChargeKind::CpuLastValid
    ) || charge.capacity() < frame.data.capacity() as u64
    {
        return Err(HistoryError::IdentityMismatch);
    }
    Ok(charge.id())
}

impl CanonicalCpuHistoryOwner {
    fn history_backing(&self) -> Option<Arc<CpuHistoryBacking<AllocationCharge>>> {
        match self {
            Self::History(backing) => Some(Arc::clone(backing)),
            Self::Native(owner) => owner.retained_owner().and_then(|parent| {
                Arc::clone(parent)
                    .downcast::<CpuHistoryBacking<AllocationCharge>>()
                    .ok()
            }),
        }
    }

    fn validate(
        &self,
        frame: &Arc<VideoFrame>,
        context: GpuContextId,
    ) -> Result<Option<GpuAllocationId>, HistoryError> {
        let managed = match self {
            Self::History(backing) => {
                if !Arc::ptr_eq(&backing.frame, frame) {
                    return Err(HistoryError::IdentityMismatch);
                }
                backing.native_source_charge()?.map(AllocationCharge::id)
            }
            Self::Native(owner) => {
                if !Arc::ptr_eq(&owner.frame, frame) {
                    return Err(HistoryError::IdentityMismatch);
                }
                let managed = owner
                    .charge
                    .as_ref()
                    .map(|charge| canonical_cpu_charge(charge, frame))
                    .transpose()?;
                if let Some(backing) = self.history_backing() {
                    let source = Self::History(backing).validate(frame, context)?;
                    if source != managed {
                        return Err(HistoryError::IdentityMismatch);
                    }
                }
                managed
            }
        };
        if managed.is_some_and(|allocation| {
            allocation.context.owner_id != context.owner_id
                || allocation.context.device_generation > context.device_generation
        }) {
            return Err(HistoryError::IdentityMismatch);
        }
        Ok(managed)
    }

    fn into_retained(self) -> Arc<dyn std::any::Any + Send + Sync> {
        if let Some(backing) = self.history_backing() {
            return match backing.origin() {
                CpuBackingOrigin::Canonical { owner, .. } => Arc::clone(owner),
                _ => backing,
            };
        }
        match self {
            Self::History(_) => unreachable!("history owners have a backing"),
            Self::Native(owner) => owner,
        }
    }
}

impl CpuHistoryBacking<AllocationCharge> {
    fn native_source_charge(&self) -> Result<Option<&AllocationCharge>, HistoryError> {
        let charge = match self.origin() {
            CpuBackingOrigin::Imported => None,
            CpuBackingOrigin::Materialized { descriptor, charge } => {
                super::check_pixels(&self.frame, &descriptor.layer)?;
                if descriptor.layer.context != charge.id().context
                    || descriptor.allocation.context != descriptor.layer.context
                    || descriptor.completed_submission.context != descriptor.layer.context
                {
                    return Err(HistoryError::IdentityMismatch);
                }
                Some(charge)
            }
            CpuBackingOrigin::Recovered { charge } => Some(charge),
            CpuBackingOrigin::Canonical {
                owner,
                managed_allocation,
            } => {
                let charge = if let Some(native) = owner.downcast_ref::<NativeCpuFrameOwner>() {
                    if !Arc::ptr_eq(&native.frame, &self.frame) {
                        return Err(HistoryError::IdentityMismatch);
                    }
                    native.charge.as_deref()
                } else if let Some(history) = owner.downcast_ref::<Self>() {
                    if !Arc::ptr_eq(&history.frame, &self.frame) {
                        return Err(HistoryError::IdentityMismatch);
                    }
                    history.native_source_charge()?
                } else {
                    return Err(HistoryError::IdentityMismatch);
                };
                if charge.map(AllocationCharge::id) != *managed_allocation {
                    return Err(HistoryError::IdentityMismatch);
                }
                charge
            }
        };
        if let Some(charge) = charge {
            canonical_cpu_charge(charge, &self.frame)?;
        }
        Ok(charge)
    }

    /// Register an already charged post-FX CPU history frame with G's shared
    /// canonical executor. The exact same allocation ID and complete P backing
    /// remain alive; imported host-only frames retain their P owner as well.
    pub(crate) fn native_cpu_owner(
        self: &Arc<Self>,
    ) -> Result<Arc<NativeCpuFrameOwner>, HistoryError> {
        let parent: Arc<dyn std::any::Any + Send + Sync> = self.clone();
        let charge = self
            .native_source_charge()?
            .map(|charge| Arc::new(charge.alias_retaining(Arc::clone(&parent))));
        Ok(Arc::new(NativeCpuFrameOwner::with_retained_owner(
            Arc::clone(&self.frame),
            charge,
            parent,
        )))
    }
}

struct ActiveCpuHistoryAlias {
    // Release the host managed-alias classification before its last G charge.
    _host: super::HistoryHostCharge,
    _owner: Arc<NativeCpuFrameOwner>,
    _charge: Arc<AllocationCharge>,
}

/// Add G admission to an existing host-only input without detaching any P
/// owner. If it came from P, the same bounded host ledger records an alias only
/// for the active managed lifetime; its original Imported charge is unchanged.
pub(crate) fn activate_native_cpu_owner(
    prior: Arc<NativeCpuFrameOwner>,
    charge: Arc<AllocationCharge>,
) -> Arc<NativeCpuFrameOwner> {
    let frame = Arc::clone(&prior.frame);
    let mut source = Arc::clone(&prior);
    let backing = loop {
        let Some(parent) = source.retained_owner() else {
            break None;
        };
        if let Ok(backing) = Arc::clone(parent).downcast::<CpuHistoryBacking<AllocationCharge>>() {
            break Arc::ptr_eq(&backing.frame, &frame).then_some(backing);
        }
        let Ok(next) = Arc::clone(parent).downcast::<NativeCpuFrameOwner>() else {
            break None;
        };
        if !Arc::ptr_eq(&next.frame, &frame) {
            break None;
        }
        source = next;
    };
    let parent: Arc<dyn std::any::Any + Send + Sync> = match backing {
        Some(backing) => Arc::new(ActiveCpuHistoryAlias {
            _host: backing.host.managed_alias(),
            _owner: prior,
            _charge: Arc::clone(&charge),
        }),
        None => prior,
    };
    Arc::new(NativeCpuFrameOwner::with_retained_owner(
        frame,
        Some(charge),
        parent,
    ))
}

impl NativeGpuHistory {
    /// All-or-nothing CPU-to-native import of canonical slots and their complete
    /// owners. Failure returns every input intact. Same-ledger history aliases
    /// reuse their backing; another ledger receives one host alias while the
    /// original G charge remains in its complete upstream owner.
    pub(crate) fn import_canonical_queue_with_owners(
        context: GpuContextId,
        scope: super::HistoryScope,
        host: HistoryHostBudget,
        frames: Vec<CanonicalCpuHistoryFrame>,
    ) -> Result<Self, CanonicalCpuHistoryImportFailure> {
        let checked = (|| {
            let history = Self::new(context, scope, host)?;
            if frames.len() > super::CAPACITY {
                return Err(HistoryError::InvalidPolicy);
            }
            let mut checked = Vec::with_capacity(frames.len());
            for input in &frames {
                let frame = &input.frame;
                let identity = LayerResultIdentity {
                    context,
                    frame: scope.frame_scope,
                    entry: input.entry,
                    layer_id: scope.layer_id,
                    source_generation: scope.source_generation,
                    content_sequence: input.entry.sequence,
                    pts_ms: frame.pts_ms,
                    duration_ms: frame.duration_ms,
                    width: frame.width,
                    height: frame.height,
                    format: frame.format,
                    origin: LayerOrigin::ImportedCpu,
                };
                history.check_identity(&identity)?;
                let resource_identity = LayerResultIdentity {
                    layer_id: frame.layer_id,
                    ..identity
                };
                super::check_pixels(frame, &resource_identity)?;
                let managed = match &input.owner {
                    Some(owner) => owner.validate(frame, context)?,
                    None => None,
                };
                let reuse = input
                    .owner
                    .as_ref()
                    .and_then(CanonicalCpuHistoryOwner::history_backing)
                    .filter(|backing| history.host.owns(&backing.host));
                checked.push((identity, resource_identity, managed, reuse));
            }
            let metadata = checked
                .iter()
                .map(|(identity, _, _, _)| super::HistoryMeta {
                    id: identity.entry,
                    pts_ms: identity.pts_ms,
                    duration_ms: identity.duration_ms,
                })
                .collect::<Vec<_>>();
            super::plan_history_update(&metadata, &[], super::CAPACITY, 0)
                .map_err(|_| HistoryError::InvalidPolicy)?;
            let charges = history.host.charge_all(
                frames
                    .iter()
                    .zip(&checked)
                    .filter_map(|(input, (_, _, managed, reuse))| {
                        reuse
                            .is_none()
                            .then(|| HostAllocation::shared_frame(&input.frame, managed.is_some()))
                    })
                    .collect(),
            )?;
            Ok((history, checked, charges))
        })();
        let (mut history, checked, charges) = match checked {
            Ok(checked) => checked,
            Err(error) => return Err(CanonicalCpuHistoryImportFailure { error, frames }),
        };
        let mut charges = charges.into_iter();
        for (input, (identity, resource_identity, managed_allocation, reuse)) in
            frames.into_iter().zip(checked)
        {
            let backing = if let Some(backing) = reuse {
                backing
            } else {
                let origin = match input.owner {
                    Some(owner) => CpuBackingOrigin::Canonical {
                        owner: owner.into_retained(),
                        managed_allocation,
                    },
                    None => CpuBackingOrigin::Imported,
                };
                Arc::new(CpuHistoryBacking {
                    frame: input.frame,
                    origin,
                    host: charges.next().expect("preflight charged every new backing"),
                })
            };
            history.push_imported(identity, resource_identity, backing);
        }
        Ok(history)
    }
}

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
#[cfg(test)]
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
