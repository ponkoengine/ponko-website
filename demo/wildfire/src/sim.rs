//! The fire's rules: a grid of cells over the valley, run on the CPU.
//!
//! Every cell has a fuel (grass, shrub, forest, houses, or nothing that
//! burns), how much of it is left, how wet it is and how hot it burns. A
//! burning cell lights its eight neighbours with a chance that grows with
//! the wind blowing toward them, the slope up to them and how dry they are;
//! a hot forest fire also throws embers ahead of itself downwind (spot
//! fires). This module knows nothing about drawing, so the rules run in
//! plain tests (see the bottom of the file).

/// Cells per side; one cell is one meter.
pub const N: usize = 128;
pub const HALF: f32 = N as f32 / 2.0;
/// Simulation steps per second.
pub const STEP: f32 = 1.0 / 20.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fuel {
    Grass,
    Shrub,
    Forest,
    Houses,
    Rock,
    Water,
    Road,
    /// A firebreak the player dug.
    Break,
}

impl Fuel {
    /// How readily the cell catches from a burning neighbour.
    pub fn spread(self) -> f32 {
        match self {
            Fuel::Grass => 1.5,
            Fuel::Shrub => 0.95,
            Fuel::Forest => 0.6,
            Fuel::Houses => 0.5,
            _ => 0.0,
        }
    }
    /// Seconds a full cell burns.
    pub fn burn_time(self) -> f32 {
        match self {
            Fuel::Grass => 5.0,
            Fuel::Shrub => 9.0,
            Fuel::Forest => 20.0,
            Fuel::Houses => 16.0,
            _ => 1.0,
        }
    }
    /// How hot it burns at its peak (0 to 1).
    pub fn heat(self) -> f32 {
        match self {
            Fuel::Grass => 0.5,
            Fuel::Shrub => 0.75,
            Fuel::Forest => 1.0,
            Fuel::Houses => 0.9,
            _ => 0.0,
        }
    }
    pub fn burns(self) -> bool {
        self.spread() > 0.0
    }
}

/// A small, fast, deterministic random number generator (xorshift32).
#[derive(Clone)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Rng(seed.max(1))
    }
    pub fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    /// 0 to 1.
    pub fn f32(&mut self) -> f32 {
        (self.next() >> 8) as f32 / (1u32 << 24) as f32
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f32()
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next() % n.max(1)
    }
}

/// A cell's index from its column and row.
pub fn index(x: usize, z: usize) -> usize {
    z * N + x
}

/// The middle of cell `c` in the world (x, z).
pub fn cell_center(c: usize) -> (f32, f32) {
    ((c % N) as f32 + 0.5 - HALF, (c / N) as f32 + 0.5 - HALF)
}

/// The cell under world point (x, z), if it's on the map.
pub fn cell_at(x: f32, z: f32) -> Option<usize> {
    let (u, v) = (x + HALF, z + HALF);
    (u >= 0.0 && v >= 0.0 && u < N as f32 && v < N as f32).then(|| index(u as usize, v as usize))
}

/// What happened in a step, for the sounds of the page and the effects.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Happening {
    /// A spot fire started at this cell, from embers thrown from `from`.
    SpotFire { from: usize, to: usize },
    /// A house cell caught fire.
    HouseCaught(usize),
}

/// The fire and everything it burns.
#[derive(Clone)]
pub struct Sim {
    pub kind: Vec<Fuel>,
    /// Fuel left, 1 for an untouched cell, 0 burnt out.
    pub fuel: Vec<f32>,
    /// 0 bone dry to 1 soaked; drifts back to `dryness` after a water drop.
    pub moisture: Vec<f32>,
    pub base_moisture: Vec<f32>,
    /// How hot the cell is burning now (0 to 1); fades after it burns out.
    pub heat: Vec<f32>,
    pub burning: Vec<bool>,
    /// Ground height, for the slope.
    pub height: Vec<f32>,
    /// The wind: where it blows to (unit vector, x and z) and how strong (0 to 2).
    pub wind: (f32, f32),
    pub wind_speed: f32,
    pub rng: Rng,
    pub happenings: Vec<Happening>,
    /// Cells burning now (kept by `step`).
    pub burning_count: usize,
    ignite: Vec<usize>,
}

impl Sim {
    /// A map of `kind` cells at `height`, nothing burning.
    pub fn new(kind: Vec<Fuel>, height: Vec<f32>, base_moisture: Vec<f32>, seed: u32) -> Self {
        assert_eq!(kind.len(), N * N);
        let fuel = kind.iter().map(|k| if k.burns() { 1.0 } else { 0.0 }).collect();
        Sim {
            fuel,
            moisture: base_moisture.clone(),
            base_moisture,
            heat: vec![0.0; N * N],
            burning: vec![false; N * N],
            height,
            kind,
            wind: (1.0, 0.0),
            wind_speed: 1.0,
            rng: Rng::new(seed),
            happenings: Vec::new(),
            burning_count: 0,
            ignite: Vec::new(),
        }
    }

    /// Whether cell `c` could catch fire now.
    pub fn can_burn(&self, c: usize) -> bool {
        self.kind[c].burns() && !self.burning[c] && self.fuel[c] > 0.05
    }

    /// Sets cell `c` alight.
    pub fn light(&mut self, c: usize) {
        if self.can_burn(c) {
            self.burning[c] = true;
            self.heat[c] = self.heat[c].max(0.15);
            if self.kind[c] == Fuel::Houses {
                self.happenings.push(Happening::HouseCaught(c));
            }
        }
    }

    /// A lightning strike: lights `c` and what burns around it.
    pub fn strike(&mut self, c: usize) {
        let (x, z) = ((c % N) as i32, (c / N) as i32);
        for dz in -1..=1 {
            for dx in -1..=1 {
                let (nx, nz) = (x + dx, z + dz);
                if (0..N as i32).contains(&nx) && (0..N as i32).contains(&nz) {
                    let n = index(nx as usize, nz as usize);
                    self.moisture[n] = 0.0;
                    self.light(n);
                }
            }
        }
    }

    /// Digs a firebreak on `c`: clears what would burn. Refused on burning
    /// cells, houses, water and rock.
    pub fn dig(&mut self, c: usize) -> bool {
        let k = self.kind[c];
        if self.burning[c] || !matches!(k, Fuel::Grass | Fuel::Shrub | Fuel::Forest | Fuel::Road) {
            return false;
        }
        if k == Fuel::Road {
            return true; // already bare
        }
        self.kind[c] = Fuel::Break;
        self.fuel[c] = 0.0;
        true
    }

    /// A water drop along a strip `length` long and `width` wide, centered
    /// on (x, z) and running along `dir`. Returns how many burning cells it
    /// put out.
    pub fn drench(&mut self, x: f32, z: f32, dir: (f32, f32), length: f32, width: f32) -> usize {
        let (dx, dz) = dir;
        let side = (-dz, dx);
        let reach = (length.max(width) * 0.5 + 1.0) as i32;
        let Some(center) = cell_at(x, z) else {
            return 0;
        };
        let (cx, cz) = ((center % N) as i32, (center / N) as i32);
        let mut out = 0;
        for nz in (cz - reach).max(0)..=(cz + reach).min(N as i32 - 1) {
            for nx in (cx - reach).max(0)..=(cx + reach).min(N as i32 - 1) {
                let c = index(nx as usize, nz as usize);
                let (px, pz) = cell_center(c);
                let (rx, rz) = (px - x, pz - z);
                let along = (rx * dx + rz * dz) / (length * 0.5);
                let across = (rx * side.0 + rz * side.1) / (width * 0.5);
                let r = along * along + across * across;
                if r > 1.0 {
                    continue;
                }
                self.moisture[c] = self.moisture[c].max(1.0 - r * 0.3);
                if self.burning[c] {
                    self.burning[c] = false;
                    self.heat[c] *= 0.3;
                    out += 1;
                }
            }
        }
        out
    }

    /// Advances the fire by one step of `STEP` seconds.
    pub fn step(&mut self) {
        let dt = STEP;
        let (wx, wz) = self.wind;
        let ws = self.wind_speed;
        self.ignite.clear();
        let mut burning = 0;
        for c in 0..N * N {
            // Water dries out slowly.
            let base = self.base_moisture[c];
            if self.moisture[c] != base {
                self.moisture[c] += (base - self.moisture[c]) * (dt * 0.025);
            }
            if !self.burning[c] {
                if self.heat[c] > 0.0 {
                    self.heat[c] = (self.heat[c] - dt * 0.12).max(0.0);
                }
                continue;
            }
            let kind = self.kind[c];
            if self.moisture[c] > 0.6 {
                self.burning[c] = false;
                continue;
            }
            burning += 1;
            // Hot as the fuel allows, cooling as the last of it goes.
            let target = kind.heat() * (self.fuel[c] * 4.0).min(1.0);
            self.heat[c] += (target - self.heat[c]) * (dt * 1.2).min(1.0);
            self.fuel[c] -= dt / kind.burn_time();
            if self.fuel[c] <= 0.0 {
                self.fuel[c] = 0.0;
                self.burning[c] = false;
            }
            let heat = self.heat[c];
            let (x, z) = ((c % N) as i32, (c / N) as i32);
            let h = self.height[c];
            for (ox, oz) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
                let (nx, nz) = (x + ox, z + oz);
                if !(0..N as i32).contains(&nx) || !(0..N as i32).contains(&nz) {
                    continue;
                }
                let n = index(nx as usize, nz as usize);
                if !self.can_burn(n) {
                    continue;
                }
                let dist = if ox != 0 && oz != 0 { std::f32::consts::SQRT_2 } else { 1.0 };
                let (ux, uz) = (ox as f32 / dist, oz as f32 / dist);
                let wind = ((ux * wx + uz * wz) * ws * 1.1).exp();
                let slope = (((self.height[n] - h) / dist) * 0.45).exp().clamp(0.3, 4.0);
                let dry = (1.0 - self.moisture[n]).max(0.0);
                let rate = self.kind[n].spread() * heat * wind * slope * dry * dry / dist;
                if self.rng.f32() < 1.0 - (-rate * dt * 0.5).exp() {
                    self.ignite.push(n);
                }
            }
            // A hot forest fire in the wind throws embers ahead of it.
            if kind == Fuel::Forest && heat > 0.7 && ws > 0.5 && self.rng.f32() < dt * 0.012 * ws {
                let far = self.rng.range(5.0, 9.0 + ws * 4.0);
                let off = self.rng.range(-2.5, 2.5);
                let (px, pz) = cell_center(c);
                let (tx, tz) = (px + wx * far - wz * off, pz + wz * far + wx * off);
                if let Some(n) = cell_at(tx, tz)
                    && self.can_burn(n)
                    && self.moisture[n] < 0.45
                    && self.fuel[n] > 0.3
                {
                    self.ignite.push(n);
                    self.happenings.push(Happening::SpotFire { from: c, to: n });
                }
            }
        }
        let ignite = std::mem::take(&mut self.ignite);
        for &n in &ignite {
            self.light(n);
        }
        self.ignite = ignite;
        self.burning_count = burning;
    }

    /// How much of what could burn at the start is still unburnt (0 to 1),
    /// counting the vegetation only.
    pub fn green_left(&self, start: &Sim) -> f32 {
        let (mut had, mut has) = (0.0, 0.0);
        for c in 0..N * N {
            if matches!(start.kind[c], Fuel::Grass | Fuel::Shrub | Fuel::Forest) {
                had += 1.0;
                // A dug break saved what was there only in the sense that
                // it's still standing nearby; count it as lost.
                if self.kind[c] != Fuel::Break {
                    has += self.fuel[c];
                }
            }
        }
        if had > 0.0 { has / had } else { 1.0 }
    }

    /// How burnt cell `c` is, 0 to 1, for drawing.
    pub fn char(&self, c: usize, start: &Sim) -> f32 {
        if !start.kind[c].burns() || self.kind[c] == Fuel::Break {
            return 0.0;
        }
        (1.0 - self.fuel[c] / start.fuel[c].max(1e-3)).clamp(0.0, 1.0)
    }
}

/// Value noise in 0..1, smooth between whole coordinates.
pub fn noise(x: f32, z: f32, seed: u32) -> f32 {
    let hash = |i: i32, j: i32| {
        let mut h = (i as u32).wrapping_mul(0x27d4_eb2d) ^ (j as u32).wrapping_mul(0x1656_67b1) ^ seed;
        h ^= h >> 15;
        h = h.wrapping_mul(0x85eb_ca6b);
        h ^= h >> 13;
        (h & 0xFFFF) as f32 / 65535.0
    };
    let (i, j) = (x.floor() as i32, z.floor() as i32);
    let (fx, fz) = (x - x.floor(), z - z.floor());
    let (sx, sz) = (fx * fx * (3.0 - 2.0 * fx), fz * fz * (3.0 - 2.0 * fz));
    let a = hash(i, j) + (hash(i + 1, j) - hash(i, j)) * sx;
    let b = hash(i, j + 1) + (hash(i + 1, j + 1) - hash(i, j + 1)) * sx;
    a + (b - a) * sz
}

/// Four octaves of `noise`, in 0..1.
pub fn fbm(x: f32, z: f32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut f, mut total) = (0.0, 0.5, 1.0, 0.0);
    for o in 0..4 {
        sum += noise(x * f, z * f, seed.wrapping_add(o * 7919)) * amp;
        total += amp;
        amp *= 0.5;
        f *= 2.03;
    }
    sum / total
}

/// The valley: a river winding west to east between wooded ridges.
pub mod valley {
    use super::*;

    const SEED: u32 = 0x00F1_2E00;

    /// Where the river runs (z) at `x`.
    pub fn river_z(x: f32) -> f32 {
        13.0 * (x * 0.043 + 0.7).sin() + 5.0 * (x * 0.105 + 2.1).sin()
    }

    /// Ground height at world point (x, z).
    pub fn height(x: f32, z: f32) -> f32 {
        let d = (z - river_z(x)).abs();
        let ridge = (d / 44.0).min(1.25);
        let hills = fbm(x * 0.035, z * 0.035, SEED) - 0.35;
        let bumps = fbm(x * 0.12, z * 0.12, SEED ^ 0x55) - 0.5;
        let mut h = ridge.powf(1.4) * 15.0 + hills * (3.0 + ridge * 9.0) + bumps * 1.2;
        // Nothing but the river below the water.
        h = h.max(0.25 + bumps * 0.4);
        // A flat floor by the river, and the river bed below the water.
        let floor = smooth(3.0, 9.0, d);
        h = h * floor + 0.35 * (1.0 - floor);
        let bed = smooth(1.2, 2.6, d);
        h * bed + -0.9 * (1.0 - bed)
    }

    fn smooth(a: f32, b: f32, x: f32) -> f32 {
        let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// Where the road runs (z) at `x`: alongside the river, on its north bank.
    pub fn road_z(x: f32) -> f32 {
        river_z(x) - 8.5
    }

    /// The villages: their middle cell (x, z in cells), north or south of the river.
    pub fn villages() -> Vec<(f32, f32)> {
        [(-46.0, -1.0), (-20.0, 1.0), (4.0, -1.0), (27.0, 1.0), (50.0, -1.0)]
            .iter()
            .map(|&(x, side)| {
                let z = if side < 0.0 { road_z(x) - 3.5 } else { river_z(x) + 11.0 };
                (x, z)
            })
            .collect()
    }

    /// The whole map: fuel, heights, dryness, and the house cells of each village.
    pub fn generate() -> (Vec<Fuel>, Vec<f32>, Vec<f32>, Vec<Vec<usize>>) {
        let mut kind = vec![Fuel::Grass; N * N];
        let mut height = vec![0.0; N * N];
        let mut moisture = vec![0.0; N * N];
        for c in 0..N * N {
            let (x, z) = cell_center(c);
            let h = self::height(x, z);
            height[c] = h;
            let d = (z - river_z(x)).abs();
            let patch = fbm(x * 0.06, z * 0.06, SEED ^ 0xABCD);
            let fine = noise(x * 0.4, z * 0.4, SEED ^ 0x1234);
            kind[c] = if d < 1.9 {
                Fuel::Water
            } else if h > 24.0 + fine * 5.0 {
                Fuel::Rock
            } else if d < 5.5 + patch * 4.0 || patch < 0.3 {
                Fuel::Grass
            } else if patch < 0.4 || h > 13.0 + fine * 3.0 {
                Fuel::Shrub
            } else {
                Fuel::Forest
            };
            // Wetter near the river and in the hollows, drier on the ridges.
            moisture[c] = (0.28 - d * 0.004 + (fine - 0.5) * 0.08).clamp(0.05, 0.3);
            if d < 4.0 {
                moisture[c] = 0.35;
            }
        }
        // The road along the valley.
        for x in 0..N {
            let wx = x as f32 + 0.5 - HALF;
            let rz = road_z(wx) + HALF;
            for z in [rz.floor() as i32, rz.floor() as i32 + 1] {
                if (0..N as i32).contains(&z) {
                    let c = index(x, z as usize);
                    if kind[c] != Fuel::Water {
                        kind[c] = Fuel::Road;
                    }
                }
            }
        }
        // Villages: houses in a loose block, a meadow around them.
        let mut rng = Rng::new(SEED);
        let mut villages = Vec::new();
        for (vx, vz) in self::villages() {
            let (cx, cz) = ((vx + HALF) as i32, (vz + HALF) as i32);
            for dz in -6i32..=6 {
                for dx in -7i32..=7 {
                    let (x, z) = (cx + dx, cz + dz);
                    if !(0..N as i32).contains(&x) || !(0..N as i32).contains(&z) {
                        continue;
                    }
                    let c = index(x as usize, z as usize);
                    if matches!(kind[c], Fuel::Forest | Fuel::Shrub | Fuel::Rock) && dx * dx + dz * dz < 40 {
                        kind[c] = Fuel::Grass;
                    }
                }
            }
            let mut houses = Vec::new();
            for dz in -2i32..=2 {
                for dx in -3i32..=3 {
                    if (dx + dz) % 2 != 0 || rng.f32() < 0.25 {
                        continue;
                    }
                    let (x, z) = (cx + dx, cz + dz);
                    if !(0..N as i32).contains(&x) || !(0..N as i32).contains(&z) {
                        continue;
                    }
                    let c = index(x as usize, z as usize);
                    if matches!(kind[c], Fuel::Grass) {
                        kind[c] = Fuel::Houses;
                        houses.push(c);
                    }
                }
            }
            villages.push(houses);
        }
        (kind, height, moisture, villages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat, dry field of `fuel`, calm or windy.
    fn field(fuel: Fuel) -> Sim {
        Sim::new(vec![fuel; N * N], vec![0.0; N * N], vec![0.1; N * N], 7)
    }

    fn run(sim: &mut Sim, seconds: f32) {
        for _ in 0..(seconds / STEP) as usize {
            sim.step();
        }
    }

    fn burnt_or_burning(sim: &Sim, c: usize) -> bool {
        sim.burning[c] || sim.fuel[c] < 0.99 && sim.kind[c].burns()
    }

    #[test]
    fn fire_spreads_through_dry_grass() {
        let mut sim = field(Fuel::Grass);
        sim.wind_speed = 0.0;
        sim.strike(index(64, 64));
        run(&mut sim, 10.0);
        assert!(burnt_or_burning(&sim, index(68, 64)), "fire should reach 4 cells away in 10 s of grass");
    }

    #[test]
    fn fire_runs_faster_downwind_than_upwind() {
        let mut sim = field(Fuel::Grass);
        sim.wind = (1.0, 0.0);
        sim.wind_speed = 1.5;
        sim.strike(index(64, 64));
        run(&mut sim, 8.0);
        let reach = |dir: i32| {
            (1..60).take_while(|&k| burnt_or_burning(&sim, index((64 + dir * k) as usize, 64))).count()
        };
        let (down, up) = (reach(1), reach(-1));
        assert!(down > up * 2, "downwind {down} cells, upwind {up}");
    }

    #[test]
    fn fire_climbs_uphill_faster() {
        let height = (0..N * N).map(|c| (c % N) as f32 * 0.8).collect();
        let mut sim = Sim::new(vec![Fuel::Grass; N * N], height, vec![0.1; N * N], 3);
        sim.wind_speed = 0.0;
        sim.strike(index(64, 64));
        run(&mut sim, 8.0);
        let reach = |dir: i32| {
            (1..60).take_while(|&k| burnt_or_burning(&sim, index((64 + dir * k) as usize, 64))).count()
        };
        assert!(reach(1) > reach(-1), "uphill {} vs downhill {}", reach(1), reach(-1));
    }

    #[test]
    fn a_wide_firebreak_stops_a_calm_fire() {
        let mut sim = field(Fuel::Grass);
        sim.wind_speed = 0.0;
        for z in 0..N {
            for x in 70..73 {
                assert!(sim.dig(index(x, z)));
            }
        }
        sim.strike(index(60, 64));
        run(&mut sim, 60.0);
        assert!((0..N).all(|z| !burnt_or_burning(&sim, index(80, z))), "the fire jumped a 3-cell break");
        assert!(burnt_or_burning(&sim, index(69, 64)), "the fire should reach the break");
    }

    #[test]
    fn water_puts_fire_out_and_keeps_cells_from_catching() {
        let mut sim = field(Fuel::Grass);
        sim.wind_speed = 0.0;
        sim.strike(index(64, 64));
        run(&mut sim, 2.0);
        let (x, z) = cell_center(index(64, 64));
        let out = sim.drench(x, z, (1.0, 0.0), 30.0, 30.0);
        assert!(out > 0, "the drop should put out burning cells");
        run(&mut sim, 1.0);
        assert_eq!(sim.burning_count, 0);
    }

    #[test]
    fn houses_and_water_refuse_a_break() {
        let mut sim = field(Fuel::Houses);
        assert!(!sim.dig(index(3, 3)));
        sim.kind[index(4, 4)] = Fuel::Water;
        assert!(!sim.dig(index(4, 4)));
    }

    #[test]
    fn the_same_seed_burns_the_same_way() {
        let (kind, height, moisture, _) = valley::generate();
        let mut a = Sim::new(kind.clone(), height.clone(), moisture.clone(), 11);
        let mut b = Sim::new(kind, height, moisture, 11);
        for sim in [&mut a, &mut b] {
            let c = (0..N * N).find(|&c| sim.kind[c] == Fuel::Forest).unwrap();
            sim.strike(c);
            run(sim, 30.0);
        }
        assert_eq!(a.fuel, b.fuel);
    }

    #[test]
    fn the_valley_has_every_kind_of_ground_and_five_villages() {
        let (kind, height, _, villages) = valley::generate();
        for k in [Fuel::Grass, Fuel::Shrub, Fuel::Forest, Fuel::Water, Fuel::Road, Fuel::Houses] {
            assert!(kind.contains(&k), "{k:?} missing");
        }
        let forest = kind.iter().filter(|&&k| k == Fuel::Forest).count();
        assert!(forest > N * N / 5, "only {forest} forest cells");
        assert_eq!(villages.len(), 5);
        assert!(villages.iter().all(|v| v.len() >= 5), "{:?}", villages.iter().map(Vec::len).collect::<Vec<_>>());
        assert!(height.iter().all(|h| h.is_finite()));
    }
}

#[cfg(test)]
mod pacing {
    use super::*;

    /// Not a check: prints how a fire nobody fights plays out
    /// (`cargo test pacing -- --ignored --nocapture`).
    #[test]
    #[ignore]
    fn an_unfought_fire() {
        let (kind, height, moisture, villages) = valley::generate();
        for seed in [1u32, 2, 3] {
            let mut sim = Sim::new(kind.clone(), height.clone(), moisture.clone(), seed);
            let start = sim.clone();
            let mut rng = Rng::new(seed * 77);
            let forest: Vec<usize> = (0..N * N)
                .filter(|&c| {
                    let (x, z) = cell_center(c);
                    sim.kind[c] == Fuel::Forest
                        && villages.iter().flatten().all(|&h| {
                            let (hx, hz) = cell_center(h);
                            (hx - x).hypot(hz - z) > 30.0
                        })
                })
                .collect();
            let mut first_house = None;
            let mut t = 0.0;
            let mut lost_at = Vec::new();
            for step in 0..(300.0 / STEP) as usize {
                t = step as f32 * STEP;
                for at in [0.5, 45.0, 95.0] {
                    if (t - at).abs() < STEP / 2.0 {
                        let c = forest[rng.below(forest.len() as u32) as usize];
                        sim.strike(c);
                    }
                }
                sim.step();
                for h in sim.happenings.drain(..) {
                    if let Happening::HouseCaught(_) = h {
                        first_house.get_or_insert(t);
                    }
                }
                for (v, cells) in villages.iter().enumerate() {
                    let gone = cells.iter().filter(|&&c| sim.fuel[c] < 0.6).count();
                    if gone * 2 >= cells.len() && !lost_at.iter().any(|&(w, _)| w == v) {
                        lost_at.push((v, t));
                    }
                }
                if t > 96.0 && sim.burning_count == 0 {
                    break;
                }
            }
            println!(
                "seed {seed}: first house {first_house:?} s, villages lost {:?}, out at {t:.0} s, green left {:.0}%, max burning {}",
                lost_at.iter().map(|&(v, t)| (v, t as u32)).collect::<Vec<_>>(),
                sim.green_left(&start) * 100.0,
                sim.burning_count
            );
        }
    }
}
