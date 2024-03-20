use std::sync::{Arc, Mutex};

use serde_derive::{Deserialize, Serialize};
use tracing::debug;

use crate::synth::adsr_envelope::AmplitudeEnvelope;
use crate::synth::tremolo::TremoloEffect;
use crate::synth::waveform_generator::WaveformGenerator;

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum OscillatorWaveform {
    Silence,
    Sine,
    Square,
    Sawtooth,
    Triangle,
}

#[derive(Debug)]
pub struct Oscillator {
    waveform_generator: WaveformGenerator,
    envelope: AmplitudeEnvelope,
    tremolo_effect: Arc<Mutex<TremoloEffect>>,
    pub note: String,
    start_time: Option<f32>,
}

impl Oscillator {
    pub fn new(
        frequency: f32,
        sample_rate: f32,
        waveform: OscillatorWaveform,
        note: String,
        attack_time: f32,
        decay_time: f32,
        sustain_level: f32,
        release_time: f32,
        tremolo_effect: Arc<Mutex<TremoloEffect>>,
    ) -> Self {
        Oscillator {
            waveform_generator: WaveformGenerator::new(waveform, frequency, sample_rate),
            envelope: AmplitudeEnvelope {
                attack_time,
                decay_time,
                sustain_level,
                release_time,
            },
            tremolo_effect,
            note,
            start_time: None,
        }
    }

    pub fn builder() -> OscillatorBuilder {
        OscillatorBuilder::default()
    }

    pub fn generate_wave(&mut self, current_time: f32) -> f32 {
        let sample = self.waveform_generator.get_sample();

        let envelope_value = if let Some(start_time) = self.start_time {
            self.envelope.amplitude_at_time(current_time - start_time)
        } else {
            1.0
        };

        let mut output = sample * envelope_value;

        let mut tremolo_effect = self.tremolo_effect.lock().unwrap();
        if tremolo_effect.enabled {
            output = tremolo_effect.process(output, self.waveform_generator.sample_rate);
        }

        output
    }

    pub fn start_note(&mut self, start_time: f32) {
        self.start_time = Some(start_time);
    }

    pub fn release_note(&mut self, current_time: f32) {
        if let Some(start_time) = self.start_time {
            let envelope_value = self.envelope.amplitude_at_time(current_time - start_time);
            if envelope_value <= 0.0 {
                self.start_time = None;
            }
        }
    }

    pub fn set_waveform(&mut self, waveform: OscillatorWaveform) {
        debug!("Setting waveform to {:?}", waveform);
        self.waveform_generator = WaveformGenerator::new(
            waveform,
            self.waveform_generator.get_frequency(),
            self.waveform_generator.sample_rate,
        );
        debug!(
            "Waveform set to {:?}",
            self.waveform_generator.get_waveform()
        )
    }

    pub fn set_frequency(&mut self, frequency: f32) {
        self.waveform_generator.set_frequency(frequency);
    }

    pub fn get_frequency(&self) -> f32 {
        self.waveform_generator.get_frequency()
    }

    pub fn get_waveform(&self) -> OscillatorWaveform {
        self.waveform_generator.get_waveform()
    }
}

pub struct OscillatorBuilder {
    frequency: f32,
    sample_rate: f32,
    waveform: OscillatorWaveform,
    note: String,
    attack_time: f32,
    decay_time: f32,
    sustain_level: f32,
    release_time: f32,
    tremolo_effect: Option<Arc<Mutex<TremoloEffect>>>,
}

impl Default for OscillatorBuilder {
    fn default() -> Self {
        OscillatorBuilder {
            frequency: 440.0,
            sample_rate: 44100.0,
            waveform: OscillatorWaveform::Sine,
            note: "A4".to_string(),
            attack_time: 0.1,
            decay_time: 0.1,
            sustain_level: 0.7,
            release_time: 0.2,
            tremolo_effect: None,
        }
    }
}

impl OscillatorBuilder {
    pub fn build(self) -> Oscillator {
        let tremolo_effect = self.tremolo_effect.unwrap_or_else(|| {
            Arc::new(Mutex::new(
                TremoloEffect::builder()
                    .rate(5.0)
                    .depth(0.5)
                    .enabled(false)
                    .build(self.sample_rate),
            ))
        });

        Oscillator::new(
            self.frequency,
            self.sample_rate,
            self.waveform,
            self.note,
            self.attack_time,
            self.decay_time,
            self.sustain_level,
            self.release_time,
            tremolo_effect,
        )
    }

    pub fn tremolo_effect(mut self, effect: Arc<Mutex<TremoloEffect>>) -> Self {
        self.tremolo_effect = Some(effect);
        self
    }

    pub fn frequency(mut self, frequency: f32) -> Self {
        self.frequency = frequency;
        self
    }

    pub fn waveform(mut self, waveform: OscillatorWaveform) -> Self {
        self.waveform = waveform;
        self
    }

    pub fn attack_time(mut self, attack_time: f32) -> Self {
        self.attack_time = attack_time;
        self
    }

    pub fn release_time(mut self, release_time: f32) -> Self {
        self.release_time = release_time;
        self
    }
}
