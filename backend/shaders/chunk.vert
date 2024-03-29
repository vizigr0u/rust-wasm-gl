#version 300 es

layout (location = 0) in vec3 mesh_pos;
layout (location = 1) in int data;
layout (location = 2) in vec3 world_pos;

uniform mat4 view;
uniform mat4 projection;

out vec2 v_texcoord;
flat out int v_depth;

// out vec3 o_normal;

vec3 NORMALS[] = vec3[]( //
vec3(0.0f, 1.0f, 0.0f),  // +Y
vec3(0.0f, -1.0f, 0.0f),   // -Y
vec3(0.0f, 0.0f, 1.0f),  // +Z
vec3(0.0f, 0.0f, -1.0f),  // -Z
vec3(1.0f, 0.0f, 0.0f), // +X
vec3(-1.0f, 0.0f, 0.0f)  // -X
);

const int SideUp = 0;
const int SideDown = 1;
const int SideFront = 2;
const int SideBack = 3;
const int SideRight = 4;
const int SideLeft = 5;

void main() {
    int x = data & 63;
    int y = (data >> 6) & 63;
    int z = (data >> 12) & 63;
    int face = (data >> 18) & 7;
    int depth = (data >> 21) & 63;

    vec3 position = vec3(x, y, z);
    vec3 normal = NORMALS[face];

    vec2 uvs = vec2(mesh_pos.z, mesh_pos.x);

    vec3 mesh_pos = mesh_pos;
    if (face == SideUp) {
        mesh_pos.y = 1.0f;
        uvs = uvs.yx;
    } else if (face == SideLeft) {
        mesh_pos.xy = mesh_pos.yx;
        // mesh_pos.x -= 0.0f;
        uvs = vec2(uvs.x, 1.0f - uvs.y);
    } else if (face == SideRight) {
        mesh_pos.zxy = vec3(mesh_pos.x, 1.0f, mesh_pos.z);
        uvs = vec2(1.0f - uvs.y, 1.0f - uvs.x);
    } else if (face == SideDown) {
        uvs = vec2(uvs.y, 1.0f - uvs.x);
    } else if (face == SideFront) {
        mesh_pos.xyz = vec3(mesh_pos.x, mesh_pos.z, 1.0f);
        uvs = vec2(uvs.y, 1.0f - uvs.x);
    } else if (face == SideBack) {
        mesh_pos = vec3(mesh_pos.x, mesh_pos.z, -0.0f);
        uvs = vec2(1.0f - uvs.y, 1.0f - uvs.x);
    }

        // vec2 uvs = UVS[uv];
    // vec2 uvs;
    // if (abs(normal.x) > 0.5f) {
    //     // Side faces
    //     uvs = vec2(1.0f - mesh_pos.z, 1.0f - mesh_pos.x);
    // } else if (abs(normal.y) > 0.5f) {
    //     // Top and bottom faces
    //     uvs = vec2(mesh_pos.z, mesh_pos.x);
    // } else if (abs(normal.z) > 0.5f) {
    //     // Front and back faces
    //     uvs = vec2(1.0f - mesh_pos.x, 1.0f - mesh_pos.x);
    // }

    // o_normal = normal;
    gl_Position = projection * view * vec4(mesh_pos, 1.0f);
    gl_Position = projection * view * vec4(mesh_pos + position + world_pos, 1.0f);
    // gl_Position = vec4(mesh_pos, 1.0f);
    v_texcoord = uvs;
    v_depth = depth;
}
