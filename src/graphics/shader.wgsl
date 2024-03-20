// Vertex shader

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

struct AudioData {
    samples: array<vec4<f32>, 256>,
};

@group(0) @binding(0)
var<uniform> audio: AudioData;

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
    model: VertexInput,
) -> VertexOutput {
    let scale: f32 = 0.8;

    let audio_sample = audio.samples[in_vertex_index / 4u][in_vertex_index % 4u];

    var position: vec2<f32>;
    var color: vec4<f32>;

    switch in_vertex_index % 3u {
        case 0u: {
            position = vec2<f32>(0.0, 1.0);
            color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
        }
        case 1u: {
            position = vec2<f32>(-1.0, -1.0);
            color = vec4<f32>(0.0, 1.0, 0.0, 1.0);
        }
        default: {
            position = vec2<f32>(1.0, -1.0);
            color = vec4<f32>(0.0, 0.0, 1.0, 1.0);
        }
    }

    position = position * (1.0 + audio_sample * 0.5);

    let clip_position = vec4<f32>(position * scale, 0.0, 1.0);

    return VertexOutput(clip_position, color);
}

// Fragment shader

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
