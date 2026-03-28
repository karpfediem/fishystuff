use fishystuff_api::models::calculator::CalculatorCatalogResponse;

use crate::error::AppResult;
use crate::store::FishLang;

use super::calculator_defaults::{
    build_calculator_default_signals, build_calculator_fishing_levels,
    build_calculator_lifeskill_levels, build_calculator_session_presets,
    build_calculator_session_units,
};
use super::DoltMySqlStore;

impl DoltMySqlStore {
    fn calculator_catalog_cache_key(lang: FishLang, ref_id: Option<&str>) -> String {
        let lang = match lang {
            FishLang::En => "en",
            FishLang::Ko => "ko",
        };
        match ref_id {
            Some(ref_id) => format!("{lang}:{ref_id}"),
            None => format!("{lang}:head"),
        }
    }

    pub(super) fn query_calculator_catalog(
        &self,
        lang: FishLang,
        ref_id: Option<&str>,
    ) -> AppResult<CalculatorCatalogResponse> {
        let cache_key = Self::calculator_catalog_cache_key(lang, ref_id);
        if let Ok(cache) = self.calculator_catalog_cache.lock() {
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        let catalog = CalculatorCatalogResponse {
            items: self.query_calculator_items(lang, ref_id)?,
            lifeskill_levels: build_calculator_lifeskill_levels(),
            fishing_levels: build_calculator_fishing_levels(lang),
            session_units: build_calculator_session_units(lang),
            session_presets: build_calculator_session_presets(lang),
            pets: self.query_calculator_pet_catalog(lang, ref_id)?,
            defaults: build_calculator_default_signals(),
        };

        if let Ok(mut cache) = self.calculator_catalog_cache.lock() {
            cache.insert(cache_key, catalog.clone());
        }

        Ok(catalog)
    }
}
