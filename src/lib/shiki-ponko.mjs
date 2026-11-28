// Code blocks for the blog: Ponko's colours, and a frame around each block
// with the language (or a file name) and a copy button.

const languages = { rs: 'Rust', rust: 'Rust', toml: 'TOML', sh: 'Shell', bash: 'Shell', shell: 'Shell', zsh: 'Shell', console: 'Shell', wgsl: 'WGSL', js: 'JavaScript', ts: 'TypeScript', json: 'JSON', c: 'C', cpp: 'C++', glsl: 'GLSL', yaml: 'YAML', md: 'Markdown', diff: 'Diff', html: 'HTML', css: 'CSS', plaintext: 'Text', text: 'Text', txt: 'Text' };

/** Wraps each block in a frame. Name a file with ```rust title="src/main.rs" */
export const codeFrame = {
  name: 'ponko-code-frame',
  root(root) {
    const pre = root.children.find((n) => n.type === 'element' && n.tagName === 'pre');
    if (!pre) return;
    const meta = this.options.meta;
    const raw = typeof meta === 'string' ? meta : meta?.__raw ?? '';
    const title = /title=(?:"([^"]*)"|'([^']*)')/.exec(raw);
    const lang = this.options.lang;
    const label = title ? title[1] ?? title[2] : languages[lang] ?? lang;
    const text = (value) => ({ type: 'text', value });
    root.children = [{
      type: 'element',
      tagName: 'div',
      properties: { className: ['code-block'] },
      children: [
        {
          type: 'element',
          tagName: 'div',
          properties: { className: ['code-block-bar'] },
          children: [
            { type: 'element', tagName: 'span', properties: {}, children: [text(label ?? '')] },
            { type: 'element', tagName: 'button', properties: { type: 'button', className: ['code-copy'] }, children: [text('Copy')] },
          ],
        },
        pre,
      ],
    }];
  },
};

// Code colours for the blog, in Ponko's palette. Night: cream text on warm
// night, orange keywords, amber functions, sage strings. Paper (below) is the
// same roles in ink on white.
export const ponkoNight = {
  name: 'ponko-night',
  type: 'dark',
  colors: {
    'editor.background': '#1a1411',
    'editor.foreground': '#eadccd',
  },
  tokenColors: [
    { scope: ['comment', 'punctuation.definition.comment'], settings: { foreground: '#8a7b70', fontStyle: 'italic' } },
    { scope: ['keyword', 'storage', 'storage.type', 'storage.modifier', 'keyword.control', 'keyword.other'], settings: { foreground: '#f08a5d' } },
    { scope: ['keyword.operator', 'punctuation', 'meta.brace'], settings: { foreground: '#b3a397' } },
    { scope: ['string', 'string.quoted', 'markup.inline.raw'], settings: { foreground: '#b9cf8a' } },
    { scope: ['constant.character.escape', 'string.regexp'], settings: { foreground: '#d6e3a8' } },
    { scope: ['entity.name.function', 'support.function', 'meta.function-call', 'variable.function'], settings: { foreground: '#f3c26b' } },
    { scope: ['entity.name.type', 'support.type', 'entity.name.class', 'entity.name.struct', 'entity.name.enum', 'entity.name.trait', 'storage.type.rust', 'support.class'], settings: { foreground: '#f2d9b0' } },
    { scope: ['constant.numeric', 'constant.language', 'constant.other', 'support.constant'], settings: { foreground: '#ff9d6e' } },
    { scope: ['entity.name.function.macro', 'support.macro', 'meta.macro'], settings: { foreground: '#e79ab5' } },
    { scope: ['meta.attribute', 'entity.other.attribute-name', 'punctuation.definition.attribute', 'storage.modifier.lifetime', 'entity.name.lifetime'], settings: { foreground: '#c9a7d8' } },
    { scope: ['variable.parameter', 'variable.other.member', 'entity.name.tag'], settings: { foreground: '#eadccd' } },
    { scope: ['entity.name.namespace', 'entity.name.module'], settings: { foreground: '#d8c3ae' } },
    { scope: ['markup.inserted', 'meta.diff.header.to-file'], settings: { foreground: '#b9cf8a' } },
    { scope: ['markup.deleted', 'meta.diff.header.from-file'], settings: { foreground: '#ff8a7a' } },
    { scope: ['markup.heading'], settings: { foreground: '#f08a5d', fontStyle: 'bold' } },
  ],
};

// The same roles on paper: each night colour and the ink that stands in for it.
const onPaper = {
  '#eadccd': '#2b221c', '#8a7b70': '#8c8078', '#b3a397': '#75675d', '#b9cf8a': '#4a7419',
  '#d6e3a8': '#3c6410', '#f08a5d': '#b8430f', '#f3c26b': '#8a5a00', '#f2d9b0': '#6b4a2b',
  '#ff9d6e': '#c2410c', '#e79ab5': '#a83e73', '#c9a7d8': '#7a4b99', '#d8c3ae': '#5c4a3c',
  '#ff8a7a': '#b3261e',
};
export const ponkoPaper = {
  name: 'ponko-paper',
  type: 'light',
  colors: { 'editor.background': '#ffffff', 'editor.foreground': '#2b221c' },
  tokenColors: ponkoNight.tokenColors.map((t) => ({
    ...t,
    settings: { ...t.settings, foreground: onPaper[t.settings.foreground] ?? t.settings.foreground },
  })),
};
