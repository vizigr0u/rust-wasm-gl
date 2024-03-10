#version 300 es

in vec3 position;
in vec3 vertexColor;

uniform mat4 transform;

out vec3 color;

void main() {
    gl_Position = transform * vec4(position, 1.0f);
    color = vertexColor;
}
