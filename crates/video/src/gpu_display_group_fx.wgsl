// Resident group FX bridge between P's packed straight-alpha RGBA8 group
// buffer and the ISF texture stages. Both directions are exact for 8-bit
// unorm: unpack4x8unorm/textureStore and textureLoad/pack4x8unorm round-trip
// every byte, matching the canonical upload and readback.

struct Extent {
    width: u32,
    height: u32,
    _reserved0: u32,
    _reserved1: u32,
}

@group(0) @binding(0) var<storage, read> packed: array<u32>;
@group(0) @binding(1) var unpacked: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> unpack_extent: Extent;

@compute @workgroup_size(8, 8)
fn unpack_group(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= unpack_extent.width || id.y >= unpack_extent.height) {
        return;
    }
    let word = packed[id.y * unpack_extent.width + id.x];
    textureStore(unpacked, vec2<i32>(id.xy), unpack4x8unorm(word));
}

@group(0) @binding(3) var result: texture_2d<f32>;
@group(0) @binding(4) var<storage, read_write> repacked: array<u32>;
@group(0) @binding(5) var<uniform> pack_extent: Extent;

@compute @workgroup_size(8, 8)
fn pack_group(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= pack_extent.width || id.y >= pack_extent.height) {
        return;
    }
    let texel = textureLoad(result, vec2<i32>(id.xy), 0);
    repacked[id.y * pack_extent.width + id.x] = pack4x8unorm(texel);
}
