// 1:1 port of chunk.vert / chunk.frag.
// Quad word: x[0..4) y[4..8) z[8..12) face[12..15) layer[15..21)

struct Globals {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
}

struct ChunkUniform {
    // chunk origin relative to the player chunk (dynamic offset per chunk)
    offset: vec3<f32>,
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
    let face = (data >> 12u) & 7;
    let layer = (data >> 15u) & 63;

    let position = vec3<f32>(f32(x), f32(y), f32(z));

    var uvs = vec2<f32>(base_pos.z, base_pos.x);
    var mesh_pos = base_pos;
    switch face {
        case SideUp: {
            mesh_pos = vec3<f32>(base_pos.x, 1.0, 1.0 - base_pos.z);
            uvs = vec2<f32>(uvs.y, 1.0 - uvs.x);
        }
        case SideDown: {
            uvs = vec2<f32>(uvs.y, 1.0 - uvs.x);
        }
        case SideFront: {
            mesh_pos = vec3<f32>(base_pos.x, base_pos.z, 1.0);
            uvs = vec2<f32>(uvs.y, 1.0 - uvs.x);
        }
        case SideBack: {
            mesh_pos = vec3<f32>(base_pos.x, 1.0 - base_pos.z, 0.0);
            uvs = vec2<f32>(1.0 - uvs.y, uvs.x);
        }
        case SideRight: {
            mesh_pos = vec3<f32>(1.0, base_pos.x, base_pos.z);
            uvs = vec2<f32>(1.0 - uvs.x, 1.0 - uvs.y);
        }
        case SideLeft: {
            mesh_pos = vec3<f32>(0.0, base_pos.x, 1.0 - base_pos.z);
            uvs = vec2<f32>(1.0 - uvs.x, 1.0 - uvs.y);
        }
        default: {}
    }

    var out: VertexOut;
    out.clip_position = globals.projection * globals.view * vec4<f32>(mesh_pos + position + chunk.offset, 1.0);
    out.texcoord = uvs;
    out.layer = layer;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(blocks, blocks_sampler, in.texcoord, in.layer);
}
