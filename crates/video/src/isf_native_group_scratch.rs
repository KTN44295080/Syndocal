//! Same-transaction scratch ownership, separate from immutable layer leases.
use super::*;

/// Borrow only an existing, exclusively selected stateless layer's two pooled
/// scratch textures, never its assembled final image or retained history.
/// Require output extent, Rgba8Unorm, render/sample usage and storage on pong.
/// Native pooled RGBA8 textures include storage usage so the exact existing
/// unpack needs no extra copy. Cold/incompatible/stateful selections fall back
/// to the composition's own charged targets.
///
/// Both IDs enter NativeAllocationPlan as existing aliases, not new charges.
/// ISF submits before composition in this frame transaction. The composition
/// phase retains this token (and records both IDs) before encoding. The pool's
/// exclusive checkout cannot reacquire its owner until every dependent phase
/// drains, including failure/cancellation and fixed-clock recovery. Invalidating
/// pool metadata cannot revoke that pin. No token is stored in history/output;
/// after retirement the next transaction may safely write the pair again.
pub(crate) struct GroupFxScratch {
    owner: Arc<ScratchAllocation>,
    context: GpuContextId,
    frame: NativeFrameId,
    extent: (u32, u32),
}

impl std::fmt::Debug for GroupFxScratch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupFxScratch")
            .field("frame", &self.frame)
            .field("ids", &self.ids())
            .finish()
    }
}

impl GroupFxScratch {
    pub(crate) fn ids(&self) -> [GpuAllocationId; 2] {
        [self.owner.charges[0].id(), self.owner.charges[1].id()]
    }

    pub(crate) fn matches(
        &self,
        context: GpuContextId,
        frame: NativeFrameId,
        extent: (u32, u32),
    ) -> bool {
        self.context == context && self.frame == frame && self.extent == extent
    }

    pub(crate) fn targets(&self) -> crate::isf_runtime::ResidentGroupFxTargets {
        crate::isf_runtime::ResidentGroupFxTargets::borrowed(
            self.owner.resources.group_scratch_images(),
        )
    }
}

impl PreparedNativeInputs {
    pub(crate) fn group_fx_scratch(&self, extent: (u32, u32)) -> Option<Arc<GroupFxScratch>> {
        self.allocation_plan().ok()?;
        self.pooled.iter().find_map(|selected| {
            if selected.key.resources.is_some()
                || (selected.key.width, selected.key.height) != extent
            {
                return None;
            }
            let owner = selected.scratch.as_ref()?;
            if !owner.resources.group_scratch_compatible(extent) {
                return None;
            }
            Some(Arc::new(GroupFxScratch {
                owner: Arc::clone(owner),
                context: self.context,
                frame: self.frame,
                extent,
            }))
        })
    }
}

#[cfg(test)]
#[path = "isf_native_group_scratch_tests.rs"]
mod tests;
