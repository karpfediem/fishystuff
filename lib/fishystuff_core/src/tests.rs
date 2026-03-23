use std::f64::consts::LN_2;

use crate::constants::{LEFT, SECTOR_PER_PIXEL, SECTOR_SCALE, TOP};
use crate::coord::{pixel_in_bounds, pixel_to_world, world_to_pixel_f, world_to_pixel_round};
use crate::gaussian::gaussian_blur_grid;
use crate::masks::{
    pack_rgb_u32, unpack_rgb_u32, WaterMask, WaterSampler, ZoneLookupRows, ZoneMask,
};
use crate::prob::{dirichlet_posterior_mean, js_divergence};
use crate::snap::snap_to_water;
use crate::transform::{MapToWaterTransform, TransformKind};
use image::RgbImage;

#[test]
fn world_pixel_roundtrip() {
    let samples = [(0, 0), (11559, 10539), (100, 200), (5000, 8000)];
    for (px, py) in samples {
        assert!(pixel_in_bounds(px, py));
        let (wx, wz) = pixel_to_world(px as f64, py as f64);
        let (fx, fy) = world_to_pixel_f(wx, wz);
        assert!((fx - px as f64).abs() < 1e-6);
        assert!((fy - py as f64).abs() < 1e-6);
        let back = world_to_pixel_round(wx, wz);
        assert_eq!(back.x, px);
        assert_eq!(back.y, py);
    }
}

#[test]
fn watermask_exact_rgb() {
    let data = vec![0, 0, 255, 0, 0, 254];
    let mask = WaterMask::from_rgb(2, 1, data).expect("mask");
    assert!(mask.is_water(0, 0));
    assert!(!mask.is_water(1, 0));
}

#[test]
fn snapping_deterministic_tie_break() {
    // 5x5, water at (2,0) and (0,2); snap from (2,2)
    let mut data = vec![0u8; 5 * 5 * 3];
    let idx_a = (0 * 5 + 2) * 3;
    data[idx_a] = 0;
    data[idx_a + 1] = 0;
    data[idx_a + 2] = 255;
    let idx_b = (2 * 5 + 0) * 3;
    data[idx_b] = 0;
    data[idx_b + 1] = 0;
    data[idx_b + 2] = 255;
    let mask = WaterMask::from_rgb(5, 5, data).expect("mask");
    let snap = snap_to_water(&mask, 2, 2, 2);
    assert!(snap.water_ok);
    assert_eq!(snap.water_px, Some(2));
    assert_eq!(snap.water_py, Some(0));
}

#[test]
fn gaussian_blur_constant_grid() {
    let input = vec![2.0f32; 9];
    let output = gaussian_blur_grid(&input, 3, 3, 1.0);
    for v in output {
        assert!((v - 2.0).abs() < 1e-6);
    }
}

#[test]
fn jsd_properties() {
    let p = vec![0.2f64, 0.8f64];
    let q = vec![0.6f64, 0.4f64];
    let js1 = js_divergence(&p, &p);
    assert!(js1.abs() < 1e-12);
    let js2 = js_divergence(&p, &q);
    let js3 = js_divergence(&q, &p);
    assert!((js2 - js3).abs() < 1e-12);
    assert!(js2 <= LN_2 + 1e-12);
}

#[test]
fn dirichlet_mean_sums_to_one() {
    let p0 = vec![0.25f64, 0.75f64];
    let counts = vec![1.0f64, 1.0f64];
    let mean = dirichlet_posterior_mean(1.0, &p0, &counts);
    let sum: f64 = mean.iter().sum();
    assert!((sum - 1.0).abs() < 1e-12);
}

#[test]
fn scale_to_fit_mapping() {
    let t = TransformKind::ScaleToFit {
        map_w: 4,
        map_h: 4,
        water_w: 2,
        water_h: 2,
    };
    let (wx0, wy0) = t.map_to_water(0.0, 0.0);
    let (wx1, wy1) = t.map_to_water(3.0, 3.0);
    assert!((wx0 - 0.0).abs() < 1e-6);
    assert!((wy0 - 0.0).abs() < 1e-6);
    assert!((wx1 - 1.0).abs() < 1e-6);
    assert!((wy1 - 1.0).abs() < 1e-6);
}

#[test]
fn world_extent_mapping() {
    let map_x0 = 100.0;
    let map_x1 = 200.0;
    let map_y0 = 50.0;
    let map_y1 = 150.0;
    let world_left = (LEFT + map_x0 * SECTOR_PER_PIXEL) * SECTOR_SCALE;
    let world_right = (LEFT + map_x1 * SECTOR_PER_PIXEL) * SECTOR_SCALE;
    let world_top = (TOP - (map_y0 + 1.0) * SECTOR_PER_PIXEL) * SECTOR_SCALE;
    let world_bottom = (TOP - (map_y1 + 1.0) * SECTOR_PER_PIXEL) * SECTOR_SCALE;
    let t = TransformKind::WorldExtent {
        world_left,
        world_right,
        world_bottom,
        world_top,
        map_pixel_center_offset: 1.0,
        water_w: 101,
        water_h: 101,
    };
    let (wx0, wy0) = t.map_to_water(map_x0, map_y0);
    let (wx1, wy1) = t.map_to_water(map_x1, map_y1);
    assert!((wx0 - 0.0).abs() < 1e-6);
    assert!((wy0 - 0.0).abs() < 1e-6);
    assert!((wx1 - 100.0).abs() < 1e-6);
    assert!((wy1 - 100.0).abs() < 1e-6);
}

#[test]
fn water_sampler_scale_offset() {
    let mut img = RgbImage::new(2, 2);
    // water at (1,0)
    img.put_pixel(1, 0, image::Rgb([0, 0, 255]));
    let sampler = WaterSampler::from_image(
        img,
        TransformKind::ScaleOffset {
            sx: 1.0,
            sy: 1.0,
            ox: 1.0,
            oy: 0.0,
        },
    );
    assert!(sampler.is_water_at_map_px(0, 0));
    assert!(!sampler.is_water_at_map_px(0, 1));
}

#[test]
fn water_sampler_bilinear_projection_rgb() {
    let mut img = RgbImage::new(2, 2);
    img.put_pixel(0, 0, image::Rgb([0, 0, 255]));
    img.put_pixel(1, 0, image::Rgb([255, 0, 0]));
    img.put_pixel(0, 1, image::Rgb([0, 255, 0]));
    img.put_pixel(1, 1, image::Rgb([255, 255, 255]));
    let sampler = WaterSampler::from_image(
        img,
        TransformKind::ScaleToFit {
            map_w: 3,
            map_h: 3,
            water_w: 2,
            water_h: 2,
        },
    );
    assert_eq!(sampler.sample_rgb_bilinear_at_map_px(0.0, 0.0), [0, 0, 255]);
    assert_eq!(
        sampler.sample_rgb_bilinear_at_map_px(1.0, 1.0),
        [128, 128, 128]
    );
}

#[test]
fn rgb_pack_unpack_roundtrip() {
    let rgb = pack_rgb_u32(12, 34, 56);
    let (r, g, b) = unpack_rgb_u32(rgb);
    assert_eq!((r, g, b), (12, 34, 56));
}

#[test]
fn zonemask_sample_clamped() {
    // 2x2: top-left red, top-right green, bottom-left blue, bottom-right white
    let data = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];
    let mask = ZoneMask::from_rgb(2, 2, data).expect("mask");
    let rgb = mask.sample_rgb_u32_clamped(-5, -5);
    assert_eq!(rgb, pack_rgb_u32(255, 0, 0));
    let rgb = mask.sample_rgb_u32_clamped(99, 99);
    assert_eq!(rgb, pack_rgb_u32(255, 255, 255));
}

#[test]
fn zone_lookup_rows_roundtrip_and_sample() {
    let data = vec![
        1, 2, 3, 1, 2, 3, 4, 5, 6, 4, 5, 6, //
        7, 8, 9, 7, 8, 9, 4, 5, 6, 4, 5, 6,
    ];
    let mask = ZoneMask::from_rgb(4, 2, data).expect("mask");
    let lookup = mask.to_lookup_rows().expect("lookup");
    assert_eq!(lookup.segment_count(), 4);
    assert_eq!(lookup.rgb_u32(0, 0), Some(pack_rgb_u32(1, 2, 3)));
    assert_eq!(lookup.rgb_u32(2, 0), Some(pack_rgb_u32(4, 5, 6)));
    assert_eq!(lookup.rgb_u32(1, 1), Some(pack_rgb_u32(7, 8, 9)));
    assert_eq!(lookup.rgb_u32(4, 0), None);

    let bytes = lookup.to_bytes();
    let decoded = ZoneLookupRows::from_bytes(&bytes).expect("decode");
    assert_eq!(decoded, lookup);
    assert_eq!(decoded.sample_rgb_u32_clamped(-4, 9), pack_rgb_u32(7, 8, 9));
}

#[test]
fn zone_lookup_rows_from_rgba_matches_zone_mask_rows() {
    let rgb_data = vec![
        1, 2, 3, 1, 2, 3, 4, 5, 6, 4, 5, 6, //
        7, 8, 9, 7, 8, 9, 4, 5, 6, 4, 5, 6,
    ];
    let rgba_data = vec![
        1, 2, 3, 255, 1, 2, 3, 255, 4, 5, 6, 255, 4, 5, 6, 255, //
        7, 8, 9, 255, 7, 8, 9, 255, 4, 5, 6, 255, 4, 5, 6, 255,
    ];
    let mask = ZoneMask::from_rgb(4, 2, rgb_data).expect("mask");
    let from_mask = mask.to_lookup_rows().expect("mask rows");
    let from_rgba = ZoneLookupRows::from_rgba(4, 2, &rgba_data).expect("rgba rows");

    assert_eq!(from_rgba, from_mask);
}

#[test]
fn zone_lookup_rows_matching_spans_return_expected_ranges() {
    let rgba_data = vec![
        1, 2, 3, 255, 1, 2, 3, 255, 4, 5, 6, 255, 4, 5, 6, 255, //
        4, 5, 6, 255, 7, 8, 9, 255, 7, 8, 9, 255, 4, 5, 6, 255,
    ];
    let lookup = ZoneLookupRows::from_rgba(4, 2, &rgba_data).expect("rgba rows");
    let mut spans = Vec::new();
    lookup.for_each_span_matching(pack_rgb_u32(4, 5, 6), |row, start_x, end_x| {
        spans.push((row, start_x, end_x));
    });

    assert_eq!(spans, vec![(0, 2, 4), (1, 0, 1), (1, 3, 4)]);
}
