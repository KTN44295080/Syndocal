//! Resident group FX. A group's straightened isolated composite runs its FX
//! stages inside P's CompositionSnapshot command buffer, between the member
//! composition and the group transform/outer blend. Only the group input and
//! one ping/pong pair are live, exactly like a shared-host stack, so admitted
//! stages are stateless: feedback, extra images, audio and custom targets stay
//! on the canonical route.
use super::*;
use crate::isf_chain_execution::PreparedStage;
use crate::native_gpu_budget::AllocationCharge;
use crate::native_gpu_context::NativeGpuContext;
use crate::native_gpu_contract::FrozenChainClocks;
use std::sync::Arc;

/// Every stage samples at most the group image and writes the full extent.
pub(crate) fn resident_group_stages(stages: &[PreparedStage]) -> bool {
    !stages.is_empty()
        && stages.iter().all(|stage| {
            let inputs = IsfStageInputs {
                shader: &stage.shader,
                controls: &stage.controls,
                images: &[],
            };
            stage.images.is_empty()
                && inputs.can_share_host_buffers()
                && inputs.native_target_restriction().is_none()
        })
}

/// Pass uniforms in encode order (stage-major), byte-identical to the
/// canonical stack's for the same extent and clocks.
pub(crate) fn resident_group_uniforms(
    stages: &[PreparedStage],
    extent: (u32, u32),
    clocks: FrozenChainClocks,
) -> Vec<Vec<u8>> {
    stages
        .iter()
        .flat_map(|stage| {
            (0..stage.shader.passes.len()).map(move |pass| {
                isf_uniform_data(
                    extent.0,
                    extent.1,
                    f32::from_bits(clocks.time_seconds_bits),
                    f32::from_bits(clocks.time_delta_seconds_bits),
                    clocks.frame_index,
                    pass,
                    clocks.date_bits.map(f32::from_bits),
                    &stage.controls,
                )
            })
        })
        .collect()
}

/// The canonical ISF sampler, created once by P at bootstrap.
pub(crate) fn resident_group_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    stack_workspace::isf_sampler(device)
}

/// The stage ping/pong. P's unpack pass writes the group input into `pong`,
/// which stage 0 samples while rendering `ping`; stage 1 may then overwrite
/// `pong`. The charges live exactly as long as the textures, which P's
/// composed resources retain until the composition submission retires.
pub(crate) struct ResidentGroupFxTargets {
    targets: [GpuImage; 2],
    _charges: Option<[AllocationCharge; 2]>,
}

impl ResidentGroupFxTargets {
    /// The composition phase separately pins the pool owner and its aliases.
    pub(crate) fn borrowed(targets: [GpuImage; 2]) -> Self {
        Self {
            targets,
            _charges: None,
        }
    }

    pub(crate) fn new(
        device: &wgpu::Device,
        (width, height): (u32, u32),
        charges: [AllocationCharge; 2],
    ) -> Self {
        let ping = GpuImage::new(
            device,
            width,
            height,
            false,
            "Syndocal resident group FX ping",
        );
        let pong = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Syndocal resident group FX input and pong"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: target_format(false),
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });
        let pong = GpuImage {
            view: pong.create_view(&wgpu::TextureViewDescriptor::default()),
            texture: pong,
            width,
            height,
            float: false,
            native_origin: None,
            native_charge: None,
        };
        Self {
            targets: [ping, pong],
            _charges: Some(charges),
        }
    }

    pub(crate) fn input_view(&self) -> &wgpu::TextureView {
        &self.targets[1].view
    }

    /// Stage `i` renders into `targets[i % 2]` and samples the other one.
    pub(crate) fn result_view(&self, stages: usize) -> &wgpu::TextureView {
        &self.targets[stages.saturating_sub(1) % 2].view
    }
}

impl IsfGpuRuntime {
    /// One canonical FRAMEINDEX per executing group chain, from the same
    /// logical clock the canonical stack consumes.
    pub(crate) fn reserve_resident_group_frame_index(&mut self) -> u32 {
        let frame_index = self.frame_index;
        self.advance_frame_index(1);
        frame_index
    }

    /// Encodes every admitted pass into P's encoder. The caller's phase pins
    /// the returned pipelines until its submission retires.
    pub(crate) fn encode_resident_group_fx(
        &mut self,
        context: &Arc<NativeGpuContext>,
        encoder: &mut wgpu::CommandEncoder,
        targets: &ResidentGroupFxTargets,
        sampler: &wgpu::Sampler,
        stages: &[PreparedStage],
        uniforms: &[&wgpu::Buffer],
        pins: &mut Vec<Arc<IsfGpuPipeline>>,
    ) -> Result<(), IsfRuntimeError> {
        if self
            .native_context()
            .is_none_or(|own| !Arc::ptr_eq(own, context))
        {
            return Err(runtime_error(
                "resident group FX requires its shared device phase",
            ));
        }
        if !resident_group_stages(stages) {
            return Err(runtime_error(
                "resident group FX received a stage outside its admission",
            ));
        }
        let mut uniforms = uniforms.iter();
        let mut current = targets.input_view();
        for (index, stage) in stages.iter().enumerate() {
            let texture_count = self.ensure_stage_pipelines(&stage.shader, Some(pins))?;
            let output = &targets.targets[index % 2];
            let sampled = std::iter::repeat_n(current, texture_count).collect::<Vec<_>>();
            for _ in &stage.shader.passes {
                let uniform = uniforms.next().ok_or_else(|| {
                    runtime_error("resident group FX uniforms do not match its passes")
                })?;
                self.encode_pass(
                    encoder,
                    &stage.shader.wgsl,
                    &sampled,
                    sampler,
                    uniform,
                    output,
                );
            }
            current = &output.view;
        }
        if uniforms.next().is_some() {
            return Err(runtime_error(
                "resident group FX uniforms do not match its passes",
            ));
        }
        Ok(())
    }
}
