//! An asset-free Asteroids prototype. All simulation runs in Rust.
use glam::Vec2;
use std::f32::consts::{FRAC_PI_2, TAU};

pub const SIZE: Vec2 = Vec2::new(960.0, 600.0);
#[derive(Default)]
pub struct Input {
    pub turn: f32,
    pub thrust: bool,
    pub shoot: bool,
}
pub struct Rock {
    pub pos: Vec2,
    pub vel: Vec2,
    pub radius: f32,
}
pub struct Bullet {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
}
pub struct Spark {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
}
pub struct Asteroids {
    pub pos: Vec2,
    pub vel: Vec2,
    pub angle: f32,
    pub rocks: Vec<Rock>,
    pub bullets: Vec<Bullet>,
    pub sparks: Vec<Spark>,
    pub score: u32,
    pub lives: u32,
    pub wave: u32,
    pub invulnerable: f32,
    cooldown: f32,
    seed: u32,
}
impl Default for Asteroids {
    fn default() -> Self {
        let mut g = Self {
            pos: SIZE / 2.0,
            vel: Vec2::ZERO,
            angle: -FRAC_PI_2,
            rocks: Vec::with_capacity(64),
            bullets: Vec::with_capacity(32),
            sparks: Vec::with_capacity(256),
            score: 0,
            lives: 3,
            wave: 1,
            invulnerable: 2.0,
            cooldown: 0.0,
            seed: 1701,
        };
        g.spawn_wave();
        g
    }
}
impl Asteroids {
    pub fn restart(&mut self) {
        *self = Self::default();
    }
    fn random(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed as f32 / u32::MAX as f32
    }
    fn spawn_wave(&mut self) {
        for i in 0..(3 + self.wave).min(14) {
            let angle = i as f32 * 2.39996 + self.random();
            let pos = wrap(SIZE / 2.0 + Vec2::from_angle(angle) * 380.0);
            let velocity = Vec2::from_angle(self.random() * TAU)
                * (36.0 + self.random() * 32.0 + self.wave as f32 * 5.0);
            self.rocks.push(Rock {
                pos,
                vel: velocity,
                radius: 42.0,
            });
        }
    }
    pub fn step(&mut self, dt: f32, input: Input) {
        if self.lives == 0 {
            return;
        }
        let dt = dt.clamp(0.0, 0.05);
        self.cooldown = (self.cooldown - dt).max(0.0);
        self.invulnerable = (self.invulnerable - dt).max(0.0);
        self.angle += input.turn * 4.2 * dt;
        let forward = Vec2::from_angle(self.angle);
        if input.thrust {
            self.vel += forward * 260.0 * dt;
        }
        self.vel *= (-0.35 * dt).exp();
        self.vel = self.vel.clamp_length_max(340.0);
        self.pos = wrap(self.pos + self.vel * dt);
        if input.shoot && self.cooldown == 0.0 {
            self.bullets.push(Bullet {
                pos: wrap(self.pos + forward * 19.0),
                vel: forward * 570.0 + self.vel,
                life: 1.15,
            });
            self.cooldown = 0.17;
        }
        for b in &mut self.bullets {
            b.pos = wrap(b.pos + b.vel * dt);
            b.life -= dt;
        }
        for r in &mut self.rocks {
            r.pos = wrap(r.pos + r.vel * dt);
        }
        for s in &mut self.sparks {
            s.pos += s.vel * dt;
            s.life -= dt;
        }
        self.sparks.retain(|s| s.life > 0.0);
        for i in (0..self.bullets.len()).rev() {
            if self.bullets[i].life <= 0.0 {
                continue;
            }
            if let Some(j) = self
                .rocks
                .iter()
                .position(|r| distance(r.pos, self.bullets[i].pos) < r.radius + 3.0)
            {
                self.bullets.swap_remove(i);
                let rock = self.rocks.swap_remove(j);
                self.score += if rock.radius > 30.0 {
                    20
                } else if rock.radius > 18.0 {
                    50
                } else {
                    100
                };
                for k in 0..12 {
                    let a = k as f32 * TAU / 12.0;
                    let speed = 50.0 + self.random() * 100.0;
                    self.sparks.push(Spark {
                        pos: rock.pos,
                        vel: Vec2::from_angle(a) * speed,
                        life: 0.45,
                    });
                }
                if rock.radius > 18.0 {
                    for side in [-1.0, 1.0] {
                        let vel =
                            rock.vel + Vec2::from_angle(rock.vel.to_angle() + side * 1.0) * 58.0;
                        self.rocks.push(Rock {
                            pos: rock.pos,
                            vel,
                            radius: rock.radius * 0.57,
                        });
                    }
                }
            }
        }
        self.bullets.retain(|b| b.life > 0.0);
        if self.invulnerable == 0.0
            && self
                .rocks
                .iter()
                .any(|r| distance(r.pos, self.pos) < r.radius + 9.0)
        {
            self.lives -= 1;
            self.pos = SIZE / 2.0;
            self.vel = Vec2::ZERO;
            self.invulnerable = 2.5;
        }
        if self.rocks.is_empty() {
            self.wave += 1;
            self.spawn_wave();
            self.invulnerable = self.invulnerable.max(1.5);
        }
    }
}
fn wrap(p: Vec2) -> Vec2 {
    Vec2::new(p.x.rem_euclid(SIZE.x), p.y.rem_euclid(SIZE.y))
}
fn distance(a: Vec2, b: Vec2) -> f32 {
    let d = (a - b).abs();
    Vec2::new(d.x.min(SIZE.x - d.x), d.y.min(SIZE.y - d.y)).length()
}
