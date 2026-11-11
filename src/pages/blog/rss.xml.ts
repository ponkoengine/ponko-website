import rss from '@astrojs/rss';
import type { APIContext } from 'astro';
import { getPosts, postHref } from '../../lib/blog';

export async function GET(context: APIContext) {
  const posts = (await getPosts()).filter((p) => !p.data.draft);
  return rss({
    title: 'Ponko blog',
    description: 'News from Ponko: releases, how the engine works inside, and games people make with it.',
    site: context.site!,
    items: posts.map((post) => ({
      title: post.data.title,
      description: post.data.description,
      pubDate: post.data.date,
      link: postHref(post),
      categories: post.data.tags,
      author: post.data.authors.join(', '),
    })),
    customData: '<language>en</language>',
  });
}
