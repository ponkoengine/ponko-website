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
leads the list of posts when the post is the newest, and is on the post's
share card. The palette and shape are
picked from the post's address, so a post always looks the same and two posts
rarely match. To choose them yourself:

```yaml
art: { palette: amber, shape: sphere }
```

- `palette`, from dark to light in one hue: `ember`, `amber`, `rose`, `sage`,
  `dusk`, `sea`, `ice` or `ash`; or through several hues: `lava`, `synth`,
  `sunset`, `boreal`, `neon` or `acid`.
- `shape`: `blobs`, `ripple`, `waves`, `sphere`, `swirl`, `interference`,
  `aurora`, `plasma`, `orbits`, `life` (Conway's Game of Life), `donut` (a
  spinning torus), `tunnel`, `starfield`, `rain`, `fire`, `contours`, `moire`,
  `lissajous`, `voronoi` or `equalizer`.
- `seed`: any word. Change it to get a different picture of the same kind.

To try them all, open the dot lab at [/dot-lab/](https://ponko.dev/dot-lab/)
(`npm run dev`, then http://127.0.0.1:4321/dot-lab/). It shows your title over
each choice as a post would look, and gives you the `art:` line to copy.

### Cover image

A `cover` is optional. It's shown under the title and in the list of posts.
It should be at least 1600 pixels wide.

### Sharing

Every post gets a share card, the picture social sites and chats show for a
link to it: the post's title beside its picture in dots, made at build time
at `/og/blog/<post>.png` (see `src/lib/og-card.ts`). The page also carries
the title, description, dates, authors and tags for social sites (Open
Graph and Twitter tags) and search engines (structured data). Nothing to do
but write a good `title` and `description`; check how a card looks by
opening its address on the dev server.

The build checks all of this. A missing description or a misspelled field
stops `npm run build` with a message saying which post and which field.

## Writing style

Release posts are written in the style of Bevy's release posts and Godot's
release-candidate posts: warm, concrete, and generous with links and code.
Other posts follow the same voice.

### Voice

- "We" is the people making Ponko, "you" is the person using it. Talk to
  them, not about them.
- Warm and a little excited, never salesy. Short bits of humour are welcome
  inside technical prose; jokes are not a section of their own.
- Plain about limits. Say what doesn't work yet, on which platform, in one
  flat sentence ("Mesh shaders are not supported on the web."). Explain a
  trade-off when you make one ("We think this is worth it, because ...").
- Sentences mostly of 12 to 25 words, paragraphs of one to four sentences.
  Follow a long explanation with a short line.
- Introduce a name the first time with what it is: "Feathers, our
  editor-first UI toolkit, now has more widgets".
- Link generously: the guide chapter, docs.rs, the example, the pull request.
- Use _italics_ for the odd intensifier, bold only for the names in lists.

### A release post

1. **The opening.** Who made it and how much ("Thanks to N contributors and
   M pull requests, we're happy to announce Ponko 0.3 on crates.io!"), then
   two sentences on what Ponko is with links to the guide, the source and
   the examples. End with "It's free and open source forever!" Point to the
   migration guide in one sentence.
2. **Highlights.** "There's a _lot_ in this release, but here are some of
   the highlights:" and a bullet list of five to eight items, each a bold
   name, a colon and one sentence.
3. **One section per feature**, as `##` headings in Title Case, plain noun
   phrases ("Compute Readback", "Game States"). Under the heading, a credit
   line with the authors' GitHub handles and the pull requests:

   ```md
   Authors: @someone, @someone-else · PRs: #702, #710
   ```

   Then: what was hard or missing before (one short paragraph), what you can
   do now, and code. Show changes to existing APIs as a before and after
   (`// Before` and `// After`, or two titled blocks). Sub-features get `###`
   headings. End with where to see it: "Check out the `compute` example to
   see it in action!" A section can be three sentences or a page; size it to
   the feature.
4. **Breaking changes** are explained where they happen, inside the
   feature's section, with before and after code. The migration guide is
   linked once at the top and once at the end.
5. **What's next**, a few lines on what we're working on.
6. **Thanks**, with the number of contributors and a link to the full
   changelog.

### A release-candidate post

The same voice, in Godot's order:

1. What a candidate is (features locked, only fixes for regressions from now
   on, the final release when a week passes without one) and why testing
   matters. Ask people to back up or use version control.
2. Highlights, or a link to the post that had them.
3. What changed since the last candidate or preview, in the section
   style above, as a short list for small fixes.
4. The commit and tag it's built from, and the changelog.
5. Downloads: the exact `cargo add` and `cargo install` lines (Cargo never
   picks a pre-release by itself) and the C library downloads.
6. Known issues, each linked to its issue.
7. Bug reports: where, and what is most useful.
8. Thanks.

### Credits

Posts are signed by the people who made the work (`authors`) or by the
Ponko team. Commits and pull requests for posts carry no tool attribution:
no co-author lines and no "generated with" footers.

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
