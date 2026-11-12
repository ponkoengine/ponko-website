// Pictures in dots for blog posts. A seed (the post's address) picks a
// palette and a shape unless the post names them, so every post has its own
// art and the same post always looks the same. Built into SVG at build time.

export const palettes = {
  // From dark to light. The last is the brightest highlight.
  ember: ['#4a1f10', '#b5441d', '#f0672f', '#ffd9b8'],
  amber: ['#43300f', '#b77a1e', '#f3c26b', '#fff1d6'],
  rose: ['#45182a', '#b13d5e', '#f07a95', '#ffe3ea'],
  sage: ['#1e3020', '#5f8a3e', '#b9d98a', '#f0f8de'],
  dusk: ['#25204a', '#5f52b8', '#a99cf0', '#f1edff'],
  sea: ['#0f3034', '#2a8a90', '#7fd8cd', '#e2fdf8'],
  // These run through more than one hue on the way up.
  lava: ['#3a0d0b', '#a3201a', '#ff6a1f', '#ffe27a'],
  synth: ['#2c1250', '#a12a8f', '#ff5f8a', '#ffd88a'],
  sunset: ['#2e1438', '#c23b4a', '#f7a03c', '#fff2c4'],
  boreal: ['#0c2140', '#1f7a8c', '#5ee6a8', '#eaffd9'],
  ice: ['#14263c', '#3f6fa8', '#93c8f2', '#f2faff'],
  neon: ['#101448', '#3048ff', '#2ee0ff', '#eafdff'],
  acid: ['#1f2b0a', '#5f9414', '#c6f53c', '#faffd4'],
  ash: ['#2b2826', '#6c6661', '#bfb7ad', '#fcf8f2'],
} as const;
export type PaletteName = keyof typeof palettes;
export const paletteNames = Object.keys(palettes) as PaletteName[];

export const shapes = [
  'blobs', 'ripple', 'waves', 'sphere', 'swirl', 'interference', 'aurora', 'plasma', 'orbits',
  'life', 'donut', 'tunnel', 'starfield', 'rain', 'fire', 'contours', 'moire', 'lissajous', 'voronoi', 'equalizer',
] as const;
export type Shape = (typeof shapes)[number];

export type ArtOptions = { palette?: PaletteName; shape?: Shape; seed?: string };

/** The grid of dots a picture is drawn on, for shapes that work cell by cell. */
export type Grid = { cols: number; rows: number };

function hashOf(text: string) {
  let h = 2166136261;
  for (const ch of text) h = Math.imul(h ^ ch.charCodeAt(0), 16777619);
  return h >>> 0;
}

function random(seed: number) {
  let h = seed | 0;
  return () => {
    h = (h + 0x6d2b79f5) | 0;
    let x = Math.imul(h ^ (h >>> 15), 1 | h);
    x = (x + Math.imul(x ^ (x >>> 7), 61 | x)) ^ x;
    return ((x ^ (x >>> 14)) >>> 0) / 4294967296;
  };
}

/** The palette and shape a post gets: its own choice, or one from its seed. */
export function resolveArt(id: string, art: ArtOptions = {}) {
  const h = hashOf(art.seed ?? id);
  return {
    seed: art.seed ?? id,
    palette: art.palette ?? paletteNames[h % paletteNames.length],
    shape: art.shape ?? shapes[(h >>> 8) % shapes.length],
  };
}

export type Art = {
  cols: number;
  rows: number;
  /** One path per colour and size: zero-length lines with round caps. */
  groups: { color: string; width: number; d: string }[];
  ring: string;
};

type Field = (x: number, y: number, t: number) => number;
type Kit = {
  seed: string;
  rand: () => number;
  noise: (x: number, y: number) => number;
  /** A few octaves of noise added up, from 0 to 1. */
  fbm: (x: number, y: number) => number;
  aspect: number;
  grid: Grid;
};

const clamp01 = (v: number) => Math.min(1, Math.max(0, v));
const smooth = (v: number) => v * v * (3 - 2 * v);
const TAU = Math.PI * 2;

/**
 * For shapes that build a whole frame at once (a simulation, a rasterised
 * model): one value per grid cell, rebuilt only when the time changes, since
 * every dot of a frame asks for the same time.
 */
function perFrame({ grid, aspect }: Kit, build: (t: number, out: Float32Array) => void): Field {
  const { cols, rows } = grid;
  const buffer = new Float32Array(cols * rows);
  let built = NaN;
  return (x, y, t) => {
    if (t !== built) {
      buffer.fill(0);
      build(t, buffer);
      built = t;
    }
    const i = Math.min(cols - 1, Math.max(0, Math.floor((x / aspect) * cols)));
    const j = Math.min(rows - 1, Math.max(0, Math.floor(y * rows)));
    return buffer[j * cols + i];
  };
}

const builders: Record<Shape, (kit: Kit) => Field> = {
  blobs({ rand, aspect }) {
    // Metaballs, each circling slowly on its own path.
    const balls = Array.from({ length: 3 + Math.floor(rand() * 2) }, () => ({
      x: (0.2 + rand() * 0.6) * aspect, y: 0.2 + rand() * 0.6, r: 0.16 + rand() * 0.16,
      sx: 0.25 + rand() * 0.35, sy: 0.2 + rand() * 0.3, p: rand() * 6.28,
    }));
    return (x, y, t) => {
      let sum = 0;
      for (const b of balls) {
        const bx = b.x + Math.sin(t * b.sx + b.p) * 0.12 * aspect;
        const by = b.y + Math.cos(t * b.sy + b.p * 1.3) * 0.1;
        sum += (b.r * b.r) / ((x - bx) ** 2 + (y - by) ** 2 + 0.002);
      }
      return Math.min(1, Math.max(0, (sum - 0.55) / 1.6));
    };
  },

  ripple({ rand, aspect }) {
    // Rings that spread out from a point.
    const cx = (0.3 + rand() * 0.4) * aspect, cy = 0.3 + rand() * 0.4, freq = 26 + rand() * 14;
    return (x, y, t) => {
      const d = Math.hypot(x - cx, y - cy);
      const wave = 0.5 + 0.5 * Math.cos(d * freq - t * 2.4);
      return Math.max(0, wave * Math.max(0, 1 - d / 0.75) ** 1.2);
    };
  },

  waves({ rand, aspect }) {
    // Lines along sine curves that travel sideways, each at its own speed.
    const lines = Array.from({ length: 3 }, (_, i) => ({
      y: 0.3 + i * 0.2 + (rand() - 0.5) * 0.08, amp: 0.05 + rand() * 0.08, freq: 3 + rand() * 4,
      phase: rand() * 6.28, speed: (0.5 + rand() * 0.6) * (i % 2 ? -1 : 1),
    }));
    return (x, y, t) => {
      let best = 0;
      for (const l of lines) {
        const d = Math.abs(y - (l.y + Math.sin((x / aspect) * l.freq + l.phase - t * l.speed) * l.amp));
        best = Math.max(best, Math.max(0, 1 - d / 0.09));
      }
      return best ** 1.4;
    };
  },

  swirl({ rand, aspect }) {
    // A spiral galaxy: arms wound around a centre, turning slowly.
    const cx = (0.42 + rand() * 0.16) * aspect, cy = 0.42 + rand() * 0.16;
    const arms = 2 + Math.floor(rand() * 2), twist = 9 + rand() * 6, turn = rand() < 0.5 ? -1 : 1;
    return (x, y, t) => {
      const dx = x - cx, dy = y - cy, d = Math.hypot(dx, dy);
      const arm = 0.5 + 0.5 * Math.cos(arms * Math.atan2(dy, dx) + d * twist * turn - t * 0.9);
      const core = Math.max(0, 1 - d / 0.09);
      return Math.max(core, arm ** 2 * Math.max(0, 1 - d / 0.62) * 1.15);
    };
  },

  interference({ rand, aspect }) {
    // Two sources of ripples drifting apart and together; where their
    // rings meet they add up or cancel out.
    const cy = 0.4 + rand() * 0.2, cx = (0.4 + rand() * 0.2) * aspect, gap = 0.18 + rand() * 0.1;
    const freq = 20 + rand() * 8, p = rand() * 6.28;
    return (x, y, t) => {
      const sway = gap + Math.sin(t * 0.3 + p) * 0.06;
      const a = Math.hypot(x - cx + sway, y - cy), b = Math.hypot(x - cx - sway, y - cy);
      const wave = (Math.cos(a * freq - t * 2.2) + Math.cos(b * freq - t * 2.2)) / 2;
      const fade = Math.max(0, 1 - Math.hypot(x - cx, y - cy) / 0.8);
      return (0.5 + 0.5 * wave) ** 1.6 * fade ** 0.7;
    };
  },

  aurora({ rand, aspect }) {
    // Northern lights: a curtain that sways, bright at its edge and fading
    // upward, with folds running down it.
    const base = 0.6 + rand() * 0.1, p = rand() * 6.28, folds = 9 + rand() * 5;
    return (x, y, t) => {
      const u = x / aspect;
      const edge = base + Math.sin(u * 4 + p + t * 0.5) * 0.08 + Math.sin(u * 9 - t * 0.8 + p * 2) * 0.03;
      const above = edge - y;
      if (above < 0) return Math.max(0, 0.9 + above * 12);
      const glow = Math.exp(-above * 4.2);
      const fold = 0.7 + 0.3 * Math.sin(u * folds + Math.sin(u * 2.5 + t * 0.6) * 1.5 + p);
      return Math.min(1, glow * fold * 1.1);
    };
  },

  plasma({ rand, aspect }) {
    // The old demo-scene plasma: sines over x, y and distance, added up.
    const k = 5 + rand() * 4, p = rand() * 6.28, cx = rand() * aspect, cy = rand();
    return (x, y, t) => {
      const v =
        Math.sin(x * k + t * 0.7 + p) +
        Math.sin(y * k * 1.3 - t * 0.5) +
        Math.sin((x + y) * k * 0.7 + t * 0.4) +
        Math.sin(Math.hypot(x - cx, y - cy) * k * 1.6 - t * 0.9);
      return Math.max(0, Math.max(0, v / 4 + 0.5) ** 2.4 * 1.6 - 0.2);
    };
  },

  orbits({ rand, aspect }) {
    // A small system: a bright body in the middle and moons on tilted
    // ellipses around it, each at its own speed.
    const cx = (0.42 + rand() * 0.16) * aspect, cy = 0.45 + rand() * 0.1;
    const moons = Array.from({ length: 3 + Math.floor(rand() * 2) }, (_, i) => ({
      rx: 0.26 + i * 0.13 + rand() * 0.04, tilt: 0.4 + rand() * 0.25, size: 0.06 + rand() * 0.035,
      speed: (0.9 - i * 0.15) * (rand() < 0.5 ? -1 : 1), p: rand() * 6.28,
    }));
    return (x, y, t) => {
      let v = Math.max(0, 1 - Math.hypot(x - cx, y - cy) / 0.16) ** 0.6;
      for (const m of moons) {
        // The path itself, faint.
        const ex = (x - cx) / m.rx, ey = (y - cy) / (m.rx * m.tilt);
        v = Math.max(v, 0.3 * Math.max(0, 1 - Math.abs(Math.hypot(ex, ey) - 1) * 6));
        const a = t * m.speed + m.p;
        const mx = cx + Math.cos(a) * m.rx, my = cy + Math.sin(a) * m.rx * m.tilt;
        v = Math.max(v, Math.max(0, 1 - Math.hypot(x - mx, y - my) / m.size) ** 0.5);
      }
      return v;
    };
  },

  sphere({ rand, aspect }) {
    // A ball lit from a light that circles around it.
    const cx = (0.4 + rand() * 0.2) * aspect, cy = 0.45 + rand() * 0.1, r = 0.36, p = rand() * 6.28;
    return (x, y, t) => {
      const dx = (x - cx) / r, dy = (y - cy) / r, dd = dx * dx + dy * dy;
      if (dd > 1) return Math.max(0, 0.22 - (Math.sqrt(dd) - 1) * 0.9);
      const a = t * 0.35 + p;
      const lx = Math.cos(a) * 0.75, ly = -0.45, lz = 0.5 + Math.sin(a) * 0.3;
      const dz = Math.sqrt(1 - dd);
      return Math.max(0.08, (dx * lx + dy * ly + dz * lz) / Math.hypot(lx, ly, lz)) ** 1.1;
    };
  },

  life(kit) {
    // Conway's Game of Life, one cell per dot, on a board whose edges wrap
    // around. Cells that die leave a fading trail, so gliders look like
    // comets. Every so often a new pattern lands, so the board never
    // settles into still life.
    const { cols, rows } = kit.grid;
    const n = cols * rows;
    const seed = hashOf(kit.seed);
    const RATE = 9; // generations per unit of time
    const WARM = 36; // generations run before the first picture
    const EVERY = 16; // generations between new patterns
    const patterns = [
      [[1, 0], [2, 1], [0, 2], [1, 2], [2, 2]], // glider
      [[1, 0], [2, 1], [0, 2], [1, 2], [2, 2]], // another glider: they're the best part
      [[1, 0], [4, 0], [0, 1], [0, 2], [4, 2], [0, 3], [1, 3], [2, 3], [3, 3]], // spaceship
      [[1, 0], [2, 0], [0, 1], [1, 1], [1, 2]], // R-pentomino
      [[1, 0], [3, 1], [0, 2], [1, 2], [4, 2], [5, 2], [6, 2]], // acorn
    ];

    let cells = new Uint8Array(n), next = new Uint8Array(n);
    let heat = new Float32Array(n), after = new Float32Array(n);
    let generation = 0;

    function drop(g: number) {
      const r = random(seed ^ Math.imul(g + 1, 0x9e3779b1));
      const shape = patterns[Math.floor(r() * patterns.length)];
      const ox = Math.floor(r() * cols), oy = Math.floor(r() * rows);
      const fx = r() < 0.5 ? -1 : 1, fy = r() < 0.5 ? -1 : 1, swap = r() < 0.5;
      for (const [a, b] of shape) {
        const px = (swap ? b : a) * fx, py = (swap ? a : b) * fy;
        cells[(((oy + py) % rows) + rows) % rows * cols + ((((ox + px) % cols) + cols) % cols)] = 1;
      }
    }
    function step() {
      for (let j = 0; j < rows; j++) {
        const up = ((j + rows - 1) % rows) * cols, row = j * cols, down = ((j + 1) % rows) * cols;
        for (let i = 0; i < cols; i++) {
          const l = (i + cols - 1) % cols, r = (i + 1) % cols;
          const around =
            cells[up + l] + cells[up + i] + cells[up + r] +
            cells[row + l] + cells[row + r] +
            cells[down + l] + cells[down + i] + cells[down + r];
          const alive = around === 3 || (around === 2 && cells[row + i] === 1);
          next[row + i] = alive ? 1 : 0;
        }
      }
      [cells, next] = [next, cells];
      generation++;
      if (generation % EVERY === 0) drop(generation);
    }
    function glow() {
      for (let k = 0; k < n; k++) after[k] = cells[k] ? 1 : heat[k] * 0.6;
    }
    function reset() {
      const r = random(seed);
      cells.fill(0);
      for (let k = 0; k < n; k++) cells[k] = r() < 0.3 ? 1 : 0;
      heat.fill(0);
      generation = 0;
      for (let g = 0; g < WARM; g++) {
        step();
        glow();
        [heat, after] = [after, heat];
      }
      // `heat` is now this generation, `after` the next one.
      step();
      glow();
    }
    reset();

    return perFrame(kit, (t, out) => {
      const want = WARM + Math.max(0, Math.floor(t * RATE));
      if (want < generation - 1) reset();
      while (generation - 1 < want) {
        [heat, after] = [after, heat];
        step();
        glow();
      }
      // Fade from one generation into the next, so births and deaths are soft.
      const f = smooth(clamp01(t * RATE - Math.floor(t * RATE)));
      for (let k = 0; k < n; k++) out[k] = heat[k] + (after[k] - heat[k]) * f;
    });
  },

  donut(kit) {
    // The spinning torus from the old ASCII demo: a ring turning on two
    // axes, shaded by a light above and behind the viewer.
    const { rand } = kit;
    const { cols, rows } = kit.grid;
    const p = rand() * TAU, spinA = 0.55 + rand() * 0.3, spinB = 0.3 + rand() * 0.25;
    const cx = cols * (0.48 + rand() * 0.1), cy = rows * 0.5;
    const R1 = 1, R2 = 2, K2 = 5, K1 = (rows * K2 * 0.9) / (2 * (R1 + R2));
    const depth = new Float32Array(cols * rows);
    return perFrame(kit, (t, out) => {
      depth.fill(0);
      const A = t * spinA + p, B = t * spinB + p * 0.7;
      const cA = Math.cos(A), sA = Math.sin(A), cB = Math.cos(B), sB = Math.sin(B);
      for (let th = 0; th < TAU; th += 0.09) {
        const ct = Math.cos(th), st = Math.sin(th);
        for (let ph = 0; ph < TAU; ph += 0.035) {
          const cp = Math.cos(ph), sp = Math.sin(ph);
          const hx = R2 + R1 * ct, hy = R1 * st;
          const x = hx * (cB * cp + sA * sB * sp) - hy * cA * sB;
          const y = hx * (sB * cp - sA * cB * sp) + hy * cA * cB;
          const z = K2 + cA * hx * sp + hy * sA;
          const ooz = 1 / z;
          const i = Math.floor(cx + K1 * ooz * x), j = Math.floor(cy - K1 * ooz * y);
          if (i < 0 || j < 0 || i >= cols || j >= rows) continue;
          const k = j * cols + i;
          if (ooz <= depth[k]) continue;
          depth[k] = ooz;
          const light = cp * ct * sB - cA * ct * sp - sA * st + cB * (cA * st - ct * sA * sp);
          out[k] = 0.14 + 0.86 * clamp01(light / Math.SQRT2) ** 0.9;
        }
      }
    });
  },

  tunnel({ rand, aspect }) {
    // Flying down a tunnel: rings rush outward from a far point that wanders
    // a little, with stripes along the walls turning slowly.
    const cx = (0.5 + rand() * 0.12) * aspect, cy = 0.45 + rand() * 0.1;
    const segments = 6 + Math.floor(rand() * 5), twist = (rand() < 0.5 ? -1 : 1) * (0.15 + rand() * 0.2);
    return (x, y, t) => {
      const ox = cx + Math.sin(t * 0.37) * 0.1, oy = cy + Math.cos(t * 0.29) * 0.06;
      const dx = x - ox, dy = y - oy, d = Math.hypot(dx, dy) + 1e-4;
      const z = 0.32 / d + t * 1.1;
      const a = Math.atan2(dy, dx) / TAU + t * twist + 0.12 / d;
      const ring = 0.5 + 0.5 * Math.cos(z * TAU);
      const stripe = 0.5 + 0.5 * Math.cos(a * segments * TAU);
      const near = clamp01(d * 3.2) * clamp01(1.25 - d * 0.9);
      return (ring * 0.7 + ring * stripe * 0.5) ** 1.3 * near;
    };
  },

  starfield(kit) {
    // Flying through space: stars stream out from a point ahead, drawn as
    // streaks that grow longer and brighter as they pass.
    const { rand } = kit;
    const { cols, rows } = kit.grid;
    const cx = cols * (0.5 + rand() * 0.1), cy = rows * (0.45 + rand() * 0.1);
    const stars = Array.from({ length: 90 }, () => ({
      a: rand() * TAU, p: rand(), s: 0.1 + rand() * 0.16,
    }));
    return perFrame(kit, (t, out) => {
      for (const st of stars) {
        const z = (st.p + t * st.s) % 1;
        const far = rows * (0.04 + z * z * 1.25), near = far * (1 - 0.55 * z);
        const ux = Math.cos(st.a), uy = Math.sin(st.a);
        const bright = 0.2 + 0.8 * z * z, size = 0.5 + z * 0.55;
        for (let r = near; r <= far; r += 0.5) {
          const px = cx + ux * r, py = cy + uy * r;
          const i0 = Math.floor(px - size), j0 = Math.floor(py - size);
          for (let j = j0; j <= Math.floor(py + size); j++) {
            for (let i = i0; i <= Math.floor(px + size); i++) {
              if (i < 0 || j < 0 || i >= cols || j >= rows) continue;
              const v = bright * Math.max(0, 1 - Math.hypot(i + 0.5 - px, j + 0.5 - py) / size);
              if (v > out[j * cols + i]) out[j * cols + i] = v;
            }
          }
        }
      }
    });
  },

  rain(kit) {
    // Falling code: every column has drops sliding down it, each with a
    // bright head and a tail that fades behind it.
    const { rand, aspect } = kit;
    const { cols } = kit.grid;
    const drops = Array.from({ length: cols }, () =>
      Array.from({ length: 2 }, () => ({ s: 0.18 + rand() * 0.35, p: rand(), len: 0.2 + rand() * 0.45 })),
    );
    return (x, y, t) => {
      const column = drops[Math.min(cols - 1, Math.max(0, Math.floor((x / aspect) * cols)))];
      let v = 0;
      for (const d of column) {
        const head = ((d.p + t * d.s) % 1) * (1.2 + d.len) - 0.1;
        const behind = head - y;
        if (behind < 0 || behind > d.len) continue;
        v = Math.max(v, behind < 0.03 ? 1 : 0.85 * (1 - behind / d.len) ** 1.5);
      }
      return v;
    };
  },

  fire({ fbm, rand, aspect }) {
    // Flames: turbulence that rises from the bottom, hottest at its base.
    const p = rand() * 16, sway = 0.3 + rand() * 0.3;
    return (x, y, t) => {
      const u = x / aspect;
      const n = fbm(x * 3.2 + Math.sin(y * 3 + t * 0.7) * sway + p, y * 2.6 + t * 1.5);
      const heat = y * 1.55 - 0.45 + (n - 0.5) * 1.5 - Math.abs(u - 0.6) * 0.35;
      return clamp01(heat) ** 1.25;
    };
  },

  contours({ noise, rand }) {
    // A map's contour lines over hills that rise and sink slowly; the higher
    // the line, the brighter it is.
    const p = rand() * 16, q = rand() * 16, bands = 6 + Math.floor(rand() * 3);
    return (x, y, t) => {
      // Broad hills only: fine detail would turn the lines into noise.
      const raw = noise(x * 1.3 + p + t * 0.06, y * 1.3 + q - t * 0.04) * 0.75 + noise(x * 2.6 + q, y * 2.6 + p + t * 0.05) * 0.25;
      const h = clamp01((raw - 0.18) / 0.64);
      const level = h * bands, edge = Math.abs(level - Math.round(level));
      const line = Math.max(0, 1 - edge / 0.2);
      return Math.max(line * (0.45 + 0.55 * h), h ** 3 * 0.5);
    };
  },

  moire({ rand, aspect }) {
    // Two gratings of fine lines laid over each other and turning at
    // different speeds: the broad bands are the moiré between them.
    const cx = 0.55 * aspect, cy = 0.5, freq = 34 + rand() * 10, p = rand() * TAU;
    return (x, y, t) => {
      const a1 = p + t * 0.11, a2 = p + 0.22 + t * 0.04;
      const dx = x - cx, dy = y - cy;
      const g1 = 0.5 + 0.5 * Math.cos((dx * Math.cos(a1) + dy * Math.sin(a1)) * freq);
      const g2 = 0.5 + 0.5 * Math.cos((dx * Math.cos(a2) + dy * Math.sin(a2)) * freq);
      const fade = clamp01(1.2 - Math.hypot(dx, dy) * 1.3);
      return (g1 * g2) ** 0.9 * fade;
    };
  },

  lissajous(kit) {
    // An oscilloscope trace: a bright point drawing a Lissajous figure, its
    // trail fading behind it, while the figure itself slowly turns.
    const { rand } = kit;
    const { cols, rows } = kit.grid;
    const ratios = [[3, 2], [5, 4], [3, 4], [5, 6], [1, 2], [2, 3]];
    const [fa, fb] = ratios[Math.floor(rand() * ratios.length)];
    const cx = cols * (0.52 + rand() * 0.08), cy = rows * 0.5, rx = rows * 0.62, ry = rows * 0.4;
    const plot = (out: Float32Array, px: number, py: number, w: number) => {
      const i0 = Math.floor(px - 1.5), j0 = Math.floor(py - 1.5);
      for (let j = j0; j <= j0 + 3; j++) {
        for (let i = i0; i <= i0 + 3; i++) {
          if (i < 0 || j < 0 || i >= cols || j >= rows) continue;
          const v = w * Math.max(0, 1 - Math.hypot(i + 0.5 - px, j + 0.5 - py) / 1.4);
          if (v > out[j * cols + i]) out[j * cols + i] = v;
        }
      }
    };
    return perFrame(kit, (t, out) => {
      const delta = t * 0.25;
      const at = (s: number): [number, number] => [cx + Math.sin(fa * s + delta) * rx, cy + Math.sin(fb * s) * ry];
      // The whole figure, faintly.
      for (let s = 0; s < TAU; s += 0.01) plot(out, ...at(s), 0.32);
      // The trail behind the moving point.
      const head = t * 1.2;
      for (let k = 0; k < 160; k++) plot(out, ...at(head - k * 0.012), 1 - (k / 160) * 0.6);
    });
  },

  voronoi({ rand, aspect }) {
    // Stained glass: panes around points that drift, lead lines between
    // them, and each pane its own shade.
    const points = Array.from({ length: 12 }, () => ({
      x: rand() * aspect, y: rand(), sx: 0.2 + rand() * 0.3, sy: 0.2 + rand() * 0.3,
      p: rand() * TAU, shade: 0.12 + rand() * 0.45,
    }));
    return (x, y, t) => {
      let d1 = Infinity, d2 = Infinity, shade = 0;
      for (const pt of points) {
        const px = pt.x + Math.sin(t * pt.sx + pt.p) * 0.12;
        const py = pt.y + Math.cos(t * pt.sy + pt.p * 1.7) * 0.1;
        const d = Math.hypot(x - px, y - py);
        if (d < d1) {
          d2 = d1;
          d1 = d;
          shade = pt.shade;
        } else if (d < d2) d2 = d;
      }
      const edge = Math.max(0, 1 - (d2 - d1) / 0.05);
      return Math.max(edge, shade * clamp01(1 - d1 * 2.2), d1 < 0.02 ? 1 : 0);
    };
  },

  equalizer(kit) {
    // A music visualiser: bars that jump to a beat, low notes on the left,
    // and a dot above each bar that falls back slowly from its peak.
    const { rand, aspect } = kit;
    const { cols } = kit.grid;
    const bars = Math.floor(cols / 3);
    const voices = Array.from({ length: bars }, (_, b) => ({
      f1: 1.4 + rand() * 2.2, f2: 2.5 + rand() * 3.5, p: rand() * TAU, q: rand() * TAU,
      top: 1 - (b / bars) * 0.35,
    }));
    const height = (b: number, t: number) => {
      const v = voices[b];
      const beat = 0.7 + 0.3 * Math.max(0, Math.sin(t * 3.2)) ** 6;
      const level = 0.5 + 0.3 * Math.sin(t * v.f1 + v.p) + 0.2 * Math.sin(t * v.f2 + v.q);
      return 0.06 + v.top * level * beat;
    };
    // Every dot of a frame asks for the same time: work out the bars once.
    const levels = new Float32Array(bars), peaks = new Float32Array(bars);
    let built = NaN;
    return (x, y, t) => {
      if (t !== built) {
        for (let b = 0; b < bars; b++) {
          levels[b] = peaks[b] = height(b, t);
          for (let k = 1; k <= 8; k++) peaks[b] = Math.max(peaks[b], height(b, t - k * 0.06) - k * k * 0.004);
        }
        built = t;
      }
      const i = Math.min(cols - 1, Math.max(0, Math.floor((x / aspect) * cols)));
      if (i % 3 === 2) return 0;
      const b = Math.min(bars - 1, Math.floor(i / 3));
      const h = levels[b], peak = peaks[b], up = 1 - y;
      if (Math.abs(up - peak - 0.035) < 0.016) return 0.95;
      if (up > h) return 0;
      return 0.3 + 0.7 * (up / h) ** 1.4;
    };
  },
};

/**
 * A post's picture as a field: for a point (x across, y down, in a space
 * one unit tall and `aspect` wide) and a time in seconds, how strongly it
 * lights up, from 0 to 1. Everything random is fixed by the seed, so time
 * is the only thing that changes the picture. `grid` is the grid of dots it
 * will be drawn on; shapes that work cell by cell need it.
 */
export function artField(id: string, options: ArtOptions, aspect: number, grid?: Grid) {
  const { seed, palette, shape } = resolveArt(id, options);
  const rand = random(hashOf(seed));

  // Value noise, for a little grain in every shape.
  const lattice = Array.from({ length: 256 }, rand);
  const at = (x: number, y: number) => lattice[((y & 15) << 4) | (x & 15)];
  const noise = (x: number, y: number) => {
    const xi = Math.floor(x), yi = Math.floor(y);
    const u = smooth(x - xi), v = smooth(y - yi);
    const a = at(xi, yi), b = at(xi + 1, yi), c = at(xi, yi + 1), d = at(xi + 1, yi + 1);
    return a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v;
  };
  const fbm = (x: number, y: number) =>
    (noise(x, y) * 0.5 + noise(x * 2.03 + 5.2, y * 2.03 + 1.3) * 0.25 + noise(x * 4.01 + 9.7, y * 4.01 + 3.1) * 0.125) /
    0.875;

  const rows = grid?.rows ?? 40;
  const field = builders[shape]({
    seed,
    rand,
    noise,
    fbm,
    aspect,
    grid: grid ?? { cols: Math.round(rows * aspect), rows },
  });

  return {
    colours: palettes[palette],
    /** The field plus a fixed grain, so the dots don't look machine-even. */
    at: (x: number, y: number, t: number) =>
      Math.min(1, Math.max(0, field(x, y, t) + (noise(x * 7, y * 7) - 0.5) * 0.18)),
  };
}

export const SIZES = 6;
/** A dot's colour index and size step for a field value, or null for none. */
export function dotFor(f: number, shades: number) {
  if (f < 0.12) return null;
  return {
    shade: Math.min(shades - 1, Math.floor(f * shades * 0.999)),
    level: Math.min(SIZES, Math.max(1, Math.round(f * SIZES))),
  };
}
/** A dot's diameter, in grid cells, for a size step. */
export const dotWidth = (level: number) => 0.2 + (level / SIZES) * 0.66;

/** The still picture, at time 0, as SVG paths. */
export function drawArt(id: string, options: ArtOptions, cols: number, rows: number): Art {
  const aspect = cols / rows;
  const { colours, at } = artField(id, options, aspect, { cols, rows });
  const buckets = new Map<string, { color: string; width: number; d: string[] }>();
  for (let j = 0; j < rows; j++) {
    for (let i = 0; i < cols; i++) {
      const dot = dotFor(at(((i + 0.5) / cols) * aspect, (j + 0.5) / rows, 0), colours.length);
      if (!dot) continue;
      const key = `${dot.shade}:${dot.level}`;
      const bucket = buckets.get(key) ?? { color: colours[dot.shade], width: dotWidth(dot.level), d: [] };
      bucket.d.push(`M${i + 0.5} ${j + 0.5}h0`);
      buckets.set(key, bucket);
    }
  }
  return {
    cols,
    rows,
    groups: [...buckets.values()]
      .sort((a, b) => a.width - b.width)
      .map((b) => ({ color: b.color, width: Number(b.width.toFixed(3)), d: b.d.join('') })),
    ring: colours[0],
  };
}

/** Colours for the soft glow behind a post's hero, and where its blobs sit. */
export function glowOf(id: string, options: ArtOptions = {}) {
  const { seed, palette } = resolveArt(id, options);
  const rand = random(hashOf(seed) ^ 0x9e3779b9);
  const [, mid, bright, light] = palettes[palette];
  return {
    palette,
    style: [
      `--glow-a:${bright}`,
      `--glow-b:${mid}`,
      `--glow-c:${light}`,
      `--glow-ax:${Math.round(62 + rand() * 22)}%`,
      `--glow-ay:${Math.round(18 + rand() * 30)}%`,
      `--glow-bx:${Math.round(22 + rand() * 30)}%`,
      `--glow-by:${Math.round(25 + rand() * 35)}%`,
    ].join(';'),
  };
}
