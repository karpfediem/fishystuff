use crate::map::spaces::layer_transform::{TileSpace, WorldTransform};
use crate::map::spaces::LayerPoint;
use crate::map::streaming::TileKey;
use crate::prelude::*;

pub fn tile_map_corners_from_key(
    key: &TileKey,
    tile_space: TileSpace,
    world_transform: WorldTransform,
) -> Option<[Vec2; 4]> {
    let corners = tile_layer_corners(key.tx, key.ty, key.z, tile_space)?;
    let mut map = [Vec2::ZERO; 4];
    for (idx, corner) in corners.into_iter().enumerate() {
        let m = world_transform.layer_to_map(corner);
        map[idx] = Vec2::new(m.x as f32, m.y as f32);
    }
    Some(map)
}

fn tile_layer_corners(
    tile_x: i32,
    tile_y: i32,
    z: i32,
    tile_space: TileSpace,
) -> Option<[LayerPoint; 4]> {
    let tile_span = tile_space.tile_span_px(z)?;
    let x0 = tile_x as f64 * tile_span;
    let x1 = x0 + tile_span;
    let y0 = tile_y as f64 * tile_span;
    let y1 = y0 + tile_span;

    if tile_space.y_flip {
        Some([
            LayerPoint::new(x0, y1),
            LayerPoint::new(x1, y1),
            LayerPoint::new(x1, y0),
            LayerPoint::new(x0, y0),
        ])
    } else {
        Some([
            LayerPoint::new(x0, y0),
            LayerPoint::new(x1, y0),
            LayerPoint::new(x1, y1),
            LayerPoint::new(x0, y1),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::tile_map_corners_from_key;
    use crate::map::layers::LayerId;
    use crate::map::spaces::layer_transform::{LayerTransform, TileSpace, WorldTransform};
    use crate::map::spaces::world::MapToWorld;
    use crate::map::streaming::TileKey;

    #[test]
    fn tile_corners_follow_tile_space_orientation() {
        let key = TileKey {
            layer: LayerId::from_raw(1),
            map_version: 1,
            tx: 1,
            ty: 2,
            z: 3,
        };
        let corners = tile_map_corners_from_key(
            &key,
            TileSpace {
                tile_px: 256,
                y_flip: false,
            },
            WorldTransform::new(LayerTransform::IdentityMapSpace, MapToWorld::default())
                .expect("transform"),
        )
        .expect("corners");

        assert_eq!(corners[0].x, 2048.0);
        assert_eq!(corners[0].y, 4096.0);
        assert_eq!(corners[2].x, 4096.0);
        assert_eq!(corners[2].y, 6144.0);
    }
}
