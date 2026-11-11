import { getCollection, type CollectionEntry } from 'astro:content';

export type Post = CollectionEntry<'blog'>;

/** Published posts, newest first. Drafts are included only in development. */
export async function getPosts(): Promise<Post[]> {
  const posts = await getCollection('blog', ({ data }) => import.meta.env.DEV || !data.draft);
  return posts.sort((a, b) => b.data.date.valueOf() - a.data.date.valueOf());
}

/** A post's address: /blog/<id>/. A folder post's id is its folder name. */
export const postHref = (post: Post) => `/blog/${post.id}/`;

export const tagSlug = (tag: string) =>
  tag.toLowerCase().trim().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');

export { PER_PAGE, pageHref } from './pages';

export const tagHref = (tag: string) => `/blog/tags/${tagSlug(tag)}/`;

/** Every tag with its posts, most used first. */
export function tagsOf(posts: Post[]) {
  const tags = new Map<string, { name: string; posts: Post[] }>();
  for (const post of posts) {
    for (const name of post.data.tags) {
      const slug = tagSlug(name);
      const entry = tags.get(slug) ?? { name, posts: [] };
      entry.posts.push(post);
      tags.set(slug, entry);
    }
  }
  return [...tags.entries()]
    .map(([slug, t]) => ({ slug, ...t }))
    .sort((a, b) => b.posts.length - a.posts.length || a.name.localeCompare(b.name));
}

const dateFormat = new Intl.DateTimeFormat('en-GB', { day: 'numeric', month: 'long', year: 'numeric', timeZone: 'UTC' });
export const formatDate = (date: Date) => dateFormat.format(date);

/** Minutes to read, at about 230 words a minute; code counts for less. */
export function readingMinutes(body = '') {
  const count = (text: string) => text.split(/\s+/).filter(Boolean).length;
  let code = 0;
  const prose = body
    .replace(/```[\s\S]*?```/g, (block) => ((code += count(block)), ' '))
    .replace(/^\s*(import|export)\s.*$/gm, '')
    .replace(/<[^>]+>/g, ' ');
  return Math.max(1, Math.round((count(prose) + code / 3) / 230));
}
