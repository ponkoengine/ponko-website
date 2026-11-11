// Paging for the blog's list. Kept apart from blog.ts so the browser can
// use it too.

/** Posts on one page of the blog's list. */
export const PER_PAGE = 20;

/** The address of page n of the list, with topics picked or not. */
export function pageHref(n: number, tags: string[] = []) {
  if (tags.length) return `/blog/?tags=${tags.map(encodeURIComponent).join(',')}${n > 1 ? `&page=${n}` : ''}`;
  return n > 1 ? `/blog/page/${n}/` : '/blog/';
}

/**
 * The page numbers to show for page `page` of `pages`: always the first and
 * the last, this page and the ones next to it, and null where numbers are
 * left out. Short runs are shown whole rather than leaving a gap of one.
 */
export function pageNumbers(page: number, pages: number): (number | null)[] {
  const keep = new Set([1, pages, page - 1, page, page + 1].filter((n) => n >= 1 && n <= pages));
  const out: (number | null)[] = [];
  let last = 0;
  for (const n of [...keep].sort((a, b) => a - b)) {
    if (n - last === 2) out.push(last + 1);
    else if (n - last > 2) out.push(null);
    out.push(n);
    last = n;
  }
  return out;
}
