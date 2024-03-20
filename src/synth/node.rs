use crate::synth::AudioBuffer;

pub trait AudioNode {
    fn process(&mut self, input: &AudioBuffer, output: &mut AudioBuffer);
}

pub struct WaveShaperNode<F: FnMut(f32) -> f32> {
    pub transfer_fn: F,
}

impl<F: FnMut(f32) -> f32> AudioNode for WaveShaperNode<F> {
    fn process(&mut self, input: &AudioBuffer, output: &mut AudioBuffer) {
        let num_channels = input.num_channels();
        assert_eq!(num_channels, output.num_channels());

        let transfer_fn = &mut self.transfer_fn;

        for i in 0..num_channels {
            let input_channel = input.channel(i);
            let output_channel = output.channel_mut(i);
            for (input_sample, output_sample) in input_channel.iter().zip(output_channel.iter_mut())
            {
                *output_sample = transfer_fn(*input_sample);
            }
        }
    }
}
