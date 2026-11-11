// The sitemap, built from the pages and posts so new posts are in it
// without anyone remembering to add them.
import type { APIContext } from 'astro';
import { getPosts, postHref, tagsOf } from '../lib/blog';

export async function GET(context: APIContext) {
  const posts = (await getPosts()).filter((p) => !p.data.draft);
  const day = (d: Date) => d.toISOString().slice(0, 10);
  const urls: [string, string?][] = [
    ['/'],
    ['/guide/'],
    ['/blog/', posts[0] && day(posts[0].data.date)],
    ...posts.map((p): [string, string] => [postHref(p), day(p.data.updated ?? p.data.date)]),
    ...tagsOf(posts).map((t): [string] => [`/blog/tags/${t.slug}/`]),
  ];
  const body = urls
    .map(([path, lastmod]) => `<url><loc>${new URL(path, context.site)}</loc>${lastmod ? `<lastmod>${lastmod}</lastmod>` : ''}</url>`)
    .join('');
  return new Response(`<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">${body}</urlset>\n`, {
    headers: { 'Content-Type': 'application/xml; charset=utf-8' },
  });
}
