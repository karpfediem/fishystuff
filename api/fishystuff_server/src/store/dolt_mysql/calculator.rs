use fishystuff_api::models::calculator::CalculatorCatalogResponse;

use crate::error::AppResult;
use crate::store::FishLang;

use super::calculator_defaults::{
    build_calculator_default_signals, build_calculator_fishing_levels,
    build_calculator_lifeskill_levels, build_calculator_session_presets,
    build_calculator_session_units, build_calculator_trade_levels,
};
use super::DoltMySqlStore;

impl DoltMySqlStore {
    fn calculator_catalog_cache_key(lang: FishLang, revision: &str) -> String {
        let lang = match lang {
            FishLang::En => "en",
            FishLang::Ko => "ko",
        };
        format!("{lang}:{revision}")
    }

    pub(super) fn resolve_calculator_catalog_ref(
        &self,
        ref_id: Option<&str>,
    ) -> AppResult<(String, String)> {
        let revision = self.query_dolt_revision_uncached(ref_id)?;
        let resolved_ref = revision
            .strip_prefix("dolt:")
            .unwrap_or(revision.as_str())
            .to_string();
        Ok((revision, resolved_ref))
    }

    pub(super) fn query_calculator_catalog(
        &self,
        lang: FishLang,
        ref_id: Option<&str>,
    ) -> AppResult<CalculatorCatalogResponse> {
        let (revision, resolved_ref) = self.resolve_calculator_catalog_ref(ref_id)?;
        let cache_key = Self::calculator_catalog_cache_key(lang, &revision);
        loop {
            if let Ok(cache) = self.calculator_catalog_cache.lock() {
                if let Some(cached) = cache.get(&cache_key) {
                    return Ok(cached.clone());
                }
            }

            let (inflight_lock, inflight_cvar) = &*self.calculator_catalog_inflight;
            let mut inflight = inflight_lock
                .lock()
                .expect("calculator catalog inflight lock poisoned");
            if !inflight.contains(&cache_key) {
                inflight.insert(cache_key.clone());
                drop(inflight);
                break;
            }
            inflight = inflight_cvar
                .wait(inflight)
                .expect("calculator catalog inflight wait poisoned");
            drop(inflight);
        }

        let query_ref = Some(resolved_ref.as_str());
        let result: AppResult<CalculatorCatalogResponse> = (|| {
            let items = self.query_calculator_items(lang, query_ref)?;
            let mastery_prize_curve = self.query_calculator_mastery_prize_curve(query_ref)?;
            let zone_group_rates = self.query_calculator_zone_group_rates(query_ref)?;
            let pets = self.query_calculator_pet_catalog(lang, query_ref)?;
            Ok(CalculatorCatalogResponse {
                items,
                lifeskill_levels: build_calculator_lifeskill_levels(),
                mastery_prize_curve,
                zone_group_rates,
                fishing_levels: build_calculator_fishing_levels(lang),
                trade_levels: build_calculator_trade_levels(lang),
                session_units: build_calculator_session_units(lang),
                session_presets: build_calculator_session_presets(lang),
                defaults: build_calculator_default_signals(&pets),
                pets,
            })
        })();

        let (inflight_lock, inflight_cvar) = &*self.calculator_catalog_inflight;
        let mut inflight = inflight_lock
            .lock()
            .expect("calculator catalog inflight lock poisoned");
        inflight.remove(&cache_key);
        inflight_cvar.notify_all();
        drop(inflight);

        let catalog = result?;

        if let Ok(mut cache) = self.calculator_catalog_cache.lock() {
            cache.insert(cache_key, catalog.clone());
        }

        Ok(catalog)
    }
}
