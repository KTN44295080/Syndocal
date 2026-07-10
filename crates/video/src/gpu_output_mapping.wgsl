struct Params {
    width: u32,
    height: u32,
    _padding_0: u32,
    _padding_1: u32,
    offset_scale: vec4<f32>,
    rotation_lens_keystone: vec4<f32>,
    top_corners: vec4<f32>,
    bottom_corners: vec4<f32>,
};

@group(0) @binding(0)
var<storage, read> source_pixels: array<u32>;

@group(0) @binding(1)
var<storage, read_write> output_pixels: array<u32>;

@group(0) @binding(2)
var<uniform> params: Params;

fn corner_offset(centered_x: f32, centered_y: f32) -> vec2<f32> {
    let u = clamp(centered_x + 0.5, 0.0, 1.0);
    let v = clamp(centered_y + 0.5, 0.0, 1.0);
    let top = mix(params.top_corners.xy, params.top_corners.zw, u);
    let bottom = mix(params.bottom_corners.xy, params.bottom_corners.zw, u);
    return mix(top, bottom, v);
}

@compute @workgroup_size(64)
fn map_output(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let index = invocation_id.x;
    let pixel_count = params.width * params.height;
    if (index >= pixel_count) {
        return;
    }

    let output_x = index % params.width;
    let output_y = index / params.width;
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
    output_pixels[index] = source_pixels[pixel_y * params.width + pixel_x];
}
