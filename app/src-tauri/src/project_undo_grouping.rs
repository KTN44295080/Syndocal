//! History metadata only; every publication retains its exact ticket/receipt.
pub(crate) const PREFIX: &str = "syndocal.undo.v1:";
const BATCH_PREFIX: &str = "syndocal.undo.v1:batch:";
fn append_field(key: &mut String, field: &str) {
    key.push_str(&field.len().to_string());
    key.push(':');
    key.push_str(field);
}
fn owner_key(mode: &str, owner: &str, window: &str, incarnation: u64) -> String {
    let mut key = format!("{PREFIX}{mode}:");
    append_field(&mut key, owner);
    append_field(&mut key, window);
    append_field(&mut key, &incarnation.to_string());
    key
}
pub(crate) fn media_relink_undo_key(
    group_id: &str,
    owner: &str,
    window: &str,
    incarnation: u64,
) -> Result<String, String> {
    if group_id.is_empty()
        || group_id.len() > 128
        || !group_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("Undo group ID must contain 1 to 128 ASCII letters, digits, '-' or '_'".into());
    }
    let mut key = owner_key("batch", owner, window, incarnation);
    append_field(&mut key, "media_relink");
    append_field(&mut key, group_id);
    Ok(key)
}
pub(crate) fn isf_control_undo_key(
    owner: &str,
    window: &str,
    incarnation: u64,
    scope: &str,
    stage: u64,
    control: &str,
) -> String {
    let mut key = owner_key("idle", owner, window, incarnation);
    append_field(&mut key, "isf_control");
    append_field(&mut key, scope);
    append_field(&mut key, &stage.to_string());
    append_field(&mut key, control);
    key
}
/// Caller additionally requires E/R/H continuity and empty redo stack.
pub(crate) fn history_keys_can_coalesce(
    previous: &str,
    next: &str,
    previous_ms: u64,
    next_ms: u64,
    idle_ms: u64,
) -> bool {
    if next.is_empty() || previous != next {
        return false;
    }
    if next.starts_with(BATCH_PREFIX) {
        return true;
    }
    next_ms
        .checked_sub(previous_ms)
        .is_some_and(|elapsed| elapsed <= idle_ms)
}

/// Only one continuous control's raw value may change. All catalog metadata,
/// stage IDs/order, shader source, other controls and legacy stacks must match.
pub(crate) fn single_isf_control_change(
    before: &[protocol::VideoEffectChainSummary],
    after: &[protocol::VideoEffectChainSummary],
) -> Option<(String, u64, String)> {
    use protocol::{VideoEffectKind, VideoIsfControlKind};
    if before.len() != after.len() {
        return None;
    }
    let mut changed = None;
    for (chain_index, (old_chain, new_chain)) in before.iter().zip(after).enumerate() {
        if old_chain.stages.len() != new_chain.stages.len() {
            return None;
        }
        for (stage_index, (old_stage, new_stage)) in
            old_chain.stages.iter().zip(&new_chain.stages).enumerate()
        {
            let VideoEffectKind::Isf { effect: old_effect } = &old_stage.effect.kind;
            let VideoEffectKind::Isf { effect: new_effect } = &new_stage.effect.kind;
            if old_effect.controls.len() != new_effect.controls.len() {
                return None;
            }
            for (control_index, (old, new)) in old_effect
                .controls
                .iter()
                .zip(&new_effect.controls)
                .enumerate()
            {
                if old == new {
                    continue;
                }
                if changed.is_some()
                    || new_stage.id.0 == 0
                    || old.value == new.value
                    || new.tempo_sync.is_some()
                    || !matches!(
                        new.kind,
                        VideoIsfControlKind::Float
                            | VideoIsfControlKind::Long
                            | VideoIsfControlKind::Point2d
                            | VideoIsfControlKind::Color
                    )
                {
                    return None;
                }
                let mut restored_control = new.clone();
                restored_control.value = old.value;
                restored_control.tempo_sync = old.tempo_sync;
                if &restored_control != old {
                    return None;
                }
                changed = Some((chain_index, stage_index, control_index));
            }
        }
    }
    let (ci, si, pi) = changed?;
    let mut restored = after.to_vec();
    let VideoEffectKind::Isf { effect: old_effect } = &before[ci].stages[si].effect.kind;
    let VideoEffectKind::Isf {
        effect: restored_effect,
    } = &mut restored[ci].stages[si].effect.kind;
    restored_effect.controls[pi] = old_effect.controls[pi].clone();
    if restored.as_slice() != before {
        return None;
    }
    Some((
        serde_json::to_string(&after[ci].scope).ok()?,
        after[ci].stages[si].id.0.into(),
        old_effect.controls[pi].name.clone(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::*;
    fn catalog() -> Vec<VideoEffectChainSummary> {
        vec![VideoEffectChainSummary {
            id: VideoEffectChainId(1),
            scope: VideoEffectScope::Layer { layer_id: 2 },
            bypassed: false,
            stages: vec![VideoEffectStageSummary {
                id: VideoEffectStageId(3),
                enabled: true,
                label: "stage".into(),
                effect: VideoEffectSummary {
                    id: VideoEffectId(4),
                    kind: VideoEffectKind::Isf {
                        effect: VideoIsfEffectSummary {
                            enabled: true,
                            label: "shader".into(),
                            source: "void main(){}".into(),
                            source_path: None,
                            description: None,
                            categories: vec![],
                            stack: vec![],
                            controls: vec![control("amount"), control("speed")],
                        },
                    },
                },
            }],
        }]
    }
    fn control(name: &str) -> VideoIsfControlSummary {
        VideoIsfControlSummary {
            name: name.into(),
            kind: VideoIsfControlKind::Float,
            value: [0.0; 4],
            default: [0.0; 4],
            minimum: [0.0; 4],
            maximum: [1.0; 4],
            labels: vec![],
            values: vec![],
            tempo_sync: None,
        }
    }
    fn effect(chains: &mut [VideoEffectChainSummary]) -> &mut VideoIsfEffectSummary {
        let VideoEffectKind::Isf { effect } = &mut chains[0].stages[0].effect.kind;
        effect
    }
    #[test]
    fn folder_group_survives_long_hash_but_not_another_batch_or_owner() {
        let key = media_relink_undo_key("batch-1", "owner", "main", 7).unwrap();
        assert!(history_keys_can_coalesce(&key, &key, 1, 90_001, 750));
        for other in [
            media_relink_undo_key("batch-2", "owner", "main", 7).unwrap(),
            media_relink_undo_key("batch-1", "other", "main", 7).unwrap(),
            media_relink_undo_key("batch-1", "owner", "other", 7).unwrap(),
            media_relink_undo_key("batch-1", "owner", "main", 8).unwrap(),
        ] {
            assert!(!history_keys_can_coalesce(&key, &other, 1, 2, 750));
        }
    }
    #[test]
    fn fader_idle_boundary_and_backwards_clock_are_explicit() {
        let key = isf_control_undo_key("owner", "main", 7, "layer2", 3, "amount");
        assert!(history_keys_can_coalesce(&key, &key, 1000, 1750, 750));
        assert!(!history_keys_can_coalesce(&key, &key, 1000, 1751, 750));
        assert!(!history_keys_can_coalesce(&key, &key, 1000, 999, 750));
        assert!(!history_keys_can_coalesce("", "", 0, 1, 750));
    }
    #[test]
    fn malformed_group_ids_fail_closed() {
        for value in ["", "space here", "x:y", "\u{65e5}", &"x".repeat(129)] {
            assert!(media_relink_undo_key(value, "o", "w", 1).is_err());
        }
    }
    #[test]
    fn single_raw_control_edit_has_stable_target() {
        let before = catalog();
        let mut after = before.clone();
        effect(&mut after).controls[0].value = [0.5; 4];
        let (scope, stage, name) = single_isf_control_change(&before, &after).unwrap();
        assert_eq!(scope, serde_json::to_string(&before[0].scope).unwrap());
        assert_eq!((stage, name), (3, "amount".into()));
        assert!(single_isf_control_change(&before, &before).is_none());
    }
    #[test]
    fn unrelated_catalog_edits_never_join_a_fader_group() {
        let before = catalog();
        let mut after = before.clone();
        effect(&mut after).controls[0].value = [0.5; 4];
        let mut changed = after.clone();
        effect(&mut changed).controls[1].value = [0.4; 4];
        assert!(single_isf_control_change(&before, &changed).is_none());
        let mut changed = after.clone();
        changed[0].stages[0].enabled = false;
        assert!(single_isf_control_change(&before, &changed).is_none());
        let mut changed = after.clone();
        changed[0].stages[0].id = VideoEffectStageId(9);
        assert!(single_isf_control_change(&before, &changed).is_none());
        let mut changed = after.clone();
        effect(&mut changed).source = "different".into();
        assert!(single_isf_control_change(&before, &changed).is_none());
        let mut changed = after;
        effect(&mut changed).controls[0].maximum = [2.0; 4];
        assert!(single_isf_control_change(&before, &changed).is_none());
    }
    #[test]
    fn raw_control_can_leave_tempo_sync_but_sync_change_is_a_separate_step() {
        let mut before = catalog();
        effect(&mut before).controls[0].tempo_sync = Some(VideoIsfTempoSync {
            unit: VideoIsfTempoSyncUnit::Beats,
            count: 1.0,
        });
        let mut after = before.clone();
        effect(&mut after).controls[0].value = [0.5; 4];
        effect(&mut after).controls[0].tempo_sync = None;
        assert!(single_isf_control_change(&before, &after).is_some());
        effect(&mut after).controls[0].tempo_sync = Some(VideoIsfTempoSync {
            unit: VideoIsfTempoSyncUnit::Beats,
            count: 2.0,
        });
        assert!(single_isf_control_change(&before, &after).is_none());
    }
}
