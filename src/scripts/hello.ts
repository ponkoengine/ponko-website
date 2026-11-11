// "Hello, world" step by step: the code grows a few lines at a time while
// the game window beside it shows what those lines do. It plays by itself
// while it's on screen; the dots and the back/next buttons take over, and
// the pause button stops it. The page scrolls normally the whole time.

const STEP_MS = 3200;
const HOLD_MS = 9000;

export function hello(root: HTMLElement, still: boolean) {
  const lines = [...root.querySelectorAll<HTMLElement>('.ln')];
  const preview = root.querySelector<HTMLElement>('.game-stage')!;
  const crowd = root.querySelector<HTMLElement>('.crowd');
  const caption = root.querySelector<HTMLElement>('#step-caption');
  const dots = [...root.querySelectorAll<HTMLButtonElement>('[data-go]')];
  const back = root.querySelector<HTMLButtonElement>('[data-step-back]');
  const ahead = root.querySelector<HTMLButtonElement>('[data-step-next]');
  const play = root.querySelector<HTMLButtonElement>('[data-step-play]');
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
    root.style.setProperty('--progress', String(n / last));
    if (caption) {
      caption.innerHTML = texts[n - 1] ?? '';
      if (animate) {
        caption.classList.remove('swap', 'back');
        void caption.offsetWidth;
        caption.classList.add('swap');
        caption.classList.toggle('back', n < before);
      }
    }
    dots.forEach((dot, i) => {
      dot.classList.toggle('past', i < n);
      if (i === n - 1) dot.setAttribute('aria-current', 'step');
      else dot.removeAttribute('aria-current');
    });
    if (back) back.disabled = n <= 1;
    if (ahead) ahead.disabled = n >= last;
  }

  // Playing by itself.
  let timer = 0;
  let playing = !still;
  let onScreen = false;
  function schedule() {
    clearTimeout(timer);
    if (!playing || !onScreen) return;
    timer = window.setTimeout(() => {
      show(step >= last ? 1 : step + 1, true);
      schedule();
    }, step >= last ? HOLD_MS : STEP_MS);
  }
  function setPlaying(on: boolean) {
    playing = on;
    root.classList.toggle('playing', on);
    if (play) {
      play.setAttribute('aria-pressed', String(!on));
      play.setAttribute('aria-label', on ? 'Pause the steps' : 'Play the steps');
    }
    schedule();
  }

  // Any step chosen by hand stops the playing, so it stays where it was put.
  function go(n: number) {
    show(Math.max(1, Math.min(last, n)), !still);
    setPlaying(false);
  }
  for (const dot of dots) dot.addEventListener('click', () => go(Number(dot.dataset.go)));
  back?.addEventListener('click', () => go(step - 1));
  ahead?.addEventListener('click', () => go(step + 1));
  play?.addEventListener('click', () => {
    if (!playing && step >= last) show(1, !still);
    setPlaying(!playing);
  });
  root.addEventListener('keydown', (e) => {
    if ((e.target as HTMLElement).closest('input, textarea')) return;
    if (e.key === 'ArrowRight') go(step + 1);
    else if (e.key === 'ArrowLeft') go(step - 1);
  });

  show(still ? last : 1, false);
  setPlaying(playing);
  new IntersectionObserver(([e]) => {
    onScreen = e.isIntersecting;
    schedule();
  }, { threshold: 0.35 }).observe(root);
}
