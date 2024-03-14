#version 300 es

precision highp float;
in vec2 v_texcoord;
in vec4 v_color;

uniform sampler2D u_texture;

out vec4 outColor;

vec3 srgb_gamma_from_linear(vec3 rgb) {
    bvec3 cutoff = lessThan(rgb, vec3(0.0031308f));
    vec3 lower = rgb * vec3(12.92f);
    vec3 higher = vec3(1.055f) * pow(rgb, vec3(1.0f / 2.4f)) - vec3(0.055f);
    return mix(higher, lower, vec3(cutoff));
}

vec4 srgba_gamma_from_linear(vec4 rgba) {
    return vec4(srgb_gamma_from_linear(rgba.rgb), rgba.a);
}

void main() {
    // outColor = v_color;
    // outColor = vec4(v_texcoord.x, v_texcoord.y, 1.0f, 1.0f);
    vec4 textureColor = texture(u_texture, v_texcoord);
    vec4 texture_in_gamma = srgba_gamma_from_linear(textureColor);
    outColor = texture_in_gamma * v_color;
}
