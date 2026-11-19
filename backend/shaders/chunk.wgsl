// Greedy quads. CPU reference: voxel-core/src/render/shader_ref.rs.
// Quad word: x[0..4) y[4..8) z[8..12) layer[12..18) w-1[18..22) h-1[22..26)
// The quad spans w along its `a` axis and h along `b`; UVs repeat over 0..w, 0..h.
// The side comes from the draw's uniform (not the word).

struct Globals {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
}

struct ChunkUniform {
    // chunk origin relative to the player chunk (dynamic offset per chunk side range)
    offset: vec3<f32>,
    side: i32,
}

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var blocks: texture_2d_array<f32>;
@group(0) @binding(2) var blocks_sampler: sampler;
@group(1) @binding(0) var<uniform> chunk: ChunkUniform;

const SideUp: i32 = 0;
const SideDown: i32 = 1;
const SideFront: i32 = 2;
const SideBack: i32 = 3;
const SideRight: i32 = 4;
const SideLeft: i32 = 5;

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) texcoord: vec2<f32>,
    @location(1) @interpolate(flat) layer: i32,
}

@vertex
fn vs_main(@location(0) base_pos: vec3<f32>, @location(1) data: i32) -> VertexOut {
    let x = data & 15;
    let y = (data >> 4u) & 15;
    let z = (data >> 8u) & 15;
    let face = chunk.side;
    let layer = (data >> 12u) & 63;
    let a = f32(((data >> 18u) & 15) + 1);
    let b = f32(((data >> 22u) & 15) + 1);

    let position = vec3<f32>(f32(x), f32(y), f32(z));

    var uvs = vec2<f32>(base_pos.z, base_pos.x);
    var mesh_pos = base_pos;
    var size = vec3<f32>(1.0);
    var uv_scale = vec2<f32>(a, b);
    switch face {
        case SideUp: {
            mesh_pos = vec3<f32>(base_pos.x, 1.0, 1.0 - base_pos.z);
            uvs = vec2<f32>(uvs.y, 1.0 - uvs.x);
            size = vec3<f32>(a, 1.0, b);
        }
        case SideDown: {
            uvs = vec2<f32>(uvs.y, 1.0 - uvs.x);
            size = vec3<f32>(a, 1.0, b);
        }
        case SideFront: {
            mesh_pos = vec3<f32>(base_pos.x, base_pos.z, 1.0);
            uvs = vec2<f32>(uvs.y, 1.0 - uvs.x);
            size = vec3<f32>(a, b, 1.0);
        }
        case SideBack: {
            mesh_pos = vec3<f32>(base_pos.x, 1.0 - base_pos.z, 0.0);
            uvs = vec2<f32>(1.0 - uvs.y, uvs.x);
            size = vec3<f32>(a, b, 1.0);
        }
        case SideRight: {
            mesh_pos = vec3<f32>(1.0, base_pos.x, base_pos.z);
            uvs = vec2<f32>(1.0 - uvs.x, 1.0 - uvs.y);
            size = vec3<f32>(1.0, a, b);
            uv_scale = vec2<f32>(b, a);
        }
        case SideLeft: {
            mesh_pos = vec3<f32>(0.0, base_pos.x, 1.0 - base_pos.z);
            uvs = vec2<f32>(1.0 - uvs.x, 1.0 - uvs.y);
            size = vec3<f32>(1.0, a, b);
            uv_scale = vec2<f32>(b, a);
        }
        default: {}
    }

    var out: VertexOut;
    out.clip_position = globals.projection * globals.view * vec4<f32>(mesh_pos * size + position + chunk.offset, 1.0);
    out.texcoord = uvs * uv_scale;
    out.layer = layer;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(blocks, blocks_sampler, in.texcoord, in.layer);
}
