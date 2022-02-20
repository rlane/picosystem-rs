use embedded_graphics::image::Image;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use log::info;
use picosystem::display::{HEIGHT, WIDTH};
use picosystem::fps_monitor::FpsMonitor;
use picosystem::hardware;
use picosystem::map::{Map, MapTile, INVALID_TILE};
use picosystem::tile::{self, GenMapTile, TILE_SIZE};
use picosystem_macros::{atlas, map, sprite};

atlas!(atlas, "games/src/mathemagic/terrain_atlas.png", 32);

sprite!(protagonist, "games/src/mathemagic/lidia.png", 576);

sprite!(
    slime,
    "games/assets/slime/slime_monster_spritesheet.png",
    72
);

const _: &[u8] = include_bytes!("map.tmx");
map!(worldmap, "games/src/mathemagic/map.tmx");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    North,
    South,
    East,
    West,
}

fn generate_map(position: Point) -> GenMapTile {
    let map = worldmap();

    let ocean_tiles = [
        atlas451(),
        atlas452(),
        atlas453(),
        atlas454(),
        atlas455(),
        atlas456(),
        atlas456(),
        atlas456(),
        atlas456(),
        atlas456(),
        atlas456(),
        atlas456(),
        atlas456(),
        atlas456(),
        atlas456(),
        atlas456(),
    ];

    use hash32::{Hash, Hasher};
    let mut hasher = hash32::Murmur3Hasher::default();
    position.x.hash(&mut hasher);
    position.y.hash(&mut hasher);
    let hash = hasher.finish();
    let map_x = position.x / TILE_SIZE;
    let map_y = position.y / TILE_SIZE;
    let mut layers = heapless::Vec::new();

    if (0..(map.width as i32)).contains(&map_x) && (0..(map.height as i32)).contains(&map_y) {
        let index = (map_x + map_y * map.width as i32) as usize;
        for tile_index in map.tiles[index].layers {
            if tile_index != INVALID_TILE {
                let _ = layers.push(map.tile_functions[tile_index as usize]());
            }
        }
    }

    if layers.is_empty() {
        let _ = layers.push(ocean_tiles[hash as usize % ocean_tiles.len()]);
    }

    GenMapTile { layers }
}

pub fn main(hw: &mut hardware::Hardware) -> ! {
    let mut fps_monitor = FpsMonitor::new();

    unsafe {
        let regs = &*rp_pico::pac::XIP_SSI::PTR;
        info!("Flash clock divider: {}", regs.baudr.read().bits());
    }

    let mut position = Point::new((100 * 32 - 240) / 2, (100 * 32 - 240) / 2);
    let slime_position = position + Point::new(64, 32);
    let mut frame = 0;
    let mut walk_frame = 0;
    let mut player_direction = Direction::North;
    loop {
        let speed = 2;
        if hw.input.dpad_left.is_held() {
            position.x -= speed;
            player_direction = Direction::West;
            walk_frame += 1;
        } else if hw.input.dpad_right.is_held() {
            position.x += speed;
            player_direction = Direction::East;
            walk_frame += 1;
        } else if hw.input.dpad_up.is_held() {
            position.y -= speed;
            player_direction = Direction::North;
            walk_frame += 1;
        } else if hw.input.dpad_down.is_held() {
            position.y += speed;
            player_direction = Direction::South;
            walk_frame += 1;
        } else {
            walk_frame = 0;
        }

        tile::draw(&mut hw.display, position, &generate_map, frame % 60 == 0);

        hw.draw(|display| {
            {
                let s: u32 = 64;
                let player_atlas = protagonist();
                let walk_anim = if walk_frame == 0 {
                    0
                } else {
                    1 + (walk_frame / 3) % 8
                };
                let atlas_coord = match player_direction {
                    Direction::North => Point::new(0, 0),
                    Direction::East => Point::new(0, 3 * s as i32),
                    Direction::South => Point::new(0, 2 * s as i32),
                    Direction::West => Point::new(0, s as i32),
                } + Point::new(walk_anim * s as i32, 0);
                let player_sprite =
                    player_atlas.sub_image(&Rectangle::new(atlas_coord, Size::new(s, s)));
                Image::new(&player_sprite, Point::new(0, 0))
                    .translate(Point::new(
                        (WIDTH as i32 - s as i32) / 2,
                        (HEIGHT as i32 - s as i32) / 2,
                    ))
                    .draw(display)
                    .unwrap();
            }

            {
                let s: u32 = 24;
                let slime_atlas = slime();
                let mut anim_frame = frame / 20 % 4;
                if anim_frame == 3 {
                    anim_frame = 1;
                }
                let atlas_coord = Point::new(anim_frame * 24, 48);
                let slime_sprite =
                    slime_atlas.sub_image(&Rectangle::new(atlas_coord, Size::new(s, s)));
                Image::new(&slime_sprite, Point::new(0, 0))
                    .translate(slime_position - position - Point::new(s as i32, s as i32) / 2)
                    .draw(display)
                    .unwrap();
            }
        });

        fps_monitor.update();
        frame += 1;
    }
}