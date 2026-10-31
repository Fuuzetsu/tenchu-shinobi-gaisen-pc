Tenchu: Shinobi Gaisen PC — browser build

This is a static website, including the WASM module and JavaScript bindings.
It needs assets prepared from your own Shinobi Gaisen disc. Use a matching
native build to create the data directory inside this extracted archive:
  tenchu-pc prepare-web '/path/to/game.cue' ./data

For HD artwork, download tenchu-hd.tar.zst from the artwork-v3 release and place
it beside the native executable before preparing assets. TENCHU_ASSET_PACK can
also select the archive or an unpacked directory. Preparation exports generated
layers and recipes alongside your disc data; reconstruction runs in WASM.

Then serve this directory, for example with Python 3:
  python3 serve-web.py .
Open http://127.0.0.1:8000 and click Start game or Quick launcher.
For remote hosting, serve over HTTPS. Do not open index.html as a file:// URL.
WebGPU is preferred; WebGL2 is available as a compatibility renderer.

Keep index.html, app.js, style.css, pkg/ and your data/ directory together.
No disc image or extracted game assets are included. Do not redistribute the
data directory created from your disc.

build-info.json identifies this download. Keep licenses/ with the site. The
dependency-source download covers shared libraries in the Linux bundle; it is
not needed for this browser download.
