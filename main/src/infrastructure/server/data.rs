//! Contains the actual application interface and implementation without any HTTP-specific stuff.

use crate::application::{
    ControllerPreset, Preset, PresetManager, SourceCategory, TargetCategory, UnitModel,
};
use crate::domain::{Compartment, MappingKey, ProjectionFeedbackValue};
use crate::infrastructure::data::{ControllerPresetData, PresetData};
use crate::infrastructure::plugin::BackboneShell;
use helgoboss_learn::UnitValue;
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::rc::Rc;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
// Right now just a placeholder
pub struct SessionResponseData {}

pub enum DataError {
    SessionNotFound,
    SessionHasNoActiveController,
    ControllerNotFound,
    OnlyPatchReplaceIsSupported,
    OnlyCustomDataKeyIsSupportedAsPatchPath,
    ControllerUpdateFailed,
}

pub enum DataErrorCategory {
    NotFound,
    BadRequest,
    MethodNotAllowed,
    InternalServerError,
}

impl DataError {
    pub fn description(&self) -> &'static str {
        use DataError::*;
        match self {
            SessionNotFound => "session not found",
            SessionHasNoActiveController => "session doesn't have an active controller",
            ControllerNotFound => "session has controller but controller not found",
            OnlyPatchReplaceIsSupported => "only 'replace' is supported as op",
            OnlyCustomDataKeyIsSupportedAsPatchPath => {
                "only '/customData/{key}' is supported as path"
            }
            ControllerUpdateFailed => "couldn't update controller",
        }
    }

    pub fn category(&self) -> DataErrorCategory {
        use DataError::*;
        match self {
            SessionNotFound | SessionHasNoActiveController | ControllerNotFound => {
                DataErrorCategory::NotFound
            }
            OnlyPatchReplaceIsSupported => DataErrorCategory::MethodNotAllowed,
            OnlyCustomDataKeyIsSupportedAsPatchPath => DataErrorCategory::BadRequest,
            ControllerUpdateFailed => DataErrorCategory::InternalServerError,
        }
    }
}

#[derive(Deserialize)]
pub struct PatchRequest {
    op: PatchRequestOp,
    path: String,
    value: serde_json::value::Value,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum PatchRequestOp {
    Replace,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerRouting {
    main_preset: Option<LightMainPresetData>,
    routes: HashMap<MappingKey, Vec<TargetDescriptor>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LightMainPresetData {
    id: String,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TargetDescriptor {
    label: String,
}

pub fn get_session_data(session_id: String) -> Result<SessionResponseData, DataError> {
    let _ = BackboneShell::get()
        .find_session_by_id(&session_id)
        .ok_or(DataError::SessionNotFound)?;
    Ok(SessionResponseData {})
}

pub fn get_controller_routing_by_session_id(
    session_id: String,
) -> Result<ControllerRouting, DataError> {
    let session = BackboneShell::get()
        .find_session_by_id(&session_id)
        .ok_or(DataError::SessionNotFound)?;
    let routing = get_controller_routing(&session.borrow());
    Ok(routing)
}

pub fn get_controller_preset_data(session_id: String) -> Result<ControllerPresetData, DataError> {
    let session = BackboneShell::get()
        .find_session_by_id(&session_id)
        .ok_or(DataError::SessionNotFound)?;
    let session = session.borrow();
    get_controller_preset_data_internal(&session)
}

pub fn get_controller_routing(session: &UnitModel) -> ControllerRouting {
    let main_preset = session.active_main_preset().map(|mp| LightMainPresetData {
        id: mp.id().to_string(),
        name: mp.name().to_string(),
    });
    let instance_state = session.unit().borrow();
    let routes = session
        .mappings(Compartment::Controller)
        .filter_map(|m| {
            let m = m.borrow();
            if !m.visible_in_projection() {
                return None;
            }
            let target_descriptor = if instance_state.mapping_is_on(m.qualified_id()) {
                if m.target_model.category() == TargetCategory::Virtual {
                    // Virtual
                    let control_element = m.target_model.create_control_element();
                    let matching_main_mappings = session.mappings(Compartment::Main).filter(|mp| {
                        let mp = mp.borrow();
                        mp.visible_in_projection()
                            && mp.source_model.category() == SourceCategory::Virtual
                            && mp.source_model.create_control_element() == control_element
                            && instance_state.mapping_is_on(mp.qualified_id())
                    });
                    let descriptors: Vec<_> = matching_main_mappings
                        .map(|m| {
                            let m = m.borrow();
                            TargetDescriptor {
                                label: m.effective_name(),
                            }
                        })
                        .collect();
                    if descriptors.is_empty() {
                        return None;
                    }
                    descriptors
                } else {
                    // Direct
                    let single_descriptor = TargetDescriptor {
                        label: m.effective_name(),
                    };
                    vec![single_descriptor]
                }
            } else {
                return None;
            };
            Some((m.key().clone(), target_descriptor))
        })
        .collect();
    ControllerRouting {
        main_preset,
        routes,
    }
}

pub fn patch_controller(controller_id: String, req: PatchRequest) -> Result<(), DataError> {
    if req.op != PatchRequestOp::Replace {
        return Err(DataError::OnlyPatchReplaceIsSupported);
    }
    let split_path: Vec<_> = req.path.split('/').collect();
    let custom_data_key = if let ["", "customData", key] = split_path.as_slice() {
        key
    } else {
        return Err(DataError::OnlyCustomDataKeyIsSupportedAsPatchPath);
    };
    // Update the global controller preset.
    let controller_manager = BackboneShell::get().controller_preset_manager();
    let mut controller_manager = controller_manager.borrow_mut();
    let mut controller_preset = controller_manager
        .find_by_id(&controller_id)
        .ok_or(DataError::ControllerNotFound)?;
    controller_preset.update_custom_data(custom_data_key.to_string(), req.value.clone());
    controller_manager
        .update_preset(controller_preset)
        .map_err(|_| DataError::ControllerUpdateFailed)?;
    // Update all sessions which use this preset. If we don't do that, the Companion app will not
    // get the saved changes - they will just disappear (#591) - because we made a change in
    // v1.13.0-pre.4 that /realearn/session/.../controller queries the session, not the global
    // controller preset.
    // TODO-low In future versions of the Companion app, we should not update the controller
    //  source directly but update a session. This makes more sense because now ReaLearn treats
    //  custom data exactly like mappings - it's saved with the session.
    BackboneShell::get().with_instances(|instances| {
        let sessions = instances.iter().filter_map(|s| s.unit_model.upgrade());
        for session in sessions {
            let mut session = session.borrow_mut();
            session.update_custom_compartment_data(
                Compartment::Controller,
                custom_data_key.to_string(),
                req.value.clone(),
            );
        }
    });
    Ok(())
}

#[derive(Deserialize)]
pub struct WebSocketRequest {
    pub topics: String,
}

impl WebSocketRequest {
    pub fn parse_topics(&self) -> Topics {
        self.topics.split(',').flat_map(Topic::try_from).collect()
    }
}

pub type Topics = HashSet<Topic>;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum Topic {
    Session { session_id: String },
    ActiveController { session_id: String },
    ControllerRouting { session_id: String },
    Feedback { session_id: String },
}

impl TryFrom<&str> for Topic {
    type Error = &'static str;

    fn try_from(topic_expression: &str) -> Result<Self, Self::Error> {
        let topic_segments: Vec<_> = topic_expression.split('/').skip(1).collect();
        let topic = match topic_segments.as_slice() {
            ["realearn", "session", id, "controller-routing"] => Topic::ControllerRouting {
                session_id: id.to_string(),
            },
            ["realearn", "session", id, "controller"] => Topic::ActiveController {
                session_id: id.to_string(),
            },
            ["realearn", "session", id, "feedback"] => Topic::Feedback {
                session_id: id.to_string(),
            },
            ["realearn", "session", id] => Topic::Session {
                session_id: id.to_string(),
            },
            _ => return Err("invalid topic expression"),
        };
        Ok(topic)
    }
}

pub fn send_initial_feedback(session_id: &str) {
    if let Some(session) = BackboneShell::get().find_session_by_id(session_id) {
        session.borrow_mut().send_all_feedback();
    }
}

pub fn get_active_controller_updated_event(
    session_id: &str,
    session: Option<&UnitModel>,
) -> Event<Option<ControllerPresetData>> {
    Event::put(
        format!("/realearn/session/{session_id}/controller"),
        session.and_then(get_controller),
    )
}

pub fn get_projection_feedback_event(
    session_id: &str,
    feedback_value: ProjectionFeedbackValue,
) -> Event<HashMap<Rc<str>, UnitValue>> {
    Event::patch(
        format!("/realearn/session/{session_id}/feedback"),
        hashmap! {
            feedback_value.mapping_key => feedback_value.value
        },
    )
}

pub fn get_session_updated_event(
    session_id: &str,
    session_data: Option<SessionResponseData>,
) -> Event<Option<SessionResponseData>> {
    Event::put(format!("/realearn/session/{session_id}"), session_data)
}

pub fn get_controller_routing_updated_event(
    session_id: &str,
    session: Option<&UnitModel>,
) -> Event<Option<ControllerRouting>> {
    Event::put(
        format!("/realearn/session/{session_id}/controller-routing"),
        session.map(get_controller_routing),
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event<T> {
    /// Roughly corresponds to the HTTP method of the resource.
    r#type: EventType,
    /// Corresponds to the HTTP path of the resource.
    path: String,
    /// Corresponds to the HTTP body.
    ///
    /// HTTP 404 corresponds to this value being `null` or undefined in JSON. If this is not enough
    /// in future use cases, we can still add another field that resembles the HTTP status.
    body: T,
}

impl<T> Event<T> {
    pub fn put(path: String, body: T) -> Event<T> {
        Event {
            r#type: EventType::Put,
            path,
            body,
        }
    }

    pub fn patch(path: String, body: T) -> Event<T> {
        Event {
            r#type: EventType::Patch,
            path,
            body,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum EventType {
    Put,
    Patch,
}

fn get_controller(session: &UnitModel) -> Option<ControllerPresetData> {
    get_controller_preset_data_internal(session).ok()
}

fn get_controller_preset_data_internal(
    session: &UnitModel,
) -> Result<ControllerPresetData, DataError> {
    let data = session.extract_compartment_model(Compartment::Controller);
    if data.mappings.is_empty() {
        return Err(DataError::SessionHasNoActiveController);
    }
    let id = session.active_controller_preset_id();
    let name = id
        .and_then(|id| {
            BackboneShell::get()
                .controller_preset_manager()
                .borrow()
                .find_by_id(id)
        })
        .map(|preset| preset.name().to_string());
    let preset = ControllerPreset::new(
        id.map(|id| id.to_string()).unwrap_or_default(),
        name.unwrap_or_else(|| "<Not saved>".to_string()),
        data,
    );
    Ok(ControllerPresetData::from_model(&preset))
}
