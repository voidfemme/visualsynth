use crate::synth::OscillatorWaveform;
use lazy_static::lazy_static;
use std::f32::consts::PI;

pub const TWO_PI: f32 = 2.0 * PI;
pub const WAVETABLE_SIZE: usize = 1024;

lazy_static! {
    static ref WAVETABLES: [[f32; WAVETABLE_SIZE]; 5] = [
        [0.0; WAVETABLE_SIZE],
        {
            let mut wavetable = [0.0; WAVETABLE_SIZE];
            for i in 0..WAVETABLE_SIZE {
                wavetable[i] = ((i as f32 * TWO_PI) / WAVETABLE_SIZE as f32).sin();
            }
            wavetable
        },
        {
            let mut wavetable = [0.0; WAVETABLE_SIZE];
            for i in 0..WAVETABLE_SIZE {
                wavetable[i] = if i < WAVETABLE_SIZE / 2 { 1.0 } else { -1.0 };
            }
            wavetable
        },
        {
            let mut wavetable = [0.0; WAVETABLE_SIZE];
            for i in 0..WAVETABLE_SIZE {
                wavetable[i] = 2.0 * (i as f32 / WAVETABLE_SIZE as f32) - 1.0;
            }
            wavetable
        },
        {
            let mut wavetable = [0.0; WAVETABLE_SIZE];
            for i in 0..WAVETABLE_SIZE {
                let phase = i as f32 / WAVETABLE_SIZE as f32;
                wavetable[i] = if phase < 0.5 {
                    4.0 * phase - 1.0
                } else {
                    3.0 - 4.0 * phase
                };
            }
            wavetable
        },
    ];
}

#[derive(Debug)]
pub struct WaveformGenerator {
    wavetable: &'static [f32; WAVETABLE_SIZE],
    phase: f32,
    phase_inc: f32,
    pub sample_rate: f32,
}

impl WaveformGenerator {
    pub fn new(waveform: OscillatorWaveform, frequency: f32, sample_rate: f32) -> Self {
        let wavetable = match waveform {
            OscillatorWaveform::Silence => &WAVETABLES[0],
            OscillatorWaveform::Sine => &WAVETABLES[1],
            OscillatorWaveform::Square => &WAVETABLES[2],
            OscillatorWaveform::Sawtooth => &WAVETABLES[3],
            OscillatorWaveform::Triangle => &WAVETABLES[4],
        };
        let phase_inc = frequency / sample_rate;
        WaveformGenerator {
            wavetable,
            phase: 0.0,
            phase_inc,
            sample_rate,
        }
    }

    pub fn get_waveform(&self) -> OscillatorWaveform {
        match self.wavetable {
            wavetable if *wavetable == WAVETABLES[0] => OscillatorWaveform::Silence,
            wavetable if *wavetable == WAVETABLES[1] => OscillatorWaveform::Sine,
            wavetable if *wavetable == WAVETABLES[2] => OscillatorWaveform::Square,
            wavetable if *wavetable == WAVETABLES[3] => OscillatorWaveform::Sawtooth,
            wavetable if *wavetable == WAVETABLES[4] => OscillatorWaveform::Triangle,
            _ => unreachable!(),
        }
    }

    pub fn get_sample(&mut self) -> f32 {
        let index = (self.phase * WAVETABLE_SIZE as f32) as usize;
        let frac = self.phase * WAVETABLE_SIZE as f32 - index as f32;
        let sample = self.wavetable[index];
        let next_sample = self.wavetable[(index + 1) % WAVETABLE_SIZE];
        let interpolated_sample = sample + frac * (next_sample - sample);
        self.update_phase();
        interpolated_sample
    }
    pub fn update_phase(&mut self) {
        self.phase = (self.phase + self.phase_inc) % 1.0;
    }

    pub fn set_frequency(&mut self, frequency: f32) {
        self.phase_inc = frequency / self.sample_rate;
    }

    pub fn get_frequency(&self) -> f32 {
        self.phase_inc * self.sample_rate
    }
}
