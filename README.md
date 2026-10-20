# Tenchu: Shinobi Gaisen PC

A Rust reimplementation of the gameplay in the PlayStation release
**SLPS-019.01**, using assets from the user's game disc.

The target is faithful mechanics, including gameplay glitches, with a modern
renderer and portable runtime. Gameplay advances at the original nominal rate;
display refresh and graphics settings do not determine game speed. Reproducing
PS1 hardware slowdown is outside the target.

All ten campaign missions and training are playable with both characters,
including gameplay cinematics, progression and save/retry. A simple launcher
replaces the original main menu. The final mission returns to a completion
screen; the movie from the separate `ENDING.EXE` is not implemented.

See [PLAN.md](PLAN.md) for deliverables and acceptance criteria and
[AGENTS.md](AGENTS.md) for development conventions.

Game data, captured RAM and disc images must remain outside Git. No BIOS is
required. This is an early PC port with full campaign and training regression
routes, native window checks and source/retail comparisons described below.
Those checks do not establish perfect parity for every possible playthrough.
A browser host, asset replacement packs and HD content are later work; the
shared assets, audio and gameplay crates already compile for WASM.

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

## Play

```sh
./dev.sh cargo run --release -- play '/path/to/game.cue'
./dev.sh cargo run --release -- play '/path/to/game.cue' --character ayame --mission 1 --layout 1
```

Mission numbers follow campaign order (1–10); `--mission training` selects
training. Each mission has layouts 1–3; `--layout random` selects one at the
source construction boundary. `--difficulty easy|normal|hard` selects the source
AI difficulty; normal is the default. The launcher's **Language** row selects
English, French, Italian or Japanese dialogue and subtitles. Use Left/Right to
change it before starting a mission or continuing a save. The choice is saved;
`--language english|french|italian|japanese` overrides the saved preference at launch.
The placeholder menu labels remain in English. Japanese subtitles use a bundled
[replacement font](crates/tenchu-assets/data/japanese-font/README.md); no BIOS is
needed. Latin subtitles retain the original disc font.

| Action | Keyboard | Controller |
| --- | --- | --- |
| Move and turn | WASD / arrows | D-pad / left stick |
| Jump | Space | South (Cross / A) |
| Attack / draw weapon | J | West (Square / X) |
| Crouch | Shift | East (Circle / B), or R1 / RB |
| Use item | E | North (Triangle / Y) |
| Look / aim | Hold Q + directions | Hold L1 / LB + directions |
| Previous / next item | Z / C | L2 / R2 (LT / RT) |
| Map | Hold M / Tab | Hold Select / Back |
| Pause / resume | P | Start / Menu |

In menus, use the arrows, D-pad or stick to choose and change values; Enter or
the south face button confirms, Escape or east goes back, and R or west retries
from results. In the equipment screen, Up/Down selects an item and Left/Right
removes/adds one. Enter/south starts the mission; the hook is carried automatically.
In gameplay, F5 or Escape abandons the attempt and opens retry equipment. On a
controller, pause first, then use east/west to abandon/retry. Escape/east from
the launcher quits. Controller hints use physical face-button positions because
labels differ between controller brands.

With your weapon drawn, back away (S / Down) to guard frontal melee attacks.
Quick keyboard and controller taps are retained until the next gameplay tick.

The first controller used is selected until disconnected. Keyboard and controller
can be used together. Disconnecting the active controller pauses gameplay.
Held controls must be released after menu or focus changes. Losing focus freezes
simulation and audio. Controller mappings use [gilrs](https://docs.rs/gilrs/0.11.2/gilrs/)
and accept its `SDL_GAMECONTROLLERCONFIG` environment override.

The preview currently supports movement,
jumping, crouching, wall hugging/sliding, ledge control, swimming, melee combos, guard/damage reactions, stealth
kills, dropped inventory, pickups, enemy arrows/gunshots/shuriken, item damage
and the source follow/look camera. Grappling, shuriken, medicine, caltrops,
smoke bombs, grenades, mines, poisoned food, colored rice, sleeping gas, fire breath and the lure flute work through normal item selection.
Planted rice remains marked on the map and can be picked up again. The eight-shot
shuriken item uses its original spread, delayed hitboxes and independent trails.
Speed scrolls retain the original cast, steering, terrain checks and cancellation.
Protection charms apply their original melee damage modifiers for 450 logic ticks.
Decoy dolls grow, draw nearby enemies and break under attack; only three can be active.
Disguise changes the original body meshes and motion scale, with source alert,
expiry and ledge-grab restrictions.
Ninja dogs summon, select and attack enemies, then expire after 1,800 logic ticks.
Mei-Oh's four lightning attacks retain repeated targeting, damage and original bolt jitter.
Resurrection leaves restore health after a lethal hit. Damage floors and Manji flame attacks share the original napalm behavior. Mission 1 now runs its authored encounter scripts, camera cuts and subtitles,
then returns to gameplay. Results, stock rewards, retry and progression now run
through a simple launcher. Unexpected or unsupported data pauses with a diagnostic. Caltrops,
grenades, smoke bombs and poisoned food also use the original corner-throw behavior.
Held and sheathed weapons use their original attachment rules. Original shadows,
weapon trails, blood and implemented particle effects are displayed with their
texture palettes and transparency modes. The HUD shows inventory selection and
counts, player/enemy health and the proximity indicator using original graphics.
The equipment screen shows both limits: 15 selected items and five item types.
The automatic hook uses one type, leaving four for selected equipment.
The launcher uses the source initial stock: choose equipment with Up/Down and
Left/Right, then Enter to start. The hook is always carried. Tap E for a quick
shuriken throw or hold it to aim,
then release to throw; J, Space or Shift cancels the held item. With the hook
selected, hold E, aim with the direction keys and release to grapple. Doors, pitfalls, animated water, fire emitters, snow and ambient sound now run
from the original construction data. Scene faces now use back-face culling and their original transparency flags and
blend modes, sorted together with particles. Meshes retain their baked vertex
colors and fade into black using the original camera-depth fog curve. The GPU
evaluates that curve smoothly per pixel; original per-polygon cue quantization
and distance/sector visibility are not yet reproduced. HUD, sprites and shadow
meshes retain their separate drawing rules.

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
Training completion is checked for both
[Rikimaru](crates/tenchu-pc/tests/data/training-rikimaru.inputs) and
[Ayame](crates/tenchu-pc/tests/data/training-ayame.inputs), including all seven
opponents, the instructor's closing sequence, saved ranking and retry with
the fixed hook and five shuriken. These routes finish with 70 and 90 HP.

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

Add `--record DIRECTORY` when launching to record normal play:

```sh
./dev.sh cargo run --release -- play '/path/to/game.cue' --record ./recordings
./dev.sh cargo run --release -- replay '/path/to/game.cue' ./recordings/mission1-EXAMPLE.inputs
```

The terminal prints a unique filename for each mission attempt. Recordings store
launch settings, starting RNG, equipment and inputs for each logic tick. Pause
and menu time add no inputs; retry and subsequent missions start new files.
Files contain no game assets or captured RAM. Replay supplies the recorded
settings automatically, including random-layout selection and armour; extra
launch options are rejected. Use the same port revision for deterministic
comparisons.

`replay DISC INPUT.txt [OPTIONS]` runs without a window and writes a CSV trace,
including health, game clock, cinematic activity and mission outcome. Add
`--capture OUTPUT.png` to render its final state through the ordinary renderer;
renderer messages go to stderr so stdout remains CSV.
Older input-only tapes use the diagnostic fallback seed 1 and loadout
(hook, 10 shuriken, 5 caltrops, 2 medicine), with launch options from the command line.
Each input line contains a decimal tick count and hexadecimal PS1 button mask;
`#` starts a comment. For example, `30 1000` holds forward for one second and
`1 1040` presses jump while holding forward. The replay and window use the same
scalar gameplay code.

Current saved-campaign regressions complete Missions 1–8 for both characters
after the swimming-camera and drawing-initialization
corrections. Stock and RNG carry through real encoded/decoded saves at every
boundary. Refreshed recordings include their starting seed and equipment, so
`replay` can run them directly.

| Character | Mission | Results tick | Health |
| --- | --- | ---: | ---: |
| Rikimaru | [1](crates/tenchu-pc/tests/data/mission1-rikimaru.inputs) | 5,667 | 64 |
| Ayame | [1](crates/tenchu-pc/tests/data/mission1-ayame.inputs) | 4,834 | 94 |
| Rikimaru | [3](crates/tenchu-pc/tests/data/mission3-rikimaru-campaign.inputs) | 5,952 | 7 |
| Ayame | [3](crates/tenchu-pc/tests/data/mission3-ayame-campaign.inputs) | 5,883 | 46 |
| Rikimaru | [4](crates/tenchu-pc/tests/data/mission4-rikimaru-campaign.inputs) | 9,128 | 57 |
| Ayame | [4](crates/tenchu-pc/tests/data/mission4-ayame-campaign.inputs) | 12,062 | 24 |
| Rikimaru | [5](crates/tenchu-pc/tests/data/mission5-rikimaru-campaign.inputs) | 8,034 | 100 |
| Ayame | [5](crates/tenchu-pc/tests/data/mission5-ayame-campaign.inputs) | 7,919 | 46 |
| Rikimaru | [6](crates/tenchu-pc/tests/data/mission6-rikimaru-campaign.inputs) | 6,139 | 50 |
| Ayame | [6](crates/tenchu-pc/tests/data/mission6-ayame-campaign.inputs) | 6,393 | 5 |
| Rikimaru | [7](crates/tenchu-pc/tests/data/mission7-rikimaru-campaign.inputs) | 3,744 | 100 |
| Ayame | [7](crates/tenchu-pc/tests/data/mission7-ayame-campaign.inputs) | 2,546 | 100 |
| Rikimaru | [8](crates/tenchu-pc/tests/data/mission8-rikimaru-campaign.inputs) | 8,448 | 74 |
| Ayame | [8](crates/tenchu-pc/tests/data/mission8-ayame-campaign.inputs) | 8,126 | 56 |

Both complete Mission 2 with 100 HP. The complete campaign tests currently fail
on the old Mission 9 continuations. The independent [Mission 7 route](crates/tenchu-pc/tests/data/mission7-rikimaru.inputs)
now also passes with the corrected swimming camera, finishing at tick 3,461 with
93 HP. Both ten-mission campaigns and training passed before the corrections; those
historical results and the implemented feature coverage are recorded in
[PLAN.md](PLAN.md). They are not a current passing campaign gate.

```sh
TENCHU_DISC='/path/to/game.cue' \
  ./dev.sh cargo test --release -p tenchu-pc controller_route -- --ignored
```

Native window checks cover movement, combat/items, pause/resize, language
selection and death/retry. Arbitrary inputs and every difficulty/layout
combination have not been exhaustively compared with retail.

The private-disc cadence test exercises both characters and compares player,
animation, effects, items, RNG, camera, collision, coordinate caches and activation at every tick
under 24 Hz and 144 Hz host frames:

```sh
TENCHU_DISC='/path/to/game.cue' \
  ./dev.sh cargo test -p tenchu-pc retail_movement -- --ignored
TENCHU_DISC='/path/to/game.cue' \
  ./dev.sh cargo test -p tenchu-pc all_retail_layouts -- --ignored
```

This checks display independence. A separate private comparison checks seven
original Castle-start recordings for 4,630 consecutive updates, including
swimming, grappling, climbing and enemy attacks that reduce health to 73:

```sh
TENCHU_DISC='/path/to/game.cue' \
TENCHU_CASTLE_CORPUS='/path/to/tenchu-castle-approach-validation-20260925' \
  ./dev.sh cargo test --release -p tenchu-core castle_start_matches_retail -- --ignored
```

It constructs the world from disc assets, supplies the recorded launch seed and
loadout, and aligns the initial player pose and ambient-sound deadlines once.
The original loading sequence left those deadlines at clock 644; the PC launcher
starts them at zero. All subsequent actor, camera, RNG, collision and drawing state advances
in the port without corrections from the recordings. This is finite coverage,
not a claim of full-campaign parity.

Terrain collision probes retain the original diagonal sample positions and mask
order. A private comparison runs the original C probe with the executable's
direction table and recorded terrain responses; an ordinary-input regression
checks wall pressure and retreat for both characters.

Every mission/layout constructs the source actor roster, collision slots,
weapons and patrol data. The ordered actor loop and AI controllers are implemented;
NPC activation runs at the source cadence. Activation workload counts use
original geometry, independent of PC rendering. The seven continuous recordings
check exact drawing counts as well as gameplay. Empty
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
