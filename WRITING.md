# Writing a blog post

Posts live in `src/content/blog/`. Each one is a Markdown (`.md`) or MDX
(`.mdx`) file. MDX is Markdown that can also use the components below.

## Start a post

```sh
npm run new-post -- "What's new in Ponko 0.1"
npm run dev
```

This makes `src/content/blog/whats-new-in-ponko-0-1/index.mdx`, a draft dated
today, in a folder of its own for its pictures. Open
http://127.0.0.1:4321/blog/ to see it. Drafts show in `npm run dev` with a
Draft badge and are left out of the built site, the RSS feed and the sitemap.

The folder name is the address: `/blog/whats-new-in-ponko-0-1/`. Don't rename
it after the post is published, or links to it break. A single file
(`src/content/blog/my-post.mdx`) works too. Files whose names start with `_`
are ignored.

## The top of the file

```yaml
---
title: What's new in Ponko 0.1          # up to 90 characters
description: One or two sentences.      # list, search results and social cards
date: 2026-11-11                        # the day it's published
updated: 2026-12-01                     # optional: a change readers should know about
authors: [Ada Lovelace]                 # optional: defaults to "The Ponko team"
tags: [Release, Rendering]              # optional: each tag gets its own page
art: { palette: ember, shape: blobs }   # optional: see "The picture in dots"
cover: ./cover.png                      # optional: a picture next to the file
coverAlt: The new shadow cascades in the island demo
draft: true                             # set to false to publish
---
```

### The picture in dots

Every post has its own picture in dots. It sits behind the title at the top
of the post, moving slowly (still for readers who ask for reduced motion),
and appears again in the list of posts. The palette and shape are
picked from the post's address, so a post always looks the same and two posts
rarely match. To choose them yourself:

```yaml
art: { palette: amber, shape: sphere }
```

- `palette`: `ember`, `amber`, `rose`, `sage`, `dusk` or `sea`.
- `shape`: `blobs`, `ripple`, `waves`, `sphere`, `swirl`, `interference`,
  `aurora`, `plasma` or `orbits`.
- `seed`: any word. Change it to get a different picture of the same kind.

### Cover image

A `cover` is optional. It's shown under the title, and is used as the share
image on social sites (otherwise they show the site's own card). It should be
at least 1600 pixels wide.

The build checks all of this. A missing description or a misspelled field
stops `npm run build` with a message saying which post and which field.

## Markdown

Everything in ordinary Markdown works: headings (start at `##`, as the title
is the page's only `#`), lists, links, **bold**, _italic_, `code`, tables,
footnotes[^1], quotes and `---` for a break (it's drawn as three dots).

Pictures next to the post are resized and compressed for you:

```md
![The island at dusk](./island.png)
```

Code blocks are highlighted in the site's colours and get a copy button. Name
the language, and add a `title` to show a file name instead:

````md
```rust title="src/main.rs"
fn main() {}
```
````

[^1]: Like this one.

## Components

These work in `.mdx` posts with no `import`. Leave an empty line between a
component and Markdown inside it.

### Callout

A note set apart from the text. `type` is `note` (the default), `tip` or
`warning`. `title` is optional.

```mdx
<Callout type="warning" title="Breaking change">
  `Ctx::spawn` is now `ctx.world.spawn`.
</Callout>
```

### Figure

A picture with a caption. `wide` lets it run wider than the text, and
`pixelated` keeps pixel art sharp. Import the picture so it gets optimised:

```mdx
import island from './island.png';

<Figure src={island} alt="The island at dusk" caption="Shadows from the new cascades." wide />
```

`src` can also be a path under `public/`, like `"/play/tide/poster.webp"`.

### Video

A short clip. Without `controls` it plays silently in a loop, like a moving
picture (it stops for readers who ask for reduced motion). Use `controls` for
anything with sound. Put the files in `public/blog/` and keep them small:
an MP4 or WebM under 5 MB.

```mdx
<Video src="/blog/water.mp4" poster="/blog/water.webp" caption="Water, at 60 fps." wide />
```

### YouTube

Loads nothing from YouTube until someone clicks play.

```mdx
<YouTube id="dQw4w9WgXcQ" title="Ponko in ten minutes" />
```

### Play

A game you can play from the post. Clicking opens it in a new tab, where it
fills the window. Games live in `public/play/<name>/` (see DEPLOYMENT.md).

```mdx
<Play src="/play/tide/" poster="/play/tide/poster.webp" title="Tide" note="2.1 MB, needs WebGPU" />
```

### Steps

Numbered steps joined by a line. Put an ordered list inside, with empty lines
around it:

```mdx
<Steps>

1. Install the engine.

   ```sh
   cargo add ponko
   ```

2. Run the game.

</Steps>
```

### Stats

Numbers on the dot-matrix boards from the home page. Each value is up to six
characters from `0-9 , . M B` and space.

```mdx
<Stats>
  <Stat value="12,000">units on the board</Stat>
  <Stat value="2.1 MB">to download</Stat>
</Stats>
```

### Keys

Use `<kbd>` for keys: `<kbd>Space</kbd>`.

## Adding a component

Write it in `src/components/mdx/`, add it to `src/components/mdx/index.ts`,
style it in `src/styles/blog.css`, and describe it here.

## Before publishing

1. Set `draft: false` and check the date.
2. Run `npm run build:site` and `npm run check` to catch broken links.
3. Open a pull request. Cloudflare builds a preview of it to read through.

Merging to `main` publishes the post, and adds it to `/blog/rss.xml` and
`/sitemap.xml`.
