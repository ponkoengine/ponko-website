// Starts a new blog post: npm run new-post -- "The title of the post"
// Makes src/content/blog/<slug>/index.mdx as a draft dated today, with a
// folder of its own for pictures. See WRITING.md.
import { mkdir, writeFile, access } from "node:fs/promises";
import { resolve } from "node:path";

export function slugify(title) {
  return title
    .normalize("NFKD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/['’]/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 60)
    .replace(/-+$/, "");
}

export function template(title, date) {
  const quoted = JSON.stringify(title);
  return `---
title: ${quoted}
description: One or two sentences that say what the post is about. They show in the list of posts, in search results and on social cards.
date: ${date}
tags: []
draft: true
---

Start writing here. Plain Markdown works, and so do the components in
WRITING.md, such as <Callout>, <Figure>, <Steps> and <Play>, with no import.

## A heading

\`\`\`rust title="src/main.rs"
use ponko::prelude::*;
\`\`\`
`;
}

async function main() {
  const title = process.argv.slice(2).join(" ").trim();
  if (!title) {
    console.error('Give the post a title: npm run new-post -- "The title of the post"');
    process.exit(1);
  }
  const slug = slugify(title);
  if (!slug) {
    console.error("The title needs at least one letter or number for the address.");
    process.exit(1);
  }
  const dir = resolve("src/content/blog", slug);
  const file = resolve(dir, "index.mdx");
  const exists = await access(file).then(() => true, () => false);
  if (exists) {
    console.error(`There is already a post at src/content/blog/${slug}/. Pick another title.`);
    process.exit(1);
  }
  await mkdir(dir, { recursive: true });
  await writeFile(file, template(title, new Date().toISOString().slice(0, 10)));
  console.log(`Made src/content/blog/${slug}/index.mdx`);
  console.log(`It's a draft: see it at http://127.0.0.1:4321/blog/${slug}/ with npm run dev,`);
  console.log("and set draft: false when it's ready to publish.");
}

if (import.meta.url === `file://${process.argv[1]}`) await main();
