// 1:1 port of cube.vert / cube.frag. Shares group 0 with chunk.wgsl.

struct Globals {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
}

struct ModelUniform {
    model: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var blocks: texture_2d_array<f32>;
@group(0) @binding(2) var blocks_sampler: sampler;
@group(1) @binding(0) var<uniform> object: ModelUniform;

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) texcoord: vec2<f32>,
    @location(1) @interpolate(flat) layer: i32,
}

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) depth: f32,
) -> VertexOut {
    var out: VertexOut;
    out.clip_position = globals.projection * globals.view * object.model * vec4<f32>(position, 1.0);
    out.texcoord = uv;
    out.layer = i32(depth);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(blocks, blocks_sampler, in.texcoord, in.layer);
}
