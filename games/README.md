# Playable examples

Three asset-free games using Ponko's actual renderer:

- **Asteroids:** inertia, screen wrapping, shooting, rock splitting, lives, waves.
- **Breakout:** fifty bricks, paddle-angle rebounds, lives, win state.
- **Arena:** 3D meshes, a shadowed directional light, five point lights, cover,
  chasing drones, projectiles, debris, dash, health, escalating waves up to 200 drones.

The simulation modules are independent of the window/GPU, so gameplay tests run
headlessly. `src/bin/*.rs` connects the simulations to Ponko. `src/runtime.rs`
shares HUD rendering and a small browser input/status bridge. The HTML shell
handles loading, errors, touch controls, focus, pause, and restart. Game logic
and drawing run in Rust; there is no JavaScript substitute for the games.

From the website root:

```sh
npm run build:games     # downloads the pinned public engine and compiles WASM
npm run test:games      # native simulation tests, no GPU
npm run dev            # serve the site with generated games
```

The full `npm run build` also builds the games. The builder installs Rust and
wasm-bindgen if needed, under the ignored `.cache` directory when the host lacks
Rust. The exact engine SHA, Rust version and wasm-bindgen version are in
`engine.json`; dependency resolution is locked in `Cargo.lock`. Generated assets
live under ignored `public/play/`, with checksums and a Rust source fingerprint
in `public/play/build-info.json`. No sibling checkout is read during a build.

Native play (after `npm run build:games` prepares the public dependency):

```sh
cargo run --manifest-path games/Cargo.toml --release --features runtime --bin arena
```

The homepage reads the simulation and renderer files directly for its syntax
highlighted code panels. Edit a Rust file and rerun `npm run build:games` before
comparing the game with the source viewer. The simulations use a 120 Hz fixed
step, clamped frame catch-up, and bounded enemy/projectile lifetimes. Games pause
when their tab is hidden, their canvas loses focus, or the parent marks them
outside the viewport. The parent removes the iframe when you stop, switch games,
or select the code-only view.

Browser checks: build the site, `npx playwright install chromium`, then
`npm run test:browser`. Tests download actual WASM, play through keyboard input,
check scoring/restart/pause, exercise layout controls and loading failures.
Set `UPDATE_SCREENSHOTS=1` to regenerate the homepage's real gameplay screenshots.
