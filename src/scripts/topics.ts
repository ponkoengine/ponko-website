// Topics and pages on the blog's list. Each topic link becomes a toggle; the
// list keeps the posts that have any picked topic, split into pages of
// PER_PAGE. Every post is already in the page, so nothing loads: changes
// animate with a view transition where the browser has one. The state is
// kept in the address, so a filtered page can be shared, reloaded, or reached
// with the back button.
import { pageHref, pageNumbers } from '../lib/pages';

type State = { picked: string[]; page: number };

export function topics(root: HTMLElement) {
  const group = root.querySelector<HTMLElement>('.topics');
  const toggles = [...(group?.querySelectorAll<HTMLAnchorElement>('[data-tag]') ?? [])];
  const clear = group?.querySelector<HTMLButtonElement>('.topics-clear');
  const posts = [...root.querySelectorAll<HTMLElement>('[data-tags]')];
  const more = root.querySelector<HTMLElement>('.blog-more');
  const status = root.querySelector<HTMLElement>('.filter-status');
  const pager = root.querySelector<HTMLElement>('.pager');
  const perPage = Number(root.dataset.perPage) || 20;
  const known = new Set(toggles.map((t) => t.dataset.tag!));

  // What the server drew: the topics pressed and the page in data-page.
  let state: State = {
    picked: toggles.filter((t) => t.getAttribute('aria-pressed') === 'true').map((t) => t.dataset.tag!),
    page: Number(root.dataset.page) || 1,
  };

  // The address can say more than the server knew (?tags=, ?page=).
  function fromAddress(): State {
    const url = new URL(location.href);
    const tags = url.searchParams.get('tags');
    const fromPath = url.pathname.match(/^\/blog\/page\/(\d+)\/?$/);
    return {
      picked: tags !== null ? tags.split(',').filter((t) => known.has(t)) : state.picked,
      page: Number(url.searchParams.get('page') ?? fromPath?.[1] ?? (tags !== null ? 1 : state.page)) || 1,
    };
  }

  const matching = (picked: string[]) =>
    posts.filter((p) => !picked.length || p.dataset.tags!.split(' ').some((tag) => picked.includes(tag)));
  // A state with its page kept within the pages there are.
  function settle(next: State): State {
    const pages = Math.max(1, Math.ceil(matching(next.picked).length / perPage));
    return { picked: next.picked, page: Math.min(Math.max(1, next.page), pages) };
  }

  function render(next: State) {
    state = settle(next);
    const matches = matching(state.picked);
    const pages = Math.max(1, Math.ceil(matches.length / perPage));
    const page = state.page;

    const onPage = new Set(matches.slice((page - 1) * perPage, page * perPage));
    for (const p of posts) p.hidden = !onPage.has(p);
    if (more) more.hidden = !more.querySelector('li:not([hidden])');

    for (const t of toggles) t.setAttribute('aria-pressed', String(state.picked.includes(t.dataset.tag!)));
    if (clear) clear.hidden = !state.picked.length;
    if (status) {
      status.hidden = !state.picked.length;
      const count = document.createElement('b');
      count.textContent = String(matches.length);
      status.replaceChildren('Showing ', count, ` of ${posts.length} posts.`);
    }
    if (pager) drawPager(pager, page, pages, state.picked);
    document.title = document.title.replace(/^Blog(, page \d+)?/, page > 1 ? `Blog, page ${page}` : 'Blog');
  }

  // How the address should read for a state.
  const addressOf = (s: State) => pageHref(s.page, s.picked);

  function go(wanted: State, how: 'push' | 'replace', scroll = false) {
    const next = settle(wanted);
    const update = () => render(next);
    const doc = document as Document & { startViewTransition?: (cb: () => void) => unknown };
    if (doc.startViewTransition && !matchMedia('(prefers-reduced-motion: reduce)').matches) {
      doc.startViewTransition(update);
    } else {
      update();
    }
    history[how === 'push' ? 'pushState' : 'replaceState'](null, '', addressOf(next));
    // A new page starts at its top, not wherever the pager was.
    if (scroll) root.querySelector('.topics, .blog-head')?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }

  // Topic links become toggles. Picking topics stays on the same history
  // entry; it starts over at page 1.
  for (const t of toggles) {
    t.setAttribute('role', 'button');
    t.addEventListener('click', (e) => {
      if (e.metaKey || e.ctrlKey || e.shiftKey || e.button) return; // a new tab gets the tag's page
      e.preventDefault();
      const tag = t.dataset.tag!;
      const picked = state.picked.includes(tag) ? state.picked.filter((x) => x !== tag) : [...state.picked, tag];
      go({ picked, page: 1 }, 'replace');
    });
    t.addEventListener('keydown', (e) => {
      if (e.key === ' ') {
        e.preventDefault();
        t.click();
      }
    });
  }
  clear?.addEventListener('click', () => {
    go({ picked: [], page: 1 }, 'replace');
    toggles[0]?.focus();
  });

  // Page links change the page in place, as a new history entry.
  pager?.addEventListener('click', (e) => {
    const link = (e.target as HTMLElement).closest<HTMLAnchorElement>('a[data-page]');
    if (!link || e.metaKey || e.ctrlKey || e.shiftKey || e.button) return;
    e.preventDefault();
    go({ picked: state.picked, page: Number(link.dataset.page) }, 'push', true);
  });
  addEventListener('popstate', () => render(fromAddress()));

  const start = fromAddress();
  if (start.page !== state.page || start.picked.join() !== state.picked.join()) render(start);
}

/** The pager's markup, the same as Pager.astro draws. */
function drawPager(nav: HTMLElement, page: number, pages: number, picked: string[]) {
  nav.hidden = pages < 2;
  if (pages < 2) return;
  const el = <K extends keyof HTMLElementTagNameMap>(tag: K, attrs: Record<string, string>, text = '') => {
    const node = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
    node.textContent = text;
    return node;
  };
  const link = (n: number, attrs: Record<string, string>, text: string) =>
    el('a', { href: pageHref(n, picked), 'data-page': String(n), ...attrs }, text);
  const step = (n: number, label: string, rel: string, on: boolean) =>
    on ? link(n, { class: 'pager-step', rel }, label) : el('span', { class: 'pager-step', 'aria-disabled': 'true' }, label);

  const list = el('ol', { class: 'pager-pages' });
  for (const n of pageNumbers(page, pages)) {
    if (n === null) {
      list.append(el('li', { class: 'pager-gap', 'aria-hidden': 'true' }, '…'));
      continue;
    }
    const li = el('li', {});
    li.append(link(n, { 'aria-label': `Page ${n}`, ...(n === page ? { 'aria-current': 'page' } : {}) }, String(n)));
    list.append(li);
  }
  nav.replaceChildren(step(page - 1, 'Previous', 'prev', page > 1), list, step(page + 1, 'Next', 'next', page < pages));
}
