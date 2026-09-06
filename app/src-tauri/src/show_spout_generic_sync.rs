//! Policy boundary between the generic external-video synchronizer and the
//! two fixed show Spout outputs.
//!
//! A valid active show pair is owned by the show-output lifecycle, so its
//! Spout routes are removed from the generic plan.  A fully disabled exact
//! pair remains in the plan long enough for the generic runtime to retire any
//! stale routes.  The disabled case is admitted only after the same strict
//! validator accepts the pair with its two `enabled` bits temporarily set for
//! validation; the cloned values are never published or passed to a driver.

use protocol::{CompositionSummary, VideoOutputKind, VideoOutputSummary};
use video::ExternalVideoIoRoutePlans;

/// Apply the strict show-pair policy before generic external-video route sync.
///
/// The generic synchronizer must see a fully disabled exact pair so it can
/// stop stale routes.  Every other strict-pair failure remains fail-closed.
pub(crate) fn filter_generic_spout_sync_plans(
    mut plans: ExternalVideoIoRoutePlans,
    outputs: &[VideoOutputSummary],
    compositions: &[CompositionSummary],
) -> Result<ExternalVideoIoRoutePlans, String> {
    let has_reserved_show_output = outputs.iter().any(|output| {
        is_reserved_show_name(
            &output.label,
            output.endpoint_name.as_deref().unwrap_or_default(),
        )
    });

    match crate::show_spout_outputs::validate_show_spout_outputs_with_compositions(
        outputs,
        compositions,
    ) {
        Ok(_) => {
            if plans.outputs.iter().any(|plan| {
                plan.kind == VideoOutputKind::SpoutSender
                    && !is_reserved_show_name(&plan.label, &plan.endpoint_name)
            }) {
                return Err(
                    "Generic Spout synchronization is blocked while the strict show pair is authored or active"
                        .to_string(),
                );
            }
            plans
                .outputs
                .retain(|plan| plan.kind != VideoOutputKind::SpoutSender);
        }
        Err(crate::show_spout_outputs::ShowSpoutValidationError::Disabled)
            if fully_disabled_pair_is_structurally_valid(outputs, compositions) =>
        {
            // Keep the authored disabled pair in the generic plan. Its `live`
            // values are false, so the generic runtime retires stale routes
            // without creating or sending a new route.
        }
        Err(crate::show_spout_outputs::ShowSpoutValidationError::MissingPair)
            if !has_reserved_show_output => {}
        Err(error) if has_reserved_show_output => {
            return Err(format!(
                "Generic Spout synchronization is blocked by an invalid or conflicting strict show pair: {error}"
            ));
        }
        Err(_) => {}
    }

    Ok(plans)
}

fn is_reserved_show_name(label: &str, endpoint_name: &str) -> bool {
    matches!(
        label,
        crate::show_spout_outputs::SHOW_SPOUT_BACKGROUND_NAME
            | crate::show_spout_outputs::SHOW_SPOUT_FOREGROUND_NAME
    ) || matches!(
        endpoint_name,
        crate::show_spout_outputs::SHOW_SPOUT_BACKGROUND_NAME
            | crate::show_spout_outputs::SHOW_SPOUT_FOREGROUND_NAME
    )
}

/// Return true only for the exact two-sender pair with both senders disabled,
/// after all strict non-activation invariants have been checked.
fn fully_disabled_pair_is_structurally_valid(
    outputs: &[VideoOutputSummary],
    compositions: &[CompositionSummary],
) -> bool {
    let spout_count = outputs
        .iter()
        .filter(|output| output.kind == VideoOutputKind::SpoutSender)
        .count();
    if spout_count != 2
        || outputs
            .iter()
            .filter(|output| output.kind == VideoOutputKind::SpoutSender)
            .any(|output| output.enabled)
    {
        return false;
    }

    let mut structurally_enabled_outputs = outputs.to_vec();
    for output in structurally_enabled_outputs
        .iter_mut()
        .filter(|output| output.kind == VideoOutputKind::SpoutSender)
    {
        output.enabled = true;
    }

    crate::show_spout_outputs::validate_show_spout_outputs_with_compositions(
        &structurally_enabled_outputs,
        compositions,
    )
    .is_ok()
}

#[cfg(test)]
mod tests {
    include!("show_spout_generic_sync_tests.rs");
}
