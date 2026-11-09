//! A complete single-screen brick breaker with paddle-angle rebounds.
use glam::Vec2;
pub struct Breakout {
    pub ball: Vec2,
    pub velocity: Vec2,
    pub paddle: f32,
    pub bricks: Vec<Vec2>,
    pub score: u32,
    pub lives: u32,
    pub launched: bool,
}
impl Default for Breakout {
    fn default() -> Self {
        let bricks = (0..5)
            .flat_map(|row| {
                (0..10)
                    .map(move |col| Vec2::new(111.0 + col as f32 * 82.0, 90.0 + row as f32 * 28.0))
            })
            .collect();
        Self {
            ball: Vec2::new(480.0, 536.0),
            velocity: Vec2::ZERO,
            paddle: 480.0,
            bricks,
            score: 0,
            lives: 3,
            launched: false,
        }
    }
}
impl Breakout {
    pub fn restart(&mut self) {
        *self = Self::default();
    }
    pub fn step(&mut self, dt: f32, direction: f32, launch: bool) {
        if self.lives == 0 || self.bricks.is_empty() {
            return;
        }
        let dt = dt.clamp(0.0, 0.05);
        self.paddle = (self.paddle + direction * 520.0 * dt).clamp(56.0, 904.0);
        if !self.launched {
            self.ball = Vec2::new(self.paddle, 536.0);
            if launch {
                self.launched = true;
                self.velocity = Vec2::new(140.0, -340.0);
            }
            return;
        }
        let previous = self.ball;
        self.ball += self.velocity * dt;
        if self.ball.x < 8.0 {
            self.ball.x = 8.0;
            self.velocity.x = self.velocity.x.abs();
        }
        if self.ball.x > 952.0 {
            self.ball.x = 952.0;
            self.velocity.x = -self.velocity.x.abs();
        }
        if self.ball.y < 40.0 {
            self.ball.y = 40.0;
            self.velocity.y = self.velocity.y.abs();
        }
        if self.velocity.y > 0.0
            && previous.y <= 537.0
            && self.ball.y >= 537.0
            && (self.ball.x - self.paddle).abs() < 64.0
        {
            self.ball.y = 537.0;
            let ratio = ((self.ball.x - self.paddle) / 56.0).clamp(-1.0, 1.0);
            let speed = (self.velocity.length() + 8.0).min(620.0);
            self.velocity = Vec2::new(ratio * 1.1, -1.0).normalize() * speed;
        }
        if let Some(i) = self.bricks.iter().position(|b| {
            let closest = self
                .ball
                .clamp(*b - Vec2::new(37.0, 10.0), *b + Vec2::new(37.0, 10.0));
            self.ball.distance_squared(closest) < 64.0
        }) {
            let brick = self.bricks.swap_remove(i);
            if (previous.y - brick.y).abs() >= 18.0 {
                self.velocity.y = -self.velocity.y;
            } else {
                self.velocity.x = -self.velocity.x;
            }
            self.ball = previous;
            self.score += 10;
        }
        if self.ball.y >= 610.0 {
            self.lives -= 1;
            self.launched = false;
            self.ball = Vec2::new(self.paddle, 536.0);
            self.velocity = Vec2::ZERO;
        }
    }
}
