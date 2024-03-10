#version 300 es

precision highp float;
in vec2 v_texcoord;

uniform sampler2D u_texture;

out vec4 outColor;

void main() {
    outColor = vec4(0.8f, 0.7f, 1.0f, 1.0f);
    outColor = vec4(v_texcoord.x, v_texcoord.y, 1.0f, 1.0f);
    outColor = texture(u_texture, v_texcoord);

}
