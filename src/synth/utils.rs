// visiosynth/src/main.rs

use crate::synth::{DownsampledAudioData, NoteState, OscillatorWaveform, Scale, TremoloEffect};
use anyhow::Result;
use cpal::traits::{DeviceTrait, StreamTrait};
use rodio;
use rodio::Source;
use std::io::BufReader;
use std::sync::{Arc, Mutex};

pub fn pan(sample: f32, panning: f32) -> (f32, f32) {
    let left = sample * (1.0 - panning.abs());
    let right = sample * (1.0 - left);
    (left, right)
}

#[allow(dead_code)]
fn run_audio_clip<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    _waveform_type: Arc<Mutex<OscillatorWaveform>>,
    _note_state: Arc<Mutex<NoteState>>,
    _octave_shift: Arc<Mutex<i32>>,
    _global_time: Arc<Mutex<f32>>,
    _tremolo_effect: Arc<Mutex<TremoloEffect>>,
    _scale: Arc<Mutex<Scale>>,
    downsampled_audio_data: Arc<Mutex<DownsampledAudioData>>,
) -> Result<(), anyhow::Error>
where
    T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
{
    let sample_rate: f32 = config.sample_rate.0 as f32;
    let downsample_factor = (sample_rate / 60.0) as usize;
    let mut accumulated_samples = Vec::new();
    let channels = config.channels as usize;

    // Load the MP3 file
    let audio_file = std::fs::File::open(
        "/home/rsp/music/Doom Scroll/Doom Scroll - Immoral Compass - 06 Immoral Compass.mp3",
    )?;
    let source = rodio::Decoder::new(BufReader::new(audio_file))?;
    let mut source_peekable = source.convert_samples().peekable();

    let err_fn = |err| eprintln!("An error occurred on the audio stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut output_samples = vec![0.0; data.len() / channels];

            // Read audio samples from the MP3 file
            for sample in output_samples.iter_mut() {
                if let Some(&s) = source_peekable.peek() {
                    *sample = s;
                    source_peekable.next();
                } else {
                    break;
                }
            }

            // Duplicate mono samples across all channels
            for (i, sample) in output_samples.iter().enumerate() {
                for j in 0..channels {
                    data[i * channels + j] = T::from_sample(*sample);
                }
            }

            accumulated_samples.extend(output_samples);

            if accumulated_samples.len() >= downsample_factor {
                let mut downsampled_samples = Vec::new();

                for chunk in accumulated_samples.chunks(downsample_factor) {
                    let sum: f32 = chunk.iter().sum();
                    let average = sum / chunk.len() as f32;
                    downsampled_samples.push(average);
                }

                if let Ok(mut downsampled_audio_data) = downsampled_audio_data.lock() {
                    let num_frames = downsampled_samples.len().min(256);
                    downsampled_audio_data.samples = [[0.0; 16]; 256];
                    for (i, chunk) in downsampled_samples.chunks(16).enumerate().take(num_frames) {
                        for (j, &sample) in chunk.iter().enumerate() {
                            downsampled_audio_data.samples[i][j] = sample;
                        }
                    }
                }

                accumulated_samples.clear();
            }
        },
        err_fn,
        None,
    )?;

    stream.play()?;
    std::thread::sleep(std::time::Duration::from_secs(100));

    Ok(())
}
