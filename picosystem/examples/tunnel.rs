#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use heapless::Vec;
use log::info;
use picosystem::display::WIDTH;
use picosystem::fps_monitor::FpsMonitor;
use picosystem::hardware;

#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

#[entry]
fn main() -> ! {
    let mut hw = hardware::Hardware::new();
    info!("Finished initialization");

    let center = Point::new(119, 119);
    let mut sizes: Vec<f32, 32> = Vec::new();
    let multiplier = 1.01;
    let initial_size = 1.0;
    let mut fps_monitor = FpsMonitor::new();
    let interval = 60;
    let mut countdown = 0;

    loop {
        for size in sizes.iter_mut() {
            *size *= multiplier;
        }

        sizes = sizes
            .iter()
            .cloned()
            .filter(|size| *size < WIDTH as f32)
            .collect();

        if countdown == 0 {
            let _ = sizes.push(initial_size);
            countdown = interval;
        } else {
            countdown -= 1;
        }

        hw.display.draw(|display| {
            display.clear(Rgb565::CSS_DARK_SLATE_BLUE).unwrap();
            for &size in sizes.iter() {
                Rectangle::with_center(center, Size::new(size as u32, size as u32))
                    .into_styled(PrimitiveStyle::with_stroke(Rgb565::GREEN, 1))
                    .draw(display)
                    .unwrap();
            }
        });

        fps_monitor.update();
    }
}
