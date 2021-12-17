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

    let mut total_draw_time = 0;
    let mut frame = 0;
    loop {
        let mut drawn_y: i32 = 0;
        let mut flush_finished = false;
        loop {
            let progress = hw.display.flush_progress();
            let safe_y = (progress as i32 - WIDTH as i32 + 1) / WIDTH as i32;
            if safe_y - drawn_y < 32 && progress < (WIDTH * HEIGHT) as usize {
                continue;
            }
            let row_start_time = time::time_us();
            for x in (0..WIDTH as i32).step_by(32) {
                draw_tile(
                    &mut hw.display,
                    &atlas_sprite,
                    Point::new(32, 800),
                    Point::new(x, drawn_y),
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
