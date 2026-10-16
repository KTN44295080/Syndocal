//! Canonical deterministic RGBA8 generator path. No wall clock, random state,
//! platform trigonometry or floating point enters the generated pixels.
use super::*;
use protocol::VideoGenerator;

const CACHE_BYTES: usize = 128 * 1024 * 1024;
const CACHE_ENTRIES: usize = 16;
const INTERNAL_LONG_EDGE: u32 = 512;
const ONE: u32 = 65536;

/// Generated static frames keep PTS zero so their cached storage is reusable.
/// Retire frames from a previous source identity before queue selection can
/// prefer an older animated frame with a later PTS over its replacement.
#[derive(Default)]
pub(super) struct GeneratorQueueSources {
    identities: HashMap<(protocol::VideoRenderInputKey, VideoLayerId), VideoGenerator>,
}

pub(super) fn input_identity(
    input: &VideoRenderInput,
) -> (
    protocol::VideoRenderInputKey,
    VideoLayerId,
    Option<VideoGenerator>,
) {
    let generator = match input.request.source.kind {
        VideoSourceKind::Generator(generator) => Some(generator),
        _ => None,
    };
    (input.key, input.request.layer_id, generator)
}

impl GeneratorQueueSources {
    /// Keyed-input render entry points own no derived layer output queue.
    pub(super) fn sync_inputs(&mut self, runtime: &mut VideoRuntime, inputs: &[VideoRenderInput]) {
        self.sync(runtime, false, inputs.iter().map(input_identity));
    }

    pub(super) fn sync(
        &mut self,
        runtime: &mut VideoRuntime,
        layer_outputs: bool,
        sources: impl Iterator<
                Item = (
                    protocol::VideoRenderInputKey,
                    VideoLayerId,
                    Option<VideoGenerator>,
                ),
            > + Clone,
    ) {
        self.identities.retain(|&(key, layer_id), _| {
            let retained = sources.clone().any(|(current_key, current_layer_id, _)| {
                current_key == key && current_layer_id == layer_id
            });
            if !retained {
                retire_queue(runtime, key, layer_id, layer_outputs);
            }
            retained
        });
        for (key, layer_id, generator) in sources {
            let previous = match generator {
                Some(generator) => self.identities.insert((key, layer_id), generator),
                None => self.identities.remove(&(key, layer_id)),
            };
            if previous != generator {
                retire_queue(runtime, key, layer_id, layer_outputs);
            }
        }
    }
}

fn retire_queue(
    runtime: &mut VideoRuntime,
    key: protocol::VideoRenderInputKey,
    layer_id: VideoLayerId,
    layer_outputs: bool,
) {
    // A clip transition queues its blended result under the legacy layer key.
    // Either keyed input changing invalidates that result as well.
    runtime.queues.retain(|queue| {
        queue.layer_id() != layer_id
            || (queue.key() != key
                && (!layer_outputs || queue.key() != protocol::VideoRenderInputKey::LEGACY))
    });
}

#[derive(Default)]
pub(super) struct GeneratorFrameCache {
    entries: Vec<Entry>,
}

struct Entry {
    key: protocol::VideoRenderInputKey,
    generator: VideoGenerator,
    tick: u64,
    frame: Arc<VideoFrame>,
}

impl GeneratorFrameCache {
    pub(super) fn retain_layers(&mut self, ids: &[VideoLayerId]) {
        self.entries
            .retain(|entry| ids.contains(&entry.frame.layer_id));
    }

    pub(super) fn retain_inputs(&mut self, inputs: &[VideoRenderInput]) {
        self.entries.retain(|entry| {
            inputs.iter().any(|input| {
                input.key == entry.key && input.request.layer_id == entry.frame.layer_id
            })
        });
    }

    pub(super) fn frame(
        &mut self,
        input: &VideoRenderInput,
        generator: VideoGenerator,
        cancellation: Option<&dyn VideoRenderCancellation>,
    ) -> Result<Arc<VideoFrame>, VideoFrameProviderError> {
        let request = &input.request;
        let invalid = |message: String| VideoFrameProviderError::Decode {
            layer_id: request.layer_id,
            label: request.label.clone(),
            error: VideoDecodeError::Decode {
                layer_id: request.layer_id,
                label: request.label.clone(),
                message: format!("Invalid generator source: {message}"),
            },
        };
        generator
            .validate_source(&request.source)
            .map_err(invalid)?;
        let bytes = (request.width as usize)
            .checked_mul(request.height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .filter(|bytes| *bytes > 0 && *bytes <= CACHE_BYTES)
            .ok_or_else(|| {
                invalid("requested RGBA8 size must be positive and at most 128 MiB".into())
            })?;
        if cancellation.is_some_and(VideoRenderCancellation::is_cancelled) {
            return Err(VideoFrameProviderError::Cancelled);
        }
        let tick = match generator {
            VideoGenerator::Noise { speed, .. } if speed > 0 => {
                ((request.position_ms as u128 * 30) / 1000) as u64
            }
            _ => 0,
        };
        if let Some(index) = self.entries.iter().position(|entry| {
            entry.key == input.key
                && entry.frame.layer_id == request.layer_id
                && entry.generator == generator
                && entry.tick == tick
                && entry.frame.width == request.width
                && entry.frame.height == request.height
        }) {
            let entry = self.entries.remove(index);
            let frame = Arc::clone(&entry.frame);
            self.entries.push(entry);
            return Ok(frame);
        }
        // One current frame per source/size; previous frame ownership remains
        // with the runtime queue until it releases it.
        self.entries.retain(|entry| {
            entry.key != input.key
                || entry.frame.layer_id != request.layer_id
                || entry.frame.width != request.width
                || entry.frame.height != request.height
        });
        while self.entries.len() >= CACHE_ENTRIES
            || self
                .entries
                .iter()
                .map(|entry| entry.frame.data.len())
                .sum::<usize>()
                + bytes
                > CACHE_BYTES
        {
            self.entries.remove(0);
        }
        let data = render(generator, request.width, request.height, tick, cancellation)?;
        cancelled(cancellation)?;
        let animated = matches!(generator, VideoGenerator::Noise { speed, .. } if speed > 0);
        let pts_ms = if animated {
            (tick as u128 * 1000).div_ceil(30) as u64
        } else {
            0
        };
        let frame = Arc::new(VideoFrame {
            layer_id: request.layer_id,
            width: request.width,
            height: request.height,
            pts_ms,
            duration_ms: if animated {
                (((tick as u128 + 1) * 1000).div_ceil(30) - pts_ms as u128) as u64
            } else {
                u64::MAX
            },
            format: VideoPixelFormat::Rgba8,
            data,
        });
        self.entries.push(Entry {
            key: input.key,
            generator,
            tick,
            frame: Arc::clone(&frame),
        });
        Ok(frame)
    }
}

fn cancelled(
    cancellation: Option<&dyn VideoRenderCancellation>,
) -> Result<(), VideoFrameProviderError> {
    if cancellation.is_some_and(VideoRenderCancellation::is_cancelled) {
        Err(VideoFrameProviderError::Cancelled)
    } else {
        Ok(())
    }
}

fn mix(a: u32, b: u32, t: u32) -> u32 {
    ((a as u64 * (ONE - t) as u64 + b as u64 * t as u64 + 32768) >> 16) as u32
}

fn color(a: [u8; 4], b: [u8; 4], t: u32) -> [u8; 4] {
    std::array::from_fn(|channel| mix(a[channel] as u32, b[channel] as u32, t) as u8)
}

fn smooth(t: u32) -> u32 {
    let square = (t as u64 * t as u64) >> 16;
    ((square * (3 * ONE - 2 * t) as u64) >> 16) as u32
}

fn hash(seed: u32, x: u32, y: u32, z: u32) -> u32 {
    let mut v =
        seed ^ x.wrapping_mul(0x9e3779b9) ^ y.wrapping_mul(0x85ebca6b) ^ z.wrapping_mul(0xc2b2ae35);
    v ^= v >> 16;
    v = v.wrapping_mul(0x7feb352d);
    v ^= v >> 15;
    v = v.wrapping_mul(0x846ca68b);
    (v ^ (v >> 16)) >> 16
}

fn noise(seed: u32, x: u32, y: u32, z: u64) -> u32 {
    let (ix, iy, iz) = (x >> 16, y >> 16, (z >> 16) as u32);
    let (tx, ty, tz) = (
        smooth(x & 65535),
        smooth(y & 65535),
        smooth((z & 65535) as u32),
    );
    let plane = |z| {
        mix(
            mix(hash(seed, ix, iy, z), hash(seed, ix + 1, iy, z), tx),
            mix(hash(seed, ix, iy + 1, z), hash(seed, ix + 1, iy + 1, z), tx),
            ty,
        )
    };
    mix(plane(iz), plane(iz.wrapping_add(1)), tz)
}

// Q15 sine, degrees 0..=90. Baked literals specify every platform's angles.
const SIN: [i32; 91] = [
    0, 572, 1144, 1715, 2286, 2856, 3425, 3993, 4560, 5126, 5690, 6252, 6813, 7371, 7927, 8481,
    9032, 9580, 10126, 10668, 11207, 11743, 12275, 12803, 13328, 13848, 14365, 14876, 15384, 15886,
    16384, 16877, 17364, 17847, 18324, 18795, 19261, 19720, 20174, 20622, 21063, 21498, 21926,
    22348, 22763, 23170, 23571, 23965, 24351, 24730, 25102, 25466, 25822, 26170, 26510, 26842,
    27166, 27482, 27789, 28088, 28378, 28660, 28932, 29197, 29452, 29698, 29935, 30163, 30382,
    30592, 30792, 30983, 31164, 31336, 31499, 31651, 31795, 31928, 32052, 32166, 32270, 32365,
    32449, 32524, 32589, 32644, 32688, 32723, 32748, 32763, 32768,
];

fn sin(angle: u16) -> i32 {
    match angle % 360 {
        a @ 0..=90 => SIN[a as usize],
        a @ 91..=180 => SIN[(180 - a) as usize],
        a @ 181..=270 => -SIN[(a - 180) as usize],
        a => -SIN[(360 - a) as usize],
    }
}

fn render(
    generator: VideoGenerator,
    width: u32,
    height: u32,
    tick: u64,
    cancellation: Option<&dyn VideoRenderCancellation>,
) -> Result<Vec<u8>, VideoFrameProviderError> {
    let mut data = vec![0; width as usize * height as usize * 4];
    match generator {
        VideoGenerator::Solid { color } => {
            for row in data.chunks_exact_mut(width as usize * 4) {
                cancelled(cancellation)?;
                for pixel in row.chunks_exact_mut(4) {
                    pixel.copy_from_slice(&color);
                }
            }
        }
        VideoGenerator::LinearGradient {
            color_a,
            color_b,
            angle_degrees,
        } => {
            let dx = sin((angle_degrees + 90) % 360) as i64;
            let dy = sin(angle_degrees) as i64;
            let ex = dx * (width - 1) as i64;
            let ey = dy * (height - 1) as i64;
            let minimum = ex.min(0) + ey.min(0);
            let extent = ex.abs() + ey.abs();
            for (y, row) in data.chunks_exact_mut(width as usize * 4).enumerate() {
                cancelled(cancellation)?;
                for (x, pixel) in row.chunks_exact_mut(4).enumerate() {
                    let t = if extent == 0 {
                        ONE / 2
                    } else {
                        (((dx * x as i64 + dy * y as i64 - minimum) * ONE as i64 + extent / 2)
                            / extent) as u32
                    };
                    pixel.copy_from_slice(&color(color_a, color_b, t));
                }
            }
        }
        VideoGenerator::Noise {
            seed,
            scale,
            speed,
            color_a,
            color_b,
        } => {
            let longest = width.max(height);
            let divisor = longest.div_ceil(INTERNAL_LONG_EDGE);
            let sw = width.div_ceil(divisor);
            let sh = height.div_ceil(divisor);
            let mut small = vec![[0u8; 4]; sw as usize * sh as usize];
            // Modulo is deliberate: the hash's integer time cell wraps at 2^32.
            let z =
                ((tick as u128 * speed as u128 * ONE as u128 / 3000) & ((1u128 << 48) - 1)) as u64;
            let denominator = sw.max(sh).saturating_sub(1).max(1) as u64;
            for y in 0..sh {
                cancelled(cancellation)?;
                let py = (y as u64 * scale as u64 * ONE as u64 / denominator) as u32;
                for x in 0..sw {
                    let px = (x as u64 * scale as u64 * ONE as u64 / denominator) as u32;
                    small[(y * sw + x) as usize] = color(color_a, color_b, noise(seed, px, py, z));
                }
            }
            let xs: Vec<usize> = (0..width)
                .map(|x| (x as u64 * sw as u64 / width as u64) as usize)
                .collect();
            for (y, row) in data.chunks_exact_mut(width as usize * 4).enumerate() {
                cancelled(cancellation)?;
                let sy = y * sh as usize / height as usize;
                for (pixel, &sx) in row.chunks_exact_mut(4).zip(&xs) {
                    pixel.copy_from_slice(&small[sy * sw as usize + sx]);
                }
            }
        }
    }
    Ok(data)
}

#[cfg(test)]
#[path = "generator_tests.rs"]
mod tests;
