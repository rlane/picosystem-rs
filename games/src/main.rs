#![no_std]
#![no_main]

mod draw;
mod hangman;
mod invaders;
mod life;
mod maze;
mod music;
mod tanks;

use cortex_m_rt::entry;
use log::info;
use picosystem::hardware;

use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    text::Text,
};

#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

struct MenuItem {
    name: &'static str,
    main: fn(&mut hardware::Hardware) -> !,
}

#[entry]
fn main() -> ! {
    let mut hw = hardware::Hardware::new();
    info!("Finished initialization");

    hw.display.clear(Rgb565::BLACK).unwrap();
    hw.display.flush();

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
    ];

    let mut selected_index = 0;

    loop {
        if hw.input.dpad_up.is_pressed() && selected_index > 0 {
            selected_index -= 1;
        } else if hw.input.dpad_down.is_pressed() && selected_index < items.len() - 1 {
            selected_index += 1;
        } else if hw.input.button_a.is_pressed() {
            break;
        }

        hw.display.clear(Rgb565::BLACK).unwrap();
        for (i, item) in items.iter().enumerate() {
            let color = if i == selected_index {
                Rgb565::GREEN
            } else {
                Rgb565::WHITE
            };
            let text_style = MonoTextStyle::new(&FONT_10X20, color);
            const SPACING: i32 = 24;
            Text::new(
                item.name,
                Point::new(SPACING, (i as i32 + 1) * SPACING),
                text_style,
            )
            .draw(&mut hw.display)
            .unwrap();
        }
        hw.display.flush();
    }

    hw.display.clear(Rgb565::BLACK).unwrap();
    hw.display.flush();

    (items[selected_index].main)(&mut hw)
}
