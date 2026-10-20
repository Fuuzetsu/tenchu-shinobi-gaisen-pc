# Settings and native inspection tools

Keep the current experience as the factory defaults. F1 opens a settings and
inspection menu in the existing window. Changes apply without restarting.
Presentation preferences persist separately from campaign progress. Debug
camera state, simulation controls and one-shot edits start disabled each launch.

## Deliverables

- [x] F1 menu with keyboard, controller and pointer navigation, contextual help,
  reset-to-defaults, persistent preferences and visible feedback for errors.
- [x] FPS/tick display, pause on defocus, fullscreen, VSync, HUD visibility and
  master volume. Preserve the existing focus, audio and input behavior by default.
- [x] Fog enable/distance, textures, vertex shading, exposure, nearest/bilinear
  texture filtering and GPU-supported MSAA choices. Keep original defaults.
  Paletted VRAM needs decoded texture sampling; ordinary hardware anisotropic
  filtering is not automatically applicable.
- [ ] Optional dynamic lights and shadow maps, disabled by default. Inspect
  authored fire emitters as light sources; provide a movable debug light,
  distance/intensity controls and explicit performance/coverage limits.
- [x] Enemy sight-distance multiplier, using the original angular, stance,
  occlusion and update rules at 100%. Clearly identify this as a gameplay edit.
- [x] Free camera, flight speed, camera bookmarks, focus selected actor, overhead
  view, player teleport to a picked terrain surface and exact coordinates.
- [x] Live body/attack hitboxes, sight bounds, patrol paths, actor labels and
  inspector, event bounds, terrain collision surfaces, X-ray overlays and player
  trail. Read live state directly; inspection must not advance RNG or mutate
  the gameplay camera, terrain query cache or render-derived gameplay state.
- [x] Pause/frame advance, simulation speed and screenshots. Keep edits and
  recordings honest: unsupported state edits must not silently produce a tape
  that claims to replay the edited session.
- [x] Cutscene fast-forward: hold-to-speed-up and optional automatic speed,
  including a bounded maximum-throughput mode. Execute every original logic
  update and keep UI/input responsive; default to ordinary speed.
- [ ] Meaningful regression checks for unchanged defaults, settings persistence,
  sight thresholds, geometry/picking, camera isolation and GPU settings; exercise
  the actual native menu. Format, document and commit coherent increments.

## Viewer reference

`~/programming/tenchu-viewer` is the reference for inspection features. Its stage
atlas includes collision surfaces, live ConflictObject hitboxes, standing/sneaking
vision bounds, event zones, patrol routes, actor inspection, teleporting, fly
navigation, X-ray overlays and rendering controls. This port can read those
systems directly from its own game state. Keep that repository unchanged,
including its existing uncommitted courtyard research.

Reference revision: `677cdc1f37710eec17b0479e7714dc3be09f2003`.
Consulted files include `README.md`, `web/index.html`, `web/live.js`,
`src/web.rs`, `src/collision.rs` and `src/sight.rs`. Record any adapted algorithms
and exact source hashes in `SOURCES.md`.

The viewer's emulator bridge, offline animation lab, disc exporter and particular
TAS/courtyard experiments are separate tools. Import useful runtime inspection
capabilities rather than their network plumbing or research datasets.

## Verification and limits

Run default campaign regressions after gameplay-facing changes. Compare default
render output and exercise supported MSAA modes with the actual GPU backend.
Use an isolated native window/audio sink for interactive checks. Shared core and
asset crates must still compile for WASM. No new disc assets belong in Git.

2026-10-20: the first menu increment passes eight fixture-free host tests,
including settings upgrade/corruption and atomic replacement. An isolated native
window check opened/closed F1 during Mission 1, changed preferences, displayed
FPS and verified that ticks stop while the menu is open. With another window
focused, ticks continue only when pause on defocus is disabled. Screenshots were
inspected; a footer placed too low was moved within the viewport.

The graphics increment passes all three offscreen GPU tests: unchanged source
blending/fog defaults, decoded bilinear samples, UI isolation and all supported
MSAA sample counts (queried for both color and depth). The native menu switches
filtering/MSAA while running. Mipmaps/anisotropic sampling remain later work;
MSAA is edge anti-aliasing, not a substitute for texture minification.

Fixed a reported rapid-Escape exit crash before continuing the inspection work.
The user's coredump showed NVIDIA EGL destroying Wayland proxies after the
winit event loop had closed its display. Graphics resources now shut down in
`ApplicationHandler::exiting`; GPU resources/surfaces drop before the native
window. An isolated headless Wayland compositor and virtual keyboard reproduced
the old binary's SIGSEGV (exit 139) and verified three clean patched exits
(exit 0) through gameplay → inventory → launcher → quit. Audio used a private
null sink. This supplements the existing X11 checks.

The inspection/playback increment passes masked-hole/slope/stacked-floor picking,
parallel ray-box edges, live inspection state isolation and teleport collision/RNG
checks for both characters. The actual host scheduler reaches identical states
at the end of Mission 2's opening at 2x, 16x, Max and held Backspace, and leaves
no accelerated backlog. Pause/frame advance executes exactly one update.
The original-C sight oracle and all twelve controller-route regressions (including
both full campaigns and training) pass at default settings.

An isolated native window exercised scrolling overlay pages, free flight/RMB,
overhead focus, typed coordinates, teleport, F9 PNG capture and F3/F4. Teleport
ended its recording at tick 27; replay consumed that complete prefix. Menu help
is fitted to the original 320-wide safe area, including at 4:3. Fast-forward
silences audio instead of trying to play accelerated dialogue; music restarts and
speech resumes on its next cue when ordinary speed returns.
