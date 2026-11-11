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
} as const;
export type PaletteName = keyof typeof palettes;
export const paletteNames = Object.keys(palettes) as PaletteName[];

export const shapes = ['blobs', 'ripple', 'waves', 'sphere', 'swirl', 'interference', 'aurora', 'plasma', 'orbits'] as const;
export type Shape = (typeof shapes)[number];

export type ArtOptions = { palette?: PaletteName; shape?: Shape; seed?: string };

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

/**
 * A post's picture as a field: for a point (x across, y down, in a space
 * one unit tall and `aspect` wide) and a time in seconds, how strongly it
 * lights up, from 0 to 1. Everything random is fixed by the seed, so time
 * is the only thing that changes the picture.
 */
export function artField(id: string, options: ArtOptions, aspect: number) {
  const { seed, palette, shape } = resolveArt(id, options);
  const rand = random(hashOf(seed));

  // Value noise, for a little grain in every shape.
  const lattice = Array.from({ length: 256 }, rand);
  const at = (x: number, y: number) => lattice[((y & 15) << 4) | (x & 15)];
  const ease = (v: number) => v * v * (3 - 2 * v);
  const noise = (x: number, y: number) => {
    const xi = Math.floor(x), yi = Math.floor(y);
    const u = ease(x - xi), v = ease(y - yi);
    const a = at(xi, yi), b = at(xi + 1, yi), c = at(xi, yi + 1), d = at(xi + 1, yi + 1);
    return a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v;
  };

  let field: (x: number, y: number, t: number) => number;
  if (shape === 'blobs') {
    // Metaballs, each circling slowly on its own path.
    const balls = Array.from({ length: 3 + Math.floor(rand() * 2) }, () => ({
      x: (0.2 + rand() * 0.6) * aspect, y: 0.2 + rand() * 0.6, r: 0.16 + rand() * 0.16,
      sx: 0.25 + rand() * 0.35, sy: 0.2 + rand() * 0.3, p: rand() * 6.28,
    }));
    field = (x, y, t) => {
      let sum = 0;
      for (const b of balls) {
        const bx = b.x + Math.sin(t * b.sx + b.p) * 0.12 * aspect;
        const by = b.y + Math.cos(t * b.sy + b.p * 1.3) * 0.1;
        sum += (b.r * b.r) / ((x - bx) ** 2 + (y - by) ** 2 + 0.002);
      }
      return Math.min(1, Math.max(0, (sum - 0.55) / 1.6));
    };
  } else if (shape === 'ripple') {
    // Rings that spread out from a point.
    const cx = (0.3 + rand() * 0.4) * aspect, cy = 0.3 + rand() * 0.4, freq = 26 + rand() * 14;
    field = (x, y, t) => {
      const d = Math.hypot(x - cx, y - cy);
      const wave = 0.5 + 0.5 * Math.cos(d * freq - t * 2.4);
      return Math.max(0, wave * Math.max(0, 1 - d / 0.75) ** 1.2);
    };
  } else if (shape === 'waves') {
    // Lines along sine curves that travel sideways, each at its own speed.
    const lines = Array.from({ length: 3 }, (_, i) => ({
      y: 0.3 + i * 0.2 + (rand() - 0.5) * 0.08, amp: 0.05 + rand() * 0.08, freq: 3 + rand() * 4,
      phase: rand() * 6.28, speed: (0.5 + rand() * 0.6) * (i % 2 ? -1 : 1),
    }));
    field = (x, y, t) => {
      let best = 0;
      for (const l of lines) {
        const d = Math.abs(y - (l.y + Math.sin((x / aspect) * l.freq + l.phase - t * l.speed) * l.amp));
        best = Math.max(best, Math.max(0, 1 - d / 0.09));
      }
      return best ** 1.4;
    };
  } else if (shape === 'swirl') {
    // A spiral galaxy: arms wound around a centre, turning slowly.
    const cx = (0.42 + rand() * 0.16) * aspect, cy = 0.42 + rand() * 0.16;
    const arms = 2 + Math.floor(rand() * 2), twist = 9 + rand() * 6, turn = rand() < 0.5 ? -1 : 1;
    field = (x, y, t) => {
      const dx = x - cx, dy = y - cy, d = Math.hypot(dx, dy);
      const arm = 0.5 + 0.5 * Math.cos(arms * Math.atan2(dy, dx) + d * twist * turn - t * 0.9);
      const core = Math.max(0, 1 - d / 0.09);
      return Math.max(core, arm ** 2 * Math.max(0, 1 - d / 0.62) * 1.15);
    };
  } else if (shape === 'interference') {
    // Two sources of ripples drifting apart and together; where their
    // rings meet they add up or cancel out.
    const cy = 0.4 + rand() * 0.2, cx = (0.4 + rand() * 0.2) * aspect, gap = 0.18 + rand() * 0.1;
    const freq = 20 + rand() * 8, p = rand() * 6.28;
    field = (x, y, t) => {
      const sway = gap + Math.sin(t * 0.3 + p) * 0.06;
      const a = Math.hypot(x - cx + sway, y - cy), b = Math.hypot(x - cx - sway, y - cy);
      const wave = (Math.cos(a * freq - t * 2.2) + Math.cos(b * freq - t * 2.2)) / 2;
      const fade = Math.max(0, 1 - Math.hypot(x - cx, y - cy) / 0.8);
      return (0.5 + 0.5 * wave) ** 1.6 * fade ** 0.7;
    };
  } else if (shape === 'aurora') {
    // Northern lights: a curtain that sways, bright at its edge and fading
    // upward, with folds running down it.
    const base = 0.6 + rand() * 0.1, p = rand() * 6.28, folds = 9 + rand() * 5;
    field = (x, y, t) => {
      const u = x / aspect;
      const edge = base + Math.sin(u * 4 + p + t * 0.5) * 0.08 + Math.sin(u * 9 - t * 0.8 + p * 2) * 0.03;
      const above = edge - y;
      if (above < 0) return Math.max(0, 0.9 + above * 12);
      const glow = Math.exp(-above * 4.2);
      const fold = 0.7 + 0.3 * Math.sin(u * folds + Math.sin(u * 2.5 + t * 0.6) * 1.5 + p);
      return Math.min(1, glow * fold * 1.1);
    };
  } else if (shape === 'plasma') {
    // The old demo-scene plasma: sines over x, y and distance, added up.
    const k = 5 + rand() * 4, p = rand() * 6.28, cx = rand() * aspect, cy = rand();
    field = (x, y, t) => {
      const v =
        Math.sin(x * k + t * 0.7 + p) +
        Math.sin(y * k * 1.3 - t * 0.5) +
        Math.sin((x + y) * k * 0.7 + t * 0.4) +
        Math.sin(Math.hypot(x - cx, y - cy) * k * 1.6 - t * 0.9);
      return Math.max(0, (v / 4 + 0.5) ** 2.4 * 1.6 - 0.2);
    };
  } else if (shape === 'orbits') {
    // A small system: a bright body in the middle and moons on tilted
    // ellipses around it, each at its own speed.
    const cx = (0.42 + rand() * 0.16) * aspect, cy = 0.45 + rand() * 0.1;
    const moons = Array.from({ length: 3 + Math.floor(rand() * 2) }, (_, i) => ({
      rx: 0.26 + i * 0.13 + rand() * 0.04, tilt: 0.4 + rand() * 0.25, size: 0.06 + rand() * 0.035,
      speed: (0.9 - i * 0.15) * (rand() < 0.5 ? -1 : 1), p: rand() * 6.28,
    }));
    field = (x, y, t) => {
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
  } else {
    // A ball lit from a light that circles around it.
    const cx = (0.4 + rand() * 0.2) * aspect, cy = 0.45 + rand() * 0.1, r = 0.36, p = rand() * 6.28;
    field = (x, y, t) => {
      const dx = (x - cx) / r, dy = (y - cy) / r, dd = dx * dx + dy * dy;
      if (dd > 1) return Math.max(0, 0.22 - (Math.sqrt(dd) - 1) * 0.9);
      const a = t * 0.35 + p;
      const lx = Math.cos(a) * 0.75, ly = -0.45, lz = 0.5 + Math.sin(a) * 0.3;
      const dz = Math.sqrt(1 - dd);
      return Math.max(0.08, (dx * lx + dy * ly + dz * lz) / Math.hypot(lx, ly, lz)) ** 1.1;
    };
  }

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
  const { colours, at } = artField(id, options, aspect);
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
