// Addresses and facts used across the site.
export const site = {
  name: 'Ponko Engine',
  url: 'https://ponko.dev',
  github: 'https://github.com/ponkoengine/ponko',
  discussions: 'https://github.com/ponkoengine/ponko/discussions',
  api: 'https://docs.rs/ponko/',
  websiteSource: 'https://github.com/ponkoengine/ponko-website',
  /** The engine's web gallery, one page per example. */
  gallery: 'https://examples.ponko.dev/',
};

// The release the site describes. Change these four lines when a release
// goes out: the home page, the Learn page and the install lines read them.
export const release = {
  version: '0.2.0-rc.1',
  /** The day the version was published on crates.io. */
  date: '2026-11-28',
  /** What `cargo add ponko` installs without a version. */
  stable: '0.1.0',
  /** The command that installs the version above. */
  install: 'cargo add ponko@0.2.0-rc.1',
};

/** The header's links, also used by the footer and the phone menu. */
export const nav: [string, string][] = [
  ['Learn', '/learn/'],
  ['Examples', '/examples/'],
  ['Blog', '/blog/'],
  ['Community', '/community/'],
  ['GitHub', site.github],
];
