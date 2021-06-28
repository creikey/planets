#version 330 core

out vec4 Color;
in vec2 pos;

void main()
{
    float len = length(pos);
    if(len > 1.0) {
        Color = vec4(0.0);
    } else {
        Color = vec4(0.0, 0.0, 0.0, 1.0);
    }
}
