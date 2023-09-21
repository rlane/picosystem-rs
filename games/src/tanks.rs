use log::info;
use micromath::{vector::Vector, F32Ext};
use picosystem::{
    display::{Display, HEIGHT, WIDTH},
    hardware, time,
};

use embedded_graphics::mono_font::{ascii::FONT_10X20, MonoTextStyle};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::Text;

pub fn main(hw: &mut hardware::Hardware) -> ! {
    loop {
        run_game(hw);
    }
}

struct Tank {
    position: Point,
    angle: f32,
    power: f32,
    index: i32,
}

fn run_game(hw: &mut hardware::Hardware) {
    let tank0_x = 30;
    let tank1_x = WIDTH as i32 - tank0_x;
    let mut terrain = generate_terrain();
    let mut tanks = [
        Tank {
            position: Point::new(tank0_x, terrain[tank0_x as usize] + TANK_HEIGHT / 2),
            angle: core::f32::consts::PI / 4.0,
            power: 50.0,
            index: 0,
        },
        Tank {
            position: Point::new(tank1_x, terrain[tank1_x as usize] + TANK_HEIGHT / 2),
            angle: core::f32::consts::PI / 4.0,
            power: 50.0,
            index: 1,
        },
    ];

    for tank in tanks.iter() {
        let height = terrain[(tank.position.x + 1) as usize];
        for i in -10..10 {
            terrain[(tank.position.x + i) as usize] = core::cmp::min(height, tank.position.y);
        }
    }

    let mut tank_index = 0;
    loop {
        {
            let (a, b) = tanks.split_at_mut(1);
            let (tank, other) = if tank_index == 0 {
                (&mut a[0], &b[0])
            } else {
                (&mut b[0], &a[0])
            };
            let angle_inc = 2.0 * core::f32::consts::PI / 360.0;
            if hw.input.dpad_left.is_held() && tank.power > 10.0 {
                tank.power -= 1.0;
            } else if hw.input.dpad_right.is_held() && tank.power < 99.0 {
                tank.power += 1.0;
            } else if hw.input.dpad_up.is_held() && tank.angle.to_degrees() < 90.0 {
                tank.angle += angle_inc;
            } else if hw.input.dpad_down.is_held() && tank.angle.to_degrees() > 0.0 {
                tank.angle -= angle_inc;
            } else if hw.input.button_a.is_pressed() {
                info!("Firing angle={} power={}", tank.angle, tank.power);
                if fire_shot(hw, &mut terrain, tank, other) {
                    info!("Shot hit");
                    break;
                }
                tank_index = (tank_index + 1) % 2;
            }
        }

        hw.draw(|display| {
            display.clear(Rgb565::CYAN).unwrap();
            draw_terrain(display, &terrain);
            for tank in tanks.iter() {
                draw_tank(display, tank, tank.index == tank_index);
            }
            display.flush();
        });
    }
}

type Terrain = [i32; WIDTH];

fn generate_terrain() -> Terrain {
    let mut rng = oorandom::Rand32::new(time::time_us() as u64);
    let mut terrain = [0; WIDTH];
    let w = WIDTH as f32;
    let off0 = rng.rand_float() * 100.0;
    let off1 = rng.rand_float() * 100.0;
    let freq0 = rng.rand_float() * 100.0;
    let freq1 = rng.rand_float() * 100.0;
    let scale0 = rng.rand_float();
    let scale1 = rng.rand_float();
    for (i, v) in terrain.iter_mut().enumerate() {
        let c0 = (i as f32 + off0) / freq0;
        let c1 = (i as f32 + off1) / freq1;
        *v = ((c0.sin() * scale0 + c1.sin() * scale1) * w / 8.0 + w / 4.0) as i32;
    }
    terrain
}

fn draw_terrain(display: &mut Display, terrain: &Terrain) {
    let style = PrimitiveStyle::with_stroke(Rgb565::GREEN, 1);
    for (x, &height) in terrain.iter().enumerate() {
        Line::new(
            Point::new(x as i32, HEIGHT as i32 - 1),
            Point::new(x as i32, HEIGHT as i32 - height),
        )
        .into_styled(style)
        .draw(display)
        .unwrap();
    }
}

const TANK_WIDTH: i32 = 16;
const TANK_HEIGHT: i32 = 8;
const GUN_LENGTH: f32 = 15.0;

fn draw_tank(display: &mut Display, tank: &Tank, selected: bool) {
    let screen_position = Point::new(tank.position.x, HEIGHT as i32 - tank.position.y);
    let style = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb565::BLACK)
        .stroke_width(1)
        .fill_color(Rgb565::RED)
        .build();
    Rectangle::new(
        screen_position - Point::new(TANK_WIDTH / 2, TANK_HEIGHT / 2),
        Size::new(TANK_WIDTH as u32, TANK_HEIGHT as u32),
    )
    .into_styled(style)
    .draw(display)
    .unwrap();

    let angle = if tank.index == 0 {
        tank.angle
    } else {
        tank.angle + 90.0f32.to_radians()
    };
    Line::new(
        screen_position,
        screen_position
            + Point::new(
                (angle.cos() * GUN_LENGTH) as i32,
                -(angle.sin() * GUN_LENGTH) as i32,
            ),
    )
    .into_styled(PrimitiveStyle::with_stroke(Rgb565::RED, 4))
    .draw(display)
    .unwrap();

    let angle_str: heapless::String<16> = heapless::String::from(tank.angle.to_degrees() as i32);
    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::BLUE);
    Text::new(&angle_str, Point::new(tank.position.x, 20), text_style)
        .draw(display)
        .unwrap();

    let power_str: heapless::String<16> = heapless::String::from(tank.power as i32);
    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::BLUE);
    Text::new(&power_str, Point::new(tank.position.x, 40), text_style)
        .draw(display)
        .unwrap();

    let style = PrimitiveStyleBuilder::new()
        .stroke_color(if selected { Rgb565::BLUE } else { Rgb565::CYAN })
        .stroke_width(1)
        .build();
    Rectangle::new(Point::new(tank.position.x - 2, 3), Size::new(24, 44))
        .into_styled(style)
        .draw(display)
        .unwrap();
}

fn draw_explosion(hw: &mut hardware::Hardware, p: Point, r: i32) {
    let p = Point::new(p.x, HEIGHT as i32 - p.y);
    let tmax: i32 = r * 2;
    for t in 0..tmax {
        hw.draw(|display| {
            let style = PrimitiveStyleBuilder::new()
                .stroke_color(Rgb565::BLACK)
                .stroke_width(1)
                .fill_color(Rgb565::RED)
                .build();
            let r = tmax / 2 - (t - tmax / 2).abs();
            Circle::with_center(p, r as u32 * 2)
                .into_styled(style)
                .draw(display)
                .unwrap();
        });
    }
}

fn fire_shot(
    hw: &mut hardware::Hardware,
    terrain: &mut Terrain,
    tank: &Tank,
    other: &Tank,
) -> bool {
    use micromath::vector::F32x2;
    let mut p = F32x2 {
        x: tank.position.x as f32,
        y: tank.position.y as f32,
    };
    let angle = if tank.index == 0 {
        tank.angle
    } else {
        tank.angle + 90.0f32.to_radians()
    };
    let dir = F32x2 {
        x: angle.cos(),
        y: angle.sin(),
    };
    p += dir * GUN_LENGTH;
    let mut v = dir * (tank.power / 100.0) * 1.5;
    loop {
        p += v;
        v.y -= (9.8 / 60.0) / 30.0;
        let style = PrimitiveStyleBuilder::new()
            .fill_color(Rgb565::new(100, 80, 100))
            .build();
        hw.draw(|display| {
            Circle::with_center(Point::new(p.x as i32, HEIGHT as i32 - p.y as i32), 3)
                .into_styled(style)
                .draw(display)
                .unwrap();
        });
        if p.x < 0.0 || p.x >= WIDTH as f32 {
            break;
        }
        if p.y as i32 <= terrain[p.x as usize] {
            let r = 15;
            draw_explosion(hw, Point::new(p.x as i32, p.y as i32), r);

            let impact_position = Point::new(p.x as i32 - r, HEIGHT as i32 - p.y as i32 - r);
            let arc = Arc::new(impact_position, r as u32 * 2, 0.0.deg(), 360.0.deg());
            for point in arc.points() {
                if point.x < 0 || point.x >= terrain.len() as i32 {
                    continue;
                }
                terrain[point.x as usize] =
                    core::cmp::min(terrain[point.x as usize], HEIGHT as i32 - point.y);
            }

            let other_p = F32x2 {
                x: other.position.x as f32,
                y: other.position.y as f32,
            };
            if (other_p - p).magnitude() < r as f32 {
                draw_explosion(hw, other.position, r * 2);
                return true;
            } else {
                return false;
            }
        }
    }

    false
}
