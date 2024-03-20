use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

use crate::synth::oscillator::OscillatorWaveform;

pub const NOTE_SEQUENCE: [&str; 13] = [
    "C", "C_SHARP", "D", "D_SHARP", "E", "F", "F_SHARP", "G", "G_SHARP", "A", "A_SHARP", "B",
    "C_HIGH",
];

#[derive(Debug)]
pub enum NoteEvent {
    On(String),
    Off(String),
    ChangeWaveform(OscillatorWaveform),
    ChangeOctave(String),
    ToggleTremolo,
    ChangeKey(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub keybindings: KeyBindings,
    pub action_keys: ActionKeys,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyBindings {
    pub notes: NoteKeys,
    pub octave: OctaveKeys,
    pub bass_notes: BassNoteKeys,
    pub key_change: KeyChangeKeys,
    pub tremolo: TremoloKeys,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TremoloKeys {
    pub toggle: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WaveformKeys {
    pub keys: HashMap<String, OscillatorWaveform>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NoteKeys {
    pub keys: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OctaveKeys {
    pub up: String,
    pub down: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BassNoteKeys {
    pub keys: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyChangeKeys {
    pub keys: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionKeys {
    pub toggle_notes: HashMap<String, String>,
    pub change_waveform: HashMap<String, OscillatorWaveform>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Scale {
    pub root_note: String,
    pub intervals: Vec<i32>,
}

impl Scale {
    // Assuming `position` is a scale degree (1-indexed for ease of understanding musical context)
    pub fn get_note_from_position(&self, position: usize) -> Option<String> {
        debug!("Getting note from position: {}", position);
        if position == 0 || self.intervals.is_empty() {
            debug!("Invalid position or empty intervals");
            return None;
        }

        // Find the root note index in NOTE_SEQUENCE
        let root_index = NOTE_SEQUENCE
            .iter()
            .position(|&n| n == self.root_note)
            .unwrap_or(0);

        // Calculate the actual note position in the chromatic sequence based on intervals
        let mut note_index = root_index;
        for idx in 0..(position - 1) {
            // Ensure we loop within the scale intervals
            let step = self.intervals.get(idx % self.intervals.len()).unwrap_or(&0);
            note_index = (note_index as i32 + step) as usize % NOTE_SEQUENCE.len();
        }

        NOTE_SEQUENCE.get(note_index).map(|&note| note.to_string())
    }

    pub fn change_root_note(&mut self, new_root: String) {
        self.root_note = new_root;
        // Optionally adjust intervals if changing modes
    }

    pub fn calculate_frequency(&self, note: &str) -> Option<f32> {
        let a4_index = 12; // A4 is the 13th note in the sequence (including accidentals), but index 12 in a 0-indexed array
        let a4_frequency = 440.0;

        debug!("Calculating frequency for note: {}", note);

        if let Some(note_index) = NOTE_SEQUENCE.iter().position(|&n| n == note.to_uppercase()) {
            let semitone_distance = note_index as i32 - a4_index as i32;
            let frequency = a4_frequency * (2.0f32).powf(semitone_distance as f32 / 12.0);
            debug!(
                "Note index: {}, Semitone distance: {}, Frequency: {}",
                note_index, semitone_distance, frequency
            );
            Some(frequency)
        } else {
            debug!("Note not found in sequence: {}", note);
            None
        }
    }
}
