//! Breakout rendered by Ponko's 2D shape API.
use ponko::*;
use ponko_arcade::{breakout::Breakout, runtime::*};
fn main() {
    Game::new("Breakout / Ponko", 960, 600).run(|_| {
        let mut game = Breakout::default();
        let mut accumulator = 0.;
        move |ctx: &mut Ctx| {
            fit(ctx);
            ctx.clear(Color::hex(0x10171e));
            let bits = touch();
            if ctx.key_pressed(Key::R) || bits & 32 != 0 {
                game.restart();
            }
            let direction = axis(
                ctx.key_down(Key::Right) || ctx.key_down(Key::D) || bits & 2 != 0,
                ctx.key_down(Key::Left) || ctx.key_down(Key::A) || bits & 1 != 0,
            );
            let launch = ctx.key_down(Key::Space) || bits & 8 != 0;
            if !paused() {
                accumulator += ctx.dt().min(0.05);
                while accumulator >= 1. / 120. {
                    game.step(1. / 120., direction, launch);
                    accumulator -= 1. / 120.;
                }
            }
            for brick in &game.bricks {
                let row = ((brick.y - 90.) / 28.) as usize;
                let color =
                    Color::hex([0xe7a889, 0xd8b66e, 0xbbc97e, 0x88b7a8, 0x819eaf][row.min(4)]);
                ctx.draw_rect(Rect::new(brick.x - 37., brick.y - 10., 74., 20.), color);
                ctx.draw_rect(
                    Rect::new(brick.x - 37., brick.y - 10., 74., 3.),
                    Color::WHITE.with_alpha(0.14),
                );
            }
            ctx.draw_line(vec2(8., 40.), vec2(8., 590.), 1., Color::hex(0x344550));
            ctx.draw_line(vec2(952., 40.), vec2(952., 590.), 1., Color::hex(0x344550));
            ctx.draw_rect(
                Rect::new(game.paddle - 56., 545., 112., 12.),
                Color::hex(0xd9e5d7),
            );
            ctx.draw_circle(game.ball, 7., Color::hex(0xf3f2df));
            hud(ctx, game.score, game.lives, 1);
            if game.lives == 0 {
                message(ctx, "RUN OVER", "Press R to try again");
            } else if game.bricks.is_empty() {
                message(ctx, "SCREEN CLEARED", "Press R to play again");
            } else if !game.launched {
                text(
                    ctx,
                    "SPACE TO SERVE",
                    vec2(385., 360.),
                    19.,
                    Color::hex(0xb2c0bb),
                );
            }
            report(game.score, game.lives, 1, game.bricks.len() as u32);
        }
    });
}
