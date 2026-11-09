//! Asteroids rendered by Ponko. No textures or JavaScript game loop.
use ponko::*;
use ponko_arcade::{
    asteroids::{Asteroids, Input},
    runtime::*,
};
fn main() {
    Game::new("Asteroids / Ponko", 960, 600).run(|_| {
        let mut game = Asteroids::default();
        let mut accumulator = 0.;
        move |ctx: &mut Ctx| {
            fit(ctx);
            ctx.clear(Color::hex(0x10171e));
            let bits = touch();
            if ctx.key_pressed(Key::R) || bits & 32 != 0 {
                game.restart();
            }
            let turn = axis(
                ctx.key_down(Key::Right) || ctx.key_down(Key::D) || bits & 2 != 0,
                ctx.key_down(Key::Left) || ctx.key_down(Key::A) || bits & 1 != 0,
            );
            let thrust = ctx.key_down(Key::Up) || ctx.key_down(Key::W) || bits & 4 != 0;
            let shoot = ctx.key_down(Key::Space) || bits & 8 != 0;
            if !paused() {
                accumulator += ctx.dt().min(0.05);
                while accumulator >= 1. / 120. {
                    game.step(
                        1. / 120.,
                        Input {
                            turn,
                            thrust,
                            shoot,
                        },
                    );
                    accumulator -= 1. / 120.;
                }
            }
            stars(ctx);
            let rock_color = Color::hex(0xb8b1a2);
            for rock in &game.rocks {
                // Repeated at edges so the outline wraps continuously.
                for offset in [
                    Vec2::ZERO,
                    vec2(-960., 0.),
                    vec2(960., 0.),
                    vec2(0., -600.),
                    vec2(0., 600.),
                ] {
                    let center = rock.pos + offset;
                    for j in 0..10 {
                        let point = |i: u32| {
                            let a = i as f32 * std::f32::consts::TAU / 10.;
                            center
                                + Vec2::from_angle(a)
                                    * rock.radius
                                    * (0.82 + 0.18 * (i as f32 * 7.3 + rock.radius).sin())
                        };
                        ctx.draw_line(point(j), point((j + 1) % 10), 1.6, rock_color);
                    }
                }
            }
            for shot in &game.bullets {
                ctx.draw_line(
                    shot.pos - shot.vel.normalize_or_zero() * 8.,
                    shot.pos,
                    2.,
                    Color::hex(0xe9f8a5),
                );
            }
            for spark in &game.sparks {
                ctx.draw_circle(
                    spark.pos,
                    1.8,
                    Color::hex(0xe4a766).with_alpha(spark.life / 0.45),
                );
            }
            if game.lives > 0
                && (game.invulnerable == 0. || ((ctx.time() * 9.) as u32).is_multiple_of(2))
            {
                let forward = Vec2::from_angle(game.angle);
                let side = vec2(-forward.y, forward.x);
                let nose = game.pos + forward * 17.;
                let left = game.pos - forward * 12. + side * 11.;
                let right = game.pos - forward * 12. - side * 11.;
                for (a, b) in [
                    (nose, left),
                    (left, game.pos - forward * 6.),
                    (game.pos - forward * 6., right),
                    (right, nose),
                ] {
                    ctx.draw_line(a, b, 2., Color::hex(0xc8e6cf));
                }
                if thrust && !paused() {
                    let tail = game.pos - forward * (23. + (ctx.time() * 40.).sin() as f32 * 5.);
                    ctx.draw_line(left, tail, 1.5, Color::hex(0xe4a766));
                    ctx.draw_line(right, tail, 1.5, Color::hex(0xe4a766));
                }
            }
            hud(ctx, game.score, game.lives, game.wave);
            if game.lives == 0 {
                message(ctx, "RUN OVER", "Press R to try again");
            }
            report(game.score, game.lives, game.wave, game.rocks.len() as u32);
        }
    });
}
