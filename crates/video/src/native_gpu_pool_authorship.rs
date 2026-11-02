//! Allocation-bank identity excludes live control values: moving a fader must
//! preserve feedback. Removing/replacing an authored stage releases idle banks.
use crate::{ResolvedVideoEffectChain, ResolvedVideoEffectStage};
use protocol::{
    VideoEffectChainId, VideoEffectId, VideoEffectScope, VideoEffectStageId, VideoIsfImageBinding,
};
use std::sync::Arc;

#[derive(Default)]
pub(super) struct PoolAuthorship(Vec<ChainIdentity>);

struct ChainIdentity {
    chain_id: Option<VideoEffectChainId>,
    scope: VideoEffectScope,
    stages: Vec<StageIdentity>,
}

struct StageIdentity {
    stage_id: Option<VideoEffectStageId>,
    effect_id: Option<VideoEffectId>,
    source: Arc<str>,
    source_path: Option<String>,
    bindings: Vec<VideoIsfImageBinding>,
}

impl StageIdentity {
    fn matches(&self, stage: &ResolvedVideoEffectStage) -> bool {
        self.stage_id == stage.stage_id
            && self.effect_id == stage.effect_id
            && self.source == stage.effect.source
            && self.source_path == stage.effect.source_path
            && self.bindings == stage.effect.image_bindings
    }
    fn new(stage: &ResolvedVideoEffectStage) -> Self {
        Self {
            stage_id: stage.stage_id,
            effect_id: stage.effect_id,
            source: Arc::clone(&stage.effect.source),
            source_path: stage.effect.source_path.clone(),
            bindings: stage.effect.image_bindings.clone(),
        }
    }
}

impl PoolAuthorship {
    pub(super) fn matches<'a>(
        &self,
        chains: impl Iterator<Item = &'a ResolvedVideoEffectChain>,
    ) -> bool {
        let mut stored = self.0.iter();
        for chain in chains {
            let Some(identity) = stored.next() else {
                return false;
            };
            if identity.chain_id != chain.chain_id || identity.scope != chain.scope {
                return false;
            }
            let mut stages = identity.stages.iter();
            for stage in chain.stages.iter().filter(|stage| stage.enabled) {
                if !stages.next().is_some_and(|old| old.matches(stage)) {
                    return false;
                }
            }
            if stages.next().is_some() {
                return false;
            }
        }
        stored.next().is_none()
    }

    pub(super) fn capture<'a>(chains: impl Iterator<Item = &'a ResolvedVideoEffectChain>) -> Self {
        Self(
            chains
                .map(|chain| ChainIdentity {
                    chain_id: chain.chain_id,
                    scope: chain.scope.clone(),
                    stages: chain
                        .stages
                        .iter()
                        .filter(|stage| stage.enabled)
                        .map(StageIdentity::new)
                        .collect(),
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_gpu_pool_authorship_keeps_controls_and_invalidates_retired_shader() {
        let effect = crate::builtin_isf_effect("posterize").unwrap().unwrap();
        let chain = ResolvedVideoEffectChain {
            chain_id: Some(VideoEffectChainId(1)),
            scope: VideoEffectScope::Layer { layer_id: 1 },
            bypassed: false,
            stages: vec![ResolvedVideoEffectStage {
                stage_id: Some(VideoEffectStageId(1)),
                effect_id: Some(VideoEffectId(1)),
                enabled: true,
                label: "stage".into(),
                effect,
            }],
        };
        let identity = PoolAuthorship::capture(std::iter::once(&chain));
        let mut controls = chain.clone();
        controls.stages[0].label = "renamed".into();
        assert!(!controls.stages[0].effect.controls.is_empty());
        controls.stages[0].effect.controls[0].value[0] += 1.0;
        assert!(identity.matches(std::iter::once(&controls)));
        let mut changed = chain.clone();
        changed.stages[0].effect.source = Arc::from("replacement shader");
        assert!(!identity.matches(std::iter::once(&changed)));
        let mut disabled = chain.clone();
        disabled.stages[0].enabled = false;
        assert!(!identity.matches(std::iter::once(&disabled)));
        let mut removed = chain.clone();
        removed.stages.clear();
        assert!(!identity.matches(std::iter::once(&removed)));
        let mut scope = chain.clone();
        scope.scope = VideoEffectScope::Layer { layer_id: 2 };
        assert!(!identity.matches(std::iter::once(&scope)));
    }
}
