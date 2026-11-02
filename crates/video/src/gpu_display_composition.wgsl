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
    blend_sample(sampled, output_y * params.output_width + output_x);
}

fn blend_sample(sampled: vec4<f32>, index: u32) {
    let source_alpha = (sampled.a / 255.0) * clamp(params.opacity, 0.0, 1.0);
    if (source_alpha <= 0.0) {
        return;
    }

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
        case 4u: {
            let low = 2.0 * destination_rgb * source_rgb / 255.0;
            let high = vec3<f32>(255.0) - 2.0 * (vec3<f32>(255.0) - destination_rgb) * (vec3<f32>(255.0) - source_rgb) / 255.0;
            blended = select(high, low, destination_rgb <= vec3<f32>(127.5));
        }
        case 5u: {
            blended = max(destination_rgb - source_rgb, vec3<f32>(0.0));
        }
        case 6u: {
            blended = abs(destination_rgb - source_rgb);
        }
        case 7u: {
            blended = max(destination_rgb, source_rgb);
        }
        case 8u: {
            blended = min(destination_rgb, source_rgb);
        }
        case 9u: {
            let d = destination_rgb / 255.0;
            let s = source_rgb / 255.0;
            let curve = select(sqrt(d), ((16.0 * d - 12.0) * d + 4.0) * d, d <= vec3<f32>(0.25));
            let low = d - (1.0 - 2.0 * s) * d * (1.0 - d);
            let high = d + (2.0 * s - 1.0) * (curve - d);
            blended = select(high, low, s <= vec3<f32>(0.5)) * 255.0;
        }
        default: {}
    }

    let output_rgb = destination_rgb + (blended - destination_rgb) * source_alpha;
    let destination_alpha = unpack_alpha(destination) / 255.0;
    let output_alpha = (source_alpha + destination_alpha * (1.0 - source_alpha)) * 255.0;
    output_pixels[index] = pack_rgba(output_rgb, output_alpha);
}

// Resident groups consume a packed GPU buffer directly. This binding is used
// only by the resident entry point; the original native presenter keeps its
// three-binding layout and packed-frame lookup unchanged.
@group(0) @binding(3) var<storage, read> resident_lookup: array<u32>;

// Returns canonical integer nearest-neighbour coordinates. The factorized
// rotated case uploads only the two coordinate axes and eight transform words.
fn resident_coordinates(x: u32, y: u32) -> vec2<u32> {
    if (params.lookup_offset == 0u) {
        return vec2<u32>(resident_lookup[x], resident_lookup[params.output_width + y]);
    }
    let base = params.output_width + params.output_height;
    let cx = bitcast<f32>(resident_lookup[x]);
    let cy = bitcast<f32>(resident_lookup[params.output_width + y]);
    let sine = bitcast<f32>(resident_lookup[base]);
    let cosine = bitcast<f32>(resident_lookup[base + 1u]);
    let rotated_x = cx * cosine - cy * sine;
    let rotated_y = cx * sine + cy * cosine;
    let local_x = rotated_x / bitcast<f32>(resident_lookup[base + 2u]) + 0.5;
    let local_y = rotated_y / bitcast<f32>(resident_lookup[base + 3u]) + 0.5;
    let span_x = bitcast<f32>(resident_lookup[base + 6u]);
    let span_y = bitcast<f32>(resident_lookup[base + 7u]);
    if (local_x < 0.0 || local_x >= 1.0 || local_y < 0.0 || local_y >= 1.0 || span_x <= 0.0 || span_y <= 0.0) {
        return vec2<u32>(NOT_COVERED);
    }
    let u = bitcast<f32>(resident_lookup[base + 4u]) + local_x * span_x;
    let v = bitcast<f32>(resident_lookup[base + 5u]) + local_y * span_y;
    return vec2<u32>(
        u32(clamp(floor(u * f32(params.source_width)), 0.0, f32(params.source_width - 1u))),
        u32(clamp(floor(v * f32(params.source_height)), 0.0, f32(params.source_height - 1u))),
    );
}

@compute @workgroup_size(8, 8)
fn composite_resident_group(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.output_width || id.y >= params.output_height) { return; }
    let coordinates = resident_coordinates(id.x, id.y);
    if (coordinates.x == NOT_COVERED || coordinates.y == NOT_COVERED) { return; }
    let pixel = source[coordinates.y * params.source_width + coordinates.x];
    blend_sample(vec4<f32>(unpack_rgb(pixel), unpack_alpha(pixel)), id.y * params.output_width + id.x);
}
@compute @workgroup_size(8, 8)
fn clear_resident_output(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < params.output_width && id.y < params.output_height) {
        output_pixels[id.y * params.output_width + id.x] = params.source_format;
    }
}
@compute @workgroup_size(8, 8)
fn straighten_resident_output(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.output_width || id.y >= params.output_height) { return; }
    let index = id.y * params.output_width + id.x;
    let pixel = output_pixels[index];
    let alpha = pixel >> 24u;
    var rgb = vec3<u32>(pixel & 255u, (pixel >> 8u) & 255u, (pixel >> 16u) & 255u);
    if (alpha == 0u) { rgb = vec3<u32>(0u); }
    else if (alpha < 255u) { rgb = min((rgb * 255u + vec3<u32>(alpha / 2u)) / alpha, vec3<u32>(255u)); }
    output_pixels[index] = rgb.x | (rgb.y << 8u) | (rgb.z << 16u) | (alpha << 24u);
}
