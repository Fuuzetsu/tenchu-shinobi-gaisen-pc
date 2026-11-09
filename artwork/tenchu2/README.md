# Tenchu 2 replacement artwork

Download `tenchu2-hd.tar.zst` from
[tenchu2-artwork-v2](https://github.com/Fuuzetsu/tenchu-shinobi-gaisen-pc/releases/tag/tenchu2-artwork-v2).
Use game v0.20.1 or newer. Put the archive beside the executable or Linux
launcher, or import it with **Import Tenchu 2 HD artwork** in the Android launcher.
Enable **F1 → Visual assets → Replacement artwork**. The Gaisen artwork pack is
independent; either or both can be installed.

Version 2 contains 4,576 replacements. It retains version 1's Europe coverage
and adds all 981 missing identities from the Japanese mission-resource audit:
terrain, props, skies/distant scenery, sequence-specific layouts, character
bodies/costumes, weapons and items. All 3,908 unique identities in that JP audit
are covered, including the evening background variants. This does
not promise HD replacements for story-exclusive artwork or menu sprites.

It preserves the source meshes, UV layouts and cutout shapes. The Gaisen disc
and the appropriate Tenchu 2 disc remain required. See the
[JP gameplay integration](../../docs/tenchu2-native.md) and
[Gaisen scenery sandbox](../../docs/tenchu2-sandbox.md) for gameplay support;
installing artwork does not change which simulation runs.

Related textures were painted together in material/character atlases with the
built-in OpenAI image tool, then cut into individual replacements. Output images
are four times the source texture dimensions; the painted detail varies with the
source and atlas resolution. Original textures were private layout references.
Every shipped RGB image comes from a generated painting. Original alpha/cutout
masks are reconstructed from the user's disc by Rust recipes at load time.
No original reference sheets, meshes or extracted disc files are included.

To edit, unpack into `assets/hd-tenchu2/` or point `TENCHU2_ASSET_PACK` at any
directory containing `pack.json`. An empty environment value disables this pack.
Change `paintings/<identity>.png`, or change the corresponding recipe's file path.
Remove an entry to restore its original disc texture. Shared identities use the
same file across areas. See [pack format and recipes](../../docs/guide.md).

This directory tracks only the manifest, provenance and release checksum. The
large images live in the separate download. Provenance records the source usage,
generated atlas/prompt hashes, crop cells and every shipped image's hash.
Original contributions use the project's MIT License without warranty, to the
extent the authors hold rights. Original Tenchu designs, characters and artwork
remain outside that grant; use of references is not a claim of independent design.
