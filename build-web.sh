#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
if [[ $# -lt 1 || $# -gt 2 || $1 == --help ]]; then
    echo 'Usage: ./build-web.sh /path/to/game.cue [output directory]'
    echo 'Default output: private/web. Existing prepared assets are reused.'
    exit 0
fi
web_disc=$(realpath -- "$1")
web_output=$(realpath -m -- "${2:-private/web}")
./dev.sh cargo build --locked --release -p tenchu-pc
if [[ ! -e "$web_output/data/manifest.json" ]]; then
    ./dev.sh target/release/tenchu-pc prepare-web "$web_disc" "$web_output/data"
elif [[ -n ${TENCHU_ASSET_PACK:-} ]]; then
    ./dev.sh target/release/tenchu-pc prepare-web "$web_disc" "$web_output/data" --artwork
fi
./dev.sh cargo rustc --locked --release --target wasm32-unknown-unknown -p tenchu-pc --lib --crate-type cdylib
./dev.sh wasm-bindgen --target web --out-dir "$web_output/pkg" target/wasm32-unknown-unknown/release/tenchu_pc.wasm
cp crates/tenchu-pc/web/{index.html,app.js,style.css} "$web_output/"
mkdir -p "$web_output/licenses/japanese-font"
cp crates/tenchu-assets/data/japanese-font/{OFL-1.1.txt,README.md} "$web_output/licenses/japanese-font/"
mkdir -p "$web_output/licenses/hd-font"
cp crates/tenchu-pc/data/hd-font/{OFL-1.1.txt,README.md} "$web_output/licenses/hd-font/"
printf 'Built %s\nServe with: ./dev.sh python3 tools/serve-web.py %q\n' "$web_output" "$web_output"
