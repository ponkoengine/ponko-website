// A small dot-matrix display. It shows the text in its `data-text`, and when
// it first scrolls into view the digits roll for a moment before they settle.

const GLYPHS: Record<string, string[]> = {
  '0': ['01110', '10001', '10011', '10101', '11001', '10001', '01110'],
  '1': ['00100', '01100', '00100', '00100', '00100', '00100', '01110'],
  '2': ['01110', '10001', '00001', '00010', '00100', '01000', '11111'],
  '3': ['11110', '00001', '00001', '01110', '00001', '00001', '11110'],
  '4': ['00010', '00110', '01010', '10010', '11111', '00010', '00010'],
  '5': ['11111', '10000', '11110', '00001', '00001', '10001', '01110'],
  '6': ['00110', '01000', '10000', '11110', '10001', '10001', '01110'],
  '7': ['11111', '00001', '00010', '00100', '01000', '01000', '01000'],
  '8': ['01110', '10001', '10001', '01110', '10001', '10001', '01110'],
  '9': ['01110', '10001', '10001', '01111', '00001', '00010', '01100'],
  ',': ['00000', '00000', '00000', '00000', '00000', '00100', '01000'],
  '.': ['00000', '00000', '00000', '00000', '00000', '00000', '00100'],
  M: ['10001', '11011', '10101', '10101', '10001', '10001', '10001'],
  B: ['11110', '10001', '10001', '11110', '10001', '10001', '11110'],
  ' ': ['00000', '00000', '00000', '00000', '00000', '00000', '00000'],
};
const COLS = 6 * 6 - 1, ROWS = 7;

export function board(canvas: HTMLCanvasElement, still: boolean) {
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  const text = (canvas.dataset.text ?? '').slice(0, 6);
  let shown = text;

  function draw() {
    const dpr = Math.min(2, devicePixelRatio || 1);
    const w = canvas.clientWidth, cell = w / COLS, h = cell * ROWS;
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
    ctx!.setTransform(dpr, 0, 0, dpr, 0, 0);
    for (const lit of [false, true]) {
      ctx!.fillStyle = lit ? '#f4e9db' : '#2a201c';
      ctx!.beginPath();
      for (let j = 0; j < ROWS; j++) {
        for (let i = 0; i < COLS; i++) {
          const glyph = i % 6 < 5 ? GLYPHS[shown[Math.floor(i / 6)] ?? ' '] : null;
          if ((glyph?.[j][i % 6] === '1') !== lit) continue;
          const x = (i + 0.5) * cell, y = (j + 0.5) * cell, r = cell * (lit ? 0.4 : 0.22);
          ctx!.moveTo(x + r, y);
          ctx!.arc(x, y, r, 0, Math.PI * 2);
        }
      }
      ctx!.fill();
    }
  }

  draw();
  new ResizeObserver(draw).observe(canvas);
  if (still) return;
  const seen = new IntersectionObserver(([e]) => {
    if (!e.isIntersecting) return;
    seen.disconnect();
    const start = performance.now();
    const roll = (now: number) => {
      const settled = Math.floor((now - start) / 140);
      shown = [...text].map((ch, i) => (i < settled || !/\d/.test(ch) ? ch : String(Math.floor(Math.random() * 10)))).join('');
      draw();
      if (settled < text.length) setTimeout(() => requestAnimationFrame(roll), 45);
    };
    requestAnimationFrame(roll);
  }, { threshold: 0.6 });
  seen.observe(canvas);
}
