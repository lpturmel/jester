#version 450
layout(location = 0) in vec2 in_pos;
layout(location = 1) in vec2 in_uv;

layout(location = 2) in vec4 inst_pos_size;   // x, y, w, h  (pixels)
layout(location = 3) in vec4 inst_uv;         // u0,v0,u1,v1

layout(location = 0) out vec2 vUV;

layout(push_constant) uniform PC {
    vec2 screen;
    vec2 camCenter;
    float camZoom;
} pc;

void main() {
    vec2 pixel = (inst_pos_size.xy - pc.camCenter) * pc.camZoom
                 + in_pos * inst_pos_size.zw * pc.camZoom;

    vec2 ndc = pixel / pc.screen * 2.0 - 1.0;
    ndc.y = -ndc.y;
    gl_Position = vec4(ndc, 0.0, 1.0);
    vUV = mix(inst_uv.xy, inst_uv.zw, in_uv);
    vUV.y = 1.0 - vUV.y;
}
