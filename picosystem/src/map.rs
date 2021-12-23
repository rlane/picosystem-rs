use crate::tile::Tile;

pub const MAP_SIZE: usize = 100;
pub const INVALID_TILE: u16 = !0;
pub const NUM_LAYERS: usize = 4;

pub struct Map {
    pub tiles: [MapTile; MAP_SIZE * MAP_SIZE],
    pub tile_functions: [fn() -> &'static Tile; 2048],
}

#[derive(Debug)]
pub struct MapTile {
    pub layers: [u16; NUM_LAYERS],
}
