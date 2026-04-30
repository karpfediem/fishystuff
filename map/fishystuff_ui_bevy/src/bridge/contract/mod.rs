mod events;
mod input;
mod normalize;
mod search;
mod snapshot;

pub use events::FishyMapOutputEvent;
pub use input::{
    FishyMapBookmarkEntry, FishyMapCommands, FishyMapFiltersPatch, FishyMapFiltersState,
    FishyMapInputState, FishyMapStatePatch, FishyMapThemeColors, FishyMapThemePatch,
    FishyMapThemeState, FishyMapUiPatch, FishyMapUiState, FishyMapViewMode,
    FishyMapWorldPointCommand,
};
pub use normalize::{
    normalize_i32_list, normalize_layer_clip_mask_map, normalize_layer_opacity_map,
    normalize_string_list, normalize_u32_list, normalize_u32_map,
};
pub use search::{
    deserialize_search_expression_field, deserialize_search_expression_state,
    normalize_fish_filter_term, normalize_fish_filter_terms, selected_search_terms_from_expression,
    FishyMapPatchBound, FishyMapSearchExpressionNode, FishyMapSearchExpressionOperator,
    FishyMapSearchProjection, FishyMapSearchTerm, FishyMapSharedFishState,
};
pub use snapshot::{
    FishyMapCameraSnapshot, FishyMapCatalogSnapshot, FishyMapEffectiveFiltersSnapshot,
    FishyMapEffectiveSemanticFieldFilterSnapshot, FishyMapEffectiveZoneMembershipFilterSnapshot,
    FishyMapFishSummary, FishyMapHoverLayerSampleSnapshot, FishyMapHoverSnapshot,
    FishyMapLayerFilterBindingSummary, FishyMapLayerSummary, FishyMapPatchSummary,
    FishyMapPointSampleSnapshot, FishyMapSelectionPointKind, FishyMapSelectionSnapshot,
    FishyMapSemanticTermSummary, FishyMapStateSnapshot, FishyMapStatusSnapshot,
    FishyMapViewSnapshot, FishyMapZoneConfidenceSnapshot, FishyMapZoneDriftSnapshot,
    FishyMapZoneEvidenceEntrySnapshot, FishyMapZoneStatsSnapshot, FishyMapZoneWindowSnapshot,
};

pub const FISHYMAP_CONTRACT_VERSION: u8 = 1;
pub const FISHYMAP_POINT_ICON_SCALE_MIN: f32 = 1.0;
pub const FISHYMAP_POINT_ICON_SCALE_MAX: f32 = 5.0;
pub const FISHYMAP_POINT_ICON_SCALE_DEFAULT: f32 = 2.0;

fn default_contract_version() -> u8 {
    FISHYMAP_CONTRACT_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn state_patch_deserializes_versioned_shape() {
        let patch: FishyMapStatePatch = serde_json::from_str(
            r#"{
                "version": 1,
                "filters": {
                    "fishIds": [10, 20],
                    "zoneRgbs": [1193046, 1193046, 6636321],
                    "fromPatchId": "2026-02-26",
                    "toPatchId": "2026-03-12",
                    "layerIdsOrdered": ["zones", "terrain", "minimap"],
                    "layerFilterBindingIdsDisabledByLayer": {
                        "fish_evidence": ["zone_selection"],
                        "regions": ["fish_selection"]
                    },
                    "layerOpacities": {
                        "zones": 0.8,
                        "terrain": 0.35
                    },
                    "layerClipMasks": {
                        "terrain": "zones"
                    }
                },
                "ui": {
                    "showPoints": false,
                    "showPointIcons": true,
                    "pointIconScale": 2.25,
                    "bookmarks": [
                        {
                            "id": "bookmark-a",
                            "label": "Marker A",
                            "worldX": 123.5,
                            "worldZ": -456.25
                        }
                    ]
                },
                "commands": {
                    "setViewMode": "2d"
                }
            }"#,
        )
        .expect("patch");

        assert_eq!(patch.version, FISHYMAP_CONTRACT_VERSION);
        assert_eq!(
            patch
                .filters
                .as_ref()
                .and_then(|filters| filters.fish_ids.clone()),
            Some(vec![10, 20])
        );
        assert_eq!(
            patch
                .filters
                .as_ref()
                .and_then(|filters| filters.zone_rgbs.clone()),
            Some(vec![1193046, 1193046, 6636321])
        );
        assert_eq!(
            patch
                .filters
                .as_ref()
                .and_then(|filters| filters.from_patch_id.clone()),
            Some(Some("2026-02-26".to_string()))
        );
        assert_eq!(
            patch
                .filters
                .as_ref()
                .and_then(|filters| filters.to_patch_id.clone()),
            Some(Some("2026-03-12".to_string()))
        );
        assert_eq!(
            patch
                .filters
                .as_ref()
                .and_then(|filters| filters.layer_ids_ordered.clone()),
            Some(vec![
                "zones".to_string(),
                "terrain".to_string(),
                "minimap".to_string(),
            ])
        );
        assert_eq!(
            patch
                .filters
                .as_ref()
                .and_then(|filters| filters.layer_filter_binding_ids_disabled_by_layer.clone()),
            Some(BTreeMap::from([
                (
                    "fish_evidence".to_string(),
                    vec!["zone_selection".to_string()],
                ),
                ("regions".to_string(), vec!["fish_selection".to_string()]),
            ]))
        );
        assert_eq!(
            patch
                .filters
                .as_ref()
                .and_then(|filters| filters.layer_opacities.clone()),
            Some(BTreeMap::from([
                ("terrain".to_string(), 0.35),
                ("zones".to_string(), 0.8),
            ]))
        );
        assert_eq!(
            patch
                .filters
                .as_ref()
                .and_then(|filters| filters.layer_clip_masks.clone()),
            Some(BTreeMap::from([(
                "terrain".to_string(),
                "zones".to_string(),
            )]))
        );
        assert_eq!(
            patch
                .commands
                .as_ref()
                .and_then(|commands| commands.set_view_mode),
            Some(FishyMapViewMode::TwoD)
        );
        assert_eq!(
            patch.ui.as_ref().and_then(|ui| ui.point_icon_scale),
            Some(2.25)
        );
        assert_eq!(patch.ui.as_ref().and_then(|ui| ui.show_points), Some(false));
        assert_eq!(
            patch.ui.as_ref().and_then(|ui| ui.show_point_icons),
            Some(true)
        );
        assert_eq!(
            patch.ui.as_ref().map(|ui| ui.bookmarks.clone()),
            Some(Some(vec![FishyMapBookmarkEntry {
                id: "bookmark-a".to_string(),
                label: Some("Marker A".to_string()),
                point_label: None,
                world_x: 123.5,
                world_z: -456.25,
                layer_samples: Vec::new(),
                zone_rgb: None,
                created_at: None,
            }]))
        );
    }

    #[test]
    fn apply_patch_updates_one_mode_without_clobbering_others() {
        let mut state = FishyMapInputState::default();
        state.filters.search_text = "coel".to_string();
        state.filters.from_patch_id = Some("2026-02-26".to_string());
        state.filters.to_patch_id = Some("2026-03-12".to_string());

        let commands = state.apply_patch(
            serde_json::from_str(
                r#"{
                    "version": 1,
                    "ui": {
                        "diagnosticsOpen": true,
                        "showPoints": false,
                        "showPointIcons": false,
                        "pointIconScale": 9.0,
                        "bookmarks": [
                            {
                                "id": "bookmark-a",
                                "label": "Marker A",
                                "worldX": 123.5,
                                "worldZ": -456.25
                            }
                        ]
                    },
                    "commands": {
                        "setViewMode": "2d"
                    }
                }"#,
            )
            .expect("patch"),
        );

        assert!(state.ui.diagnostics_open);
        assert!(!state.ui.show_points);
        assert!(!state.ui.show_point_icons);
        assert_eq!(state.ui.point_icon_scale, FISHYMAP_POINT_ICON_SCALE_MAX);
        assert_eq!(state.ui.bookmarks.len(), 1);
        assert_eq!(state.ui.bookmarks[0].id, "bookmark-a");
        assert_eq!(state.filters.search_text, "coel");
        assert_eq!(state.filters.from_patch_id.as_deref(), Some("2026-02-26"));
        assert_eq!(state.filters.to_patch_id.as_deref(), Some("2026-03-12"));
        assert_eq!(state.filters.patch_id, None);
        assert_eq!(commands.set_view_mode, Some(FishyMapViewMode::TwoD));
    }

    #[test]
    fn repeated_patches_are_idempotent_for_owned_state() {
        let patch: FishyMapStatePatch = serde_json::from_str(
            r#"{
                "version": 1,
                "filters": {
                    "fishIds": [821015, 821015, 42],
                    "zoneRgbs": [1193046, 1193046, 6636321],
                    "layerIdsVisible": ["zones", "zones", "terrain"],
                    "layerIdsOrdered": ["zones", "terrain", "zones", "minimap"],
                    "layerFilterBindingIdsDisabledByLayer": {
                        "fish_evidence": [" zone_selection ", "zone_selection"],
                        "regions": ["fish_selection", ""]
                    },
                    "layerOpacities": {
                        "zones": 1.2,
                        " terrain ": 0.25,
                        "": 0.7
                    },
                    "layerClipMasks": {
                        "terrain": "zones",
                        "": "terrain",
                        "region_groups": "  "
                    }
                }
            }"#,
        )
        .expect("patch");

        let mut state = FishyMapInputState::default();
        state.apply_patch(patch.clone());
        let once = state.clone();
        state.apply_patch(patch);

        assert_eq!(state, once);
        assert_eq!(state.filters.fish_ids, vec![821015, 42]);
        assert_eq!(state.filters.zone_rgbs, vec![1193046, 6636321]);
        assert_eq!(
            state.filters.layer_ids_visible,
            Some(vec!["zones".to_string(), "terrain".to_string()])
        );
        assert_eq!(
            state.filters.layer_ids_ordered,
            Some(vec![
                "zones".to_string(),
                "terrain".to_string(),
                "minimap".to_string(),
            ])
        );
        assert_eq!(
            state.filters.layer_filter_binding_ids_disabled_by_layer,
            Some(BTreeMap::from([
                (
                    "fish_evidence".to_string(),
                    vec!["zone_selection".to_string()],
                ),
                ("regions".to_string(), vec!["fish_selection".to_string()]),
            ]))
        );
        assert_eq!(
            state.filters.layer_opacities,
            Some(BTreeMap::from([
                ("terrain".to_string(), 0.25),
                ("zones".to_string(), 1.0),
            ]))
        );
        assert_eq!(
            state.filters.layer_clip_masks,
            Some(BTreeMap::from([(
                "terrain".to_string(),
                "zones".to_string(),
            )]))
        );
    }

    #[test]
    fn empty_layer_opacity_override_map_clears_existing_overrides() {
        let mut state = FishyMapInputState::default();
        state.filters.layer_opacities = Some(BTreeMap::from([("zones".to_string(), 0.4)]));

        state.apply_patch(
            serde_json::from_str(
                r#"{
                    "version": 1,
                    "filters": {
                        "layerOpacities": {}
                    }
                }"#,
            )
            .expect("patch"),
        );

        assert_eq!(state.filters.layer_opacities, None);
    }

    #[test]
    fn empty_layer_clip_mask_map_clears_existing_overrides() {
        let mut state = FishyMapInputState::default();
        state.filters.layer_clip_masks = Some(BTreeMap::from([(
            "terrain".to_string(),
            "zones".to_string(),
        )]));

        state.apply_patch(
            serde_json::from_str(
                r#"{
                    "version": 1,
                    "filters": {
                        "layerClipMasks": {}
                    }
                }"#,
            )
            .expect("patch"),
        );

        assert_eq!(state.filters.layer_clip_masks, None);
    }

    #[test]
    fn empty_layer_filter_binding_override_map_clears_existing_overrides() {
        let mut state = FishyMapInputState::default();
        state.filters.layer_filter_binding_ids_disabled_by_layer = Some(BTreeMap::from([(
            "fish_evidence".to_string(),
            vec!["zone_selection".to_string()],
        )]));

        state.apply_patch(
            serde_json::from_str(
                r#"{
                    "version": 1,
                    "filters": {
                        "layerFilterBindingIdsDisabledByLayer": {}
                    }
                }"#,
            )
            .expect("patch"),
        );

        assert_eq!(
            state.filters.layer_filter_binding_ids_disabled_by_layer,
            None
        );
    }

    #[test]
    fn patch_id_can_be_cleared_with_null() {
        let mut state = FishyMapInputState::default();
        state.filters.patch_id = Some("2026-02-26".to_string());
        state.filters.from_patch_id = Some("2026-02-26".to_string());
        state.filters.to_patch_id = Some("2026-02-26".to_string());

        state.apply_patch(
            serde_json::from_str(
                r#"{
                    "version": 1,
                    "filters": {
                        "patchId": null
                    }
                }"#,
            )
            .expect("patch"),
        );

        assert_eq!(state.filters.patch_id, None);
        assert_eq!(state.filters.from_patch_id, None);
        assert_eq!(state.filters.to_patch_id, None);
    }

    #[test]
    fn legacy_patch_id_alias_expands_to_exact_range() {
        let mut state = FishyMapInputState::default();

        state.apply_patch(
            serde_json::from_str(
                r#"{
                    "version": 1,
                    "filters": {
                        "patchId": "2026-02-26"
                    }
                }"#,
            )
            .expect("patch"),
        );

        assert_eq!(state.filters.patch_id.as_deref(), Some("2026-02-26"));
        assert_eq!(state.filters.from_patch_id.as_deref(), Some("2026-02-26"));
        assert_eq!(state.filters.to_patch_id.as_deref(), Some("2026-02-26"));
    }

    #[test]
    fn patch_range_canonicalizes_legacy_alias_to_none_for_multi_patch_selection() {
        let mut state = FishyMapInputState::default();
        state.filters.patch_id = Some("2026-02-26".to_string());
        state.filters.from_patch_id = Some("2026-02-26".to_string());
        state.filters.to_patch_id = Some("2026-02-26".to_string());

        state.apply_patch(
            serde_json::from_str(
                r#"{
                    "version": 1,
                    "filters": {
                        "fromPatchId": "2026-02-26",
                        "toPatchId": "2026-03-12"
                    }
                }"#,
            )
            .expect("patch"),
        );

        assert_eq!(state.filters.patch_id, None);
        assert_eq!(state.filters.from_patch_id.as_deref(), Some("2026-02-26"));
        assert_eq!(state.filters.to_patch_id.as_deref(), Some("2026-03-12"));
    }

    #[test]
    fn bookmark_patches_are_normalized_and_replace_existing_entries() {
        let mut state = FishyMapInputState::default();
        state.ui.bookmarks = vec![FishyMapBookmarkEntry {
            id: "existing".to_string(),
            label: Some("Existing".to_string()),
            point_label: None,
            world_x: 10.0,
            world_z: 20.0,
            layer_samples: Vec::new(),
            zone_rgb: None,
            created_at: None,
        }];

        state.apply_patch(
            serde_json::from_str(
                r#"{
                    "version": 1,
                    "ui": {
                        "bookmarks": [
                            {
                                "id": " bookmark-a ",
                                "label": " Marker A ",
                                "worldX": 123.5,
                                "worldZ": -456.25
                            },
                            {
                                "id": "bookmark-a",
                                "worldX": 999,
                                "worldZ": 999
                            },
                            {
                                "id": "",
                                "worldX": 1,
                                "worldZ": 2
                            }
                        ]
                    }
                }"#,
            )
            .expect("patch"),
        );

        assert_eq!(
            state.ui.bookmarks,
            vec![FishyMapBookmarkEntry {
                id: "bookmark-a".to_string(),
                label: Some("Marker A".to_string()),
                point_label: None,
                world_x: 123.5,
                world_z: -456.25,
                layer_samples: Vec::new(),
                zone_rgb: None,
                created_at: None,
            }]
        );
    }

    #[test]
    fn restore_view_command_deserializes_with_camera_payload() {
        let patch: FishyMapStatePatch = serde_json::from_str(
            r#"{
                "version": 1,
                "commands": {
                    "restoreView": {
                        "viewMode": "2d",
                        "camera": {
                            "pivotWorldX": 10.5,
                            "pivotWorldY": 15.0,
                            "pivotWorldZ": -20.25,
                            "yaw": 0.5,
                            "pitch": -0.7,
                            "distance": 4200.0
                        }
                    }
                }
            }"#,
        )
        .expect("patch");

        let restore_view = patch
            .commands
            .expect("commands")
            .restore_view
            .expect("restore view");
        assert_eq!(restore_view.view_mode, FishyMapViewMode::TwoD);
        assert_eq!(restore_view.camera.pivot_world_x, Some(10.5));
        assert_eq!(restore_view.camera.distance, Some(4200.0));
    }

    #[test]
    fn output_event_serializes_browser_facing_kebab_case_tag() {
        let json = serde_json::to_string(&FishyMapOutputEvent::SelectionChanged {
            version: 1,
            world_x: Some(10.0),
            world_z: Some(20.0),
            point_kind: Some(FishyMapSelectionPointKind::Waypoint),
            point_label: Some("Olvia Academy".to_string()),
            layer_samples: Vec::new(),
            point_samples: Vec::new(),
        })
        .expect("serialize");

        assert!(json.contains(r#""type":"selection-changed""#));
        assert!(json.contains(r#""worldX":10.0"#));
        assert!(json.contains(r#""worldZ":20.0"#));
        assert!(json.contains(r#""pointKind":"waypoint""#));
        assert!(json.contains(r#""pointLabel":"Olvia Academy""#));
    }

    #[test]
    fn state_snapshot_serializes_zone_stats_in_selection() {
        let mut snapshot = FishyMapStateSnapshot::default();
        snapshot.selection.zone_stats = Some(FishyMapZoneStatsSnapshot {
            zone_rgb: 0x123456,
            zone_name: Some("Coastal Shelf".to_string()),
            window: FishyMapZoneWindowSnapshot {
                from_ts_utc: 10,
                to_ts_utc: 20,
                tile_px: 32,
                sigma_tiles: 3.0,
                alpha0: 1.0,
                ..FishyMapZoneWindowSnapshot::default()
            },
            confidence: FishyMapZoneConfidenceSnapshot {
                ess: 12.5,
                total_weight: 18.0,
                status: "FRESH".to_string(),
                ..FishyMapZoneConfidenceSnapshot::default()
            },
            distribution: vec![FishyMapZoneEvidenceEntrySnapshot {
                fish_id: 821015,
                item_id: 821015,
                encyclopedia_key: Some(1015),
                encyclopedia_id: Some(9015),
                fish_name: Some("Blue Bat Star".to_string()),
                evidence_weight: 1.25,
                p_mean: 0.42,
                ci_low: Some(0.35),
                ci_high: Some(0.49),
            }],
        });

        let json = serde_json::to_string(&snapshot).expect("serialize");

        assert!(json.contains(r#""zoneStats":{"#));
        assert!(json.contains(r#""zoneRgb":1193046"#));
        assert!(json.contains(r#""fishId":821015"#));
        assert!(json.contains(r#""status":"FRESH""#));
    }
}
