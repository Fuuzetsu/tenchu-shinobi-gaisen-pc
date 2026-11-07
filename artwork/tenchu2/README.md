# Tenchu 2 replacement artwork

Download `tenchu2-hd.tar.zst` from
[tenchu2-artwork-v1](https://github.com/Fuuzetsu/tenchu-shinobi-gaisen-pc/releases/tag/tenchu2-artwork-v1).
Use game v0.20.1 or newer. Put the archive beside the executable or Linux
launcher, or import it with **Import Tenchu 2 HD artwork** in the Android launcher.
Enable **F1 → Visual assets → Replacement artwork**. The Gaisen artwork pack is
independent; either or both can be installed.

The pack covers all 3,595 unique texture identities used by the sandbox's 23
nonempty areas, static props and 81 imported character bodies/alternate costumes.
It preserves the source meshes, UV layouts and cutout shapes. This is an artwork
pack, not a port of Tenchu 2's missions or scripts. Both the Gaisen disc and the
supported Tenchu 2 Europe disc remain required; see [sandbox scope](../../docs/tenchu2-sandbox.md).

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
