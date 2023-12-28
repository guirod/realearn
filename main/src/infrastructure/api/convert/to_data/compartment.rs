use std::collections::HashMap;

use crate::domain::{CompartmentParamIndex, ParamSetting};
use crate::infrastructure::api::convert::to_data::group::convert_group;
use crate::infrastructure::api::convert::to_data::parameter::convert_parameter;
use crate::infrastructure::api::convert::to_data::{convert_mapping, ApiToDataConversionContext};
use crate::infrastructure::api::convert::{convert_multiple, ConversionResult};
use crate::infrastructure::data::{CompartmentModelData, GroupModelData};
use realearn_api::persistence::*;

pub fn convert_compartment(c: Compartment) -> ConversionResult<CompartmentModelData> {
    struct ConversionContext {
        parameters: HashMap<CompartmentParamIndex, ParamSetting>,
        groups: Vec<GroupModelData>,
    }
    fn param_index_by_key(
        parameters: &HashMap<CompartmentParamIndex, ParamSetting>,
        key: &str,
    ) -> Option<CompartmentParamIndex> {
        parameters
            .iter()
            .find(|(_, p)| p.key_matches(key))
            .map(|(i, _)| *i)
    }
    impl ApiToDataConversionContext for ConversionContext {
        fn param_index_by_key(&self, key: &str) -> Option<CompartmentParamIndex> {
            param_index_by_key(&self.parameters, key)
        }
    }
    let parameters = {
        let res: ConversionResult<HashMap<_, _>> = c
            .parameters
            .unwrap_or_default()
            .into_iter()
            .map(|p| {
                Ok((
                    CompartmentParamIndex::try_from(p.index).map_err(anyhow::Error::msg)?,
                    convert_parameter(p)?,
                ))
            })
            .collect();
        res?
    };
    let groups = convert_multiple(c.groups.unwrap_or_default(), |g| {
        convert_group(g, false, |key| param_index_by_key(&parameters, key))
    })?;
    let context = ConversionContext { parameters, groups };
    let data = CompartmentModelData {
        default_group: Some(convert_group(
            c.default_group.unwrap_or_default(),
            true,
            |key| context.param_index_by_key(key),
        )?),
        mappings: convert_multiple(c.mappings.unwrap_or_default(), |m| {
            convert_mapping(m, &context)
        })?,
        parameters: context
            .parameters
            .iter()
            .map(|(key, value)| (key.to_string(), value.clone()))
            .collect(),
        groups: context.groups,
        custom_data: c.custom_data.unwrap_or_default(),
        notes: c.notes.unwrap_or_default(),
    };
    Ok(data)
}
