use crate::masks::WaterQuery;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapResult {
    pub water_ok: bool,
    pub water_px: Option<i32>,
    pub water_py: Option<i32>,
}

impl SnapResult {
    pub fn not_found() -> Self {
        Self {
            water_ok: false,
            water_px: None,
            water_py: None,
        }
    }

    pub fn found(px: i32, py: i32) -> Self {
        Self {
            water_ok: true,
            water_px: Some(px),
            water_py: Some(py),
        }
    }
}

pub fn snap_to_water<W: WaterQuery>(mask: &W, px: i32, py: i32, radius: i32) -> SnapResult {
    if mask.is_water_at_map_px(px, py) {
        return SnapResult::found(px, py);
    }
    if radius <= 0 {
        return SnapResult::not_found();
    }

    for r in 1..=radius {
        let mut best: Option<(i32, i32, i32)> = None; // (px, py, dist2)
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                let nx = px + dx;
                let ny = py + dy;
                if !mask.is_water_at_map_px(nx, ny) {
                    continue;
                }
                let dist2 = dx * dx + dy * dy;
                match best {
                    None => best = Some((nx, ny, dist2)),
                    Some((_, _, best_dist2)) => {
                        if dist2 < best_dist2 {
                            best = Some((nx, ny, dist2));
                        }
                    }
                }
            }
        }
        if let Some((nx, ny, _)) = best {
            return SnapResult::found(nx, ny);
        }
    }

    SnapResult::not_found()
}
