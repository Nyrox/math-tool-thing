#version 330

in vec2 frag_pos;
in vec2 frag_uv;

out vec4 frag_color;

uniform sampler2D glyph_atlas;

void main() {
    float alpha = texture(glyph_atlas, vec2(frag_uv.x, 1.0 - frag_uv.y)).r;
    if (alpha < 0.01) { discard; }
    frag_color = vec4(1.0, 0.0, 0.0, alpha);
}