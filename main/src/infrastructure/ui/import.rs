use anyhow::{bail, Context};
use std::error::Error;
use std::fmt::Debug;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::domain::SafeLua;
use crate::infrastructure::api::convert::from_data::ConversionStyle;
use crate::infrastructure::api::convert::to_data::ApiToDataConversionContext;
use crate::infrastructure::api::convert::{from_data, to_data};
use crate::infrastructure::data::{
    ActivationConditionData, CompartmentModelData, MappingModelData, ModeModelData, SessionData,
    SourceModelData, TargetModelData,
};
use crate::infrastructure::plugin::BackboneShell;
use crate::infrastructure::ui::lua_serializer;
use crate::infrastructure::ui::util::open_in_browser;
use mlua::{Lua, LuaSerdeExt, Value};
use realearn_api::persistence;
use realearn_api::persistence::{ApiObject, Envelope};
use realearn_csi::{deserialize_csi_object_from_csi, AnnotatedResult, CsiObject};
use reaper_high::Reaper;
use semver::Version;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum UntaggedDataObject {
    Tagged(DataObject),
    PresetLike(CommonPresetData),
}

impl UntaggedDataObject {
    pub fn version(&self) -> Option<&Version> {
        match self {
            UntaggedDataObject::Tagged(o) => o.version(),
            UntaggedDataObject::PresetLike(d) => d.version.as_ref(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DataObject {
    Session(Envelope<Box<SessionData>>),
    #[cfg(feature = "playtime")]
    ClipMatrix(Envelope<Box<Option<playtime_api::persistence::FlexibleMatrix>>>),
    MainCompartment(Envelope<Box<CompartmentModelData>>),
    ControllerCompartment(Envelope<Box<CompartmentModelData>>),
    Mappings(Envelope<Vec<MappingModelData>>),
    Mapping(Envelope<Box<MappingModelData>>),
    Source(Envelope<Box<SourceModelData>>),
    #[serde(alias = "Mode")]
    Glue(Envelope<Box<ModeModelData>>),
    Target(Envelope<Box<TargetModelData>>),
    ActivationCondition(Envelope<Box<ActivationConditionData>>),
}

/// This corresponds to the way controller and main presets are structured.
///
/// They don't have an envelope. We also want to be able to import their data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonPresetData {
    #[serde(default)]
    pub version: Option<Version>,
    pub name: String,
    #[serde(flatten)]
    pub data: Box<CompartmentModelData>,
}

impl DataObject {
    pub fn try_from_api_object(
        api_object: ApiObject,
        conversion_context: &impl ApiToDataConversionContext,
    ) -> Result<Self, Box<dyn Error>> {
        let data_object = match api_object {
            #[cfg(feature = "playtime")]
            ApiObject::ClipMatrix(envelope) => DataObject::ClipMatrix(envelope),
            ApiObject::MainCompartment(Envelope { value: c, version }) => {
                let data_compartment = to_data::convert_compartment(*c)?;
                DataObject::MainCompartment(Envelope::new(version, Box::new(data_compartment)))
            }
            ApiObject::ControllerCompartment(Envelope { value: c, version }) => {
                let data_compartment = to_data::convert_compartment(*c)?;
                DataObject::ControllerCompartment(Envelope::new(
                    version,
                    Box::new(data_compartment),
                ))
            }
            ApiObject::Mappings(Envelope {
                value: mappings,
                version,
            }) => {
                let data_mappings = Self::try_from_api_mappings(mappings, conversion_context);
                DataObject::Mappings(Envelope::new(version, data_mappings?))
            }
            ApiObject::Mapping(Envelope { value: m, version }) => {
                let data_mapping = to_data::convert_mapping(*m, conversion_context)?;
                DataObject::Mapping(Envelope::new(version, Box::new(data_mapping)))
            }
        };
        Ok(data_object)
    }

    pub fn try_from_api_mappings(
        api_mappings: Vec<persistence::Mapping>,
        conversion_context: &impl ApiToDataConversionContext,
    ) -> anyhow::Result<Vec<MappingModelData>> {
        api_mappings
            .into_iter()
            .map(|m| to_data::convert_mapping(m, conversion_context))
            .collect()
    }

    pub fn try_into_api_object(
        self,
        conversion_style: ConversionStyle,
    ) -> anyhow::Result<ApiObject> {
        let api_object = match self {
            DataObject::Session(Envelope { .. }) => todo!("session API not yet implemented"),
            #[cfg(feature = "playtime")]
            DataObject::ClipMatrix(envelope) => ApiObject::ClipMatrix(envelope),
            DataObject::MainCompartment(Envelope { value: c, version }) => {
                let api_compartment = from_data::convert_compartment(*c, conversion_style)?;
                ApiObject::MainCompartment(Envelope::new(version, Box::new(api_compartment)))
            }
            DataObject::ControllerCompartment(Envelope { value: c, version }) => {
                let api_compartment = from_data::convert_compartment(*c, conversion_style)?;
                ApiObject::ControllerCompartment(Envelope::new(version, Box::new(api_compartment)))
            }
            DataObject::Mappings(Envelope {
                value: mappings,
                version,
            }) => {
                let api_mappings: Result<Vec<_>, _> = mappings
                    .into_iter()
                    .map(|m| from_data::convert_mapping(m, conversion_style))
                    .collect();
                ApiObject::Mappings(Envelope::new(version, api_mappings?))
            }
            DataObject::Mapping(Envelope { value: m, version }) => {
                let api_mapping = from_data::convert_mapping(*m, conversion_style)?;
                ApiObject::Mapping(Envelope::new(version, Box::new(api_mapping)))
            }
            _ => {
                bail!("conversion from source/mode/target data object not supported at the moment");
            }
        };
        Ok(api_object)
    }

    pub fn version(&self) -> Option<&Version> {
        use DataObject::*;
        match self {
            Session(v) => v.version.as_ref(),
            #[cfg(feature = "playtime")]
            ClipMatrix(v) => v.version.as_ref(),
            MainCompartment(v) => v.version.as_ref(),
            ControllerCompartment(v) => v.version.as_ref(),
            Mappings(v) => v.version.as_ref(),
            Mapping(v) => v.version.as_ref(),
            Source(v) => v.version.as_ref(),
            Glue(v) => v.version.as_ref(),
            Target(v) => v.version.as_ref(),
            ActivationCondition(v) => v.version.as_ref(),
        }
    }
}

/// Attempts to deserialize a data object supporting both JSON and Lua.
pub fn deserialize_data_object(
    text: &str,
    conversion_context: &impl ApiToDataConversionContext,
) -> Result<AnnotatedResult<UntaggedDataObject>, Box<dyn Error>> {
    let json_err = match deserialize_untagged_data_object_from_json(text) {
        Ok(o) => {
            return Ok(AnnotatedResult::without_annotations(o));
        }
        Err(e) => e,
    };
    let lua_err = match deserialize_data_object_from_lua(text, conversion_context) {
        Ok(o) => {
            return Ok(AnnotatedResult::without_annotations(
                UntaggedDataObject::Tagged(o),
            ));
        }
        Err(e) => e,
    };
    let csi_err = match deserialize_data_object_from_csi(text, conversion_context) {
        Ok(r) => {
            let untagged_data_object = UntaggedDataObject::Tagged(r.value);
            let annotated_result = AnnotatedResult {
                value: untagged_data_object,
                annotations: r.annotations,
            };
            return Ok(annotated_result);
        }
        Err(e) => e,
    };
    let msg = format!(
        "Clipboard content doesn't look like proper ReaLearn import data:\n\n\
        Invalid JSON: \n\
        {json_err}\n\n\
        Invalid Lua: \n\
        {lua_err}\n\n\
        Invalid CSI: \n\
        {csi_err}"
    );
    Err(msg.into())
}

pub fn deserialize_data_object_from_json(text: &str) -> Result<DataObject, Box<dyn Error>> {
    Ok(serde_json::from_str(text)?)
}

pub fn deserialize_untagged_data_object_from_json(
    text: &str,
) -> Result<UntaggedDataObject, Box<dyn Error>> {
    Ok(serde_json::from_str(text)?)
}

pub fn deserialize_data_object_from_csi(
    text: &str,
    conversion_context: &impl ApiToDataConversionContext,
) -> Result<AnnotatedResult<DataObject>, Box<dyn Error>> {
    let csi_object = deserialize_csi_object_from_csi(text)?;
    let api_object_res = CsiObject::try_into_api_object(csi_object)?;
    let res = AnnotatedResult {
        value: DataObject::try_from_api_object(api_object_res.value, conversion_context)?,
        annotations: api_object_res.annotations,
    };
    Ok(res)
}

pub fn deserialize_data_object_from_lua(
    text: &str,
    conversion_context: &impl ApiToDataConversionContext,
) -> Result<DataObject, Box<dyn Error>> {
    let api_object = deserialize_api_object_from_lua(text)?;
    let data_object = DataObject::try_from_api_object(api_object, conversion_context)?;
    Ok(data_object)
}

pub fn serialize_data_object_to_json(object: DataObject) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(&object).context("couldn't serialize object")?)
}

/// Runs without importing the result and also doesn't have an execution time limit.
pub fn dry_run_lua_script(text: &str) -> Result<(), Box<dyn Error>> {
    let lua = SafeLua::new()?;
    let value = execute_lua_import_script(&lua, text)?;
    let json = serde_json::to_string_pretty(&value)?;
    match BackboneShell::get_temp_dir() {
        None => {
            Reaper::get().show_console_msg(json);
        }
        Some(dir) => {
            let json_file = dir.path().join("dry-run.json");
            std::fs::write(&json_file, json)?;
            open_in_browser(&json_file.to_string_lossy())
        }
    }
    Ok(())
}

pub enum SerializationFormat {
    JsonDataObject,
    LuaApiObject(ConversionStyle),
}

pub fn serialize_data_object(
    data_object: DataObject,
    format: SerializationFormat,
) -> anyhow::Result<String> {
    match format {
        SerializationFormat::JsonDataObject => serialize_data_object_to_json(data_object),
        SerializationFormat::LuaApiObject(style) => {
            serialize_data_object_to_lua(data_object, style)
        }
    }
}

pub fn serialize_data_object_to_lua(
    data_object: DataObject,
    conversion_style: ConversionStyle,
) -> anyhow::Result<String> {
    let api_object = data_object.try_into_api_object(conversion_style)?;
    Ok(lua_serializer::to_string(&api_object)?)
}

pub fn deserialize_api_object_from_lua(text: &str) -> Result<ApiObject, Box<dyn Error>> {
    let lua = SafeLua::new()?;
    let lua = lua.start_execution_time_limit_countdown(Duration::from_millis(200))?;
    let value = execute_lua_import_script(&lua, text)?;
    Ok(lua.as_ref().from_value(value)?)
}

fn execute_lua_import_script<'a>(
    lua: &'a SafeLua,
    text: &str,
) -> Result<mlua::Value<'a>, Box<dyn Error>> {
    let env = lua.create_fresh_environment(true)?;
    // Add some useful functions (hidden, undocumented, subject to change!)
    let realearn_table = {
        let lua: &Lua = lua.as_ref();
        let table = lua.create_table()?;
        let get_track_guid_by_index = lua.create_function(|_, index: u32| {
            let guid = Reaper::get()
                .current_project()
                .track_by_index(index)
                .map(|t| t.guid().to_string_without_braces());
            Ok(guid)
        })?;
        table.set("get_track_guid_by_index", get_track_guid_by_index)?;
        let get_track_guid_by_name_prefix = lua.create_function(|_, prefix: String| {
            let guid = Reaper::get().current_project().tracks().find_map(|t| {
                if !t.name()?.to_str().starts_with(&prefix) {
                    return None;
                }
                Some(t.guid().to_string_without_braces())
            });
            Ok(guid)
        })?;
        table.set(
            "get_track_guid_by_name_prefix",
            get_track_guid_by_name_prefix,
        )?;
        let print = lua.create_function(|_, arg: mlua::Value| {
            let text: String = match arg {
                Value::String(s) => format!("{}\n", s.to_string_lossy()),
                arg => format!("{arg:?}\n"),
            };
            Reaper::get().show_console_msg(text);
            Ok(())
        })?;
        table.set("print", print)?;
        table
    };
    env.set("realearn", realearn_table)?;
    lua.compile_and_execute("Import", text, env)
}
