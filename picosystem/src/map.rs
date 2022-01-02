use crate::tile::Tile;

pub const INVALID_TILE: u16 = !0;
pub const NUM_LAYERS: usize = 4;

pub struct Map {
    pub width: usize,
    pub height: usize,
    pub tiles: &'static [MapTile],
    pub tile_functions: [fn() -> &'static Tile; 2048],
}

#[derive(Debug)]
pub struct MapTile {
    pub layers: [u16; NUM_LAYERS],
}
