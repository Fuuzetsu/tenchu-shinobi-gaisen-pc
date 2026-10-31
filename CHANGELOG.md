# Changelog

User-visible changes are collected under **Unreleased** until a version is
tagged. Internal refactors, formatting and test-only changes are omitted.

## Unreleased

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
- Speed up texture and height-map preparation during loading and texture updates,
  preserving mip coverage, transparency and surface detail.

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
