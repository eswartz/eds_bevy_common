//! This module models musical audio by allowing
//! clients to send [SynthEvent] to generate notes on [MidiSynth] components
//! at some point in the future.
use bevy::prelude::*;
use crate::midi_synth::synth::MidiSynthParams;
use std::time::Duration;

pub struct SynthPlugin;

impl Plugin for SynthPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SynthNote>()
            .register_type::<SynthCommand>()
            .register_type::<SynthChannel>()
            .register_type::<SynthClock>()
            .insert_resource(SynthClock::new(Duration::from_secs_f32(0.0)))
            .add_message::<SynthMessage>()
            ;
    }
}

/// This clock aligns multiple synths to the same beat.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct SynthClock {
    beat_time: Duration,
    beat_tick: Duration,
}

impl SynthClock {
    pub fn new(beat_time: Duration) -> Self {
        Self {
            beat_time,
            beat_tick: beat_time
        }
    }
    pub fn delay_to_next(&self) -> Duration {
        self.beat_time.saturating_sub(self.beat_tick)
    }
    pub fn tick(&mut self, delta: Duration) {
        if !self.beat_time.is_zero() {
            self.beat_tick = Duration::from_secs_f32((self.beat_tick + delta).as_secs_f32() % self.beat_time.as_secs_f32());
        }
    }
}

/// A note for the synthesizer.
/// This represents values as midi notes shifted left 8, to make room for pitch bend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect, serde::Serialize, serde::Deserialize)]
#[reflect(Clone)]
#[type_path = "game"]
pub struct SynthNote(u16);

#[allow(unused)]
impl SynthNote {
    /// Create from a frequency in Hz.
    pub fn hz(freq: f32) -> Self {
        let note = ((12.0 * (freq / 440.).log2() + 69.0) * 256.0).clamp(0., 255.) as u16;
        Self(note)
    }
    /// Create from a MIDI note.
    pub fn midi(note: u8) -> Self {
        Self((note as u16) << 8)
    }
    /// Add a pitch bend, range -1...1.
    pub fn with_bend(self, bend: f32) -> Self {
        Self(
            self.0
                .saturating_add_signed((bend.clamp(-1.0, 1.0) * 256.0) as i16),
        )
    }

    pub fn to_midi(&self) -> u8 {
        (((self.0.saturating_add(128)) >> 8) as u8).clamp(0, 127)
    }
}

/// Specifies the target of synth notes and events.
///
/// The indices of the voice and drum do NOT have any relationship with midi.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub struct SynthVirtualChannel();


/// Specifies the target of synth notes and events.
///
/// The indices of the voice and drum do NOT have any relationship with midi.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect, serde::Serialize, serde::Deserialize)]
#[reflect(Clone)]
#[type_path = "game"]
pub enum SynthChannel {
    Voice(u8),
    Drums(u8),
}

impl Default for SynthChannel {
    fn default() -> Self {
        Self::Voice(0)
    }
}

#[allow(unused)]
impl SynthChannel {
    pub fn drums(v: u8) -> Self {
        Self::Drums(v)
    }

    pub(crate) fn next(&self) -> SynthChannel {
        match self {
            SynthChannel::Voice(v) => SynthChannel::Voice(v.wrapping_add(1)),
            SynthChannel::Drums(d) => SynthChannel::Drums(d.wrapping_add(1)),
        }
    }
}

/// The different kinds of commands for the synth layer.
#[derive(Clone, Debug, PartialEq, Reflect, serde::Serialize, serde::Deserialize)]
#[reflect(Clone)]
#[type_path = "game"]
pub enum SynthCommand {
    NoteOn(SynthChannel, SynthNote, f32),
    NoteOff(SynthChannel, SynthNote),
    ChannelOff(SynthChannel),
    /// Program is 0-based.
    ProgramChange(SynthChannel, u8),
    ChannelVolume(SynthChannel, f32),
    Reset,
}

/// A command which will play on the MidiSynth in the given entity after the given time.
#[derive(Message, Clone, Debug, PartialEq, Reflect, serde::Serialize, serde::Deserialize)]
#[reflect(Clone)]
#[type_path = "game"]
pub struct SynthMessage(pub Entity, pub SynthCommand, pub Duration);

#[allow(unused)]
impl SynthMessage {
    pub fn new(entity: Entity, command: SynthCommand) -> Self {
        Self(entity, command, Duration::ZERO)
    }

    pub fn after(self, after: Duration) -> Self {
        SynthMessage(self.0, self.1, after)
    }

    pub fn after_secs(self, secs: f32) -> Self {
        SynthMessage(self.0, self.1, Duration::from_secs_f32(secs))
    }
}


/// This component, added by the server, marks an entity as a proxy for
/// various SynthEvents. Its entity is passed along in SynthEvents.
/// The client will farm the events out to actual MidiSynths, based on
/// the user's CPU capacity.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub struct MidiSynthProxy{
    pub params: MidiSynthParams,
    pub bank: String,
    // pub channel: AudioVirtualChannel,
}
