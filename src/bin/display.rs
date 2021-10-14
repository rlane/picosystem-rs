#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_hal::digital::v2::OutputPin;
use log::info;
use rlane_picosystem_games as rpsg;
use rpsg::hardware;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;

#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

#[entry]
fn main() -> ! {
    let mut hw = hardware::Hardware::new();

    info!("Finished initialization");
    let mut i = 0;
    loop {
        hw.display.clear(Rgb565::BLACK).unwrap();
        for i in 0..240 {
            if i % 16 == 0 {
                let line = Line::new(Point::new(i, 0), Point::new(i, 239))
                    .into_styled(PrimitiveStyle::with_stroke(Rgb565::GREEN, 1));
                line.draw(&mut hw.display).unwrap();
            }
            if i % 16 == 0 {
                let line = Line::new(Point::new(0, i), Point::new(239, i))
                    .into_styled(PrimitiveStyle::with_stroke(Rgb565::BLUE, 1));
                line.draw(&mut hw.display).unwrap();
            }
        }
        let circle1 = Circle::new(Point::new((i + 64) % 240, 64), 64)
            .into_styled(PrimitiveStyle::with_fill(Rgb565::RED));
        let circle2 = Circle::new(Point::new(i % 240, 64), 64)
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::GREEN, 1));
        circle1.draw(&mut hw.display).unwrap();
        circle2.draw(&mut hw.display).unwrap();
        hw.display.flush();

        if i % 2 == 0 {
            hw.blue_led_pin.set_high().unwrap();
        } else {
            hw.blue_led_pin.set_low().unwrap();
        }
        if i % 30 == 0 {
            info!("Frame {}", i);
        }
        i += 1;
    }
}
