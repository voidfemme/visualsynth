#[derive(Debug)]
pub struct AmplitudeEnvelope {
    pub attack_time: f32,
    pub decay_time: f32,
    pub sustain_level: f32,
    pub release_time: f32,
}

impl AmplitudeEnvelope {
    pub fn amplitude_at_time(&self, time: f32) -> f32 {
        if time < self.attack_time {
            // Attack stage
            time / self.attack_time
        } else if time < self.attack_time + self.decay_time {
            // Decay stage
            1.0 - (time - self.attack_time) / self.decay_time * (1.0 - self.sustain_level)
        } else if time < self.attack_time + self.decay_time + self.release_time {
            // Release stage
            self.sustain_level
                * (1.0 - (time - self.attack_time - self.decay_time) / self.release_time)
        } else {
            // Envelope finished
            0.0
        }
    }
}
