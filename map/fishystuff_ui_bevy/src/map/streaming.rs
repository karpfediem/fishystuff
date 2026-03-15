use std::cmp::Ordering;

use bevy::prelude::Resource;

use crate::config::{MAX_INFLIGHT, START_BUDGET_PER_FRAME};
use crate::map::layers::LayerId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestKind {
    PickProbe,
    BaseCoverage,
    DetailRefine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileKey {
    pub layer: LayerId,
    pub map_version: u64,
    pub z: i32,
    pub tx: i32,
    pub ty: i32,
}

#[derive(Debug, Clone)]
pub struct TileRequest {
    pub key: TileKey,
    pub url: String,
    pub priority: f32,
    pub kind: RequestKind,
}

#[derive(Resource, Debug)]
pub struct TileStreamer {
    pending_pick: Vec<TileRequest>,
    cursor_pick: usize,
    pending_base: Vec<TileRequest>,
    cursor_base: usize,
    last_base_layer: Option<LayerId>,
    pending_detail: Vec<TileRequest>,
    cursor_detail: usize,
    last_detail_layer: Option<LayerId>,
    pub inflight: usize,
    pub max_inflight: usize,
    pub max_new_requests_per_frame: usize,
}

impl Default for TileStreamer {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl TileStreamer {
    pub fn clear(&mut self) {
        self.pending_pick.clear();
        self.cursor_pick = 0;
        self.pending_base.clear();
        self.cursor_base = 0;
        self.last_base_layer = None;
        self.pending_detail.clear();
        self.cursor_detail = 0;
        self.last_detail_layer = None;
        self.inflight = 0;
    }

    pub fn clear_layer(&mut self, layer_id: LayerId) {
        self.pending_pick.retain(|req| req.key.layer != layer_id);
        self.pending_base.retain(|req| req.key.layer != layer_id);
        self.pending_detail.retain(|req| req.key.layer != layer_id);
        self.cursor_pick = 0;
        self.cursor_base = 0;
        self.last_base_layer = None;
        self.cursor_detail = 0;
        self.last_detail_layer = None;
    }

    pub fn replace_layer(&mut self, layer_id: LayerId, mut requests: Vec<TileRequest>) {
        self.pending_pick.retain(|req| req.key.layer != layer_id);
        self.pending_base.retain(|req| req.key.layer != layer_id);
        self.pending_detail.retain(|req| req.key.layer != layer_id);
        for req in requests.drain(..) {
            match req.kind {
                RequestKind::PickProbe => self.pending_pick.push(req),
                RequestKind::BaseCoverage => self.pending_base.push(req),
                RequestKind::DetailRefine => self.pending_detail.push(req),
            }
        }
        self.pending_pick.sort_by(request_order);
        self.pending_base.sort_by(request_order);
        self.pending_detail.sort_by(request_order);
        self.cursor_pick = 0;
        self.cursor_base = 0;
        self.last_base_layer = None;
        self.cursor_detail = 0;
        self.last_detail_layer = None;
    }

    pub fn push_request(&mut self, request: TileRequest) {
        match request.kind {
            RequestKind::PickProbe => self.pending_pick.push(request),
            RequestKind::BaseCoverage => self.pending_base.push(request),
            RequestKind::DetailRefine => self.pending_detail.push(request),
        }
        self.pending_pick.sort_by(request_order);
        self.pending_base.sort_by(request_order);
        self.pending_detail.sort_by(request_order);
        self.cursor_pick = self.cursor_pick.min(self.pending_pick.len());
        self.cursor_base = self.cursor_base.min(self.pending_base.len());
        self.cursor_detail = self.cursor_detail.min(self.pending_detail.len());
    }

    pub fn has_queued_key(&self, key: &TileKey) -> bool {
        self.pending_pick[self.cursor_pick..]
            .iter()
            .any(|req| req.key == *key)
            || self.pending_base[self.cursor_base..]
                .iter()
                .any(|req| req.key == *key)
            || self.pending_detail[self.cursor_detail..]
                .iter()
                .any(|req| req.key == *key)
    }

    pub fn next_request(&mut self) -> Option<TileRequest> {
        if self.inflight >= self.max_inflight {
            return None;
        }
        if self.cursor_pick < self.pending_pick.len() {
            let req = self.pending_pick[self.cursor_pick].clone();
            self.cursor_pick += 1;
            return Some(req);
        }
        if self.cursor_base < self.pending_base.len() {
            let req = take_with_layer_fairness(
                &mut self.pending_base,
                &mut self.cursor_base,
                &mut self.last_base_layer,
                24,
            );
            if let Some(req) = req {
                return Some(req);
            }
        }
        if self.cursor_detail < self.pending_detail.len() {
            let req = take_with_layer_fairness(
                &mut self.pending_detail,
                &mut self.cursor_detail,
                &mut self.last_detail_layer,
                24,
            );
            if let Some(req) = req {
                return Some(req);
            }
        }
        None
    }

    pub fn pending_len(&self) -> usize {
        self.pending_pick
            .len()
            .saturating_sub(self.cursor_pick)
            .saturating_add(self.pending_base.len().saturating_sub(self.cursor_base))
            .saturating_add(self.pending_detail.len().saturating_sub(self.cursor_detail))
    }
}

fn take_with_layer_fairness(
    pending: &mut [TileRequest],
    cursor: &mut usize,
    last_layer: &mut Option<LayerId>,
    lookahead: usize,
) -> Option<TileRequest> {
    if *cursor >= pending.len() {
        return None;
    }

    let current_layer = pending[*cursor].key.layer;
    let mut pick_idx = *cursor;
    if Some(current_layer) == *last_layer {
        let end = (*cursor + lookahead).min(pending.len());
        if let Some(found_idx) =
            ((*cursor + 1)..end).find(|idx| pending[*idx].key.layer != current_layer)
        {
            pick_idx = found_idx;
        }
    }

    if pick_idx != *cursor {
        pending.swap(*cursor, pick_idx);
    }
    let req = pending[*cursor].clone();
    *cursor += 1;
    *last_layer = Some(req.key.layer);
    Some(req)
}

fn request_order(lhs: &TileRequest, rhs: &TileRequest) -> Ordering {
    let priority_order = lhs.priority.total_cmp(&rhs.priority);
    if priority_order != Ordering::Equal {
        return priority_order;
    }
    let kind_order = kind_rank(lhs.kind).cmp(&kind_rank(rhs.kind));
    if kind_order != Ordering::Equal {
        return kind_order;
    }
    lhs.url.cmp(&rhs.url)
}

fn kind_rank(kind: RequestKind) -> u8 {
    match kind {
        RequestKind::PickProbe => 0,
        RequestKind::BaseCoverage => 1,
        RequestKind::DetailRefine => 2,
    }
}

impl TileStreamer {
    pub fn pending_len_for_layer(&self, layer_id: LayerId) -> usize {
        let pick = self.pending_pick[self.cursor_pick..]
            .iter()
            .filter(|req| req.key.layer == layer_id)
            .count();
        let base = self.pending_base[self.cursor_base..]
            .iter()
            .filter(|req| req.key.layer == layer_id)
            .count();
        let detail = self.pending_detail[self.cursor_detail..]
            .iter()
            .filter(|req| req.key.layer == layer_id)
            .count();
        pick + base + detail
    }
}

impl TileStreamer {
    pub fn with_defaults() -> Self {
        Self {
            pending_pick: Vec::new(),
            cursor_pick: 0,
            pending_base: Vec::new(),
            cursor_base: 0,
            last_base_layer: None,
            pending_detail: Vec::new(),
            cursor_detail: 0,
            last_detail_layer: None,
            inflight: 0,
            max_inflight: MAX_INFLIGHT,
            max_new_requests_per_frame: START_BUDGET_PER_FRAME,
        }
    }
}
