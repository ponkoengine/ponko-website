//! A 3D arena shooter. Simulation coordinates are the floor's X/Z plane.
use glam::Vec2;
use std::f32::consts::TAU;
#[derive(Default)]
pub struct Input {
    pub movement: Vec2,
    pub aim: Vec2,
    pub shoot: bool,
    pub dash: bool,
}
pub struct Drone {
    pub pos: Vec2,
    pub hp: u32,
}
pub struct Shot {
    pub pos: Vec2,
    pub velocity: Vec2,
    pub life: f32,
}
pub struct Debris {
    pub pos: Vec2,
    pub velocity: Vec2,
    pub life: f32,
}
pub const PILLARS: [Vec2; 8] = [
    Vec2::new(-9., -9.),
    Vec2::new(0., -12.),
    Vec2::new(9., -9.),
    Vec2::new(-12., 0.),
    Vec2::new(12., 0.),
    Vec2::new(-9., 9.),
    Vec2::new(0., 12.),
    Vec2::new(9., 9.),
];
pub struct Arena {
    pub pos: Vec2,
    pub aim: Vec2,
    pub drones: Vec<Drone>,
    pub shots: Vec<Shot>,
    pub debris: Vec<Debris>,
    pub health: u32,
    pub score: u32,
    pub wave: u32,
    pub dash_cooldown: f32,
    pub invulnerable: f32,
    cooldown: f32,
    dash_time: f32,
}
impl Default for Arena {
    fn default() -> Self {
        let mut g = Self {
            pos: Vec2::ZERO,
            aim: Vec2::NEG_Y,
            drones: Vec::with_capacity(220),
            shots: Vec::with_capacity(80),
            debris: Vec::with_capacity(512),
            health: 5,
            score: 0,
            wave: 1,
            dash_cooldown: 0.,
            invulnerable: 0.,
            cooldown: 0.,
            dash_time: 0.,
        };
        g.spawn_wave();
        g
    }
}
impl Arena {
    pub fn restart(&mut self) {
        *self = Self::default();
    }
    fn spawn_wave(&mut self) {
        let count = (48 + (self.wave - 1) * 16).min(200);
        for i in 0..count {
            let a = i as f32 * TAU / count as f32 + self.wave as f32 * 0.23;
            self.drones.push(Drone {
                pos: Vec2::from_angle(a) * (19.0 + (i % 3) as f32),
                hp: if self.wave >= 4 { 2 } else { 1 },
            });
        }
    }
    pub fn step(&mut self, dt: f32, input: Input) {
        if self.health == 0 {
            return;
        }
        let dt = dt.clamp(0., 0.05);
        self.cooldown = (self.cooldown - dt).max(0.);
        self.invulnerable = (self.invulnerable - dt).max(0.);
        self.dash_cooldown = (self.dash_cooldown - dt).max(0.);
        self.dash_time = (self.dash_time - dt).max(0.);
        if input.dash && self.dash_cooldown == 0. && input.movement.length_squared() > 0. {
            self.dash_time = 0.16;
            self.dash_cooldown = 1.4;
            self.invulnerable = self.invulnerable.max(0.22);
        }
        let speed = if self.dash_time > 0. { 28. } else { 8. };
        self.pos = move_body(
            self.pos,
            input.movement.normalize_or_zero() * speed * dt,
            0.65,
        );
        if input.aim.length_squared() > 0.01 {
            self.aim = input.aim.normalize();
        }
        if input.shoot && self.cooldown == 0. {
            self.shots.push(Shot {
                pos: self.pos + self.aim,
                velocity: self.aim * 45.,
                life: 1.2,
            });
            self.cooldown = 0.1;
        }
        for d in &mut self.drones {
            let dir = (self.pos - d.pos).normalize_or_zero();
            let side = Vec2::new(-dir.y, dir.x) * (d.pos.x * 0.7 + d.pos.y).sin() * 0.3;
            d.pos = move_body(
                d.pos,
                (dir + side) * (2.1 + self.wave as f32 * 0.16).min(4.5) * dt,
                0.55,
            );
        }
        for s in &mut self.shots {
            s.pos += s.velocity * dt;
            s.life -= dt;
            if s.pos.abs().max_element() > 23.
                || PILLARS
                    .iter()
                    .any(|p| (s.pos - *p).abs().max_element() < 1.55)
            {
                s.life = 0.;
            }
            if s.life > 0.
                && let Some(d) = self
                    .drones
                    .iter_mut()
                    .find(|d| d.hp > 0 && d.pos.distance_squared(s.pos) < 1.1)
            {
                d.hp -= 1;
                s.life = 0.;
            }
        }
        for i in (0..self.drones.len()).rev() {
            if self.drones[i].hp == 0 {
                let d = self.drones.swap_remove(i);
                self.score += 100;
                // A bounded pool prevents a large kill burst from growing forever.
                for j in 0..8 {
                    if self.debris.len() >= 512 {
                        break;
                    }
                    let a = j as f32 * TAU / 8.;
                    self.debris.push(Debris {
                        pos: d.pos,
                        velocity: Vec2::from_angle(a) * 6.,
                        life: 0.55,
                    });
                }
            }
        }
        self.shots.retain(|s| s.life > 0.);
        for d in &mut self.debris {
            d.pos += d.velocity * dt;
            d.life -= dt;
        }
        self.debris.retain(|d| d.life > 0.);
        if self.invulnerable == 0.
            && self
                .drones
                .iter()
                .any(|d| d.pos.distance_squared(self.pos) < 1.5)
        {
            self.health -= 1;
            self.invulnerable = 1.2;
        }
        if self.drones.is_empty() {
            self.wave += 1;
            self.spawn_wave();
            self.health = (self.health + 1).min(5);
            self.invulnerable = 1.5;
        }
    }
}
fn move_body(pos: Vec2, delta: Vec2, radius: f32) -> Vec2 {
    let mut next = (pos + delta).clamp(Vec2::splat(-21.), Vec2::splat(21.));
    for p in PILLARS {
        let half = 1.5 + radius;
        if (next - p).abs().max_element() < half {
            let x = Vec2::new(next.x, pos.y);
            let y = Vec2::new(pos.x, next.y);
            next = if (x - p).abs().max_element() >= half {
                x
            } else if (y - p).abs().max_element() >= half {
                y
            } else {
                pos
            };
        }
    }
    next
}
