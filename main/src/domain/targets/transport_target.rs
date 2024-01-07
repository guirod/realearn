use crate::domain::{
    format_value_as_on_off, transport_is_enabled_unit_value, AdditionalFeedbackEvent, Compartment,
    CompoundChangeEvent, ControlContext, ExtendedProcessorContext, FeedbackResolution, HitResponse,
    MappingControlContext, RealearnTarget, ReaperTarget, ReaperTargetType, TargetCharacter,
    TargetSection, TargetTypeDef, TransportAction, UnresolvedReaperTargetDef, DEFAULT_TARGET,
};
use helgoboss_learn::{AbsoluteValue, ControlType, ControlValue, Target, UnitValue};
use reaper_high::{ChangeEvent, Project, Reaper};
use std::borrow::Cow;

#[derive(Debug)]
pub struct UnresolvedTransportTarget {
    pub action: TransportAction,
}

impl UnresolvedReaperTargetDef for UnresolvedTransportTarget {
    fn resolve(
        &self,
        context: ExtendedProcessorContext,
        _: Compartment,
    ) -> Result<Vec<ReaperTarget>, &'static str> {
        Ok(vec![ReaperTarget::Transport(TransportTarget {
            project: context.context().project_or_current_project(),
            action: self.action,
        })])
    }

    fn feedback_resolution(&self) -> Option<FeedbackResolution> {
        Some(FeedbackResolution::Beat)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransportTarget {
    pub project: Project,
    pub action: TransportAction,
}

impl RealearnTarget for TransportTarget {
    fn control_type_and_character(&self, _: ControlContext) -> (ControlType, TargetCharacter) {
        self.action.control_type_and_character()
    }

    fn format_value(&self, value: UnitValue, _: ControlContext) -> String {
        format_value_as_on_off(value).to_string()
    }

    fn hit(
        &mut self,
        value: ControlValue,
        _: MappingControlContext,
    ) -> Result<HitResponse, &'static str> {
        use TransportAction::*;
        let on = !value.to_unit_value()?.is_zero();
        let response = match self.action {
            PlayStop => {
                if on {
                    self.project.play();
                } else {
                    self.project.stop();
                }
                HitResponse::processed_with_effect()
            }
            PlayPause => {
                if on {
                    self.project.play();
                } else {
                    self.project.pause();
                }
                HitResponse::processed_with_effect()
            }
            Stop => {
                if on {
                    self.project.stop();
                    HitResponse::processed_with_effect()
                } else {
                    HitResponse::ignored()
                }
            }
            Pause => {
                if on {
                    self.project.pause();
                    HitResponse::processed_with_effect()
                } else {
                    HitResponse::ignored()
                }
            }
            RecordStop => {
                if on {
                    Reaper::get().enable_record_in_current_project();
                } else {
                    Reaper::get().disable_record_in_current_project();
                }
                HitResponse::processed_with_effect()
            }
            Repeat => {
                if on {
                    self.project.enable_repeat();
                } else {
                    self.project.disable_repeat();
                }
                HitResponse::processed_with_effect()
            }
        };
        Ok(response)
    }

    fn is_available(&self, _: ControlContext) -> bool {
        self.project.is_available()
    }

    fn project(&self) -> Option<Project> {
        Some(self.project)
    }

    fn process_change_event(
        &self,
        evt: CompoundChangeEvent,
        _: ControlContext,
    ) -> (bool, Option<AbsoluteValue>) {
        match evt {
            CompoundChangeEvent::Reaper(evt) => {
                use ChangeEvent::*;
                use TransportAction::*;
                match self.action {
                    PlayStop | PlayPause => match evt {
                        PlayStateChanged(e) if e.project == self.project => (
                            true,
                            Some(AbsoluteValue::Continuous(transport_is_enabled_unit_value(
                                e.new_value.is_playing,
                            ))),
                        ),
                        _ => (false, None),
                    },
                    Stop => match evt {
                        PlayStateChanged(e) if e.project == self.project => (
                            true,
                            Some(AbsoluteValue::Continuous(transport_is_enabled_unit_value(
                                !e.new_value.is_playing && !e.new_value.is_paused,
                            ))),
                        ),
                        _ => (false, None),
                    },
                    Pause => match evt {
                        PlayStateChanged(e) if e.project == self.project => (
                            true,
                            Some(AbsoluteValue::Continuous(transport_is_enabled_unit_value(
                                e.new_value.is_paused,
                            ))),
                        ),
                        _ => (false, None),
                    },
                    RecordStop => match evt {
                        PlayStateChanged(e) if e.project == self.project => (
                            true,
                            Some(AbsoluteValue::Continuous(transport_is_enabled_unit_value(
                                e.new_value.is_recording,
                            ))),
                        ),
                        _ => (false, None),
                    },
                    Repeat => match evt {
                        RepeatStateChanged(e) if e.project == self.project => (
                            true,
                            Some(AbsoluteValue::Continuous(transport_is_enabled_unit_value(
                                e.new_value,
                            ))),
                        ),
                        _ => (false, None),
                    },
                }
            }
            CompoundChangeEvent::Additional(AdditionalFeedbackEvent::BeatChanged(e))
                if self.action != TransportAction::Repeat
                    && e.project == self.project
                    && e.project != Reaper::get().current_project() =>
            {
                (true, None)
            }
            _ => (false, None),
        }
    }

    fn text_value(&self, context: ControlContext) -> Option<Cow<'static, str>> {
        Some(format_value_as_on_off(self.current_value(context)?.to_unit_value()).into())
    }

    fn reaper_target_type(&self) -> Option<ReaperTargetType> {
        Some(ReaperTargetType::Transport)
    }
}

impl<'a> Target<'a> for TransportTarget {
    type Context = ControlContext<'a>;

    fn current_value(&self, _: Self::Context) -> Option<AbsoluteValue> {
        use TransportAction::*;
        let play_state = self.project.play_state();
        let value = match self.action {
            PlayStop | PlayPause => transport_is_enabled_unit_value(play_state.is_playing),
            Stop => {
                transport_is_enabled_unit_value(!play_state.is_playing && !play_state.is_paused)
            }
            Pause => transport_is_enabled_unit_value(play_state.is_paused),
            RecordStop => transport_is_enabled_unit_value(play_state.is_recording),
            Repeat => transport_is_enabled_unit_value(self.project.repeat_is_enabled()),
        };
        Some(AbsoluteValue::Continuous(value))
    }

    fn control_type(&self, context: Self::Context) -> ControlType {
        self.control_type_and_character(context).0
    }
}

pub const TRANSPORT_TARGET: TargetTypeDef = TargetTypeDef {
    section: TargetSection::Project,
    name: "Invoke transport action",
    short_name: "Transport",
    ..DEFAULT_TARGET
};
