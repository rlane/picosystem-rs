#![no_std]
#![no_main]

use cortex_m_rt::entry;
use log::info;
use rlane_picosystem_games as rpsg;
use rpsg::{hardware, time};

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

    let mut rng = oorandom::Rand32::new(time::time_us() as u64);

    let mut board = Board::new();
    for x in 0..BOARD_SIZE {
        for y in 0..BOARD_SIZE {
            board.set(x, y, rng.rand_u32() < 1_000_000_000);
        }
    }

    let mut cursorx = 60;
    let mut cursory = 60;
    let mut paused = false;

    let mut frame = 0;
    let mut prev_time_us = time::time_us();
    let mut prev_frame = 0;
    loop {
        if !paused {
            let prev_board = board;
            board = Board::new();
            for y in 0..BOARD_SIZE {
                for x in 0..BOARD_SIZE {
                    board.set(x, y, update(&prev_board, x, y));
                }
            }
        }

        hw.display.clear(Rgb565::BLUE).unwrap();
        for y in 0..BOARD_SIZE {
            for x in 0..BOARD_SIZE {
                if board.get(x, y) {
                    let ix = (x * 2) as i32;
                    let iy = (y * 2) as i32;
                    Rectangle::new(Point::new(ix, iy), Size::new(2, 2))
                        .into_styled(
                            PrimitiveStyleBuilder::new()
                                .fill_color(Rgb565::WHITE)
                                .build(),
                        )
                        .draw(&mut hw.display)
                        .unwrap();
                }
            }
        }

        if hw.input.dpad_left() {
            cursorx = wrap(cursorx - 1);
        }
        if hw.input.dpad_right() {
            cursorx = wrap(cursorx + 1);
        }
        if hw.input.dpad_up() {
            cursory = wrap(cursory - 1);
        }
        if hw.input.dpad_down() {
            cursory = wrap(cursory + 1);
        }
        if hw.input.button_a() {
            board.set(cursorx, cursory, true);
        }
        if hw.input.button_y() {
            paused = !paused;
        }

        {
            let ix = (cursorx * 2) as i32;
            let iy = (cursory * 2) as i32;
            Rectangle::new(Point::new(ix, iy), Size::new(2, 2))
                .into_styled(PrimitiveStyleBuilder::new().fill_color(Rgb565::RED).build())
                .draw(&mut hw.display)
                .unwrap();
        }

        hw.display.flush();

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

const BOARD_SIZE: usize = 120;

struct Board {
    data: [bool; BOARD_SIZE * BOARD_SIZE],
}

fn wrap(x: usize) -> usize {
    if x == usize::MAX {
        BOARD_SIZE - 1
    } else if x == BOARD_SIZE {
        0
    } else {
        x
    }
}

impl Board {
    pub fn new() -> Self {
        Board {
            data: [false; BOARD_SIZE * BOARD_SIZE],
        }
    }

    pub fn get(&self, x: usize, y: usize) -> bool {
        self.data[wrap(y) * BOARD_SIZE + wrap(x)]
    }

    pub fn set(&mut self, x: usize, y: usize, v: bool) {
        self.data[wrap(y) * BOARD_SIZE + wrap(x)] = v;
    }
}

fn update(prev_board: &Board, x: usize, y: usize) -> bool {
    let mut count = 0;
    for dx in -1..=1 {
        for dy in -1..=1 {
            if dx != 0 || dy != 0 {
                count += prev_board.get(x + dx as usize, y + dy as usize) as i32
            }
        }
    }
    let prev = prev_board.get(x, y);
    if prev && (count == 2 || count == 3) {
        true
    } else if !prev && count == 3 {
        true
    } else {
        false
    }
}
