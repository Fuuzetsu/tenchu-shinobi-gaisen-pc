# Distribution and attribution

The project's original code, documentation and resources are available under
the [MIT License](LICENSE), unless a file or its accompanying notice states
otherwise. This permits reuse, modification and redistribution, including
commercial use, with the copyright and license notice retained. The software
is provided as is, without warranty; see LICENSE for the full terms.

Third-party components retain their respective licenses. Binary downloads carry
their notices in `licenses/`; the Nix package installs them under
`share/doc/tenchu-shinobi-gaisen-pc/`. The separate dependency-sources download
contains the bundled native libraries' source, patches and build recipes, plus
the Rust dependencies and toolchain source. Keep it available beside the binaries.

The outline and Japanese bitmap fonts are modified, renamed/subsetted OFL fonts;
their provenance and upstream notices are in their respective data directories.
The egui default fonts are covered by the Rust dependency notices.

This is an unofficial reimplementation. Tenchu's original game data, artwork,
music and trademarks are not licensed by this project. Supply your own supported
disc. Do not redistribute the disc, extracted `data/` directories, BIOS files,
emulator memory captures or private reference images. Optional artwork packs
need their own provenance and distribution review.

[SOURCES.md](SOURCES.md) records the reverse-engineering and algorithm references.
It is provenance documentation, not a claim that the implementation was developed
in a clean room or a grant of rights to the original game.
