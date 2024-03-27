#version 300 es

layout (location = 0) in vec3 mesh_pos;
layout (location = 1) in int data;
layout (location = 2) in vec3 world_pos;

uniform mat4 view;
uniform mat4 projection;

out vec2 v_texcoord;
flat out int v_depth;

out vec3 o_normal;

vec3 NORMALS[] = vec3[]( //
vec3(0.0f, 1.0f, 0.0f),  // +Y
vec3(0.0f, -1.0f, 0.0f),   // -Y
vec3(0.0f, 0.0f, 1.0f),  // +Z
vec3(0.0f, 0.0f, -1.0f),  // -Z
vec3(1.0f, 0.0f, 0.0f), // +X
vec3(-1.0f, 0.0f, 0.0f)  // -X
);

void main() {
    int x = data & 63;
    int y = (data >> 6) & 63;
    int z = (data >> 12) & 63;
    int face = (data >> 18) & 7;
    int depth = (data >> 21) & 63;
    int uv = (data >> 27) & 3;

    vec3 position = vec3(x, y, z);
    vec3 normal = NORMALS[face];

    // vec2 uvs = UVS[uv];
    vec2 uvs;
    if (abs(normal.x) > 0.5f) {
        // Side faces
        uvs = vec2(1 - z, 1 - y);
    } else if (abs(normal.y) > 0.5f) {
        // Top and bottom faces
        uvs = vec2(z, x);
    } else if (abs(normal.z) > 0.5f) {
        // Front and back faces
        uvs = vec2(1 - x, 1 - y);
    }

    o_normal = normal;
    gl_Position = projection * view * vec4(mesh_pos + position + world_pos, 1.0f);
    v_texcoord = uvs;
    v_depth = depth;
}
