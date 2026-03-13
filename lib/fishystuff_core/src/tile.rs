pub fn pixel_to_tile(px: i32, py: i32, tile_px: i32) -> (i32, i32) {
    (px / tile_px, py / tile_px)
}

pub fn tile_dimensions(width: i32, height: i32, tile_px: i32) -> (i32, i32) {
    let tiles_x = (width + tile_px - 1) / tile_px;
    let tiles_y = (height + tile_px - 1) / tile_px;
    (tiles_x, tiles_y)
}
