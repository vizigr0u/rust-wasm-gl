#version 300 es

in vec3 position;
in vec2 uv;
in vec3 normal;

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

out vec2 v_texcoord;

out vec3 o_normal;

void main() {
    o_normal = normal;
    gl_Position = projection * view * model * vec4(position, 1.0f);
    v_texcoord = uv;
}
