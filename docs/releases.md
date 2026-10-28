# Releasing

The workflow builds Linux x86-64, Linux ARM64, Windows x86-64 and WASM. Releases
contain those four archives, a dependency-source archive and `SHA256SUMS`.
Every archive has `build-info.json` identifying its version, commit and Nixpkgs
revision; native `tenchu-pc --version` also identifies its build.

## Development downloads

Pushes to `main`/`master` build and test all targets. Successful builds of the
repository's **default branch** update the `head` tag and **HEAD (development)**
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
3. Move the release's entries beneath `## 0.1.0 — YYYY-MM-DD` (use the actual
   version/date). Leave an empty `## Unreleased` above it for subsequent work.
   The initial Unreleased section describes the first public release, not a
   history of releases that never happened.
4. Run the checks below, perform a short disc-backed gameplay check, and commit.
   Push the commit, then create and push an annotated tag, for example:

   ```sh
   git tag -a v0.1.0 -m 'Tenchu PC 0.1.0'
   git push origin v0.1.0
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
python3 packaging/release.py notes refs/tags/v0.1.0 /tmp/tenchu-release-notes.md
```

Use the intended tag in the last command. No retail disc is required by hosted
CI. It checks native unit tests, Clippy, formatting, WASM builds, workflow/scripts,
Linux startup/library loading without `/nix`, and Windows executable startup.
It cannot validate gameplay/GPU drivers without a disc and suitable hardware.

## Publication review (2026-10-28)

- **MIT license for original project code/resources.** [COPYING.md](../COPYING.md)
  distinguishes the project's license from third-party notices. The
  reverse-engineering provenance in [SOURCES.md](../SOURCES.md) does
  not grant rights to the original game or claim clean-room development.
- **No remote or version tags configured yet.** The hosted workflow and actual
  public downloads need verification after the repository's first push. This
  checkout uses `main`, despite older instructions referring to `master`.
- **Keep private:** disc images, extracted browser `data/`, BIOS, RAM/trace
  captures, recordings/reference screenshots and `private/` artwork packs.
  The entire reachable Git history was checked: its only binary file paths are
  the two documented OFL fonts. No obvious credentials or private output paths
  were found. A filename/content scan is not a legal certification of the code.
- **Publish with the binaries:** changelog, concise run instructions, checksums,
  build identity, font/Rust/native-library notices and corresponding dependency
  sources. The Linux bundle contains LGPL libraries; preserve the source archive
  alongside redistribution. It includes upstream sources, applied patches,
  complete pinned Nixpkgs recipes and this port's packaging code.
- **Artwork:** the private HD campaign pack is not installed by the replacement
  artwork checkbox and is not in the downloads. Its images/provenance need a
  separate distribution review before offering it publicly.
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
Prefer a first **release candidate** while the hosted/platform checks are exercised.

Local preparation checks passed: 137 ordinary workspace tests, native/WASM
Clippy, formatting, all-system Nix evaluation, actionlint/shellcheck and ten
release-policy tests. Linux x86-64, Windows x86-64, WASM and dependency-source
archives built and passed checksum/build-identity checks. Linux starts and loads
its dynamic libraries with `/nix` hidden; the extracted bundle rendered Mission 1
on NVIDIA Vulkan. Windows `--version` passed under Wine/X11 (Wine's Wayland
initialisation failed independently of the game). The browser archive serves its
module with the proper WASM MIME type and isolation headers. These are local
packaging checks, not evidence that the GitHub workflow has run or that physical
Windows/ARM/macOS GPU gameplay is validated.
