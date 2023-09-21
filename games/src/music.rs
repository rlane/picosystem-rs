use display::{HEIGHT, WIDTH};
use log::info;
use picosystem::{display, hardware, time};

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use heapless::Vec;

pub fn main(hw: &mut hardware::Hardware) -> ! {
    let mut cursorx: i32 = 1;
    let mut cursory: i32 = 1;
    let mut notes = Vec::<i32, 240>::new();
    let mut playing: Option<i32> = None;
    const IHEIGHT: i32 = HEIGHT as i32;
    const IWIDTH: i32 = WIDTH as i32;

    let _ = notes.push(-1);
    for x in 1..240 {
        let _ = notes.push(x);
    }

    let mut frame = -1_i32;
    let mut prev_time_us = time::time_us();
    let mut prev_frame = 0;
    loop {
        if hw.input.dpad_left.is_held() && cursorx > 0 {
            cursorx -= 1;
        }
        if hw.input.dpad_right.is_held() && cursorx < IWIDTH - 1 {
            cursorx += 1;
        }
        if hw.input.dpad_up.is_held() && cursory > 0 {
            cursory -= 1;
        }
        if hw.input.dpad_down.is_held() && cursory < IHEIGHT - 1 {
            cursory += 1;
        }

        if hw.input.button_a.is_held() {
            notes[cursorx as usize] = cursory;
        }

        if hw.input.button_b.is_held() {
            notes[cursorx as usize] = -1;
        }

        if hw.input.button_x.is_pressed() {
            playing = Some(0);
        }
        
        if let Some(x) = playing {
            Line::new(Point::new(x, 0), Point::new(x, IHEIGHT - 1))
                .into_styled(PrimitiveStyle::with_stroke(Rgb565::CYAN, 1))
                .draw(&mut hw.display)
                .unwrap();
        }

        if frame % 2 == 0 {
            if let Some(x) = playing {
                if x as usize == WIDTH - 1 {
                    playing = None;
                    hw.audio.stop();
                } else {
                    let note = notes[x as usize];
                    if note >= 0 {
                        let freq = make_freq(note);
                        hw.audio.start_tone(freq);
                    } else {
                        hw.audio.stop();
                    }
                    playing = Some(x + 1)
                }
            }
        }

        hw.draw(|display| {
            display.clear(Rgb565::BLACK).unwrap();
            for (x, &note) in notes.iter().enumerate() {
                if note >= 0 {
                    Pixel(Point::new(x as i32, note), Rgb565::GREEN)
                        .draw(display)
                        .unwrap();
                }
            }
            let cursor_color = if frame % 32 < 16 {
                Rgb565::BLUE
            } else {
                Rgb565::WHITE
            };
            let cursor = Circle::new(Point::new(cursorx - 1, cursory - 1), 3).into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(cursor_color)
                    .build(),
            );
            cursor.draw(display).unwrap();
        });

        let now = time::time_us();
        if now - prev_time_us > 1_000_000 {
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
