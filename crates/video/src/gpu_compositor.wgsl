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
    fx_a: vec4<f32>,
    fx_b: vec4<f32>,
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

fn raw_pixel(x: u32, y: u32) -> vec4<f32> {
    let pixel = source_pixels[y * params.source_width + x];
    return vec4<f32>(unpack_rgb(pixel), unpack_alpha(pixel));
}

fn average_pixel(source_x: u32, source_y: u32, radius: i32) -> vec4<f32> {
    var channels = vec4<u32>(0u);
    var count = 0u;
    for (var offset_y = -radius; offset_y <= radius; offset_y += 1) {
        let y = i32(source_y) + offset_y;
        if (y < 0 || y >= i32(params.source_height)) {
            continue;
        }
        for (var offset_x = -radius; offset_x <= radius; offset_x += 1) {
            let x = i32(source_x) + offset_x;
            if (x < 0 || x >= i32(params.source_width)) {
                continue;
            }
            channels += vec4<u32>(raw_pixel(u32(x), u32(y)));
            count += 1u;
        }
    }
    if (count == 0u) {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(channels / vec4<u32>(count));
}

fn bright_neighbor_average(source_x: u32, source_y: u32, radius: i32) -> vec3<f32> {
    var channels = vec3<f32>(0.0);
    var weight_sum = 0.0;
    for (var offset_y = -radius; offset_y <= radius; offset_y += 1) {
        let y = i32(source_y) + offset_y;
        if (y < 0 || y >= i32(params.source_height)) {
            continue;
        }
        for (var offset_x = -radius; offset_x <= radius; offset_x += 1) {
            let x = i32(source_x) + offset_x;
            if (x < 0 || x >= i32(params.source_width)) {
                continue;
            }
            let pixel = raw_pixel(u32(x), u32(y)).rgb;
            let brightness = max(max(pixel.r, pixel.g), pixel.b) / 255.0;
            let weight = clamp((brightness - 0.6) / 0.4, 0.0, 1.0);
            if (weight > 0.0) {
                channels += pixel * weight;
                weight_sum += weight;
            }
        }
    }
    if (weight_sum <= 0.00000011920928955078125) {
        return vec3<f32>(0.0);
    }
    return channels / weight_sum;
}

fn luminance(pixel: vec3<f32>) -> f32 {
    return (0.299 * pixel.r + 0.587 * pixel.g + 0.114 * pixel.b) / 255.0;
}

fn edge_intensity(source_x: u32, source_y: u32) -> f32 {
    let center = luminance(raw_pixel(source_x, source_y).rgb);
    let left = luminance(raw_pixel(select(source_x - 1u, 0u, source_x == 0u), source_y).rgb);
    let right = luminance(raw_pixel(min(source_x + 1u, params.source_width - 1u), source_y).rgb);
    let top = luminance(raw_pixel(source_x, select(source_y - 1u, 0u, source_y == 0u)).rgb);
    let bottom = luminance(raw_pixel(source_x, min(source_y + 1u, params.source_height - 1u)).rgb);
    return clamp(max(max(abs(center - left), abs(center - right)), max(abs(center - top), abs(center - bottom))), 0.0, 1.0);
}

fn sample_with_fx(input_x: u32, input_y: u32) -> vec4<f32> {
    var source_x = input_x;
    var source_y = input_y;
    let pixelate = u32(clamp(floor(params.fx_a.x + 0.5), 1.0, 128.0));
    if (pixelate > 1u) {
        source_x = min((source_x / pixelate) * pixelate + pixelate / 2u, params.source_width - 1u);
        source_y = min((source_y / pixelate) * pixelate + pixelate / 2u, params.source_height - 1u);
    }

    let blur_radius = i32(clamp(floor(params.fx_a.y + 0.5), 0.0, 8.0));
    var sample = select(raw_pixel(source_x, source_y), average_pixel(source_x, source_y, blur_radius), blur_radius > 0);

    if (params.fx_a.z > 0.0) {
        let glow_radius = i32(clamp(ceil(params.fx_a.z * 2.0), 1.0, 8.0));
        let glow = bright_neighbor_average(source_x, source_y, glow_radius);
        sample = vec4<f32>(clamp(floor(sample.rgb + glow * params.fx_a.z + vec3<f32>(0.5)), vec3<f32>(0.0), vec3<f32>(255.0)), sample.a);
    }
    if (params.fx_a.w > 0.0) {
        let edge = edge_intensity(source_x, source_y) * 255.0;
        let amount = clamp(params.fx_a.w, 0.0, 4.0);
        sample = vec4<f32>(clamp(floor(sample.rgb + (vec3<f32>(edge) - sample.rgb) * amount + vec3<f32>(0.5)), vec3<f32>(0.0), vec3<f32>(255.0)), sample.a);
    }
    return sample;
}

fn apply_color_key(input_pixel: vec4<f32>) -> vec4<f32> {
    var pixel = input_pixel;
    if (params.fx_b.w <= 0.0) {
        return pixel;
    }
    let distance = length(pixel.rgb / 255.0 - params.fx_b.rgb);
    let alpha_scale = clamp(distance / max(params.fx_b.w, 0.001), 0.0, 1.0);
    pixel.a = floor(pixel.a * alpha_scale + 0.5);
    return pixel;
}

@compute @workgroup_size(8, 8)
fn composite_layer(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let output_x = invocation_id.x;
    let output_y = invocation_id.y;
    if (output_x >= params.output_width || output_y >= params.output_height) {
        return;
    }
    let index = output_y * params.output_width + output_x;
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
    let sampled = sample_with_fx(source_x, source_y);
    let sampled_pixel = pack_rgba(sampled.rgb, sampled.a);
    let source = apply_color_key(adjust_color(sampled_pixel));
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
