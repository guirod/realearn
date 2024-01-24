use crate::domain::{
    format_value_as_on_off, CompartmentKind, CompoundChangeEvent, ControlContext, DomainEvent,
    Exclusivity, ExtendedProcessorContext, HitInstruction, HitInstructionContext,
    HitInstructionResponse, HitResponse, MappingControlContext, MappingData,
    MappingEnabledChangeRequestedEvent, RealearnTarget, ReaperTarget, ReaperTargetType, TagScope,
    TargetCharacter, TargetSection, TargetTypeDef, UnitEvent, UnresolvedReaperTargetDef,
    DEFAULT_TARGET,
};
use helgoboss_learn::{AbsoluteValue, ControlType, ControlValue, Target, UnitValue};
use std::borrow::Cow;
use std::collections::HashSet;

#[derive(Debug)]
pub struct UnresolvedEnableMappingsTarget {
    pub compartment: CompartmentKind,
    pub scope: TagScope,
    pub exclusivity: Exclusivity,
}

impl UnresolvedReaperTargetDef for UnresolvedEnableMappingsTarget {
    fn resolve(
        &self,
        _: ExtendedProcessorContext,
        _: CompartmentKind,
    ) -> Result<Vec<ReaperTarget>, &'static str> {
        Ok(vec![ReaperTarget::EnableMappings(EnableMappingsTarget {
            compartment: self.compartment,
            scope: self.scope.clone(),
            exclusivity: self.exclusivity,
        })])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnableMappingsTarget {
    /// This must always correspond to the compartment of the containing mapping, otherwise it will
    /// lead to strange behavior.
    pub compartment: CompartmentKind,
    pub scope: TagScope,
    pub exclusivity: Exclusivity,
}

impl RealearnTarget for EnableMappingsTarget {
    fn control_type_and_character(&self, _: ControlContext) -> (ControlType, TargetCharacter) {
        (
            ControlType::AbsoluteContinuousRetriggerable,
            TargetCharacter::Switch,
        )
    }

    fn hit(
        &mut self,
        value: ControlValue,
        context: MappingControlContext,
    ) -> Result<HitResponse, &'static str> {
        let value = value.to_unit_value()?;
        let is_enable = !value.is_zero();
        struct EnableMappingsInstruction {
            compartment: CompartmentKind,
            scope: TagScope,
            mapping_data: MappingData,
            is_enable: bool,
            exclusivity: Exclusivity,
        }
        impl HitInstruction for EnableMappingsInstruction {
            fn execute(self: Box<Self>, context: HitInstructionContext) -> HitInstructionResponse {
                let mut activated_inverse_tags = HashSet::new();
                for m in context.mappings.values_mut() {
                    // Don't touch ourselves.
                    if m.id() == self.mapping_data.mapping_id {
                        continue;
                    }
                    // Determine how to change the mappings.
                    let flag = match self.scope.determine_enable_disable_change(
                        self.exclusivity,
                        m.tags(),
                        self.is_enable,
                    ) {
                        None => continue,
                        Some(f) => f,
                    };
                    if self.exclusivity == Exclusivity::Exclusive && !self.is_enable {
                        // Collect all *other* mapping tags because they are going to be activated
                        // and we have to know about them!
                        activated_inverse_tags.extend(m.tags().iter().cloned());
                    }
                    // Finally request change of mapping enabled state!
                    context.domain_event_handler.handle_event_ignoring_error(
                        DomainEvent::MappingEnabledChangeRequested(
                            MappingEnabledChangeRequestedEvent {
                                compartment: m.compartment(),
                                mapping_id: m.id(),
                                is_enabled: if self.is_enable { flag } else { !flag },
                            },
                        ),
                    );
                }
                let mut instance_state = context.control_context.unit.borrow_mut();
                use Exclusivity::*;
                if self.exclusivity == Exclusive
                    || (self.exclusivity == ExclusiveOnOnly && self.is_enable)
                {
                    // Completely replace
                    let new_active_tags = if self.is_enable {
                        self.scope.tags.clone()
                    } else {
                        activated_inverse_tags
                    };
                    instance_state.set_active_mapping_tags(self.compartment, new_active_tags);
                } else {
                    // Add or remove
                    instance_state.activate_or_deactivate_mapping_tags(
                        self.compartment,
                        &self.scope.tags,
                        self.is_enable,
                    );
                }
                HitInstructionResponse::CausedEffect(vec![])
            }
        }
        let instruction = EnableMappingsInstruction {
            compartment: self.compartment,
            // So far this clone is okay because enabling/disable mappings is not something that
            // happens every few milliseconds. No need to use a ref to this target.
            scope: self.scope.clone(),
            mapping_data: context.mapping_data,
            is_enable,
            exclusivity: self.exclusivity,
        };
        Ok(HitResponse::hit_instruction(Box::new(instruction)))
    }

    fn is_available(&self, _: ControlContext) -> bool {
        true
    }

    fn process_change_event(
        &self,
        evt: CompoundChangeEvent,
        _: ControlContext,
    ) -> (bool, Option<AbsoluteValue>) {
        match evt {
            CompoundChangeEvent::Unit(UnitEvent::ActiveMappingTags { compartment, .. })
                if *compartment == self.compartment =>
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
        Some(ReaperTargetType::EnableMappings)
    }
}

impl<'a> Target<'a> for EnableMappingsTarget {
    type Context = ControlContext<'a>;

    fn current_value(&self, context: Self::Context) -> Option<AbsoluteValue> {
        let instance_state = context.unit.borrow();
        use Exclusivity::*;
        let active = match self.exclusivity {
            NonExclusive => instance_state
                .at_least_those_mapping_tags_are_active(self.compartment, &self.scope.tags),
            Exclusive | ExclusiveOnOnly => instance_state
                .only_these_mapping_tags_are_active(self.compartment, &self.scope.tags),
        };
        let uv = if active {
            UnitValue::MAX
        } else {
            UnitValue::MIN
        };
        Some(AbsoluteValue::Continuous(uv))
    }

    fn control_type(&self, context: Self::Context) -> ControlType {
        self.control_type_and_character(context).0
    }
}

pub const ENABLE_MAPPINGS_TARGET: TargetTypeDef = TargetTypeDef {
    section: TargetSection::ReaLearn,
    name: "Enable/disable mappings",
    short_name: "Enable/disable mappings",
    supports_tags: true,
    supports_exclusivity: true,
    ..DEFAULT_TARGET
};
