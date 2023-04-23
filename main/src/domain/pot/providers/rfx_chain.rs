use crate::domain::pot::provider_database::{
    Database, InnerBuildOutput, SortablePresetId, CONTENT_TYPE_FACTORY_ID, FAVORITE_FAVORITE_ID,
};
use crate::domain::pot::{BuildInput, FilterItemId, InnerPresetId, Preset};

use enum_iterator::IntoEnumIterator;
use realearn_api::persistence::PotFilterItemKind;
use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsStr;
use std::path::PathBuf;
use walkdir::WalkDir;
use wildmatch::WildMatch;

pub struct RfxChainDatabase {
    root_dir: PathBuf,
    rfx_chains: HashMap<InnerPresetId, RfxChain>,
}

impl RfxChainDatabase {
    pub fn open(root_dir: PathBuf) -> Result<Self, Box<dyn Error>> {
        if !root_dir.try_exists()? {
            return Err("path to FX chains directory doesn't exist".into());
        }
        let db = Self {
            root_dir,
            rfx_chains: Default::default(),
        };
        Ok(db)
    }
}

struct RfxChain {
    preset_name: String,
    relative_path: String,
}

impl Database for RfxChainDatabase {
    fn filter_item_name(&self) -> String {
        "FX chains".to_string()
    }

    fn refresh(&mut self) -> Result<(), Box<dyn Error>> {
        let rfx_chains = WalkDir::new(&self.root_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                if !entry.file_type().is_file() {
                    return None;
                }
                if entry.path().extension() != Some(OsStr::new("RfxChain")) {
                    return None;
                }
                let relative_path = entry.path().strip_prefix(&self.root_dir).ok()?;
                // Immediately exclude relative paths that can't be represented as valid UTF-8.
                // Otherwise we will potentially open a can of worms (regarding persistence etc.).
                let rfx_chain = RfxChain {
                    preset_name: entry.path().file_stem()?.to_str()?.to_string(),
                    relative_path: relative_path.to_str()?.to_string(),
                };
                Some(rfx_chain)
            });
        self.rfx_chains = rfx_chains
            .enumerate()
            .map(|(i, rfx_chain)| (InnerPresetId(i as _), rfx_chain))
            .collect();
        Ok(())
    }

    fn build_collections(&self, input: BuildInput) -> Result<InnerBuildOutput, Box<dyn Error>> {
        let mut build_output = InnerBuildOutput::default();
        for (kind, filter) in input.filter_settings.iter() {
            use PotFilterItemKind::*;
            let matches = match kind {
                NksContentType => filter != Some(FilterItemId(Some(CONTENT_TYPE_FACTORY_ID))),
                NksFavorite => filter != Some(FilterItemId(Some(FAVORITE_FAVORITE_ID))),
                NksProductType | NksBank | NksSubBank | NksCategory | NksSubCategory | NksMode => {
                    matches!(filter, None | Some(FilterItemId::NONE))
                }
                _ => true,
            };
            if !matches {
                return Ok(build_output);
            }
        }
        let lowercase_search_expression = input.search_expression.trim().to_lowercase();
        let wild_match = WildMatch::new(&lowercase_search_expression);
        build_output.preset_collection = self
            .rfx_chains
            .iter()
            .filter_map(|(id, rfx_chain)| {
                let matches = if lowercase_search_expression.is_empty() {
                    true
                } else {
                    let lowercase_preset_name = rfx_chain.preset_name.to_lowercase();
                    if input.use_wildcard_search {
                        wild_match.matches(&lowercase_preset_name)
                    } else {
                        lowercase_preset_name.contains(&lowercase_search_expression)
                    }
                };
                if matches {
                    Some(SortablePresetId::new(*id, rfx_chain.preset_name.clone()))
                } else {
                    None
                }
            })
            .collect();
        Ok(build_output)
    }

    fn find_preset_by_id(&self, preset_id: InnerPresetId) -> Option<Preset> {
        let rfx_chain = self.rfx_chains.get(&preset_id)?;
        let preset = Preset {
            favorite_id: rfx_chain.relative_path.clone(),
            name: rfx_chain.preset_name.clone(),
            // At the moment, we can only add RfxChains relative to the official FXChains path anyway.
            file_name: PathBuf::from(&rfx_chain.relative_path),
            file_ext: "RfxChain".to_string(),
        };
        Some(preset)
    }

    fn find_preview_by_preset_id(&self, _preset_id: InnerPresetId) -> Option<PathBuf> {
        None
    }
}
