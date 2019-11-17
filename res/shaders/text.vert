#version 330

in vec2 position;
in vec2 uv;

out vec2 frag_uv;
out vec2 frag_pos;

uniform mat4 model;
uniform mat4 proj;

void main() {
    vec4 pos = proj * model * vec4(position, 1.0, 1.0);
    gl_Position = vec4(pos.xy, 1.0, 1.0);

    frag_pos = position;
    frag_uv = uv;
}