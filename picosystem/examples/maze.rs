#![no_std]
#![no_main]

use cortex_m_rt::entry;
use display::WIDTH;
use log::info;
use picosystem::{display, hardware, time};

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;

#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

const MAZE_SCALE: i32 = 16;
const MAZE_SIZE: i32 = WIDTH as i32 / MAZE_SCALE;

const N: u8 = 1;
const S: u8 = 2;
const E: u8 = 4;
const W: u8 = 8;

fn draw_cell(display: &mut display::Display, point: Point, cell: Cell, inner_color: Rgb565) {
    let ix = point.x * MAZE_SCALE;
    let iy = point.y * MAZE_SCALE;
    let nw = Point::new(ix, iy);
    let ne = nw + Point::new(MAZE_SCALE, 0);
    let sw = nw + Point::new(0, MAZE_SCALE);
    Rectangle::new(
        nw + Point::new(3, 2),
        Size::new(MAZE_SCALE as u32 - 4, MAZE_SCALE as u32 - 4),
    )
    .into_styled(PrimitiveStyleBuilder::new().fill_color(inner_color).build())
    .draw(display)
    .unwrap();

    let wall_style = PrimitiveStyle::with_stroke(Rgb565::BLUE, 1);
    if !cell.has_tunnel(N) {
        Line::new(ne, nw)
            .into_styled(wall_style)
            .draw(display)
            .unwrap();
    }
    if !cell.has_tunnel(W) {
        Line::new(nw, sw)
            .into_styled(wall_style)
            .draw(display)
            .unwrap();
    }
}

fn draw_link(display: &mut display::Display, point: Point, dir: u8, color: Rgb565) {
    let offset = Point::new(MAZE_SCALE / 2, MAZE_SCALE / 2);
    let v0 = point * MAZE_SCALE + offset;
    let v1 = v0 + delta(dir) * MAZE_SCALE;
    Line::new(v0, v1)
        .into_styled(PrimitiveStyle::with_stroke(color, MAZE_SCALE as u32 - 3))
        .draw(display)
        .unwrap();
}

fn draw_maze(display: &mut display::Display, maze: &Maze, cursor: Point, target: Point) {
    display.clear(Rgb565::BLACK).unwrap();
    for y in 0..MAZE_SIZE {
        for x in 0..MAZE_SIZE {
            let p = Point::new(x, y);
            draw_cell(display, p, maze.get(p), Rgb565::BLACK);
        }
    }

    draw_cell(display, cursor, maze.get(cursor), Rgb565::RED);
    draw_cell(display, target, maze.get(target), Rgb565::GREEN);

    let n = WIDTH as i32 - 1;
    let wall_style = PrimitiveStyle::with_stroke(Rgb565::BLUE, 1);
    Line::new(Point::new(0, 0), Point::new(n, 0))
        .into_styled(wall_style)
        .draw(display)
        .unwrap();
    Line::new(Point::new(0, 0), Point::new(0, n))
        .into_styled(wall_style)
        .draw(display)
        .unwrap();
    Line::new(Point::new(n, n), Point::new(n, 0))
        .into_styled(wall_style)
        .draw(display)
        .unwrap();
    Line::new(Point::new(n, n), Point::new(0, n))
        .into_styled(wall_style)
        .draw(display)
        .unwrap();
}

#[entry]
fn main() -> ! {
    let mut hw = hardware::Hardware::new();
    info!("Finished initialization");

    loop {
        let mut cursor = Point::new(0, 0);
        let target = Point::new(MAZE_SIZE - 1, MAZE_SIZE - 1);
        let mut maze = generate_maze(cursor, &mut hw.display);
        maze.set_visited(cursor, true);

        draw_maze(&mut hw.display, &maze, cursor, target);
        hw.display.flush();

        let mut frame = 0;
        let mut prev_time_us = time::time_us();
        let mut prev_frame = 0;
        loop {
            let mut dir: Option<u8> = None;
            if hw.input.dpad_left.is_pressed() {
                dir = Some(W);
            } else if hw.input.dpad_right.is_pressed() {
                dir = Some(E);
            } else if hw.input.dpad_up.is_pressed() {
                dir = Some(N);
            } else if hw.input.dpad_down.is_pressed() {
                dir = Some(S);
            }
            if let Some(d) = dir {
                let next_cursor = cursor + delta(d);
                if valid(next_cursor) && maze.get(cursor).has_tunnel(d) {
                    if maze.get_visited(next_cursor) {
                        draw_cell(&mut hw.display, cursor, maze.get(cursor), Rgb565::BLACK);
                        draw_link(&mut hw.display, cursor, d, Rgb565::BLACK);
                        maze.set_visited(cursor, false);
                    } else {
                        let path_color = Rgb565::new(200, 0, 0);
                        draw_cell(&mut hw.display, cursor, maze.get(cursor), path_color);
                        draw_link(&mut hw.display, cursor, d, path_color);
                    }
                    cursor = next_cursor;
                    draw_cell(&mut hw.display, cursor, maze.get(cursor), Rgb565::RED);
                    maze.set_visited(cursor, true);
                    hw.display.flush();
                }
            }

            if cursor == target {
                hw.audio.start_tone(440);
                hw.delay.delay_ms(100);
                hw.audio.start_tone(880);
                hw.delay.delay_ms(100);
                hw.audio.stop();
                break;
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
}

fn opposite(direction: u8) -> u8 {
    match direction {
        N => S,
        S => N,
        E => W,
        W => E,
        _ => unimplemented!(),
    }
}

fn delta(direction: u8) -> Point {
    match direction {
        N => Point::new(0, -1),
        S => Point::new(0, 1),
        E => Point::new(1, 0),
        W => Point::new(-1, 0),
        _ => unimplemented!(),
    }
}

fn directions() -> heapless::Vec<u8, 4> {
    heapless::Vec::from_slice(&[N, S, E, W]).unwrap()
}

fn valid(point: Point) -> bool {
    point.x >= 0 && point.x < MAZE_SIZE && point.y >= 0 && point.y < MAZE_SIZE
}

#[derive(Clone, Copy)]
struct Cell {
    data: u8,
}

impl Cell {
    pub fn new() -> Self {
        Cell { data: 0 }
    }

    pub fn has_tunnel(&self, direction: u8) -> bool {
        (self.data & direction) != 0
    }

    pub fn set_tunnel(&mut self, direction: u8) {
        self.data |= direction;
    }

    pub fn visited(&self) -> bool {
        self.data != 0
    }
}

struct Maze {
    cells: [Cell; (MAZE_SIZE * MAZE_SIZE) as usize],
    visited: [bool; (MAZE_SIZE * MAZE_SIZE) as usize],
}

impl Maze {
    pub fn new() -> Self {
        Maze {
            cells: [Cell::new(); (MAZE_SIZE * MAZE_SIZE) as usize],
            visited: [false; (MAZE_SIZE * MAZE_SIZE) as usize],
        }
    }

    fn index(point: Point) -> usize {
        (point.y * MAZE_SIZE + point.x) as usize
    }

    pub fn get(&self, point: Point) -> Cell {
        self.cells[Maze::index(point)]
    }

    pub fn get_mut(&mut self, point: Point) -> &mut Cell {
        &mut self.cells[Maze::index(point)]
    }

    pub fn get_visited(&self, point: Point) -> bool {
        self.visited[Maze::index(point)]
    }

    pub fn set_visited(&mut self, point: Point, v: bool) {
        self.visited[Maze::index(point)] = v;
    }

    pub fn clear_visited(&mut self) {
        self.visited = [false; (MAZE_SIZE * MAZE_SIZE) as usize];
    }
}

fn generate_maze(start: Point, display: &mut display::Display) -> Maze {
    let mut maze = Maze::new();
    const STACK_SIZE: usize = (MAZE_SIZE * MAZE_SIZE) as usize;
    let mut stack = heapless::Vec::<Point, STACK_SIZE>::new();
    stack.push(start).unwrap();
    let mut rng = oorandom::Rand32::new(time::time_us() as u64);

    let mut pos = start;
    let mut i = 0;
    while !stack.is_empty() {
        maze.set_visited(pos, true);

        let mut options = heapless::Vec::<(u8, Point), 4>::new();
        for dir in directions() {
            let neighbor = pos + delta(dir);
            if valid(neighbor) && !maze.get(neighbor).visited() {
                options.push((dir, neighbor)).unwrap();
            }
        }

        if options.is_empty() {
            draw_cell(display, pos, maze.get(pos), Rgb565::new(50, 50, 50));
            pos = stack.pop().unwrap();
        } else {
            draw_cell(display, pos, maze.get(pos), Rgb565::new(100, 100, 100));
            stack.push(pos).unwrap();
            let n = options.len() as u32;
            let (dir, new_pos) = options[rng.rand_range(0..n) as usize];
            maze.get_mut(pos).set_tunnel(dir);
            maze.get_mut(new_pos).set_tunnel(opposite(dir));
            pos = new_pos;
            draw_cell(display, pos, maze.get(pos), Rgb565::WHITE);
        }
        if i % 8 == 0 {
            display.flush();
        }
        i += 1;
    }

    maze.clear_visited();
    maze
}
