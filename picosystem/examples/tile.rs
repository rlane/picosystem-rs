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
use picosystem::sprite::Sprite;
use picosystem::time;
use picosystem_macros::sprite;

sprite!(sprite_atlas, "picosystem/examples/terrain_atlas.png", 1032);

const TILE_SIZE: i32 = 32;

struct MapTile {
    base_atlas_coord: Point,
    overlay_atlas_coord: Option<Point>,
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

fn load_tile(atlas: &Sprite, src: Point, dst: &mut LoadedTile) {
    let mut buf = [0u16; TILE_SIZE as usize];
    unsafe {
        let mut dma_channel = dma::DmaChannel::new(1);
        let mut src_addr = atlas
            .data
            .as_ptr()
            .add((src.x + src.y * atlas.size.width as i32) as usize)
            as u32;
        let mut dst_addr = dst.data.as_ptr() as u32;
        for y in 0..TILE_SIZE {
            dma::copy_flash_to_mem(
                &mut dma_channel,
                src_addr,
                buf.as_mut_ptr() as u32,
                TILE_SIZE as u32 / 2,
            );
            dma::start_copy_mem_bswap(
                &mut dma_channel,
                buf.as_ptr() as u32,
                dst_addr,
                2,
                TILE_SIZE as u32,
            );
            let mut mask: u32 = 0;
            for x in 0..TILE_SIZE {
                let color = buf[x as usize];
                if color != 0 {
                    mask |= 1 << x;
                }
            }
            dst.mask[y as usize] = mask;
            src_addr += 2 * atlas.size.width as u32;
            dst_addr += 2 * TILE_SIZE as u32;
            dma_channel.wait();
        }
    }
}

fn draw_tile(display: &mut Display, atlas: &Sprite, src: Point, dst: Point, size: Size) -> bool {
    let mut buf = [0u16; TILE_SIZE as usize];
    let clipped_dst = Rectangle::new(dst, size).intersection(&display.bounding_box());
    let mut dma_channel0 = unsafe { dma::DmaChannel::new(1) };
    let mut dma_channel1 = unsafe { dma::DmaChannel::new(2) };

    let src = src + clipped_dst.top_left - dst;
    let dst = clipped_dst.top_left;

    let src_data = &atlas.data;
    let dst_data = picosystem::display::framebuffer();
    let mut src_index = src.x + src.y * atlas.size.width as i32;
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
            dma::start_copy_mem_bswap(
                &mut dma_channel1,
                buf_addr,
                dst_addr,
                2,
                clipped_dst.size.width,
            );
        }
        src_index += atlas.size.width as i32;
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

fn draw_tiles<F>(
    display: &mut Display,
    atlas: &Sprite,
    position: Point,
    map_generator: &F,
    verbose: bool,
) where
    F: Fn(Point) -> MapTile,
{
    let subtile_mask = 32 - 1;
    let enable_tile_cache = true;

    let mut drawn_y: i32 = 0;
    let mut world_y = position.y;
    let subtile_y = position.y & subtile_mask;

    let mut tile_cache = heapless::LinearMap::<Point, Point, 64>::new();
    let mut base_tile_cache_misses = 0;
    let mut base_tile_cache_lookups = 0;
    let mut base_tile_cache_insert_failures = 0;

    let mut overlay_tile_cache = heapless::LinearMap::<Point, LoadedTile, 4>::new();
    let mut overlay_tile_cache_misses = 0;
    let mut overlay_tile_cache_lookups = 0;
    let mut overlay_tile_cache_insert_failures = 0;

    let mut missing_transparent_tiles = heapless::Vec::<(Point, Point), 64>::new();

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
            if let Some(cached_src) = tile_cache.get(&map_tile.base_atlas_coord) {
                copy_tile(display, *cached_src, screen_coord, Size::new(32, 32));
                if let Some(overlay_atlas_coord) = map_tile.overlay_atlas_coord {
                    overlay_tile_cache_lookups += 1;
                    if let Some(cached_overlay_tile) = overlay_tile_cache.get(&overlay_atlas_coord)
                    {
                        draw_transparent_tile(
                            display,
                            cached_overlay_tile,
                            screen_coord,
                            Size::new(32, 32),
                        );
                    } else {
                        overlay_tile_cache_misses += 1;
                        let mut loaded_tile = LoadedTile::new();
                        load_tile(atlas, overlay_atlas_coord, &mut loaded_tile);
                        draw_transparent_tile(
                            display,
                            &loaded_tile,
                            screen_coord,
                            Size::new(32, 32),
                        );
                        if let Err(_) = overlay_tile_cache.insert(overlay_atlas_coord, loaded_tile)
                        {
                            overlay_tile_cache_insert_failures += 1;
                        }
                    }
                }
            } else {
                base_tile_cache_misses += 1;
                if (draw_tile(
                    display,
                    &atlas,
                    map_tile.base_atlas_coord,
                    screen_coord,
                    Size::new(32, 32),
                ) || (screen_x >= 0 && screen_y < 0))
                    && enable_tile_cache
                {
                    if let Err(_) = tile_cache.insert(map_tile.base_atlas_coord, screen_coord) {
                        base_tile_cache_insert_failures += 1;
                    }
                }
                if let Some(overlay_atlas_coord) = map_tile.overlay_atlas_coord {
                    missing_transparent_tiles
                        .push((screen_coord, overlay_atlas_coord))
                        .unwrap();
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
    for (screen_coord, overlay_atlas_coord) in missing_transparent_tiles {
        overlay_tile_cache_lookups += 1;
        if let Some(cached_overlay_tile) = overlay_tile_cache.get(&overlay_atlas_coord) {
            draw_transparent_tile(
                display,
                cached_overlay_tile,
                screen_coord,
                Size::new(32, 32),
            );
        } else {
            overlay_tile_cache_misses += 1;
            let mut loaded_tile = LoadedTile::new();
            load_tile(atlas, overlay_atlas_coord, &mut loaded_tile);
            draw_transparent_tile(display, &loaded_tile, screen_coord, Size::new(32, 32));
            if let Err(_) = overlay_tile_cache.insert(overlay_atlas_coord, loaded_tile) {
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
    let blank_tile = Point::new(896, 192);

    let grass_tiles = [
        Point::new(0, 800),
        Point::new(32, 800),
        Point::new(64, 800),
        Point::new(672, 160),
        Point::new(704, 160),
        Point::new(736, 160),
    ];

    let grey_brick_tiles = [
        Point::new(512, 832),
        Point::new(544, 832),
        Point::new(576, 832),
        Point::new(512, 864),
        Point::new(544, 864),
        Point::new(576, 864),
        Point::new(512, 896),
        Point::new(544, 896),
        Point::new(576, 896),
    ];

    let _brick_tiles = [
        // Tan brick
        Point::new(704, 960),
        // Grey brick 2
        Point::new(672, 864),
        // Grey brick 3
        Point::new(608, 832),
        Point::new(608, 876),
        Point::new(608, 896),
        // Brown brick
        Point::new(672, 704),
        // Grey slabs
        Point::new(512, 928),
        Point::new(544, 928),
        Point::new(576, 928),
        Point::new(512, 960),
        Point::new(544, 960),
        Point::new(576, 960),
        Point::new(512, 992),
        Point::new(544, 992),
        Point::new(576, 992),
    ];

    let rock_tiles = [
        Point::new(832, 800),
        Point::new(864, 800),
        Point::new(896, 800),
        Point::new(928, 800),
        Point::new(928, 800),
        Point::new(928, 768),
        Point::new(928, 736),
    ];

    let sparse_grass = Point::new(32, 992);

    let shore_tile_north = Point::new(224, 416);
    let shore_tile_south = Point::new(224, 352);
    let shore_tile_east = Point::new(192, 384);
    let shore_tile_west = Point::new(256, 384);
    let shore_tile_northeast = Point::new(256, 288);
    let shore_tile_northwest = Point::new(224, 288);
    let shore_tile_southeast = Point::new(256, 320);
    let shore_tile_southwest = Point::new(224, 320);

    let ocean_tiles = [
        Point::new(96, 448),
        Point::new(128, 448),
        Point::new(160, 448),
        Point::new(192, 448),
        Point::new(224, 448),
        Point::new(256, 448),
        Point::new(256, 448),
        Point::new(256, 448),
        Point::new(256, 448),
        Point::new(256, 448),
        Point::new(256, 448),
        Point::new(256, 448),
        Point::new(256, 448),
        Point::new(256, 448),
        Point::new(256, 448),
        Point::new(256, 448),
    ];

    let translate_map = |c: char| -> Point {
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

    if false {
        return MapTile {
            base_atlas_coord: Point::new(position.x.rem_euclid(1024), position.y.rem_euclid(1024)),
            overlay_atlas_coord: Some(sparse_grass),
        };
    }

    use hash32::{Hash, Hasher};
    let mut hasher = hash32::Murmur3Hasher::default();
    position.x.hash(&mut hasher);
    position.y.hash(&mut hasher);
    let hash = hasher.finish();
    let map_x = position.x / TILE_SIZE;
    let map_y = position.y / TILE_SIZE;
    let mut base_atlas_coord = if (0..MAP_SIZE).contains(&map_x) && (0..MAP_SIZE).contains(&map_y) {
        let index = (map_x + map_y * MAP_SIZE) as usize;
        let c = BASE_MAP.as_bytes()[index as usize];
        translate_map(c as char)
    } else {
        ocean_tiles[hash as usize % ocean_tiles.len()]
    };

    if base_atlas_coord == grass_tiles[0] {
        base_atlas_coord = grass_tiles[(hash % grass_tiles.len() as u32) as usize];
    } else if base_atlas_coord == grey_brick_tiles[0] {
        base_atlas_coord = grey_brick_tiles[((map_x % 3) + 3 * (map_y % 3)) as usize];
    }

    let overlay_atlas_coord = if base_atlas_coord == grass_tiles[0] {
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
        base_atlas_coord,
        overlay_atlas_coord,
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

    let atlas_sprite = sprite_atlas();

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

        draw_tiles(
            &mut hw.display,
            &atlas_sprite,
            position,
            &generate_map,
            frame % 60 == 0,
        );

        hw.draw(|_display| {});

        fps_monitor.update();
        frame += 1;
    }
}
