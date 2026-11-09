use glam::Vec2;
use ponko_arcade::{
    asteroids::{Asteroids, Bullet, Input, Rock},
    breakout::Breakout,
};
#[test]
fn asteroids_starts_with_a_playable_wave() {
    let g = Asteroids::default();
    assert!(g.rocks.len() >= 4);
    assert_eq!(g.lives, 3);
}
#[test]
fn ship_wraps_across_the_playfield() {
    let mut g = Asteroids::default();
    g.pos = Vec2::new(959., 300.);
    g.vel = Vec2::new(200., 0.);
    g.step(0.02, Input::default());
    assert!(g.pos.x < 10.);
}
#[test]
fn fire_is_rate_limited_and_bullets_expire() {
    let mut g = Asteroids::default();
    g.step(
        0.01,
        Input {
            shoot: true,
            ..Default::default()
        },
    );
    assert_eq!(g.bullets.len(), 1);
    g.step(
        0.01,
        Input {
            shoot: true,
            ..Default::default()
        },
    );
    assert_eq!(g.bullets.len(), 1);
    for _ in 0..300 {
        g.step(0.01, Input::default());
    }
    assert!(g.bullets.is_empty());
}
#[test]
fn hitting_a_large_rock_scores_and_splits_it() {
    let mut g = Asteroids::default();
    g.rocks = vec![Rock {
        pos: Vec2::new(200., 200.),
        vel: Vec2::ZERO,
        radius: 40.,
    }];
    g.bullets = vec![Bullet {
        pos: Vec2::new(200., 200.),
        vel: Vec2::ZERO,
        life: 1.,
    }];
    g.step(0.01, Input::default());
    assert!(g.score > 0);
    assert_eq!(g.rocks.len(), 2);
    assert!(g.rocks.iter().all(|r| r.radius < 40.));
    assert!(g.bullets.is_empty());
}
#[test]
fn collision_costs_one_life_then_grants_protection() {
    let mut g = Asteroids::default();
    g.invulnerable = 0.;
    g.rocks = vec![Rock {
        pos: g.pos,
        vel: Vec2::ZERO,
        radius: 40.,
    }];
    g.step(0.01, Input::default());
    assert_eq!(g.lives, 2);
    g.step(0.01, Input::default());
    assert_eq!(g.lives, 2);
    assert!(g.invulnerable > 0.);
}
#[test]
fn game_over_freezes_until_restart() {
    let mut g = Asteroids::default();
    g.lives = 0;
    g.score = 500;
    let pos = g.pos;
    g.step(
        0.1,
        Input {
            thrust: true,
            shoot: true,
            turn: 1.,
        },
    );
    assert_eq!(g.pos, pos);
    assert!(g.bullets.is_empty());
    g.restart();
    assert_eq!(g.lives, 3);
    assert_eq!(g.score, 0);
    assert!(!g.rocks.is_empty());
}
#[test]
fn clearing_a_wave_starts_the_next() {
    let mut g = Asteroids::default();
    g.rocks.clear();
    g.step(0.01, Input::default());
    assert_eq!(g.wave, 2);
    assert!(!g.rocks.is_empty());
}
#[test]
fn breakout_waits_for_launch_and_clamps_the_paddle() {
    let mut g = Breakout::default();
    assert_eq!(g.bricks.len(), 50);
    for _ in 0..100 {
        g.step(0.05, -1., false);
    }
    assert!(g.paddle >= 56.);
    assert_eq!(g.ball.x, g.paddle);
    assert!(!g.launched);
    g.step(0.01, 0., true);
    assert!(g.launched);
    assert!(g.velocity.y < 0.);
}
#[test]
fn breakout_ball_bounces_off_the_wall() {
    let mut g = Breakout {
        launched: true,
        ball: Vec2::new(9., 300.),
        velocity: Vec2::new(-200., 100.),
        ..Default::default()
    };
    g.step(0.02, 0., false);
    assert!(g.velocity.x > 0.);
}
#[test]
fn breakout_destroys_a_brick_and_scores() {
    let mut g = Breakout::default();
    g.launched = true;
    g.ball = g.bricks[0] + Vec2::new(0., 18.);
    g.velocity = Vec2::new(0., -300.);
    g.step(0.01, 0., false);
    assert_eq!(g.bricks.len(), 49);
    assert_eq!(g.score, 10);
    assert!(g.velocity.y > 0.);
}
#[test]
fn breakout_miss_loses_a_life_and_resets_serve() {
    let mut g = Breakout {
        launched: true,
        ball: Vec2::new(20., 610.),
        ..Default::default()
    };
    g.step(0.01, 0., false);
    assert_eq!(g.lives, 2);
    assert!(!g.launched);
    assert_eq!(g.ball.x, g.paddle);
}
#[test]
fn breakout_restart_restores_bricks_and_score() {
    let mut g = Breakout::default();
    g.bricks.clear();
    g.score = 500;
    g.lives = 0;
    g.restart();
    assert_eq!(g.bricks.len(), 50);
    assert_eq!(g.score, 0);
    assert_eq!(g.lives, 3);
}
