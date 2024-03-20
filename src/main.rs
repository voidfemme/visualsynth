// visiosynth/src/main.rs

use anyhow::{Context, Ok, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use futures;
use serde_yaml;
use std::fs::File;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, error, info, Level};
use tracing_subscriber;
use visiosynth::{
    graphics::{AudioData, State},
    synth::{
        AudioBuffer, AudioNode, Config, NoteEvent, NoteState, Oscillator, OscillatorWaveform,
        Scale, TremoloEffect, WaveShaperNode,
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
    let global_time = Arc::new(Mutex::new(0.0));
    let waveform_type = Arc::new(Mutex::new(OscillatorWaveform::Silence));
    let octave_shift = Arc::new(Mutex::new(0));
    let note_state = Arc::new(Mutex::new(NoteState::new()));
    let keys_config = Arc::new(keys_config);
    let tremolo_effect = Arc::new(Mutex::new(
        TremoloEffect::builder()
            .rate(5.0)
            .depth(0.5)
            .enabled(false)
            .build(config.sample_rate().0 as f32),
    ));
    let scale = Arc::new(Mutex::new(Scale {
        root_note: "C".to_string(),
        intervals: vec![2, 2, 1, 2, 2, 2, 1],
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
            ),
            _ => panic!("Unsupported sample format"),
        }
    });

    // Run the main event loop
    // - Handle window events (e.g., close, resize)
    // - Handle user events (e.g., redraw)
    // - Update audio data based on the current time and sample rate
    // - Render the graphics and audio
    info!("Starting event loop");
    run_event_loop(
        event_loop,
        &window,
        note_state.clone(),
        keys_config,
        waveform_type.clone(),
        octave_shift.clone(),
        tremolo_effect.clone(),
        scale.clone(),
    )
    .await?;

    audio_thread.join().unwrap()?;

    // Sleep for a specified duration to keep the application running

    Ok(())
}

fn run_audio_loop<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    waveform_type: Arc<Mutex<OscillatorWaveform>>,
    note_state: Arc<Mutex<NoteState>>,
    octave_shift: Arc<Mutex<i32>>,
    global_time: Arc<Mutex<f32>>,
    tremolo_effect: Arc<Mutex<TremoloEffect>>,
    scale: Arc<Mutex<Scale>>,
) -> Result<(), anyhow::Error>
where
    T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
{
    info!(
        "Initial waveform type: {:?}",
        *waveform_type.lock().unwrap()
    );

    let sample_rate: f32 = config.sample_rate.0 as f32;
    let channels = config.channels as usize;

    // Initialize oscillator and modulator
    let mut wave_shaper_node = WaveShaperNode {
        transfer_fn: |x| x.sin(),
    };

    let err_fn = |err| eprintln!("An error occurred on the audio stream: {}", err);

    let global_time_clone = Arc::clone(&global_time);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut output_buffer = AudioBuffer {
                data: vec![0.0; data.len()],
                num_channels: channels,
            };

            let mut note_state = note_state.lock().unwrap();
            let octave_shift = octave_shift.lock().unwrap();

            let playing_notes: Vec<(String, bool)> =
                note_state.playing_notes.clone().into_iter().collect();

            let mut global_time = global_time_clone.lock().unwrap();
            let current_time = *global_time;
            *global_time += data.len() as f32 / sample_rate;

            note_state.oscillators.retain(|osc| {
                playing_notes
                    .iter()
                    .any(|(note, is_playing)| note == &osc.note && *is_playing)
            });

            {
                for (note, is_playing) in playing_notes.iter() {
                    if *is_playing && !note_state.oscillators.iter().any(|osc| osc.note == *note) {
                        if let Some(frequency) = scale.lock().unwrap().calculate_frequency(note) {
                            let adjusted_frequency = frequency * 2.0f32.powf(*octave_shift as f32);
                            let mut oscillator = Oscillator::builder()
                                .frequency(adjusted_frequency)
                                .waveform(*waveform_type.lock().unwrap())
                                .attack_time(0.5)
                                .release_time(0.5)
                                .tremolo_effect(tremolo_effect.clone())
                                .build();
                            oscillator.start_note(current_time);
                            note_state.add_oscillator(oscillator);
                        }
                    }
                }

                for oscillator in note_state.oscillators.iter_mut() {
                    let current_waveform = *waveform_type.lock().unwrap();
                    if current_waveform != oscillator.get_waveform() {
                        oscillator.set_waveform(current_waveform);
                    }

                    for (i, sample) in output_buffer.data.iter_mut().enumerate() {
                        let current_time = i as f32 / sample_rate;
                        *sample += oscillator.generate_wave(current_time) * 0.01;
                        // Adjust the volume here
                    }
                }
            }

            let mut output_buffer_copy = output_buffer.clone();
            wave_shaper_node.process(&output_buffer, &mut output_buffer_copy);

            for (i, sample) in output_buffer.data.iter_mut().enumerate() {
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
    waveform_type: Arc<Mutex<OscillatorWaveform>>,
    octave_shift: Arc<Mutex<i32>>,
    tremolo_effect: Arc<Mutex<TremoloEffect>>,
    scale: Arc<Mutex<Scale>>,
) -> Result<()> {
    info!("run_event_loop function called");
    let mut state = State::new(&window)
        .await
        .context("Failed to initialize state")?;

    let mut audio_data = AudioData {
        samples: [[0.0; 16]; 256],
    };
    let mut current_time = 0.0;
    let sample_rate = 60.0;
    let frequency = 1.0;

    let _ = event_loop.run(move |event, event_loop_window_target| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            info!("The close button was pressed; stopping");
            event_loop_window_target.exit();
            std::process::exit(0);
        }

        Event::UserEvent(_) => {
            info!("Received a UserEvent");
            window.request_redraw();
        }

        Event::WindowEvent {
            event:
                WindowEvent::KeyboardInput {
                    device_id,
                    event:
                        KeyEvent {
                            state, logical_key, ..
                        },
                    is_synthetic,
                    ..
                },
            ..
        } => {
            info!("Received a KeyboardInput");
            if is_synthetic {
                println!("Received a synthetic keyboard event.");
            } else {
                let key_str = format!("{:?}", logical_key);
                let mut note_state = note_state.lock().unwrap();
                let mut octave_shift = octave_shift.lock().unwrap();
                let mut waveform_type = waveform_type.clone();
                let mut tremolo_effect = tremolo_effect.clone();
                let mut scale = scale.clone();

                info!("Current state: {:#?}", state);
                if state == ElementState::Pressed {
                    info!("Key {} pressed", key_str);
                    if let Some(event) = keycode_to_action(&key_str, &*keys_config) {
                        match event {
                            NoteEvent::ChangeOctave(direction) => {
                                *octave_shift += if direction == "up" { 1 } else { -1 };
                                *octave_shift = octave_shift.clamp(-2, 2);
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
                    info!("Key {} released", key_str);
                    if let Some(event) = keycode_to_action(&key_str, &*keys_config) {
                        match event {
                            NoteEvent::On(note) => note_state.note_off(note),
                            NoteEvent::ChangeOctave(direction) => {
                                *octave_shift += if direction == "up" { 1 } else { -1 };
                                *octave_shift = octave_shift.clamp(-2, 2);
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
            // Update the current time based on the elapsed time
            current_time += 1.0 / sample_rate;

            for i in 0..audio_data.samples.len() {
                for j in 0..audio_data.samples[i].len() {
                    let t = (i * 16 + j) as f32 / sample_rate;
                    let sample =
                        (2.0 * std::f32::consts::PI * frequency * (current_time + t)).sin();
                    audio_data.samples[i][j] = sample;
                }
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

// Helper functions:
// - keycode_to_action: Convert key events to note events based on the config
fn keycode_to_action(key: &str, config: &Config) -> Option<NoteEvent> {
    // Convert the key string to lowercase for case-insensitive comparison
    let key_str = key.to_string();
    info!("Keycode to action called with key {}\n", key_str);

    // Check if the key matches any of the waveform change keys
    if let Some(waveform) = config.action_keys.change_waveform.get(&key_str) {
        info!("Change waveform: {:?}, Key: {}\n", waveform, key_str);
        return Some(NoteEvent::ChangeWaveform(waveform.clone()));
    }

    // Check if the key matches the octave up key
    if key_str == config.keybindings.octave.up {
        info!("Octave up key pressed: {}\n", key_str);
        return Some(NoteEvent::ChangeOctave("up".to_string()));
    }

    // Check if the key matches the octave down key
    if key_str == config.keybindings.octave.down {
        info!("Octave down key pressed: {}\n", key_str);
        return Some(NoteEvent::ChangeOctave("down".to_string()));
    }

    // Check if the key matches the tremolo toggle key
    if key_str == config.keybindings.tremolo.toggle {
        info!("Tremolo Toggled: {}\n", key_str);
        return Some(NoteEvent::ToggleTremolo);
    }

    // Check if the key matches any of the note keys
    if let Some(note) = config.keybindings.notes.keys.get(&key_str) {
        info!("Note: {} key: {}\n", note, key_str);
        return Some(NoteEvent::On(note.clone()));
    }

    // Check if the key matches any of the bass note keys
    if let Some(note) = config.keybindings.bass_notes.keys.get(&key_str) {
        info!("Bass note: {} key: {}\n", note, key_str);
        return Some(NoteEvent::On(note.clone()));
    }

    // Check if the key matches any of the key change keys
    if let Some(note) = config.keybindings.key_change.keys.get(&key_str) {
        info!("Key change: {} key {}\n", note, key_str);
        // Implement the logic for handling key change events
        // For example, you can update the scale or root note based on the key change
        // Return the appropriate NoteEvent or None
    }

    // If no matching key is found, return None
    info!("NO MATCHING KEY FOUND, RETURNING NONE. KEY: {}\n", key_str);
    None
}