use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

#[derive(Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct Parameter {
    pub index: u32,
    /// An optional ID that you can assign to this parameter in order to refer
    /// to it from somewhere else.
    ///
    /// This ID should be unique within all parameters in the same compartment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_count: Option<NonZeroU32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_labels: Option<Vec<String>>,
}
