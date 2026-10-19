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

2026-10-20: foundation implemented. The actual mixed-mode CUE verifies the
SLPS-019.01 executable, indexes 1,048 files in DATA.VOL, lists all eleven stages
and reads Mission 1's STAGE.CON directly from the disc. Asset/core libraries
compile for WASM. Synthetic tests cover raw/cooked sectors, pregaps, mixed-mode
extents, archive parent indices, Shift-JIS names and malformed/truncated data.
The scheduler produces identical ticks across 24–240 Hz presentation and stalls.
Nix packages the native executable and checks native tests, WASM and formatting.

Scalar math, command history, RNG, animation playback and terrain probes are now
implemented. MAD/TMD, TIM/TPD, AMD, ACM, CON, layout and executable metadata
decoders construct all eleven stages for either character. A real-disc audit
samples every decoded motion track across its duration. The native wgpu renderer
produces a checked Mission 1 capture with original textures and posed Rikimaru.
The native movement preview now loads either player directly from the disc,
accepts keyboard input and supports pause/restart and resize. A 381-tick replay
exercises running, jump/landing, turning, crouching and looking for both characters.
Source terrain recovery and ledge control are implemented, but not yet validated
against continuous retail state. Smoke lifetime/allocation/RNG and animation
hold/sweep transitions have focused tests. The private-disc cadence test compares
every player/animation/effect/RNG state under 24 Hz and 144 Hz host frames.
An actual X11 window was driven through movement, jump, crouch, resize and pause;
the resulting original-asset scene was visually inspected.

The gameplay update is still player-only. Enemy activation/AI, combat, items,
events, HUD and audio remain to be joined. Lighting,
transparency and effect graphics also remain. Unsupported actions stop
with a diagnostic. This is not the Mission 1 acceptance milestone.

Source collision and persistent coordinate caches now run in the player update.
Contact iteration retains disabled and vacated slot data. Body resolution keeps
the original push/standing/underside reactions, including the world-Y-as-yaw bug.
Rendering consumes matrices prepared at the logical draw phase; it cannot mutate
the scene cache. Full actor/camera/effect visit ordering still needs validation.
The private cadence test now includes collision slots and coordinate caches.

The source follow/look camera now runs before player control, with original
terrain probes, smoothing history, head tracking and an integer SDK view matrix.
Its 512-case smoothing comparison agrees with matching C; its view matrix agrees
with a private retail capture. These are component checks, not continuous world
parity. Cinematic camera ownership and grapple targeting remain to be joined.

Next: actor activation/AI and combat. The original TAS architecture remains a
reference; there are no multi-world slabs or source-C runtime dependencies.

Weapon geometry and hitbox/attack definitions now load from the executable and
disc. Actors construct attachments and swap active/stowed slots according to
`SetupWeapon`/`EquipWeapon`. The live render includes held weapons; no attack
hitboxes or damage are enabled by this checkpoint. All-stage asset auditing and
the both-character cadence replay pass with these attachments.
