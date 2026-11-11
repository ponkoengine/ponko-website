import { test } from "node:test";
import assert from "node:assert/strict";
import { slugify, template } from "./new-post.mjs";

test("slugify makes a short, plain address from a title", () => {
  assert.equal(slugify("Ponko 0.1: What's new?"), "ponko-0-1-whats-new");
  assert.equal(slugify("  Déjà vu — again  "), "deja-vu-again");
  assert.equal(slugify("!!!"), "");
  assert.ok(slugify("a".repeat(100)).length <= 60);
});

test("a new post starts as a draft with today's date", () => {
  const text = template('Say "hi"', "2026-11-11");
  assert.match(text, /^title: "Say \\"hi\\""$/m);
  assert.match(text, /^date: 2026-11-11$/m);
  assert.match(text, /^draft: true$/m);
});
