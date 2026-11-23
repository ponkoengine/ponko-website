//! Hole: a black hole whose accretion disk is hundreds of thousands of
//! simulated particles.
//!
//! Every particle is an ECS entity, moved every frame on the CPU: it orbits
//! the hole at the speed its radius gives, drifts slowly inward, falls in
//! past the last stable orbit and comes back at the outer edge. Particles
//! start in clumps, which the disk's shear draws out into streaks. A click
//! or tap throws a star: a ball of a few thousand particles under the
//! hole's gravity, torn into a stream and swallowed.
//!
//! Each frame the particles are added up into a map of the disk (a small
//! texture, angle across and radius down), and one full-screen shader
//! (`trace.wgsl`) traces every pixel's light back past the hole: where a
//! ray crosses the disk, it reads the map. So the gas's clumps and a star's
//! stream are bent by the hole like everything else, with the far side of
//! the disk over the top, its second image under the bottom and the photon
//! ring around the shadow.
//!
//! The particle count adapts to the machine: it starts low and grows over
//! the first seconds while the display's frame rate holds; the traced
//! resolution backs off if the GPU can't keep up.

use ponko::d3::*;
use ponko::perf::Instant;
use ponko::prelude::*;
use ponko::render::{Effect, Material, MaterialDesc, MeshData, ScreenEffect, Tonemap};
use std::cell::{Cell, RefCell};
use std::f32::consts::TAU;

/// The disk: from the last stable orbit, a little beyond.
const DISK_OUTER: f32 = 15.5;
/// Simulated time per second: the inner edge turns in about three seconds.
const TIME_SCALE: f32 = 10.0;
/// A thrown star falls faster than the disk turns, so it reads as an event.
const STAR_TIME_SCALE: f32 = 22.0;
/// Particles per thrown star, and how many stars can be in flight.
const STAR_PARTICLES: usize = 3_000;
const STARS: usize = 3;
/// The disk's particle count is kept between these.
const MIN_DISK: usize = 20_000;
const MAX_DISK: usize = 600_000;
const DEFAULT_DISK: usize = 50_000;
/// Disk particles start (and respawn) in clumps of about this many.
const CLUMP: usize = 240;

/// The disk map: angle across, sqrt(radius) down, from MAP_R0 to MAP_R1
/// (the same numbers as in `trace.wgsl`).
const MAP_W: usize = 512;
const MAP_H: usize = 128;
const MAP_R0: f32 = 1.2;
const MAP_R1: f32 = 17.0;

/// One particle of the disk: where it is around the hole, and its own
/// brightness.
#[derive(Component, Clone, Copy)]
struct Gas {
    r: f32,
    angle: f32,
    glow: f32,
    slot: u32,
}

/// One particle of a thrown star.
#[derive(Component, Clone, Copy)]
struct Debris {
    pos: Vec3,
    vel: Vec3,
    glow: f32,
    alive: bool,
    star: u8,
}

/// Camera and interaction state.
struct View {
    yaw: f32,
    pitch: f32,
    distance: f32,
    /// Seconds since the last touch; the camera drifts on its own after a while.
    idle: f32,
    drift_time: f32,
}

/// A thrown star, as the tracer sees it: a glowing ball until the hole
/// pulls it apart.
#[derive(Clone, Copy, Default)]
struct Star {
    center: Vec3,
    spread: f32,
    alive: usize,
    age: f32,
}

/// The simulation's shared state.
struct Sim {
    effect: Effect,
    material: Material,
    desc: MaterialDesc,
    map: Texture,
    /// The disk map's layers, MAP_W x MAP_H each: this frame's gas, its
    /// smoothed density relative to each radius, a star's debris now, and
    /// the heat its debris left in the disk.
    gas: Vec<f32>,
    smooth: Vec<f32>,
    debris: Vec<f32>,
    heat: Vec<f32>,
    scratch: Vec<f32>,
    texels: Vec<u8>,
    /// Linear 0..1 (in 1/4096 steps) to the sRGB byte the GPU decodes back.
    encode: Vec<u8>,
    heat_left: bool,
    stars: [Star; STARS],
    flare: f32,
    /// The fraction of the window the trace is drawn at, and its floor.
    scale: f32,
    /// Held still: the tracer off (the engine keeps running frames while the
    /// page is hidden).
    held: bool,
    /// Disk particles simulated and drawn, and spawned so far.
    active: usize,
    spawned: usize,
    /// The count may not grow past this (set after a shrink).
    ceiling: usize,
    next_star: usize,
    rng: Rng,
    last_frame: Instant,
    refresh: f32,
    fixed_count: bool,
    stable_time: f32,
    frame_debt: f32,
    fps: f32,
    sim_ms: f32,
    frames: u32,
}

#[derive(Clone, Copy, Default)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
struct Hud {
    particles: u32,
    fps: f32,
    sim_ms: f32,
    frames: u32,
    scale: f32,
}

thread_local! {
    static DRAG: Cell<Vec2> = const { Cell::new(Vec2::ZERO) };
    static ZOOM: Cell<f32> = const { Cell::new(1.0) };
    static THROWS: RefCell<Vec<Vec2>> = const { RefCell::new(Vec::new()) };
    static PAUSED: Cell<bool> = const { Cell::new(false) };
    static HUD: Cell<Hud> = Cell::new(Hud::default());
    static ASKED: Cell<usize> = const { Cell::new(0) };
    static REFRESH: Cell<f32> = const { Cell::new(1.0 / 60.0) };
    static FIXED_COUNT: Cell<bool> = const { Cell::new(false) };
    static SCALE: Cell<f32> = const { Cell::new(1.0) };
    static SHADER: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Circular orbital speed at radius r (Paczyński–Wiita: close to the real
/// thing outside the last stable orbit), in units of light speed.
fn orbit_speed(r: f32) -> f32 {
    (r * 0.5).sqrt() / (r - 1.0).max(0.25)
}

/// Inward drift, Schwarzschild radii per real second: slow in the disk,
/// a plunge inside the last stable orbit.
fn drift(r: f32) -> f32 {
    if r > 3.0 {
        0.10 + 0.25 / r
    } else {
        0.2 + (3.0 - r) * 2.5
    }
}

/// A new clump center: anywhere in the disk at the start, at the outer
/// edge afterwards.
fn clump(rng: &mut Rng, fill: bool) -> (f32, f32, f32) {
    let r = if fill {
        let t = rng.f32();
        2.6 + (DISK_OUTER - 2.6) * t.powf(1.15)
    } else {
        rng.range(DISK_OUTER - 1.5, DISK_OUTER)
    };
    let glow = 0.25 + 1.6 * rng.f32().powi(3);
    (r, rng.angle(), glow)
}

fn gaussian(rng: &mut Rng) -> f32 {
    // Sum of uniforms: cheap and good enough for a spread.
    (rng.f32() + rng.f32() + rng.f32() - 1.5) * 1.15
}

fn place(rng: &mut Rng, center: (f32, f32, f32)) -> Gas {
    let (cr, cphi, cglow) = center;
    // Clumps are arcs already, as if the disk's shear had been at them.
    let r = (cr + gaussian(rng) * (0.14 + 0.03 * cr)).clamp(1.3, DISK_OUTER + 0.8);
    let angle = (cphi + gaussian(rng) * 0.5 + (r - cr) * 0.6).rem_euclid(TAU);
    let glow = cglow * (0.5 + rng.f32().powi(3) * 1.2);
    Gas {
        r,
        angle,
        glow,
        slot: 0,
    }
}

/// The disk map's row for radius r (may be off the map).
fn map_row(r: f32) -> f32 {
    let (s0, s1) = (MAP_R0.sqrt(), MAP_R1.sqrt());
    (r.sqrt() - s0) / (s1 - s0) * MAP_H as f32
}

fn row_radius(row: usize) -> f32 {
    let (s0, s1) = (MAP_R0.sqrt(), MAP_R1.sqrt());
    let s = s0 + (row as f32 + 0.5) / MAP_H as f32 * (s1 - s0);
    s * s
}

/// The texel a point of the disk adds to, if it is on the map.
fn map_cell(r: f32, angle: f32) -> Option<usize> {
    let row = map_row(r);
    if !(0.0..MAP_H as f32).contains(&row) {
        return None;
    }
    // The tracer reads u = angle / TAU + 0.5 for an angle from atan2.
    let u = (angle / TAU + 0.5).rem_euclid(1.0);
    let col = ((u * MAP_W as f32) as usize).min(MAP_W - 1);
    Some(row as usize * MAP_W + col)
}

fn camera_basis(view: &View) -> Vec3 {
    vec3(
        view.pitch.cos() * view.yaw.cos(),
        view.pitch.sin(),
        view.pitch.cos() * view.yaw.sin(),
    ) * view.distance
}

fn setup(ctx: &mut Ctx, asked: usize) {
    ctx.gfx.clear(Color::BLACK);
    ctx.scene3d.sky.atmosphere = false;
    ctx.scene3d.sky.sky_color = Color::BLACK;
    ctx.scene3d.sky.ground_color = Color::BLACK;
    ctx.scene3d.sky.fog_color = Color::BLACK;
    ctx.scene3d.lighting.sun_intensity = 0.0;
    ctx.scene3d.lighting.shadows = false;
    ctx.scene3d.lighting.ambient_intensity = 0.0;
    ctx.scene3d.post.msaa = 1;
    ctx.scene3d.post.bloom = 0.1;
    ctx.scene3d.post.bloom_threshold = 1.5;
    ctx.scene3d.post.exposure = 0.0;
    ctx.scene3d.post.tonemap = Tonemap::AgX;
    ctx.scene3d.post.vignette = 0.3;
    ctx.scene3d.post.saturation = 1.15;
    ctx.scene3d.camera.fov_y = 0.72;
    ctx.scene3d.camera.near = 0.05;
    ctx.scene3d.camera.far = 400.0;
    ctx.scene3d.camera.occlusion_culling = false;

    let effect = ctx
        .gfx
        .create_effect("trace.wgsl", include_str!("trace.wgsl"));
    let map = ctx
        .gfx
        .create_texture(MAP_W as u32, MAP_H as u32, &vec![0; MAP_W * MAP_H * 4]);
    let mut desc = MaterialDesc::default();
    desc.custom_textures = [Some(map), None];
    let material = ctx.gfx.create_material(desc);
    ctx.scene3d
        .passes
        .effects
        .push(ScreenEffect::new(effect).with_material(material));

    // The engine draws a 3D frame (and runs its effects) only for a scene
    // with something in it: a speck inside the hole, which the trace
    // paints over anyway.
    let black = ctx.gfx.create_material(MaterialDesc::color(Color::BLACK));
    let speck = ctx.gfx.create_mesh(
        &MeshData {
            positions: vec![Vec3::ZERO, vec3(0.01, 0.0, 0.0), vec3(0.0, 0.01, 0.0)],
            normals: vec![Vec3::Z; 3],
            uvs: vec![Vec2::ZERO; 3],
            indices: vec![0, 1, 2],
            ..Default::default()
        },
        black,
    );
    ctx.world.spawn((Transform::default(), Model::new(speck)));

    let scale = SCALE.with(Cell::get);
    let mut sim = Sim {
        effect,
        material,
        desc,
        map,
        gas: vec![0.0; MAP_W * MAP_H],
        smooth: vec![0.0; MAP_W * MAP_H],
        debris: vec![0.0; MAP_W * MAP_H],
        heat: vec![0.0; MAP_W * MAP_H],
        scratch: vec![0.0; MAP_W * MAP_H],
        texels: vec![255; MAP_W * MAP_H * 4],
        encode: (0..=4096)
            .map(|i| {
                let v = i as f32 / 4096.0;
                let s = if v <= 0.003_130_8 {
                    v * 12.92
                } else {
                    1.055 * v.powf(1.0 / 2.4) - 0.055
                };
                (s * 255.0).round() as u8
            })
            .collect(),
        heat_left: false,
        stars: [Star::default(); STARS],
        flare: 0.0,
        scale,
        held: false,
        active: 0,
        spawned: 0,
        ceiling: MAX_DISK,
        next_star: 0,
        rng: Rng::new(0x0B1A_C401E),
        last_frame: Instant::now(),
        refresh: REFRESH.with(Cell::get),
        fixed_count: FIXED_COUNT.with(Cell::get),
        stable_time: -1.0,
        frame_debt: 0.0,
        fps: 0.0,
        sim_ms: 0.0,
        frames: 0,
    };
    ctx.scene3d.post.render_scale = scale;
    // The star debris pool: parked, invisible until thrown.
    for slot in 0..STARS * STAR_PARTICLES {
        ctx.world.spawn((Debris {
            pos: Vec3::ZERO,
            vel: Vec3::ZERO,
            glow: 0.0,
            alive: false,
            star: (slot / STAR_PARTICLES) as u8,
        },));
    }
    let start = if asked > 0 { asked } else { DEFAULT_DISK };
    grow(ctx, &mut sim, start.clamp(MIN_DISK, MAX_DISK), true);
    ctx.world.insert_resource(sim);
    ctx.world.insert_resource(View {
        yaw: 0.6,
        pitch: 0.17,
        distance: 32.0,
        idle: 100.0,
        drift_time: 0.0,
    });
}

/// Spawns disk particles up to `target` and simulates that many.
fn grow(ctx: &mut Ctx, sim: &mut Sim, target: usize, fill: bool) {
    while sim.spawned < target {
        let center = clump(&mut sim.rng, fill);
        let n = CLUMP.min(target - sim.spawned);
        for _ in 0..n {
            let mut g = place(&mut sim.rng, center);
            g.slot = sim.spawned as u32;
            ctx.world.spawn((g,));
            sim.spawned += 1;
        }
    }
    sim.active = target;
}

/// Camera: drag to orbit, wheel or pinch to zoom, a slow drift otherwise.
fn camera(ctx: &mut Ctx) {
    let dt = ctx.time.dt().min(0.1);
    if ctx.world.get_resource::<View>().is_none() {
        return;
    }
    let paused = PAUSED.with(|p| p.get());
    let drag = DRAG.with(|d| d.replace(Vec2::ZERO));
    let zoom = ZOOM.with(|z| z.replace(1.0));
    #[cfg(not(target_arch = "wasm32"))]
    let (drag, zoom) = {
        let mut drag = drag;
        let mut zoom = zoom;
        if ctx.input.mouse_down(MouseButton::Left) {
            drag += ctx.input.mouse_delta() / ctx.window_size().y.max(1.0);
        }
        zoom *= 0.9f32.powf(ctx.input.mouse_wheel().y);
        (drag, zoom)
    };
    let size = ctx.window_size();
    let view = ctx.world.resource_mut::<View>();
    if drag != Vec2::ZERO || zoom != 1.0 {
        view.idle = 0.0;
    }
    view.yaw += drag.x * 3.0;
    view.pitch = (view.pitch + drag.y * 2.0).clamp(-1.45, 1.45);
    view.distance = (view.distance * zoom).clamp(9.0, 75.0);
    if !paused {
        view.idle += dt;
        // After a few seconds untouched, drift: a slow turn, and the tilt
        // easing between nearly edge-on and a little above the disk.
        let ease = ((view.idle - 4.0) / 3.0).clamp(0.0, 1.0);
        view.drift_time += dt * ease;
        view.yaw += dt * 0.045 * ease;
        let goal = 0.16 + 0.1 * (view.drift_time * 0.09).sin();
        view.pitch += (goal - view.pitch) * (dt * 0.25 * ease);
    }
    // A narrow screen (a phone held upright) needs the camera further back.
    let aspect = size.x / size.y.max(1.0);
    let fit = (1.45 / aspect.max(0.4)).max(1.0);
    let eye = {
        let mut v = View { ..*view };
        v.distance *= fit.powf(0.9);
        camera_basis(&v)
    };
    let cam = &mut ctx.scene3d.camera;
    cam.position = eye;
    cam.look_at(Vec3::ZERO);
}

/// Throws a star from where the page was clicked: it starts out in the
/// disk's plane, under the click, and falls in on a curving orbit.
fn throw(ctx: &mut Ctx, sim: &mut Sim, at: Vec2) {
    let size = ctx.window_size();
    let (from, dir) = ctx.scene3d.camera.screen_ray(at * size);
    // Where the click's ray meets the disk's plane, or failing that, the
    // point of the ray nearest the hole.
    let mut p = if dir.y.abs() > 0.05 && (-from.y / dir.y) > 0.0 {
        from + dir * (-from.y / dir.y)
    } else {
        from + dir * (-from).dot(dir).max(0.0)
    };
    p.y = 0.0;
    if p.length() < 1.0 {
        p = ctx.scene3d.camera.rotation * Vec3::X;
        p.y = 0.0;
    }
    let r = p.length().clamp(9.5, 11.5);
    let p = p.normalize_or(Vec3::X) * r + Vec3::Y * 0.8;
    let tangent = vec3(-p.z, 0.0, p.x).normalize();
    // A little under orbital speed, so it curves inward; the hole's tide
    // stretches it into a stream that winds in over a few seconds.
    let speed = orbit_speed(r);
    let vel = tangent * speed * 0.92 - p.normalize() * speed * 0.05;
    let star = sim.next_star;
    sim.next_star = (sim.next_star + 1) % STARS;
    sim.stars[star] = Star {
        center: p,
        spread: 0.0,
        alive: STAR_PARTICLES,
        age: 0.0,
    };
    let mut rng = Rng::new(sim.rng.u64());
    ctx.world.query::<&mut Debris>().for_each(|d| {
        if d.star as usize != star {
            return;
        }
        // A ball, denser in the middle.
        let dir =
            vec3(gaussian(&mut rng), gaussian(&mut rng), gaussian(&mut rng)).normalize_or_zero();
        let radius = rng.f32().powf(1.6);
        d.pos = p + dir * radius;
        d.vel = vel + dir * (speed * 0.03) * rng.f32();
        d.glow = 0.7 + rng.f32() * 0.6;
        d.alive = true;
    });
}

/// Every particle, every frame.
fn simulate(ctx: &mut Ctx) {
    let started = Instant::now();
    if ctx.world.get_resource::<Sim>().is_none() {
        return;
    }
    let real_dt = ctx.time.dt().min(0.05);
    let paused = PAUSED.with(|p| p.get());
    let dt = if paused { 0.0 } else { real_dt };
    let mut sim = ctx
        .world
        .remove_resource::<Sim>()
        .expect("the simulation's state");

    if let Some(source) = SHADER.with(|s| s.borrow_mut().take()) {
        // A shader that doesn't compile keeps the old one (the engine logs why).
        if let Ok(effect) = ctx.gfx.try_create_effect("trace.wgsl", &source) {
            sim.effect = effect;
            sim.held = !paused;
        }
    }
    for at in THROWS.with(|t| std::mem::take(&mut *t.borrow_mut())) {
        throw(ctx, &mut sim, at);
    }

    if paused != sim.held {
        sim.held = paused;
        let passes = &mut ctx.scene3d.passes.effects;
        passes.clear();
        if !paused {
            passes.push(ScreenEffect::new(sim.effect).with_material(sim.material));
        }
    }

    let elapsed = started.duration_since(sim.last_frame).as_secs_f32();
    sim.last_frame = started;
    if dt > 0.0 {
        step_gas(ctx, &mut sim, dt);
        step_debris(ctx, &mut sim, dt);
        build_map(&mut sim, dt);
        ctx.gfx.update_texture(sim.map, &sim.texels);
    }
    update_tracer(ctx, &mut sim);

    let ms = started.elapsed().as_secs_f32() * 1000.0;
    sim.sim_ms += (ms - sim.sim_ms) * 0.1;
    if paused {
        sim.stable_time = -0.5;
        sim.frame_debt = 0.0;
    } else if elapsed > 0.0 {
        // Average frame duration before taking the reciprocal, so a burst of
        // short callbacks can't produce a runaway FPS estimate.
        let prior = if sim.fps > 0.0 {
            sim.fps.recip()
        } else {
            elapsed
        };
        sim.fps = (prior + (elapsed - prior) * 0.12).recip();
        adapt(ctx, &mut sim, elapsed, ms);
    }
    sim.frames += 1;
    let live_debris: usize = sim.stars.iter().map(|s| s.alive).sum();
    HUD.with(|h| {
        h.set(Hud {
            particles: (sim.active + live_debris) as u32,
            fps: sim.fps,
            sim_ms: sim.sim_ms,
            frames: sim.frames,
            scale: sim.scale,
        })
    });
    ctx.world.insert_resource(sim);
}

/// The disk: each particle turns by its orbit's angle and drifts inward,
/// and is added to this frame's map.
fn step_gas(ctx: &mut Ctx, sim: &mut Sim, dt: f32) {
    let ts = dt * TIME_SCALE;
    let active = sim.active as u32;
    let mut gas = std::mem::take(&mut sim.gas);
    gas.fill(0.0);
    let mut respawn: Vec<u32> = Vec::new();
    ctx.world.query::<&mut Gas>().for_each(|g| {
        if g.slot >= active {
            return;
        }
        let r = g.r;
        let nr = r - drift(r) * dt;
        if nr < 1.15 {
            respawn.push(g.slot);
            return;
        }
        let mut angle = g.angle + orbit_speed(r) / r * ts;
        if angle >= TAU {
            angle -= TAU;
        }
        g.angle = angle;
        g.r = nr;
        if let Some(cell) = map_cell(nr, angle) {
            gas[cell] += g.glow;
        }
    });
    if !respawn.is_empty() {
        // Swallowed particles come back at the outer edge, in clumps.
        respawn.sort_unstable();
        let mut rng = Rng::new(sim.rng.u64());
        let mut center = clump(&mut rng, false);
        let mut left = CLUMP / 4;
        let mut i = 0;
        ctx.world.query::<&mut Gas>().for_each(|g| {
            if i < respawn.len() && respawn.binary_search(&g.slot).is_ok() {
                if left == 0 {
                    center = clump(&mut rng, false);
                    left = CLUMP / 4;
                }
                left -= 1;
                i += 1;
                let slot = g.slot;
                *g = place(&mut rng, center);
                g.slot = slot;
            }
        });
    }
    sim.gas = gas;
}

/// Thrown stars: free fall in the hole's field (finer steps close in),
/// with a little drag so the stream settles into the disk.
fn step_debris(ctx: &mut Ctx, sim: &mut Sim, dt: f32) {
    let step = dt * STAR_TIME_SCALE;
    let keep = 1.0 - 0.05 * dt;
    let mut debris = std::mem::take(&mut sim.debris);
    debris.fill(0.0);
    let mut sums = [(Vec3::ZERO, 0.0f32, 0usize); STARS];
    let mut swallowed = 0usize;
    // A star still in one piece is drawn as a ball of light, not as gas.
    let torn = sim.stars.map(|s| ((s.spread - 0.6) / 0.7).clamp(0.0, 1.0));
    let fade = (-dt / 7.0).exp();
    let mut deposit = std::mem::take(&mut sim.scratch);
    deposit.fill(0.0);
    let any = sim.stars.iter().any(|s| s.alive > 0);
    if any {
        ctx.world.query::<&mut Debris>().for_each(|d| {
            if !d.alive {
                return;
            }
            let steps = if d.pos.length() > 5.0 { 2 } else { 8 };
            let h = step / steps as f32;
            for _ in 0..steps {
                let r = d.pos.length();
                let pull = 0.5 / ((r - 1.0).max(0.1) * (r - 1.0).max(0.1));
                d.vel -= d.pos / r * pull * h;
                // The disk's plane pulls the stream into it.
                d.vel.y -= (d.pos.y * 0.004 + d.vel.y * 0.06) * h;
                d.pos += d.vel * h;
            }
            d.vel *= keep;
            d.glow *= fade;
            let r = d.pos.length();
            if !(1.25..45.0).contains(&r) || !d.pos.is_finite() {
                d.alive = false;
                swallowed += 1;
                return;
            }
            let s = &mut sums[d.star as usize];
            s.0 += d.pos;
            s.1 += d.pos.length_squared();
            s.2 += 1;
            let flat = (d.pos.x * d.pos.x + d.pos.z * d.pos.z).sqrt();
            if let Some(cell) = map_cell(flat, d.pos.z.atan2(d.pos.x)) {
                let near = (-d.pos.y * d.pos.y).exp();
                let torn = torn[d.star as usize];
                debris[cell] += d.glow * near * torn;
                // Close in, the stream heats the disk it runs into.
                let into = ((8.0 - flat) / 4.0).clamp(0.0, 1.0);
                deposit[cell] += d.glow * near * into * torn * dt * 4.0;
            }
        });
    }
    for (star, (sum, sq, n)) in sim.stars.iter_mut().zip(sums) {
        if star.alive == 0 && n == 0 {
            continue;
        }
        star.age += dt;
        star.alive = n;
        if n > 0 {
            let c = sum / n as f32;
            star.center = c;
            star.spread = (sq / n as f32 - c.length_squared()).max(0.0).sqrt();
        }
    }
    sim.flare = sim.flare * (-dt / 0.7).exp() + swallowed as f32 / STAR_PARTICLES as f32 * 1.2;
    sim.flare = sim.flare.min(2.5);

    // The heat a star left turns with the disk, sheared by its differential
    // rotation into an arc, and fades over a few seconds.
    if sim.heat_left || any {
        let fade = (-dt / 2.6).exp();
        let mut peak = 0.0f32;
        let mut row_buf = [0.0f32; MAP_W];
        for row in 0..MAP_H {
            let r = row_radius(row);
            let turn = orbit_speed(r) / r * dt * TIME_SCALE / TAU * MAP_W as f32;
            let line = &mut sim.heat[row * MAP_W..(row + 1) * MAP_W];
            let whole = turn.floor() as usize;
            let frac = turn - turn.floor();
            for (i, out) in row_buf.iter_mut().enumerate() {
                // Values move toward higher angles: read from behind.
                let a = (i + 2 * MAP_W - whole) % MAP_W;
                let b = (a + MAP_W - 1) % MAP_W;
                *out = (line[a] * (1.0 - frac) + line[b] * frac) * fade;
            }
            for (i, v) in line.iter_mut().enumerate() {
                *v = row_buf[i] + deposit[row * MAP_W + i];
                peak = peak.max(*v);
            }
        }
        sim.heat_left = peak > 1e-3;
    }
    sim.scratch = deposit;
    sim.debris = debris;
}

/// Boxes a map along each row (wrapping around) and then down the rows.
fn blur(src: &[f32], dst: &mut [f32], tmp: &mut [f32], across: usize, down: usize) {
    let w = MAP_W;
    let norm = 1.0 / (2 * across + 1) as f32;
    for row in 0..MAP_H {
        let line = &src[row * w..(row + 1) * w];
        let mut sum: f32 = (0..=2 * across).map(|k| line[(k + w - across) % w]).sum();
        for i in 0..w {
            tmp[row * w + i] = sum * norm;
            sum += line[(i + across + 1) % w] - line[(i + w - across) % w];
        }
    }
    for row in 0..MAP_H {
        let lo = row.saturating_sub(down);
        let hi = (row + down).min(MAP_H - 1);
        let norm = 1.0 / (hi - lo + 1) as f32;
        for i in 0..w {
            let mut sum = 0.0;
            for k in lo..=hi {
                sum += tmp[k * w + i];
            }
            dst[row * w + i] = sum * norm;
        }
    }
}

/// Turns this frame's particles into the texture the tracer reads.
fn build_map(sim: &mut Sim, dt: f32) {
    let w = MAP_W;
    // Fewer particles: a wider blur, so a low count still looks like gas.
    let sparse = (160_000.0 / sim.active as f32).sqrt();
    let across = (2.0 * sparse).round().clamp(1.0, 7.0) as usize;
    let down = usize::from(sparse > 1.2) + 1;
    let mut tmp = vec![0.0f32; w * MAP_H];
    let mut blurred = vec![0.0f32; w * MAP_H];
    blur(&sim.gas, &mut blurred, &mut tmp, across, down);
    // Relative to the average around each radius (smoothed over nearby
    // radii), so the map holds the gas's structure and the tracer its
    // brightness by radius.
    let mut means = [0.0f32; MAP_H];
    for (row, mean) in means.iter_mut().enumerate() {
        *mean = blurred[row * w..(row + 1) * w].iter().sum::<f32>() / w as f32;
    }
    let keep = (-dt / 0.05).exp().min(0.6);
    for row in 0..MAP_H {
        let lo = row.saturating_sub(6);
        let hi = (row + 6).min(MAP_H - 1);
        let mean = means[lo..=hi].iter().sum::<f32>() / (hi - lo + 1) as f32;
        let inv = if mean > 1e-6 { 1.0 / mean } else { 0.0 };
        for i in 0..w {
            let at = row * w + i;
            let now = blurred[at] * inv;
            sim.smooth[at] = sim.smooth[at] * keep + now * (1.0 - keep);
        }
    }
    let mut debris = vec![0.0f32; w * MAP_H];
    blur(&sim.debris, &mut debris, &mut tmp, 1, 1);
    for at in 0..w * MAP_H {
        let gas = (sim.smooth[at] * 0.25).clamp(0.0, 1.0);
        let hot = ((debris[at] * 0.06 + sim.heat[at] * 0.05).sqrt()).clamp(0.0, 1.0);
        sim.texels[at * 4] = sim.encode[(gas * 4096.0) as usize];
        sim.texels[at * 4 + 1] = sim.encode[(hot * 4096.0) as usize];
    }
}

/// The tracer's values: its step size, the time, the flare and the stars.
fn update_tracer(ctx: &mut Ctx, sim: &mut Sim) {
    let view = 2.0 * (ctx.scene3d.camera.fov_y * 0.5).tan();
    // Smaller traces (slower GPUs) also take longer steps.
    let quality = if sim.scale >= 0.8 { 0.085 } else { 0.11 };
    let mut desc = sim.desc;
    desc.custom[0] = vec4(quality, ctx.time.elapsed() as f32, sim.flare, view);
    for (i, star) in sim.stars.iter().enumerate() {
        let intact = (1.0 - (star.spread - 0.7) / 1.0).clamp(0.0, 1.0);
        let born = (star.age / 0.2).clamp(0.0, 1.0);
        let shine = if star.alive > 0 {
            intact * intact * born * 4.0
        } else {
            0.0
        };
        desc.custom[1 + i] = star.center.extend(shine);
    }
    ctx.gfx.update_material(sim.material, desc);
}

/// Back off promptly on repeated missed refreshes. Probe upward in
/// small steps after a clean stretch, and remember the failed range.
/// When the CPU has time to spare, the GPU is what's slow: then the
/// trace's resolution comes down instead of the particle count.
fn adapt(ctx: &mut Ctx, sim: &mut Sim, elapsed: f32, sim_ms: f32) {
    if sim.fixed_count {
        return;
    }
    let interval = sim.refresh;
    if sim.stable_time < 0.0 {
        sim.stable_time = (sim.stable_time + elapsed.min(interval)).min(0.0);
        return;
    }
    let over_budget = (elapsed / interval - 1.035).clamp(-0.25, 2.0);
    sim.frame_debt = (sim.frame_debt * 0.985 + over_budget).max(0.0);
    if sim.frame_debt >= 1.9 || sim.sim_ms > interval * 650.0 {
        let cpu_bound = sim.sim_ms > interval * 450.0;
        if !cpu_bound && sim.scale > 0.36 {
            sim.scale = (sim.scale * 0.85).max(0.35);
            ctx.scene3d.post.render_scale = sim.scale;
        } else if sim.active > MIN_DISK {
            let next = (sim.active * 3 / 4).max(MIN_DISK);
            sim.ceiling = sim.ceiling.min(sim.active * 17 / 20).max(next);
            sim.active = next;
        }
        sim.frame_debt = 0.0;
        sim.stable_time = -0.7;
    } else if elapsed <= interval * 1.12 && sim_ms < interval * 400.0 {
        sim.stable_time += elapsed;
        if sim.stable_time > 1.35 && sim.active < sim.ceiling {
            let next = (sim.active + (sim.active / 10).clamp(2_000, 8_000)).min(sim.ceiling);
            grow(ctx, sim, next, true);
            sim.stable_time = 0.0;
        }
    } else {
        sim.stable_time = 0.0;
    }
}

/// Runs the demo.
pub fn run() {
    let asked = ASKED.with(|a| a.get());
    #[cfg(not(target_arch = "wasm32"))]
    let asked = std::env::var("PARTICLES")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(asked);
    Game::new("Hole · ponko", 1280, 720)
        .system(camera)
        .system(simulate)
        .run(move |ctx| setup(ctx, asked));
}

#[cfg(target_arch = "wasm32")]
mod web {
    use super::*;
    use wasm_bindgen::prelude::*;

    /// Starts with this many disk particles (0: the default) and the
    /// trace drawn at `scale` of the window; both then adapt to the machine.
    #[wasm_bindgen]
    pub fn start(particles: u32, refresh_ms: f32, fixed_count: bool, scale: f32) {
        ASKED.with(|a| a.set(particles as usize));
        REFRESH.with(|r| r.set(refresh_ms.clamp(4.0, 35.0) * 0.001));
        FIXED_COUNT.with(|f| f.set(fixed_count));
        if scale.is_finite() {
            SCALE.with(|s| s.set(scale.clamp(0.35, 1.0)));
        }
        run();
    }

    /// A drag: the pointer moved this far, in canvas heights.
    #[wasm_bindgen]
    pub fn drag(dx: f32, dy: f32) {
        DRAG.with(|d| d.set(d.get() + vec2(dx, dy)));
    }

    /// Zoom by this factor (below 1: closer).
    #[wasm_bindgen]
    pub fn zoom(factor: f32) {
        if factor.is_finite() && factor > 0.0 {
            ZOOM.with(|z| z.set(z.get() * factor));
        }
    }

    /// A click or tap at x and y, 0 to 1 across the canvas: throw a star.
    #[wasm_bindgen]
    pub fn throw_star(x: f32, y: f32) {
        THROWS.with(|t| t.borrow_mut().push(vec2(x, y)));
    }

    /// Test pages only: swaps in another version of the tracer's shader.
    #[wasm_bindgen]
    pub fn shader(source: String) {
        SHADER.with(|s| *s.borrow_mut() = Some(source));
    }

    /// Holds the simulation still (the page is hidden).
    #[wasm_bindgen]
    pub fn pause(paused: bool) {
        PAUSED.with(|p| p.set(paused));
    }

    /// [particles, fps, simulation ms, frames drawn, problem (always 0),
    ///  trace resolution scale]
    #[wasm_bindgen]
    pub fn stats() -> Vec<f32> {
        let h = HUD.with(|h| h.get());
        vec![
            h.particles as f32,
            h.fps,
            h.sim_ms,
            h.frames as f32,
            0.0,
            h.scale,
        ]
    }
}
