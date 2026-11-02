// Texture-source variant of gpu_display_composition.wgsl. Only RGBA8-unorm
// normalized post-FX results are admitted. Axis-aligned CPU lookup supplies
// exact integer coordinates; rotations use canonical nearest-neighbour sampling.
// textureLoad has no filtering, sRGB or premultiplication step.
struct Params {
    output_width: u32,
    output_height: u32,
    source_width: u32,
    source_height: u32,
    opacity: f32,
    blend_mode: u32,
    clear_color: u32,
    padding: u32,
};
const NOT_COVERED: u32 = 0xffffffffu;
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> output_pixels: array<u32>;
@group(0) @binding(2) var<uniform> params: Params;
@group(0) @binding(3) var<storage, read> source_lookup: array<u32>;


// Returns canonical integer nearest-neighbour coordinates. The factorized
// rotated case uploads only the two coordinate axes and eight transform words.
fn resident_coordinates(x: u32, y: u32) -> vec2<u32> {
    if (params.padding == 0u) {
        return vec2<u32>(source_lookup[x], source_lookup[params.output_width + y]);
    }
    let base = params.output_width + params.output_height;
    let cx = bitcast<f32>(source_lookup[x]);
    let cy = bitcast<f32>(source_lookup[params.output_width + y]);
    let sine = bitcast<f32>(source_lookup[base]);
    let cosine = bitcast<f32>(source_lookup[base + 1u]);
    let rotated_x = cx * cosine - cy * sine;
    let rotated_y = cx * sine + cy * cosine;
    let local_x = rotated_x / bitcast<f32>(source_lookup[base + 2u]) + 0.5;
    let local_y = rotated_y / bitcast<f32>(source_lookup[base + 3u]) + 0.5;
    let span_x = bitcast<f32>(source_lookup[base + 6u]);
    let span_y = bitcast<f32>(source_lookup[base + 7u]);
    if (local_x < 0.0 || local_x >= 1.0 || local_y < 0.0 || local_y >= 1.0 || span_x <= 0.0 || span_y <= 0.0) {
        return vec2<u32>(NOT_COVERED);
    }
    let u = bitcast<f32>(source_lookup[base + 4u]) + local_x * span_x;
    let v = bitcast<f32>(source_lookup[base + 5u]) + local_y * span_y;
    return vec2<u32>(
        u32(clamp(floor(u * f32(params.source_width)), 0.0, f32(params.source_width - 1u))),
        u32(clamp(floor(v * f32(params.source_height)), 0.0, f32(params.source_height - 1u))),
    );
}

fn sample(x: u32, y: u32) -> vec4<f32> {
    // Restore original normalized 8-bit channel values before canonical blend.
    return floor(textureLoad(source, vec2<i32>(i32(x), i32(y)), 0) * 255.0 + 0.5);
}

@compute @workgroup_size(8, 8)
fn clear_output(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < params.output_width && id.y < params.output_height) {
        output_pixels[id.y * params.output_width + id.x] = params.clear_color;
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
    let coordinates = resident_coordinates(output_x, output_y);
    let source_x = coordinates.x;
    let source_y = coordinates.y;
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
