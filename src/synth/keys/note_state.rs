use std::sync::{Arc, Mutex};

use crate::synth::{NoteEvent, Oscillator, OscillatorWaveform, Scale, TremoloEffect};

#[derive(Debug, Default)]
pub struct NoteState {
    pub playing_notes: std::collections::HashMap<String, bool>,
    pub activation_order: std::collections::HashMap<String, usize>,
    pub oscillators: Vec<Oscillator>,
}

impl NoteState {
    pub fn new() -> Self {
        Self {
            playing_notes: std::collections::HashMap::new(),
            activation_order: std::collections::HashMap::new(),
            oscillators: Vec::new(),
        }
    }

    pub fn add_oscillator(&mut self, oscillator: Oscillator) {
        self.oscillators.push(oscillator);
    }

    pub fn remove_oscillator(&mut self, note: &str) {
        self.oscillators.retain(|osc| osc.note != note);
    }

    pub fn handle_event(
        &mut self,
        event: NoteEvent,
        waveform_type: &Arc<Mutex<OscillatorWaveform>>,
        tremolo_effect: &Arc<Mutex<TremoloEffect>>,
        scale: &Arc<Mutex<Scale>>,
    ) {
        println!("Event: {:?}", event);
        match event {
            NoteEvent::On(note) => self.note_on(note),
            NoteEvent::Off(note) => self.note_off(note),
            NoteEvent::ChangeWaveform(waveform) => {
                let mut waveform_type = waveform_type.lock().unwrap();
                *waveform_type = waveform;
            }
            NoteEvent::ChangeOctave(direction) => self.change_octave(direction),
            NoteEvent::ToggleTremolo => {
                let mut tremolo_effect = tremolo_effect.lock().unwrap();
                tremolo_effect.toggle();
            }
            NoteEvent::ChangeKey(new_key) => {
                let mut scale = scale.lock().unwrap();
                scale.change_root_note(new_key);
            }
        }
    }

    pub fn change_octave(&mut self, direction: String) {
        let octave_shift = match direction.as_str() {
            "up" => 1,
            "down" => -1,
            _ => 0,
        };

        for oscillator in self.oscillators.iter_mut() {
            let new_frequency = oscillator.get_frequency() * 2.0f32.powf(octave_shift as f32);
            oscillator.set_frequency(new_frequency);
        }
    }

    pub fn note_on(&mut self, note: String) {
        // info!("Note on: {}", note);
        self.playing_notes.insert(note, true);
    }

    pub fn note_off(&mut self, note: String) {
        // info!("Note off: {}", note);
        self.playing_notes.insert(note, false);
    }

    pub fn is_playing(&self, note: &String) -> bool {
        *self.playing_notes.get(note).unwrap_or(&false)
    }

    pub fn find_active_note(&self) -> Option<String> {
        self.playing_notes
            .iter()
            .filter(|(_note, &is_playing)| is_playing)
            .max_by_key(|(note, _)| self.activation_order.get(*note))
            .map(|(note, _)| note.clone())
    }
}
