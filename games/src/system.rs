use core::fmt::Write;
use picosystem::display::{HEIGHT, WIDTH};
use picosystem::{hardware, time};

use embedded_graphics::mono_font::{ascii::FONT_10X20, MonoTextStyle};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, Text};

struct MenuItem {
    name: &'static str,
    main: fn(&mut hardware::Hardware) -> (),
}

pub fn main(hw: &mut hardware::Hardware) {
    let items = [MenuItem {
        name: "Battery test",
        main: battery_test,
    }];

    let mut selected_index = 0;

    loop {
        if hw.input.button_b.is_pressed() {
            break;
        } else if hw.input.dpad_up.is_pressed() && selected_index > 0 {
            selected_index -= 1;
        } else if hw.input.dpad_down.is_pressed() && selected_index < items.len() - 1 {
            selected_index += 1;
        } else if hw.input.button_a.is_pressed() {
            (items[selected_index].main)(hw);
            hw.delay.delay_ms(300);
        }

        hw.draw(|display| {
            display.clear(Rgb565::BLACK).unwrap();

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
    }
}

fn battery_test(hw: &mut hardware::Hardware) {
    let mut readings = heapless::Deque::<(u64, u16, f32), 32>::new();
    let mut last_reading = 0;

    loop {
        if hw.input.button_b.is_pressed() {
            break;
        }

        let battery_raw = hw.read_battery_raw_slow();
        let battery_fraction = hw.read_battery_fraction();

        if last_reading == 0 || time::time_us64() - last_reading > 600 * 1_000_000 {
            last_reading = time::time_us64();
            if readings.len() >= 10 {
                readings.pop_front().unwrap();
            }
            readings
                .push_back((last_reading, battery_raw, battery_fraction))
                .unwrap();
        }

        hw.draw(|display| {
            display.clear(Rgb565::BLACK).unwrap();

            let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
            let mut s = heapless::String::<1024>::new();
            writeln!(
                s,
                "now: t={} raw={:.1} {:.0}%",
                time::time_us64() / 1_000_000,
                battery_raw,
                battery_fraction * 100.0
            )
            .unwrap();
            for (t, battery_raw, battery_fraction) in readings.iter() {
                writeln!(
                    s,
                    "t={} raw={:.1} {:.0}%",
                    t / 1_000_000,
                    battery_raw,
                    battery_fraction * 100.0
                )
                .unwrap();
            }
            Text::new(&s, Point::new(1, 20), text_style)
                .draw(display)
                .unwrap();
        });
    }
}
