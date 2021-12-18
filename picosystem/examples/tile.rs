#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle, Rectangle};
use log::info;
use picosystem::display::{Display, HEIGHT, WIDTH};
use picosystem::dma;
use picosystem::fps_monitor::FpsMonitor;
use picosystem::hardware;
use picosystem::sprite::Sprite;
use picosystem::time;
use picosystem_macros::sprite;

sprite!(sprite_atlas, "picosystem/examples/terrain_atlas.png", 1032);

fn draw_tile(display: &mut Display, atlas: &Sprite, src: Point, dst: Point, size: Size) {
    if Rectangle::new(dst, size)
        .intersection(&display.bounding_box())
        .size
        != size
    {
        return;
    }

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
}

#[entry]
fn main() -> ! {
    let mut hw = hardware::Hardware::new();
    info!("Finished initialization");
    let mut fps_monitor = FpsMonitor::new();

    let atlas_sprite = sprite_atlas();
    let grass0_tile = Point::new(0, 800);
    let grass1_tile = Point::new(32, 800);
    let grass2_tile = Point::new(64, 800);
    let grass3_tile = Point::new(128, 736);
    let grass4_tile = Point::new(672, 160);
    let grass5_tile = Point::new(704, 160);
    let grass6_tile = Point::new(736, 160);

    let grass_tiles = [
        grass0_tile,
        grass1_tile,
        grass2_tile,
        grass3_tile,
        grass4_tile,
        grass5_tile,
        grass6_tile,
    ];

    let generate_map = |position: Point| -> Point {
        use hash32::{Hash, Hasher};
        let mut hasher = hash32::Murmur3Hasher::default();
        position.x.hash(&mut hasher);
        position.y.hash(&mut hasher);
        grass_tiles[hasher.finish() as usize % grass_tiles.len()]
    };

    let mut position = Point::new(0, 0);
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
        let mut flush_finished = false;
        let subtile_y = position.y & subtile_mask;
        loop {
            let progress = hw.display.flush_progress();
            let safe_y = (progress as i32 - WIDTH as i32 + 1) / WIDTH as i32;
            if safe_y - drawn_y < 32 && progress < (WIDTH * HEIGHT) as usize {
                continue;
            }
            let row_start_time = time::time_us();

            let screen_y = drawn_y - subtile_y;

            let subtile_x = position.x & subtile_mask;

            for screen_x in (-subtile_x..(WIDTH as i32 - subtile_x)).step_by(32) {
                let world_x = position.x + screen_x;

                let tile =
                    generate_map(Point::new(world_x & !subtile_mask, world_y & !subtile_mask));
                draw_tile(
                    &mut hw.display,
                    &atlas_sprite,
                    tile,
                    Point::new(screen_x, screen_y),
                    Size::new(32, 32),
                );
            }
            total_draw_time += time::time_us() - row_start_time;

            if !flush_finished && progress == (WIDTH * HEIGHT) as usize {
                flush_finished = true;
                Line::new(
                    Point::new(0, drawn_y),
                    Point::new(WIDTH as i32 - 1, drawn_y),
                )
                .into_styled(PrimitiveStyle::with_stroke(Rgb565::CSS_DARK_SLATE_BLUE, 1))
                .draw(&mut hw.display)
                .unwrap();
            }
            //log::info!("drawn_y: {} progress: {}", drawn_y, progress);
            drawn_y += 32;
            world_y += 32;
            if drawn_y >= HEIGHT as i32 {
                break;
            }
        }

        hw.draw(|_display| {});

        fps_monitor.update();

        frame += 1;
        if frame % 60 == 0 {
            log::info!("frame: {} draw_time: {}us", frame, total_draw_time / 60);
            total_draw_time = 0;
        }
    }
}
