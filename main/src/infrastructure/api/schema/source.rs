pub use midi::*;
pub use osc::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
pub use virt::*;

#[derive(Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind")]
pub enum Source {
    // None
    #[serde(rename = "None")]
    NoneSource,
    // REAPER
    MidiDeviceChanges(MidiDeviceChangesSource),
    RealearnInstanceStart(RealearnInstanceStartSource),
    // MIDI
    MidiNoteVelocity(MidiNoteVelocitySource),
    MidiNoteKeyNumber(MidiNoteKeyNumberSource),
    MidiPolyphonicKeyPressureAmount(MidiPolyphonicKeyPressureAmountSource),
    MidiControlChangeValue(MidiControlChangeValueSource),
    MidiProgramChangeNumber(MidiProgramChangeNumberSource),
    MidiChannelPressureAmount(MidiChannelPressureAmountSource),
    MidiPitchBendChangeValue(MidiPitchBendChangeValueSource),
    MidiParameterNumberValue(MidiParameterNumberValueSource),
    MidiClockTempo(MidiClockTempoSource),
    MidiClockTransport(MidiClockTransportSource),
    MidiRaw(MidiRawSource),
    MidiScript(MidiScriptSource),
    MackieLcd(MackieLcd),
    MackieSevenSegmentDisplay(MackieSevenSegmentDisplay),
    SiniConE24Display(SiniConE24Display),
    LaunchpadProScrollingTextDisplay(LaunchpadProScrollingTextDisplay),
    // OSC
    Osc(OscSource),
    Virtual(VirtualSource),
}

impl Default for Source {
    fn default() -> Self {
        Source::NoneSource
    }
}

mod midi {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use ts_rs::TS;

    #[derive(Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(rename_all = "snake_case")]
    #[serde(deny_unknown_fields)]
    pub struct MidiNoteVelocitySource {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub channel: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub key_number: Option<u8>,
    }

    #[derive(Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(rename_all = "snake_case")]
    #[serde(deny_unknown_fields)]
    pub struct MidiNoteKeyNumberSource {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub channel: Option<u8>,
    }

    #[derive(Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(rename_all = "snake_case")]
    #[serde(deny_unknown_fields)]
    pub struct MidiPolyphonicKeyPressureAmountSource {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub channel: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub key_number: Option<u8>,
    }

    #[derive(Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(rename_all = "snake_case")]
    #[serde(deny_unknown_fields)]
    pub struct MidiControlChangeValueSource {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub channel: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub controller_number: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub character: Option<SourceCharacter>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub fourteen_bit: Option<bool>,
    }

    #[derive(Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(rename_all = "snake_case")]
    #[serde(deny_unknown_fields)]
    pub struct MidiProgramChangeNumberSource {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub channel: Option<u8>,
    }

    #[derive(Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(rename_all = "snake_case")]
    #[serde(deny_unknown_fields)]
    pub struct MidiChannelPressureAmountSource {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub channel: Option<u8>,
    }

    #[derive(Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(rename_all = "snake_case")]
    #[serde(deny_unknown_fields)]
    pub struct MidiPitchBendChangeValueSource {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub channel: Option<u8>,
    }

    #[derive(Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(rename_all = "snake_case")]
    #[serde(deny_unknown_fields)]
    pub struct MidiParameterNumberValueSource {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub channel: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub number: Option<u16>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub fourteen_bit: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub registered: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub character: Option<SourceCharacter>,
    }

    #[derive(Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(deny_unknown_fields)]
    pub struct MidiClockTempoSource;

    #[derive(Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(deny_unknown_fields)]
    pub struct MidiDeviceChangesSource;

    #[derive(Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(deny_unknown_fields)]
    pub struct RealearnInstanceStartSource;

    #[derive(Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(rename_all = "snake_case")]
    #[serde(deny_unknown_fields)]
    pub struct MidiClockTransportSource {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub message: Option<MidiClockTransportMessage>,
    }

    #[derive(Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(rename_all = "snake_case")]
    #[serde(deny_unknown_fields)]
    pub struct MidiRawSource {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub pattern: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub character: Option<SourceCharacter>,
    }

    #[derive(Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(rename_all = "snake_case")]
    #[serde(deny_unknown_fields)]
    pub struct MidiScriptSource {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub script: Option<String>,
    }

    #[derive(Copy, Clone, Serialize, Deserialize, JsonSchema, TS)]
    pub enum SourceCharacter {
        Range,
        Button,
        Relative1,
        Relative2,
        Relative3,
        StatefulButton,
    }

    impl Default for SourceCharacter {
        fn default() -> Self {
            SourceCharacter::Range
        }
    }

    #[derive(Copy, Clone, Serialize, Deserialize, JsonSchema, TS)]
    pub enum MidiClockTransportMessage {
        Start,
        Continue,
        Stop,
    }

    impl Default for MidiClockTransportMessage {
        fn default() -> Self {
            MidiClockTransportMessage::Start
        }
    }

    #[derive(Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(rename_all = "snake_case")]
    #[serde(deny_unknown_fields)]
    pub struct MackieLcd {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub channel: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub line: Option<u8>,
    }

    #[derive(Copy, Clone, Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(rename_all = "snake_case")]
    #[serde(deny_unknown_fields)]
    pub struct MackieSevenSegmentDisplay {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub scope: Option<MackieSevenSegmentDisplayScope>,
    }

    #[derive(Copy, Clone, Serialize, Deserialize, JsonSchema, TS)]
    pub enum MackieSevenSegmentDisplayScope {
        All,
        Assignment,
        Tc,
        TcHoursBars,
        TcMinutesBeats,
        TcSecondsSub,
        TcFramesTicks,
    }

    impl Default for MackieSevenSegmentDisplayScope {
        fn default() -> Self {
            MackieSevenSegmentDisplayScope::Assignment
        }
    }

    #[derive(Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(rename_all = "snake_case")]
    #[serde(deny_unknown_fields)]
    pub struct SiniConE24Display {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub cell_index: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub item_index: Option<u8>,
    }

    #[derive(Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(deny_unknown_fields)]
    pub struct LaunchpadProScrollingTextDisplay;
}

mod osc {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use ts_rs::TS;

    #[derive(Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(rename_all = "snake_case")]
    #[serde(deny_unknown_fields)]
    pub struct OscSource {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub address: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub argument: Option<OscArgument>,
    }

    #[derive(Copy, Clone, Default, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(rename_all = "snake_case")]
    #[serde(deny_unknown_fields)]
    pub struct OscArgument {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub index: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub kind: Option<OscArgKind>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub relative: Option<bool>,
    }

    #[derive(Copy, Clone, Serialize, Deserialize, JsonSchema, TS)]
    pub enum OscArgKind {
        Float,
        Double,
        Bool,
        Nil,
        Inf,
        Int,
        String,
        Blob,
        Time,
        Long,
        Char,
        Color,
        Midi,
        Array,
    }

    impl Default for OscArgKind {
        fn default() -> Self {
            Self::Float
        }
    }
}

mod virt {
    use crate::infrastructure::api::schema::{VirtualControlElementId, VirtualControlElementKind};
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use ts_rs::TS;

    #[derive(Clone, Serialize, Deserialize, JsonSchema, TS)]
    #[serde(rename_all = "snake_case")]
    #[serde(deny_unknown_fields)]
    pub struct VirtualSource {
        pub id: VirtualControlElementId,
        pub kind: VirtualControlElementKind,
    }
}
