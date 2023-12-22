use crate::application::{MainPreset, Preset, PresetManager};
use crate::domain::Compartment;
use crate::infrastructure::data::{
    CompartmentModelData, ExtendedPresetManager, FileBasedPresetManager, PresetData, PresetInfo,
};
use base::default_util::{deserialize_null_default, is_default};

use crate::infrastructure::plugin::BackboneShell;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

pub type FileBasedMainPresetManager = FileBasedPresetManager<MainPreset, MainPresetData>;

pub type SharedMainPresetManager = Rc<RefCell<FileBasedMainPresetManager>>;

impl PresetManager for SharedMainPresetManager {
    type PresetType = MainPreset;

    fn find_by_id(&self, id: &str) -> Option<MainPreset> {
        self.borrow().find_by_id(id)
    }
}

impl ExtendedPresetManager for SharedMainPresetManager {
    fn find_index_by_id(&self, id: &str) -> Option<usize> {
        self.borrow().find_index_by_id(id)
    }

    fn find_id_by_index(&self, index: usize) -> Option<String> {
        self.borrow().find_id_by_index(index)
    }

    fn remove_preset(&mut self, id: &str) -> Result<(), &'static str> {
        self.borrow_mut().remove_preset(id)
    }

    fn preset_infos(&self) -> Vec<PresetInfo> {
        self.borrow().preset_infos()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MainPresetData {
    // Since ReaLearn 1.12.0-pre18
    #[serde(
        default,
        deserialize_with = "deserialize_null_default",
        skip_serializing_if = "is_default"
    )]
    version: Option<Version>,
    #[serde(skip_deserializing, skip_serializing_if = "is_default")]
    id: Option<String>,
    name: String,
    #[serde(flatten)]
    data: CompartmentModelData,
}

impl PresetData for MainPresetData {
    type P = MainPreset;

    fn from_model(preset: &MainPreset) -> MainPresetData {
        MainPresetData {
            version: Some(BackboneShell::version().clone()),
            id: Some(preset.id().to_string()),
            data: CompartmentModelData::from_model(preset.data()),
            name: preset.name().to_string(),
        }
    }

    fn to_model(&self, id: String) -> Result<MainPreset, Box<dyn Error>> {
        let preset = MainPreset::new(
            id,
            self.name.clone(),
            self.data
                .to_model(self.version.as_ref(), Compartment::Main, None)?,
        );
        Ok(preset)
    }

    fn clear_id(&mut self) {
        self.id = None;
    }

    fn version(&self) -> Option<&Version> {
        self.version.as_ref()
    }
}
