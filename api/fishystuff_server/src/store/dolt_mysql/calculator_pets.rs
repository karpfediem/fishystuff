use fishystuff_api::models::calculator::{CalculatorOptionEntry, CalculatorPetCatalog};
use mysql::prelude::Queryable;

use crate::error::AppResult;
use crate::store::{validate_dolt_ref, FishLang};

use super::util::{db_unavailable, is_missing_table, normalize_optional_string};
use super::DoltMySqlStore;

type CalculatorPetOptionDbRow = (Option<String>, Option<String>, Option<String>);

fn localized_label(lang: FishLang, en: &'static str, ko: &'static str) -> String {
    match lang {
        FishLang::En => en.to_string(),
        FishLang::Ko => ko.to_string(),
    }
}

fn push_unique_option(options: &mut Vec<CalculatorOptionEntry>, key: &str, label: String) {
    if options.iter().any(|option| option.key == key) {
        return;
    }
    options.push(CalculatorOptionEntry {
        key: key.to_string(),
        label,
    });
}

fn canonical_pet_option_key(
    option_kind: &str,
    skill_name_ko: Option<&str>,
    skill_description_ko: Option<&str>,
) -> Option<&'static str> {
    let text = [
        skill_name_ko.unwrap_or(""),
        skill_description_ko.unwrap_or(""),
    ]
    .join("\n");
    if text.contains("자동 낚시") {
        return Some("auto_fishing_time_reduction");
    }
    if text.contains("내구도 소모 감소 저항") {
        return Some("durability_reduction_resistance");
    }
    if text.contains("낚시 경험치") {
        return Some("fishing_exp");
    }
    if text.contains("생활 경험치") {
        return Some("life_exp");
    }
    if option_kind == "special" {
        return Some("auto_fishing_time_reduction");
    }
    None
}

fn calculator_pet_option_label(lang: FishLang, key: &str) -> String {
    match key {
        "auto_fishing_time_reduction" => {
            localized_label(lang, "Auto-Fishing Time Reduction", "자동 낚시 시간 감소")
        }
        "durability_reduction_resistance" => localized_label(
            lang,
            "Durability Reduction Resistance",
            "내구도 소모 감소 저항",
        ),
        "life_exp" => localized_label(lang, "Life EXP", "생활 경험치"),
        "fishing_exp" => localized_label(lang, "Fishing EXP", "낚시 경험치"),
        _ => key.to_string(),
    }
}

fn build_calculator_pet_catalog(lang: FishLang) -> CalculatorPetCatalog {
    let tiers = (1..=5)
        .map(|tier| CalculatorOptionEntry {
            key: tier.to_string(),
            label: match lang {
                FishLang::En => format!("Tier {tier}"),
                FishLang::Ko => format!("{tier}세대"),
            },
        })
        .collect();
    let specials = vec![
        CalculatorOptionEntry {
            key: String::new(),
            label: localized_label(lang, "None", "없음"),
        },
        CalculatorOptionEntry {
            key: "auto_fishing_time_reduction".to_string(),
            label: localized_label(lang, "Auto-Fishing Time Reduction", "자동 낚시 시간 감소"),
        },
    ];
    let talents = vec![
        CalculatorOptionEntry {
            key: String::new(),
            label: localized_label(lang, "None", "없음"),
        },
        CalculatorOptionEntry {
            key: "durability_reduction_resistance".to_string(),
            label: localized_label(
                lang,
                "Durability Reduction Resistance",
                "내구도 소모 감소 저항",
            ),
        },
    ];
    let skills = vec![CalculatorOptionEntry {
        key: "fishing_exp".to_string(),
        label: localized_label(lang, "Fishing EXP", "낚시 경험치"),
    }];

    CalculatorPetCatalog {
        slots: 5,
        tiers,
        specials,
        talents,
        skills,
    }
}

impl DoltMySqlStore {
    pub(super) fn query_calculator_pet_catalog(
        &self,
        lang: FishLang,
        ref_id: Option<&str>,
    ) -> AppResult<CalculatorPetCatalog> {
        let mut catalog = build_calculator_pet_catalog(lang);
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let query = format!(
            "SELECT \
                option_kind, \
                skill_name_ko, \
                skill_description_ko \
             FROM calculator_pet_skill_options{as_of}"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<CalculatorPetOptionDbRow> = match conn.query(query) {
            Ok(rows) => rows,
            Err(err)
                if is_missing_table(&err, "calculator_pet_skill_options")
                    || is_missing_table(&err, "skilltype_table_new")
                    || is_missing_table(&err, "pet_base_skill_table")
                    || is_missing_table(&err, "pet_equipskill_table")
                    || is_missing_table(&err, "pet_setstats_table") =>
            {
                return Ok(catalog);
            }
            Err(err) => return Err(db_unavailable(err)),
        };

        for (option_kind, skill_name_ko, skill_description_ko) in rows {
            let option_kind = normalize_optional_string(option_kind).unwrap_or_default();
            let skill_name_ko = normalize_optional_string(skill_name_ko);
            let skill_description_ko = normalize_optional_string(skill_description_ko);
            let Some(key) = canonical_pet_option_key(
                &option_kind,
                skill_name_ko.as_deref(),
                skill_description_ko.as_deref(),
            ) else {
                continue;
            };
            let label = calculator_pet_option_label(lang, key);
            match key {
                "auto_fishing_time_reduction" => {
                    push_unique_option(&mut catalog.specials, key, label);
                }
                "durability_reduction_resistance" | "life_exp" => {
                    push_unique_option(&mut catalog.talents, key, label);
                }
                "fishing_exp" => {
                    push_unique_option(&mut catalog.skills, key, label);
                }
                _ => {}
            }
        }

        Ok(catalog)
    }
}

#[cfg(test)]
mod tests {
    use crate::store::FishLang;

    use super::{calculator_pet_option_label, canonical_pet_option_key};

    #[test]
    fn canonical_pet_option_key_maps_current_source_rows() {
        assert_eq!(
            canonical_pet_option_key("skill", Some("낚시 경험치 획득량 증가 +5%"), None),
            Some("fishing_exp")
        );
        assert_eq!(
            canonical_pet_option_key("talent", Some("생활 경험치 획득량 증가 +5%"), None),
            Some("life_exp")
        );
        assert_eq!(
            canonical_pet_option_key("special", Some("자동 낚시 시간 감소 5%"), None),
            Some("auto_fishing_time_reduction")
        );
        assert_eq!(
            canonical_pet_option_key("talent", Some("내구도 소모 감소 저항 +5%"), None),
            Some("durability_reduction_resistance")
        );
        assert_eq!(
            canonical_pet_option_key("other", Some("낚시 1단계 상승"), None),
            None
        );
    }

    #[test]
    fn calculator_pet_option_label_localizes_known_keys() {
        assert_eq!(
            calculator_pet_option_label(FishLang::En, "fishing_exp"),
            "Fishing EXP"
        );
        assert_eq!(
            calculator_pet_option_label(FishLang::Ko, "life_exp"),
            "생활 경험치"
        );
    }
}
