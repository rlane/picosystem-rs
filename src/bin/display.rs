#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_hal::digital::v2::OutputPin;
use log::info;
use rlane_picosystem_games as rpsg;
use rpsg::{hardware, time};

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
    let mut frame = 0;
    let mut prev_time_us = time::time_us();
    let mut prev_frame = 0;
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
        let circle1 = Circle::new(Point::new((frame + 64) % 240, 64), 64)
            .into_styled(PrimitiveStyle::with_fill(Rgb565::RED));
        let circle2 = Circle::new(Point::new(frame % 240, 64), 64)
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::GREEN, 1));
        circle1.draw(&mut hw.display).unwrap();
        circle2.draw(&mut hw.display).unwrap();
        hw.display.flush();

        if frame % 2 == 0 {
            hw.blue_led_pin.set_high().unwrap();
        } else {
            hw.blue_led_pin.set_low().unwrap();
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
