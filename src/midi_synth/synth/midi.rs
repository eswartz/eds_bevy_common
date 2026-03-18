#![doc = r#"
MIDI definitions.
"#]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiRenderMessage {
    /// Render this many frames.
    RenderFrame(usize),
    /// Nothing to render.
    Idle(usize),
}

/// Messages driving MIDI synthesis.
///
/// Channels are given their MIDI meaning: 0 through 15, where 9 = drums.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MidiMessage {
    /// Turn off the a note on the given channel.
    NoteOff {
        ///
        channel: u8,
        ///
        note: u8
    },
    /// Turn on the note on the given channel, with the given velocity (0=silent, 127=max).
    NoteOn {
        ///
        channel: u8,
        ///
        note: u8,
        ///
        velocity: u8
    },
    /// Send a controller command.
    Controller {
        ///
        channel: u8,
        ///
        ctrl: u8,
        ///
        data: u8
    },
    /// Change the program of the channel
    SetPatch {
        ///
        channel: u8,
        /// 1-based patch
        patch: u8
    },
    /// Initiate a pitch bend.
    PitchBend {
        ///
        channel: u8,
        ///
        data1: u8,
        ///
        data2: u8
    },
}

impl std::fmt::Debug for MidiMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoteOff { channel, note } => write!(f, "NoteOff{{ channel={channel:2}, note={note:2} }}"),
            Self::NoteOn { channel, note, velocity } => write!(f, "NoteOn {{ channel={channel:2}, note={note:2}, velocity={velocity} }}"),
            Self::Controller { channel, ctrl, data } => f.debug_struct("Controller").field("channel", channel).field("ctrl", ctrl).field("data", data).finish(),
            Self::SetPatch { channel, patch } => f.debug_struct("SetPatch").field("channel", channel).field("patch", patch).finish(),
            Self::PitchBend { channel, data1, data2 } => f.debug_struct("PitchBend").field("channel", channel).field("data1", data1).field("data2", data2).finish(),
        }
    }
}

impl MidiMessage {
    ///
    pub const NOTE_OFF: u8 = 0x80;
    ///
    pub const NOTE_ON: u8 = 0x90;
    ///
    pub const CONTROLLER: u8 = 0xB0;
    ///
    pub const PROGRAM_CHANGE: u8 = 0xC0;
    ///
    pub const PITCH_BEND: u8 = 0xE0;
    ///
    pub const CTRL_SET_BANK: u8 = 0x00;
    ///
    pub const CTRL_SET_MODULATION_COARSE: u8 = 0x01;
    ///
    pub const CTRL_SET_MODULATION_FINE: u8 = 0x21;
    ///
    pub const CTRL_DATA_ENTRY_COARSE: u8 = 0x06;
    ///
    pub const CTRL_DATA_ENTRY_FINE: u8 = 0x26;
    ///
    pub const CTRL_SET_VOLUME_COARSE: u8 = 0x07;
    ///
    pub const CTRL_SET_VOLUME_FINE: u8 = 0x27;
    ///
    pub const CTRL_SET_PAN_COARSE: u8 = 0x0A;
    ///
    pub const CTRL_SET_PAN_FINE: u8 = 0x2A;
    ///
    pub const CTRL_SET_EXPRESSION_COARSE: u8 = 0x0B;
    ///
    pub const CTRL_SET_EXPRESSION_FINE: u8 = 0x2B;
    ///
    pub const CTRL_SET_HOLD_PEDAL: u8 = 0x40;
    ///
    pub const CTRL_SET_REVERB_SEND: u8 = 0x5B;
    ///
    pub const CTRL_SET_CHORUS_SEND: u8 = 0x5D;
    ///
    pub const CTRL_SET_NRPN_COARSE: u8 = 0x63;
    ///
    pub const CTRL_SET_NRPN_FINE: u8 = 0x62;
    ///
    pub const CTRL_SET_RPN_COARSE: u8 = 0x65;
    ///
    pub const CTRL_SET_RPN_FINE: u8 = 0x64;
    ///
    pub const CTRL_ALL_SOUNDS_OFF: u8 = 0x78;

    pub(crate) fn data_1_byte(&self) -> u8 {
        match self {
            Self::NoteOff { note, .. } => *note,
            Self::NoteOn { note, .. } => *note,
            Self::Controller { ctrl, .. } => *ctrl,
            Self::SetPatch { patch, .. } => *patch,
            Self::PitchBend { data1, .. } => *data1,
        }
    }

    pub(crate) fn data_2_byte(&self) -> u8 {
        match self {
            Self::NoteOff { .. } => 0,
            Self::NoteOn { velocity, .. } => *velocity,
            Self::Controller { data, .. } => *data,
            Self::SetPatch { .. } => 0,
            Self::PitchBend { data2, .. } => *data2,
        }
    }

    pub(crate) fn channel(&self) -> u8 {
        match self {
            Self::NoteOff { channel, .. }
            | Self::NoteOn { channel, .. }
            | Self::Controller { channel, .. }
            | Self::SetPatch { channel, .. }
            | Self::PitchBend { channel, .. } => *channel,
        }
    }

    pub(crate) fn command(&self) -> u8 {
        match self {
            Self::NoteOff { .. } => Self::NOTE_OFF,
            Self::NoteOn { .. } => Self::NOTE_ON,
            Self::Controller { .. } => Self::CONTROLLER,
            Self::SetPatch { .. } => Self::PROGRAM_CHANGE,
            Self::PitchBend { .. } => Self::PITCH_BEND,
        }
    }
}
