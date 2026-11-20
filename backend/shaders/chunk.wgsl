// Greedy quads. CPU reference: voxel-core/src/render/shader_ref.rs.
// Quad word: x[0..4) y[4..8) z[8..12) layer[12..18) w-1[18..22) h-1[22..26)
// The quad spans w along its `a` axis and h along `b`; UVs repeat over 0..w, 0..h.
// The side comes from the draw slot (instance_index = chunk_slot * 6 + side), not the word.
// Vertex pulling: no vertex buffers. vertex_index = base_vertex (4 * first quad) + index value
// (4k + corner), so quad = vertex_index >> 2, corner = vertex_index & 3.

struct Globals {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    // chunk the camera origin is in; chunk offsets are computed relative to it in integers
    player_chunk: vec3<i32>,
}

struct ChunkEntry {
    // chunk position in chunk units, indexed by chunk slot, written once at upload
    pos: vec3<i32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var blocks: texture_2d_array<f32>;
@group(0) @binding(2) var blocks_sampler: sampler;
@group(1) @binding(0) var<storage, read> quads: array<i32>;
@group(1) @binding(1) var<storage, read> chunks: array<ChunkEntry>;

// Corners of the base quad in strip order (make_base_quad_data in voxel-core).
const BASE_QUAD = array<vec3<f32>, 4>(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0));

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
fn vs_main(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) slot: u32) -> VertexOut {
    let data = quads[vertex_index >> 2u];
    let base_pos = BASE_QUAD[vertex_index & 3u];
    let chunk_pos = chunks[slot / 6u].pos;
    let x = data & 15;
    let y = (data >> 4u) & 15;
    let z = (data >> 8u) & 15;
    let face = i32(slot % 6u);
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

    let chunk_offset = vec3<f32>((chunk_pos - globals.player_chunk) * 16);

    var out: VertexOut;
    out.clip_position = globals.projection * globals.view * vec4<f32>(mesh_pos * size + position + chunk_offset, 1.0);
    out.texcoord = uvs * uv_scale;
    out.layer = layer;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(blocks, blocks_sampler, in.texcoord, in.layer);
}
