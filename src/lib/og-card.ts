// Share cards: the 1200×630 picture a link to a post shows on social sites
// and in chats. Laid out like the site's own card (public/og.png): the logo,
// the title large on the left, and the post's own picture in dots on the
// right, fading out toward the words. Built at build time: the words are
// turned into outlines with opentype.js (so no fonts need installing where
// the site is built), and sharp turns the SVG into a PNG.
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join } from 'node:path';
import opentype from 'opentype.js';
import sharp from 'sharp';
import { artField, dotFor, dotWidth, SIZES, type ArtOptions } from './dot-art';

export const CARD = { width: 1200, height: 630 } as const;

const require = createRequire(import.meta.url);
const font = (weight: number) => {
  const file = readFileSync(require.resolve(`@fontsource/jost/files/jost-latin-${weight}-normal.woff`));
  // A Buffer can be a window on a bigger shared one: hand over only its bytes.
  return opentype.parse(file.buffer.slice(file.byteOffset, file.byteOffset + file.byteLength) as ArrayBuffer);
};
const fonts = { regular: font(400), medium: font(500), bold: font(600) };
// From the project root: this module is bundled before it runs.
const face = readFileSync(join(process.cwd(), 'public/brand/ponko-face.svg'));

const INK = '#f4e9db';
const MUTED = '#9c8f84';
const NIGHT = '#120e0c';

type Font = opentype.Font;
const width = (f: Font, text: string, size: number) => f.getAdvanceWidth(text, size, { kerning: true });
const words = (f: Font, text: string, x: number, y: number, size: number, fill: string) =>
  `<path fill="${fill}" d="${f.getPath(text, x, y, size, { kerning: true }).toPathData(2)}"/>`;

/** Splits text into lines no wider than `max`, or null if it needs more than `most`. */
function wrap(f: Font, text: string, size: number, max: number, most: number) {
  const lines: string[] = [];
  let line = '';
  for (const word of text.split(/\s+/)) {
    const next = line ? `${line} ${word}` : word;
    if (line && width(f, next, size) > max) {
      lines.push(line);
      line = word;
    } else {
      line = next;
    }
  }
  if (line) lines.push(line);
  return lines.length <= most && lines.every((l) => width(f, l, size) <= max) ? lines : null;
}

/** The post's picture in dots on the right of the card, as SVG. */
const ART_X = 480;
function dots(id: string, art: ArtOptions) {
  const cell = 15;
  const cols = Math.ceil((CARD.width - ART_X) / cell), rows = Math.ceil(CARD.height / cell);
  const aspect = cols / rows;
  const { colours, at } = artField(id, art, aspect, { cols, rows });
  let rings = '';
  let fills = '';
  for (let j = 0; j < rows; j++) {
    for (let i = 0; i < cols; i++) {
      const x = ART_X + (i + 0.5) * cell, y = (j + 0.5) * cell;
      rings += `M${x + cell * 0.3} ${y}a${cell * 0.3} ${cell * 0.3} 0 1 0 0 .01`;
      const f = at(((i + 0.5) / cols) * aspect, (j + 0.5) / rows, 0);
      const dot = dotFor(f, colours.length);
      if (dot) fills += `<circle cx="${x}" cy="${y}" r="${((dotWidth(f * SIZES) * cell) / 2).toFixed(2)}" fill="${colours[dot.shade]}"/>`;
    }
  }
  return `<path d="${rings}" fill="none" stroke="${colours[0]}" stroke-width="${cell * 0.06}" opacity="0.6"/>${fills}`;
}

export type CardInput = {
  /** Seeds the picture; the post's id. */
  id: string;
  art?: ArtOptions;
  title: string;
  /** A short line above the title, like "Blog". */
  kicker: string;
  /** The line at the bottom, like a date. */
  footer: string;
};

/** A share card as a PNG. */
export async function card({ id, art = {}, title, kicker, footer }: CardInput) {
  const left = 72;
  const max = 620;
  // The largest size that fits the title in three lines.
  let size = 76;
  let lines = wrap(fonts.medium, title, size, max, 3);
  while (!lines && size > 40) {
    size -= 4;
    lines = wrap(fonts.medium, title, size, max, 3);
  }
  lines ??= [title];
  const lead = size * 1.06;
  const top = 316 - ((lines.length - 1) * lead) / 2;
  const titlePaths = lines.map((l, k) => words(fonts.medium, l, left - 2, top + k * lead, size, INK)).join('');

  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${CARD.width}" height="${CARD.height}" viewBox="0 0 ${CARD.width} ${CARD.height}">
  <defs>
    <linearGradient id="fade" x1="0" x2="1" y1="0" y2="0">
      <stop offset="0.46" stop-color="#fff" stop-opacity="0"/>
      <stop offset="0.7" stop-color="#fff" stop-opacity="1"/>
    </linearGradient>
    <mask id="art"><rect width="100%" height="100%" fill="url(#fade)"/></mask>
  </defs>
  <rect width="100%" height="100%" fill="${NIGHT}"/>
  <g mask="url(#art)">${dots(id, art)}</g>
  <image href="data:image/svg+xml;base64,${face.toString('base64')}" x="${left}" y="74" width="54" height="42"/>
  ${words(fonts.bold, 'Ponko', left + 66, 108, 32, INK)}
  ${words(fonts.regular, kicker, left + 66 + width(fonts.bold, 'Ponko', 32) + 18, 108, 32, MUTED)}
  ${titlePaths}
  ${words(fonts.regular, footer, left, 552, 26, MUTED)}
</svg>`;
  return sharp(Buffer.from(svg)).png({ compressionLevel: 9 }).toBuffer();
}
