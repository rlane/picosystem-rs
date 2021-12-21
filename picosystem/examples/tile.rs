#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use log::info;
use picosystem::display::{Display, HEIGHT, WIDTH};
use picosystem::dma;
use picosystem::fps_monitor::FpsMonitor;
use picosystem::hardware;
use picosystem::tile::Tile;
use picosystem::time;
use picosystem_macros::atlas;

atlas!(atlas, "picosystem/examples/terrain_atlas.png", 32);

const TILE_SIZE: i32 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TileId(u32);

fn tile_id(tile: &Tile) -> TileId {
    TileId(tile as *const Tile as u32)
}

struct MapTile {
    base: &'static Tile,
    overlay: Option<&'static Tile>,
}

struct LoadedTile {
    data: [u16; (TILE_SIZE * TILE_SIZE) as usize],
    mask: [u32; TILE_SIZE as usize],
}

impl LoadedTile {
    fn new() -> Self {
        LoadedTile {
            data: [0; (TILE_SIZE * TILE_SIZE) as usize],
            mask: [0; TILE_SIZE as usize],
        }
    }
}

fn load_tile(src: &Tile, dst: &mut LoadedTile) {
    unsafe {
        let mut dma_channel = dma::DmaChannel::new(1);
        dma::copy_flash_to_mem(
            &mut dma_channel,
            src.data.as_ptr() as u32,
            dst.data.as_ptr() as u32,
            (TILE_SIZE * TILE_SIZE) as u32 / 2,
        );
        dma::copy_flash_to_mem(
            &mut dma_channel,
            src.mask.as_ptr() as u32,
            dst.mask.as_ptr() as u32,
            TILE_SIZE as u32,
        );
    }
}

fn draw_tile(display: &mut Display, tile: &Tile, src: Point, dst: Point, size: Size) -> bool {
    let mut buf = [0u16; TILE_SIZE as usize];
    let clipped_dst = Rectangle::new(dst, size).intersection(&display.bounding_box());
    let mut dma_channel0 = unsafe { dma::DmaChannel::new(1) };
    let mut dma_channel1 = unsafe { dma::DmaChannel::new(2) };

    let src = src + clipped_dst.top_left - dst;
    let dst = clipped_dst.top_left;

    let src_data = &tile.data;
    let dst_data = picosystem::display::framebuffer();
    let mut src_index = src.x + src.y * TILE_SIZE;
    let mut dst_index = dst.x + dst.y * WIDTH as i32;
    for _ in 0..clipped_dst.size.height {
        unsafe {
            let src_addr = src_data.as_ptr().add(src_index as usize) as u32;
            let dst_addr = dst_data.as_mut_ptr().add(dst_index as usize) as u32;
            let buf_addr = buf.as_mut_ptr() as u32;
            dma_channel1.wait();
            dma::copy_flash_to_mem(
                &mut dma_channel0,
                src_addr,
                buf_addr,
                clipped_dst.size.width / 2,
            );
            dma::start_copy_mem(
                &mut dma_channel1,
                buf_addr,
                dst_addr,
                2,
                clipped_dst.size.width,
            );
        }
        src_index += TILE_SIZE;
        dst_index += WIDTH as i32;
    }

    dma_channel1.wait();
    clipped_dst.size == size
}

fn draw_transparent_tile(display: &mut Display, tile: &LoadedTile, dst: Point, size: Size) {
    let clipped_dst = Rectangle::new(dst, size).intersection(&display.bounding_box());
    let src = clipped_dst.top_left - dst;
    let dst = clipped_dst.top_left;

    unsafe {
        let mut dma_channel = dma::DmaChannel::new(1);
        let mut src_ptr: *const u16 = tile.data.as_ptr();
        let mut dst_ptr: *mut u16 = picosystem::display::framebuffer().as_mut_ptr();
        let mut mask_ptr: *const u32 = tile.mask.as_ptr().add(src.y as usize);
        src_ptr = src_ptr.add((src.x + src.y * TILE_SIZE) as usize);
        dst_ptr = dst_ptr.add((dst.x + dst.y * WIDTH as i32) as usize);
        for _ in 0..clipped_dst.size.height {
            let w = clipped_dst.size.width;
            let mut mask = *mask_ptr;
            mask >>= src.x;
            if w < 32 {
                mask &= (1 << w) - 1;
            }
            let mut x = 0;
            while mask != 0 {
                const LOOKAHEAD: u32 = 0x7;
                let n = if mask & LOOKAHEAD == LOOKAHEAD {
                    let n = mask.trailing_ones();
                    dma_channel.wait();
                    dma::start_copy_mem(&mut dma_channel, src_ptr as u32, dst_ptr as u32, 2, n);
                    n
                } else if mask & LOOKAHEAD == 0x0 {
                    mask.trailing_zeros()
                } else {
                    let color = *src_ptr;
                    if mask & 1 != 0 {
                        *dst_ptr = color;
                    }
                    1
                };
                src_ptr = src_ptr.add(n as usize);
                dst_ptr = dst_ptr.add(n as usize);
                mask >>= n;
                x += n;
            }
            src_ptr = src_ptr.add(TILE_SIZE as usize - x as usize);
            dst_ptr = dst_ptr.add(WIDTH as usize - x as usize);
            mask_ptr = mask_ptr.add(1);
        }
        dma_channel.wait();
    }
}

fn copy_tile(display: &mut Display, src: Point, dst: Point, size: Size) {
    let clipped_dst = Rectangle::new(dst, size).intersection(&display.bounding_box());
    let mut dma_channel = unsafe { dma::DmaChannel::new(2) };
    let fb_data = picosystem::display::framebuffer();

    let src = src + clipped_dst.top_left - dst;
    let dst = clipped_dst.top_left;
    let size = clipped_dst.size;

    let mut src_index = src.x + src.y * WIDTH as i32;
    let mut dst_index = dst.x + dst.y * WIDTH as i32;
    for _ in 0..size.height {
        unsafe {
            let src_addr = fb_data.as_ptr().add(src_index as usize) as u32;
            let dst_addr = fb_data.as_mut_ptr().add(dst_index as usize) as u32;
            dma::copy_mem(&mut dma_channel, src_addr, dst_addr, 2, size.width);
        }
        src_index += WIDTH as i32;
        dst_index += WIDTH as i32;
    }
}

fn draw_tiles<F>(display: &mut Display, position: Point, map_generator: &F, verbose: bool)
where
    F: Fn(Point) -> MapTile,
{
    let subtile_mask = 32 - 1;
    let enable_tile_cache = true;

    let mut drawn_y: i32 = 0;
    let mut world_y = position.y;
    let subtile_y = position.y & subtile_mask;

    let mut tile_cache = heapless::LinearMap::<TileId, Point, 64>::new();
    let mut base_tile_cache_misses = 0;
    let mut base_tile_cache_lookups = 0;
    let mut base_tile_cache_insert_failures = 0;

    let mut overlay_tile_cache = heapless::LinearMap::<TileId, LoadedTile, 4>::new();
    let mut overlay_tile_cache_misses = 0;
    let mut overlay_tile_cache_lookups = 0;
    let mut overlay_tile_cache_insert_failures = 0;

    let mut missing_transparent_tiles = heapless::Vec::<(Point, &'static Tile), 64>::new();

    let mut slow_draw = false;
    let mut draw_time = 0;
    loop {
        let progress = display.flush_progress();
        let safe_y = (progress as i32 - WIDTH as i32 + 1) / WIDTH as i32;
        if safe_y - drawn_y < 32 && progress < (WIDTH * HEIGHT) as usize {
            continue;
        } else if safe_y - drawn_y > 64 {
            slow_draw = true;
        }
        let draw_start_time = time::time_us();

        let screen_y = drawn_y - subtile_y;

        let subtile_x = position.x & subtile_mask;

        for screen_x in (-subtile_x..(WIDTH as i32)).step_by(32) {
            let world_x = position.x + screen_x;
            let map_coord = Point::new(world_x & !subtile_mask, world_y & !subtile_mask);
            let screen_coord = Point::new(screen_x, screen_y);
            let map_tile = map_generator(map_coord);
            base_tile_cache_lookups += 1;
            if let Some(cached_src) = tile_cache.get(&tile_id(map_tile.base)) {
                copy_tile(display, *cached_src, screen_coord, Size::new(32, 32));
                if let Some(overlay) = map_tile.overlay {
                    overlay_tile_cache_lookups += 1;
                    if let Some(cached_overlay_tile) = overlay_tile_cache.get(&tile_id(overlay)) {
                        draw_transparent_tile(
                            display,
                            cached_overlay_tile,
                            screen_coord,
                            Size::new(32, 32),
                        );
                    } else {
                        overlay_tile_cache_misses += 1;
                        let mut loaded_tile = LoadedTile::new();
                        load_tile(overlay, &mut loaded_tile);
                        draw_transparent_tile(
                            display,
                            &loaded_tile,
                            screen_coord,
                            Size::new(32, 32),
                        );
                        if let Err(_) = overlay_tile_cache.insert(tile_id(overlay), loaded_tile) {
                            overlay_tile_cache_insert_failures += 1;
                        }
                    }
                }
            } else {
                base_tile_cache_misses += 1;
                if (draw_tile(
                    display,
                    map_tile.base,
                    Point::new(0, 0),
                    screen_coord,
                    Size::new(32, 32),
                ) || (screen_x >= 0 && screen_y < 0))
                    && enable_tile_cache
                {
                    if let Err(_) = tile_cache.insert(tile_id(map_tile.base), screen_coord) {
                        base_tile_cache_insert_failures += 1;
                    }
                }
                if let Some(overlay) = map_tile.overlay {
                    let _ = missing_transparent_tiles.push((screen_coord, overlay));
                }
            }
        }

        draw_time += time::time_us() - draw_start_time;

        drawn_y += 32;
        world_y += 32;
        if screen_y < 0 {
            tile_cache.clear();
        } else if screen_y + 32 >= HEIGHT as i32 {
            break;
        }
    }

    let draw_start_time = time::time_us();
    for (screen_coord, overlay) in missing_transparent_tiles {
        overlay_tile_cache_lookups += 1;
        if let Some(cached_overlay_tile) = overlay_tile_cache.get(&tile_id(overlay)) {
            draw_transparent_tile(
                display,
                cached_overlay_tile,
                screen_coord,
                Size::new(32, 32),
            );
        } else {
            overlay_tile_cache_misses += 1;
            let mut loaded_tile = LoadedTile::new();
            load_tile(overlay, &mut loaded_tile);
            draw_transparent_tile(display, &loaded_tile, screen_coord, Size::new(32, 32));
            if let Err(_) = overlay_tile_cache.insert(tile_id(overlay), loaded_tile) {
                overlay_tile_cache_insert_failures += 1;
            }
        }
    }
    draw_time += time::time_us() - draw_start_time;

    if verbose {
        log::info!("draw_time: {}us", draw_time);
        log::info!("position: {:?}", position);
        log::info!(
            "Base tile cache: misses={} lookups={} insert_failures={} miss_rate={:.2}%",
            base_tile_cache_misses,
            base_tile_cache_lookups,
            base_tile_cache_insert_failures,
            base_tile_cache_misses as f32 / base_tile_cache_lookups as f32 * 100.0
        );
        log::info!(
            "Overlay tile cache: misses={} lookups={} insert_failures={} miss_rate={:.2}%",
            overlay_tile_cache_misses,
            overlay_tile_cache_lookups,
            overlay_tile_cache_insert_failures,
            overlay_tile_cache_misses as f32 / overlay_tile_cache_lookups as f32 * 100.0
        );
        if slow_draw {
            log::info!("Slow draw detected");
        }
    }
}

fn generate_map(position: Point) -> MapTile {
    let blank_tile = atlas220();

    let grass_tiles = [
        atlas800(),
        atlas801(),
        atlas802(),
        atlas181(),
        atlas182(),
        atlas183(),
    ];

    let grey_brick_tiles = [
        atlas848(),
        atlas849(),
        atlas850(),
        atlas880(),
        atlas881(),
        atlas882(),
        atlas912(),
        atlas913(),
        atlas914(),
    ];

    let _brick_tiles = [
        // Tan brick
        atlas982(),
        // Grey brick 2
        atlas885(),
        // Grey brick 3
        atlas851(),
        atlas895(),
        atlas915(),
        // Brown brick
        atlas725(),
        // Grey slabs
        atlas944(),
        atlas945(),
        atlas946(),
        atlas976(),
        atlas977(),
        atlas978(),
        atlas1008(),
        atlas1009(),
        atlas1010(),
    ];

    let rock_tiles = [
        atlas826(),
        atlas827(),
        atlas828(),
        atlas829(),
        atlas829(),
        atlas797(),
        atlas765(),
    ];

    let sparse_grass = atlas993();

    let shore_tile_north = atlas423();
    let shore_tile_south = atlas359();
    let shore_tile_east = atlas390();
    let shore_tile_west = atlas392();
    let shore_tile_northeast = atlas296();
    let shore_tile_northwest = atlas295();
    let shore_tile_southeast = atlas328();
    let shore_tile_southwest = atlas327();

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

    let translate_map = |c: char| -> &'static Tile {
        match c {
            'a' => grass_tiles[0],
            'b' => grey_brick_tiles[0],
            'w' => shore_tile_north,
            'W' => shore_tile_south,
            'x' => shore_tile_east,
            'X' => shore_tile_west,
            'y' => shore_tile_northeast,
            'Y' => shore_tile_northwest,
            'z' => shore_tile_southeast,
            'Z' => shore_tile_southwest,
            _ => blank_tile,
        }
    };

    const BASE_MAP : &'static str = concat!(
        "Ywwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwy",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "Xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaax",
        "ZWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWz");

    const MAP_SIZE: i32 = 100;

    use hash32::{Hash, Hasher};
    let mut hasher = hash32::Murmur3Hasher::default();
    position.x.hash(&mut hasher);
    position.y.hash(&mut hasher);
    let hash = hasher.finish();
    let map_x = position.x / TILE_SIZE;
    let map_y = position.y / TILE_SIZE;
    let mut base_tile = if (0..MAP_SIZE).contains(&map_x) && (0..MAP_SIZE).contains(&map_y) {
        let index = (map_x + map_y * MAP_SIZE) as usize;
        let c = BASE_MAP.as_bytes()[index as usize];
        translate_map(c as char)
    } else {
        ocean_tiles[hash as usize % ocean_tiles.len()]
    };

    if tile_id(base_tile) == tile_id(grass_tiles[0]) {
        base_tile = grass_tiles[(hash % grass_tiles.len() as u32) as usize];
    } else if tile_id(base_tile) == tile_id(grey_brick_tiles[0]) {
        base_tile = grey_brick_tiles[((map_x % 3) + 3 * (map_y % 3)) as usize];
    }

    let overlay_tile = if tile_id(base_tile) == tile_id(grass_tiles[0]) {
        if hash & 0xff < 0x30 {
            Some(rock_tiles[(hash % rock_tiles.len() as u32) as usize])
        } else if hash & 0xff < 0x40 {
            Some(sparse_grass)
        } else {
            None
        }
    } else {
        None
    };
    MapTile {
        base: base_tile,
        overlay: overlay_tile,
    }
}

#[entry]
fn main() -> ! {
    let mut hw = hardware::Hardware::new();
    info!("Finished initialization");
    let mut fps_monitor = FpsMonitor::new();

    unsafe {
        let regs = &*pico::pac::XIP_SSI::PTR;
        info!("Flash clock divider: {}", regs.baudr.read().bits());
    }

    let mut position = Point::new((100 * 32 - 240) / 2, (100 * 32 - 240) / 2);
    let mut frame = 0;
    loop {
        let speed = 8;
        if hw.input.dpad_left.is_held() {
            position.x -= speed;
        }
        if hw.input.dpad_right.is_held() {
            position.x += speed;
        }
        if hw.input.dpad_up.is_held() {
            position.y -= speed;
        }
        if hw.input.dpad_down.is_held() {
            position.y += speed;
        }

        draw_tiles(&mut hw.display, position, &generate_map, frame % 60 == 0);

        hw.draw(|_display| {});

        fps_monitor.update();
        frame += 1;
    }
}
