#version 300 es

in vec3 position;
in vec3 color;

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

out vec3 fragColor;

void main() {
    gl_Position = projection * view * model * vec4(position, 1.0f);
    fragColor = color;
}
