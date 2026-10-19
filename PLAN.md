# Implementation plan

## Goal and scope

Deliver a playable modern Rust reimplementation of Tenchu: Shinobi Gaisen
SLPS-019.01, equivalent in game functionality to the original `MAIN.EXE`.
Preserve movement, combat, collision, AI, RNG, item behavior, mission rules and
glitches. Preserve fixed-width integer and GTE arithmetic wherever it affects
gameplay. Original main-menu presentation is out of scope; provide a small
character/difficulty/mission launcher. Continue after each milestone until the
complete game works.

Use the original nominal game rate (30 game updates per second in this release),
independent of display rate. Rendering may interpolate but may not mutate the
game. Keep drawing-related coordinate, effect, RNG and activation side effects
in the ordered game update. Do not recreate PS1 frame drops. PC overload is a
host scheduling/presentation concern, not a source of gameplay `SkipFrame` flags.

## Existing resources

These sibling repositories contain extensive existing work and must be consulted
before reimplementing a subsystem:

- **`~/programming/tenchu-decomp-glitch-research`**: current TAS/search research,
  especially `tools/tenchu-sim` (large source-faithful Rust gameplay model),
  `tools/tenchu-wire`, recorded controller tapes and differential tests. The model
  owns actors, camera, AI/RNG, collision, terrain, effects, items and allocation.
  Use it for algorithm and behavioral reference, not as code to copy wholesale.
  Its imported-state boundary, search stop conditions, batched worlds, SoA/SIMD
  machinery and source-C build dependencies do not belong in the new runtime.
  Implement straightforward scalar logic over ordinary owned game state;
  compiler auto-vectorisation is welcome without special parallel-world layouts.
  Read `docs/tas-native-validation.md`, `docs/tas-native-coverage.md`,
  `docs/tas-native-player-items.md` and `docs/tas-campaign.md`.
- **`~/programming/tenchu-decomp`**: matching C, original assembly, symbols,
  recovered structures, source-file groupings, archive formats and local retail
  files. Use the matching implementation to resolve ambiguous behavior. In the
  research inventory 537/555 main game functions are matching C, with eighteen
  canonical assembly drawing/spline helpers. This does not mean other
  executables or the Rust runtime are complete.
- **`~/programming/tenchu-viewer`**: Rust asset decoders/exporter, stage/model
  construction, original textures, WebGL renderer, model animation and a native
  simulation comparison viewer. Reuse the data knowledge and rendering work;
  its preview physics and floating-point animation are not gameplay authority.

Keep these repositories intact. Import source with a repository revision and
content manifest, including any uncommitted source changes actually imported.
The port must ultimately build and run without sibling checkout dependencies.

## Architecture

Start with a small Cargo workspace, adding crates only at real dependency seams:

- Disc/assets: checked byte readers for ISO/BIN/CUE and the AFS volume, then
  models, textures, terrain, motion and scripts. Work with byte sources so a
  native file and a browser-selected file can use the same decoders.
- Gameplay: fixed-step native state and source-ordered scalar updates. Consult
  the Rust simulator and preserve its useful regression evidence. Runtime code must not
  require an emulator, source-C toolchain, OS clock or filesystem.
- Presentation/host: gamepad and keyboard input, a renderer, audio and a minimal
  launcher. Immutable scene snapshots/events cross the game/presentation boundary.
  Avoid a desktop-only engine requirement in the game/asset crates.

Use logical asset names and an explicit override-before-disc lookup. Initially
load original data directly from the supplied disc. Do not require committing
or prebundling extracted assets. High-resolution models and textures are later
deliverables; visual meshes must not silently replace gameplay collision data.

## Deliverables and acceptance

### 0. Repository and executable foundation

- [x] Initialise Git, document scope, resources and development conventions.
- [x] Cargo workspace, pinned Nix toolchain/package/checks, formatting.
- [x] Read ISO-9660 data from ISO and PS1 raw BIN/CUE; locate `MAIN.EXE` and
  `DATA.VOL`, identify the supported release and list mission resources.
- [x] Compile the reusable asset/core boundary for native and WASM.
- [x] Fixed-step scheduler with a meaningful different-display-cadence test.

Acceptance: a command reads the real user disc without a BIOS or emulator; the
same asset parser and scheduling/core code compile for WASM.

### 1. Mission 1 interactive gameplay

- [ ] Implement simple scalar gameplay state and source-ordered updates, guided
  by the existing Rust model, matching C and continuous regression evidence.
- [ ] Drive one live world from keyboard/gamepad, with source camera and controls.
- [ ] Render original stage, characters, weapons, items and essential effects.
- [ ] Construct Mission 1 from disc assets and launch options, without captured
  RAM or emulator setup. An imported checkpoint is an intermediate diagnostic.
- [ ] Movement, jumping, climbing, grappling, swimming where applicable, stealth,
  alert/chase, combat, all available items, player death and restart.
- [ ] Basic HUD and sound feedback. Exercise both playable characters.
- [ ] Reach and fight the mission bosses; report cinematic boundaries explicitly
  until milestone 2 implements them. Do not invent state transitions to claim a win.

Acceptance: ordinary interactive play starts directly from the disc, remains
stable under varied input and rendering rates, and reaches the authored mission
events with source-faithful state. Validate representative continuous gameplay
against the existing retail corpus at comparable logical update boundaries.

### 2. Mission 1 from launch through results

- [ ] Execute CVA actor/camera/motion/dialogue commands and return to gameplay.
- [ ] Complete both boss encounters, results/ranking, inventory rewards and retry.
- [ ] Persist character, difficulty, inventory and mission progress to a save.
- [ ] Complete music, voices and necessary movie/audio decoding.
- [ ] Minimal launcher for new game, continue, character and difficulty; original
  title/menu artwork and exact menu timing remain out of scope.

Acceptance: complete Mission 1 and continue into Mission 2 without state imports,
emulator handoffs, fabricated event completions or manual asset preparation.

### 3. Complete MAIN.EXE gameplay

- [ ] All ten missions and training, both characters, supported difficulties and
  three authored layouts, including character-specific event branches.
- [ ] Every actor, enemy/boss behavior, item, environmental effect, door/pitfall,
  reinforcement, death/retry and mission transition used by those missions.
- [ ] Full campaign persistence, unlock/reward rules and final mission handoff.
- [ ] Continuous campaign runs and ordinary exploratory play beyond the TAS route.
- [ ] Original presentation coverage: HUD, fog, transparency, effects, audio,
  subtitles and cinematics. Exact PS1 raster artifacts are not a requirement.

Acceptance: a fresh game can progress through the campaign with both characters;
save/reload and death/retry work; known unsupported gameplay branches are resolved
or individually documented as outside `MAIN.EXE` scope. Passing one fixed route
does not establish arbitrary-play coverage.

### 4. Portable, maintainable release foundation

- [ ] Desktop builds with portable CPU requirements and controller settings.
- [ ] Browser build using the same gameplay/assets, user-selected disc data and
  browser persistence; if host features differ, expose that limitation explicitly.
- [ ] Simple asset replacement directory/pack with original assets as fallback.
- [ ] Reproducible gameplay recordings and useful state-difference diagnostics.
- [ ] Document controls, supported disc format, saves, overrides and actual limits.

The goal is not complete at the first playable prototype. Finish milestone 3 and
the portability/maintenance requirements needed for the delivered game, then
report evidence and remaining optional enhancement work honestly.

## Verification policy

Use targeted tests for malformed/truncated archives, address/offset conversion,
integer edge behavior, actor/update order, RNG and continuous replay comparisons.
Test display independence with the same input-by-tick sequence under different
host frame schedules. Use private retail fixtures from their existing locations;
tests requiring them must say so and must not silently claim they ran.

For each meaningful implementation, run formatting and relevant tests, inspect
actual rendered output when visuals change, and commit a coherent increment.
Do not spend effort on tests that restate getters or implementation details.
No historical PS1 lag prediction or matching of console wall-clock finish times
is required. Preserve deliberate gameplay quirks instead of cleaning them up.

## Current status

2026-10-20: foundation complete; Mission 1 gameplay is in progress. The native
executable reads the actual SLPS-019.01 mixed-mode disc, indexes 1,048 DATA.VOL
entries and constructs all 66 mission/character/layout combinations without RAM
imports. Assets and core compile for WASM. Nix packages the desktop executable
and checks native tests, WASM and formatting.

Ordinary play now runs source-ordered actor control and periodic enemy activation.
Implemented systems include scalar fixed-point math, command history, RNG,
player movement/jump/crouch/ledge control, persistent coordinate caches, ordered
body collision, camera follow/look, original animation, weapon attachment/swap,
patrol/investigation/combat decisions, reinforcement replacement, melee combos,
weapon hitboxes/clashes, guarding, damage reactions, authored stealth deaths,
dropped inventory, pickups, arrow/gun/shuriken attacks, player aiming/cancellation,
medicine, caltrops and item damage. The native
renderer displays original textured scenery, visible actors, held weapons,
shadows, blood pools, weapon trails and the implemented particle effects.
Unsupported actions still pause with a diagnostic. Grappling, remaining item use, special ranged attacks, mission events, map display and audio remain unfinished; this is not
playable Mission 1.

Enemy activation retains the source's 30-tick sampling delay, ordered budget,
wakeup guards and raw parking-coordinate writes. Integer geometry counters use
original meshes, construction cell/depth ordering, adaptive subdivision, draw
banks, shadows and implemented effects. They do not measure host performance,
drop frames or change with render resolution. The sample remains incomplete
until remaining effects, items and environmental producers are implemented;
activation is therefore not yet certified against continuous retail gameplay.

Smoke, splashes, droplets, blood/gore, impacts and attached flashes share the source 200-slot pool
and overflow dummy. DrawShadow preserves waist-cache, map-flag and splash side
effects. The renderer consumes prepared effect graphics without mutating the game, using
original TIM palettes, sprite origins and four transparency modes. It uses modern
depth testing rather than reproducing PS1 ordering-table raster artifacts. Sound
events are still unplayed. Damage-floor napalm remains an item-system boundary.
The HUD now updates inventory selection after items, including the reserved-slot
quirk, and displays original icons/counts, health bars and the proximity indicator.
Five temporary enemy bars retain request order, captured health and expiry. Their
logical packet contributions are included in activation sampling. The desktop
launcher supplies the original fallback loadout (hook, 10 shuriken, 5 caltrops,
2 medicine). Shuriken aiming/release/cancellation, medicine and caltrops now run
through normal input, actor animations and item updates. AI can request medicine.
Grappling remains the next integration step.
CVA camera/actor ownership, remaining ranged/item combat, swimming/grappling, results,
saving, transitions and the remaining campaign are still required.

Verification completed so far:

- A 594-tick replay exercises both characters at 24 Hz and 144 Hz presentation,
  including melee inputs and comparing the complete roster, combat/scoring,
  animation, effects, RNG, camera, collision,
  coordinate caches, activation and drawing state at each tick. The actual X11
  window was also driven through movement, jumping, crouching, looking, resize
  and pause, and its output inspected.
- All six character/angle stealth pairs complete through ordinary game ticks in
  constructed private-disc scenarios, including the critical camera, authored gore
  or its absence, collision removal, blood-pool growth and return to control.
  A regression preserves the weapon-clash bug that indexes BattleDB with the
  collision slot. These are controlled scenarios, not complete mission runs.
- DamageControl and its original cleanup/scoring helpers match in 8,192 varied
  human/item cases: health, reactions, knockback, attacker recoil, RNG consumption, scores,
  sound requests and collision-pool size. The native C harness supplies open
  terrain, motion descriptors and effect-allocation stubs; rendering and complete
  continuous gameplay are outside this isolated comparison.
- Item rolling matches the original C in 4,096 cases using recorded terrain
  responses, including bounce/out-of-map behavior, narrow velocities, RNG and
  splashes. Another 2,048 complete projectile arcs match original setup, jitter,
  every flight position and rolling handoff. Targeting rotation matches 2,048
  executions of the retail SDK routine. Both characters collect a pickup through ordinary ticks. Regressions
  preserve interrupted final-tick transfer and eviction before failed allocation.
- Controlled ordinary-tick scenarios exercise bow, pistol, rifle and ninja
  attacks through projectile launch, collision, player damage and disposal;
  arrows attach to the struck body. The native window renders an archer and
  flying arrow using the original meshes. These scenarios do not establish
  full mission coverage.
- Both characters exercise quick and aimed throws, cancellation refunds, interrupted
  throws and medicine drops, medicine healing and caltrop contact damage through
  ordinary ticks. A low-health enemy heals through the AI/item loop. Pool eviction
  preserves cancellation of an aimed item owned by the currently executing actor.
  Interactive window checks display the aiming sight and flying shuriken trail.
- Original-C comparisons cover 512 camera-smoothing cases, 45,760 enemy sight
  cases with exact ray arguments, 81,920 engaged combat-controller cases and
  4,096 varied activation worlds. Reinforcement continuation has a private-disc
  regression check. These isolate components; they do not establish full AI or
  complete gameplay parity.
- The integer view matrix matches a retail capture. Constructed roster totals,
  weapons, patrols and layout homes match persistent retail fields. All 339
  scenery objects in that capture match transforms, bounds, subdivision flags,
  vertex geometry and spatial-list order. CVA-mutated homes are excluded from
  the construction check.
- The projection kernel matches an independent arithmetic oracle in 67,584
  full-width/depth cases. Focused tests preserve custom-versus-SDK clipping,
  degenerate subdivision stitches, contact-list tails, current-epoch coordinate
  caches, effect allocation/RNG and animation hold/sweep behavior.

Next: grappling, then remaining item behaviors, swimming and mission events. Fill the remaining drawing producers alongside their owning gameplay
systems. No batched-world storage, PS1 timing model or source-C runtime dependency
has been introduced.
