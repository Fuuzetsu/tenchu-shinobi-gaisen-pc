# Campaign HD artwork

The artwork is a separate download in this repository's **artwork-v1** GitHub
Release: `tenchu-hd.tar.zst` plus `SHA256SUMS`. Keep the archive beside the game
executable (beside the Linux launcher), or in your working directory. The game
finds it automatically. Enable **F1 → Visual assets → Replacement artwork**;
Modern presets enable it too. The supported retail disc is still required.

Alternatively set `TENCHU_ASSET_PACK` to an archive or an unpacked directory
containing `pack.json`. An empty value disables loading. Archives are unpacked
once into the user's cache, keyed by SHA-256; subsequent loads reuse that copy.
`TENCHU_ASSET_CACHE` overrides the cache location. Nothing is extracted into the
game installation. No external zstd, Python or image generator is needed to play.
Browser preparation accepts the same pack and exports its files for prefetch;
the browser does not download or unpack the tarball itself.

## Editing a pack

Unpack into `assets/hd/` beside the executable or in the source checkout, or
choose any directory with `TENCHU_ASSET_PACK`. `assets/hd/` takes priority over a
neighbouring archive. Edit this directory, not the game's disposable cache.

```sh
mkdir -p assets/hd
tar --zstd -xf tenchu-hd.tar.zst -C assets/hd
```

The root contains `pack.json`, ordinary PNG textures, OBJ meshes, this README,
license and provenance. `pack.json` maps texture identities to relative paths:

```json
{"version": 1, "textures": {"texture-id-from-the-catalog": "textures/roof.png"}}
```

Filenames can be descriptive; identities stay stable because they identify the
original game texture. Export a catalog with the game's `export-assets` command
to see previews and find the intended surface ([asset guide](../docs/guide.md)).
Change an existing PNG, or point its entry at a new one. Use RGBA PNGs up to
1024×1024; preserve the aspect ratio, transparency and matching tile borders.
An identity in `recipes` is reconstructed from disc pixels plus generated layers.
To replace it with a complete painting, remove that recipe and add its identity
to `textures`. The guide describes recipes, materials and model overrides.
Use **F1 → Visual assets → Reload asset pack** to reload an edited native pack.
Provenance hashes describe the official release; the game does not enforce them
on custom packs. Keep your changes in your own directory for future updates.

## Contents and provenance

The pack covers 1,719 texture identities across Missions 1–10, characters, items,
HUD and all four interface languages, plus two authored weapon meshes. Cutscenes
use their existing game assets. The manifest keeps tile borders, blend classes,
surface rules and model corrections together.

301 identities use Rust recipes at menu/mission load. Mixed original colour,
source cutouts, snow coverage and interface blend masks come from the user's
decoded disc textures. The download supplies generated layers and operations,
not precomposited copies of those source pixels. Results match the accepted pack
pixel for pixel. Unrelated mission images are not decoded; reconstructed images
are not written into the pack or cache.

This source directory keeps the manifest, per-entry provenance and hashes;
large PNG/OBJ inputs live only in the separate download. `provenance.json`
records replacement methods, authoring source labels and weapon provenance.
Those labels refer to the development archive, not runtime dependencies.
Original textures/screenshots were used as references for the repaints. This
is not a claim of independent character/environment design or legal clearance.

Original contributions use the accompanying MIT License without warranty.
Original Tenchu designs, names, artwork and trademarks are outside that grant.
The project's renamed/subsetted fonts retain their OFL notices with the game.

Maintainers: use `tools/package-artwork.sh` with the reviewed unpacked pack.
It verifies the input inventory/hashes and writes a deterministic tar.zst plus
checksum. See [release instructions](../docs/releases.md#artwork-releases).
