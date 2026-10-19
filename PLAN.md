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
- [x] Construct Mission 1 from disc assets and launch options, without captured
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

- [x] Execute Mission 1 CVA actor/camera/motion/text commands and return to gameplay.
  Latin subtitles and original localized dialogue play from the disc.
- [ ] Complete both boss encounters in ordinary interactive play.
- [x] Results/ranking, inventory rewards, retry and source training records.
- [x] Persist character, difficulty, inventory and mission progress to a save.
- [ ] Complete music, voices and necessary movie/audio decoding.
- [x] Minimal launcher for new game, continue, character and difficulty; original
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
player movement/jump/crouch/ledge/swimming/wall control, persistent coordinate caches, ordered
body collision, camera follow/look, original animation, weapon attachment/swap,
patrol/investigation/combat decisions, reinforcement replacement, melee combos,
weapon hitboxes/clashes, guarding, damage reactions, authored stealth deaths,
dropped inventory, pickups, arrow/gun/shuriken attacks, player aiming/cancellation,
medicine, caltrops, grappling, smoke bombs, grenades, mines, poisoned food and item damage. The native
renderer displays original textured scenery, visible actors, held weapons,
shadows, blood pools, weapon trails and the implemented particle effects.
Unsupported actions still pause with a diagnostic. Remaining item use, special
ranged attacks and continuous mission validation remain unfinished;
this is not a completed Mission 1.

Enemy activation retains the source's 30-tick sampling delay, ordered budget,
wakeup guards and raw parking-coordinate writes. Integer geometry counters use
original meshes, construction cell/depth ordering, adaptive subdivision, draw
banks, shadows and implemented effects. They do not measure host performance,
drop frames or change with render resolution. The sample remains incomplete
until remaining effects and items are implemented;
activation is therefore not yet certified against continuous retail gameplay.

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
launcher supplies the original fallback loadout (hook, 10 shuriken, 5 caltrops,
2 medicine). Shuriken aiming/release/cancellation, medicine and caltrops now run
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
Remaining ranged/item combat and continuous campaign validation are still required.

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
