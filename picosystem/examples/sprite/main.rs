#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_graphics::image::Image;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use heapless::Vec;
use log::info;
use picosystem::display::{HEIGHT, WIDTH};
use picosystem::hardware;
use picosystem_macros::sprite;

sprite!(
    sprite_ship,
    "picosystem/examples/sprite/assets/playerShip2_red.png",
    56
);

sprite!(
    sprite_laser,
    "picosystem/examples/sprite/assets/laserGreen04.png",
    6
);

#[entry]
fn main() -> ! {
    let mut hw = hardware::Hardware::new();
    info!("Finished initialization");

    let background_color = Rgb565::CSS_DARK_SLATE_BLUE;

    let player_img = Image::new(sprite_ship(), Point::zero());
    let laser_img = Image::new(sprite_laser(), Point::zero());

    let mut p = Point::new(120, 120);
    let speed = 2;
    let mut lasers: Vec<Point, 32> = Vec::new();

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
        if hw.input.button_a.is_pressed() {
            let _ = lasers.push(p);
        }

        hw.draw(|display| {
            display.clear(background_color).unwrap();

            for l in lasers.iter_mut() {
                l.y -= speed * 2;
                laser_img
                    .translate(*l - Point::new(laser_img.bounding_box().size.width as i32 / 2, 0))
                    .draw(display)
                    .unwrap();
            }

            lasers = lasers
                .iter()
                .filter(|l| l.y > -(laser_img.bounding_box().size.height as i32))
                .cloned()
                .collect();

            player_img
                .translate(p - Point::new(player_img.bounding_box().size.width as i32 / 2, 0))
                .draw(display)
                .unwrap();
        });
    }
}
