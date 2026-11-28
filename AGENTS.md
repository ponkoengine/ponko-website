# Working on ponko-website

The site at https://ponko.dev. Astro builds the pages and the blog; the guide
under `/guide/` is the engine's mdBook, built from the engine's `main`.

## Where things are

| Path | What |
|---|---|
| `src/site.ts` | Addresses, the header's links, and the release the site describes. Change `release` when a version ships. |
| `src/pages/` | One file per page: home, Learn, Examples, Community, the blog. |
| `src/components/` | Shared parts: header, footer, the install line, the game cards, the dot art. |
| `src/styles/global.css` | Every page's styles. The colours and fonts are the variables at the top. |
| `src/content/blog/` | Posts. [WRITING.md](WRITING.md) says how to write one. |
| `demo/` | The games behind `/play/`, built by hand into `public/play/`. |
| `scripts/` | The guide build and the checks. |

## Branches and pull requests

- Never commit to `main`: every push to it deploys the live site. Work on a
  branch, in a worktree, and open a pull request. One pull request per piece
  of work, squash-merged.
- There is no CI. Before you open or merge a pull request, run:

  ```sh
  npm ci
  npm test
  npm run build     # the site and the guide
  npm run check     # every local link and asset in dist/
  ```

- Look at a changed page in a browser at a desktop width and a phone width
  (390 px) before merging: no sideways scrolling, nothing that jumps as the
  page loads, and it still works with JavaScript off and with reduced motion.
- Keep GitHub traffic low: no polling loops, and no more than two or three
  agents working against GitHub at once.

## What goes on the site

- **Every claim has a source.** A number (entities, frame times, sizes, build
  times) comes from a measurement in the engine repository or one you ran,
  and the pull request says where. Say what was measured, on what machine.
- **Say what is not done.** The site says which platforms have a recorded run
  of the checks and which have none. Keep that current; do not round it up.
- No made-up users, quotes, logos or counts. No words like "production-ready"
  or "blazing fast".
- Only things that are public. Nothing from a private project goes on the
  site: no names, screenshots, numbers or story.
- Pictures and clips come from the engine running. Commit the picture, not a
  script that made it.
- Posts have a person's name as the author and no tool credits. Commits that
  only add a post or artwork carry no co-author line.

## Design

- Two themes from one set of variables in `global.css` (`--night`,
  `--panel`, `--ink`, `--accent`, ...): paper, which is the default, and ink,
  which the button in the header switches to. Each variable is written
  `light-dark(paper, ink)`. Use the variables, never a fixed colour, so both
  themes keep working.
- Code, terminals, number boards and anything a game draws are "screens":
  they stay dark in both themes. Add a new one to the "Screens" rule in
  `global.css`.
- Two fonts, Jost and JetBrains Mono, served from the site. Do not add a font
  or script from another host.
- The site's own mark is the dot: the logo, the post pictures, the number
  boards and the benchmark bars are all drawn in dots. Reach for that before
  adding a new kind of decoration.
- Pages are HTML first. Scripts are small and optional: a page must read
  correctly before they run. Anything that moves stops under
  `prefers-reduced-motion`.
- Give every image a `width` and `height`, and real alt text.
- Write plainly, as in [WRITING.md](WRITING.md): short sentences, the fact
  first, no slogans.
