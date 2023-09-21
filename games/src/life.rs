use log::info;
use picosystem::{hardware, time};

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;

pub fn main(hw: &mut hardware::Hardware) -> ! {
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
            const LIMIT: usize = BOARD_SIZE - 1;
            let prev_board = board;
            board = Board::new();
            for x in 0..LIMIT {
                board.set_fast(board.index(x, 0), update(&prev_board, x, 0));
            }
            for y in 1..LIMIT {
                let start_index = board.index(0, y);
                let end_index = board.index(LIMIT, y);
                board.set_fast(start_index, update(&prev_board, 0, y));
                for i in (start_index + 1)..end_index {
                    board.set_fast(i, update_fast(&prev_board, i));
                }
                board.set_fast(end_index, update(&prev_board, LIMIT, y));
            }
            for x in 0..BOARD_SIZE {
                board.set_fast(board.index(x, LIMIT), update(&prev_board, x, LIMIT));
            }
        }

        hw.draw(|display| {
            display.clear(Rgb565::BLUE).unwrap();
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
                            .draw(display)
                            .unwrap();
                    }
                }
            }

            {
                let ix = (cursorx * 2) as i32;
                let iy = (cursory * 2) as i32;
                Rectangle::new(Point::new(ix, iy), Size::new(2, 2))
                    .into_styled(PrimitiveStyleBuilder::new().fill_color(Rgb565::RED).build())
                    .draw(display)
                    .unwrap();
            }
        });

        if hw.input.dpad_left.is_held() {
            cursorx = wrap(cursorx - 1);
        }
        if hw.input.dpad_right.is_held() {
            cursorx = wrap(cursorx + 1);
        }
        if hw.input.dpad_up.is_held() {
            cursory = wrap(cursory - 1);
        }
        if hw.input.dpad_down.is_held() {
            cursory = wrap(cursory + 1);
        }
        if hw.input.button_a.is_held() {
            board.set(cursorx, cursory, true);
        }
        if hw.input.button_y.is_pressed() {
            paused = !paused;
        }

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

    pub fn index(&self, x: usize, y: usize) -> usize {
        y * BOARD_SIZE + x
    }

    pub fn get_fast(&self, i: usize) -> bool {
        self.data[i]
    }

    pub fn set_fast(&mut self, i: usize, v: bool) {
        self.data[i] = v;
    }

    pub fn count_neighbors_fast(&self, i: usize) -> i32 {
        let data = &self.data;
        (data[i - BOARD_SIZE - 1] as i32
            + data[i - BOARD_SIZE] as i32
            + data[i - BOARD_SIZE + 1] as i32)
            + (data[i - 1] as i32 + data[i + 1] as i32)
            + (data[i + BOARD_SIZE - 1] as i32
                + data[i + BOARD_SIZE] as i32
                + data[i + BOARD_SIZE + 1] as i32)
    }
}

fn update(prev_board: &Board, x: usize, y: usize) -> bool {
    let mut count = 0;
    for dx in -1..=1 {
        for dy in -1..=1 {
            if dx != 0 || dy != 0 {
                let x2 = x as i32 + dx;
                let y2 = y as i32 + dy;
                if x2 >= 0 && y2 >= 0 {
                    let prev =
                        prev_board.get(x2.try_into().unwrap(), y2.try_into().unwrap()) as i32;
                    count += prev;
                }
            }
        }
    }
    let prev = prev_board.get(x, y);
    count == 3 || (prev && count == 2)
}

fn update_fast(prev_board: &Board, i: usize) -> bool {
    let count = prev_board.count_neighbors_fast(i);
    let prev = prev_board.get_fast(i);
    count == 3 || (prev && count == 2)
}
