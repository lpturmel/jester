#version 450

layout(set = 0, binding = 0) uniform sampler2D u_tex;

layout(location = 0) in  vec2 v_uv;
layout(location = 0) out vec4 out_color;

void main()
{
    out_color = texture(u_tex, v_uv);
}
