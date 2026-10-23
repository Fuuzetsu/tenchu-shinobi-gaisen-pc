# Outline interface font

`TenchuUISans-Regular.ttf` is a static, renamed subset of Noto Sans JP,
distributed under the SIL Open Font License 1.1 in `OFL-1.1.txt`.
It contains 7,193 Unicode mappings: the original Shift-JIS repertoire, half-width
katakana, Latin characters and punctuation needed by the supported languages.
The original game bitmap font remains available; this font is presentation only.

Upstream: [Google Fonts, pinned revision](https://github.com/google/fonts/tree/66a36c8c94b1a5d992ee4e7f392fccfe4945767c/ofl/notosansjp).
Source file: `NotoSansJP[wght].ttf`, SHA-256
`c2f3b4d463500a2ddcd3849cded1fceeb9fd6d1c32e6cbecd568453ba50fc68f`.
The source is instantiated at weight 400, subset and renamed using FontTools
from the repository's pinned Nix development shell. No font service or system
font installation is required at runtime, including in the browser.

Regenerate from the downloaded source at the repository root:

```sh
./dev.sh python tools/make_hd_font.py '/path/to/NotoSansJP[wght].ttf' \
  crates/tenchu-pc/data/hd-font/TenchuUISans-Regular.ttf
```

Retain the upstream OFL copyright and license text beside the font.
