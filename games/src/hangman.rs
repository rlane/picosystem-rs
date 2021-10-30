use picosystem::{display::WIDTH, hardware, time};

use embedded_graphics::mono_font::{ascii::FONT_10X20, MonoTextStyle};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::{Alignment, Text};

type Word = heapless::String<16>;

const NUM_LETTERS: i32 = 26;

pub fn main(hw: &mut hardware::Hardware) -> ! {
    loop {
        run_game(hw);
    }
}

fn letter_from_index(mut index: i32) -> char {
    if index < 0 {
        index += NUM_LETTERS;
    }
    if index >= NUM_LETTERS {
        index -= NUM_LETTERS;
    }
    (index as u8 + b'a') as char
}

fn pick_word() -> Word {
    let words = [
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
    let mut rng = oorandom::Rand32::new(time::time_us() as u64);
    let index = rng.rand_range(0..(words.len() as u32));
    words[index as usize].into()
}

fn run_game(hw: &mut hardware::Hardware) {
    let mut errors = 0;
    let target = pick_word();
    let mut guessed = heapless::Vec::<char, 26>::new();
    let mut letter_index: i32 = 0;

    loop {
        let letter = letter_from_index(letter_index);
        if hw.input.button_a.is_pressed() {
            if !guessed.contains(&letter) {
                guessed.push(letter).unwrap();
                if !target.contains(letter) {
                    errors += 1;
                    if errors == 6 {
                        draw(hw, &target, &guessed, errors, letter_index);
                        animate_lose(hw);
                        return;
                    }
                    hw.audio.start_tone(200);
                } else {
                    if target.chars().all(|c| guessed.contains(&c)) {
                        draw(hw, &target, &guessed, errors, letter_index);
                        animate_win(hw);
                        return;
                    }
                    hw.audio.start_tone(880);
                }
                hw.delay.delay_ms(50);
                hw.audio.stop();
            }
        } else if hw.input.dpad_right.is_pressed() {
            letter_index = (letter_index + 1) % NUM_LETTERS;
        } else if hw.input.dpad_left.is_pressed() {
            if letter_index == 0 {
                letter_index = NUM_LETTERS - 1;
            } else {
                letter_index -= 1;
            }
        }

        draw(hw, &target, &guessed, errors, letter_index);
    }
}

fn draw(
    hw: &mut hardware::Hardware,
    target: &Word,
    guessed: &heapless::Vec<char, 26>,
    errors: u8,
    letter_index: i32,
) {
    hw.display.clear(Rgb565::BLACK).unwrap();
    const LETTER_WIDTH: i32 = 10;

    let mut guess = Word::new();
    for ch in target.chars() {
        if guessed.contains(&ch) {
            guess.push(ch).unwrap();
        } else {
            guess.push('_').unwrap();
        }
    }
    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
    Text::new(
        &guess,
        Point::new((WIDTH as i32 - LETTER_WIDTH * guess.len() as i32) / 2, 160),
        text_style,
    )
    .draw(&mut hw.display)
    .unwrap();

    let mut letters = heapless::String::<26>::new();
    let mut crosses = heapless::String::<26>::new();
    const LETTERS_DISPLAYED: i32 = 9;
    for i in 0..LETTERS_DISPLAYED {
        let offset = i - LETTERS_DISPLAYED / 2;
        let ch = letter_from_index(letter_index + offset);
        if offset != 0 {
            letters.push(ch).unwrap();
        } else {
            letters.push(' ').unwrap();
        }
        if guessed.contains(&ch) {
            crosses.push('X').unwrap();
        } else {
            crosses.push(' ').unwrap();
        }
    }
    Text::new(
        &letters,
        Point::new((WIDTH as i32 - LETTERS_DISPLAYED * LETTER_WIDTH) / 2, 200),
        MonoTextStyle::new(&FONT_10X20, Rgb565::CSS_LIGHT_SLATE_GRAY),
    )
    .draw(&mut hw.display)
    .unwrap();

    letters.clear();
    letters.push(letter_from_index(letter_index)).unwrap();
    Text::new(
        &letters,
        Point::new((WIDTH as i32 - LETTER_WIDTH) / 2, 199),
        MonoTextStyle::new(&FONT_10X20, Rgb565::GREEN),
    )
    .draw(&mut hw.display)
    .unwrap();

    Text::new(
        &crosses,
        Point::new((WIDTH as i32 - LETTERS_DISPLAYED * LETTER_WIDTH) / 2, 202),
        MonoTextStyle::new(&FONT_10X20, Rgb565::CSS_DARK_RED),
    )
    .draw(&mut hw.display)
    .unwrap();

    let mid = WIDTH as i32 / 2;
    let top = 30;

    {
        let style = PrimitiveStyleBuilder::new()
            .stroke_color(Rgb565::RED)
            .stroke_width(1)
            .build();
        if errors > 0 {
            let head = Circle::with_center(Point::new(mid, top + 10), 20).into_styled(style);
            head.draw(&mut hw.display).unwrap();
        }

        if errors > 1 {
            let body =
                Line::new(Point::new(mid, top + 20), Point::new(mid, top + 60)).into_styled(style);
            body.draw(&mut hw.display).unwrap();
        }

        if errors > 2 {
            let left_arm = Line::new(Point::new(mid, top + 30), Point::new(mid - 15, top + 40))
                .into_styled(style);
            left_arm.draw(&mut hw.display).unwrap();
        }

        if errors > 3 {
            let right_arm = Line::new(Point::new(mid, top + 30), Point::new(mid + 15, top + 40))
                .into_styled(style);
            right_arm.draw(&mut hw.display).unwrap();
        }

        if errors > 4 {
            let left_leg = Line::new(Point::new(mid, top + 60), Point::new(mid - 10, top + 80))
                .into_styled(style);
            left_leg.draw(&mut hw.display).unwrap();
        }

        if errors > 5 {
            let right_leg = Line::new(Point::new(mid, top + 60), Point::new(mid + 10, top + 80))
                .into_styled(style);
            right_leg.draw(&mut hw.display).unwrap();
        }
    }

    {
        let style = PrimitiveStyleBuilder::new()
            .stroke_color(Rgb565::CSS_BROWN)
            .stroke_width(1)
            .build();

        Line::new(
            Point::new(mid - 60, top + 90),
            Point::new(mid - 20, top + 90),
        )
        .into_styled(style)
        .draw(&mut hw.display)
        .unwrap();

        Line::new(
            Point::new(mid - 40, top - 10),
            Point::new(mid - 40, top + 90),
        )
        .into_styled(style)
        .draw(&mut hw.display)
        .unwrap();

        Line::new(Point::new(mid - 40, top - 10), Point::new(mid, top - 10))
            .into_styled(style)
            .draw(&mut hw.display)
            .unwrap();

        Line::new(Point::new(mid, top - 10), Point::new(mid, top))
            .into_styled(style)
            .draw(&mut hw.display)
            .unwrap();
    }

    hw.display.flush();
}

fn animate_win(hw: &mut hardware::Hardware) {
    Rectangle::new(Point::new(40, 100), Size::new(160, 40))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_GREEN))
        .draw(&mut hw.display)
        .unwrap();
    Text::with_alignment(
        "You win!",
        Point::new(120, 127),
        MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE),
        Alignment::Center,
    )
    .draw(&mut hw.display)
    .unwrap();
    hw.display.flush();

    hw.audio.start_tone(440);
    hw.delay.delay_ms(100);
    hw.audio.start_tone(880);
    hw.delay.delay_ms(100);
    hw.audio.stop();

    hw.delay.delay_ms(2000);
}

fn animate_lose(hw: &mut hardware::Hardware) {
    Rectangle::new(Point::new(40, 100), Size::new(160, 40))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_DARK_RED))
        .draw(&mut hw.display)
        .unwrap();
    Text::with_alignment(
        "You lose!",
        Point::new(120, 127),
        MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE),
        Alignment::Center,
    )
    .draw(&mut hw.display)
    .unwrap();

    hw.display.flush();
    hw.audio.start_tone(400);
    hw.delay.delay_ms(100);
    hw.audio.start_tone(200);
    hw.delay.delay_ms(100);
    hw.audio.stop();

    hw.delay.delay_ms(2000);
}
