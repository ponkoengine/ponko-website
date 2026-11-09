// A small floating island: a low-poly mesh, flat shaded and sorted back to
// front on a plain 2D canvas. No WebGL, no downloads. Drag to turn it, click
// for a new one.

const RINGS = 16;
const SECTORS = 48;
const UNDER = 6;
const DIST = 4;
const TARGET = -0.12;
const L = (() => {
  const v = [-0.45, 0.8, 0.4];
  const n = Math.hypot(v[0], v[1], v[2]);
  return v.map((x) => x / n);
})();

// water, sand, grass, forest, rock, snow, cliff, dirt, deep rock, tree
const COLORS = [
  [62, 128, 118],
  [196, 188, 128],
  [160, 214, 98],
  [96, 150, 66],
  [118, 126, 110],
  [230, 236, 220],
  [132, 110, 78],
  [92, 76, 56],
  [56, 48, 40],
  [58, 112, 60],
];
const WATER = 0, SAND = 1, GRASS = 2, FOREST = 3, ROCK = 4, SNOW = 5, CLIFF = 6, DIRT = 7, DEEP = 8, TREE = 9;
// Vertex kinds: land moves with the grow animation, water ripples, rock
// stays put, trees ride on the land vertex they stand on.
const LAND = 0, SEA = 1, STONE = 2, LEAF = 3;

export function island(canvas: HTMLCanvasElement, label: HTMLElement | null, still: boolean) {
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  let seed = (Math.random() * 0xffff) | 0;

  const hash = (x: number, y: number) => {
    let h = (x * 374761393 + y * 668265263 + seed * 982451653) | 0;
    h = Math.imul(h ^ (h >>> 13), 1274126177);
    return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
  };
  const smooth = (t: number) => t * t * (3 - 2 * t);
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

  // The mesh, rebuilt for every seed.
  let bx = new Float32Array(0), by = bx, bz = bx;
  let kind = new Uint8Array(0);
  let anchor = new Int32Array(0);
  let tris = new Int32Array(0);
  let mats = new Uint8Array(0);
  // Per frame.
  let px = bx, py = bx, pz = bx, vx = bx, vy = bx, vz = bx, sx = bx, sy = bx;
  let depth = bx;

  function build() {
    const X: number[] = [], Y: number[] = [], Z: number[] = [], K: number[] = [], A: number[] = [];
    const T: number[] = [], M: number[] = [];
    const vert = (x: number, y: number, z: number, k: number, a = -1) => {
      X.push(x), Y.push(y), Z.push(z), K.push(k), A.push(a);
      return X.length - 1;
    };
    // Each triangle faces away from a point inside the solid it belongs to.
    const tri = (a: number, b: number, c: number, m: number, ix: number, iy: number, iz: number) => {
      const ux = X[b] - X[a], uy = Y[b] - Y[a], uz = Z[b] - Z[a];
      const wx = X[c] - X[a], wy = Y[c] - Y[a], wz = Z[c] - Z[a];
      const nx = uy * wz - uz * wy, ny = uz * wx - ux * wz, nz = ux * wy - uy * wx;
      const cx = (X[a] + X[b] + X[c]) / 3 - ix, cy = (Y[a] + Y[b] + Y[c]) / 3 - iy, cz = (Z[a] + Z[b] + Z[c]) / 3 - iz;
      if (nx * cx + ny * cy + nz * cz < 0) T.push(a, c, b);
      else T.push(a, b, c);
      M.push(m);
    };
    const height = (u: number, v: number) => {
      const d = Math.hypot(u, v);
      return (fbm(u * 2.1 + 10, v * 2.1 + 10) - 0.3) * 1.7 - d * d * 0.95 + 0.28;
    };
    const top = (u: number, v: number) => {
      const h = height(u, v);
      return h > 0 ? vert(u, h * 0.5, v, LAND) : vert(u, 0, v, SEA);
    };

    // The top: rings of jittered vertices around a centre.
    const rings: number[][] = [[top(0, 0)]];
    const angles: number[] = [];
    for (let r = 1; r <= RINGS; r++) {
      const row: number[] = [];
      for (let s = 0; s < SECTORS; s++) {
        const a = ((s + (hash(r, s) - 0.5) * 0.5) / SECTORS) * Math.PI * 2;
        const rad = (r + (r < RINGS ? (hash(s, r + 50) - 0.5) * 0.45 : 0)) / RINGS;
        if (r === RINGS) angles.push(a);
        row.push(top(Math.cos(a) * rad, Math.sin(a) * rad));
      }
      rings.push(row);
    }
    const material = (a: number, b: number, c: number) => {
      const k = K[a] + K[b] + K[c];
      if (k === SEA * 3) return WATER;
      const h = (Y[a] + Y[b] + Y[c]) / 3;
      const ux = X[b] - X[a], uy = Y[b] - Y[a], uz = Z[b] - Z[a];
      const wx = X[c] - X[a], wy = Y[c] - Y[a], wz = Z[c] - Z[a];
      const nx = uy * wz - uz * wy, ny = uz * wx - ux * wz, nz = ux * wy - uy * wx;
      const slope = Math.abs(ny) / Math.hypot(nx, ny, nz);
      if (k > 0 || h < 0.035) return SAND;
      if (h > 0.4) return SNOW;
      if (h > 0.27 || (slope < 0.6 && h > 0.12)) return ROCK;
      return h < 0.13 ? GRASS : FOREST;
    };
    const face = (a: number, b: number, c: number) => tri(a, b, c, material(a, b, c), X[a], Y[a] - 5, Z[a]);
    for (let s = 0; s < SECTORS; s++) face(rings[0][0], rings[1][s], rings[1][(s + 1) % SECTORS]);
    for (let r = 2; r <= RINGS; r++) {
      for (let s = 0; s < SECTORS; s++) {
        const n = (s + 1) % SECTORS;
        const a = rings[r - 1][s], b = rings[r - 1][n], c = rings[r][s], d = rings[r][n];
        face(a, c, d);
        face(a, d, b);
      }
    }

    // Trees, on gentle low ground.
    const landCount = X.length;
    for (let i = 0; i < landCount; i++) {
      if (K[i] !== LAND || Y[i] < 0.03 || Y[i] > 0.24 || hash(i, 999) > 0.13) continue;
      const tall = 0.09 + hash(i, 998) * 0.07;
      const apex = vert(X[i], Y[i] + tall, Z[i], LEAF, i);
      const base: number[] = [];
      for (let k = 0; k < 5; k++) {
        const a = (k / 5) * Math.PI * 2 + hash(i, k);
        base.push(vert(X[i] + Math.cos(a) * 0.034, Y[i] - 0.01, Z[i] + Math.sin(a) * 0.034, LEAF, i));
      }
      for (let k = 0; k < 5; k++) tri(apex, base[k], base[(k + 1) % 5], TREE, X[i], Y[i] + tall * 0.3, Z[i]);
    }

    // The rock underneath, so the world floats.
    let prev = rings[RINGS];
    for (let k = 0; k <= UNDER; k++) {
      const row: number[] = [];
      const t = k / (UNDER + 1);
      for (let s = 0; s < SECTORS; s++) {
        const rad = k === 0 ? 1 : (1 - Math.pow(t, 1.15)) * (1 + (hash(k + 300, s) - 0.5) * 0.16);
        const a = angles[s] + (k ? (hash(s, k + 400) - 0.5) * 0.08 : 0);
        const y = k === 0 ? -0.07 : -0.07 - k * 0.13 * (0.8 + hash(k + 500, s) * 0.45);
        row.push(vert(Math.cos(a) * rad, y, Math.sin(a) * rad, STONE));
      }
      const m = k === 0 ? CLIFF : k < 3 ? DIRT : DEEP;
      for (let s = 0; s < SECTORS; s++) {
        const n = (s + 1) % SECTORS;
        tri(prev[s], row[s], row[n], m, 0, -0.35, 0);
        tri(prev[s], row[n], prev[n], m, 0, -0.35, 0);
      }
      prev = row;
    }
    const tip = vert(0, -0.07 - (UNDER + 1) * 0.15, 0, STONE);
    for (let s = 0; s < SECTORS; s++) tri(prev[s], tip, prev[(s + 1) % SECTORS], DEEP, 0, -0.35, 0);

    // Trees store their height above the vertex they stand on.
    for (let i = landCount; i < X.length; i++) if (K[i] === LEAF) Y[i] -= Y[A[i]];

    bx = Float32Array.from(X);
    by = Float32Array.from(Y);
    bz = Float32Array.from(Z);
    kind = Uint8Array.from(K);
    anchor = Int32Array.from(A);
    tris = Int32Array.from(T);
    mats = Uint8Array.from(M);
    const n = X.length;
    [px, py, pz, vx, vy, vz, sx, sy] = Array.from({ length: 8 }, () => new Float32Array(n));
    depth = new Float32Array(M.length);
    if (label) label.textContent = `seed 0x${seed.toString(16).padStart(4, '0')} · ${M.length.toLocaleString('en-US')} triangles`;
  }

  // View state.
  let W = 0, H = 0, F = 1, dpr = 1;
  let yaw = 0.6, pitch = 0.5, spin = 0;
  let grow = still ? 1 : 0, growFrom = performance.now();
  let hover = false;
  const stars = Array.from({ length: 22 }, (_, i) => ({ x: (i * 0.618 + 0.07) % 1, y: ((i * 0.377 + 0.02) % 1) * 0.9, p: i * 1.7 }));

  function resize() {
    dpr = Math.min(2, devicePixelRatio || 1);
    W = canvas.clientWidth;
    H = canvas.clientHeight;
    canvas.width = Math.round(W * dpr);
    canvas.height = Math.round(H * dpr);
    F = Math.min(W * 1.62, H * 1.62);
  }

  function draw(t: number) {
    const g = grow;
    const cy = Math.cos(yaw), sny = Math.sin(yaw), cp = Math.cos(pitch), sp = Math.sin(pitch);
    const n = bx.length;
    for (let i = 0; i < n; i++) {
      const x = bx[i], z = bz[i];
      const k = kind[i];
      const y =
        k === LAND ? by[i] * g
        : k === SEA ? 0.012 * (Math.sin(t * 1.6 + x * 9 + z * 6) + Math.sin(t * 1.1 - x * 5 + z * 11))
        : k === STONE ? by[i]
        : py[anchor[i]] + by[i] * g;
      px[i] = x, py[i] = y, pz[i] = z;
      const x1 = x * cy - z * sny, z1 = x * sny + z * cy, y0 = y - TARGET;
      const y2 = y0 * cp + z1 * sp, z2 = -y0 * sp + z1 * cp + DIST;
      vx[i] = x1, vy[i] = y2, vz[i] = z2;
      sx[i] = W / 2 + (F * x1) / z2;
      sy[i] = H / 2 - (F * y2) / z2;
    }

    ctx!.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx!.clearRect(0, 0, W, H);
    const glow = ctx!.createRadialGradient(W / 2, H * 0.5, 0, W / 2, H * 0.5, W * 0.55);
    glow.addColorStop(0, 'rgba(192,250,115,0.07)');
    glow.addColorStop(1, 'rgba(192,250,115,0)');
    ctx!.fillStyle = glow;
    ctx!.fillRect(0, 0, W, H);
    for (const s of stars) {
      const a = 0.25 + 0.35 * Math.sin(t * 1.3 + s.p);
      if (a <= 0) continue;
      ctx!.fillStyle = `rgba(192,250,115,${a.toFixed(2)})`;
      ctx!.fillRect(s.x * W, s.y * H * 0.35 + 8, 1.5, 1.5);
    }

    // Shade, cull and sort.
    const visible: number[] = [];
    const fills: string[] = [];
    for (let f = 0, m = mats.length; f < m; f++) {
      const a = tris[f * 3], b = tris[f * 3 + 1], c = tris[f * 3 + 2];
      const ux = px[b] - px[a], uy = py[b] - py[a], uz = pz[b] - pz[a];
      const wx = px[c] - px[a], wy = py[c] - py[a], wz = pz[c] - pz[a];
      let nx = uy * wz - uz * wy, ny = uz * wx - ux * wz, nz = ux * wy - uy * wx;
      const len = Math.hypot(nx, ny, nz) || 1;
      nx /= len, ny /= len, nz /= len;
      const n1x = nx * cy - nz * sny, n1z = nx * sny + nz * cy;
      const n2y = ny * cp + n1z * sp, n2z = -ny * sp + n1z * cp;
      if (n1x * vx[a] + n2y * vy[a] + n2z * vz[a] >= 0) continue;
      const mat = mats[f];
      const lit =
        mat === WATER
          ? Math.min(1.3, Math.max(0.55, 0.8 + (nx * L[0] + nz * L[2]) * 9))
          : 0.38 + 0.68 * Math.max(0, nx * L[0] + ny * L[1] + nz * L[2]);
      const col = COLORS[mat];
      fills[f] = `rgb(${Math.min(255, col[0] * lit) | 0},${Math.min(255, col[1] * lit) | 0},${Math.min(255, col[2] * lit) | 0})`;
      depth[f] = vz[a] + vz[b] + vz[c];
      visible.push(f);
    }
    visible.sort((p, q) => depth[q] - depth[p]);

    ctx!.lineJoin = 'round';
    ctx!.lineWidth = hover ? 0.7 : 0.6;
    for (const f of visible) {
      const a = tris[f * 3], b = tris[f * 3 + 1], c = tris[f * 3 + 2];
      ctx!.beginPath();
      ctx!.moveTo(sx[a], sy[a]);
      ctx!.lineTo(sx[b], sy[b]);
      ctx!.lineTo(sx[c], sy[c]);
      ctx!.closePath();
      ctx!.fillStyle = fills[f];
      ctx!.fill();
      ctx!.strokeStyle = hover ? 'rgba(192,250,115,0.45)' : fills[f];
      ctx!.stroke();
    }
  }

  function reseed() {
    seed = (Math.random() * 0xffff) | 0;
    build();
    grow = still ? 1 : 0;
    growFrom = performance.now();
    if (still) draw(0);
  }

  // Drag to turn, click for a new world.
  let down: { x: number; y: number; moved: number } | null = null;
  canvas.addEventListener('pointerdown', (e) => {
    down = { x: e.clientX, y: e.clientY, moved: 0 };
    canvas.setPointerCapture(e.pointerId);
  });
  canvas.addEventListener('pointermove', (e) => {
    if (!down) return;
    const dx = e.clientX - down.x, dy = e.clientY - down.y;
    down.moved += Math.abs(dx) + Math.abs(dy);
    down.x = e.clientX;
    down.y = e.clientY;
    yaw -= dx * 0.009;
    spin = dx * 0.5;
    pitch = Math.max(0.15, Math.min(0.95, pitch + dy * 0.004));
    if (still) draw(0);
  });
  const release = () => {
    if (down && down.moved < 6) reseed();
    down = null;
  };
  canvas.addEventListener('pointerup', release);
  canvas.addEventListener('pointercancel', () => (down = null));
  canvas.addEventListener('pointerenter', () => (hover = true));
  canvas.addEventListener('pointerleave', () => (hover = false));

  build();
  resize();
  new ResizeObserver(() => {
    resize();
    if (still) draw(0);
  }).observe(canvas);
  if (still) {
    draw(0);
    return;
  }

  let visible = true;
  new IntersectionObserver(([e]) => (visible = e.isIntersecting)).observe(canvas);
  let last = performance.now();
  const t0 = last;
  const frame = (now: number) => {
    requestAnimationFrame(frame);
    const dt = Math.min(0.05, (now - last) / 1000);
    last = now;
    if (!visible || document.hidden) return;
    if (grow < 1) grow = 1 - Math.pow(1 - Math.min(1, (now - growFrom) / 1300), 3);
    if (!down) {
      yaw -= dt * (0.16 + spin);
      spin *= Math.pow(0.04, dt);
    }
    draw((now - t0) / 1000);
  };
  requestAnimationFrame(frame);
}
