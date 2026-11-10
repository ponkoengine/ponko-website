// Smooth scrolling for mouse wheels and trackpads: the page glides toward
// where the wheel points instead of jumping in steps. A section can also ask
// for stops: while the page is inside it, each flick of the wheel moves
// exactly one stop. Touch screens keep their own scrolling, and so does
// anyone who asks for reduced motion.

const stops: { el: HTMLElement; count: number }[] = [];

// Positions of `count` evenly spaced stops through the part of `el` that
// scrolls past while it is pinned.
export function addStops(el: HTMLElement, count: number) {
  stops.push({ el, count: Math.max(2, count) });
}

export function smoothScroll() {
  if (matchMedia('(pointer: coarse)').matches || matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  const root = document.documentElement;
  const max = () => root.scrollHeight - innerHeight;
  let target = scrollY, current = scrollY, last = 0, running = false;
  // After a step, the rest of that flick (trackpads keep sending events for
  // a while) is swallowed until the wheel goes quiet.
  let swallowing = false, lockedUntil = 0, quietAt = 0;

  function step(now: number) {
    const dt = Math.min(0.05, (now - last) / 1000 || 1 / 60);
    last = now;
    current += (target - current) * (1 - Math.exp(-dt * 8));
    if (Math.abs(target - current) < 0.5) current = target;
    scrollTo({ top: current, behavior: 'instant' });
    if (current === target) running = false;
    else requestAnimationFrame(step);
  }
  const go = () => {
    if (running) return;
    running = true;
    last = performance.now();
    requestAnimationFrame(step);
  };

  // Where the next wheel movement should land, given the stops.
  function aim(from: number, delta: number): { to: number; stop: boolean } {
    const to = from + delta;
    for (const { el, count } of stops) {
      const top = el.getBoundingClientRect().top + scrollY;
      const span = el.offsetHeight - innerHeight;
      if (span <= 0) continue;
      const end = top + span;
      const gap = span / (count - 1);
      const inside = from >= top - 1 && from <= end + 1;
      if (inside) {
        const k = Math.round((from - top) / gap) + Math.sign(delta);
        if (k < 0 || k > count - 1) return { to, stop: false };
        return { to: top + k * gap, stop: true };
      }
      // Crossing into the section from either side lands on its first or last stop.
      if (from < top && to > top) return { to: top, stop: true };
      if (from > end && to < end) return { to: end, stop: true };
    }
    return { to, stop: false };
  }

  addEventListener('wheel', (e) => {
    // Pinch zoom and sideways scrolling stay with the browser.
    if (e.ctrlKey || Math.abs(e.deltaX) > Math.abs(e.deltaY)) return;
    e.preventDefault();
    const now = performance.now();
    const quiet = now - quietAt > 160;
    quietAt = now;
    if (quiet) swallowing = false;
    if (now < lockedUntil || swallowing) return;
    const unit = e.deltaMode === 1 ? 40 : e.deltaMode === 2 ? innerHeight : 1;
    if (!running) target = current = scrollY;
    const { to, stop } = aim(target, e.deltaY * unit);
    target = Math.max(0, Math.min(max(), to));
    if (stop) swallowing = true, lockedUntil = now + 380;
    go();
  }, { passive: false });

  // Keys, the scrollbar and links move the page by themselves; follow them.
  addEventListener('scroll', () => {
    if (!running) target = current = scrollY;
  }, { passive: true });
}
