#![no_std]
#![no_main]

mod blob;
mod draw;
mod hangman;
mod invaders;
mod life;
mod maze;
mod system;
mod tanks;
mod wordsearch;

#[cfg(feature = "music")]
mod music;

use cortex_m_rt::entry;
use log::info;
use picosystem::display::{Display, HEIGHT, WIDTH};
use picosystem::fps_monitor::FpsMonitor;
use picosystem::hardware;
use picosystem::time;

use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Alignment, Text},
};
use heapless::Vec;
use micromath::vector::I16x2;

struct MenuItem {
    name: &'static str,
    main: fn(&mut hardware::Hardware) -> !,
}

#[entry]
fn main() -> ! {
    let mut hw = hardware::Hardware::new();
    info!("Finished initialization");

    let items = [
        MenuItem {
            name: "maze",
            main: maze::main,
        },
        MenuItem {
            name: "draw",
            main: draw::main,
        },
        MenuItem {
            name: "life",
            main: life::main,
        },
        #[cfg(feature = "music")]
        MenuItem {
            name: "music",
            main: music::main,
        },
        MenuItem {
            name: "tanks",
            main: tanks::main,
        },
        MenuItem {
            name: "hangman",
            main: hangman::main,
        },
        MenuItem {
            name: "invaders",
            main: invaders::main,
        },
        MenuItem {
            name: "blob",
            main: blob::main,
        },
        MenuItem {
            name: "wordsearch",
            main: wordsearch::main,
        },
    ];

    let mut selected_index = 0;

    let mut stars = Stars::new();
    stars.populate();

    let mut fps_monitor = FpsMonitor::new();

    loop {
        if hw.input.dpad_up.is_pressed() && selected_index > 0 {
            selected_index -= 1;
        } else if hw.input.dpad_down.is_pressed() && selected_index < items.len() - 1 {
            selected_index += 1;
        } else if hw.input.button_a.is_pressed() {
            break;
        } else if hw.input.button_x.is_held() && hw.input.button_y.is_held() {
            system::main(&mut hw);
        }

        hw.display.draw(|display| {
            display.clear(Rgb565::BLACK).unwrap();
            stars.draw(display);

            for (i, item) in items.iter().enumerate() {
                let color = if i == selected_index {
                    Rgb565::GREEN
                } else {
                    Rgb565::WHITE
                };
                let text_style = MonoTextStyle::new(&FONT_10X20, color);
                const SPACING: i32 = 22;
                let y_offset = (HEIGHT as i32 - items.len() as i32 * SPACING) / 2;
                Text::with_alignment(
                    item.name,
                    Point::new(WIDTH as i32 / 2, y_offset + (i as i32) * SPACING),
                    text_style,
                    Alignment::Center,
                )
                .draw(display)
                .unwrap();
            }
        });

        stars.update();
        fps_monitor.update();
    }

    hw.display.draw(|display| {
        display.clear(Rgb565::BLACK).unwrap();
    });

    (items[selected_index].main)(&mut hw)
}

const FRACTION: i32 = 64;

fn transform(p: I16x2) -> Point {
    Point::new(p.x as i32 / FRACTION, p.y as i32 / FRACTION)
}

#[derive(Debug, Clone)]
struct Star {
    p: I16x2,
    v: I16x2,
    color: Rgb565,
}

struct Stars {
    stars: Vec<Star, 100>,
    rng: oorandom::Rand32,
}

impl Stars {
    const MAX_WIDTH: i16 = WIDTH as i16 * FRACTION as i16;
    const MAX_HEIGHT: i16 = HEIGHT as i16 * FRACTION as i16;

    fn new() -> Self {
        Self {
            stars: Vec::new(),
            rng: oorandom::Rand32::new(time::time_us() as u64),
        }
    }

    fn add_star(&mut self, p: I16x2, v: I16x2, color: Rgb565) {
        self.stars.push(Star { p, v, color }).unwrap();
    }

    fn rand16(&mut self, min: i16, max: i16) -> i16 {
        self.rng.rand_range((min as u32)..(max as u32)) as i16
    }

    fn populate(&mut self) {
        let minv = FRACTION;
        let maxv = 3 * FRACTION;
        for _ in 0..100 {
            let p = I16x2 {
                x: self.rand16(0, Self::MAX_WIDTH),
                y: self.rand16(0, Self::MAX_HEIGHT),
            };
            let v = I16x2 {
                x: 0,
                y: self.rand16(minv as i16, maxv as i16),
            };
            let color = Rgb565::new(
                self.rand16(0, 255) as u8,
                self.rand16(0, 255) as u8,
                self.rand16(0, 255) as u8,
            );
            self.add_star(p, v, color);
        }
    }

    #[allow(clippy::assign_op_pattern)]
    fn update(&mut self) {
        for i in 0..self.stars.len() {
            self.stars[i].p = self.stars[i].p + self.stars[i].v;
            if self.stars[i].p.y >= Self::MAX_HEIGHT {
                self.stars[i].p = I16x2 {
                    x: self.rand16(0, Self::MAX_WIDTH),
                    y: 0,
                };
            }
        }
    }

    fn draw(&self, display: &mut Display) {
        for star in self.stars.iter() {
            let mut color = star.color;
            for i in 0..2 {
                Pixel(transform(star.p) - Point::new(0, i), color)
                    .draw(display)
                    .unwrap();
                color = Rgb565::new(color.r() / 2, color.g() / 2, color.b() / 2);
            }
        }
    }
}
