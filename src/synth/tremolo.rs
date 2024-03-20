use std::f32::consts::PI;
use tracing::{debug, info};

const TWO_PI: f32 = 2.0 * PI;
const TREMOLO_TABLE_SIZE: usize = 1024;

#[derive(Debug)]
pub struct TremoloEffect {
    tremolo: Tremolo,
    pub enabled: bool,
}

impl TremoloEffect {
    pub fn builder() -> TremoloEffectBuilder {
        debug!("Building TremoloEffect");
        TremoloEffectBuilder::default()
    }

    pub fn process(&mut self, sample: f32, sample_rate: f32) -> f32 {
        debug!("Processing sample: {}", sample);
        if self.enabled {
            self.tremolo.process(sample, sample_rate)
        } else {
            sample
        }
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
        debug!("Tremolo effect enabled: {}", self.enabled);
    }
}

#[derive(Debug)]
pub struct Tremolo {
    rate: f32,
    depth: f32,
    phase: f32,
    tremolo_table: [f32; TREMOLO_TABLE_SIZE],
    table_index: usize,
    samples_per_tremolo_cycle: usize,
    sample_counter: usize,
}

impl Tremolo {
    pub fn new(rate: f32, depth: f32, sample_rate: f32) -> Self {
        debug!("Creating new Tremolo with rate: {}, depth: {}", rate, depth);
        let samples_per_tremolo_cycle = (sample_rate / rate) as usize;
        let mut tremolo_table = [0.0; TREMOLO_TABLE_SIZE];
        for i in 0..TREMOLO_TABLE_SIZE {
            let phase = i as f32 / TREMOLO_TABLE_SIZE as f32;
            tremolo_table[i] = 1.0 - depth * (phase * TWO_PI).sin();
        }
        Tremolo {
            rate,
            depth,
            phase: 0.0,
            tremolo_table,
            table_index: 0,
            samples_per_tremolo_cycle,
            sample_counter: 0,
        }
    }

    pub fn process(&mut self, sample: f32, _sample_rate: f32) -> f32 {
        debug!("Processing sample: {}", sample);
        let amplitude = self.tremolo_table[self.table_index];
        self.sample_counter += 1;
        if self.sample_counter >= self.samples_per_tremolo_cycle {
            self.sample_counter = 0;
            self.table_index = (self.table_index + 1) % TREMOLO_TABLE_SIZE;
            debug!("Updated table index: {}", self.table_index);
        }
        sample * amplitude
    }
}

pub struct TremoloEffectBuilder {
    rate: f32,
    depth: f32,
    enabled: bool,
}

impl Default for TremoloEffectBuilder {
    fn default() -> Self {
        debug!("Creating default TremoloEffectBuilder");
        TremoloEffectBuilder {
            rate: 5.0,
            depth: 0.5,
            enabled: false,
        }
    }
}

impl TremoloEffectBuilder {
    pub fn rate(mut self, rate: f32) -> Self {
        debug!("Setting rate: {}", rate);
        self.rate = rate;
        self
    }

    pub fn depth(mut self, depth: f32) -> Self {
        debug!("Setting depth: {}", depth);
        self.depth = depth;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        debug!("Setting enabled: {}", enabled);
        self.enabled = enabled;
        self
    }

    pub fn build(self, sample_rate: f32) -> TremoloEffect {
        debug!("Building TremoloEffect with sample rate: {}", sample_rate);
        TremoloEffect {
            tremolo: Tremolo::new(self.rate, self.depth, sample_rate),
            enabled: self.enabled,
        }
    }
}
