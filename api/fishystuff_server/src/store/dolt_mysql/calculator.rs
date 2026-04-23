use fishystuff_api::models::calculator::CalculatorCatalogResponse;
use serde::Deserialize;
use std::sync::OnceLock;

use crate::error::{AppError, AppResult};
use crate::store::FishLang;

use super::DoltMySqlStore;

#[derive(Debug, Clone, Deserialize)]
struct BundledCalculatorCatalogSnapshot {
    dolt_revision: String,
    catalogs: BundledCalculatorCatalogs,
}

#[derive(Debug, Clone, Deserialize)]
struct BundledCalculatorCatalogs {
    en: CalculatorCatalogResponse,
    ko: CalculatorCatalogResponse,
}

fn bundled_calculator_catalog_snapshot() -> AppResult<&'static BundledCalculatorCatalogSnapshot> {
    static SNAPSHOT: OnceLock<Result<BundledCalculatorCatalogSnapshot, String>> = OnceLock::new();
    SNAPSHOT
        .get_or_init(|| {
            serde_json::from_str(include_str!("../../../assets/calculator_catalogs.json"))
                .map_err(|err| err.to_string())
        })
        .as_ref()
        .map_err(|err| {
            AppError::internal(format!("parse bundled calculator catalog snapshot: {err}"))
        })
}

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

    fn bundled_calculator_catalog(
        &self,
        lang: FishLang,
        ref_id: Option<&str>,
    ) -> AppResult<CalculatorCatalogResponse> {
        let snapshot = bundled_calculator_catalog_snapshot()?;
        let requested_revision = self.query_dolt_revision(ref_id).ok_or_else(|| {
            AppError::unavailable(
                "could not resolve requested Dolt revision for calculator catalog",
            )
        })?;
        if requested_revision != snapshot.dolt_revision {
            return Err(AppError::unavailable(format!(
                "calculator catalog snapshot is for {}; requested {requested_revision}",
                snapshot.dolt_revision
            )));
        }
        Ok(match lang {
            FishLang::En => snapshot.catalogs.en.clone(),
            FishLang::Ko => snapshot.catalogs.ko.clone(),
        })
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

        let catalog = self.bundled_calculator_catalog(lang, ref_id)?;

        if let Ok(mut cache) = self.calculator_catalog_cache.lock() {
            cache.insert(cache_key, catalog.clone());
        }

        Ok(catalog)
    }
}

#[cfg(test)]
mod tests {
    use super::bundled_calculator_catalog_snapshot;

    #[test]
    fn bundled_calculator_catalog_snapshot_parses() {
        let snapshot = bundled_calculator_catalog_snapshot().unwrap();
        assert!(snapshot.dolt_revision.starts_with("dolt:"));
        assert!(!snapshot.catalogs.en.items.is_empty());
        assert!(!snapshot.catalogs.ko.items.is_empty());
        assert!(!snapshot.catalogs.en.pets.pets.is_empty());
        assert!(!snapshot.catalogs.ko.pets.pets.is_empty());
    }
}
