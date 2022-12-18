mod clip_engine;

use crate::base;
use crate::base::ClipSlotAddress;
pub use clip_engine::*;
use playtime_api::runtime::ClipPlayState;
use reaper_medium::{InputMonitoringMode, RecordingInput, RgbColor};

impl SlotAddress {
    pub fn from_engine(address: ClipSlotAddress) -> Self {
        Self {
            column_index: address.column() as _,
            row_index: address.row() as _,
        }
    }

    pub fn to_engine(&self) -> ClipSlotAddress {
        ClipSlotAddress::new(self.column_index as _, self.row_index as _)
    }
}

impl ClipAddress {
    pub fn from_engine(address: base::ClipAddress) -> Self {
        Self {
            slot_address: Some(SlotAddress::from_engine(address.slot_address)),
            clip_index: address.clip_index as _,
        }
    }

    pub fn to_engine(&self) -> Result<base::ClipAddress, &'static str> {
        let addr = base::ClipAddress {
            slot_address: self
                .slot_address
                .as_ref()
                .ok_or("slot address missing")?
                .to_engine(),
            clip_index: self.clip_index as usize,
        };
        Ok(addr)
    }
}

impl SlotPlayState {
    pub fn from_engine(play_state: ClipPlayState) -> Self {
        use ClipPlayState::*;
        match play_state {
            Stopped => Self::Stopped,
            ScheduledForPlayStart => Self::ScheduledForPlayStart,
            Playing => Self::Playing,
            Paused => Self::Paused,
            ScheduledForPlayStop => Self::ScheduledForPlayStop,
            ScheduledForRecordingStart => Self::ScheduledForRecordingStart,
            Recording => Self::Recording,
            ScheduledForRecordingStop => Self::ScheduledForRecordingStop,
        }
    }
}

impl TrackColor {
    pub fn from_engine(color: Option<RgbColor>) -> Self {
        Self {
            color: color
                .map(|c| (((c.r as u32) << 16) + ((c.g as u32) << 8) + (c.b as u32)) as i32),
        }
    }
}

impl TrackInput {
    pub fn from_engine(input: Option<RecordingInput>) -> Self {
        use track_input::Input;
        use RecordingInput::*;
        let input = match input {
            Some(Mono(ch)) => Some(Input::Mono(ch)),
            Some(Stereo(ch)) => Some(Input::Stereo(ch)),
            Some(Midi { device_id, channel }) => {
                let midi_input = TrackMidiInput {
                    device: device_id.map(|id| id.get() as _),
                    channel: channel.map(|ch| ch.get() as _),
                };
                Some(Input::Midi(midi_input))
            }
            _ => None,
        };
        Self { input }
    }
}

impl TrackInputMonitoring {
    pub fn from_engine(mode: InputMonitoringMode) -> Self {
        match mode {
            InputMonitoringMode::Off => Self::Off,
            InputMonitoringMode::Normal => Self::Normal,
            InputMonitoringMode::NotWhenPlaying => Self::TapeStyle,
            InputMonitoringMode::Unknown(_) => Self::Unknown,
        }
    }
}

impl ArrangementPlayState {
    pub fn from_engine(play_state: reaper_medium::PlayState) -> Self {
        if play_state.is_recording {
            if play_state.is_paused {
                Self::RecordingPaused
            } else {
                Self::Recording
            }
        } else if play_state.is_playing {
            if play_state.is_paused {
                Self::PlayingPaused
            } else {
                Self::Playing
            }
        } else if play_state.is_paused {
            Self::PlayingPaused
        } else {
            Self::Stopped
        }
    }
}
