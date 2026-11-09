use glam::Vec2;
use ponko_arcade::arena::{Arena, Drone, Input};
#[test]
fn arena_starts_with_a_wave_of_chasing_enemies() {
    let mut g = Arena::default();
    assert!(g.drones.len() >= 24);
    let before = g.drones[0].pos.length();
    g.step(0.05, Input::default());
    assert!(g.drones[0].pos.length() < before);
}
#[test]
fn diagonal_movement_has_the_same_speed_and_stays_in_arena() {
    let mut a = Arena::default();
    let mut b = Arena::default();
    a.step(
        0.05,
        Input {
            movement: Vec2::X,
            ..Default::default()
        },
    );
    b.step(
        0.05,
        Input {
            movement: Vec2::ONE,
            ..Default::default()
        },
    );
    assert!((a.pos.length() - b.pos.length()).abs() < 0.001);
    for _ in 0..500 {
        a.step(
            0.05,
            Input {
                movement: Vec2::X,
                ..Default::default()
            },
        );
    }
    assert!(a.pos.x <= 21.);
}
#[test]
fn arena_shots_kill_drones_and_score() {
    let mut g = Arena::default();
    g.drones = vec![Drone {
        pos: Vec2::new(0., -3.),
        hp: 1,
    }];
    for _ in 0..20 {
        g.step(
            1. / 120.,
            Input {
                shoot: true,
                aim: Vec2::NEG_Y,
                ..Default::default()
            },
        );
    }
    assert!(g.score > 0);
}
#[test]
fn contact_damage_has_a_cooldown_and_restart_restores_health() {
    let mut g = Arena::default();
    g.drones = vec![Drone {
        pos: Vec2::ZERO,
        hp: 2,
    }];
    g.step(0.01, Input::default());
    assert_eq!(g.health, 4);
    g.step(0.01, Input::default());
    assert_eq!(g.health, 4);
    g.health = 0;
    g.score = 400;
    g.restart();
    assert_eq!(g.health, 5);
    assert_eq!(g.score, 0);
}
#[test]
fn arena_clear_advances_to_a_larger_wave() {
    let mut g = Arena::default();
    let initial = g.drones.len();
    g.drones.clear();
    g.step(0.01, Input::default());
    assert_eq!(g.wave, 2);
    assert!(g.drones.len() > initial);
}
#[test]
fn dash_has_a_cooldown() {
    let mut g = Arena::default();
    g.step(
        0.01,
        Input {
            movement: Vec2::X,
            dash: true,
            ..Default::default()
        },
    );
    assert!(g.dash_cooldown > 0.);
    let cooldown = g.dash_cooldown;
    g.step(
        0.01,
        Input {
            movement: Vec2::X,
            dash: true,
            ..Default::default()
        },
    );
    assert!(g.dash_cooldown < cooldown);
}
