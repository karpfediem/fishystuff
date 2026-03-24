use crate::bridge::contract::FishyMapSemanticTermSummary;
use fishystuff_core::field_metadata::{
    detail_facts, preferred_detail_fact, FieldHoverMetadataEntry,
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
    let primary = preferred_detail_fact(detail_facts(&entry.detail_sections))?;
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
