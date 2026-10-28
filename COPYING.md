# Distribution and attribution

The project's original code, documentation and resources are available under
the [MIT License](LICENSE), unless a file or its accompanying notice states
otherwise. This permits reuse, modification and redistribution, including
commercial use, with the copyright and license notice retained. The software
is provided as is, without warranty; see LICENSE for the full terms.

Third-party components retain their respective licenses. Binary downloads carry
their notices in `licenses/`; the Nix package installs them under
`share/doc/tenchu-shinobi-gaisen-pc/`. The separate dependency-sources download
contains matching source, patches and build recipes for the copyleft shared
libraries in the portable Linux download. Keep it available beside that bundle.
`bundled-libraries.json` identifies the shipped libraries and their source entries.
Permissively licensed components retain their notices without a duplicate source
download; the archive excludes the Rust compiler and Cargo dependency sources.

The outline and Japanese bitmap fonts are modified, renamed/subsetted OFL fonts;
their provenance and upstream notices are in their respective data directories.
The egui default fonts are covered by the Rust dependency notices. Image recipe
resampling/compositing carries the Pillow/PIL MIT-CMU notice in
`crates/tenchu-pc/data/image-compat/`.

This is an unofficial reimplementation. Tenchu's original game data, artwork,
music and trademarks are not licensed by this project. Supply your own supported
disc. Do not redistribute the disc, extracted `data/` directories, BIOS files,
emulator memory captures or private reference images. The separately distributed HD
pack and disc-backed recipes have their own [provenance record](artwork/README.md);
the MIT grant covers original contributions, not underlying Tenchu designs or
trademarks. Additional artwork packs need their own distribution review.

[SOURCES.md](SOURCES.md) records the reverse-engineering and algorithm references.
It is provenance documentation, not a claim that the implementation was developed
in a clean room or a grant of rights to the original game.
