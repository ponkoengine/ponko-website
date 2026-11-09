// "Hello, world" step by step: the code grows a few lines at a time while
// the game window beside it shows what those lines do. It plays by itself.

const STEP_MS = 2600;
const HOLD_MS = 9000;

export function hello(root: HTMLElement, still: boolean) {
  const lines = [...root.querySelectorAll<HTMLElement>('.ln')];
  const preview = root.querySelector<HTMLElement>('.game-stage')!;
  const crowd = root.querySelector<HTMLElement>('.crowd');
  const caption = root.querySelector<HTMLElement>('#step-caption');
  const texts = JSON.parse(root.dataset.captions ?? '[]') as string[];
  const last = texts.length;
  let step = 0;

  // The crowd: 1,000 little crabs, made when they're first needed.
  function fillCrowd() {
    if (!crowd || crowd.childElementCount) return;
    const frag = document.createDocumentFragment();
    for (let i = 0; i < 1000; i++) {
      const el = document.createElement('i');
      el.style.setProperty('--d', `${((i * 7919) % 1000) / 1000}s`);
      frag.append(el);
    }
    crowd.append(frag);
  }

  function show(n: number, animate: boolean) {
    const before = step;
    step = n;
    if (n >= 6) fillCrowd();
    let fresh = 0;
    for (const line of lines) {
      const at = Number(line.dataset.step);
      line.hidden = at > n;
      line.classList.remove('new');
      if (animate && at === n && n > before) {
        void line.offsetWidth;
        line.style.setProperty('--k', String(fresh++));
        line.classList.add('new');
      }
    }
    preview.dataset.step = String(n);
    if (caption) caption.innerHTML = texts[n - 1] ?? '';
  }

  if (still) {
    show(last, false);
    return;
  }

  let timer = 0;
  function next() {
    const n = step >= last ? 1 : step + 1;
    show(n, true);
    timer = window.setTimeout(next, n === last ? HOLD_MS : STEP_MS);
  }

  show(1, false);
  // Play while it's on screen; start over from the top each time it comes back.
  new IntersectionObserver(([e]) => {
    clearTimeout(timer);
    if (e.isIntersecting) {
      step = 0;
      next();
    }
  }, { threshold: 0.3 }).observe(root);
}
