struct Params {
    output_width: u32,
    output_height: u32,
    source_width: u32,
    source_height: u32,
    opacity: f32,
    blend_mode: u32,
    _padding_0: u32,
    _padding_1: u32,
    transform_a: vec4<f32>,
    transform_b: vec4<f32>,
    transform_c: vec4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0)
var<storage, read> source_pixels: array<u32>;

@group(0) @binding(1)
var<storage, read_write> output_pixels: array<u32>;

@group(0) @binding(2)
var<uniform> params: Params;

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

fn adjust_tone(value: f32, brightness: f32, contrast: f32, gamma: f32) -> f32 {
    let corrected = clamp((clamp(value, 0.0, 1.0) - 0.5) * contrast + 0.5 + brightness, 0.0, 1.0);
    if (abs(gamma - 1.0) <= 0.00000011920928955078125) {
        return corrected;
    }
    return pow(corrected, 1.0 / gamma);
}

fn adjust_color(pixel: u32) -> vec4<f32> {
    var rgb = unpack_rgb(pixel) / 255.0;
    let brightness = params.transform_c.y;
    let contrast = params.transform_c.z;
    let hue_radians = params.transform_c.w;
    let saturation = params.color.x;
    let gamma = params.color.y;

    if (abs(hue_radians) > 0.00000011920928955078125 || abs(saturation - 1.0) > 0.00000011920928955078125) {
        let y = 0.299 * rgb.r + 0.587 * rgb.g + 0.114 * rgb.b;
        let i = 0.596 * rgb.r - 0.274 * rgb.g - 0.322 * rgb.b;
        let q = 0.211 * rgb.r - 0.523 * rgb.g + 0.312 * rgb.b;
        let sine = sin(hue_radians);
        let cosine = cos(hue_radians);
        let rotated_i = (i * cosine - q * sine) * saturation;
        let rotated_q = (i * sine + q * cosine) * saturation;
        rgb = vec3<f32>(
            y + 0.956 * rotated_i + 0.621 * rotated_q,
            y - 0.272 * rotated_i - 0.647 * rotated_q,
            y - 1.106 * rotated_i + 1.703 * rotated_q,
        );
    }

    rgb = vec3<f32>(
        adjust_tone(rgb.r, brightness, contrast, gamma),
        adjust_tone(rgb.g, brightness, contrast, gamma),
        adjust_tone(rgb.b, brightness, contrast, gamma),
    );
    return vec4<f32>(rgb * 255.0, unpack_alpha(pixel));
}

@compute @workgroup_size(64)
fn composite_layer(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let index = invocation_id.x;
    let pixel_count = params.output_width * params.output_height;
    if (index >= pixel_count) {
        return;
    }

    let output_x = index % params.output_width;
    let output_y = index / params.output_width;
    let centered_x = (f32(output_x) + 0.5) / f32(params.output_width) - 0.5 - params.transform_a.x;
    let centered_y = (f32(output_y) + 0.5) / f32(params.output_height) - 0.5 - params.transform_a.y;
    let sine = sin(params.transform_b.x);
    let cosine = cos(params.transform_b.x);
    let rotated_x = centered_x * cosine - centered_y * sine;
    let rotated_y = centered_x * sine + centered_y * cosine;
    let local_x = rotated_x / params.transform_a.z + 0.5;
    let local_y = rotated_y / params.transform_a.w + 0.5;
    if (local_x < 0.0 || local_x >= 1.0 || local_y < 0.0 || local_y >= 1.0) {
        return;
    }

    let source_u = params.transform_b.y + local_x * (params.transform_b.w - params.transform_b.y);
    let source_v = params.transform_b.z + local_y * (params.transform_c.x - params.transform_b.z);
    if (params.transform_b.w <= params.transform_b.y || params.transform_c.x <= params.transform_b.z) {
        return;
    }
    let source_x = min(u32(floor(source_u * f32(params.source_width))), params.source_width - 1u);
    let source_y = min(u32(floor(source_v * f32(params.source_height))), params.source_height - 1u);
    let source = adjust_color(source_pixels[source_y * params.source_width + source_x]);
    let source_alpha = (source.a / 255.0) * clamp(params.opacity, 0.0, 1.0);
    if (source_alpha <= 0.0) {
        return;
    }

    let destination = output_pixels[index];
    let source_rgb = source.rgb;
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
