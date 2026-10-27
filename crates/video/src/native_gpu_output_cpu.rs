//! I-owned lifetime bridges for exact CPU history materializations.
//!
//! G proves copy/completion/unmap. The original single-frame test owner keeps
//! that evidence and its physical allocation charge inseparable. The `history`
//! adapter receives P's full charged CPU handover and retains whole owners for
//! queue and LastValid consumers. Neither adapter mints a completion proof.

#[path = "native_gpu_output_cpu_history.rs"]
pub(crate) mod history;

#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
use crate::native_gpu_budget::{AllocationCharge, ChargeKind};
#[cfg(test)]
use crate::native_gpu_contract::{
    GpuAllocationId, GpuPhase, LayerResultIdentity, MaterializationDescriptor, NativeGpuFault,
    NativeGpuFaultKind,
};
#[cfg(test)]
use crate::native_gpu_result::ValidatedCpuMaterialization;
#[cfg(test)]
use crate::{VideoFrame, VideoPixelFormat};

/// Share this complete owner with Arc::clone. There is deliberately no raw-frame
/// Arc accessor, mutable pixel access, Clone implementation or parts extraction.
/// Fields drop in declaration order: the pixels go before their charge.
#[cfg(test)]
pub(crate) struct ChargedCpuHistoryFrame {
    frame: VideoFrame,
    descriptor: MaterializationDescriptor,
    charge: AllocationCharge,
}

#[cfg(test)]
impl ChargedCpuHistoryFrame {
    /// Consume real G evidence against the history entry selected by the caller.
    /// Rejection retains all three original parts; it never discards pixels or
    /// releases accounting merely because the receiving history changed.
    pub(crate) fn from_materialization(
        expected: LayerResultIdentity,
        materialization: ValidatedCpuMaterialization,
    ) -> Result<Arc<Self>, RejectedCpuHistoryFrame> {
        let (frame, descriptor, charge) = materialization.into_parts();
        let owner = Self {
            frame,
            descriptor,
            charge,
        };
        if let Err(fault) = owner.validate(expected) {
            return Err(RejectedCpuHistoryFrame { fault, owner });
        }
        Ok(Arc::new(owner))
    }

    pub(crate) fn frame(&self) -> &VideoFrame {
        &self.frame
    }

    pub(crate) fn descriptor(&self) -> &MaterializationDescriptor {
        &self.descriptor
    }

    /// This is the CPU Vec allocation, distinct from descriptor.allocation,
    /// which identifies the GPU image read by G.
    pub(crate) fn cpu_allocation_id(&self) -> GpuAllocationId {
        self.charge.id()
    }

    pub(crate) fn charged_capacity_bytes(&self) -> u64 {
        self.charge.capacity()
    }

    fn validate(&self, expected: LayerResultIdentity) -> Result<(), NativeGpuFault> {
        let descriptor = &self.descriptor;
        let layer = descriptor.layer;
        if layer != expected
            || descriptor.allocation.context != expected.context
            || descriptor.completed_submission.context != expected.context
            || self.charge.id().context != expected.context
        {
            return Err(self.fault(
                NativeGpuFaultKind::IdentityMismatch,
                "CPU history materialization does not match its expected entry/context",
            ));
        }
        if layer.format != VideoPixelFormat::Rgba8
            || self.frame.layer_id != layer.layer_id
            || self.frame.width != layer.width
            || self.frame.height != layer.height
            || self.frame.pts_ms != layer.pts_ms
            || self.frame.duration_ms != layer.duration_ms
            || self.frame.format != layer.format
        {
            return Err(self.fault(
                NativeGpuFaultKind::IdentityMismatch,
                "CPU history pixels do not match their materialization descriptor",
            ));
        }
        let packed_bytes = u64::from(layer.width)
            .checked_mul(u64::from(layer.height))
            .and_then(|pixels| pixels.checked_mul(4))
            .filter(|bytes| *bytes != 0);
        if self.charge.kind() != ChargeKind::CpuSnapshot
            || packed_bytes != u64::try_from(self.frame.data.len()).ok()
            || u64::try_from(self.frame.data.capacity())
                .ok()
                .is_none_or(|capacity| capacity > self.charge.capacity())
        {
            return Err(self.fault(
                NativeGpuFaultKind::SnapshotCopy,
                "CPU history pixel capacity is not covered by its materialization charge",
            ));
        }
        Ok(())
    }

    fn fault(&self, kind: NativeGpuFaultKind, message: &str) -> NativeGpuFault {
        NativeGpuFault {
            context: self.descriptor.layer.context,
            frame: Some(self.descriptor.layer.frame),
            phase: GpuPhase::HistoricalReplay,
            kind,
            message: message.into(),
        }
    }
}

/// A rejected handover still owns the complete G evidence. Keep this value in
/// the caller's failure/cleanup owner until it is safe to discard the CPU frame.
/// No method turns a mismatch into a valid history backing or extracts a charge.
#[must_use]
#[cfg(test)]
pub(crate) struct RejectedCpuHistoryFrame {
    fault: NativeGpuFault,
    owner: ChargedCpuHistoryFrame,
}

#[cfg(test)]
impl RejectedCpuHistoryFrame {
    pub(crate) fn fault(&self) -> &NativeGpuFault {
        &self.fault
    }

    pub(crate) fn frame(&self) -> &VideoFrame {
        self.owner.frame()
    }

    pub(crate) fn descriptor(&self) -> &MaterializationDescriptor {
        self.owner.descriptor()
    }

    pub(crate) fn charged_capacity_bytes(&self) -> u64 {
        self.owner.charged_capacity_bytes()
    }
}

#[cfg(test)]
#[path = "native_gpu_output_cpu_tests.rs"]
mod tests;
