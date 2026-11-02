//! Composition's narrow adapter for G's same-frame scratch loan.
use super::*;

impl NativeCompositionPlan {
    pub(crate) fn needs_group_fx_scratch(&self) -> bool {
        self.groups.iter().any(|group| group.fx.is_some())
    }

    /// Install before merging/quoting. Absence is the ordinary charged fallback.
    pub(crate) fn reuse_group_fx_scratch(
        &mut self,
        scratch: Option<Arc<GroupFxScratch>>,
    ) -> Result<(), NativeGpuDecline> {
        let Some(scratch) = scratch else {
            return Ok(());
        };
        if !self.needs_group_fx_scratch()
            || self.group_fx_scratch.is_some()
            || !scratch.matches(self.context, self.frame, self.extent)
        {
            return Err(NativeGpuDecline::QuoteExpired);
        }
        let reuse = AllocationReuse {
            group_fx: Some(scratch.ids()),
            ..self.reuse()
        };
        self.native_allocations = composition_allocations(
            self.frame,
            self.allocations.premapping_bytes,
            &self.layers_bottom_first,
            &self.groups,
            reuse,
        )?;
        self.group_fx_scratch = Some(scratch);
        Ok(())
    }
}
