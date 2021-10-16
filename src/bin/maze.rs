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

fn draw_block(display: &mut display::Display, point: Point, color: Rgb565) {
    let ix = point.x * MAZE_SCALE;
    let iy = point.y * MAZE_SCALE;
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

    let mut cursor = Point::new(0, 0);
    let target = Point::new(MAZE_SIZE - 1, MAZE_SIZE - 1);
    let mut maze = generate_maze(cursor, target, &mut hw.display);

    hw.display.clear(Rgb565::BLACK).unwrap();
    for y in 0..MAZE_SIZE {
        for x in 0..MAZE_SIZE {
            let p = Point::new(x, y);
            if maze.get_wall(p) {
                draw_block(&mut hw.display, p, Rgb565::BLUE);
            }
        }
    }

    let path_color = Rgb565::new(200, 0, 0);
    draw_block(&mut hw.display, cursor, Rgb565::RED);
    draw_block(&mut hw.display, target, Rgb565::GREEN);
    hw.display.flush();

    let mut frame = 0;
    let mut prev_time_us = time::time_us();
    let mut prev_frame = 0;
    loop {
        let mut next_cursor = cursor.clone();
        if hw.input.dpad_left.is_pressed() && cursor.x > 0 {
            next_cursor.x -= 1;
        }
        if hw.input.dpad_right.is_pressed() && cursor.x < MAZE_SIZE - 1 {
            next_cursor.x += 1;
        }
        if hw.input.dpad_up.is_pressed() && cursor.y > 0 {
            next_cursor.y -= 1;
        }
        if hw.input.dpad_down.is_pressed() && cursor.y < MAZE_SIZE - 1 {
            next_cursor.y += 1;
        }
        if next_cursor != cursor && !maze.get_wall(next_cursor) {
            if maze.get_visited(next_cursor) {
                draw_block(&mut hw.display, cursor, Rgb565::BLACK);
                maze.set_visited(cursor, false);
            } else {
                draw_block(&mut hw.display, cursor, path_color);
            }
            cursor = next_cursor;
            draw_block(&mut hw.display, cursor, Rgb565::RED);
            maze.set_visited(cursor, true);
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

const MAZE_SCALE: i32 = 16;
const MAZE_SIZE: i32 = WIDTH as i32 / MAZE_SCALE;

struct Maze {
    walls: [bool; (MAZE_SIZE * MAZE_SIZE) as usize],
    visited: [bool; (MAZE_SIZE * MAZE_SIZE) as usize],
}

impl Maze {
    pub fn new() -> Self {
        Maze {
            walls: [false; (MAZE_SIZE * MAZE_SIZE) as usize],
            visited: [false; (MAZE_SIZE * MAZE_SIZE) as usize],
        }
    }

    fn index(point: Point) -> usize {
        (point.y * MAZE_SIZE + point.x) as usize
    }

    pub fn get_wall(&self, point: Point) -> bool {
        self.walls[Maze::index(point)]
    }

    pub fn set_wall(&mut self, point: Point, v: bool) {
        self.walls[Maze::index(point)] = v;
    }

    pub fn get_visited(&self, point: Point) -> bool {
        self.visited[Maze::index(point)]
    }

    pub fn set_visited(&mut self, point: Point, v: bool) {
        self.visited[Maze::index(point)] = v;
    }
}

fn neighbors(pos: Point) -> heapless::Vec<Point, 4> {
    let mut result = heapless::Vec::<Point, 4>::new();
    if pos.x > 0 {
        result.push(pos + Point::new(-1, 0)).unwrap();
    }
    if pos.x < MAZE_SIZE - 1 {
        result.push(pos + Point::new(1, 0)).unwrap();
    }
    if pos.y > 0 {
        result.push(pos + Point::new(0, -1)).unwrap();
    }
    if pos.y < MAZE_SIZE - 1 {
        result.push(pos + Point::new(0, 1)).unwrap();
    }
    result
}

fn generate_maze(start: Point, target: Point, display: &mut display::Display) -> Maze {
    let mut maze = Maze::new();
    const STACK_SIZE: usize = (MAZE_SIZE * MAZE_SIZE) as usize;
    let mut stack = heapless::Vec::<Point, STACK_SIZE>::new();
    let mut rng = oorandom::Rand32::new(time::time_us() as u64);

    let mut pos = start;
    while pos != target {
        maze.set_visited(pos, true);
        let mut options = heapless::Vec::<Point, 4>::new();
        for &p in neighbors(pos).iter() {
            if !maze.get_visited(p) {
                let mut good = true;
                for &p2 in neighbors(p).iter() {
                    if p2 != pos && maze.get_visited(p2) {
                        good = false;
                    }
                }
                if good {
                    options.push(p).unwrap();
                }
            }
        }
        if options.is_empty() {
            draw_block(display, pos, Rgb565::new(50, 50, 50));
            pos = stack.pop().unwrap();
        } else {
            draw_block(display, pos, Rgb565::new(100, 100, 100));
            stack.push(pos).unwrap();
            let n = options.len() as u32;
            pos = options[rng.rand_range(0..n) as usize];
            draw_block(display, pos, Rgb565::WHITE);
        }
        display.flush();
    }
    maze.set_visited(pos, true);

    for x in 0..MAZE_SIZE {
        for y in 0..MAZE_SIZE {
            let p = Point::new(x, y);
            if !maze.get_visited(p) {
                maze.set_wall(p, true);
            } else {
                maze.set_visited(p, false);
            }
        }
    }

    maze
}
