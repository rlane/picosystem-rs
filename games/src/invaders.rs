use embedded_graphics::image::Image;
use embedded_graphics::mono_font::{ascii::FONT_10X20, MonoTextStyle};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::{Alignment, Text};
use heapless::Vec;
use picosystem::display::{HEIGHT, WIDTH};
use picosystem::hardware;
use picosystem::time;
use picosystem_macros::sprite;

sprite!(sprite_ship, "games/assets/playerShip2_red.png", 56);
sprite!(sprite_laser, "games/assets/laserGreen04.png", 6);
sprite!(sprite_enemy, "games/assets/enemyGreen1.png", 31);

#[derive(Debug, Clone)]
struct Entity {
    p: Point,
    size: Size,
    dead: bool,
}

impl Entity {
    fn top_left(&self) -> Point {
        self.p - self.size / 2
    }

    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(self.top_left(), self.size)
    }

    fn intersects_bb(&self, other: &Rectangle) -> bool {
        self.bounding_box().intersection(&other).size != Size::new(0, 0)
    }

    fn intersects(&self, other: &Entity) -> bool {
        self.intersects_bb(&other.bounding_box())
    }
}

pub fn main(hw: &mut hardware::Hardware) -> ! {
    let background_color = Rgb565::CSS_DARK_SLATE_BLUE;
    hw.display.clear(background_color).unwrap();
    hw.display.flush();

    let player_img = Image::new(sprite_ship(), Point::zero());
    let laser_img = Image::new(sprite_laser(), Point::zero());
    let enemy_img = Image::new(sprite_enemy(), Point::zero());

    let mut rng = oorandom::Rand32::new(time::time_us() as u64);
    let speed = 2;
    let mut player = Entity {
        p: Point::new(120, 120),
        size: player_img.bounding_box().size,
        dead: false,
    };
    let mut lasers: Vec<Entity, 32> = Vec::new();
    let mut enemies: Vec<Entity, 32> = Vec::new();
    let mut tick = 0;
    let mut score = 0;
    let mut enemy_countdown = 0;
    let mut sound = Sound::Silent;
    let screen_bounding_box =
        Rectangle::new(Point::new(0, 0), Size::new(WIDTH as u32, HEIGHT as u32));

    loop {
        if hw.input.dpad_left.is_held() && player.p.x > 0 {
            player.p.x -= speed;
        }
        if hw.input.dpad_right.is_held() && player.p.x < WIDTH as i32 - speed {
            player.p.x += speed;
        }
        if hw.input.dpad_up.is_held() && player.p.y > 0 {
            player.p.y -= speed;
        }
        if hw.input.dpad_down.is_held() && player.p.y < HEIGHT as i32 - speed {
            player.p.y += speed;
        }
        if hw.input.button_a.is_pressed() {
            let _ = lasers.push(Entity {
                p: player.p,
                size: laser_img.bounding_box().size,
                dead: false,
            });
            sound = Sound::LaserFired { start_tick: tick };
        }

        hw.display.clear(background_color).unwrap();

        let score_str: heapless::String<16> = heapless::String::from(score);
        let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::GREEN);
        Text::with_alignment(
            &score_str,
            Point::new(WIDTH as i32 / 2, 20),
            text_style,
            Alignment::Center,
        )
        .draw(&mut hw.display)
        .unwrap();

        if enemy_countdown > 0 {
            enemy_countdown -= 1;
        }
        if enemy_countdown == 0 {
            let margin = 30;
            let x = rng.rand_range(margin..(WIDTH as u32 - margin));
            let _ = enemies.push(Entity {
                p: Point::new(
                    x as i32,
                    -(enemy_img.bounding_box().size.height as i32) / 2 + 1,
                ),
                size: enemy_img.bounding_box().size,
                dead: false,
            });
            enemy_countdown = rng.rand_range(30..100);
        }

        for l in lasers.iter_mut() {
            l.p.y -= speed * 2;
            if !l.intersects_bb(&screen_bounding_box) {
                l.dead = true;
            }
            laser_img
                .translate(l.top_left())
                .draw(&mut hw.display)
                .unwrap();
        }

        for e in enemies.iter_mut() {
            if tick % 2 == 0 {
                e.p.y += 1;
                if !e.intersects_bb(&screen_bounding_box) {
                    e.dead = true;
                    if score > 0 {
                        score -= 1;
                    }
                    sound = Sound::EnemyEscaped { start_tick: tick };
                }
            }
            enemy_img
                .translate(e.top_left())
                .draw(&mut hw.display)
                .unwrap();
        }

        for l in lasers.iter_mut() {
            for e in enemies.iter_mut() {
                if e.intersects(l) {
                    e.dead = true;
                    l.dead = true;
                    score += 1;
                    sound = Sound::EnemyDestroyed { start_tick: tick };
                }
            }
        }

        lasers = lasers.iter().filter(|l| !l.dead).cloned().collect();
        enemies = enemies.iter().filter(|e| !e.dead).cloned().collect();

        player_img
            .translate(player.top_left())
            .draw(&mut hw.display)
            .unwrap();

        sound.play(tick, hw);

        hw.display.flush();

        tick += 1;
    }
}

enum Sound {
    Silent,
    LaserFired { start_tick: i32 },
    EnemyDestroyed { start_tick: i32 },
    EnemyEscaped { start_tick: i32 },
}

impl Sound {
    fn play(&self, tick: i32, hw: &mut hardware::Hardware) {
        let freq = match self {
            Sound::LaserFired { start_tick } => match tick - start_tick {
                0..=1 => 500,
                2 => 400,
                3 => 300,
                _ => 0,
            },
            Sound::EnemyDestroyed { start_tick } => match tick - start_tick {
                0..=3 => 880,
                _ => 0,
            },
            Sound::EnemyEscaped { start_tick } => match tick - start_tick {
                0 => 200,
                1 => 250,
                2 => 200,
                3 => 150,
                4 => 100,
                5 => 50,
                _ => 0,
            },
            _ => 0,
        };
        if freq > 0 {
            hw.audio.start_tone(freq);
        } else {
            hw.audio.stop();
        }
    }
}
