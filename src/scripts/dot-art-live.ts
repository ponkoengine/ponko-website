// Brings a post's picture in dots to life: the same field as the still SVG
// (src/lib/dot-art.ts), redrawn on a canvas with time moving. It runs only
// while the picture is on screen and the tab is visible, and not at all for
// anyone who asks for reduced motion; they keep the still picture.
import { artField, dotFor, dotWidth, SIZES, type ArtOptions } from '../lib/dot-art';

/** Starts the picture moving. Returns a function that stops it again. */
export function liveArt(root: HTMLElement): () => void {
  const none = () => {};
  if (matchMedia('(prefers-reduced-motion: reduce)').matches) return none;
  const cols = Number(root.dataset.cols), rows = Number(root.dataset.rows);
  const options = JSON.parse(root.dataset.art ?? '{}') as ArtOptions;
  const aspect = cols / rows;
  const { colours, at } = artField(root.dataset.id ?? '', options, aspect, { cols, rows });

  const canvas = document.createElement('canvas');
  canvas.setAttribute('aria-hidden', 'true');
  const ctx = canvas.getContext('2d');
  if (!ctx) return none;
  root.append(canvas);

  // Laid out like the SVG's "slice": cells stay square and the grid is
  // centred, cut off at whichever side is too long.
  let W = 0, H = 0, cell = 1, ox = 0, oy = 0;
  let rings: HTMLCanvasElement | null = null;
  function resize() {
    const dpr = Math.min(2, devicePixelRatio || 1);
    W = root.clientWidth;
    H = root.clientHeight;
    // Hidden (a filtered-out post, say): nothing to draw until it's back.
    if (!W || !H) {
      rings = null;
      return;
    }
    canvas.width = Math.round(W * dpr);
    canvas.height = Math.round(H * dpr);
    ctx!.setTransform(dpr, 0, 0, dpr, 0, 0);
    cell = Math.max(W / cols, H / rows);
    ox = (W - cell * cols) / 2;
    oy = (H - cell * rows) / 2;
    // The faint rings never change: draw them once.
    rings = document.createElement('canvas');
    rings.width = canvas.width;
    rings.height = canvas.height;
    const r = rings.getContext('2d')!;
    r.setTransform(dpr, 0, 0, dpr, 0, 0);
    r.strokeStyle = colours[0];
    r.lineWidth = cell * 0.06;
    r.beginPath();
    for (let j = 0; j < rows; j++) {
      for (let i = 0; i < cols; i++) {
        const x = ox + (i + 0.5) * cell, y = oy + (j + 0.5) * cell;
        r.moveTo(x + cell * 0.3, y);
        r.arc(x, y, cell * 0.3, 0, Math.PI * 2);
      }
    }
    r.stroke();
  }

  const paths = colours.map(() => new Path2D());
  function draw(t: number) {
    if (!W || !H) return;
    ctx!.setTransform(1, 0, 0, 1, 0, 0);
    ctx!.clearRect(0, 0, canvas.width, canvas.height);
    if (rings) ctx!.drawImage(rings, 0, 0);
    const dpr = canvas.width / (W || 1);
    ctx!.setTransform(dpr, 0, 0, dpr, 0, 0);
    for (let k = 0; k < paths.length; k++) paths[k] = new Path2D();
    for (let j = 0; j < rows; j++) {
      for (let i = 0; i < cols; i++) {
        const f = at(((i + 0.5) / cols) * aspect, (j + 0.5) / rows, t);
        const dot = dotFor(f, colours.length);
        if (!dot) continue;
        // Sizes follow the field smoothly here, so slow motion doesn't step.
        const x = ox + (i + 0.5) * cell, y = oy + (j + 0.5) * cell, r = (dotWidth(f * SIZES) * cell) / 2;
        paths[dot.shade].moveTo(x + r, y);
        paths[dot.shade].arc(x, y, r, 0, Math.PI * 2);
      }
    }
    paths.forEach((p, k) => {
      ctx!.fillStyle = colours[k];
      ctx!.fill(p);
    });
  }

  // Slow and calm, like a console menu's background: the picture's own
  // time runs at a third of real time. About 30 frames a second is plenty.
  const PACE = 0.32;
  let visible = false, frame = 0, last = 0;
  const start = performance.now();
  function tick(now: number) {
    frame = 0;
    if (!visible || document.hidden) return;
    if (now - last > 32) {
      last = now;
      draw(((now - start) / 1000) * PACE);
    }
    frame = requestAnimationFrame(tick);
  }
  const run = () => {
    if (!frame && visible && !document.hidden) frame = requestAnimationFrame(tick);
  };

  resize();
  draw(0);
  root.classList.add('live');
  const sized = new ResizeObserver(() => {
    resize();
    draw(((performance.now() - start) / 1000) * PACE);
  });
  sized.observe(root);
  const seen = new IntersectionObserver(([e]) => {
    visible = e.isIntersecting;
    run();
  });
  seen.observe(root);
  document.addEventListener('visibilitychange', run);
  return () => {
    sized.disconnect();
    seen.disconnect();
    document.removeEventListener('visibilitychange', run);
    cancelAnimationFrame(frame);
    frame = 0;
    visible = false;
    canvas.remove();
    root.classList.remove('live');
  };
}
