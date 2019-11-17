#version 330

in vec3 position;

flat out vec3 frag_color;

const float MAX_HEIGHT = 1.0;


uniform mat4 mvp;

void main() {        
    gl_Position = mvp * vec4(position, 1.0);

    const float val_min = -1.0;
    const float val_max = 1.0;


    float val_med = val_min + (val_max - val_min) / 2;
    float val_range = (val_max - val_min);

    if (position.z > val_med) {
        frag_color = mix(vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0), (position.z - val_med) / (val_range / 2.0));
    }
    else {
        frag_color = mix(vec3(0.0, 1.0, 0.0), vec3(0.0, 0.0, 1.0), (val_med - position.z) / (val_range / 2.0));
    }
}