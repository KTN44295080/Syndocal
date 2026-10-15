// Composites one layer of a native Display GPU composition
// (`VideoOutputGpuComposition`): an unrotated layer with no colour
// adjustment or effect, blended onto `output_pixels` as the CPU's
// `blend_pixel` does.
//
// `source` holds the layer's frame bytes as decoded, followed at
// `params.lookup_offset` (in words) by its sampling lookup
// (`axis_aligned_source_lookup`): the source column of each output column,
// then the source row of each output row, NOT_COVERED outside the layer.
// Which pixel to sample is never computed here, and DXT blocks are decoded
// with the CPU decoder's integer arithmetic, so every sample is the CPU's.
struct Params {
    output_width: u32,
    output_height: u32,
    source_width: u32,
    source_height: u32,
    opacity: f32,
    blend_mode: u32,
    source_format: u32,
    lookup_offset: u32,
};

const NOT_COVERED: u32 = 0xffffffffu;
// params.source_format: 0 RGBA8, 1 BGRA8, 2 DXT1, 3 DXT5.

@group(0) @binding(0)
var<storage, read> source: array<u32>;

@group(0) @binding(1)
var<storage, read_write> output_pixels: array<u32>;

@group(0) @binding(2)
var<uniform> params: Params;

// `rgb565_to_rgba8`.
fn rgb565(value: u32) -> vec3<u32> {
    let red = (value >> 11u) & 0x1fu;
    let green = (value >> 5u) & 0x3fu;
    let blue = value & 0x1fu;
    return vec3<u32>(
        (red << 3u) | (red >> 2u),
        (green << 2u) | (green >> 4u),
        (blue << 3u) | (blue >> 2u),
    );
}

// `dxt_color_palette` entry for `pixel` (0..16) of a colour block.
fn dxt_color(colors: u32, indices: u32, pixel: u32, dxt1: bool) -> vec4<u32> {
    let color0 = colors & 0xffffu;
    let color1 = colors >> 16u;
    let c0 = rgb565(color0);
    let c1 = rgb565(color1);
    let index = (indices >> (pixel * 2u)) & 3u;
    if (index == 0u) {
        return vec4<u32>(c0, 255u);
    }
    if (index == 1u) {
        return vec4<u32>(c1, 255u);
    }
    if (!dxt1 || color0 > color1) {
        if (index == 2u) {
            return vec4<u32>((c0 * 2u + c1) / 3u, 255u);
        }
        return vec4<u32>((c0 + c1 * 2u) / 3u, 255u);
    }
    if (index == 2u) {
        return vec4<u32>((c0 + c1) / 2u, 255u);
    }
    return vec4<u32>(0u);
}

// `dxt5_alpha_palette` entry `index` (0..8).
fn dxt5_alpha(alpha0: u32, alpha1: u32, index: u32) -> u32 {
    if (index == 0u) {
        return alpha0;
    }
    if (index == 1u) {
        return alpha1;
    }
    if (alpha0 > alpha1) {
        return ((8u - index) * alpha0 + (index - 1u) * alpha1) / 7u;
    }
    if (index == 6u) {
        return 0u;
    }
    if (index == 7u) {
        return 255u;
    }
    return ((6u - index) * alpha0 + (index - 1u) * alpha1) / 5u;
}

// The source pixel as the 8-bit RGBA channels the CPU blends.
fn sample(x: u32, y: u32) -> vec4<f32> {
    switch params.source_format {
        case 1u: {
            let word = source[y * params.source_width + x];
            return vec4<f32>(
                f32((word >> 16u) & 255u),
                f32((word >> 8u) & 255u),
                f32(word & 255u),
                f32(word >> 24u),
            );
        }
        case 2u: {
            let blocks_x = (params.source_width + 3u) / 4u;
            let block = ((y / 4u) * blocks_x + x / 4u) * 2u;
            let pixel = (y % 4u) * 4u + x % 4u;
            return vec4<f32>(dxt_color(source[block], source[block + 1u], pixel, true));
        }
        case 3u: {
            let blocks_x = (params.source_width + 3u) / 4u;
            let block = ((y / 4u) * blocks_x + x / 4u) * 4u;
            let pixel = (y % 4u) * 4u + x % 4u;
            // The 48 alpha index bits follow the two alpha endpoints.
            let alpha_word = source[block];
            let alpha_bits_low = (alpha_word >> 16u) | (source[block + 1u] << 16u);
            let alpha_bits_high = source[block + 1u] >> 16u;
            let shift = pixel * 3u;
            var alpha_index: u32;
            if (shift + 3u <= 32u) {
                alpha_index = (alpha_bits_low >> shift) & 7u;
            } else if (shift >= 32u) {
                alpha_index = (alpha_bits_high >> (shift - 32u)) & 7u;
            } else {
                alpha_index = ((alpha_bits_low >> shift) | (alpha_bits_high << (32u - shift))) & 7u;
            }
            let color = dxt_color(source[block + 2u], source[block + 3u], pixel, false);
            let alpha = dxt5_alpha(alpha_word & 255u, (alpha_word >> 8u) & 255u, alpha_index);
            return vec4<f32>(vec3<f32>(color.rgb), f32(alpha));
        }
        default: {
            let word = source[y * params.source_width + x];
            return vec4<f32>(
                f32(word & 255u),
                f32((word >> 8u) & 255u),
                f32((word >> 16u) & 255u),
                f32(word >> 24u),
            );
        }
    }
}

fn unpack_rgb(pixel: u32) -> vec3<f32> {
    return vec3<f32>(
        f32(pixel & 255u),
        f32((pixel >> 8u) & 255u),
        f32((pixel >> 16u) & 255u),
    );
}

fn unpack_alpha(pixel: u32) -> f32 {
    return f32((pixel >> 24u) & 255u);
}

fn rounded_channel(value: f32) -> u32 {
    return u32(clamp(floor(value + 0.5), 0.0, 255.0));
}

fn pack_rgba(rgb: vec3<f32>, alpha: f32) -> u32 {
    return rounded_channel(rgb.x)
        | (rounded_channel(rgb.y) << 8u)
        | (rounded_channel(rgb.z) << 16u)
        | (rounded_channel(alpha) << 24u);
}

@compute @workgroup_size(8, 8)
fn composite_layer(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let output_x = invocation_id.x;
    let output_y = invocation_id.y;
    if (output_x >= params.output_width || output_y >= params.output_height) {
        return;
    }
    let source_x = source[params.lookup_offset + output_x];
    let source_y = source[params.lookup_offset + params.output_width + output_y];
    if (source_x == NOT_COVERED || source_y == NOT_COVERED) {
        return;
    }
    let sampled = sample(source_x, source_y);
    let source_alpha = (sampled.a / 255.0) * clamp(params.opacity, 0.0, 1.0);
    if (source_alpha <= 0.0) {
        return;
    }

    let index = output_y * params.output_width + output_x;
    let destination = output_pixels[index];
    let source_rgb = sampled.rgb;
    let destination_rgb = unpack_rgb(destination);
    var blended = source_rgb;
    switch params.blend_mode {
        case 1u: {
            blended = min(destination_rgb + source_rgb, vec3<f32>(255.0));
        }
        case 2u: {
            blended = destination_rgb * source_rgb / 255.0;
        }
        case 3u: {
            blended = vec3<f32>(255.0) - ((vec3<f32>(255.0) - destination_rgb) * (vec3<f32>(255.0) - source_rgb) / 255.0);
        }
        default: {}
    }

    let output_rgb = destination_rgb + (blended - destination_rgb) * source_alpha;
    let destination_alpha = unpack_alpha(destination) / 255.0;
    let output_alpha = (source_alpha + destination_alpha * (1.0 - source_alpha)) * 255.0;
    output_pixels[index] = pack_rgba(output_rgb, output_alpha);
}
