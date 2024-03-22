struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

struct AudioData {
    samples: array<vec4<f32>, 256>,
};

struct Uniform {
    time: f32,
};

@group(0) @binding(0)
var<uniform> audio: AudioData;

@group(1) @binding(0)
var<uniform> uni: Uniform;

fn hue2rgb(p: f32, q: f32, t: f32) -> vec3<f32> {
    var tt = t;
    if tt < 0.0 {
        tt += 1.0;
    }
    if tt > 1.0 {
        tt -= 1.0;
    }
    if tt < 1.0 / 6.0 {
        return vec3<f32>(p + (q - p) * 6.0 * tt);
    }
    if tt < 1.0 / 2.0 {
        return vec3<f32>(q);
    }
    if tt < 2.0 / 3.0 {
        return vec3<f32>(p + (q - p) * (2.0 / 3.0 - tt) * 6.0);
    }
    return vec3<f32>(p);
}

fn hsv2rgb(h: f32, s: f32, v: f32) -> vec4<f32> {
    if s == 0.0 {
        return vec4<f32>(v, v, v, 1.0);
    }
    var hh = h;
    if hh >= 360.0 {
        hh -= 360.0;
    }
    hh /= 60.0;
    let i = i32(hh);
    let ff = hh - f32(i);
    let p = v * (1.0 - s);
    let q = v * (1.0 - (s * ff));
    let t = v * (1.0 - (s * (1.0 - ff)));

    switch (i) {
        case 0: {
            return vec4<f32>(v, t, p, 1.0);
        }
        case 1: {
            return vec4<f32>(q, v, p, 1.0);
        }
        case 2: {
            return vec4<f32>(p, v, t, 1.0);
        }
        case 3: {
            return vec4<f32>(p, q, v, 1.0);
        }
        case 4: {
            return vec4<f32>(t, p, v, 1.0);
        }
        default: {
            return vec4<f32>(v, p, q, 1.0);
        }
    }
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    let x = model.position.x;
    let i = i32((x + 1.0) * 0.5 * 255.0);
    let j = i / 16;
    let k = i % 16;

    if j >= 0 && j < 16 && k >= 0 && k < 16 {
        let index = j * 16 + k;
        let sample = audio.samples[index];

        let wave_amplitude = sample[0] * 0.5;
        let wave_frequency = sample[1] * 10.0;
        let wave_phase = uni.time * sample[2] * 6.0;

        let y = sin(x * wave_frequency + wave_phase) * wave_amplitude * 5.0;

        let clip_position = vec4<f32>(x, y, 0.0, 1.0);

        let hue = degrees(atan2(y, x)) + uni.time * 100.0;
        let saturation = length(vec2<f32>(sample[0], sample[1])) * 2.0;
        let value = sample[3];

        let color = hsv2rgb(hue, saturation, value);

        return VertexOutput(clip_position, color);
    } else {
        let clip_position = vec4<f32>(x, 0.0, 0.0, 1.0);
        let color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        return VertexOutput(clip_position, color);
    }
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
