# Implementation plan

## Goal and scope

Deliver a playable modern Rust reimplementation of Tenchu: Shinobi Gaisen
SLPS-019.01, equivalent in game functionality to the original `MAIN.EXE`.
Preserve movement, combat, collision, AI, RNG, item behavior, mission rules and
glitches. Preserve fixed-width integer and GTE arithmetic wherever it affects
gameplay. The original menu and ending are now in scope, using the completed
decompilation on `tenchu-decomp` master. Keep the small development launcher
available with F6. Continue after each milestone until the complete game works.

## Performance, presets and first launch (active)

Preserve gameplay and visual quality while making performance measurable and
the port straightforward to configure and launch. Commit these deliverables in
coherent stages:

1. Repeatable native/browser benchmarks using the live renderer, fixed scene
   paths, excluded loading/warm-up and CPU/GPU frame distributions. Provide F1
   controls, cancellation/session restoration and a native JSON report command.
2. Profile representative scenes before changing algorithms. Address measured
   bottlenecks and simplify duplicated work; record settings, resolution,
   hardware, before/after results and any untested limits.
3. Coherent Low/Medium/High/Ultra visual tiers and measured auto-optimisation.
   Ultra includes the useful lighting, shadow, model, motion, water and effect
   enhancements. Do not force weather changes or alter gameplay preferences.
4. A persistent user preset library with save/overwrite, separate rename and
   deletion, protected built-ins and an Auto-optimised result. Keep preset
   storage out of per-frame settings clones.
5. Friendly no-disc startup with a native file picker and actionable validation
   errors for the supported Shinobi Gaisen disc. Preserve streamed browser
   startup and keep copyrighted data out of packages.
6. A brief build/run/download README linked to detailed documentation.

Validate statistics/selection and persistence with meaningful tests; exercise
the benchmark and settings UI, native and WASM builds, strict Clippy, formatting
and relevant gameplay/render regressions. A benchmark is a sampling tool, not a
guarantee that every scene meets its target on every machine. Existing hosted
artifact checks still await a configured Git remote.

The first benchmark implementation uses the live window/render loop and shared
demo scene setup. It has F1 controls, cancellation/session restoration, native
JSON reports, CPU phase and GPU pass percentiles, and an explicit offscreen mode
for profiling without virtual-display waits. Warm-up, delayed GPU results and
duplicate timestamps are excluded. Native and WASM strict Clippy pass; workspace
tests and focused statistics checks pass. Four 4K Original/Ultra baselines and
a CPU profile identify discarded opaque vegetation transforms as a major CPU
cost; Mission 5 also hits the existing replacement-pack pixel budget. Measurements
and their limits are recorded in `docs/performance.md`. Auto-tuning, preset
management, measured optimisations and first-launch improvements remain pending.
The native F1 benchmark also completed all four scenes and returned to the same
paused Mission 1 tick, camera and settings; Escape cancellation restores the
session too. GPU warm-up/duplicate handling has a focused regression test.

Profiling led to filtering opaque triangles before transforming geometry for
the transparency pass. Fixed Ultra benchmarks reduce CPU p95 by 66–82% (17–33 ms
to 5–10 ms), with unchanged GPU costs. The regression compares retained triangle
order and complete vertex bytes under different transforms, fog and vegetation
variation. This removes work without reducing visual settings or touching logic.
All 41 renderer regressions pass after the transform change. Pack loading now
shares identical decoded images between texture identities and height maps;
Mission 5 was only about 72 KiB above its 64 MiB limit because duplicates were
counted and retained separately. It renders with the complete pack after sharing,
without increasing the memory budget. The asset-selection regression verifies
that duplicate identities share their image allocation.

## Automated build artifacts (implemented; hosted run pending)

Build Linux x86-64, Linux ARM64, Windows x86-64 and the WASM browser package on
pushes to `master` and `main`, with manual dispatch as well. Use the pinned Nix
toolchain and publish downloadable archives with instructions, font licenses,
commit identity and checksums. Exercise the archives, not only the build outputs.
Keep all disc assets out of CI. Linux downloads must account for their runtime
libraries rather than distribute an executable with unavailable Nix store paths.
There is no Git remote configured here, so hosted execution will need the first
push to GitHub; validate the workflow and packaging locally in the meantime.

One matrix workflow builds `linux-archive` on x86-64 and native ARM64 runners,
`windows-archive` with the existing MinGW cross-toolchain, and `web-archive` with
the pinned wasm-bindgen. Artifacts include SHA256SUMS, instructions, font
licenses and build provenance, with 14-day retention. Windows startup also runs
on a Windows runner after downloading the archive. The workflow is validated
with actionlint and shellcheck through a Nix check.

Linux archives include a relocatable loader/library set, PulseAudio and
X11/Wayland/Vulkan/OpenGL dispatch libraries, keyboard data and ALSA configuration.
They use host GPU drivers and external FFmpeg, leaving the normal Nix package
unchanged. The launcher keeps its libc search path out of child processes so
system FFmpeg uses its own libraries. The x86-64 archive is about 25 MiB; it
starts and loads all graphics/audio dispatch libraries with `/nix` hidden, and
renders Mission 1 through the host NVIDIA Vulkan driver. Windows ZIP startup
passes under Wine. The extracted browser package serves its JS/WASM/HTML with
the correct MIME types. The ARM64 archive expression evaluates; its build and
isolated-library checks await the hosted ARM64 runner. No gameplay code changed.
The final Linux archive also passes live X11 input, PulseAudio initialization,
screenshots, save and repeated-Escape shutdown. All seven local Nix checks,
strict Clippy, formatting and archive checksums pass. Validation outputs remain
in ignored `private/ci-packaging`.

## Platform validation (complete for available hosts)

Check native Linux and the Windows executable, with macOS and ARM where the
available environment permits. Keep compatibility-layer/emulated checks distinct
from testing on physical target hardware. No remote CI or Mac runner is configured.

The x86-64 Linux release passes all 126 unit tests, 41 renderer and three menu GPU
regressions. Live X11 gameplay exercises movement/jumping, Modern High, F8,
resize, screenshots, save and repeated-Escape exit with PulseAudio output. The
Windows executable exercises Win32 window/input, WASAPI output and Vulkan
under Wine 11, including gameplay, preferences, a 1000×700 client resize and
clean shutdown. Its Original capture is pixel-identical to Linux. All 33,404
updates in the ten-mission retail
comparison pass on Windows with zero state corrections and an identical report.
The same 44 GPU regressions also pass on Windows. Both movie regressions decode
all five opening/training/ending streams, including captions/audio and early
exit, with a checksum-verified Gyan Windows FFmpeg 8.1.2 build. Live opening,
pause/skip and title were inspected. The Windows test shell must retain the
Windows `TENCHU_FFMPEG` path; the ordinary Linux dev shell selects Linux FFmpeg.
Missing-decoder/encoder messages now explain the portable FFmpeg override.
All 126 workspace tests also execute as ARM64 Linux binaries under QEMU; ARM
graphics/audio devices were not tested.

Windows testing exposed concurrent replacement failures while a second rename
still holds the destination. Save replacement now retries transient permission
errors with 31 ms of total backoff, retaining atomic rename and cleanup on
permanent error.
The eight-writer save regression passes twenty repetitions under Wine. Another
failure came from building an unused native `cdylib`: debug Windows exceeded the
PE export limit. Native builds now use an ordinary Rust library; browser builds
explicitly request `cdylib` through `cargo rustc`. Windows unit tests and Clippy
pass, and browser package generation and WebGPU/WebGL2 smoke checks still work.

`nix build .#windows` produces the Windows executable, and `nix develop .#windows`
provides the pinned compiler, linker, Wine runner and Clippy. Linux Nix checks now
include Windows compilation; all six checks pass. A verified Windows test ZIP
with the executable, instructions and font licenses is available at
`private/platform-check/tenchu-windows-x64.zip`, without game assets or FFmpeg.
Apple Silicon package/check expressions evaluate; actual Metal, CoreAudio and
macOS execution require a Mac. The pinned Nixpkgs
has removed Intel macOS, so the invalid `x86_64-darwin` output is no longer
advertised. Windows DirectX/physical drivers and controllers remain unverified.
Evidence and captured images stay in ignored `private/platform-check`.

## Shipping cleanup (complete)

Keep this pass focused on DRY, KISS and YAGNI. Preserve game behavior, existing
save/pack formats and native/browser controls; do not add speculative frameworks
or make a new recording.

- Consolidate repeated F1 preference edits, resets and slider limits. Compare
  the previous and revised control behavior before committing.
- Simplify asset/presentation ownership and duplicate pixel conversion; remove
  unused declarations and unnecessary native code/dependencies from the browser.
- Separate settings validation from file access and share platform path rules.
  Review tests/comments for actual value; keep source arithmetic and retail
  regressions instead of deleting useful coverage to reduce the test count.
- Run formatting, strict Clippy, workspace tests, targeted disc/replay checks,
  native packaging and the browser build/smoke checks. Record what was verified
  and any platform/release limitations rather than claim universal readiness.

F1 preference cleanup: 37 toggles and 59 numeric preferences now share field
access for editing/reset; mouse sliders and keyboard edits use the same limits.
Gameplay actions retain their explicit host handlers. All 2,220 before/after
cases across Original, Modern High and Showcase match exactly, including large
steps, wraparound and resets. The comparison harness/results stay in ignored
`private/code-polish`; existing UI regressions remain in the suite. The 67 PC
unit tests, eight disc-backed host tests and strict workspace Clippy pass.

Persistence cleanup: settings own their validation and a small store module owns
JSON I/O. Saves and preferences share the platform-directory rules without
changing existing paths or formats. Atomic writes use exclusively created,
unique sibling files instead of clobbering a fixed `.tmp` path. The save test
now exercises eight simultaneous writers, unrelated-file preservation and
cleanup after a failed rename. Storage/settings tests, strict workspace Clippy
and WASM checking pass.

Asset/presentation cleanup: menu, world textures and exported artwork share one
PS1 color/alpha conversion. Triangle patches use fixed arrays instead of heap
allocation; immutable built-in corrections are decoded once. Model swaps borrow
the player during ordinary rendering and compare rig iterators directly. Disc
tracing uses the cache's page-size constant; native capture/compression
dependencies are scoped to the native target. All 126 workspace unit tests,
strict Clippy, WASM checking and seven disc/GPU model/menu/sprite checks pass,
including the folding-screen repair and both players' weapon attachments.

Core cleanup removes sixteen unused motion constants, shares item disposal's
common reset and makes context-only helpers independent of actor/item borrows.
Lossless sign extension uses `From` while retaining the deliberate narrowing
casts and arithmetic order. All 33,404 recorded campaign updates (349 draw
skips) pass the full retail RAM comparison with no state corrections; the entire
report equals the pre-cleanup report. Workspace tests, the serialized campaign
replay and both players' explosive-item regression also pass. The three remaining
`repr(C)` declarations describe GPU buffer layouts and are required.

Release checks now include strict Clippy in `nix flake check`. All five
`x86_64-linux` checks pass: native package/tests, browser package, portable WASM
crates, formatting and Clippy. The packaged native executable reads the disc and
renders with Vulkan outside the development shell. The original-C damage oracle
also passes. Browser smoke scripts now follow the current F1 preset layout;
Chromium WebGPU and WebGL2 pass gameplay/input/audio, save/restart, movie/title,
resize/fullscreen, offline readiness, surface-relief and High/Ultra controls.
Retro mode, independent gameplay preferences, Shift+Tab, hovered reset and clear
weather preserve the expected state. Captures were reviewed and contain the
rendered scenes; no GPU or JavaScript errors were reported.

Validation evidence is kept in ignored `private/code-polish`. The browser visual
checks used a virtual X11 display; Chromium's headless launch produced a black
canvas on this machine. Windows, macOS, ARM and Firefox were not revalidated in
this pass. These checks protect the refactored behavior and the recorded retail
route, rather than establish exhaustive game parity or universal driver support.
No dependencies were added; game formats and recordings are unchanged.

## Screen and health-bar repairs (complete)

Mission 5's folding screen belongs to Bizenya's root model alongside his body
and furniture. Character refinement incorrectly relaxed edges joining opposite
faces, producing scalloped, cloth-like borders. Subdivision now treats those
edges as boundaries; the real screen retains its planar area at both refinement
levels. Its replacement tiles were also misclassified as costume cloth. A
joined gold-leaf repaint now supplies four consistent, padded 256px tiles.

The missing half was a separate depth-buffer problem: the original screen
penetrates the room's rear wall by roughly 110–170 world units at the reviewed
camera. Four texture-matched presentation corrections move only its panels 256
local Z units into the room. They apply with original or replacement artwork;
custom replacement models own their geometry. Animation, collisions, RNG and
actor placement are unchanged. The existing patch format now supports optional
material-wide translations while retaining the earlier triangle UV/color repair.

The health artwork had misregistered generated outlines and color composited
over the original sprites. Four revised player/enemy layers now share the
source crop layout, with one frame and an interior fill. Additive-neutral black
pixels stay black. The live campaign pack contains these eight corrected
textures; other pack files and the frozen recording artwork remain intact.
Reproducible sources, prompt and mask registration are under
`private/campaign-hd-source/{m05-screen-v2,ui-hud-v2,fix_hud_screen.py}`.

Validation: 67 native unit tests, workspace/all-target Clippy, formatting,
portable WASM checking and the release build pass. Focused disc checks preserve
the entire gameplay state, all non-screen vertices and the screen's area.
Actual retail cutscene stills confirm all four panels with both artwork paths,
including a later animated pose. Composed HUD previews cover five player health
levels and enemy bars from full to empty; all five original-art captures remain
pixel-identical. Evidence, prior artwork and installation hashes are under
`private/fixes-hud-screen`. No recording was made or replaced. Future recordings
require an explicit user request; the withdrawn lighting question needs no change.

## Full campaign showcase revision (complete)

Re-record the validated campaign with the visual enhancements actually applied:
dynamic lantern/moon lighting and shadows, surface relief, refined characters
and vegetation, hair/cloth motion, grapple geometry/sparks/light, blood, improved
water/waterfalls, splashes and foliage effects. Preserve original mission weather
and use a fixed sky; do not add weather or time cycles. Keep all core input,
recorded skip decisions, cutscenes and audio timing unchanged.

The first recording selected Modern Ultra but did not configure its GPU renderer.
View construction now applies settings before the first frame. A disc/GPU check
verifies the applied renderer configuration and interpolation without mutating
game state. Each recording writes its actual renderer settings beside the clip.
The new Showcase preset makes this configuration available in F1 and the preset
demo, preserving independent artwork/weather/gameplay choices.

Recorded PS1 draw skips deliberately leave the authoritative actor draw list
empty. Presentation now rebuilds non-suspended visible model parts from cloned
actors on those updates instead of blanking every model. All 349 skips in the
33,404-update campaign pass a pose/state-independence regression, including
mission/cinematic transitions. Core draw lists and caches remain untouched.

Ayame's nose underside had collapsed UVs and an extra green tint on its three
vertices. The artwork pack now carries a targeted triangle UV/color repair,
matched by original texture identity and all three local positions. Geometry,
outfits and hair motion stay intact. A generic optional `model_patches` format
keeps this asset correction out of engine code. Six original/replacement head
views and an actual 4K Mission 2 close-up were reviewed; the green strip is gone.
Original captures differ at most by one channel value at a few filtering pixels.

GPU previews show lit grapple sparks, the braided rope and real lantern shadows.
Native unit tests (65), workspace/all-target Clippy, formatting and portable WASM
checking pass. The actual renderer settings include all requested effects and
original mission weather. Offline capture can render independent chapters in
parallel with separate logs and consistent actual-settings checks.

The completed viewing edit is
`private/retail-showcase/render/tenchu-ayame-showcase-4k60-youtube.mp4`:
**19:44.8, 3840×2160, exactly 60 fps, H.264/AAC game sound and ten chapters**
(7.80 GB). All 71,088 video timestamps and complete video/audio decoding pass.
Twenty soundtrack comparisons across all missions have zero measured lag at
12 kHz and correlation above 0.9998. The edit retains all 33,404 updates and
cutscenes, removing only 4:44.6 of boot/loading waits. The unshortened 24:29.4
version also passes all 88,164 timestamps and full decoding.

The frozen build is `b77eb9c`. All eleven source chapters share the audited GPU
settings, and the executable, tape, soundtrack and all 1,724 artwork files match
the input manifest after capture. The Showcase fill/exposure was adjusted after
rooftop review to retain readable unlit surfaces without changing the night sky.
Visual review covers 58 gameplay/cutscene frames across all ten missions, extra
combat/closing samples, both sides of all ten joins, the title and final result.
Actors remain present in the sampled recorded skips. Tight camera views during
acrobatics and the cave/cliff route match the previous validated run.

Frozen executable/artwork, reproduction scripts, actual settings, frame/sample
edit map, validation reports, visual evidence and a YouTube description/chapters
are preserved under `private/retail-showcase`. The previous recording and its
original artwork snapshot remain intact under `private/retail-campaign`.
This revision changes presentation only; original mission weather, core replay
timing, ordinary preferences and saves are unchanged. As before, the source tape
ends at the ending movie's start; the video ends after the final result.

## Archived full campaign replay and recording (complete)

Replay the fixed Ayame/Normal boot-to-ending campaign identified by
`~/tenchu-searches/FINAL-RUN.txt`. The archive is
`~/tenchu-searches/mission-1-action-repair-20261013/exports/final-campaign-20261021/replay`:
88,075 emulator fields, ten mission OTAS jobs and one continuous boot/menu
controller stream. Preserve the archive and ongoing searches. All new captures
and copyrighted evidence stay in the port's ignored `private/retail-campaign`.

1. Freeze tape/provenance hashes and replay the continuous stream in the pinned
   BizHawk runtime. Observe mission entries, RNG, consumed controller latches,
   draw skips/display swaps and gameplay comparison state. Verify original
   checkpoint/final RAM; distinguish field samples from exact update boundaries.
2. Add an explicit recorded-cadence replay path to the scalar port. Import only
   required initial state at mission boundaries; subsequent gameplay must run
   from recorded inputs without correcting state from the reference trace.
   Original source skip decisions affect animation/collision caches/activation;
   keep them separate from host rendering and never derive them from PC speed.
3. Validate all ten missions, cinematic transitions and results against the
   reference. Diagnose the first divergence and fix general source behavior;
   do not substitute a successful but different controller route. Keep ordinary
   play's existing fixed-rate behavior and native/WASM portability.
4. Add reproducible offline full-run capture with enhanced graphics, full HD
   artwork, smooth 60 fps presentation and synchronized game sound. Preserve
   source gameplay timing. Shorten boot and loading gaps as requested while
   retaining every gameplay update and in-engine cutscene; make identical audio
   cuts and preserve an explicit frame/sample edit map.
5. Produce a YouTube-compatible 3840×2160 H.264/AAC MP4 with mission chapters.
   Verify every timestamp, audio/video synchronization and full decoding; review
   gameplay and chapter boundaries. Preserve executable/settings/tape hashes,
   validation reports and a replay/export launcher. Commit coherent increments.

Reference capture: a continuous cold-boot replay with field telemetry matched
all 77 archived endpoint RAM images byte-for-byte. A separate read-only update
observer now records controller latches, source phases, draw/swap decisions and
RAM at EndDrawing. Its 1,292-field first-mission trial also matches the archived
endpoint. Lossless changed-page storage reproduces every record/snapshot from
the full-RAM observer (10,311 events, 645 snapshots), reducing that trial from
424 MB to 13 MB. The observer reuses the archive's hash-pinned core/CPU mapping
checks and handles MAIN/MENU address overlap explicitly; source repositories and
the archived run are unchanged.

Full native validation now passes: **33,404 consecutive gameplay/cinematic
updates, all ten mission completions and all archived score components**. The
349 skipped draws and display swaps are explicit replay input. Comparisons cover
RNG, actor state/commands/inventory, AI, camera/history, collision results, model
matrices, activation budgets, drawing packets and mission statistics. Mission
initialization carries the recorded seed, inventory/settings, display epoch,
loader terrain caches and ambient sound timers; there are no state corrections
after initialization. Native and console result clocks/scores agree throughout.

The corpus exposed missing source behavior: expired blood can overwrite its own
effect slot while spawning a droplet; Balma temporarily uses BALMER.ACM, and
particles retain their terrain node across that map swap. These are general core
fixes, covered by targeted checks and the continuous campaign replay. Normal
play keeps fixed-rate updates and never derives console skips from host speed.
The checker and private JSON evidence are ready. The compact replay export now
replays all ten missions independently with every RNG/clock check. The first offline
capture selected Modern Ultra (the GPU application defect is fixed in the revision
above), the full artwork pack and sharp text without reading/writing
ordinary preferences or saves. It converts the original rational NTSC field rate
to exact 60 Hz, preserving the archived lossless soundtrack's duration. Native
title/mission/results screens cover original boot/loading intervals; those menus
are presentation, outside the exact gameplay validation. Resumable chapter
rendering, audio muxing, timestamp/decode verification and input hashes are in
`tools/retail-capture/record.py`. The completed viewing edit is
`private/retail-campaign/render/tenchu-ayame-campaign-4k60-youtube.mp4`:
**19:44.8, 3840×2160, exactly 60 fps, H.264/AAC with game sound and ten chapters**.
It removes 4:44.6 of boot/loading waits while retaining every gameplay update and
in-engine cutscene at source speed. The source tape ends as the ending movie
begins; the export ends after the final result, without that movie or credits.

All 71,088 consecutive timestamps and full video/audio decoding pass. Twenty
audio samples across all ten missions have zero measured lag against the original
lossless console soundtrack, with correlation above 0.9998. Cuts copy video
packets and retain exactly matching audio intervals, with 10 ms fades at joins.
The frame/sample edit map and original rendered parts are preserved. Gameplay
and cinematic samples from every mission, the title, results and shortened
transitions were visually reviewed against the console reference where relevant.

Capture found and fixed MP4 chapter-duration rounding at joins, and fractional
audio-seek resampling phase differences in the validation utility. Format/Clippy
and focused recording checks pass; an intentional 80 ms audio offset is rejected.
The frozen native executable, reproduction scripts, source/artwork hashes,
verification reports and YouTube description are beside the MP4. The executable,
tape, soundtrack and all 1,722 artwork files were rechecked after rendering.
Normal preferences, saves, the source archive and ongoing searches are unchanged.

## Remake presentation and usability follow-up (complete)

Implemented the requested 18 improvements and recorded a focused showcase of
this batch. Original rendering and gameplay remain available; new visual presets
and gameplay profiles are independent. Native and portable WASM paths share the
presentation logic.

- Snow footprints use compact tessellation and reserved fade capacity, avoiding
  abrupt disappearance when their geometry budget fills. Fresh cover gradually
  buries them. Continuous coverage, filtered grain and world-space shelter
  softness remove rectangular patches and soften directional tree silhouettes.
  Weather's clear action removes accumulation, tracks and weather particles.
- Sight inspection bounds stop at stage geometry using a cached triangle BVH.
  This is a visual diagnostic; the original AI is unchanged. Moving doors are not
  part of this static inspection mesh.
- Modern High/Ultra integrate useful effects with 2x/4x MSAA, no black sight fog
  and no original blob shadows. They preserve weather choice and artwork-pack
  preference. F8 temporarily compares Original visuals and restores the selected
  settings. Independent gameplay profiles group guard activation, sight range,
  cutscene speed and opt-in practice tools without changing inventory or graphics.
  AGENTS.md now requires reviewing these presets when adding presentation features.
- Local lighting supports eight shadow slots, stable assignments and 0.3-second
  handovers. High/Ultra select six/eight. Finite sources use blocker-dependent
  penumbrae; emitting paper panels diffuse their own light while caps and other
  solid geometry still occlude it. Sunlight suppresses local fill on exposed
  surfaces while covered lanterns remain useful. Spatial shadow batches cull
  against each face with wind padding.
- Botanical/Botanical dense build folded leaf sprays and rounded bamboo stalks
  from existing materials. Autumn, snowy shrub and bamboo families participate.
  Stable per-placement crown variants keep roots anchored; trunks, branches and
  leaves have separate matching visible/shadow wind motion. These are procedural
  geometry improvements, not a replacement set of hand-authored tree assets.
- Contact effects add round braided grapple rope, refined hook geometry, impact
  sparks and a brief light; water droplets/ripples and temporary nearby wet marks;
  falling cherry petals/autumn leaves that settle, fade and scatter; and snow/
  puddle scatter scaled by walking, running and landing. Cosmetic state has
  bounded budgets and its own RNG. Ceiling/wall sparks spray away from the struck
  surface instead of immediately colliding with it. Generic original combat
  sparks remain, because they also serve unrelated weapon and item effects.

Verification: 63 PC unit checks and focused native GPU/actual-disc checks pass.
The real-stage shadow-culling comparison matches the unculled render exactly.
Tests cover snow shelter/burial, daylight and finite-source shadows, vegetation
budgets/anchoring, contact lifetimes/wetting, ceiling/wall sparks, visual/gameplay
preset separation and unchanged core state. Real models from missions 1/3/4/5
were exercised. Formatting, workspace/all-target Clippy, release native build and
WASM checking pass. Chromium WebGPU and WebGL2 exercise High/Ultra, F8 round-trip,
gameplay profiles, clear weather and contact controls without GPU/page errors.
WebGL2 uses a same-pass R32F attachment for readable blocker/shelter depth.

The final recording is
`private/remake-showcase/tenchu-remake-showcase-4k60.mp4`: **3:45, silent 4K/60,
20 chapters**. Every one of its 13,500 consecutive timestamps, full decoding,
chapter metadata and local browser chapter playback pass. Decoded samples from
every chapter were reviewed. The snow view holds the path; tree cameras frame
roots/crowns; the grapple comparison shows an actual lit ceiling impact. Frozen
inputs, input hashes, isolated settings, original/pickup footage, packet-copy
assembly, a watch page and YouTube chapter text are preserved beside the video.
The 20 `remake-*` clips are also in the built-in demo catalog. Private capture and
review evidence remains outside Git under `private/remake-review` and
`private/remake-showcase`.

Limits: litter, wet marks and scatter are bounded visual approximations, not a
fluid/ecosystem simulation. High/Ultra are demanding: at roughly 1340×758 on this
GTX 1080 Ti, High measured about 30 FPS on Chromium WebGPU and 6–7 FPS on WebGL2.
Offline 4K/60 capture preserves every frame but is not a real-time performance
claim. Normal preferences and campaign saves are preserved by the showcase.

## Weather follow-up showcase (baseline captured)

Record only additions since the 3:06 effects showcase at `289d25a`: stronger wet
reflections, puddles, rain haze and recurring eave drips; directional rain/snow
shelter; snow footprints and blood staining; grenade/kaen light spill; and F1
inventory, god/stealth and click-teleport tools. Hold the full campaign HD pack
constant. Use short, clearly framed clips, pre-settled snow where useful, captions,
silent 4K/60 offline rendering and YouTube chapters. Preserve earlier recordings,
normal preferences and campaign saves. Review decoded frames from every clip,
check every timestamp and full decode, and retain a reproducible live/export
playlist plus frozen capture inputs. Explain the existing full-texture launcher.

The baseline is captured under private/weather-showcase (2:46, 9,960 frames,
11 chapters). Full decoding and exact 60 Hz timestamp checks pass. The later
18-item follow-up supersedes delivery of this reel; its new showcase must also
improve the footprint camera, which follows too far past the visible actor.

## Directional weather follow-up (complete)

Use the same incoming direction for rain/snow particles, shelter and surface
deposition. Remove wetness from leeward/parallel walls; allow windward snow dusting
while retaining stronger coverage on upward surfaces. Wind edits must redirect
airborne particles and recompute their roof/wall hits without clearing accumulated
weather. Recheck footprint burial when wind changes shelter. Verify reversed/calm
wind, covered walls, grazing roofs and live changes on CPU/GPU and in browsers;
preserve Original presentation and gameplay, then format, lint and commit.

Rain/snow velocities now share the shelter-map direction at every particle fall
speed. Wind changes redirect existing particles, discard newly sheltered ones
and retrace their remaining paths. Wetness uses incoming surface incidence;
snow combines it with stronger retention on upward surfaces and a light coating
on windward walls. Footprint burial rechecks shelter after wind edits. Airborne
roof drips respond to wind, and catchment rejects slopes facing away from rain.

Weather has an unbiased depth pass and four plane-corrected depth comparisons,
fixing rain leaking through close upstream walls at grazing angles/low quality.
Lighting shadows retain their existing depth bias. Nine native GPU weather checks
pass, including calm/reversed wind, both wall faces, a wall only 100 units behind
cover at low/high quality, roof shelter, distant coverage and snow stains. Mission
1 roof/wall captures were reviewed with HD artwork. CPU regressions verify live
particle redirection, collision/shelter, retained accumulation and track burial.
Workspace tests (111 passing), formatting, Clippy and native/WASM builds pass.
Chromium WebGPU/WebGL2 comparisons show changed rain/snow exposure when reversing
wind, preserve paused gameplay state and report no graphics/page errors. Evidence
is under private/wind-review. Accumulation still uses a global amount with the
current wind/shelter mask, not a separate weather history for each surface.

## Rain presentation follow-up (complete)

Make rain legible in actual play without changing gameplay or Original weather.
Add independently adjustable wet reflections, shallow puddle coverage/build-up,
rain haze and streak visibility. Wet highlights should respond to nearby lights
and sky, with shared world-space ripples rather than texture-tile seams. Improve
roof runoff so small streams travel downhill and visibly drip from eaves, with
an amount control and bounded work. Keep wind/shelter and accumulated snow intact.

Verify accumulation/decay, pause and render-cadence independence, roof runoff
and geometry bounds, shader shelter/light response, native scene captures and
WebGPU/WebGL2 controls. Update the rain demo and document what the cosmetic
puddles/reflections approximate. Format, lint and commit coherent increments.

Implemented separate F1 controls for wet highlights (including shadowed point
lights), puddle coverage, 0.25–10x pooling, shelter-aware rain haze, streak visibility
and roof drips. Puddle masks/ripples share world coordinates across texture tiles.
Catchment probes seed downhill rivulets independently of visible rain particles;
discovered eaves emit recurring drips, bounded to 96 sources and 192 runoff drops.
Streaks trail behind falling drops rather than extending through their impact
surface. Cosmetic state preserves pause, model-rebuild and mission-reset behavior.
Native GPU checks cover reflection/ripple/haze controls and shelter; Mission 1
roof/ground captures were reviewed. Six CPU weather checks and workspace tests
pass. Native playback completes all three rain demos. Chromium WebGPU and WebGL2
checks visibly compare wet reflections and haze with fog off, edit/reset the new
F1 values, retain identical paused game state and report no graphics/page errors.
In the 1280×724 smoke check, WebGPU sustains 60 FPS; WebGL2 is functional but
slower (about 15 FPS with rain and four lights, versus 19 with weather Original
and those lights still enabled). This is not a 4K performance claim. Native/WASM
builds, formatting, Clippy and the point-light/shadow regressions pass. Private
captures, browser results and demo logs are under private/rain-review. Puddles
approximate shallow surface variation; reflections include sky/light highlights,
not mirrored scenery or a fluid/drainage simulation.

## Weather integration and experimentation tools (complete)

Address the follow-up from actual play, preserving default gameplay and portable
native/browser rendering. Commit and verify each coherent stage.

1. Preserve snow/wetness across model-detail and artwork changes. Separate
   precipitation shelter from moving actor shadows, cover distant visible terrain,
   and trace wind-driven rain/snow past roofs and against walls.
2. Add bounded, ground-projected footprints that fresh snow gradually covers.
   Make blood stains blend with snowy surfaces instead of exposing the old ground.
3. Give kaen and explosions transient dynamic light using actual item/effect
   state, while respecting the existing lighting switch and budgets.
4. Add an F1 experimentation page with editable mission inventory, god mode,
   never-noticed stealth and persistent click-to-teleport. Keep cheats opt-in,
   avoid invalid inventory states, and stop ordinary input recording before edits.
5. Make loaded replacement-artwork status explicit and document the full campaign
   pack launcher. Verify the on/off comparison with that pack.

Exercise combined snow, blood, lighting and model changes in real scenes. Check
weather bounds/shelter and footprint lifetime, gameplay with cheats disabled,
item effects and inventory edits, native and browser paths, formatting and Clippy.

Weather integration: model rebuilds now invalidate particle/surface references
without clearing accumulation; mission loads still reset it. Moving actors no
longer cut dry silhouettes into snow. Near and whole-stage shelter maps follow
precipitation direction, and a bounded distant particle layer has an F1 range
control independent of fog. Four CPU regressions cover accumulation, particle
bounds, wind entering eaves and wall shelter. Two native GPU regressions cover
coatings, original restoration, moving actor exclusion and shifted wind shelter.
Footprints now follow both players' and humanoid enemies' animated feet, project
onto actual ground triangles, and fade beneath fresh snowfall. F1 exposes a
toggle, burial time and clear action. Ground blood absorbs the snowy material
instead of revealing the substrate. Real-disc walking checks cover both players,
read-only state and bounded geometry; native GPU checks cover distant coating,
stains, footprints and restoration when disabled. Ground-projected decals sample
opacity within their triangles, avoiding pale wedges at terrain joins, and have
a small surface offset to prevent depth artifacts. Grenades, embers and kaen
contribute short-lived, clustered light sources
using actual effect lifetimes, with two priority slots in the existing light
budget. Real-disc item-use tests verify flashes appear, expire and leave game
state untouched; camera-orbit stability still passes. F1 has a mission-only
Experiment page: god mode, never-noticed stealth,
left-click teleport, per-item quantities (255 unlimited), refill/clear and armour
protection. Inventory edits preserve internal timers and stop ordinary recording;
cheats reset with a new mission. Real combat verifies both characters retain
health and hit reactions, then take damage after disabling; an alerted guard
returns idle under stealth and detects again when disabled. Inventory validation
and actual item use pass. C-oracle damage and fall comparisons still pass with
cheats disabled. Combat replays retain matching game state over 5,667 ticks for
Rikimaru and 4,834 ticks for Ayame.

F1 Visual assets reports active-mission texture/model counts, the native pack
folder and load errors. Plain play still requires TENCHU_ASSET_PACK; the existing
private/campaign-hd-pack/play.sh loads the full campaign artwork. Browser tests
exercise actual item use, god/stealth reset, repeated normal-camera teleport,
artwork comparison and accumulated snow across model changes on both WebGPU and
WebGL2, with no GPU/page errors. Mission 1 loads 327 textures and two models for
Rikimaru; switching artwork visibly changes the scene. Native scene review covers
both players' tracks, wind shelter, blood on snow and grenade/kaen light spill.
Evidence is in private/weather-followup. Workspace tests, formatting, Clippy and
native/WASM builds pass; original gameplay/default presentation remain intact.

## Persistent effects, weather, models and water (complete)

Deliver optional modern presentation features while retaining the original
default appearance and exact gameplay/collision/AI/RNG. The scope includes the
subsequently requested water improvements. Keep native, WebGPU and WebGL2
paths usable, cap memory/work per effect, and commit coherent stages.

1. **Persistent blood:** model-attached wound stains, visible bleeding and
   projected stains on actual environment surfaces. Respect animation, moving
   parts, opaque/cutout materials and replacement models; cap retained marks,
   clear/reset predictably, and avoid staining a new actor after slot reuse.
2. **Weather and sky:** configurable rain/snow and wind; shelter-aware particles,
   wet surfaces, restrained optional snow/depression coverage, and roof runoff
   where surface geometry supports it. Add a controllable sun/moon clock with
   continuous light/color/shadow changes and a timelapse option. No flooding or
   gameplay changes are required.
3. **Vegetation and characters:** improve tree/bush volume and character detail,
   using the existing HD materials and replaceable geometry. Preserve silhouettes,
   texture/UV boundaries, animation and weapon attachments. Use explicit portable
   asset metadata for authored distinctions rather than mission-specific hacks.
4. **Secondary motion:** stable visual-only hair, clothing and equipment motion,
   including Ayame. Bound displacement, handle pauses/teleports and changing
   models, and keep skinning/attachments and shadow silhouettes consistent.
5. **Water:** improve water and waterfalls with coherent flow, surface detail,
   foam/spray and optional geometry refinement that softens abrupt visual bends.
   Preserve original collision and animated texture intent; handle connected
   tiles and transparent ordering, with affordable quality options.

Each stage includes clear F1 controls, default/reset behavior, focused regression
checks and actual scene review. Verify fixed-tick state equality with effects
enabled, bounded resource use and native/browser rendering; extend the feature
showcase with useful demonstrations. Keep asset authoring/provenance private,
document genuine limitations, and continue through all five deliverables.

Blood implementation: F1 has an opt-in persistent-effects page with amount,
bleeding duration, retained-mark budget and clear action. Original damage/blood
events drive cosmetic wound placement, attached body-part decals and drips.
A presentation-only surface BVH projects environment marks onto actual triangles;
hinged doors/trapdoors keep marks in local coordinates. UVs, material cutouts,
interpolated poses and replacement meshes are preserved. Marks, vertices, wounds
and drops all have independent bounds; reloads and disabling clear the state.
No game/core changes or campaign-save format changes are involved.

Native GPU checks cover alpha blending, cutouts, occlusion, clearing and unchanged
original PS1 blend modes. Both Mission 1 combat tapes produced attached wounds
and persistent ground stains while matching the unmodified game state (Rikimaru
5,667 ticks; Ayame 4,834 ticks). Original and HD rendering were reviewed, including
moving body-part closeups. Native/WASM compilation and geometry/budget checks
pass. Browser runtime review remains part of the combined-effects stage: the
native headless GL adapter currently fails at device initialization, so that is
not evidence of a successful WebGL2 run. Stains are cosmetic and bounded, not
permanent save-file edits; complex hit locations are inferred from source effects.
Subsequent stages and combined validation are recorded below.

Time-of-day implementation: an optional F1 page drives a continuous, interpolated
sun/moon path, twilight color, separate day/night ambient levels, configurable
path orientation, sky gradient and real directional shadows. The clock follows
simulation ticks and pauses, with a midnight-safe interpolation path. An explicit
daylight-relighting control fades baked night vertex shading on textured scenery;
without this, the original assets remain visibly night-lit even under noon sun.
Turning the cycle off restores the original manual-light behavior. The built-in
`day-cycle` clip stages Mission 1 rooftops for a twenty-second day. Geometry/GPU
checks confirm moving shadows and exact restoration when disabled; an actual
scene comparison isolates shadows and baked shading. Native and WASM checks pass.

Weather implementation: an optional F1 page selects Original/Clear/Rain/Snow,
wind, density, detail, wetness, snow cover and runoff. Bounded particles stop at
static stage triangles; sloped hits create downhill droplets that spill over
edges. A separate top-down cutout-aware depth pass controls nearby surface
exposure, including moving shadow geometry. Wetness grows and dries; thin snow
cover grows and melts, favors upward surfaces, and preserves sheltered floors.
There is no fluid volume, depression filling, snow collision or footprint model.
Particle shelter checks use solid static triangles, conservatively treating
texture holes as covered; surface shading uses the actual texture masks.
All cosmetic state is independent of game RNG, bounded and reset on reload.
Original mission snow keeps updating in the core when its drawing is overridden.

Native checks cover roof/floor intersections, budgets, weather transitions,
render-cadence independence, sheltered pixels and exact restoration on disable.
The 5,667-tick Rikimaru combat tape still matches reference state with blood,
alternating rain/snow and the day clock. Mission 1 roof reviews and a 54-second
4K/60 capture exercise the new `rain,snow` clips. Chromium WebGL2 and desktop
Wayland WebGPU exercise precipitation, daylight, F1 resets and unchanged paused
state. WebGPU caught a sky-return/derivative uniformity error accepted by native
Vulkan; moving the sky return after derivative evaluation fixes it. Both browser
paths now pass without graphics errors. Animated effects can refresh between
logic ticks even when model interpolation is off; static paused views stay idle.
Native/WASM Clippy, formatting and all 42 fixture-free PC tests pass. Browser
evidence and private review captures are under `private/effects-review`.

Water implementation: an opt-in F1 page controls coherent world-space ripple
normals, sky Fresnel sheen, shadowed sun/moon glints, foam, flow and bounded mist.
Reviewed texture fingerprints distinguish water/falls from lava and mixed bank
tiles; packs can override classifications explicitly. Waterfall detail adds
four/eight-division cubic patches with pinned open edges, unchanged UVs, a 250-unit
displacement limit and a shared 262,144-vertex budget. Mist follows exposed steep
edges and downstream ends, with at most 640 particles. It is drawn after water
and before HUD overlays. Tagged water does not cast a solid curtain shadow or
receive solid-surface snow/blood. Original settings restore the original geometry.

Native Mission 1 pond and Mission 4 waterfall comparisons preserve the complete
game snapshot. Geometry checks cover pinned seams, UVs, finite normals, bounds
and budget fallback; GPU checks cover animated water-only changes and exact
restoration, plus original PS1 blends/backing faces. The `water,waterfalls` demos
provide matching comparisons. Reflection is of the sky and directional light,
not nearby scene objects; these changes do not simulate fluids or replace original
collision/swimming. Existing authored UV joins can remain visible. All 43
fixture-free PC tests and native/WASM Clippy pass. Desktop Chromium WebGPU and
WebGL2 run water together with rain/snow/daylight, reset each F1 page and preserve
paused state without graphics errors. Combined model and secondary-motion
validation is recorded below.

Model refinement implementation: F1 offers independent Original/Refined/Fine
character surfaces and Original/Fuller/Dense vegetation. Character refinement
uses bounded subdivision with welded positions, separate UVs, pinned open or
nonmanifold joints and smooth normals; it supersedes the older contour control
while active. Custom replacement characters and weapon geometry remain supplied
as authored. Explicit reviewed material metadata distinguishes leaves/grass,
dense shrubs, trunks and excluded flat bamboo stalks. Connected multi-texture
branches share their added layer transforms, while dense shrubs use rounded
surfaces instead of overlapping leaf cards. Existing original/HD artwork is reused.
No new copyrighted or generated bitmap assets are committed.

Separate 1,048,576/524,288 generated-vertex budgets bound character refinement
and vegetation. Tests cover joint/UV preservation, connected material borders,
finite geometry and budget fallback. Native Mission 1/4/5 reviews include both
players, garden context, cherry/cedar/grass/shrubs and snow-covered branches,
with exact game snapshots preserved. The `vegetation,character-refinement`
demos exercise the controls. All 45 fixture-free PC tests, native and WASM
Clippy and formatting pass. Combined browser/replay review is recorded below.
These are refinements of the original shapes, not newly sculpted models
or individual geometric leaves; original default settings remain unchanged.

Secondary motion implementation: independent F1 hair/cloth/equipment switches
drive bounded fixed-step springs on reviewed material-masked regions of Rikimaru
and Ayame. Anchored influence ramps keep seams fixed; normals, translucent parts,
attached wound stains and shadow casters follow the same deformation. Render
sampling is read-only. Teleports, changing bodies, tick gaps and disabling reset
state; equipped weapons retain their rigid original transforms. Custom geometry
requires explicit pack rules. A separate breeze animates classified foliage and
its point/directional shadows on the GPU, with fixed roots and no CPU mesh upload
per frame. Weather precipitation wind remains independently adjustable.

Motion defaults off and does not simulate individual strands, cloth collision or
anatomical deformation. Up to four rules per part and 256 active springs bound
the work. Native GPU checks exercise both real players and visible foliage/shadow
movement, fixed roots and exact original-image restoration. CPU checks cover
bounded impulses, settling, rendering cadence and resets. Both native and WASM
Clippy and all 104 fixture-free workspace tests pass. Both complete Mission 1
combat tapes match reference state with the combined effects (5,667 and 4,834
ticks), including additional intermediate motion samples. Chromium WebGPU and
WebGL2 render all effects, exercise F1 resets and preserve paused state without
graphics errors; the short 1280×724 samples reached approximately 58/60 rendered
fps respectively on this desktop. Those samples are not a broad performance
guarantee.

Combined visual review: a 56-second 4K/60 capture covers the pond, waterfall bend,
vegetation refinement, character surfaces, both players' secondary motion and
foliage breeze. All 3,360 frames decode correctly. Water glints now use slowly
varying intersecting waves, breaking up the overly uniform reflection bands
seen in the first capture. The actual-water GPU regressions and both browser
paths pass again after this adjustment. A separate twenty-second `blood` clip
stages a frontal guard encounter using ordinary combat; fixed framing with wider
clearance avoids foreground branches after knockback. Body wounds and stains on
the stone bank were inspected in the final capture. Native Clippy, demo tests,
formatting, release and browser builds pass. All five feature areas and their F1
controls/demos are implemented. Final x86_64-linux Nix desktop/browser packages,
portable WASM libraries and formatting checks pass at implementation commit
`566bf23`. The packaged demo catalog contains all 48 clips, including the new
blood, sky/weather, water, model and motion demonstrations. Private review
artifacts and machine-readable validation are under `private/effects-review`.
Original defaults and game/core source are unchanged throughout this goal.

## Effects showcase recording (complete)

Delivered `private/effects-showcase/tenchu-effects-showcase-4k60.mp4`: **3:06,
3840×2160, 60 fps, silent**, with eleven chapters covering water/falls, moving
sky, rain/runoff, snow coating, vegetation and breeze, character refinement,
Ayame/Rikimaru secondary motion and persistent blood. Comparisons switch quickly;
weather and combat have longer holds. The existing campaign HD artwork remains
constant. The frozen executable, catalog, isolated settings and asset hashes sit
beside the recording, together with a live launcher and reproducible export script.

All 11,160 presentation timestamps occur exactly once. Full decoding reports no
errors; 4K/60, progressive H.264/BT.709, silent output, fast-start layout, chapter
titles/durations and unchanged input hashes pass. Forty-four sampled frames,
four per chapter, were visually inspected. Player-shot captions now sit above
the action; fixed morning light makes character refinement and secondary motion
clearer. Weather demonstrations use the highest bounded particle detail.
Formatting, workspace Clippy and all six fixture-free demo tests pass.

The watch page at `http://127.0.0.1:44115/` passed actual browser playback and all
eleven chapter jumps without media or page errors. `youtube-description.txt`
contains the upload chapter text; nothing was uploaded. Run
`./private/effects-showcase/play.sh` for the live sequence and its clip picker,
or `./private/effects-showcase/render.sh /path/to/new-video.mp4` to export again.
The reusable `tools/verify-demo-video.py` checks any offline demo export.

## Campaign artwork showcase (complete)

Delivered `private/campaign-showcase/tenchu-campaign-hd-showcase-4k60.mp4`:
11:42, 3840×2160, 60 fps, silent. The 95 matched artwork comparisons use three
seconds per side, covering distinct scenery/gameplay from all ten missions,
characters/bosses, weapons/items/HUD and four-language menus/fonts. Short gameplay
passages use the original 30 Hz logic with interpolated presentation. Every pair
shares its camera, inputs and graphics settings; marked HUD/menu pairs also
compare outline text.

All 42,120 consecutive presentation timestamps occur exactly once. Full decoding
reports no errors; dimensions, progressive H.264/BT.709 metadata, no-audio output,
fast-start layout and all 13 chapter boundaries pass. Midpoint/end frames from
all 204 clips were reviewed, with clearer replacements for an obscured Mission 2
shot and an estate excerpt that ended after a camera cut. The frozen pack, input
recordings, catalogs, hashes and validation are beside the video. The local watch
page at `http://127.0.0.1:44113/` passed actual browser playback, six seeks and
chapter-button navigation. `youtube-description.txt` supplies grouped timestamps
for an upload; nothing was uploaded.

The demo catalog now supports animated frontend pages, explicit HUD visibility
and outline text. Menu staging is deterministic and keeps gameplay/progress
frozen; interaction is disabled during showcase playback. A disc-backed regression
checks repeatable pages in all four languages, and a 4K preview checks character,
mission-selection and options rendering. Native/WASM Clippy, formatting, demo
timeline checks and recorded original/HD state equality pass. Normal settings
remain unchanged, and all capture/encoding jobs are finished.

## Campaign-wide HD artwork and fonts (complete)

The accepted local pack is `private/campaign-hd-pack` (candidate43): 1,719 PNG
replacements, comprising 1,561 world/HUD identities and 158 interface identities,
plus the two existing Mission 1 weapon mesh replacements. It covers all ten
missions, both players and all audited outfits/actors, scenery, props, equipment,
world effects, HUD and active illustrated menus. The previous 1,544 accepted
PNGs are byte-identical. The 22 documented world fallbacks are nine fully
transparent weapon faces and thirteen anomalous/ambiguous source UV spans;
these retain their source behavior. Unused menu images, the flat records
highlight, original bitmap-font fallback and palette-animated title blood remain
original. Movies and their subtitle artwork were not redrawn.

Shared HUD/menu acceptance includes health bars, Ki glyphs, inventory selection,
particles, all ten mission previews, four-language character/mission/results
screens, title art, options/controller/volume panels and populated records.
Corrections cover explicit blend-class masks, localized text spacing, controller
callout destinations and the distinct-width French pirate preview. Outline fonts
also cover runtime titles/ranks, captions and numbers. Source prompts, family
atlases, crop recipes, fallback reasons and acceptance evidence are private in
`campaign-hd-source`; `coverage.json`, `ui-acceptance.json` and
`installed-sha256.json` record the final state. Related tiles were reviewed in
native map/model/gameplay views; original mesh stretch, UV mirroring and vertex
lighting can still expose some joins.

`./private/campaign-hd-pack/play.sh` opens the original menu with this pack and
outline text, using a separate settings file. The browser build is `private/web-hd`:
207 menu/selection files arrive at boot and 1,516 files download in the background
(143.4 MiB compressed total). Native installed-pack rendering and final WebGL2
menu/language changes pass. Both browser backends passed offline loading/rendering
of all ten missions, with exact frozen-image restoration after artwork toggles
and reload. The native all-language frontend check covers 96 page/mission
combinations without altering full gameplay/save state. Native/WASM Clippy,
formatting, focused pack/export/GPU checks and release builds pass.

The following entries retain the implementation and review history.

Browser exports now include optional asset packs selected by `TENCHU_ASSET_PACK`.
The boot set carries all menu languages and selection characters; the remaining
compressed PNGs and models download in batches in the background, before mission
entry. The same scoped loader decodes only the active mission. `build-web.sh`
refreshes artwork without re-encoding existing movies/audio, and includes the HD
font license. Native export checks cover dependency partitioning, missing files,
path escape and excluding authoring sources. Native/WASM Clippy, formatting,
pack tests and release builds pass.

Actual Chromium WebGPU and WebGL2 checks loaded Missions 1–10 offline with the
candidate campaign pack. Delayed mission artwork did not prevent the menu from
opening; toggling/reloading the pack preserved a frozen gameplay snapshot and
restored the exact HD image. Both runs completed without reported graphics
errors. Evidence is in `browser-art-webgpu-final` and `browser-art-webgl2-final`.
The first headless Chromium capture was blank; desktop Wayland rendering works.
These checks cover loading/presentation, not complete browser mission playthroughs.
The frontend regression also now captures every mission-selection preview in all
four languages. Final shared HUD/menu installation and coverage records follow.

Explicit asset-pack replacements now reach the in-game HUD as well as world
sprites. Original HUD images still bypass generic texture upscaling, lighting
and grading. GPU checks cover shared world/HUD identities, cutouts, opaque and
blended pixels, reload, pack-off restoration and the upscale-only fallback;
Vulkan and Mesa software OpenGL pass, as do native/WASM Clippy, formatting and
the release build. The headless NVIDIA OpenGL device failed during setup, as in
the earlier frontend checks. An actual Mission 1 capture confirms the new Ki,
life-bar and inventory artwork. The 18 shared HUD/effect candidates and 155
interface candidates remain under review in `private/campaign-hd-source`;
the accepted campaign pack has not yet been replaced with those candidates.
Frontend capture checks now fail on an explicitly configured invalid pack path
instead of silently producing an original-only review.

Outline lettering now also covers all ten mission titles and all five ranks in
English, Japanese, French and Italian. Records retain the source layout and
rank colors, with the confirmation footer moved below the training row. The
GPU regression now exercises populated records and results across 15 pages in
all four languages, retaining identical gameplay and save state. Glyph coverage,
native/WASM Clippy, formatting and release build pass; captures are in
`private/campaign-hd-source/frontend-label-review2`. Text baked into menu/result
background illustrations still needs replacement artwork; movie captions remain
outside this artwork pass.

Initial inventory exported all ten missions with both characters, all three
enemy layouts and all five character/outfit combinations: 50 exports and 2,057
distinct loaded texture identities. The accepted M1 pack supplies 368 of these.
This is not yet the actual-use count: unused common/weapon archive contents are
still being distinguished from required art. References and the merged catalogue
are in `private/campaign-hd-source` and `private/campaign-hd-catalog`.

Campaign loading now selects current mission texture identities and replacement
model dependencies before opening image files. Mission/outfit reload refreshes
that selection; F1 reload remains transactional. The 16M decoded-pixel bound now
applies to the active selection, and the authoring tool checks every exported
mission/outfit with `--campaign`. The loader regression covers unrelated missing
files, model texture dependencies and required missing assets. Pack-tool checks,
native/WASM Clippy and release build pass. M1 and M5 captures are pixel-identical
before/after selective loading with the accepted M1 pack. This does not yet add
browser delivery of replacement packs or HD interface rendering.

`export-interface` now inventories the title/options archives, character and
mission selection in all four languages, localized ranks/results, shared HUD
sprites and the Latin font with its original glyph advances/nudges. The disc
export contains 298 source/index references deduplicated to 248 image identities;
the private catalogue is `private/campaign-hd-interface`. Title, options, English
mission selection and font reference sheets have been inspected. Native Clippy,
formatting, release build and the complete disc export pass. Interface replacement
rendering is implemented below; most replacement interface artwork is still pending.

Optional outline text now covers HUD numbers, in-game subtitles and the quick
launcher/equipment/pause panels, including offline captures. The embedded,
OFL-licensed font has 7,193 Unicode mappings for Shift-JIS and supported Latin
languages, with a pinned, reproducible subset recipe and Nix license packaging.
The setting defaults off. Real captions in all four languages retain identical
game state and world geometry; 720p/4K captions and launcher layouts were checked
and HUD captures inspected. Native/WASM Clippy, font tests and release build
pass. Original frontend lettering baked into artwork and movie subtitles remain
original; they are not covered by this first font implementation.

The frontend now accepts scoped PNG replacements using the exported identities
and renders sprites/selection models on the GPU at window resolution. Pipeline
creation is lazy; the unmodified default keeps its original raster path. PS1
opaque/average/add/subtract/quarter blends, cutouts, crop regions and transitions
are retained, with atlas-region clamping for filtered replacements. Language
changes and F1 reload refresh the interface selection; missing edits preserve
the previous successful reload. Sharp text also covers title choices, character
headings, options, controller bindings and numeric menu values. Baked background
lettering, mission/rank images and movie captions still need their separate pass.

Three GPU regressions cover all blend modes against the old rasterizer, 4K
replacement detail plus atlas-row isolation, and 14 menu pages in all four
languages without changing full gameplay state. The latter records original,
sharp-text and candidate-art captures in `private/campaign-hd-source/frontend-gpu-review`.
Vulkan and native/WASM Clippy pass. Blend/detail tests also pass with Mesa
software OpenGL; the headless NVIDIA OpenGL device failed during setup before
the new frontend pipeline was created. Actual browser delivery of packs remains
pending. The first reviewed interface replacement is the subdued burgundy silk
options background, with exact generation provenance and original blend classes.

The separate `private/campaign-hd-pack` contains the 368 accepted M1 textures,
64 additional Mission 2 scenery textures and one interface background. The M2
scenery families were inspected in 256 matched camera pairs: roofs, plaster,
foundations, streets, timber, brickwork and patterned shop screens. Coarse gravel,
incorrect roof repeat counts and a mismatched gutter corner were rejected and
revised. The large circular screen was assembled before repainting, then cut
with texel-center edge padding. The final scenery selection is candidate08;
exact prompts, original/generated references and crop recipes remain private.
Original alpha classes are checked, with explicit authored thatch cutouts.
M1 images remain byte-identical and its recordings remain intact. Existing
geometry stretch and baked vertex lighting can still expose some joins.

A conservative usage audit now distinguishes 1,583 reachable world/UI identities
from the 2,057 loaded identities. It follows actor weapon attachments and actual
door/pitfall/item mesh references, retaining all actors/outfits, stage objects
and shared interface/effect sprites. It does not claim every retained resource
has appeared in a playback. The private `usage.json` records every inclusion and
exclusion per mission/outfit. Mission 2 still needs its additional actors, props,
equipment and shared interface artwork; other mission families remain pending.

Mission 2's additional 87 actor, nine prop and two equipment identities are now
reviewed and installed. The campaign pack contains 530 world/UI replacements
plus the first interface background. Seventeen posed models were checked from
six angles, including cutout baskets/fences, all six new character families and
four weapon meshes. The hooked weapon's pixel steps were repainted as smooth
metal before acceptance. Ten matched gameplay captures cover both mission
routes through completion with identical state traces and 100 HP. All M1 PNGs
remain byte-identical. Mission 2 now covers 369 of 387 conservatively reachable
identities; the remaining 18 are shared HUD/interface images, handled separately.
This also supplies shared characters to Mission 7. Native map/model evidence,
exact source prompts and acceptance records remain in the private campaign
directory; browser delivery and the remaining missions are still pending.

Mission 3 adds 105 reviewed identities: 36 scenery, 63 actor, three prop and
three equipment images. Rock faces, prison timber, tree bark, foliage and ground
were authored as related atlases. The first leaf-litter pass exposed repeated
dark islands; a second pass reduces that contrast. The narrow iron trim uses
extended color beneath its original alpha mask, and the bear's mouth was
corrected separately after the first repaint wrongly added fur inside it.
The evidence includes 144 map camera pairs, 36 revised-ground pairs, ten models
from six angles, and twelve original/replacement campaign playback endpoints.
Both characters complete Mission 3 with matching traces; combat, item effects,
the bear, Goo and prison scenes were inspected. Original mesh faceting, UV scale
changes, vertex colors and some coarse original cutout masks remain visible.

The accepted campaign pack is candidate16: 635 world/UI images plus one menu
background, 25.66M decoded world pixels globally and 14.70M at most in one active
mission/outfit. Missions 1–3 now cover all audited world images, leaving the same
18 shared HUD/interface identities for the UI pass. Mission 4's remaining audit
contains 73 scenery, 39 prop, 50 actor and two equipment images, plus those 18
shared interface images. Missions 4–10 and the remaining menu art/browser pack
delivery are still active work.

The Mission 4 material preview exposed an audit-tool omission: translucent
scenery was absent from its texture search and captures. Previews now include
those faces and sort them for each camera using the same helper as model
reviews. All 73 new Mission 4 scenery identities now produce four original/pack
view pairs, including path/terrain overlays and the scrolling waterfall.
Gameplay rendering is unchanged. Formatting, workspace Clippy, the cutout-ray
regression and the release build pass.

Mission 4 adds 164 reviewed world identities: 73 scenery, 39 prop, 50 actor and
two equipment images. Matching atlases cover the checkpoint timber, paving,
bamboo, cedar, ground transitions, shrine objects, clothing and armor. Prominent
cliff fractures were revised into quieter stone grain after the original UV
mirroring produced repeated symbols. A mismatched moss transition was repainted
separately. The gate labels use the original Japanese text with licensed outline
glyphs fitted to their stretched UV proportions; the blade tip and foliage use
reviewed authored silhouettes.

Evidence includes 292 map pairs, 48 revised-cliff pairs, nineteen posed models
from six angles, twelve gameplay endpoints, and river/waterfall captures across
five moving phases after the opening scene. Both characters complete their
recorded Mission 4 routes with identical original/replacement traces. The river
uses the shared world-mapped water material; the waterfall retains its own
vertical scrolling. Original mesh faceting, UV scale and vertex-lighting joins
remain. The accepted pack is now candidate19: 799 world/UI images plus one menu
background, 34.10M world pixels globally and 14.72M at most in an active outfit.
Missions 1–4 cover all audited world artwork, with the eighteen shared HUD images
still pending. Mission 5 begins with 252 additional world images; Missions 5–10,
remaining menu artwork and browser pack delivery remain active work.

Mission 5's larger candidate exposed an overbroad pack selection: it included
every image recorded in VRAM, including unused archive material, instead of the
mission's actual sprite list. Selection now matches the exported model/sprite
identities. The disc regression compares those sets for Mission 5; pack tests,
native/WASM Clippy and the release build pass. The 16.12M-pixel candidate now
loads for native map/model reviews without exceeding the existing limit.
Mission 5 artwork is still under review, particularly cutout shapes and joins.

Mission 5's accepted pack is candidate24, adding 252 world identities: 120
scenery, 66 prop, 64 actor and two equipment images. Snow transitions reuse the
adjacent soil material with the original coverage boundaries. Roof fields were
revised for quieter contrast; the gold mural and storage shelves were assembled
before splitting at their original UV endpoints. Reviews also corrected missing
wall timbers, misplaced robe details, window alpha, an incomplete bow texture,
and overly sparse foliage. The dojo lettering and stone-basin engraving remain.

Evidence includes 480 first-pass map pairs, 128 revised pairs, 28 final context
pairs, 38 models captured from six angles, and twelve gameplay endpoints across
both characters' completed routes with identical original/replacement traces.
The new Mission 5 identities contain no scrolling textures. The shared water
material remains in place. Original mesh faceting, UV scale/mirroring and baked
vertex lighting can still expose joins. Private prompts, crops, rejected passes
and the acceptance ledger are retained; all previously accepted PNGs are byte
identical. The pack now has 1,051 world/UI images plus one menu background,
43.93M world pixels globally and 16.12M at most in an active mission/outfit.
Missions 1–5 cover all audited world artwork; their eighteen shared HUD images
remain for the UI pass. Mission 6 begins with 98 new world images, while the
remaining menus, fonts and browser pack delivery remain active work.

Mission 6 adds 98 reviewed world identities: 41 scenery, twelve prop, 38 actor
and seven equipment images. The temple timber, roofs, stone idol and ground
transitions use matching families. The temple sign was joined before repainting
and split with padding at the original UV endpoints. Grilles, railings, torii,
pine, hanging paper and weapon cutouts retain their original blend classes.
Review corrected a cropped warrior face and a mistaken skin interpretation of
the cult leader's red hood and robe. One scrambled original sole UV region uses
plain sole skin in the optional pack; original geometry and UVs are unchanged.

Evidence includes 164 map pairs, 27 models captured from six angles, a final
six-angle robe review and twelve gameplay endpoints across both completed routes.
Original/replacement stdout traces match; the new images contain no scrolling
textures. Original mesh faceting, UV scale and baked-lighting joins remain.
Candidate27 is installed: 1,149 world/UI images plus one menu background,
47.92M world pixels globally and 16.15M at most in an active mission/outfit.
All 1,052 previously accepted PNGs remain byte-identical. Missions 1–6 now cover
their audited world artwork, leaving eighteen shared HUD images. Mission 7 has
124 remaining world identities; Missions 7–10, menu artwork and browser pack
delivery remain active work.

Mission 7 adds 111 reviewed identities: 42 scenery, eight prop, 57 actor and
four equipment images. Ninety-five are new repaints; twelve Balma body/wrap
pieces and four equipment pieces adapt previously authored material grain while
retaining original broad shading, tattoos and masks. The image service rejected
combined limb/weapon atlases; those attempts and the reuse recipe are recorded.
The ship's deck, carved hull, rigging, rails, sails and harbor curtains use
related families. The blue curtain's first assembly exposed swapped lower UV
pieces; the corrected joined repaint now connects on the actual map. Skeleton
art retains its mixed opaque/semitransparent classes and reviewed cutouts.

Evidence comprises 168 map pairs, sixteen corrected-curtain pairs, twelve models
from six angles and twelve gameplay endpoints. Both complete Mission 7 routes
have identical original/replacement stdout traces and finish with 100 HP.
Nine all-transparent equipment identities remain original; the visible saber
uses vertex colors. Four off-map `MOKOKU3` prop identities also remain original:
their crossed planes show unrelated atlas fragments even without replacements.
Their context is captured near x=74000, z=48000; this pass does not repair those
source UVs. These exceptions are explicit in the private acceptance ledger.

Candidate29 is installed: 1,260 world/UI images plus one menu background,
51.90M world pixels globally and 16.19M at most in an active mission/outfit.
All 1,150 previously accepted PNGs remain byte-identical. Mission 7 covers 379
of 410 conservative identities; its remaining 31 are the thirteen source
fallbacks above and eighteen shared HUD images. Mission 8 begins with 46 world
identities. Missions 8–10, menu artwork/fonts and browser pack delivery remain
active work; original geometry, UV scale and vertex-lighting joins still apply.

Mission 8 adds 38 reviewed identities: fourteen scenery, four prop, nineteen
actor and one equipment image. Related atlases cover the autumn cliffs, leaf
transitions, cave stone, foliage, medicinal flower and Tengu clothing/head.
Three hand pieces reuse authored skin grain while preserving source landmarks.
The staff rings, flower petals and foliage have reviewed cutout silhouettes;
Tengu's hair retains its original mixed transparency classes.

Evidence includes 56 map pairs, eleven models from six angles and twelve
gameplay endpoints through both completed routes. Original/replacement stdout
traces match: Rikimaru finishes at tick 8,448 with 74 HP and Ayame at tick 8,126
with 56 HP. Seven common door/pitfall images retain original blank fill or atlas
fragments; one Tengu sleeve triangle retains an unrelated 208x6 source VRAM
strip. These eight fallbacks are recorded explicitly, alongside the eighteen
shared HUD images still awaiting the interface pass.

Candidate30 is installed: 1,298 world/UI images plus one menu background,
53.40M world pixels globally and 16.21M at most in an active mission/outfit.
All 1,261 previously accepted PNGs remain byte-identical. Mission 9 begins with
98 additional world identities. Missions 9–10, menu artwork/fonts and browser
pack delivery remain active work.

Mission 9 adds 97 reviewed identities: 26 scenery, 27 prop, 42 actor and two
equipment images. Related atlases cover raked gravel, castle stone, timber,
screens, gold walls, statues and the three new enemy costumes. Twenty-one
originally solid statue pieces receive subtle authored bronze grain. The first
Onikuno face repaint added a second mouth; the corrected single mouth has been
reviewed on both models. One original Onikerai underside-of-head quad retains
its unrelated source atlas span as an explicit fallback.

Evidence includes 104 map pairs, twelve models from six angles and twelve
gameplay endpoints through both completed routes. Original/replacement stdout
traces match; Rikimaru finishes at tick 6,292 with 3 HP and Ayame at tick 5,741
with 67 HP. This checks recorded traces, not full serialized-state equality.
Original UV mirroring, scale, faceting and vertex lighting still expose joins.
Candidate32 is installed: 1,395 world/UI images plus one menu background,
56.28M world pixels globally and 16.24M at most in one active mission/outfit.
All 1,299 previously accepted PNGs remain byte-identical. Mission 10 has 148
additional world identities; shared HUD/menu artwork, fonts and browser pack
delivery remain active work.

Mission 10 adds 148 reviewed identities: 83 scenery, thirteen prop, 44 actor
and eight equipment images. The large floor seal and snowy paving were assembled
before repainting and split along their original UV orientations with edge
padding. Six snow/soil transitions reuse adjacent authored materials beneath
the original coverage boundaries. Portal and lava images wrap at both edges.
Review replaced overly coarse dungeon grain with quieter stone and restored
color beneath the skull door's original mixed-transparency mask.

Evidence includes 332 map pairs, forty revised-dungeon pairs, 25 models from
six angles, twelve gameplay endpoints and forty scrolling-material pairs across
five phases. Both recorded final missions complete with matching stdout traces:
Rikimaru at tick 15,082 with 30 HP, Ayame at tick 16,015 with 7 HP. This does not
compare full serialized state. Original geometry, UV scale, vertex lighting and
the seal's coarse cutout mask still expose some boundaries.

Candidate35 is installed: 1,543 world/UI images plus one menu background,
63.11M world pixels globally and 16.27M at most in an active mission/outfit.
All 1,396 previously accepted PNGs remain byte-identical. Missions 1–10 now
cover their audited world art except the 22 documented source fallbacks in
Missions 7–9. Eighteen shared HUD identities, remaining menu artwork/font work
and browser pack delivery remain active work.

Extend the optional, reviewed Mission 1 pack through Missions 1–10 and the
menus/interface. Keep original artwork as the fallback and preserve gameplay,
UV meaning, transparency, animation and the current default experience.
Cutscene artwork is explicitly exempt from this repainting pass.

Deliverables, in implementation order:

1. Export both characters and all enemy layouts for every mission; account for
   outfits, bosses, props, equipment and effects. Deduplicate by content identity
   and distinguish actual use from unused archive contents. Inventory menu/UI
   artwork separately, including all four languages.
2. Make campaign-sized packs practical: load the assets needed by the current
   mission/interface, preserve reload and fallback, and keep native/browser
   resource use bounded. Reuse accepted Mission 1 assets and provenance.
3. Add optional high-definition fonts with real glyphs, accents and Japanese
   coverage. Preserve original text, placement, timing and hit targets; review
   menus, equipment, HUD/results and captions at normal and 4K window sizes.
4. Author the remaining mission families in coherent groups. Use assembled
   reference surfaces for adjoining roof/gable/wall/ground tiles; preserve
   cutouts, terminal edges, scrolling periods and animated-family continuity.
   Review actual stage geometry before declaring a family complete.
5. Upgrade menu and interface artwork without repainting letters as guessed
   symbols. Include title/character/mission/options/equipment/results screens;
   preserve the quick launcher and existing navigation.
6. Complete a per-mission/per-menu coverage ledger and before/after gallery.
   Exercise both characters, outfits, enemies/bosses, item effects, material
   joins and water/weather motion. Check original fallback, pack reload,
   native/browser builds and meaningful rendering/gameplay regressions.

Commit coherent stages and continue through the whole scope. Keep generated
artwork, disc-derived references, candidate history and exact provenance in
ignored `private/` directories. The completed Mission 1 pack and recordings
remain intact while the campaign pack is built. Existing decompilation,
research and viewer repositories remain read-only references.

## Complete Mission 1 texture pack

Faster follow-up recording is complete at
`private/m1-showcase-v2/tenchu-mission1-showcase-v2-4k60.mp4`: 156 seconds,
26 matched original/replacement pairs, three seconds per side instead of ten.
The latest pack is frozen beside the catalog, including the terrain/eaves/roof
refinements below. Fourteen additional comparisons cover garden paths, ground,
blossoms, lanterns, eaves, interior materials, props, guards and character detail.
Gameplay remains at its original speed; item excerpts start near the useful
effect timing. Two obstructed candidate cameras were reframed before capture.
The silent H.264 High/BT.709 MP4 contains all 9,360 unique consecutive 60 Hz
presentation timestamps at 3840×2160 and decodes completely without errors.
Middle/end frames from all 52 shots (104 frames) and a 4K caption crop were
inspected. All paired cameras/inputs/settings match and capture hashes verify.
The MP4 has 52 per-shot chapters; the YouTube description groups them into
13 twelve-second chapters. The review hub features this version and preserves
the earlier recording. Authoring/render/review scripts and validation remain
beside the private video; no game or renderer code changed for this edit.

Boundary follow-up: the map review covers all 100 scenery texture identities and
an audit of 6,780 shared coplanar UV/material edges within exported map parts.
The audit prioritizes inspection rather than declaring every material change a
defect. Actual stage captures identified inconsistent path-corner stones,
garden ground transitions and an eaves-corner stripe phase mismatch. Twenty
revised textures are installed: nine garden ground, nine path/paving and two
eaves pieces, authored as related atlases with original alpha and dimensions.
Optional `uv_tile` extends full-period mapping to static replacement tiles;
two repeating roofs opt in while terminal roof pieces retain their edge artwork.
Its GPU regression fails before the correction (86-level jump across the join)
and passes after it, including filtering, reload and exact disable restoration.
The 22 changed materials were inspected in 88 matched map views, plus 12
three-way original/previous/revised gameplay captures. Both characters retain
identical complete state traces through Mission 1 completion. Original mesh
distortion and baked vertex lighting still expose some joins; repainting does
not remove those. `private/m1-boundaries/index.html` provides the comparisons;
its provenance ledger records accepted/rejected artwork, exact prompts and
reproducible crops. The earlier video retains its exact capture pack under
`private/m1-boundaries/previous-pack` and predates this refinement.
All 18 Vulkan renderer checks pass (the disc-dependent shadow check was rerun
with its required `TENCHU_DISC` after the first invocation omitted it). Both
static/animated tiling checks also pass on OpenGL. Pack-loader/tool checks,
formatting and native/WASM Clippy pass; release and browser builds pass.

Follow-up: the white cliff patch visible in the item-use comparisons was three
untextured backing quads exactly coincident with an earlier textured quad in
Mission 1 map part 70. The opaque depth test now preserves the earlier face on
equal depth, matching the original ordering-table tie order. Transparent texels
still reveal backing surfaces; effect blending retains its existing depth rule.
The synthetic regression failed before the fix and passes on Vulkan and OpenGL
after it, as do all 17 Vulkan renderer checks. Native/WASM Clippy and formatting
pass. All 21 item-use comparison pairs were refreshed and retain
identical original/pack gameplay traces. The repaired original and pack views
were inspected. The requested before/after recording is complete (details below).

Dynamic-light follow-up: gameplay light/shadow selection now follows player
position instead of the orbiting camera. The Mission 1 regression preserves the
selected lights across 24 camera angles in a scene where the old selection swaps
sources. Free-camera and cinematic selection still follows those views. Native
and WASM Clippy pass; configured light/shadow budgets and gameplay are unchanged.

Recording preparation adds authored camera paths, fixed artwork states and
self-contained input excerpts to custom demo catalogs. The excerpt regression
compares complete gameplay state with direct replay before/after toggling art;
all seven demo tests pass. Native/WASM Clippy and release build pass. The final
video includes the subsequently reviewed gable, water, face and weapon fixes.

The animated seam report refers to water, not player models. Replacement water
UVs previously omitted one source texel per mesh tile; scroll phases then made
the skipped strip visible at joins. Replacement scrolling now spans one complete
period across each tile while retaining the original 32-texel animation period.
The adjacent-tile GPU regression fails before the fix (40-level channel jump)
and passes after it on Vulkan/OpenGL, alongside phase/reload/disable checks.
Original artwork retains its previous mapping. Native/WASM Clippy and release
build pass. The assembled gable, face and weapon follow-ups are installed below.

Weapon review found that the player's blades are predominantly untextured,
especially Ayame's kodachi. Texture replacement alone cannot modernize those
surfaces. The isolated model preview now uses the same OBJ replacement path as
gameplay, including transparency and material binding, and fits both versions
with one camera. Six original Ayame preview images remain pixel-identical;
authored weapon candidates visibly change all six comparison angles. Native
Clippy, formatting and release build pass. The attachment/animation review and
installation are complete, as recorded below.

Installed follow-ups: ten gable textures now share weathered ceramic/plaster/
timber styling. The attic was painted as one assembled surface and cut back into
its original UV pieces with texel-center bleed, preserving the frame corners.
Ayame's normal/alternate shared mouth patch was corrected and reviewed on both
assembled outfits. The optional pack includes two authored player weapon meshes:
1,914 triangles for the ninjato and 1,298 for the kodachi, retaining exact original
local bounds and original attachment transforms. Six-angle comparisons and
in-game attacks were reviewed; both characters' attack replays retain identical
complete state traces when the pack is toggled. Original assets remain the
fallback. The model OBJ/round-trip regressions and native/WASM checks pass;
the browser distribution was rebuilt. The current sources, rejected candidates,
prompts and generator are indexed in the private corrections provenance ledger.
The four-minute before/after recording includes both weapons during attacks.

Recording review exposed a separate pond issue: its static base and edge artwork
use independently rotated/cropped mesh UVs, beyond the scrolling canal fix.
Optional `surfaces.world_tile` now projects a replacement in world X/Z with
artist-specified periods. The pond base/edge share a continuous pattern and its
glints retain their original scroll phase. No game texture identities are built
into the renderer. The regression compares flipped/cropped UV layouts, live
scrolling, reload and exact disable restoration on Vulkan and OpenGL; all 17
Vulkan renderer checks pass. Native/WASM Clippy and the rebuilt browser package
pass. The corrected bridge view was inspected; phase continuity has a GPU
regression.

Completed recording: `private/m1-before-after-video/tenchu-mission1-before-after-4k60.mp4`
is 240 seconds, silent H.264 High at 3840×2160, 60 fps, BT.709 and square pixels.
All 14,400 presentation timestamps occur exactly once at consecutive 1/60-second
intervals. The 12 original/replacement pairs use matching settings, camera paths,
inputs and timing; scenery, both players, interiors and three items are included.
All 24 embedded chapters match the adjacent YouTube chapter text. Two decoded
moments per clip (48 frames) were visually inspected, including the final shot
of each clip. `validation.json`, paired-shot checks, capture input hashes and
the reproducible catalog/input excerpts remain beside the video. The review hub
links the MP4 and ready-to-paste description; nothing was uploaded.

Extend the optional native pilot into a complete Mission 1 texture pack. Check
material intent against actual geometry and gameplay, preserving UV layout,
transparency, animation, game logic and the unchanged default renderer.

Completed: 368 scoped world texture identities, with the optional pack and
review hub available locally. Two optional player weapon meshes add detail;
other meshes and interface artwork retain their original assets.

- [x] Correct the water replacement and verify the canal in-game. The pilot
  misclassified its grey ripple image as masonry; that candidate is withdrawn.
  The corrected water also follows the original texture scrolling. Canonical
  content identity is stable when first seen mid-animation; the GPU shifts UVs
  without rebuilding the replacement. Canal comparisons at ticks 0/24/64 show
  moving ripples, with original fallback and reload covered by rendering tests.
- [x] Inventory all Mission 1 materials across both players and their equipment,
  including scenery, props, foliage, actors, weapons, items and effects. Record
  usage, blend/alpha classes and review status rather than guessing from thumbnails.
- [x] Author coherent building groups, roads, water and landscape materials;
  verify repeated edges and all distinct uses in actual stage views.
- [x] Complete props, foliage, character/equipment and effect artwork while
  preserving silhouettes, UV islands, readable identity and animation.
- [x] Package with explicit coverage accounting and reproducible provenance;
  validate pack memory, performance, reload and exact restoration when disabled.
- [x] Review the whole mission and both player variants, document limitations,
  format/check and commit reusable tooling and results. Do not count a resized
  original or an unreviewed candidate as a completed replacement.

Sources, references, generated images, prompts and review captures remain under
ignored `private/`. The original ten-image pilot is the starting point, not
evidence of full coverage or correct material interpretation.

Current progress: both character exports with all three enemy layouts, armour
and alternate outfits merge to 422 distinct 3D texture identities, including
100 used by scenery, plus 62 shared sprite images. The searchable
catalogue records models, triangles, original alpha/blend, map placements, water
attributes, scrolling and sprite archive indices. The shared sprite archive
includes interface art and effects used in other missions; actual effect usage
still needs review. Its presence in the export is not a completion denominator.

The native pack now contains replacements for all 100 scenery texture identities
(9,535,488 decoded pixels): roofs/gables, timber/plaster, doors/windows, tatami,
interior paintings and trim, roads, moss/petals, pond layers and scenery lantern
surfaces. Each material was reviewed in paired stage views; the gallery has 400
pairs. Some artwork is an interpretation of ambiguous low-resolution details,
and broader whole-stage review remains. The carved transoms and pond glints use
authored cutouts to avoid retaining coarse original holes. Preview cameras trace
through texture holes so layered pond surfaces can be reviewed.
The 100-material native walk/pause/toggle/reload check restores identical paused
pixels at tick 81. The gallery and existing pilot launcher use this expanded pack.
The subsequent prop/foliage pass brings the pack to 135 reviewed identities and
10,846,208 decoded pixels, covering all scenery and placed-prop catalogue entries.
It adds finer leaf/blossom cutouts, tree bark, barrels, straw bales, pottery,
treasure chests, lantern panes/framework and folding screens. Two scroll variants
share the authored painting after checking their original layout; each retains
its original mask. The gallery now has 540 paired stage views. The 135-material
native check also restores identical paused pixels at tick 81 after reload.
These remain partial results: actor/equipment and world-effect artwork remain,
as does whole-mission review. Their loaded catalogue includes outfit variants;
interface-only and other-mission effect images still need explicit scope review.

World sprite replacements now preserve cutouts and blend classes while shared
HUD images remain original. Vulkan and OpenGL checks cover filtering, live reload
and exact disable restoration; all 16 Vulkan renderer regressions pass. The pack
tool also supports explicitly authored cutout masks for finer foliage and water
glints, retaining the source's nonzero blend class. Original masks remain the
default, with a coverage check to reject missing paint under their visible pixels.
Native and WASM Clippy, synthetic pack-mask checks and all four x86_64-linux Nix
checks pass. Full prompts, generated atlases, exact crop rectangles, original
references and rejected alternatives stay in the private authoring directory.

Actor/equipment authoring now has `preview-models`: assembled idle actors and
standalone equipment, six camera angles, texture identity lists and original/pack
pairs through the normal renderer. The first Mission 1 normal-Rikimaru export
renders 91 models (546 view pairs). The shared capture setup preserves eight
existing scenery captures byte-for-byte, and the cutout visibility regression,
native/WASM Clippy and release build pass. Shared texture metadata now retains
all outfit labels and per-export triangle counts instead of silently replacing
the earlier label. This caught missing face-patch attribution during authoring;
an early generated mask incorrectly included eyes and remains a rejected private
candidate, not part of the stable pack. The corrected normal Rikimaru outfit,
his armour and normal Ayame outfit now bring the stable pack to 200 reviewed
materials (11,919,360 decoded pixels), with 18 model view pairs. The 158-material
normal-Rikimaru native walk/toggle/reload check restores identical paused pixels
at tick 80. The subsequent actor pass completes all 174 actor texture identities
in the merged Mission 1 catalogue: all five player outfits, both guard bodies,
Echigoya, Hanbe, both civilians, dogs and cat. The stable pack now has 309
reviewed materials and 13,584,896 decoded pixels, with 84 model view pairs.
These preserve the original meshes, UVs and masks; skin, garment patterns and
fur are authored interpretations of the original low-resolution art. Shared
hand textures are reused where their UV layouts match. Equipment, world effects
and whole-mission review remain; isolated model views are not that final review.
The global common/weapon model archives also contain assets not placed in this
mission, some referencing VRAM that belongs to other missions. Their raw loaded
texture count must not be mistaken for actual Mission 1 replacement coverage.

Exports now include `mission_usage`: loaded actor weapon kinds and environmental
placements with their door/pitfall model and sprite indices. Merged catalogues
retain this per export. Both fresh normal-player exports preserve every earlier
PNG byte-for-byte (407/411 images), and the synthetic merge regression, native
Clippy and release build pass. Mission 1 places common door models 0/1 and 29/30;
the actor definitions and core weapon attachment rules identify its weapon set.
Together with common item/effect models, this leaves 15 actual 3D textures to
author after the 309-material pass. The other 98 loaded 3D identities have no
use in this mission; their exact IDs and source evidence are recorded privately.

The equipment pass brings the private pack to 324 reviewed 3D identities and
13,995,904 decoded pixels. It covers the placed doors, weapon attachments,
decoy, blood pool and blob shadow; the model gallery has 180 paired views.
A polearm candidate with misaligned painted edges was replaced with neutral
brushed steel inside its original cutout. Whole-mission review remains.
`preview-models --background grey` makes dark cutouts visible, and
`preview-sprites` exercises all five original blend modes against black, grey
and white backgrounds through the actual world-sprite renderer. Nine baseline
pairs remain pixel-identical without matching overrides, and the Vulkan
world-sprite/HUD/reload regression passes. The scoped remaining artwork is
44 world/item/effect sprites; the other 18 archive images are interface-only,
unused or for other stages, with the source-based scope recorded privately.

All 44 scoped world sprites now have reviewed candidates, bringing the private
pack to 368 identities and 15,738,240 decoded pixels. The sprite gallery has
132 original/pack pairs across three backgrounds and five blend modes.
Particle alpha needed a separate packaging conversion: soft coverage is
premultiplied into RGB over black while PNG alpha retains the PS1 blend flag.
This avoids hard, bright edges caused by treating glows as foliage cutouts;
the synthetic regression checks both conversions. Paired blood/stain sprites
share exactly the same authored silhouettes. Two atlas crops were realigned to
remove neighboring item fragments, and the flame atlas was toned down after
review. The final private coverage ledger accounts for all 484 loaded identities:
368 replacements, 98 model textures not used by Mission 1, and 18 excluded
interface-only/unused/other-stage sprites. No in-scope identity is missing.

Final review includes 540 scenery/prop pairs, 180 assembled-model pairs, 132
sprite pairs and 36 gameplay/cinematic pairs spanning both full Mission 1
controller routes. Both routes finish with exactly matching complete state
traces with and without replacement artwork (Rikimaru: tick 5,667, 64 HP;
Ayame: tick 4,834, 94 HP). Separate item-use captures exercise smoke and fire
in gameplay. Both native walk/pause/disable/reload checks restore identical
paused pixels and exit normally after repeated Escape, at ticks 81/80.

The pack occupies about 30 MB of PNGs and 15,738,240 decoded pixels. At 4K on
the GTX 1080 Ti/Vulkan, GPU timestamps for a fixed paused start view measure
1.73–1.76 ms average with replacements versus 1.05 ms without, with matching
filtering and fog disabled. These are GPU scene timings on an isolated X server;
its expensive presentation copies do not establish desktop frame rate. The
private performance record includes settings and before/after captures.
All 16 Vulkan renderer regressions pass, including scrolling water and world
sprites. The disc-dependent character-shadow check was rerun successfully after
supplying its required fixture. Native Clippy, formatting, synthetic pack checks
and all four x86_64-linux Nix checks pass. Other build platforms and browser
loading of this private pack were not exercised. All source images, generated
atlases, prompts, crop corrections and review evidence remain ignored/private.

## Mission 1 material pilot

Try a small optional high-resolution scenery pack using original texture images
and actual stage views as generation references. Preserve gameplay, geometry,
UVs and default appearance. Keep disc-derived references and candidate artwork
private; commit reusable authoring tools and evidence.

- [x] Select ten frequently used roof, timber, plaster, earth and stone images;
  capture their actual surfaces at several angles in the shared renderer.
- [x] Generate color candidates and a roof height candidate with the built-in
  image tool; retain prompts, sources and original transparency classes.
- [x] Package at 512×512 and compare all ten textures in the game. Thirty paired
  captures verify that disabling the pack restores the original pixels exactly.
- [x] Check authored roof relief and ordinary native play/reload, provide an easy
  launcher and local comparison gallery, and record visual limitations.

The pilot uses eleven image generations for ten color images and one aligned
roof height image. Its native pack has 2,883,584 decoded pixels. Review tooling
finds textures by content identity and stage geometry without runtime material
name rules. Full-resolution generation sources and prompts are retained in
`private/m1-hd-source`; the optional pack is `private/m1-hd-pilot`.

Completed: all thirty final views are unobstructed, change with the pack and
restore pixel-identical originals when disabled. Original alpha/blend classes
are retained in all ten PNGs. An isolated native session walks, pauses at tick
81, toggles artwork through F1, reloads and restores the exact paused image.
The separate roof height map changes close/angled renders and fades at distance;
its effect is subtle at the pilot's conservative strength. Most of the visible
improvement comes from the color artwork. Native pack checks, Clippy and all
four x86_64-linux Nix checks pass. The local comparison page loads all thirty
view pairs and its original/pack/split controls pass browser checks.

Run `./private/m1-hd-pilot/play.sh` to try the optional native pack using the local
disc, or supply a disc path as its first argument. F1 → Visual assets → Replacement
artwork compares live. `private/m1-hd-review/index.html` is the local review page.
The normal launcher and user preferences are unchanged. This is a ten-texture
art experiment, not a complete mission remaster: foliage, characters, roof trim
and many props remain original. Generated painted shading, imperfect repeated
edges and some layout/palette drift still need curation. No geometry, normal/
roughness material importer or browser asset-pack support was added.

## Browser frame-rate follow-up

Target steady 60 FPS in Firefox WebGL2 at 4K on this desktop, preserving the
Original preset's pixels, full render resolution and fixed-rate game logic.

- [x] Establish repeatable browser measurements for stationary and moving views,
  separating CPU preparation/submission, GPU rendering and browser presentation.
- [x] Remove measured bottlenecks with simple shared-renderer or browser-host
  changes; compare each candidate against the same scene and settings.
- [x] Check actual rendered images, transparent textures, HUD, lighting and preset
  changes; exercise multiple missions and native/WebGPU regressions.
- [x] Document before/after timings and remaining limits; format, run Clippy and
  Nix checks, and commit validated improvements.

Profiling corrected the initial headless result: Firefox 147 release on the
GTX 1080 Ti/NVIDIA 580/Wayland desktop renders about 24 FPS at 3840×2124 on a
60 Hz display. `about:support` identifies `DMABUF_WEBGL` as blocklisted. The
browser profiler spends substantial time uploading the canvas to its compositor;
the game callback and a GL completion wait take only a few milliseconds.
Omitting only the final present blit reaches 60 callbacks/second but freezes the
display, so that experiment is diagnostic evidence, not a shipped optimization.
Nightly 160 reaches 60 rendered FPS at the same resolution with Original plus
model interpolation; Mozilla enables the newer-driver sharing path on Nightly.
The final fullscreen run also renders 300 frames in five seconds at 3840×2160,
both in normal play and during free-camera movement, with about 1.1 ms mean CPU
callback time and the simulation still at 30 Hz. Its `DMABUF_WEBGL` is available.
No resolution reduction, browser blocklist override or game-rate change is used.

Implemented: retain unchanged browser frames between original ticks and while
paused, cache unchanged egui shapes, count actual renders for FPS, and separate
callback/render/tick rates in repeatable browser measurements. Native vsync
pacing remains intact. F9 explicitly renders a fresh frame before downloading.
A separate repeated-start hang was traced to CPAL querying Firefox's synchronous
`outputLatency` getter. Only the game output context replaces that unused
callback-timestamp estimate; audio scheduling still uses its original clock.
Regression checks exercise retained images, paused screenshots, menu/resize
updates, audible PCM and absence of hardware-latency getter calls.
Firefox 147/160 checks, Chromium WebGL2/WebGPU mission 1/5 checks, 28 fixture-free
PC tests, native/WASM Clippy and all four x86_64-linux Nix checks pass. Firefox 147
release's full-resolution presentation ceiling remains an external limitation;
the documented separate Nightly profile is the measured 4K/60 route.

## Browser and surface-detail follow-up

Deliver a real browser game using the shared campaign, fixed-rate gameplay,
renderer and original frontend. Prepare server assets locally from the user's
own disc; do not bundle copyrighted data in the repository. Keep native play
and the Original preset unchanged.

- [x] Browser build and static deployment through pinned Nix, asynchronous WebGPU
  startup, keyboard/mouse/controller input, resizing and focus behavior.
- [x] Small boot asset set, background transfer of remaining game data during
  menus, and readiness checks before entering a mission. Once play starts, no
  network reads or loading screens in the simulation path. Cache media in advance.
- [x] Browser audio and original movies/captions, original menus and F6 launcher,
  local persistent saves/preferences with import/export, useful error recovery.
- [x] Real-browser boot, ordinary play, mission changes, audio/movie playback,
  persistence, resize and network-readiness checks; native regressions, Clippy,
  formatting, Nix packaging and WASM checks. Document serving and browser limits.
- [x] Investigate roof/wood relief using normal/height mapping and bounded
  parallax sampling, with filtered detail at distance. Inspect actual game
  surfaces; avoid pretending brightness is reliable geometry everywhere.
- [x] Implement useful optional surface-detail controls and content-based
  material overrides, preserving silhouettes, collision, gameplay and Original.
  Compare native/browser renders and performance at near and distant views.
- [x] Document limitations, source references, measured results and usage;
  commit coherent increments until both deliverables are working.

The asset archive is about 84 MiB. The first approach is a small sparse disc
cache for boot and a background transfer of all remaining gameplay assets,
keeping the synchronous source-faithful game loaders independent of HTTP.
Movies and XA tracks can be converted by the host preparation tool into
browser media while preserving original caption/voice scheduling. Browser
storage, audio and decoding stay at the platform boundary.

Implemented browser host: the shared game/frontend/renderer now compile to WASM;
Chromium with WebGPU renders Mission 1 and advances ordinary 30 Hz gameplay.
A prepared sparse disc cache supplies synchronous game reads, with a 15.5 MiB
compressed boot set and remaining gameplay pages fetched during menus. XA tracks
are exported as Opus and prefetched per selected language, with a bounded decoded
audio cache; movies use browser decoding and the shared caption renderer. Native
unit tests, all 87 disc-backed PC tests, and native/WASM Clippy pass after the
shared-library/platform split. The browser smoke check covers both characters,
missions 1 and 5, movement, F1, resize, opening playback/pause/skip, persistent
saves, screenshots and continued play with the network disconnected. Mission
entry waits for all gameplay pages, selected-language audio and all movies.
Nix x86_64-linux native/browser/core packages and formatting checks passed.
The final material changes pass those checks as well.
Headed Chromium on the NVIDIA Vulkan adapter is used for visual validation;
the headless compositor did not display WebGPU output reliably in this setup.

Browser compatibility correction: the original smoke test explicitly enabled
Chromium WebGPU and did not establish normal Firefox/Linux startup. The host now
includes automatic WebGL2 fallback and an explicit graphics selector/query that
avoids WebGPU probes entirely. Material metadata uses exact integer texture
records when storage buffers are unavailable. Depth reads use an unfilterable
float view with an explicit nearest sampler, and GL array textures are padded to
avoid wgpu 24's cubemap heuristic. The sampler fixes zero depth reads in WebGL
that hid the life medallion and broke contact shading; browser screenshots check
that HUD pixels survive post-processing unchanged.
All 14 renderer tests pass through both Vulkan and GL, including point shadows,
HDR finishing and surface relief. Edge MSAA and GPU timestamps remain unavailable
on the compatibility path. Firefox 147 ordinary movement and visual-preset checks
pass in an isolated profile without enabling experimental WebGPU. This does not
fix Firefox's own reported WebGPU assertion crash.
The 4K slowdown is reproduced with Firefox's NVIDIA WebGL2 adapter: roughly
60 FPS at 1280×678 versus 19.5 FPS at 3840×2038 in Original. Compile-time shader
variants remove disabled lighting, filtering, relief and HDR paths; the latter
early test measured about 42 FPS. These five-second headless measurements reproduce
the reported resolution dependence, not normal desktop presentation performance;
the frame-rate follow-up above records the corrected measurements and working setup.
Distinct shader modules avoid wgpu 24 GL's program cache ignoring override values;
feature-toggle renderer tests cover both backends. No resolution, game tick or
Original appearance changes are used to obtain the improvement.

Surface depth is implemented in the shared renderer: Off/Bump/Parallax, strength,
a separate Surface relief preset, inferred height/gradient mip chains and
content-identity height-map/disable overrides in native packs. Original/defaults
stay off. Opaque roof tiles with PS1 STP bits remain eligible; actual holes,
translucent faces, UI and emissive panels are excluded. Automatic inference is
scenery-only; explicit author maps can target models. No geometry or game state
changes. The parallax loop reads atlas metadata directly instead of copying whole
material arrays, uses eight bounded samples and fades to bump at distance.
Native GPU tests cover exact restoration, unchanged actors/cutouts/distant faces,
custom maps and per-material disabling. All 90 PC tests pass with the private
disc/GPU/audio fixtures, along with fixture-free workspace tests and native/WASM
Clippy. Chromium checks cover depth modes without advancing the simulation,
fullscreen entry/settings/exit with restored keyboard focus, exit/restart and an
unavailable asset response with a working Reload action. Actual roof/wood
near/far captures are private local artifacts.
A 1280×720 GTX 1080 Ti comparison with the same scene, cubic filtering and one
inspection light measured scene passes around 0.8–1.5 ms Off, 0.9–1.8 ms Bump,
and 1.0–2.0 ms Parallax; these are GPU scene timings, not whole-game frame times
or a guarantee for another device. Height is an estimate from painted artwork;
silhouettes and cast shadows remain geometric, with no recovered HD assets.

## Original frontend and ending follow-up

Reference `tenchu-decomp` revision `7e13c76e` (matching MENU and ENDING).
Port ordinary owned state and presentation; do not emulate PS1 executables or
their hardware scheduling. Read original artwork, animation, movies, captions
and sound from the supplied disc. Keep platform movie decoding at the host
boundary and reusable asset/timeline logic portable.

- [x] Animated title, original character models/motions, localized character
  selection and mission map, with progress/layout restrictions and records.
- [x] Original options presentation and working difficulty, language, sound,
  controls and save operations adapted to the PC host. Preserve existing PC
  preferences and saves. Explicitly identify the separate TRIAL mission editor.
- [x] Original opening/training movies, character epilogues and credits,
  including source caption/voice timing, skip rules and final-results handoff.
- [x] F6 opens the existing quick launcher; returning to the main menu and F1
  settings remain convenient. Normal game ticks and demo exports stay isolated.
- [x] Meaningful asset/transition/timeline regression tests, native visual checks,
  decoded audio/caption checks, Nix/Clippy/format/WASM validation,
  documentation and source provenance. Commit coherent implementation stages.

The port connects the original title, animated character selection,
mission map, options and ending results to the existing campaign. F6 opens the
quick launcher and Esc returns to the title. All four language asset sets load;
both selection motions finish and all five movie streams decode to their source
endpoints with XA audio and caption/voice cues. Native input has been exercised
from boot through character selection into Mission 1, then back through language
selection and the quick launcher. Controlled final-completion tests cover both
epilogues, credits, results and retry; these are transition tests, not playthroughs.
Controller feedback is connected to the original vibration option and source
gameplay cues. Damage cue comparisons against C pass; physical motor output
still needs a supported controller for a hands-on check. Movie cues agree at
5/15/60 Hz display cadence and repeated early cancellation joins decoder workers.
Unavailable movies show an explanation and allow continuing into the menu/game.
All 143 core and 87 desktop tests pass, including real-disc checks. Formatting,
Clippy with warnings denied, Nix native packaging and portable WASM checks pass
on x86_64 Linux. Native checks exercise Mission 1 and training, F1 interaction
isolation, F6/title navigation, saved language/difficulty, explicit command-line
overrides, manual load/save with autosave disabled and missing-decoder recovery.
Rapid window resizing exposed and fixed an overlay/surface size race. The
overlay now uses the configured render-target size. `--quick` bypasses the
opening and starts in the development launcher without starting a movie worker.
The built Nix package also passed mouse navigation through Ayame selection,
Mission 1, difficulty selection and the quick launcher, fifteen rapid resize
pairs and a clean exit.

Native saves replace the PS1 memory-card interface; original cheat-button chords
are not implemented (F6 provides unrestricted mission/layout selection and all
four languages are available). The separate TRIAL.EXE mission editor remains
outside this port. Existing equipment and intermediate mission-results screens
remain in place. These checks do not establish pixel-perfect rendering or a new
full campaign playthrough; final-completion tests deliberately use controlled
state to exercise both ending flows.

Use the original nominal game rate (30 game updates per second in this release),
independent of display rate. Rendering may interpolate but may not mutate the
game. Keep drawing-related coordinate, effect, RNG and activation side effects
in the ordered game update. Do not recreate PS1 frame drops. PC overload is a
host scheduling/presentation concern, not a source of gameplay `SkipFrame` flags.

## Showcase mode follow-up

Build a separate, modular native demo mode using the same game and renderer.
Ordinary play, user preferences and campaign saves must remain unchanged.

- A named catalog of short clips, each with a mission, staged camera, feature
  animation, readable explanation and progress/value display. Use recovered
  scene lights, actors and terrain as anchors instead of asset-number hacks.
- Reproducible per-clip resets and a presentation timeline independent of
  display cadence. The simulation continues through the existing 30 Hz update;
  explicit speed demonstrations change tick pacing, not gameplay arithmetic.
- A full playlist, command-line clip selection/order, optional looping, mouse
  clip picker, pause/restart/previous/next and clean capture controls.
- Show presets, point/moon shadows, texture reconstruction/filtering, edge
  antialiasing, model detail/swaps/interpolation, atmosphere and image effects,
  collision/AI inspection, free camera, simulation and cinematic acceleration,
  and live profiling across several stages and both player characters.
- Document recording and clip reuse. Validate timing/transport, scene loading,
  persistence isolation and native rendering. Keep disc assets/captures outside
  Git, run the Nix checks, and commit coherent implementation increments.

Delivered as `tenchu-pc demo`: 37 selectable clips, JSON catalog, independent
scene resets, smooth animated controls/captions, native transport/picker and
clean screenshots. The F1 coverage map and recording/authoring instructions are
in `DEMOS.md`. Cadence, persistence, native playback and packaging checks pass;
normal play and saved preferences retain their previous behavior.

This is a staged presentation, not a campaign route or a fidelity oracle. A
browser presentation host and authored replacement HD artwork remain separate.

The recording follow-up is delivered as `demo --video OUTPUT.mp4`: silent
3840×2160 / 60 fps offline export, preserving every presentation frame regardless
of GPU/encoder speed. Pixels stream to the Nix-provided FFmpeg encoder; captions
scale for 4K and each clip has an MP4 chapter and YouTube description timestamp.
The full 502-second recording has 30,120 frames with consecutive 1/60-second
timestamps, correct SDR color tags and 37 exact chapter boundaries. Previews
from all clips were inspected. Cancellation, overwrite protection, passive
settings navigation and normal window resizing were exercised natively. Nix
packaging, portable libraries, formatting and Clippy pass. Outputs stay outside
Git; profiling in footage reports capture-host timings.

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
  `ENDING.EXE` movie handoff originally used a completion screen; the frontend follow-up now plays the epilogue and credits before final results.
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

Completed visual/configuration refinement:

- [x] Balance lantern spill, surface glow and moon shadows; retain detail near lights.
- [x] Restrict emission to emitting models and side surfaces, fixing shared-texture/snow classification; bound Cubic cutout colors to prevent bright edge artifacts.
- [x] Add Crisp classic, Soft classic, Lantern night and Moonlit presets.
- [x] Allow F1 to stay open while gameplay runs, with pausing still the default.
- [x] Use clear overlay text for actor/light labels, FPS and debug readouts.
- [x] Measure MSAA/distant-view costs and remove redundant texture-layer and shadow sampling.
- [x] Add live CPU/GPU performance plots, averages, p95, freeze and clear.
- [x] Add optional persistent activation/animation of distant enemies, with default behavior unchanged.
- [x] Finish final packaging checks and commit verification evidence.

Evidence: all 139 core fixture/oracle tests and all 73 desktop tests passed with
private fixtures enabled. Formatting and warning-free workspace Clippy passed;
Nix native packaging, portable WASM and formatting checks passed. This includes default activation
parity and a distant guard patrol with the override. All 66 mission/character/layout
combinations ran with distant AI enabled. Thirteen GPU regression cases passed,
including mixed moon/lantern shadows, back-facing caps, highlight contrast and
Cubic cutout overshoot. The original Mission 5 capture is byte-identical to the
pre-change capture. Native window checks exercised labels/charts, live F1, distant
guards, lantern views, all new presets, photo mode and repeated-Escape exit.

On a GTX 1080 Ti, the fixed 2560×1440 distant-camera sweep with Cinematic settings,
4× MSAA and contact shading measured world GPU time 11.19 → 2.70 ms, shadows
0.82 → 0.82 ms and image effects 1.08 → 1.07 ms. This identifies world shading
as the main cost in that case; redundant MSAA depth reads were not the dominant
cost and were left alone. These are offscreen GPU measurements, not a guarantee
of desktop FPS. The Xvfb native check is dominated by presentation waits, which
the new profiler reports separately. Emission detection remains a configurable
mesh/texture heuristic over the game's baked artwork.

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
At this milestone menu labels remained English. The later frontend follow-up uses the completed MENU decompilation and original artwork.
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
external ENDING.EXE movie are covered by the frontend follow-up at the top of
this plan. A browser host and authored HD content remain separate work. Source-level and continuous-retail comparisons
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
