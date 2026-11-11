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

// Code colours for the blog, in Ponko's palette: cream text on warm night,
// orange keywords, amber functions, sage strings.
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
