use fishystuff_api::models::meta::PatchInfo;
use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::bridge::contract::{FishyMapSearchExpressionNode, FishyMapSharedFishState};
use crate::map::layers::{LayerFilterBindingSpec, LayerSpec};

use crate::prelude::*;

pub type Patch = PatchInfo;
pub const POINT_ICON_SCALE_MIN: f32 = 1.0;
pub const POINT_ICON_SCALE_DEFAULT: f32 = 2.0;
pub const POINT_ICON_SCALE_MAX: f32 = 5.0;

#[derive(Resource, Default)]
pub struct PatchFilterState {
    pub from_ts: Option<i64>,
    pub to_ts: Option<i64>,
    pub patches: Vec<Patch>,
    pub selected_patch: Option<String>,
}

#[derive(Resource, Default)]
pub struct FishFilterState {
    pub selected_fish_ids: Vec<i32>,
}

#[derive(Resource, Default)]
pub struct SemanticFieldFilterState {
    pub selected_field_ids_by_layer: BTreeMap<String, Vec<u32>>,
}

#[derive(Resource, Default)]
pub struct SearchExpressionState {
    pub expression: FishyMapSearchExpressionNode,
    pub shared_fish_state: FishyMapSharedFishState,
}

#[derive(Resource, Default)]
pub struct LayerFilterBindingOverrideState {
    pub disabled_binding_ids_by_layer: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct ZoneMembershipFilter {
    pub active: bool,
    pub zone_rgbs: HashSet<u32>,
    pub revision: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SemanticFieldSelectionFilter {
    pub active: bool,
    pub field_ids: Vec<u32>,
    pub revision: u64,
}

#[derive(Resource, Default)]
pub struct LayerEffectiveFilterState {
    zone_membership_by_layer: BTreeMap<String, ZoneMembershipFilter>,
    semantic_field_filters_by_layer: BTreeMap<String, SemanticFieldSelectionFilter>,
}

impl LayerFilterBindingOverrideState {
    pub fn set_disabled_binding_ids_by_layer(
        &mut self,
        disabled_binding_ids_by_layer: BTreeMap<String, Vec<String>>,
    ) {
        let mut next = BTreeMap::new();
        for (layer_id_raw, binding_ids) in disabled_binding_ids_by_layer {
            let layer_id = layer_id_raw.trim();
            if layer_id.is_empty() {
                continue;
            }
            let normalized = binding_ids
                .into_iter()
                .map(|binding_id| binding_id.trim().to_string())
                .filter(|binding_id| !binding_id.is_empty())
                .collect::<BTreeSet<_>>();
            if normalized.is_empty() {
                continue;
            }
            next.insert(layer_id.to_string(), normalized);
        }
        self.disabled_binding_ids_by_layer = next;
    }

    pub fn disabled_binding_ids_for_layer(&self, layer_id: &str) -> Vec<String> {
        self.disabled_binding_ids_by_layer
            .get(layer_id.trim())
            .map(|binding_ids| binding_ids.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn is_binding_enabled(&self, layer: &LayerSpec, binding: &LayerFilterBindingSpec) -> bool {
        if !binding.default_enabled {
            return false;
        }
        !self
            .disabled_binding_ids_by_layer
            .get(layer.key.as_str())
            .is_some_and(|disabled| disabled.contains(binding.binding_id.as_str()))
    }
}

impl SemanticFieldFilterState {
    pub const ZONE_MASK_LAYER_ID: &str = "zone_mask";

    pub fn field_ids_for_layer(&self, layer_id: &str) -> &[u32] {
        self.selected_field_ids_by_layer
            .get(layer_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn set_field_ids_for_layer(
        &mut self,
        layer_id: impl Into<String>,
        mut field_ids: Vec<u32>,
    ) {
        field_ids.sort_unstable();
        field_ids.dedup();
        let layer_id = layer_id.into();
        if field_ids.is_empty() {
            self.selected_field_ids_by_layer.remove(&layer_id);
            return;
        }
        self.selected_field_ids_by_layer.insert(layer_id, field_ids);
    }

    pub fn selected_zone_rgbs(&self) -> &[u32] {
        self.field_ids_for_layer(Self::ZONE_MASK_LAYER_ID)
    }
}

impl LayerEffectiveFilterState {
    pub fn zone_membership_filter(&self, layer_id: &str) -> Option<&ZoneMembershipFilter> {
        self.zone_membership_by_layer.get(layer_id.trim())
    }

    pub fn semantic_field_filter(&self, layer_id: &str) -> Option<&SemanticFieldSelectionFilter> {
        self.semantic_field_filters_by_layer.get(layer_id.trim())
    }

    pub fn semantic_field_ids_for_layer(&self, layer_id: &str) -> &[u32] {
        self.semantic_field_filters_by_layer
            .get(layer_id.trim())
            .map(|filter| filter.field_ids.as_slice())
            .unwrap_or(&[])
    }

    pub fn sync_zone_membership_filter_for_layer(
        &mut self,
        layer_id: impl Into<String>,
        next_active: bool,
        next_zone_rgbs: HashSet<u32>,
    ) {
        let layer_id = layer_id.into();
        let filter = self.zone_membership_by_layer.entry(layer_id).or_default();
        if filter.active != next_active || filter.zone_rgbs != next_zone_rgbs {
            filter.active = next_active;
            filter.zone_rgbs = next_zone_rgbs;
            filter.revision = filter.revision.wrapping_add(1);
        }
    }

    pub fn sync_semantic_field_filter_for_layer(
        &mut self,
        layer_id: impl Into<String>,
        next_active: bool,
        mut field_ids: Vec<u32>,
    ) {
        field_ids.sort_unstable();
        field_ids.dedup();
        let layer_id = layer_id.into();
        let filter = self
            .semantic_field_filters_by_layer
            .entry(layer_id)
            .or_default();
        if filter.active != next_active || filter.field_ids != field_ids {
            filter.active = next_active;
            filter.field_ids = field_ids;
            filter.revision = filter.revision.wrapping_add(1);
        }
    }

    pub fn sync_to_registry(&mut self, layer_registry: &crate::map::layers::LayerRegistry) {
        let active_layer_ids = layer_registry
            .ordered()
            .iter()
            .map(|layer| layer.key.clone())
            .collect::<BTreeSet<_>>();
        self.zone_membership_by_layer
            .retain(|layer_id, _| active_layer_ids.contains(layer_id));
        self.semantic_field_filters_by_layer
            .retain(|layer_id, _| active_layer_ids.contains(layer_id));
        for layer in layer_registry.ordered() {
            if !self
                .zone_membership_by_layer
                .contains_key(layer.key.as_str())
                && layer.zone_membership_filter_bindings().next().is_some()
            {
                self.zone_membership_by_layer
                    .insert(layer.key.clone(), ZoneMembershipFilter::default());
            }
            if !self
                .semantic_field_filters_by_layer
                .contains_key(layer.key.as_str())
                && layer.semantic_selection_filter_bindings().next().is_some()
            {
                self.semantic_field_filters_by_layer
                    .insert(layer.key.clone(), SemanticFieldSelectionFilter::default());
            }
        }
    }

    pub fn resolve_zone_membership_filter_for_layer(
        &mut self,
        layer: &LayerSpec,
        overrides: &LayerFilterBindingOverrideState,
        fish_selection_filter: &ZoneMembershipFilter,
        semantic_filter: &SemanticFieldFilterState,
    ) {
        let explicit_zone_rgbs = semantic_filter
            .selected_zone_rgbs()
            .iter()
            .copied()
            .collect::<HashSet<_>>();
        let mut active_sources = Vec::new();
        for binding in layer.zone_membership_filter_bindings() {
            if !overrides.is_binding_enabled(layer, binding) {
                continue;
            }
            match binding.source {
                crate::map::layers::LayerFilterSourceKind::FishSelection
                    if fish_selection_filter.active =>
                {
                    active_sources.push(fish_selection_filter.zone_rgbs.clone());
                }
                crate::map::layers::LayerFilterSourceKind::ZoneSelection
                    if !explicit_zone_rgbs.is_empty() =>
                {
                    active_sources.push(explicit_zone_rgbs.clone());
                }
                _ => {}
            }
        }
        let mut next_zone_rgbs = active_sources.into_iter();
        let Some(mut intersection) = next_zone_rgbs.next() else {
            self.sync_zone_membership_filter_for_layer(layer.key.clone(), false, HashSet::new());
            return;
        };
        for source_zones in next_zone_rgbs {
            intersection.retain(|zone_rgb| source_zones.contains(zone_rgb));
        }
        self.sync_zone_membership_filter_for_layer(layer.key.clone(), true, intersection);
    }

    pub fn resolve_semantic_field_filter_for_layer(
        &mut self,
        layer: &LayerSpec,
        overrides: &LayerFilterBindingOverrideState,
        semantic_filter: &SemanticFieldFilterState,
    ) {
        let active = layer
            .semantic_selection_filter_bindings()
            .any(|binding| overrides.is_binding_enabled(layer, binding));
        let next_field_ids = if active {
            semantic_filter
                .field_ids_for_layer(layer.key.as_str())
                .to_vec()
        } else {
            Vec::new()
        };
        self.sync_semantic_field_filter_for_layer(layer.key.clone(), active, next_field_ids);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashSet};

    use crate::map::layers::{
        LayerFilterBindingSpec, LayerFilterSourceKind, LayerFilterTargetKind,
        LAYER_FILTER_BINDING_ZONE_SELECTION,
    };

    use super::{
        LayerEffectiveFilterState, LayerFilterBindingOverrideState, SemanticFieldFilterState,
        ZoneMembershipFilter,
    };

    #[test]
    fn semantic_field_filter_state_normalizes_and_clears_layer_ids() {
        let mut filter = SemanticFieldFilterState::default();
        filter.set_field_ids_for_layer("regions", vec![8, 3, 8, 5]);
        assert_eq!(filter.field_ids_for_layer("regions"), &[3, 5, 8]);

        filter.set_field_ids_for_layer("regions", Vec::new());
        assert!(filter.field_ids_for_layer("regions").is_empty());
    }

    #[test]
    fn semantic_field_filter_state_exposes_zone_mask_ids() {
        let mut filter = SemanticFieldFilterState::default();
        filter.set_field_ids_for_layer(SemanticFieldFilterState::ZONE_MASK_LAYER_ID, vec![9, 4]);
        assert_eq!(filter.selected_zone_rgbs(), &[4, 9]);
    }

    #[test]
    fn layer_filter_binding_override_state_normalizes_and_clears_layer_ids() {
        let mut overrides = LayerFilterBindingOverrideState::default();
        overrides.set_disabled_binding_ids_by_layer(BTreeMap::from([(
            " fish_evidence ".to_string(),
            vec![
                format!(" {} ", LAYER_FILTER_BINDING_ZONE_SELECTION),
                "".to_string(),
                LAYER_FILTER_BINDING_ZONE_SELECTION.to_string(),
            ],
        )]));

        assert_eq!(
            overrides.disabled_binding_ids_for_layer("fish_evidence"),
            vec![LAYER_FILTER_BINDING_ZONE_SELECTION.to_string()]
        );

        overrides.set_disabled_binding_ids_by_layer(BTreeMap::new());
        assert!(overrides
            .disabled_binding_ids_for_layer("fish_evidence")
            .is_empty());
    }

    #[test]
    fn layer_effective_filter_state_intersects_active_zone_sources() {
        let layer = crate::map::layers::LayerSpec {
            id: crate::map::layers::LayerId::from_raw(1),
            key: "fish_evidence".to_string(),
            name: "Fish Evidence".to_string(),
            visible_default: true,
            opacity_default: 1.0,
            z_base: 0.0,
            kind: crate::map::layers::LayerKind::Waypoints,
            tileset_url: String::new(),
            tile_url_template: String::new(),
            tileset_version: String::new(),
            vector_source: None,
            waypoint_source: None,
            transform: crate::map::spaces::layer_transform::LayerTransform::IdentityMapSpace,
            tile_px: 0,
            max_level: 0,
            y_flip: false,
            field_source: None,
            field_metadata_source: None,
            filter_bindings: vec![
                LayerFilterBindingSpec {
                    binding_id: "fish_selection".to_string(),
                    target: LayerFilterTargetKind::ZoneMembership,
                    source: LayerFilterSourceKind::FishSelection,
                    default_enabled: true,
                },
                LayerFilterBindingSpec {
                    binding_id: "zone_selection".to_string(),
                    target: LayerFilterTargetKind::ZoneMembership,
                    source: LayerFilterSourceKind::ZoneSelection,
                    default_enabled: true,
                },
            ],
            lod_policy: crate::map::layers::LodPolicy {
                target_tiles: 1,
                hysteresis_hi: 1.0,
                hysteresis_lo: 0.0,
                margin_tiles: 0,
                enable_refine: false,
                refine_debounce_ms: 0,
                max_detail_tiles: 0,
                max_resident_tiles: 1,
                pinned_coarse_levels: 0,
                coarse_pin_min_level: None,
                warm_margin_tiles: 0,
                protected_margin_tiles: 0,
                detail_eviction_weight: 1.0,
                max_detail_requests_while_camera_moving: 1,
                motion_suppresses_refine: true,
            },
            request_weight: 1.0,
            pick_mode: crate::map::layers::PickMode::None,
            display_order: 0,
        };
        let mut semantic_filter = SemanticFieldFilterState::default();
        semantic_filter.set_field_ids_for_layer("zone_mask", vec![0x222222, 0x333333]);
        let fish_filter = ZoneMembershipFilter {
            active: true,
            zone_rgbs: HashSet::from([0x111111, 0x222222]),
            revision: 1,
        };
        let mut effective = LayerEffectiveFilterState::default();

        effective.resolve_zone_membership_filter_for_layer(
            &layer,
            &LayerFilterBindingOverrideState::default(),
            &fish_filter,
            &semantic_filter,
        );

        let resolved = effective
            .zone_membership_filter("fish_evidence")
            .expect("layer filter");
        assert!(resolved.active);
        assert_eq!(resolved.zone_rgbs, HashSet::from([0x222222]));
    }

    #[test]
    fn layer_effective_filter_state_keeps_empty_zone_intersections_active() {
        let layer = crate::map::layers::LayerSpec {
            id: crate::map::layers::LayerId::from_raw(1),
            key: "fish_evidence".to_string(),
            name: "Fish Evidence".to_string(),
            visible_default: true,
            opacity_default: 1.0,
            z_base: 0.0,
            kind: crate::map::layers::LayerKind::Waypoints,
            tileset_url: String::new(),
            tile_url_template: String::new(),
            tileset_version: String::new(),
            vector_source: None,
            waypoint_source: None,
            transform: crate::map::spaces::layer_transform::LayerTransform::IdentityMapSpace,
            tile_px: 0,
            max_level: 0,
            y_flip: false,
            field_source: None,
            field_metadata_source: None,
            filter_bindings: vec![
                LayerFilterBindingSpec {
                    binding_id: "fish_selection".to_string(),
                    target: LayerFilterTargetKind::ZoneMembership,
                    source: LayerFilterSourceKind::FishSelection,
                    default_enabled: true,
                },
                LayerFilterBindingSpec {
                    binding_id: "zone_selection".to_string(),
                    target: LayerFilterTargetKind::ZoneMembership,
                    source: LayerFilterSourceKind::ZoneSelection,
                    default_enabled: true,
                },
            ],
            lod_policy: crate::map::layers::LodPolicy {
                target_tiles: 1,
                hysteresis_hi: 1.0,
                hysteresis_lo: 0.0,
                margin_tiles: 0,
                enable_refine: false,
                refine_debounce_ms: 0,
                max_detail_tiles: 0,
                max_resident_tiles: 1,
                pinned_coarse_levels: 0,
                coarse_pin_min_level: None,
                warm_margin_tiles: 0,
                protected_margin_tiles: 0,
                detail_eviction_weight: 1.0,
                max_detail_requests_while_camera_moving: 1,
                motion_suppresses_refine: true,
            },
            request_weight: 1.0,
            pick_mode: crate::map::layers::PickMode::None,
            display_order: 0,
        };
        let mut semantic_filter = SemanticFieldFilterState::default();
        semantic_filter.set_field_ids_for_layer("zone_mask", vec![0x333333]);
        let fish_filter = ZoneMembershipFilter {
            active: true,
            zone_rgbs: HashSet::from([0x111111, 0x222222]),
            revision: 1,
        };
        let mut effective = LayerEffectiveFilterState::default();

        effective.resolve_zone_membership_filter_for_layer(
            &layer,
            &LayerFilterBindingOverrideState::default(),
            &fish_filter,
            &semantic_filter,
        );

        let resolved = effective
            .zone_membership_filter("fish_evidence")
            .expect("layer filter");
        assert!(resolved.active);
        assert!(resolved.zone_rgbs.is_empty());
    }
}

#[derive(Resource)]
pub struct MapDisplayState {
    pub show_effort: bool,
    pub show_points: bool,
    pub show_point_icons: bool,
    pub point_icon_scale: f32,
    pub point_z_base: f32,
    pub show_drift: bool,
    pub show_zone_mask: bool,
    pub zone_mask_opacity: f32,
    pub hovered_zone_rgb: Option<u32>,
}

impl Default for MapDisplayState {
    fn default() -> Self {
        Self {
            show_effort: true,
            show_points: true,
            show_point_icons: true,
            point_icon_scale: POINT_ICON_SCALE_DEFAULT,
            point_z_base: 40.0,
            show_drift: false,
            show_zone_mask: true,
            zone_mask_opacity: 0.55,
            hovered_zone_rgb: None,
        }
    }
}
