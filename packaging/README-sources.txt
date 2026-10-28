Tenchu PC — dependency sources

This archive accompanies the native and browser downloads from the same release.
It is not needed to run the game. Keep it available alongside redistributed
binary archives. Each component retains its own upstream license.

upstream/: exact native-library, Rust toolchain and MinGW source archives/trees.
  Original archives retain their contents but are named for their package;
  use tar -xf FILE to unpack an archive. Some entries are already directories.
patches/: patches applied by the pinned Nixpkgs package definitions.
native-dependencies.json: package names, source identities, patches and licenses.
rust-crates/: complete locked Cargo dependencies, including embedded font notices.
nixpkgs/: complete pinned Nix build recipes, local patches and helper scripts.
port/: source of this port, including flake.nix/flake.lock and packaging scripts.

Build from port/ using the README. Nix fetches the pinned build tools and inputs;
this is corresponding dependency source, not an offline Nix binary cache.
For native libraries, their recipes are in nixpkgs/pkgs/ and identify configure
flags and additional patch steps. The port changes ELF loader/search paths when
assembling Linux archives; see port/packaging/bundle-linux.py and tenchu-linux.sh.
Libraries stay separate and dynamically linked so they can be replaced.

The game's disc assets, BIOS and optional private artwork packs are not included.
