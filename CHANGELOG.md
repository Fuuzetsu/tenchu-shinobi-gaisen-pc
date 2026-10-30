# Changelog

User-visible changes are collected under **Unreleased** until a version is
tagged. Internal refactors, formatting and test-only changes are omitted.

## Unreleased

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
