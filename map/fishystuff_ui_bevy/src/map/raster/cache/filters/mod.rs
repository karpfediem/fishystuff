mod clip_mask;
mod compose;

use crate::map::camera::mode::ViewMode;
use crate::map::exact_lookup::ExactLookupCache;
use crate::map::layers::{LayerRegistry, LayerRuntime, PickMode};
use crate::plugins::points::EvidenceZoneFilter;
use crate::plugins::vector_layers::VectorLayerRuntime;
use crate::prelude::*;

use self::clip_mask::clip_mask_state_revision;
use self::compose::{
    compose_raster_visuals_in_place, restore_rgba_in_place, update_hover_highlight_in_place,
    RasterVisualComposeContext,
};
use super::{RasterTileCache, TileState};

pub(crate) struct VisualFilterContext<'a> {
    pub(crate) filter: &'a EvidenceZoneFilter,
    pub(crate) hover_zone_rgb: Option<u32>,
    pub(crate) layer_registry: &'a LayerRegistry,
    pub(crate) layer_runtime: &'a LayerRuntime,
    pub(crate) exact_lookups: &'a ExactLookupCache,
    pub(crate) vector_runtime: &'a VectorLayerRuntime,
    pub(crate) map_version: Option<&'a str>,
    pub(crate) view_mode: ViewMode,
}

impl RasterTileCache {
    pub(crate) fn sync_visual_filters(
        &mut self,
        images: &mut Assets<Image>,
        commands: &mut Commands,
        context: VisualFilterContext<'_>,
    ) {
        crate::perf_scope!("raster.sync_visual_filters");
        let VisualFilterContext {
            filter,
            hover_zone_rgb,
            layer_registry,
            layer_runtime,
            exact_lookups,
            vector_runtime,
            map_version,
            view_mode,
        } = context;
        let keys = self.entries.keys().copied().collect::<Vec<_>>();
        for key in keys {
            let Some(spec) = layer_registry.get(key.layer) else {
                continue;
            };
            let Some(read_entry) = self.entries.get(&key) else {
                continue;
            };
            if read_entry.state != TileState::Ready || !read_entry.visible {
                continue;
            }
            if read_entry.pixel_data.is_none() {
                continue;
            }
            let handle = read_entry.handle.clone();
            let entity = read_entry.entity;
            let previous_filter_active = read_entry.filter_active;
            let previous_filter_revision = read_entry.filter_revision;
            let previous_pixel_filtered = read_entry.pixel_filtered;
            let previous_hover_highlight_zone = read_entry.hover_highlight_zone;
            let previous_clip_mask_layer = read_entry.clip_mask_layer;
            let previous_clip_mask_revision = read_entry.clip_mask_revision;
            let previous_clip_mask_applied = read_entry.clip_mask_applied;
            let zone_rgbs = &read_entry.zone_rgbs;
            let source = match read_entry.pixel_data.as_ref() {
                Some(source) => source,
                None => continue,
            };
            let zone_lookup_rows = read_entry.zone_lookup_rows.as_ref();

            let apply_pick_filter =
                spec.pick_mode == PickMode::ExactTilePixel && view_mode == ViewMode::Map2D;
            let next_filter_active = apply_pick_filter && filter.active;
            let next_filter_revision = if next_filter_active {
                filter.revision
            } else {
                0
            };
            let has_intersection = if next_filter_active {
                zone_rgbs
                    .iter()
                    .any(|zone_rgb| filter.zone_rgbs.contains(zone_rgb))
            } else {
                true
            };
            let all_selected = next_filter_active
                && !zone_rgbs.is_empty()
                && zone_rgbs
                    .iter()
                    .all(|zone_rgb| filter.zone_rgbs.contains(zone_rgb));
            let target_hover_zone = if apply_pick_filter {
                hover_zone_rgb
                    .filter(|hover_rgb| zone_rgbs.iter().any(|zone_rgb| zone_rgb == hover_rgb))
            } else {
                None
            };
            let requires_pixel_filter = has_intersection && next_filter_active && !all_selected;
            let clip_mask_layer = layer_runtime.clip_mask_layer(key.layer);
            let clip_mask_revision =
                clip_mask_state_revision(layer_registry, layer_runtime, clip_mask_layer, filter);

            if apply_pick_filter {
                if let Some(entity) = entity {
                    commands.entity(entity).insert(if has_intersection {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    });
                }
            }

            let filter_state_same = previous_filter_active == next_filter_active
                && previous_filter_revision == next_filter_revision;
            let hover_state_same = previous_hover_highlight_zone == target_hover_zone;
            let clip_state_same = previous_clip_mask_layer == clip_mask_layer
                && previous_clip_mask_revision == clip_mask_revision
                && previous_clip_mask_applied == clip_mask_layer.is_some();
            if filter_state_same
                && hover_state_same
                && clip_state_same
                && previous_pixel_filtered == requires_pixel_filter
            {
                continue;
            }

            if apply_pick_filter && !has_intersection {
                if let Some(entry) = self.entries.get_mut(&key) {
                    entry.filter_active = next_filter_active;
                    entry.filter_revision = next_filter_revision;
                    entry.pixel_filtered = false;
                    entry.hover_highlight_zone = None;
                    entry.clip_mask_layer = clip_mask_layer;
                    entry.clip_mask_revision = clip_mask_revision;
                    entry.clip_mask_applied = clip_mask_layer.is_some();
                }
                continue;
            }

            let Some(image) = images.get_mut(&handle) else {
                continue;
            };
            let Some(image_data) = image.data.as_mut() else {
                continue;
            };
            if image_data.len() != source.data.len() {
                continue;
            }

            let hover_only_fast_path = !next_filter_active
                && !previous_filter_active
                && !previous_pixel_filtered
                && clip_mask_layer.is_none()
                && !previous_clip_mask_applied
                && zone_lookup_rows.is_some();

            if hover_only_fast_path {
                if let Some(zone_rows) = zone_lookup_rows {
                    update_hover_highlight_in_place(
                        source,
                        image_data,
                        zone_rows,
                        previous_hover_highlight_zone,
                        target_hover_zone,
                    );
                }
            } else if !requires_pixel_filter
                && target_hover_zone.is_none()
                && clip_mask_layer.is_none()
            {
                if previous_pixel_filtered
                    || previous_hover_highlight_zone.is_some()
                    || previous_clip_mask_applied
                {
                    restore_rgba_in_place(source, image_data);
                }
            } else {
                compose_raster_visuals_in_place(
                    source,
                    image_data,
                    &RasterVisualComposeContext {
                        key,
                        layer: spec,
                        filter,
                        requires_pixel_filter,
                        hover_zone_rgb: target_hover_zone,
                        clip_mask_layer,
                        layer_registry,
                        exact_lookups,
                        tile_cache: self,
                        vector_runtime,
                        map_version,
                    },
                );
            }

            if let Some(entry) = self.entries.get_mut(&key) {
                entry.filter_active = next_filter_active;
                entry.filter_revision = next_filter_revision;
                entry.pixel_filtered = requires_pixel_filter;
                entry.hover_highlight_zone = target_hover_zone;
                entry.clip_mask_layer = clip_mask_layer;
                entry.clip_mask_revision = clip_mask_revision;
                entry.clip_mask_applied = clip_mask_layer.is_some();
            }
        }
    }
}
