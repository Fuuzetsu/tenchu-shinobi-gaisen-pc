# Releasing

The workflow builds Linux x86-64, Linux ARM64, Windows x86-64 and WASM. Releases
contain those four archives, a dependency-source archive and `SHA256SUMS`.
Every archive has `build-info.json` identifying its version, commit and Nixpkgs
revision; native `tenchu-pc --version` also identifies its build.

## Development downloads

Keep `master` as the repository's default branch. Pushes to `master` build and
test all targets. Successful builds update the `head` tag and **HEAD (development)**
prerelease. HEAD does not replace the latest stable release. Other branches and
pull requests only produce temporary Actions artifacts.

Publication is serialized, waits for all targets and the Windows smoke test,
and verifies checksums and build identities. A rerun for an old branch commit
cannot overwrite a newer HEAD. New pushes do not cancel an ongoing publication.
HEAD is temporarily a draft during replacement;
failed uploads leave a draft instead of a partially updated public download.
Rerun the failed job to finish it. Keep normal version tags protected against
updates/deletion; exclude `head` from that rule.

The HEAD release must stay mutable. If you want GitHub's release immutability
for versioned releases, let this workflow create HEAD **before** enabling it;
GitHub applies that setting to future releases. The publisher refuses to replace
an immutable HEAD or overwrite any published versioned release, including prereleases.
See [GitHub's release immutability documentation](https://docs.github.com/en/code-security/how-tos/secure-your-supply-chain/establish-provenance-and-integrity/prevent-release-changes).

## Versioned releases

1. Add user-facing features, fixes and compatibility changes to `CHANGELOG.md`
   under `Unreleased` as work lands. Do not copy the commit log or record internal
   refactors, formatting or test-only changes there.
2. Choose the version. Set `[workspace.package].version` in `Cargo.toml`, then
   run `./dev.sh cargo check --workspace` to update the workspace packages in
   `Cargo.lock`. Nix reads the same version; there is no second version to edit.
3. Move the release's entries beneath `## 0.2.0 — YYYY-MM-DD` (use the actual
   version/date). Leave an empty `## Unreleased` above it for subsequent work.
   The initial Unreleased section describes the first public release, not a
   history of releases that never happened.
4. Run the checks below, perform a short disc-backed gameplay check, and commit.
   Push the commit, then create and push an annotated tag, for example:

   ```sh
   git tag -a v0.2.0 -m 'Tenchu PC 0.2.0'
   git push origin v0.2.0
   ```

Tags must be `vMAJOR.MINOR.PATCH`, optionally `-alpha.N`, `-beta.N` or `-rc.N`.
The version must match Cargo and have dated, nonempty changelog notes. Prerelease
tags produce GitHub prereleases. Stable tags use GitHub's version/date ordering
for Latest. Builds upload to a draft and publish only after every asset arrives.
Failed draft releases can be retried; fixes to a published version need a new tag.
Do not move a published tag or silently replace its binaries.

```sh
nix flake check
nix build .#linux-archive .#windows-archive .#web-archive .#dependency-sources-archive
python3 packaging/release.py notes refs/tags/v0.2.0 /tmp/tenchu-release-notes.md
```

Use the intended tag in the last command. No retail disc is required by hosted
CI. It checks native unit tests, Clippy, formatting, WASM builds, workflow/scripts,
Linux startup/library loading without `/nix`, and Windows executable startup.
It cannot validate gameplay/GPU drivers without a disc and suitable hardware.

## Artwork releases

Host the HD pack in this same repository's GitHub Releases, independently of
code releases. The compatible tag, archive name, size and checksum are recorded
in `artwork/release.json`; game release notes link that tag. `artwork-v1` does
not match the `v*` workflow trigger and does not become a game release. Keep
`master` as the default branch.

1. Keep the editable pack outside tracked source. Update the reviewed
   `artwork/pack.json` and `artwork/provenance.json` when its inputs change.
   Original disc exports and precomposited source pixels stay out of the pack.
2. Package the reviewed directory using pinned tools:

   ```sh
   ./dev.sh bash tools/package-artwork.sh /path/to/reviewed-pack /path/to/new-output
   ```

   This checks all referenced inputs and hashes, writes a deterministic
   `tenchu-hd.tar.zst` with `pack.json` at its root, and produces `SHA256SUMS`.
   Use ordinary files/directories, no symlinks; extraction accepts at most
   8,192 entries, 512 MiB total, 64 MiB per file and a 4 MiB manifest.
3. Record the resulting checksum/size and the new `artwork-vN` tag in
   `artwork/release.json`. Verify packed and unpacked loading and browser export.
   Commit metadata, instructions and provenance, never the archive or PNG/OBJ blobs.
4. After the public repository exists, create a draft release at that commit,
   titled **HD artwork vN**, tag `artwork-vN`. Upload the tarball and checksum,
   explain compatibility and the disc requirement, then publish with **Set as
   latest release** disabled. Publish this before game releases that link it.
   Do not replace published artwork: make `artwork-v2` for changed contents.

The game extracts the archive with pure Rust into a content-addressed user cache,
without external tools. Unpacked folders remain editable. Browser preparation
copies the same referenced files into menu/background prefetch groups; generated
recipes execute from local disc textures in WASM. Native and browser binaries
build independently of the artwork download. There is no automatic network
fetch during gameplay.

## Public history

The full audit through the separation commit (270 reachable commits,
4,545 unique blobs) found 1,710 unique PNG blobs, all introduced under `assets/hd/`
in the recent HD import, plus the two documented OFL font binaries. It found no
disc/retail executables, BIOS, RAM captures, screenshots or recording files,
and no matches for the checked credential patterns.

Before publication, `assets/hd/` was removed from this repository's entire
history using `git filter-repo`. The cleanup retained all 272 existing commits
and left the current source tree unchanged. A fresh history audit found only
the two documented OFL font binaries, no HD PNG/OBJ inputs, no suspect resource
paths and no matches for the checked credential patterns. Git's integrity check
passed with no unreachable objects; the old asset blobs were pruned.

Continue development and publish from the usual
`~/programming/tenchu-shinobi-gaisen-pc` directory on `master`. The temporary
`private/public-repository` checkout was removed; there is only one repository
to maintain. The editable HD pack and its release tarball remain outside Git.
Rebuild release artifacts from the rewritten history because commit IDs changed.
Do not merge or push any older, unfiltered history back into this repository.

## Publication review (2026-10-28)

- **MIT license for original project code/resources.** [COPYING.md](../COPYING.md)
  distinguishes the project's license from third-party notices. The
  reverse-engineering provenance in [SOURCES.md](../SOURCES.md) does
  not grant rights to the original game or claim clean-room development.
- **v0.1.0 and artwork-v1 are published.** Both initial hosted game workflows
  completed successfully; Linux x86-64/ARM64, Windows, WASM, source downloads
  and checksums are public. `master` is the hosted default branch in
  `Fuuzetsu/tenchu-shinobi-gaisen-pc`. The v0.2.0 release reuses `artwork-v1` and
  adds input remapping, vibration controls, remembered discs and smaller sources.
- **Keep private:** disc images, extracted browser `data/`, BIOS, RAM/trace
  captures and recordings/reference screenshots.
  Full-history findings and the completed history cleanup are above.
  Generated composites and original reference assets remain ignored. A
  filename/content scan is not a legal certification of the code.
- **Publish with the binaries:** changelog, concise run instructions, checksums,
  build identity, font/Rust/native-library notices and corresponding dependency
  sources. The Linux bundle contains copyleft libraries; preserve the source
  archive alongside redistribution. It includes their upstream sources, applied
  patches, relevant pinned Nixpkgs recipes and this port's packaging code.
  `bundled-libraries.json` maps the shipped libraries to `source-packages.json`;
  publication checks that every required source and recipe is present and that
  no source component is unrelated to the Linux bundles. Windows/browser
  downloads retain notices but do not need this Linux source archive.
- **HD artwork is a separate download.** The pack contains 1,418 direct generated PNG
  replacements and 301 disc-backed recipes (1,719 texture identities), generated
  technical layers and two authored weapon meshes: 142.7 MiB of referenced inputs.
  Native packages discover the pack automatically; browser preparation exports its
  recipe dependencies for prefetch. Mixed RGB, sampled colours, cutouts, snow
  coverage and interface blend masks are reconstructed from the user's disc in
  Rust. No Python interpreter or online generator is used by the game.
  All 301 recipes match the accepted artwork byte for byte. Actual disc-backed
  loading also matches the previous pack for both characters in training/all ten
  missions and all four menu languages. `packaging/check-artwork.py` checks the
  manifest/provenance metadata in CI and every input hash when packaging.
  [Artwork provenance](../artwork/README.md) explicitly records use of original
  references; reconstruction removes embedded source pixels, not the underlying
  original character/environment designs. The MIT grant covers original
  contributions and is not a legal clearance for Tenchu's intellectual property.
- **Platform limits:** local Linux gameplay and browser tests exist; Windows has
  Wine coverage and CI startup checks. Physical Windows/ARM GPU gameplay and a
  real Apple Silicon build/runtime still need testing. No macOS binary is offered.
  Windows downloads are unsigned; there is no installer or automatic updater.
- **External dependency:** FFmpeg is not bundled in portable downloads. Movies
  and video export need it installed (or `TENCHU_FFMPEG` set). Nix supplies it for
  source builds. Browser preparation also needs the native tool and user's disc.
- **Game scope:** the separate `TRIAL.EXE` mission editor is not ported; its menu
  entry explains this. The supported retail disc is SLPS-019.01 only. Unknown
  action/asset variants still produce diagnostics instead of silently guessing;
  campaign/replay validation does not prove every possible gameplay path.
- **Dependency review:** `cargo audit` against the current RustSec database found
  no known vulnerabilities. It reported unmaintained `paste 1.0.15` and
  `ttf-parser 0.25.1` transitively. Track upstream replacements; this does not
  justify an untested renderer/font upgrade immediately before release. This is
  a Rust dependency scan, not an audit of every bundled native library or driver.

The ordinary game remains independent of sibling research repositories; optional
retail/oracle tests need private fixtures and are explicitly ignored by normal CI.
Hosted checks must pass before the workflow publishes the versioned download.

Local preparation checks passed: 137 ordinary workspace tests, native/WASM
Clippy, formatting, all-system Nix evaluation, actionlint/shellcheck and ten
release-policy tests. Linux x86-64, Windows x86-64, WASM and dependency-source
archives built and passed checksum/build-identity checks. Linux starts and loads
its dynamic libraries with `/nix` hidden; the extracted bundle rendered Mission 1
on NVIDIA Vulkan. Windows `--version` passed under Wine/X11 (Wine's Wayland
initialisation failed independently of the game). The browser archive serves its
module with the proper WASM MIME type and isolation headers. These are local
packaging checks. The later v0.1.0 hosted workflows passed too; neither result
validates physical Windows/ARM/macOS GPU gameplay.

HD-distribution verification passed: 143 ordinary workspace tests, focused
archive safety tests on Linux and Windows/Wine, native/Windows/WASM strict Clippy,
formatting, all-system Nix evaluation and release-policy checks. Both platforms
load the archive with pixel-identical results for both characters in training,
all ten missions and all four menu languages. Release-mode native scene/menu
loads take about 60–200 ms with a warm filesystem cache on the test desktop;
this is asset-load work, not a per-frame cost.

Updated archives have verified checksums/build identity, complete notices and
no HD PNG/OBJ inputs: Linux x86-64 26.6 MiB, Windows x86-64 17.7 MiB, WASM 6.5 MiB.
The separate artwork download is 141.8 MiB. The v0.1.0 dependency-source archive
was 817.8 MiB; the follow-up on `master` reduces it to about 149.3 MiB by keeping
only copyleft libraries shipped with Linux. Shared upstream inputs are deduplicated,
and patches for both x86-64 and ARM are included. Rust compiler/Cargo sources,
permissive-library sources and unrelated Nixpkgs packages are omitted; all
required license notices remain. The source inventory matches the actual x86-64
bundle, and its launcher starts with `/nix` hidden. Formatting, strict native
Clippy, all-system Nix evaluation and 12 release-policy tests pass. The final
two-architecture inventory check also runs before hosted publication.
Linux library loading passes with `/nix` hidden. Windows/Wine automatically finds
the neighbouring archive and exports all 1,722 inputs with matching hashes.
The browser shows its menu before mission artwork finishes downloading, loads
all ten missions offline, and preserves paused state and image restoration
across artwork toggles, with no reported WebGL2/page errors. These checks do
not replace physical Windows/ARM GPU or Mac testing.
