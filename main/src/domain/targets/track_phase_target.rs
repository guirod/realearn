use crate::domain::ui_util::convert_bool_to_unit_value;
use crate::domain::{
    change_track_prop, format_value_as_on_off,
    get_control_type_and_character_for_track_exclusivity, get_effective_tracks, CompartmentKind,
    ControlContext, ExtendedProcessorContext, FeedbackResolution, HitResponse,
    MappingControlContext, RealearnTarget, ReaperTarget, ReaperTargetType, TargetCharacter,
    TargetSection, TargetTypeDef, TrackDescriptor, TrackExclusivity, TrackGangBehavior,
    UnresolvedReaperTargetDef, DEFAULT_TARGET,
};
use helgoboss_learn::{AbsoluteValue, ControlType, ControlValue, Target, UnitValue};
use reaper_high::{Project, Track};
use reaper_medium::TrackPolarity;
use std::borrow::Cow;

#[derive(Debug)]
pub struct UnresolvedTrackPhaseTarget {
    pub track_descriptor: TrackDescriptor,
    pub exclusivity: TrackExclusivity,
    pub gang_behavior: TrackGangBehavior,
    pub poll_for_feedback: bool,
}

impl UnresolvedReaperTargetDef for UnresolvedTrackPhaseTarget {
    fn resolve(
        &self,
        context: ExtendedProcessorContext,
        compartment: CompartmentKind,
    ) -> Result<Vec<ReaperTarget>, &'static str> {
        Ok(
            get_effective_tracks(context, &self.track_descriptor.track, compartment)?
                .into_iter()
                .map(|track| {
                    ReaperTarget::TrackPhase(TrackPhaseTarget {
                        track,
                        exclusivity: self.exclusivity,
                        gang_behavior: self.gang_behavior,
                        poll_for_feedback: self.poll_for_feedback,
                    })
                })
                .collect(),
        )
    }

    fn track_descriptor(&self) -> Option<&TrackDescriptor> {
        Some(&self.track_descriptor)
    }

    fn feedback_resolution(&self) -> Option<FeedbackResolution> {
        if self.poll_for_feedback {
            Some(FeedbackResolution::High)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackPhaseTarget {
    pub track: Track,
    pub exclusivity: TrackExclusivity,
    pub gang_behavior: TrackGangBehavior,
    pub poll_for_feedback: bool,
}

impl RealearnTarget for TrackPhaseTarget {
    fn control_type_and_character(&self, _: ControlContext) -> (ControlType, TargetCharacter) {
        get_control_type_and_character_for_track_exclusivity(self.exclusivity)
    }

    fn format_value(&self, value: UnitValue, _: ControlContext) -> String {
        format_value_as_on_off(value).to_string()
    }

    fn hit(
        &mut self,
        value: ControlValue,
        _: MappingControlContext,
    ) -> Result<HitResponse, &'static str> {
        let (gang_behavior, grouping_behavior) = self.gang_behavior.gang_and_grouping_behavior();
        change_track_prop(
            &self.track,
            self.exclusivity,
            value.to_unit_value()?,
            |t| t.set_phase_inverted(TrackPolarity::Inverted, gang_behavior, grouping_behavior),
            |t| t.set_phase_inverted(TrackPolarity::Normal, gang_behavior, grouping_behavior),
        );
        Ok(HitResponse::processed_with_effect())
    }

    fn is_available(&self, _: ControlContext) -> bool {
        self.track.is_available()
    }

    fn project(&self) -> Option<Project> {
        Some(self.track.project())
    }

    fn track(&self) -> Option<&Track> {
        Some(&self.track)
    }

    fn track_exclusivity(&self) -> Option<TrackExclusivity> {
        Some(self.exclusivity)
    }

    fn supports_automatic_feedback(&self) -> bool {
        self.poll_for_feedback
    }

    fn text_value(&self, context: ControlContext) -> Option<Cow<'static, str>> {
        Some(format_value_as_on_off(self.current_value(context)?.to_unit_value()).into())
    }

    fn reaper_target_type(&self) -> Option<ReaperTargetType> {
        Some(ReaperTargetType::TrackPhase)
    }
}

impl<'a> Target<'a> for TrackPhaseTarget {
    type Context = ControlContext<'a>;

    fn current_value(&self, _: Self::Context) -> Option<AbsoluteValue> {
        let val = convert_bool_to_unit_value(self.track.phase_is_inverted());
        Some(AbsoluteValue::Continuous(val))
    }

    fn control_type(&self, context: Self::Context) -> ControlType {
        self.control_type_and_character(context).0
    }
}

pub const TRACK_PHASE_TARGET: TargetTypeDef = TargetTypeDef {
    section: TargetSection::Track,
    name: "Phase invert/normal",
    short_name: "Track phase",
    hint: "Ganging/grouping support from REAPER v6.70",
    supports_track: true,
    supports_track_exclusivity: true,
    supports_gang_selected: true,
    supports_gang_grouping: true,
    supports_track_grouping_only_gang_behavior: true,
    supports_poll_for_feedback: true,
    ..DEFAULT_TARGET
};
