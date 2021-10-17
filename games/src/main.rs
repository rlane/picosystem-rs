#![no_std]
#![no_main]

use cortex_m_rt::entry;
use log::info;
use picosystem::hardware;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

#[entry]
fn main() -> ! {
    let mut hw = hardware::Hardware::new();
    info!("Finished initialization");

    hw.display.clear(Rgb565::GREEN).unwrap();
    hw.display.flush();

    loop {}
}
