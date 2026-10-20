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
A browser host and authored HD content are later work; editable texture/model
packs are supported. The shared assets, audio and gameplay crates compile for WASM.

The visual-upgrade work is tracked in [CONFIGURATION.md](CONFIGURATION.md).
For the optional upgraded look, open **F1 → Graphics → Visual preset**:

- **Original** restores the original presentation settings and is the factory default.
- **Enhanced** combines 4× MSAA, 8× anisotropic filtering, subtle character
  contours, smooth model lighting, lantern/sky shadows, contact shading,
  restrained bloom and blue distance haze.
- **Cinematic** increases anisotropy and the point-light/shadow budget, with
  stronger bloom/contact shading, haze and a slight vignette. It costs more GPU time.

Each effect remains adjustable. Changes show the preset as **Custom**; R on the
preset row restores Original. Presets preserve the replacement-artwork switch
and never change gameplay or simulation settings. Source textures still limit
close-up detail; the editable pack workflow below supplies genuinely new art.

An asset export is available for reviewing and editing the original art:

```sh
./dev.sh cargo run --release -- export-assets '/path/to/game.cue' /tmp/tenchu-art --mission 1
```

Use an empty output directory. `catalog.json` records original prop/model names;
`textures/` contains decoded PNGs with content-based identities, and `models/`
contains textured OBJ geometry in original local units (negative Y is up).
Actor parts are separate files corresponding to the existing animation rig.
PNG alpha 0 means a hole, 128 preserves the original semi-transparency flag,
and 255 is opaque. The original face still decides whether that flag blends.
These are local exports of your disc, not assets bundled with the port.
Edit the PNGs with your preferred image editor, then launch with that folder:

```sh
TENCHU_ASSET_PACK=/tmp/tenchu-art ./dev.sh cargo run --release -- play '/path/to/game.cue'
```

F1 → **Visual assets → Reload asset pack** applies edits without restarting.
Unchanged exported textures are skipped, preserving the live original fallback.
The **Replacement artwork** switch compares against the originals. A failed
reload keeps the previous pack. `pack.json` maps content identities to relative
PNG paths; it can contain only the replacements you want to distribute.
Textures may be RGB/RGBA, grayscale or indexed PNG, up to 1024 pixels per side,
with a pack budget of 16 million decoded pixels. Alpha 0 is transparent,
1–191 selects semi-transparency and 192–255 is opaque; these are game blend
classes rather than continuous opacity. Higher resolutions retain the original
UV mapping. New cutouts also affect model shadows. HUD/menu textures stay original.
`render` and replay captures also accept `TENCHU_ASSET_PACK`.

**Texture reconstruction** optionally applies
[Scale2x/Scale4x](https://www.scale2x.it/algorithm) to original world textures.
It smooths pixel stair-steps without inventing detail, and can change cutout edges.
It suits some patterns better than photographic textures; compare it in game.
Authored replacements take precedence and are not automatically enlarged.
To replace a model part, edit its OBJ and add its catalogue identity and file
path to the `models` object in `pack.json`, for example:

```json
"models": { "actors/0000/2": "models/actors/0000/2.obj" }
```

Reload from F1. Export triangulated faces, preserve local coordinates and the
part's origin, and retain its material names. The loader accepts OBJ positions,
UVs, normals and optional vertex RGB colors; it uses `pack.json` material
definitions rather than loading MTL files. The exported MTL is for editor
previews. Material `blend` values are 0 opaque, 1 half blend, 2 additive,
3 subtractive and 4 quarter additive; `double_sided` controls back faces.
New materials can reference additional PNG identities declared in `textures`.
UVs address each individual image, with clamp-to-edge sampling.

Character parts and weapons follow the original animation transforms; scenery
uses its original placements. The same edited geometry casts shadows. Collision,
AI and simulation remain original. This is a rigid-part replacement workflow,
not a new skeletal-animation importer. Empty source parts cannot be replaced.

**Smooth model lighting** generates crease-aware normals for dynamic lighting.
**Character contours** optionally rounds original actor silhouettes, with
Subtle/Rounded settings. It uses
[curved PN triangles](https://www.cs.jhu.edu/~misha/ReadingSeminar/Papers/Vlachos00.pdf),
keeps original vertices and open boundaries fixed, and leaves scenery and
authored replacement geometry unrounded. These options start off; finer
geometry costs more to transform and shadow. They can soften the original
shapes but cannot invent new costume or facial detail.

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
through its duration. `render` captures after 30 ordinary logic ticks, using the
same camera, textures, characters, effects and HUD as the window. It accepts the
same mission/character/layout options as `play`; Mission 1 is the default.
Captures use Original settings unless `TENCHU_SETTINGS` explicitly names a
settings file. That also works with `replay --capture`, allowing repeatable
Original/Enhanced comparisons of the same simulation state.

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

**F1** opens settings from any screen. Use arrows/D-pad and Enter/South, or
click a row (right-click decreases), then F1/Escape/East to close. The settings
menu pauses the game without changing an existing manual pause. It provides FPS
and tick display, pause on defocus, borderless fullscreen, VSync, HUD visibility
and master volume. Factory defaults preserve the preceding experience.
Preferences save separately from progress in
`$XDG_CONFIG_HOME/tenchu-shinobi-gaisen-pc/settings.json` (normally
`~/.config/tenchu-shinobi-gaisen-pc/settings.json` on Linux). `TENCHU_SETTINGS`
overrides the file. Disabling pause on defocus keeps simulation/audio running
but still releases controls; input from another application never controls the game.
Tab/Shift+Tab, Page Down/Page Up or controller R1/L1 changes settings pages.
Hover a row or select it with the arrows, then press R to reset only that value;
one-shot actions such as teleport cannot be reset or triggered with R. Graphics
settings include black fog and its distance, textures, baked vertex shading,
exposure, nearest/bilinear/trilinear filtering, 2×–16× anisotropic filtering and
supported MSAA modes. MSAA smooths polygon edges. Mipmaps reduce distant texture
shimmer; anisotropic filtering retains detail on angled floors and roofs.
Each decoded texture has isolated mip levels and separate opaque/semitransparent
colors, so filtering does not borrow from neighboring textures or blend modes.
Animated VRAM textures remain live. HUD and menu textures remain sharp.

Inspection pages include live body/attack hitboxes, sight bounds, masked terrain
surfaces, patrol routes, spatial event regions, actor labels/state, X-ray overlays
and a recent player trail. These tools start disabled each launch. Sight bounds
show calm-state ranges and angles, not walls/occlusion or a guarantee of detection.
Terrain overlays show ACM floors/slopes rather than inferred wall meshes.

**F2** toggles free camera. Hold right mouse to look, WASD to fly, Q/E down/up,
and Shift for a speed boost. Left-click selects an actor. F1's Camera page adds
focus, overhead view, FOV, speed and a mission-local bookmark. Free camera
suppresses player input; **F3** pauses/resumes while keeping the scene visible,
and **F4** runs exactly one logic tick. The gameplay camera still runs internally.
Some effects already projected by that camera (wire lines, weapon trails and
sprites) are omitted from the inspection view.

The Teleport page accepts exact X/Y/Z values (Enter to edit, Delete to clear,
Enter to apply), floor snapping or a click on a terrain surface. Negative Y is
up. Teleporting and changing the enemy sight-distance multiplier are explicit
gameplay edits: they end an active recording before the edit, leaving a replayable
prefix. A new attempt can record again with original sight distance. Sight
distance does not change hearing, actor activation, attack range or occlusion.

Hold **Backspace** during a cutscene to fast-forward at maximum bounded throughput.
The Simulation page offers automatic 2×/4×/8×/16×/Max cutscene speed and general
25–400% or Unlimited simulation speed. Unlimited runs across gameplay and
cutscenes, muting audio and yielding periodically for input/rendering. Every
logic tick runs; cutscene-only accelerated backlog is discarded
when control returns. Fast-forward mutes sound and resumes background music at
ordinary speed; releasing it mid-dialogue resumes speech at the next cue.
**F9** saves the current view as PNG under `screenshots/`, or `TENCHU_SCREENSHOTS`.
**F10** toggles a clean photo view, hiding HUD, captions, FPS and inspection
helpers while preserving world effects. Combine it with F2/F3 for a paused
free-camera shot; F1 remains available. It does not alter saved HUD preferences.

The Lighting page adds optional dynamic point lights and model/weapon shadows.
In a mission, select **Preview model shadows** to enable a light above/beside the
player, lower ambient brightness and hide the original blob shadows. Close F1
and move or attack to inspect the animated silhouette. This changes presentation
preferences; General → Restore defaults restores the original look.
For manual setup, enable **Dynamic lighting**. Authored fires and detected lanterns
supply lights; a movable light can also be placed at the camera or above the
player. The movable light lasts for the session and can follow free camera or
move along X/Y/Z. Radius, intensity, ambient brightness, shadow filtering and
256–2048 shadow resolution are adjustable.
Lighting starts off; changing it never affects AI, visibility, RNG or logic timing.

This is a new visual treatment over the original baked colors.
**Point-light budget** allows 0/1/2/4/8/16 simultaneous sources (default 2).
Nearest sources fill the budget; the movable light takes priority.
**Shadowed point lights** independently limits shadow rendering to the nearest
0–4 lights. Unshadowed lights are cheaper but can shine through walls.
The menu shows active lights, shadow views and depth-map memory. Each shadowed
point light costs six extra scene passes and about 6 MiB at 512 or 96 MiB at 2048.
Maps are allocated only for active shadowed lights; disabling shadows releases
them. Filtering radius 0 retains bilinear depth comparisons; 1–8 adds nine-tap
filtering. This softens edges, rather than simulating a full area light.

**Lantern / prop lights** recognizes names such as AKARI, TORO and ANDON from
the disc, then samples bright textured surfaces to locate/color the emitter.
Placements and carried-weapon transforms supply world positions. Bright panels
glow and transmit light; their darker frames still cast shadows. This is a
reusable heuristic, not recovered original lighting data. **Light source
markers** labels active sources; **Inspect nearest lantern** focuses free camera.

Exported `pack.json` includes editable `lights` rules. An omitted list uses the
built-in rules; `[]` disables detection. The last matching rule wins. Match with
case-insensitive `name_contains`, or an exact catalogue `model` identity. Rules
accept `enabled`, `threshold` (0.05–1 brightness), `emission` (0–10),
`radius_scale` (0.1–4), optional RGB `color` and a local-space `offset`.
Defaults derive color and position from the original mesh/texture; offsets let
replacement authors adjust them. Reload through Visual assets. No mission
coordinates are embedded in the renderer.

Actual live actors, held/sheathed weapons, scenery, doors and trapdoors cast
shadows even outside the gameplay camera. Other rendered world item meshes also
cast. Original texture holes and translucent pixels remain transparent to
shadows, including mixed opaque/translucent textured faces. Particles do not cast.
**Original blob shadows** controls the original ground decals independently.
Lighting already baked into textures/vertex colors remains.

F1 → **Sky lighting** adds a directional sun/moon source and optional cool ambient
tint. Its strength starts at zero. Enable Dynamic lighting and raise Sun / moon
strength, then choose neutral/cool/warm color, compass angle and elevation.
Its single shadow map has independent 512–4096 resolution and a 10k–60k coverage
radius around the view. The projection moves in texel increments to reduce
shimmer, with a gradual fade at its boundary. Wider coverage reduces detail;
2048 is a useful starting point. The shared shadow switch and filtering radius
also apply to sky shadows. A point-light budget of zero leaves just sky/ambient
lighting and glowing panels.

F1 → **Image effects** enables HDR finishing with bloom, contact shading,
filmic tone mapping, contrast, saturation, warm/cool balance, shadow brightness,
vignette and sharpening. Sky lighting also offers distance-fog colors and
atmospheric haze, which require Image effects. HUD/menu rendering bypasses
these effects. Contact shading uses visible depth, so off-screen objects cannot
contribute; turn its strength to zero to reduce GPU cost. The filmic curve
retains bright-light detail but is an optional artistic treatment of the original
baked colors. All these effects start disabled in Original.

Presentation interpolation could smooth animation at higher display rates while
keeping the original 30 Hz simulation; it remains a follow-up.

The export/edit/reload workflow above supports higher-resolution textures and
replacement models without changing collision or logic. Automatic processing
cannot supply bespoke new detail or remove lighting baked into the art.
Broader presentation upgrades are tracked in `CONFIGURATION.md`.

The first controller used is selected until disconnected. Keyboard and controller
can be used together. Disconnecting the active controller pauses gameplay.
Held controls must be released after menu or focus changes. Losing focus freezes
simulation and audio. Controller mappings use [gilrs](https://docs.rs/gilrs/0.11.2/gilrs/)
and accept its `SDL_GAMECONTROLLERCONFIG` environment override.

The game supports movement,
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

Saved-campaign regressions complete all ten missions for both characters,
including the final bosses, results and completion screen. Stock and RNG carry
through real encoded/decoded saves at every boundary. The recordings include
their starting seed and equipment, so
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
| Rikimaru | [9](crates/tenchu-pc/tests/data/mission9-rikimaru-campaign.inputs) | 6,292 | 3 |
| Ayame | [9](crates/tenchu-pc/tests/data/mission9-ayame-campaign.inputs) | 5,741 | 67 |
| Rikimaru | [10](crates/tenchu-pc/tests/data/mission10-rikimaru-campaign.inputs) | 15,082 | 30 |
| Ayame | [10](crates/tenchu-pc/tests/data/mission10-ayame-campaign.inputs) | 16,015 | 7 |

Both complete Mission 2 with 100 HP. The independent
[Mission 7 route](crates/tenchu-pc/tests/data/mission7-rikimaru.inputs)
also passes with the corrected swimming camera, finishing at tick 3,461 with
93 HP. [PLAN.md](PLAN.md) records the feature coverage, native play checks and
source/retail comparisons. These routes exercise ordinary controller inputs;
they do not prove exhaustive parity with retail.

```sh
TENCHU_DISC='/path/to/game.cue' \
  ./dev.sh cargo test --release -p tenchu-pc controller_route -- --ignored
```

Native window checks cover movement, combat/items, pause/resize, language
selection and death/retry. Arbitrary inputs and every difficulty/layout
combination have not been exhaustively compared with retail.

Optional GPU checks cover source blending/fog, filtering/MSAA, light occlusion
and actual character/weapon shadows. Run these sequentially to avoid concurrent
graphics-device initialization:

```sh
TENCHU_DISC='/path/to/game.cue' \
  ./dev.sh cargo test --release -p tenchu-pc renderer::tests -- --ignored --test-threads=1
```

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
