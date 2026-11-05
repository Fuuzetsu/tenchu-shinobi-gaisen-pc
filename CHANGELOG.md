# Changelog

User-visible changes are collected under **Unreleased** until a version is
tagged. Internal refactors, formatting and test-only changes are omitted.

## Unreleased

- Add optional enemy damage escalation under F1 → Simulation. A guard’s first direct sighting does normal damage; repeated encounters after fully calming down add a configurable bonus, with a configurable cap. Shared alarms and brief sight loss do not raise damage. Disabled by default.

- Show wind-driven rain and snow outside the item-shop window when **Shop window and lighting** is enabled. It follows the weather controls, including light original snowfall on snowy missions; Clear removes it. The shop demo now includes weather as well as the day/night lighting cycle.

- Add an optional English **Shuriken** shop title under F1 → General, matching the throwing-star icon. The disc's Crimson Blade title remains the default; the description still comes from the disc or texture pack.

- Add weapon-blood coverage and flow controls under F1 → Blood. Wet streaks follow gravity and drip from the lowest edge; dry traces remain and swings still shed blood. Modern presets enable the effect, while saved custom settings keep their previous coatings.

- Add adjustable cosmetic wound spurts under F1 → Blood, with jets that follow animated wounds and taper over the bleeding duration. Modern High/Ultra enable them; a sparring demo compares them with drips. Existing custom settings and Original stay unchanged.

- Add selectable rolling thunder with layered rumbles, stereo reverberation and overlapping tails, plus an optional retro 8-bit/11 kHz sound. Modern presets and storm demos use Rolling; the previous sound remains available.

- Add **Blizzard** weather: dense wind-driven snow with adjustable howling-wind ambience and no lightning/thunder. Snowstorm remains available separately. Includes a Blizzard feature demo.

- Add continuous stereo rain ambience with its own F1 weather volume. It follows rain intensity, muffles smoothly under cover, pauses with the game and is included in recordings. Modern presets enable it when Rain or Rainstorm is selected; Original stays unchanged.

- Add optional **Strict corpse visibility** under F1 → Gameplay. Guards must have a clear eye-to-body terrain ray before discovering a corpse; visible bodies still raise shared alerts. Original behavior remains the default.
- Update the mid-mission savestate format for the new gameplay options, per-guard encounter memory and wound/weapon-flow data. Older snapshots require the build that created them; campaign saves are unchanged.

- Let tinted and non-metallic weapons receive blood when no silver blade surface is identified, fixing the Mission 9 Kunoichi's clean weapons after successful hits.

- Keep the selected savestate slot when rewinding across model/scene rebuilds or restoring a state.

- Make packaged README and build information writable by their owner, avoiding permission errors when updating an extracted installation.

- Add an optional persistent-blood surface cache under F1 → Rendering trials. Dense modern stains on flat terrain reuse their coverage while lighting and weather remain live; other surfaces and blood styles retain their existing renderer.

- Add an opt-in adaptive shadow renderer: smaller distant light maps, cheaper soft filtering and staggered foliage updates, while keeping lamps lit and character shadows live.

- Add an optional half-resolution ambient-occlusion renderer under F1 → Rendering trials, with depth-aware reconstruction at full display resolution.


- Keep shared terrain lighting when rain reflections are disabled, and use it for accumulated snow to reduce rendering cost.


## 0.15.6 — 2026-11-05

- Avoid repeating GPU character poses on the CPU solely to calculate shadow
  bounds. Conservative bounds preserve moving characters and their shadows;
  custom skins and WebGL2 retain CPU transforms.

- Improve animated-character frame times on native and WebGPU by caching model
  geometry and transferring part transforms instead of complete posed meshes.
  Model detail, secondary motion and the WebGL2 fallback are preserved.

- Further improve soft-shadow frame times in reflective rain scenes by reusing
  depth summaries for smaller groups of samples, preserving shadow detail.

- Reduce water rendering cost in reflective rain scenes by reusing the existing
  material and lighting passes, preserving waves, foam and sky reflections.

- Speed up animated vegetation shadows by sampling compact opacity data instead
  of full-color replacement textures. Cutout shapes and lantern transmission
  remain unchanged.

- Reduce soft-shadow filtering work in reflective rain scenes using conservative
  GPU depth summaries. Shadow resolution, samples and appearance are unchanged;
  other graphics configurations keep their existing rendering.

- Improve frame times for reflective wet terrain with soft shadows by sharing
  lighting between surface shading and rain reflections. The optimisation uses
  HDR without edge anti-aliasing on native/WebGPU backends; other configurations
  retain the existing rendering path.

- Small soft-shadow rendering performance improvements with weather enabled;
  shadow resolution and sample count are unchanged.

- Reduce GPU load from overlapping modern blood stains, especially with rain or
  snow, by sharing shading on the same flat surface. Sparse stains, slopes,
  foliage and mixed surfaces retain their existing rendering. This optimisation
  uses HDR with edge anti-aliasing disabled and a compatible graphics backend.
  Retained stains now blend in stable creation order.

- Update the mid-mission savestate format for retained blood surface data.
  Older snapshots require the build that created them; campaign saves are unchanged.

- Exclude character-only meshes from rain/snow shelter updates so they do not
  invalidate cached shelter. Moving doors and other scenery still update it.

- Improve frame times around animated textures by preserving cached shadows
  when only water or other noncaster artwork changes. Animated cutouts, lantern
  transmission and replacement-artwork changes still refresh their shadows.

- Reduce shadow-map GPU cost around buildings and trees by drawing solid scenery
  before foliage and nearer casters first on large maps, preserving animated
  leaves and their shadows.

- Reduce GPU work for opaque HD scenery with nearest or bilinear texture
  filtering, skipping unnecessary texture-relief calculations in the depth pass.

- With **Keep all enemies active**, stealth kills no longer make distant guards
  turn toward the player or wake sleeping guards across the map. Nearby guards
  retain the original game's reactions.

- Reduce shadow-bound preparation in crowded scenes by scanning shared model
  vertices once per actor, preserving all geometry and shadow casters.

- Reduce animated-geometry CPU copies by repacking material attributes only
  when they change. Positions, normals and changed materials still update each
  frame as required.

- Reduce CPU work for blood and lantern lighting in crowded scenes by requesting
  poses only for affected or light-emitting model parts. Attached stains, wounds,
  bloody weapons and moving lights keep their existing behavior.

- Reduce model-interpolation CPU work in crowded scenes by matching ordered
  pose snapshots in one pass. Poses, camera cuts and saved states are unchanged.

- Reduce GPU work for overlapping blood stains on snow by reusing conservative
  shelter-height bounds. Roof edges, sloped surfaces and moving blockers retain
  the existing shelter filtering and appearance.

- Reduce rewind recording spikes when new blood pages and footprints are added,
  by retaining encoded data for unchanged effects.

- Reduce frame-time spikes and CPU work with many snow footprints by assembling
  only tracks inside the camera view and avoiding repeated geometry allocations.
  Distant tracks, wind movement, fading and rewind history remain intact.

- Reduce GPU work for bump-mapped surfaces by omitting unused parallax code,
  particularly when rain and many blood stains overlap. Surface-depth modes
  retain their existing appearance.

- Speed up refined model transforms with portable vector math, reducing actor
  geometry CPU work while preserving poses, normals and secondary motion.

- Reduce moving-light selection work by rejecting lights that cannot enter a
  region's configured light budget. Light choices and tie ordering are unchanged.

- Reduce rain and snow geometry CPU work by caching particle depth for sorting
  and skipping fully faded streaks before constructing them. Particle counts,
  motion, visibility and blending order are unchanged.

- Stop weather shelter queries at the first blocking surface, reducing CPU work
  especially when wind redirects existing snow. Rain impacts, roof runoff and
  shelter coverage retain their existing behavior.

- Reduce CPU work when creating blood stains, death pools and snow footprints
  by preparing each mark once and skipping unused opacity calculations. Shapes,
  surface clipping, textures and retained-effect limits are unchanged.

- Reduce CPU allocation and texture uploads when fire, sparks and other lights
  move. Unchanged light-table rows are reused without changing light selection,
  brightness or shadow settings.

- Reduce rewind recording work by sharing unchanged animation and weapon-model
  resources between captures. Edited wall poses, imported movesets and weapon
  geometry remain part of the saved state.

- Reduce CPU work in visibility and logical drawing calculations by building
  the fixed reciprocal lookup table at compile time. Original projection
  rounding, clipping and gameplay behavior are unchanged.

- Speed up terrain queries used by movement, AI and effects, especially with
  **Keep all enemies active** and crowded levels. Terrain edits and original
  collision behavior are preserved.

- Reduce GPU shading work in rain and snow by sharing daylight-shadow checks
  on opaque surfaces and simplifying surface-variation interpolation. Retain
  the existing shadow-sampling paths for dry scenes and blood.

- Reduce CPU work for camera and shadow visibility checks, reusing moving
  casters' visibility when determining which shadow-map regions need refreshing.
  Shadow coverage, wind animation and rendering quality are unchanged.

## 0.15.5 — 2026-11-03

- Fix guards repeatedly drawing and lowering their weapons at the alarm-range
  boundary with **Keep all enemies active** enabled. Guards remember an alarm
  they have heard until it ends; distant patrols still ignore unheard alarms.

- Allow validated unfinished PS1 replay scenarios to be recorded with
  `prepare-retail --scenario`. `record-retail` now accepts explicit
  `TENCHU_SETTINGS` presentation settings for comparisons.

## 0.15.4 — 2026-11-03

- Improve rendering performance without reducing visual quality, light or shadow
  counts, retained effects, or gameplay activity. Four moving 4K Modern High
  benchmarks with HD artwork on a GTX 1080 Ti / Ryzen 9 9950X3D improved from
  45.6–54.6 FPS (31.8 FPS in the garden) to 62.8–92.0 FPS. Heavy rain and extreme
  blood accumulation can still fall below 60 FPS. See the
  [measurements and limits](https://github.com/Fuuzetsu/tenchu-shinobi-gaisen-pc/blob/v0.15.4/docs/performance.md#november-optimisation-loop-cumulative-result).

- Reduce the cost of terrain lighting, soft shadows, accumulated snow, rain
  reflections and overlapping blood. Avoid expensive shading of hidden scenery
  and reuse visibility checks for unchanged scenery and lights. Performance
  charts now show the scenery visibility pass separately.

- Reduce CPU work and GPU transfers for refined/replacement character models,
  animated blood and dense effects. Reuse buffers and unchanged model data,
  avoid large repeated copies, and skip redundant particle drawing passes.

- Speed up wind-blown rain and snow terrain checks. Avoid building and uploading
  off-screen precipitation, leaves, petals, contact particles, splash rings and
  body stains, while keeping their simulation, accumulation and lights active.

- Keep rewind responsive with many blood stains, snow tracks and wet splash
  marks by sharing unchanged geometry between snapshots. Avoid recording spikes
  when old blood expires, and count shared storage once toward the history budget.
  The first capture of a large existing collection still takes additional time.

- Reduce the CPU cost of projecting new blood stains, death pools and footprints
  onto scenery, preserving their shapes, overlap order, textures and fading.

## 0.15.3 — 2026-11-02

- Diagnostic reports now include the active controller, driver-reported vibration
  support, mapping and recent vibration errors to help investigate controller
  problems across platforms.

- Keep distant lamps lit as the player and camera move. **Lights per region**
  now limits overlapping lights locally instead of switching all but the nearest
  lamps off across the map. Cached world light lists bound shading work; the
  separate shadow budget still prioritizes nearby sources with gradual handovers.

- Active enemies now leave snow footprints regardless of camera distance,
  including distant patrols with all enemies active and fog disabled. Applies
  to accumulated snow and tracks on the original snowy terrain.

## 0.15.2 — 2026-11-02

- Bloody weapons now wait for actual blood contact during stealth kills instead
  of following earlier vibration cues. Rikimaru's neck break keeps clean blades;
  delayed stab animations no longer shed weapon droplets before drawing blood.

- Restore modern wound stains and dripping after stealth kills. They now start
  with the victim's blood spray even when the kill reduced health earlier in the
  animation, and retain the configured bleeding duration.

## 0.15.1 — 2026-11-02

- Enable system clipboard copy, cut and paste in desktop text fields, including
  model folders, searches and shared randomizer settings.

- Keep **Follow Actor** attached to the selected actor after the player dies.
  Following and orbiting also work while waiting on a frozen death scene;
  player-control camera restrictions no longer interrupt the debug camera.

## 0.15.0 — 2026-11-02

- Add optional **Call aid after witnessed kills** under F1 → Simulation.
  With all enemies active, the nearest eligible guard who sees a non-stealth
  guard kill retreats to seek reinforcements. It respects sight, cover, call
  counts and population limits; summoned guards cannot call again (#33).

- Rework high-detail hero smoothing to round silhouettes without puffing out
  faces, limbs and clothing. Preserve limb lengths and open seams so separate
  animated parts stay joined. Applies to Rikimaru and both Ayame outfits with
  the existing HD pack.

- Choose a moveset independently of the player model in F1 → Player model.
  Ayame and Rikimaru can keep their normal appearance while using any available
  actor's moves, with a searchable source selector. Existing model-following,
  player-movement and idle-taunt options remain available (#46).

- With all enemies active, reject call-aid spawn locations outside the map or
  without a valid floor. Missing terrain no longer qualifies as hidden ground;
  cover is checked at the new guard's floor height. Failed attempts preserve the
  caller and its ability to recruit later.

## 0.14.0 — 2026-11-01

- Add a rewind timeline with backward/forward seeking, single-frame steps and
  jumps to the oldest/latest recorded state. The timeline appears while rewinding
  and under F1 → Simulation. Seeking keeps the complete buffer; resuming play
  discards only the recorded future after the selected point.

## 0.13.1 — 2026-11-01

- Fix ground-item icons trembling as the gameplay camera moves. Pickups now use
  the displayed camera's subpixel projection, including model interpolation,
  while retaining their original depth offset, size and gameplay behavior (#47).

## 0.13.0 — 2026-11-01

- Substantially reduce rewind recording overhead by sharing unchanged mission
  assets and blood geometry, copying texture words in bulk and reusing buffers.
  Backward playback no longer rebuilds periodic keyframes. Keep every logic update
  and all visual effects, with bounded history and unchanged savestate files.
  The performance chart and diagnostic reports now show rewind CPU time separately.

- Custom player-model folders support optional four-joint skin weights for
  smooth bending, including interpolated poses, shadows and gallery previews.
  Asset-pack PNGs can be up to 2048 pixels per side, within the existing decoded
  image budget and the GPU's atlas limits.

## 0.12.0 — 2026-11-01

- Add optional high-detail hero geometry with rounded surfaces and finger
  creases for Rikimaru and both Ayame outfits, using the existing HD artwork.
  Modern High/Ultra enable it; original rigs, motions and gameplay are preserved.
  Includes three hero comparison demos and auto-optimisation support.

- Add optional blood coatings to player/enemy blades, with small tip drips and
  stronger shedding on swings, plus adjustable sky/light reflections on weapon
  metal. Modern High/Ultra enable both; Original stays unchanged. Includes a
  weapon-finishes demo, savestate/rewind support and bounded effect storage.

- Play rewind continuously backward, with configurable 0.25×–8× speed (default
  2×), including model interpolation, hair/cloth motion and accumulated effects.
  Ctrl+F12 starts/stops; F3 resumes forward. Unchanged scene resources stay cached;
  per-update compressed history remains optional and bounded to 96 MiB.

- Synchronize alternate player death pools and blood-impact sounds with the
  visible animation. Each variant uses its own authored ground movement and
  stops after settling; gameplay timing, other actors' sounds and Original/Retro
  presentation remain unchanged.

- Make flying grapple/weapon sparks and burning characters illuminate their
  surroundings. Replace the soft yellow weapon-impact orb with a short, sharp
  burst. Burning bodies no longer block all of their own flame light; walls
  still cast shadows. Uses the existing spark and fire-light options.

- Fix HIME/Kiku using unrelated terrain textures in cross-stage model swaps and
  imports. The model library now includes Ayame-specific story resources.

## 0.11.0 — 2026-11-01

- Fix small camera shakes caused by rounding interpolated views far from the
  map origin. Orbit/follow cameras now match the model cadence when interpolation
  is disabled, including Retro mode.

- Extend **F1 → Contact effects → Modern impact sparks** to weapon blocks,
  clashes and metal impacts, with smooth hot trails and short block/clash lights.
  Modern presets enable it; spark amount remains adjustable. Add a weapon-sparks
  comparison to the demo library.

- Add optional **F1 → Simulation → Rewind history**, up to 30 seconds in
  one-second steps. **Ctrl+F12** shows the previous checkpoint and pauses;
  **F3** resumes from there, replacing the abandoned future. Restores gameplay,
  RNG, live items, blood and weather effects. Off by default, with bounded memory;
  retries, mission changes and file loads clear the history.

- Add five persistent mid-mission savestate slots under **F1 → Simulation**.
  F11 saves and F12 loads; loading pauses until F3. Restore enemies, RNG, items,
  cutscenes and accumulated visual effects, including randomized and edited
  missions. Slots survive restarts, stay separate from campaign saves and check
  the game version and file integrity. Browser slots persist in site storage.

- Add **F1 → Teleport / actor → Place / edit mission actors**: search all disc
  models, place extra actors with their own AI, motions and voices, or move/remove
  ordinary actors. Pick a floor point or enter coordinates and facing. Edits last
  until retry; story actors remain protected and TRIAL records are disabled.

- Add **F1 → Camera → Follow selected actor**. Track another actor with a
  wall-aware orbit camera while retaining player control; right-drag/right stick
  rotates, middle click/R3 recenters and F2 returns. Aiming/cutscenes retain their
  cameras, and following never changes enemy activation or AI.

## 0.10.0 — 2026-11-01

- Make the shared training input tapes directly replayable with their correct
  character, seed and equipment; document input timing and button masks for
  other hosts, and identify which public examples currently complete.

- Add optional **Quick shuriken evasion** under F1 → Simulation. Armed enemies
  can sidestep a visible shuriken aim promptly using their own dodge animations,
  with a one-second cooldown. Off by default; existing attacks, damage reactions
  and scripted scenes remain uninterrupted.

- Add **Rainstorm / Snowstorm** under F1 → Weather: denser precipitation,
  sheltered cloud flashes and delayed thunder. Lightning strength, interval and
  thunder volume are adjustable; `rainstorm` and `snowstorm` demos include audio
  when recording with `--sound`. Gameplay and its RNG remain unchanged.

- Add **Tracks on existing snow** in F1 → Weather for Missions 5/10 and TRIAL
  terrain using their snowy materials. It works without extra snow or weather
  overrides, respects bare parts of each texture, and is enabled in modern presets.

- Fix camera-relative directional attacks turning the player before choosing
  the slash. Document the guard/side-slash modifier in Movement and Controls:
  hold Left Ctrl or R1/RB, then back to guard or left/right + attack to slash.

- Add **Multi-target player attacks** under F1 → Simulation. Each melee weapon
  can hit several enemies once per swing, with normal damage, blocking and wall
  checks. Off by default; items and enemy attacks retain their original rules.

## 0.9.10 — 2026-11-01

- Share all four ordinary player/ninja death motions between Ayame and Rikimaru,
  including each other's original collapse. **Alternate player deaths** remains
  optional and does not change gameplay or retry timing.
- Make death blood spurts follow the chosen animation's torso and duration,
  including their flight, ground stains and original/HD/modern appearances.
  Short collapses no longer keep spraying from the original standing pose.

## 0.9.9 — 2026-11-01

- **Alternate player deaths** now chooses among all three ninja death motions,
  excluding the hero's existing animation. Completed alternate corpses stop
  drifting with the original animation, and longer variants play to their end.
  Failure timing and game RNG remain unchanged.
- Make the settled player corpse non-solid during **Keep world running after
  death**, so guards cannot push or turn it. Player blood-pool settings still
  control its pool independently of enemy corpses.

## 0.9.8 — 2026-11-01

- Keep mission music playing while watching after death, and restore it after a
  scripted death cutscene. Boss failures now retain the complete scene and let
  **Keep world running after death** continue updating it, instead of losing
  particles and freezing after the cutscene. Confirmation and retries retain
  their original behavior.

- Add **F1 → Player model → Alternate player deaths**, a configurable chance of
  using the male/female ninja collapse for Rikimaru/Ayame. Defaults to 0%; try
  50% for variety. Visual only, with weapons and blood pools following the pose;
  original death timing, game RNG and retries are preserved. Special deaths and
  swapped movesets retain their animations.

## 0.9.7 — 2026-11-01

- Add **F1 → General → Keep world running after death**. After the campaign
  death scene, guards return to patrol and effects keep running until you confirm.
  Optional and off by default; the frozen view remains available. Watching longer
  does not change retry RNG, inventory or progress.

## 0.9.6 — 2026-11-01

- Add **F1 → Simulation → Independent enemy attacks**: enemies use individual
  attack cooldowns so several guards can press their attacks together. Disabled
  by default; retains individual attack speed and works with either activation
  mode. This gameplay option ends input recording and disables TRIAL records.

## 0.9.5 — 2026-11-01

- Stop recursive reinforcement waves with **Keep all enemies active**: each placed
  guard can call once per alert, and summoned guards cannot recruit more guards.
  F1 → Simulation adds a fixed recruits-per-call count (default 1, 0 disables)
  and a random 1–N option. Group calls respect the optional population limit;
  original activation/AI rules remain unchanged.

- Blend rain wetness and puddles beneath roof edges, and smooth rain haze when
  looking between exposed and sheltered areas. Enclosed interiors stay dry and
  clear; the same shelter handling applies to campaign and TRIAL maps.

- Let snow settle mainly on upward-facing surfaces. Windward walls receive a
  light, uneven dusting instead of a broad white coat; sheltered faces and
  undersides remain clear.

## 0.9.4 — 2026-11-01

- Add **Original HD** blood under F1 → Persistent effects: smoother retail spatter,
  stars and patterned pools, independently selectable for hits, drips, death pools
  and ground spots. Uses the user's disc and cached textures, needs no HD pack,
  and preserves gameplay and the existing Original/Modern choices.

## 0.9.3 — 2026-10-31

- Link the separate HD artwork download and installation/update guide from
  **F1 → Visual assets**, with clearer instructions when no pack is loaded.

- Add **F1 → General → Circle confirms in menus** to choose Japanese or Western
  controller confirmation, with matching button prompts. Menu buttons remain
  independent of gameplay rebinding; Enter/Esc and START-to-retry are unchanged.

- Fix white outlines around original blood sprites with MSAA, including filtered
  textures and free-camera views.

- F1 → General adds **Music while paused** for pausing and frame stepping during
  missions. Gameplay sounds, frontend movies and narration still pause normally.

- Restore the distinct retail pause and resume sounds. The pause chime finishes
  while gameplay audio is suspended; frame stepping does not repeat it.

- START on the mission results screen retries the current mission, matching
  retail, instead of advancing. Space does the same on the keyboard.

- Restore the retail character-selection sounds in Options → Records. Pressing
  toward the already selected character stays silent.

## 0.9.2 — 2026-10-31

- Reduce shadow-map work during active gameplay, especially in crowded TRIAL
  missions: keep distant casters in separate batches and refresh only the regions
  touched by moving characters or wind. Shadow detail and animation are unchanged.

- Prevent an intermittent browser crash when changing TRIAL missions or editor
  views by retaining interface textures until their GPU work has been submitted.

- Reduce CPU overhead from performance charts and refined character meshes,
  preserving all chart samples, animation detail and shadow geometry.

- Give Linux playback a stable “Tenchu: Shinobi Gaisen” name in PulseAudio/PipeWire
  mixers and let the server restore the game’s volume, mute and output device
  across launches.

- Expand performance charts with GPU timings for terrain/static props, characters,
  blood decals, particles, sky and MSAA resolve work. Detailed timings are also
  included in diagnostic reports and benchmark exports on supported adapters.

- Reduce GPU work in heavily lit scenes, including TRIAL's second built-in mission,
  while preserving shadow resolution, light counts and rendered detail.
- Reduce contact-shading cost with MSAA and redundant HD texture reads without
  changing the image or graphics settings.
- Speed up refined character animation and mesh bounds preparation, including
  scenes with many active enemies.
- Skip point-light shadow-map faces that cannot affect the current view, while
  retaining off-camera objects that cast visible shadows.
- Reuse unchanged shadow maps during play and camera movement. Moving casters,
  animated foliage, light changes and texture updates refresh affected maps
  without reducing shadow quality.
- Reduce terrain shading cost by reusing texture and height-map lookups within
  filtering loops and specializing shaders for the selected filtering modes.
  Texture detail, lighting and graphics settings remain unchanged.
- Speed up texture and height-map preparation during loading and texture updates,
  preserving mip coverage, transparency and surface detail.

## 0.9.1 — not published

Superseded by 0.9.2 before publication after a release-automation date error.

## 0.9.0 — 2026-10-31

- HD artwork v3 adds terrain and ornament replacements for all 12 TRIAL themes
  and all 16 built-in missions. TRIAL lanterns, braziers and cave lights now emit
  dynamic light and cast shadows; mine carts no longer count as lanterns.
- Add optional blood runoff down walls and slopes under F1 → Persistent effects,
  enabled by Modern presets. Trails follow surfaces, stop at edges or level ground,
  and use the selected retained blood style without changing gameplay.
- F1 → Movement adds **Start aiming toward camera** for camera-relative controls.
  Grapple, item aim and look mode turn the player toward the view and begin at
  its pitch; ongoing aim and classic controls keep their existing behavior.
- Display Japanese mission names and text throughout the TRIAL editor, including
  editable fields and records. Free-camera keys no longer trigger paused gameplay
  menu bindings, so Shift boosts 3D preview movement without leaving the preview.

## 0.8.1 — 2026-10-31

- Fix thin white borders on rotated original blood sprites, including when texture
  filtering is enabled. Extra death pools now wait for the finished death pose and
  landing; F1 → Persistent effects adds a 10–200% size control for original stars
  and modern pools beneath both players and enemies.
- Keep the final complete scene, including effects and formed blood pools, while
  waiting after death. Ignored navigation keys no longer play hidden menu sounds;
  confirming still finishes the retail failure sequence and preserves the retry seed.
- With all enemies active, reinforcements require terrain cover from the player
  instead of relying on distance alone. F1 → Simulation adds a configurable
  reinforcement population limit, defaulting to Unlimited in all-active mode;
  queued calls respect a chosen limit. Larger crowds no longer exhaust the old
  actor/collision pools or fail during weapon clashes. Original enemy settings
  keep the retail rules.

## 0.8.0 — 2026-10-31

- Port TRIAL.EXE: play all 16 original challenges as either ninja, or build custom
  missions across 12 themes with terrain, ornaments, traps, characters and patrols.
  Five objectives, optional stealth constraints, original time bonuses/penalties,
  result scenes and best-time records use the shared gameplay engine.
- Open the mission editor from the title or F10 in menus. Mouse painting, undo,
  3D preview and immediate playtesting work on desktop and in the browser. Save
  missions separately from campaign progress; import/export .msn files or share
  codes. Trial art, geometry, missions and sound come from the user's verified disc.

## 0.7.0 — 2026-10-31

- F1 → General can relight the item shop: the window follows the game's sky and
  fog colors, and directional light projects soft window shadows into the painted
  room. Modern presets enable it; Original retains the untouched backdrop. A new
  `shop-lighting` demo shows the day/night changes.

- Keep the frontend Randomizer shortcut on one line instead of wrapping it into
  a narrow vertical column.

- Independently choose Off, Original or Modern blood for hit spray, wound drips,
  death stars/pools and ground spots. Original splats and death stars can remain
  persistently; player death pools and body stains have separate toggles. Existing
  blood preferences migrate automatically, and changing one category preserves
  the others' stains. Original blood uses disc artwork even with the HD pack enabled.

- F1 → General can hold the finished death scene until a new button press before
  opening mission failure. The default retains retail's automatic transition.

- Match retail menu music: the shop and main options screen are quiet. Volume
  previews the last sound-test selection (initially character-selection music),
  and leaving Volume, Sound Test or Records stops their music.

- Experimental camera-relative controls, available in F1 → Movement: continuous
  left-stick direction, camera-relative keyboard/D-pad movement, right-stick or
  held-right-mouse orbit, adjustable sensitivity/deadzone/distance and camera
  recentering. The follow camera avoids walls; classic controls remain the default.

## 0.6.0 — 2026-10-30

- Modern presets enable soft blood spray and persistent stains, with much higher
  retention defaults and a configurable maximum of 65,535 stains. Cached ground
  stains and compact soft-edged geometry reduce CPU work and repeated GPU uploads.
- Wound bleeding now defaults to three seconds; changing its duration also affects
  existing wounds. Extra wound drips and hit bursts use soft modern particles.
- Ignore discarded overflow effects when drawing modern particles and lights,
  preventing stuck blood or flashes after the game's effect slots fill up.
- Replace legacy corpse pools with soft pools that follow the ground, including
  a new pool beneath the player on death. Pools grow briefly and remain with other
  retained blood; Original preserves its existing presentation.
- Add optional modern gunfire for Echigoya, with short muzzle flashes and light,
  drifting smoke and impact sparks. Modern presets enable it; the gunfire demo
  compares both styles without changing attack timing, damage or gameplay RNG.
- Randomizing unprotected story actors now includes dormant bosses such as Hanbe
  and Echigoya. Their rolled models keep the original cutscene choreography and
  scripted activation, then use their randomized combat moves and AI.

## 0.5.1 — 2026-10-30

- Align shop quantities with their own item slots in both the stock grid and
  carried-item row.
- Keep item quantities above their icons in the carried-item row and gameplay
  inventory, matching retail drawing order.
- Support replacement artwork for narrated mission briefings on desktop and in
  the browser, and include all restored mission screens in the interface export.
- HD artwork v2 adds paintings for all ten narrated briefings and sharp shop,
  item-help, mission-results and training artwork in English, Japanese, French
  and Italian. The separate pack also refreshes final results typography and
  preserves the original scrolling brush lettering through disc-backed recipes.

## 0.5.0 — 2026-10-30

- Restore narrated mission briefings and the original item-shop grid in all four
  languages. The shop replaces the placeholder equipment screen, with item help,
  stock/carry limits, mouse controls and cancellation; retries skip the briefing
  and training starts with its fixed equipment.
- Show the retail mission results after ordinary missions, including elapsed
  time, score breakdown, best scores, rank and the correct mission reward icon.
  Training uses its separate leaderboard; final results still follow the ending.
- Correct browser mouse/touch targeting when the displayed canvas and drawing
  buffer have different scales, including shop buttons near the window edges.

## 0.4.0 — 2026-10-30

- Add optional soft blood spray and modern Mei-Oh lightning with branching arcs,
  impact sparks and brief blue lights. Modern presets enable them; Original keeps
  its existing effects. Persistent blood's F1 help now explains its non-fading,
  bounded retention, and wound drips face the free camera correctly.

- Fix distant actors wrapping into nearby detection/target ranges with “Keep all
  enemies active”. Remote patrols no longer hear every local alarm or keep the
  player's radar alerted across the map; default PS1 activation is unchanged.

- Correct stage-end actor control after player death, including guards sheathing
  their weapons. Reference replays now restore equipped armour correctly.

- Restore explosions, smoke, fire, lightning, weapon trails and grapple lines in
  showcase/free-camera views, including effects outside the gameplay camera.
  Preserve randomized movesets during demos so imported attacks actually launch;
  keep the blood demo's camera on the moving combatants.

- F1 → General can save a portable diagnostic report with build/platform/GPU
  details, active settings and controls, recent timings/inputs and gameplay/AI
  state. Desktop saves JSON; browsers download it. Personal paths are redacted;
  reports contain no game assets and can be inspected on any platform.

- Add a short randomizer showcase catalog and optional synchronized game audio
  for offline 4K/60 demo exports (`--sound`).

- Add an optional randomizer with full-disc player/enemy models and movesets,
  configurable HP, equipment, pickups, stages and guard behavior. F7, the title
  screen and F1 open its searchable pools, blacklists, presets and seed sharing.
  Retries reproduce the same roll; runs are remembered separately from campaign saves.

## 0.3.2 — 2026-10-29

### Fixed

- Restore missing stealth-kill effects and other sounds in mission sound banks
  with gaps between tone slots. Training worked because its relevant sounds
  appeared before the incorrect cutoff.

## 0.3.1 — 2026-10-29

### Fixed

- Honour disabled VSync on focused Wayland windows instead of also waiting for
  compositor frame callbacks.
- Reduce frame-time spikes by reusing animated geometry buffers and avoiding
  repeated shadow sorting. Always-active enemies and the free camera also share
  actor geometry between the scene and shadow passes, reducing GPU uploads.
- Align performance-chart intervals with the work they measure and show time
  spent outside the timed phases, including event-loop/compositor waits.

## 0.3.0 — 2026-10-29

### Added

- F1 player-model gallery with 58 disc character, boss, animal and outfit models
  available across missions, name/resource search and animated previews for
  visible tiles. Custom folders remain supported; selected models use their own
  textures and keep weapons attached to the replacement body.
- Optional **Use model's moveset** checkbox for actor animations, attacks and
  weapon profiles with normal player controls. Missing actions retain player
  fallbacks; appearance-only remains the default.
- Model-moveset options to use the protagonist's movement animations/speeds while
  keeping the selected actor's attacks, and to disable random idle/combat taunts
  that interrupt player control. Both are optional and saved across launches.

### Fixed

- Model-gallery search and help text stay within the settings window instead
  of gradually expanding it to the edge of the screen.
- Avoid recurring gameplay stutters when an effect or HUD texture first appears
  by uploading only its atlas regions instead of rebuilding the entire atlas.

## 0.2.0 — 2026-10-28

### Added

- F1 → Vibration controls with a global intensity slider and separate sliders
  for all 17 vibration actions, including hits, blocks, stealth kills, landings,
  ledge grabs and water entry. Settings persist, scale both motors and share the
  original menu's enable switch; graphics presets leave them unchanged.
- Keyboard and controller remapping in F1 → General → Controls, including both
  sticks, alternate bindings, conflict reassignment and per-device reset. Bindings
  persist across launches on desktop and in the browser.
- Desktop startup remembers the last verified disc path. Launching without a
  path reuses it when available; missing or invalid discs return to the picker.
  An explicitly supplied path always takes priority.

### Changed

- Smaller dependency-source download, limited to the copyleft libraries bundled
  with Linux and their build recipes. The Linux archive now lists its included
  shared libraries and matching source entries.

## 0.1.0 — 2026-10-28

### Added

- Separately downloadable campaign HD artwork is discovered automatically.
  Native loading and browser preparation accept `.tar.zst` archives or editable
  unpacked directories, with cached extraction and original-art toggles.
- Disc-backed HD texture recipes run in native Rust and WASM at asset load,
  preserving the existing artwork without shipping original colour/mask data
  in those textures or requiring Python.
- Play all ten missions and training as Rikimaru or Ayame, with the original
  menus, equipment selection, saves, cutscenes and endings. English, Japanese,
  French and Italian use the supported Shinobi Gaisen disc.
- Native Linux and Windows builds, plus a browser version with WebGPU and
  WebGL2 renderers. Gameplay keeps its original update rate independently of
  display frame rate, with optional visual interpolation.
- An F1 settings overlay with mouse support, controller configuration, a quick
  launcher, free camera, teleporting, inventory tools and gameplay modifiers.
- Original, Low, Medium, High and Ultra graphics choices; named user presets;
  a graphics-only retro toggle; and benchmarks that tune an Auto-optimised
  preset to a chosen frame-rate target.
- Optional dynamic lights and shadows, sun/moon lighting, texture filtering
  and relief, anti-aliasing, model smoothing, hair motion, improved vegetation,
  water, waterfalls, grapple effects and persistent blood.
- Optional wind-driven rain and snow, wet surfaces, roof runoff, footprints,
  falling leaves/petals and a time-of-day cycle.
- Replacement texture/model packs, live player-model selection and clear
  outline fonts. Replacement artwork is supplied separately.
- Feature demos, performance charts, input recording/replay and deterministic
  video export, including captured retail-run playback.
- A disc picker and validation of the supported disc's executable/artwork
  hashes at startup. No disc image, BIOS or extracted game assets are included.
- Permanent versioned downloads and a rolling HEAD development download, with
  checksums, build identity and dependency notices/sources.
- MIT licensing for the project's original code and resources.
