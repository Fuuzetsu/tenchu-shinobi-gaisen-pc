# Settings and native inspection tools

Keep the current experience as the factory defaults. F1 opens a settings and
inspection menu in the existing window. Changes apply without restarting.
Presentation preferences persist separately from campaign progress. Debug
camera state, simulation controls and one-shot edits start disabled each launch.

## Deliverables

- [ ] F1 menu with keyboard, controller and pointer navigation, contextual help,
  reset-to-defaults, persistent preferences and visible feedback for errors.
- [ ] FPS/tick display, pause on defocus, fullscreen, VSync, HUD visibility and
  master volume. Preserve the existing focus, audio and input behavior by default.
- [ ] Fog enable/distance, textures, vertex shading, exposure, nearest/bilinear
  texture filtering and GPU-supported MSAA choices. Keep original defaults.
  Paletted VRAM needs decoded texture sampling; ordinary hardware anisotropic
  filtering is not automatically applicable.
- [ ] Enemy sight-distance multiplier, using the original angular, stance,
  occlusion and update rules at 100%. Clearly identify this as a gameplay edit.
- [ ] Free camera, flight speed, camera bookmarks, focus selected actor, overhead
  view, player teleport to a picked terrain surface and exact coordinates.
- [ ] Live body/attack hitboxes, sight bounds, patrol paths, actor labels and
  inspector, event bounds, terrain collision surfaces, X-ray overlays and player
  trail. Read live state directly; inspection must not advance RNG or mutate
  the gameplay camera, terrain query cache or render-derived gameplay state.
- [ ] Pause/frame advance, simulation speed and screenshots. Keep edits and
  recordings honest: unsupported state edits must not silently produce a tape
  that claims to replay the edited session.
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
