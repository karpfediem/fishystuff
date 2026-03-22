use crate::map::layers::GeometrySpace;
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::{MapPoint, WorldPoint};

#[derive(Debug, Clone)]
pub struct PolygonPiece {
    pub color_rgba: [u8; 4],
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub centroid_map: [f64; 2],
}

#[derive(Debug, Clone)]
pub struct ProjectedPolygon {
    pub world_rings: Vec<Vec<[f32; 2]>>,
    pub centroid_map: [f64; 2],
}

pub fn validate_polygon_piece(piece: &PolygonPiece) -> Result<(), String> {
    if piece.positions.len() < 3 {
        return Err("polygon piece has fewer than 3 vertices".to_string());
    }
    if piece.indices.len() < 3 || !piece.indices.len().is_multiple_of(3) {
        return Err("polygon piece has invalid triangle index count".to_string());
    }
    for index in &piece.indices {
        let idx = *index as usize;
        if idx >= piece.positions.len() {
            return Err(format!(
                "polygon piece index {} out of bounds for {} vertices",
                idx,
                piece.positions.len()
            ));
        }
    }
    Ok(())
}

pub fn triangle_count(piece: &PolygonPiece) -> usize {
    piece.indices.len() / 3
}

pub fn triangulate_polygon(
    rings: &[Vec<[f64; 2]>],
    geometry_space: GeometrySpace,
    map_to_world: MapToWorld,
) -> Result<Option<PolygonPiece>, String> {
    let Some(projected) = project_polygon(rings, geometry_space, map_to_world) else {
        return Ok(None);
    };
    triangulate_projected_polygon(&projected)
}

pub fn project_polygon(
    rings: &[Vec<[f64; 2]>],
    geometry_space: GeometrySpace,
    map_to_world: MapToWorld,
) -> Option<ProjectedPolygon> {
    crate::perf_scope!("vector.triangulation");
    if rings.is_empty() {
        return None;
    }

    let mut world_rings = Vec::<Vec<[f32; 2]>>::new();
    let mut map_points = Vec::<[f64; 2]>::new();

    for ring in rings {
        let Some(cleaned) = prepare_ring(ring) else {
            continue;
        };
        let mut world_ring = Vec::with_capacity(cleaned.len());
        for point in cleaned {
            let (map_x, map_y, world_x, world_z) = match geometry_space {
                GeometrySpace::MapPixels => {
                    let map = MapPoint::new(point[0], point[1]);
                    let world = map_to_world.map_to_world(map);
                    (map.x, map.y, world.x, world.z)
                }
                GeometrySpace::World => {
                    let world = WorldPoint::new(point[0], point[1]);
                    let map = map_to_world.world_to_map(world);
                    (map.x, map.y, world.x, world.z)
                }
            };
            map_points.push([map_x, map_y]);
            world_ring.push([world_x as f32, world_z as f32]);
        }
        if !world_ring.is_empty() {
            world_rings.push(world_ring);
        }
    }

    if world_rings.iter().map(Vec::len).sum::<usize>() < 3 {
        return None;
    }

    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    for point in &map_points {
        sum_x += point[0];
        sum_y += point[1];
    }

    Some(ProjectedPolygon {
        world_rings,
        centroid_map: [
            sum_x / map_points.len() as f64,
            sum_y / map_points.len() as f64,
        ],
    })
}

pub fn triangulate_projected_polygon(
    projected: &ProjectedPolygon,
) -> Result<Option<PolygonPiece>, String> {
    let mut flat_world = Vec::<f64>::new();
    let mut positions = Vec::<[f32; 3]>::new();
    let mut hole_indices = Vec::<usize>::new();
    let mut vertex_count = 0usize;

    for (ring_index, ring) in projected.world_rings.iter().enumerate() {
        if ring_index > 0 {
            hole_indices.push(vertex_count);
        }
        for point in ring {
            flat_world.push(point[0] as f64);
            flat_world.push(point[1] as f64);
            positions.push([point[0], point[1], 0.0]);
            vertex_count += 1;
        }
    }

    if vertex_count < 3 {
        return Ok(None);
    }

    let triangulated = earcutr::earcut(&flat_world, &hole_indices, 2)
        .map_err(|err| format!("earcut triangulation failed: {err}"))?;
    if triangulated.len() < 3 {
        return Ok(None);
    }

    let mut indices = Vec::with_capacity(triangulated.len());
    for index in triangulated {
        let value = u32::try_from(index).map_err(|_| "polygon index overflow".to_string())?;
        indices.push(value);
    }

    Ok(Some(PolygonPiece {
        color_rgba: [0, 0, 0, 0],
        positions,
        indices,
        centroid_map: projected.centroid_map,
    }))
}

fn prepare_ring(ring: &[[f64; 2]]) -> Option<Vec<[f64; 2]>> {
    let mut out: Vec<[f64; 2]> = Vec::with_capacity(ring.len());
    for point in ring {
        if !point[0].is_finite() || !point[1].is_finite() {
            continue;
        }
        if out
            .last()
            .map(|last| {
                (last[0] - point[0]).abs() < f64::EPSILON
                    && (last[1] - point[1]).abs() < f64::EPSILON
            })
            .unwrap_or(false)
        {
            continue;
        }
        out.push(*point);
    }

    if out.len() < 3 {
        return None;
    }

    if let (Some(first), Some(last)) = (out.first().copied(), out.last().copied()) {
        if (first[0] - last[0]).abs() < f64::EPSILON && (first[1] - last[1]).abs() < f64::EPSILON {
            out.pop();
        }
    }

    if out.len() < 3 {
        return None;
    }

    Some(out)
}

#[cfg(test)]
mod tests {
    use super::{triangle_count, triangulate_polygon, validate_polygon_piece};
    use crate::map::layers::GeometrySpace;
    use crate::map::spaces::world::MapToWorld;
    use crate::map::spaces::{MapPoint, WorldPoint};

    #[test]
    fn triangulates_simple_polygon() {
        let rings = vec![vec![
            [0.0, 0.0],
            [16.0, 0.0],
            [16.0, 16.0],
            [0.0, 16.0],
            [0.0, 0.0],
        ]];
        let piece = triangulate_polygon(&rings, GeometrySpace::MapPixels, MapToWorld::default())
            .expect("triangulate")
            .expect("piece");
        validate_polygon_piece(&piece).expect("piece must be valid");
        assert_eq!(piece.positions.len(), 4);
        assert_eq!(piece.indices.len(), 6);
        assert_eq!(triangle_count(&piece), 2);
    }

    #[test]
    fn triangulates_multipolygon_with_combined_triangle_count() {
        let polygons = vec![
            vec![vec![
                [0.0, 0.0],
                [10.0, 0.0],
                [10.0, 10.0],
                [0.0, 10.0],
                [0.0, 0.0],
            ]],
            vec![vec![
                [20.0, 20.0],
                [30.0, 20.0],
                [30.0, 30.0],
                [20.0, 30.0],
                [20.0, 20.0],
            ]],
        ];

        let mut triangles = 0usize;
        for rings in polygons {
            let piece =
                triangulate_polygon(&rings, GeometrySpace::MapPixels, MapToWorld::default())
                    .expect("triangulate")
                    .expect("piece");
            validate_polygon_piece(&piece).expect("piece must be valid");
            triangles += triangle_count(&piece);
        }
        assert_eq!(triangles, 4);
    }

    #[test]
    fn triangulates_polygon_with_hole() {
        let rings = vec![
            vec![
                [0.0, 0.0],
                [20.0, 0.0],
                [20.0, 20.0],
                [0.0, 20.0],
                [0.0, 0.0],
            ],
            vec![
                [5.0, 5.0],
                [15.0, 5.0],
                [15.0, 15.0],
                [5.0, 15.0],
                [5.0, 5.0],
            ],
        ];
        let piece = triangulate_polygon(&rings, GeometrySpace::MapPixels, MapToWorld::default())
            .expect("triangulate")
            .expect("piece");
        validate_polygon_piece(&piece).expect("piece must be valid");
        assert!(piece.positions.len() >= 8);
        assert!(triangle_count(&piece) >= 4);
    }

    #[test]
    fn map_pixels_to_world_uses_canonical_map_to_world() {
        let rings = vec![vec![
            [128.0, 256.0],
            [132.0, 256.0],
            [132.0, 260.0],
            [128.0, 260.0],
        ]];
        let map_to_world = MapToWorld::default();
        let piece = triangulate_polygon(&rings, GeometrySpace::MapPixels, map_to_world)
            .expect("triangulate")
            .expect("piece");
        validate_polygon_piece(&piece).expect("piece must be valid");
        let expected = map_to_world.map_to_world(MapPoint::new(128.0, 256.0));
        let first = piece.positions[0];
        assert!((first[0] as f64 - expected.x).abs() < 0.1);
        assert!((first[1] as f64 - expected.z).abs() < 0.1);
    }

    #[test]
    fn world_geometry_space_roundtrips_through_map_projection() {
        let map_to_world = MapToWorld::default();
        let w0 = map_to_world.map_to_world(MapPoint::new(7000.0, 5000.0));
        let w1 = map_to_world.map_to_world(MapPoint::new(7004.0, 5000.0));
        let w2 = map_to_world.map_to_world(MapPoint::new(7004.0, 5004.0));
        let w3 = map_to_world.map_to_world(MapPoint::new(7000.0, 5004.0));
        let rings = vec![vec![
            [w0.x, w0.z],
            [w1.x, w1.z],
            [w2.x, w2.z],
            [w3.x, w3.z],
            [w0.x, w0.z],
        ]];
        let piece = triangulate_polygon(&rings, GeometrySpace::World, map_to_world)
            .expect("triangulate")
            .expect("piece");
        validate_polygon_piece(&piece).expect("piece must be valid");
        let first_world =
            WorldPoint::new(piece.positions[0][0] as f64, piece.positions[0][1] as f64);
        let first_map = map_to_world.world_to_map(first_world);
        assert!((first_map.x - 7000.0).abs() < 0.5);
        assert!((first_map.y - 5000.0).abs() < 0.5);
    }
}
