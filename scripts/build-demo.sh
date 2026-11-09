#!/usr/bin/env bash
# Builds the Tide browser demo (demo/tide) into public/play/tide/.
#
# The output is committed, so website deployments need no Rust toolchain.
# Rebuild after changing the demo or its pinned engine revision:
#
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version 0.2.130 --locked
#   scripts/build-demo.sh
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
crate="$root/demo/tide"
out="$root/public/play/tide"

cargo build --manifest-path "$crate/Cargo.toml" --release --lib --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --remove-name-section --remove-producers-section \
  --out-dir "$out" --out-name game "$crate/target/wasm32-unknown-unknown/release/tide.wasm"
if command -v wasm-opt >/dev/null; then
  wasm-opt -O3 --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
    "$out/game_bg.wasm" -o "$out/game_bg.wasm"
fi
raw=$(wc -c < "$out/game_bg.wasm" | tr -d " ")
packed=$(gzip -9c "$out/game_bg.wasm" | wc -c | tr -d " ")
echo "built $out: game_bg.wasm $((raw / 1024)) KiB, $((packed / 1024)) KiB gzipped"
# The loading screen's progress bar needs the size of the (uncompressed) wasm.
sed -i.bak -E "s/data-wasm-bytes=\"[0-9]+\"/data-wasm-bytes=\"$raw\"/" "$out/index.html" && rm "$out/index.html.bak"
