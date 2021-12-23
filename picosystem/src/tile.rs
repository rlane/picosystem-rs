pub const TILE_SIZE: i32 = 32;

pub struct Tile {
    pub data: &'static [u16],
    pub mask: &'static [u32],
}
