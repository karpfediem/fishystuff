use crate::bridge::contract::FishyMapSemanticTermSummary;
use fishystuff_core::field_metadata::{
    detail_fact_is_visible, detail_facts, preferred_detail_fact, FieldDetailFact,
    FieldHoverMetadataEntry, FIELD_DETAIL_FACT_KEY_REGION, FIELD_DETAIL_FACT_KEY_RESOURCE_GROUP,
    FIELD_DETAIL_FACT_KEY_ZONE,
};

use super::super::super::*;
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::field_semantics::ordered_semantic_layers;

pub(in crate::bridge::host::snapshot) fn current_semantic_term_summaries(
    layer_registry: &LayerRegistry,
    field_metadata: &FieldMetadataCache,
) -> Vec<FishyMapSemanticTermSummary> {
    let mut summaries = Vec::new();
    for layer in ordered_semantic_layers(layer_registry) {
        if layer.key == "zone_mask" {
            continue;
        }
        let Some(metadata_url) = layer.field_metadata_url() else {
            continue;
        };
        let Some(metadata) = field_metadata.get(layer.id, &metadata_url) else {
            continue;
        };
        for (field_id, entry) in &metadata.entries {
            let Some(summary) = semantic_term_summary(layer, *field_id, entry) else {
                continue;
            };
            summaries.push(summary);
        }
    }
    summaries
}

fn semantic_term_summary(
    layer: &crate::map::layers::LayerSpec,
    field_id: u32,
    entry: &FieldHoverMetadataEntry,
) -> Option<FishyMapSemanticTermSummary> {
    let primary = semantic_term_primary_fact(&layer.key, entry)?;
    let label = primary.value.trim();
    if label.is_empty() {
        return None;
    }

    let description = entry
        .targets
        .first()
        .map(|target| target.label.trim())
        .filter(|value| !value.is_empty() && *value != label)
        .map(ToOwned::to_owned)
        .or_else(|| {
            detail_facts(&entry.detail_sections)
                .find(|fact| fact.key != primary.key && !fact.value.trim().is_empty())
                .map(|fact| fact.value.trim().to_string())
        });

    let mut search_parts = vec![
        layer.name.trim().to_string(),
        layer.key.trim().to_string(),
        label.to_string(),
        field_id.to_string(),
    ];
    for fact in detail_facts(&entry.detail_sections) {
        let label = fact.label.trim();
        if !label.is_empty() {
            search_parts.push(label.to_string());
        }
        let value = fact.value.trim();
        if !value.is_empty() {
            search_parts.push(value.to_string());
        }
    }
    for target in &entry.targets {
        let label = target.label.trim();
        if !label.is_empty() {
            search_parts.push(label.to_string());
        }
    }
    let search_text = search_parts.join(" ");

    Some(FishyMapSemanticTermSummary {
        layer_id: layer.key.clone(),
        layer_name: layer.name.clone(),
        field_id,
        label: label.to_string(),
        description,
        search_text,
    })
}

fn semantic_term_primary_fact<'a>(
    layer_key: &str,
    entry: &'a FieldHoverMetadataEntry,
) -> Option<&'a FieldDetailFact> {
    canonical_semantic_fact_key(layer_key)
        .and_then(|fact_key| {
            detail_facts(&entry.detail_sections)
                .find(|fact| fact.key == fact_key && detail_fact_is_visible(fact))
        })
        .or_else(|| preferred_detail_fact(detail_facts(&entry.detail_sections)))
}

fn canonical_semantic_fact_key(layer_key: &str) -> Option<&'static str> {
    match layer_key {
        "zone_mask" => Some(FIELD_DETAIL_FACT_KEY_ZONE),
        "regions" => Some(FIELD_DETAIL_FACT_KEY_REGION),
        "region_groups" => Some(FIELD_DETAIL_FACT_KEY_RESOURCE_GROUP),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::semantic_term_primary_fact;
    use fishystuff_core::field_metadata::{
        FieldDetailFact, FieldDetailSection, FieldHoverMetadataEntry,
        FIELD_DETAIL_FACT_KEY_ORIGIN_REGION, FIELD_DETAIL_FACT_KEY_REGION,
        FIELD_DETAIL_FACT_KEY_RESOURCE_GROUP,
    };

    fn fact(key: &str, label: &str, value: &str) -> FieldDetailFact {
        FieldDetailFact {
            key: key.to_string(),
            label: label.to_string(),
            value: value.to_string(),
            icon: None,
            status_icon: None,
            status_icon_tone: None,
        }
    }

    fn entry(facts: Vec<FieldDetailFact>) -> FieldHoverMetadataEntry {
        FieldHoverMetadataEntry {
            targets: Vec::new(),
            detail_pane: None,
            detail_sections: vec![FieldDetailSection {
                id: "test".to_string(),
                kind: "facts".to_string(),
                title: None,
                facts,
                targets: Vec::new(),
            }],
        }
    }

    #[test]
    fn semantic_terms_for_regions_use_region_identity_not_origin() {
        let entry = entry(vec![
            fact(FIELD_DETAIL_FACT_KEY_ORIGIN_REGION, "Origin", "Velia (R5)"),
            fact(FIELD_DETAIL_FACT_KEY_REGION, "Region", "Cron Castle (R42)"),
        ]);

        let primary = semantic_term_primary_fact("regions", &entry).unwrap();

        assert_eq!(primary.key, FIELD_DETAIL_FACT_KEY_REGION);
        assert_eq!(primary.value, "Cron Castle (R42)");
    }

    #[test]
    fn semantic_terms_for_region_groups_use_region_group_identity() {
        let entry = entry(vec![
            fact(FIELD_DETAIL_FACT_KEY_ORIGIN_REGION, "Origin", "Velia (R5)"),
            fact(
                FIELD_DETAIL_FACT_KEY_RESOURCE_GROUP,
                "Region Group",
                "Velia (RG1)",
            ),
        ]);

        let primary = semantic_term_primary_fact("region_groups", &entry).unwrap();

        assert_eq!(primary.key, FIELD_DETAIL_FACT_KEY_RESOURCE_GROUP);
        assert_eq!(primary.value, "Velia (RG1)");
    }
}
