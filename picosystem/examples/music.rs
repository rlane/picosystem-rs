#![no_std]
#![no_main]

use cortex_m_rt::entry;
use display::{HEIGHT, WIDTH};
use log::info;
use picosystem::{display, hardware, time};

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;

#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

#[entry]
fn main() -> ! {
    let mut hw = hardware::Hardware::new();
    info!("Finished initialization");

    let mut cursorx = 0;
    let mut cursory = HEIGHT / 2;
    let mut notes: [i32; WIDTH] = [-1; WIDTH];
    let mut playing: Option<i32> = None;

    for x in 0..WIDTH {
        notes[x] = x as i32;
    }

    let mut frame = 0;
    let mut prev_time_us = time::time_us();
    let mut prev_frame = 0;
    let mut cur_note = -1;
    loop {
        hw.display.clear(Rgb565::BLACK).unwrap();

        if hw.input.dpad_left.is_held() && cursorx > 0 {
            cursorx = cursorx - 1;
        }
        if hw.input.dpad_right.is_held() && cursorx < WIDTH - 1 {
            cursorx = cursorx + 1;
        }
        if hw.input.dpad_up.is_held() && cursory > 0 {
            cursory = cursory - 1;
        }
        if hw.input.dpad_down.is_held() && cursory < HEIGHT - 1 {
            cursory = cursory + 1;
        }

        if hw.input.button_a.is_held() {
            notes[cursorx] = cursory as i32;
        }

        if hw.input.button_b.is_held() {
            notes[cursorx] = -1;
        }

        if hw.input.button_x.is_pressed() {
            playing = Some(0);
        }

        if let Some(x) = playing {
            Line::new(Point::new(x, 0), Point::new(x, HEIGHT as i32 - 1))
                .into_styled(PrimitiveStyle::with_stroke(Rgb565::CYAN, 1))
                .draw(&mut hw.display)
                .unwrap();
        }

        if frame % 2 == 0 {
            if let Some(x) = playing {
                if x as usize == WIDTH - 1 {
                    playing = None;
                    cur_note = -1;
                    hw.audio.stop();
                } else {
                    let note = notes[x as usize];
                    if cur_note == note {
                        // nop
                    } else if note >= 0 {
                        cur_note = note;
                        let freq = make_freq(note);
                        hw.audio.start_tone(freq);
                    } else {
                        cur_note = -1;
                        hw.audio.stop();
                    }
                    playing = Some(x + 1)
                }
            }
        }

        for x in 0..WIDTH {
            if notes[x] >= 0 {
                Pixel(Point::new(x as i32, notes[x]), Rgb565::GREEN)
                    .draw(&mut hw.display)
                    .unwrap();
            }
        }

        {
            let cursor_color = if frame % 32 < 16 {
                Rgb565::BLUE
            } else {
                Rgb565::WHITE
            };
            let cursor = Circle::new(Point::new(cursorx as i32 - 1, cursory as i32 - 1), 3)
                .into_styled(
                    PrimitiveStyleBuilder::new()
                        .fill_color(cursor_color)
                        .build(),
                );
            cursor
                .draw(&mut display::XorDisplay::new(&mut hw.display))
                .unwrap();
            hw.display.flush();
            cursor
                .draw(&mut display::XorDisplay::new(&mut hw.display))
                .unwrap();
        }

        let now = time::time_us();
        if now - prev_time_us > 1000_000 {
            let frame_time = (now - prev_time_us) / (frame - prev_frame) as u32;
            info!("Frame time: {} us", frame_time);
            prev_frame = frame;
            prev_time_us = now;
        }
        frame += 1;
    }
}

fn make_freq(note: i32) -> u32 {
    100 + (HEIGHT as u32 - note as u32) * 4
}
