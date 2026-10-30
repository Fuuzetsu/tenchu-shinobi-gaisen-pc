Tenchu PC — dependency sources

This archive supplies matching sources for the copyleft shared libraries included
in the portable Linux download. It is not needed to run the game or to build the
port normally with Nix. Keep it available alongside redistributed Linux bundles.
Each component retains its upstream license; the game remains MIT-licensed.

upstream/: exact sources for bundled GPL/LGPL library components.
  Original archives retain their contents but are named for their package;
  use tar -xf FILE to unpack an archive. Some entries are already directories.
  Packages built from the same source share one copy, including systemd variants.
patches/: patches applied by the pinned Nixpkgs package definitions.
source-packages.json: component-to-source, patch and recipe mapping, with the
  exact Nixpkgs revision. Compare it with bundled-libraries.json in the Linux
  download to see which distributed libraries it covers.
recipes/: relevant Nixpkgs package definitions, local patches and shared build
  helpers. This is a reviewable subset, not a standalone Nixpkgs checkout.
port/: source of this port, including flake.nix/flake.lock and packaging scripts.

Build from port/ using the README. Nix fetches the pinned build tools and inputs;
this is corresponding dependency source, not an offline Nix binary cache.
The original upstream GCC archive is included for the libgcc runtime we ship;
we do not distribute the compiler executable. Rust compiler sources, Cargo
dependencies, unrelated Nixpkgs packages and permissively licensed library
sources are omitted. Their required license notices remain in the game downloads.
The port changes ELF loader/search paths when
assembling Linux archives; see port/packaging/bundle-linux.py and tenchu-linux.sh.
Libraries stay separate and dynamically linked so they can be replaced.

HD artwork is a separate tenchu-hd.tar.zst download in the artwork-v2 release.
port/artwork contains its manifest, recipes, provenance and editing instructions.
Original disc assets, BIOS, reference screenshots and reconstructed composites
are not included.
