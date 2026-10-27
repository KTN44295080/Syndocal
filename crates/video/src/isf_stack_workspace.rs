//! One in-flight stack's scratch resources, separate from effect feedback.
use super::GpuImage;

// Reviewed against wgpu-{core,hal} 25.0.2 (see the lock-version regression).
// Called once while creating the runtime, never on the rendering hot path.
pub(in crate::isf_runtime) fn queue_upload_pitch(adapter: &wgpu::Adapter) -> Option<u64> {
    let pitch = match adapter.get_info().backend {
        wgpu::Backend::Vulkan => {
            #[cfg(any(
                windows,
                target_os = "linux",
                target_os = "android",
                target_os = "freebsd"
            ))]
            {
                // SAFETY: only read properties of the live adapter through its
                // owning instance; no handles escape or are destroyed. No wgpu
                // operation is called from inside the as_hal read guard.
                unsafe {
                    adapter.as_hal::<wgpu::hal::api::Vulkan, _, _>(|hal| {
                        hal.map(|hal| {
                            hal.shared_instance()
                                .raw_instance()
                                .get_physical_device_properties(hal.raw_physical_device())
                                .limits
                                .optimal_buffer_copy_row_pitch_alignment
                        })
                    })
                }
            }
            #[cfg(not(any(
                windows,
                target_os = "linux",
                target_os = "android",
                target_os = "freebsd"
            )))]
            {
                None
            }
        }
        // hal dx12/adapter.rs:610; metal/adapter.rs:1070; gles/adapter.rs:855.
        wgpu::Backend::Dx12 => Some(256),
        wgpu::Backend::Metal | wgpu::Backend::Gl => Some(4),
        // Browser WebGPU, noop and any future backend have no established bound.
        _ => None,
    }?;
    // core queue.rs:772 casts to u32 and takes the LCM with RGBA8's 4-byte
    // block size. Power-of-two pitches make max(pitch, 256) a conservative
    // multiple of that LCM and preserve our existing 256-byte reservation.
    (pitch.is_power_of_two() && pitch <= u64::from(u32::MAX)).then_some(pitch)
}

#[derive(Clone)]
pub(super) struct StackWorkspace {
    pub(super) input: GpuImage,
    pub(super) ping: GpuImage,
    pub(super) pong: GpuImage,
    pub(super) readback: Option<wgpu::Buffer>,
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

    // Queue writes may retain a backend-padded upload until submission ends.
    // Unknown/invalid backend pitch must decline before moving scratch/upload.
    pub(super) fn upload_staging_bytes(width: u32, height: u32, pitch: u64) -> Option<u64> {
        if !pitch.is_power_of_two() || pitch > u64::from(u32::MAX) {
            return None;
        }
        let alignment = pitch.max(u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT));
        u64::from(width)
            .checked_mul(4)?
            .checked_add(alignment.checked_sub(1)?)?
            .checked_div(alignment)?
            .checked_mul(alignment)?
            .checked_mul(u64::from(height))
    }

    pub(super) fn bytes_per_row(width: u32) -> u32 {
        (width * 4).div_ceil(256) * 256
    }

    pub(super) fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        Self::create(device, width, height, true)
    }

    pub(super) fn resident(device: &wgpu::Device, width: u32, height: u32) -> Self {
        Self::create(device, width, height, false)
    }

    fn create(device: &wgpu::Device, width: u32, height: u32, readback: bool) -> Self {
        Self {
            input: GpuImage::new(device, width, height, false, "Syndocal ISF input"),
            ping: GpuImage::new(device, width, height, false, "Syndocal ISF stack ping"),
            pong: GpuImage::new(device, width, height, false, "Syndocal ISF stack pong"),
            readback: readback.then(|| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Syndocal ISF readback"),
                    size: u64::from(Self::bytes_per_row(width)) * u64::from(height),
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                })
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
