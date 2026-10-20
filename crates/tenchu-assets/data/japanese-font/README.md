# Japanese subtitle font

`glyphs.bin` is a subset and format conversion of GNU Unifont JP 16.0.04,
distributed here under the SIL Open Font License 1.1. See [OFL-1.1.txt](OFL-1.1.txt)
for copyright and license notices. This is a replacement font, not PS1 BIOS data.

Source: [unifont_jp-16.0.04.hex.gz](https://unifoundry.com/pub/unifont/unifont-16.0.04/font-builds/unifont_jp-16.0.04.hex.gz).
SHA-256: `5ba84e901b9f7fad3bce0571c7e4b4b0aef4ce0acc4ee6622ef0cfa2eef38a6f`.
The upstream source archive is [unifont-16.0.04.tar.gz](https://unifoundry.com/pub/unifont/unifont-16.0.04/unifont-16.0.04.tar.gz),
SHA-256 `2bd4e4679757126f48e1bf2c1be40b09aa92162bfedda4683ce5fbc70a2a5972`.
Its `COPYING` file supplies the dual-license grant; `font/Makefile` supplies the
copyright notice. No upstream program code is included.

The binary contains 6,879 sorted 32-byte records: a big-endian Shift-JIS code
followed by fifteen big-endian 16-bit bitmap rows. Standard double-byte Shift-JIS
characters are retained. Narrow glyphs are centered. To fit the game's fifteen
rows, a used bottom row moves up if the first row is empty; otherwise the bottom
two rows are combined. The game's custom star comes from the user's executable
and replaces the bundled star at runtime.

Regenerate from the downloaded font, at the repository root:

```sh
python3 tools/make_japanese_font.py unifont_jp-16.0.04.hex.gz \
  crates/tenchu-assets/data/japanese-font/glyphs.bin
```

The conversion was made on 2026-10-20. The generated bitmap is embedded, so runtime
and offline Nix builds need neither font packages nor a network connection.
