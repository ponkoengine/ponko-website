//! Shared Ponko rendering and a small DOM bridge for touch controls.
use ponko::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(
    inline_js = "export function arcadeInput(){return window.__ponkoInput||0} export function arcadePaused(){return document.hidden||window.__ponkoPaused||window.__ponkoOffscreen||!document.hasFocus()} export function arcadeReport(score,lives,wave,entities){window.__ponkoState={score,lives,wave,entities}; if(!window.__ponkoReady){window.__ponkoReady=true;window.dispatchEvent(new Event('ponko-ready'))} }"
)]
extern "C" {
    fn arcadeInput() -> u32;
    fn arcadePaused() -> bool;
    fn arcadeReport(score: u32, lives: u32, wave: u32, entities: u32);
}
pub fn touch() -> u32 {
    #[cfg(target_arch = "wasm32")]
    {
        arcadeInput()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0
    }
}
pub fn paused() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        arcadePaused()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}
pub fn report(score: u32, lives: u32, wave: u32, entities: u32) {
    #[cfg(target_arch = "wasm32")]
    arcadeReport(score, lives, wave, entities);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (score, lives, wave, entities);
}
pub fn axis(positive: bool, negative: bool) -> f32 {
    positive as u8 as f32 - negative as u8 as f32
}
pub fn fit(ctx: &mut Ctx) {
    let viewport = ctx.window_size();
    let zoom = (viewport.x / 960.).min(viewport.y / 600.);
    ctx.camera.zoom = zoom;
    ctx.camera.position = (vec2(960., 600.) - viewport / zoom) * 0.5;
}
pub fn text(ctx: &mut Ctx, value: &str, at: Vec2, size: f32, color: Color) {
    let screen = ctx.camera.world_to_screen(at);
    ctx.draw_text(value, screen, size * ctx.camera.zoom, color);
}
pub fn hud(ctx: &mut Ctx, score: u32, lives: u32, wave: u32) {
    text(
        ctx,
        &format!("SCORE {score:06}"),
        vec2(24., 20.),
        18.,
        Color::hex(0xe4e7df),
    );
    text(
        ctx,
        &format!("LIVES {lives}"),
        vec2(750., 20.),
        18.,
        Color::hex(0xe4e7df),
    );
    text(
        ctx,
        &format!("WAVE {wave:02}"),
        vec2(420., 20.),
        18.,
        Color::hex(0x8d9995),
    );
}
pub fn message(ctx: &mut Ctx, title: &str, subtitle: &str) {
    ctx.draw_rect(
        Rect::new(180., 235., 600., 130.),
        Color::hex(0x0e151b).with_alpha(0.94),
    );
    text(ctx, title, vec2(285., 255.), 34., Color::hex(0xe4e7df));
    text(ctx, subtitle, vec2(290., 310.), 17., Color::hex(0xb2c0bb));
}
pub fn stars(ctx: &mut Ctx) {
    for i in 0..90 {
        let x = ((i * 277 + 43) % 960) as f32;
        let y = ((i * 157 + 61) % 540 + 50) as f32;
        ctx.draw_circle(
            vec2(x, y),
            if i % 7 == 0 { 1.4 } else { 0.7 },
            Color::hex(0x55646e),
        );
    }
}
