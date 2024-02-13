use crate::domain::{
    CompartmentKind, ExtendedProcessorContext, ReaperTarget, TargetSection, TargetTypeDef,
    UnresolvedReaperTargetDef, VirtualPlaytimeSlot, DEFAULT_TARGET,
};

use playtime_api::persistence::SlotAddress;
use realearn_api::persistence::PlaytimeSlotManagementAction;

#[derive(Debug)]
pub struct UnresolvedPlaytimeSlotManagementActionTarget {
    pub slot: VirtualPlaytimeSlot,
    pub action: PlaytimeSlotManagementAction,
}

impl UnresolvedReaperTargetDef for UnresolvedPlaytimeSlotManagementActionTarget {
    fn resolve(
        &self,
        context: ExtendedProcessorContext,
        compartment: CompartmentKind,
    ) -> Result<Vec<ReaperTarget>, &'static str> {
        let target = PlaytimeSlotManagementActionTarget {
            slot_address: self.slot.resolve(context, compartment)?,
            action: self.action.clone(),
        };
        Ok(vec![ReaperTarget::PlaytimeSlotManagementAction(target)])
    }

    fn clip_slot_descriptor(&self) -> Option<&VirtualPlaytimeSlot> {
        Some(&self.slot)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlaytimeSlotManagementActionTarget {
    pub slot_address: SlotAddress,
    pub action: PlaytimeSlotManagementAction,
}

pub const PLAYTIME_SLOT_MANAGEMENT_TARGET: TargetTypeDef = TargetTypeDef {
    lua_only: true,
    section: TargetSection::Playtime,
    name: "Slot management action",
    short_name: "Playtime slot management action",
    supports_clip_slot: true,
    ..DEFAULT_TARGET
};

#[cfg(not(feature = "playtime"))]
mod no_playtime_impl {
    use crate::domain::{ControlContext, PlaytimeSlotManagementActionTarget, RealearnTarget};
    use helgoboss_learn::Target;

    impl RealearnTarget for PlaytimeSlotManagementActionTarget {}
    impl<'a> Target<'a> for PlaytimeSlotManagementActionTarget {
        type Context = ControlContext<'a>;
    }
}

#[cfg(feature = "playtime")]
mod playtime_impl {
    use crate::domain::ui_util::convert_bool_to_unit_value;
    use crate::domain::{
        Backbone, CompoundChangeEvent, ControlContext, HitResponse, MappingControlContext,
        PlaytimeSlotManagementActionTarget, RealearnTarget, ReaperTargetType, TargetCharacter,
    };
    use helgoboss_learn::{AbsoluteValue, ControlType, ControlValue, PropValue, Target};
    use playtime_api::persistence::SlotAddress;
    use playtime_clip_engine::base::{ClipAddress, ClipMatrixEvent};
    use playtime_clip_engine::rt::{ClipChangeEvent, QualifiedClipChangeEvent};
    use realearn_api::persistence::PlaytimeSlotManagementAction;

    impl PlaytimeSlotManagementActionTarget {
        fn hit_internal(
            &mut self,
            value: ControlValue,
            context: MappingControlContext,
        ) -> anyhow::Result<HitResponse> {
            use PlaytimeSlotManagementAction as A;
            match &self.action {
                A::ClearSlot => {
                    if !value.is_on() {
                        return Ok(HitResponse::ignored());
                    }
                    self.with_matrix(context, |matrix| {
                        matrix.clear_slot(self.slot_address)?;
                        Ok(HitResponse::processed_with_effect())
                    })?
                }
                A::FillSlotWithSelectedItem => {
                    if !value.is_on() {
                        return Ok(HitResponse::ignored());
                    }
                    self.with_matrix(context, |matrix| {
                        matrix.import_selected_items(self.slot_address)?;
                        Ok(HitResponse::processed_with_effect())
                    })?
                }
                A::EditClip => self.with_matrix(context, |matrix| {
                    let clip_address = ClipAddress::new(self.slot_address, 0);
                    if value.is_on() {
                        matrix.start_editing_clip(clip_address)?;
                    } else {
                        matrix.stop_editing_clip(clip_address)?;
                    }
                    Ok(HitResponse::processed_with_effect())
                })?,
                A::AdjustClipSectionLength(a) => {
                    if !value.is_on() {
                        return Ok(HitResponse::ignored());
                    }
                    self.with_matrix(context, |matrix| {
                        matrix.adjust_slot_dynamic_section_length(self.slot_address, a.factor)?;
                        Ok(HitResponse::processed_with_effect())
                    })?
                }
                A::CopyOrPasteClip => {
                    if !value.is_on() {
                        return Ok(HitResponse::ignored());
                    }
                    self.with_matrix(context, |matrix| {
                        if matrix.slot_is_empty(self.slot_address) {
                            matrix.paste_slot(self.slot_address)?;
                        } else {
                            matrix.copy_slot(self.slot_address)?;
                        }
                        Ok(HitResponse::processed_with_effect())
                    })?
                }
                A::QuantizationOnOffState => self.with_matrix(context, |matrix| {
                    if value.is_on() {
                        matrix.quantize_clip(ClipAddress::new(self.slot_address, 0))?;
                    } else {
                        matrix.unquantize_clip(ClipAddress::new(self.slot_address, 0))?;
                    }
                    Ok(HitResponse::processed_with_effect())
                })?,
                A::Duplicate => {
                    if !value.is_on() {
                        return Ok(HitResponse::ignored());
                    }
                    self.with_matrix(context, |matrix| {
                        matrix.duplicate_slot(self.slot_address)?;
                        Ok(HitResponse::processed_with_effect())
                    })?
                }
                A::Double => {
                    if !value.is_on() {
                        return Ok(HitResponse::ignored());
                    }
                    self.with_matrix(context, |matrix| {
                        matrix.adjust_slot_dynamic_section_length(self.slot_address, 2.0)?;
                        Ok(HitResponse::processed_with_effect())
                    })?
                }
            }
        }

        fn with_matrix<R>(
            &self,
            context: MappingControlContext,
            f: impl FnOnce(&mut playtime_clip_engine::base::Matrix) -> R,
        ) -> anyhow::Result<R> {
            Backbone::get().with_clip_matrix_mut(context.control_context.instance(), f)
        }
    }

    impl RealearnTarget for PlaytimeSlotManagementActionTarget {
        fn control_type_and_character(&self, _: ControlContext) -> (ControlType, TargetCharacter) {
            use PlaytimeSlotManagementAction as A;
            match self.action {
                A::ClearSlot
                | A::FillSlotWithSelectedItem
                | A::CopyOrPasteClip
                | A::Duplicate
                | A::Double
                | A::AdjustClipSectionLength(_) => (
                    ControlType::AbsoluteContinuousRetriggerable,
                    TargetCharacter::Trigger,
                ),
                A::EditClip | A::QuantizationOnOffState => {
                    (ControlType::AbsoluteContinuous, TargetCharacter::Switch)
                }
            }
        }

        fn hit(
            &mut self,
            value: ControlValue,
            context: MappingControlContext,
        ) -> Result<HitResponse, &'static str> {
            self.hit_internal(value, context)
                .map_err(|_| "couldn't carry out clip management action")
        }

        fn reaper_target_type(&self) -> Option<ReaperTargetType> {
            Some(ReaperTargetType::PlaytimeSlotManagementAction)
        }

        // TODO-high-clip-engine Return clip as result of clip() function for all clip targets (just like track())
        //  and make this property available in all clip targets.
        // TODO-high-clip-engine Also add a "Clip" target, just like "Track" target
        fn prop_value(&self, key: &str, context: ControlContext) -> Option<PropValue> {
            match key {
                "clip.name" => Backbone::get()
                    .with_clip_matrix_mut(context.instance(), |matrix| {
                        let clip = matrix.find_slot(self.slot_address)?.clips().next()?;
                        let name = clip.name()?;
                        Some(PropValue::Text(name.to_string().into()))
                    })
                    .ok()?,
                _ => None,
            }
        }

        fn is_available(&self, _: ControlContext) -> bool {
            true
        }

        fn clip_slot_address(&self) -> Option<SlotAddress> {
            Some(self.slot_address)
        }

        fn process_change_event(
            &self,
            evt: CompoundChangeEvent,
            _context: ControlContext,
        ) -> (bool, Option<AbsoluteValue>) {
            match self.action {
                PlaytimeSlotManagementAction::QuantizationOnOffState => match evt {
                    CompoundChangeEvent::ClipMatrix(ClipMatrixEvent::ClipChanged(
                        QualifiedClipChangeEvent {
                            clip_address,
                            event: ClipChangeEvent::Content | ClipChangeEvent::Everything,
                        },
                    )) if clip_address.slot_address == self.slot_address => (true, None),
                    _ => (false, None),
                },
                _ => (false, None),
            }
        }
    }

    impl<'a> Target<'a> for PlaytimeSlotManagementActionTarget {
        type Context = ControlContext<'a>;

        fn current_value(&self, context: ControlContext<'a>) -> Option<AbsoluteValue> {
            use PlaytimeSlotManagementAction as A;
            match self.action {
                A::ClearSlot
                | A::FillSlotWithSelectedItem
                | A::CopyOrPasteClip
                | A::AdjustClipSectionLength(_)
                | A::Duplicate
                | A::Double => Some(AbsoluteValue::default()),
                A::EditClip => Backbone::get()
                    .with_clip_matrix(context.instance(), |matrix| {
                        let clip_address = ClipAddress::new(self.slot_address, 0);
                        let is_editing = matrix.is_editing_clip(clip_address);
                        let value = convert_bool_to_unit_value(is_editing);
                        Some(AbsoluteValue::Continuous(value))
                    })
                    .ok()?,
                A::QuantizationOnOffState => Backbone::get()
                    .with_clip_matrix(context.instance(), |matrix| {
                        let clip_address = ClipAddress::new(self.slot_address, 0);
                        let is_quantized = matrix.clip_is_quantized(clip_address).ok()?;
                        let value = convert_bool_to_unit_value(is_quantized);
                        Some(AbsoluteValue::Continuous(value))
                    })
                    .ok()?,
            }
        }

        fn control_type(&self, context: Self::Context) -> ControlType {
            self.control_type_and_character(context).0
        }
    }
}