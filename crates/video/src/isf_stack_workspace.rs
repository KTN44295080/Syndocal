//! One synchronous stack's scratch resources, separate from effect feedback.
use super::GpuImage;

pub(super) struct StackWorkspace {
    pub(super) input: GpuImage,
    pub(super) ping: GpuImage,
    pub(super) pong: GpuImage,
    pub(super) readback: wgpu::Buffer,
    pub(super) sampler: wgpu::Sampler,
}

impl StackWorkspace {
    pub(super) fn matches(&self, width: u32, height: u32) -> bool {
        self.input.width == width && self.input.height == height
    }

    // The same reservation applies to fresh and reused resources. Stage cache
    // eviction must leave room for all three RGBA textures and padded readback.
    pub(super) fn required_bytes(width: u32, height: u32) -> u64 {
        u64::from(width) * u64::from(height) * 12
            + u64::from(Self::bytes_per_row(width)) * u64::from(height)
    }

    pub(super) fn bytes_per_row(width: u32) -> u32 {
        (width * 4).div_ceil(256) * 256
    }

    pub(super) fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        Self {
            input: GpuImage::new(device, width, height, false, "Syndocal ISF input"),
            ping: GpuImage::new(device, width, height, false, "Syndocal ISF stack ping"),
            pong: GpuImage::new(device, width, height, false, "Syndocal ISF stack pong"),
            readback: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Syndocal ISF readback"),
                size: u64::from(Self::bytes_per_row(width)) * u64::from(height),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Syndocal ISF sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
        }
    }
}
