# Settings and native inspection tools

Keep the current experience as the factory defaults. F1 opens a settings and
inspection menu in the existing window. Changes apply without restarting.
Presentation preferences persist separately from campaign progress. Debug
camera state, simulation controls and one-shot edits start disabled each launch.
Opening a category preserves its values, including preset values between slider
steps. Sliders clamp and round only when edited; a regression test covers sky
tint, light strength, angle, exposure, temperature and volume without input.

## Deliverables

- [x] F1 menu with keyboard, controller and pointer navigation, contextual help,
  reset-to-defaults, persistent preferences and visible feedback for errors.
- [x] FPS/tick display, pause on defocus, fullscreen, VSync, HUD visibility and
  master volume. Preserve the existing focus, audio and input behavior by default.
- [x] Fog enable/distance, textures, vertex shading, exposure, nearest/bilinear
  texture filtering and GPU-supported MSAA choices. Keep original defaults.
  Paletted VRAM needs decoded texture sampling; ordinary hardware anisotropic
  filtering is not automatically applicable.
- [x] Optional dynamic lights and shadow maps, disabled by default. Inspect
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
- [x] Meaningful regression checks for unchanged defaults, settings persistence,
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

The lighting increment adds up to two optional point lights selected from
registered fires/bonfires and a movable, camera-following inspection light.
Six depth faces per light preserve original paletted cutouts; moving actors
outside the gameplay camera can still cast shadows. Resolution, light radius,
intensity and ambient contribution are adjustable. Disabled output matches the
existing GPU regressions. Baked colors/blob shadows remain, translucent pixels
and particles do not cast, and baked lamp textures are not inferred emitters.
See README for the performance/coverage limits.

The shadow GPU regression checks real occlusion, moving lights, transparent
holes versus opaque texels, the second light, all four shadow resolutions with
supported MSAA, and restoration of unchanged defaults. Native screenshots
confirm actor/scenery shadows and live resolution changes. All 43 host tests,
including private-disc, audio, GPU, full campaign and new tool checks, pass.
A native Mission 2 run held Backspace through its opening, resumed music and
returned to normal-speed gameplay after tick 181.

Final verification: `nix flake check --no-update-lock-file` passes the native
package, shared-core WASM and formatting checks on x86_64 Linux. Other host
platforms were not exercised. The optimized desktop binary is rebuilt.
After the full tool/lighting integration, an isolated Wayland compositor again
exercised F1, optional lighting, free camera and repeated Escape exits: three
clean exits, with the graphics-before-display shutdown fix retained.

## Follow-up usability and shadows

- [x] Shift+Tab goes back a page. Pointer movement selects the hovered row without
  shifting a scrolled list. R restores that value, preserving unrelated settings;
  it does not execute one-shot actions.
- [x] Unlimited simulation speed runs every tick, including across cutscenes,
  while preserving pause/focus/single-step controls. Audio is muted at unlimited
  speed. Host state-sequence regression passes.
- [x] Make model/weapon/object shadows easier to inspect: preview action,
  original-blob toggle and active-light feedback.
- [x] Expand the point-light budget beyond two and allocate depth maps for active
  lights only. Check model/weapon and off-camera moving-object caster coverage.
- [x] Document current texture replacement limitations and a practical visual
  improvement/asset editing path.

The light budget now offers 1/2/4/8/16 sources with its original default of two.
Normal gameplay ranks nearby sources around the player, keeping both lights and
shadow slots stable when the camera orbits. Free-camera and cinematic views rank
around their own position. Sources still share the configured budgets; travelling
to another area can select a different set. A Mission 1 regression orbits through
24 views between two lanterns and confirms that the former camera-distance
ranking would switch sources there.
The menu reports actual sources, six shadow views per source and depth memory;
shadow arrays grow/shrink with active sources and release when disabled. A
preview action creates a movable light and changes presentation preferences for
clearer shadows. Blob decals have a separate switch, defaulting to on.

Shadow geometry includes opaque texels of mixed translucent faces. All live
actor/weapon meshes and hinged objects contribute independently of gameplay
camera culling. Hinged presentation uses the same pure local-transform helper
as the original gameplay draw path; it does not update scene caches or game state.
GPU checks verify actual Rikimaru/Ayame body and weapon silhouettes, the sixteenth
light, transparent cutouts and unchanged default rendering. A ten-mission check
matches visible hinged poses and verifies off-camera doors/trapdoors still move.

Native checks exercised reverse tabs, hover-reset from a budget of 16 back to 2,
Unlimited execution and R restoring 100%. Shadow-on/off captures in Mission 1
show the body/sword silhouette with blob decals disabled. Inspection-camera
focus/overhead had an inverted vertical sign; both now place the view above the
target, with a projection regression.

All 49 host tests pass, including both complete campaigns, private-disc/audio
fixtures and five GPU tests. One parallel run stalled with all GPU tests pending;
the full sequential run (`--include-ignored --test-threads=1`) completed in
25.49 seconds. Nix native packaging, WASM libraries and formatting pass on
x86_64 Linux. The release binary is rebuilt. Three isolated Wayland runs enabled
the shadow preview and exited cleanly under repeated Escape presses.

## Whole-game visual upgrade goal

Deliver an optional, coherent enhanced presentation, including textures and
models rather than only lights. The original look stays the factory default.
Changes remain presentation-only: source geometry/collision, animation updates,
AI, RNG and 30 Hz simulation do not depend on the chosen visual options.

- [x] Recover reusable asset identities, original texture regions and prop names
  from the loaded disc. Export a reviewable catalogue, decoded PNG textures and
  model/UV geometry; avoid tables of mission-specific coordinates/resources.
- [x] Implement an editable visual asset pack with original fallback, validation,
  reload and useful errors. Support higher-resolution texture overrides and
  visual mesh overrides attached to the existing animation parts.
- [x] Improve texture minification with isolated mip chains and anisotropic
  sampling, preserving texture holes and original semi-transparency. Offer
  optional automatic texture reconstruction/upscaling, tested against real art.
- [x] Improve model presentation with smooth lighting and an optional
  silhouette-smoothing mode where it helps, plus the replacement-mesh path.
  Retain sharp architecture and verify animation/weapon attachments and shadows.
- [x] Make lanterns and luminous props emit light, using recovered names/material
  information and editable reusable rules. Provide visible source markers and
  overrides for ambiguous assets. Add useful ambient/directional lighting.
- [x] Add restrained bloom, atmosphere/color controls and presentation polish,
  preserving readable HUD/menu rendering. Keep effects individually adjustable.
- [x] Add Original/Enhanced/Cinematic presets and clear F1 controls, including
  asset reload and quality/performance feedback.
- [x] Compare actual mission views and moving characters before/after, including
  indoor/outdoor scenes and both players. Verify original rendering/gameplay
  regressions, portable libraries, Nix packaging and repeated-exit behavior.

Acceptance is an enhanced preset that visibly improves the game using the
supplied disc alone, plus a working export/edit/reload path for genuinely new
texture/model detail. Automated processing cannot invent accurate bespoke art;
record where it helps, where it fails and how replacements take over. Do not
declare completion after only the lighting or asset-catalogue stage.

Asset foundation: retain uploaded TIM image regions and original CON/weapon
names as presentation metadata. `export-assets` writes decoded PNGs identified
by their content, a named catalogue and textured local-space OBJ model parts.
Mission 1's first export produced 342 images and 633 parts; actual AKARI/TORO
textures contain the bright lamp panels. A region/decoder regression covers
texture-page offsets, palette separation, edge UVs and opaque/transparent/STP
black distinctions. All twelve ordinary asset-crate tests pass. The original
renderer continues to use its existing VRAM path at this stage.

Texture filtering: F1 now offers trilinear and 2×–16× anisotropic sampling.
Decoded TIM regions have independently padded mip chains, with separate opaque
and semi-transparent colors and coverage. Anisotropy follows the texture
footprint's major axis, including diagonal footprints; live VRAM updates refresh
the decoded images. Original nearest sampling remains the default.
Six sequential GPU regressions pass, including rotated minification, preserved
short-axis detail, animation, original blends/fog and actual model shadows.
Thirteen fixture-free desktop tests pass. Native Mission 1 captures show smoother
ground/roof textures with unchanged HUD, followed by movement and a clean exit.
This removes sampling artifacts; it does not add new source detail.

Texture packs: the export includes an editable `pack.json`. `TENCHU_ASSET_PACK`
selects it; F1's Visual assets page reloads files and toggles replacements.
Unchanged exported PNGs are skipped, retaining live original animation. Failed
loads retain the previous pack. Size/path/PNG validation happens before applying
changes. Higher-resolution colors, cutouts and blend classes are shared by scene
and shadow shaders. New missions clear the old region cache before rebuilding.
Optional Scale2x/Scale4x reconstruction follows the
[published algorithm](https://www.scale2x.it/algorithm) and does not process
authored replacements.
Native Mission 1 reloaded 256px blue then orange checker fixtures on its floor,
kept the orange version after malformed JSON, and restored original art and
reconstruction successfully. These checker images test the editing workflow;
they are not proposed artwork. The unfiltered Scale4x view is still visibly
pixelated; it is a stylistic option, not a substitute for replacement detail.
Fourteen fixture-free desktop tests and seven sequential GPU tests pass,
including replacement detail, dynamic fallback and changed shadow cutouts.

Models: packs now accept triangulated OBJ part replacements, with explicit
material texture/blend/double-sided definitions. Original animation transforms,
placements, visibility and simulation remain in charge. Authored normals and
subpixel UVs survive import; imported geometry participates in ordinary drawing,
free-camera inspection and shadows. The default drawing path remains unchanged.
Optional generated normals preserve lighting creases, and Subtle/Rounded PN
patches soften only original actors. Original vertices, open seams, scenery and
authored replacement contours stay fixed. The stronger setting is intentionally
stylized; replacement art is still needed for new costume/face detail.

Small UV/color/position repairs can use `model_patches` in `pack.json` instead of replacing
an entire model. Each model ID maps to an array of patches with a `texture`
identity and optional three `corners`, each containing `position`, normalized OBJ
`uv` (V points up), and RGB `color` modulation. When supplied, all three original
positions must match. An optional `translation` moves the matched triangles in
local model units; without `corners`, it applies to every face using that texture
in that model part. Pack patches run before optional refinement, only with
replacement artwork enabled. Animation and simulation stay unchanged.
This can repair collapsed UVs, inconsistent baked colors or misplaced props while
retaining the original model's outfit and secondary motion. `preview-models`
accepts an individual part such as `actors/0001/2` for close-up inspection.

Built-in presentation corrections use the same format in
`crates/tenchu-pc/data/model_corrections.json`, independent of the artwork toggle.
Mission 5's screen is included in Bizenya's root model; two panels originally
penetrate the room's rear wall. Its four matching materials move 256 local Z
units into the room so the depth buffer can show the complete screen. The
merchant, furniture, source models and collision data remain fixed. Corrections
do not apply to authored replacement models. Refinement also pins edges between
back-to-back faces, preserving the screen's straight borders and flat panels.

The disc round-trip check exports/reloads both players' Mission 1 assets and
compares live actors/weapons, scenery and hinged objects without changing game
state. It caught an omitted degenerate quad; export now preserves that source
omission. Separate regressions cover malformed OBJ indices, per-corner UV/normals,
curved shared edges and fixed open boundaries. Native Rikimaru/Ayame views were
compared with model lighting, rounded/subtle silhouettes and actual shadows.
A larger, subdivided diagnostic head mesh reloads, follows movement/jumping and
casts its own shadow, then reverts through the artwork toggle. Both sessions exit
cleanly. The isolated X11 test reached only about 25 FPS during steady gameplay
with 4× MSAA, anisotropy, curved models and a 1024 point shadow; profile quality
costs and the presentation path before choosing the final presets.
All 57 desktop checks pass sequentially in release mode with the private disc,
audio/reference fixtures and GPU, including both complete ten-mission campaigns.
The x86_64-linux Nix package, native checks, portable WASM libraries and formatting
check also pass. Other declared platforms remain unverified.

Odd-sized replacement textures use area-weighted mip reduction, avoiding an
unequal color/coverage contribution from the final row or column. A three-color
3×1 regression verifies equal contributions, and the GPU minification/anisotropy
regression still passes.

Animation performance: a native profile found that moving source textures were
rebuilding every decoded mip chain and uploading the whole atlas. The renderer
now compares changed VRAM rows, including packed pixels and wrapping palettes,
and refreshes only affected images and padded mip rectangles. Atlas storage and
unchanged mips survive ordinary animation; dimensions/new materials trigger a
repack. A palette/pixel-boundary regression and all seven sequential GPU checks
pass. Repeating the isolated X11 gameplay sequence improved the sampled enhanced
view from 29 to 42 FPS and original view from 35 to 49 FPS. These are indicative
single captures, not a platform benchmark; driver presentation dominates the
remaining sampled CPU time. Both runs exited cleanly.

Lamp emitters now use recovered model names plus bright texture/geometry samples,
with positions from actual placements and weapon attachments. Exported packs
include editable matching, brightness, emission, color, radius and offset rules.
The first-layout campaign check finds 57/34/101/37/31/28 placed lamps in missions
1/2/5/6/7/9; other starting missions have no matching placed props. An ANDON
attachment fixture verifies movement with its owner, and source discovery leaves
game state unchanged. This is heuristic lighting metadata, not original authored
lights. Changed replacement shapes can require a rule offset.

Bright panels emit and transmit light while dark frames remain shadow casters.
The GPU regression covers transmission, opaque frames, lamp toggling, independent
light/shadow budgets and filtered shadow edges. Up to 16 point lights can be
active, with a separate 0–4 shadow budget. F1 labels active sources and can focus
free camera on the nearest lantern. Native inspection found excessive speckling
from paper mottling; transmission now includes the dimmer paper tones, and
optional nine-tap filtering softens edges. The master lighting switch remains
off by default.

Native Mission 1 checks compare lamps on/off, source markers, asset-rule reload,
shadow toggling and normal movement/jumping, followed by clean exits. The close
lamp inspection also exposed receiver self-shadowing; a surface-plane depth
gradient for the point-light projection now corrects comparisons across filter
taps and removes the speckling/moiré in that view. See Microsoft's discussion of
[shadow-map precision and bias](https://learn.microsoft.com/en-us/windows/win32/dxtecharts/common-techniques-to-improve-shadow-depth-maps).
All seven GPU regressions still pass, including actual actor/weapon silhouettes;
17 fixture-free desktop checks pass. HDR highlight handling and broad scene
lighting remain the next stage; the current lamp panels can clip to white.

Sky lighting adds an independently colored/directed sun or moon source, cool
ambient tint and one orthographic shadow map with separate resolution/coverage.
The projection follows the view in texel increments and fades shadows at the
boundary. Point sources can be budgeted to zero while retaining sky light.
Factory strength is zero and the lighting master remains off.
The additional GPU regression verifies occlusion without point lights, small
camera motion, color changes and restoration of the original output. Existing
seven GPU regressions pass as well.
Two native Mission 1 comparisons toggle sky lighting with the same camera,
inspect the player/ground overhead, then resume movement and exit cleanly.
Low-angle light is correctly blocked by the alley's buildings; a higher angle
lights the street and gives a clearer comparison. Final preset values still
need tuning with post-processing enabled.

Image effects now render the world into a linear HDR target, retain bright lamp
panels and additive effects for bloom, and finish the image before drawing the
HUD and inspection overlays. F1 / Image effects controls bloom and its threshold,
screen-space contact shading and radius, a filmic tone curve, contrast,
saturation, warm/cool balance, shadow brightness, vignette and sharpening.
Sky lighting adds fog color and gradual atmospheric haze. All are behind an
off-by-default master switch. The fitted filmic curve uses the author's
[CC0 implementation](https://knarkowicz.wordpress.com/2016/01/06/aces-filmic-tone-mapping-curve/);
this is a compact tone curve, not a complete ACES color-management system.

Contact shading uses only visible depth, so hidden/off-screen geometry cannot
contribute. Model and scenery shadows still use the actual caster meshes.
World depth is resolved separately for unprocessed overlays when MSAA is active.
GPU regressions verify glow outside a bright surface, unchanged UI under
grading, contact shading at corners, unshaded flat regions, every supported
sample count, and exact restoration of the original image. All eight previous
renderer regressions also pass, including live character/weapon shadows.
Mission 1 window checks cover image-effects toggling, overhead/free-camera views,
moving/jumping and repeated Escape exit. In one isolated 1280x720 X11 comparison,
the paused view measured 41 FPS with effects and 44 without; these are diagnostic
samples, not a general performance guarantee.

Graphics now offers Original, Enhanced and Cinematic presets. Enhanced combines
the texture/model improvements with the lighting and finishing passes; Cinematic
adds shadow quality/budget and stronger atmosphere. The controls remain
independent and label edited combinations Custom. The factory default is still
Original. A state comparison covers changing/resetting presets and photo mode,
preserving gameplay state, simulation speed, unrelated preferences and the
artwork-pack switch.

F10 hides HUD/captions and inspection helpers for clean F9 captures, leaving F1,
free camera and simulation pause available. Photo mode removes only UI geometry
and retains world effects/blend batches. Headless render/replay captures apply
an explicitly named TENCHU_SETTINGS file; unconfigured captures retain Original.
Render also accepts normal mission/character/layout options. Forty captures
(both characters, ten missions, Original/Enhanced) completed on the GPU; selected
forest, temple steps and snowy courtyard pairs were reviewed. Native Mission 1
comparisons exercised all three presets and a lantern close-up. A dark MSAA
fringe at distant geometry was fixed by clearing scene samples to the fog color,
with a GPU regression for the uniform fog/sky boundary.

Final validation: all 62 desktop checks pass with the private disc, FFmpeg,
retail-state and GPU fixtures. This includes both complete campaign routes,
source model export/import, lantern transforms, texture animation, original
blend/fog behavior, shadows and HDR/UI separation. `nix flake check` passes
x86_64-linux packaging/tests, formatting and the shared WASM build. Other host
architectures were not built on this machine.

Native comparisons cover both players in Mission 1 and Ayame in Mission 10,
switching presets at the same paused pose, free/overhead cameras, clean photo
view, movement and jumping. A recorded Mission 1 route at tick 5000 supplies an
enclosed-room comparison with ceiling occlusion and warm lamps. An enhanced
window successfully reloads diagnostic high-resolution PNGs and a modified,
subdivided OBJ head, retains the last working pack after a missing-file error,
then restores the original art. Three isolated native Wayland launches with
Cinematic effects, MSAA and shadows exit with status zero after repeated Escape.

The visual-upgrade goal is complete. These are optional improvements using disc
art, plus a working path for new art; they do not turn the source textures into
authored HD content. Source model seams and coarse scenery remain visible in
some views (including snow, with the new effects disabled). Texture
reconstruction and rounded contours remain individual choices because they do
not improve every asset. Baked lighting, heuristic emitter detection, limited
shadow coverage/budgets and screen-space contact shading retain the limits
described above. No game assets or diagnostic captures are included in Git.

## Follow-up: texture detail, compact controls and interpolation

- [x] Separate close-up sampling from mipmaps/aniso: Smooth, Crisp, bounded
  Catmull–Rom Cubic and Pixel, with unchanged Original defaults. GPU checks
  cover preserved texel interiors, narrower cubic blending and identical
  minification across modes; same-state Mission 1 room captures were inspected.
- [x] Replace the fullscreen bitmap-font settings panel with a compact, movable
  mouse overlay, retaining keyboard/controller navigation and per-value reset.
  Egui shares the existing wgpu device and draws after scene finishing with its
  own DPI-aware fonts. Categories, toggles, GPU-supported dropdowns, numeric
  sliders/typing, exact coordinates and reset context menus reuse the existing
  setting actions. Native checks cover pointer editing, a volume drag to 45%,
  R restoring only volume, exact coordinate entry, presets, Shift+Tab, long-page
  scrolling, resize, screenshots and repeated Escape exits. A GUI input
  regression distinguishes an Off checkbox from the Off MSAA dropdown.
- [x] Correct the enhanced lamp shadow budgets and inspect room occlusion.
  In the Mission 1 tick-5000 room, the visible AKARI01 wall lanterns were lights
  three and four, while Enhanced only shadowed light one. Both enhanced presets
  now give all four active lights shadow maps, reduce ambient from 85% to 45%
  and increase lamp strength from 120% to 220%. Same-state comparisons with
  zero/four point shadow maps show occlusion from props, characters and the
  lantern frames. This uses the existing emitter data and geometry, with no
  mission-coordinate overrides. Higher manual unshadowed budgets still leak
  light by design and are identified in the new overlay.
- [x] Add optional visual model interpolation between 30 Hz ticks, including
  held weapons and discontinuity handling, without changing gameplay state.
  Presentation-only history captures every simulation tick, including multiple
  ticks per display. Actor scene/model identities guard swaps and spawns;
  quaternion interpolation retains rigid rotation across angle wrapping. The
  same sampled matrices feed opaque/translucent models, hinged props, carried
  emitters and dynamic shadow casters. The gameplay camera shares that timing;
  cinematic cuts keep their source view. Pause/step, teleports and reloads clear
  history. Original defaults remain off. Full state comparisons for both players
  cover intermediate weapon/body/shadow poses, unchanged exact endpoints and
  reset behavior. Non-model sprites, trails/projectiles and blob shadows stay
  at 30 Hz; this adds up to one tick of visual latency at normal speed.
  Native Ayame movement/jump/attack and pause/step checks pass. Sampling the
  actual isolated Xvfb framebuffer at 120 Hz for three seconds counted 90 image
  changes with interpolation off and 309 with it on at 640×360. This establishes
  between-tick presentation, not a hardware performance benchmark (1280×720
  Xvfb presentation was limited to about 32 frames/s). No display rate is baked
  into the interpolation.
- [x] Verify actual native controls/rendering, regression checks, Nix packaging,
  portable shared crates and clean Wayland exit. All 66 desktop tests pass with
  private disc/audio/GPU fixtures, including both complete campaigns. Focused
  interpolation and GUI regressions pass after the final fixes. The final
  x86_64-linux Nix package, native tests, shared WASM libraries and formatting
  checks pass. Native X11 checks cover both players, model motion, mouse controls
  and repeated Escape exits. An isolated headless Wayland session enters actual
  gameplay, enables interpolation in F1, uses free camera/pause and exits with
  repeated Escape, returning zero. Its recorded game ticks and saved setting
  confirm those actions were processed. Other platforms and physical gamepads
  remain unverified. The interpolation and rendering limits are listed above.


## Browser and surface depth (2026-10-22)

The same F1 overlay, renderer and fixed-rate gameplay now run in WebGPU browsers.
PNG screenshots download; saves/settings use browser storage with import/export.
The web build prefills its sparse disc cache and selected-language media while
the menu is open, and gates mission entry until ready. See README for serving.

Graphics gains Off/Bump/Parallax surface depth and an independent strength slider.
A new Surface relief preset is opt-in; all previous defaults stay unchanged.
Inferred gradients and an eight-step parallax path add visual relief to opaque
scenery, with mip/edge/grazing fades. Native packs accept explicit height maps or
per-texture disabling via content identities. The shader avoids changing depth,
collision or silhouettes. The feature is shared by native and browser rendering.
