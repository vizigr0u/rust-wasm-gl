#version 300 es

in vec3 position;

uniform vec3 color;
uniform mat4 transform;

out vec3 fragColor;

void main() {
    gl_Position = transform * vec4(position, 1.0f);
    fragColor = color;
}
