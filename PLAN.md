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

- [x] Implement simple scalar gameplay state and source-ordered updates, guided
  by the existing Rust model, matching C and continuous regression evidence.
- [x] Drive one live world from keyboard/gamepad, with source camera and controls.
- [x] Render original stage, characters, weapons, items and essential effects.
- [x] Construct Mission 1 from disc assets and launch options, without captured
  RAM or emulator setup. An imported checkpoint is an intermediate diagnostic.
- [x] Movement, jumping, climbing, grappling, swimming where applicable, stealth,
  alert/chase, combat, all available items, player death and restart.
- [x] Basic HUD and sound feedback. Exercise both playable characters.
- [x] Reach and fight the mission bosses; report cinematic boundaries explicitly
  until milestone 2 implements them. Do not invent state transitions to claim a win.

Acceptance: ordinary interactive play starts directly from the disc, remains
stable under varied input and rendering rates, and reaches the authored mission
events with source-faithful state. Validate representative continuous gameplay
against the existing retail corpus at comparable logical update boundaries.

### 2. Mission 1 from launch through results

- [x] Execute Mission 1 CVA actor/camera/motion/text commands and return to gameplay.
  Latin subtitles and original localized dialogue play from the disc.
- [x] Complete both boss encounters through ordinary controller input, alongside
  separate native controls and presentation checks.
- [x] Results/ranking, inventory rewards, retry and source training records.
- [x] Persist character, difficulty, inventory and mission progress to a save.
- [x] Music, voices and gameplay cinematic/audio decoding. The external
  `ENDING.EXE` movie handoff uses a completion screen.
- [x] Minimal launcher for new game, continue, character and difficulty; original
  title/menu artwork and exact menu timing remain out of scope.

Acceptance: complete Mission 1 and continue into Mission 2 without state imports,
emulator handoffs, fabricated event completions or manual asset preparation.

### 3. Complete MAIN.EXE gameplay

- [x] All ten missions and training, both characters, supported difficulties and
  three authored layouts, including character-specific event branches.
- [x] Every actor, enemy/boss behavior, item, environmental effect, door/pitfall,
  reinforcement, death/retry and mission transition used by those missions.
- [x] Full campaign persistence, unlock/reward rules and final mission handoff.
- [x] Continuous campaign runs and ordinary exploratory play beyond the TAS route.
- [x] Original presentation coverage: HUD, fog, transparency, effects, audio,
  subtitles and cinematics. Exact PS1 raster artifacts are not a requirement.

Acceptance: a fresh game can progress through the campaign with both characters;
save/reload and death/retry work; known unsupported gameplay branches are resolved
or individually documented as outside `MAIN.EXE` scope. Passing one fixed route
does not establish arbitrary-play coverage.

### 4. Portable, maintainable release foundation

- [x] Nix desktop packaging with portable CPU requirements and documented
  keyboard/controller mappings. Linux x86-64 is verified; physical gamepad
  hardware and the other declared build platforms remain unverified.
- [x] Portable assets, audio and gameplay compile for WASM without a native host.
- [ ] Later: browser window/input/file-selection/persistence host. This is a
  follow-up to the requested portable design, not needed to play on desktop.
- [ ] Later: asset replacement directory/pack with original assets as fallback.
  Logical asset names and byte-source readers provide the existing seam; visual
  replacements must remain separate from original gameplay geometry.
- [x] Reproducible gameplay recordings and useful state-difference diagnostics.
- [x] Document controls, supported disc format, saves, recordings and actual limits;
  asset overrides remain a later deliverable.

The goal is not complete at the first playable prototype. Finish milestone 3 and
the portability/maintenance requirements needed for the delivered game, then
report evidence and remaining optional enhancement work honestly. The checkboxes
record implemented systems and the evidence below; they do not assert exhaustive
retail parity or a manual playthrough of every difficulty/layout combination.

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

Live player appearance swapping and the idiomatic Rust refactor are complete.
Gameplay behavior is preserved while the implementation uses named types and
explicit operations instead of retaining C-shaped data structures.

- [x] Select compatible original bodies in F1 without changing player identity,
  animation, inventory, collision or disguise effects; keep weapons attached.
- [x] Load/reload custom bodies from a user-selected directory using the existing
  OBJ/PNG workflow, with validation, original fallback and documented exports.
- [x] Exercise both players, combat/weapon swaps, interpolation and shadows;
  compare complete gameplay state across appearance changes.
- [x] Introduce named coordinate types, remove unnecessary enum representations
  and numeric casts, and simplify equivalent arithmetic/iteration using Rust APIs.
- [x] Resolve Clippy findings, run source/gameplay regressions and native/WASM
  packaging checks, document retained binary/fixed-point boundaries and commit
  coherent increments.

The model swapper is presentation-only. F1 offers compatible mission bodies and
folder bodies; export-assets generates editable fifteen-part rig definitions
alongside OBJ/PNG data. New proportions move the rendered weapon attachments,
while animations, collision and gameplay remain original. All 67 desktop tests
pass, including complete campaigns and both-player body/weapon/interpolation
checks. An isolated native window changed Rikimaru to a ronin and an exported
Ayame, reloaded edited offsets, retained the previous body after an invalid path,
played movement/jump/attack with Enhanced interpolation/shadows, restored
Original with R and exited cleanly. Arbitrary skeleton retargeting is outside
this rigid-part workflow; incompatible rigs are rejected.

World positions now use `Position` with named `x`, `y`, `z` fields throughout
actors, cameras, collision, terrain, effects and scene transforms. Enum behavior
uses explicit matches, including difficulty and camera submodes; numeric IDs
remain explicit only at save/oracle boundaries. Related particle, item, AI,
HUD and rendering parameters have named request/context structures. Equivalent
arithmetic and iteration use standard Rust helpers. Signed rounding, narrowing,
wrapping, source bit flags and packed asset vectors retain their required
semantics; `repr(C)` remains only for actual GPU buffer layouts. All 138 core
regressions and all 67 desktop tests pass with private source/retail fixtures,
and the complete workspace passes Clippy with warnings denied. `nix flake check`
passes native packaging/tests, shared-crate WASM compilation and formatting.
After the refactor, the native window loaded the saved custom Ayame body, reset
to Original, changed to a ronin, played movement/jump/sword attacks and exited
cleanly. The window checks used separate settings/save files and left personal
game settings intact.

Optional configuration, native inspection tools and visual upgrades are
implemented while preserving the Original defaults. The latest additions are
sharper texture variants, a compact mouse settings overlay, corrected lamp
shadow budgets and visual-only model interpolation. See
[CONFIGURATION.md](CONFIGURATION.md) for deliverables, verification and limits.

2026-10-20: all MAIN.EXE systems are implemented. Current recordings complete
all ten campaign missions and training for both characters from ordinary
controller input and original disc assets, including the source-backed
corrections below. Each campaign carries
stock and RNG through encoded/decoded saves at every mission boundary. Native
window checks cover controls, combat/items, pause, resize, language selection
and retry. The native executable reads the actual SLPS-019.01 mixed-mode disc, indexes 1,048 DATA.VOL
entries and constructs all 66 mission/character/layout combinations without RAM
imports. Assets and core compile for WASM. Nix packages the desktop executable
and checks native tests, WASM and formatting.

Ordinary play now runs source-ordered actor control and periodic enemy activation.
Implemented systems include scalar fixed-point math, command history, RNG,
player movement/jump/crouch/ledge/swimming/wall control, persistent coordinate caches, ordered
body collision, camera follow/look, original animation, weapon attachment/swap,
patrol/investigation/combat decisions, reinforcement replacement, melee combos,
weapon hitboxes/clashes, guarding, damage reactions, authored stealth deaths,
dropped inventory, pickups, arrow/gun/shuriken attacks, player aiming/cancellation,
medicine, caltrops, grappling, smoke bombs, grenades, mines, poisoned food and item damage. The native
renderer displays original textured scenery, visible actors, held weapons,
shadows, blood pools, weapon trails and the implemented particle effects.
Unexpected or unsupported data pauses with a diagnostic. The current private
checks pass training for both characters, an independent Ayame Mission 2 route,
both standalone Mission 5 branches and independent Rikimaru Mission 3, 4 and
6–10 routes. The saved campaigns also pass from new game through final results,
completion screen and saved Mission 10 replay selection. These controller routes and native window checks
provide playability evidence, not exhaustive retail parity across all inputs.

Seven original Castle-start recordings now match for 4,630 consecutive updates in
total, including a 4,000-update traversal with grappling, climbing, swimming and
enemy attacks that reduce health from 100 to 73. Each
world is constructed from the disc; the test supplies its launch seed/loadout
and aligns the player's initial pose and ambient deadlines once. The captured
loading tick advanced motion counters without animation, and ambient sounds
retained the pre-loading clock 644. Later recorded state is only an
assertion target; controller words are the only subsequent inputs. Checks cover
all 23 actors, animation/coordinate caches, camera and smoothing history, shared
RNG, collision, event time, activation budgets and exact drawing allocation.

The comparison exposed two runtime defects: the swimming camera used state 13
instead of 3, and adaptive drawing used pre-initialization SDK globals instead
of `jt_init4`'s depth 602 and extents 512. Both are corrected. The original
campaign recordings have been refreshed for both characters using their actual
saved stock/RNG. Rikimaru finishes Mission 10 at tick 15,082 with 30 HP and
Ayame at tick 16,015 with 7 HP. Both defeat Onikage and Mei-Oh and save all ten
mission flags. The independent Mission 7 route also passes with the corrected
swimming camera. The longer continuous comparison covers receiving
attacks as well as traversal. Console
frame-cost prediction remains outside scope.

At the MAIN.EXE milestone, the Linux host suite passed all 37 tests, including private-disc,
full-campaign, audio and GPU checks. The final Nix desktop package, native tests,
WASM libraries and formatting checks pass on x86_64-linux. The packaged
executable successfully inspects the real disc and resumes the actual completed
Ayame save into Mission 10. Physical gamepad hardware and the other declared
build platforms remain unverified. These checks do not establish exhaustive
retail parity.

A native Rikimaru training attempt also reaches actual combat death and the
mission-failed screen, then retries through fixed equipment selection. The
recordings replay to game over at tick 2,177 and a new attempt at tick 74 with
100 HP and five shuriken; the second attempt inherits the first attempt's final
RNG seed (`c6bac074`). The source death animation and countdown both finish
before the failure screen appears. Screenshots of failure, equipment and pause
were inspected on an isolated X server with a private silent audio sink.

The ongoing TAS campaign conversation (`01a0faf5-e0e4-7f81-a768-0172851c36b1`)
now uses `/home/shana/tenchu-searches/mission-1-action-repair-20261013`; its current
Mission 10 work and existing boot-connected OTAS routes are additional validation
resources. Read them without disturbing the running search. If used for whole
campaign parity, collect actual logic/controller/draw-skip boundaries during a
BizHawk replay and confine those skips to a test harness. Feature implementation
takes priority over an exhaustive console-validation suite.

## Implementation and verification record

The following entries record incremental work. Early statements about missing
features are historical; the current deliverable checklist and status above
summarize what is implemented now.

Enemy activation retains the source's 30-tick sampling delay, ordered budget,
wakeup guards and raw parking-coordinate writes. Integer geometry counters use
original meshes, construction cell/depth ordering, adaptive subdivision, draw
banks, shadows and implemented effects. They do not measure host performance,
drop frames or change with render resolution. Drawing coverage and activation
are not yet certified against continuous retail gameplay.

Smoke, splashes, droplets, blood/gore, impacts and attached flashes share the source 200-slot pool
and overflow dummy. DrawShadow preserves waist-cache, map-flag and splash side
effects. The renderer consumes prepared effect graphics without mutating the game, using
original TIM palettes, sprite origins and four transparency modes. It uses modern
depth testing rather than reproducing PS1 ordering-table raster artifacts. Sound
events now play through the portable mixer. Damage-floor napalm is implemented below.
The HUD now updates inventory selection after items, including the reserved-slot
quirk, and displays original icons/counts, health bars and the proximity indicator.
Five temporary enemy bars retain request order, captured health and expiry. Their
logical packet contributions are included in activation sampling. The desktop
diagnostic replay supplies the original fallback loadout (hook, 10 shuriken,
5 caltrops, 2 medicine); the launcher uses source stock and equipment selection.
Shuriken aiming/release/cancellation, medicine and caltrops now run
through normal input, actor animations and item updates. AI can request medicine.
Return-to-normal transitions now apply the source player camera reset, including
shuriken recovery and the held-look-button exception.
Smoke bombs apply the source radius/owner filters and choke motion; grenades and
mines preserve arming, fuse/blast lifetimes, dud recovery, placement refunds and
water disposal. Smoke, explosion and ember effects share the existing ordered
pool and original textures.
Grappling now includes source targeting, flight/countdown, pull/contact exits,
cable RNG, impact particles and original hook geometry. Empty cable curves omit
the hook draw instead of reading undefined retail stack locals; their exact
original packet cost is therefore outside the fidelity claim.
Swimming now preserves water entry, splash RNG, attack cleanup, hidden body parts,
steering, reverse blockage, hook-only use/cancellation, shore exit and non-swimmer
drowning. Falling also performs the original attack cleanup.
Wall hugging now preserves shared static animation edits, cardinal alignment,
slide/roll/release transitions, and caltrop/grenade/smoke/poisoned-food corner throws. Corner
throws latch selection and decrement inventory bytes even from zero or 255.
The original null-map probe reads a zero vector byte from low RAM in the available
retail capture; the portable implementation explicitly uses reflected vector zero
for that branch. Other BIOS low-RAM contents are outside that fidelity claim.
Poisoned food now supports rolling, owner recovery, ordered target selection,
luring, eating/interruptions, poison reactions/expiry and cleanup of the saved
AI callback. Smoke and food share the source target filter.
Mission 1 event routing and CVA camera/actor ownership now run from the disc.
The complete campaign and training results are recorded below.

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
  Interactive window checks display the aiming sight, flying shuriken trail and
  medicine animation/healing particles.
- Both characters aim, grapple into Mission 1 geometry, return from flight/pull,
  cancel and reject distant targets through ordinary input and game updates.
  Hook setup, flight/straightening timing, random bends and impact RNG match the
  original C in 2,048 cases. This oracle supplies the integer square-root result
  and stubs draw/sound/particle sinks. The native window renders the hook/cable.
- Both characters throw smoke bombs/grenades and plant mines through ordinary
  input, animation, item/contact updates and enemy damage in controlled scenarios.
  Separate checks cover grenade dud recovery, invalid/water mine refunds and
  water extinguishing. Smoke/explosion/ember state and RNG match original C
  through expiry in 384 cases with graphics submission stubbed. Window checks
  show the grenade blast, embers and smoke cloud using a temporary QA loadout;
  the launcher retains the original fallback inventory.
- Both characters swim, reverse, cancel a hook and exit to dry terrain through
  ordinary ticks in controlled scenarios. Entry checks cover splash RNG/order,
  one-time cleanup and body visibility; a non-swimmer completes drowning and
  loses body collision. A separate fall regression checks attack cleanup. A
  native window check places the player in actual Mission 1 water and exercises
  swimming/hook cancellation. The water surface now uses its original animated texture.
- Both characters hug/slide against actual Mission 1 terrain and return to
  normal control. Controlled corner-throw checks cover selection changes,
  release during the throw, both sides and byte underflow; a missing-wall
  check preserves source advance/revert behavior. A native window displays
  wall entry, sliding and release. The private asset check confirms editable
  wall tracks do not alias other tracks in either player's original AMD, and
  display-cadence snapshots now include shared wall motion keys.
- Both characters throw poisoned food and lure a guard through eating, poison
  reactions and expiry in controlled ordinary-tick scenarios. Tests preserve
  interrupted-food detachment/RNG, restoration of the original idle callback,
  owner-pickup dependence on roster order, and poisoned-food corner throws.
  The native window displays the original thrown-food sprite and its landing.
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

Continue filling the remaining drawing producers alongside their owning gameplay
systems. No batched-world storage, PS1 timing model or source-C runtime dependency
has been introduced.

Stage event assets load with each mission: all ESD tables and 88 localized
character CAD files pass the private-disc parser check. Event triggers and cached
targets, both successor slots, death roots, training grade selection and cinematic
returns now run in the core. The host tick count is separate from the original
signed game clock, which advances during CVA then restores at the event return.
Controller input stays at the entry poll throughout CVA, including the case where
a later item clears ActionHalt. Script-owned motions retain ordinary collision/action updates while guarded
transitions and damage respect CVA ownership. Normal and cinematic cameras share
the original view position. The presentation snapshot retains the view used for
the completed frame, even when subsequent script commands change the camera.

Original Latin glyphs, proportional spacing, accents, letterboxing, Tanka sprites
and full-screen fade effects are supported. Japanese subtitle rasterization still
needs a portable font source instead of the PS1 BIOS KROM. Music commands are
queued for native playback from the original XA streams. The deliberate VSync(60) audio tail
holds the last frame for one second without world updates. It is distinct from
PS1 rendering slowdown, which remains excluded.

New verification: both characters' twelve Mission 1 sequence entries execute with
live actors. Authored event chains activate bosses and restore the mission clock;
controlled boss/player-death states exercise the real routes to completion and
game over. Those injected death conditions are not evidence of an ordinary
complete playthrough. Source-C comparisons cover 576 event worlds, 512 cinematic
camera cuts/pans, 512 score cases and 3,072 fade updates (including clock rewind).
An isolated campaign diagnostic attempts all 126 English sequence entries; each
finishes or takes a source-defined early return without a runtime error. This is
not campaign playthrough coverage. A separate check protects script-owned motion
and the frozen audio tail. Native
window captures show the opening, subtitles, actor poses and return to combat.
The intro/return also produces identical state at different display cadences and
with simulated host stalls. Full-workspace strict Clippy currently reports 25
pre-existing style lints; no Clippy-clean claim is made.

Result bookkeeping now exists in the portable core: packed counters/time,
per-character/layout best records, completion flags, training leaderboard,
source item awards/leftover returns, next-unplayed-layout selection and the stock
selection limits. The desktop connects these rules to result,
retry and inventory screens and save storage. C comparisons pass for 2,048 award
and return cases with exact RNG, plus the existing 512 score cases. Portable
scenario checks cover character isolation, ranking ties and failure stock loss.


The native window now starts in a minimal character/difficulty/mission/layout
launcher and source-stock equipment selection. Stage completion shows rank,
score and best records, persists rewards, and advances to the next unplayed
layout. Failure/retry keeps the remaining shop stock and loses carried items.
Training has fixed equipment and its separate leaderboard. Original armour
models are loaded and its existing damage reduction is enabled on equipment.
Random layouts are selected after player/dummy construction with the original
RNG call ordering; all three candidate layouts are loaded only when required.

Saves are a portable versioned byte format with integrity and range checks; the
native host writes them atomically to the application data directory (or
TENCHU_SAVE). Continue starts at equipment selection. A running attempt is not a
mid-mission save. The initial deterministic seed is 1 and later attempts carry
forward the prior gameplay/reward seed; menu wait/render duration never consumes
RNG. Original VSync-based startup seeding is an explicit host-policy boundary.

New checks cover corruption/truncation and replacement of saves; controlled
completion/death transitions for both characters across all ten campaign stages;
save/continue; random layouts; and armour variants. These transitions are not
ordinary gameplay completion evidence. All eight private PC checks pass,
including roster/assets, 66 fixed constructors and both cadence tests. Window
checks exercise equipment selection, ordinary movement, retry and the result-to-
next-mission presentation. Remaining items and ordinary playthrough validation still
prevent a complete Mission 1/game claim.

Original audio asset decoding is now portable: VAB programs/tones/samples,
SPU ADPCM loops, XA ADPCM mono/stereo streams and the executable's music/dialogue
cue tables. Raw Mode 2 BIN/CUE retains XA audio that cooked 2048-byte ISO images
cannot contain. All 104 non-trial banks and 419 distinct XA cues decode from the
retail disc; 10,604,160 XA samples match FFmpeg exactly in a private comparison.
Native tests and the WASM asset/core build pass. Playback and spatial dispatch are implemented below; remaining sound producers
are implemented with their owning gameplay systems.

Desktop playback now uses a portable 24-voice sample mixer plus original XA
music/dialogue. Restored movement sound triggers include walking, running on
wood, turning, crouching, jumping, ledge catches and landing. The host dispatches
requests per game tick and decodes XA on a cancellable worker with bounded cache.
Pausing/focus loss freezes audio. The mixer uses modern resampling and no SPU
reverb/register emulation; listener positions are sampled at completed ticks.
All 74 core checks pass, including 4,096 original-C spatial sound comparisons;
the eight previous private host/cadence checks still pass. New checks cover audio
buffer/pause independence, voice expiry/replacement, asynchronous cancellation and
localized opening-dialogue handoff to background music. Native window recordings
exercise music, effects, dialogue and silence during pause. Remaining item and special attack sound producers will arrive with their owning systems.

Next: finish remaining items and presentation gaps, then ordinary Mission 1
boss encounters and continuous campaign validation. The guard-specific heavy-fall
damage, slam direction, sound and health-bar branch now matches 2,048 original-C
cases, including narrow-integer overflow and RNG use. NPC gesture priority and
the shared camera reset on backflips/flip landings are also restored and checked.
These audio/progression
increments do not establish complete Mission 1 or campaign playability.


Environmental construction records now load the original door/pitfall tables,
sprites and scrolling TIMs. All activation callbacks run before object updates,
at the source ten-tick cadence around the camera reference. Doors/pitfalls share
the ordered collision pool, retain their different pause/resume rules and use
shared leaf coordinate caches. The unchecked exhausted collision result retains
its original behavior: every authored door has a constant direction over the
possible aliased ordering-table links, so no GPU-address emulation is needed.

Fire eruptions, steam puffs, bonfires, decorative flames, snowfall and ambient
sound schedules preserve their RNG/effect allocation order. Snow wraps with the
source unsigned remainder and queries first-hit terrain. Scrolling textures keep
persistent effect slots and 1/16-word offsets. Their original texture copies are
applied in reverse submission order at the next completed-frame boundary; native
uploads are keyed by texture revision, independent of display cadence.

Verification: 4,096 original-C emitter cases (particle RNG costs stubbed), 4,096
door/pitfall transition cases (SDK angle results supplied), 2,048 snow trajectories
with flat terrain/projection stubs, and 40,960 texture-scroll updates pass. All 83
core checks plus a separate all-43-authored-doors proof pass. Ten relevant private
host checks pass, including 66 constructors, progression and cadence comparisons.
Native window checks show a door opening as the player walks through it, animated
water during swimming, snow and decorative flames. These were relocated QA starts,
not complete mission runs. The lava-area check reaches the existing unimplemented
damage-floor napalm diagnostic, now resolved by the next increment. Camera-side
scene culling and full stage transparency also need presentation follow-up.


Damage-floor napalm, Manji flame attacks and the player fire-breath item now run
in scalar owned state. Napalm retains inventory consumption and pool allocation
before even-tick rejection, curved motion, delayed collision, attached flashes,
RNG/color/rotation order, terrain-exit cache behavior and both sprite passes.
Fire breath retains its fixed cast origin, guarded motion entry, interruption
refund, 40-tick firing window, steering/aiming and return-to-normal cleanup.

Verification: 4,096 original-C napalm cases and 3,072 fire-breath callback cases
pass, with terrain responses and already-verified rotation/motion/projection calls
stubbed as stated by each test. Ordinary update tests exercise both player casts,
Manji damage and the retail final-stage lava floor. All 90 current core checks
pass except the separately excluded long campaign cinematic coverage check.
A native keyboard check visibly casts and recovers from fire breath; a relocated
lava start runs through damage without the former diagnostic. Its camera is
occluded by scene geometry, reinforcing the remaining renderer culling work.
Ten private host checks, including both cadence comparisons, pass. Nix native,
WASM and formatting checks pass on x86_64-linux. These are subsystem checks,
not ordinary completed mission runs.


Renderer follow-up: original mesh face winding now culls back faces while
preserving explicitly double-sided faces and screen sprites. Transparent stage,
placement, character and weapon faces join the depth-sorted effect surfaces;
their opaque texture pixels still write depth before semitransparent pixels blend.
Original texture STP handling and all four blend equations are exercised in an
ignored offscreen GPU check, together with front/back/double-sided faces. Stage
and character traversal is shared between the opaque and transparent builders.

Native windows confirm the final-stage lava start is now visible instead of
being covered by back faces, and render the fire-breath effect and Mission 4
scenery. The `render` command now captures 30 real gameplay ticks through the
same camera, character/effect and HUD path. No game state or tick ordering changed.
Lighting, fog, original distance/sector visibility and continuous-play validation
remain separate work; this is not a complete original renderer claim.
Public workspace tests and the GPU check pass; the Nix native package, WASM
boundary and formatting checks pass on x86_64-linux. The production binary was
rebuilt after removing the temporary QA starts, and its `render` output was
inspected.


Sleeping gas now follows the original hand release, moving soft collision,
sinusoidal sprite pulse, particle RNG, contact consumption, discarded hit jitter,
weapon sheathing, sleeping AI callback and lifetime/terrain cleanup. Its release
and early exits preserve the coordinate cache ordering. Item icons, napalm and
gas share one sprite projection/draw helper.

Verification: 4,096 original-C gas flight/contact cases pass with supplied sine
and terrain results, previously checked particle RNG costs and projection stubs.
Separate checks exercise release-cache preservation, an interrupted cast without
a refund, and both characters throwing at an armed guard through ordinary ticks;
the guard sleeps without losing health. The full 93-check core suite passes,
excluding the unchanged long campaign-cinematic coverage check. Native keyboard
checks visibly throw and draw the gas for both characters. Their temporary QA
loadouts are not included in the runtime.
All 11 relevant host/GPU checks pass, including both cadence comparisons. Nix
native, WASM and formatting checks pass on x86_64-linux. The debug executable
was rebuilt from clean runtime sources after the visual check.


The lure flute and resurrection leaves now use the shared source item pool.
The flute clears the global alert, plays its local cue, and holds the AI active
flag for thirty updates. Forced disposal deliberately leaves that flag set,
matching the original callback. Resurrection runs after lethal-hit feedback,
consumes carried stock, restores maximum health, preserves launch/swim reactions
and emits the original thirty-one bursts of blue recovery particles.

Verification: 512 original-C support-item callback cases compare counters, flags,
RNG and every particle field; 288 original-C damage-tail cases cover both players,
finite/infinite stock, launch, swimming and CVA suppression. Ordinary-tick checks
exercise both players surviving lethal arrows and the complete effect lifetime.
A native window check uses the flute, while a relocated lava start displays blue
revival particles and restored health. These are controlled subsystem scenarios.
All 97 current core checks pass, excluding the unchanged long campaign cinematic
coverage check; all 11 relevant private host/GPU checks pass.
Nix native, WASM and formatting checks pass on x86_64-linux. The production
debug binary was rebuilt after removing the temporary QA setup.

Replay diagnostics now report health, game clock, cinematic activity and outcome,
and optionally capture the final frame through the shared native renderer. A
200-tick keyboard-mask tape starts Mission 1 directly from disc and runs down
the starting street; its final capture was inspected. This is the beginning of
continuous mission validation, not a completed run.


The original `CHIZU.TIM` map and per-stage executable transforms now load with
every mission. Holding Select (M/Tab on desktop) opens the source slide animation
and local sound while the world continues running. The overlay retains its
subtractive backdrop, ghost image, player/rice-marker filtering, integer
projection and original red/blue channel reversal. Colored-rice use itself is
still pending. Cinematics suppress the overlay without resetting its saved mode.
Ordinary HUD updates now precede environmental objects as in `START.C`; cinematic
text remains after those objects as in `CHRANIM.C`.

Verification: 1,024 executions of the extracted `PutMap`, `draw_map_items_` and
`DrawTargetS` callbacks match animation state, submitted sprites/lines, sound and
packet counts, including full-width coordinate overflow and narrow line endpoints.
SDK submission and supplied trigonometric results are stubbed. All 99 current
core checks pass, excluding the unchanged long campaign cinematic check; all 11
relevant host/GPU checks pass. The cadence tape now includes moving with the map
open, and all 66 constructors load their map texture and placement. Native
keyboard checks open, move with and close the map for both characters.

Continuous Mission 1 exploration reached the compound roof/garden and ordinary
guard combat, without state imports. A subsequent controller-only route reached
Hanbe, defeated him, completed both encounter cinematics and used medicine.
That run exposed the terrain-probe defect corrected below; it is not validation
of the corrected runtime or a completed mission. A converted older TAS controller
proposal did not reproduce its original route at the nominal tick rate, before
or after this fix; it is not playthrough or parity evidence.
Nix native, WASM and formatting checks pass for the map increment on x86_64-linux.


Terrain movement probes now use the executable's four diagonal offsets in their
original bit order. The first port incorrectly substituted compass directions,
which misclassified straight walls against the original reflection tables. This
caused sideways wall recovery, trapped movement and sideways aiming-camera
corrections. The same shared query now correctly serves actors, camera, item
rolling, shadows and ledge/wall checks.

Verification: 4,096 extracted `GetAreaMapVector` executions match masks, height,
attributes, retained cursor and ordered query coordinates/modes. The C harness
reads the real direction table and uses recorded responses from the terrain
lookup; it does not independently validate `GetAreaMapLevel`. Public geometric
cases distinguish all four straight walls from corners. Both characters also
press against Hanbe's west wall for 100 ticks and retreat through ordinary input.
Native keyboard checks repeat wall pressure and retreat from controlled relocated
starts for both characters. The temporary starts were removed and the binary
rebuilt. The earlier 101-check core suite plus this new integration check and all
11 relevant host/GPU checks pass. Nix native, WASM and formatting checks pass
on x86_64-linux.

The corrected controller-only Mission 1 route now completes both boss fights,
all four encounter cinematics and the authored completion event at tick 7,684,
with 75 HP. The checked-in input file contains no state imports or event edits.
Its integration test selects six shuriken, five caltrops and two medicine from
new-game stock through the launcher actions, executes the route, checks results,
encodes/decodes the resulting save and starts Mission 2 through its opening scene.
This is a continuous regression for one Rikimaru/normal/layout-1 route, not retail
parity, a full manual window playthrough or arbitrary-play coverage. Both
characters, all layouts and remaining items still require completion work.

The keyboard's Shift binding now sends the core crouch action (`CIRCLE`); it
previously sent an unused `R1` bit. Native keyboard checks confirm crouching and
returning to standing for both characters from ordinary mission starts. No
temporary relocated starts or other QA state overrides were used for these checks.

Colored rice now supports ordinary selection, planting, hand release, rolling,
map marking and pickup. It retains the source's unusual state-controller override
for the shared planting animation, interrupted-cast restart, raw rotated velocity
and disposal-before-drop allocation order. Both characters pass a full tick-driven
plant/map/pickup check; a controlled full-pool case checks interruption, halt,
hand selection and release ordering. All 104 current core checks pass, excluding
the unchanged long isolated campaign cinematic check.
The shared item initializer now refreshes reused coordinate rotations at the
original initialization boundary. The full Mission 1 controller regression still
passes through saved continuation. Native inventory/keyboard checks plant, mark
and recollect rice for both characters from ordinary starts.
Nix native, WASM and formatting checks pass on x86_64-linux for this increment.

The eight-shot shuriken item now creates eight independently targeted curved
flights, with original yaw jitter, per-projectile trails, an eight-tick hitbox
delay, thirty-point damage and terrain-impact effects. Humanoid contact removes
the hitbox while retaining the projectile and trail; terrain impact disposes
both. Existing flight, collision, rendering and damage routines are shared.
An extracted-C check covers 128 requests/1,024 shots, comparing RNG, rotation,
curve setup, arming, trail binding and sound. Terrain target positions and SDK
distances are supplied; the C-generated aim angles are separately fed back into
the native target lookup. Controlled ordinary-tick fights verify both characters,
and a contact regression distinguishes humanoid and terrain disposal. All 107
current core checks pass, excluding the unchanged long cinematic coverage check.
Native keyboard checks display the throw, separate trails and wall-impact sparks
for both characters. Those checks use a temporary save with the item unlocked;
normal gameplay then selects it through the equipment screen. No runtime state
override or source-code QA hook is present.
Nix native, WASM and formatting checks pass on x86_64-linux.

Speed scrolls now cast, add their original movement after the normal actor update,
steer by 64 angle units per tick and run for 75 active ticks. Terrain probes,
smoke/trail cadence, cancellation and camera changes follow the source. Interrupted
casting drops a recoverable scroll; interruption during the boost does not.
Forced item eviction deliberately leaves the source's motion/camera state intact.
Three focused checks cover both characters' full duration and cancellation,
terrain threshold edges, movement-before-turning, coordinate cache preservation,
trail fields, interruption refunds and forced disposal. Both colored-rice checks
also pass after sharing their scalar rotated-vector request setup.
Native keyboard checks cast, move and recover for both characters, with visible
speed trails. A temporary save supplies unlocked scrolls; equipment selection
and gameplay use the normal UI and input paths. Nix native, WASM and formatting
checks pass on x86_64-linux.

Protection charms now cast, return control and apply the existing melee damage
modifiers for 450 ticks, with torso particles and attached periodic pulses.
Interrupted casts drop a recoverable charm. Expiry and forced disposal clear
the owner's active-item flag unconditionally, preserving overlap behavior.
Both characters pass a complete cast/move/expiry check; an interrupted-cast
check covers the refund and cleanup of an unrelated active flag.
Seventy-two extracted-C callback cases match timer wrapping, pulse fields/RNG
and disposal. Native keyboard checks use unlocked temporary saves to equip,
cast and return to play with both characters, through expiry. Nix native, WASM
and formatting checks pass on x86_64-linux.

Decoy dolls now throw and roll, wait 90 ticks, grow over 16 ticks and create the
original standing/soft hitbox. At most three grow; additional dolls become
recoverable pickups. They periodically redirect closer non-boss targets and
restore those targets to the player on disposal, including suspended actors.
Retarget ticks consume and ignore contacts; three points of health break on
the fourth hit. Water disposal, knockback, smoke, sparks and original asset 27
use the shared source routines.
Four focused checks cover both characters' complete throw/growth, target
eligibility and restoration, contact ordering, bump thresholds/RNG, fourth-hit
break, overflow and water cleanup. The relocated arena fixture explicitly snaps
its camera to the new start. Native keyboard checks from normal mission starts
show both characters throwing and the original doll appearing with smoke.
The native checks use temporary saves to unlock equipment, with ordinary
launcher and keyboard paths afterward.
Nix native, WASM and formatting checks pass on x86_64-linux.

Disguise now swaps the original body meshes, short rest offsets and motion height,
while preserving player identity, collision definitions and coordinate caches.
It sets the original alert state and lasts 600 active ticks; damage, death,
specified attacks, replacement and forced disposal restore the original body.
Four focused checks cover both characters, full duration, crouch recovery,
interruption refunds, replacement, eviction and status boundaries. Ninety-six
extracted-C snapshot/apply cases cover short truncation and untouched cache,
parent and rotation fields. Medicine, charm, speed and disguise now share their
identical source-ordered interrupted-cast refund helper.
All 121 core checks pass, excluding the unchanged long cinematic coverage check.
The twelve relevant private desktop checks also pass, including all 66 starts,
display-cadence independence, GPU blends and the full Mission 1 route through
saved continuation. Native keyboard checks visibly transform and restore both
characters, using temporary saves to unlock the equipment and ordinary UI paths.
Nix native, WASM and formatting checks pass on x86_64-linux.

Ninja dogs now throw, retry occupied/invalid spawn points, create or reuse the
original dog actor, select enemies and attack through the existing AI and combat.
They retain the 1,800-tick lifetime, status/clock target gates, repeated idle
sheathing and disposal-before-refund allocation. Expiry drops collected inventory;
forced disposal parks the dog without dropping it. The original stage dog slot
reference survives layout replacement and is checked before summoning.
Four focused checks cover both characters' full lifetime/resummon and actual
attacks, plus controlled targeting, retries, refunds and cleanup. All 125 current
core checks pass, excluding the unchanged long isolated cinematic coverage check.
Native keyboard checks use temporary unlocked saves and show the original dog
appearing for both characters. Nix native, WASM and formatting checks pass on
x86_64-linux. Already-freed actor references remain a diagnostic boundary rather
than an attempt to reproduce a freed-memory read.

Mei-Oh's four lightning attacks now launch on frame 13, re-aim on the original
three-tick clock boundaries and retain their hitboxes between strikes. The
postdecrement lifetime includes the final zero-count callback. Bolts, sparks,
impact flashes, damage and local/player casting sound are implemented. Off-screen
segments consume the same RNG; visible lines contribute their source packet cost
independently of display rendering. Grappling cables and lightning share the
native screen-line geometry helper with separate colors/blends.
Ninety-six extracted-C segment cases verify coordinates/RNG with SDK square roots
supplied. Focused checks cover all four attacks against both characters, repeated
aiming, suspended-target inclusion, collision lifetime and forced cleanup. All
128 current core checks and twelve relevant private desktop checks pass, including
GPU blends, display cadence and Mission 1 through saved continuation. An offscreen
native GPU capture visually checks the additive bolt in a controlled effect
fixture; it is not a complete final-boss window playthrough. The temporary capture
module was removed. Nix native, WASM and formatting checks pass on x86_64-linux.

The original debug teleport item now follows the camera ray, applies the source
distance cutoff, preserves raw coordinate writes and emits its original particles.
It remains outside the normal equipment menu. Its camera ray uses a scalar port
of SDK VectorNormalSS, including short overflow and the zero-vector table path.
An instruction-level SDK oracle checks 1,031 vectors against the user's executable;
two item checks cover holding, release, camera ownership, range edges, failed rays
and forced disposal. All 131 current core checks and twelve relevant private
desktop checks pass, including Mission 1 through saved continuation. Nix native,
WASM and formatting checks pass on x86_64-linux.

The desktop host now accepts gamepads through pinned gilrs, with the original
face/shoulder button layout and digital D-pad/left-stick movement. Launcher,
equipment, results, pause and retry share their actions with keyboard input.
Stick hysteresis prevents threshold chatter; held input is suppressed across
menu/focus changes, and disconnecting the active controller pauses gameplay.
Three focused checks cover diagonal/threshold behavior, focus/menu edges,
controller ownership and independent keyboard releases. Twelve private desktop
checks pass, including the complete Mission 1 regression. Isolated native windows
verify keyboard menus, crouch/recovery, map, pause/resume and retry for both
characters; controller-aware help screens were visually inspected. No physical
controller was attached, so device/backend interaction is not yet manually
verified. Nix native, WASM and formatting checks pass on x86_64-linux; the Linux
package and development shell now include libudev.

World geometry now fades into black with the original camera-forward depth-cue
curve, sharing the scalar projection constants. The GPU evaluates a continuous
per-pixel curve; original polygon cue quantization/bank boundaries are outside
this presentation change. HUD/sprites and plain shadow meshes remain separate,
and the hook keeps its selected fog bank. No new lighting is applied to baked
vertex colors. The renderer now clears to the original black background.
An offscreen GPU check covers near/far colors, camera translation/rotation,
textured additive blending, explicitly unfogged meshes and screen graphics.
Both GPU checks, all thirteen relevant private desktop checks and the 67,584-case
integer projection oracle pass. Captures of both Mission 1 starts and 1,000 ticks
of its existing input route were visually inspected. Nix native, WASM and
formatting checks pass on x86_64-linux. Original sector/distance visibility and
the lit-tail omission in actor `0x8b`'s hand remain separate rendering work.

Native opaque and transparent mesh traversal now stops at the first lit primitive,
matching `DrawTMD` and the existing logical packet counter. This preserves ANI's
missing hand triangle instead of drawing it with an invented lighting model.
A private-disc regression checks the seven triangles and three quads preceding
that face, including the stop if another unlit face follows. The GPU blend/fog
checks and the core custom-versus-SDK traversal check pass. Nix native, WASM and
formatting checks pass on x86_64-linux.

Mission 2 now has complete ordinary-input routes for both characters. They cross
the town, swim the river, leave the water and trigger the authored delivery
cinematic/results with 100 HP. Rikimaru finishes in 2,457 ticks (game clock 1,860),
Ayame in 2,465 (clock 1,879). The tests carry only the default hook and perform no
runtime position, health, event or equipment overrides.
Rikimaru's existing Mission 1 regression now saves, reloads, runs the complete
Mission 2 route with the continued RNG, saves again and starts Mission 3 for
30 ordinary ticks. Ayame's separate Mission 2 test verifies results and saved
Mission 3 selection. These tapes are newly authored port inputs, not retail
state captures or a full-campaign parity claim. Offscreen native captures show
both characters swimming along the recorded route.
All fifteen relevant private desktop checks pass, along with Nix native, WASM
and formatting checks on x86_64-linux. Later complete mission playthroughs,
Ayame's Mission 1 and broader exploratory coverage remain outstanding.

An independent Rikimaru Mission 3 controller route now crosses the mine's gaps,
uses four shuriken against a guard, defeats Goo through melee and runs both
authored cinematics. It reaches results at tick 5,010 with 28 HP and game clock
3,279, then saves, reloads and starts Mission 4. Equipment is selected through
the ordinary menu from new-game stock (six shuriken, six caltrops, two medicine
and the automatic hook); no runtime state is overridden. The input tape was
authored in the port. Ayame's rescue route and broader exploratory play remain
outstanding.
All three private controller-route regressions pass. Native offscreen captures
of the pit crossing, boss fight and rescue dialogue were visually inspected.
Nix native, WASM and formatting checks pass on x86_64-linux.

The continuous Rikimaru test now completes Missions 1–3, encoding and decoding
the actual save at each boundary. A separate Mission 3 tape handles the continued
RNG and accumulated stock: five shuriken, six caltrops and one medicine, selected
through the ordinary equipment menu. It uses all five shuriken and finishes with
68 HP at tick 4,853, game clock 3,122; both cinematics and Goo's death are checked.
The resulting save starts Mission 4. The standalone Mission 3 route remains as
coverage of the new-game seed/loadout. All three private controller-route
regressions pass, including the full continuous campaign from fresh stock.
Nix native, WASM and formatting checks also pass on x86_64-linux.

An independent Rikimaru Mission 4 controller route now defeats both checkpoint
bosses, traverses the river road and runs sequences 10, 20, 40 and 50. It reaches
results at tick 7,961 with 84 HP and game clock 4,635, then saves, reloads and
starts Mission 5. The ordinary equipment menu supplies six shuriken, six caltrops
and two medicine from new-game stock; all six shuriken and both medicine are
used. No runtime state is overridden. Both bosses finish at zero health in
their authored death state. This is a standalone route; the continued campaign
with accumulated stock and RNG is still validated through Mission 3.
All four private controller-route regressions pass. Native offscreen captures
of both checkpoint boss fights were visually inspected.
Nix native, WASM and formatting checks pass on x86_64-linux.

Both Mission 5 character branches now have ordinary controller routes from
fresh seed/stock through results and a saved transition into Mission 6. Hikone's
authored health threshold triggers his retreat. Rikimaru then follows sequence
100 and finishes at tick 7,586 with 56 HP, game clock 3,480, carrying only the hook.
Ayame instead follows sequences 40 and 45, defeats Kataoka through five shuriken
and melee, and finishes at tick 7,618 with 80 HP, clock 3,747. She selects six
shuriken, six caltrops and two medicine through the equipment menu, using one
medicine. No runtime state is overridden. These standalone cases do not extend
the accumulated-stock campaign beyond Mission 3.
All five private controller-route checks pass, including both new branches.
Native offscreen captures of Rikimaru's final dialogue and Ayame's boss fight
were visually inspected. Controller-route tests now live in their own module,
separate from controlled transition and loadout tests.
Nix native, WASM and formatting checks pass on x86_64-linux.

The Mission 6 route exposed a mistranslation in ledge release: `HangCheck`
adjusts the actor's height, but `ActHANG` advances a separate local height by
100 for each release probe. Reusing the adjusted position could repeat the same
probe until the port's diagnostic limit. The scalar loop now preserves the local
counter, confirmed against the original executable's saved `s0` counter and
matching C. The same source review restored the missing disguise restriction on
ledge grabs. A 144-case comparison executes the original `HangCheck` and
`ActHANG` against a controlled two-level ledge, covering repeated release probes,
different body dimensions, beast exclusion and disguise. It passes; all 132 core
checks pass, excluding the unchanged long isolated cinematic coverage check.
Eighteen existing private desktop checks also pass.

A new Rikimaru Mission 6 controller regression grapples the statue, climbs and
jumps into the encounter trigger, defeats the cult leader and brings the second
opponent to the authored retreat threshold. It includes the guard release that
previously stopped at tick 4,290. All three cinematics run; results arrive at
tick 5,458 with 5 HP and game clock 3,187, followed by an encoded/decoded save
and Mission 7 startup. Equipment comes from the ordinary menu: six shuriken,
six caltrops and two medicine plus the hook. It uses all shuriken and no medicine.
This new private-disc regression passes without any runtime state overrides.
Native offscreen captures of the statue climb and second fight were visually
inspected. Nix native, WASM and formatting checks pass on x86_64-linux.

A separate Rikimaru Mission 7 route now crosses the port, boards the ship,
defeats Balma and runs sequences 10, 20 and 100. It reaches results at tick 3,309
with 100 HP and clock 2,678, then saves, reloads and starts Mission 8. It carries
six shuriken, six caltrops and two medicine selected from fresh stock, uses all
shuriken and one medicine, and picks up a dropped smoke bomb during the approach.
The private-disc controller regression passes on the corrected ledge runtime,
checks Balma's death state and requires no state overrides. A native offscreen
capture of the shipboard fight was visually inspected. This standalone case does
not extend the continuous saved campaign beyond Mission 3.
Nix native, WASM and formatting checks pass on x86_64-linux.

A standalone Rikimaru Mission 9 controller route climbs the castle stairs,
defeats Onikage and runs sequences 10 and 50 at ticks 1,936 and 4,255. It reaches
results at tick 4,535 with 53 HP and game clock 3,449, then encodes/decodes the save
and starts Mission 10. The normal equipment menu supplies six shuriken, six
caltrops and two medicine; one medicine is used and no shuriken remain. The
private-disc regression checks the boss's death state and saved transition with
no runtime state overrides. An offscreen native capture of the rooftop fight was
visually inspected. This does not extend the continuous saved campaign beyond
Mission 3.
Nix native, WASM and formatting checks pass on x86_64-linux.

The placeholder launcher now offers all four dialogue/subtitle languages without
reimplementing the original menu executable. A chosen language applies to both
new missions and Continue, is stored in the existing save format, and is restored
on launch unless an explicit `--language` option overrides it. The launcher labels
remain English and Japanese selection displays the outstanding font limitation.
A private-disc regression changes new-game and saved-game languages, loads the
corresponding mission scripts, checks persistence and explicit CLI precedence.
The native menu, Japanese notice and restored French preference were inspected
in an isolated X11 window with audio routed to a private null sink. Recreating or
decompiling the original main menu remains deferred, as requested.
The language-selection regression, all eight localized Mission 2 openings and
the existing campaign save/retry/continuation check pass. Nix native, WASM and
formatting checks pass on x86_64-linux.

A standalone Rikimaru Mission 8 controller route now crosses the mountain,
uses repeated grapples/jumps/ledge climbs, fights guards and reaches the cure.
Sequence 11 begins at tick 10,502; results arrive at tick 10,732 with 100 HP and
game clock 10,501. Six shuriken and two medicine are used from ordinary equipment
selection; six caltrops remain. The private-disc regression checks the final
position, health, inventory, result and encoded/decoded save into Mission 9.
No runtime positions, health or event state are overridden. An offscreen native
capture of the upper grapple was visually inspected. This standalone route does
not extend the accumulated-stock campaign beyond Mission 3.
Nix native, WASM and formatting checks pass on x86_64-linux.

A standalone Rikimaru Mission 10 controller route now defeats Onikage, traverses
the inner caverns and lava crossings, descends the authored pitfall and defeats
Mei-Oh. Sequences 7, 10, 15, 35 and 40 start at ticks 1,473, 1,563, 4,607, 9,092
and 9,773. Results arrive at tick 15,556 with 10 HP and game clock 12,135. The
ordinary equipment menu supplies six shuriken, six caltrops and two medicine;
all shuriken and medicine are used. Both bosses reach zero health and their
authored death state. No runtime state is overridden. The private-disc test
checks the result, encoded/decoded save, final completion flags, completion
screen, return to the launcher and saved Mission 10 replay selection.
Native offscreen captures of the inner area, pitfall entrance and Mei-Oh fight
were visually inspected. This finishes standalone route coverage of Rikimaru's
ten missions, but the continuous saved campaign is still verified through
Mission 3. Ayame's remaining routes, broader play and presentation work remain.
The original handoff to the external ending executable remains a placeholder
completion screen.
The final mission regression and Nix native, WASM and formatting checks pass on
x86_64-linux.


The continuous saved Rikimaru campaign now completes Mission 4 with its actual
accumulated stock and RNG. It carries two shuriken, six caltrops, three medicine
and four grenades through ordinary equipment selection. Both checkpoint bosses
die, sequences 10/20/40/50 finish, and results arrive at tick 7,742 with 16 HP
and game clock 4,416. Both shuriken, all four grenades and two medicine are used.
The test encodes/decodes the resulting save and starts Mission 5; no actor state,
event completion or RNG is overridden. The standalone fresh-stock route remains
as a separate regression. Later continuous missions and remaining Ayame routes
are still pending.
The full four-mission regression passes, including the Mission 5 launch.


Japanese subtitles now render with an embedded GNU Unifont JP replacement under
SIL OFL 1.1. The existing language selector no longer needs a font warning. The
original Shift-JIS detection, sixteen-character limit, fifteen-row texture strip,
outline, custom executable-supplied star and centered width-minus-one quad are
preserved. A Japanese caption contributes the original forty-byte textured quad
to logical drawing costs. Prepared textures survive repeated frames and clear
with the caption; host rendering still does not update gameplay.

The font is portable embedded data, with a pinned regeneration script, provenance
and license installed by the Nix package. No BIOS or system font is needed. All
Japanese captions across the 22 character/stage script files have glyphs. Both
characters' openings pass in all four languages, including subtitle presence and
return to background music. Native captures show Japanese text for both characters
and the original French/Italian text; caption changes and clearing were checked.
Menu labels remain English. Original menu-binary decompilation remains deferred.
Nix native tests/package, portable WASM and formatting checks pass on x86_64-linux.


Ayame's saved campaign now completes Missions 7 and 8. The ship route swims
past the armed quay, heals and defeats Balma with a grenade, smoke and melee.
Sequences 10/20/100 begin at ticks 1,655/1,745/3,052; results arrive at tick
3,292 with 93 HP and clock 2,701. One medicine, grenade and smoke bomb are used;
a smoke pickup replaces the consumed bomb. The mountain route fights the lower
guards and the archer covering the upper crossing, then climbs to the cure.
Sequence 11 starts at tick 5,891; results arrive at tick 6,096 with 75 HP and
clock 5,890. It uses one shuriken, one medicine, two grenades and three smoke
bombs. The full eight-mission save/reload regression passes and starts Mission 9
with carried stock and RNG; formatting passes. These routes require no runtime
changes or game-state overrides.

Ayame's saved castle continuation defeats Onikage with one mine and melee.
It jumps past a courtyard guard and climbs the castle; sequences 10/50 start
at ticks 1,830/4,624. Results arrive at tick 4,904 with 28 HP and clock 3,783.
The carried shuriken, six caltrops and three smoke bombs remain. The nine-mission
regression saves/reloads every boundary and starts Mission 10 with the original
stock/reward and RNG rules, without game-state overrides.

Ayame's saved final mission defeats Onikage and Mei-Oh, using two shuriken,
two grenades and one smoke bomb. The lava route crosses both bridges, smokes
the ghost at the upper crossing, grapples onto the shelf and uses a running
jump to reach the inner path. Sequences 7/10/15/35/40 start at ticks
1,460/1,550/5,156/9,456/10,157; results arrive at tick 17,130 with 4 HP and
clock 13,744. Six caltrops and two smoke bombs remain. The campaign regression
now covers all ten missions for each character, including encoded/decoded final
saves, completion flags, launcher return and saved final-mission selection.
Training, broader native play and presentation acceptance remain outstanding.


The continuous saved Rikimaru campaign now also completes Mission 5. Ordinary
movement and melee trigger Hikone's retreat and the return to the minister;
sequences 5/6/10/20/100 run. Only the automatic hook is carried. Results arrive at
tick 7,300 with 72 HP and game clock 3,194, then the save reloads into Mission 6.
The fresh-stock Rikimaru and Ayame Mission 5 routes remain separate regressions.
The full five-mission save/reload regression and both standalone Mission 5
branches pass.


The equipment screen now shows the original five-type limit alongside the
15-item count. The automatic hook occupies one type. The limit is shared with
the loadout rules; behavior is unchanged. The existing inventory regression
passes, and an isolated native window confirmed the empty and full-type displays
and rejection of a fifth selected equipment type.


The continuous saved Rikimaru campaign now completes Mission 6 with its actual
stock and RNG. It grapples the statue, defeats the cult leader and reaches
Onikage's authored retreat threshold. Sequences 20/110/150 start at ticks
1,238/3,285/5,409; results arrive at tick 5,819 with 12 HP and clock 3,548.
The normal equipment menu supplies one shuriken, two medicine, two grenades and
four smoke bombs plus the hook. The shuriken, grenades and two smoke bombs are
used; both medicine remain. No actor state, events or RNG are overridden.
The full six-mission regression passes, including encoded/decoded saves at
each boundary and startup of Mission 7. These routes establish specific
completion coverage; broader play and the remaining Ayame campaign still need
validation.


Native play now accepts `--record DIRECTORY`. Each mission attempt gets a new
input file containing its original launch options, initial RNG, selected gear
and tick-by-tick buttons. Random-layout requests are captured before construction
normalizes the selected layout, and armour/flute gear is captured before runtime
inventory conversion. Pausing or losing focus flushes pending inputs; menus and
pause time add no ticks. Retry and later missions create separate files, and
recording failures are reported without stopping gameplay.

The replay command recognizes the versioned header, loads the recorded state
and rejects conflicting launch options. Legacy input-only tapes keep their
diagnostic defaults. Tests reject malformed headers, preserve inputs across
buffer flushes, and compare complete existing state snapshots after replaying
seeded, random-layout, armoured, localized and training starts. All three checks
pass. An isolated native window recorded two Ayame attempts, verified no changes
during pause, then replayed both files: tick counts, health and motion match, and
the retry seed equals the first run's final RNG. The replay's rendered frame was
visually inspected. Audio was routed to a private null sink.
Nix native tests/package, portable WASM and formatting checks pass on x86_64-linux.


The continuous saved Rikimaru campaign now completes Mission 7. An ordinary
swimming route bypasses the armed quay, boards the ship and defeats Balma.
Sequences 10/20/100 begin at ticks 1,640/1,730/2,577; results arrive at tick 2,817
with 100 HP and clock 2,186. Equipment is selected from the actual saved stock:
two medicine, one grenade, two smoke bombs and four mines, plus the hook. One
medicine, the grenade and one smoke bomb are used; a smoke pickup is retained.
The full seven-mission regression passes, including saved continuation into
Mission 8. No runtime state is overridden.


The continuous saved Rikimaru campaign now completes Mission 8. Its carried gear
is one shuriken, two medicine, one grenade and three smoke bombs, plus the hook.
It fights the lower guards, grapples onto the mountain terraces and passes the
upper guards to reach the cure. Sequence 11 starts at tick 5,336; results arrive
at tick 5,566 with 50 HP and clock 5,335. One medicine remains. The regression
continues from the preceding seven missions, encodes/decodes each save and starts
Mission 9 without overriding gameplay state.
The full eight-mission regression and formatting check pass.


The saved Rikimaru campaign now completes Mission 9 with its remaining stock:
six caltrops, two medicine and six mines, plus the hook. Both medicine and four
mines are used; caltrops remain. The fight retains mine self-damage and finishes
Onikage before he can heal. Sequences 10/50 start at ticks 1,957/4,832; results
arrive at tick 5,112 with 22 HP and clock 4,026. The regression checks the boss's
death, accumulated completion flags and encoded/decoded continuation into
Mission 10. No gameplay state is overridden.
The full nine-mission regression and formatting check pass.


The saved Rikimaru campaign now completes all ten missions. Mission 10 selects
six caltrops, one medicine, one smoke bomb and two mines from the accumulated
stock, plus the hook. Ordinary controller input defeats Onikage, crosses the
lava, descends the pitfall and defeats Mei-Oh. The medicine, smoke and one mine
are used. Sequences 7/10/15/35/40 begin at ticks
1,523/1,613/4,216/9,201/9,882; results arrive at tick 14,348 with 30 HP and clock
10,927. The regression verifies both bosses' deaths, all ten mission flags,
the encoded/decoded final save, completion screen, launcher return and saved
Mission 10 replay selection. No gameplay state is overridden.
The complete ten-mission regression and independent final mission regression
pass in the release build; formatting passes. Remaining Ayame routes, training,
broader interactive play and presentation validation are still required. This
specific controller route is not a claim of complete-game or retail parity.


Ayame now completes Mission 1 from new-game stock through ordinary grappling,
movement and melee. Both bosses die; sequences 0/220/150/151 begin at ticks
1,486/3,146/4,573/5,031. Results arrive at tick 5,421 with 70 HP and clock 3,530.
All selected equipment remains. The saved continuation completes Mission 2
carrying only the hook: sequences 10/20, tick 2,464, 100 HP and clock 1,878.
Its path accounts for different enemy behavior from the standalone seed-1 run.
Both saves are encoded/decoded and Mission 3 starts with Ayame selected.
The two-mission Ayame regression, full saved Rikimaru campaign and formatting
checks pass. Remaining Ayame missions and broader interactive validation continue.


Ayame's saved campaign now also completes Missions 3 and 4. The mine route uses
four shuriken and melee to defeat Goo, then completes the rescue: sequences
30/40 at ticks 2,185/3,991, results at tick 5,471, 64 HP and clock 3,684.
The checkpoint route defeats both bosses and completes sequences 10/20/40/50
at ticks 1/1,775/4,623/6,836. Results arrive at tick 7,476 with 67 HP and clock
4,905. All three carried shuriken, three medicine and four grenades are used;
two of three smoke bombs remain. Guarding while smoke spreads allows the final
medicine to complete amid the surrounding enemies. No game state is overridden.
Encoded/decoded saves carry stock and RNG across all four missions and start
Mission 5 with Ayame selected. The Ayame and Rikimaru campaign regressions pass.


The saved Ayame campaign completes Mission 5's separate return-and-Kataoka branch.
Sequences 5/6/10/20/40/45 start at ticks 1/211/2,781/4,193/5,339/7,079;
results arrive at tick 7,469 with 27 HP and clock 3,598. The normal equipment
menu supplies six caltrops, two medicine, four smoke bombs and two mines, plus
the hook. The attempted medicine is interrupted before healing and the smoke
throw is refunded after interruption; melee finishes Kataoka. One medicine and
all other carried items remain. The full five-mission Ayame save/reload regression
passes and starts Mission 6 with carried stock and RNG.


Ayame's continuous saved campaign completes Mission 6. It grapples and climbs
the statue, defeats the cult leader and drives Onikage to his retreat threshold
with 94 HP remaining. Sequences 20/110/150 begin at ticks 1,181/2,949/4,627;
results arrive at tick 5,037 with 25 HP and clock 2,776. Ordinary selection supplies
two shuriken, one medicine, five smoke bombs and two mines, plus the hook; four
smoke bombs remain. The full six-mission regression verifies saved continuation
into Mission 7. The native window also continues from the actual Mission 6 save,
records movement/crouch and a separate grapple attempt, retains pause through
resize, and debits carried stock on retry. Its audio uses an isolated null sink.


Native window testing exposed missed keyboard taps shorter than a logic tick.
The host now retains new keyboard and controller presses until the next game
update, then consumes that pending state once. Held input remains held; menu,
pause, focus loss and disconnection discard pending input as appropriate. Game
logic and its 30 Hz cadence are unchanged. A regression covers taps between
updates, held input, keyboard aliases and host transitions. All four control
tests pass. An isolated native window records a one-tick jump and one-tick
attack from the same quick taps previously missed; both animations were visually
inspected. Pause/resize/retry and a subsequent grapple attempt also pass.
Nix native tests/package, portable WASM and formatting checks pass on x86_64-linux.

Training now has complete ordinary-input routes for both characters. Both defeat
all seven opponents, use two of the five training shuriken, traverse the jump,
ledge and grapple sections, and reach the instructor. Rikimaru finishes at tick
4,062 with 70 HP and clock 3,842; Ayame finishes at tick 3,724 with 90 HP and
clock 3,513. Both trigger opening sequence 20 and grade-zero closing sequence 5.
The regression verifies their saved ranking, unchanged campaign stock/progress,
Continue, and retry with the fixed hook and five shuriken. It passes in release.
An isolated native Ayame/Japanese training session exercises the first jump,
shuriken use, look controls, pause/resize and retry. Its two recordings replay to
matching ticks, health and motion; the retry seed matches the preceding RNG.
The recordings remain unchanged while paused. Audio uses a private null sink.
Native 4:3 inspection exposed clipped pause instructions, to be corrected next.

Pause controls now use two short lines so their meaning remains visible at 4:3.
The final completion notice is also shorter, and the training failure screen
correctly says its equipment resets. A rebuilt native window was inspected at
960×720 and 1280×720; all pause text fits. Formatting and the release build pass.


After correcting drawing initialization, refreshed ordinary-input recordings
complete Missions 1–3 for both characters and Mission 4 for Rikimaru. The
campaign tests verify all intervening results, boss deaths, inventory and
encoded/decoded saves before reaching the next stale routes. Mission 1 finishes
with 64/94 HP, Mission 2 with 100 HP each, Mission 3 with 7/46 HP, and Rikimaru's
checkpoint with 57 HP. The updated tapes carry their exact launch settings for
standalone replay. Full-chain tests remain failing at Ayame Mission 4 and
Rikimaru Mission 5; those inputs are still being refreshed.


The longer Castle-start comparison now passes 4,000 consecutive updates without
state refreshes, bringing the seven-recording total to 4,630. Player health falls
from 100 to 73 through actual enemy attacks. It revealed that copying a weapon's
hitbox also writes its radius into the collision count word (`offset.pad`),
until the next collision pass resets that word. The port now preserves that
source assignment. The weapon-clash regression also passes. The fixture's
initial ambient deadline is aligned to its recorded pre-loading clock 644;
shipping gameplay continues to initialize from the PC launcher's zero clock.
The Linux Nix package, native fixture-free tests, portable WASM build and
formatting checks pass after this correction. Other declared build platforms
remain unverified. The separate private full-campaign failures above remain
open; the Nix checks do not run those private-disc recordings.


The independent Rikimaru pirate-ship route is refreshed for the corrected
swimming camera. It runs all three cinematics, defeats Balma and reaches
results at tick 3,461, health 93 and clock 2,830. It consumes five shuriken and
one medicine and retains the smoke bomb picked up on the quay. A native GPU
capture at tick 1,003 was inspected on an isolated X server and confirms the
swimming view, water, actor and HUD. Audio uses a private silent sink.

Ayame's refreshed saved Mission 4 defeats both bosses, completes all four
cinematics and reaches results at tick 12,062, health 24 and clock 9,491. It
uses one shuriken and three medicine while retaining two smoke bombs and two
mines. The resulting encoded/decoded save carries seed 777,873,059 into Mission
5. Thirty-five desktop checks pass with private disc/audio/GPU fixtures,
excluding the two full campaigns whose later recordings still need refresh.

Rikimaru's refreshed Mission 5 carries six caltrops, two medicine, three smoke
bombs and two mines from the actual saved stock. It completes scenes 5, 6, 10,
20 and 100, reaches results at tick 8,034 with 100 HP and clock 3,928, and
saves seed 831,904,835 for Mission 6. Both medicine and one smoke bomb are used.

Ayame's refreshed Mission 5 also completes her additional Kataoka encounter,
reaching results at tick 7,919, health 46 and clock 4,048. Two smoke bombs are
used; six caltrops, one smoke bomb and two mines remain. Its actual save starts
Mission 6 with seed 2,861,527,239.

Refreshed saved campaigns now complete Missions 6 and 7 for both characters.
The cult leader dies and Onikage retreats through the original scenes. Rikimaru
finishes Mission 6 at tick 6,139, health 50 and clock 3,868; Ayame at tick 6,393,
health 5 and clock 4,132. Mission 7 defeats Balma and finishes with 100 HP each,
at ticks 3,744/2,546 and clocks 3,113/1,955. Actual encoded/decoded saves carry
seeds 1,341,414,700 and 176,276,537 into Mission 8. No runtime changes were
needed for these refreshed routes.

Both refreshed Mission 8 continuations reach the cure, play scene 11 and save
their results. Rikimaru finishes at tick 8,448, health 74 and clock 8,217; Ayame
at tick 8,126, health 56 and clock 7,920. The paths clear two interfering archers
and climb back through the mountain using the hook. Their remaining equipment
is carried into the actual Mission 9 saves.

Both saved Mission 9 continuations now complete the castle and Onikage fight.
Rikimaru finishes at tick 6,292 with 3 HP and clock 5,206; Ayame at tick 5,741
with 67 HP and clock 4,620. Saves carry seeds 37,031,008 and 4,197,738,911 into
Mission 10. The final mission recordings are the remaining campaign gate.


Both refreshed saved campaigns now complete all ten missions. Rikimaru's final
mission uses one medicine, one smoke bomb and one mine, finishing with 30 HP,
clock 11,661 and results at tick 15,082. Ayame uses two shuriken, one smoke bomb
and two mines, finishing with 7 HP, clock 12,629 and results at tick 16,015.
Both defeat Onikage and Mei-Oh, play scenes 7/10/15/35/40, save all ten mission
flags and pass the completion-screen/launcher/reload checks. The recordings
include their actual campaign seed and loadout for standalone replay. No
runtime adjustments were needed for these refreshed final-mission routes.
All 37 desktop tests pass with the private disc, audio and offscreen GPU
fixtures, including both complete campaigns and training routes.

Final x86_64-linux Nix packaging, native tests, WASM libraries and formatting
checks pass. The packaged executable verifies the real disc and opens the
completed Ayame save. Native input changes the language to Italian, continues
into Mission 10, moves/jumps, pauses, resizes to 4:3 and returns to retry
equipment. Its recording replays to tick 148, 100 HP and jump motion `0902`;
the saved language and RNG are retained. The equipment, language and pause
screens were visually inspected, as was a GPU capture of Ayame fighting Mei-Oh.
These checks used isolated X servers and private silent audio sinks.

The requested desktop MAIN.EXE scope is complete. The original main menu and
external ENDING.EXE movie, browser host and authored HD content
remain separate follow-ups. Source-level and continuous-retail comparisons
provide fidelity evidence, not exhaustive equivalence for every input sequence.

Optional desktop settings, native inspection and lighting/shadow improvements
are tracked in `CONFIGURATION.md`. Their factory defaults retain the original
presentation and simulation; the shared gameplay core remains portable.

The optional visual-upgrade deliverables are complete: editable PNG/OBJ packs
with live fallback/reload, isolated texture mipmaps and anisotropy, optional
texture reconstruction and character contours, recovered lantern emitters,
point/sky shadows, HDR bloom, contact shading, atmosphere/color controls,
Original/Enhanced/Cinematic presets and clean photo capture. Final evidence and
remaining visual limits are recorded in `CONFIGURATION.md`. The original
presentation remains the factory default; all 62 desktop checks and the native,
WASM and formatting Nix checks pass.

The texture/overlay/interpolation follow-up is also complete. Enhanced and
Cinematic now shadow all four active lamps and enable optional smooth model
motion. Original retains its previous rendering and exact 30 Hz poses. All 66
desktop checks pass, including both campaigns and full gameplay-state checks
across intermediate rendered poses for both players. Native X11 mouse/gameplay
checks and Wayland gameplay/overlay checks pass, including repeated Escape
exits. The final Nix package, native tests, shared WASM libraries and formatting checks pass. The
browser host, other build platforms and physical gamepads remain unverified.
