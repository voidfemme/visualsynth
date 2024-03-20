pub mod adsr_envelope;
pub mod audiobuffer;
pub mod keys;
pub mod modulator;
pub mod node;
pub mod oscillator;
pub mod tremolo;
pub mod utils;
pub mod waveform_generator;

pub use audiobuffer::AudioBuffer;
pub use keys::{
    keys::Scale,
    keys::{Config, NoteEvent},
    note_state::NoteState,
};
pub use node::{AudioNode, WaveShaperNode};
pub use oscillator::{Oscillator, OscillatorWaveform};
pub use tremolo::TremoloEffect;
