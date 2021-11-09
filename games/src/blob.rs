use core::fmt::Write;
use embedded_graphics::mono_font::{ascii::FONT_10X20, MonoTextStyle};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::{Alignment, Text};
use heapless::Vec;
use micromath::vector::I32x2;
use micromath::F32Ext;
use picosystem::display::{HEIGHT, WIDTH};
use picosystem::fps_monitor::FpsMonitor;
use picosystem::hardware;
use picosystem::time;

const FRAC: i32 = 128;

fn world2screen(p: I32x2) -> Point {
    let x = p.x / FRAC;
    let y = p.y / FRAC;
    Point::new(x, y)
}

fn screen2world(p: Point) -> I32x2 {
    let x = p.x * FRAC;
    let y = p.y * FRAC;
    I32x2 { x, y }
}

fn mass2radius(mass: f32) -> i32 {
    mass.sqrt() as i32 * FRAC
}

#[derive(Debug, Clone)]
struct Blob {
    p: I32x2,
    v: I32x2,
    r: i32,
    mass: f32,
    dead: bool,
}

impl Blob {
    fn intersects(&self, other: &Blob) -> bool {
        let dp = self.p - other.p;
        let r_sum = self.r + other.r;
        if dp.x.abs() > r_sum || dp.y.abs() > r_sum {
            return false;
        }
        let dist_squared = dp.x * dp.x + dp.y * dp.y;
        dist_squared < r_sum * r_sum
    }
}

pub fn main(hw: &mut hardware::Hardware) -> ! {
    let mut rng = oorandom::Rand32::new(time::time_us() as u64);
    let mut fps_monitor = FpsMonitor::new();
    let world_size = screen2world(Point::new(WIDTH as i32, HEIGHT as i32));
    let mut drag_a = 500.0;
    let mut drag_b = 100e3;
    let mut tick = 0;
    const TWEAK_DRAG: bool = false;
    let starting_mass = 100.0;
    let mut level = 0;

    loop {
        level += 1;

        let mut player = Blob {
            p: I32x2 {
                x: world_size.x / 2,
                y: world_size.y / 2,
            },
            v: I32x2 { x: 0, y: 0 },
            r: mass2radius(starting_mass),
            mass: starting_mass,
            dead: false,
        };

        let mut make_enemy = || {
            let s = FRAC;
            let mass = rng.rand_float() * 20.0 + 8.0;
            Blob {
                p: I32x2 {
                    x: rng.rand_range(0..world_size.x as u32) as i32,
                    y: rng.rand_range(0..world_size.y as u32) as i32,
                },
                v: I32x2 {
                    x: rng.rand_range(0..(s as u32 * 2)) as i32 - s,
                    y: rng.rand_range(0..(s as u32 * 2)) as i32 - s,
                },
                r: mass2radius(mass),
                mass,
                dead: false,
            }
        };

        let mut blobs: Vec<Blob, 128> = Vec::new();
        for _ in 0..level {
            let enemy = make_enemy();
            if !player.intersects(&enemy) {
                let _ = blobs.push(enemy);
            }
        }

        loop {
            let mut acc = I32x2 { x: 0, y: 0 };
            let a = (20.0 * FRAC as f32 / player.mass).max(1.0) as i32;
            if hw.input.dpad_left.is_held() {
                acc.x -= a;
            }
            if hw.input.dpad_right.is_held() {
                acc.x += a;
            }
            if hw.input.dpad_up.is_held() {
                acc.y -= a;
            }
            if hw.input.dpad_down.is_held() {
                acc.y += a;
            }

            if TWEAK_DRAG {
                if hw.input.button_a.is_held() {
                    drag_a *= 1.01;
                }
                if hw.input.button_b.is_held() {
                    drag_a *= 0.99;
                }
                if hw.input.button_x.is_held() {
                    drag_b *= 1.01;
                }
                if hw.input.button_y.is_held() {
                    drag_b *= 0.99;
                }
                if tick % 16 == 0 {
                    log::info!("drag_a: {}, drag_b: {}", drag_a, drag_b);
                }
            }

            player.v += acc;
            let dampen = |i: i32| {
                let j = i as f32;
                let k = j / drag_a + j.abs() * j / drag_b;
                let q = (j - k * starting_mass / player.mass) as i32;
                if q == 0 {
                    i
                } else {
                    q
                }
            };
            player.v = I32x2 {
                x: dampen(player.v.x),
                y: dampen(player.v.y),
            };

            let do_physics = |blob: &mut Blob| {
                blob.p += blob.v;
                if blob.p.x < blob.r || blob.p.x >= world_size.x - blob.r {
                    blob.v.x = -blob.v.x;
                }
                blob.p.x = blob.p.x.clamp(blob.r, world_size.x - blob.r);
                if blob.p.y < blob.r || blob.p.y >= world_size.y - blob.r {
                    blob.v.y = -blob.v.y;
                }
                blob.p.y = blob.p.y.clamp(blob.r, world_size.y - blob.r);
            };

            do_physics(&mut player);
            hw.audio.stop();
            for blob in blobs.iter_mut() {
                if blob.dead {
                    continue;
                }
                if player.intersects(blob) {
                    blob.dead = true;
                    player.mass += blob.mass;
                    player.r = mass2radius(player.mass).min(world_size.x - 20);
                    hw.audio.start_tone(440 * 3);
                } else {
                    do_physics(blob);
                }
            }

            blobs = blobs.into_iter().filter(|blob| !blob.dead).collect();
            if blobs.is_empty() {
                animate_win(hw, level + 1);
                break;
            }

            hw.display.draw(|display| {
                display.clear(Rgb565::CSS_DARK_SLATE_BLUE).unwrap();

                Circle::with_center(world2screen(player.p), 2 * (player.r / FRAC) as u32)
                    .into_styled(
                        PrimitiveStyleBuilder::new()
                            .stroke_color(Rgb565::CSS_DARK_SEA_GREEN)
                            .stroke_width(1)
                            .fill_color(Rgb565::GREEN)
                            .build(),
                    )
                    .draw(display)
                    .unwrap();

                Circle::with_center(world2screen(player.p), 2 * (player.r / FRAC) as u32)
                    .into_styled(
                        PrimitiveStyleBuilder::new()
                            .stroke_color(Rgb565::CSS_DARK_SEA_GREEN)
                            .stroke_width(1)
                            .fill_color(Rgb565::CSS_LAWN_GREEN)
                            .build(),
                    )
                    .draw(display)
                    .unwrap();

                for eye in &[-1, 1] {
                    let diameter = player.r / (2 * FRAC);
                    let eye_pos = world2screen(
                        player.p
                            + player.v
                            + I32x2 {
                                x: eye * player.r / 3,
                                y: -player.r / 4,
                            },
                    );
                    Circle::with_center(eye_pos, diameter as u32)
                        .into_styled(
                            PrimitiveStyleBuilder::new()
                                .stroke_color(Rgb565::CSS_DARK_SEA_GREEN)
                                .stroke_width(1)
                                .fill_color(Rgb565::WHITE)
                                .build(),
                        )
                        .draw(display)
                        .unwrap();
                }

                for blob in blobs.iter() {
                    Circle::with_center(world2screen(blob.p), 2 * (blob.r / FRAC) as u32)
                        .into_styled(
                            PrimitiveStyleBuilder::new()
                                .stroke_color(Rgb565::CSS_DARK_RED)
                                .stroke_width(1)
                                .fill_color(Rgb565::CSS_CRIMSON)
                                .build(),
                        )
                        .draw(display)
                        .unwrap();
                }
            });

            fps_monitor.update();

            tick += 1;
        }
    }
}

fn animate_win(hw: &mut hardware::Hardware, next_level: usize) {
    hw.display.draw(|display| {
        Rectangle::new(Point::new(40, 100), Size::new(160, 40))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_GREEN))
            .draw(display)
            .unwrap();
        let mut txt: heapless::String<16> = heapless::String::new();
        write!(txt, "Level {}", next_level).unwrap();
        Text::with_alignment(
            &txt,
            Point::new(120, 127),
            MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE),
            Alignment::Center,
        )
        .draw(display)
        .unwrap();
    });

    hw.audio.start_tone(440);
    hw.delay.delay_ms(100);
    hw.audio.start_tone(880);
    hw.delay.delay_ms(100);
    hw.audio.stop();

    hw.delay.delay_ms(2000);

    hw.display.draw(|display| {
        display.clear(Rgb565::CSS_DARK_SLATE_BLUE).unwrap();
    });
}
