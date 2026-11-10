// "Hello, world" step by step: the code grows a few lines at a time while
// the game window beside it shows what those lines do. Given a section to
// follow, the scroll position picks the step while `pinned` matches;
// otherwise it plays by itself.

const STEP_MS = 2600;
const HOLD_MS = 9000;

export function hello(root: HTMLElement, still: boolean, follow?: HTMLElement, pinned?: MediaQueryList) {
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
    root.dataset.step = String(n);
    root.classList.toggle('done', n === last);
    if (caption) {
      caption.innerHTML = texts[n - 1] ?? '';
      if (animate) {
        caption.classList.remove('swap');
        void caption.offsetWidth;
        caption.classList.add('swap', n < before ? 'back' : 'ahead');
        caption.classList.toggle('back', n < before);
      }
    }
    dots.forEach((dot, i) => {
      dot.classList.toggle('past', i < n);
      if (i === n - 1) dot.setAttribute('aria-current', 'step');
      else dot.removeAttribute('aria-current');
    });
  }
  const dots = [...root.querySelectorAll<HTMLButtonElement>('[data-go]')];

  let timer = 0;
  function next() {
    const n = step >= last ? 1 : step + 1;
    show(n, true);
    timer = window.setTimeout(next, n === last ? HOLD_MS : STEP_MS);
  }

  // Following the scroll: the section is pinned and its scroll position
  // picks the step. This only happens while `pinned` matches (wide screens
  // with a mouse or trackpad); otherwise the steps play by themselves.
  const scrubbing = () => !!follow && !!pinned?.matches;
  const span = () => Math.max(1, (follow?.offsetHeight ?? 0) - innerHeight);
  const update = () => {
    if (!scrubbing()) return;
    const at = Math.min(1, Math.max(0, -follow!.getBoundingClientRect().top / span()));
    // A continuous reading for the meter, and the nearest step for the rest.
    root.style.setProperty('--progress', at.toFixed(4));
    const n = Math.min(last, 1 + Math.round(at * (last - 1)));
    if (n !== step) show(n, !still);
  };

  // The dots under the caption jump straight to their step.
  for (const dot of dots) {
    dot.addEventListener('click', () => {
      const n = Number(dot.dataset.go);
      if (scrubbing()) {
        const top = follow!.getBoundingClientRect().top + scrollY;
        scrollTo({ top: top + (span() * (n - 1)) / (last - 1), behavior: still ? 'auto' : 'smooth' });
        return;
      }
      clearTimeout(timer);
      show(n, !still);
      if (!still) timer = window.setTimeout(next, HOLD_MS);
    });
  }

  let onScreen = false;
  function mode() {
    clearTimeout(timer);
    root.classList.toggle('scrubbed', scrubbing());
    if (scrubbing()) return update();
    if (still) return show(last, false);
    if (onScreen) {
      step = 0;
      next();
    }
  }

  show(1, false);
  addEventListener('scroll', update, { passive: true });
  addEventListener('resize', update);
  pinned?.addEventListener('change', mode);
  // When playing by itself: play while it's on screen, and start over from
  // the top each time it comes back.
  new IntersectionObserver(([e]) => {
    onScreen = e.isIntersecting;
    if (scrubbing() || still) return;
    clearTimeout(timer);
    if (onScreen) {
      step = 0;
      next();
    }
  }, { threshold: 0.3 }).observe(root);
  mode();
}
