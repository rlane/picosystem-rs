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

    info!("Drawing");
    let circle1 =
        Circle::new(Point::new(128, 64), 64).into_styled(PrimitiveStyle::with_fill(Rgb565::RED));
    let circle2 = Circle::new(Point::new(64, 64), 64)
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::GREEN, 1));
    hw.display.draw_target().clear(Rgb565::BLACK).unwrap();
    circle1.draw(hw.display.draw_target()).unwrap();
    circle2.draw(hw.display.draw_target()).unwrap();
    hw.display.enable_backlight();

    info!("Finished initialization");
    let mut i = 0;
    loop {
        info!("Info log {}", i);
        log::error!("Error log");
        hw.blue_led_pin.set_high().unwrap();
        hw.delay.delay_ms(500);
        hw.blue_led_pin.set_low().unwrap();
        hw.delay.delay_ms(500);
        i += 1;
    }
}
