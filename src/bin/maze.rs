#![no_std]
#![no_main]

use cortex_m_rt::entry;
use display::WIDTH;
use log::info;
use rlane_picosystem_games as rpsg;
use rpsg::{display, hardware, time};

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;

#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

fn draw_block(display: &mut display::Display, x: usize, y: usize, color: Rgb565) {
    let ix = x as i32 * MAZE_SCALE as i32;
    let iy = y as i32 * MAZE_SCALE as i32;
    Rectangle::new(
        Point::new(ix, iy),
        Size::new(MAZE_SCALE as u32, MAZE_SCALE as u32),
    )
    .into_styled(PrimitiveStyleBuilder::new().fill_color(color).build())
    .draw(display)
    .unwrap();
}

#[entry]
fn main() -> ! {
    let mut hw = hardware::Hardware::new();
    info!("Finished initialization");

    let mut maze = generate_maze(&mut hw.display);

    hw.display.clear(Rgb565::BLACK).unwrap();
    for y in 0..MAZE_SIZE {
        for x in 0..MAZE_SIZE {
            if maze.get_wall(x, y) {
                draw_block(&mut hw.display, x, y, Rgb565::BLUE);
            }
        }
    }

    let mut cursorx = 0;
    let mut cursory = 0;
    let path_color = Rgb565::new(200, 0, 0);
    draw_block(&mut hw.display, cursorx, cursory, Rgb565::RED);
    draw_block(&mut hw.display, MAZE_SIZE - 1, MAZE_SIZE - 1, Rgb565::GREEN);
    hw.display.flush();

    let mut frame = 0;
    let mut prev_time_us = time::time_us();
    let mut prev_frame = 0;
    loop {
        let mut next_cursorx = cursorx;
        let mut next_cursory = cursory;
        if hw.input.dpad_left.is_pressed() && cursorx > 0 {
            next_cursorx = cursorx - 1;
        }
        if hw.input.dpad_right.is_pressed() && cursorx < MAZE_SIZE - 1 {
            next_cursorx = cursorx + 1;
        }
        if hw.input.dpad_up.is_pressed() && cursory > 0 {
            next_cursory = cursory - 1;
        }
        if hw.input.dpad_down.is_pressed() && cursory < MAZE_SIZE {
            next_cursory = cursory + 1;
        }
        if (next_cursorx != cursorx || next_cursory != cursory)
            && !maze.get_wall(next_cursorx, next_cursory)
        {
            if maze.get_visited(next_cursorx, next_cursory) {
                draw_block(&mut hw.display, cursorx, cursory, Rgb565::BLACK);
                maze.set_visited(cursorx, cursory, false);
            } else {
                draw_block(&mut hw.display, cursorx, cursory, path_color);
            }
            cursorx = next_cursorx;
            cursory = next_cursory;
            draw_block(&mut hw.display, cursorx, cursory, Rgb565::RED);
            maze.set_visited(cursorx, cursory, true);
            hw.display.flush();
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

const MAZE_SCALE: usize = 16;
const MAZE_SIZE: usize = WIDTH / MAZE_SCALE;

struct Maze {
    walls: [bool; MAZE_SIZE * MAZE_SIZE],
    visited: [bool; MAZE_SIZE * MAZE_SIZE],
}

impl Maze {
    pub fn new() -> Self {
        Maze {
            walls: [false; MAZE_SIZE * MAZE_SIZE],
            visited: [false; MAZE_SIZE * MAZE_SIZE],
        }
    }

    pub fn get_wall(&self, x: usize, y: usize) -> bool {
        self.walls[y * MAZE_SIZE + x]
    }

    pub fn set_wall(&mut self, x: usize, y: usize, v: bool) {
        self.walls[y * MAZE_SIZE + x] = v;
    }

    pub fn get_visited(&self, x: usize, y: usize) -> bool {
        self.visited[y * MAZE_SIZE + x]
    }

    pub fn set_visited(&mut self, x: usize, y: usize, v: bool) {
        self.visited[y * MAZE_SIZE + x] = v;
    }
}

fn neighbors(pos: Point) -> heapless::Vec<Point, 4> {
    let mut result = heapless::Vec::<Point, 4>::new();
    if pos.x > 0 {
        result.push(pos + Point::new(-1, 0)).unwrap();
    }
    if pos.x < MAZE_SIZE as i32 - 1 {
        result.push(pos + Point::new(1, 0)).unwrap();
    }
    if pos.y > 0 {
        result.push(pos + Point::new(0, -1)).unwrap();
    }
    if pos.y < MAZE_SIZE as i32 - 1 {
        result.push(pos + Point::new(0, 1)).unwrap();
    }
    result
}

fn generate_maze(display: &mut display::Display) -> Maze {
    let mut maze = Maze::new();
    const STACK_SIZE: usize = MAZE_SIZE * MAZE_SIZE;
    let mut stack = heapless::Vec::<Point, STACK_SIZE>::new();
    let mut rng = oorandom::Rand32::new(time::time_us() as u64);

    let mut pos = Point::new(0, 0);
    let target = Point::new(MAZE_SIZE as i32 - 1, MAZE_SIZE as i32 - 1);
    while pos != target {
        maze.set_visited(pos.x as usize, pos.y as usize, true);
        let mut options = heapless::Vec::<Point, 4>::new();
        for p in neighbors(pos).iter() {
            if !maze.get_visited(p.x as usize, p.y as usize) {
                let mut good = true;
                for p2 in neighbors(*p).iter() {
                    if *p2 != pos && maze.get_visited(p2.x as usize, p2.y as usize) {
                        good = false;
                    }
                }
                if good {
                    options.push(p.clone()).unwrap();
                }
            }
        }
        if options.is_empty() {
            draw_block(
                display,
                pos.x as usize,
                pos.y as usize,
                Rgb565::new(50, 50, 50),
            );
            pos = stack.pop().unwrap();
        } else {
            draw_block(
                display,
                pos.x as usize,
                pos.y as usize,
                Rgb565::new(100, 100, 100),
            );
            stack.push(pos).unwrap();
            let n = options.len() as u32;
            pos = options[rng.rand_range(0..n) as usize];
            draw_block(display, pos.x as usize, pos.y as usize, Rgb565::WHITE);
        }
        display.flush();
    }
    maze.set_visited(pos.x as usize, pos.y as usize, true);

    for x in 0..MAZE_SIZE {
        for y in 0..MAZE_SIZE {
            if !maze.get_visited(x, y) {
                maze.set_wall(x, y, true);
            } else {
                maze.set_visited(x, y, false);
            }
        }
    }

    maze
}
