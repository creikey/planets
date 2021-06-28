#version 330 core

layout (location = 0) in vec2 Position;

out vec2 pos;

uniform mat4 camera;
uniform mat4 projection;
uniform vec2 offset;
uniform float radius;

void main()
{
    pos = Position;
    gl_Position = projection * camera * vec4(Position * radius + offset, 0.0, 1.0);
}
