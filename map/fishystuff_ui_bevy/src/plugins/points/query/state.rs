use bevy::prelude::*;
use fishystuff_api::models::events::EventsQueryMode;

pub use crate::plugins::api::ZoneMembershipFilter as EvidenceZoneFilter;

#[derive(Resource)]
pub struct PointsState {
    pub status: String,
    pub points: Vec<RenderPoint>,
    pub total: usize,
    pub represented_sample_count: usize,
    pub mode: Option<EventsQueryMode>,
    pub bucket_px: Option<i32>,
    pub sample_step: usize,
    pub candidate_count: usize,
    pub rendered_point_count: usize,
    pub rendered_cluster_count: usize,
    pub spatial_bucket_px: i32,
    pub(in crate::plugins::points::query) request_sig: Option<PointsQuerySignature>,
    pub(in crate::plugins::points) dirty: bool,
    pub(in crate::plugins::points) icons_enabled: bool,
    pub(in crate::plugins::points) icon_size_world_units: f32,
}

impl Default for PointsState {
    fn default() -> Self {
        Self {
            status: "points: idle".to_string(),
            points: Vec::new(),
            total: 0,
            represented_sample_count: 0,
            mode: None,
            bucket_px: None,
            sample_step: 1,
            candidate_count: 0,
            rendered_point_count: 0,
            rendered_cluster_count: 0,
            spatial_bucket_px: 0,
            request_sig: None,
            dirty: false,
            icons_enabled: false,
            icon_size_world_units: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::plugins::points::query) struct PointsQuerySignature {
    pub(in crate::plugins::points::query) revision: Option<String>,
    pub(in crate::plugins::points::query) zone_filter_revision: u64,
    pub(in crate::plugins::points::query) zone_lookup_url: Option<String>,
    pub(in crate::plugins::points::query) zone_lookup_ready: bool,
    pub(in crate::plugins::points::query) from_ts_utc: i64,
    pub(in crate::plugins::points::query) to_ts_utc: i64,
    pub(in crate::plugins::points::query) fish_ids: Vec<i32>,
    pub(in crate::plugins::points::query) viewport_qmin_x: i32,
    pub(in crate::plugins::points::query) viewport_qmin_y: i32,
    pub(in crate::plugins::points::query) viewport_qmax_x: i32,
    pub(in crate::plugins::points::query) viewport_qmax_y: i32,
    pub(in crate::plugins::points::query) tile_scope_min_x: i32,
    pub(in crate::plugins::points::query) tile_scope_min_y: i32,
    pub(in crate::plugins::points::query) tile_scope_max_x: i32,
    pub(in crate::plugins::points::query) tile_scope_max_y: i32,
    pub(in crate::plugins::points::query) cluster_bucket_px: i32,
}

#[derive(Debug, Clone)]
pub struct RenderPoint {
    pub map_px_x: i32,
    pub map_px_y: i32,
    pub world_x: Option<i32>,
    pub world_z: Option<i32>,
    pub fish_id: Option<i32>,
    pub zone_rgb_u32: Option<u32>,
    pub sample_count: u32,
    pub aggregated: bool,
}
