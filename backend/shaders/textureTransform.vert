#version 300 es

in vec3 position;
in vec2 uv;

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

out vec2 v_texcoord;

void main() {
    gl_Position = projection * view * model * vec4(position, 1.0f);
    v_texcoord = uv;
}
