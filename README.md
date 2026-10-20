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
through a simple launcher. Broader campaign and presentation coverage remain
unfinished; unsupported actions pause with a diagnostic. Caltrops,
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

The checked-in [Mission 1 route](crates/tenchu-pc/tests/data/mission1-rikimaru.inputs)
completes both boss fights and their cinematics as Rikimaru, normal difficulty,
layout 1. Its private-disc test chooses equipment from new-game stock, reaches
results with 75 HP, saves, reloads, completes the
[Mission 2 delivery](crates/tenchu-pc/tests/data/mission2-rikimaru.inputs) with
100 HP, saves again, completes the
[Mission 3 rescue](crates/tenchu-pc/tests/data/mission3-rikimaru-campaign.inputs)
with 68 HP, then completes the
[Mission 4 checkpoint](crates/tenchu-pc/tests/data/mission4-rikimaru-campaign.inputs)
with 16 HP, then the
[Mission 5 punishment](crates/tenchu-pc/tests/data/mission5-rikimaru-campaign.inputs)
with 72 HP, then the
[Mission 6 cult encounters](crates/tenchu-pc/tests/data/mission6-rikimaru-campaign.inputs)
with 12 HP, then the
[Mission 7 pirate ship](crates/tenchu-pc/tests/data/mission7-rikimaru-campaign.inputs)
with 100 HP, then the
[Mission 8 mountain](crates/tenchu-pc/tests/data/mission8-rikimaru-campaign.inputs)
with 50 HP, then the
[Mission 9 castle](crates/tenchu-pc/tests/data/mission9-rikimaru-campaign.inputs)
with 22 HP, then the
[final mission](crates/tenchu-pc/tests/data/mission10-rikimaru-campaign.inputs)
with 30 HP. Mission 4 uses the saved stock of two
shuriken, four grenades and two of the three carried medicine; Mission 5 carries
only the hook. Mission 6 uses one shuriken, two grenades and two smoke bombs.
Mission 7 swims past the armed quay and uses one medicine, one grenade and one
smoke bomb against Balma. Mission 8 uses one shuriken, one medicine, one grenade
and three smoke bombs, then climbs past the upper guards to the cure.
Mission 9 uses both medicine and four mines alongside melee, retaining the
original blast self-damage. Mission 10 uses its remaining medicine and smoke
bomb, crosses the lava and uses one mine alongside melee against Mei-Oh.
Inventory and RNG carry through all ten missions, with a save/reload at every
boundary and final completion flags checked. A separate
[Ayame route](crates/tenchu-pc/tests/data/mission2-ayame.inputs) completes Mission 2
with 100 HP. The delivery routes use ordinary movement and swimming, carrying
only the automatically supplied hook in the campaign tests.

The saved Ayame campaign now completes
[Mission 1](crates/tenchu-pc/tests/data/mission1-ayame.inputs) with 70 HP,
using grappling and melee for both encounters without consuming equipment.
It saves, reloads and completes
[Mission 2](crates/tenchu-pc/tests/data/mission2-ayame-campaign.inputs) with
100 HP, then saves into Mission 3. Its carried RNG produces different enemy
behavior from the standalone delivery route.

An independent [Rikimaru Mission 3 route](crates/tenchu-pc/tests/data/mission3-rikimaru.inputs)
crosses the mine, defeats Goo, runs the rescue cinematic and reaches results
with 28 HP. It selects six shuriken, six caltrops and two medicine from new-game
stock; four shuriken are used. Its test saves and reloads into Mission 4. This
exercises a different starting RNG and loadout from the continuous campaign,
which carries five shuriken, six caltrops and one medicine into Mission 3 and
uses all five shuriken.

An independent [Rikimaru Mission 4 route](crates/tenchu-pc/tests/data/mission4-rikimaru.inputs)
defeats both checkpoint bosses, runs all four cinematics and reaches results
with 84 HP. It selects six shuriken, six caltrops and two medicine from new-game
stock, uses the shuriken and medicine, then saves and reloads into Mission 5.

Mission 5 has separate [Rikimaru](crates/tenchu-pc/tests/data/mission5-rikimaru.inputs)
and [Ayame](crates/tenchu-pc/tests/data/mission5-ayame.inputs) routes. Both trigger
Hikone's retreat and follow their character's event branch through results and
a saved transition into Mission 6. Rikimaru carries only the hook and finishes
with 56 HP; Ayame defeats Kataoka using five shuriken and melee, uses one medicine
and finishes with 80 HP.

The [Rikimaru Mission 6 route](crates/tenchu-pc/tests/data/mission6-rikimaru.inputs)
grapples and climbs the statue, completes both encounters and reaches results
with 5 HP, then saves into Mission 7. It also exercises a nearby guard releasing
a ledge during combat. That exposed a port error: release probes now advance
using the original independent height counter, preserving each probe's position
adjustments without feeding them into the next counter value.

The [Rikimaru Mission 7 route](crates/tenchu-pc/tests/data/mission7-rikimaru.inputs)
boards the pirate ship, defeats Balma, runs all three cinematics and reaches
results with 100 HP. It uses six shuriken and one medicine, picks up a dropped
smoke bomb along the route, then saves and starts Mission 8.

The [Rikimaru Mission 8 route](crates/tenchu-pc/tests/data/mission8-rikimaru.inputs)
traverses the mountain with repeated grapples, jumps and ledge climbs, fights
its guards and collects the cure. It uses six shuriken and two medicine, reaches
results with 100 HP, then saves and starts Mission 9.

An independent [Rikimaru Mission 9 route](crates/tenchu-pc/tests/data/mission9-rikimaru.inputs)
climbs the castle, defeats Onikage and completes both cinematics with 53 HP.
Its private-disc test uses ordinary equipment selection, saves the results,
reloads and starts Mission 10. This exercises a different starting stock and RNG
from the continuous saved campaign.

The [Rikimaru Mission 10 route](crates/tenchu-pc/tests/data/mission10-rikimaru.inputs)
defeats Onikage, crosses the inner caverns and lava, descends the pitfall and
defeats Mei-Oh. All five cinematics run; results arrive with 10 HP after using
six shuriken and both medicine. Its private-disc test verifies the final record,
saved completion flags, completion screen and saved Mission 10 replay selection.
Both the independent routes and the accumulated-stock campaign now complete
Rikimaru's ten missions. The external
ending movie executable is still represented by a completion screen.

```sh
TENCHU_DISC='/path/to/game.cue' \
  ./dev.sh cargo test -p tenchu-pc controller_route -- --ignored
```

These are regressions for specific authored routes, not a complete-game or retail
parity claim. Both characters' keyboard crouch and return to standing have also
been checked in the native window.

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

Terrain collision probes retain the original diagonal sample positions and mask
order. A private comparison runs the original C probe with the executable's
direction table and recorded terrain responses; an ordinary-input regression
checks wall pressure and retreat for both characters.

Every mission/layout constructs the source actor roster, collision slots,
weapons and patrol data. The ordered actor loop and AI controllers are implemented;
NPC activation now runs at the source cadence. Remaining effects and full-campaign
coverage are unfinished. Activation workload counts use original geometry,
independent of PC rendering; complete drawing coverage still needs verification
before this has full-game fidelity. Empty
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
