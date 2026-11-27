//! Wildfire: keep a fire in a wooded valley away from its five villages.
//!
//! The rules run on the CPU in a 128 x 128 grid (see `sim`): fuel, wind,
//! slope, moisture and spot fires. The GPU draws on top of them:
//!
//! - a compute pass at 512 x 512 turns the grid into flickering flame,
//!   streaked downwind, and charred ground with ragged edges;
//! - every tree is an instance in one GPU buffer, and its vertex shader
//!   reads the fire grid to sway, char and shed its crown: no CPU work per tree;
//! - embers are GPU particles that rise from burning ground and drift
//!   downwind, respawning on whatever is burning now.
//!
//! The page passes pointer input in through the exported functions at the
//! bottom: a drag digs a firebreak, a click calls a water bomber.

pub mod sim;

use ponko::d3::*;
use ponko::perf::Instant;
use ponko::prelude::*;
use ponko::render::{ComputePass, Material, MaterialDesc, Mesh, MeshData, Shader};
use sim::{Fuel, HALF, Happening, N, STEP, Sim, cell_at, cell_center, index, valley};
use std::cell::{Cell, RefCell};
use std::sync::Arc;

/// Cells of firebreak the crew can dig at most, and how fast they get more.
const BREAK_MAX: f32 = 80.0;
const BREAK_PER_SECOND: f32 = 1.6;
const DROPS_MAX: u32 = 3;
const DROP_RECHARGE: f32 = 9.0;
const DROP_LENGTH: f32 = 14.0;
const DROP_WIDTH: f32 = 5.0;
/// Seconds of play before each lightning strike.
const STRIKES: [f32; 3] = [0.5, 45.0, 95.0];
const FINE: u32 = 512;
const EMBERS: u32 = 12_000;
const FIRE_LIGHTS: usize = 14;
const SMOKES: usize = 14;
const MISTS: usize = 6;
/// Hidden things wait here, under the ground and out of view.
const PARKED: Vec3 = vec3(0.0, -80.0, 0.0);

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Title,
    Playing,
    Over,
}

/// Events for the page to explain what happened.
#[derive(Clone, Copy)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
enum Event {
    Strike = 1,
    WindWarning = 2,
    WindShift = 3,
    VillageLost = 4,
    SpotFire = 5,
    NoBreak = 6,
    NoWater = 7,
    Bomber = 8,
    Doused = 9,
    FireOut = 10,
    Dug = 11,
    CantDig = 12,
}

#[derive(Clone, Copy, Default)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
struct Hud {
    phase: u8,
    villages: u32,
    villages_total: u32,
    green: f32,
    burning: u32,
    wind_angle: f32,
    wind_speed: f32,
    next_wind_angle: f32,
    wind_warning: f32,
    breaks: f32,
    drops: u32,
    drop_charge: f32,
    time: f32,
    sim_ms: f32,
    fps: f32,
    strikes_left: u32,
    score: u32,
    best: u32,
    trees: u32,
}

thread_local! {
    /// The pointer over the canvas, 0..1 across and down; `None` off it.
    static HOVER: Cell<Option<Vec2>> = const { Cell::new(None) };
    /// Pointer strokes from the page: (x, y, kind) with kind 0 down, 1 move, 2 up.
    static STROKES: RefCell<Vec<(Vec2, u8)>> = const { RefCell::new(Vec::new()) };
    static START: Cell<bool> = const { Cell::new(false) };
    static EVENTS: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static HUD: Cell<Hud> = Cell::new(Hud::default());
}

fn emit(e: Event) {
    EVENTS.with(|v| {
        let mut v = v.borrow_mut();
        if v.len() < 64 {
            v.push(e as u8);
        }
    });
}

/// The map as it was generated, to start each game from.
struct Start {
    sim: Sim,
    villages: Vec<Vec<usize>>,
}

/// One house: its cell and its two models.
struct House {
    cell: usize,
    walls: Entity,
    roof: Entity,
    lost: bool,
}

struct Bomber {
    body: Entity,
    from: Vec3,
    to: Vec3,
    t: f32,
    dropped: bool,
    target: Vec2,
    dir: Vec2,
}

struct State {
    start: Arc<Start>,
    phase: Phase,
    sim: Sim,
    time: f32,
    acc: f32,
    breaks: f32,
    drops: u32,
    drop_charge: f32,
    strikes_done: usize,
    wind_angle: f32,
    /// Seconds until the wind turns, and where it turns to.
    wind_in: f32,
    next_wind: (f32, f32),
    warned: bool,
    houses: Vec<House>,
    villages_lost: Vec<bool>,
    calm_for: f32,
    score: u32,
    best: u32,
    sim_ms: f32,
    rng: sim::Rng,
    /// The pointer stroke: where it went down and the last cell dug.
    down: Option<(Vec2, bool, Option<usize>)>,
    bombers: Vec<Bomber>,
    flash: f32,
    bolt: Vec<Vec3>,
    doused_told: bool,
    dug_told: bool,
}

impl State {
    fn new(start: Arc<Start>, houses: Vec<House>) -> Self {
        State {
            phase: Phase::Title,
            sim: start.sim.clone(),
            time: 0.0,
            acc: 0.0,
            breaks: BREAK_MAX,
            drops: DROPS_MAX,
            drop_charge: 0.0,
            strikes_done: 0,
            wind_angle: 0.0,
            wind_in: 38.0,
            next_wind: (0.0, 1.0),
            warned: false,
            houses,
            villages_lost: vec![false; start.villages.len()],
            start: start.clone(),
            calm_for: 0.0,
            score: 0,
            best: 0,
            sim_ms: 0.0,
            rng: sim::Rng::new(0xF12E),
            down: None,
            bombers: Vec::new(),
            flash: 0.0,
            bolt: Vec::new(),
            doused_told: false,
            dug_told: false,
        }
    }

    fn set_wind(&mut self, angle: f32, speed: f32) {
        self.wind_angle = angle;
        self.sim.wind = (angle.cos(), angle.sin());
        self.sim.wind_speed = speed;
    }

    fn villages_left(&self) -> u32 {
        self.villages_lost.iter().filter(|l| !**l).count() as u32
    }
}

/// Handles the renderer needs each frame.
struct Look {
    state_texture: Texture,
    state_pixels: Vec<u8>,
    compute_material: Material,
    ember_material: Material,
    house_material: Material,
    bomber: Mesh,
    trees: u32,
}

#[derive(Component)]
struct FireLight(usize);
#[derive(Component)]
struct Smoke(usize);
#[derive(Component)]
struct Mist(usize);
#[derive(Component)]
struct Bolt;
#[derive(Component)]
struct BreakPreview;

/// Ground height at a world point, from the grid (smooth between cells).
fn ground(sim: &Sim, x: f32, z: f32) -> f32 {
    let (u, v) = ((x + HALF - 0.5).clamp(0.0, N as f32 - 1.001), (z + HALF - 0.5).clamp(0.0, N as f32 - 1.001));
    let (i, j) = (u as usize, v as usize);
    let (fx, fz) = (u - i as f32, v - j as f32);
    let h = |x: usize, z: usize| sim.height[index(x.min(N - 1), z.min(N - 1))].max(-0.35);
    let a = h(i, j) + (h(i + 1, j) - h(i, j)) * fx;
    let b = h(i, j + 1) + (h(i + 1, j + 1) - h(i, j + 1)) * fx;
    a + (b - a) * fz
}

/// Where the page's pointer meets the ground.
fn pointed(ctx: &Ctx, sim: &Sim, at: Vec2) -> Option<Vec2> {
    let size = ctx.window_size();
    let (from, dir) = ctx.scene3d.camera.screen_ray(at * size);
    let mut t = 0.0;
    let mut last = from;
    for _ in 0..800 {
        t += 0.4;
        let p = from + dir * t;
        if p.y < ground(sim, p.x, p.z) {
            // Halve the step a few times for the exact spot.
            let (mut a, mut b) = (last, p);
            for _ in 0..8 {
                let m = (a + b) * 0.5;
                if m.y < ground(sim, m.x, m.z) {
                    b = m;
                } else {
                    a = m;
                }
            }
            return (b.x.abs() < HALF && b.z.abs() < HALF).then(|| vec2(b.x, b.z));
        }
        last = p;
    }
    None
}

/// How far back the camera must sit to see the whole valley.
fn fit_distance(target: Vec3, dir: Vec3, aspect: f32, fov_y: f32) -> f32 {
    let forward = -dir;
    let right = forward.cross(Vec3::Y).normalize();
    let up = right.cross(forward);
    let (ty, tx) = ((fov_y * 0.5).tan() * 0.84, (fov_y * 0.5).tan() * aspect * 0.97);
    let e = HALF;
    let corners = [
        vec3(-e, 0.0, -e),
        vec3(e, 0.0, -e),
        vec3(-e, 0.0, e),
        vec3(e, 0.0, e),
        vec3(-e, 14.0, -e),
        vec3(e, 14.0, -e),
    ];
    let mut d = 40.0;
    while d < 600.0 {
        let eye = target + dir * d;
        let fits = corners.iter().all(|&c| {
            let v = c - eye;
            let z = v.dot(forward);
            (v.dot(right) / z).abs() < tx && (v.dot(up) / z).abs() < ty
        });
        if fits {
            return d;
        }
        d += 1.0;
    }
    d
}

fn reset(ctx: &mut Ctx) {
    let s = ctx.world.resource_mut::<State>();
    let start_sim = s.start.sim.clone();
    let villages = s.start.villages.len();
    let best = s.best;
    s.sim = start_sim;
    s.phase = Phase::Playing;
    s.time = 0.0;
    s.acc = 0.0;
    s.breaks = BREAK_MAX;
    s.drops = DROPS_MAX;
    s.drop_charge = 0.0;
    s.strikes_done = 0;
    s.wind_in = 38.0;
    s.warned = false;
    s.villages_lost = vec![false; villages];
    s.calm_for = 0.0;
    s.score = 0;
    s.best = best;
    s.down = None;
    s.doused_told = false;
    s.dug_told = false;
    for h in &mut s.houses {
        h.lost = false;
    }
    let angle = s.rng.range(-0.5, 0.5);
    s.set_wind(angle, 1.0);
    let bodies: Vec<Entity> = s.bombers.drain(..).map(|b| b.body).collect();
    for b in bodies {
        ctx.world.despawn(b);
    }
}

/// A spot for lightning: forest, upwind of the middle, away from the villages.
fn strike_spot(s: &mut State, start: &Start) -> usize {
    let (wx, wz) = s.sim.wind;
    let mut best = (f32::MIN, 0);
    for _ in 0..200 {
        let c = s.rng.below((N * N) as u32) as usize;
        if s.sim.kind[c] != Fuel::Forest || !s.sim.can_burn(c) {
            continue;
        }
        let (x, z) = cell_center(c);
        let near_village = start
            .villages
            .iter()
            .flatten()
            .map(|&h| {
                let (hx, hz) = cell_center(h);
                ((hx - x).powi(2) + (hz - z).powi(2)).sqrt()
            })
            .fold(f32::MAX, f32::min);
        // Upwind of the middle, and far enough from houses to give a chance.
        let upwind = -(x * wx + z * wz) / HALF;
        let score = upwind * 20.0 + near_village.min(30.0) + s.rng.f32() * 8.0;
        if near_village > 18.0 && score > best.0 {
            best = (score, c);
        }
    }
    best.1
}

/// Lightning, the wind, digging, bombers, and the end of the game.
fn control(ctx: &mut Ctx) {
    let dt = ctx.time.dt().min(0.05);
    // The whole valley in view, whatever the window's shape.
    let size = ctx.window_size();
    let aspect = (size.x / size.y.max(1.0)).max(0.3);
    let phase = ctx.world.resource::<State>().phase;
    let now = ctx.time.elapsed() as f32;
    let sway = if phase == Phase::Title { (now * 0.05).sin() * 0.12 } else { 0.0 };
    let dir = vec3(sway.sin() * 0.72, 0.7, sway.cos() * 0.72).normalize();
    let target = vec3(0.0, 0.0, 4.0);
    let distance = fit_distance(target, dir, aspect, ctx.scene3d.camera.fov_y);
    ctx.scene3d.camera.position = target + dir * distance;
    ctx.scene3d.camera.look_at(target);
    ctx.scene3d.sky.fog_start = distance + 40.0;
    ctx.scene3d.sky.fog_end = distance + 260.0;

    let start = START.with(|s| s.replace(false)) || ctx.input.key_pressed(Key::Enter);
    let strokes: Vec<(Vec2, u8)> = STROKES.with(|t| std::mem::take(&mut *t.borrow_mut()));
    if start && phase != Phase::Playing {
        reset(ctx);
        return;
    }

    let mut s = ctx.world.remove_resource::<State>().expect("state");
    let start_map = s.start.clone();

    s.flash = (s.flash - dt * 3.0).max(0.0);
    match s.phase {
        Phase::Title => {
            // The attract loop: the valley burns by itself, relit now and then.
            s.time += dt;
            if s.sim.burning_count == 0 && s.time > 1.0 {
                let c = strike_spot(&mut s, &start_map);
                lightning(&mut s, c);
                s.time = 0.0;
            }
            if s.sim.green_left(&start_map.sim) < 0.45 {
                s.sim = start_map.sim.clone();
                s.time = 0.0;
            }
        }
        Phase::Over => {}
        Phase::Playing => {
            s.time += dt;
            s.breaks = (s.breaks + BREAK_PER_SECOND * dt).min(BREAK_MAX);
            if s.drops < DROPS_MAX {
                s.drop_charge += dt / DROP_RECHARGE;
                if s.drop_charge >= 1.0 {
                    s.drop_charge = 0.0;
                    s.drops += 1;
                }
            }
            if s.strikes_done < STRIKES.len() && s.time >= STRIKES[s.strikes_done] {
                let c = strike_spot(&mut s, &start_map);
                lightning(&mut s, c);
                s.strikes_done += 1;
                emit(Event::Strike);
            }
            // The wind turns every 35 to 45 seconds, with a warning first.
            s.wind_in -= dt;
            if !s.warned && s.wind_in < 7.0 {
                let turn = s.rng.range(0.7, 1.9) * if s.rng.f32() < 0.5 { -1.0 } else { 1.0 };
                let speed = s.rng.range(0.7, 1.7);
                s.next_wind = (s.wind_angle + turn, speed);
                s.warned = true;
                emit(Event::WindWarning);
            }
            if s.wind_in <= 0.0 {
                let (a, sp) = s.next_wind;
                s.set_wind(a, sp);
                s.wind_in = s.rng.range(35.0, 45.0);
                s.warned = false;
                emit(Event::WindShift);
            }
            // Pointer strokes: a drag digs, a click calls a bomber.
            for (at, kind) in strokes {
                let Some(p) = pointed(ctx, &s.sim, at) else {
                    if kind == 2 {
                        s.down = None;
                    }
                    continue;
                };
                match kind {
                    0 => s.down = Some((at, false, None)),
                    1 => {
                        let Some((from, dragging, last)) = s.down else { continue };
                        let dragging = dragging || (at - from).length() * size.y > 14.0;
                        let mut last = last;
                        if dragging {
                            last = dig_to(&mut s, last, p);
                        }
                        s.down = Some((from, dragging, last));
                    }
                    _ => {
                        if let Some((_, false, _)) = s.down {
                            call_bomber(ctx, &mut s, p);
                        }
                        s.down = None;
                    }
                }
            }
            // The end: every strike done and the fire out, or no village left.
            if s.strikes_done == STRIKES.len() && s.sim.burning_count == 0 {
                s.calm_for += dt;
            } else {
                s.calm_for = 0.0;
            }
            if s.calm_for > 2.0 || s.villages_left() == 0 {
                let green = s.sim.green_left(&start_map.sim);
                s.score = s.villages_left() * 1000 + (green * 1000.0) as u32;
                s.best = s.best.max(s.score);
                s.phase = Phase::Over;
                s.down = None;
                emit(Event::FireOut);
            }
        }
    }

    // Run the fire in fixed steps.
    let started = Instant::now();
    s.acc += dt;
    while s.acc >= STEP {
        s.acc -= STEP;
        s.sim.step();
    }
    let happenings = std::mem::take(&mut s.sim.happenings);
    let ms = started.elapsed().as_secs_f32() * 1000.0;
    s.sim_ms += (ms - s.sim_ms) * 0.05;
    for h in &happenings {
        if let Happening::SpotFire { .. } = h
            && s.phase == Phase::Playing
        {
            emit(Event::SpotFire);
        }
    }
    // Houses burn; a village is lost when half of its houses are.
    for v in 0..start_map.villages.len() {
        if s.villages_lost[v] {
            continue;
        }
        let cells = &start_map.villages[v];
        let gone = cells.iter().filter(|&&c| s.sim.fuel[c] < 0.6).count();
        if gone * 2 >= cells.len() && !cells.is_empty() {
            s.villages_lost[v] = true;
            if s.phase == Phase::Playing {
                emit(Event::VillageLost);
            }
        }
    }
    ctx.world.insert_resource(s);
}

fn lightning(s: &mut State, c: usize) {
    s.sim.strike(c);
    s.flash = 1.0;
    let (x, z) = cell_center(c);
    let y = ground(&s.sim, x, z);
    let mut points = Vec::new();
    let (mut px, mut pz) = (x + s.rng.range(-6.0, 6.0), z + s.rng.range(-6.0, 6.0));
    for i in 0..=14 {
        let k = i as f32 / 14.0;
        let h = y + 70.0 * (1.0 - k);
        points.push(vec3(px, h, pz));
        px += (x - px) * k + s.rng.range(-1.5, 1.5);
        pz += (z - pz) * k + s.rng.range(-1.5, 1.5);
    }
    points.push(vec3(x, y, z));
    s.bolt = points;
}

/// Digs from the last dug cell to `p`, as far as the crew can. Returns the
/// last cell dug.
fn dig_to(s: &mut State, last: Option<usize>, p: Vec2) -> Option<usize> {
    let Some(target) = cell_at(p.x, p.y) else { return last };
    let from = last.map_or(p, |c| {
        let (x, z) = cell_center(c);
        vec2(x, z)
    });
    let steps = ((p - from).length() * 2.0).ceil().max(1.0) as usize;
    let mut last = last;
    for k in 0..=steps {
        let q = from + (p - from) * (k as f32 / steps as f32);
        let Some(c) = cell_at(q.x, q.y) else { continue };
        if Some(c) == last {
            continue;
        }
        if s.sim.kind[c] == Fuel::Break {
            last = Some(c);
            continue;
        }
        if s.breaks < 1.0 {
            emit(Event::NoBreak);
            return last;
        }
        if s.sim.dig(c) {
            if s.sim.kind[c] == Fuel::Break {
                s.breaks -= 1.0;
                if !s.dug_told {
                    s.dug_told = true;
                    emit(Event::Dug);
                }
            }
        } else if s.sim.burning[c] || s.sim.kind[c] == Fuel::Houses {
            emit(Event::CantDig);
        }
        last = Some(c);
    }
    let _ = target;
    last
}

fn call_bomber(ctx: &mut Ctx, s: &mut State, p: Vec2) {
    if s.drops == 0 {
        emit(Event::NoWater);
        return;
    }
    s.drops -= 1;
    emit(Event::Bomber);
    // It flies in across the wind, so the strip lies along the fire's front.
    let wind = vec2(s.sim.wind.0, s.sim.wind.1);
    let dir = vec2(-wind.y, wind.x);
    let dir = if dir.dot(vec2(p.x, p.y)) > 0.0 { -dir } else { dir };
    let height = ground(&s.sim, p.x, p.y) + 9.0;
    let from = vec3(p.x - dir.x * 120.0, height + 10.0, p.y - dir.y * 120.0);
    let to = vec3(p.x + dir.x * 120.0, height + 14.0, p.y + dir.y * 120.0);
    let look = ctx.world.resource::<Look>();
    let mesh = look.bomber;
    let body = ctx.world.spawn((Transform::at(from), Model::new(mesh)));
    s.bombers.push(Bomber { body, from, to, t: 0.0, dropped: false, target: p, dir });
}

/// Bombers fly, drop and leave.
fn bombers(ctx: &mut Ctx) {
    let dt = ctx.time.dt().min(0.05);
    let mut s = ctx.world.remove_resource::<State>().expect("state");
    let mut mists: Vec<(Vec3, Vec2)> = Vec::new();
    let mut gone = Vec::new();
    for (i, b) in s.bombers.iter_mut().enumerate() {
        b.t += dt / 4.0;
        // Ease down toward the drop and climb away after it.
        let k = b.t;
        let mut p = b.from.lerp(b.to, k);
        let dip = 1.0 - ((k - 0.5) * 2.4).powi(2).min(1.0);
        p.y -= dip * 10.0;
        if let Some(t) = ctx.world.get_mut::<Transform>(b.body) {
            t.position = p;
            let heading = (b.to - b.from).normalize();
            t.rotation = Quat::from_rotation_arc(Vec3::Z, heading)
                * Quat::from_rotation_z((k * 6.0).sin() * 0.05);
        }
        if !b.dropped && k >= 0.5 {
            b.dropped = true;
            mists.push((vec3(b.target.x, p.y, b.target.y), b.dir));
        }
        if k >= 1.0 {
            gone.push(i);
        }
    }
    for &(at, dir) in &mists {
        let out = s.sim.drench(at.x, at.z, (dir.x, dir.y), DROP_LENGTH, DROP_WIDTH);
        if out > 0 && !s.doused_told {
            s.doused_told = true;
            emit(Event::Doused);
        }
    }
    for i in gone.into_iter().rev() {
        let b = s.bombers.remove(i);
        ctx.world.despawn(b.body);
    }
    let sim_ground: Vec<f32> = mists.iter().map(|(at, _)| ground(&s.sim, at.x, at.z)).collect();
    ctx.world.insert_resource(s);
    if mists.is_empty() {
        return;
    }
    // Spray along the strip, from the plane down to the ground.
    let mut slot = 0;
    for (i, (at, dir)) in mists.iter().enumerate() {
        let floor = sim_ground[i];
        for (t, emitter, m) in ctx.world.query::<(&mut Transform, &mut ParticleEmitter, &Mist)>().iter() {
            if m.0 / 2 != slot {
                continue;
            }
            let along = if m.0 % 2 == 0 { -0.25 } else { 0.25 };
            t.position = vec3(at.x + dir.x * DROP_LENGTH * along, floor + 3.0, at.z + dir.y * DROP_LENGTH * along);
            emitter.burst(220);
        }
        slot = (slot + 1) % (MISTS / 2);
    }
}

/// Uploads the grid and steers the GPU layers; moves lights, smoke, the
/// bolt and the houses; fills the HUD.
fn present(ctx: &mut Ctx) {
    let now = ctx.time.elapsed() as f32;
    let dt = ctx.time.dt().min(0.05);
    let fps = ctx.time.fps();
    let start = ctx.world.resource::<State>().start.clone();

    // The grid as a texture: heat, char, moisture, firebreak.
    let mut pixels = std::mem::take(&mut ctx.world.resource_mut::<Look>().state_pixels);
    let s = ctx.world.resource::<State>();
    let sim = &s.sim;
    for c in 0..N * N {
        let heat = if sim.burning[c] { sim.heat[c] } else { sim.heat[c] * 0.5 };
        pixels[c * 4] = (heat.clamp(0.0, 1.0) * 255.0) as u8;
        pixels[c * 4 + 1] = (sim.char(c, &start.sim) * 255.0) as u8;
        pixels[c * 4 + 2] = (sim.moisture[c].clamp(0.0, 1.0) * 255.0) as u8;
        pixels[c * 4 + 3] = if sim.kind[c] == Fuel::Break { 255 } else { 0 };
    }

    // Fire lights and smoke on the hottest blocks of 8 x 8 cells.
    const B: usize = 8;
    let mut blocks: Vec<(f32, Vec3)> = Vec::with_capacity((N / B) * (N / B));
    for bz in 0..N / B {
        for bx in 0..N / B {
            let (mut sum, mut at, mut w) = (0.0, Vec3::ZERO, 0.0);
            for z in bz * B..(bz + 1) * B {
                for x in bx * B..(bx + 1) * B {
                    let c = index(x, z);
                    if sim.burning[c] {
                        let h = sim.heat[c];
                        sum += h;
                        let (px, pz) = cell_center(c);
                        at += vec3(px, sim.height[c].max(0.0), pz) * h;
                        w += h;
                    }
                }
            }
            if sum > 0.5 {
                blocks.push((sum, at / w));
            }
        }
    }
    blocks.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
    let wind = vec3(sim.wind.0, 0.0, sim.wind.1) * sim.wind_speed;
    let (flash, bolt, phase) = (s.flash, s.bolt.clone(), s.phase);
    let hud = Hud {
        phase: s.phase as u8,
        villages: s.villages_left(),
        villages_total: start.villages.len() as u32,
        green: sim.green_left(&start.sim),
        burning: sim.burning_count as u32,
        wind_angle: s.wind_angle,
        wind_speed: sim.wind_speed,
        next_wind_angle: s.next_wind.0,
        wind_warning: if s.warned { s.wind_in.max(0.0) } else { -1.0 },
        breaks: s.breaks / BREAK_MAX,
        drops: s.drops,
        drop_charge: s.drop_charge,
        time: s.time,
        sim_ms: s.sim_ms,
        fps,
        strikes_left: (STRIKES.len() - s.strikes_done) as u32,
        score: s.score,
        best: s.best,
        trees: ctx.world.resource::<Look>().trees,
    };
    let house_state: Vec<(Entity, Entity, f32, f32)> = s
        .houses
        .iter()
        .map(|h| (h.walls, h.roof, sim.char(h.cell, &start.sim), if sim.burning[h.cell] { sim.heat[h.cell] } else { 0.0 }))
        .collect();
    let preview: Vec<Vec3> = Vec::new();
    let _ = phase;
    HUD.with(|h| h.set(hud));

    let look = ctx.world.resource::<Look>();
    let (state_texture, compute_material, ember_material) = (look.state_texture, look.compute_material, look.ember_material);
    ctx.gfx.update_texture(state_texture, &pixels);
    ctx.world.resource_mut::<Look>().state_pixels = pixels;
    let w4 = vec4(wind.x, wind.z, 0.0, 0.0);
    ctx.gfx.update_material(
        compute_material,
        MaterialDesc::default()
            .with_custom([w4, Vec4::ZERO, Vec4::ZERO, Vec4::ZERO])
            .with_custom_textures([Some(state_texture), None]),
    );
    let mut ember = ember_desc(ctx);
    ember.custom[0] = vec4(wind.x, wind.z, 0.0, EMBERS as f32);
    ctx.gfx.update_material(ember_material, ember);

    for (t, light, f) in ctx.world.query::<(&mut Transform, &mut PointLight, &FireLight)>().iter() {
        if let Some(&(sum, at)) = blocks.get(f.0) {
            let flicker = 0.8 + 0.2 * (now * 13.0 + f.0 as f32 * 3.1).sin() * (now * 7.3 + f.0 as f32).cos();
            t.position = at + Vec3::Y * 2.5;
            light.intensity = (sum * 3.0).min(90.0) * flicker;
            light.range = 10.0 + (sum * 0.6).min(14.0);
        } else {
            light.intensity = 0.0;
            t.position = PARKED;
        }
    }
    for (t, emitter, sm) in ctx.world.query::<(&mut Transform, &mut ParticleEmitter, &Smoke)>().iter() {
        if let Some(&(sum, at)) = blocks.get(sm.0) {
            t.position = at + Vec3::Y * 1.0 + wind * 1.5;
            // A puff in proportion to the fire, as often as the frame rate allows.
            let want = (sum * 1.4).min(40.0) * dt;
            let n = want as usize + usize::from(((now * 977.0 + sm.0 as f32 * 31.0).sin() * 0.5 + 0.5) < want.fract());
            if n > 0 {
                emitter.burst(n);
            }
        }
    }
    // The lightning: a bright jagged line for a moment, and the sky lit.
    for (line, _) in ctx.world.query::<(&mut Line, &Bolt)>().iter() {
        if flash > 0.0 {
            line.points.clone_from(&bolt);
            let k = flash * flash * 6.0;
            line.color = Color::rgba(k, k, k * 1.3, 1.0);
        } else {
            line.points.clear();
        }
    }
    ctx.scene3d.lighting.ambient_intensity = 0.8 + flash * 2.5;
    for (line, _) in ctx.world.query::<(&mut Line, &BreakPreview)>().iter() {
        line.points.clone_from(&preview);
    }
    // Houses: lit windows while they stand, glowing then black when they burn.
    for (walls, roof, ch, heat) in house_state {
        let fire = heat * 3.0;
        let tint = Color::rgb(1.0 - ch * 0.92 + fire, 1.0 - ch * 0.94 + fire * 0.35, 1.0 - ch * 0.95);
        for e in [walls, roof] {
            if let Some(m) = ctx.world.get_mut::<Model>(e) {
                m.color = tint;
            }
        }
        if let Some(t) = ctx.world.get_mut::<Transform>(roof) {
            t.scale.y = 0.42 * (1.0 - ch * 0.8);
        }
    }
}

fn ember_desc(ctx: &Ctx) -> MaterialDesc {
    let mut d = MaterialDesc::color(Color::rgb(1.0, 0.5, 0.15));
    d.shader = EMBER_SHADER.with(Cell::get);
    let look = ctx.world.resource::<Look>();
    d.custom_textures = [Some(look.state_texture), HEIGHT_TEXTURE.with(Cell::get)];
    d
}

thread_local! {
    static EMBER_SHADER: Cell<Option<Shader>> = const { Cell::new(None) };
    static HEIGHT_TEXTURE: Cell<Option<Texture>> = const { Cell::new(None) };
}

/// The fine fire layer: flame that flickers and streaks downwind, from the
/// grid's heat. custom_texture0: the grid; custom[0].xy: the wind.
const FLAME: &str = "
fn compute(c: Cell) -> vec4<f32> {
    let t = frame.time.x;
    let dt = min(frame.time.y, 0.05);
    let wind = material.custom[0].xy;
    let grid = textureSampleLevel(custom_texture0, material_sampler, c.uv, 0.0);
    // Last frame's flame, carried a little downwind.
    let back = c.uv - wind * dt * 0.012;
    let prev = cell_at(vec2<i32>(back * vec2<f32>(c.size)));
    let n = fbm2(c.uv * 70.0 - wind * t * 0.6 + vec2<f32>(t * 0.3, -t * 0.2));
    let lick = noise2(c.uv * 300.0 + vec2<f32>(0.0, t * 4.0));
    let flame = grid.r * smoothstep(0.32, 0.7, n * 0.9 + grid.r * 0.45) * (0.55 + 0.9 * lick);
    let heat = max(flame, prev.r * exp(-dt * 4.0));
    // Charred ground with ragged edges.
    let edge = fbm2(c.uv * 160.0 + 3.0);
    let char_ = smoothstep(edge * 0.8 - 0.05, edge * 0.8 + 0.1, grid.g);
    // Embers glowing on fresh ash.
    let glow = grid.g * smoothstep(0.75, 0.95, noise2(c.uv * 520.0 + vec2<f32>(t * 0.4, 0.0))) * clamp(grid.r * 4.0, 0.0, 1.0);
    return vec4<f32>(heat, char_, glow, 1.0);
}";

const FIRE_RAMP: &str = "
fn fire_color(h: f32) -> vec3<f32> {
    let deep = vec3<f32>(1.0, 0.12, 0.02);
    let mid = vec3<f32>(1.0, 0.45, 0.08);
    let hot = vec3<f32>(1.0, 0.7, 0.3);
    let c = mix(mix(deep, mid, smoothstep(0.1, 0.5, h)), hot, smoothstep(0.6, 1.0, h));
    return c * h * h * 1.6;
}
fn grid_uv(world: vec3<f32>) -> vec2<f32> {
    return (world.xz + 64.0) / 128.0;
}
";

/// The ground: its own colors (vertex colors), charred, soaked, dug or
/// burning. custom_texture0: the fine layer; custom_texture1: the grid.
const GROUND: &str = "
fn surface(s: Surface) -> Surface {
    var out = s;
    let uv = grid_uv(s.world);
    // Sample first: browsers want every texture read in uniform control
    // flow, so not after the early return below.
    let grid = textureSample(custom_texture1, material_sampler, uv);
    let fine = textureSample(custom_texture0, material_sampler, uv);
    if max(abs(s.world.x), abs(s.world.z)) > 63.97 && abs(s.normal.y) < 0.5 {
        // The sides of the block of land.
        out.albedo = vec3<f32>(0.07, 0.05, 0.04);
        out.roughness = 0.95;
        return out;
    }
    var albedo = s.albedo * (0.85 + 0.3 * noise2(s.world.xz * 1.7));
    // Wet ground: darker, glossy, a little blue.
    let wet = smoothstep(0.45, 0.9, grid.b);
    albedo = mix(albedo, albedo * vec3<f32>(0.45, 0.5, 0.6), wet);
    out.roughness = mix(0.92, 0.35, wet);
    // A firebreak: bare earth with furrows.
    let dug = smoothstep(0.3, 0.7, grid.a);
    let furrow = 0.8 + 0.2 * sin(s.world.x * 9.0 + s.world.z * 9.0);
    albedo = mix(albedo, vec3<f32>(0.32, 0.2, 0.11) * furrow, dug);
    // Char, then flame on top.
    albedo = mix(albedo, vec3<f32>(0.025, 0.022, 0.02), fine.g);
    out.albedo = albedo;
    out.emissive = fire_color(fine.r) * 1.4 + vec3<f32>(1.0, 0.22, 0.04) * fine.b * 1.2;
    return out;
}";

/// Trees from the GPU buffer: items[i] = (x, y, z, size). The grid
/// (custom_texture0) makes them sway while they burn and lose their crown
/// when they're burnt. uv.x is 1 on the crown, 0 on the trunk.
const TREES: &str = "
fn tree_state(at: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(custom_texture0, material_sampler, grid_uv(vec3<f32>(at.x, 0.0, at.y)), 0.0);
}
fn vertex(v: Vertex) -> Vertex {
    var out = v;
    let it = items[v.instance_index];
    let grid = tree_state(it.xz);
    let seed = hash2(it.xz * 1.37);
    let crown = v.uv.x;
    let burnt = smoothstep(0.35, 0.85, grid.g + (seed - 0.5) * 0.35);
    var p = (v.position - v.origin) * it.w;
    // A burnt crown shrinks to a few black stubs; a burning one thrashes.
    let shrink = mix(1.0, 0.25, burnt * crown);
    p = vec3<f32>(p.x * shrink, mix(p.y, p.y * 0.85, burnt * crown), p.z * shrink);
    let sway = sin(frame.time.x * 11.0 + seed * 40.0) * grid.r * 0.08 + sin(frame.time.x * 1.3 + seed * 9.0) * 0.015;
    p.x += sway * p.y;
    p.z += sway * 0.6 * p.y;
    // A slight lean and turn per tree.
    let a = seed * 6.28;
    p = vec3<f32>(p.x * cos(a) - p.z * sin(a), p.y, p.x * sin(a) + p.z * cos(a));
    out.position = v.origin + p + it.xyz;
    return out;
}
fn surface(s: Surface) -> Surface {
    var out = s;
    let grid = tree_state(s.world.xz);
    let seed = hash2(floor(s.world.xz * 2.0));
    let crown = s.uv.x;
    let burnt = smoothstep(0.35, 0.85, grid.g + (seed - 0.5) * 0.35);
    let tint = mix(vec3<f32>(0.8), vec3<f32>(1.15, 1.05, 0.7), seed);
    out.albedo = mix(s.albedo * tint, vec3<f32>(0.02, 0.018, 0.016), burnt);
    let flame = grid.r * smoothstep(0.2, 0.9, noise2(s.world.xz * 2.0 + vec2<f32>(0.0, frame.time.x * 5.0) + s.world.y * 1.5));
    out.emissive = fire_color(flame) * (0.25 + crown * 0.6) * smoothstep(0.15, 0.4, grid.r);
    return out;
}";

/// Embers: items[i] = (position, life), items[i + n] = (velocity, max life).
/// custom_texture0: the grid; custom_texture1: heights; custom[0]: wind xy, count w.
const EMBER_SIM: &str = "
fn update(i: u32) {
    let n = u32(material.custom[0].w);
    if i >= n { return; }
    var p = items[i];
    var v = items[i + n];
    let dt = min(frame.time.y, 0.05);
    let t = frame.time.x;
    let wind = material.custom[0].xy;
    p.w -= dt;
    if p.w <= 0.0 {
        let r = vec2<f32>(hash2(vec2<f32>(f32(i) * 0.731, t * 17.3)), hash2(vec2<f32>(t * 9.17, f32(i) * 1.31)));
        let cell = vec2<i32>(r * 128.0);
        let grid = textureLoad(custom_texture0, cell, 0);
        let hh = textureLoad(custom_texture1, cell, 0);
        let h = (hh.r + hh.g / 255.0) * 40.0 - 4.0;
        if grid.r > 0.3 && hash2(vec2<f32>(f32(i), t * 3.7)) < grid.r * 0.9 {
            let j = vec2<f32>(hash2(r * 7.1), hash2(r * 3.3)) - 0.5;
            p = vec4<f32>(f32(cell.x) - 63.5 + j.x, max(h, -0.3) + 0.4, f32(cell.y) - 63.5 + j.y, 0.0);
            let life = 1.2 + hash2(r * 5.9) * 2.8;
            p.w = life;
            v = vec4<f32>(wind.x * 0.6 + j.x * 1.5, 2.0 + hash2(r * 2.7) * 3.5, wind.y * 0.6 + j.y * 1.5, life);
        } else {
            p = vec4<f32>(0.0, -1000.0, 0.0, 0.0);
        }
    } else {
        let swirl = vec2<f32>(noise2(p.xz * 0.6 + t), noise2(p.zx * 0.6 - t)) - 0.5;
        let air = vec3<f32>(wind.x * 2.2 + swirl.x * 3.0, 0.4, wind.y * 2.2 + swirl.y * 3.0);
        v = vec4<f32>(v.xyz + (air - v.xyz) * min(dt * 0.9, 1.0), v.w);
        p = vec4<f32>(p.xyz + v.xyz * dt, p.w);
    }
    items[i] = p;
    items[i + n] = v;
}";

const EMBER_DRAW: &str = "
fn vertex(v: Vertex) -> Vertex {
    var out = v;
    let it = items[v.instance_index];
    let life = items[v.instance_index + u32(material.custom[0].w)].w;
    let k = clamp(it.w / max(life, 0.01), 0.0, 1.0);
    let size = select(0.0, 0.6 + k * 0.8, it.w > 0.0);
    out.position = v.origin + (v.position - v.origin) * size + it.xyz;
    out.color = vec4<f32>(k, k, k, 1.0);
    return out;
}
fn surface(s: Surface) -> Surface {
    var out = s;
    let k = s.color.r;
    out.albedo = vec3<f32>(0.0);
    out.emissive = mix(vec3<f32>(1.2, 0.15, 0.02), vec3<f32>(4.0, 1.8, 0.5), k) * (0.4 + k * 1.6);
    return out;
}";

/// Houses: windows lit warm on every side (uv), dark when the house burns.
const HOUSE: &str = "
fn surface(s: Surface) -> Surface {
    var out = s;
    let wall = 1.0 - step(0.5, abs(s.normal.y));
    let w = step(0.3, s.uv.x) * step(s.uv.x, 0.7) * step(0.35, s.uv.y) * step(s.uv.y, 0.65);
    let alive = clamp(s.color.b * 1.2 - 0.2, 0.0, 1.0);
    out.emissive = vec3<f32>(2.2, 1.3, 0.5) * w * wall * alive;
    out.emissive += vec3<f32>(3.0, 0.8, 0.1) * clamp(s.color.r - 1.0, 0.0, 2.0);
    return out;
}";

fn kind_color(k: Fuel) -> Vec3 {
    match k {
        Fuel::Grass => vec3(0.34, 0.40, 0.15),
        Fuel::Shrub => vec3(0.24, 0.28, 0.11),
        Fuel::Forest => vec3(0.11, 0.15, 0.06),
        Fuel::Houses => vec3(0.30, 0.31, 0.20),
        Fuel::Rock => vec3(0.34, 0.33, 0.31),
        Fuel::Water => vec3(0.06, 0.08, 0.08),
        Fuel::Road => vec3(0.36, 0.31, 0.24),
        Fuel::Break => vec3(0.32, 0.2, 0.11),
    }
}

/// The land: a block of ground with the valley on top and earth sides.
fn ground_mesh(sim: &Sim) -> MeshData {
    let side = N + 1;
    let mut m = MeshData::default();
    let at = |x: usize, z: usize| {
        let (wx, wz) = (x as f32 - HALF, z as f32 - HALF);
        valley::height(wx, wz).max(-0.9)
    };
    for z in 0..side {
        for x in 0..side {
            m.positions.push(vec3(x as f32 - HALF, at(x, z), z as f32 - HALF));
            m.uvs.push(vec2(x as f32 / N as f32, z as f32 / N as f32));
            // The average color of the cells around this corner.
            let mut c = Vec3::ZERO;
            let mut w = 0.0;
            for (dx, dz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (cx, cz) = (x as i32 - dx, z as i32 - dz);
                if (0..N as i32).contains(&cx) && (0..N as i32).contains(&cz) {
                    c += kind_color(sim.kind[index(cx as usize, cz as usize)]);
                    w += 1.0;
                }
            }
            let c = c / w;
            m.colors.push(Color::rgb(c.x, c.y, c.z));
        }
    }
    for z in 0..N {
        for x in 0..N {
            let i = (z * side + x) as u32;
            let s = side as u32;
            m.indices.extend([i, i + s, i + 1, i + 1, i + s, i + s + 1]);
        }
    }
    // Skirts down each edge.
    let bottom = -8.0;
    let mut skirt = |edge: Vec<(usize, usize)>, flip: bool| {
        for w in edge.windows(2) {
            let base = m.positions.len() as u32;
            for &(x, z) in w {
                let top = vec3(x as f32 - HALF, at(x, z), z as f32 - HALF);
                m.positions.push(top);
                m.positions.push(vec3(top.x, bottom, top.z));
                m.uvs.push(Vec2::ZERO);
                m.uvs.push(Vec2::ZERO);
                m.colors.push(Color::rgb(0.2, 0.15, 0.1));
                m.colors.push(Color::rgb(0.2, 0.15, 0.1));
            }
            let (a, b, c, d) = (base, base + 1, base + 2, base + 3);
            if flip {
                m.indices.extend([a, c, b, b, c, d]);
            } else {
                m.indices.extend([a, b, c, b, d, c]);
            }
        }
    };
    skirt((0..side).map(|x| (x, 0)).collect(), true);
    skirt((0..side).map(|x| (x, N)).collect(), false);
    skirt((0..side).map(|z| (0, z)).collect(), false);
    skirt((0..side).map(|z| (N, z)).collect(), true);
    m
}

/// A low-poly conifer: a trunk (uv.x 0) and three stacked cones (uv.x 1).
fn tree_mesh() -> MeshData {
    let mut m = MeshData::default();
    let seg = 7;
    let cone = |m: &mut MeshData, y0: f32, y1: f32, r: f32, crown: f32, color: Vec3| {
        let base = m.positions.len() as u32;
        let tip = base + seg as u32;
        for i in 0..seg {
            let a = i as f32 / seg as f32 * std::f32::consts::TAU;
            m.positions.push(vec3(a.cos() * r, y0, a.sin() * r));
            m.uvs.push(vec2(crown, 0.0));
            m.colors.push(Color::rgb(color.x * 0.75, color.y * 0.75, color.z * 0.75));
        }
        m.positions.push(vec3(0.0, y1, 0.0));
        m.uvs.push(vec2(crown, 1.0));
        m.colors.push(Color::rgb(color.x, color.y, color.z));
        for i in 0..seg as u32 {
            let j = (i + 1) % seg as u32;
            m.indices.extend([base + i, tip, base + j]);
        }
    };
    let green = vec3(0.13, 0.26, 0.11);
    cone(&mut m, 0.0, 1.0, 0.12, 0.0, vec3(0.2, 0.13, 0.08));
    cone(&mut m, 0.5, 1.9, 0.75, 1.0, green);
    cone(&mut m, 1.1, 2.5, 0.58, 1.0, green * 1.1);
    cone(&mut m, 1.7, 3.0, 0.4, 1.0, green * 1.2);
    m
}

/// A tiny diamond for each ember.
fn ember_mesh() -> MeshData {
    let s = 0.07;
    let mut m = MeshData::default();
    m.positions = vec![
        vec3(s, 0.0, 0.0),
        vec3(-s, 0.0, 0.0),
        vec3(0.0, s, 0.0),
        vec3(0.0, -s, 0.0),
        vec3(0.0, 0.0, s),
        vec3(0.0, 0.0, -s),
    ];
    m.indices = vec![0, 2, 4, 4, 2, 1, 1, 2, 5, 5, 2, 0, 4, 3, 0, 1, 3, 4, 5, 3, 1, 0, 3, 5];
    m
}

fn setup(ctx: &mut Ctx) {
    let night = Color::hex(0x07090D);
    ctx.gfx.clear(night);
    ctx.scene3d.sky.atmosphere = false;
    ctx.scene3d.sky.fog_color = night;
    ctx.scene3d.sky.sky_color = Color::hex(0x34425E);
    ctx.scene3d.sky.ground_color = Color::hex(0x141A14);
    ctx.scene3d.lighting.sun_direction = vec3(0.35, -1.0, 0.55);
    ctx.scene3d.lighting.sun_intensity = 1.6;
    ctx.scene3d.lighting.sun_color = Color::hex(0x9FB4E0);
    ctx.scene3d.lighting.ambient_intensity = 0.8;
    ctx.scene3d.post.bloom = 0.6;
    ctx.scene3d.post.bloom_threshold = 1.0;
    ctx.scene3d.post.vignette = 0.35;
    ctx.scene3d.camera.far = 900.0;
    ctx.scene3d.camera.fov_y = 0.55;

    let (kind, height, moisture, villages) = valley::generate();
    let sim = Sim::new(kind, height, moisture, 0xF12E);

    // The grid's textures: state (every frame) and heights (once).
    let state_pixels = vec![0u8; N * N * 4];
    let state_texture = ctx.gfx.create_texture(N as u32, N as u32, &state_pixels);
    let mut heights = vec![0u8; N * N * 4];
    for c in 0..N * N {
        let h = ((sim.height[c] + 4.0) / 40.0).clamp(0.0, 0.9999) * 255.0;
        heights[c * 4] = h.floor() as u8;
        heights[c * 4 + 1] = ((h - h.floor()) * 255.0) as u8;
        heights[c * 4 + 3] = 255;
    }
    let height_texture = ctx.gfx.create_texture(N as u32, N as u32, &heights);
    ctx.gfx.set_texture_filter(height_texture, TextureFilter::Nearest);
    HEIGHT_TEXTURE.with(|t| t.set(Some(height_texture)));

    // The fine flame layer, computed on the GPU from the grid.
    let fine = ctx.gfx.create_render_target(FINE, FINE);
    let flame = ctx.gfx.create_compute("flame", FLAME);
    let compute_material = ctx.gfx.create_material(MaterialDesc::default().with_custom_textures([Some(state_texture), None]));
    ctx.scene3d.passes.compute.push(ComputePass::new(flame, fine).with_material(compute_material));

    // The land.
    let ground_shader = ctx.gfx.create_shader("ground", &format!("{FIRE_RAMP}{GROUND}"));
    let ground_material = ctx.gfx.create_material(
        MaterialDesc::color(Color::WHITE)
            .with_roughness(0.9)
            .with_shader(ground_shader)
            .with_custom_textures([Some(fine.texture()), Some(state_texture)]),
    );
    let land = ctx.gfx.create_mesh(&ground_mesh(&sim), ground_material);
    ctx.world.spawn((Transform::default(), Model::new(land)));
    // The river and a dark floor far below.
    let water = ctx.gfx.create_material(MaterialDesc::color(Color::hex(0x0C1820)).with_roughness(0.08).with_metallic(0.2));
    let river = ctx.gfx.create_mesh(&MeshData::plane(N as f32 - 0.02), water);
    ctx.world.spawn((Transform::at(vec3(0.0, -0.35, 0.0)), Model::new(river)));
    let floor_material = ctx.gfx.create_material(MaterialDesc::color(Color::hex(0x050607)).with_roughness(1.0));
    let floor = ctx.gfx.create_mesh(&MeshData::plane(2000.0), floor_material);
    ctx.world.spawn((Transform::at(vec3(0.0, -8.0, 0.0)), Model::new(floor)));

    // Trees: every one an element of one GPU buffer.
    let mut rng = sim::Rng::new(99);
    let mut items = Vec::new();
    for c in 0..N * N {
        let (per, size) = match sim.kind[c] {
            Fuel::Forest => (4, (0.8, 1.35)),
            Fuel::Shrub => (1, (0.45, 0.7)),
            Fuel::Grass if rng.f32() < 0.05 => (1, (0.7, 1.1)),
            _ => (0, (0.0, 0.0)),
        };
        let (cx, cz) = cell_center(c);
        for _ in 0..per {
            let (x, z) = (cx + rng.range(-0.48, 0.48), cz + rng.range(-0.48, 0.48));
            items.push(vec4(x, ground(&sim, x, z) - 0.05, z, rng.range(size.0, size.1)));
        }
    }
    let trees = items.len() as u32;
    let tree_shader = ctx.gfx.create_shader("trees", &format!("{FIRE_RAMP}{TREES}"));
    let tree_material = ctx.gfx.create_material(
        MaterialDesc::color(Color::WHITE)
            .with_roughness(0.85)
            .with_shader(tree_shader)
            .with_custom_textures([Some(state_texture), None]),
    );
    let tree = ctx.gfx.create_mesh(&tree_mesh(), tree_material);
    let tree_buffer = ctx.gfx.create_gpu_buffer(trees, &items);
    ctx.gfx.set_mesh_gpu_buffer(tree, tree_buffer, trees);
    ctx.gfx.set_mesh_gpu_bounds(tree, Vec3::ZERO, HALF * 1.5);
    ctx.gfx.set_mesh_shadows(tree, false);
    ctx.world.spawn((Transform::default(), Model::new(tree)));

    // Embers: simulated and drawn on the GPU.
    let ember_shader = ctx.gfx.create_shader("embers", EMBER_DRAW);
    EMBER_SHADER.with(|s| s.set(Some(ember_shader)));
    let embers = ctx.gfx.create_gpu_buffer(EMBERS * 2, &[]);
    let ember_sim = ctx.gfx.create_compute("ember-sim", EMBER_SIM);
    let mut ember_material_desc = MaterialDesc::color(Color::rgb(1.0, 0.5, 0.15)).with_shader(ember_shader);
    ember_material_desc.custom_textures = [Some(state_texture), Some(height_texture)];
    ember_material_desc.custom[0] = vec4(1.0, 0.0, 0.0, EMBERS as f32);
    let ember_material = ctx.gfx.create_material(ember_material_desc);
    ctx.scene3d.passes.compute.push(ComputePass::for_buffer(ember_sim, embers).with_material(ember_material));
    let ember = ctx.gfx.create_mesh(&ember_mesh(), ember_material);
    ctx.gfx.set_mesh_gpu_buffer(ember, embers, EMBERS);
    ctx.gfx.set_mesh_gpu_bounds(ember, Vec3::ZERO, HALF * 2.0);
    ctx.gfx.set_mesh_shadows(ember, false);
    ctx.world.spawn((Transform::default(), Model::new(ember)));

    // Houses: a box and a roof per house cell.
    let house_shader = ctx.gfx.create_shader("house", HOUSE);
    let house_material = ctx.gfx.create_material(
        MaterialDesc::color(Color::hex(0xB9A58A)).with_roughness(0.8).with_shader(house_shader),
    );
    let roof_material = ctx.gfx.create_material(MaterialDesc::color(Color::hex(0x7A3A2A)).with_roughness(0.7));
    let walls_mesh = ctx.gfx.create_mesh(&MeshData::cube(1.0), house_material);
    let roof_mesh = ctx.gfx.create_mesh(&MeshData::cube(1.0), roof_material);
    let mut houses = Vec::new();
    for v in &villages {
        for &c in v {
            let (x, z) = cell_center(c);
            let y = ground(&sim, x, z);
            let turn = Quat::from_rotation_y(rng.range(-0.3, 0.3));
            let walls = ctx.world.spawn((
                Transform { position: vec3(x, y + 0.3, z), rotation: turn, scale: vec3(0.8, 0.6, 0.62) },
                Model::new(walls_mesh),
            ));
            let roof = ctx.world.spawn((
                Transform {
                    position: vec3(x, y + 0.72, z),
                    rotation: turn * Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
                    scale: vec3(0.42, 0.42, 0.66),
                },
                Model::new(roof_mesh),
            ));
            houses.push(House { cell: c, walls, roof, lost: false });
        }
    }

    // The water bomber: a body and wings.
    let paint = ctx.gfx.create_material(MaterialDesc::color(Color::hex(0xE8D24A)).with_roughness(0.4).with_metallic(0.3));
    let mut body = MeshData::cube(1.0);
    for p in &mut body.positions {
        *p *= vec3(0.9, 0.8, 4.2);
    }
    let mut wing = MeshData::cube(1.0);
    for p in &mut wing.positions {
        *p *= vec3(7.5, 0.18, 1.2);
    }
    let base = body.positions.len() as u32;
    body.positions.extend(wing.positions);
    body.normals.extend(wing.normals);
    body.uvs.extend(wing.uvs);
    body.indices.extend(wing.indices.iter().map(|i| i + base));
    let bomber = ctx.gfx.create_mesh(&body, paint);

    // Firelight, pooled and moved to the hottest blocks each frame.
    for i in 0..FIRE_LIGHTS {
        ctx.world.spawn((
            Transform::at(PARKED),
            PointLight { color: Color::hex(0xFF8A3D), intensity: 0.0, range: 14.0, ..Default::default() },
            FireLight(i),
        ));
    }
    // Smoke columns.
    for i in 0..SMOKES {
        let mut settings = ParticleSettings::default()
            .with_rate(0.0)
            .with_lifetime(4.0, 7.0)
            .with_speed(0.5, 1.5)
            .with_shape(ParticleShape::Sphere { radius: 2.0 })
            .with_drag(0.3)
            .with_size(1.4, 2.6)
            .with_rotation(0.0, std::f32::consts::TAU)
            .with_size_over_life(ponko::particles::ParticleCurve::new(vec![(0.0, 0.4), (1.0, 2.6)]))
            .with_color_over_life(ponko::particles::ParticleGradient::new(vec![
                (0.0, Color::rgba(0.9, 0.45, 0.2, 0.0)),
                (0.12, Color::rgba(0.55, 0.42, 0.36, 0.4)),
                (0.5, Color::rgba(0.38, 0.37, 0.4, 0.25)),
                (1.0, Color::rgba(0.3, 0.31, 0.35, 0.0)),
            ]))
            .with_blend(ParticleBlend::Alpha)
            .with_max_particles(300)
            .with_seed(i as u64 + 1);
        settings.gravity = vec3(0.0, 1.0, 0.0);
        ctx.world.spawn((Transform::at(PARKED), ParticleEmitter::new(settings), Smoke(i)));
    }
    // Spray from the bombers.
    for i in 0..MISTS {
        let settings = ParticleSettings::default()
            .with_rate(0.0)
            .with_lifetime(1.2, 2.4)
            .with_speed(0.5, 3.0)
            .with_shape(ParticleShape::Sphere { radius: 3.0 })
            .with_gravity(vec3(0.0, -2.5, 0.0))
            .with_drag(1.2)
            .with_size(0.6, 1.4)
            .with_size_over_life(ponko::particles::ParticleCurve::new(vec![(0.0, 0.5), (1.0, 2.0)]))
            .with_color_over_life(ponko::particles::ParticleGradient::new(vec![
                (0.0, Color::rgba(0.7, 0.8, 0.9, 0.5)),
                (1.0, Color::rgba(0.6, 0.65, 0.7, 0.0)),
            ]))
            .with_blend(ParticleBlend::Alpha)
            .with_max_particles(500)
            .with_seed(100 + i as u64);
        ctx.world.spawn((Transform::at(PARKED), ParticleEmitter::new(settings), Mist(i)));
    }
    ctx.world.spawn((
        Transform::default(),
        Line { blend: ParticleBlend::Additive, ..Line::new(Vec::new(), 0.35, Color::WHITE) },
        Bolt,
    ));
    ctx.world.spawn((
        Transform::default(),
        Line { blend: ParticleBlend::Additive, ..Line::new(Vec::new(), 0.3, Color::rgba(1.6, 1.2, 0.4, 1.0)) },
        BreakPreview,
    ));

    ctx.world.insert_resource(Look {
        state_texture,
        state_pixels,
        compute_material,
        ember_material,
        house_material,
        bomber,
        trees,
    });
    let start = Arc::new(Start { sim, villages });
    let mut state = State::new(start, houses);
    state.set_wind(0.2, 1.1);
    ctx.world.insert_resource(state);

    // Desktop testing: WARP=seconds runs the attract fire ahead.
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(warp) = std::env::var("WARP").ok().and_then(|w| w.parse::<f32>().ok()) {
        let s = ctx.world.resource_mut::<State>();
        let map = s.start.clone();
        let c = strike_spot(s, &map);
        s.sim.strike(c);
        for _ in 0..(warp / STEP) as usize {
            s.sim.step();
        }
        s.time = 1.0;
    }
}

/// Saves a screenshot after a number of frames when `SHOT` is set (desktop).
fn screenshot(ctx: &mut Ctx) {
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(path) = std::env::var("SHOT") {
        let at: u64 = std::env::var("SHOT_FRAME").ok().and_then(|f| f.parse().ok()).unwrap_or(90);
        if ctx.time.frame() == at {
            ctx.gfx.save_screenshot(path);
        }
        if ctx.time.frame() == at + 3 {
            ctx.quit();
        }
    }
    #[cfg(target_arch = "wasm32")]
    if web::debugging() && matches!(ctx.time.frame(), 60 | 300) {
        web::log(&format!("models drawn: {}", ctx.gfx.models_drawn()));
        for st in ctx.gfx.shader_status() {
            if !st.is_ok() {
                for d in &st.diagnostics {
                    web::log(&format!("FAIL {} {:?}: {}", st.shader, st.kind, d.message.replace('\n', " | ")));
                }
            }
        }
    }
    let _ = ctx;
}

/// Runs the game.
pub fn run() {
    Game::new("Wildfire · ponko", 1280, 720)
        .system(control)
        .system(bombers)
        .system(present)
        .system(screenshot)
        .setup(setup)
        .run();
}

#[cfg(target_arch = "wasm32")]
mod web {
    use super::*;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console, js_name = warn)]
        pub fn log(s: &str);
    }

    thread_local! {
        static DEBUG: Cell<bool> = const { Cell::new(false) };
    }

    pub fn debugging() -> bool {
        DEBUG.with(Cell::get)
    }

    /// Logs the GPU state to the console a few seconds in.
    #[wasm_bindgen]
    pub fn debug() {
        DEBUG.with(|d| d.set(true));
    }

    #[wasm_bindgen]
    pub fn start() {
        run();
    }

    /// The pointer moved over the canvas (x and y from 0 to 1); negative: it left.
    #[wasm_bindgen]
    pub fn hover(x: f32, y: f32) {
        HOVER.with(|h| h.set((x >= 0.0 && y >= 0.0).then_some(vec2(x, y))));
    }

    /// A pointer stroke: kind 0 pressed, 1 moved while pressed, 2 released.
    #[wasm_bindgen]
    pub fn stroke(x: f32, y: f32, kind: u8) {
        STROKES.with(|t| t.borrow_mut().push((vec2(x, y), kind)));
    }

    #[wasm_bindgen]
    pub fn begin() {
        START.with(|s| s.set(true));
    }

    /// What happened since the last call (see `Event`).
    #[wasm_bindgen]
    pub fn events() -> Vec<u8> {
        EVENTS.with(|v| std::mem::take(&mut *v.borrow_mut()))
    }

    /// [phase (0 title, 1 playing, 2 over), villages left, villages,
    ///  vegetation left 0..1, burning cells, wind angle (radians, where it
    ///  blows to; x east, y south), wind speed 0..2, next wind angle,
    ///  seconds to the wind turning (-1: no warning), firebreak left 0..1,
    ///  water drops, next drop 0..1, seconds played, sim ms, fps,
    ///  strikes still to come, score, best, trees]
    #[wasm_bindgen]
    pub fn hud() -> Vec<f32> {
        let h = HUD.with(Cell::get);
        vec![
            h.phase as f32,
            h.villages as f32,
            h.villages_total as f32,
            h.green,
            h.burning as f32,
            h.wind_angle,
            h.wind_speed,
            h.next_wind_angle,
            h.wind_warning,
            h.breaks,
            h.drops as f32,
            h.drop_charge,
            h.time,
            h.sim_ms,
            h.fps,
            h.strikes_left as f32,
            h.score as f32,
            h.best as f32,
            h.trees as f32,
        ]
    }
}
