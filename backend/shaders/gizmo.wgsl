// 1:1 port of gizmo.vert / gizmo.frag. Shares group 0 with chunk.wgsl.

struct Globals {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
}

struct ModelUniform {
    model: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> object: ModelUniform;

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) color: vec3<f32>) -> VertexOut {
    var out: VertexOut;
    out.clip_position = globals.projection * globals.view * object.model * vec4<f32>(position, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 0.0);
}
