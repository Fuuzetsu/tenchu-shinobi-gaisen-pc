# Tenchu: Shinobi Gaisen PC

A Rust reimplementation of the gameplay in the PlayStation release
**SLPS-019.01**, using assets from the user's game disc.

The target is faithful mechanics, including gameplay glitches, with a modern
renderer and portable runtime. Gameplay advances at the original nominal rate;
display refresh and graphics settings do not determine game speed. Reproducing
PS1 hardware slowdown is outside the target.

Development starts with Mission 1. The complete goal includes the functionality
of `MAIN.EXE`, all missions, both characters, cinematics, progression and
save/retry. A simple launcher replaces the original main menu.

See [PLAN.md](PLAN.md) for deliverables and acceptance criteria and
[AGENTS.md](AGENTS.md) for development conventions.

Game data, captured RAM and disc images must remain outside Git. No BIOS is
required by the intended native runtime. The initial implementation is in
progress; this repository is not yet a playable game.

## Build and inspect a disc

Nix supplies the pinned Rust toolchain and dependencies:

```sh
nix develop
cargo run -p tenchu-pc -- inspect '/path/to/game.cue'
cargo test --workspace
cargo fmt --all -- --check
```

`./dev.sh cargo ...` enters the same environment for one command. Use `nix build`
for the native package, `nix run -- inspect /path/to/game.cue` to run it, and
`nix flake check` for tests, formatting and a WASM build of the reusable crates.
ISO files and raw BIN data tracks also work. Disc data is read on demand; it is
never an input to the Nix package or copied into the Nix store.

```sh
./dev.sh cargo run --release -- audit '/path/to/game.cue'
./dev.sh cargo run --release -- render '/path/to/game.cue' /tmp/mission1.png
```

`audit` constructs every stage for both characters and samples each motion track
through its duration. `render` captures Mission 1 with original textures.

## Movement preview

```sh
./dev.sh cargo run --release -- play '/path/to/game.cue'
./dev.sh cargo run --release -- play '/path/to/game.cue' --character ayame --mission 1 --layout 1
```

Mission numbers follow campaign order (1–10); `--mission training` selects
training. Each mission has layouts 1–3. `--difficulty easy|normal|hard` selects
the source AI difficulty; normal is the default.

WASD/arrows move and turn, Space jumps, Shift crouches, J attacks/draws, E uses
an item, Q looks, and Z/C select items. P pauses, F5 restarts and Escape quits.
Losing focus pauses simulation. The preview currently supports movement,
jumping, crouching, ledge control and the source follow/look camera. Combat,
items and enemies are unfinished; unsupported actions pause with a diagnostic.
Held and sheathed weapons use their original attachment rules. Lighting,
transparency, effects and audio remain in progress.

`replay DISC INPUT.txt [OPTIONS]` runs without graphics and writes a CSV trace.
Each input line contains a decimal tick count and hexadecimal PS1 button mask;
`#` starts a comment. For example, `30 1000` holds forward for one second and
`1 1040` presses jump while holding forward. The replay and window use the same
scalar gameplay code.

The private-disc cadence test exercises both characters and compares player,
animation, effects, RNG, camera, collision and coordinate caches at every tick
under 24 Hz and 144 Hz host frames:

```sh
TENCHU_DISC='/path/to/game.cue' \
  ./dev.sh cargo test -p tenchu-pc retail_movement -- --ignored
TENCHU_DISC='/path/to/game.cue' \
  ./dev.sh cargo test -p tenchu-pc all_retail_layouts -- --ignored
```

This checks display independence, not parity with complete PS1 gameplay. Retail
continuous state comparison remains a separate acceptance requirement.

Every mission/layout constructs the source actor roster, collision slots,
weapons and patrol data. The ordered actor loop and AI controllers are implemented;
NPC activation, draw effects and combat still need to be joined before enemies
are enabled in ordinary play. Stage scripts remain unfinished.
An additional private check compares the roster, mission totals, weapon hitbox
indices and layout patrol points with the existing Reclaim the Castle capture:

```sh
TENCHU_DISC='/path/to/game.cue' TENCHU_ROSTER_TRACE='/path/to/kagi-aim-v1.trace' \
  ./dev.sh cargo test -p tenchu-pc roster_and_weapon -- --ignored
```

Optional core checks compare camera smoothing, enemy sight and combat decisions
with matching C, and the integer view matrix with an existing retail trace. The
combat check covers 81,920 engaged-controller cases, including RNG and command
history; it does not yet validate the entire AI/terrain/update pipeline. These private files remain outside
the repository and are never build or runtime dependencies:

```sh
TENCHU_RETAIL_EXE='/path/to/main.exe' TENCHU_DECOMP='/path/to/tenchu-decomp' \
TENCHU_CAMERA_TRACE='/path/to/kagi-aim-v1.trace' \
TENCHU_DISC='/path/to/game.cue' \
  ./dev.sh cargo test -p tenchu-core -- --ignored
```
