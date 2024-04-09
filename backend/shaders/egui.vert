#version 300 es

in vec2 position;
in vec4 color;
in vec2 uv;

uniform mat4 u_ortho;

out vec2 v_texcoord;
out vec4 v_color;

void main() {
    gl_Position = u_ortho * vec4(position, 0.0f, 1.0f);
    v_color = color;
    v_texcoord = uv;
}
