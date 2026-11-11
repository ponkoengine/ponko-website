// A share card for each post: /og/blog/<post>.png. See src/lib/og-card.ts.
import type { APIRoute } from 'astro';
import { getPosts, formatDate, type Post } from '../../../lib/blog';
import { card } from '../../../lib/og-card';
import type { ArtOptions } from '../../../lib/dot-art';

export async function getStaticPaths() {
  const posts = await getPosts();
  return posts.map((post) => ({ params: { slug: post.id }, props: { post } }));
}

export const GET: APIRoute = async ({ props }) => {
  const { post } = props as { post: Post };
  const png = await card({
    id: post.id,
    art: post.data.art as ArtOptions,
    title: post.data.title,
    kicker: 'Blog',
    footer: `${formatDate(post.data.date)}   ·   ponko.dev/blog`,
  });
  return new Response(new Uint8Array(png), { headers: { 'Content-Type': 'image/png' } });
};
