//! Shared chain preparation/execution and bounded independent Layer jobs.
use super::*;

#[derive(Clone)]
pub(crate) struct PreparedStage {
    pub(crate) shader: Arc<PreparedIsfShader>,
    pub(crate) controls: Vec<[f32; 4]>,
    pub(crate) images: Vec<(String, Arc<VideoFrame>)>,
    pub(crate) fault_context: VideoEffectFaultContext,
}

#[derive(Clone)]
pub(crate) struct PreparedChain<'a> {
    pub(crate) source: Cow<'a, ResolvedVideoEffectChain>,
    pub(crate) stages: Vec<PreparedStage>,
    pub(crate) keys: Vec<isf_runtime::IsfInstanceKey>,
    pub(crate) faults: Vec<VideoEffectStageFault>,
}

impl PreparedChain<'_> {
    pub(crate) fn independent(&self) -> bool {
        !self.stages.is_empty()
            && self.stages.iter().all(|stage| {
                stage.images.is_empty()
                    && isf_runtime::IsfStageInputs {
                        shader: &stage.shader,
                        controls: &stage.controls,
                        images: &[],
                    }
                    .can_share_host_buffers()
            })
    }

    pub(crate) fn finish_faults(
        mut self,
        failures: Vec<(usize, IsfRuntimeError)>,
    ) -> Vec<VideoEffectStageFault> {
        for (index, error) in failures {
            let mut context = self.stages[index].fault_context.clone();
            context.stage_label = Some(
                self.source.stages[context.stage_index.unwrap()]
                    .label
                    .clone(),
            );
            self.faults
                .push(video_effect_fault(context, error.to_string()));
        }
        self.faults
    }
}

pub(super) struct PendingLayerChain {
    frame: Arc<VideoFrame>,
    prepared: PreparedChain<'static>,
    date: [f32; 4],
}

impl<P: VideoFrameProvider> VideoPreviewRenderer<P> {
    #[cfg(test)]
    pub(crate) fn prepare_native_cpu_invocation(
        &mut self,
        chain: ResolvedVideoEffectChain,
        input: Arc<VideoFrame>,
        render_input_key: Option<protocol::VideoRenderInputKey>,
        cancellation: Option<&dyn VideoRenderCancellation>,
    ) -> Result<crate::isf_native_gpu::PreparedNativeCpuInvocation, VideoRuntimeError> {
        check_runtime_cancellation(cancellation)?;
        let prepared = if resolved_chain_executes(&chain) {
            self.prepare_effect_chain(
                Cow::Owned(chain),
                input.layer_id,
                render_input_key,
                cancellation,
            )?
        } else {
            PreparedChain {
                source: Cow::Owned(chain),
                stages: Vec::new(),
                keys: Vec::new(),
                faults: Vec::new(),
            }
        };
        Ok(crate::isf_native_gpu::PreparedNativeCpuInvocation::new(
            input,
            prepared,
            self.isf_frame_inputs.date,
            self.isf_audio_frame.clone(),
        ))
    }

    fn prepare_effect_chain<'chain>(
        &mut self,
        chain: Cow<'chain, ResolvedVideoEffectChain>,
        layer_id: VideoLayerId,
        render_input_key: Option<protocol::VideoRenderInputKey>,
        cancellation: Option<&dyn VideoRenderCancellation>,
    ) -> Result<PreparedChain<'chain>, VideoRuntimeError> {
        let bpm = self.frame_provider.bpm();
        let mut prepared = PreparedChain {
            source: chain,
            stages: Vec::new(),
            keys: Vec::new(),
            faults: Vec::new(),
        };
        for (stage_index, stage) in prepared.source.stages.iter().enumerate() {
            check_runtime_cancellation(cancellation)?;
            if !stage.enabled {
                continue;
            }
            let fault_context = VideoEffectFaultContext {
                scope: prepared.source.scope.clone(),
                chain_id: prepared.source.chain_id,
                stage_id: stage.stage_id,
                effect_id: stage.effect_id,
                render_input_key,
                stage_index: Some(stage_index),
                stage_label: None,
            };
            let input = self
                .cached_isf_shader(
                    &stage.effect.source,
                    stage.effect.source_path.as_deref(),
                    &stage.effect.image_bindings,
                )
                .map_err(|error| error.to_string())
                .and_then(|shader| {
                    let controls = resolved_isf_control_values_from_controls(
                        &shader,
                        &stage.effect.controls,
                        bpm,
                    );
                    let images = self
                        .isf_frame_inputs
                        .resolve_layer_images(&stage.effect.image_bindings)?;
                    Ok((shader, controls, images))
                });
            match input {
                Ok((shader, controls, images)) => {
                    prepared.keys.push(isf_runtime::IsfInstanceKey {
                        namespace: self.isf_render_namespace,
                        scope: Some(prepared.source.scope.clone()),
                        input: render_input_key,
                        chain_id: prepared.source.chain_id,
                        stage_id: stage.stage_id,
                        effect_id: stage.effect_id,
                        legacy_stage_index: if stage.stage_id.is_some() {
                            0
                        } else {
                            stage_index
                        },
                        layer_id,
                    });
                    prepared.stages.push(PreparedStage {
                        shader,
                        controls,
                        images,
                        fault_context,
                    });
                }
                Err(error) => {
                    let mut context = fault_context;
                    context.stage_label = Some(stage.label.clone());
                    prepared.faults.push(video_effect_fault(context, error));
                }
            }
        }
        Ok(prepared)
    }

    fn initialize_chain_runtime(&mut self) {
        if self.isf_runtime.is_none() {
            self.isf_runtime = Some(IsfGpuRuntime::new());
        }
    }

    fn execute_prepared_chain<'frame>(
        &mut self,
        prepared: PreparedChain<'_>,
        frame: Cow<'frame, VideoFrame>,
        date: [f32; 4],
        cancellation: Option<&dyn VideoRenderCancellation>,
    ) -> Result<(Cow<'frame, VideoFrame>, Vec<VideoEffectStageFault>), VideoRuntimeError> {
        self.initialize_chain_runtime();
        let runtime = match self.isf_runtime.as_mut().expect("ISF runtime initialized") {
            Ok(runtime) => runtime,
            Err(error) => {
                let failures = (0..prepared.stages.len())
                    .map(|index| (index, error.clone()))
                    .collect();
                return Ok((frame, prepared.finish_faults(failures)));
            }
        };
        if prepared.stages.is_empty() {
            if let Err(error) = runtime.advance_stage_local_chain_without_prepared_stages() {
                let mut faults = prepared.faults;
                faults.push(video_effect_fault(
                    VideoEffectFaultContext {
                        scope: prepared.source.scope.clone(),
                        chain_id: prepared.source.chain_id,
                        stage_id: None,
                        effect_id: None,
                        render_input_key: None,
                        stage_index: None,
                        stage_label: None,
                    },
                    error.message,
                ));
                return Ok((frame, faults));
            }
            return Ok((frame, prepared.faults));
        }
        check_runtime_cancellation(cancellation)?;
        let image_refs = prepared
            .stages
            .iter()
            .map(|stage| {
                stage
                    .images
                    .iter()
                    .map(|(name, frame)| isf_runtime::IsfImageInput {
                        name,
                        frame: frame.as_ref(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let stages = prepared
            .stages
            .iter()
            .zip(&image_refs)
            .map(|(stage, images)| isf_runtime::IsfStageInputs {
                shader: &stage.shader,
                controls: &stage.controls,
                images,
            })
            .collect::<Vec<_>>();
        runtime.set_audio_frame(self.isf_audio_frame.as_ref());
        let (rendered, failures) = runtime.apply_stack_stage_local_with_inputs_and_keys(
            &frame,
            &stages,
            frame.pts_ms as f32 / 1_000.0,
            frame.duration_ms as f32 / 1_000.0,
            date,
            &prepared.keys,
        );
        check_runtime_cancellation(cancellation)?;
        Ok((Cow::Owned(rendered), prepared.finish_faults(failures)))
    }

    pub(super) fn execute_effect_chain_cow<'frame>(
        &mut self,
        chain: &ResolvedVideoEffectChain,
        frame: Cow<'frame, VideoFrame>,
        render_input_key: Option<protocol::VideoRenderInputKey>,
        cancellation: Option<&dyn VideoRenderCancellation>,
    ) -> Result<(Cow<'frame, VideoFrame>, Vec<VideoEffectStageFault>), VideoRuntimeError> {
        check_runtime_cancellation(cancellation)?;
        if !resolved_chain_executes(chain) {
            return Ok((frame, Vec::new()));
        }
        let prepared = self.prepare_effect_chain(
            Cow::Borrowed(chain),
            frame.layer_id,
            render_input_key,
            cancellation,
        )?;
        self.execute_prepared_chain(prepared, frame, self.isf_frame_inputs.date, cancellation)
    }

    /// Called only for no-callback, dependency-free Layer chains without an
    /// executing Clip/transition. Preparation is reused even if admission fails.
    pub(super) fn defer_independent_layer_chain(
        &mut self,
        pending: &mut isf_runtime::StackBatch<PendingLayerChain>,
        frame: Arc<VideoFrame>,
        chain: ResolvedVideoEffectChain,
        faults: &mut Vec<VideoEffectStageFault>,
    ) -> Result<(), VideoRuntimeError> {
        if pending
            .iter()
            .any(|job| job.frame.layer_id == frame.layer_id)
        {
            self.flush_independent_layer_chains(pending, faults);
        }
        let prepared = self.prepare_effect_chain(Cow::Owned(chain), frame.layer_id, None, None)?;
        if !prepared.independent() {
            self.flush_independent_layer_chains(pending, faults);
            self.publish_prepared_layer(frame, prepared, self.isf_frame_inputs.date, faults);
            return Ok(());
        }
        pending.push(PendingLayerChain {
            frame,
            prepared,
            date: self.isf_frame_inputs.date,
        });
        if pending.len() == 3 {
            self.flush_independent_layer_chains(pending, faults);
        }
        Ok(())
    }

    fn publish_prepared_layer(
        &mut self,
        input: Arc<VideoFrame>,
        prepared: PreparedChain<'_>,
        date: [f32; 4],
        faults: &mut Vec<VideoEffectStageFault>,
    ) {
        let (frame, chain_faults) = self
            .execute_prepared_chain(prepared, Cow::Borrowed(input.as_ref()), date, None)
            .expect("a chain without a cancellation callback cannot be cancelled");
        let frame = match frame {
            Cow::Owned(frame) => frame,
            Cow::Borrowed(_) => Arc::unwrap_or_clone(input),
        };
        self.publish_independent_result(frame, chain_faults, faults);
    }

    fn publish_independent_result(
        &mut self,
        frame: VideoFrame,
        chain_faults: Vec<VideoEffectStageFault>,
        faults: &mut Vec<VideoEffectStageFault>,
    ) {
        for fault in chain_faults {
            if !faults.contains(&fault) {
                faults.push(fault);
            }
        }
        self.runtime.push_frame(frame);
    }

    pub(super) fn flush_independent_layer_chains(
        &mut self,
        pending: &mut isf_runtime::StackBatch<PendingLayerChain>,
        faults: &mut Vec<VideoEffectStageFault>,
    ) {
        if pending.is_empty() {
            return;
        }
        self.initialize_chain_runtime();
        let results = if let Some(Ok(runtime)) = self.isf_runtime.as_mut() {
            let stages = isf_runtime::StackBatch::from_fn(pending.len(), |index| {
                let job = &pending[index];
                job.prepared
                    .stages
                    .iter()
                    .map(|stage| isf_runtime::IsfStageInputs {
                        shader: &stage.shader,
                        controls: &stage.controls,
                        images: &[],
                    })
                    .collect::<Vec<_>>()
            });
            let chains = isf_runtime::StackBatch::from_fn(pending.len(), |index| {
                let job = &pending[index];
                isf_runtime::IndependentChain {
                    frame: Arc::clone(&job.frame),
                    stages: &stages[index],
                    keys: &job.prepared.keys,
                    time_seconds: job.frame.pts_ms as f32 / 1_000.0,
                    time_delta_seconds: job.frame.duration_ms as f32 / 1_000.0,
                    date: job.date,
                }
            });
            runtime.apply_independent_chains(&chains)
        } else {
            None
        };
        match results {
            Some(results) => {
                assert_eq!(results.len(), pending.len());
                for (job, (frame, failures)) in pending.drain().zip(results) {
                    let chain_faults = job.prepared.finish_faults(failures);
                    self.publish_independent_result(frame, chain_faults, faults);
                }
            }
            None => {
                // Declined before any GPU job ran; reuse prepared shaders and
                // source Arcs. No decode, prepare or FX retry is performed.
                for job in pending.drain() {
                    self.publish_prepared_layer(job.frame, job.prepared, job.date, faults);
                }
            }
        }
    }
}
