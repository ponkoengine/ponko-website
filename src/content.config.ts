// The blog. Each post is a .md or .mdx file in src/content/blog/, or a
// folder with an index.mdx and its own images. See WRITING.md.
import { defineCollection } from 'astro:content';
import { glob } from 'astro/loaders';
import { z } from 'astro/zod';
import { paletteNames, shapes } from './lib/dot-art';

const blog = defineCollection({
  loader: glob({ base: './src/content/blog', pattern: '**/[^_]*.{md,mdx}' }),
  schema: ({ image }) =>
    z.object({
      title: z.string().max(90),
      /** One or two sentences: shown in the list, in search results and on social cards. */
      description: z.string().max(220),
      date: z.coerce.date(),
      /** Set when a post is changed in a way readers should know about. */
      updated: z.coerce.date().optional(),
      authors: z.array(z.string()).default(['The Ponko team']),
      tags: z.array(z.string()).default([]),
      /** The post's picture in dots. Both are picked from the address unless set. */
      art: z
        .object({
          palette: z.enum(paletteNames as [string, ...string[]]).optional(),
          shape: z.enum(shapes).optional(),
          /** Any word: change it to get a different picture of the same kind. */
          seed: z.string().optional(),
        })
        .default({}),
      /** A picture for the post's hero and its social card, in place of the dot art. */
      cover: image().optional(),
      coverAlt: z.string().optional(),
      /** Drafts show in `npm run dev` but are left out of the built site. */
      draft: z.boolean().default(false),
    }),
});

export const collections = { blog };
