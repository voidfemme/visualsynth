#[derive(Clone)]
pub struct AudioBuffer {
    pub data: Vec<f32>,
    pub num_channels: usize,
}

impl AudioBuffer {
    pub fn num_channels(&self) -> usize {
        self.num_channels
    }

    pub fn num_frames(&self) -> usize {
        self.data.len() / self.num_channels
    }

    pub fn channel(&self, channel_index: usize) -> &[f32] {
        let start_index = channel_index * self.num_frames();
        &self.data[start_index..start_index + self.num_frames()]
    }

    pub fn channel_mut(&mut self, channel_index: usize) -> &mut [f32] {
        let start_index = channel_index * self.num_frames();
        let channel_len = self.num_frames();
        &mut self.data[start_index..start_index + channel_len]
    }
}
