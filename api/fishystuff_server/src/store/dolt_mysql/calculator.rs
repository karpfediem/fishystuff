use fishystuff_api::models::calculator::CalculatorCatalogResponse;

use crate::error::{AppError, AppResult};
use crate::store::DataLang;

use super::calculator_defaults::{
    build_calculator_default_signals, build_calculator_fishing_levels,
    build_calculator_lifeskill_levels, build_calculator_session_presets,
    build_calculator_session_units, build_calculator_trade_levels,
};
use super::DoltMySqlStore;

impl DoltMySqlStore {
    fn calculator_catalog_cache_key(lang: &DataLang, revision: &str) -> String {
        format!("{}:{revision}", lang.code())
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
        lang: DataLang,
        ref_id: Option<&str>,
    ) -> AppResult<CalculatorCatalogResponse> {
        let (revision, resolved_ref) = self.resolve_calculator_catalog_ref(ref_id)?;
        let cache_key = Self::calculator_catalog_cache_key(&lang, &revision);
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
            self.validate_data_lang_available(&lang, query_ref)?;
            let worker_span = tracing::Span::current();
            let (source_data, mastery_prize_curve, zone_group_rates, pets, trade_npcs) =
                std::thread::scope(|scope| -> AppResult<_> {
                    let source_lang = lang.clone();
                    let source_revision = revision.clone();
                    let source_resolved_ref = resolved_ref.clone();
                    let source_data_handle = scope.spawn({
                        let worker_span = worker_span.clone();
                        move || {
                            let _worker = worker_span.enter();
                            let _span = tracing::info_span!(
                                "store.calculator_catalog.source_data",
                                lang = %source_lang.code(),
                                revision = %source_revision,
                            )
                            .entered();
                            self.query_calculator_catalog_source_data_at_revision(
                                &source_lang,
                                &source_revision,
                                &source_resolved_ref,
                            )
                        }
                    });
                    let mastery_prize_curve_handle = scope.spawn({
                        let worker_span = worker_span.clone();
                        move || {
                            let _worker = worker_span.enter();
                            let _span =
                                tracing::info_span!("store.calculator_catalog.mastery_prize_curve")
                                    .entered();
                            self.query_calculator_mastery_prize_curve(query_ref)
                        }
                    });
                    let zone_group_rates_handle = scope.spawn({
                        let worker_span = worker_span.clone();
                        move || {
                            let _worker = worker_span.enter();
                            let _span =
                                tracing::info_span!("store.calculator_catalog.zone_group_rates")
                                    .entered();
                            self.query_calculator_zone_group_rates(query_ref)
                        }
                    });
                    let pet_lang = lang.clone();
                    let pets_handle = scope.spawn({
                        let worker_span = worker_span.clone();
                        move || {
                            let _worker = worker_span.enter();
                            let _span = tracing::info_span!("store.calculator_catalog.pet_catalog")
                                .entered();
                            self.query_calculator_pet_catalog(&pet_lang, query_ref)
                        }
                    });
                    let trade_npcs_handle = scope.spawn({
                        let worker_span = worker_span.clone();
                        move || {
                            let _worker = worker_span.enter();
                            let _span = tracing::info_span!("store.calculator_catalog.trade_npcs")
                                .entered();
                            self.query_trade_npc_catalog(query_ref)
                        }
                    });

                    let source_data = source_data_handle.join().map_err(|_| {
                        AppError::internal("calculator catalog source data worker panicked")
                    })??;
                    let mastery_prize_curve =
                        mastery_prize_curve_handle.join().map_err(|_| {
                            AppError::internal("calculator catalog mastery curve worker panicked")
                        })??;
                    let zone_group_rates = zone_group_rates_handle.join().map_err(|_| {
                        AppError::internal("calculator catalog zone group rates worker panicked")
                    })??;
                    let pets = pets_handle.join().map_err(|_| {
                        AppError::internal("calculator catalog pet catalog worker panicked")
                    })??;
                    let trade_npcs = trade_npcs_handle.join().map_err(|_| {
                        AppError::internal("calculator catalog trade NPC worker panicked")
                    })??;
                    Ok((
                        source_data,
                        mastery_prize_curve,
                        zone_group_rates,
                        pets,
                        trade_npcs,
                    ))
                })?;
            let items = {
                let _span = tracing::info_span!("store.calculator_catalog.build_items").entered();
                self.build_calculator_items_from_source_data(&lang, source_data)?
            };
            let defaults = {
                let _span =
                    tracing::info_span!("store.calculator_catalog.default_signals").entered();
                build_calculator_default_signals(&pets)
            };
            Ok(CalculatorCatalogResponse {
                items,
                lifeskill_levels: build_calculator_lifeskill_levels(),
                mastery_prize_curve,
                zone_group_rates,
                fishing_levels: build_calculator_fishing_levels(&lang),
                trade_levels: build_calculator_trade_levels(&lang),
                session_units: build_calculator_session_units(&lang),
                session_presets: build_calculator_session_presets(&lang),
                trade_npcs,
                defaults,
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
