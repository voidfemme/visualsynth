use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tracing::debug;

const TWO_PI: f32 = 2.0 * PI;
const TREMOLO_TABLE_SIZE: usize = 1024;
const SCALE_FACTOR: u32 = 1000;

#[derive(Debug)]
pub struct TremoloEffect {
    tremolo: Arc<Mutex<Tremolo>>,
    pub enabled: AtomicBool,
    rate: AtomicU32,
    depth: AtomicU32,
}

impl TremoloEffect {
    pub fn builder() -> TremoloEffectBuilder {
        debug!("Building TremoloEffect");
        TremoloEffectBuilder::default()
    }

    pub fn process(&mut self, sample: f32, sample_rate: f32) -> f32 {
        debug!("Processing sample: {}", sample);
        if self.enabled.load(Ordering::Relaxed) {
            let mut tremolo = self.tremolo.lock().unwrap();
            tremolo.process(sample, sample_rate)
        } else {
            sample
        }
    }

    pub fn toggle(&self) {
        let enabled = self.enabled.fetch_xor(true, Ordering::Relaxed);
        if enabled {
            let mut tremolo = self.tremolo.lock().unwrap();
            tremolo.reset();
        }
    }

    pub fn set_rate(&self, rate: f32) {
        self.rate
            .store((rate * SCALE_FACTOR as f32) as u32, Ordering::Relaxed);
    }

    pub fn set_depth(&self, depth: f32) {
        self.rate
            .store((depth * SCALE_FACTOR as f32) as u32, Ordering::Relaxed);
    }

    pub fn get_rate(&self) -> f32 {
        self.rate.load(Ordering::Relaxed) as f32 / SCALE_FACTOR as f32
    }

    pub fn get_depth(&self) -> f32 {
        self.depth.load(Ordering::Relaxed) as f32 / SCALE_FACTOR as f32
    }
}

#[derive(Debug)]
pub struct Tremolo {
    rate: f32,
    depth: f32,
    tremolo_table: [f32; TREMOLO_TABLE_SIZE],
    table_index: AtomicUsize,
    samples_per_tremolo_cycle: usize,
    sample_counter: AtomicUsize,
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
            tremolo_table,
            table_index: AtomicUsize::new(0),
            samples_per_tremolo_cycle,
            sample_counter: AtomicUsize::new(0),
        }
    }

    pub fn process(&self, sample: f32, _sample_rate: f32) -> f32 {
        debug!("Processing sample: {}", sample);
        let table_index = self.table_index.load(Ordering::Relaxed);
        let amplitude = self.tremolo_table[table_index];

        let sample_counter = self.sample_counter.fetch_add(1, Ordering::Relaxed);
        if sample_counter + 1 >= self.samples_per_tremolo_cycle {
            self.sample_counter.store(0, Ordering::Relaxed);
            self.table_index
                .store((table_index + 1) % TREMOLO_TABLE_SIZE, Ordering::Relaxed);
        }
        sample * amplitude
    }

    pub fn reset(&self) {
        self.table_index.store(0, Ordering::Relaxed);
        self.sample_counter.store(0, Ordering::Relaxed);
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
            tremolo: Arc::new(Mutex::new(Tremolo::new(self.rate, self.depth, sample_rate))),
            enabled: AtomicBool::new(self.enabled),
            rate: AtomicU32::new((self.rate * SCALE_FACTOR as f32) as u32),
            depth: AtomicU32::new((self.depth * SCALE_FACTOR as f32) as u32),
        }
    }
}
