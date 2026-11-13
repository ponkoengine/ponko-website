//! Tide: hold the core against waves of thousands of units.
//!
//! Units march in through the open gates and follow a flow field to the
//! core in the middle. Towers zap the units near them with chain lightning
//! and bend their path: every placement rebuilds the field at once, and
//! every unit on the board re-routes in the same frame. A placement that
//! would seal off part of the board is refused.
//!
//! Every unit is an ECS entity drawn with one instanced mesh. Units live in
//! a fixed pool, so spawning and dying never allocate entities. Direction,
//! crowding and targeting read per-cell maps, which keeps the frame cost
//! linear in the number of units on the single thread WebAssembly gets.
//!
//! The page passes pointer input in through the exported functions at the
//! bottom (mouse and touch alike). Keys: Enter starts, Space overcharges.

use ponko::d3::*;
use ponko::perf::Instant;
use ponko::prelude::*;
use std::cell::{Cell, RefCell};

/// Cells per side of the board; one cell is one meter.
const N: usize = 26;
const HALF: f32 = N as f32 / 2.0;
/// The core's cells: a 2 x 2 block in the middle.
const CORE: [usize; 2] = [N / 2 - 1, N / 2];
/// Where the gates are along each edge.
const GATE: std::ops::Range<usize> = (N / 2 - 3)..(N / 2 + 3);
const TOWER_RANGE: f32 = 2.6;
/// Damage per second to each unit a tower's lightning reaches.
const TOWER_DPS: f32 = 3.2;
/// Units one tower's chain reaches (twice that, overcharged).
const CHAIN: usize = 3;
const CORE_HP: f32 = 30.0;
const START_TOWERS: u32 = 12;
const OVERCHARGE_TIME: f32 = 5.0;
const OVERCHARGE_COOLDOWN: f32 = 22.0;
const CALM: f32 = 7.0;
const SPARKS: usize = 40;
const UNREACHABLE: u16 = u16::MAX;
/// Hidden units wait here, under the floor and out of view.
const PARKED: Vec3 = vec3(0.0, -60.0, 0.0);

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
    Blocked = 1,
    NoTowers = 2,
    WaveCleared = 3,
    Leak = 4,
    WaveStarted = 5,
    Overcharged = 6,
    Built = 7,
}

#[derive(Clone, Copy, Default)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
struct Hud {
    phase: u8,
    kills: u32,
    best: u32,
    core: f32,
    towers: u32,
    alive: u32,
    sim_ms: f32,
    fps: f32,
    wave: u32,
    attacking: bool,
    countdown: f32,
    gates: u8,
    overcharge: f32,
    overcharged: bool,
}

thread_local! {
    /// The pointer over the canvas, 0..1 across and down; `None` off it.
    static HOVER: Cell<Option<Vec2>> = const { Cell::new(None) };
    static TAPS: RefCell<Vec<Vec2>> = const { RefCell::new(Vec::new()) };
    static START: Cell<bool> = const { Cell::new(false) };
    static OVERCHARGE: Cell<bool> = const { Cell::new(false) };
    static EVENTS: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static HUD: Cell<Hud> = Cell::new(Hud::default());
    static POOL_SIZE: Cell<usize> = const { Cell::new(0) };
}

fn emit(e: Event) {
    EVENTS.with(|v| {
        let mut v = v.borrow_mut();
        if v.len() < 64 {
            v.push(e as u8);
        }
    });
}

fn cell_center(c: usize) -> Vec3 {
    vec3(
        (c % N) as f32 + 0.5 - HALF,
        0.0,
        (c / N) as f32 + 0.5 - HALF,
    )
}

fn cell_at(p: Vec2) -> Option<usize> {
    let (x, z) = (p.x + HALF, p.y + HALF);
    (x >= 0.0 && z >= 0.0 && x < N as f32 && z < N as f32).then(|| z as usize * N + x as usize)
}

fn is_core(c: usize) -> bool {
    CORE.contains(&(c % N)) && CORE.contains(&(c / N))
}

/// The edge cells of gate `g`: 0 north (far side), 1 south, 2 west, 3 east.
fn gate_cells(g: usize) -> [usize; 6] {
    let mut out = [0; 6];
    for (k, i) in GATE.enumerate() {
        out[k] = match g {
            0 => i,
            1 => (N - 1) * N + i,
            2 => i * N,
            _ => i * N + N - 1,
        };
    }
    out
}

/// The board: towers, and the maps units read every frame.
struct Board {
    wall: Vec<bool>,
    /// Steps to the core (4-connected), `UNREACHABLE` for walls.
    dist: Vec<u16>,
    /// Which way to go from each cell.
    dir: Vec<Vec2>,
    /// The tower entities on each cell: pillar, cap, lightning.
    towers: Vec<Option<[Entity; 3]>>,
    reachable: usize,
    /// Bumped whenever the towers change.
    version: u32,
}

impl Board {
    fn new() -> Self {
        let mut b = Board {
            wall: vec![false; N * N],
            dist: vec![0; N * N],
            dir: vec![Vec2::ZERO; N * N],
            towers: vec![None; N * N],
            reachable: 0,
            version: 0,
        };
        b.rebuild();
        b
    }

    /// Breadth-first distances from the core; returns how many cells it
    /// reached. With `keep`, stores them in `dist`.
    fn flood(&mut self, wall: &[bool], keep: bool) -> usize {
        let mut dist = vec![UNREACHABLE; N * N];
        let mut queue = std::collections::VecDeque::new();
        for (c, d) in dist.iter_mut().enumerate() {
            if is_core(c) {
                *d = 0;
                queue.push_back(c);
            }
        }
        let mut reached = 0;
        while let Some(c) = queue.pop_front() {
            reached += 1;
            let (x, z) = (c % N, c / N);
            let mut visit = |n: usize| {
                if !wall[n] && dist[n] == UNREACHABLE {
                    dist[n] = dist[c] + 1;
                    queue.push_back(n);
                }
            };
            if x > 0 {
                visit(c - 1);
            }
            if x + 1 < N {
                visit(c + 1);
            }
            if z > 0 {
                visit(c - N);
            }
            if z + 1 < N {
                visit(c + N);
            }
        }
        if keep {
            self.dist = dist;
        }
        reached
    }

    /// Why a tower can't go on `c`, if it can't.
    fn refusal(&mut self, c: usize) -> Option<Event> {
        if self.wall[c] || is_core(c) || (0..4).any(|g| gate_cells(g).contains(&c)) {
            return Some(Event::Blocked);
        }
        let mut wall = self.wall.clone();
        wall[c] = true;
        (self.flood(&wall, false) != self.reachable - 1).then_some(Event::Blocked)
    }

    /// Distances and directions, after the towers changed.
    fn rebuild(&mut self) {
        self.version += 1;
        self.reachable = self.flood(&self.wall.clone(), true);
        for c in 0..N * N {
            let (x, z) = ((c % N) as i32, (c / N) as i32);
            let open = |x: i32, z: i32| {
                (0..N as i32).contains(&x)
                    && (0..N as i32).contains(&z)
                    && !self.wall[z as usize * N + x as usize]
            };
            let mut best = (
                if self.wall[c] {
                    u32::MAX
                } else {
                    self.dist[c] as u32 * 2
                },
                Vec2::ZERO,
            );
            for (dx, dz) in [
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ] {
                let (nx, nz) = (x + dx, z + dz);
                if !open(nx, nz) {
                    continue;
                }
                // No cutting corners past a tower.
                if dx != 0 && dz != 0 && !(open(x + dx, z) && open(x, z + dz)) {
                    continue;
                }
                let d = self.dist[nz as usize * N + nx as usize] as u32 * 2
                    + (dx != 0 && dz != 0) as u32;
                if d < best.0 {
                    best = (d, vec2(dx as f32, dz as f32).normalize());
                }
            }
            self.dir[c] = best.1;
        }
    }

    /// The path a unit takes from the middle of gate `g` to the core.
    fn route(&self, g: usize) -> Vec<Vec3> {
        let mut c = gate_cells(g)[2];
        let mut points = vec![cell_center(c) + Vec3::Y * 0.05];
        for _ in 0..N * N {
            if self.dist[c] == 0 || self.dist[c] == UNREACHABLE {
                break;
            }
            let d = self.dir[c];
            let (x, z) = (
                (c % N) as i32 + d.x.round() as i32,
                (c / N) as i32 + d.y.round() as i32,
            );
            if !(0..N as i32).contains(&x) || !(0..N as i32).contains(&z) {
                break;
            }
            c = z as usize * N + x as usize;
            points.push(cell_center(c) + Vec3::Y * 0.05);
        }
        points
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Grunt,
    Runner,
    Brute,
}

impl Kind {
    fn speed(self) -> f32 {
        match self {
            Kind::Grunt => 1.6,
            Kind::Runner => 2.8,
            Kind::Brute => 1.0,
        }
    }
    fn hp(self) -> f32 {
        match self {
            Kind::Grunt => 1.0,
            Kind::Runner => 0.6,
            Kind::Brute => 7.0,
        }
    }
    fn size(self) -> f32 {
        match self {
            Kind::Grunt => 1.0,
            Kind::Runner => 0.75,
            Kind::Brute => 1.9,
        }
    }
    fn damage(self) -> f32 {
        match self {
            Kind::Brute => 4.0,
            _ => 1.0,
        }
    }
    fn color(self) -> Vec3 {
        match self {
            Kind::Grunt => vec3(1.0, 0.36, 0.28),
            Kind::Runner => vec3(1.1, 0.75, 0.2),
            Kind::Brute => vec3(1.0, 0.25, 0.65),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Life {
    Idle,
    Alive,
    Dying,
}

#[derive(Component)]
struct Unit {
    life: Life,
    kind: Kind,
    velocity: Vec2,
    hp: f32,
    max_hp: f32,
    timer: f32,
    seed: f32,
    /// Set by the towers this frame, for the glow.
    zapped: bool,
}

#[derive(Component)]
struct Rising(f32);
#[derive(Component)]
struct Core;
#[derive(Component)]
struct Ghost;
#[derive(Component)]
struct GhostRing;
#[derive(Component)]
struct Spark(usize);
#[derive(Component)]
struct Route(usize);
#[derive(Component)]
struct GateMark(usize);

#[derive(Clone, Copy)]
struct Look {
    pillar: Mesh,
    cap: Mesh,
    cap_material: Material,
    cap_desc_strength: f32,
}

/// The alive units in each cell this frame, for targeting and crowding.
#[derive(Default)]
struct Crowd {
    cells: Vec<Vec<(Entity, Vec3)>>,
    /// Where units died this frame, for the sparks.
    deaths: Vec<(Vec3, Kind)>,
    leaks: Vec<Vec3>,
}

struct State {
    phase: Phase,
    time: f32,
    kills: u32,
    best: u32,
    core: f32,
    towers: u32,
    wave: u32,
    attacking: bool,
    /// Seconds of calm left before the next wave.
    calm: f32,
    /// Units still to send this wave, and how fast.
    to_send: u32,
    send_rate: f32,
    send_debt: f32,
    gates: u8,
    next_gates: u8,
    hp_scale: f32,
    runners: f32,
    brutes: f32,
    overcharge: f32,
    overcharge_cooldown: f32,
    hit_flash: f32,
    sim_ms: f32,
    alive: u32,
    next_spark: usize,
    rng: Rng,
}

impl State {
    fn new() -> Self {
        State {
            phase: Phase::Title,
            time: 0.0,
            kills: 0,
            best: 0,
            core: CORE_HP,
            towers: START_TOWERS,
            wave: 0,
            attacking: false,
            calm: CALM,
            to_send: 0,
            send_rate: 0.0,
            send_debt: 0.0,
            gates: 0b1111,
            next_gates: 0b0001,
            hp_scale: 1.0,
            runners: 0.0,
            brutes: 0.0,
            overcharge: 0.0,
            overcharge_cooldown: 0.0,
            hit_flash: 0.0,
            sim_ms: 0.0,
            alive: 0,
            next_spark: 0,
            rng: Rng::new(17),
        }
    }

    /// Picks the gates for the wave after `wave`: one at first, then more.
    fn pick_gates(&mut self, wave: u32) -> u8 {
        let open = match wave {
            0 => 1,
            1 => 2,
            w if w % 4 == 3 => 4,
            _ => 2 + self.rng.below(2) as usize,
        };
        let mut order = [0usize, 1, 2, 3];
        for i in (1..4).rev() {
            order.swap(i, self.rng.below(i as u32 + 1) as usize);
        }
        order[..open].iter().fold(0, |m, g| m | (1 << g))
    }

    fn start_wave(&mut self) {
        self.wave += 1;
        let n = self.wave as f32;
        self.attacking = true;
        self.gates = self.next_gates;
        self.to_send = (30.0 + n * 26.0 + n * n * 9.0) as u32;
        self.send_rate = self.to_send as f32 / (8.0 + n * 0.8);
        self.hp_scale = 1.0 + (n - 1.0) * 0.22;
        self.runners = if self.wave >= 3 { 0.25 } else { 0.0 };
        self.brutes = if self.wave >= 5 {
            0.03 + n * 0.004
        } else {
            0.0
        };
        emit(Event::WaveStarted);
    }
}

fn place_tower(ctx: &mut Ctx, c: usize) {
    let look = *ctx.world.resource::<Look>();
    let at = cell_center(c);
    let pillar = ctx.world.spawn((
        Transform3D {
            scale: vec3(0.8, 0.001, 0.8),
            ..Transform3D::at(at)
        },
        Model::new(look.pillar),
        Rising(0.0),
    ));
    let cap = ctx.world.spawn((
        Transform3D {
            scale: vec3(0.46, 0.08, 0.46),
            ..Transform3D::at(at + Vec3::Y * 0.02)
        },
        Model::new(look.cap),
        Rising(0.0),
    ));
    let bolt = ctx.world.spawn((
        Transform3D::default(),
        Line {
            blend: ParticleBlend::Additive,
            ..Line::new(Vec::new(), 0.06, Color::WHITE)
        },
    ));
    let board = ctx.world.resource_mut::<Board>();
    board.wall[c] = true;
    board.towers[c] = Some([pillar, cap, bolt]);
}

fn remove_tower(ctx: &mut Ctx, c: usize) {
    let board = ctx.world.resource_mut::<Board>();
    board.wall[c] = false;
    if let Some(parts) = board.towers[c].take() {
        for e in parts {
            ctx.world.despawn(e);
        }
    }
}

/// A starter maze: two broken rings that make the flow worth watching.
fn starter_layout(ctx: &mut Ctx) {
    let mut cells = Vec::new();
    for i in [6, 7, 9, 10, 15, 16, 18, 19] {
        cells.push(7 * N + i);
        cells.push(18 * N + i);
        cells.push(i * N + 7);
        cells.push(i * N + 18);
    }
    for c in cells {
        if ctx.world.resource_mut::<Board>().refusal(c).is_none() {
            place_tower(ctx, c);
            ctx.world.resource_mut::<Board>().rebuild();
        }
    }
}

fn reset(ctx: &mut Ctx) {
    let towers: Vec<usize> = (0..N * N)
        .filter(|&c| ctx.world.resource::<Board>().wall[c])
        .collect();
    for c in towers {
        remove_tower(ctx, c);
    }
    ctx.world.resource_mut::<Board>().rebuild();
    ctx.world
        .query::<(&mut Transform3D, &mut Unit)>()
        .for_each(|(t, u)| {
            u.life = Life::Idle;
            t.position = PARKED;
        });
    let best = ctx.world.resource::<State>().best;
    let s = ctx.world.resource_mut::<State>();
    *s = State::new();
    s.best = best;
    s.phase = Phase::Playing;
    s.gates = 0;
    s.calm = 8.0;
    s.next_gates = s.pick_gates(0);
}

/// Which cell the page's pointer is over.
fn pointed_cell(ctx: &Ctx, at: Vec2) -> Option<usize> {
    let size = ctx.window_size();
    let (from, dir) = ctx.scene3d.camera.screen_ray(at * size);
    if dir.y >= -1e-4 {
        return None;
    }
    let hit = from + dir * (-from.y / dir.y);
    cell_at(vec2(hit.x, hit.z))
}

/// How far back the camera must sit to see the whole board (with room for
/// the page's HUD at the top and bottom).
fn fit_distance(target: Vec3, aspect: f32, fov_y: f32) -> f32 {
    let dir = vec3(0.0, 0.84, 0.54).normalize();
    let forward = -dir;
    let right = forward.cross(Vec3::Y).normalize();
    let up = right.cross(forward);
    let (ty, tx) = (
        (fov_y * 0.5).tan() * 0.86,
        (fov_y * 0.5).tan() * aspect * 0.94,
    );
    let e = HALF + 0.3;
    let corners = [
        vec3(-e, 0.0, -e),
        vec3(e, 0.0, -e),
        vec3(-e, 0.0, e),
        vec3(e, 0.0, e),
    ];
    let mut d = 15.0;
    while d < 200.0 {
        let eye = target + dir * d;
        let fits = corners.iter().all(|&c| {
            let v = c - eye;
            let z = v.dot(forward);
            (v.dot(right) / z).abs() < tx && (v.dot(up) / z).abs() < ty
        });
        if fits {
            return d;
        }
        d += 0.5;
    }
    d
}

/// Starting, building, overcharging and the fixed camera.
fn control(ctx: &mut Ctx) {
    let dt = ctx.dt().min(0.05);
    // The whole board in view, whatever the window's shape; the fog keeps
    // its distance from the camera so a tall phone screen isn't fogged out.
    let size = ctx.window_size();
    let aspect = (size.x / size.y.max(1.0)).max(0.3);
    let target = vec3(0.0, 0.0, 1.0);
    let distance = fit_distance(target, aspect, ctx.scene3d.camera.fov_y);
    ctx.scene3d.camera.position = target + vec3(0.0, 0.84, 0.54).normalize() * distance;
    ctx.scene3d.camera.look_at(target);
    ctx.scene3d.sky.fog_start = distance + 8.0;
    ctx.scene3d.sky.fog_end = distance + 60.0;

    let start = START.with(|s| s.replace(false)) || ctx.key_pressed(Key::Enter);
    let overcharge = OVERCHARGE.with(|s| s.replace(false)) || ctx.key_pressed(Key::Space);
    let taps: Vec<Vec2> = TAPS.with(|t| std::mem::take(&mut *t.borrow_mut()));
    let phase = ctx.world.resource::<State>().phase;
    if (start || (overcharge && phase == Phase::Over)) && phase != Phase::Playing {
        reset(ctx);
        return;
    }
    if phase == Phase::Playing {
        let s = ctx.world.resource_mut::<State>();
        if overcharge && s.overcharge_cooldown <= 0.0 {
            s.overcharge = OVERCHARGE_TIME;
            s.overcharge_cooldown = OVERCHARGE_COOLDOWN;
            emit(Event::Overcharged);
        }
        for tap in taps {
            let Some(c) = pointed_cell(ctx, tap) else {
                continue;
            };
            let towers = ctx.world.resource::<State>().towers;
            let board = ctx.world.resource_mut::<Board>();
            if board.wall[c] {
                remove_tower(ctx, c);
                ctx.world.resource_mut::<Board>().rebuild();
                ctx.world.resource_mut::<State>().towers += 1;
            } else if let Some(why) = board.refusal(c) {
                if !is_core(c) {
                    emit(why);
                }
            } else if towers == 0 {
                emit(Event::NoTowers);
            } else {
                place_tower(ctx, c);
                ctx.world.resource_mut::<Board>().rebuild();
                ctx.world.resource_mut::<State>().towers -= 1;
                emit(Event::Built);
            }
        }
    }

    // The ghost square under the pointer: green to build, red if refused,
    // with the range a tower there would reach.
    let hover = HOVER.with(|h| h.get()).and_then(|p| pointed_cell(ctx, p));
    let can = match hover {
        Some(c) if phase == Phase::Playing => {
            let towers = ctx.world.resource::<State>().towers;
            let board = ctx.world.resource_mut::<Board>();
            Some(board.wall[c] || (towers > 0 && board.refusal(c).is_none()))
        }
        _ => None,
    };
    for (t, line, _) in ctx
        .world
        .query::<(&mut Transform3D, &mut Line, &Ghost)>()
        .iter()
    {
        match (hover, can) {
            (Some(c), Some(ok)) => {
                t.position = cell_center(c) + Vec3::Y * 0.04;
                t.scale = Vec3::ONE;
                line.color = if ok {
                    Color::rgba(0.9, 1.4, 0.5, 0.9)
                } else {
                    Color::rgba(1.6, 0.4, 0.35, 0.9)
                };
            }
            _ => t.scale = Vec3::ZERO,
        }
    }
    for (t, _) in ctx.world.query::<(&mut Transform3D, &GhostRing)>().iter() {
        match (hover, can) {
            (Some(c), Some(true)) => {
                t.position = cell_center(c) + Vec3::Y * 0.04;
                t.scale = Vec3::splat(TOWER_RANGE);
            }
            _ => t.scale = Vec3::ZERO,
        }
    }

    // New towers grow out of the floor.
    for (t, r) in ctx.world.query::<(&mut Transform3D, &mut Rising)>().iter() {
        r.0 = (r.0 + dt * 5.0).min(1.0);
        let k = 1.0 - (1.0 - r.0).powi(3);
        if t.scale.x > 0.6 {
            t.scale.y = 0.95 * k;
            t.position.y = 0.475 * k;
        } else {
            t.position.y = 0.95 * k + 0.04;
        }
    }
}

/// Waves: calm, announce the gates, send the units, repeat.
fn waves(ctx: &mut Ctx) {
    let dt = ctx.dt().min(0.05);
    let s = ctx.world.resource_mut::<State>();
    s.hit_flash = (s.hit_flash - dt * 2.0).max(0.0);
    s.overcharge = (s.overcharge - dt).max(0.0);
    s.overcharge_cooldown = (s.overcharge_cooldown - dt).max(0.0);
    match s.phase {
        Phase::Title => {
            // The attract loop: a steady stream from every gate.
            s.gates = 0b1111;
            s.attacking = true;
            s.to_send = u32::MAX;
            s.send_rate = 220.0;
            s.hp_scale = 2.4;
            s.runners = 0.2;
            s.brutes = 0.02;
        }
        Phase::Over => {
            s.to_send = 0;
        }
        Phase::Playing => {
            s.time += dt;
            if s.attacking {
                if s.to_send == 0 && s.alive == 0 {
                    s.attacking = false;
                    s.calm = CALM;
                    s.towers += 2;
                    s.next_gates = s.pick_gates(s.wave);
                    emit(Event::WaveCleared);
                }
            } else {
                s.calm -= dt;
                if s.calm <= 0.0 {
                    s.start_wave();
                }
            }
        }
    }
}

/// Spawning, moving and scoring every unit; fills the crowd map.
fn tide(ctx: &mut Ctx) {
    let started = Instant::now();
    let dt = ctx.dt().min(0.05);
    let now = ctx.time.elapsed() as f32;
    let s = ctx.world.resource_mut::<State>();
    let playing = s.phase == Phase::Playing;
    let mut to_spawn = 0u32;
    if s.attacking && s.to_send > 0 {
        s.send_debt += s.send_rate * dt;
        to_spawn = (s.send_debt as u32).min(s.to_send);
        s.send_debt -= to_spawn as f32;
        if s.to_send != u32::MAX {
            s.to_send -= to_spawn;
        }
    }
    let (gates, hp_scale, runners, brutes) = (s.gates, s.hp_scale, s.runners, s.brutes);
    let gate_list: Vec<usize> = (0..4).filter(|g| gates & (1 << g) != 0).collect();
    let mut rng = Rng::new(ctx.time.frame() ^ 0x5EED_1234);

    // A copy of the board's maps (a few kilobytes), so the query can borrow the world.
    let board = ctx.world.resource::<Board>();
    let (dir, wall, dist) = (board.dir.clone(), board.wall.clone(), board.dist.clone());
    let mut crowd = std::mem::take(ctx.world.resource_mut::<Crowd>());
    let mut density = vec![0u16; N * N];
    for (c, cell) in crowd.cells.iter().enumerate() {
        density[c] = cell.len().min(u16::MAX as usize) as u16;
    }
    for cell in &mut crowd.cells {
        cell.clear();
    }
    crowd.deaths.clear();
    crowd.leaks.clear();

    let (mut alive, mut leaked) = (0u32, 0.0f32);
    let mut kills = 0u32;
    ctx.world
        .query::<(Entity, &mut Transform3D, &mut Model, &mut Unit)>()
        .for_each(|(e, tr, model, u)| match u.life {
            Life::Idle => {
                if to_spawn == 0 || gate_list.is_empty() {
                    return;
                }
                to_spawn -= 1;
                let g = gate_list[rng.below(gate_list.len() as u32) as usize];
                let cell = gate_cells(g)[rng.below(6) as usize];
                let roll = rng.f32();
                let kind = if roll < brutes {
                    Kind::Brute
                } else if roll < brutes + runners {
                    Kind::Runner
                } else {
                    Kind::Grunt
                };
                let hp = kind.hp() * hp_scale;
                *u = Unit {
                    life: Life::Alive,
                    kind,
                    velocity: Vec2::ZERO,
                    hp,
                    max_hp: hp,
                    timer: 0.0,
                    seed: rng.f32(),
                    zapped: false,
                };
                tr.position =
                    cell_center(cell) + vec3(rng.range(-0.4, 0.4), 0.0, rng.range(-0.4, 0.4));
                tr.scale = Vec3::splat(kind.size());
            }
            Life::Dying => {
                u.timer -= dt;
                let k = (u.timer / 0.25).max(0.0);
                tr.scale = Vec3::splat(u.kind.size() * (0.2 + k * 1.3));
                tr.position.y += dt * 2.0;
                model.color = Color::rgb(3.0, 2.6, 1.8);
                if u.timer <= 0.0 {
                    u.life = Life::Idle;
                    tr.position = PARKED;
                }
            }
            Life::Alive => {
                if u.hp <= 0.0 {
                    kills += 1;
                    u.life = Life::Dying;
                    u.timer = 0.25;
                    crowd.deaths.push((tr.position, u.kind));
                    return;
                }
                let p = vec2(tr.position.x, tr.position.z);
                let Some(c) = cell_at(p) else {
                    u.life = Life::Idle;
                    tr.position = PARKED;
                    return;
                };
                if dist[c] == 0 {
                    leaked += u.kind.damage();
                    crowd.leaks.push(tr.position);
                    u.life = Life::Idle;
                    tr.position = PARKED;
                    return;
                }
                alive += 1;
                crowd.cells[c].push((e, tr.position));
                // Head down the field, spread out where it's crowded.
                let (x, z) = (c % N, c / N);
                let count = |dx: i32, dz: i32| {
                    let (nx, nz) = (x as i32 + dx, z as i32 + dz);
                    if (0..N as i32).contains(&nx) && (0..N as i32).contains(&nz) {
                        density[nz as usize * N + nx as usize] as f32
                    } else {
                        0.0
                    }
                };
                let spread = vec2(count(-1, 0) - count(1, 0), count(0, -1) - count(0, 1)) * 0.06;
                let speed = u.kind.speed() * (0.85 + u.seed * 0.3);
                let want = dir[c] * speed + spread.clamp_length_max(1.0);
                u.velocity += (want - u.velocity) * (1.0 - (-6.0 * dt).exp());
                let step = u.velocity * dt;
                // Slide along towers instead of walking through them.
                let free = |q: Vec2| cell_at(q).is_some_and(|n| !wall[n]);
                let mut next = p + step;
                if !wall[c] && !free(next) {
                    next = if free(vec2(p.x + step.x, p.y)) {
                        vec2(p.x + step.x, p.y)
                    } else if free(vec2(p.x, p.y + step.y)) {
                        vec2(p.x, p.y + step.y)
                    } else {
                        p
                    };
                }
                let size = u.kind.size();
                let hop = (now * (7.0 + u.kind.speed() * 2.0) + u.seed * 40.0)
                    .sin()
                    .abs()
                    * 0.08
                    * size;
                tr.position = vec3(next.x, 0.12 * size + hop, next.y);
                // Their own color while healthy, white-hot while being zapped.
                let base = u.kind.color();
                let health = (u.hp / u.max_hp).clamp(0.0, 1.0);
                let heat = if u.zapped { 1.0 } else { 0.0 } + (1.0 - health) * 0.6;
                let c = base + vec3(1.6, 1.4, 1.0) * heat;
                model.color = Color::rgb(c.x, c.y, c.z);
                u.zapped = false;
            }
        });
    *ctx.world.resource_mut::<Crowd>() = crowd;

    let s = ctx.world.resource_mut::<State>();
    s.alive = alive;
    if playing {
        s.kills += kills;
        if leaked > 0.0 {
            s.core -= leaked;
            s.hit_flash = 1.0;
            emit(Event::Leak);
            if s.core <= 0.0 {
                s.core = 0.0;
                s.phase = Phase::Over;
                s.attacking = false;
                s.best = s.best.max(s.kills);
            }
        }
    }
    let ms = started.elapsed().as_secs_f32() * 1000.0;
    s.sim_ms += (ms - s.sim_ms) * 0.05;
}

/// Each tower's lightning jumps between the nearest units in its range.
fn towers(ctx: &mut Ctx) {
    let dt = ctx.dt().min(0.05);
    let now = ctx.time.elapsed() as f32;
    let s = ctx.world.resource::<State>();
    let over = s.overcharge > 0.0;
    let (chain, dps) = if over {
        (CHAIN * 2, TOWER_DPS * 2.0)
    } else {
        (CHAIN, TOWER_DPS)
    };
    let board = ctx.world.resource::<Board>();
    let towers: Vec<(usize, Entity)> = (0..N * N)
        .filter_map(|c| board.towers[c].map(|t| (c, t[2])))
        .collect();
    let crowd = std::mem::take(ctx.world.resource_mut::<Crowd>());
    let reach = TOWER_RANGE.ceil() as i32;
    let mut picked: Vec<(f32, Entity, Vec3)> = Vec::with_capacity(32);
    let mut hits: Vec<Entity> = Vec::with_capacity(towers.len() * chain);
    for (c, bolt) in towers {
        let at = cell_center(c);
        let (tx, tz) = ((c % N) as i32, (c / N) as i32);
        picked.clear();
        for z in (tz - reach).max(0)..=(tz + reach).min(N as i32 - 1) {
            for x in (tx - reach).max(0)..=(tx + reach).min(N as i32 - 1) {
                for &(e, p) in &crowd.cells[z as usize * N + x as usize] {
                    let d = vec2(p.x - at.x, p.z - at.z).length();
                    if d <= TOWER_RANGE {
                        picked.push((d, e, p));
                    }
                }
            }
        }
        picked.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
        picked.truncate(chain);
        let Some(line) = ctx.world.get_mut::<Line>(bolt) else {
            continue;
        };
        line.points.clear();
        if !picked.is_empty() {
            line.points.push(at + Vec3::Y * 1.05);
            for (i, &(_, _, p)) in picked.iter().enumerate() {
                // A little jitter, so the bolt crackles.
                let j = (now * 60.0 + i as f32 * 7.3 + c as f32).sin() * 0.12;
                let prev = *line.points.last().unwrap();
                let mid = (prev + p) * 0.5 + vec3(j, 0.15 + j.abs(), -j);
                line.points.push(mid);
                line.points.push(p + Vec3::Y * 0.1);
            }
            let flicker = 0.75 + 0.25 * (now * 40.0 + c as f32).sin();
            line.width = if over { 0.11 } else { 0.07 };
            line.color = if over {
                Color::rgba(2.6 * flicker, 2.6 * flicker, 2.6 * flicker, 1.0)
            } else {
                Color::rgba(1.4 * flicker, 2.4 * flicker, 0.8 * flicker, 1.0)
            };
        }
        hits.extend(picked.iter().map(|p| p.1));
    }
    *ctx.world.resource_mut::<Crowd>() = crowd;
    for e in hits {
        if let Some(u) = ctx.world.get_mut::<Unit>(e) {
            u.hp -= dps * dt;
            u.zapped = true;
        }
    }
}

/// Sparks, routes, gates, the core, the towers' glow and the HUD.
fn present(ctx: &mut Ctx) {
    let fps = ctx.time.fps();
    let now = ctx.time.elapsed() as f32;

    // Sparks where units died or broke through.
    let crowd = std::mem::take(ctx.world.resource_mut::<Crowd>());
    let mut next = ctx.world.resource::<State>().next_spark;
    let bursts: Vec<(Vec3, usize)> = crowd
        .deaths
        .iter()
        .take(SPARKS / 2)
        .map(|&(p, kind)| (p, if kind == Kind::Brute { 30 } else { 7 }))
        .chain(crowd.leaks.iter().take(4).map(|&p| (p, 24)))
        .collect();
    *ctx.world.resource_mut::<Crowd>() = crowd;
    if !bursts.is_empty() {
        let mut slots: [Option<(Vec3, usize)>; SPARKS] = [None; SPARKS];
        for burst in bursts {
            slots[next % SPARKS] = Some(burst);
            next += 1;
        }
        ctx.world
            .query::<(&mut Transform3D, &mut ParticleEmitter, &Spark)>()
            .for_each(|(t, emitter, spark)| {
                if let Some((p, count)) = slots[spark.0] {
                    t.position = p;
                    emitter.burst(count);
                }
            });
    }
    let s = ctx.world.resource_mut::<State>();
    s.next_spark = next % SPARKS;

    let s = ctx.world.resource::<State>();
    let (phase, flash, over) = (s.phase, s.hit_flash, s.overcharge > 0.0);
    // Gates: lit while open, blinking while announced for the next wave.
    let (open, soon) = match phase {
        Phase::Playing if s.attacking => (s.gates, 0),
        Phase::Playing => (0, s.next_gates),
        Phase::Title => (0b1111, 0),
        Phase::Over => (0, 0),
    };
    HUD.with(|h| {
        h.set(Hud {
            phase: phase as u8,
            kills: s.kills,
            best: s.best,
            core: s.core / CORE_HP,
            towers: s.towers,
            alive: s.alive,
            sim_ms: s.sim_ms,
            fps,
            wave: s.wave,
            attacking: s.attacking,
            countdown: s.calm.max(0.0),
            gates: if s.attacking { s.gates } else { s.next_gates },
            overcharge: 1.0 - s.overcharge_cooldown / OVERCHARGE_COOLDOWN,
            overcharged: s.overcharge > 0.0,
        })
    });
    let blink = 0.5 + 0.5 * (now * 6.0).sin();
    for (line, g) in ctx.world.query::<(&mut Line, &GateMark)>().iter() {
        let bit = 1 << g.0;
        line.color = if open & bit != 0 {
            Color::rgba(2.4, 0.7, 0.5, 1.0)
        } else if soon & bit != 0 {
            Color::rgba(
                0.6 + 1.8 * blink,
                0.25 + 0.4 * blink,
                0.2 + 0.3 * blink,
                1.0,
            )
        } else {
            Color::rgba(0.25, 0.3, 0.22, 0.6)
        };
    }

    // Each open (or announced) gate's route to the core.
    let show = open | soon;
    let board = ctx.world.resource::<Board>();
    let routes: Vec<Vec<Vec3>> = (0..4)
        .map(|g| {
            if show & (1 << g) != 0 {
                board.route(g)
            } else {
                Vec::new()
            }
        })
        .collect();
    for (line, r) in ctx.world.query::<(&mut Line, &Route)>().iter() {
        line.points.clone_from(&routes[r.0]);
        let a = if soon != 0 { 0.35 + 0.35 * blink } else { 0.28 };
        line.color = Color::rgba(1.6 * a, 0.5 * a, 0.35 * a, 1.0);
    }

    let now_y = 1.0 + (now * 1.6).sin() * 0.08;
    for (t, m, _) in ctx
        .world
        .query::<(&mut Transform3D, &mut Model, &Core)>()
        .iter()
    {
        t.rotation = Quat::from_rotation_y(now * 0.6) * Quat::from_rotation_x(0.6);
        t.position.y = now_y;
        t.scale = Vec3::splat(1.0 + flash * 0.25);
        m.color = Color::rgb(1.0 + flash * 1.4, 1.0 - flash * 0.7, 1.0 - flash * 0.8);
    }

    // Tower caps flare white while overcharged.
    let look = *ctx.world.resource::<Look>();
    let strength = if over { 9.0 } else { 3.0 };
    if (look.cap_desc_strength - strength).abs() > 0.01 {
        ctx.gfx.update_material(
            look.cap_material,
            cap_desc(
                if over {
                    Color::WHITE
                } else {
                    Color::hex(0xC0FA73)
                },
                strength,
            ),
        );
        ctx.world.resource_mut::<Look>().cap_desc_strength = strength;
    }
}

fn cap_desc(color: Color, strength: f32) -> MaterialDesc {
    let mut desc = MaterialDesc::color(color);
    desc.shader = GLOW_SHADER.with(|s| s.get());
    desc.custom = [
        Vec4::new(strength, 0.0, 0.0, 0.0),
        Vec4::ZERO,
        Vec4::ZERO,
        Vec4::ZERO,
    ];
    desc
}

thread_local! {
    static GLOW_SHADER: Cell<Option<Shader>> = const { Cell::new(None) };
}

const GLOW: &str = "
fn surface(s: Surface) -> Surface {
    var out = s;
    out.emissive = s.albedo * material.custom[0].x;
    out.albedo = s.albedo * 0.1;
    return out;
}";

const FLOOR: &str = "
fn surface(s: Surface) -> Surface {
    var out = s;
    let p = s.world.xz;
    let d = abs(fract(p + 0.5) - 0.5) / fwidth(p);
    let line = 1.0 - min(min(d.x, d.y), 1.0);
    let edge = max(abs(p.x), abs(p.y));
    let board = 1.0 - smoothstep(13.0, 13.2, edge);
    out.emissive = vec3<f32>(0.45, 0.62, 0.28) * line * 0.14 * board;
    out.albedo = s.albedo * (0.5 + 0.5 * board);
    return out;
}";

fn circle(segments: usize) -> Vec<Vec3> {
    (0..=segments)
        .map(|i| {
            let a = i as f32 / segments as f32 * std::f32::consts::TAU;
            vec3(a.cos(), 0.0, a.sin())
        })
        .collect()
}

fn setup(ctx: &mut Ctx, pool: usize) {
    ctx.gfx.clear(Color::hex(0x0B0E0B));
    ctx.scene3d.sky.atmosphere = false;
    ctx.scene3d.sky.fog_color = Color::hex(0x0B0E0B);
    ctx.scene3d.lighting.sun_direction = vec3(-0.45, -1.0, -0.3);
    ctx.scene3d.lighting.sun_intensity = 1.2;
    ctx.scene3d.lighting.sun_color = Color::hex(0xDDE8CF);
    ctx.scene3d.lighting.ambient_intensity = 0.5;
    ctx.scene3d.sky.sky_color = Color::hex(0x2A3326);
    ctx.scene3d.sky.ground_color = Color::hex(0x0B0E0B);
    ctx.scene3d.post.bloom = 0.7;
    ctx.scene3d.post.bloom_threshold = 0.9;
    ctx.scene3d.post.vignette = 0.3;
    ctx.scene3d.camera.far = 300.0;
    ctx.scene3d.camera.fov_y = 0.62;

    let glow = ctx.gfx.create_shader("glow", GLOW);
    GLOW_SHADER.with(|s| s.set(Some(glow)));
    let glowing = |ctx: &mut Ctx, color: Color, strength: f32| {
        ctx.gfx.create_material(cap_desc(color, strength))
    };

    let floor_shader = ctx.gfx.create_shader("floor", FLOOR);
    let floor_material = ctx.gfx.create_material({
        let mut desc = MaterialDesc::color(Color::hex(0x161B15)).with_roughness(0.6);
        desc.shader = Some(floor_shader);
        desc
    });
    let floor = ctx.gfx.create_mesh(&MeshData::plane(200.0), floor_material);
    ctx.world.spawn((Transform3D::default(), Model::new(floor)));

    // The board's frame, the four gates and their routes.
    let square = vec![
        vec3(-1.0, 0.0, -1.0),
        vec3(1.0, 0.0, -1.0),
        vec3(1.0, 0.0, 1.0),
        vec3(-1.0, 0.0, 1.0),
        vec3(-1.0, 0.0, -1.0),
    ];
    ctx.world.spawn((
        Transform3D::at(Vec3::Y * 0.02).scaled(HALF),
        Line {
            blend: ParticleBlend::Additive,
            ..Line::new(
                square.clone(),
                0.06 / HALF,
                Color::rgba(0.4, 0.6, 0.25, 0.8),
            )
        },
    ));
    let (a, b) = (GATE.start as f32 - HALF, GATE.end as f32 - HALF);
    for (g, (from, to)) in [
        (vec3(a, 0.0, -HALF), vec3(b, 0.0, -HALF)),
        (vec3(a, 0.0, HALF), vec3(b, 0.0, HALF)),
        (vec3(-HALF, 0.0, a), vec3(-HALF, 0.0, b)),
        (vec3(HALF, 0.0, a), vec3(HALF, 0.0, b)),
    ]
    .into_iter()
    .enumerate()
    {
        ctx.world.spawn((
            Transform3D::at(Vec3::Y * 0.05),
            Line {
                blend: ParticleBlend::Additive,
                ..Line::new(vec![from, to], 0.22, Color::WHITE)
            },
            GateMark(g),
        ));
        ctx.world.spawn((
            Transform3D::default(),
            Line {
                blend: ParticleBlend::Additive,
                ..Line::new(Vec::new(), 0.07, Color::WHITE)
            },
            Route(g),
        ));
    }

    // The core.
    let core_material = glowing(ctx, Color::hex(0xC0FA73), 4.0);
    let core = ctx.gfx.create_mesh(&MeshData::cube(1.1), core_material);
    ctx.world.spawn((
        Transform3D::at(vec3(0.0, 1.0, 0.0)),
        Model::new(core),
        Core,
        PointLight {
            color: Color::hex(0xC0FA73),
            intensity: 50.0,
            range: 9.0,
            ..Default::default()
        },
    ));
    let pad = ctx.gfx.create_material(
        MaterialDesc::color(Color::hex(0x2B3428))
            .with_roughness(0.5)
            .with_metallic(0.6),
    );
    let base = ctx.gfx.create_mesh(&MeshData::cube(1.0), pad);
    ctx.world.spawn((
        Transform3D {
            scale: vec3(1.9, 0.16, 1.9),
            ..Transform3D::at(vec3(0.0, 0.08, 0.0))
        },
        Model::new(base),
    ));

    // Towers: a dark pillar with a glowing cap.
    let stone = ctx.gfx.create_material(
        MaterialDesc::color(Color::hex(0x2E372B))
            .with_roughness(0.45)
            .with_metallic(0.3),
    );
    let pillar = ctx.gfx.create_mesh(&MeshData::cube(1.0), stone);
    let cap_material = glowing(ctx, Color::hex(0xC0FA73), 3.0);
    let cap = ctx.gfx.create_mesh(&MeshData::cube(1.0), cap_material);
    ctx.world.insert_resource(Look {
        pillar,
        cap,
        cap_material,
        cap_desc_strength: 3.0,
    });

    // The pointer's ghost square and the range it would cover.
    let ghost: Vec<Vec3> = square.iter().map(|p| *p * 0.46).collect();
    ctx.world.spawn((
        Transform3D::default().scaled(0.0),
        Line {
            blend: ParticleBlend::Additive,
            ..Line::new(ghost, 0.05, Color::WHITE)
        },
        Ghost,
    ));
    ctx.world.spawn((
        Transform3D::default().scaled(0.0),
        Line {
            blend: ParticleBlend::Additive,
            ..Line::new(
                circle(64),
                0.035 / TOWER_RANGE,
                Color::rgba(0.5, 0.8, 0.3, 0.6),
            )
        },
        GhostRing,
    ));

    // A pool of spark emitters, reused round robin.
    for i in 0..SPARKS {
        let mut settings = ParticleSettings::default()
            .with_rate(0.0)
            .with_burst(0)
            .with_shape(ParticleShape::Sphere { radius: 0.1 })
            .with_gravity(vec3(0.0, -6.0, 0.0))
            .with_drag(1.5)
            .with_size_over_life(ParticleCurve::new(vec![(0.0, 1.0), (1.0, 0.0)]))
            .with_color_over_life(ParticleGradient::new(vec![
                (0.0, Color::rgba(3.0, 2.2, 1.2, 1.0)),
                (1.0, Color::rgba(1.2, 0.3, 0.1, 0.0)),
            ]))
            .with_blend(ParticleBlend::Additive)
            .with_max_particles(64)
            .with_seed(i as u64 + 1);
        settings.lifetime = (0.25, 0.6);
        settings.speed = (1.5, 4.5);
        settings.size = (0.05, 0.12);
        let emitter = ParticleEmitter::new(settings);
        ctx.world
            .spawn((Transform3D::at(PARKED), emitter, Spark(i)));
    }

    // The units: a fixed pool, parked until they're needed.
    let unit_material = glowing(ctx, Color::WHITE, 1.6);
    let mut shape = MeshData::sphere(0.15, 6);
    for p in &mut shape.positions {
        p.y *= 0.75;
    }
    let unit = ctx.gfx.create_mesh(&shape, unit_material);
    ctx.gfx.set_mesh_shadows(unit, false);
    let batch: Vec<_> = (0..pool)
        .map(|_| {
            (
                Transform3D::at(PARKED),
                Model::new(unit),
                Unit {
                    life: Life::Idle,
                    kind: Kind::Grunt,
                    velocity: Vec2::ZERO,
                    hp: 0.0,
                    max_hp: 1.0,
                    timer: 0.0,
                    seed: 0.0,
                    zapped: false,
                },
            )
        })
        .collect();
    ctx.world.spawn_batch(batch);

    ctx.world.insert_resource(Crowd {
        cells: vec![Vec::new(); N * N],
        ..Default::default()
    });
    ctx.world.insert_resource(Board::new());
    ctx.world.insert_resource(State::new());
    starter_layout(ctx);
}

/// The most units alive at once: `?units=` on the web, `UNITS` on desktop.
const DEFAULT_POOL: usize = 12_000;

/// Runs the game.
pub fn run() {
    let asked = POOL_SIZE.with(|p| p.get());
    #[cfg(not(target_arch = "wasm32"))]
    let asked = std::env::var("UNITS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(asked);
    let pool = if asked > 0 {
        asked.clamp(500, 60_000)
    } else {
        DEFAULT_POOL
    };
    Game::new("Tide · ponko", 1280, 720)
        .system(control)
        .system(waves)
        .system(tide)
        .system(towers)
        .system(present)
        .run(move |ctx| setup(ctx, pool));
}

#[cfg(target_arch = "wasm32")]
mod web {
    use super::*;
    use wasm_bindgen::prelude::*;

    /// Starts the game with room for this many units (0: the default).
    #[wasm_bindgen]
    pub fn start(units: u32) {
        POOL_SIZE.with(|p| p.set(units as usize));
        run();
    }

    /// The pointer moved: x and y from 0 to 1 across the canvas. Negative: it left.
    #[wasm_bindgen]
    pub fn hover(x: f32, y: f32) {
        HOVER.with(|h| h.set((x >= 0.0 && y >= 0.0).then_some(vec2(x, y))));
    }

    /// A click or tap at x and y, 0 to 1 across the canvas.
    #[wasm_bindgen]
    pub fn tap(x: f32, y: f32) {
        TAPS.with(|t| t.borrow_mut().push(vec2(x, y)));
    }

    #[wasm_bindgen]
    pub fn begin() {
        START.with(|s| s.set(true));
    }

    #[wasm_bindgen]
    pub fn overcharge() {
        OVERCHARGE.with(|s| s.set(true));
    }

    /// What happened since the last call (see `Event`).
    #[wasm_bindgen]
    pub fn events() -> Vec<u8> {
        EVENTS.with(|v| std::mem::take(&mut *v.borrow_mut()))
    }

    /// [phase (0 title, 1 playing, 2 over), kills, best, core 0..1, towers,
    ///  units alive, sim ms, fps, wave, attacking (0/1), seconds to the next
    ///  wave, gate mask (bit 0 north, 1 south, 2 west, 3 east),
    ///  overcharge ready 0..1, overcharged (0/1)]
    #[wasm_bindgen]
    pub fn hud() -> Vec<f32> {
        let h = HUD.with(|h| h.get());
        vec![
            h.phase as f32,
            h.kills as f32,
            h.best as f32,
            h.core,
            h.towers as f32,
            h.alive as f32,
            h.sim_ms,
            h.fps,
            h.wave as f32,
            h.attacking as u8 as f32,
            h.countdown,
            h.gates as f32,
            h.overcharge,
            h.overcharged as u8 as f32,
        ]
    }
}
