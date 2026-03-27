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
    pub(super) fn query_calculator_catalog(
        &self,
        lang: FishLang,
        ref_id: Option<&str>,
    ) -> AppResult<CalculatorCatalogResponse> {
        Ok(CalculatorCatalogResponse {
            items: self.query_calculator_items(lang, ref_id)?,
            lifeskill_levels: build_calculator_lifeskill_levels(),
            fishing_levels: build_calculator_fishing_levels(lang),
            session_units: build_calculator_session_units(lang),
            session_presets: build_calculator_session_presets(lang),
            pets: self.query_calculator_pet_catalog(lang, ref_id)?,
            defaults: build_calculator_default_signals(),
        })
    }
}
