//! Pass scheduling and GPU resource ownership for the ISF runtime.
//! Every named buffer has separate read/write textures. Feedback is owned by
//! an effect instance, never by a compiled shader or by a control value.
use std::{borrow::Cow, collections::BTreeMap};
use wgpu::util::DeviceExt;

use super::{
    convert_frame_to_rgba8, isf_uniform_data, runtime_error, runtime_from_composite_error,
    source_hash, IsfGpuPipeline, IsfGpuRuntime, IsfInstanceKey, IsfRuntimeError, IsfStageInputs,
    PreparedIsfShader, VideoFrame, VideoPixelFormat, ISF_GPU_PIPELINE_CAPACITY,
};

const RESOURCE_CACHE_CAPACITY: usize = 64;
const MAX_STAGE_RESOURCE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_CACHED_RESOURCE_BYTES: u64 = 512 * 1024 * 1024;

fn resource_eviction_candidate<T>(
    entries: &BTreeMap<IsfInstanceKey, T>,
    reserved_bytes: u64,
    protected: Option<&IsfInstanceKey>,
    metadata: impl Fn(&T) -> (u64, u64),
) -> Option<IsfInstanceKey> {
    let mut count = 0;
    let mut bytes = 0_u64;
    let mut oldest: Option<(&IsfInstanceKey, u64)> = None;
    for (key, state) in entries {
        if protected == Some(key) {
            continue;
        }
        let (capacity, last_used) = metadata(state);
        count += 1;
        bytes = bytes.saturating_add(capacity);
        if oldest.is_none_or(|(_, oldest_use)| last_used < oldest_use) {
            oldest = Some((key, last_used));
        }
    }
    if count > RESOURCE_CACHE_CAPACITY
        || bytes.saturating_add(reserved_bytes) > MAX_CACHED_RESOURCE_BYTES
    {
        oldest.map(|(key, _)| key.clone())
    } else {
        None
    }
}

#[path = "isf_stack_workspace.rs"]
mod stack_workspace;
pub(super) use stack_workspace::queue_upload_pitch;
use stack_workspace::StackWorkspace;

#[path = "isf_stack_transaction.rs"]
mod stack_transaction;
pub(super) use stack_transaction::IndependentStack;
use stack_transaction::PendingStack;

#[cfg(test)]
#[path = "isf_stack_phase_timing.rs"]
mod phase_timing;

#[derive(Default)]
pub(super) struct IsfResourcePool {
    pub(super) queue_upload_pitch: Option<u64>,
    // Default standalone/test fixtures retain their existing construction.
    // Only with_native_context associates the pool with the shared broker.
    #[cfg(test)]
    pub(super) native_context: Option<std::sync::Arc<crate::native_gpu_context::NativeGpuContext>>,
    // Identifies this logical runtime and a non-wrapping FRAMEINDEX epoch.
    // Default fixtures get their own token without another runtime constructor.
    #[cfg(test)]
    pub(super) native_clock_epoch: std::sync::Arc<()>,
    #[cfg(test)]
    pub(super) native_cpu_inputs:
        Vec<std::sync::Weak<crate::native_gpu_result::NativeCpuFrameOwner>>,
    entries: BTreeMap<IsfInstanceKey, StageResources>,
    clock: u64,
    workspace: Option<StackWorkspace>,
    batch_workspaces: Vec<StackWorkspace>,
    poisoned_transaction: Option<stack_transaction::PoisonedStackTransaction>,
    #[cfg(test)]
    phase_timing: Option<phase_timing::StackPhaseTiming>,
    #[cfg(test)]
    phase_timing_suppressed: bool,
    #[cfg(test)]
    phase_sequence: u64,
    #[cfg(test)]
    independent_batch_count: u64,
    #[cfg(test)]
    stack_upload_count: u64,
    #[cfg(test)]
    stack_submit_count: u64,
}

#[derive(Clone)]
pub(crate) struct GpuImage {
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) float: bool,
    #[cfg(test)]
    native_origin: Option<crate::native_gpu_contract::GpuContextId>,
    #[cfg(test)]
    native_charge: Option<std::sync::Arc<crate::native_gpu_budget::AllocationCharge>>,
}

impl GpuImage {
    fn new(device: &wgpu::Device, width: u32, height: u32, float: bool, label: &str) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: target_format(float),
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture,
            view,
            width,
            height,
            float,
            #[cfg(test)]
            native_origin: None,
            #[cfg(test)]
            native_charge: None,
        }
    }

    fn upload_rgba(&self, queue: &wgpu::Queue, data: &[u8]) {
        #[cfg(feature = "qa-native-frame-timing")]
        let _qa_upload =
            crate::native_frame_timing::span(crate::native_frame_timing::Phase::IsfUpload);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.width * 4),
                rows_per_image: Some(self.height),
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
    }

    #[cfg(test)]
    pub(crate) fn native_origin(&self) -> Option<crate::native_gpu_contract::GpuContextId> {
        self.native_origin
    }
}

struct TargetBuffer {
    name: String,
    persistent: bool,
    images: [GpuImage; 2],
    front: usize,
}

struct StageResources {
    fingerprint: u64,
    sizes: Vec<(u32, u32)>,
    targets: Vec<TargetBuffer>,
    imports: Vec<GpuImage>,
    bound_images: Vec<GpuImage>,
    audio: Vec<GpuImage>,
    audio_identity: Option<(u64, u64)>,
    audio_initialized: bool,
    bytes: u64,
    transient_bytes: u64,
    last_used: u64,
    has_history: bool,
}

struct StageResourceLayout {
    sizes: Vec<(u32, u32)>,
    bytes: u64,
    transient_bytes: u64,
}

#[cfg(test)]
struct NativeCpuAllocationScope<'a, 'b> {
    phase: &'a mut crate::native_gpu_context::GpuPhaseEncoder<'b>,
    buffers: Vec<std::sync::Arc<NativeCpuBuffer>>,
    failure: Option<crate::native_gpu_contract::NativeGpuFault>,
}
#[cfg(test)]
struct NativeCpuBuffer {
    buffer: wgpu::Buffer,
    _charge: std::sync::Arc<crate::native_gpu_budget::AllocationCharge>,
}
#[cfg(test)]
impl NativeCpuAllocationScope<'_, '_> {
    fn charge(
        &mut self,
        kind: crate::native_gpu_budget::ChargeKind,
        bytes: u64,
    ) -> Result<std::sync::Arc<crate::native_gpu_budget::AllocationCharge>, IsfRuntimeError> {
        let result = self.phase.bind_allocation(kind, bytes);
        match result {
            Ok(charge) => {
                let charge = std::sync::Arc::new(charge);
                if let Err(fault) = self
                    .phase
                    .retain_resource(std::sync::Arc::clone(&charge), &[charge.id()])
                {
                    self.failure = Some(crate::native_gpu_result::copy_fault(&fault));
                    return Err(runtime_error(fault.message));
                }
                Ok(charge)
            }
            Err(fault) => {
                self.failure = Some(crate::native_gpu_result::copy_fault(&fault));
                Err(runtime_error(fault.message))
            }
        }
    }

    fn retain_image(&mut self, image: &GpuImage) -> Result<(), IsfRuntimeError> {
        let charge = image
            .native_charge
            .as_ref()
            .ok_or_else(|| runtime_error("shared CPU stage contains an uncharged texture"))?;
        if let Err(fault) = self
            .phase
            .retain_resource(std::sync::Arc::new(image.clone()), &[charge.id()])
        {
            self.failure = Some(crate::native_gpu_result::copy_fault(&fault));
            return Err(runtime_error(fault.message));
        }
        Ok(())
    }

    fn buffer(
        &mut self,
        device: &wgpu::Device,
        contents: Option<&[u8]>,
        bytes: u64,
        kind: crate::native_gpu_budget::ChargeKind,
        usage: wgpu::BufferUsages,
        label: &str,
    ) -> Result<wgpu::Buffer, IsfRuntimeError> {
        let charge = self.charge(kind, bytes)?;
        let buffer = match contents {
            Some(contents) => device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage,
            }),
            None => device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: bytes,
                usage,
                mapped_at_creation: false,
            }),
        };
        let retained = std::sync::Arc::new(NativeCpuBuffer {
            buffer: buffer.clone(),
            _charge: charge,
        });
        if let Err(fault) = self
            .phase
            .retain_resource(std::sync::Arc::clone(&retained), &[retained._charge.id()])
        {
            self.failure = Some(crate::native_gpu_result::copy_fault(&fault));
            return Err(runtime_error(fault.message));
        }
        self.buffers.push(retained);
        Ok(buffer)
    }
}

fn planned_image(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    float: bool,
    label: &str,
    #[cfg(test)] native: &mut Option<&mut NativeCpuAllocationScope<'_, '_>>,
    #[cfg(test)] cached: bool,
) -> Result<GpuImage, IsfRuntimeError> {
    #[cfg(test)]
    if let Some(native) = native.as_deref_mut() {
        use crate::native_gpu_budget::ChargeKind;
        let bytes = u64::from(width) * u64::from(height) * if float { 16 } else { 4 };
        let charge = native.charge(
            if cached {
                ChargeKind::CachedEffectResource
            } else {
                ChargeKind::IsfScratch
            },
            bytes,
        )?;
        let mut image = GpuImage::new(device, width, height, float, label);
        image.native_origin = Some(native.phase.context().id());
        image.native_charge = Some(charge);
        native.retain_image(&image)?;
        return Ok(image);
    }
    Ok(GpuImage::new(device, width, height, float, label))
}

fn planned_upload(
    image: &GpuImage,
    queue: &wgpu::Queue,
    data: &[u8],
    #[cfg(test)] native: &mut Option<&mut NativeCpuAllocationScope<'_, '_>>,
) -> Result<(), IsfRuntimeError> {
    #[cfg(test)]
    if let Some(native) = native.as_deref_mut() {
        let bytes = crate::native_gpu_budget::checked_upload_staging_bytes(
            image.width,
            image.height,
            native.phase.context().capabilities().upload_row_pitch,
        )
        .map_err(|reason| runtime_error(format!("native upload pitch: {reason:?}")))?;
        native.charge(
            crate::native_gpu_budget::ChargeKind::QueueUploadStaging,
            bytes,
        )?;
        native.phase.mark_queue_write();
    }
    image.upload_rgba(queue, data);
    Ok(())
}

fn planned_rgba<'a>(
    frame: &'a VideoFrame,
    #[cfg(test)] native: &mut Option<&mut NativeCpuAllocationScope<'_, '_>>,
) -> Result<Cow<'a, VideoFrame>, IsfRuntimeError> {
    if frame.format == VideoPixelFormat::Rgba8 {
        return Ok(Cow::Borrowed(frame));
    }
    #[cfg(test)]
    if let Some(native) = native.as_deref_mut() {
        native.charge(
            crate::native_gpu_budget::ChargeKind::ActiveCpuInput,
            u64::from(frame.width) * u64::from(frame.height) * 4,
        )?;
    }
    Ok(Cow::Owned(
        convert_frame_to_rgba8(frame).map_err(runtime_from_composite_error)?,
    ))
}

/// The live input is the configured mono mix: one row, bounded columns.
/// Waveform zero is 0.5; FFT is a normalized linear magnitude, silence is 0.
/// Downsampled FFT columns keep peaks, and waveform columns interpolate.
fn audio_texture_rgba(
    input: &super::IsfAudioInputDefinition,
    frame: Option<&protocol::LiveAudioTextureFrame>,
) -> [u8; protocol::LIVE_AUDIO_TEXTURE_SAMPLES * 4] {
    let mut rgba = [0; protocol::LIVE_AUDIO_TEXTURE_SAMPLES * 4];
    let width = input.width as usize;
    for x in 0..width {
        let value = match frame {
            None => {
                if input.fft {
                    0.0
                } else {
                    0.5
                }
            }
            Some(frame) if input.fft => {
                let start = x * protocol::LIVE_AUDIO_TEXTURE_SAMPLES / width;
                let end = ((x + 1) * protocol::LIVE_AUDIO_TEXTURE_SAMPLES / width).max(start + 1);
                frame.fft[start..end]
                    .iter()
                    .copied()
                    .fold(0.0_f32, f32::max)
            }
            Some(frame) if width == 1 => {
                let rms = (frame
                    .waveform
                    .iter()
                    .map(|value| value * value)
                    .sum::<f32>()
                    / protocol::LIVE_AUDIO_TEXTURE_SAMPLES as f32)
                    .sqrt();
                rms * 0.5 + 0.5
            }
            Some(frame) => {
                let position = x as f32 * (protocol::LIVE_AUDIO_TEXTURE_SAMPLES - 1) as f32
                    / (width - 1) as f32;
                let left = position.floor() as usize;
                let right = (left + 1).min(protocol::LIVE_AUDIO_TEXTURE_SAMPLES - 1);
                let value = frame.waveform[left]
                    + (frame.waveform[right] - frame.waveform[left]) * (position - left as f32);
                value * 0.5 + 0.5
            }
        };
        let byte = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        rgba[x * 4..x * 4 + 4].copy_from_slice(&[byte, byte, byte, 255]);
    }
    rgba
}

fn target_format(float: bool) -> wgpu::TextureFormat {
    if float {
        wgpu::TextureFormat::Rgba32Float
    } else {
        wgpu::TextureFormat::Rgba8Unorm
    }
}

fn clear_image(encoder: &mut wgpu::CommandEncoder, image: &GpuImage) {
    let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Syndocal ISF buffer initialization"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &image.view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
}

fn copy_rgba_readback(
    mapped: &[u8],
    row_bytes: usize,
    padded_row_bytes: usize,
    expected_len: usize,
) -> Vec<u8> {
    // Aligned rows already form one packed frame. One contiguous copy avoids
    // a separate Vec length/capacity check and copy for every scanline.
    if row_bytes == padded_row_bytes {
        return mapped[..expected_len].to_vec();
    }
    let mut data = Vec::with_capacity(expected_len);
    for row in mapped.chunks_exact(padded_row_bytes) {
        data.extend_from_slice(&row[..row_bytes]);
    }
    data
}

impl IsfGpuRuntime {
    fn commit_stage_states(
        &mut self,
        states: impl IntoIterator<Item = (IsfInstanceKey, StageResources)>,
    ) {
        for (key, state) in states {
            if state.bytes != 0 {
                self.resources.entries.insert(key, state);
            }
        }
        while self.resources.entries.len() > RESOURCE_CACHE_CAPACITY {
            let oldest = self
                .resources
                .entries
                .iter()
                .min_by_key(|(_, state)| state.last_used)
                .map(|(key, _)| key.clone())
                .expect("nonempty cache");
            self.resources.entries.remove(&oldest);
        }
    }

    fn invalidate_stage_states(&mut self, keys: &[IsfInstanceKey]) {
        for key in keys {
            self.resources.entries.remove(key);
        }
    }

    /// One encoder for ordinary single stacks and independent batch tickets.
    /// `batch_reservation` includes every live scratch/input/output in the group;
    /// its preflight has already proved that feedback eviction is unnecessary.
    fn encode_pass_stack(
        &mut self,
        pending: &mut PendingStack,
        frame: &VideoFrame,
        stages: &[IsfStageInputs<'_>],
        batch_reservation: Option<u64>,
        #[cfg(test)] mut native: Option<&mut NativeCpuAllocationScope<'_, '_>>,
    ) -> Result<(), IsfRuntimeError> {
        // Uploads are explicit children; encode exclusive time excludes their host wall.
        #[cfg(feature = "qa-native-frame-timing")]
        let _qa_encode =
            crate::native_frame_timing::span(crate::native_frame_timing::Phase::IsfEncode);
        let keys = pending.keys.as_ref();
        let stack_transaction::StackClocks {
            time_seconds,
            time_delta_seconds,
            frame_index,
            date,
        } = pending.clocks;
        if keys.len() != stages.len() {
            return Err(runtime_error(
                "ISF stage ownership key count does not match the stack",
            ));
        }
        self.validate_pass_size(frame.width, frame.height)?;
        let padded_bytes_per_row = StackWorkspace::bytes_per_row(frame.width);
        // The encoder retains each intermediate allocation until submission.
        // Reserve upload, ping/pong and padded readback before creating them.
        let baseline_bytes = batch_reservation
            .unwrap_or_else(|| StackWorkspace::required_bytes(frame.width, frame.height));
        if baseline_bytes > MAX_CACHED_RESOURCE_BYTES {
            return Err(runtime_error(
                "ISF pass size exceeds the GPU buffer memory budget",
            ));
        }
        if batch_reservation.is_none() {
            self.evict_resources_for(baseline_bytes);
        }
        let expected_len = frame.width as usize * frame.height as usize * 4;
        // Upload borrows RGBA bytes; only format conversion owns a temporary.
        let rgba = planned_rgba(
            frame,
            #[cfg(test)]
            &mut native,
        )?;
        if rgba.data.len() != expected_len {
            return Err(runtime_error(
                "ISF input frame byte length does not match its dimensions",
            ));
        }
        // filter drops a mismatched old allocation before creating its
        // replacement; scratch is never retained per layer or per resolution.
        pending.workspace = Some(
            pending
                .workspace
                .take()
                .filter(|workspace| workspace.matches(frame.width, frame.height))
                .unwrap_or_else(|| StackWorkspace::new(&self.device, frame.width, frame.height)),
        );
        let workspace = pending.workspace.as_ref().unwrap();
        let input = &workspace.input;
        let ping = &workspace.ping;
        let pong = &workspace.pong;
        let sampler = &workspace.sampler;
        let readback = workspace.readback.as_ref();
        #[cfg(test)]
        if let Some(timing) = self.resources.phase_timing.as_mut() {
            timing.mark(1);
        }
        planned_upload(
            input,
            &self.queue,
            &rgba.data,
            #[cfg(test)]
            &mut native,
        )?;
        #[cfg(test)]
        {
            self.resources.stack_upload_count += 1;
        }
        #[cfg(test)]
        if let Some(timing) = self.resources.phase_timing.as_mut() {
            timing.mark(2);
        }
        pending.encoder = Some(self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Syndocal ISF pass stack encoder"),
            },
        ));
        let encoder = pending.encoder.as_mut().unwrap();
        let mut current = input.clone();
        let mut pending_bytes = baseline_bytes;
        for (stage_index, (stage, key)) in stages.iter().zip(keys).enumerate() {
            let shader = stage.shader;
            let controls = stage.controls;
            let texture_count = self.ensure_stage_pipelines(
                shader,
                #[cfg(test)]
                if !pending.native_uniforms.is_empty() || native.is_some() {
                    Some(&mut pending.native_pipeline_pins)
                } else {
                    None
                },
            )?;
            let needs_transparent =
                shader
                    .image_input_names
                    .iter()
                    .enumerate()
                    .any(|(index, name)| {
                        !shader.bound_images.iter().any(|image| &image.name == name)
                            && !stage.images.iter().any(|image| image.name == name.as_str())
                            && (index != 0 || shader.image_binding_names.contains(name))
                    });
            let dynamic_bytes = self.dynamic_image_bytes(stage, needs_transparent)?;
            // Validate and convert every CPU input before taking cached state or
            // uploading any secondary texture.
            let dynamic_rgba = stage
                .images
                .iter()
                .map(|input| {
                    let image = input.frame;
                    let rgba = planned_rgba(
                        image,
                        #[cfg(test)]
                        &mut native,
                    )?;
                    if rgba.data.len() != image.width as usize * image.height as usize * 4 {
                        return Err(runtime_error(
                            "ISF image input byte length does not match its dimensions",
                        ));
                    }
                    Ok(rgba)
                })
                .collect::<Result<Vec<_>, IsfRuntimeError>>()?;
            let mut local_state = self.take_stage_resources(
                shader,
                controls,
                frame,
                key,
                pending_bytes,
                dynamic_bytes,
                batch_reservation.is_some(),
                #[cfg(test)]
                &mut native,
            )?;
            // Taking cached feedback transfers ownership to the ticket before
            // later validation/encoding can fail. Cleanup retains that owner.
            let state = if local_state.bytes != 0 {
                pending.states.push((key.clone(), local_state));
                &mut pending.states.last_mut().unwrap().1
            } else {
                // Resource-free metadata has no GPU owner to retain.
                &mut local_state
            };
            let dynamic_images = dynamic_rgba
                .iter()
                .map(|rgba| {
                    let gpu = planned_image(
                        &self.device,
                        rgba.width,
                        rgba.height,
                        false,
                        "Syndocal ISF image input",
                        #[cfg(test)]
                        &mut native,
                        #[cfg(test)]
                        false,
                    )?;
                    planned_upload(
                        &gpu,
                        &self.queue,
                        &rgba.data,
                        #[cfg(test)]
                        &mut native,
                    )?;
                    Ok(gpu)
                })
                .collect::<Result<Vec<_>, IsfRuntimeError>>()?;
            pending.images.extend(dynamic_images.iter().cloned());
            let transparent = if needs_transparent {
                let image = planned_image(
                    &self.device,
                    1,
                    1,
                    false,
                    "Syndocal ISF unbound image",
                    #[cfg(test)]
                    &mut native,
                    #[cfg(test)]
                    false,
                )?;
                planned_upload(
                    &image,
                    &self.queue,
                    &[0, 0, 0, 0],
                    #[cfg(test)]
                    &mut native,
                )?;
                Some(image)
            } else {
                None
            };
            pending.images.extend(transparent.iter().cloned());
            let audio_identity = self
                .audio_frame
                .as_ref()
                .map(|frame| (frame.generation, frame.feature_sequence));
            if !state.audio_initialized || state.audio_identity != audio_identity {
                for (input, image) in shader.audio_inputs.iter().zip(&state.audio) {
                    let pixels = audio_texture_rgba(input, self.audio_frame.as_ref());
                    planned_upload(
                        image,
                        &self.queue,
                        &pixels[..input.width as usize * 4],
                        #[cfg(test)]
                        &mut native,
                    )?;
                }
                state.audio_identity = audio_identity;
                state.audio_initialized = true;
            }
            for target in &state.targets {
                if !target.persistent || !state.has_history {
                    clear_image(encoder, &target.images[target.front]);
                }
            }
            let stage_output = if stage_index % 2 == 0 { ping } else { pong };
            let stage_input = current.clone();
            for (pass_index, pass) in shader.passes.iter().enumerate() {
                let (width, height) = state.sizes[pass_index];
                let target_index = pass
                    .target
                    .as_ref()
                    .and_then(|name| state.targets.iter().position(|target| &target.name == name));
                let output = if let Some(index) = target_index {
                    let target = &state.targets[index];
                    target.images[1 - target.front].clone()
                } else if width == frame.width && height == frame.height && !pass.float {
                    stage_output.clone()
                } else {
                    let image = planned_image(
                        &self.device,
                        width,
                        height,
                        pass.float,
                        "Syndocal ISF unnamed pass",
                        #[cfg(test)]
                        &mut native,
                        #[cfg(test)]
                        false,
                    )?;
                    pending.images.push(image.clone());
                    image
                };
                let mut sampled = Vec::with_capacity(texture_count);
                for (index, name) in shader.image_input_names.iter().enumerate() {
                    let image = if let Some(bound) = shader
                        .bound_images
                        .iter()
                        .position(|image| &image.name == name)
                    {
                        &state.bound_images[bound]
                    } else if let Some(dynamic) = stage
                        .images
                        .iter()
                        .position(|image| image.name == name.as_str())
                    {
                        &dynamic_images[dynamic]
                    } else if index == 0 && !shader.image_binding_names.contains(name) {
                        &stage_input
                    } else {
                        transparent
                            .as_ref()
                            .ok_or_else(|| runtime_error("ISF unbound image texture is missing"))?
                    };
                    sampled.push(&image.view);
                }
                for target in &state.targets {
                    sampled.push(&target.images[target.front].view);
                }
                sampled.extend(state.imports.iter().map(|image| &image.view));
                sampled.extend(state.audio.iter().map(|image| &image.view));
                #[cfg(test)]
                let resident_uniform = (!pending.native_uniforms.is_empty())
                    .then(|| pending.native_uniforms[stage_index].clone());
                #[cfg(not(test))]
                let resident_uniform: Option<wgpu::Buffer> = None;
                let uniforms = match resident_uniform {
                    Some(uniform) => uniform,
                    None => self.planned_pass_uniforms(
                        width,
                        height,
                        time_seconds,
                        time_delta_seconds,
                        frame_index,
                        pass_index,
                        date,
                        controls,
                        #[cfg(test)]
                        &mut native,
                    )?,
                };
                self.encode_pass(encoder, &shader.wgsl, &sampled, sampler, &uniforms, &output);
                if let Some(index) = target_index {
                    state.targets[index].front = 1 - state.targets[index].front;
                }
                current = output;
            }
            // The host frame keeps its requested dimensions/format, including
            // a final named FLOAT target or a size expression on the last pass.
            if current.width != frame.width || current.height != frame.height || current.float {
                self.ensure_pass_pipeline(BLIT_SHADER, 1, false)?;
                #[cfg(test)]
                if !pending.native_uniforms.is_empty() || native.is_some() {
                    pending.native_pipeline_pins.push(std::sync::Arc::clone(
                        self.pipelines
                            .get(&(source_hash(BLIT_SHADER), false, 1))
                            .expect("the normalization pipeline was just ensured"),
                    ));
                }
                let uniforms = self.planned_pass_uniforms(
                    frame.width,
                    frame.height,
                    time_seconds,
                    time_delta_seconds,
                    frame_index,
                    0,
                    date,
                    &[],
                    #[cfg(test)]
                    &mut native,
                )?;
                self.encode_pass(
                    encoder,
                    BLIT_SHADER,
                    &[&current.view],
                    sampler,
                    &uniforms,
                    stage_output,
                );
                current = stage_output.clone();
            }
            state.has_history = true;
            pending_bytes += state.bytes + state.transient_bytes;
        }
        if let Some(readback) = readback {
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &current.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(padded_bytes_per_row),
                        rows_per_image: Some(frame.height),
                    },
                },
                wgpu::Extent3d {
                    width: frame.width,
                    height: frame.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        #[cfg(test)]
        if let Some(timing) = self.resources.phase_timing.as_mut() {
            timing.mark(3);
        }
        Ok(())
    }

    fn dynamic_image_bytes(
        &self,
        stage: &IsfStageInputs<'_>,
        needs_transparent: bool,
    ) -> Result<u64, IsfRuntimeError> {
        for (name, layer_id) in &stage.shader.required_layer_images {
            if !stage.images.iter().any(|input| input.name == name.as_str()) {
                return Err(runtime_error(format!(
                    "ISF image input layer {layer_id} is unavailable in this render context for '{name}'"
                )));
            }
        }
        if stage.images.len() > stage.shader.image_input_names.len() {
            return Err(runtime_error(
                "ISF image input count exceeds declared inputs",
            ));
        }
        let mut names = std::collections::HashSet::new();
        let mut bytes = if needs_transparent { 4 } else { 0 };
        for input in stage.images {
            if !stage
                .shader
                .image_input_names
                .iter()
                .any(|name| name.as_str() == input.name)
            {
                return Err(runtime_error(format!(
                    "ISF image input '{}' is not declared",
                    input.name
                )));
            }
            if !names.insert(input.name) {
                return Err(runtime_error(format!(
                    "ISF image input '{}' is duplicated",
                    input.name
                )));
            }
            if stage
                .shader
                .bound_images
                .iter()
                .any(|image| image.name == input.name)
            {
                return Err(runtime_error(format!(
                    "ISF image input '{}' already has an imported still",
                    input.name
                )));
            }
            let frame = input.frame;
            self.validate_pass_size(frame.width, frame.height)?;
            let required = u64::from(frame.width) * u64::from(frame.height) * 4;
            if frame.format == VideoPixelFormat::Rgba8 && frame.data.len() as u64 != required {
                return Err(runtime_error(
                    "ISF image input byte length does not match its dimensions",
                ));
            }
            bytes += required;
            if bytes > MAX_STAGE_RESOURCE_BYTES {
                return Err(runtime_error(
                    "ISF image inputs exceed the GPU buffer memory budget",
                ));
            }
        }
        Ok(bytes)
    }

    fn evict_resources_for(&mut self, reserved_bytes: u64) {
        while let Some(oldest) =
            resource_eviction_candidate(&self.resources.entries, reserved_bytes, None, |state| {
                (state.bytes, state.last_used)
            })
        {
            self.resources.entries.remove(&oldest);
        }
    }

    fn validate_pass_size(&self, width: u32, height: u32) -> Result<(), IsfRuntimeError> {
        let max = self.device.limits().max_texture_dimension_2d;
        if width == 0 || height == 0 || width > max || height > max {
            return Err(runtime_error(format!(
                "ISF pass size {width}x{height} is invalid; dimensions must be 1..={max}"
            )));
        }
        Ok(())
    }

    fn stage_resource_layout(
        &self,
        shader: &PreparedIsfShader,
        controls: &[[f32; 4]],
        frame: &VideoFrame,
        pending_bytes: u64,
        dynamic_bytes: u64,
    ) -> Result<StageResourceLayout, IsfRuntimeError> {
        let mut sizes = Vec::with_capacity(shader.passes.len());
        let mut bytes = 0_u64;
        let mut transient_bytes = dynamic_bytes;
        for pass in &shader.passes {
            let width = match &pass.width {
                Some(expression) => expression
                    .evaluate(frame.width, frame.height, &shader.controls, controls)
                    .map_err(|error| runtime_error(error.message))?,
                None => frame.width,
            };
            let height = match &pass.height {
                Some(expression) => expression
                    .evaluate(frame.width, frame.height, &shader.controls, controls)
                    .map_err(|error| runtime_error(error.message))?,
                None => frame.height,
            };
            self.validate_pass_size(width, height)?;
            if pass.target.is_some() {
                bytes += u64::from(width) * u64::from(height) * if pass.float { 32 } else { 8 };
            } else if width != frame.width || height != frame.height || pass.float {
                transient_bytes +=
                    u64::from(width) * u64::from(height) * if pass.float { 16 } else { 4 };
            }
            sizes.push((width, height));
        }
        for image in shader.imported_images.iter().chain(&shader.bound_images) {
            self.validate_pass_size(image.width, image.height)?;
            bytes += u64::from(image.width) * u64::from(image.height) * 4;
        }
        bytes += shader
            .audio_inputs
            .iter()
            .map(|input| u64::from(input.width) * 4)
            .sum::<u64>();
        if bytes + transient_bytes > MAX_STAGE_RESOURCE_BYTES
            || bytes + transient_bytes + pending_bytes > MAX_CACHED_RESOURCE_BYTES
        {
            return Err(runtime_error(
                "ISF pass size exceeds the GPU buffer memory budget",
            ));
        }
        Ok(StageResourceLayout {
            sizes,
            bytes,
            transient_bytes,
        })
    }

    fn take_stage_resources(
        &mut self,
        shader: &PreparedIsfShader,
        controls: &[[f32; 4]],
        frame: &VideoFrame,
        key: &IsfInstanceKey,
        pending_bytes: u64,
        dynamic_bytes: u64,
        preserve_cache: bool,
        #[cfg(test)] native: &mut Option<&mut NativeCpuAllocationScope<'_, '_>>,
    ) -> Result<StageResources, IsfRuntimeError> {
        let StageResourceLayout {
            sizes,
            bytes,
            transient_bytes,
        } = self.stage_resource_layout(shader, controls, frame, pending_bytes, dynamic_bytes)?;
        self.resources.clock = self.resources.clock.wrapping_add(1);
        let last_used = self.resources.clock;
        let reusable = self
            .resources
            .entries
            .remove(key)
            .filter(|state| state.fingerprint == shader.fingerprint && state.sizes == sizes);
        // A cache hit still allocates anonymous pass textures on this frame.
        if preserve_cache {
            let cached_bytes = self
                .resources
                .entries
                .values()
                .map(|state| state.bytes)
                .sum::<u64>();
            if cached_bytes + pending_bytes + bytes + transient_bytes > MAX_CACHED_RESOURCE_BYTES {
                return Err(runtime_error(
                    "ISF independent stack reservation changed during encoding",
                ));
            }
        } else {
            self.evict_resources_for(pending_bytes + bytes + transient_bytes);
        }
        if let Some(mut state) = reusable {
            #[cfg(test)]
            if let Some(native) = native.as_deref_mut() {
                for image in state
                    .targets
                    .iter()
                    .flat_map(|target| &target.images)
                    .chain(&state.imports)
                    .chain(&state.bound_images)
                    .chain(&state.audio)
                {
                    native.retain_image(image)?;
                }
            }
            state.last_used = last_used;
            state.transient_bytes = transient_bytes;
            return Ok(state);
        }
        let mut targets = Vec::new();
        for (pass, &(width, height)) in shader.passes.iter().zip(&sizes) {
            if let Some(name) = &pass.target {
                targets.push(TargetBuffer {
                    name: name.clone(),
                    persistent: pass.persistent,
                    front: 0,
                    images: [
                        planned_image(
                            &self.device,
                            width,
                            height,
                            pass.float,
                            "Syndocal ISF target read",
                            #[cfg(test)]
                            native,
                            #[cfg(test)]
                            true,
                        )?,
                        planned_image(
                            &self.device,
                            width,
                            height,
                            pass.float,
                            "Syndocal ISF target write",
                            #[cfg(test)]
                            native,
                            #[cfg(test)]
                            true,
                        )?,
                    ],
                });
            }
        }
        let imports = shader
            .imported_images
            .iter()
            .map(|image| {
                let gpu = planned_image(
                    &self.device,
                    image.width,
                    image.height,
                    false,
                    "Syndocal ISF imported image",
                    #[cfg(test)]
                    native,
                    #[cfg(test)]
                    true,
                )?;
                planned_upload(
                    &gpu,
                    &self.queue,
                    &image.rgba,
                    #[cfg(test)]
                    native,
                )?;
                Ok(gpu)
            })
            .collect::<Result<Vec<_>, IsfRuntimeError>>()?;
        let bound_images = shader
            .bound_images
            .iter()
            .map(|image| {
                let gpu = planned_image(
                    &self.device,
                    image.width,
                    image.height,
                    false,
                    "Syndocal ISF bound still",
                    #[cfg(test)]
                    native,
                    #[cfg(test)]
                    true,
                )?;
                planned_upload(
                    &gpu,
                    &self.queue,
                    &image.rgba,
                    #[cfg(test)]
                    native,
                )?;
                Ok(gpu)
            })
            .collect::<Result<Vec<_>, IsfRuntimeError>>()?;
        let audio = shader
            .audio_inputs
            .iter()
            .map(|input| {
                planned_image(
                    &self.device,
                    input.width,
                    1,
                    false,
                    "Syndocal ISF live audio",
                    #[cfg(test)]
                    native,
                    #[cfg(test)]
                    true,
                )
            })
            .collect::<Result<Vec<_>, IsfRuntimeError>>()?;
        Ok(StageResources {
            fingerprint: shader.fingerprint,
            sizes,
            targets,
            imports,
            bound_images,
            audio,
            audio_identity: None,
            audio_initialized: false,
            bytes,
            transient_bytes,
            last_used,
            has_history: false,
        })
    }

    fn planned_pass_uniforms(
        &self,
        width: u32,
        height: u32,
        time: f32,
        delta: f32,
        frame: u32,
        pass: usize,
        date: [f32; 4],
        controls: &[[f32; 4]],
        #[cfg(test)] native: &mut Option<&mut NativeCpuAllocationScope<'_, '_>>,
    ) -> Result<wgpu::Buffer, IsfRuntimeError> {
        #[cfg(test)]
        if let Some(native) = native.as_deref_mut() {
            let bytes = isf_uniform_data(width, height, time, delta, frame, pass, date, controls);
            return native.buffer(
                &self.device,
                Some(&bytes),
                bytes.len() as u64,
                crate::native_gpu_budget::ChargeKind::CachedEffectResource,
                wgpu::BufferUsages::UNIFORM,
                "Syndocal ISF pass uniforms",
            );
        }
        Ok(self.pass_uniforms(width, height, time, delta, frame, pass, date, controls))
    }

    fn pass_uniforms(
        &self,
        width: u32,
        height: u32,
        time: f32,
        delta: f32,
        frame: u32,
        pass: usize,
        date: [f32; 4],
        controls: &[[f32; 4]],
    ) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Syndocal ISF pass uniforms"),
                contents: &isf_uniform_data(
                    width, height, time, delta, frame, pass, date, controls,
                ),
                usage: wgpu::BufferUsages::UNIFORM,
            })
    }

    fn encode_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        wgsl: &str,
        images: &[&wgpu::TextureView],
        sampler: &wgpu::Sampler,
        uniforms: &wgpu::Buffer,
        output: &GpuImage,
    ) {
        let pipeline = self
            .pipelines
            .get(&(source_hash(wgsl), output.float, images.len()))
            .expect("ISF pass pipeline prepared before encoding");
        let mut entries = Vec::with_capacity(images.len() + 2);
        for (index, image) in images.iter().enumerate() {
            entries.push(wgpu::BindGroupEntry {
                binding: if index == 0 { 0 } else { index as u32 + 2 },
                resource: wgpu::BindingResource::TextureView(image),
            });
        }
        entries.push(wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::Sampler(sampler),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: 2,
            resource: uniforms.as_entire_binding(),
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Syndocal ISF pass bind group"),
            layout: &pipeline.bind_group_layout,
            entries: &entries,
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Syndocal ISF pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &output.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    fn ensure_stage_pipelines(
        &mut self,
        shader: &PreparedIsfShader,
        #[cfg(test)] mut pins: Option<&mut Vec<std::sync::Arc<IsfGpuPipeline>>>,
    ) -> Result<usize, IsfRuntimeError> {
        let texture_count = shader.image_input_names.len()
            + shader
                .passes
                .iter()
                .filter(|pass| pass.target.is_some())
                .count()
            + shader.imported_images.len()
            + shader.audio_inputs.len();
        let formats = [false, true]
            .into_iter()
            .filter(|float| shader.passes.iter().any(|pass| pass.float == *float))
            .collect::<Vec<_>>();
        let missing = formats
            .iter()
            .filter(|float| {
                !self
                    .pipelines
                    .contains_key(&(source_hash(&shader.wgsl), **float, texture_count))
            })
            .count();
        if self.pipelines.len() + missing > ISF_GPU_PIPELINE_CAPACITY {
            self.make_pipeline_room(missing)?;
        }
        for float in formats {
            self.ensure_pass_pipeline(&shader.wgsl, texture_count, float)?;
            #[cfg(test)]
            if let Some(pins) = pins.as_deref_mut() {
                pins.push(std::sync::Arc::clone(
                    self.pipelines
                        .get(&(source_hash(&shader.wgsl), float, texture_count))
                        .expect("just ensured pipeline"),
                ));
            }
        }
        Ok(texture_count)
    }

    fn ensure_pass_pipeline(
        &mut self,
        wgsl: &str,
        texture_count: usize,
        float: bool,
    ) -> Result<(), IsfRuntimeError> {
        if float
            && !self
                .device
                .features()
                .contains(wgpu::Features::FLOAT32_FILTERABLE)
        {
            return Err(runtime_error(
                "ISF FLOAT buffers require 32-bit float texture filtering, which this GPU does not support",
            ));
        }
        let key = (source_hash(wgsl), float, texture_count);
        if self.pipelines.contains_key(&key) {
            return Ok(());
        }
        if self.pipelines.len() >= ISF_GPU_PIPELINE_CAPACITY {
            self.make_pipeline_room(1)?;
        }
        let pipeline = self.create_pass_pipeline(wgsl, texture_count, float)?;
        self.pipelines.insert(key, std::sync::Arc::new(pipeline));
        Ok(())
    }

    fn make_pipeline_room(&mut self, missing: usize) -> Result<(), IsfRuntimeError> {
        if missing > ISF_GPU_PIPELINE_CAPACITY {
            return Err(runtime_error("ISF pipeline request exceeds cache capacity"));
        }
        #[cfg(test)]
        if self.native_context().is_some() {
            // Recipes retain exact pipelines. Eviction must not turn a bounded
            // cache into an unbounded list of externally pinned GPU pipelines.
            self.pipelines
                .retain(|_, pipeline| std::sync::Arc::strong_count(pipeline) > 1);
            if self.pipelines.len() + missing > ISF_GPU_PIPELINE_CAPACITY {
                return Err(runtime_error(
                    "ISF native recipe pipeline capacity exhausted",
                ));
            }
            return Ok(());
        }
        self.pipelines.clear();
        Ok(())
    }

    fn create_pass_pipeline(
        &self,
        wgsl: &str,
        texture_count: usize,
        float: bool,
    ) -> Result<IsfGpuPipeline, IsfRuntimeError> {
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut entries = Vec::with_capacity(texture_count + 2);
        for index in 0..texture_count {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: if index == 0 { 0 } else { index as u32 + 2 },
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
        }
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Syndocal ISF bind group layout"),
                    entries: &entries,
                });
        let layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Syndocal ISF pipeline layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });
        let fragment_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Syndocal translated ISF fragment shader"),
                source: wgpu::ShaderSource::Wgsl(wgsl.into()),
            });
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Syndocal ISF pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &self.vertex_shader,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_shader,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_format(float),
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });
        if let Some(error) = pollster::block_on(self.device.pop_error_scope()) {
            return Err(runtime_error(format!(
                "ISF GPU pipeline validation failed: {error}"
            )));
        }
        Ok(IsfGpuPipeline {
            bind_group_layout,
            pipeline,
        })
    }
}

/// Resource handles retained by the broker before the first queue write.
/// A completed lease takes only the final image, never this workspace owner.
#[cfg(test)]
pub(crate) struct NativeStackResources {
    workspace: StackWorkspace,
    uniforms: Vec<wgpu::Buffer>,
}

#[cfg(test)]
impl NativeStackResources {
    pub(crate) fn final_image(&self, stage_count: usize) -> GpuImage {
        if stage_count % 2 == 1 {
            self.workspace.ping.clone()
        } else {
            self.workspace.pong.clone()
        }
    }

    pub(crate) fn readback(&self) -> Option<&wgpu::Buffer> {
        self.workspace.readback.as_ref()
    }

    pub(crate) fn sampler(&self) -> wgpu::Sampler {
        self.workspace.sampler.clone()
    }

    pub(crate) fn use_frozen_sampler(&mut self, sampler: &wgpu::Sampler) {
        self.workspace.sampler = sampler.clone();
    }
}

#[cfg(test)]
impl IsfGpuRuntime {
    pub(crate) fn native_create_stack_resources(
        &self,
        phase: &crate::native_gpu_context::GpuPhaseEncoder<'_>,
        frame: &VideoFrame,
        stages: &[IsfStageInputs<'_>],
        clocks: crate::native_gpu_contract::FrozenChainClocks,
        cpu_readback: bool,
    ) -> Result<NativeStackResources, IsfRuntimeError> {
        if self
            .native_context()
            .is_none_or(|context| !std::sync::Arc::ptr_eq(context, phase.context()))
        {
            return Err(runtime_error(
                "native stack allocation requires its device phase capability",
            ));
        }
        self.validate_pass_size(frame.width, frame.height)?;
        if stages
            .iter()
            .any(|stage| !stage.can_share_host_buffers() || stage.shader.passes.len() != 1)
        {
            return Err(runtime_error(
                "native stack resources require one resource-neutral pass per stage",
            ));
        }
        let mut workspace = if cpu_readback {
            StackWorkspace::new(&self.device, frame.width, frame.height)
        } else {
            StackWorkspace::resident(&self.device, frame.width, frame.height)
        };
        for image in [
            &mut workspace.input,
            &mut workspace.ping,
            &mut workspace.pong,
        ] {
            image.native_origin = Some(phase.context().id());
        }
        let uniforms = stages
            .iter()
            .map(|stage| {
                self.pass_uniforms(
                    frame.width,
                    frame.height,
                    f32::from_bits(clocks.time_seconds_bits),
                    f32::from_bits(clocks.time_delta_seconds_bits),
                    clocks.frame_index,
                    0,
                    clocks.date_bits.map(f32::from_bits),
                    stage.controls,
                )
            })
            .collect();
        Ok(NativeStackResources {
            workspace,
            uniforms,
        })
    }

    pub(crate) fn native_encode_canonical_stack(
        &mut self,
        phase: &crate::native_gpu_context::GpuPhaseEncoder<'_>,
        resources: &NativeStackResources,
        frame: &std::sync::Arc<VideoFrame>,
        stages: &[IsfStageInputs<'_>],
        keys: &[IsfInstanceKey],
        clocks: crate::native_gpu_contract::FrozenChainClocks,
    ) -> Result<(wgpu::CommandBuffer, Vec<std::sync::Arc<IsfGpuPipeline>>), IsfRuntimeError> {
        if self
            .native_context()
            .is_none_or(|context| !std::sync::Arc::ptr_eq(context, phase.context()))
        {
            return Err(runtime_error(
                "native stack encoding requires its device phase capability",
            ));
        }
        let mut pending = PendingStack::new(
            frame,
            Some(std::sync::Arc::clone(frame)),
            Cow::Borrowed(keys),
            stack_transaction::StackClocks {
                time_seconds: f32::from_bits(clocks.time_seconds_bits),
                time_delta_seconds: f32::from_bits(clocks.time_delta_seconds_bits),
                frame_index: clocks.frame_index,
                date: clocks.date_bits.map(f32::from_bits),
            },
            Some(resources.workspace.clone()),
        );
        pending.native_uniforms = resources.uniforms.clone();
        // G's single ledger has already reserved all native resources. A Some
        // reservation keeps the legacy executor from evicting feedback/cache.
        self.encode_pass_stack(&mut pending, frame, stages, Some(0), None)?;
        let command = pending
            .encoder
            .take()
            .expect("canonical encoder produced commands")
            .finish();
        Ok((command, pending.native_pipeline_pins))
    }
}

#[cfg(test)]
pub(crate) use cpu_bridge::{NativeCpuCacheEntry, NativeCpuCacheModel};

#[cfg(test)]
mod cpu_bridge {
    use super::*;
    use crate::native_gpu_budget::{
        checked_upload_staging_bytes, ChargeKind, NativeAllocationPlan,
    };
    use crate::native_gpu_context::GpuPhaseEncoder;
    use crate::native_gpu_contract::{
        FrozenChainClocks, GpuAllocationId, NativeFrameId, NativeGpuFault,
    };
    use crate::native_gpu_result::NativeCpuFrameOwner;
    use std::sync::Arc;

    #[derive(Clone, Default)]
    pub(crate) struct NativeCpuCacheEntry {
        pub(crate) fingerprint: u64,
        pub(crate) sizes: Vec<(u32, u32)>,
        pub(crate) bytes: u64,
        pub(crate) last_used: u64,
        pub(crate) audio_identity: Option<(u64, u64)>,
        pub(crate) audio_initialized: bool,
        pub(crate) allocations: Vec<GpuAllocationId>,
        pub(crate) reclaimable: bool,
    }
    #[derive(Clone)]
    pub(crate) struct NativeCpuCacheModel {
        entries: BTreeMap<IsfInstanceKey, NativeCpuCacheEntry>,
        clock: u64,
    }
    impl NativeCpuCacheModel {
        pub(crate) fn new(
            entries: BTreeMap<IsfInstanceKey, NativeCpuCacheEntry>,
            clock: u64,
        ) -> Self {
            Self { entries, clock }
        }
        pub(crate) fn reclaimable_bytes(&self) -> u64 {
            self.entries
                .values()
                .filter(|entry| entry.reclaimable)
                .map(|entry| entry.bytes)
                .sum()
        }
        pub(crate) fn additional_peak(initial_reclaimable: u64, projected_peak: u64) -> u64 {
            projected_peak.saturating_sub(initial_reclaimable)
        }
        pub(crate) fn evict(
            &mut self,
            reserved: u64,
            protected: Option<&IsfInstanceKey>,
        ) -> Vec<IsfInstanceKey> {
            let mut evicted = Vec::new();
            while let Some(key) =
                resource_eviction_candidate(&self.entries, reserved, protected, |entry| {
                    (entry.bytes, entry.last_used)
                })
            {
                self.entries.remove(&key);
                evicted.push(key);
            }
            evicted
        }
        fn commit(&mut self, states: Vec<(IsfInstanceKey, NativeCpuCacheEntry)>) {
            for (key, state) in states {
                if state.bytes != 0 {
                    self.entries.insert(key, state);
                }
            }
            while self.entries.len() > RESOURCE_CACHE_CAPACITY {
                let key = self
                    .entries
                    .iter()
                    .min_by_key(|(_, entry)| entry.last_used)
                    .map(|(key, _)| key.clone())
                    .unwrap();
                self.entries.remove(&key);
            }
        }
    }

    pub(crate) struct NativeCpuChunkPlan {
        pub(crate) plan: NativeAllocationPlan,
    }

    pub(crate) struct NativeCpuEncodedChunk {
        pending: PendingStack<'static>,
        readback: Arc<NativeCpuBuffer>,
        input: Arc<VideoFrame>,
        output_charge: Arc<crate::native_gpu_budget::AllocationCharge>,
        host_error: Option<IsfRuntimeError>,
    }
    impl NativeCpuEncodedChunk {
        pub(crate) fn take_commands(&mut self) -> Vec<wgpu::CommandBuffer> {
            match self.pending.encoder.take() {
                Some(encoder) if self.host_error.is_none() => vec![encoder.finish()],
                _ => Vec::new(),
            }
        }
        pub(crate) fn readback(&self) -> &wgpu::Buffer {
            &self.readback.buffer
        }
        pub(crate) fn host_error(&self) -> Option<&IsfRuntimeError> {
            self.host_error.as_ref()
        }
        pub(crate) fn copy_mapped(&self) -> Arc<NativeCpuFrameOwner> {
            let mapped = self.readback.buffer.slice(..).get_mapped_range();
            let data = copy_rgba_readback(
                &mapped,
                self.input.width as usize * 4,
                StackWorkspace::bytes_per_row(self.input.width) as usize,
                self.input.width as usize * self.input.height as usize * 4,
            );
            drop(mapped);
            Arc::new(NativeCpuFrameOwner {
                frame: Arc::new(VideoFrame {
                    layer_id: self.input.layer_id,
                    width: self.input.width,
                    height: self.input.height,
                    pts_ms: self.input.pts_ms,
                    duration_ms: self.input.duration_ms,
                    format: VideoPixelFormat::Rgba8,
                    data,
                }),
                charge: Some(Arc::clone(&self.output_charge)),
            })
        }
    }

    fn add(
        plan: &mut NativeAllocationPlan,
        kind: ChargeKind,
        bytes: u64,
    ) -> Result<(), IsfRuntimeError> {
        plan.add_new(kind, bytes)
            .map_err(|reason| runtime_error(format!("native CPU allocation plan: {reason:?}")))
    }
    fn image_bytes(width: u32, height: u32, float: bool) -> Result<u64, IsfRuntimeError> {
        u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|bytes| bytes.checked_mul(if float { 16 } else { 4 }))
            .filter(|bytes| *bytes != 0)
            .ok_or_else(|| runtime_error("ISF image capacity overflow"))
    }
    fn upload(
        plan: &mut NativeAllocationPlan,
        width: u32,
        height: u32,
        pitch: Option<u64>,
    ) -> Result<(), IsfRuntimeError> {
        add(
            plan,
            ChargeKind::QueueUploadStaging,
            checked_upload_staging_bytes(width, height, pitch)
                .map_err(|reason| runtime_error(format!("ISF upload extent/pitch: {reason:?}")))?,
        )
    }
    fn validate_input_bytes(frame: &VideoFrame) -> Result<(), IsfRuntimeError> {
        let valid = match frame.format {
            VideoPixelFormat::Rgba8 | VideoPixelFormat::Bgra8 => {
                crate::rgba_frame_len(frame.width, frame.height)
            }
            VideoPixelFormat::Dxt1 => {
                crate::dxt_block_count(frame.width, frame.height).and_then(|n| n.checked_mul(8))
            }
            VideoPixelFormat::Dxt5 | VideoPixelFormat::YcoCgDxt5 | VideoPixelFormat::Bc7 => {
                crate::dxt_block_count(frame.width, frame.height).and_then(|n| n.checked_mul(16))
            }
        };
        if frame.width == 0 || frame.height == 0 || valid != Some(frame.data.len()) {
            return Err(runtime_error(
                "ISF input frame byte length does not match its dimensions",
            ));
        }
        Ok(())
    }
    fn transparent_needed(stage: &IsfStageInputs<'_>) -> bool {
        stage
            .shader
            .image_input_names
            .iter()
            .enumerate()
            .any(|(index, name)| {
                !stage
                    .shader
                    .bound_images
                    .iter()
                    .any(|image| &image.name == name)
                    && !stage.images.iter().any(|image| image.name == name.as_str())
                    && (index != 0 || stage.shader.image_binding_names.contains(name))
            })
    }

    impl IsfGpuRuntime {
        pub(crate) fn native_cpu_cache_model(&self) -> NativeCpuCacheModel {
            let entries = self
                .resources
                .entries
                .iter()
                .map(|(key, state)| {
                    let images: Vec<_> = state
                        .targets
                        .iter()
                        .flat_map(|target| &target.images)
                        .chain(&state.imports)
                        .chain(&state.bound_images)
                        .chain(&state.audio)
                        .collect();
                    let reclaimable = images.iter().all(|image| {
                        image
                            .native_charge
                            .as_ref()
                            .is_some_and(|charge| Arc::strong_count(charge) == 1)
                    });
                    let allocations = images
                        .iter()
                        .filter_map(|image| image.native_charge.as_ref().map(|charge| charge.id()))
                        .collect();
                    (
                        key.clone(),
                        NativeCpuCacheEntry {
                            fingerprint: state.fingerprint,
                            sizes: state.sizes.clone(),
                            bytes: state.bytes,
                            last_used: state.last_used,
                            audio_identity: state.audio_identity,
                            audio_initialized: state.audio_initialized,
                            allocations,
                            reclaimable,
                        },
                    )
                })
                .collect();
            NativeCpuCacheModel::new(entries, self.resources.clock)
        }

        pub(crate) fn native_cpu_apply_baseline_evictions(
            &mut self,
            frame: &VideoFrame,
        ) -> Result<(), IsfRuntimeError> {
            self.validate_pass_size(frame.width, frame.height)?;
            let baseline = StackWorkspace::required_bytes(frame.width, frame.height);
            if baseline > MAX_CACHED_RESOURCE_BYTES {
                return Err(runtime_error(
                    "ISF pass size exceeds the GPU buffer memory budget",
                ));
            }
            self.evict_resources_for(baseline);
            Ok(())
        }

        pub(crate) fn native_cpu_validate_input(
            &self,
            frame: &VideoFrame,
        ) -> Result<(), IsfRuntimeError> {
            self.validate_pass_size(frame.width, frame.height)?;
            validate_input_bytes(frame)
        }

        pub(crate) fn native_cpu_invalidate_keys(&mut self, keys: &[IsfInstanceKey]) {
            self.invalidate_stage_states(keys);
        }

        pub(crate) fn native_cpu_simulate_baseline(
            &self,
            cache: &mut NativeCpuCacheModel,
            frame: &VideoFrame,
        ) -> Result<(), IsfRuntimeError> {
            self.validate_pass_size(frame.width, frame.height)?;
            let baseline = StackWorkspace::required_bytes(frame.width, frame.height);
            if baseline > MAX_CACHED_RESOURCE_BYTES {
                return Err(runtime_error(
                    "ISF pass size exceeds the GPU buffer memory budget",
                ));
            }
            cache.evict(baseline, None);
            Ok(())
        }

        fn cpu_model_take(
            &self,
            cache: &mut NativeCpuCacheModel,
            frame: &VideoFrame,
            stages: &[IsfStageInputs<'_>],
            keys: &[IsfInstanceKey],
            audio: Option<&protocol::LiveAudioTextureFrame>,
        ) -> Result<Vec<(IsfInstanceKey, NativeCpuCacheEntry)>, IsfRuntimeError> {
            if stages.len() != keys.len() {
                return Err(runtime_error(
                    "ISF stage ownership key count does not match the stack",
                ));
            }
            let mut pending = StackWorkspace::required_bytes(frame.width, frame.height);
            let mut states = Vec::new();
            for (stage, key) in stages.iter().zip(keys) {
                let dynamic = self.dynamic_image_bytes(stage, transparent_needed(stage))?;
                for image in stage.images {
                    validate_input_bytes(image.frame)?;
                }
                let layout = self.stage_resource_layout(
                    stage.shader,
                    stage.controls,
                    frame,
                    pending,
                    dynamic,
                )?;
                let reused = cache
                    .entries
                    .get(key)
                    .filter(|entry| {
                        entry.fingerprint == stage.shader.fingerprint && entry.sizes == layout.sizes
                    })
                    .cloned();
                if reused.is_none() {
                    cache.entries.remove(key);
                }
                cache.evict(pending + layout.bytes + layout.transient_bytes, Some(key));
                cache.clock = cache.clock.wrapping_add(1);
                states.push((
                    key.clone(),
                    NativeCpuCacheEntry {
                        fingerprint: stage.shader.fingerprint,
                        sizes: layout.sizes,
                        bytes: layout.bytes,
                        last_used: cache.clock,
                        audio_identity: audio
                            .map(|frame| (frame.generation, frame.feature_sequence)),
                        audio_initialized: true,
                        allocations: reused
                            .as_ref()
                            .map(|entry| entry.allocations.clone())
                            .unwrap_or_default(),
                        reclaimable: reused.as_ref().is_none_or(|entry| entry.reclaimable),
                    },
                ));
                pending += layout.bytes + layout.transient_bytes;
            }
            Ok(states)
        }

        pub(crate) fn native_cpu_apply_take_evictions(
            &mut self,
            frame: &VideoFrame,
            stages: &[IsfStageInputs<'_>],
            keys: &[IsfInstanceKey],
        ) -> Result<(), IsfRuntimeError> {
            let mut cache = self.native_cpu_cache_model();
            self.cpu_model_take(&mut cache, frame, stages, keys, self.audio_frame.as_ref())?;
            self.resources
                .entries
                .retain(|key, _| cache.entries.contains_key(key));
            Ok(())
        }

        pub(crate) fn native_cpu_simulate_chunk(
            &self,
            cache: &mut NativeCpuCacheModel,
            request: NativeFrameId,
            frame: &VideoFrame,
            original: bool,
            stages: &[IsfStageInputs<'_>],
            keys: &[IsfInstanceKey],
            audio: Option<&protocol::LiveAudioTextureFrame>,
        ) -> Result<u64, IsfRuntimeError> {
            self.validate_pass_size(frame.width, frame.height)?;
            let baseline = StackWorkspace::required_bytes(frame.width, frame.height);
            if baseline > MAX_CACHED_RESOURCE_BYTES {
                return Err(runtime_error(
                    "ISF pass size exceeds the GPU buffer memory budget",
                ));
            }
            cache.evict(baseline, None);
            let before_take = cache.reclaimable_bytes();
            if original {
                validate_input_bytes(frame)?;
            }
            let states = self.cpu_model_take(cache, frame, stages, keys, audio)?;
            let planned = self.native_cpu_chunk_plan_in_model(
                request,
                frame,
                if original {
                    frame.format
                } else {
                    VideoPixelFormat::Rgba8
                },
                original,
                stages,
                keys,
                audio,
                cache,
            )?;
            let peak = before_take.max(
                cache
                    .reclaimable_bytes()
                    .saturating_add(planned.plan.requested_bytes()),
            );
            cache.commit(states);
            Ok(peak)
        }

        pub(crate) fn native_prepare_cpu_pipelines(
            &mut self,
            phase: &GpuPhaseEncoder<'_>,
            stages: &[IsfStageInputs<'_>],
        ) -> (Vec<Arc<IsfGpuPipeline>>, Result<(), IsfRuntimeError>) {
            let mut pins = Vec::new();
            let prepared = (|| {
                if self
                    .native_context()
                    .is_none_or(|context| !Arc::ptr_eq(context, phase.context()))
                {
                    return Err(runtime_error(
                        "CPU pipeline preparation requires its shared phase",
                    ));
                }
                for stage in stages {
                    self.ensure_stage_pipelines(stage.shader, Some(&mut pins))?;
                }
                // Normalization stays at its canonical post-pass point. A
                // failure there must discard already-taken feedback state.
                Ok(())
            })();
            // Even a failed prefix can have created pipelines. The broker
            // retains those exact owners until the preparation scopes drain.
            (pins, prepared)
        }

        /// Allocation and encoding share the same canonical pass scheduler;
        /// only resource construction/queue writes receive the broker capability.
        pub(crate) fn native_encode_cpu_chunk(
            &mut self,
            phase: &mut GpuPhaseEncoder<'_>,
            frame: &Arc<VideoFrame>,
            stages: &[IsfStageInputs<'_>],
            keys: &[IsfInstanceKey],
            clocks: FrozenChainClocks,
        ) -> Result<NativeCpuEncodedChunk, NativeGpuFault> {
            if self
                .native_context()
                .is_none_or(|context| !Arc::ptr_eq(context, phase.context()))
            {
                return Err(NativeGpuFault {
                    context: phase.context().id(),
                    frame: None,
                    phase: crate::native_gpu_contract::GpuPhase::IsfBatch,
                    kind: crate::native_gpu_contract::NativeGpuFaultKind::IdentityMismatch,
                    message: "CPU chunk requires its shared device phase".into(),
                });
            }
            let mut native = NativeCpuAllocationScope {
                phase,
                buffers: Vec::new(),
                failure: None,
            };
            let result = (|| -> Result<NativeCpuEncodedChunk, IsfRuntimeError> {
                let mut allocation = Some(&mut native);
                let input = planned_image(
                    &self.device,
                    frame.width,
                    frame.height,
                    false,
                    "Syndocal ISF input",
                    &mut allocation,
                    false,
                )?;
                let ping = planned_image(
                    &self.device,
                    frame.width,
                    frame.height,
                    false,
                    "Syndocal ISF stack ping",
                    &mut allocation,
                    false,
                )?;
                let pong = planned_image(
                    &self.device,
                    frame.width,
                    frame.height,
                    false,
                    "Syndocal ISF stack pong",
                    &mut allocation,
                    false,
                )?;
                let readback_bytes =
                    u64::from(StackWorkspace::bytes_per_row(frame.width)) * u64::from(frame.height);
                let readback_buffer = native.buffer(
                    &self.device,
                    None,
                    readback_bytes,
                    ChargeKind::SnapshotReadback,
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    "Syndocal ISF readback",
                )?;
                let readback = Arc::clone(native.buffers.last().expect("readback owner"));
                let output_charge = native.charge(
                    ChargeKind::CpuSnapshot,
                    image_bytes(frame.width, frame.height, false)?,
                )?;
                let workspace = StackWorkspace {
                    input,
                    ping,
                    pong,
                    readback: Some(readback_buffer),
                    sampler: self.device.create_sampler(&wgpu::SamplerDescriptor {
                        label: Some("Syndocal ISF sampler"),
                        address_mode_u: wgpu::AddressMode::ClampToEdge,
                        address_mode_v: wgpu::AddressMode::ClampToEdge,
                        mag_filter: wgpu::FilterMode::Linear,
                        min_filter: wgpu::FilterMode::Linear,
                        ..Default::default()
                    }),
                };
                let mut pending = PendingStack::new(
                    frame,
                    Some(Arc::clone(frame)),
                    Cow::Owned(keys.to_vec()),
                    stack_transaction::StackClocks {
                        time_seconds: f32::from_bits(clocks.time_seconds_bits),
                        time_delta_seconds: f32::from_bits(clocks.time_delta_seconds_bits),
                        frame_index: clocks.frame_index,
                        date: clocks.date_bits.map(f32::from_bits),
                    },
                    Some(workspace),
                );
                if let Err(fault) = native.phase.retain_resource(
                    Arc::new(pending.workspace.as_ref().unwrap().sampler.clone()),
                    &[],
                ) {
                    native.failure = Some(crate::native_gpu_result::copy_fault(&fault));
                    return Err(runtime_error(fault.message));
                }
                let host_error = self
                    .encode_pass_stack(&mut pending, frame, stages, Some(0), Some(&mut native))
                    .err();
                // Keep the exact pipeline/layout owners before cache eviction.
                let pins = std::mem::take(&mut pending.native_pipeline_pins);
                if let Err(fault) = native.phase.retain_resource(Arc::new(pins), &[]) {
                    native.failure = Some(crate::native_gpu_result::copy_fault(&fault));
                    return Err(runtime_error(fault.message));
                }
                Ok(NativeCpuEncodedChunk {
                    pending,
                    readback,
                    input: Arc::clone(frame),
                    output_charge,
                    host_error,
                })
            })();
            if let Some(fault) = native.failure {
                return Err(fault);
            }
            result.map_err(|error| NativeGpuFault {
                context: native.phase.context().id(),
                frame: None,
                phase: crate::native_gpu_contract::GpuPhase::IsfBatch,
                kind: crate::native_gpu_contract::NativeGpuFaultKind::Validation,
                message: error.message,
            })
        }

        pub(crate) fn native_finish_cpu_chunk(
            &mut self,
            mut chunk: NativeCpuEncodedChunk,
            success: bool,
            scope_failed: bool,
        ) {
            if scope_failed {
                self.invalidate_stage_states(&chunk.pending.keys);
            }
            if success {
                self.commit_stage_states(chunk.pending.states.drain(..));
            }
            // On a host error, only state actually taken by canonical encoding
            // is discarded. A GPU scope failure invalidates all affected keys,
            // exactly as finish_stack_transaction does for the standalone path.
        }

        pub(crate) fn native_cpu_cache_stamp(&self) -> (u64, Vec<(GpuAllocationId, usize)>) {
            let ids = self
                .resources
                .entries
                .values()
                .flat_map(|state| {
                    state
                        .targets
                        .iter()
                        .flat_map(|target| &target.images)
                        .chain(&state.imports)
                        .chain(&state.bound_images)
                        .chain(&state.audio)
                        .filter_map(|image| {
                            image
                                .native_charge
                                .as_ref()
                                .map(|charge| (charge.id(), Arc::strong_count(charge)))
                        })
                })
                .collect();
            (self.resources.clock, ids)
        }

        /// Pure allocation description of the same canonical encoder. Invalid
        /// stage inputs return their canonical-local failure without removing
        /// feedback, uploading, compiling a shader or consuming a frame index.
        pub(crate) fn native_cpu_chunk_plan(
            &self,
            request: NativeFrameId,
            frame: &VideoFrame,
            stages: &[IsfStageInputs<'_>],
            keys: &[IsfInstanceKey],
            audio: Option<&protocol::LiveAudioTextureFrame>,
        ) -> Result<NativeCpuChunkPlan, IsfRuntimeError> {
            self.native_cpu_chunk_plan_in_model(
                request,
                frame,
                frame.format,
                true,
                stages,
                keys,
                audio,
                &self.native_cpu_cache_model(),
            )
        }

        fn native_cpu_chunk_plan_in_model(
            &self,
            request: NativeFrameId,
            frame: &VideoFrame,
            input_format: VideoPixelFormat,
            validate_original: bool,
            stages: &[IsfStageInputs<'_>],
            keys: &[IsfInstanceKey],
            audio: Option<&protocol::LiveAudioTextureFrame>,
            cache: &NativeCpuCacheModel,
        ) -> Result<NativeCpuChunkPlan, IsfRuntimeError> {
            self.validate_pass_size(frame.width, frame.height)?;
            if validate_original {
                validate_input_bytes(frame)?;
            }
            if stages.is_empty() || stages.len() != keys.len() {
                return Err(runtime_error(
                    "ISF stage ownership key count does not match the stack",
                ));
            }
            let pitch = self.resources.queue_upload_pitch;
            let mut plan = NativeAllocationPlan::new(request);
            let pixels = image_bytes(frame.width, frame.height, false)?;
            for _ in 0..3 {
                add(&mut plan, ChargeKind::IsfScratch, pixels)?;
            }
            add(
                &mut plan,
                ChargeKind::SnapshotReadback,
                checked_upload_staging_bytes(frame.width, frame.height, Some(256))
                    .map_err(|reason| runtime_error(format!("ISF readback size: {reason:?}")))?,
            )?;
            add(&mut plan, ChargeKind::CpuSnapshot, pixels)?;
            upload(&mut plan, frame.width, frame.height, pitch)?;
            if input_format != VideoPixelFormat::Rgba8 {
                add(&mut plan, ChargeKind::ActiveCpuInput, pixels)?;
            }
            for (stage, key) in stages.iter().zip(keys) {
                let shader = stage.shader;
                let transparent = transparent_needed(stage);
                let dynamic = self.dynamic_image_bytes(stage, transparent)?;
                let layout =
                    self.stage_resource_layout(shader, stage.controls, frame, 0, dynamic)?;
                let cached = cache.entries.get(key).filter(|state| {
                    state.fingerprint == shader.fingerprint && state.sizes == layout.sizes
                });
                if let Some(state) = cached {
                    for id in &state.allocations {
                        plan.add_existing(*id);
                    }
                } else {
                    for (pass, &(width, height)) in shader.passes.iter().zip(&layout.sizes) {
                        if pass.target.is_some() {
                            let bytes = image_bytes(width, height, pass.float)?;
                            add(&mut plan, ChargeKind::CachedEffectResource, bytes)?;
                            add(&mut plan, ChargeKind::CachedEffectResource, bytes)?;
                        }
                    }
                    for image in shader.imported_images.iter().chain(&shader.bound_images) {
                        add(
                            &mut plan,
                            ChargeKind::CachedEffectResource,
                            image_bytes(image.width, image.height, false)?,
                        )?;
                        upload(&mut plan, image.width, image.height, pitch)?;
                    }
                    for input in &shader.audio_inputs {
                        add(
                            &mut plan,
                            ChargeKind::CachedEffectResource,
                            image_bytes(input.width, 1, false)?,
                        )?;
                    }
                }
                for input in stage.images {
                    validate_input_bytes(input.frame)?;
                    let bytes = image_bytes(input.frame.width, input.frame.height, false)?;
                    add(&mut plan, ChargeKind::IsfScratch, bytes)?;
                    if input.frame.format != VideoPixelFormat::Rgba8 {
                        add(&mut plan, ChargeKind::ActiveCpuInput, bytes)?;
                    }
                    upload(&mut plan, input.frame.width, input.frame.height, pitch)?;
                }
                if transparent {
                    add(&mut plan, ChargeKind::IsfScratch, 4)?;
                    upload(&mut plan, 1, 1, pitch)?;
                }
                let audio_identity = audio.map(|frame| (frame.generation, frame.feature_sequence));
                if cached.is_none_or(|state| {
                    !state.audio_initialized || state.audio_identity != audio_identity
                }) {
                    for input in &shader.audio_inputs {
                        upload(&mut plan, input.width, 1, pitch)?;
                    }
                }
                for (pass, &(width, height)) in shader.passes.iter().zip(&layout.sizes) {
                    if pass.target.is_none()
                        && (width != frame.width || height != frame.height || pass.float)
                    {
                        add(
                            &mut plan,
                            ChargeKind::IsfScratch,
                            image_bytes(width, height, pass.float)?,
                        )?;
                    }
                    add(
                        &mut plan,
                        ChargeKind::CachedEffectResource,
                        48 + super::super::ISF_MAX_CONTROL_INPUTS as u64 * 16,
                    )?;
                }
                if shader.passes.last().zip(layout.sizes.last()).is_some_and(
                    |(pass, &(width, height))| {
                        width != frame.width || height != frame.height || pass.float
                    },
                ) {
                    add(
                        &mut plan,
                        ChargeKind::CachedEffectResource,
                        48 + super::super::ISF_MAX_CONTROL_INPUTS as u64 * 16,
                    )?;
                }
            }
            Ok(NativeCpuChunkPlan { plan })
        }
    }
}

const BLIT_SHADER: &str = r#"
@group(0) @binding(0) var source_image: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    return textureSample(source_image, source_sampler, uv);
}
"#;

#[cfg(test)]
#[path = "isf_stack_workspace_tests.rs"]
mod workspace_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prepare_isf_shader;

    #[test]
    fn isf_native_gpu_pipeline_pins_survive_mid_stack_capacity_eviction() {
        use crate::native_gpu_budget::{
            checked_upload_staging_bytes, ChargeKind, NativeAllocationPlan,
        };
        use crate::native_gpu_context::{
            NativeGpuFactory, NativeGpuRequirements, RetirementProgress,
        };
        use crate::native_gpu_contract::{
            CompletionProgress, FrozenChainClocks, GpuPhase, NativeFrameId,
        };
        use std::sync::Arc;
        use std::time::{Duration, Instant};

        let mut owner = NativeGpuFactory::new()
            .create_headless(9821, NativeGpuRequirements::default())
            .expect("pipeline retention regression requires a GPU");
        let mut runtime = IsfGpuRuntime::with_native_context(&mut owner).unwrap();
        let base = prepare_isf_shader(
            r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
            void main() { gl_FragColor = IMG_THIS_PIXEL(inputImage); }"#,
        )
        .unwrap();
        let shader = |ordinal: usize| {
            let mut shader = base.clone();
            // The compile keys are distinct while pixels remain an identity
            // filter, so this exercises real cache pressure without changing FX.
            shader
                .wgsl
                .push_str(&format!("\n// pipeline retention key {ordinal}\n"));
            shader
        };
        owner
            .initialize(|_, _| {
                for ordinal in 0..40 {
                    runtime.ensure_pass_pipeline(&shader(ordinal).wgsl, 1, false)?;
                }
                Ok::<(), IsfRuntimeError>(())
            })
            .unwrap()
            .unwrap();
        assert_eq!(runtime.pipelines.len(), 40);
        assert!(runtime
            .pipelines
            .values()
            .all(|pipeline| Arc::strong_count(pipeline) == 1));

        let shaders: Vec<_> = (40..72).map(shader).collect();
        let stages: Vec<_> = shaders
            .iter()
            .map(|shader| IsfStageInputs {
                shader,
                controls: &[],
                images: &[],
            })
            .collect();
        let input = Arc::new(VideoFrame {
            layer_id: 42,
            width: 1,
            height: 1,
            pts_ms: 0,
            duration_ms: 16,
            format: VideoPixelFormat::Rgba8,
            data: vec![21, 53, 87, 255],
        });
        let keys: Vec<_> = (0..stages.len())
            .map(|index| IsfInstanceKey::legacy(input.layer_id, index))
            .collect();
        let frame = NativeFrameId {
            output_id: 9821,
            project_epoch: 1,
            request_sequence: 1,
            geometry_generation: 1,
        };
        let clocks = FrozenChainClocks {
            frame_index: 7,
            time_seconds_bits: 0.0f32.to_bits(),
            time_delta_seconds_bits: 0.016f32.to_bits(),
            date_bits: [2026.0f32, 10.0, 26.0, 0.0].map(f32::to_bits),
        };
        let uniform_bytes = 48 + crate::isf_runtime::ISF_MAX_CONTROL_INPUTS as u64 * 16;
        let mut allocations = vec![
            (ChargeKind::IsfScratch, 4),
            (ChargeKind::IsfScratch, 4),
            (ChargeKind::ResidentLayer, 4),
            (ChargeKind::ActiveCpuInput, input.data.capacity() as u64),
            (
                ChargeKind::QueueUploadStaging,
                checked_upload_staging_bytes(1, 1, owner.capabilities().upload_row_pitch).unwrap(),
            ),
        ];
        allocations
            .extend((0..stages.len()).map(|_| (ChargeKind::CachedEffectResource, uniform_bytes)));
        let mut plan = NativeAllocationPlan::new(frame);
        for (kind, bytes) in &allocations {
            plan.add_new(*kind, *bytes).unwrap();
        }
        let quote = owner.quote(&plan).unwrap();
        let reservation = owner.reserve(quote).unwrap();
        let mut txn = owner.begin_frame(frame, reservation).unwrap();
        let mut phase = txn.begin_phase(GpuPhase::IsfBatch).unwrap();
        let charges: Vec<_> = allocations
            .into_iter()
            .map(|(kind, bytes)| phase.bind_allocation(kind, bytes).unwrap())
            .collect();
        let ids: Vec<_> = charges.iter().map(|charge| charge.id()).collect();
        struct Retained {
            stack: NativeStackResources,
            _input: Arc<VideoFrame>,
            _charges: Vec<crate::native_gpu_budget::AllocationCharge>,
        }
        let retained = Arc::new(Retained {
            stack: runtime
                .native_create_stack_resources(&phase, &input, &stages, clocks, false)
                .unwrap(),
            _input: Arc::clone(&input),
            _charges: charges,
        });
        phase.retain_resource(Arc::clone(&retained), &ids).unwrap();
        phase.mark_queue_write();
        let (commands, pins) = runtime
            .native_encode_canonical_stack(&phase, &retained.stack, &input, &stages, &keys, clocks)
            .unwrap();
        assert_eq!(
            pins.len(),
            32,
            "every stage must survive a mid-job cache eviction"
        );
        assert_eq!(
            runtime.pipelines.len(),
            32,
            "the 40 old unpinned entries were evicted"
        );
        for (shader, pin) in shaders.iter().zip(&pins) {
            let cached = runtime
                .pipelines
                .get(&(source_hash(&shader.wgsl), false, 1))
                .unwrap();
            assert!(
                Arc::ptr_eq(cached, pin),
                "retain the exact compiled pipeline"
            );
        }
        phase.retain_resource(Arc::new(pins.clone()), &[]).unwrap();
        let encoded = phase.finish(vec![commands]);
        let receipt = txn.submit_phase(encoded).unwrap();
        let mut pending = txn.finish();
        let started = Instant::now();
        loop {
            match owner.poll_transaction(&mut pending, &|| false) {
                CompletionProgress::Pending if started.elapsed() < Duration::from_secs(5) => {
                    std::thread::yield_now()
                }
                CompletionProgress::Complete(Ok(mut resolved)) => {
                    resolved
                        .take_phase(receipt)
                        .unwrap()
                        .into_validation()
                        .unwrap();
                    break;
                }
                CompletionProgress::Complete(Err(error)) => {
                    panic!("pipeline retention completion failed: {error:?}")
                }
                _ => panic!("pipeline retention completion cancelled or timed out"),
            }
        }
        assert!(
            runtime.make_pipeline_room(33).is_err(),
            "held recipe pins remain within the 64-entry cap"
        );
        assert_eq!(runtime.pipelines.len(), 32);
        drop(pins);
        runtime.make_pipeline_room(64).unwrap();
        assert!(runtime.pipelines.is_empty());
        drop(retained);
        owner.begin_stop();
        assert_eq!(owner.poll_retirement().unwrap(), RetirementProgress::Reaped);
    }

    #[test]
    fn fixed_pass_sizes_refresh_transient_budget_when_host_size_changes() {
        let source = r#"/*{
            "INPUTS":[{"NAME":"inputImage","TYPE":"image"}],
            "PASSES":[
                {"TARGET":"historyBuffer","PERSISTENT":true,"WIDTH":16,"HEIGHT":16},
                {"WIDTH":16,"HEIGHT":16}
            ]
        }*/
        void main() {
            if (PASSINDEX == 0) {
                gl_FragColor = IMG_THIS_PIXEL(historyBuffer) + vec4(1.0/255.0,0.0,0.0,1.0);
            } else { gl_FragColor = IMG_THIS_PIXEL(historyBuffer); }
        }"#;
        let shader = prepare_isf_shader(source).unwrap();
        let mut runtime = IsfGpuRuntime::new().expect("resize budget regression requires a GPU");
        for (size, transient_bytes, red) in [(16, 0, 1), (8, 1024, 2), (16, 0, 3)] {
            let frame = VideoFrame {
                layer_id: 23,
                width: size,
                height: size,
                pts_ms: 0,
                duration_ms: 16,
                format: VideoPixelFormat::Rgba8,
                data: [0, 0, 0, 255].repeat((size * size) as usize),
            };
            let output = runtime.apply(&frame, &shader, &[], 0.0, 0.016).unwrap();
            assert!(output
                .data
                .chunks_exact(4)
                .all(|pixel| pixel == [red, 0, 0, 255]));
            let state = runtime.resources.entries.values().next().unwrap();
            assert_eq!(state.bytes, 2048);
            assert_eq!(state.transient_bytes, transient_bytes);
        }
    }
}
