#version 330

flat in vec3 frag_color;

out vec4 fcolor;

uniform sampler2D glyph_atlas;

void main() {
    
    float color = texture(glyph_atlas, vec2(0.2, 0.3)).r;

    fcolor = vec4(frag_color, collor);
}