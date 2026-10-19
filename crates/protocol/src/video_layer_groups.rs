//! Group ordering policy shared by engine admission and project history preflight.
use crate::{VideoCompositionLayerOrder, VideoSnapshot};
use std::collections::BTreeSet;

pub fn apply_video_composition_layer_orders(
    video: &mut VideoSnapshot,
    orders: &[VideoCompositionLayerOrder],
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for order in orders {
        if !seen.insert(order.composition_id) {
            return Err("Video composition order may only be supplied once".into());
        }
        let composition = video
            .compositions
            .iter_mut()
            .find(|composition| composition.id == order.composition_id)
            .ok_or_else(|| format!("Missing video composition {}", order.composition_id))?;
        let before = composition
            .layer_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let after = order.layer_ids.iter().copied().collect::<BTreeSet<_>>();
        if before != after || after.len() != order.layer_ids.len() {
            return Err("Video composition order must be an exact member permutation".into());
        }
        composition.layer_ids.clone_from(&order.layer_ids);
        if order.composition_id == 1 {
            video.layers.sort_by_key(|layer| {
                order
                    .layer_ids
                    .iter()
                    .position(|id| *id == layer.id)
                    .unwrap_or(usize::MAX)
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CompositionSummary, VideoBlendMode, VideoLayerGroupSummary};

    #[test]
    fn layer_groups_legacy_defaults_are_pass_through_and_round_trip() {
        let group: VideoLayerGroupSummary =
            serde_json::from_str(r#"{"id":1,"label":"Legacy","composition_id":1,"layer_ids":[1]}"#)
                .unwrap();
        assert!(group.enabled);
        assert_eq!(group.opacity, 1.0);
        assert_eq!(group.blend_mode, VideoBlendMode::Normal);
        assert!(!group.solo && !group.bypassed);
        assert_eq!(group.transform, None);
        assert_eq!(
            serde_json::from_str::<VideoLayerGroupSummary>(&serde_json::to_string(&group).unwrap())
                .unwrap(),
            group
        );
    }

    #[test]
    fn layer_groups_order_requires_exact_unambiguous_permutation() {
        let mut video = VideoSnapshot {
            compositions: vec![CompositionSummary {
                id: 2,
                label: "Composition".into(),
                layer_ids: vec![1, 2, 3],
                timeline_layer_ids: Vec::new(),
                output_ids: Vec::new(),
            }],
            ..VideoSnapshot::default()
        };
        let order = |layer_ids| VideoCompositionLayerOrder {
            composition_id: 2,
            layer_ids,
        };
        for ids in [vec![1, 2], vec![1, 2, 2], vec![1, 2, 4]] {
            let before = video.clone();
            assert!(apply_video_composition_layer_orders(&mut video, &[order(ids)]).is_err());
            assert_eq!(video, before);
        }
        apply_video_composition_layer_orders(&mut video, &[order(vec![3, 1, 2])]).unwrap();
        assert_eq!(video.compositions[0].layer_ids, vec![3, 1, 2]);
        assert!(apply_video_composition_layer_orders(
            &mut video,
            &[order(vec![3, 1, 2]), order(vec![3, 1, 2])]
        )
        .is_err());
    }
}
