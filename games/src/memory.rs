use picosystem::display::WIDTH;
use picosystem::{hardware, time};

use embedded_graphics::mono_font::{ascii::FONT_10X20, MonoTextStyle};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::{Alignment, Baseline, Text, TextStyleBuilder};

#[derive(Debug, Clone, Copy)]
struct Item {
    word: &'static str,
}

const GRID_WIDTH: i32 = 4;
const GRID_HEIGHT: i32 = 4;
const NUM_CELLS: i32 = GRID_WIDTH * GRID_HEIGHT;
const CELL_WIDTH: i32 = WIDTH as i32 / GRID_WIDTH - 10;
const CELL_HEIGHT: i32 = CELL_WIDTH;

struct Grid {
    items: [[Item; GRID_WIDTH as usize]; GRID_HEIGHT as usize],
    found: [[bool; GRID_WIDTH as usize]; GRID_HEIGHT as usize],
}

impl Grid {
    fn new() -> Self {
        Grid {
            items: [[Item { word: "" }; GRID_WIDTH as usize]; GRID_HEIGHT as usize],
            found: [[false; GRID_WIDTH as usize]; GRID_HEIGHT as usize],
        }
    }

    fn get_item(&self, p: GridPoint) -> &Item {
        if p.x < 0 || p.x >= GRID_WIDTH || p.y < 0 || p.y >= GRID_HEIGHT {
            panic!("Invalid grid point: {:?}", p);
        }
        &self.items[p.y as usize][p.x as usize]
    }

    fn get_found(&self, p: GridPoint) -> bool {
        if p.x < 0 || p.x >= GRID_WIDTH || p.y < 0 || p.y >= GRID_HEIGHT {
            panic!("Invalid grid point: {:?}", p);
        }
        self.found[p.y as usize][p.x as usize]
    }

    fn set_found(&mut self, p: GridPoint, v: bool) {
        if p.x < 0 || p.x >= GRID_WIDTH || p.y < 0 || p.y >= GRID_HEIGHT {
            panic!("Invalid grid point: {:?}", p);
        }
        self.found[p.y as usize][p.x as usize] = v;
    }

    fn all_found(&self) -> bool {
        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                if !self.found[y as usize][x as usize] {
                    return false;
                }
            }
        }
        true
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct GridPoint {
    x: i32,
    y: i32,
}

pub fn main(hw: &mut hardware::Hardware) -> ! {
    loop {
        run_game(hw);
    }
}

fn generate_grid() -> Grid {
    let mut grid = Grid::new();
    let mut rng = oorandom::Rand32::new(time::time_us() as u64);

    let mut words = heapless::Vec::<_, 32>::new();
    for word in &[
        "cat", "dog", "cow", "pig", "emu", "ant", "fox", "owl", "bat", "bee", "rat",
    ] {
        words.push(word).unwrap();
    }

    for i in 0..words.len() {
        let j = rng.rand_range(i as u32..words.len() as u32);
        words.swap(i, j as usize);
    }

    while words.len() > NUM_CELLS as usize / 2 {
        words.pop();
    }

    for i in 0..(NUM_CELLS as usize / 2) {
        words.push(words[i]).unwrap();
    }

    for i in 0..words.len() {
        let j = rng.rand_range(i as u32..words.len() as u32);
        words.swap(i, j as usize);
    }

    let mut i = 0;
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let p = GridPoint { x, y };
            if NUM_CELLS % 2 == 1 && x == y && x == GRID_WIDTH / 2 {
                grid.set_found(p, true);
                continue;
            }
            grid.items[y as usize][x as usize] = Item { word: words[i] };
            i += 1;
        }
    }
    grid
}

fn run_game(hw: &mut hardware::Hardware) {
    let mut grid = generate_grid();
    let mut cursor = GridPoint { x: 0, y: 0 };
    let mut revealed: Option<GridPoint> = None;

    loop {
        hw.audio.stop();

        if hw.input.dpad_up.is_pressed() && cursor.y > 0 {
            cursor.y -= 1;
        }
        if hw.input.dpad_down.is_pressed() && cursor.y < GRID_HEIGHT - 1 {
            cursor.y += 1;
        }
        if hw.input.dpad_left.is_pressed() && cursor.x > 0 {
            cursor.x -= 1;
        }
        if hw.input.dpad_right.is_pressed() && cursor.x < GRID_WIDTH - 1 {
            cursor.x += 1;
        }

        if hw.input.button_a.is_pressed() {
            if revealed.is_none() && !grid.get_found(cursor) {
                revealed = Some(cursor);
                grid.set_found(cursor, true);
                grid.found[cursor.y as usize][cursor.x as usize] = true;
            } else if revealed.is_some() && !grid.get_found(cursor) {
                let r = revealed.unwrap();
                grid.set_found(cursor, true);
                let found = grid.get_item(r).word == grid.get_item(cursor).word;
                draw(hw, &grid, &cursor, revealed, found);
                if found {
                    hw.audio.start_tone(880);
                } else {
                    hw.audio.start_tone(220);
                }
                hw.delay.delay_ms(100);
                hw.audio.stop();
                hw.delay.delay_ms(400);
                if !found {
                    grid.set_found(cursor, false);
                    grid.set_found(r, false);
                }
                revealed = None;
            }
        } else if hw.input.button_b.is_pressed() && revealed.is_some() {
            let r = revealed.unwrap();
            grid.set_found(r, false);
            revealed = None;
        }

        draw(hw, &grid, &cursor, revealed, false);

        if grid.all_found() {
            animate_win(hw);
            break;
        }
    }
}

fn transform(point: GridPoint) -> Point {
    Point::new(point.x * CELL_WIDTH + 20, point.y * CELL_HEIGHT + 20)
}

fn draw(
    hw: &mut hardware::Hardware,
    grid: &Grid,
    cursor: &GridPoint,
    revealed: Option<GridPoint>,
    matched: bool,
) {
    hw.draw(|display| {
        display.clear(Rgb565::BLACK).unwrap();

        let text_style = TextStyleBuilder::new()
            .alignment(Alignment::Center)
            .baseline(Baseline::Middle)
            .build();

        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                let gp = GridPoint { x, y };
                let pos = transform(gp);
                let char_style = if matched && (*cursor == gp || revealed == Some(gp)) {
                    MonoTextStyle::new(&FONT_10X20, Rgb565::CSS_GREEN)
                } else if revealed == Some(GridPoint { x, y }) {
                    MonoTextStyle::new(&FONT_10X20, Rgb565::CSS_SALMON)
                } else {
                    MonoTextStyle::new(&FONT_10X20, Rgb565::CSS_LIGHT_SLATE_GRAY)
                };
                let t = if grid.found[y as usize][x as usize] {
                    grid.get_item(GridPoint { x, y }).word
                } else {
                    "?"
                };
                Text::with_text_style(
                    t,
                    pos + Point::new(CELL_WIDTH / 2, CELL_HEIGHT / 2),
                    char_style,
                    text_style,
                )
                .draw(display)
                .unwrap();
                let style = PrimitiveStyleBuilder::new()
                    .stroke_color(Rgb565::CSS_DARK_SLATE_GRAY)
                    .stroke_width(1)
                    .build();
                Rectangle::with_corners(pos, transform(GridPoint { x: x + 1, y: y + 1 }))
                    .into_styled(style)
                    .draw(display)
                    .unwrap();
            }
        }

        {
            let off = Point::new(2, 2);
            let style = PrimitiveStyleBuilder::new()
                .stroke_color(Rgb565::CSS_GREEN)
                .stroke_width(2)
                .build();
            Rectangle::with_corners(
                transform(*cursor) + off,
                transform(GridPoint {
                    x: cursor.x + 1,
                    y: cursor.y + 1,
                }) - off,
            )
            .into_styled(style)
            .draw(display)
            .unwrap();
        }
    });
}

fn animate_win(hw: &mut hardware::Hardware) {
    hw.draw(|display| {
        Rectangle::new(Point::new(40, 100), Size::new(160, 40))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_GREEN))
            .draw(display)
            .unwrap();
        Text::with_alignment(
            "You win!",
            Point::new(120, 127),
            MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE),
            Alignment::Center,
        )
        .draw(display)
        .unwrap();
    });

    hw.audio.start_tone(440);
    hw.delay.delay_ms(100);
    hw.audio.start_tone(880);
    hw.delay.delay_ms(100);
    hw.audio.stop();

    hw.delay.delay_ms(2000);
}
