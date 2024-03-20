pub struct Modulator {
    phase: f32,
    phase_inc: f32,
    mod_osc: f32,
}

impl Modulator {
    pub fn new(frequency: f32, sample_rate: f32) -> Self {
        let phase_inc = frequency / sample_rate;
        Modulator {
            phase: 0.0,
            phase_inc,
            mod_osc: 0.0,
        }
    }

    pub fn next(&mut self, mod_oscillator: f32) -> f32 {
        self.mod_osc = mod_oscillator;
        let mod_value = self.mod_osc;
        let frequency = self.phase_inc * (1.0 + mod_value);
        let value = self.phase.sin();
        self.phase = (self.phase + frequency) % 1.0;
        value
    }

    pub fn sine_wave(&self, _frequency: f32, _sample_rate: u32, _phase: f32) -> f32 {
        if self.phase.sin() >= 0.0 {
            1.0
        } else {
            -1.0
        }
    }
}
