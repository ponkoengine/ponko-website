// The share card for the blog's own pages: /og/blog.png.
import type { APIRoute } from 'astro';
import { card } from '../../lib/og-card';

export const GET: APIRoute = async () => {
  const png = await card({
    id: 'blog',
    art: { palette: 'ember', shape: 'blobs' },
    title: 'News from the engine',
    kicker: 'Blog',
    footer: 'ponko.dev/blog',
  });
  return new Response(new Uint8Array(png), { headers: { 'Content-Type': 'image/png' } });
};
