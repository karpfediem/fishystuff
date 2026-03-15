use crate::map::camera::mode::ViewMode;
use crate::map::layers::{LayerId, LayerManifestStatus, LayerRegistry, LayerRuntime};
use crate::map::render::tile_z;
use crate::map::spaces::layer_transform::TileSpace;
use crate::map::spaces::world::MapToWorld;
use crate::prelude::*;

use super::super::super::policy::TileResidencyState;
use super::super::{RasterTileCache, TileState};
use super::geometry::tile_world_rect;

const DETAIL_LINGER_FRAMES: u64 = 24;

pub(crate) struct VisibilityUpdateContext<'a> {
    pub(crate) materials: &'a mut Assets<ColorMaterial>,
    pub(crate) layer_registry: &'a LayerRegistry,
    pub(crate) layer_runtime: &'a LayerRuntime,
    pub(crate) residency: &'a TileResidencyState,
    pub(crate) frame: u64,
    pub(crate) camera_unstable: bool,
    pub(crate) view_mode: ViewMode,
}

impl RasterTileCache {
    pub(crate) fn update_visibility(
        &mut self,
        commands: &mut Commands,
        context: VisibilityUpdateContext<'_>,
    ) -> std::collections::HashMap<LayerId, u32> {
        crate::perf_scope!("raster.tile_render_prep");
        let VisibilityUpdateContext {
            materials,
            layer_registry,
            layer_runtime,
            residency,
            frame,
            camera_unstable,
            view_mode,
        } = context;
        let mut visible_by_layer: std::collections::HashMap<LayerId, u32> =
            std::collections::HashMap::new();
        let map_to_world = MapToWorld::default();
        for (key, entry) in self.entries.iter_mut() {
            let Some(spec) = layer_registry.get(key.layer) else {
                continue;
            };
            let Some(layer_state) = layer_runtime.get(key.layer) else {
                continue;
            };
            let target_visible = layer_state.visible
                && entry.state == TileState::Ready
                && residency.render_visible.contains(key)
                && layer_state.manifest_status == LayerManifestStatus::Ready;
            let is_detail_tile = layer_state
                .current_base_lod
                .map(|base| key.z < i32::from(base))
                .unwrap_or(false);
            if entry.visible && !target_visible && camera_unstable && is_detail_tile {
                entry.linger_until_frame = frame.saturating_add(DETAIL_LINGER_FRAMES);
            } else if target_visible {
                entry.linger_until_frame = 0;
            }
            let linger_visible = !target_visible
                && is_detail_tile
                && frame <= entry.linger_until_frame
                && layer_state.visible
                && layer_state.manifest_status == LayerManifestStatus::Ready;
            let now_visible = target_visible || linger_visible;
            let alpha = layer_state.opacity.clamp(0.0, 1.0);
            let alpha_changed = (entry.alpha - alpha).abs() > f32::EPSILON;
            let next_depth = tile_z(layer_state.z_base, spec.max_level, key.z);
            let depth_changed = (entry.depth - next_depth).abs() > f32::EPSILON;
            entry.visible = now_visible;
            entry.alpha = alpha;
            entry.depth = next_depth;

            if let Some(entity) = entry.entity {
                let entity_visible = now_visible && view_mode == ViewMode::Map2D;
                commands.entity(entity).insert(if entity_visible {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                });
                if depth_changed {
                    if entry.exact_quad {
                        commands
                            .entity(entity)
                            .insert(Transform::from_translation(Vec3::new(0.0, 0.0, next_depth)));
                    } else if let Some(world_transform) = spec.world_transform(map_to_world) {
                        let tile_space = TileSpace::new(spec.tile_px, spec.y_flip);
                        if let Some((x0, y0, w, h)) =
                            tile_world_rect(key, spec, tile_space, world_transform)
                        {
                            commands
                                .entity(entity)
                                .insert(Transform::from_translation(Vec3::new(
                                    x0 + w * 0.5,
                                    y0 + h * 0.5,
                                    next_depth,
                                )));
                        }
                    }
                }
                if alpha_changed {
                    if entry.exact_quad {
                        if let Some(material) = entry.material.as_ref() {
                            if let Some(value) = materials.get_mut(material) {
                                value.color = Color::srgba(1.0, 1.0, 1.0, alpha);
                            }
                        }
                    } else {
                        commands.entity(entity).insert(Sprite {
                            image: entry.handle.clone(),
                            custom_size: entry.sprite_size,
                            color: Color::srgba(1.0, 1.0, 1.0, alpha),
                            ..default()
                        });
                    }
                }
            }

            if now_visible {
                self.use_counter = self.use_counter.wrapping_add(1);
                entry.last_used = self.use_counter;
                *visible_by_layer.entry(key.layer).or_insert(0) += 1;
            }
        }
        visible_by_layer
    }
}
