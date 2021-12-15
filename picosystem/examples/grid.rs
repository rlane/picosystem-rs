#![no_std]
#![no_main]

use cortex_m_rt::entry;
use log::info;
use picosystem::hardware;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;

#[entry]
fn main() -> ! {
    let mut hw = hardware::Hardware::new();

    info!("Finished initialization");
    let mut x: i32 = 0;
    let mut y: i32 = 0;
    loop {
        hw.draw(|display| {
            display.clear(Rgb565::CSS_DARK_RED).unwrap();
            for i in 0..240 {
                if i % 16 == 0 {
                    Line::new(Point::new(x + i, 0), Point::new(x + i, 239))
                        .into_styled(PrimitiveStyle::with_stroke(Rgb565::GREEN, 1))
                        .draw(display)
                        .unwrap();
                }
                if i % 16 == 0 {
                    Line::new(Point::new(0, y + i), Point::new(239, y + i))
                        .into_styled(PrimitiveStyle::with_stroke(Rgb565::BLUE, 1))
                        .draw(display)
                        .unwrap();
                }
            }
            x = (x + 2) % 16;
            y = (y + 2) % 16;
        });
    }
}
