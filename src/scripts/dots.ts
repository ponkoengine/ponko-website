// One field of dots behind the whole page. The same few thousand dots are
// rearranged into a different picture for each part of the page, and the
// scroll position blends one picture into the next. Plain 2D canvas, no
// libraries, nothing to download except the logo that is already on the page.

// Cream, orange, dim brown, deep orange, warm grey.
const PALETTE = ['#f4e9db', '#f0672f', '#5e4a42', '#b5441d', '#a39488'];
const CREAM = 0, ORANGE = 1, DIM = 2, DEEP = 3, GREY = 4;

type Fill = (t: number, X: Float32Array, Y: Float32Array, S: Float32Array, C: Uint8Array, ar: number) => void;
// Where a scene sits: beside the text, across the whole screen, or centred.
type Place = 'side' | 'full' | 'centre';
// `glow`: the scene is drawn faint, and the dots light up around the cursor
// instead of moving out of its way.
type Scene = { place: Place; alpha: number; fill: Fill; glow?: boolean };

export type Frame = { index: number; between: number; count: number };

export function dots(canvas: HTMLCanvasElement, still: boolean, onFrame?: (f: Frame) => void) {
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  const small = matchMedia('(max-width: 900px)');
  const N = small.matches ? 3200 : 7200;

  // A fixed set of random numbers per dot, so every scene is repeatable.
  let s = 1234567;
  const rand = () => {
    s = (s + 0x6d2b79f5) | 0;
    let x = Math.imul(s ^ (s >>> 15), 1 | s);
    x = (x + Math.imul(x ^ (x >>> 7), 61 | x)) ^ x;
    return ((x ^ (x >>> 14)) >>> 0) / 4294967296;
  };
  const R1 = Float32Array.from({ length: N }, rand);
  const R2 = Float32Array.from({ length: N }, rand);
  const R3 = Float32Array.from({ length: N }, rand);
  const R4 = Float32Array.from({ length: N }, rand);

  const hash = (x: number, y: number) => {
    let h = (x * 374761393 + y * 668265263 + 1013) | 0;
    h = Math.imul(h ^ (h >>> 13), 1274126177);
    return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
  };
  const smooth = (v: number) => v * v * (3 - 2 * v);
  const noise = (x: number, y: number) => {
    const xi = Math.floor(x), yi = Math.floor(y);
    const u = smooth(x - xi), v = smooth(y - yi);
    const a = hash(xi, yi), b = hash(xi + 1, yi), c = hash(xi, yi + 1), d = hash(xi + 1, yi + 1);
    return a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v;
  };
  const fbm = (x: number, y: number) => {
    let sum = 0, amp = 0.5, f = 1;
    for (let o = 0; o < 5; o++) {
      sum += noise(x * f, y * f) * amp;
      amp *= 0.5;
      f *= 2.03;
    }
    return sum;
  };

  // Turns a drawing into dots on a regular grid: one dot per opaque pixel.
  // The drawing is scaled so it has about `want` opaque pixels.
  type Points = { x: Float32Array; y: Float32Array; c: Uint8Array; n: number };
  function raster(aspect: number, want: number, paint: (c: CanvasRenderingContext2D, w: number, h: number) => void, colour: (r: number, g: number, b: number) => number): Points {
    const off = document.createElement('canvas');
    const o = off.getContext('2d', { willReadFrequently: true })!;
    let w = Math.round(Math.sqrt((want / 0.4) * aspect));
    let found: number[] = [];
    for (let pass = 0; pass < 3; pass++) {
      const h = Math.max(2, Math.round(w / aspect));
      off.width = w;
      off.height = h;
      o.clearRect(0, 0, w, h);
      paint(o, w, h);
      const px = o.getImageData(0, 0, w, h).data;
      found = [];
      for (let j = 0; j < h; j++) {
        for (let i = 0; i < w; i++) {
          const k = (j * w + i) * 4;
          if (px[k + 3] > 110) found.push(((i + 0.5) / w) * 2 - 1, (((j + 0.5) / h) * 2 - 1) / aspect, colour(px[k], px[k + 1], px[k + 2]));
        }
      }
      const got = found.length / 3;
      if (!got || Math.abs(got - want) / want < 0.04) break;
      w = Math.max(8, Math.round(w * Math.sqrt(want / got)));
    }
    const got = found.length / 3;
    const order = Array.from({ length: got }, (_, i) => i).sort((a, b) => hash(a, 7) - hash(b, 7));
    const n = Math.min(got, want);
    const p: Points = { x: new Float32Array(n), y: new Float32Array(n), c: new Uint8Array(n), n };
    for (let i = 0; i < n; i++) {
      const k = order[i] * 3;
      p.x[i] = found[k], p.y[i] = found[k + 1], p.c[i] = found[k + 2];
    }
    return p;
  }

  // ---- The scenes ----------------------------------------------------------

  // The logo. Until the artwork has loaded, a plain ring.
  let logo: Points | null = null;
  const logoFill: Fill = (t, X, Y, S, C) => {
    for (let i = 0; i < N; i++) {
      if (logo && i < logo.n) {
        X[i] = logo.x[i] * 0.98;
        Y[i] = logo.y[i] * 0.98 + Math.sin(t * 0.9 + logo.x[i] * 2.2) * 0.012;
        S[i] = 0.92 + 0.12 * Math.sin(t * 1.3 + R1[i] * 6.28);
        C[i] = logo.c[i];
      } else {
        const a = R1[i] * 6.2832 + t * 0.05, r = logo ? 1.05 + R2[i] * 0.5 : 0.5 + R2[i] * 0.1;
        X[i] = Math.cos(a) * r;
        Y[i] = Math.sin(a) * r * 0.8;
        S[i] = logo ? 0.22 * R3[i] : 0.5;
        C[i] = DIM;
      }
    }
  };

  // Entities: streams of units that walk around three towers.
  const TOWERS = [[-0.3, -0.34, 0.15], [0.32, 0.06, 0.17], [-0.12, 0.5, 0.13]];
  const towerDots = Math.floor(N * 0.07);
  const swarm: Fill = (t, X, Y, S, C) => {
    for (let i = 0; i < N; i++) {
      if (i < towerDots) {
        const tw = TOWERS[i % 3];
        const ring = R2[i] < 0.5 ? 0.42 : 0.72;
        const a = R1[i] * 6.2832 + t * (ring < 0.5 ? 0.6 : -0.35);
        X[i] = tw[0] + Math.cos(a) * tw[2] * ring;
        Y[i] = tw[1] + Math.sin(a) * tw[2] * ring;
        S[i] = 0.75 + 0.25 * Math.sin(t * 3 + (i % 3) * 2);
        C[i] = CREAM;
        continue;
      }
      let x = ((R1[i] * 2.7 + t * (0.07 + R3[i] * 0.09)) % 2.7) - 1.35;
      let y = (R2[i] * 2 - 1) * 0.86 + 0.05 * Math.sin(x * 4 + R4[i] * 6.2832 + t * 0.8);
      for (const tw of TOWERS) {
        const dx = x - tw[0], dy = y - tw[1];
        const d = Math.hypot(dx, dy) || 0.001, keep = tw[2] * 1.35;
        if (d < keep) {
          x = tw[0] + (dx / d) * keep;
          y = tw[1] + (dy / d) * keep;
        }
      }
      X[i] = x, Y[i] = y;
      S[i] = (0.75 + R3[i] * 0.45) * Math.sqrt(Math.max(0, 1 - Math.abs(x) / 1.35));
      C[i] = R4[i] < 0.1 ? CREAM : R4[i] < 0.3 ? DEEP : ORANGE;
    }
  };

  // Rendering: a small island as a grid of points, slowly turning.
  const G = Math.floor(Math.sqrt(N));
  const TH = new Float32Array(N), TS = new Float32Array(N), TC = new Uint8Array(N);
  {
    const height = (u: number, v: number) => Math.max(0, (fbm(u * 2.1 + 10, v * 2.1 + 10) - 0.3) * 1.7 - (u * u + v * v) * 0.95 + 0.28);
    for (let i = 0; i < G * G; i++) {
      const u = ((i % G) / (G - 1)) * 2 - 1, v = (Math.floor(i / G) / (G - 1)) * 2 - 1;
      const h = height(u, v), e = 0.02;
      const nx = height(u - e, v) - height(u + e, v), nz = height(u, v - e) - height(u, v + e);
      const lit = Math.max(0, (nx * -0.45 + 2 * e * 0.8 + nz * 0.4) / Math.hypot(nx, 2 * e, nz));
      TH[i] = h * 0.55;
      if (h <= 0) TS[i] = 0.26, TC[i] = DIM;
      else TS[i] = 0.5 + 0.7 * lit, TC[i] = h > 0.42 ? CREAM : lit > 0.62 ? ORANGE : DEEP;
    }
  }
  const terrain: Fill = (t, X, Y, S, C) => {
    const yaw = t * 0.22, cy = Math.cos(yaw), sy = Math.sin(yaw), cp = Math.cos(0.6), sp = Math.sin(0.6);
    for (let i = 0; i < N; i++) {
      if (i >= G * G) {
        X[i] = 0, Y[i] = 0.2, S[i] = 0, C[i] = DIM;
        continue;
      }
      const u = ((i % G) / (G - 1)) * 2 - 1, v = (Math.floor(i / G) / (G - 1)) * 2 - 1;
      const y = TH[i] > 0 ? TH[i] : 0.012 * Math.sin(t * 1.5 + u * 9 + v * 6);
      const x1 = u * cy - v * sy, z1 = u * sy + v * cy;
      const y2 = y * cp + z1 * sp, z2 = -y * sp + z1 * cp + 3.4;
      const f = 3.4 / z2;
      X[i] = x1 * f * 0.98;
      Y[i] = -y2 * f * 0.98 + 0.08;
      S[i] = TS[i] * f;
      C[i] = TC[i];
    }
  };

  // Physics: crates that drop into a pile, over and over.
  const CRATES: number[][] = [];
  for (let row = 0, n = 4; n > 0; row++, n--) for (let k = 0; k < n; k++) CRATES.push([(k - (n - 1) / 2) * 0.4, 0.6 - row * 0.38]);
  const groundDots = Math.floor(N * 0.06);
  const perCrate = Math.floor((N - groundDots) / CRATES.length);
  const side = Math.floor(Math.sqrt(perCrate));
  const stack: Fill = (t, X, Y, S, C) => {
    const tau = still ? 8 : t % 11;
    for (let i = 0; i < N; i++) {
      if (i < groundDots) {
        X[i] = (i / groundDots) * 2.6 - 1.3;
        Y[i] = 0.8 + (i % 2) * 0.016;
        S[i] = 0.34, C[i] = DIM;
        continue;
      }
      const k = i - groundDots, j = k % CRATES.length, cell = Math.floor(k / CRATES.length);
      if (cell >= side * side) {
        X[i] = CRATES[j][0], Y[i] = CRATES[j][1], S[i] = 0, C[i] = DIM;
        continue;
      }
      const cx = cell % side, cz = Math.floor(cell / side);
      const edge = cx === 0 || cz === 0 || cx === side - 1 || cz === side - 1;
      const land = 0.9 + j * 0.55, since = tau - land;
      let lift = 0, size = edge ? 0.9 : 0.42;
      if (since < -0.55) lift = -2.6, size = 0;
      else if (since < 0) lift = -((since / 0.55) ** 2) * 1.9;
      else lift = -Math.abs(Math.sin(since * 10)) * 0.07 * Math.exp(-since * 5);
      if (tau > 10) size *= 11 - tau;
      X[i] = CRATES[j][0] + (cx / (side - 1) - 0.5) * 0.34;
      Y[i] = CRATES[j][1] + (cz / (side - 1) - 0.5) * 0.34 + lift;
      S[i] = size;
      C[i] = j % 3 === 0 ? CREAM : j % 3 === 1 ? ORANGE : DEEP;
    }
  };

  // Inspect: a readout from a running game, written in dots.
  let readout: Points | null = null;
  const report: Fill = (t, X, Y, S, C) => {
    for (let i = 0; i < N; i++) {
      if (readout && i < readout.n) {
        X[i] = readout.x[i], Y[i] = readout.y[i];
        S[i] = 0.72 + 0.2 * Math.sin(t * 2.4 - readout.y[i] * 7);
        C[i] = readout.c[i];
      } else {
        X[i] = R1[i] * 2 - 1, Y[i] = R2[i] * 1.6 - 0.8, S[i] = 0, C[i] = DIM;
      }
    }
  };

  // Ship: a turning globe.
  const GX = new Float32Array(N), GY = new Float32Array(N), GZ = new Float32Array(N), GC = new Uint8Array(N);
  for (let i = 0; i < N; i++) {
    const y = 1 - (2 * (i + 0.5)) / N, r = Math.sqrt(1 - y * y), a = i * 2.39996323;
    GX[i] = Math.cos(a) * r, GY[i] = y, GZ[i] = Math.sin(a) * r;
    const land = fbm(Math.atan2(GZ[i], GX[i]) * 1.3 + 20, y * 2.4 + 20);
    GC[i] = land > 0.5 ? ORANGE : land > 0.46 ? DEEP : CREAM;
  }
  const globe: Fill = (t, X, Y, S, C) => {
    const yaw = t * 0.25, cy = Math.cos(yaw), sy = Math.sin(yaw), ct = Math.cos(0.35), st = Math.sin(0.35);
    for (let i = 0; i < N; i++) {
      const x1 = GX[i] * cy - GZ[i] * sy, z1 = GX[i] * sy + GZ[i] * cy;
      const y2 = GY[i] * ct - z1 * st, z2 = GY[i] * st + z1 * ct;
      X[i] = x1 * 0.86, Y[i] = -y2 * 0.86;
      const front = z2 < 0;
      S[i] = front ? (GC[i] === CREAM ? 0.3 : 0.55 + 0.5 * -z2) : 0.16;
      C[i] = front ? GC[i] : DIM;
    }
  };

  // Behind the sections that have their own pictures: a slow, sparse drift.
  const drift: Fill = (t, X, Y, S, C, ar) => {
    for (let i = 0; i < N; i++) {
      X[i] = (R1[i] * 2 - 1) * ar;
      Y[i] = ((R2[i] * 2 + t * 0.012 * (0.4 + R3[i])) % 2) - 1;
      S[i] = 0.18 + R4[i] * 0.34;
      C[i] = R3[i] < 0.06 ? ORANGE : DIM;
    }
  };

  // The end: the name, standing over a floor of waves.
  let word: Points | null = null;
  const wordDots = Math.floor(N * 0.5);
  const floorDots = N - wordDots;
  const cols = Math.floor(Math.sqrt(floorDots * 2.6)), rows = Math.floor(floorDots / cols);
  const finale: Fill = (t, X, Y, S, C) => {
    for (let i = 0; i < N; i++) {
      if (i < wordDots) {
        if (word && i < word.n) {
          X[i] = word.x[i] * 0.8, Y[i] = word.y[i] * 0.8 + 0.27;
          S[i] = 0.8 + 0.22 * Math.sin(t * 1.6 + word.x[i] * 4);
          C[i] = word.c[i];
        } else X[i] = 0, Y[i] = 0.27, S[i] = 0, C[i] = DIM;
        continue;
      }
      const k = i - wordDots, c = k % cols, r = Math.floor(k / cols);
      if (r >= rows) {
        X[i] = 0, Y[i] = 0.6, S[i] = 0, C[i] = DIM;
        continue;
      }
      const wx = (c / (cols - 1) - 0.5) * 15, wz = 1.25 + (r / (rows - 1)) ** 1.4 * 8;
      const h = 0.24 * Math.sin(wx * 0.85 + t * 0.9) * Math.cos(wz * 0.8 - t * 0.55) + 0.1 * Math.sin(wx * 2.1 - t * 1.3 + wz);
      X[i] = (wx / wz) * 0.62;
      Y[i] = 0.42 + ((0.95 - h) / wz) * 0.98;
      S[i] = Math.min(1.5, (1.45 / wz) * (0.55 + Math.max(0, h) * 3.2));
      C[i] = h > 0.12 ? GREY : DIM;
    }
  };

  const SCENES: Record<string, Scene> = {
    logo: { place: 'side', alpha: 1, fill: logoFill },
    swarm: { place: 'side', alpha: 1, fill: swarm },
    terrain: { place: 'side', alpha: 1, fill: terrain },
    stack: { place: 'side', alpha: 1, fill: stack },
    report: { place: 'side', alpha: 1, fill: report },
    globe: { place: 'side', alpha: 1, fill: globe },
    drift: { place: 'full', alpha: 0.75, fill: drift },
    // The end is quieter than the rest: no orange, faint until the cursor
    // comes near.
    finale: { place: 'centre', alpha: 0.14, fill: finale, glow: true },
  };

  // The logo and the lettering need the artwork and the fonts first.
  fetch('/brand/ponko-face.svg')
    .then((r) => r.text())
    .then((svg) => {
      const img = new Image();
      img.onload = () => {
        logo = raster(720 / 560, Math.floor(N * 0.94), (c, w, h) => c.drawImage(img, 0, 0, w, h), (r, g) => (r > 215 && g > 190 ? CREAM : r > 150 ? ORANGE : DIM));
      };
      img.src = URL.createObjectURL(new Blob([svg.replace('<svg ', '<svg width="720" height="560" ')], { type: 'image/svg+xml' }));
    })
    .catch(() => {});
  const letter = () => {
    word = raster(2.9, wordDots, (c, w, h) => {
      c.fillStyle = '#fff';
      c.textAlign = 'center';
      c.textBaseline = 'middle';
      // As big as fits: the full height, unless the word is too wide for it.
      c.font = `600 ${h}px Jost, sans-serif`;
      const size = Math.min(h * 1.12, (h * w * 0.94) / c.measureText('Ponko').width);
      c.font = `600 ${size}px Jost, sans-serif`;
      c.fillText('Ponko', w / 2, h * 0.5);
    }, () => CREAM);
    const lines = ['> ponko inspect', 'tick      48,211', 'entities  12,000', 'sim       0.41 ms', 'draws     1'];
    readout = raster(1.5, Math.floor(N * 0.9), (c, w, h) => {
      c.textBaseline = 'middle';
      c.font = `500 ${w / 10.4}px "JetBrains Mono", monospace`;
      lines.forEach((line, i) => {
        c.fillStyle = i ? '#fff' : '#f00';
        c.fillText(line, w * 0.02, (h * (i + 0.7)) / (lines.length + 0.4));
      });
    }, (r, g) => (g < 120 ? ORANGE : CREAM));
  };
  letter();
  document.fonts?.ready.then(letter);

  // ---- Drawing ---------------------------------------------------------------

  const AX = new Float32Array(N), AY = new Float32Array(N), AS = new Float32Array(N), AC = new Uint8Array(N);
  const BX = new Float32Array(N), BY = new Float32Array(N), BS = new Float32Array(N), BC = new Uint8Array(N);
  const PX = new Float32Array(N), PY = new Float32Array(N), PR = new Float32Array(N), PC = new Uint8Array(N);
  let W = 0, H = 0, dpr = 1;
  function resize() {
    dpr = Math.min(small.matches ? 1.5 : 2, devicePixelRatio || 1);
    W = innerWidth, H = innerHeight;
    canvas.width = Math.round(W * dpr);
    canvas.height = Math.round(H * dpr);
  }
  // Centre and scale of a scene on screen.
  const rect = (place: Place): [number, number, number] =>
    place === 'full' ? [W / 2, H / 2, H / 2]
    : place === 'centre' ? [W / 2, H * 0.5, Math.min(W * 0.47, H * 0.66)]
    : small.matches ? [W / 2, H * 0.27, Math.min(W * 0.42, H * 0.22)]
    : [W * 0.73, H * 0.52, Math.min(W * 0.23, H * 0.36)];

  const anchors = [...document.querySelectorAll<HTMLElement>('[data-scene]')];
  let mx = -1e4, my = -1e4;
  addEventListener('pointermove', (e) => (mx = e.clientX, my = e.clientY), { passive: true });
  addEventListener('pointerleave', () => (mx = my = -1e4));

  function draw(t: number) {
    // Which two scenes the middle of the screen is between, and how far along.
    let index = 0, between = 0;
    for (let i = 0; i < anchors.length; i++) {
      const b = anchors[i].getBoundingClientRect();
      const centre = b.top + b.height / 2 - H / 2;
      if (centre > 0) {
        if (i > 0) {
          const pb = anchors[i - 1].getBoundingClientRect();
          const prev = pb.top + pb.height / 2 - H / 2;
          index = i - 1, between = -prev / (centre - prev);
        }
        break;
      }
      index = i, between = 0;
    }
    onFrame?.({ index, between, count: anchors.length });
    const a = SCENES[anchors[index]?.dataset.scene ?? 'drift'] ?? SCENES.drift;
    const b = SCENES[anchors[index + 1]?.dataset.scene ?? ''] ?? a;
    // Hold each picture, and change in the middle of the way between two.
    let k = a === b ? 0 : Math.min(1, Math.max(0, (between - 0.22) / 0.56));
    const ar = W / H, unit = Math.max(1.05, Math.min(2.3, W / 660));
    const [ax, ay, as] = rect(a.place);
    a.fill(t, AX, AY, AS, AC, ar);
    if (k > 0) b.fill(t, BX, BY, BS, BC, ar);
    const [bx, by, bs] = rect(b.place);
    // How much of the picture glows near the cursor, and how much of it
    // moves out of the cursor's way: one or the other.
    const ke = k * k * (3 - 2 * k);
    const glow = (a.glow ? 1 - ke : 0) + (b.glow ? ke : 0);
    const push = small.matches ? 0 : 90 * (1 - glow);

    for (let i = 0; i < N; i++) {
      let x = ax + AX[i] * as, y = ay + AY[i] * as, r = AS[i], c = AC[i];
      if (k > 0) {
        // Each dot leaves at its own moment and swings out on the way.
        const own = Math.min(1, Math.max(0, k * 1.7 - R4[i] * 0.7));
        const e = own * own * own * (own * (own * 6 - 15) + 10);
        const tx = bx + BX[i] * bs, ty = by + BY[i] * bs;
        const swing = still ? 0 : Math.sin(e * Math.PI) * (R3[i] - 0.5) * 0.7;
        x += (tx - x) * e - (ty - y) * swing;
        y = ay + AY[i] * as + (ty - (ay + AY[i] * as)) * e + (tx - (ax + AX[i] * as)) * swing;
        r += (BS[i] - r) * e;
        if (e > 0.5) c = BC[i];
      }
      if (push) {
        const dx = x - mx, dy = y - my, d2 = dx * dx + dy * dy;
        if (d2 < push * push) {
          const d = Math.sqrt(d2) || 1, f = (1 - d / push) ** 2 * 26;
          x += (dx / d) * f, y += (dy / d) * f;
        }
      }
      PX[i] = x, PY[i] = y, PR[i] = r * unit, PC[i] = c;
    }

    ctx!.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx!.clearRect(0, 0, W, H);
    ctx!.globalAlpha = a.alpha + (b.alpha - a.alpha) * ke;
    plot(ctx!);

    // The glow: the same dots at full strength on a second canvas, cut down
    // to a soft circle around the cursor, laid over the faint ones.
    if (glow > 0.01 && mx > -1e3) {
      if (lit.width !== canvas.width || lit.height !== canvas.height) {
        lit.width = canvas.width;
        lit.height = canvas.height;
      }
      const l = lit.getContext('2d')!;
      l.setTransform(dpr, 0, 0, dpr, 0, 0);
      l.globalCompositeOperation = 'source-over';
      l.globalAlpha = 1;
      l.clearRect(0, 0, W, H);
      plot(l);
      const reach = Math.min(340, Math.max(200, W * 0.2));
      const fade = l.createRadialGradient(mx, my, 0, mx, my, reach);
      fade.addColorStop(0, 'rgba(0,0,0,0.95)');
      fade.addColorStop(0.45, 'rgba(0,0,0,0.45)');
      fade.addColorStop(1, 'rgba(0,0,0,0)');
      l.globalCompositeOperation = 'destination-in';
      l.fillStyle = fade;
      l.fillRect(0, 0, W, H);
      ctx!.setTransform(1, 0, 0, 1, 0, 0);
      ctx!.globalAlpha = glow;
      ctx!.drawImage(lit, 0, 0);
    }
  }

  const lit = document.createElement('canvas');
  function plot(c2d: CanvasRenderingContext2D) {
    for (let c = 0; c < PALETTE.length; c++) {
      c2d.fillStyle = PALETTE[c];
      c2d.beginPath();
      for (let i = 0; i < N; i++) {
        if (PC[i] !== c) continue;
        const r = PR[i], x = PX[i], y = PY[i];
        if (r < 0.3 || x < -4 || x > W + 4 || y < -4 || y > H + 4) continue;
        if (r < 1.1) c2d.rect(x - r, y - r, r * 2, r * 2);
        else c2d.moveTo(x + r, y), c2d.arc(x, y, r, 0, 6.2832);
      }
      c2d.fill();
    }
  }

  resize();
  addEventListener('resize', resize);
  const t0 = performance.now();
  const frame = (now: number) => {
    requestAnimationFrame(frame);
    if (document.hidden) return;
    draw(still ? 8 : (now - t0) / 1000);
  };
  requestAnimationFrame(frame);
}
