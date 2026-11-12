// The dot lab (src/pages/dot-lab.astro). Pick a shape, a palette and a seed
// and the hero above redraws as the post would look; every choice also
// redraws the small pictures, so each shows itself with the rest of what's
// picked. Each small picture moves while it's hovered or focused. The
// choices live in the address, so a picture can be shared.
import {
  artSvg, drawArt, glowOf, paletteNames, palettes, resolveArt, shapes,
  type ArtOptions, type PaletteName, type Shape,
} from '../lib/dot-art';
import { liveArt } from './dot-art-live';
import { slugify } from '../lib/slug.mjs';

type State = {
  title: string;
  lede: string;
  /** Empty: picked from the address, as a post without its own choice. */
  shape: Shape | '';
  palette: PaletteName | '';
  seed: string;
};

const HERO = { cols: 72, rows: 40 };
const THUMB = { cols: 32, rows: 20 };
const SYLLABLES = ['ka', 'lo', 'mi', 'ne', 'po', 'ru', 'sa', 'ti', 'vo', 'zu', 'ba', 'de', 'fi', 'go', 'ha', 'ji'];

let patterns = 0;
/** A post's picture: the still SVG, which the moving canvas covers. */
function artElement(id: string, art: ArtOptions, size: { cols: number; rows: number }, className = '') {
  const el = document.createElement('span');
  el.className = `dot-art ${className}`.trim();
  el.setAttribute('aria-hidden', 'true');
  el.dataset.id = id;
  el.dataset.art = JSON.stringify(resolveArt(id, art));
  el.dataset.cols = String(size.cols);
  el.dataset.rows = String(size.rows);
  el.innerHTML = artSvg(drawArt(id, art, size.cols, size.rows), `lab-rings-${++patterns}`);
  return el;
}

const pick = <T,>(list: readonly T[]) => list[Math.floor(Math.random() * list.length)];
const word = () => Array.from({ length: 3 }, () => pick(SYLLABLES)).join('');

export function dotLab(root: HTMLElement) {
  const $ = <T extends Element = HTMLElement>(sel: string) => root.querySelector<T>(sel)!;
  const hero = $('.post-hero');
  const glow = $('.dot-glow');
  const shapeGrid = $('[data-lab-shapes]');
  const paletteGrid = $('[data-lab-palettes]');
  const seedInput = $<HTMLInputElement>('[data-lab-seed]');
  const titleInput = $<HTMLInputElement>('[data-lab-title-input]');
  const ledeInput = $<HTMLTextAreaElement>('[data-lab-lede-input]');
  const out = $('[data-lab-out]');
  const copy = $<HTMLButtonElement>('[data-lab-copy]');

  const query = new URLSearchParams(location.search);
  const state: State = {
    title: query.get('title') || root.dataset.title || '',
    lede: query.get('description') || root.dataset.lede || '',
    shape: (shapes as readonly string[]).includes(query.get('shape') ?? '') ? (query.get('shape') as Shape) : query.has('shape') ? '' : 'life',
    palette: (paletteNames as readonly string[]).includes(query.get('palette') ?? '')
      ? (query.get('palette') as PaletteName)
      : query.has('palette') ? '' : 'acid',
    seed: query.get('seed') ?? '',
  };
  titleInput.value = state.title;
  ledeInput.value = state.lede;
  seedInput.value = state.seed;

  const address = () => slugify(state.title) || 'my-post';
  const options = (over: Partial<State> = {}): ArtOptions => {
    const s = { ...state, ...over };
    return { shape: s.shape || undefined, palette: s.palette || undefined, seed: s.seed.trim() || undefined };
  };

  // The small pictures: one button per shape and per palette, plus "auto".
  type Thumb = { button: HTMLButtonElement; thumb: HTMLElement; note: HTMLElement; play: () => void; still: () => void };
  const shapeButtons = new Map<Shape | '', Thumb>();
  for (const shape of ['', ...shapes] as (Shape | '')[]) {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'lab-shape';
    button.innerHTML = `<span class="lab-thumb"></span><span class="lab-name"><span>${shape || 'Auto'}</span><small></small></span>`;
    const thumb = button.querySelector<HTMLElement>('.lab-thumb')!;
    const note = button.querySelector<HTMLElement>('small')!;
    let stop: (() => void) | null = null;
    const play = () => {
      const art = thumb.firstElementChild as HTMLElement | null;
      if (art && !stop) stop = liveArt(art);
    };
    const still = () => {
      stop?.();
      stop = null;
    };
    button.addEventListener('pointerenter', play);
    button.addEventListener('focus', play);
    button.addEventListener('pointerleave', still);
    button.addEventListener('blur', still);
    button.addEventListener('click', () => set({ shape }));
    shapeGrid.append(button);
    shapeButtons.set(shape, { button, thumb, note, play, still });
  }

  const paletteButtons = new Map<PaletteName | '', HTMLButtonElement>();
  for (const palette of ['', ...paletteNames] as (PaletteName | '')[]) {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'lab-palette';
    button.innerHTML = `<span class="lab-swatch"></span><span>${palette || 'Auto'}</span>`;
    button.addEventListener('click', () => set({ palette }));
    paletteGrid.append(button);
    paletteButtons.set(palette, button);
  }

  let stopHero: (() => void) | null = null;
  function drawHero() {
    const id = address();
    const art = options();
    stopHero?.();
    const next = artElement(id, art, HERO, 'post-hero-art');
    hero.querySelector('.post-hero-art')?.replaceWith(next);
    stopHero = liveArt(next);
    glow.setAttribute('style', glowOf(id, art).style);
  }

  function drawThumbs() {
    const id = address();
    for (const [shape, { button, thumb, note, play, still }] of shapeButtons) {
      // Redrawing replaces the picture, so whatever moved has to stop first.
      const playing = button.matches(':hover, :focus-visible');
      still();
      // Every small picture uses the palette and seed picked now.
      thumb.replaceChildren(artElement(id, options({ shape }), THUMB));
      note.textContent = shape ? '' : resolveArt(id, options({ shape: '' })).shape;
      if (playing) play();
    }
    for (const [palette, button] of paletteButtons) {
      const shown = resolveArt(id, options({ palette })).palette;
      button.querySelector('.lab-swatch')!.innerHTML =
        palettes[shown].map((c) => `<i style="background:${c}"></i>`).join('') +
        (palette ? '' : `<i class="lab-auto">${shown}</i>`);
    }
  }

  function drawWords() {
    const id = address();
    const { shape, palette } = resolveArt(id, options());
    $('[data-lab-title]').textContent = state.title || 'Untitled';
    $('[data-lab-lede]').textContent = state.lede;
    $('[data-lab-chip]').textContent = `${shape} · ${palette}`;
    $('[data-lab-address]').textContent = `/blog/${id}/`;
    for (const [s, { button }] of shapeButtons) button.setAttribute('aria-pressed', String(s === state.shape));
    for (const [p, button] of paletteButtons) button.setAttribute('aria-pressed', String(p === state.palette));

    const parts = [
      state.palette && `palette: ${state.palette}`,
      state.shape && `shape: ${state.shape}`,
      state.seed.trim() && `seed: ${JSON.stringify(state.seed.trim())}`,
    ].filter(Boolean);
    out.textContent = parts.length
      ? `art: { ${parts.join(', ')} }`
      : '# No art line: the address picks the picture.';
  }

  let saving = 0;
  function remember() {
    clearTimeout(saving);
    saving = window.setTimeout(() => {
      const q = new URLSearchParams();
      q.set('shape', state.shape);
      q.set('palette', state.palette);
      if (state.seed.trim()) q.set('seed', state.seed.trim());
      if (state.title !== root.dataset.title) q.set('title', state.title);
      if (state.lede !== root.dataset.lede) q.set('description', state.lede);
      history.replaceState(null, '', `${location.pathname}?${q}`);
    }, 300);
  }

  let drawing = 0;
  function set(change: Partial<State>, { soon = false } = {}) {
    Object.assign(state, change);
    drawWords();
    remember();
    // Typing redraws once the typing pauses, not on every key.
    clearTimeout(drawing);
    const draw = () => {
      drawHero();
      drawThumbs();
    };
    if (soon) drawing = window.setTimeout(draw, 150);
    else draw();
  }

  seedInput.addEventListener('input', () => set({ seed: seedInput.value }, { soon: true }));
  // Only the address seeds the picture, so only a new address redraws it.
  titleInput.addEventListener('input', () => {
    const before = address();
    state.title = titleInput.value;
    if (address() !== before) set({}, { soon: true });
    else {
      drawWords();
      remember();
    }
  });
  ledeInput.addEventListener('input', () => {
    state.lede = ledeInput.value;
    drawWords();
    remember();
  });
  $('[data-lab-shuffle]').addEventListener('click', () => {
    seedInput.value = word();
    set({ seed: seedInput.value });
  });
  $('[data-lab-surprise]').addEventListener('click', () => {
    seedInput.value = word();
    set({ shape: pick(shapes), palette: pick(paletteNames), seed: seedInput.value });
  });
  copy.addEventListener('click', async () => {
    try {
      await navigator.clipboard.writeText(out.textContent ?? '');
      copy.textContent = 'Copied';
    } catch {
      copy.textContent = 'Select it';
    }
    setTimeout(() => (copy.textContent = 'Copy'), 1400);
  });

  set({});
}
