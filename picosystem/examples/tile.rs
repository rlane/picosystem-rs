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

fn draw_tile(display: &mut Display, atlas: &Sprite, src: Point, dst: Point, size: Size) -> bool {
    let clipped_dst = Rectangle::new(dst, size).intersection(&display.bounding_box());
    if clipped_dst.size != size {
        let mut dma_channel = unsafe { dma::DmaChannel::new(2) };
        let src_data = &atlas.data;
        let dst_data = picosystem::display::framebuffer();

        let src = src + clipped_dst.top_left - dst;
        let dst = clipped_dst.top_left;
        let size = clipped_dst.size;

        let mut src_index = src.x + src.y * atlas.size.width as i32;
        let mut dst_index = dst.x + dst.y * WIDTH as i32;
        for _ in 0..size.height {
            unsafe {
                let src_addr = src_data.as_ptr().add(src_index as usize) as u32;
                let dst_addr = dst_data.as_mut_ptr().add(dst_index as usize) as u32;
                dma::copy_mem_bswap(&mut dma_channel, src_addr, dst_addr, 2, size.width);
            }
            src_index += atlas.size.width as i32;
            dst_index += WIDTH as i32;
        }
        false
    } else {
        let mut dma_channel = unsafe { dma::DmaChannel::new(2) };
        let src_data = &atlas.data;
        let dst_data = picosystem::display::framebuffer();

        let mut src_index = src.x + src.y * atlas.size.width as i32;
        let mut dst_index = dst.x + dst.y * WIDTH as i32;
        for _ in 0..size.height {
            unsafe {
                let src_addr = src_data.as_ptr().add(src_index as usize) as u32;
                let dst_addr = dst_data.as_mut_ptr().add(dst_index as usize) as u32;
                dma::copy_mem_bswap(&mut dma_channel, src_addr, dst_addr, 2, size.width);
            }
            src_index += atlas.size.width as i32;
            dst_index += WIDTH as i32;
        }
        true
    }
}

fn draw_tile_cached(display: &mut Display, src: Point, dst: Point, size: Size) {
    let clipped_dst = Rectangle::new(dst, size).intersection(&display.bounding_box());
    if true || clipped_dst.size != size {
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

    let _grass_tiles = [
        Point::new(0, 800),
        Point::new(32, 800),
        Point::new(64, 800),
        Point::new(128, 736),
        Point::new(672, 160),
        Point::new(704, 160),
        Point::new(736, 160),
    ];

    let brick_tiles = [
        // Tan brick
        Point::new(704, 960),
        // Grey brick
        Point::new(512, 832),
        Point::new(544, 832),
        Point::new(576, 832),
        Point::new(512, 864),
        Point::new(544, 864),
        Point::new(576, 864),
        Point::new(512, 896),
        Point::new(544, 896),
        Point::new(576, 896),
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

    let generate_map = |position: Point| -> Point {
        use hash32::{Hash, Hasher};
        let mut hasher = hash32::Murmur3Hasher::default();
        position.x.hash(&mut hasher);
        position.y.hash(&mut hasher);
        brick_tiles[hasher.finish() as usize % brick_tiles.len()]
    };

    let mut position = Point::new(16, 16);
    let mut total_draw_time = 0;
    let mut frame = 0;
    let subtile_mask = 32 - 1;
    loop {
        let speed = 2;
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

        let mut drawn_y: i32 = 0;
        let mut world_y = position.y;
        let subtile_y = position.y & subtile_mask;
        let mut tile_cache = heapless::LinearMap::<Point, Point, 32>::new();
        let mut tile_cache_misses = 0;
        let mut tile_cache_lookups = 0;
        let mut tile_cache_insert_failures = 0;
        let mut slow_draw = false;
        loop {
            let progress = hw.display.flush_progress();
            let safe_y = (progress as i32 - WIDTH as i32 + 1) / WIDTH as i32;
            if safe_y - drawn_y < 32 && progress < (WIDTH * HEIGHT) as usize {
                continue;
            } else if safe_y - drawn_y > 64 {
                slow_draw = true;
            }
            let row_start_time = time::time_us();

            let screen_y = drawn_y - subtile_y;

            let subtile_x = position.x & subtile_mask;

            for screen_x in (-subtile_x..(WIDTH as i32)).step_by(32) {
                let world_x = position.x + screen_x;
                let map_coord = Point::new(world_x & !subtile_mask, world_y & !subtile_mask);
                let tile = generate_map(map_coord);
                tile_cache_lookups += 1;
                if let Some(cached_src) = tile_cache.get(&tile) {
                    draw_tile_cached(
                        &mut hw.display,
                        *cached_src,
                        Point::new(screen_x, screen_y),
                        Size::new(32, 32),
                    );
                } else {
                    tile_cache_misses += 1;
                    let screen_coord = Point::new(screen_x, screen_y);
                    if draw_tile(
                        &mut hw.display,
                        &atlas_sprite,
                        tile,
                        screen_coord,
                        Size::new(32, 32),
                    ) || (screen_x >= 0 && screen_y < 0)
                    {
                        if let Err(_) = tile_cache.insert(tile, screen_coord) {
                            tile_cache_insert_failures += 1;
                        }
                    }
                }
            }
            total_draw_time += time::time_us() - row_start_time;

            drawn_y += 32;
            world_y += 32;
            if screen_y < 0 {
                tile_cache.clear();
            } else if screen_y + 32 >= HEIGHT as i32 {
                break;
            }
        }

        hw.draw(|_display| {});

        fps_monitor.update();

        frame += 1;
        if frame % 60 == 0 {
            log::info!("frame: {} draw_time: {}us", frame, total_draw_time / 60);
            log::info!(
                "Tile cache: misses={} lookups={} insert_failures={} miss_rate={:.2}%",
                tile_cache_misses,
                tile_cache_lookups,
                tile_cache_insert_failures,
                tile_cache_misses as f32 / tile_cache_lookups as f32 * 100.0
            );
            if slow_draw {
                log::info!("Slow draw detected");
            }
            total_draw_time = 0;
        }
    }
}
