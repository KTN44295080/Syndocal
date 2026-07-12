struct Params {
    width: u32,
    height: u32,
    _padding_0: u32,
    _padding_1: u32,
    offset_scale: vec4<f32>,
    rotation_lens_keystone: vec4<f32>,
    top_corners: vec4<f32>,
    bottom_corners: vec4<f32>,
    edge_blend: vec4<f32>,
    blend_black: vec4<f32>,
    mask_meta: vec4<f32>,
    mask_points: array<vec4<f32>, 8>,
};

@group(0) @binding(0)
var<storage, read> source_pixels: array<u32>;

@group(0) @binding(1)
var<storage, read_write> output_pixels: array<u32>;

@group(0) @binding(2)
var<uniform> params: Params;

@group(0) @binding(3)
var bitmap_mask_texture: texture_2d<f32>;

fn corner_offset(centered_x: f32, centered_y: f32) -> vec2<f32> {
    let u = clamp(centered_x + 0.5, 0.0, 1.0);
    let v = clamp(centered_y + 0.5, 0.0, 1.0);
    let top = mix(params.top_corners.xy, params.top_corners.zw, u);
    let bottom = mix(params.bottom_corners.xy, params.bottom_corners.zw, u);
    return mix(top, bottom, v);
}

fn point_segment_distance(point: vec2<f32>, start: vec2<f32>, end: vec2<f32>) -> f32 {
    let edge = end - start;
    let length_squared = dot(edge, edge);
    if (length_squared <= 0.000001) {
        return distance(point, start);
    }
    let t = clamp(dot(point - start, edge) / length_squared, 0.0, 1.0);
    return distance(point, start + edge * t);
}

fn polygon_mask_factor(u: f32, v: f32) -> f32 {
    let count = min(u32(max(params.mask_meta.x, 0.0)), 8u);
    if (count < 3u) {
        return 1.0;
    }
    let point = vec2<f32>(clamp(u, 0.0, 1.0), clamp(v, 0.0, 1.0));
    var inside = false;
    var minimum_distance = 1000.0;
    var previous = params.mask_points[count - 1u].xy;
    for (var index = 0u; index < 8u; index += 1u) {
        if (index < count) {
            let current = params.mask_points[index].xy;
            let crosses_y = (current.y > point.y) != (previous.y > point.y);
            if (crosses_y) {
                let crossing_x = (previous.x - current.x) * (point.y - current.y)
                    / (previous.y - current.y) + current.x;
                if (point.x < crossing_x) {
                    inside = !inside;
                }
            }
            minimum_distance = min(minimum_distance, point_segment_distance(point, previous, current));
            previous = current;
        }
    }
    let softness = clamp(params.mask_meta.z, 0.0, 0.5);
    var base = 0.0;
    if (inside) {
        if (softness <= 0.0001) {
            base = 1.0;
        } else {
            let t = clamp(minimum_distance / softness, 0.0, 1.0);
            base = t * t * (3.0 - 2.0 * t);
        }
    }
    return base;
}

fn bitmap_mask_sample(x: u32, y: u32) -> f32 {
    return textureLoad(bitmap_mask_texture, vec2<i32>(i32(x), i32(y)), 0).r;
}

fn bitmap_mask_factor(u: f32, v: f32) -> f32 {
    let width = min(u32(max(params.blend_black.z, 0.0)), 16u);
    let height = min(u32(max(params.blend_black.w, 0.0)), 16u);
    if (width == 0u || height == 0u) {
        return 1.0;
    }
    let x = clamp(u, 0.0, 1.0) * f32(width - 1u);
    let y = clamp(v, 0.0, 1.0) * f32(height - 1u);
    let x0 = u32(floor(x));
    let y0 = u32(floor(y));
    let x1 = min(x0 + 1u, width - 1u);
    let y1 = min(y0 + 1u, height - 1u);
    let top = mix(bitmap_mask_sample(x0, y0), bitmap_mask_sample(x1, y0), x - f32(x0));
    let bottom = mix(bitmap_mask_sample(x0, y1), bitmap_mask_sample(x1, y1), x - f32(x0));
    return mix(top, bottom, y - f32(y0));
}

fn combined_mask_factor(u: f32, v: f32) -> f32 {
    let polygon_enabled = params.mask_meta.x >= 3.0;
    let bitmap_enabled = params.blend_black.z > 0.0 && params.blend_black.w > 0.0;
    if (!polygon_enabled && !bitmap_enabled) {
        return 1.0;
    }
    let base = clamp(polygon_mask_factor(u, v) * bitmap_mask_factor(u, v), 0.0, 1.0);
    if (params.mask_meta.y > 0.5) {
        return 1.0 - base;
    }
    return base;
}

fn smooth_edge(value: f32, width: f32) -> f32 {
    if (width <= 0.0001) {
        return 1.0;
    }
    let t = clamp(value / width, 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

fn edge_blend_factor(u: f32, v: f32) -> f32 {
    let mask = smooth_edge(u, params.edge_blend.x)
        * smooth_edge(1.0 - u, params.edge_blend.y)
        * smooth_edge(v, params.edge_blend.z)
        * smooth_edge(1.0 - v, params.edge_blend.w);
    return pow(clamp(mask, 0.0, 1.0), max(params.blend_black.x, 0.1));
}

@compute @workgroup_size(8, 8)
fn map_output(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let output_x = invocation_id.x;
    let output_y = invocation_id.y;
    if (output_x >= params.width || output_y >= params.height) {
        return;
    }
    let index = output_y * params.width + output_x;
    let centered_x = (f32(output_x) + 0.5) / f32(params.width) - 0.5;
    let centered_y = (f32(output_y) + 0.5) / f32(params.height) - 0.5;
    let corner = corner_offset(centered_x, centered_y);
    var x = centered_x - corner.x - params.rotation_lens_keystone.z * centered_y;
    var y = centered_y - corner.y - params.rotation_lens_keystone.w * centered_x;
    x -= params.offset_scale.x;
    y -= params.offset_scale.y;

    let sine = sin(params.rotation_lens_keystone.x);
    let cosine = cos(params.rotation_lens_keystone.x);
    let rotated_x = x * cosine - y * sine;
    let rotated_y = x * sine + y * cosine;
    var source_x = rotated_x / params.offset_scale.z;
    var source_y = rotated_y / params.offset_scale.w;
    let distortion = params.rotation_lens_keystone.y;
    if (abs(distortion) > 0.0001) {
        let radius_squared = source_x * source_x + source_y * source_y;
        let factor = max(1.0 + distortion * radius_squared * 1.5, 0.05);
        source_x *= factor;
        source_y *= factor;
    }
    if (source_x < -0.5 || source_x >= 0.5 || source_y < -0.5 || source_y >= 0.5) {
        return;
    }

    let pixel_x = min(u32(floor((source_x + 0.5) * f32(params.width))), params.width - 1u);
    let pixel_y = min(u32(floor((source_y + 0.5) * f32(params.height))), params.height - 1u);
    let packed = source_pixels[pixel_y * params.width + pixel_x];
    let source = vec4<f32>(
        f32(packed & 0xffu),
        f32((packed >> 8u) & 0xffu),
        f32((packed >> 16u) & 0xffu),
        f32((packed >> 24u) & 0xffu)
    ) / 255.0;
    let black_level = clamp(params.blend_black.y, 0.0, 1.0);
    let output_u = (f32(output_x) + 0.5) / f32(params.width);
    let output_v = (f32(output_y) + 0.5) / f32(params.height);
    let factor = edge_blend_factor(output_u, output_v) * combined_mask_factor(output_u, output_v);
    let corrected = vec4<f32>((vec3<f32>(black_level) + source.rgb * (1.0 - black_level)) * factor, source.a * factor);
    let bytes = vec4<u32>(clamp(round(corrected * 255.0), vec4<f32>(0.0), vec4<f32>(255.0)));
    output_pixels[index] = bytes.r | (bytes.g << 8u) | (bytes.b << 16u) | (bytes.a << 24u);
}
