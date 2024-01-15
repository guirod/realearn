use crate::application::{UnitModel, VirtualControlElementType, WeakUnitModel};
use crate::domain::{
    compartment_param_index_iter, Compartment, CompartmentParamIndex, CompartmentParams, MappingId,
    ReaperTargetType, TargetSection,
};
use crate::infrastructure::data::CommonPresetInfo;
use crate::infrastructure::plugin::{ActionSection, BackboneShell, ACTION_DEFS};
use crate::infrastructure::ui::Item;
use reaper_high::{FxChainContext, Reaper};
use std::iter;
use strum::IntoEnumIterator;
use swell_ui::menu_tree::{item, item_with_opts, menu, root_menu, Entry, ItemOpts, Menu};

pub fn extension_menu() -> Menu<&'static str> {
    let entries = ActionSection::iter()
        .filter_map(|section| {
            let items: Vec<_> = ACTION_DEFS
                .iter()
                .filter(|def| def.section == section && !def.developer)
                .map(|def| item(def.action_name, def.command_name))
                .collect();
            if items.is_empty() {
                return None;
            }
            let menu = menu(section.to_string(), items);
            Some(menu)
        })
        .collect();
    // let entries = vec![
    //     #[cfg(feature = "playtime")]
    //     menu(
    //         "Playtime",
    //         vec![item("Show/hide Playtime", "_HB_SHOW_HIDE_PLAYTIME")],
    //     ),
    // ];
    let mut menu = root_menu(vec![menu("Helgobox", entries)]);
    assign_command_ids(&mut menu);
    println!("{menu:#?}");
    menu
}

pub fn reaper_target_type_menu(current_value: ReaperTargetType) -> Menu<ReaperTargetType> {
    let entries = TargetSection::iter().map(|section| {
        let sub_entries = ReaperTargetType::iter()
            .filter(|t| t.definition().section == section)
            .map(|t| {
                item_with_opts(
                    t.definition().name,
                    ItemOpts {
                        enabled: true,
                        checked: t == current_value,
                    },
                    t,
                )
            });
        menu(section.to_string(), sub_entries.collect())
    });
    root_menu(entries.collect())
}

pub fn virtual_control_element_type_menu(
    current_value: VirtualControlElementType,
) -> Menu<VirtualControlElementType> {
    let entries = VirtualControlElementType::iter().map(|t| {
        item_with_opts(
            t.to_string(),
            ItemOpts {
                enabled: true,
                checked: t == current_value,
            },
            t,
        )
    });
    root_menu(entries.collect())
}

pub fn menu_containing_realearn_params(
    session: &WeakUnitModel,
    compartment: Compartment,
    current_value: CompartmentParamIndex,
) -> Menu<CompartmentParamIndex> {
    let session = session.upgrade().expect("session gone");
    let session = session.borrow();
    root_menu(
        compartment_param_index_iter()
            .map(|i| {
                let param_name = get_optional_param_name(&session, compartment, Some(i));
                item_with_opts(
                    param_name,
                    ItemOpts {
                        enabled: true,
                        checked: i == current_value,
                    },
                    i,
                )
            })
            .collect(),
    )
}

pub fn menu_containing_realearn_params_optional(
    session: &WeakUnitModel,
    compartment: Compartment,
    current_value: Option<CompartmentParamIndex>,
) -> Menu<Option<CompartmentParamIndex>> {
    let session = session.upgrade().expect("session gone");
    let session = session.borrow();
    root_menu(
        iter::once(item_with_opts(
            NONE,
            ItemOpts {
                enabled: true,
                checked: current_value.is_none(),
            },
            None,
        ))
        .chain(compartment_param_index_iter().map(|i| {
            let value = Some(i);
            let param_name = get_optional_param_name(&session, compartment, value);
            item_with_opts(
                param_name,
                ItemOpts {
                    enabled: true,
                    checked: value == current_value,
                },
                value,
            )
        }))
        .collect(),
    )
}

pub fn menu_containing_mappings(
    session: &WeakUnitModel,
    compartment: Compartment,
    current_value: Option<MappingId>,
) -> Menu<Option<MappingId>> {
    let session = session.upgrade().expect("session gone");
    let session = session.borrow();
    let none_item = item_with_opts(
        NONE,
        ItemOpts {
            enabled: true,
            checked: current_value.is_none(),
        },
        None,
    );
    let group_items = session.groups_sorted(compartment).map(|group| {
        let group = group.borrow();
        let group_id = group.id();
        menu(
            group.effective_name(),
            session
                .mappings(compartment)
                .filter_map(|mapping| {
                    // If borrowing fails, we know it's our own mapping
                    let mapping = mapping.try_borrow().ok()?;
                    if mapping.group_id() != group_id {
                        return None;
                    }
                    let mapping_id = mapping.id();
                    let menu_item = item_with_opts(
                        mapping.effective_name(),
                        ItemOpts {
                            enabled: true,
                            checked: Some(mapping_id) == current_value,
                        },
                        Some(mapping_id),
                    );
                    Some(menu_item)
                })
                .collect(),
        )
    });
    root_menu(iter::once(none_item).chain(group_items).collect())
}

pub fn menu_containing_sessions(
    this_session: &UnitModel,
    current_other_session_id: Option<&str>,
) -> Menu<Option<String>> {
    let this_item = item_with_opts(
        THIS,
        ItemOpts {
            enabled: true,
            checked: current_other_session_id.is_none(),
        },
        None,
    );
    let items: Vec<_> = BackboneShell::get().with_unit_infos(|sessions| {
        let instance_items = sessions.iter().filter_map(|session| {
            let other_session = session.unit_model.upgrade()?;
            let other_session = other_session.try_borrow().ok()?;
            // Exclude certain sessions
            if other_session.unit_id() == this_session.unit_id() {
                // Don't include our own session.
                return None;
            }
            let this_chain_context = this_session
                .processor_context()
                .containing_fx()
                .chain()
                .context();
            let other_chain_context = other_session
                .processor_context()
                .containing_fx()
                .chain()
                .context();
            match (this_chain_context, other_chain_context) {
                // It's okay if this instance is on the monitoring FX chain and the other one
                // as well (global => global).
                (FxChainContext::Monitoring, FxChainContext::Monitoring) => {}
                // It's okay if this instance is in a specific project and the other one on the
                // monitoring FX chain (local => global).
                (FxChainContext::Track { .. }, FxChainContext::Monitoring) => {}
                // It's okay if this instance is in a specific project and the other one in the same
                // project.
                (
                    FxChainContext::Track {
                        track: this_track, ..
                    },
                    FxChainContext::Track {
                        track: other_track, ..
                    },
                ) if other_track.project() == this_track.project() => {}
                // All other combinations are not allowed!
                _ => return None,
            }
            // Build item
            let other_session_id = other_session.unit_key().to_string();
            let item = item_with_opts(
                other_session.to_string(),
                ItemOpts {
                    enabled: true,
                    checked: Some(other_session_id.as_str()) == current_other_session_id,
                },
                Some(other_session_id),
            );
            Some(item)
        });
        iter::once(this_item).chain(instance_items).collect()
    });
    root_menu(items)
}

pub fn menu_containing_banks(
    session: &WeakUnitModel,
    compartment: Compartment,
    param_index: CompartmentParamIndex,
    current_value: u32,
) -> Menu<u32> {
    let session = session.upgrade().expect("session gone");
    let session = session.borrow();
    let bank_param = session
        .params()
        .compartment_params(compartment)
        .at(param_index);
    let menu_items = if let Some(discrete_values) = bank_param.setting().discrete_values() {
        discrete_values
            .enumerate()
            // Don't block GUI if we come across a parameter that has insanely many
            // discrete values (and is probably not intended to be used with banks).
            .take(500)
            .map(|(i, s)| bank_item(s.to_string(), i, current_value))
            .collect()
    } else {
        // For continuous parameters we just choose a default of 100 values.
        let bank_count = 100;
        (0..bank_count)
            .map(|i| bank_item(i.to_string(), i, current_value))
            .collect()
    };
    root_menu(menu_items)
}

pub fn get_optional_param_name(
    session: &UnitModel,
    compartment: Compartment,
    index: Option<CompartmentParamIndex>,
) -> String {
    match index {
        None => "<None>".to_owned(),
        Some(i) => {
            let params = session.params().compartment_params(compartment);
            get_param_name(params, i)
        }
    }
}

pub fn get_param_name(params: &CompartmentParams, index: CompartmentParamIndex) -> String {
    let param_name = params.get_parameter_name(index);
    format!("{}. {}", index.get() + 1, param_name)
}

pub fn get_bank_name(
    session: &UnitModel,
    item: &dyn Item,
    bank_param_index: CompartmentParamIndex,
    bank_index: u32,
) -> String {
    let bank_param = session
        .params()
        .compartment_params(item.compartment())
        .at(bank_param_index);
    if let Some(label) = bank_param.setting().find_label_for_value(bank_index) {
        label.to_owned()
    } else {
        bank_index.to_string()
    }
}

fn bank_item(text: String, bank_index: usize, current_bank_index: u32) -> Entry<u32> {
    item_with_opts(
        text,
        ItemOpts {
            enabled: true,
            checked: bank_index == current_bank_index as usize,
        },
        bank_index as u32,
    )
}

pub fn menu_containing_compartment_presets(
    compartment: Compartment,
    current_value: Option<&str>,
) -> Menu<Option<String>> {
    let preset_manager = BackboneShell::get().compartment_preset_manager(compartment);
    let preset_manager = preset_manager.borrow();
    root_menu(
        iter::once(item_with_opts(
            NONE,
            ItemOpts {
                enabled: true,
                checked: current_value.is_none(),
            },
            None,
        ))
        .chain(build_compartment_preset_menu_entries(
            preset_manager.common_preset_infos(),
            |info| Some(info.id.clone()),
            |info| current_value.is_some_and(|id| id == &info.id),
        ))
        .collect(),
    )
}

pub fn build_compartment_preset_menu_entries<'a, T: 'static>(
    preset_infos: impl Iterator<Item = &'a CommonPresetInfo> + 'a,
    build_id: impl Fn(&CommonPresetInfo) -> T + 'a,
    is_current_value: impl Fn(&CommonPresetInfo) -> bool + 'a,
) -> impl Iterator<Item = Entry<T>> + 'a {
    let (user_preset_infos, factory_preset_infos): (Vec<_>, Vec<_>) =
        preset_infos.partition(|info| info.origin.is_user());
    [
        ("User presets", user_preset_infos),
        ("Factory presets", factory_preset_infos),
    ]
    .into_iter()
    .map(move |(label, mut infos)| {
        infos.sort_by_key(|info| &info.meta_data.name);
        menu(
            label,
            build_compartment_preset_menu_entries_internal(
                infos.into_iter(),
                &build_id,
                &is_current_value,
            )
            .collect(),
        )
    })
}

fn build_compartment_preset_menu_entries_internal<'a, T: 'static>(
    preset_infos: impl Iterator<Item = &'a CommonPresetInfo> + 'a,
    build_id: &'a (impl Fn(&CommonPresetInfo) -> T + 'a),
    is_current_value: &'a (impl Fn(&CommonPresetInfo) -> bool + 'a),
) -> impl Iterator<Item = Entry<T>> + 'a {
    preset_infos.map(move |info| {
        let id = build_id(info);
        let label = if info.meta_data.name == info.id {
            info.meta_data.name.clone()
        } else {
            format!("{} ({})", info.meta_data.name, info.id)
        };
        item_with_opts(
            label,
            ItemOpts {
                enabled: true,
                checked: is_current_value(info),
            },
            id,
        )
    })
}

pub const NONE: &str = "<None>";
pub const THIS: &str = "<This>";

/// Interprets the item payloads as REAPER command names and uses them to lookup the corresponding REAPER command
/// IDs.
///
/// This is useful for top-level menus.
pub fn assign_command_ids(menu: &mut Menu<&'static str>) {
    for e in &mut menu.entries {
        assign_command_ids_recursively(e);
    }
}

fn assign_command_ids_recursively(entry: &mut Entry<&'static str>) {
    match entry {
        Entry::Menu(m) => {
            m.id = 0;
            for e in &mut m.entries {
                assign_command_ids_recursively(e);
            }
        }
        Entry::Item(i) => {
            let command_id = Reaper::get()
                .medium_reaper()
                .named_command_lookup(format!("_{}", i.result));
            i.id = command_id.map(|id| id.get()).unwrap_or(0);
        }
        _ => {}
    }
}
