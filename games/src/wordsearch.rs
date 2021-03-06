use core::fmt::Write;
use core::ops::DerefMut;
use picosystem::display::WIDTH;
use picosystem::{hardware, time};

use embedded_graphics::mono_font::{
    ascii::{FONT_10X20, FONT_7X14},
    MonoTextStyle,
};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::{Alignment, Text};

type Word = heapless::String<32>;

const WORDS: &[&str] = &[
    "apple",
    "banana",
    "carrot",
    "durian",
    "eggplant",
    "fig",
    "grape",
    "honeydew",
    "kiwi",
    "lemon",
    "mango",
    "nectarine",
    "orange",
    "peach",
    "plum",
    "quince",
    "raspberry",
    "strawberry",
    "tangerine",
    "watermelon",
];

const GRID_WIDTH: i32 = 20;
const GRID_HEIGHT: i32 = 11;

struct Grid {
    letters: [[char; GRID_WIDTH as usize]; GRID_HEIGHT as usize],
    found: [[bool; GRID_WIDTH as usize]; GRID_HEIGHT as usize],
}

#[derive(Copy, Clone)]
struct GridPoint {
    x: i32,
    y: i32,
}

struct GridRect {
    a: GridPoint,
    b: GridPoint,
}

pub fn main(hw: &mut hardware::Hardware) -> ! {
    loop {
        run_game(hw);
    }
}

fn generate_grid() -> (Grid, i32) {
    let mut count = 0;
    let mut grid = Grid {
        letters: [['x'; GRID_WIDTH as usize]; GRID_HEIGHT as usize],
        found: [[false; GRID_WIDTH as usize]; GRID_HEIGHT as usize],
    };
    let mut rng = oorandom::Rand32::new(time::time_us() as u64);
    let mut covered = [[false; GRID_WIDTH as usize]; GRID_HEIGHT as usize];

    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let index = rng.rand_range(0..26);
            let letter = b'a' + index as u8;
            grid.letters[y as usize][x as usize] = letter as char;
        }
    }

    let mut words = WORDS.iter().cloned().collect::<heapless::Vec<_, 32>>();
    words
        .deref_mut()
        .sort_unstable_by_key(|word| -(word.len() as i32));

    let mut vertical = rng.rand_range(0..2) == 0;
    for word in words {
        'outer: for _ in 0..100 {
            let x = rng.rand_range(0..(GRID_WIDTH as u32)) as i32;
            let y = rng.rand_range(0..(GRID_HEIGHT as u32)) as i32;
            for (i, ch) in word.chars().enumerate() {
                let (x2, y2) = if vertical {
                    (x, y + i as i32)
                } else {
                    (x + i as i32, y)
                };
                if !(0..GRID_WIDTH).contains(&x2) || !(0..GRID_HEIGHT).contains(&y2) {
                    continue 'outer;
                }
                if covered[y2 as usize][x2 as usize] && grid.letters[y2 as usize][x2 as usize] != ch
                {
                    continue 'outer;
                }
            }
            for (i, c) in word.chars().enumerate() {
                let (x2, y2) = if vertical {
                    (x, y + i as i32)
                } else {
                    (x + i as i32, y)
                };
                grid.letters[y2 as usize][x2 as usize] = c;
                covered[y2 as usize][x2 as usize] = true;
            }
            vertical = !vertical;
            count += 1;
            break;
        }
    }

    (grid, count)
}

fn run_game(hw: &mut hardware::Hardware) {
    let (mut grid, target_count) = generate_grid();
    let mut found_count = 0;
    let mut cursor = GridRect {
        a: GridPoint { x: 0, y: 0 },
        b: GridPoint { x: 0, y: 0 },
    };
    let mut selecting = false;

    loop {
        hw.audio.stop();

        if !selecting {
            if hw.input.dpad_up.is_pressed() && cursor.a.y > 0 {
                cursor.a.y -= 1;
            }
            if hw.input.dpad_down.is_pressed() && cursor.a.y < GRID_HEIGHT - 1 {
                cursor.a.y += 1;
            }
            if hw.input.dpad_left.is_pressed() && cursor.a.x > 0 {
                cursor.a.x -= 1;
            }
            if hw.input.dpad_right.is_pressed() && cursor.a.x < GRID_WIDTH - 1 {
                cursor.a.x += 1;
            }
            cursor.b = cursor.a;

            if hw.input.button_a.is_pressed() {
                selecting = true;
            }
        } else {
            if cursor.a.x == cursor.b.x {
                if hw.input.dpad_up.is_pressed() && cursor.b.y > cursor.a.y {
                    cursor.b.y -= 1;
                }
                if hw.input.dpad_down.is_pressed() && cursor.b.y < GRID_HEIGHT - 1 {
                    cursor.b.y += 1;
                }
            }
            if cursor.a.y == cursor.b.y {
                if hw.input.dpad_left.is_pressed() && cursor.b.x > cursor.a.x {
                    cursor.b.x -= 1;
                }
                if hw.input.dpad_right.is_pressed() && cursor.b.x < GRID_WIDTH - 1 {
                    cursor.b.x += 1;
                }
            }

            if hw.input.button_a.is_pressed() {
                let word = get_word(&grid, &cursor);
                if WORDS.contains(&word.as_str()) && !is_found(&grid, &cursor) {
                    mark_found(&mut grid, &cursor);
                    selecting = false;
                    cursor.b = cursor.a;
                    log::info!("found {}", word);
                    found_count += 1;
                    if found_count == target_count {
                        animate_win(hw);
                        break;
                    } else {
                        hw.audio.start_tone(800);
                    }
                }
            } else if hw.input.button_b.is_pressed() {
                selecting = false;
            }
        }

        draw(hw, &grid, &cursor, selecting, found_count, target_count);
    }
}

fn get_word(grid: &Grid, rect: &GridRect) -> Word {
    let mut word = Word::new();
    for y in rect.a.y..=rect.b.y {
        for x in rect.a.x..=rect.b.x {
            word.push(grid.letters[y as usize][x as usize]).unwrap();
        }
    }
    word
}

fn mark_found(grid: &mut Grid, rect: &GridRect) {
    for y in rect.a.y..=rect.b.y {
        for x in rect.a.x..=rect.b.x {
            grid.found[y as usize][x as usize] = true;
        }
    }
}

fn is_found(grid: &Grid, rect: &GridRect) -> bool {
    for y in rect.a.y..=rect.b.y {
        for x in rect.a.x..=rect.b.x {
            if !grid.found[y as usize][x as usize] {
                return false;
            }
        }
    }
    true
}

const LETTER_WIDTH: i32 = 12;
const LETTER_HEIGHT: i32 = 20;

fn transform(point: GridPoint) -> Point {
    Point::new(point.x * LETTER_WIDTH, point.y * LETTER_HEIGHT + 12)
}

fn draw(
    hw: &mut hardware::Hardware,
    grid: &Grid,
    cursor: &GridRect,
    selecting: bool,
    found_count: i32,
    target_count: i32,
) {
    hw.draw(|display| {
        display.clear(Rgb565::BLACK).unwrap();

        let normal_text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::CSS_LIGHT_SLATE_GRAY);
        let found_text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::CSS_FOREST_GREEN);

        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                let pos = transform(GridPoint { x, y }) + Point::new(1, LETTER_HEIGHT);
                let letter = grid.letters[y as usize][x as usize];
                let mut s = heapless::String::<1>::new();
                s.push(letter).unwrap();
                let text_style = if grid.found[y as usize][x as usize] {
                    found_text_style
                } else {
                    normal_text_style
                };
                Text::new(&s, pos, text_style).draw(display).unwrap();
            }
        }

        {
            let color = if selecting {
                Rgb565::GREEN
            } else {
                Rgb565::RED
            };
            let style = PrimitiveStyleBuilder::new()
                .stroke_color(color)
                .stroke_width(1)
                .build();
            Rectangle::with_corners(
                transform(cursor.a) + Point::new(0, 3),
                transform(cursor.b) + Point::new(LETTER_WIDTH, LETTER_HEIGHT + 3),
            )
            .into_styled(style)
            .draw(display)
            .unwrap();
        }

        let mut s = heapless::String::<32>::new();
        write!(s, "Found: {}/{}", found_count, target_count).unwrap();
        Text::new(
            &s,
            Point::new(WIDTH as i32 - 12 * 7, 14),
            MonoTextStyle::new(&FONT_7X14, Rgb565::CSS_YELLOW),
        )
        .draw(display)
        .unwrap();
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
