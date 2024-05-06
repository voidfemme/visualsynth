// visiosynth/src/main.rs

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use futures;
use serde_yaml;
use std::fs::File;
use std::io::Read;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber;
use visiosynth::{
    graphics::{AudioData, State},
    synth::{
        AudioBuffer, AudioNode, Config, DownsampledAudioData, NoteEvent, NoteState, Oscillator,
        OscillatorWaveform, Scale, TremoloEffect, WaveShaperNode,
    },
};
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

// Import necessary modules and dependencies
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize tracing_subscriber
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // Load and parse the YAML config file
    info!("Attempting to open the configuration file: 'resources/config/settings.yaml'");
    let mut file = File::open("resources/config/settings.yaml")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let keys_config: Config = serde_yaml::from_str(&contents)?;

    // Set up audio host and device
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or(anyhow::Error::msg("No output device available"))?;
    let config = device.default_output_config()?;

    // Create shared state variables:
    let global_time = Arc::new(AtomicU64::new(0));
    let waveform_type = Arc::new(RwLock::new(OscillatorWaveform::Silence));
    let octave_shift = Arc::new(RwLock::new(0));
    let note_state = Arc::new(Mutex::new(NoteState::new()));

    let keys_config = Arc::new(keys_config);
    let tremolo_effect = Arc::new(
        TremoloEffect::builder()
            .rate(5.0)
            .depth(0.5)
            .enabled(false)
            .build(config.sample_rate().0 as f32),
    );
    let scale = Arc::new(Mutex::new(Scale {
        root_note: "C".to_string(),
        intervals: vec![2, 2, 1, 2, 2, 2, 1],
    }));

    let downsampled_audio_data = Arc::new(Mutex::new(DownsampledAudioData {
        samples: [[0.0; 16]; 256],
    }));

    // Create the window and event loop
    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("oscillator")
        .build(&event_loop)?;

    // Start the audio stream based on the sample format
    // - Initialize the oscillator and modulator
    // - Build and start the output stream
    //   - Update note_state and octave_shift
    //   - Generate audio samples based on the playing notes and oscillators
    //   - Apply wave shaping to the audio buffer
    //   - Write the audio samples to the output buffer
    let audio_thread = std::thread::spawn({
        let device = device.clone();
        let config = config.clone();
        let waveform_type = waveform_type.clone();
        let note_state = note_state.clone();
        let octave_shift = octave_shift.clone();
        let global_time = global_time.clone();
        let tremolo_effect = tremolo_effect.clone();
        let scale = scale.clone();
        let downsampled_audio_data = downsampled_audio_data.clone();

        move || match config.sample_format() {
            cpal::SampleFormat::F32 => run_audio_loop::<f32>(
                &device,
                &config.into(),
                waveform_type,
                note_state,
                octave_shift,
                global_time,
                tremolo_effect,
                scale,
                downsampled_audio_data,
            ),
            cpal::SampleFormat::I16 => run_audio_loop::<i16>(
                &device,
                &config.into(),
                waveform_type,
                note_state,
                octave_shift,
                global_time,
                tremolo_effect,
                scale,
                downsampled_audio_data,
            ),
            cpal::SampleFormat::U16 => run_audio_loop::<u16>(
                &device,
                &config.into(),
                waveform_type,
                note_state,
                octave_shift,
                global_time,
                tremolo_effect,
                scale,
                downsampled_audio_data,
            ),
            _ => panic!("Unsupported sample format"),
        }
    });

    // Run the main event loop
    // - Handle window events (e.g., close, resize)
    // - Handle user events (e.g., redraw)
    // - Update audio data based on the current time and sample rate
    // - Render the graphics and audio
    debug!("Starting event loop");
    run_event_loop(
        event_loop,
        &window,
        note_state.clone(),
        keys_config,
        waveform_type.clone(),
        octave_shift.clone(),
        tremolo_effect.clone(),
        scale.clone(),
        downsampled_audio_data.clone(),
    )
    .await?;

    audio_thread.join().unwrap()?;

    // Sleep for a specified duration to keep the application running

    Ok(())
}

fn run_audio_loop<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    waveform_type: Arc<RwLock<OscillatorWaveform>>,
    note_state: Arc<Mutex<NoteState>>,
    octave_shift: Arc<RwLock<i32>>,
    global_time: Arc<AtomicU64>,
    tremolo_effect: Arc<TremoloEffect>,
    scale: Arc<Mutex<Scale>>,
    downsampled_audio_data: Arc<Mutex<DownsampledAudioData>>,
) -> Result<(), anyhow::Error>
where
    T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
{
    // We calculate the sample rate and downsample factor to determine how many samples to
    // accummulate before downsampling the audio data. This helps reduce the computational load
    // while maintaining a smooth audio output.
    let sample_rate: f32 = config.sample_rate.0 as f32;
    let downsample_factor = (sample_rate / 60.0) as usize;
    let mut accumulated_samples = Vec::new();
    let channels = config.channels as usize;

    // We create a wave shaper node with a sine transfer function to apply distortion to the audio
    // output. This adds character and richness to the sound.
    let mut wave_shaper_node = WaveShaperNode {
        transfer_fn: |x| x.sin(),
    };

    // We define an error function to handle any errors that may occur during audio streaming.
    let err_fn = |err| eprintln!("An error occurred on the audio stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut output_buffer = AudioBuffer {
                data: vec![0.0; data.len()],
                num_channels: channels,
            };

            if let Ok(mut note_state) = note_state.lock() {
                if let Ok(octave_shift) = octave_shift.read() {
                    let playing_notes: Vec<(String, bool)> =
                        note_state.playing_notes.clone().into_iter().collect();

                    let current_time = global_time.load(Ordering::Relaxed) as f32 / sample_rate;
                    global_time.fetch_add(data.len() as u64, Ordering::Relaxed);

                    // We retain only the oscillators that correspond to currently playing notes.
                    // This ensures that oscillators are stopped and removed when their
                    // corresponding notes are released, and preventing unnecessary computation and
                    // memory usage.
                    note_state.oscillators.retain(|osc| {
                        playing_notes
                            .iter()
                            .any(|(note, is_playing)| note == &osc.note && *is_playing)
                    });

                    // We iterate over the playing notes to check if any new notes have been
                    // pressed. If a new note is detected and it's not already being played by an
                    // existing oscillator, we create a new oscillator for that note. this allows
                    // multiple oscillators to be played simultaneously, enabling polyphony in the
                    // synthesizer.
                    for (note, is_playing) in playing_notes.iter() {
                        if *is_playing
                            && !note_state.oscillators.iter().any(|osc| osc.note == *note)
                        {
                            if let Ok(scale) = scale.lock() {
                                if let Some(frequency) = scale.calculate_frequency(note) {
                                    // We adjust the frequency based on the octave shift to allow
                                    // the synthesizer to play notes in different octaves. This
                                    // gives the user more control over the pitch range of the
                                    // synthesizer.
                                    let adjusted_frequency =
                                        frequency * 2.0f32.powf(*octave_shift as f32);
                                    let mut oscillator = Oscillator::builder()
                                        .frequency(adjusted_frequency)
                                        .waveform(*waveform_type.read().unwrap())
                                        .attack_time(0.5)
                                        .release_time(0.5)
                                        .tremolo_effect(Arc::clone(&tremolo_effect))
                                        .build();
                                    oscillator.start_note(current_time);
                                    note_state.add_oscillator(oscillator);
                                }
                            }
                        }
                    }

                    // We update the waveform of each oscillator if the global waveform type has
                    // changed. This allows the user to switch between different waveforms (e.g.,
                    // sine, square, sawtooth) in real-time, providing variety in the timbre of the
                    // synthesized sound.
                    for oscillator in note_state.oscillators.iter_mut() {
                        if let Ok(current_waveform) = waveform_type.read() {
                            if *current_waveform != oscillator.get_waveform() {
                                oscillator.set_waveform(*current_waveform);
                            }
                        }

                        // We generate the waveform samples for each oscillator and accummulate
                        // them in the output buffer. This is done to mix the contributions of all
                        // active oscillators and create the final synthesized sound. The samples
                        // are scaled by a factor of 0.1 to prevent clipping and ensure a balanced
                        // mix.
                        let current_time = global_time.load(Ordering::Relaxed) as f32 / sample_rate;
                        let num_samples = output_buffer.data.len();
                        let generated_samples = oscillator.generate_wave(current_time, num_samples);
                        for (i, sample) in output_buffer.data.iter_mut().enumerate() {
                            *sample += generated_samples[i] * 0.1;
                        }
                    }
                }
            }

            // We apply the wave shaper effect to the output buffer to introduce distortion and
            // enhance the harmonic content of the synthesized sound. This is done to make the
            // sound more interesting and expressive.
            let mut output_buffer_copy = output_buffer.clone();
            wave_shaper_node.process(&output_buffer, &mut output_buffer_copy);

            // We accummulate the generated samples in a buffer to prepare for downsampling.
            // Downsampling is performed to reduce the computational load while maintaining a
            // smooth audio output. By accummulating samples and then averaging them, we can
            // effectively downsample the audio data without significant loss of quality.
            accumulated_samples.extend(output_buffer_copy.data.iter().cloned());
            debug!("Accumulated samples length: {}", accumulated_samples.len());

            // We check if enough samples have been accummulated to perform downsampling. This
            // ensures that downsampling occurs at regular intervals based on the calculated
            // downsample factor, which is determined by the sample rate and the desired
            // downsampled frame rate (60 fps in this case).
            if accumulated_samples.len() >= downsample_factor {
                let mut downsampled_samples = Vec::new();

                // We downsample the accumulated samples by averaging chunks of samples. This
                // reduces the sample rate while preserving the overall shape of the waveform.
                for chunk in accumulated_samples.chunks(downsample_factor) {
                    let sum: f32 = chunk.iter().sum();
                    let average = sum / chunk.len() as f32;
                    downsampled_samples.push(average);
                }

                // We store the downsampled audio data in a shared data structure to be used by
                // other parts of the application, such as visualization or further processing.
                if let Ok(mut downsampled_audio_data) = downsampled_audio_data.lock() {
                    let num_frames = downsampled_samples.len().min(256);
                    downsampled_audio_data.samples = [[0.0; 16]; 256];
                    for (i, chunk) in downsampled_samples.chunks(16).enumerate().take(num_frames) {
                        for (j, &sample) in chunk.iter().enumerate() {
                            downsampled_audio_data.samples[i][j] = sample;
                        }
                    }
                }

                // We clear the accummulated samples buffer after downsampling to prepare it for
                // the next batch of samples. This prevents the buffer from growing indefinitely
                // and consuming excessive memory.
                accumulated_samples.clear();
            }

            // We convert the floating-point samples to the output sample type and write them to
            // the audio output buffer. This ensures that the synthesized audio is compatible with
            // the audio backend and can be played back through the audio device.
            for (i, sample) in output_buffer_copy.data.iter_mut().enumerate() {
                data[i] = T::from_sample(*sample);
            }
        },
        err_fn,
        None,
    )?;
    stream.play()?;
    std::thread::sleep(Duration::from_secs(100));

    Ok(())
}

async fn run_event_loop(
    event_loop: EventLoop<()>,
    window: &winit::window::Window,
    note_state: Arc<Mutex<NoteState>>,
    keys_config: Arc<Config>,
    waveform_type: Arc<RwLock<OscillatorWaveform>>,
    octave_shift: Arc<RwLock<i32>>,
    tremolo_effect: Arc<TremoloEffect>,
    scale: Arc<Mutex<Scale>>,
    downsampled_audio_data: Arc<Mutex<DownsampledAudioData>>,
) -> Result<()> {
    info!("run_event_loop function called");
    let mut state = State::new(&window)
        .await
        .context("Failed to initialize state")?;

    let mut audio_data = AudioData {
        samples: [[0.0; 16]; 256],
    };

    let mut shift_pressed = false;

    let _ = event_loop.run(move |event, event_loop_window_target| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            debug!("The close button was pressed; stopping");
            event_loop_window_target.exit();
            std::process::exit(0);
        }

        Event::UserEvent(_) => {
            debug!("Received a UserEvent");
            window.request_redraw();
        }

        Event::WindowEvent {
            event:
                WindowEvent::KeyboardInput {
                    device_id: _,
                    event:
                        KeyEvent {
                            state, logical_key, ..
                        },
                    is_synthetic,
                    ..
                },
            ..
        } => {
            debug!("Received a KeyboardInput");
            if is_synthetic {
                println!("Received a synthetic keyboard event.");
            } else {
                let key_str = format!("{:?}", logical_key);
                let mut note_state = note_state.lock().unwrap();
                let tremolo_effect = tremolo_effect.clone();
                let scale = scale.clone();

                debug!("Current state: {:#?}", state);

                // Update the shift_pressed state based on the key event
                if key_str == "Named(Shift)" {
                    shift_pressed = state == ElementState::Pressed;
                }

                if state == ElementState::Pressed {
                    debug!("Key {} pressed", key_str);
                    if let Some(event) = keycode_to_action(&key_str, &*keys_config, shift_pressed) {
                        match event {
                            NoteEvent::ChangeOctave(direction) => {
                                if let Ok(mut octave_shift) = octave_shift.write() {
                                    *octave_shift += if direction == "up" { 1 } else { -1 };
                                    *octave_shift = octave_shift.clamp(-2, 2);
                                }
                            }
                            _ => note_state.handle_event(
                                event,
                                &waveform_type,
                                &tremolo_effect,
                                &scale,
                            ),
                        }
                        println!("Key pressed: {:?}", key_str);
                    }
                } else if state == ElementState::Released {
                    debug!("Key {} released", key_str);
                    if let Some(event) = keycode_to_action(&key_str, &*keys_config, shift_pressed) {
                        match event {
                            NoteEvent::On(note) => note_state.note_off(note),
                            NoteEvent::ChangeOctave(direction) => {
                                if let Ok(mut octave_shift) = octave_shift.write() {
                                    *octave_shift += if direction == "up" { 1 } else { -1 };
                                    *octave_shift = octave_shift.clamp(-2, 2);
                                }
                            }
                            NoteEvent::ToggleTremolo => note_state.handle_event(
                                event,
                                &waveform_type,
                                &tremolo_effect,
                                &scale,
                            ),
                            _ => (),
                        }
                        println!("Key released: {:?}", key_str);
                    }
                }
            }
        }

        Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } => {
            // Access the shared DownsampledAudioData structure to retrieve the downsampled audio samples
            if let Ok(downsampled_audio_data) = downsampled_audio_data.lock() {
                // Update the audio_data with the downsampled samples
                audio_data.samples = downsampled_audio_data.samples;
            }

            if let Err(e) = futures::executor::block_on(state.render(&window, &audio_data)) {
                error!("Render error: {}", e);
            }

            window.request_redraw();
        }

        _ => debug!("Other event: {:?}", event),
    });
    Ok(())
}

fn keycode_to_action(key: &str, config: &Config, shift_pressed: bool) -> Option<NoteEvent> {
    let key_str = key.to_string();
    debug!("Keycode to action called with key {}\n", key_str);

    // Check if the key matches any of the waveform change keys
    if let Some(waveform) = config.action_keys.change_waveform.get(&key_str) {
        debug!("Change waveform: {:?}, Key: {}\n", waveform, key_str);
        return Some(NoteEvent::ChangeWaveform(waveform.clone()));
    }

    // Check if the key matches the octave up key
    if key_str == config.keybindings.octave.up {
        debug!("Octave up key pressed: {}\n", key_str);
        return Some(NoteEvent::ChangeOctave("up".to_string()));
    }

    // Check if the key matches the octave down key
    if key_str == config.keybindings.octave.down {
        debug!("Octave down key pressed: {}\n", key_str);
        return Some(NoteEvent::ChangeOctave("down".to_string()));
    }

    // Check if the key matches the tremolo toggle key
    if key_str == config.keybindings.tremolo.toggle {
        debug!("Tremolo Toggled: {}\n", key_str);
        return Some(NoteEvent::ToggleTremolo);
    }

    // Check if the key matches any of the note keys
    if let Some(note) = config.keybindings.notes.keys.get(&key_str) {
        debug!("Note: {} key: {}\n", note, key_str);
        return Some(NoteEvent::On(note.clone()));
    }

    // Check if the Shift key is pressed and the key matches the uppercase variant of a note key
    if shift_pressed {
        let uppercase_key_str = key_str.to_uppercase();
        if let Some(note) = config.keybindings.notes.keys.get(&uppercase_key_str) {
            debug!("Note (Shift + Key): {} key: {}\n", note, uppercase_key_str);
            return Some(NoteEvent::On(note.clone()));
        }
    }

    // Check if the key matches any of the bass note keys
    if let Some(note) = config.keybindings.bass_notes.keys.get(&key_str) {
        debug!("Bass note: {} key: {}\n", note, key_str);
        return Some(NoteEvent::On(note.clone()));
    }

    // Check if the key matches any of the key change keys
    if let Some(note) = config.keybindings.key_change.keys.get(&key_str) {
        debug!("Key change: {} key {}\n", note, key_str);
        // Implement the logic for handling key change events
        // For example, you can update the scale or root note based on the key change
        // Return the appropriate NoteEvent or None
    }

    // If no matching key is found, return None
    warn!("NO MATCHING KEY FOUND, RETURNING NONE. KEY: {}\n", key_str);
    None
}
