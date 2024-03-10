#version 300 es

in vec3 position;
in vec2 uv;

uniform mat4 transform;

out vec2 v_texcoord;

void main() {
    gl_Position = transform * vec4(position, 1.0f);
    v_texcoord = uv;
}
