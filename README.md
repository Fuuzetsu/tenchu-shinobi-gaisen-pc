# Tenchu: Shinobi Gaisen PC

A portable Rust port of **Tenchu: Shinobi Gaisen (SLPS-019.01)**: all ten missions,
training, original menus and endings. Faithful game logic at its original rate,
with optional modern graphics and smooth rendering. No PlayStation BIOS needed.

**Bring your own original game disc:** select its **`.cue`** and keep all referenced
**`.bin` files** beside it. Original game data is not included. Other Tenchu releases are not
supported. Startup checks the game/menu executables and base artwork with SHA-256.
The separate original mission editor (`TRIAL.EXE`) is not ported.

## Download and play

In this repository on GitHub, open **Releases**. Choose a versioned release for
normal play, or **HEAD (development)** for the latest successful `master`
build. Download your platform's archive under **Assets** and extract it completely.
The [workflow](.github/workflows/build.yml) publishes version tags and refreshes
HEAD on `master`; release downloads do not have the Actions artifact expiry.
[What's changed](CHANGELOG.md).

| Platform | Download | Run |
| --- | --- | --- |
| Windows x64 | `tenchu-windows-x86_64.zip` | Double-click `tenchu-pc.exe` and choose your CUE. |
| Linux Intel/AMD | `tenchu-linux-x86_64.tar.gz` | Run `./tenchu-pc`; keep the whole extracted directory together. |
| Linux ARM64 | `tenchu-linux-aarch64.tar.gz` | Same as Linux above. |
| Browser | `tenchu-wasm32.tar.gz` | Prepare your disc assets and serve the folder; see below. |
| Apple Silicon Mac | Build from source below | A real Mac runtime test is still needed. |

Or launch directly: `tenchu-pc play "/path/to/game.cue"` (Windows:
`.\tenchu-pc.exe play "C:\Games\Tenchu\game.cue"`). Linux downloads include their
shared libraries; Nix is not needed to run them. Use your OS's GPU drivers.
Install **FFmpeg** for opening/ending movies, or place `ffmpeg.exe` beside the
Windows executable. [Platform details](docs/guide.md#build-and-inspect-a-disc).

**F1** opens settings. **General → Benchmarks… → Auto-optimise graphics** tests your
hardware and saves **Auto-optimised** for your chosen FPS target. Low/Medium/High/
Ultra are available under **Presets / save as…**, along with save/rename for your
own presets. New installs use Modern Low; existing preferences are kept.
**F8** toggles original graphics; **F6** opens the quick launcher from the menu.
[Controls](docs/guide.md#play) · [Benchmarks and measured improvements](docs/performance.md)

## Build from source

[Nix](https://nixos.org/download/) with flakes enabled supplies the pinned Rust toolchain and dependencies:

```sh
nix build
./result/bin/tenchu-pc play '/path/to/game.cue'
```

For development: `./dev.sh cargo run --release -- play '/path/to/game.cue'`.
Linux can cross-build Windows with `nix build .#windows`.
[Build and platform checks](docs/guide.md#build-and-inspect-a-disc).

## Browser

Extract the browser download into `web/`, then use a native build to prepare assets:

```sh
tenchu-pc prepare-web '/path/to/game.cue' web/data
python3 web/serve-web.py web
```

Open **http://127.0.0.1:8000**. WebGPU is preferred; WebGL2 is available in the
renderer selector. Assets load from your server and cache in the browser.
From source, `./build-web.sh '/path/to/game.cue'` builds and prepares the site.
[Browser details](docs/guide.md#browser-build).

The HD artwork pack is included and found automatically. **F1 → Visual assets →
Replacement artwork** enables it; Modern presets enable it too. Some textures
are reconstructed locally from your disc when needed. No Python or online image
service is required to play. Set `TENCHU_ASSET_PACK` to use a different pack, or
to an empty string to skip artwork loading. [Artwork/provenance](assets/hd/README.md).
[Full feature guide](docs/guide.md) · [Demos](DEMOS.md) · [Development plan](PLAN.md)

This is an unofficial port. Original project code/resources use the [MIT License](LICENSE),
without warranty. See [third-party attribution](COPYING.md) and
[release status / maintainer instructions](docs/releases.md). The dependency-source
archive is for redistribution/rebuilding; it is not needed to play.
