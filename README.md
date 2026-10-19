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
progress; gameplay is interactive, but the complete game is not implemented yet.

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
through its duration. `render` captures Mission 1 after 30 ordinary logic ticks, using the same camera,
textures, characters, effects and HUD as the window.

## Gameplay preview

```sh
./dev.sh cargo run --release -- play '/path/to/game.cue'
./dev.sh cargo run --release -- play '/path/to/game.cue' --character ayame --mission 1 --layout 1
```

Mission numbers follow campaign order (1–10); `--mission training` selects
training. Each mission has layouts 1–3; `--layout random` selects one at the
source construction boundary. `--difficulty easy|normal|hard` selects the source
AI difficulty; normal is the default. `--language english|french|italian|japanese`
selects the original cinematic scripts. Japanese subtitle rasterization still
needs a portable replacement for BIOS KROM.

WASD/arrows move and turn, Space jumps, Shift crouches, J attacks/draws, E uses
an item, Q looks, and Z/C select items. P pauses; F5 or Escape abandons the
attempt and opens retry equipment. Escape from the launcher quits.
Losing focus pauses simulation. The preview currently supports movement,
jumping, crouching, wall hugging/sliding, ledge control, swimming, melee combos, guard/damage reactions, stealth
kills, dropped inventory, pickups, enemy arrows/gunshots/shuriken, item damage
and the source follow/look camera. Grappling, shuriken, medicine, caltrops,
smoke bombs, grenades, mines, poisoned food, sleeping gas and fire breath work through normal item selection. Damage floors and Manji flame attacks share the original napalm behavior. Mission 1 now runs its authored encounter scripts, camera cuts and subtitles,
then returns to gameplay. Results, stock rewards, retry and progression now run
through a simple launcher. Remaining items and special attacks
are unfinished; unsupported actions pause with a diagnostic. Caltrops,
grenades, smoke bombs and poisoned food also use the original corner-throw behavior.
Held and sheathed weapons use their original attachment rules. Original shadows,
weapon trails, blood and implemented particle effects are displayed with their
texture palettes and transparency modes. The HUD shows inventory selection and
counts, player/enemy health and the proximity indicator using original graphics.
The launcher uses the source initial stock: choose equipment with Up/Down and
Left/Right, then Enter to start. The hook is always carried. Tap E for a quick
shuriken throw or hold it to aim,
then release to throw; J, Space or Shift cancels the held item. With the hook
selected, hold E, aim with the direction keys and release to grapple. Doors, pitfalls, animated water, fire emitters, snow and ambient sound now run
from the original construction data. Scene faces now use back-face culling and their original transparency flags and
blend modes, sorted together with particles. Lighting, fog and the map remain in progress.

Original sound effects, character voices, music and localized cinematic dialogue
play from the disc. Music/dialogue require a raw Mode 2 BIN/CUE; cooked 2048-byte
ISO images omit the XA payload. The mixer uses the original banks, pitch,
envelopes and spatial sound rules with modern resampling. It does not emulate
SPU reverb or CD seek timing. Pausing or losing focus freezes audio and gameplay.
An unavailable audio device is reported without preventing play. Linux uses
PulseAudio/PipeWire's PulseAudio service when available, with ALSA as fallback.
Set `TENCHU_AUDIO_TRACE=1` to log music/dialogue requests and decoded clips.

The launcher lets you choose character, difficulty, mission and layout. Results
show the original score/rank, award items and update each character's records.
Enter advances (or retries training), R replays, and Escape returns to the launcher.
Failed or abandoned attempts lose their carried items. Training keeps its separate
ranking table and fixed equipment. Armour uses its original model and damage rule.

Progress saves between attempts to
`$XDG_DATA_HOME/tenchu-shinobi-gaisen-pc/progress.sav`, or
`~/.local/share/tenchu-shinobi-gaisen-pc/progress.sav` on Linux. macOS uses
`~/Library/Application Support`; Windows uses `%APPDATA%`. Set `TENCHU_SAVE` to
choose a different file. Saves retain both characters' stock/records, mission,
layout, language and difficulty. They are checked and replaced atomically;
Continue resumes at equipment selection, not mid-mission. Closing during an
attempt retains the preceding checkpoint. Existing PS1 memory cards are not
imported. Initial gameplay uses seed 1, then subsequent attempts continue from
the preceding gameplay/reward RNG. Menu waiting and display refresh consume no
RNG; PS1 startup VSync seeding is intentionally a host policy, not a hardware model.

`replay DISC INPUT.txt [OPTIONS]` runs without graphics and writes a CSV trace.
It keeps the diagnostic fallback loadout (hook, 10 shuriken, 5 caltrops, 2 medicine).
Each input line contains a decimal tick count and hexadecimal PS1 button mask;
`#` starts a comment. For example, `30 1000` holds forward for one second and
`1 1040` presses jump while holding forward. The replay and window use the same
scalar gameplay code.

The private-disc cadence test exercises both characters and compares player,
animation, effects, items, RNG, camera, collision, coordinate caches and activation at every tick
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
NPC activation now runs at the source cadence. Remaining effects, special ranged
attacks and remaining items remain unfinished. Activation workload counts use original geometry,
independent of PC rendering; remaining items and special attacks still need
to add their original contributions before this has full-game fidelity. Empty
cable curves omit the hook mesh: the original reads undefined stack coordinates
in that case, which this port does not reproduce.
An additional private check compares the roster, mission totals, weapon hitbox
indices and layout patrol points with the existing Reclaim the Castle capture:

```sh
TENCHU_DISC='/path/to/game.cue' TENCHU_ROSTER_TRACE='/path/to/kagi-aim-v1.trace' \
  ./dev.sh cargo test -p tenchu-pc roster_and_weapon -- --ignored
```

Optional core checks compare camera smoothing, enemy sight and combat decisions
with matching C, including 4,096 activation worlds, and compare the integer view
matrix and construction geometry with an existing retail trace. The
combat check covers 81,920 engaged-controller cases, including RNG and command
history. A further 8,192 human/item cases compare the original damage routine and collision
cleanup, with supplied terrain/motion/effect boundaries. Six controlled stealth
pairs exercise both characters through death and return to control. These checks
do not yet validate the entire AI/terrain/update pipeline. These private files remain outside
the repository and are never build or runtime dependencies:

```sh
TENCHU_RETAIL_EXE='/path/to/main.exe' TENCHU_DECOMP='/path/to/tenchu-decomp' \
TENCHU_CAMERA_TRACE='/path/to/kagi-aim-v1.trace' \
TENCHU_SCENE_TRACE='/path/to/kagi-aim-v1.trace' \
TENCHU_GTE_ORACLE='/path/to/standalone-gte-oracle' \
TENCHU_SDK_ROTATION_ORACLE='/path/to/tenchu-sim/native/sdk_rotation_oracle.py' \
TENCHU_DISC='/path/to/game.cue' \
  ./dev.sh cargo test -p tenchu-core -- --ignored
```

The optional projection check uses the standalone arithmetic oracle described in
`~/programming/tenchu-decomp-glitch-research/docs/tas-native-packets.md`. That
external test executable is not linked into this port or required to build it.

Projectile checks cover 2,048 complete flight curves against original C and
2,048 targeting rotations against the retail SDK. Controlled ordinary-tick
scenarios check bow, pistol, rifle and ninja attacks through launch, damage,
attachment and disposal. The SDK test invokes the research Python oracle above;
it is optional and has no role in the runtime.
