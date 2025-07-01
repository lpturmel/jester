#version 450

layout(location = 0) in vec2 in_pos;
layout(location = 1) in vec2 in_uv;

layout(location = 2) in vec4 inst_pos_size;
layout(location = 3) in vec4 inst_uv;
layout(location = 4) in uint inst_tex;

layout(location = 0) out vec2 frag_uv;

void main() {
    vec2 scaled   = in_pos * inst_pos_size.zw;
    vec2 shifted  = scaled + inst_pos_size.xy;
    gl_Position   = vec4(shifted, 0.0, 1.0);

    frag_uv = in_uv;
}
