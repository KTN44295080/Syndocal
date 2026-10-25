//! Pass scheduling and GPU resource ownership for the ISF runtime.
//! Every named buffer has separate read/write textures. Feedback is owned by
//! an effect instance, never by a compiled shader or by a control value.
use std::{borrow::Cow, collections::BTreeMap, sync::mpsc};
use wgpu::util::DeviceExt;

use super::{
    convert_frame_to_rgba8, isf_uniform_data, runtime_error, runtime_from_composite_error,
    source_hash, IsfGpuPipeline, IsfGpuRuntime, IsfInstanceKey, IsfRuntimeError, IsfStageInputs,
    PreparedIsfShader, VideoFrame, VideoPixelFormat, ISF_GPU_PIPELINE_CAPACITY,
};

const RESOURCE_CACHE_CAPACITY: usize = 64;
const MAX_STAGE_RESOURCE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_CACHED_RESOURCE_BYTES: u64 = 512 * 1024 * 1024;

#[path = "isf_stack_workspace.rs"]
mod stack_workspace;
use stack_workspace::StackWorkspace;

#[cfg(test)]
#[path = "isf_stack_phase_timing.rs"]
mod phase_timing;

#[derive(Default)]
pub(super) struct IsfResourcePool {
    entries: BTreeMap<IsfInstanceKey, StageResources>,
    clock: u64,
    workspace: Option<StackWorkspace>,
    #[cfg(test)]
    phase_timing: Option<phase_timing::StackPhaseTiming>,
}

#[derive(Clone)]
struct GpuImage {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
    float: bool,
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
        }
    }

    fn upload_rgba(&self, queue: &wgpu::Queue, data: &[u8]) {
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
    pub(super) fn render_pass_stack(
        &mut self,
        frame: &VideoFrame,
        stages: &[IsfStageInputs<'_>],
        time_seconds: f32,
        time_delta_seconds: f32,
        frame_index: u32,
        date: [f32; 4],
        keys: &[IsfInstanceKey],
    ) -> Result<VideoFrame, IsfRuntimeError> {
        self.last_stack_stage_count = stages.len();
        self.last_stack_render_us = 0;
        if stages.is_empty() {
            return Ok(frame.clone());
        }
        #[cfg(test)]
        {
            let sequence = self.resources.phase_timing.as_ref().map_or(1, |timing| timing.sequence + 1);
            self.resources.phase_timing = Some(phase_timing::StackPhaseTiming::new(sequence));
        }
        self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        #[cfg(test)]
        self.resources.phase_timing.as_mut().unwrap().mark(0);
        // An in-flight or failed stack cannot expose reusable scratch buffers.
        // Only the outer transaction, after both GPU scopes, can return them.
        let workspace = self.resources.workspace.take();
        let result = self.render_pass_stack_inner(
            frame,
            stages,
            time_seconds,
            time_delta_seconds,
            frame_index,
            date,
            keys,
            workspace,
        );
        // Failed validation can leave write_texture staging uploads pending.
        // Submit and drain them so repeated failures cannot retain unbounded uploads.
        let cleanup_failure = if result.is_err() {
            self.queue.submit(std::iter::empty::<wgpu::CommandBuffer>());
            self.device.poll(wgpu::PollType::wait()).err()
        } else {
            None
        };
        // Always balance both scopes, including a failed cleanup wait.
        let validation = pollster::block_on(self.device.pop_error_scope());
        let allocation = pollster::block_on(self.device.pop_error_scope());
        #[cfg(test)]
        self.resources.phase_timing.as_mut().unwrap().mark(8);
        if let Some(error) = validation.or(allocation) {
            for key in keys {
                self.resources.entries.remove(key);
            }
            return Err(runtime_error(format!("ISF GPU pass failed: {error}")));
        }
        if let Some(error) = cleanup_failure {
            for key in keys {
                self.resources.entries.remove(key);
            }
            return Err(runtime_error(format!(
                "ISF GPU upload cleanup failed: {error}"
            )));
        }
        result.map(|(frame, workspace)| {
            self.resources.workspace = Some(workspace);
            frame
        })
    }

    fn render_pass_stack_inner(
        &mut self,
        frame: &VideoFrame,
        stages: &[IsfStageInputs<'_>],
        time_seconds: f32,
        time_delta_seconds: f32,
        frame_index: u32,
        date: [f32; 4],
        keys: &[IsfInstanceKey],
        workspace: Option<StackWorkspace>,
    ) -> Result<(VideoFrame, StackWorkspace), IsfRuntimeError> {
        if keys.len() != stages.len() {
            return Err(runtime_error(
                "ISF stage ownership key count does not match the stack",
            ));
        }
        self.validate_pass_size(frame.width, frame.height)?;
        let padded_bytes_per_row = StackWorkspace::bytes_per_row(frame.width);
        // The encoder retains each intermediate allocation until submission.
        // Reserve upload, ping/pong and padded readback before creating them.
        let baseline_bytes = StackWorkspace::required_bytes(frame.width, frame.height);
        if baseline_bytes > MAX_CACHED_RESOURCE_BYTES {
            return Err(runtime_error(
                "ISF pass size exceeds the GPU buffer memory budget",
            ));
        }
        self.evict_resources_for(baseline_bytes);
        let expected_len = frame.width as usize * frame.height as usize * 4;
        // Upload borrows RGBA bytes; only format conversion owns a temporary.
        let rgba = if frame.format == VideoPixelFormat::Rgba8 {
            Cow::Borrowed(frame)
        } else {
            Cow::Owned(convert_frame_to_rgba8(frame).map_err(runtime_from_composite_error)?)
        };
        if rgba.data.len() != expected_len {
            return Err(runtime_error(
                "ISF input frame byte length does not match its dimensions",
            ));
        }
        // filter drops a mismatched old allocation before creating its
        // replacement; scratch is never retained per layer or per resolution.
        let workspace = workspace
            .filter(|workspace| workspace.matches(frame.width, frame.height))
            .unwrap_or_else(|| StackWorkspace::new(&self.device, frame.width, frame.height));
        let input = &workspace.input;
        let ping = &workspace.ping;
        let pong = &workspace.pong;
        let sampler = &workspace.sampler;
        let readback = &workspace.readback;
        #[cfg(test)]
        self.resources.phase_timing.as_mut().unwrap().mark(1);
        input.upload_rgba(&self.queue, &rgba.data);
        #[cfg(test)]
        self.resources.phase_timing.as_mut().unwrap().mark(2);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Syndocal ISF pass stack encoder"),
            });
        let mut current = input.clone();
        let mut pending_states = Vec::new();
        let mut pending_bytes = baseline_bytes;
        for (stage_index, (stage, key)) in stages.iter().zip(keys).enumerate() {
            let shader = stage.shader;
            let controls = stage.controls;
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
                    !self.pipelines.contains_key(&(
                        source_hash(&shader.wgsl),
                        **float,
                        texture_count,
                    ))
                })
                .count();
            if self.pipelines.len() + missing > ISF_GPU_PIPELINE_CAPACITY {
                self.pipelines.clear();
            }
            for float in formats {
                self.ensure_pass_pipeline(&shader.wgsl, texture_count, float)?;
            }
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
                    let rgba = if image.format == VideoPixelFormat::Rgba8 {
                        Cow::Borrowed(image)
                    } else {
                        Cow::Owned(
                            convert_frame_to_rgba8(image).map_err(runtime_from_composite_error)?,
                        )
                    };
                    if rgba.data.len() != image.width as usize * image.height as usize * 4 {
                        return Err(runtime_error(
                            "ISF image input byte length does not match its dimensions",
                        ));
                    }
                    Ok(rgba)
                })
                .collect::<Result<Vec<_>, IsfRuntimeError>>()?;
            let mut state = self.take_stage_resources(
                shader,
                controls,
                frame,
                key,
                pending_bytes,
                dynamic_bytes,
            )?;
            let dynamic_images = dynamic_rgba
                .iter()
                .map(|rgba| {
                    let gpu = GpuImage::new(
                        &self.device,
                        rgba.width,
                        rgba.height,
                        false,
                        "Syndocal ISF image input",
                    );
                    gpu.upload_rgba(&self.queue, &rgba.data);
                    gpu
                })
                .collect::<Vec<_>>();
            let transparent = needs_transparent.then(|| {
                let image = GpuImage::new(&self.device, 1, 1, false, "Syndocal ISF unbound image");
                image.upload_rgba(&self.queue, &[0, 0, 0, 0]);
                image
            });
            let audio_identity = self
                .audio_frame
                .as_ref()
                .map(|frame| (frame.generation, frame.feature_sequence));
            if !state.audio_initialized || state.audio_identity != audio_identity {
                for (input, image) in shader.audio_inputs.iter().zip(&state.audio) {
                    let pixels = audio_texture_rgba(input, self.audio_frame.as_ref());
                    image.upload_rgba(&self.queue, &pixels[..input.width as usize * 4]);
                }
                state.audio_identity = audio_identity;
                state.audio_initialized = true;
            }
            for target in &state.targets {
                if !target.persistent || !state.has_history {
                    clear_image(&mut encoder, &target.images[target.front]);
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
                    GpuImage::new(
                        &self.device,
                        width,
                        height,
                        pass.float,
                        "Syndocal ISF unnamed pass",
                    )
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
                let uniforms = self.pass_uniforms(
                    width,
                    height,
                    time_seconds,
                    time_delta_seconds,
                    frame_index,
                    pass_index,
                    date,
                    controls,
                );
                self.encode_pass(
                    &mut encoder,
                    &shader.wgsl,
                    &sampled,
                    sampler,
                    &uniforms,
                    &output,
                );
                if let Some(index) = target_index {
                    state.targets[index].front = 1 - state.targets[index].front;
                }
                current = output;
            }
            // The host frame keeps its requested dimensions/format, including
            // a final named FLOAT target or a size expression on the last pass.
            if current.width != frame.width || current.height != frame.height || current.float {
                self.ensure_pass_pipeline(BLIT_SHADER, 1, false)?;
                let uniforms = self.pass_uniforms(
                    frame.width,
                    frame.height,
                    time_seconds,
                    time_delta_seconds,
                    frame_index,
                    0,
                    date,
                    &[],
                );
                self.encode_pass(
                    &mut encoder,
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
            if state.bytes != 0 {
                pending_states.push((key.clone(), state));
            }
        }
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
        #[cfg(test)]
        self.resources.phase_timing.as_mut().unwrap().mark(3);
        let started = std::time::Instant::now();
        self.queue.submit(Some(encoder.finish()));
        #[cfg(test)]
        self.resources.phase_timing.as_mut().unwrap().mark(4);
        let (sender, receiver) = mpsc::sync_channel(1);
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result.map_err(|error| error.to_string()));
            });
        self.device
            .poll(wgpu::PollType::wait())
            .map_err(|error| runtime_error(format!("ISF GPU wait failed: {error}")))?;
        receiver
            .recv()
            .map_err(|error| runtime_error(format!("ISF readback channel failed: {error}")))?
            .map_err(|error| runtime_error(format!("ISF readback failed: {error}")))?;
        #[cfg(test)]
        self.resources.phase_timing.as_mut().unwrap().mark(5);
        self.last_stack_render_us = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
        let mapped = readback.slice(..).get_mapped_range();
        let data = copy_rgba_readback(
            &mapped,
            (frame.width * 4) as usize,
            padded_bytes_per_row as usize,
            expected_len,
        );
        drop(mapped);
        readback.unmap();
        #[cfg(test)]
        self.resources.phase_timing.as_mut().unwrap().mark(6);
        // Commit feedback ownership only after the entire submitted stack has
        // produced a readable result. A failed stage cannot publish history.
        for (key, state) in pending_states {
            self.resources.entries.insert(key, state);
        }
        while self.resources.entries.len() > RESOURCE_CACHE_CAPACITY {
            let oldest = self
                .resources
                .entries
                .iter()
                .min_by_key(|(_, state)| state.last_used)
                .map(|(key, _)| key.clone())
                .expect("nonempty resource pool exceeds its capacity");
            self.resources.entries.remove(&oldest);
        }
        #[cfg(test)]
        self.resources.phase_timing.as_mut().unwrap().mark(7);
        Ok((
            VideoFrame {
                layer_id: frame.layer_id,
                width: frame.width,
                height: frame.height,
                pts_ms: frame.pts_ms,
                duration_ms: frame.duration_ms,
                format: VideoPixelFormat::Rgba8,
                data,
            },
            workspace,
        ))
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
        while self.resources.entries.len() > RESOURCE_CACHE_CAPACITY
            || self
                .resources
                .entries
                .values()
                .map(|state| state.bytes)
                .sum::<u64>()
                + reserved_bytes
                > MAX_CACHED_RESOURCE_BYTES
        {
            let Some(oldest) = self
                .resources
                .entries
                .iter()
                .min_by_key(|(_, state)| state.last_used)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
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

    fn take_stage_resources(
        &mut self,
        shader: &PreparedIsfShader,
        controls: &[[f32; 4]],
        frame: &VideoFrame,
        key: &IsfInstanceKey,
        pending_bytes: u64,
        dynamic_bytes: u64,
    ) -> Result<StageResources, IsfRuntimeError> {
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
        self.resources.clock = self.resources.clock.wrapping_add(1);
        let last_used = self.resources.clock;
        let reusable = self
            .resources
            .entries
            .remove(key)
            .filter(|state| state.fingerprint == shader.fingerprint && state.sizes == sizes);
        // A cache hit still allocates anonymous pass textures on this frame.
        self.evict_resources_for(pending_bytes + bytes + transient_bytes);
        if let Some(mut state) = reusable {
            state.last_used = last_used;
            state.transient_bytes = transient_bytes;
            return Ok(state);
        }
        let targets = shader
            .passes
            .iter()
            .zip(&sizes)
            .filter_map(|(pass, &(width, height))| {
                pass.target.as_ref().map(|name| TargetBuffer {
                    name: name.clone(),
                    persistent: pass.persistent,
                    front: 0,
                    images: [
                        GpuImage::new(
                            &self.device,
                            width,
                            height,
                            pass.float,
                            "Syndocal ISF target read",
                        ),
                        GpuImage::new(
                            &self.device,
                            width,
                            height,
                            pass.float,
                            "Syndocal ISF target write",
                        ),
                    ],
                })
            })
            .collect();
        let imports = shader
            .imported_images
            .iter()
            .map(|image| {
                let gpu = GpuImage::new(
                    &self.device,
                    image.width,
                    image.height,
                    false,
                    "Syndocal ISF imported image",
                );
                gpu.upload_rgba(&self.queue, &image.rgba);
                gpu
            })
            .collect();
        let bound_images = shader
            .bound_images
            .iter()
            .map(|image| {
                let gpu = GpuImage::new(
                    &self.device,
                    image.width,
                    image.height,
                    false,
                    "Syndocal ISF bound still",
                );
                gpu.upload_rgba(&self.queue, &image.rgba);
                gpu
            })
            .collect();
        let audio = shader
            .audio_inputs
            .iter()
            .map(|input| {
                GpuImage::new(
                    &self.device,
                    input.width,
                    1,
                    false,
                    "Syndocal ISF live audio",
                )
            })
            .collect();
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
            self.pipelines.clear();
        }
        let pipeline = self.create_pass_pipeline(wgsl, texture_count, float)?;
        self.pipelines.insert(key, pipeline);
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
