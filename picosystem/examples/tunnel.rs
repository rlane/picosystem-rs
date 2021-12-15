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

#[entry]
fn main() -> ! {
    let mut hw = hardware::Hardware::new();
    info!("Finished initialization");

    let center = Point::new(119, 119);
    let mut sizes: Vec<f32, 32> = Vec::new();
    let multiplier = 1.02;
    let initial_size = 3.0;
    let mut fps_monitor = FpsMonitor::new();
    let interval = 25;
    let mut countdown = 0;

    loop {
        sizes = sizes
            .iter()
            .cloned()
            .map(|size| size * multiplier)
            .filter(|size| *size < WIDTH as f32)
            .collect();

        if countdown == 0 {
            let _ = sizes.push(initial_size);
            countdown = interval;
        } else {
            countdown -= 1;
        }

        hw.draw(|display| {
            display.clear(Rgb565::CSS_DARK_SLATE_BLUE).unwrap();
            for &size in sizes.iter() {
                let size = size as u32 | 1;
                Rectangle::with_center(center, Size::new(size as u32, size as u32))
                    .into_styled(PrimitiveStyle::with_stroke(Rgb565::GREEN, 1))
                    .draw(display)
                    .unwrap();
            }
        });

        fps_monitor.update();
    }
}
