struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

struct Dimensions {
    width: u32,
    height: u32,
    _padding_0: u32,
    _padding_1: u32,
};

@group(0) @binding(0)
var<storage, read> frame_pixels: array<u32>;

@group(0) @binding(1)
var<uniform> dimensions: Dimensions;

@vertex
fn fullscreen_vertex(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    return output;
}

@fragment
fn frame_fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let x = min(u32(position.x), dimensions.width - 1u);
    let y = min(u32(position.y), dimensions.height - 1u);
    let pixel = frame_pixels[y * dimensions.width + x];
    return vec4<f32>(
        f32(pixel & 255u),
        f32((pixel >> 8u) & 255u),
        f32((pixel >> 16u) & 255u),
        f32((pixel >> 24u) & 255u),
    ) / 255.0;
}
