#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_graphics::image::Image;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use log::info;
use picosystem::display::{HEIGHT, WIDTH};
use picosystem::hardware;
use picosystem_macros::sprite;

#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

sprite!(
    sprite_ship,
    "picosystem/examples/sprite/assets/playerShip2_red.png"
);

#[entry]
fn main() -> ! {
    let mut hw = hardware::Hardware::new();
    info!("Finished initialization");

    hw.display.clear(Rgb565::CYAN).unwrap();
    hw.display.flush();

    let player_img = Image::new(sprite_ship(), Point::zero());

    let mut p = Point::new(120, 120);
    let speed = 2;

    loop {
        if hw.input.dpad_left.is_held() && p.x > 0 {
            p.x -= speed;
        }
        if hw.input.dpad_right.is_held() && p.x < WIDTH as i32 - speed {
            p.x += speed;
        }
        if hw.input.dpad_up.is_held() && p.y > 0 {
            p.y -= speed;
        }
        if hw.input.dpad_down.is_held() && p.y < HEIGHT as i32 - speed {
            p.y += speed;
        }

        hw.display.clear(Rgb565::CYAN).unwrap();
        player_img.translate(p).draw(&mut hw.display).unwrap();
        hw.display.flush();
    }
}
