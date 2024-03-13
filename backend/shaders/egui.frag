#version 300 es

precision highp float;
in vec2 v_texcoord;
in vec4 v_color;

uniform sampler2D u_texture;

out vec4 outColor;

void main() {
    // outColor = v_color;
    // outColor = vec4(v_texcoord.x, v_texcoord.y, 1.0f, 1.0f);
    vec4 textureColor = texture(u_texture, v_texcoord);
    outColor = textureColor * v_color;
}
