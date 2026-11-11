# Performance measurements

F1 → General → **Benchmarks…** runs four fixed scenes using the
current graphics: village lanterns/characters, Mission 5's courtyard, a garden
with vegetation/water, and a waterfall. Each scene advances the same 60 Hz
presentation timeline and original 30 Hz simulation. Loading and the first 60
frames are excluded. Escape cancels. The current game, preferences and input
recording are restored afterwards; temporary presentation particles are rebuilt.
Changing the window size or losing focus cancels a windowed measurement.

While playing native T2 JP, the same menu runs four native scenes across
Rikimaru, Ayame and Tatsumaru, including architecture, vegetation, water and
local lighting. These scenes use fixed native heroes and original gameplay,
with your selected graphics and artwork. The active campaign or randomizer
remains untouched. Weather, cloth and wound presentation are restored when
the run ends or is cancelled. Automatic tuning uses the same measurements and
selection rules below. The command-line `benchmark` also selects these scenes
when its disc is Japanese T2; it requires no T1 disc.

For individual controls, see the [graphics setting cost breakdown](graphics-performance.md):
4K measurements with one option changed at a time, plus the cost model for the
remaining sliders and event-driven effects.

## Quality and automatic tuning

Fresh installations start with **Modern Low**. Existing preferences are kept.
F1 → General → **Presets / save as…** provides Low, Medium, High and Ultra,
plus named user presets. These affect graphics and HUD text only. Gameplay,
controls, audio and window preferences are separate. The modern tiers retain
your selected weather; they do not add rain, snow or a day cycle.

| Tier | Main trade-offs |
| --- | --- |
| Low | Interpolation, clear text, lighting/shadows, water, hair/cloth, grapple effects and blood; two local lights, one local shadow, small shadow maps, original foliage geometry, no MSAA/post-processing. |
| Medium | Four local lights, two local shadows, character/foliage refinement, surface relief, bloom, falling/scattered foliage. |
| High | Eight lights, six local shadows, larger shadow maps, botanical foliage, stronger water detail, contact shading and 2× MSAA. |
| Ultra | All useful visual enhancements, highest character/foliage detail, parallax, eight local shadows, largest shadow maps and 4× MSAA. |

Original remains available as the original-presentation reference. Replacement
artwork still needs an installed pack; no preset supplies game assets.

**Auto-optimise graphics** starts at Ultra and tunes individual controls using
the [per-setting measurements](graphics-performance.md) as an ordering heuristic.
Choose a 30/60/120 FPS target and keep the game focused at your intended playing
resolution. This can take several minutes; close heavy background workloads.

- GPU pressure first tests smaller local shadow maps, 2× MSAA, fewer local
  shadows, smaller sun/moon maps and contact shading. Further reductions to
  those budgets, surface relief, filtering, post-processing and light count
  are available if needed. It keeps dynamic lighting and shadows enabled.
- CPU pressure first tests character refinement and botanical foliage density.
  It skips the Fuller/Curved foliage modes: their additional overlapping cards
  can cost more than Botanical on the measured paths.
- Hair/cloth, interpolation, modern water, grapple/particle enhancements and
  blood remain at Ultra settings. Artwork and **all** selected weather parameters
  are retained. It never adds black sight fog or switches to Original as a fallback.

Every candidate runs all four scenes, with 60 warm-up and 240 measured frames
each. A candidate is kept if it passes the target or reduces the aggregate
normalized time over budget by at least 3%; otherwise it is reverted. This
compares measured costs on the current device, rather than assuming the same
millisecond savings on every GPU. It is a bounded search, not an exhaustive
search of every setting combination.

A passing result needs CPU/GPU p95 below 85% of the frame budget and observed
frame p95 within 12% of the target interval in **every** scene. If CPU/GPU work
fits but presentation is slow, the tuner keeps quality and reports a possible
display/compositor/browser limit. Offscreen runs with GPU timestamps use only
CPU/GPU budgets: their serialized frame intervals are not interactive pacing.

The result is saved and applied as **Auto-optimised** when the original session
is restored. Cancellation changes nothing. If no candidate meets the target,
it retains the best measured configuration and says the target was missed.
Expand **Tuning decisions** in the results to see each accepted/reverted change;
JSON reports include trial scene ranges and the selected trial too.

This is a short sample, not a guarantee for every scene or unusual weather/mod.
Re-run after changing resolution, artwork or hardware. Missing GPU timestamps
are reported; that selection uses observed frames and CPU time. Display refresh
and compositor limits can prevent a high target regardless of graphics quality.

User presets support new names, overwrite, apply, separate rename and delete.
Built-ins and Auto-optimised are protected. The library is `presets.json` next to
the preferences (`TENCHU_PRESETS` overrides it); browsers use local storage.
For a non-persistent automatic report, use `benchmark DISC --auto 60 --output
report.json`. CLI benchmarks never overwrite preferences or preset libraries.

The earlier whole-tier tuner selected Low in a full-artwork 4K offscreen test at 60 FPS:
its four GPU p95 values were 5.47, 3.24, 9.21 and 4.08 ms. Medium failed the garden
at 16.74 ms GPU p95; High and Ultra failed the first scene. This describes that
machine and the former serialized offscreen selection rule, not a universal
default or FPS promise. Individual adjustment tuning supersedes that tier search.

The individual tuner was also exercised at 4K with the full artwork pack and
**always-active enemies** enabled, while the existing TAS searches continued.
It measured 18 configurations / 72 scenes (240 CPU and 239 GPU samples each),
kept the requested weather/artwork and cheap enhancements, and correctly reported
that the 60 FPS target was **not** reached:

| Scene | Initial Ultra GPU p95 | Selected GPU p95 | Selected CPU p95 |
| --- | ---: | ---: | ---: |
| Village lights | 32.10 ms | 8.33 ms | 9.62 ms |
| Snow courtyard | 20.61 ms | 4.97 ms | 8.96 ms |
| Garden water | 48.51 ms | 13.46 ms | 9.53 ms |
| Waterfall | 19.21 ms | 5.88 ms | 15.80 ms |

The selected configuration retained eight local lights, one local shadow at 512,
a 2048 sun/moon map and 2× MSAA. Character/foliage refinement, surface relief and
post-processing were removed; motion, water, particles, blood and artwork stayed.
The waterfall CPU p95 exceeds the 14.17 ms work allowance for 60 FPS. CPU spikes
also caused some useful GPU reductions to be rejected by the aggregate comparison;
this is why tuning should be repeated with other heavy work idle. These are
stress-run observations, not recommended universal settings. CLI preferences and
saves remained unchanged, and the recommendation matches the selected trial's
actual graphics. Raw results are in ignored `private/performance/heuristics`.

A Chromium/WebGPU run with base artwork at 1400×864 passed the 30 FPS target
using observed frame/CPU timing (GPU timestamps were unavailable). Its selection
kept refined characters, Botanical foliage, parallax, bloom/sharpen, motion,
water and effects, with one 1024 local shadow, a 4096 sun/moon map, no MSAA and
no contact shading. Cancellation and completion restored the exact game state;
the selected preset survived a page reload. This also exercises the missing-GPU-
timing path rather than assuming every browser exposes native GPU measurements.

For reproducible native reports:

```sh
./dev.sh cargo run --release -- benchmark '/path/to/game.cue' \
  --preset 'Modern Ultra' --size 3840x2160 --seconds 20 --output report.json
```

The bundled HD pack is discovered automatically; `TENCHU_ASSET_PACK` can override it. Reports contain the actual
graphics, adapter/backend, resolution, artwork status, sample counts and
mean/p50/p95/p99 milliseconds for each CPU phase and GPU pass. GPU timestamps
may be unavailable on some browsers/backends; missing measurements are not zero
cost. Browser presentation is normally limited by the display refresh rate.

`--offscreen` still runs the live game, geometry generation and renderer, but
replaces window presentation with an offscreen target and a GPU completion wait.
It is useful for isolating rendering work on a virtual display. Its FPS includes
serialized CPU/GPU work and **is not a prediction of interactive FPS**. Compare
the phase timings on the same machine, backend, resolution, artwork and build.
Do not run simultaneous benchmarks or compare a release build with a debug one.

## Interactive Wayland follow-up, 2026-10-29

Mission 1 was exercised on the actual Hyprland desktop, using an isolated copy
of the existing preferences: Ayame with the Mei-Oh model/moveset, always-active
enemies, HD artwork, interpolation, refined models, secondary motion, 16 lights,
six 256-pixel local shadows, moon lighting, post-processing and no sight fog.
The tiled window was 3826×2146 on a 60 Hz monitor; VSync was disabled.
The route held forward for ten seconds, turned for three and ran for five more.
Existing background workloads stayed running. These interactive runs are not
identical input-tick replays, and their tail latency varies with host load.

| Implementation | Mean frame | Frame p95 | Frame p99 |
| --- | ---: | ---: | ---: |
| Released path | 33.43 ms | 42.97 ms | 52.16 ms |
| Unlocked Wayland pacing, reused geometry buffers | 18.39 ms | 20.96 ms | 22.05 ms |
| Also share scene/shadow actor geometry | 16.05 ms | 24.66 ms | 30.17 ms |

The released path requested a Wayland compositor frame callback even with
VSync off. That gated subsequent redraws outside the timed CPU phases and
prevented CPU/GPU work from overlapping normally. Focused, unlocked rendering
now relies on surface acquisition/presentation; callbacks still pace VSync and
unfocused windows. Re-enabling the old callback with the buffer improvements
retained slowed the same route to 25.67 ms mean, confirming a separate pacing
cost rather than just a reduction in geometry work.

CPU sampling also found repeated allocation/copying and spatial sorting of
animated geometry. The live window now retains its CPU scratch allocation and
GPU vertex buffers. Static shadow geometry retains spatial sorting; dynamic
geometry already groups vertices by actor/body part and uses conservative
chunk bounds without re-sorting every frame. In inspection/always-active mode,
the opaque actor poses are identical between scene and shadow passes, so both
draw from the same buffer. Hinged scenery and translucent actor casters remain
separate. Normal camera-culling mode still builds the complete independent
shadow geometry, preserving off-camera casters.

For this scene, sharing reduced the two actor/caster uploads from approximately
9.6 MB to 4.8 MB per frame. The GTX 1080 Ti reported PCIe 3.0 ×2 while busy, with
roughly 0.6–0.8 GB/s received during the earlier run. This makes upload traffic
worth reducing, but does not establish why the link is ×2 or prove it is the
only remaining limit. The interface GPU pass measured about 0.02 ms; scene
passes totalled roughly 10–11 ms. Geometry transfer and presentation work are
outside those pass timestamps. The shared run averaged about 62 FPS, with
occasional longer frames: this is not a locked-60 guarantee at these settings.

The CPU chart now pairs each completed start-to-start interval with the
preceding frame's work and includes **Event loop + other** for the remainder.
That remainder includes scheduling, compositor pacing and untimed host work;
it is not an additional GPU stage. CPU and GPU measurements overlap and must
not be added together. Traces, isolated preferences, screenshots and sampling
results are retained locally under ignored `private/frame-pacing-review`.

The follow-up water check used ordinary Ayame and the saved Mei-Oh override at
three Mission 1 deep-water locations, including swimming, turning, grapple use
and a return to land. The optimized runs generally measured 13–17 ms; looking
toward a lit shore cost about 20 ms with roughly 10.5 ms in world/effects shading.
The first post-water land run averaged 13.89 ms. The released v0.3.0 comparison
showed about 25–27 ms in water and after returning. These checks did not reproduce
the reported persistent 15–20 FPS state, so they do not establish a separate
water-specific cause or leak fix. Teleports positioned the player for these
checks; they are not a replay of the user's route.

Validation includes 150 ordinary workspace tests, five Vulkan regressions for
streamed geometry, shadow sharing, hinged scenery, interpolation and secondary
motion, strict native/WASM Clippy, and browser WebGL2 Mission 1 movement/turning/
jumping with HD artwork and the model override. Native EGL fixture initialization
fails on this setup, before the geometry under test; browser WebGL2 is the
working OpenGL-family validation. Headless browser timings are not a desktop
performance comparison.

## Initial baseline, 2026-10-27

Linux Vulkan, Ryzen 9 9950X3D, GTX 1080 Ti, NVIDIA 580.126.09, 3840×2160.
Offscreen release build; Ultra uses the campaign replacement pack. These are
diagnostic baselines before algorithm changes, not hardware requirements.

| Scene | Original CPU p95 | Original GPU p95 | Ultra CPU p95 | Ultra GPU p95 |
| --- | ---: | ---: | ---: | ---: |
| Village lights | 0.69 ms | 0.83 ms | 17.35 ms | 31.85 ms |
| Snow courtyard | 0.66 ms | 0.44 ms | 27.11 ms | 17.90 ms |
| Garden water | 0.69 ms | 1.31 ms | 16.75 ms | 53.39 ms |
| Waterfall | 2.48 ms | 0.77 ms | 33.05 ms | 19.57 ms |

Original uses 420 measured frames per scene; Ultra uses 1,140. Mission 5's pack
exceeded the existing decoded-image budget and fell back to original artwork;
that limitation needs resolving before treating it as a full-artwork result.
The virtual X display adds about 160 ms of driver presentation wait, hence the
separate offscreen measurements. A CPU sample of Ultra identified vegetation
transformation in the transparency pass as the principal avoidable cost: opaque
triangles are being transformed and then discarded. GPU costs are concentrated
in shadows and scene shading, with a smaller post-processing contribution.

Raw reports and profiling captures are kept in ignored `private/performance`.

## Filter before transformation

The transparency pass now selects triangles before applying instance transforms,
vegetation variation and normal transforms. Triangle order and every retained
vertex attribute are byte-identical to the previous transform-then-filter path.
The same 1,140-frame-per-scene Ultra run (including the same Mission 5 artwork
fallback) gives:

| Scene | CPU p95 before | CPU p95 after | Reduction | GPU p95 after |
| --- | ---: | ---: | ---: | ---: |
| Village lights | 17.35 ms | 5.94 ms | 66% | 31.98 ms |
| Snow courtyard | 27.11 ms | 4.94 ms | 82% | 17.99 ms |
| Garden water | 16.75 ms | 5.23 ms | 69% | 53.62 ms |
| Waterfall | 33.05 ms | 10.46 ms | 68% | 19.51 ms |

GPU work is unchanged within measurement variation. The high-resolution shadow
and scene passes remain the limiting cost in the heavier Ultra scenes on this
GPU. This change does not reduce mesh detail, effects, shadow quality or AI.

Pack loading subsequently gained content-based image sharing. Mission 5 exceeded
its 64 MiB decoded-image budget by about 72 KiB; shared duplicate artwork now fits
without raising that budget. A complete-pack Mission 5 capture succeeds. The
table above deliberately keeps the old fallback in both runs for comparability.

## November 2026 renderer optimisation checks

Measured on a GTX 1080 Ti / Ryzen 9 9950X3D at 3840×2160 with the full v3
artwork pack, Modern High, original weather and all enemies active. Each moving
scene has 60 warm-up frames and 300 measured CPU / 299 GPU frames. The
per-fragment light-list iterator preserves the preset and light selection:

| Scene | Before GPU mean | After GPU mean | Before terrain | After terrain |
| --- | ---: | ---: | ---: | ---: |
| Village lights | 15.26 ms | 13.80 ms | 7.49 ms | 6.06 ms |
| Snow courtyard | 12.91 ms | 12.29 ms | 7.08 ms | 6.48 ms |
| Garden water | 25.09 ms | 20.86 ms | 17.62 ms | 13.43 ms |
| Waterfall | 9.41 ms | 8.58 ms | — | — |

These are offscreen GPU timestamps, not desktop FPS. Static 4K garden and
TRIAL 2 play/overview captures are pixel-identical before and after. Native
regressions cover overlapping light budgets from zero to sixteen, diffuse and
reflected light, lantern shadowing and rain reflections; a WebGL2 browser check
also exercises lit/unlit/restored gameplay. Native strict Clippy and the WASM
build pass. CPU frame work is broadly unchanged; the moving garden still
exceeds the 16.7 ms GPU budget for 60 FPS.


The next pass reuses each surface's lighting for accumulated snow, skips wet
reflection work on dry snow, and rejects fully transparent blood/particle pixels
before expensive shading. A fixed TRIAL 2 camera uses the same Modern High
settings, maximum weather detail, 30 seconds of precipitation, and (where noted)
4,096 overlapping terrain-projected blood marks:

| Condition | Before GPU mean | After GPU mean |
| --- | ---: | ---: |
| Original weather | 11.52 ms | 11.37 ms |
| Dense blood | 36.67 ms | 33.40 ms |
| Heavy rain and puddles | 22.39 ms | 22.26 ms |
| Accumulated snow | 23.82 ms | 16.74 ms |
| Accumulated snow and dense blood | 100.52 ms | 73.73 ms |

Each sample follows ten warm-up frames and averages forty frames, with static
shadow maps cached. All ten 4K captures across the play and overhead cameras
are pixel-identical. This deliberately overlapping blood workload is a stress
test, not typical gameplay; its remaining cost is substantial. Rain is unchanged
within measurement variation. Sixteen focused native GPU regressions, WebGL2
rain gameplay and windowed Chromium WebGPU snow gameplay pass. Headless WebGPU
screenshots were black with both the previous and current builds; the windowed
browser check avoids that capture limitation. These remain offscreen GPU
measurements, not a claim about desktop frame intervals or weather-update CPU
cost.


A CPU profile of maximum-detail precipitation found most strong-wind update
cost in triangle intersections. Surface queries traversed the entire box around
a diagonal segment. Testing that segment against each spatial node now rejects
unrelated terrain before testing its triangles. An isolated Mission 1 garden
fixture prewarms 30 seconds, then measures 600 updates and geometry builds:

| Weather / wind | Before update mean | After update mean | Before p95 | After p95 |
| --- | ---: | ---: | ---: | ---: |
| Rain / none | 0.112 ms | 0.113 ms | 0.129 ms | 0.129 ms |
| Rain / maximum | 0.810 ms | 0.138 ms | 1.071 ms | 0.162 ms |
| Snow / none | 0.016 ms | 0.016 ms | 0.023 ms | 0.024 ms |
| Snow / maximum | 1.439 ms | 0.040 ms | 2.533 ms | 0.063 ms |

Both runs use `perf` sampling. All four final particle states and generated
meshes are byte-identical; geometry construction remains about 0.08–0.13 ms.
Exhaustive segment comparisons include excluded water surfaces, diagonal rays,
parallel axes, exact face/edge contacts and zero-length segments. The index
belongs only to presentation effects; game collision and AI do not use it.

The real Wayland desktop benchmark, after the light-list and snow-shading
changes, averaged 48.6 / 53.5 / 36.1 / 58.1 FPS for village, snow courtyard,
garden and waterfall, against 45.6 / 50.9 / 31.8 / 54.6 FPS before this loop.
These are isolated Modern High 4K runs with HD v3, not a guarantee of 60 FPS;
CPU/GPU transfer, presentation and heavier effects still need work. A separate
sky shader was tested and discarded because it made no measurable difference.


Effects now reuse their GPU buffer like animated actor geometry. A paired
upload/draw fixture alternates fresh allocation and reuse, with twenty warm-up
frames and 180 samples per pass. For 1,792 particle quads, upload CPU time falls
from 0.071–0.072 ms to 0.040–0.041 ms; for 8,192 quads, from 0.535–0.563 ms to
0.233–0.279 ms. The larger case's complete upload/draw/wait falls from
2.71–2.75 ms to 2.35–2.41 ms. This removes an extra vertex copy and repeated GPU
allocation without changing particle counts, blend order or retained effects.
Fresh/reused render comparisons cover all blend modes, HDR HUD separation,
growth, shrinkage and empty frames. Ordinary and heavy-rain desktop FPS is
largely unchanged: those scenes remain limited by GPU shading and presentation.


The weather shader also reads the fixed 32-point soft-shadow sampling pattern
from uniforms, avoiding repeated trigonometry in its already large fragment
program. A repeated TRIAL 2 comparison reduces the overlapping snow/blood case
from 74.17 to 63.56 ms (play camera), and 29.66 to 25.80 ms (overhead). All ten
captures remain pixel-identical. Ordinary rain/snow timings are effectively
unchanged. Applying the same change to the smaller dry shader regressed dense
blood rendering by about 3%, so that variant retains local arithmetic. No
shadow samples, lights, effects or quality settings are removed.


Sustained bleeding exposed a separate rewind spike. With 8,192 retained marks
of 96 vertices each, inserting another mark per update eventually removes the
oldest page. The rewind cache used each resource's position in the capture
sequence, so the remaining unchanged pages were all encoded again. Stable
shared resources now reuse their existing encoded data after reordering.

Across 259 continued-bleeding captures, average capture/history time falls from
0.496 to 0.338 ms, and the maximum from 20.105 to 0.710 ms. Page turnovers at
updates 128 and 256 fall from 20.105 / 19.284 ms to 0.622 / 0.521 ms. Retained
history at the second turnover falls from 13,511 to 8,364 KiB because those
unchanged pages are shared. The first capture still needs about 20 ms to encode
the complete initial collection; it is excluded from the continued-bleeding
statistics. The serialized state format and exact reconstructed bytes are
unchanged.

Direct integer depth reads on native/WebGPU and reuse of each occlusion sample
remove redundant depth-addressing work from post-processing. The same 4K TRIAL
fixture measures 3.30 to 3.12 ms for the play camera's post pass, and 2.32 to
2.19 ms overhead. Rain and snow/blood variants show similar 0.1–0.2 ms savings.
All six comparison captures are pixel-identical; HDR, contact occlusion, MSAA,
timing and streamed-overlay checks pass. WebGL retains its explicit nearest
sampler because that path avoids incomplete depth textures on that backend.

Weather shelter on sloped surfaces already interpolates four explicit depth
comparisons to account for the receiver plane. Fetching each exact depth texel
instead of asking the hardware to bilinearly filter again avoids redundant work.
The same 4K TRIAL fixture measures snow at 16.54 to 16.08 ms and snow plus dense
blood at 63.40 to 59.40 ms (overhead: 25.72 to 24.50 ms). Dry and rain cases are
unchanged within variation. Eight of ten captures match exactly; the two overhead
snow captures each differ by one channel level at one pixel out of 8.3 million.
The weather map, wind, coverage and filter footprint are unchanged.

Character models now retain their existing shared vertices and triangle indices
through the GPU upload and shadow passes. Poses transform directly into reused
storage, preserving the expanded vertex bytes and triangle order. In the TRIAL 2
fixture, 134,028 triangle corners become 38,652 shared vertices: the per-frame
upload falls from 8.58 MB to 3.01 MB including indices. Six paired 4K captures
(dry, rain and snow with dense blood, from two cameras) are pixel-identical.

Real Wayland 4K Modern High measurements, full HD artwork and all actors active:

| Scene | Before indexed uploads | After | Heavy rain before | Heavy rain after |
| --- | ---: | ---: | ---: | ---: |
| Village | 48.8 FPS | 60.8 FPS | 34.1 FPS | 39.5 FPS |
| Snow courtyard | 53.5 FPS | 66.5 FPS | 39.9 FPS | 46.5 FPS |
| Garden | 37.2 FPS | 42.4 FPS | 24.3 FPS | 26.4 FPS |
| Waterfall | 58.1 FPS | 78.7 FPS | 50.6 FPS | 66.1 FPS |

These are unlocked rendering rates on the 60 Hz desktop. The repeat runs above
had no concurrent compiler. The GPU scene cost is largely unchanged; the gain
comes from reduced geometry preparation and transfer waits, which render-pass
timestamps do not include. The garden and heavy rain remain below 60 FPS.
Geometry comparisons cover original/refined models, multiple missions, both
characters, visibility modes and interpolation. GPU checks also exercise indexed
and unindexed buffer transitions, shrinking/empty meshes, moving casters, wind,
directional shadows and weather shelter.

Projection of new blood and snow footprints also avoids work after reaching the
subdivision limit, skips clipping allocations when a polygon lies entirely on
one side of a plane, and computes each blood stamp's rotation once. A CPU stress
fixture projects 946 marks onto actual Mission 1 surfaces at each size:

| Newly generated marks | Before | After |
| --- | ---: | ---: |
| Footprints | 10.67 ms | 7.71 ms |
| Blood stains | 7.33 ms | 5.41 ms |
| Large pools | 18.24 ms | 12.31 ms |

These are batch generation costs, not a per-frame FPS claim. All resulting
vertices, UVs, normals and opacity values are byte-identical (313,059 vertices
across the three cases). Retention limits, appearance and subdivision thresholds
are unchanged; drawing retained decals remains a separate GPU cost.

Animated meshes also retain their last uploaded triangle indices. Their vertices
still update every frame; indices upload only when their contents change. With
50,948 character vertices and 160,128 indices, an alternating reuse/forced-upload
fixture measures complete upload/draw/wait at 2.67 ms versus 3.07–3.17 ms.
The CPU upload stage remains around 0.49 ms: most savings are transfer waits.
Actual 4K High desktop runs move from 61.0 / 67.5 / 43.8 / 79.7 FPS to
62.7 / 69.3 / 44.6 / 81.8 FPS across village, snow courtyard, garden and
waterfall. Geometry checks compare repeated and changed indices, vertex
reordering, growth, empty meshes and moving cached shadows against fresh uploads.

Lit and weather-enabled scenes now establish visible depth before the colour
pass. The depth shader uses the same cutout rules and stops before lighting,
filtering and weather colour calculations. A one-unit depth bias leaves the
strict colour-pass depth test in charge of coincident face order. Plain Original
rendering skips this pass. Performance charts and diagnostic/benchmark columns
report its cost separately as `visibility`.

Paired 4K TRIAL 2 GPU measurements, same process, with/without the visibility pass:

| Scene | Before | After |
| --- | ---: | ---: |
| Dry | 10.75 ms | 8.77 ms |
| Maximum rain | 22.05 ms | 15.09 ms |
| Accumulated snow | 15.89 ms | 11.42 ms |
| Snow and 4,096 blood marks | 59.33 ms | 54.79 ms |
| Rain and 4,096 blood marks | 86.61 ms | 79.73 ms |

All ten paired play/overhead captures are pixel-identical. Overhead views with
little hidden geometry improve only slightly. An earlier depth shader that
evaluated unused weather calculations regressed those views and was replaced.

Actual Wayland 4K desktop measurements, High/full HD, unlocked on a 60 Hz display:

| Scene | High before | High after | Maximum rain before | Rain after |
| --- | ---: | ---: | ---: | ---: |
| Village | 62.7 FPS | 63.0 FPS | 40.5 FPS | 40.8 FPS |
| Snow courtyard | 69.3 FPS | 74.3 FPS | 48.1 FPS | 52.4 FPS |
| Garden | 44.6 FPS | 52.6 FPS | 27.3 FPS | 35.4 FPS |
| Waterfall | 81.8 FPS | 81.3 FPS | 67.5 FPS | 66.7 FPS |

The transparent waterfall gains little from opaque depth rejection and pays the
extra pass cost; the garden and heavy weather still need further optimisation.
Checks cover cutouts, coincident faces, backfaces, blended effects, moving indexed
meshes, all supported MSAA levels, HDR, weather shelter and timestamp accounting.
Windowed WebGL2 rain and WebGPU snow also pass gameplay and lighting-toggle checks.

The visibility shader also uses the existing fully opaque replacement-texture
metadata to skip relief/texture sampling when it cannot affect coverage. It
retains the normal cutout path for original, translucent and emissive materials;
empty scenes skip the pass. The TRIAL 2 visibility stage drops from 0.95 to
0.71–0.76 ms, its overhead view from 0.45 to 0.35 ms, and the garden from 1.38 to
1.30 ms. All nine paired 4K captures are identical. Total desktop High results
are 63.6 / 74.5 / 53.4 / 82.2 FPS; this is a small incremental saving, not a
solution to the remaining heavy-weather and overlapping-blood cost. Additional
GPU checks compare solid/cutout replacement textures, semi-transparent texels,
emissive and blood materials across filtering modes against the ordinary pass.

Projected modern blood now uses a specialised fragment entry when a retained
page's flags prove that unrelated sprite, weapon and water paths cannot apply.
Geometry, shading functions and blend order are shared with the generic path.
Original blood and other materials retain that path. The initial specialisation
was slightly slower on wet surfaces, so it initially retained the generic shader
for rain; the subsequent weather optimisation below removes that restriction.

Paired 4K TRIAL 2 measurements (4,096 marks in the play view, 2,304 overhead):

| Scene | Generic shader | Selected blood shader |
| --- | ---: | ---: |
| Dry blood | 30.51 ms | 25.50 ms |
| Snow and blood | 54.11 ms | 46.75 ms |
| Rain and blood | 79.49 ms | 79.53 ms |
| Dry blood, overhead | 14.45 ms | 12.20 ms |
| Snow and blood, overhead | 23.97 ms | 22.76 ms |
| Rain and blood, overhead | 33.07 ms | 33.03 ms |

The blood stage itself falls from 22.27 to 17.36 ms dry and from 43.01 to
35.62 ms with snow. All six paired captures are identical. Nine earlier paired
TRIAL/garden captures also match, including the rejected specialised rain path.
GPU regressions cover mixed retained styles, texture holes, moving foliage,
weapon-coated and unlit fallbacks, snow/rain, HDR and supported MSAA levels.
Native and WASM strict Clippy/builds pass, as do windowed WebGL2 rain and WebGPU
snow gameplay checks. The four desktop High scenes without this blood stress
remain effectively unchanged at 63.7 / 74.6 / 53.4 / 81.3 FPS.

Weather shading now precomputes wind direction and shelter-map scales once per
frame for opaque scenery and specialised ground blood. Compile-time entry-point
constants retain the inline calculations in the generic alpha shader, where
using the extra uniforms measured slower. Rain also skips the nine-wave ripple
calculation on wet ground with no puddle. Ripple density, reflection quality,
shelter samples and precipitation amounts are unchanged.

The resulting ground-blood shader is now faster in rain too, so wet pages can
use it. Compared with the previous commit's selected paths:

| 4K scene | Before | After |
| --- | ---: | ---: |
| TRIAL 2 rain | 14.96 ms | 14.31 ms |
| TRIAL 2 rain and 4,096 blood marks | 79.39 ms | 68.95 ms |
| TRIAL 2 snow and 4,096 blood marks | 47.14 ms | 45.24 ms |
| TRIAL 2 rain/blood, overhead | 33.33 ms | 29.51 ms |
| Garden rain | 23.87 ms | 22.99 ms |
| Garden rain and blood | 27.73 ms | 26.42 ms |

Forcing generic blood remains faster than its previous version (79.39 to
76.96 ms in TRIAL 2), protecting mixed pages and unsupported material flags.
Seven of fourteen selected-path captures are exact; the others differ by at
most five channels in a 4K image, each by one 8-bit level. The CPU/GPU floating
point evaluation and ripple branch account for these rounding differences.
Checks cover mixed blood styles, snow, wet reflections, ripples, shelter, wind,
rain haze and footprints in baked snow.
Strict native/WASM Clippy and builds pass, along with windowed WebGL2 rain and
WebGPU snow checks. Desktop High remains 63.8 / 74.7 / 53.4 / 81.0 FPS. Maximum
rain reaches 43.0 / 54.0 / 36.1 / 69.0 FPS; the earlier visibility-pass benchmark
was 40.8 / 52.4 / 35.4 / 66.7 FPS before the intervening shader improvements.

A footprint stress check projects up to the live 512-mark / 65,536-vertex
budget onto actual stage surfaces. In TRIAL 2's close view, tracks add roughly
1.4 ms of GPU effects work in snow, or 2.9 ms with rain over residual snow.
The overhead view adds about 0.8 / 1.2 ms; the garden adds about 0.3 / 0.4 ms.
These are incremental rendering costs, separate from simulation and uploads.

CPU filtering of replaced effects was also copying the entire combined geometry
even when it contained no original-weather/splash triangles. The filter now
returns immediately in that case, and compacts ordered batches in place when
removal is needed. Repeated/reordered batches retain a separate source so their
blend order and shared vertices remain valid.

Two consecutive filters, 300 measured iterations after 20 warm-up iterations:

| Combined effects | Before mean / p95 | After mean / p95 |
| --- | ---: | ---: |
| 12,288 vertices, no removals | 0.149 / 0.156 ms | 0.003 / 0.003 ms |
| 12,288 vertices, sparse removals | 0.149 / 0.155 ms | 0.007 / 0.007 ms |
| 114,687 vertices, no removals | 2.038 / 2.452 ms | 0.034 / 0.035 ms |
| 114,687 vertices, sparse removals | 2.012 / 2.345 ms | 0.090 / 0.093 ms |

The large case represents a combined footprint/particle/animated-effects buffer;
retained static blood pages are already separate. Checks compare rendered
triangle/blend sequences for ordered, overlapping and reversed ranges, partial
triangles, empty batches and every removal mask, plus streamed GPU rendering.
Native and WASM strict Clippy pass. Desktop High is effectively unchanged at
63.7 / 74.6 / 53.5 / 82.4 FPS, and maximum rain at 43.1 / 53.8 / 36.4 / 69.1 FPS;
those scenes do not fill the combined effects buffer.

Geometry uploads now separate the changing 24-byte position/normal stream from
40-byte colour/UV/material/opacity attributes. Unchanged attributes stay on the
GPU; changed attributes, index order, growth, empty frames and model swaps still
update normally. CPU vertices, saved states and shader inputs retain their
original values and precision. Static meshes do not keep an attribute cache.

This targets a cost outside the scene timestamps: queued buffer transfers occur
before the first timed render pass. A private frozen-upload experiment improved
desktop throughput substantially without a similar change to shader timings.
It was diagnostic only; the implemented cache continues to animate every pose.
The machine's GTX 1080 Ti currently runs through PCIe 3.0 x2, so these transfer
savings may be less pronounced on a wider link. Increasing presentation latency
or switching to mailbox mode did not produce a meaningful improvement and was
not retained.

A 50,948-vertex / 160,128-index moving-mesh check with reused indices reduces
upload/draw/completion time from 2.75–2.76 to 1.40–1.41 ms. CPU upload preparation
remains about 0.52–0.58 ms. Steady vertex transfers fall from 3,260,672 to
1,222,752 bytes while the attributes remain unchanged.

| 4K moving desktop scene | High before | High after | Maximum rain before | Rain after |
| --- | ---: | ---: | ---: | ---: |
| Village | 63.7 FPS | 70.1 FPS | 43.1 FPS | 45.3 FPS |
| Snow courtyard | 74.6 FPS | 82.7 FPS | 53.8 FPS | 58.0 FPS |
| Garden | 53.5 FPS | 58.2 FPS | 36.4 FPS | 38.6 FPS |
| Waterfall | 82.4 FPS | 92.1 FPS | 69.1 FPS | 76.6 FPS |

All fourteen paired 4K TRIAL captures match exactly, including dry scenery,
rain, snow, thousands of retained blood marks and full footprint buffers.
Cached-scene GPU times remain essentially unchanged: this optimisation reduces
transfers, not the heavy shading cost of overlapping rain/blood surfaces.
Focused GPU checks cover changing attributes with identical vertex counts,
pose movement, indexed/expanded transitions, growth, shrinkage, empty frames,
blending, shadow invalidation, retained decals, cutouts and weather shelter.
Native and WASM strict Clippy/builds pass. Windowed WebGL2 rain and WebGPU
snow checks pass, including play, pause, free camera and lighting changes.

Precipitation now uses the existing conservative view-frustum test before
emitting rain, snow, roof-drip and splash quads. Fully transparent quads are
also skipped. Simulation still updates the complete particle set, shelter,
wind and accumulation; only draw geometry is culled. Bounds include the whole
streak, so a visible tail is retained even when its tip is outside the view.
Sorting remains unchanged, preserving alpha-blend order.

All fourteen paired 4K rain/snow/blood/footprint captures match exactly. Four
600-tick weather checks produce byte-identical serialized states; the retained
quads are byte-identical subsequences of the old meshes. In that sheltered
camera view, rain geometry falls from about 20,900 vertices to 6–192, and snow
from about 10,700 to 0–12. Those counts are view-specific, not a general reduction
in precipitation density. CPU geometry time falls from 0.123–0.125 to
0.117–0.118 ms for rain, and 0.082–0.083 to 0.064 ms for snow.

| 4K maximum-rain desktop scene | Before | After |
| --- | ---: | ---: |
| Village | 45.8 FPS | 47.2 FPS |
| Snow courtyard | 57.9 FPS | 58.8 FPS |
| Garden | 38.3 FPS | 39.3 FPS |
| Waterfall | 78.2 FPS | 80.6 FPS |

GPU shading times remain essentially unchanged. Nine weather tests pass,
including interpolated particles entering the view and long edge-crossing
streaks, alongside native and WASM strict Clippy/builds. Windowed WebGL2 rain
and WebGPU snow checks also pass, including pause, free camera and lighting
changes.

Repeated maximum-snow runs give 51.7 → 52.6 FPS (village), 64.7 → 66.5
(courtyard), 43.4 → 43.9 (garden), and 83.8 → 85.5 (waterfall). An earlier
garden baseline reached 44.4 FPS with lower terrain shading time; rerunning
the old binary reproduces the later terrain timings, so that variation is
not attributed to precipitation culling.

The rain CPU profile also found repeated per-triangle filtering in animated
model assembly. Entirely opaque meshes now append their contiguous indices
directly; mixed meshes append complete retained triangles after reserving
space. This keeps the original order and sharing without another model cache.

| HD model preparation (CPU, all actors active) | Before | After |
| --- | ---: | ---: |
| Mission 1, 50,948 vertices / 160,128 indices | 0.359 ms | 0.246 ms |
| Mission 5, 48,501 vertices / 152,430 indices | 0.338 ms | 0.230 ms |
| Mission 9, 53,924 vertices / 166,608 indices | 0.372 ms | 0.255 ms |

The six captured vertex/index buffers match byte for byte. Existing comparisons
against expanded character geometry also pass across five mission/character
combinations, original/refined meshes, visibility and interpolation states.

Native and WASM strict Clippy pass. Moving 4K High measures 70.3 / 83.8 /
58.7 / 92.0 FPS. Maximum rain remains GPU-bound; the reduced model assembly
time is not claimed as a material GPU or whole-frame improvement.

Shadow preparation now caches each mesh's visible ranges and redraw bounds
for a light face. Its key includes the projection, wind, resolution, GPU buffer,
geometry revision and static/dynamic status. Moving actors still update each
frame; unchanged scenery avoids another traversal. Shadow invalidation clears
these entries too, releasing old mesh buffers and keeping resource changes
simple.

In fixed TRIAL views, CPU submission falls from 0.268 to 0.158 ms (dry),
0.354 to 0.153 ms (rain), and 0.346 to 0.164 ms (snow). With 4,096 blood marks,
rain submission falls from 0.401 to 0.290 ms. GPU shading is unchanged, and all
fourteen paired captures match exactly. Moving High scenes save about
0.16–0.28 ms of CPU submission; whole-frame FPS remains largely GPU-bound.

Forced full-shadow comparisons pass with moving/removed actors, indexed and
expanded meshes, model-buffer replacement, light movement, wind and resolution
changes, sun shadows and precipitation shelter. Live characters/weapons,
texture holes, directional shadows, windward walls and eave shelter checks
also pass. Native and WASM strict Clippy pass.

Windowed WebGL2 rain and WebGPU snow checks pass as well, including lighting
changes, paused free camera and resumed play. Maximum-rain desktop scenes
measure 47.3 / 57.6 / 37.4 / 81.9 FPS; this change reduces CPU work rather
than the dominant wet-surface shading.

Rain haze now projects the view ray into both orthographic shelter maps once,
then interpolates its six sample positions in map space. Sample counts,
spread, weights and shelter textures are unchanged. Repeated paired 4K TRIAL
runs measure rain at 14.29 → 14.09 ms GPU time, dense rain/blood at
68.65 → 67.16 ms, its daylight variant at 71.80 → 70.26 ms and its lightning
variant at 78.14 → 76.76 ms. Overhead rain/blood changes from 29.53 to 29.21 ms.

Ten of eighteen paired images match exactly. Six others differ in at most
41 colour channels by one 8-bit level; two overhead daylight/lightning images
have differences up to three levels in 9–25 channels. These are rounding
differences from evaluating the same ray after projection. Eight focused GPU
checks pass, covering haze crossing cover, roof edges, windward walls, wet
reflections, blood compositing, footprints and sun/moon changes.
Native and WASM checks pass, along with windowed WebGL2 rain and WebGPU snow.
The moving maximum-rain scenes measure 46.8 / 57.7 / 38.0 / 81.0 FPS; the
small saving in ordinary rain is within the variation of these desktop runs.

A separate attempt to reuse the noise function's corner calculations preserved
all eighteen images but produced no meaningful speedup, so it was not retained.

Opaque scenery now uses a specialised entry point in the shared surface shader.
Meshes qualify only when every vertex is scenery without UI, sky, blood or
alpha-decal flags. Characters and mixed meshes keep the general shader. Water,
foliage movement, lamps, texture cutouts and their original blending rules are
preserved. Classification happens when geometry changes, not every draw.

Paired 4K TRIAL runs measure dry scenery at 8.41 → 7.87 ms GPU time, rain at
14.20 → 13.55 ms, snow at 11.07 → 10.52 ms and snow with footprints at
12.54 → 11.95 ms. Dense rain/blood changes from 67.80 to 67.44 ms because
stain shading remains the dominant cost. The Mission 1 garden changes from
14.36 to 13.51 ms dry and 22.78 to 21.69 ms in rain. The Mission 4 waterfall
view is effectively unchanged. All 26 paired images match exactly.

Paired moving desktop runs at 4K measure Modern High at 69.7 / 82.4 / 57.3 /
93.6 FPS before and 72.3 / 85.8 / 59.7 / 93.4 FPS after. Maximum rain changes
from 46.4 / 57.4 / 37.9 / 79.9 to 50.5 / 62.7 / 44.3 / 81.4 FPS. These use
the same fixed camera paths and simulated updates, with separate settings files.
The focused scenery comparison covers every supported sample count, filtering,
HDR, moving foliage, water, emissive surfaces, rain and snow. Streaming and
existing lighting, cutout, relief, weather and water GPU checks also pass.
Native and WASM strict Clippy checks pass. Windowed WebGL2 rain and WebGPU
snow checks pass with lighting changes, paused free camera and resumed play.

The attached-blood CPU stress fixture now seeds actual HD wound patches on
loaded Mission 1 actors and animates their poses. Unlike the ground-stain GPU
fixture, these marks must follow moving body parts. At 64 / 1,024 / 8,192 marks
(24,342 / 387,447 / 3,082,074 vertices), geometry construction initially costs
0.157 / 5.50 / 50.54 ms. Preallocating the output helps the middle case, but
reusing the effects staging buffer across frames brings the costs to
0.154 / 2.12 / 16.62 ms. All three captured vertex streams match byte for byte.

Empty footprint, weather and contact layers also used to copy their successor's
entire vertex buffer during composition. They now take ownership of that buffer.
A composition fixture with three empty prefix layers measures 0.259 / 11.68 /
186.31 ms before and 0.181 / 2.14 / 16.64 ms with staging reuse and transfer of
ownership. Its emitted vertices also match exactly. This fixture measures CPU
geometry and assembly, not GPU upload, shading or whole-frame performance;
millions of animated stain vertices remain an extreme workload.

Five CPU checks and seven focused GPU/disc-backed checks pass, including
retention limits, independent blood styles, finite drips, death timing and
streaming through growth, empty frames and resumed drawing. Formatting and
strict native/WASM Clippy pass. Ordinary moving 4K scenes measure
73.5 / 87.5 / 62.8 / 92.2 FPS on Modern High and 50.4 / 62.5 / 44.5 / 81.4 FPS
with maximum rain. These scenes have little attached blood; the main measured
benefit is in the separate dense-blood CPU fixture.
Windowed WebGL2 rain and WebGPU snow also pass, including lighting toggles,
paused free camera and resumed play.

A per-part transform cache produced little improvement and was discarded.
Likewise, a shader shortcut that skipped zero puddle calculations on steep
surfaces preserved all fourteen images but made dense wet blood substantially
slower, so it was not retained.

Snow, snowstorms and clear weather now compile projected blood without
active-rain haze or ripple code. Those uniforms are already zero in non-rain
modes. Wetness, puddles, reflections and accumulated snow remain available when
the weather changes. Scenery and general alpha shaders keep their previous
variants: specialising those paths slowed one moving snowy scene and footprints.

With 80 warm-up frames and 40 measured frames per case, the 4K TRIAL view
measures snow at 10.44 → 10.39 ms, snow/blood at 44.77 → 43.64 ms, snow with
footprints at 11.87 → 11.87 ms and wet snow/blood at 49.63 → 46.75 ms.
The overhead snow/blood view changes from 21.96 to 21.31 ms and wet snow/blood
from 26.67 to 25.20 ms. Rain measures 13.50 → 13.44 ms and rain/blood
67.54 → 67.41 ms. All twelve paired images match exactly.
A moving max-snow repeat measures village 56.1 → 54.6 FPS, courtyard
71.7 → 71.9, garden 51.2 → 51.4 and waterfall 85.2 → 85.4; the village's
GPU p95 is unchanged at about 17.5 ms. This is a dense-blood improvement,
not a claim of faster ordinary snowfall. Modern High measures
72.9/86.1/59.7/92.0 FPS. Ten focused GPU checks, strict native/WASM Clippy
and WebGL2 rain/WebGPU snow checks pass.

Two further experiments were not retained. Indexing static terrain reduced
the garden's vertex/index storage from 15.45 to 9.09 MB, but seventeen paired
images and forced shadow redraws showed no useful frame-time gain. Reusing
shelter exposure between lightning and surface shading helped snowy flashes
slightly but substantially slowed ordinary wet blood.

Attached blood now shares immutable mark geometry during rewind capture, like
retained ground pages. Ordinary save serialization is unchanged. The recorder
and history also count shared encoded resources once toward their existing
96 MiB budget; previously both counted the same allocation.

A CPU stress fixture retains 384 vertices per body mark and replaces one mark
per tick. At 64 marks, recurring capture plus history changes from 0.480 to
0.022 ms; at 1,024 marks, 15.43 to 0.050 ms; at 8,192 marks, 55.53 to
0.271 ms. The old path exceeds its memory budget at 1,024 and 8,192 marks;
the new path keeps all 32 tested snapshots, with 10.0 and 78.3 MiB of history.
These are synthetic geometry-capture measurements, not complete game frame times.
The first capture still encodes and compresses all existing marks: approximately
21 ms for 1,024 marks and 171 ms for 8,192 in this fixture. Subsequent captures
reuse those resources. Restored data matches ordinary serialization exactly,
including expiry and removal of blood categories.

An ambient-occlusion experiment that reused inverse-projected rays was discarded:
it slightly increased post-processing time (about 3.00 to 3.05 ms) and introduced
small rounding differences. The original sampling and reconstruction remain.
The 49 MiB shared-resource budget regression, stain expiry/style changes,
rewind seeking/branching, visual-effect continuation and both heroes' mission
1–10 save round-trips pass. Native/WASM strict Clippy and release builds pass.

Combining pure-alpha effect layers can now retain the larger vertex buffer and
copy only the smaller prefix. Alpha batches keep their original draw order;
opaque geometry keeps its physical order because the opaque pass ignores the
moved alpha vertices. Retail weather and splash filtering now happens before
modern layers are combined, avoiding a later copy of reordered batches.

Using 73 actual HD wound templates on animated Mission 1 actors, plus three
synthetic alpha layers of 65,526, 20,898 and 3,072 vertices, CPU assembly changes
from 0.499 → 0.297 ms for 64 marks, 11.84 → 2.41 ms for 1,024 marks and
161.58 → 18.46 ms for 8,192 marks. The same reused staging buffer, moving poses,
180 frames and 30-frame warm-up are used on both paths. Final opaque-vertex and
alpha-batch draw data match. These are CPU composition times; GPU upload and
rendering are additional costs, especially for the 3.17-million-vertex case.

The streamed-effects GPU test now compares the old concatenation order with
empty, alpha-world, alpha-HUD and opaque prefixes through growth, shrinkage and
empty frames, with HDR on and off. Pixels match. Six other focused checks cover
filtering, overlapping blood, snow tracks, contact effects, visibility and decal
page reuse. Strict native/WASM Clippy and release builds pass.
Moving 4K routes measure High 73.6/87.4/62.7/91.8 FPS, maximum rain
50.1/62.9/44.4/79.8 and maximum snow 54.9/71.7/51.2/85.3. These ordinary
routes have little accumulated blood and do not establish a comparable large
FPS gain. Browser WebGL2 rain and WebGPU snow checks pass.

Rewind also shares the immutable meshes of retained footprints and wet contact
marks. Aging, burial, exposure and retirement remain per-snapshot data. The
ordinary save format is unchanged. With one mark replaced per tick across 90
snapshots, footprint capture plus history changes from 0.0669 → 0.0072 ms
for 32 marks, 0.2433 → 0.0119 ms for 128 and 0.9989 → 0.0218 ms for 512
(126 vertices each). History for the largest case changes from 14.3 to 2.5 MiB.
For wet marks with 510 vertices each, 24 marks change from 0.1524 → 0.0211 ms
and 96 from 0.6054 → 0.0244 ms; the latter's history changes from 9.0 to
1.9 MiB. Final serialized states match the old path. As with blood, the first
capture encodes all current geometry: 2.76 ms for 512 footprints and 1.99 ms
for 96 wet marks in these CPU fixtures.
Fifteen focused checks pass, including both heroes' actual footsteps, distant
active patrols, footprints on existing snow, wet-mark continuation, complete
visual-state restoration and rewind seeking/branching. Native/WASM strict Clippy
and release builds pass.

Contact particles and splash rings now skip geometry outside the view, with
conservative bounds covering interpolated positions, long spark/drop tails and
partially visible rings. Culling happens after the existing particle sort, so
remaining blend order is unchanged. Simulation, collisions and emitted light
continue for off-screen particles; fully faded geometry is skipped too.

At maximum falling-leaf density, a rotating Mission 1 garden view changes CPU
assembly from 0.1081 to 0.0449 ms and average geometry from 13,517 to 591
vertices. Mission 3 changes from 0.1833 to 0.0912 ms and 20,574 to 3,489 vertices.
Both complete particle states match, and the retained final vertices are exact
subsequences of the old geometry. Three dense-leaf garden images (dry, rain,
snow) match exactly, as do ten TRIAL control images without leaf emitters.
Static GPU rendering time is essentially unchanged; the benefit is reduced CPU
assembly and per-frame transfers. Tests cover interpolated particles crossing
the view, visible tails with off-screen centers, expanding rings and fade-outs.

Two weather-projection shader experiments were discarded. Reconstructing the
shelter gap directly from orthographic depth increased rain/blood from 67.35 to
74.64 ms. Removing the unused fourth inverse-projection component preserved all
fourteen images but showed no useful gain and slowed snow/blood slightly.

Seven focused contact/GPU/visual-state checks pass, along with strict native
and WASM Clippy and release builds. Browser WebGL2 rain and WebGPU snow checks
pass. The normal moving High routes measure 72.5/86.1/61.7/92.4 FPS, within the
recent run-to-run spread; they contain little settled litter. A separate longer
rain route was used for CPU profiling, so its FPS is not compared with these
short routes. Vertex packing and copying now account for the largest CPU shares.

Changing model/effect poses now write directly into `wgpu`'s transfer staging
buffer. This removes the intermediate CPU pose vector and its copy; cached
material attributes still upload only when changed. The byte layout and all
rendered geometry are unchanged.

A repeated native upload fixture uses 10,752, 49,152, 393,216 and 3,145,728
vertices, with stable attributes and with every opacity changing each frame.
For stable attributes, CPU mesh-update time changes from 0.0677 → 0.0586,
0.313 → 0.252, 3.944 → 3.354 and 36.066 → 31.464 ms respectively. Including
the completed GPU transfer, the largest case changes from 84.35 to 78.75 ms.
With changing attributes, the largest case changes from 45.14 to 40.72 ms of
CPU work; its additional attribute transfer still makes completion expensive
(170.95 → 168.94 ms). The 49,152-vertex changing-attribute case varies between
runs rather than showing a reliable gain. These are synthetic mesh updates
and transfers, not complete gameplay frames or a claim that extreme blood
counts can sustain 60 FPS.

Eight focused GPU checks pass: streamed/indexed geometry, changing materials,
effect blending/HUD ordering, moving shadows, blood/snow, contact effects and
weapon surfaces. Strict native/WASM Clippy and release builds pass, as do browser
WebGL2 rain and WebGPU snow checks. Paired moving High routes change from
72.4/86.0/59.8/93.5 to 73.2/86.9/62.6/91.8 FPS; maximum rain changes from
49.6/62.7/43.5/80.3 to 50.3/62.2/44.3/79.7. These small mixed changes do not
establish a general FPS improvement; the repeatable benefit is CPU transfer
preparation, particularly for large meshes.

Effects now record contiguous non-alpha ranges during the existing overlay
classification. Only these ranges are submitted to opaque and retail-translucent
passes, whose shaders already discard alpha-only vertices. Alpha-blended batches
keep their original ranges and order; HUD separation remains independent.

In a 768×432 isolated particle fixture, scene GPU time changes from 0.055 to
0.038 ms for 8,192 quads, 0.246 to 0.135 ms for 65,536 and 1.589 to 0.764 ms
for 524,288. With interspersed opaque triangles, the corresponding timings are
0.057 → 0.042, 0.248 → 0.136 and 1.463 → 0.768 ms. All six images match.
These timings exclude recurring uploads and are not gameplay FPS.

Eight 4K TRIAL/garden images with rain, snow, accumulated tracks and dense
falling leaves also match exactly. Their whole-scene gains are small or within
noise: snow/tracks changes from 11.919 to 11.879 ms total GPU time; garden rain
from 21.816 to 21.773 ms. The streamed-effects regression now compares filtered
draws against the former full opaque range, including all blend modes,
interleaved HUD geometry, buffer growth/shrinkage and HDR on/off.

Ten focused GPU checks pass, including retail/HD blood fading, mask edges, sprite
blend classes, modern blood cutouts, weather tracks and timestamp partitioning.
Strict native/WASM Clippy and release builds pass. Browser WebGL2 rain and WebGPU
snow checks pass. Moving High routes measure 72.5/85.9/62.7/93.3 FPS, maximum
rain 49.7/62.3/43.7/80.0, consistent with recent results.

Attached blood now caches each immutable patch's local bounds and skips its
vertex transforms and upload when the transformed bounds miss the camera view.
The bounds include secondary-motion offsets and the maximum shader wind sway.
Marks remain in memory and continue aging normally. The bounds cache is omitted
from saves and rewind serialization and is rebuilt lazily after restoration.

A rotating-camera CPU fixture uses 73 actual HD wound templates on animated
Mission 1 actors. It measures 180 frames after 30 warm-up frames, reusing the
same staging buffer:

| Body marks | Before assembly | After assembly | Before vertices | After, mean vertices |
| --- | ---: | ---: | ---: | ---: |
| 64 | 0.156 ms | 0.042 ms | 24,342 | 728 |
| 1,024 | 2.129 ms | 0.200 ms | 387,447 | 12,862 |
| 8,192 | 17.189 ms | 1.520 ms | 3,082,074 | 99,768 |

This camera usually sees only a small fraction of the stained actors. These
are CPU geometry times, not full-frame FPS. An intentionally wide view retaining
every mark measures 0.159 / 2.206 / 18.089 ms, versus
0.155 / 2.161 / 17.730 ms with culling disabled: the bounds checks add about
2–3% when nothing can be discarded. The first bounds calculation costs more
than subsequent frames; each live immutable patch reuses its cached bounds.

All three serialized blood states match. Seven captured geometry streams retain
exact ordered subsequences of the previous vertices. Sixteen full-scene images
and sixteen isolated blood-layer images match pixel for pixel across both
heroes, shifted camera edges, a near-plane view and a wide aspect ratio. The
isolated layer is also compared with an empty frame to confirm the comparison
contains rendered blood; the initial player close-up hid almost all its marks.
Twelve additional actor close-ups match. The bounds regression exercises
rotations, reflected/nonuniform scales, secondary deformation and wind offsets.

Thirteen focused checks pass, covering bounds, stain retention and style changes,
finite drips, death pools, complete visual-state restoration, rewind playback
and branching, and both heroes' save continuation across missions 1–10. Native
and WASM strict Clippy and release builds pass.

## November optimisation loop: cumulative result

The final real-desktop repeat uses the same 3840×2160 Modern High preset,
full v3 artwork, all-active enemies and four moving routes as the initial run.
The GTX 1080 Ti and Ryzen 9 9950X3D are unchanged. VSync is disabled to expose
throughput; each scene contributes 300 measured frames after 60 warm-up frames.
The final browser checks also pass on windowed Chromium: WebGL2 with rain
and WebGPU with snow, including movement, lighting toggles,
paused free camera and resumed play.

| Moving route | Initial High FPS | Final High FPS | Gain | Final maximum-rain FPS |
| --- | ---: | ---: | ---: | ---: |
| Village lights | 45.6 | 73.2 | 61% | 50.8 |
| Snow courtyard | 50.9 | 87.5 | 72% | 62.4 |
| Garden water | 31.8 | 62.8 | 97% | 44.5 |
| Waterfall | 54.6 | 92.0 | 68% | 81.7 |

High frame-interval p95 is 15.66 / 13.45 / 17.67 / 12.02 ms. The garden
therefore still drops below 60 FPS despite averaging above it. Maximum rain
and extreme overlapping blood remain expensive. These results do not establish
60 FPS throughout every mission, on every machine, or with every setting maxed.

The main gains come from less repeated lighting/weather shading, early rejection
of hidden scenery, indexed character meshes, smaller recurring pose transfers,
reused effect buffers and avoiding off-screen particle geometry. Separate stress
fixtures above cover accumulated snow, thousands of blood marks, contact effects
and rewind; their isolated CPU/GPU times should not be added together or treated
as whole-frame FPS.

Rewind now shares unchanged geometry and avoids re-encoding remaining blood
pages when older pages expire. Its first capture of a large existing collection
still has to encode that collection. Likewise, millions of simultaneously
visible animated stain vertices still cost substantial CPU work and transfer
bandwidth. This machine reports a PCIe 3.0 ×2 GPU link, making recurring uploads
particularly relevant; the optimisation results do not assume a wider link.

No preset quality, retained-effect limit, simulation tick rate or gameplay
mechanic was reduced. Paired image checks are predominantly pixel-identical;
the few retained shader arithmetic changes have the small rounding differences
documented above. Experiments that regressed representative workloads or showed
no useful gain were discarded. The loop is stopped at the user's request after
the attached-blood optimisation; these changes were released in v0.15.4.


## November 4: shadow visibility CPU work

Fresh 4K Modern High and maximum-rain runs reproduce the previous performance
range on the same GTX 1080 Ti / Ryzen 9 9950X3D. A CPU sample identified repeated
frustum tests in shadow preparation. Moving casters were tested once for draw
ranges and again for the region of the shadow map that needs refreshing. Those
results now share one traversal. The bounds arithmetic also uses the existing
portable aligned vector type, without changing the bounds or padding.

An isolated Mission 1 workload uses 1,461 actual shadow-geometry chunks and 18
camera directions. Vectorized bounds tests take 46.3 instead of 52.8 ms for
500 repetitions (about 13% less). Combining the traversals takes 9.8 instead of
13.6 ms for 40 repetitions with moving geometry (28% less); wind-animated
scenery takes 5.0 instead of 5.3 ms (7% less). These are aggregate CPU timings,
not frame times or GPU improvements. Draw ranges and refresh rectangles match
the previous calculation for every tested direction, wind and mobility mode.

Mission 1/5/9 comparison captures are pixel-identical. GPU checks compare culled
and full-stage shadows, moving regions against full redraws, wind-animated
foliage and snow shelter. Native strict workspace Clippy, WASM library Clippy
and formatting pass.

Unprofiled moving 4K runs put average render submission/preparation at
1.286 → 1.131 ms (village), 1.294 → 1.175 ms (snow courtyard),
1.147 → 1.047 ms (garden), and 2.667 → 1.585 ms (waterfall). Whole-frame
rates stay within run-to-run variation: 73.2 / 86.9 / 62.6 / 91.8 FPS before,
72.6 / 87.1 / 62.7 / 91.1 after. These views remain GPU-bound; this change
reduces CPU work rather than claiming a material FPS gain.


## November 4: weather lighting

Weather-shaded opaque surfaces reuse the same sun-shadow result for directional
light and attenuation of local lamps. The dry and alpha paths retain separate
samples: sharing across all paths regressed the garden and heavily overlapping
wet blood on this GPU. A fixed-width light-table lookup was also discarded;
removing runtime integer division made the wet-blood shader slower.

The static 4K TRIAL 2 fixture, play camera, measures:

| Workload | Before | After |
| --- | ---: | ---: |
| Rain | 13.52 ms | 12.43 ms |
| Daytime rain | 14.06 ms | 12.68 ms |
| Rain plus dense blood | 66.71 ms | 66.21 ms |
| Daytime rain plus dense blood | 70.58 ms | 69.08 ms |
| Snow plus dense blood | 44.21 ms | 43.40 ms |

These are GPU totals, not interactive frame intervals. The overhead camera's
rain case improves from 8.60 to 8.02 ms. All 14 paired images match exactly.
Moving maximum-rain runs improve from 50.0 to 52.1 FPS in the village,
62.2 to 63.0 in the courtyard and 43.3 to 44.0 in the garden; the waterfall
measures 81.1 versus 79.6 FPS. Dry High rates remain within normal variation
at 73.1 / 87.2 / 62.5 / 92.1 FPS with the final weather-only specialization.
Dense blood and heavy rain still exceed a 60 FPS budget.

Vulkan checks cover daylight shadows, local-light budgets, wet reflections,
metal weapons and blood blending. Chromium WebGL2 rain and WebGPU snow smoke
tests pass with lighting toggles, free camera, pause/resume and no rendering
errors. Native/WASM strict Clippy and formatting pass.


## November 4: terrain queries with active crowds

A CPU profile of 20,000 updates in Mission 1 with 128 extra active actors found
about one third of sampled execution in terrain-record reads and their endian
helpers. A node already validates its complete 16-byte record before reading
the fields; repeatedly validating each two-byte field prevented those reads
from being simplified. Decode the checked record directly with portable
`from_le_bytes` calls. No terrain cache or new invalidation rules are needed.

The same 1,200-update input sequence, with an invulnerable player and added actors
placed through the existing editor API, gives these mean simulation times:

| Mission | Active actors | Before | After |
| --- | ---: | ---: | ---: |
| 1 | Original population, all active | 0.0843 ms | 0.0527 ms |
| 1 | 128 extra, all active | 0.3895 ms | 0.2578 ms |
| 4 | Original population, all active | 0.0912 ms | 0.0549 ms |
| 4 | 128 extra, all active | 0.4501 ms | 0.2593 ms |
| 9 | Original population, all active | 0.0584 ms | 0.0405 ms |
| 9 | 128 extra, all active | 0.2150 ms | 0.1661 ms |

Ordinary activation also improves slightly (about 0.001 ms per update). These
are core simulation measurements, excluding audio and rendering; they do not
claim an equivalent increase in GPU-bound frame rate. Complete serialized game
state hashes match in all twelve scenarios: ordinary activation, all-active,
32 extra actors and 128 extra actors across the three missions. Parsing checks
cover unaligned records, signed extremes, live surface edits and invalid spans.
Original C comparisons for mission sight passages and movement probes pass,
along with the terrain arithmetic regressions, native/WASM strict Clippy and
formatting. The movement harness imports the decomp's newly extracted helper
functions as part of the same original C reference.


## November 4: fixed projection reciprocal table

A fresh eight-second CPU profile of Mission 1 with 128 extra active actors,
attached after mission loading, identified projection as a major remaining
cost. Disassembly showed integer division constructing a fixed reciprocal-table
entry for every projected vertex. The same 257-byte table is now generated at
compile time; reciprocal refinement, saturation, rounding and all GTE flags
remain unchanged. This uses ordinary portable Rust.

In the same twelve 1,200-update scenarios, mean simulation times improve from
0.0305 / 0.0290 / 0.0238 to 0.0293 / 0.0275 / 0.0220 ms with ordinary activation
in Missions 1 / 4 / 9. With all original actors active, means change from
0.0525 / 0.0543 / 0.0403 to 0.0510 / 0.0531 / 0.0391 ms. With 128 extra actors,
they change from 0.2442 / 0.2590 / 0.1736 to 0.2429 / 0.2533 / 0.1692 ms.
These are small CPU savings, not a claim of comparable GPU-bound FPS gains.
Full serialized game-state hashes match in all twelve cases. The 67,584-case
independent BizHawk/Mednafen GTE oracle, packet allocation/subdivision tests,
visibility boundaries, formatting and strict native/WASM Clippy all pass.

Other experiments were rejected: camera culling and tighter partial-shadow
draw bounds preserved their comparison images but gave no reliable moving
frame-time gain. Smaller light-grid entries reduced memory but slowed dry
gameplay; packing indices also regressed dense rain/blood by about 7 ms.
A conservative projection overflow fast path preserved the independent GTE
results but gave little measurable improvement. None of those changes remain.


## November 4: rewind resource capture

Steady recording compared the entire animation bank every update and serialized
all weapon geometry again. Both now use shared ownership and the existing
resource cache. Wall poses and imported movesets still modify the animation
bank through copy-on-write; unchanged wall keys avoid an unnecessary copy.
This does not change the serialized format or omit mutable data.

Two repeated, CPU-pinned comparisons run 960 Mission 1 updates with all enemies
active and record every update. Mean CPU times exclude the first 60 updates:

| Extra actors | Capture before → after | Compression/storage before → after | Combined before → after |
| --- | ---: | ---: | ---: |
| 0 | 0.153 → 0.111 ms | 0.168 → 0.166 ms | 0.321 → 0.277 ms |
| 32 | 0.206 → 0.158 ms | 0.218 → 0.215 ms | 0.425 → 0.373 ms |
| 128 | 0.372 → 0.325 ms | 0.346 → 0.339 ms | 0.718 → 0.665 ms |

This reduces recording cost by about 7–13%, not overall frame time. First
captures still encode the resources, and compression remains a significant
cost. The ordinary and 32-extra-actor cases retain the configured 30 seconds;
the 128-extra-actor case still reaches the memory budget after about 12 seconds.
Full-state hashes match the previous build for all three scenarios, and retained
snapshots decode back to identical serialized state. Existing mutation checks
cover Missions 1/2/6/9, now including changed weapon geometry, alongside wall
movement, imported movesets and all 17 built-in TRIAL missions with both heroes.


## November 4: weather variation interpolation

Rain puddles and snow coverage use smooth three-dimensional noise. Interpolating
the same eight corner values along each axis avoids repeatedly multiplying all
three corner weights. Sample positions, hashes and smoothing remain unchanged.

The first paired 4K TRIAL measurements put rain/blood at 66.28 → 65.95 ms,
daytime rain/blood at 69.39 → 68.62 ms, snow/blood at 43.59 → 43.06 ms, and
wet snow/blood at 46.52 → 46.00 ms. A repeat in reversed order with longer
warm-up also improves these cases, by 0.59–1.66 ms. Treat this as a small
shading improvement: moving maximum-rain rates are mixed at
51.8 → 52.5 / 63.2 → 63.4 / 44.4 → 44.1 / 80.9 → 79.5 FPS across the usual
four scenes, without a reliable overall gain.

Dry comparison images match exactly. Across twelve paired captures, at most
13 colour-channel values per image differ by one 8-bit level from floating-point
rounding. Twelve GPU checks cover shelter, wind, snow retention, reflections,
stains, water and restoration of original rendering. Sharing sun samples in
wet reflections was separately rejected: it increased heavy wet-blood rendering
from about 66 to 73 ms on this GPU despite doing fewer samples in source code.
Native/WASM strict Clippy, release builds and formatting pass. Chromium WebGL2
rain and WebGPU snow checks pass through movement, lighting toggles, paused
free camera and resumed play without rendering errors.


## November 4: moving-light table updates

Moving sources used to allocate a new ranking buffer and upload the complete
world light table even when its cell lists were unchanged. Ranking storage is
now reused. GPU uploads cover the span between the first and last changed rows,
or are skipped when only source positions changed and cell membership did not.
Positions, brightness and shadow uniforms still update normally.

Two CPU-pinned comparisons use each mission's actual HD lamps plus two or 32
moving sources. Each case runs 240 updates and excludes the first 40. Mean CPU
time for table construction and upload submission is:

| Mission | Two moving lights, before → after | 32 moving lights, before → after |
| --- | ---: | ---: |
| 1 | 0.317 → 0.301 ms | 0.321 → 0.309 ms |
| 2 | 0.172 → 0.148 ms | 0.175 → 0.159 ms |
| 7 | 0.133 → 0.120 ms | 0.136 → 0.125 ms |
| 9 | 0.159 → 0.129 ms | 0.162 → 0.143 ms |

This is about 4–19% less work in this small part of the frame. With two moving
sources, uploads occur on 107 / 133 / 98 / 96 of 200 measured updates instead
of every update. Stationary lamps keep their existing cache. All twelve final
table/uniform hashes match across the repeated runs, including stationary cases.
GPU readback verifies partial uploads through movement, growth, shrinkage,
disabled lighting and changed budgets against complete CPU tables. Existing
checks for distant lights, overlapping budgets and moving shadow casters pass.
Moving 4K Modern High rates remain 73.0 / 87.2 / 62.5 / 91.9 FPS. Native/WASM
strict Clippy, formatting and release builds pass. Chromium WebGL2 rain and
WebGPU snow checks perform a grapple, then movement and lighting toggles without
errors; captured upload calls include 1–12-row light-table updates instead of
the full 120-row table where applicable.


## November 4: blood and footprint projection setup

A CPU profile of repeated projection onto actual Mission 1 terrain attributes
about 16% of samples to trigonometry. Each receiving triangle recalculated the
same mark's axes, twelve clipping planes and blood-pattern rotation. These are
now prepared once per mark, including wounds projected onto multiple model
parts. Modern blood also skips the edge-opacity calculation that its analytic
shape immediately replaced. Footprints retain that calculation.

Repeated CPU-pinned comparisons project 946 marks per Mission 1 batch and
1,024 per Mission 2/9 batch, with 24 batches per case and the first excluded.
Two baseline and two final runs alternate order; mean CPU time per batch is:

| Mission | Footprints before → after | Blood stains before → after | Death pools before → after |
| --- | ---: | ---: | ---: |
| 1 | 8.07 → 6.96 ms | 5.16 → 3.30 ms | 11.61 → 7.10 ms |
| 2 | 8.93 → 7.64 ms | 5.53 → 3.51 ms | 13.34 → 8.19 ms |
| 9 | 8.92 → 7.59 ms | 5.64 → 3.52 ms | 12.08 → 7.52 ms |

This reduces creation work by 14–15% for footprints and 36–39% for blood,
not whole-frame time. All nine resulting meshes, totaling 993,588 vertices,
match the previous build byte for byte. Rendering and retention are unchanged;
heavy overlap still incurs the same GPU shading cost.
The 59 ordinary visual tests and nine disc-based regressions pass, covering
independent blood styles, finite wound bleeding, death-pool timing and size,
both heroes' footprints, distant patrols, original snow, contact effects and
combat/stealth-kill wounds. Native/WASM strict Clippy and formatting pass.


## November 4: precipitation shelter queries

Checking whether a drop is exposed previously searched for the nearest blocking
triangle. Boolean shelter and roof-edge queries now stop at any blocker. Impact
queries still search for the nearest surface, in the same traversal order.
The spatial index is used only by presentation effects, not gameplay collision.

Two baseline/final comparisons cover 24 Mission 1/2/9 cases: maximum-detail
rain and snow with no wind or maximum wind, both storm modes, and wind direction
changes every 120 updates. Each case prewarms 30 seconds and measures 1,200
updates plus geometry assembly, pinned to one CPU. All 24 complete weather
states and final geometry streams match byte for byte.

The clearest benefit is redirecting existing snow. Mean CPU time on wind-change
updates falls from 2.20 → 1.96 ms in Mission 1 and 1.18 → 0.84 ms in Mission 9;
Mission 2 stays about 0.63 ms. Steady maximum-wind Mission 1 snow changes from
0.0384 → 0.0346 ms per update, and snowstorm from 0.0577 → 0.0519 ms. Steady
rain results are small and mixed: Mission 1 maximum wind changes from
0.1441 → 0.1404 ms, Mission 2 from 0.0549 → 0.0557 ms. This does not change
GPU shading cost or establish an overall frame-rate improvement.
The exhaustive segment test now also checks boolean queries against a full
triangle scan, including empty geometry, parallel axes, exact boundaries and
zero-length segments. All 59 ordinary visual tests and four disc-based
footprint/contact regressions pass, as do formatting and native/WASM strict
Clippy.


## November 4: precipitation sorting and faded geometry

Weather geometry now calculates camera depth once per particle instead of
repeating it during every sorting comparison. The sort still includes the same
particles. Fully faded particles then skip streak-axis and width calculations
that could not produce visible geometry.

Using the same 24 cases and repeated CPU-pinned measurements as above, geometry
assembly falls by 13–23% for rain and 28–37% for snow. Selected mean times are:

| Mission | Maximum-wind rain before → after | Maximum-wind snow before → after | Snowstorm before → after |
| --- | ---: | ---: | ---: |
| 1 | 0.1219 → 0.0959 ms | 0.0660 → 0.0418 ms | 0.1020 → 0.0651 ms |
| 2 | 0.1423 → 0.1217 ms | 0.0606 → 0.0418 ms | 0.0950 → 0.0647 ms |
| 9 | 0.1507 → 0.1299 ms | 0.0619 → 0.0413 ms | 0.0958 → 0.0634 ms |

All 24 serialized weather states and final ordered geometry streams match
exactly. A separate comparison with many tied depths, five view directions and
particle counts up to 8,192 also preserves the old sorting order. These are CPU
assembly times, excluding uploads and GPU rendering. The views include many
off-screen particles, especially Mission 1; they do not establish the same
percentage improvement when every particle is visible.
Nine weather regressions, formatting, native/WASM strict Clippy and the native
release build pass. The moving 4K desktop smoke test completes at
72.2/85.8/59.7/93.3 FPS with High and 51.7/63.5/44.5/79.4 with maximum rain.
These mixed whole-frame results do not establish a GPU or overall FPS gain;
the garden still drops below 60 FPS.

## November 4: light-list insertion cutoff

A CPU profile attributes most light-table construction work to cell selection,
including repeated scans of each cell's sorted scores. A candidate that cannot
beat the last selected score now skips the scan and insertion immediately.
Strict ties retain their previous ordering.

Two baseline/final comparisons, pinned to one CPU, measure 1,200 cell-table
updates per mission with two moving sources and the actual HD lamps. Excluding
60 warm-up updates, mean construction time falls from 0.2891 / 0.1328 / 0.1075 /
0.1122 ms to 0.2684 / 0.1212 / 0.0990 / 0.1031 ms for Missions 1 / 2 / 7 / 9.
This is 7–9% less CPU work for that operation.

The complete table-update and upload-submission fixture, with two or 32 moving
sources, also improves by about 3–8%; stationary tables keep their existing
cache. All twelve table/uniform hashes match in repeated runs. An independent
exhaustive ranking test covers overlapping lights, tied scores, cell boundaries
and budgets of 1, 8 and 16. GPU readback through partial uploads and resizing,
distant-light and accumulation checks pass, as do native/WASM strict Clippy
and formatting. No GPU shading or whole-frame FPS improvement is claimed.

## November 4: rigid model transforms

Refined model transforms now use glam's portable aligned vector and matrix
types. Matrix construction and inverse-transpose calculation retain the
existing scalar arithmetic; vertices retain their packed representation.
Combining copying and transformation into one iterator was also tested, but
regressed assembly by about 25–30% and was discarded.

Repeated CPU-pinned measurements cover both heroes in Missions 1 and 2 with HD
artwork, refinement, interpolation, hair motion and all enemies active. Each
case runs 360 simulation updates and presents twice per update, excluding the
first 30 updates. Mean actor geometry assembly time is:

| Mission / hero | Normal population before → after | 32 extra guards before → after |
| --- | ---: | ---: |
| 1 / Rikimaru | 0.331 → 0.262 ms | 0.549 → 0.447 ms |
| 1 / Ayame | 0.329 → 0.260 ms | 0.552 → 0.445 ms |
| 2 / Rikimaru | 0.288 → 0.228 ms | 0.491 → 0.396 ms |
| 2 / Ayame | 0.290 → 0.227 ms | 0.496 → 0.397 ms |

This saves about 19–22% of assembly CPU time, excluding uploads and rendering.
All eight sampled vertex/index streams and final game states match exactly.
Eight 4K TRIAL/garden/waterfall comparisons, including rain and snow, are
pixel-identical. The 59 ordinary visual tests, folding-screen check and imported
skinned-model test on both heroes pass. The older disc-model export/import test
initially failed in both builds because its reference skipped material metadata
for raw weapons. Processing that metadata consistently fixes the fixture; the
full geometry/UV/blend/attachment comparison now passes on both heroes.
Native/WASM strict Clippy, formatting and release
builds pass. Chromium WebGL2 rain and WebGPU snow play checks pass through
grappling, movement, lighting toggles, paused free camera and resumed play.

## November 4: surface-detail shader specialization

The shader configuration now distinguishes bump mapping from parallax, allowing
the bump-only program to omit the unused height-ray loop. Surface-detail
settings, sampling and strength stay unchanged.

Two 90-frame 4K comparisons, with the second pair in reverse order and the last
40 frames measured, give the following TRIAL 2 GPU totals:

| Stress case | Before | After |
| --- | ---: | ---: |
| Rain with 4,096 projected blood marks | 65.77 / 66.41 ms | 60.65 / 60.55 ms |
| Snow with the same blood marks | 43.28 / 43.49 ms | 42.99 / 43.03 ms |
| Rain/blood with parallax enabled | 69.92 / 70.25 ms | 69.96 / 69.90 ms |

A daytime rain/blood comparison changes from 69.22 to 64.65 ms. The first suite
has eighteen paired images across TRIAL play/overhead, garden and waterfall
views, including all surface-depth modes; the reverse-order repeat adds ten.
Every image is pixel-identical. GPU clock logging during the repeat records
1,885 MHz under load. These deliberately extreme blood cases remain expensive.

Moving desktop High results vary with run order: the garden measures
61.8 → 59.8 FPS in the first pair and 59.4 → 60.9 in the reverse pair. Maximum
rain routes change from 51.6 / 62.9 / 44.3 / 80.6 to 51.9 / 63.3 / 44.3 / 79.9
FPS. These do not establish an ordinary-play FPS gain or a consistent garden
regression; the useful measured gain is overlapping wet blood.

Five focused GPU checks pass for surface-depth switching, scenery, blood,
weapon reflections and visibility. Native/WASM strict Clippy, formatting and
release builds pass. Chromium WebGL2 rain and WebGPU snow both switch live
through bump/parallax/off/bump, grapple, move, toggle lighting and resume from
paused free camera without errors or changing paused gameplay state.

## November 4: footprint assembly and slow-frame stress runs

Longer 4K measurements now emphasize p95/p99 frame times. Maximum rain in the
ordinary garden path has frame p95/p99 of 24.94/25.35 ms, GPU p95/p99 of
22.52/22.72 ms and CPU-work p95/p99 of 4.37/4.88 ms. Sustained GPU terrain
shading dominates this case. Combining shelter texture fetches, combining wet
reflection/diffuse lighting, and specializing absent weather were tested; their
regressions or insufficient gains did not justify keeping them.

A separate stress fixture adds projected blood and footprints while moving the
camera, with all enemies active and rewind recording. It uses 20 seconds of
fixed 60 Hz presentation per route, excluding the first 60 frames: 1,140 CPU
and 1,138 GPU samples. Sixteen stamp attempts per logic tick, one quarter for
footprints, grow the retained effects during the run; blood is capped at 4,096
marks. This is a deliberately heavy workload, not a claim about ordinary combat.

Profiling identified repeated allocation/copying in footprint assembly. Tracks
now cache their bounds, exclude marks outside the camera frustum with room for
wind displacement, and reserve the exact visible vertex count before copying.
Their positions, age, burial, retirement and serialization remain unchanged.
There is no distance cutoff: active distant guards still leave visible tracks.

Pinned CPU comparisons on actual Mission 1/5 surfaces, with approximately
65,500 footprint vertices, reduce unculled assembly from 0.297–0.312 ms to
0.075–0.079 ms. Camera culling reduces it further to 0.002–0.054 ms in eight
views. Retained vertices and serialized states match exactly; all eight
unculled/culled image pairs are pixel-identical. A regression check covers
tracks 100,000 units away, camera edges, wind and restoring the same history.

Two paired native Snow runs, with the second pair in reverse order, give:

| Route | Frame p95 before → after (two runs) | Frame p99 before → after (two runs) |
| --- | --- | --- |
| village-lights | 53.24 → 51.58; 53.47 → 51.32 ms | 55.32 → 53.51; 55.23 → 54.02 ms |
| snow-courtyard | 17.38 → 15.68; 17.46 → 15.95 ms | 17.64 → 16.09; 18.01 → 16.90 ms |
| garden-water | 28.71 → 27.33; 28.73 → 27.58 ms | 30.14 → 28.63; 30.24 → 28.62 ms |
| waterfall | 13.22 → 12.98; 13.27 → 12.64 ms | 13.57 → 13.22; 13.69 → 13.04 ms |

Geometry/upload CPU p95 falls by approximately 0.5–1.0 ms across these
routes. GPU times remain broadly similar, so the heavily stained village and
garden still miss 60 FPS. Rain stress results are mostly unchanged; the snowy
courtyard improves by about 0.7 ms at p95. This change addresses track assembly,
not the large wet-blood shading cost.

Formatting, native/WASM strict Clippy and release builds pass, together with
ordinary visual/rewind tests and four disc-based footprint/savestate checks.
Chromium WebGPU snow play passes movement, grappling, paused free camera,
lighting changes and resumed play without errors or changes to paused state.

## November 4: rewind cache insertions

Inserting a new shared resource previously replaced the next cache entry. That
could cascade through unchanged footprints and encode them again. The recorder
now inserts new Arc resources without evicting the remaining entries and finds
retained resources even when they are stored inline. The snapshot format is
unchanged; unused cache entries are still discarded after each capture.

The serialization-count regression failed before the change: inserting one
resource encoded seven unchanged resources again. It now passes insertion,
expiry, reordering, inline/shared transitions and decoding every retained state.

In two 4K snow/blood/footprint/rewind runs, village recording p99 falls from
3.45–3.55 to 1.72–1.82 ms. The matched repeat gives:

| Route | Rewind p95 before → after | Rewind p99 before → after | Frame p99 before → after |
| --- | ---: | ---: | ---: |
| Village | 1.54 → 1.51 ms | 3.45 → 1.72 ms | 53.38 → 53.72 ms |
| Snow courtyard | 0.82 → 0.81 ms | 0.95 → 0.94 ms | 16.35 → 16.18 ms |
| Garden | 1.35 → 1.33 ms | 1.73 → 1.57 ms | 28.53 → 28.47 ms |
| Waterfall | 0.74 → 0.75 ms | 0.81 → 0.89 ms | 13.18 → 13.08 ms |

Village CPU-work p99 falls from 8.34 to 7.64 ms. Overall frame times do not
establish a material improvement: heavy blood remains GPU-bound. These are the
same 1,140-frame stress samples described above, not ordinary-play guarantees.

Formatting, native/WASM strict Clippy and release builds pass. Checks cover
exact cached-versus-full mission snapshots after asset edits, item-active
rewind/branching on both heroes, timed reverse playback, restoring visual
history and the shared-resource memory budget.

## November 4: snow shelter bounds for overlapping blood

Modern ground stains can repeat the same expensive snow-shelter filter many
times over one pixel. Each weather shelter map now has a small hierarchy of
minimum/maximum world heights. On level ground, four bounds lookups can prove
that the complete filter is exposed or sheltered; ambiguous samples and slopes
still use the original filter. World heights avoid the depth gradient that wind
introduces on horizontal ground.

The hierarchy follows the existing dirty shadow regions and costs about
1.33 MiB at the highest weather resolution. It updates only when modern ground
blood needs it. A separate fragment entry keeps the original path for rain,
wet residue and ordinary scenery. No settings, sampling quality, retained-effect
limits or gameplay behavior change.

Two 20-second native 4K snow/blood/footprint/rewind pairs give:

| Village measurement | Before → after, first pair | Before → after, reverse pair |
| --- | ---: | ---: |
| Frame p95 | 51.56 → 47.74 ms | 52.42 → 47.59 ms |
| Frame p99 | 53.72 → 49.63 ms | 53.56 → 50.42 ms |
| GPU p95 | 47.24 → 43.67 ms | 48.07 → 43.13 ms |
| GPU p99 | 49.08 → 45.36 ms | 49.07 → 45.72 ms |

The courtyard, garden and waterfall show no consistent additional improvement.
These use the growing-effects stress fixture described above, with 1,140 CPU
and 1,138 GPU samples per route. The village remains far below 60 FPS at the
tail; this is not a claim that extreme overlapping blood is now inexpensive.

The final shader split was also checked in 360-frame offscreen TRIAL 2 runs,
excluding 60 warm-up frames, with 4,096 blood marks:

| Case | GPU mean before → after | GPU p95 before → after | GPU p99 before → after |
| --- | ---: | ---: | ---: |
| Rain | 60.63 → 59.85 ms | 62.58 → 60.58 ms | 63.68 → 61.01 ms |
| Snow | 43.30 → 39.45 ms | 44.07 → 41.65 ms | 46.30 → 44.18 ms |
| Snow over retained rain puddles | 46.22 → 45.93 ms | 46.55 → 46.54 ms | 46.78 → 48.12 ms |
| Snow with animated shadow casters | 43.82 → 39.28 ms | 44.43 → 40.04 ms | 44.87 → 40.47 ms |

Rain and wet-residue paths do not use the optimization; their variation does
not establish a gain. An earlier final-shader pair measured those means at
60.53/46.34 ms, and dry/animated snow at 39.00/40.13 ms.

All four final comparison images are pixel-identical. Twelve earlier pairs
also match, covering garden footprints, daytime lighting, lightning, wet
residue and disabled snow drift. An interrupted desktop repeat was discarded.
The new regression compares eighteen combinations of moving roof cover, wind,
weather quality and slopes against full filtering and full shadow redraws,
including disabling/re-enabling decals while cover changes. The existing
blood blending, weather exposure and moving-shadow checks also pass.

Formatting, native/WASM strict Clippy and release builds pass. WebGPU and
WebGL2 load the same portable snow/blood savestate, execute the bounds passes,
move, toggle lighting, use paused free camera and resume without errors or
altering paused gameplay. Both browsers' paused captures match their respective
full-filter reference images exactly. Sampling array layers explicitly also
avoids WebGL2's unsupported sampled views of individual array layers.

## November 4: crowded-scene interpolation

A 4K Modern High stress fixture adds 128 guards using the existing placement
API, keeps every actor active, and lets combat proceed during four moving
camera routes. It contains about 155 actors; this is an artificial stress case,
not a normal mission population. Each route measures 1,140 frames after warm-up.
The virtual-display offscreen mode serializes rendering and completion, so its
frame intervals must not be treated as interactive FPS.

Baseline geometry/upload p99 is 7.7–8.9 ms. CPU profiling identifies geometry
packing, model transforms and ordered pose-map searches among the larger costs.
Packing poses into a temporary CPU vector before a bulk GPU upload makes CPU
work about 0.8 ms slower; that experiment was discarded after a repeat baseline.

Interpolation now walks the previous and current ordered snapshots together,
and builds the resulting map in bulk. This replaces repeated searches and
insertions without changing stored data or the interpolation math. In an
alternating reference/candidate CPU fixture with 128 extra guards, both heroes
and missions 1, 2, 5 and 10, sampling means fall from 0.376–0.456 ms to
0.280–0.334 ms. For mission 1 Rikimaru, p95/p99 falls from 0.475/0.478 ms to
0.347/0.351 ms. Serialized presentation frames match exactly at five fractions
of each of 180 logic ticks in all eight runs.

The moving end-to-end runs do not establish an overall p95/p99 improvement;
variation in other work exceeds this approximately 0.10–0.12 ms local saving.
The larger geometry/upload costs remain the next target.

Formatting and strict native/WASM Clippy pass. Focused checks cover changing
part sets, teleports, hinged meshes, both heroes' body/weapon interpolation,
alternative deaths with attached effects, and savestate visual continuation.

## November 4: request poses only for affected effects

Lantern-source collection previously interpolated every body/weapon part before
checking whether its geometry emits light. Blood likewise built complete pose
maps whenever any attached stain, wound or coated weapon needed one. Lighting
now evaluates poses after finding an emitter; blood requests just the affected
parts, including newly struck weapons. The shared appearance path still handles
swapped rigs, attachments, visibility and inspection poses.

In the same 4K crowded fixture, two pairs reduce CPU-work means by about
0.2–0.5 ms. The repeat pair gives:

| Route | CPU work p95 before → after | CPU work p99 before → after | Frame p99 before → after |
| --- | ---: | ---: | ---: |
| Village | 9.65 → 9.32 ms | 10.75 → 10.57 ms | 31.30 → 31.78 ms |
| Snow courtyard | 10.51 → 9.95 ms | 11.84 → 11.07 ms | 32.79 → 32.12 ms |
| Garden | 9.44 → 9.25 ms | 10.36 → 10.28 ms | 33.33 → 33.12 ms |
| Waterfall | 10.43 → 10.20 ms | 11.36 → 11.18 ms | 25.91 → 25.46 ms |

Whole-frame tails remain noisy and the village p99 does not improve. This is
less CPU work, not a solution to GPU-heavy rendering or a 60 FPS guarantee.

A private old-path comparison matches the complete serialized blood state for
900 combat ticks per hero. All 3,600 rendered-geometry comparisons also match
byte for byte across inspection/ordinary views and three interpolation fractions.

Formatting and strict native/WASM Clippy pass. Focused disc checks cover both
heroes' swapped bodies and weapon attachments, lanterns across ten missions,
stealth-kill/combat coatings, independent blood styles, finite wounds, player
pools and continued visual state after saving/loading.

## November 4: avoid repacking unchanged mesh attributes

Dynamic meshes already uploaded positions/normals separately from material
attributes, but still rebuilt and copied the attribute array on every update.
The renderer now compares attribute fields directly with the retained copy and
rebuilds it only when needed. Changed attributes are packed into that retained
vector, removing the separate scratch vector and its extra copy. Comparisons
preserve float bits, including signed zero. Mesh growth/shrinkage and material
changes still trigger the necessary upload.

A byte-comparison prototype was rejected: generated code called `bcmp` for
every vertex, increasing CPU time. Direct field comparisons eliminate those
calls. The final code has fewer lines and no separate upload-state object.

The final 4K crowded High run, relative to the preceding effect-pose change:

| Route | CPU work p95 before → after | CPU work p99 before → after | Frame p99 before → after |
| --- | ---: | ---: | ---: |
| Village | 9.32 → 8.95 ms | 10.57 → 10.05 ms | 31.78 → 31.19 ms |
| Snow courtyard | 9.95 → 9.55 ms | 11.07 → 10.34 ms | 32.12 → 30.97 ms |
| Garden | 9.25 → 8.78 ms | 10.28 → 9.52 ms | 33.12 → 33.33 ms |
| Waterfall | 10.20 → 9.89 ms | 11.18 → 10.80 ms | 25.46 → 25.71 ms |

CPU-work means improve by 0.35–0.40 ms. The earlier direct-comparison run also
improves CPU p95/p99 on all four routes; whole-frame tails remain mixed. These
are the serialized offscreen stress measurements, not interactive FPS.

A separate 20-second-per-route 4K pair enables maximum rain, adds blood and
footprints continuously, and records rewind history. Geometry/upload means are
1.86/1.43/1.85/2.67 ms before and 1.84/1.40/1.74/2.57 ms after. CPU p99 is
6.58/4.85/6.16/6.72 ms before and 6.55/4.78/6.04/6.57 ms after. This checks
frequently changing effect attributes as well as stable character materials.
The heavily stained village remains GPU-bound: frame p95/p99 is
79.82/81.33 ms before and 79.94/81.47 ms after. No GPU improvement is claimed.

Formatting, strict native/WASM Clippy and release builds pass. GPU readback
checks cover growing/shrinking indexed and unindexed meshes, changed attributes,
empty frames, streamed blended effects/HUD, moving shadows and blood blending.
Chromium WebGPU and WebGL2 also load the portable snow/blood state, move, change
lighting, use paused free camera and resume without errors or modifying paused
gameplay state.

## November 4: use actor boundaries for indexed shadow casters

Animated indexed meshes now retain each actor's contiguous vertex/index ranges.
Shadow bounds scan the unique vertices in each range, instead of repeatedly
visiting triangle corners and guessing where one actor ends. The existing path
remains for geometry without known groups. No geometry is removed or reordered.

The Mission 1/2 crowded fixture has 187,000–203,000 unique vertices versus
493,000–525,000 indices. Actor groups reduce the number of culling chunks from
349–409 to 138–153. The first isolated bounds comparison reduced mean preparation
from 0.54–0.58 ms to 0.38–0.41 ms. Across eight nearby lamp projections, grouping
submits 1.3–4.3% more actor indices but fewer draw ranges; projected update area
can increase or decrease with the view. GPU savings are not assumed from fewer
CPU groups.

A sequential 20-second-per-route 4K Modern High comparison with 128 additional
active guards gives:

| Route | CPU work mean before → after | CPU work p95 before → after | CPU work p99 before → after |
|---|---|---|---|
| Village | 7.38 → 6.98 ms | 9.11 → 8.95 ms | 10.06 → 9.92 ms |
| Courtyard | 8.15 → 7.52 ms | 10.07 → 9.40 ms | 11.32 → 10.42 ms |
| Garden | 7.35 → 6.73 ms | 9.00 → 8.33 ms | 10.32 → 9.04 ms |
| Waterfall | 8.73 → 8.25 ms | 10.26 → 9.71 ms | 11.54 → 10.90 ms |

GPU means are 13.51/11.49/16.09/8.80 ms before and
13.83/11.35/15.83/8.75 ms after. Whole-frame p99 remains mixed, including a village
regression from 31.82 to 38.95 ms; this is a CPU improvement, not a demonstrated
uniform frame-tail improvement. These offscreen runs serialize CPU/GPU work and
are not interactive FPS measurements. An earlier pair was discarded because a
separate actively running game competed for the GPU; the retained pair ran while
that game was idle.

Expanded geometry remains byte-identical across five mission/hero combinations,
original/refined models, inspection and interpolation. GPU comparisons cover
changed attributes, reversed vertex storage, growth, empty meshes, separated
actors, moving light/actor shadows, wind, snow, partial updates and full redraws.
Formatting, native and WASM strict Clippy and release builds pass. Chromium
WebGPU and WebGL2 also load the portable snow/blood state, move, change lighting,
use paused free camera and resume without errors or changing paused game state.

## November 5: opaque depth coverage with unfiltered artwork

The depth pass already knows which replacement materials are fully opaque.
It now uses that information with nearest and bilinear filtering too, avoiding
texture and relief sampling that cannot change coverage. Cutouts, blended
materials and the final surface shading retain their usual paths.

A reconstructed Mission 1 view uses the diagnostic report's 3826 × 2146
resolution, camera matrix and effective graphics: HD artwork, parallax, eight
2048-pixel soft local shadows, rainstorm, wet reflections and post-processing.
The fixture reproduces the reported GPU cost closely, but does not restore the
report's actor states or exact weather accumulation. Its wetness/puddles are
pre-filled to the configured levels. On the GTX 1080 Ti, sequential runs with
60 warm-up frames and 180 measured frames give:

| View/settings | GPU mean before → after | GPU p95 before → after | GPU p99 before → after |
|---|---|---|---|
| Report camera/settings | 21.50 → 20.62 ms | 21.97 → 21.20 ms | 22.21 → 21.31 ms |
| Same view, dry control | 15.30 → 14.31 ms | 15.81 → 14.73 ms | 16.04 → 14.92 ms |
| Same view, daytime | 22.21 → 21.21 ms | 22.72 → 21.70 ms | 24.15 → 21.89 ms |
| Heavy rain and 4,096 projected stains | 27.73 → 26.94 ms | 28.28 → 27.46 ms | 28.38 → 27.59 ms |

The report view's visibility pass falls from 1.81 to 0.88 ms. An initial separate
pair measured total GPU time at 21.51 → 20.38 ms. Seven before/after 4K captures
are pixel-identical. GPU regressions cover replacement alpha classes, filtering
modes, coplanar draw order, indexed moving actors, effects, MSAA and HDR.

These are GPU measurements, not interactive frame rates. The report view remains
above the 16.67 ms budget for 60 FPS. Diagnostic changes indicate that soft local
shadow filtering and wet reflections dominate its remaining cost; rain particle
geometry costs approximately 0.13 ms. Disabling those effects was only a profiling
control. Unrolling shadow filters gave no gain, and caching one light's shadow
between diffuse lighting and reflections slowed this view; neither experiment
was retained.

Four 20-second moving benchmark routes using the report's graphics at the same
resolution also improve slightly. GPU p95 is 31.44 → 30.64 ms in the village,
28.93 → 28.76 ms in the courtyard, 30.53 → 30.20 ms in the garden and
11.10 → 10.90 ms at the waterfall. These offscreen runs include simulation and
render preparation but serialize CPU/GPU execution. Moving shadow-map updates
add substantial cost: the baseline village spends 10.52 ms on them on average,
versus almost zero with the fixed view and poses. Native/WASM strict Clippy,
release builds and Chromium WebGPU/WebGL2 gameplay, lighting, free-camera and
portable weather-state checks pass.


## November 5: shadow scenery before foliage

Direct GPU timestamps on the moving report-settings routes separate point-light,
directional and weather shadow maps. In the village their mean costs are
8.57, 1.53 and 0.42 ms respectively. Local maps, rather than the moon or rain
shelter, account for most of the moving shadow cost.

Shadow-stage geometry now groups solid scenery before foliage while retaining
spatial ordering within each group. Buildings can reject hidden leaf fragments
earlier. This changes only shadow geometry ordering; the color passes retain
their original draw order and the number, resolution and filtering of shadows
are unchanged.

Sequential 20-second offscreen routes at 3826 × 2146 with the report's graphics,
after the opaque-depth improvement above, give:

| Route | Shadow-map mean before → after | GPU p99 before → after | Whole-frame p99 before → after |
|---|---|---|---|
| Village | 10.54 → 9.34 ms | 31.14 → 29.39 ms | 39.44 → 37.28 ms |
| Courtyard | 6.87 → 6.20 ms | 29.09 → 28.84 ms | 35.92 → 35.82 ms |
| Garden | 9.17 → 8.45 ms | 30.67 → 29.76 ms | 38.27 → 37.41 ms |
| Waterfall | 2.62 → 2.51 ms | 11.28 → 11.21 ms | 22.76 → 22.85 ms |

Four paired report-view captures are pixel-identical, covering fixed poses,
animated actors and foliage, daytime, and heavy rain with 4,096 blood stains.
In that fixed-camera animated case, shadow-map time falls from 3.89 to 2.77 ms.
The moving routes remain above the budget for 60 FPS; these serialized offscreen
measurements are not interactive frame rates. Reducing geometry to each updated
shadow scissor was also tested and rejected: it increased CPU preparation without
reducing GPU time.

Shadow culling, partial redraws, wind animation and actor-independent snow
shelter regressions pass. Native/WASM strict Clippy and release builds pass;
Chromium WebGPU and WebGL2 load the portable snow/blood state, move, change
lighting, use free camera and resume with paused simulation unchanged.


## November 5: depth ordering for large shadow maps

Static shadow chunks now draw near to far within the solid and foliage groups.
This lets earlier surfaces reject more hidden fragments. The per-face culling
cache retains the ordered ranges; unchanged static geometry is not sorted every
frame. Color-pass ordering, animated poses, shadow coverage and filtering are
unchanged.

The extra draws pay off on large maps. At 512 pixels, an animated report-view
fixture saves only 0.01 ms on the GPU while adding 0.17 ms of CPU work; at 1024,
the 0.08 ms GPU saving approximately offsets the CPU cost. Depth ordering is
therefore limited to static maps of at least 2048 pixels. Smaller maps and
moving actor meshes retain their existing merged draw ranges.

Sequential 20-second offscreen routes use the same 3826 × 2146 report settings
as above, with a repeated baseline after the preceding shadow-order improvement:

| Route | Shadow-map mean before → after | GPU p99 before → after | Whole-frame p99 before → after |
|---|---|---|---|
| Village | 9.53 → 8.12 ms | 30.67 → 28.71 ms | 37.98 → 36.48 ms |
| Courtyard | 6.29 → 5.40 ms | 28.83 → 27.68 ms | 35.30 → 34.17 ms |
| Garden | 8.44 → 7.35 ms | 29.75 → 28.97 ms | 37.00 → 36.42 ms |
| Waterfall | 2.51 → 2.36 ms | 11.19 → 10.95 ms | 22.25 → 22.04 ms |

CPU means rise by 0.08–0.31 ms because there are more draws. The demanding routes
still improve overall; the waterfall's mean is essentially unchanged. These
serialized measurements remain above the 60 FPS budget in the three demanding
routes and are not interactive FPS measurements.

Four paired 4K captures, including animated foliage, daytime and rain with
4,096 blood stains, are pixel-identical. The full-stage shadow comparison now
also exercises 2048-pixel maps. Partial redraw, wind and snow-shelter checks,
native/WASM strict Clippy and release builds, and Chromium WebGPU/WebGL2
weather-state, movement, lighting and free-camera checks pass.

Two scenery-shader splits were also tested and discarded: their modest gains
in some views did not consistently improve the report's night-time workload.
Skipping only the shadow vertex wind calculation did not improve map time, so
no separate wind-position cache was added.


## November 5: preserve shadows across unrelated texture animation

Every VRAM revision previously invalidated all shadow faces and their culling
ranges. Updating material scroll coordinates did the same, even for water that
cannot cast a shadow. Ordinary texture animation therefore repeatedly discarded
otherwise reusable results.

Material records now track whether geometry can use them for shadow coverage.
VRAM and material uploads continue normally, but only changes relevant to casters
invalidate the maps. Original palette/cutout changes, animated replacement
coordinates and lantern transmission retain conservative invalidation. Legacy
stains that sample VRAM directly track their texture pages and palettes; changing
them still refreshes shadows. Pack replacement and atlas rebuilding invalidate
all maps as before. No additional shadow-map allocation is needed.

The same four 20-second offscreen routes use the report's 3826 × 2146 settings,
including eight 2048-pixel local shadows, 4096-pixel moon shadows, rain, wet
reflections, HD artwork and active enemies:

| Route | Shadow-map mean before → after | GPU p99 before → after | Whole-frame p95 before → after | Whole-frame p99 before → after |
|---|---|---|---|---|
| Village | 8.13 → 6.48 ms | 28.57 → 24.91 ms | 36.01 → 31.32 ms | 36.70 → 32.01 ms |
| Courtyard | 5.51 → 2.49 ms | 27.73 → 21.96 ms | 33.77 → 27.27 ms | 34.36 → 28.12 ms |
| Garden | 7.36 → 6.76 ms | 28.95 → 27.66 ms | 35.51 → 33.81 ms | 36.04 → 34.46 ms |
| Waterfall | 2.37 → 2.35 ms | 11.15 → 11.03 ms | 21.57 → 21.21 ms | 22.36 → 21.95 ms |

CPU work means fall from 3.74/3.13/3.61/5.74 to 3.39/2.86/3.36/5.66 ms.
These serialized offscreen measurements are not interactive FPS; the demanding
routes still exceed the 16.67 ms budget for 60 FPS.

Four 4K captures match forced full shadow redraws exactly, including animated
VRAM updates, foliage, daytime and rain with 4,096 blood stains. The point-light
regression checks that unrelated/water texture changes reuse maps while original
and HD cutouts, lantern transmission and raw legacy-texture changes stay correct.
Incremental-atlas, scrolling, partial-shadow, wind and snow-shelter checks pass,
as do formatting, native/WASM strict Clippy and release builds. Chromium WebGPU
and WebGL2 both pass weather-state loading, movement, free-camera, lighting-toggle
and resume checks.

Two earlier approaches were rejected. Conservative wall occlusion gave no
consistent moving-frame improvement. A separate static-depth cache added
768 MiB at these settings and made moving-frame tails worse because the same
texture updates kept invalidating it. Neither experiment is included.

## November 5: keep actors out of shelter-map invalidation

The shelter shader discards characters, but their meshes previously participated
in weather-map culling and cache keys. Adding or moving a character could therefore
redraw both shelter maps without changing either map's contents. Shelter drawing
now excludes meshes with no scenery bounds. Mixed meshes remain eligible, so
hinged doors and other moving scenery retain their original behavior.

The existing snow-shelter regression now checks cache reuse through actor insertion
and movement, plus redraws when the same mesh becomes or stops being a roof. It
fails on the old code (two unnecessary map draws) and passes with the filter.
Four paired 4K report-view captures, including animated actors, foliage, daytime
and heavy rain/blood, are pixel-identical; their forced full-redraw references
also match. Fixed-camera timings show no reliable whole-frame gain: animated
scenery already requires shelter updates, and mixed actor/scenery meshes still
participate. This is a targeted cache correction, not a claim that the report's
30 FPS workload now reaches 60 FPS.

Further experiments were discarded. Combining overlapping blood opacity can
roughly halve a heavy blood shader's cost, but equal shading cannot be assumed
across different surfaces. Checking base colours reduces the rain stress gain
from about 29 ms to 5 ms while adding roughly 158 MiB at 4K, and still needs
normal/height safeguards. No such shortcut is enabled. Retesting a 768 MiB static
shadow-depth cache after selective texture invalidation gives mixed moving-route
results, with regressions in the village and waterfall, so it remains excluded.

On the four 20-second moving routes, shelter filtering changes total shadow-map
means from 6.48/2.49/6.76/2.35 ms to 6.45/2.47/6.72/2.34 ms; frame tails remain
mixed and effectively unchanged overall. Six focused GPU regressions, native/WASM
strict Clippy, formatting and release builds pass. Chromium WebGPU and WebGL2 both
load the snow/blood checkpoint, move, toggle lighting, use free camera and resume
without errors or changes to paused simulation state.

## November 5: compose blood only on matching receivers

Overlapping modern ground stains previously repeated the same material, lighting,
shadow and weather calculations. A coverage pass now accumulates opacity and
identifies pixels whose contributing stains all came from the same source
triangle. Only the last contributing triangle shades those pixels. Mixed or
unknown receivers retain their individual shading and original blending order.

The proof uses the full source triangle's bitwise geometry, UVs, colour, flags
and normals, not a model-local index or allocation address. Only horizontal
receivers with a downward projection qualify. Source ranges follow immutable
blood pages through trimming, style changes, saves and rewind. Renderer IDs live
in a separate vertex buffer; texture identifiers and gameplay state are unchanged.
Retained pages now draw in creation order rather than allocation-address order.

This path requires HDR, one sample per pixel and independent attachment blending.
The renderer keeps forward drawing for sparse/distant stains, incompatible
backends or mixed visible styles. Coverage targets allocate only when useful,
adding approximately 111 MiB at 3840 × 2160. Pipelines compile when graphics
settings are applied, rather than when the first dense pool appears.

Paired 4K tests on the GTX 1080 Ti with the report's graphics settings and 4,096
stains in TRIAL Mission 2 give:

| Workload | GPU mean before → after | GPU p99 before → after |
|---|---|---|
| Rain and blood | 60.53 → 29.08 ms | 60.99 → 29.70 ms |
| Snow and blood | 38.25 → 20.98 ms | 39.64 → 21.77 ms |
| Wet snow and blood | 44.80 → 23.72 ms | 45.03 → 24.46 ms |
| Rain, blood and moving actors | 62.09 → 30.20 ms | 63.02 → 30.84 ms |

Four 20-second moving routes at 3826 × 2146 add stains and footprints while
recording rewind, exercising metadata creation and GPU uploads as well as shading:

| Route | GPU p99 before → after | Frame p95 before → after | Frame p99 before → after |
|---|---|---|---|
| Village | 72.37 → 39.69 ms | 80.10 → 48.67 ms | 82.00 → 49.86 ms |
| Courtyard, no accumulated stains | 21.73 → 21.87 ms | 27.85 → 27.86 ms | 28.35 → 28.67 ms |
| Garden | 36.04 → 31.24 ms | 42.52 → 38.82 ms | 44.79 → 40.45 ms |
| Waterfall, no accumulated stains | 10.98 → 11.00 ms | 22.53 → 22.51 ms | 23.10 → 23.13 ms |

Village CPU work averages 4.35 → 4.55 ms; the other routes remain roughly unchanged.
These are serialized offscreen timings, not interactive FPS. The ordinary Mission
1 report views stay at approximately 20.5/22.5/21.3 ms GPU for stationary,
animated-night and animated-day views, with identical images. This improvement
addresses blood accumulation; it does not make the ordinary 30 FPS report reach
60 FPS.

The composition is not bit-exact: half-float alpha accumulation and different
triangle interpolation introduce small rounding differences. The Mission 1 blood
capture differs in 5,635 colour channels out of 24.6 million, by at most 3/255.
TRIAL stress captures differ in 18,202–40,643 channels out of 24.9 million, with
mean absolute differences below 0.002/255 and isolated texel-edge differences up
to 29/255. Enlarged worst-pixel crops were inspected; final captures match those
reviewed prototype images. All report captures also match forced full shadow
redraws. Synthetic mixed-floor, slope and foliage cases are exact; matching
receivers and mixed known/unknown identities differ by at most 1/255.

Focused tests cover receiver changes/rebuilding, high renderer IDs, texture
cutouts, sparse and distant fallback, multisampling/HDR fallback, style removal,
trimming, rewind and save restoration. The internal snapshot format is incremented
so older development snapshots fail clearly rather than decoding the new metadata
as old fields. Campaign progress files are unaffected.

Formatting, 134 ordinary tests, focused GPU checks, native/WASM strict Clippy and
release builds pass. Chromium WebGPU loads a native snapshot and executes the
new coverage pass; WebGL2 passes with forward rendering. Both exercise movement,
paused-state stability, lighting changes, free camera and resume. Browser validation
uses the actual `cargo rustc --crate-type cdylib` artifact; compiling the WASM Rust
library alone does not refresh the browser binary.

A dry lit blood stress test also improves from 24.49 to 16.63 ms GPU. The same
geometry with lighting, sky, weather and surface detail disabled instead regresses
from 6.28 to 8.29 ms with composition forced on: the extra coverage pass outweighs
its saved shading. Composition therefore remains disabled when both lighting and
weather are disabled. The fallback regression checks this configuration too.


## November 5: ordinary Mission 1 baseline and shadow sample locality

A fresh ablation of the report's 3826 × 2146 Mission 1 view gives 20.44 ms GPU
with all supplied graphics settings. Disabling local shadows reduces this to
9.03 ms; retaining shadows but removing area-source softening gives 11.96 ms.
Disabling rain reflections gives 16.56 ms, all weather 14.26 ms, relief 18.80 ms,
and ambient occlusion 18.94 ms. These are diagnostic reductions, not changes to
users' settings. Soft point-light filtering is the largest remaining shading cost.

The retained change groups the existing 32 disk samples into spatial rows when
building their uniform table. Positions, weights, resolution and shadow geometry
are unchanged; only the order of texture accesses and floating-point accumulation
changes. The dry shader retains its existing local sample calculation. There are
no extra render passes or textures and no per-frame CPU sorting.

Four paired 4K captures are pixel-identical. GPU means in the reported view change
from 20.65 to 20.26 ms at night and 21.18 to 20.84 ms by day. A second Mission 1
garden view stays effectively unchanged, at 19.57 → 19.53 and 19.96 → 19.99 ms.
Longer offscreen routes show small village/courtyard gains and effectively neutral
garden/waterfall GPU tails. This is a modest improvement, not a 30-to-60 FPS fix.

Actual Wayland presentation at 3840 × 2160, using the report's graphics settings,
HD artwork and four ten-second routes, gives the following frame distributions.
The first 60 frames of each route and loading are excluded; no other GPU benchmark,
compiler or browser ran concurrently.

| Route | Frame p95 before → after | Frame p99 before → after |
|---|---|---|
| Village | 27.13 → 27.20 ms | 27.73 → 27.63 ms |
| Courtyard | 23.00 → 22.83 ms | 23.87 → 23.53 ms |
| Garden | 29.45 → 29.22 ms | 30.07 → 29.76 ms |
| Waterfall | 14.75 → 14.74 ms | 15.65 → 14.98 ms |

These changes are small relative to run-to-run variation. The difficult Mission 1
routes still average approximately 41 and 37 FPS. Presented frames are much closer
to GPU time than the serialized offscreen benchmark's CPU-plus-GPU intervals.

Rejected experiments are kept out of production: screen-space caches for diffuse
lighting or per-light visibility add more pass/memory cost than they save. A
same-fragment two-bit shadow-result cache improves the reported corner from 20.6
to 18.9 ms but worsens outdoor Mission 1 terrain by roughly 0.7 ms. Restricting it
to scenery removes the effect-particle regression but not the terrain regression.
Caching full per-light or sunlight visibility also fails the broader-view checks.

Validation: formatting, strict native/workspace and WASM Clippy, native release
and actual WASM cdylib builds pass. All 134 ordinary application tests and five
focused GPU regressions pass, including a penumbra comparison against the original
locally calculated samples. Actual Chromium WebGPU and WebGL2 smoke tests pass
with native saved-state loading, movement, lighting changes and resume.

## November 5: cumulative comparison with the diagnostic's development revision

The user's 0.15.5 diagnostic was produced by development revision
`3bbe8aa-dirty`, rather than the published tag. A saved native binary from that
revision, made before the diagnostic, provides the closest available baseline.
The dirty revision string does not prove byte-identical source. Both builds run
the same fixed camera paths and report graphics at 3840 × 2160 on Wayland, with
HD artwork, rainstorm, eight soft shadow lights and all-active enemies. Each path
has 1,200 frames, excluding 60 warm-up frames. These are actual windowed timings;
loading, compilation and other GPU benchmarks are excluded.

| Route | Average FPS, report-era → `26d291f` | Frame p95 | Frame p99 | GPU p99 |
|---|---|---|---|---|
| Mission 1 village | 36.0 → 40.6 | 33.20 → 27.56 ms | 34.42 → 28.06 ms | 30.83 → 24.97 ms |
| Mission 5 courtyard | 38.6 → 46.8 | 30.71 → 22.65 ms | 31.18 → 23.10 ms | 28.08 → 20.56 ms |
| Mission 1 garden | 33.7 → 37.7 | 32.51 → 28.48 ms | 33.10 → 28.91 ms | 29.69 → 26.12 ms |
| Mission 4 waterfall | 69.0 → 72.0 | 15.48 → 14.76 ms | 15.95 → 15.08 ms | 10.80 → 10.37 ms |

Most of the improvement comes from less shadow-map work: mean shadow time falls
from 10.23 to 7.27 ms in the village, 6.68 to 2.42 ms in the courtyard and 8.89 to
6.54 ms in the garden. Village CPU p99 falls from 5.59 to 4.88 ms. Gameplay settings
and visual quality are unchanged. Mission 1 still falls short of 60 FPS with this
configuration; these paths reconstruct the graphics workload, not the diagnostic's
complete actor and weather history. The report has no selected custom player body,
so the redacted model directory does not affect its rendered player.

Further private experiments were rejected: selectively clearing shadow pixels
adds more mask/render-pass work than it saves; early terrain lighting and ambient
occlusion gather variants fail to improve the rainy views. Precomputing radial
shadow offsets saves roughly 0.2–0.3 ms in fixed views but produces mixed moving
frame tails in repeated runs, so it is not included in the comparison above.

A subsequent conservative solid-polygon shortcut was also rejected. It skipped
32-tap filtering only where opaque scenery covered the entire sampling area.
Ten paired 4K captures and twenty cached/full-redraw comparisons were exact;
texture changes, moving geometry, large coordinates and both browser backends
passed their correctness checks. The reported corner improved from 21.04 to
17.06 ms GPU, but that did not generalise to the slowest moving route.

Against the previous production binary, the final version improved village
frame p99 from 27.66 to 26.41 ms and courtyard p99 from 23.40 to 21.59 ms, while
the garden worsened from 29.73 to 30.52 ms. Earlier repeat runs were mixed.
Shortening lighting intermediate lifetimes reduced the garden regression, but
still added about 0.34 ms GPU and gave smaller village gains. The polygon cache,
shader shortcut and lifetime experiment were removed together. None of those
experimental gains are included in the retained cumulative results above.


## November 5: separate wet materials from terrain lighting

Reflective wet terrain now prepares its materials before a dedicated lighting
pass. Diffuse light and rain reflections share each shadow visibility result,
and the lighting shader no longer carries texture and weather intermediates
through the shadow filter. Cutouts and coplanar faces retain their original
draw order; water and unlit materials use ordered forward batches. Actors,
particles and decals keep their existing rendering.

This path is limited to HDR, one sample per pixel, rain reflections and soft
local shadows on native/WebGPU backends. Dry scenes, WebGL2, MSAA, orthographic
views, mixed terrain/actor meshes and views without visible ordinary terrain
use the forward path. The last check reuses existing geometry bounds; forcing
the extra pass on the waterfall route was a measurable regression.

Three attachments cost 32 bytes per pixel, about 253 MiB at 3840 × 2160,
allocated lazily and reused. Linear depth retains 32-bit precision; normals
use 16-bit octahedral coordinates and color intermediates use half floats.
Invariant vertex positions keep the preparation and forward passes consistent
at depth boundaries. The profiler labels the first scene category
“Visibility + material preparation”; its data column remains unchanged.

Fixed 3826 × 2146 views, with the diagnostic's settings and HD artwork,
use 240 frames each and exclude 30 warm-up frames. Paired GPU means are:

| View | Night | Day | Wet blood stress |
|---|---|---|---|
| Reported Mission 1 corner | 20.37 → 18.86 ms | 20.93 → 18.87 ms | 23.88 → 22.41 ms |
| Mission 1 garden | 19.45 → 17.46 ms | 20.05 → 17.50 ms | 23.85 → 21.79 ms |

The comparisons hold geometry, lights, camera and animation time constant.
Images are not bit-identical: average absolute channel differences are
0.0042–0.0201 on the 0–255 scale. Largest differences are 2/5 in the
reported night/day views and 12 in the garden; only one/four garden channels
exceed 8, at shadow boundaries. No shadow samples or resolution were removed.

The final actual Wayland comparison uses 3840 × 2160, the same report settings,
HD artwork and 1,200 frames per route, excluding 60 warm-up frames. No compiler,
browser or other benchmark runs concurrently. Against `26d291f`:

| Route | Average FPS before → after | Frame p95 | Frame p99 |
|---|---|---|---|
| Mission 1 village | 41.0 → 42.0 | 27.44 → 26.03 ms | 27.88 → 26.39 ms |
| Mission 5 courtyard | 46.9 → 50.7 | 22.63 → 20.60 ms | 22.96 → 20.93 ms |
| Mission 1 garden | 37.8 → 38.9 | 28.29 → 26.97 ms | 29.05 → 27.56 ms |
| Mission 4 waterfall | 71.9 → 72.6 | 14.78 → 14.79 ms | 15.20 → 15.06 ms |

Point-shadow map work is effectively unchanged; the gains come from shading.
Village CPU mean rises from 3.38 to 3.67 ms with the additional pass, while its
GPU mean falls from 21.93 to 21.33 ms. Garden GPU p99 falls from 26.04 to
24.44 ms. The waterfall uses forward rendering and remains effectively neutral.
These are modest improvements; the difficult Mission 1 routes remain below
60 FPS. They reconstruct the diagnostic's graphics workload, not its complete
actor and weather history.

Validation includes the new GPU comparison for cutouts, coplanar surfaces,
indexed meshes, water/unlit materials, moving actors, blended effects and
far-plane sky ordering. Existing visibility, rain/snow shelter, scenery, blood
and daylight GPU regressions pass, as do 134 ordinary application tests.
Formatting, strict workspace/all-target and WASM Clippy, native release and
actual WASM cdylib builds pass. Chromium WebGPU records 757 wet preparation
passes through saved-state loading, movement, lighting toggles, free camera and
resume; WebGL2 completes the same flow with forward rendering. Neither reports
GPU errors, and paused gameplay remains unchanged by graphics controls.

## November 5: conservative depth summaries for soft local shadows

Reflective rain scenes now maintain small min/max depth summaries of local
shadow maps. Before the 32-sample filter, the shader checks whether the complete
filter footprint is certainly lit or shadowed. Ambiguous areas retain the full
filter. The bounds include bilinear neighbors, receiver-plane depth gradients
and a floating-point margin; no shadow samples or resolution are reduced.

The cache starts at one texel per 32 × 32 shadow pixels and costs about 2 MiB
for eight 2048-pixel cubemaps. Only changed shadow regions are reduced, with
one compute pass and seven dispatches at this resolution. CPU region buffers
are reused. It uses portable WebGPU compute without subgroup requirements;
WebGL2 and configurations outside the wet-lighting path keep their old shading.
An optional native subgroup prototype gave no useful improvement and was removed.

The final production comparison uses actual Wayland presentation at 3840 × 2160,
the diagnostic settings and HD artwork, 1,200 frames per route and 60 warm-up
frames. Compared with `a02dfc4`, with no competing compiler/browser workloads:

| Route | Average FPS before → after | Frame p95 | Frame p99 |
|---|---|---|---|
| Mission 1 village | 42.0 → 44.4 | 26.07 → 23.88 ms | 26.43 → 24.26 ms |
| Mission 5 courtyard | 50.6 → 56.0 | 20.68 → 18.88 ms | 21.21 → 19.66 ms |
| Mission 1 garden | 39.0 → 39.5 | 26.89 → 26.38 ms | 27.22 → 26.85 ms |
| Mission 4 waterfall | 73.2 → 72.6 | 14.40 → 14.62 ms | 14.75 → 14.87 ms |

Building summaries adds approximately 0.92/0.51/0.98/0.44 ms to the shadow-map
category. Reduced filtering offsets that cost in the demanding routes: total
GPU means change from 21.32/17.66/23.15 to 20.10/15.91/22.73 ms. The waterfall
is effectively neutral, with GPU p99 10.13 → 10.08 ms. The garden gain is small;
these changes do not yet deliver 60 FPS in the difficult Mission 1 scenes.

The cache regression compares partial redraws with full redraws and full
filtering through moving/disappearing casters, indexed/expanded geometry,
wind, moving lights and changed resolutions. Those images match exactly.
Twelve paired 4K prototype captures across tile-size variants, including
animated actors and foliage, also match exactly. Wet-terrain, blood composition,
cutout/transmission and daylight regressions pass, as do 134 ordinary tests,
formatting, strict native/WASM Clippy and both release builds. Actual Chromium
WebGPU records 762 reduction passes through saved-state loading, movement,
lighting toggles, free camera and resume; WebGL2 records none and passes the
same flow without GPU errors. Graphics controls leave paused gameplay unchanged.

## November 5: compact opacity data for shadow cutouts

Shadow rendering now reads one byte of opacity-class data for replacement
cutouts, instead of their full-color opaque/semi-transparent images. Solid
materials still bypass sampling; emitters retain RGB tests for lantern
transmission. Color rendering and cutout coordinates are unchanged.

A single-channel atlas shares the color atlas's coordinates and growth policy.
Only base-level coverage is uploaded, only when a tile changes; atlas growth
copies existing GPU data. It allocates one byte per atlas pixel per page,
compared with eight or twelve bytes for the color/detail layers. The browser
Mission 1 check grows this allocation to seven 2048-pixel pages, or 28 MiB.
No new user setting or asset-pack format is required.

Paired 3826 × 2146 prototype captures match exactly for static and animated
report/garden views. In the animated report view, shadow-map GPU time falls
from 2.38 to 1.54 ms; the garden changes from 7.01 to 5.64 ms. Static timings
are effectively unchanged because those maps are cached.

The integrated implementation uses actual Wayland presentation at 3840 × 2160,
the report's graphics and HD artwork, 1,200 frames per route and 60 warm-up
frames. Sequential before/after runs against `df66aad` have no concurrent
compiler, browser or other GPU workload:

| Route | Average FPS before → after | Frame p95 | Frame p99 |
|---|---|---|---|
| Mission 1 village | 44.5 → 49.5 | 23.87 → 21.44 ms | 24.28 → 22.35 ms |
| Mission 5 courtyard | 56.0 → 56.6 | 18.91 → 18.72 ms | 19.18 → 19.60 ms |
| Mission 1 garden | 38.9 → 42.3 | 26.73 → 24.68 ms | 26.99 → 24.97 ms |
| Mission 4 waterfall | 72.6 → 73.6 | 14.57 → 14.18 ms | 14.75 → 14.69 ms |

Village/garden shadow-map means fall from 8.17/7.47 to 5.85/5.85 ms; GPU
p99 falls from 21.02/24.07 to 18.64/22.14 ms. The courtyard has little shadow
work to save and its whole-frame p99 is mixed despite the small GPU reduction.
CPU means remain close. The difficult Mission 1 routes still miss 60 FPS.

The existing incremental-atlas test now reads opacity data back from the GPU
through atlas/metadata growth and animation updates. Point-light tests cover
transparency-class boundaries at alpha 1/191/192, ordinary holes and lantern
transmission. Replacement reloads, scrolling borders, moving shadows, scenery,
foliage wind, snow shelter and original blend rules pass, along with 134
ordinary tests. Formatting, strict native/WASM Clippy and release builds pass.
Actual Chromium WebGPU and WebGL2 both load/grow the new atlas, restore the
portable weather/blood state, move, toggle lighting and use free camera without
GPU errors or changes to paused gameplay.

## November 5: include water in prepared wet lighting

Opaque water now uses the existing material attachments and lighting pass when
wet-terrain lighting is active. Its material preparation retains waves, foam,
sky reflections and the original sun-only highlights. Unlit geometry retains
ordered forward rendering. The existing eligibility checks and attachment sizes
are unchanged, including the forward path for MSAA and views containing only
water. The implementation shares the water formulas with forward rendering.

A separate timing probe identified water's forward fallback as much of the
Mission 1 garden's material-pass cost. The prototype reduced total GPU time by
about 0.4–0.5 ms in fixed night/day/animated garden views. Paired 3826 × 2146
captures matched exactly away from water; garden differences reached at most
one 8-bit color level, with mean absolute differences below 0.006/255.

An initial shared helper calculated foam before shadow filtering and regressed
Modern High's forward rendering. Computing foam and reflection strength after
the filter restored that path's performance. This is why the shared helpers
retain the original calculation order rather than eagerly preparing all water
parameters together.

The final actual Wayland comparison uses 3840 × 2160, the diagnostic graphics
and HD artwork, 1,200 frames per route and 60 warm-up frames. Compared with
`4625057`, without concurrent compiler/browser/GPU workloads:

| Route | Average FPS before → after | Frame p95 | Frame p99 |
|---|---|---|---|
| Mission 1 village | 49.8 → 49.4 | 21.34 → 21.49 ms | 21.70 → 21.91 ms |
| Mission 5 courtyard | 56.8 → 56.0 | 18.59 → 18.82 ms | 19.14 → 19.17 ms |
| Mission 1 garden | 41.9 → 43.0 | 24.99 → 24.34 ms | 25.42 → 24.80 ms |
| Mission 4 waterfall | 73.4 → 73.0 | 14.25 → 14.62 ms | 14.69 → 14.84 ms |

Garden GPU mean/p99 falls from 21.32/22.36 to 20.72/21.80 ms. Other routes
shift upward by about 0.1–0.2 ms of GPU time, so this is a targeted improvement
to the slow garden route. Two earlier runs, including reversed execution order,
also improved its frame p99 by about 0.5–0.7 ms. The difficult routes remain
below 60 FPS.

A separate 600-frame Modern High comparison preserves the diagnostic's rain
mode and uses two-sample MSAA, exercising forward rendering. Garden GPU
mean/p99 changes from 19.38/22.77 to 19.50/22.80 ms; whole-frame p99 changes
from 24.78 to 24.90 ms. Other routes have small, mixed tail changes. This check
is not a dry-weather benchmark.

The expanded wet-lighting regression covers water enabled/disabled, waterfalls,
daylight, cutouts, coplanar surfaces and moving actors. Five other focused GPU
checks and 134 ordinary tests pass, along with formatting, strict native/WASM
Clippy, native release and actual WASM cdylib builds.
Chromium WebGPU and WebGL2 both pass saved-state loading, movement, lighting
controls, free camera and resume without GPU errors. WebGPU records 761 wet
preparation and 766 shadow-bound passes; WebGL2 uses its forward fallback.
Paused gameplay stays unchanged while graphics controls are exercised.

## November 5: bound smaller groups of soft-shadow samples

The existing depth summaries can now prove that a group of eight shadow samples
is fully lit or blocked, even when the complete 32-sample filter crosses a shadow
edge. Ambiguous groups retain their original samples and ordering. Group queries
are worthwhile only for broad filters: their radius must reach one 32-pixel bounds
tile. Narrower filters retain full sampling without these extra queries.

Four group rectangles fit in unused components of the existing sample uniforms.
The query reuses the same min/max pyramid, bilinear-neighbor coverage and receiver
plane bounds. A small UV margin encloses CPU/GPU product-rounding differences.
There are no additional render passes, texture allocations or reduced samples
at ambiguous boundaries.

Shader variants now use the actual supported sample count and the same eligibility
predicate as the shadow-summary cache. This omits unused grouped-filter code from
MSAA and other ineligible configurations. A runtime fallback initially kept both
filter loops in the shader and lost the improvement; selecting the variant when
configuring the renderer avoids that overhead.

Four paired 3826 × 2146 prototype captures match exactly, with static and animated
actors/foliage in the reported corner and garden. Fixed GPU means change from
13.38/14.87 to 12.17/13.68 ms in the corner and from 15.97/21.60 to 15.46/21.08 ms
in the garden. Applying group queries to narrow filters regressed the moving
garden test; the final implementation includes the radius check.

The final actual Wayland comparison against `33805da` uses the diagnostic graphics,
HD artwork, 3840 × 2160, 1,200 frames per route and 60 warm-up frames. Compiler,
browser and other benchmark workloads are kept separate:

| Route | Average FPS before → after | Frame p95 | Frame p99 |
|---|---|---|---|
| Mission 1 village | 49.6 → 51.2 | 21.31 → 20.83 ms | 21.84 → 21.18 ms |
| Mission 5 courtyard | 56.2 → 58.7 | 18.83 → 17.96 ms | 19.32 → 18.46 ms |
| Mission 1 garden | 42.5 → 43.6 | 24.57 → 23.95 ms | 25.64 → 24.38 ms |
| Mission 4 waterfall | 73.5 → 74.5 | 14.24 → 14.16 ms | 14.68 → 14.58 ms |

GPU means fall from 17.77/15.78/20.98/9.63 to 17.14/15.08/20.39/9.36 ms;
shadow-map construction stays effectively unchanged. Garden GPU p99 improves
from 21.94 to 21.32 ms. Whole-frame tail gains also include CPU/presentation
variation and should not all be attributed to shading. These demanding Mission 1
routes remain below 60 FPS.

A separate 600-frame Modern High comparison retains rain and uses two-sample
MSAA, exercising the ordinary filter. Whole-frame p99 changes from
21.90/16.89/24.61/13.80 to 21.71/16.08/24.44/13.14 ms. Garden GPU mean is roughly
unchanged, 19.42 → 19.51 ms; other routes improve. This checks the broader preset
rather than assuming a gain in one shader applies to every configuration.

The moving-region regression matches full redraws and full filtering through
caster movement/removal, indexed geometry, wind and changing light settings.
Wet surfaces, blood composition, point-light cutouts, daylight balance, complete
stage shadow culling and rain/snow shelter regressions pass, along with 134
ordinary tests. Formatting, strict native/WASM Clippy, native release and actual
WASM cdylib builds pass. Chromium WebGPU and WebGL2 both complete saved-state
loading, movement, lighting controls, free camera and resume without GPU errors.
WebGPU records 769 wet-material and 774 shadow-bound passes; WebGL2 retains its
forward path. Paused gameplay is unchanged by graphics controls.

## November 5: cache model geometry and compute GPU poses

The renderer now keeps eligible models' local positions and normals on the GPU,
transfers their part matrices and secondary-motion parameters, and computes the
posed vertex stream once before the shadow and scene passes. It keeps the same
vertex layout, indexed triangle order, materials and CPU culling bounds. Weighted
custom skins, botanical variants and parts exceeding device limits retain their
CPU upload path. WebGL2 continues to use CPU posing. Cache generations track model
replacements and material binding; mixed CPU/GPU frames share the same output.

The initial investigation measured approximately 2.5 MB of geometry uploads per
frame in the Mission 1 village and 5.3 MB in the waterfall route. An isolated
actual-character test reduced the model pose transfer from 2.07 MB to about
108 KB of matrices, deformation parameters and jobs. On this GTX 1080 Ti/PCIe
3.0 ×2 connection, explicit GPU timestamps measured 1.37 ms for the old pose
copy versus 0.11 ms for the smaller transfers plus computation. This transfer
result is hardware-dependent and is not a full-frame benchmark.

CPU transforms are deliberately retained in this increment to preserve existing
exact culling bounds. Removing that work needs a separate comparison of both CPU
savings and the shadow redraw cost of more conservative bounds. The new compute
pass is included in the first GPU timing category, alongside shadow maps; the
old large queue uploads occurred before the first timestamp. Consequently,
whole-frame time can improve even where reported GPU render time rises slightly.

The final actual Wayland comparison against `4ea7cfb` uses the diagnostic graphics,
HD artwork, 3840 × 2160, 1,200 frames per route and 60 warm-up frames. It runs the
new build first, with compiler/browser workloads kept separate:

| Route | Average FPS before → after | Frame p95 | Frame p99 |
|---|---|---|---|
| Mission 1 village | 51.2 → 55.0 | 20.81 → 19.38 ms | 21.10 → 19.72 ms |
| Mission 5 courtyard | 58.7 → 63.1 | 17.87 → 16.73 ms | 18.66 → 17.09 ms |
| Mission 1 garden | 43.7 → 46.3 | 23.97 → 22.67 ms | 24.68 → 23.11 ms |
| Mission 4 waterfall | 75.3 → 85.6 | 14.00 → 12.47 ms | 14.76 → 12.78 ms |

An earlier 600-frame comparison toggled the implementation inside one binary,
with the old path first, and improved all four routes. The longer run confirms
its direction with reversed order. The demanding Mission 1 routes still do not
reach 60 FPS.

A separate 600-frame Modern High comparison retains Rainstorm and two-sample
MSAA. Whole-frame p99 changes from 21.85/17.06/25.33/13.14 ms to
21.02/15.34/24.15/12.20 ms. GPU times vary more in the courtyard and garden than
the small compute pass alone explains; these are measured whole-frame results,
not a claim that all shader work became faster.

The actual-model regression compares 2,371,194 vertices across both heroes,
three detail levels, movement, secondary motion, generation changes, CPU-only
frames and mixed fallback frames. Maximum position and normal differences from
the original CPU transforms are 0.00390625 game units and 0.00000532. Paired
3826 × 2146 captures cover the rainy corridor, static/animated garden and
post-animation daylight close-ups. Two pairs are identical; the others differ
by at most one 8-bit color level, with mean differences below 0.000003/255.

Six focused GPU regressions and all 134 ordinary tests pass. Formatting, strict
native workspace/all-target Clippy, strict WASM workspace/library Clippy, native
release and actual WASM cdylib builds pass. Chromium WebGPU and WebGL2 complete
saved-state loading, movement, lighting controls, free camera and resume without
GPU errors or changes to paused gameplay. WebGPU records 782 character-pose
compute passes; WebGL2 records none and retains the CPU path.

## November 5: avoid duplicate CPU poses for GPU models

Eligible models now derive conservative shadow/scenery bounds from cached local
part boxes, transformed by their pose matrices and expanded for secondary motion
and floating-point roundoff. The CPU no longer transforms every vertex solely to
measure those bounds. Mixed frames still scan the world-space vertices of CPU
fallback parts; WebGL2, weighted skins and botanical variants keep their original
transforms. The GPU output, materials and triangle order are unchanged.

Two actual Wayland 3840 × 2160 comparisons use the diagnostic graphics, HD pack,
four moving routes, 600 frames and 60 warm-up frames. A same-binary toggle first
measured CPU savings of 0.19–0.38 ms. The final build, compared against `d8867e5`
with the new build first, gives:

| Route | CPU mean before → after | GPU mean before → after | Frame p99 before → after |
|---|---|---|---|
| Mission 1 village | 3.29 → 3.02 ms | 17.31 → 17.24 ms | 19.88 → 19.57 ms |
| Mission 5 courtyard | 2.68 → 2.51 ms | 15.11 → 15.11 ms | 17.56 → 17.56 ms |
| Mission 1 garden | 3.30 → 3.04 ms | 20.56 → 20.59 ms | 22.98 → 23.22 ms |
| Mission 4 waterfall | 5.81 → 5.64 ms | 9.28 → 9.29 ms | 13.00 → 12.67 ms |

This is a small CPU saving, not a solution to the GPU bottleneck. Average FPS is
effectively unchanged: 54.8 → 55.1, 63.0 → 62.9, 46.5 → 46.4 and 85.9 → 85.9.
Do not interpret the small, mixed frame-tail changes as a general FPS improvement.
The preceding GPU upload optimisation supplies the larger measured frame gains.

The actual-model GPU regression now builds a separate CPU reference and checks
both transformed vertices and conservative bound coverage, including mixed
fallback frames. All 2,371,194 vertices pass with the preceding numerical error
limits. Six paired 3826 × 2146 images are pixel-identical, including animated
characters and fixed-camera daytime garden/corridor views. The private fixture's
daytime camera is explicitly reset before each variant.

All 134 ordinary tests and six focused GPU regressions pass, along with formatting,
strict native all-target and WASM library Clippy, native release and actual WASM
builds. Real Chromium WebGPU and WebGL2 pass movement, saved-state loading,
lighting, free-camera and resume checks; WebGPU records 812 pose passes and
WebGL2 none. Experimental switches and private image hooks are removed.

## November 5: rendering design review after v0.15.6

The next rendering work should optimise perceived quality, rather than preserve
every existing shader result. The following are investigation results and design
recommendations. The released `v0.15.6` retains the preceding verified renderer;
the private probes described here were removed from the working source.

### Where the time goes

The final moving 4K benchmark above, on the GTX 1080 Ti with the diagnostic's
graphics and HD artwork, measures:

| GPU work | Village | Garden |
|---|---|---|
| Shadow/shelter maps, summaries and character posing | 5.97 ms | 6.02 ms |
| Visibility and surface-material preparation | 4.06 ms | 4.79 ms |
| Terrain lighting | 4.18 ms | 6.82 ms |
| Remaining scene work | 0.41 ms | 0.38 ms |
| Post-processing | 2.54 ms | 2.52 ms |
| UI | 0.08 ms | 0.06 ms |
| Total GPU | 17.24 ms | 20.59 ms |

Average displayed rates are 55.1 and 46.4 FPS. CPU work averages approximately
3 ms; CPU and GPU execution overlap, so these figures should not be added.
The garden's GPU alone already exceeds the 16.67 ms budget. A practical target
is nearer 14 ms GPU to leave room for submission, presentation and slower frames.
Hair and character color drawing are not the main cost in these views; their
shadow and posing work belongs to the first row.

The source explains the high cost despite relatively simple original geometry:

- [Local shadows](../crates/tenchu-pc/src/renderer/shadows.rs) allocate up to
  eight six-face depth maps. At 2048 pixels this is 768 MiB of point-shadow depth,
  plus a 64 MiB 4096-pixel sun/moon map. Allocation does not mean every texel is
  redrawn: caching, culling and changed-region redraws already exist. Animated
  foliage repeatedly invalidates large regions, however.
- [Shadow filtering](../crates/tenchu-pc/src/renderer/lighting.wgsl) performs five
  blocker-search samples and up to 32 comparison samples for a broad local soft
  shadow, per contributing light and receiver pixel. Coarse bounds avoid many
  samples, and a world-cell light list already limits relevant lights. Adding
  clustered lighting alone would not remove this filtering cost.
- [Wet materials](../crates/tenchu-pc/src/renderer/wet_lighting.rs) use three
  full-resolution attachments totalling 32 bytes per pixel, about 253 MiB at 4K.
  They preserve three different normals and separate old diffuse/emission/
  reflection conventions. This saved repeated lighting, but it is an expensive
  intermediate representation to make permanent.
- [Weather shading](../crates/tenchu-pc/src/renderer/weather.wgsl) reconstructs
  noise, shelter and puddle data per fragment. Near puddles, ripple normals visit
  nine neighbouring cells with hashing, sine and exponential calculations. Many
  screen pixels repeat calculations for the same small patch of ground.
- [Ambient occlusion](../crates/tenchu-pc/src/renderer/post.wgsl) reconstructs a
  normal from neighbouring depths and tests 16 further depths at full display
  resolution. Lowering its strength does not reduce the sampling count. These
  passes have no temporal accumulation.

### Controlled experiments

Separate fixed-camera 3840 × 2160 tests animate actors, vegetation and surface
time for 240 frames, excluding 60 warm-up frames. They use the same settings and
two Mission 1 views, with no competing compiler or GPU workload. GPU completion
is fenced after each frame: these are GPU comparisons, not interactive FPS or a
reconstruction of the diagnostic's complete state. Captures remain outside Git.

| Change in isolation | Corridor GPU mean | Garden GPU mean |
|---|---|---|
| Current renderer | 13.98 ms | 21.37 ms |
| 1024 point maps instead of 2048 | 13.34 ms | 17.33 ms |
| 2048 sun/moon map instead of 4096 | 13.64 ms | 21.11 ms |
| Ambient occlusion off | 12.43 ms | 19.82 ms |
| Bump mapping instead of parallax | 13.75 ms | 21.35 ms |
| All surface relief off | 12.99 ms | 20.51 ms |
| Vegetation wind off | 13.59 ms | 16.72 ms |
| Rain reflections off | 18.06 ms | 21.61 ms |
| Zero local-light source size | 13.83 ms | 19.61 ms |
| Weather off | 15.82 ms | 17.81 ms |
| Prototype: 16 soft-shadow samples | 13.80 ms | 19.81 ms |
| Prototype: 16 samples and 1024 point maps | 13.08 ms | 15.99 ms |

Disabled effects diagnose cost; removing them is not the proposed solution.
These differences are not additive, and some settings change rendering paths.
For example, reducing the map resolution cuts garden map work from 5.76 to
2.34 ms; disabling wind cuts it to 1.03 ms. Those savings overlap. Parallax is
already limited by footprint and viewing angle, and has little cost in the
garden; replacing all detailed surfaces would attack the wrong bottleneck.

The combined shadow prototype improves garden GPU p99 from 22.12 to 16.88 ms.
Viewed final captures are close: mean channel differences are 0.0023/255 in the
corridor and 0.0783/255 in the garden; worst differences are 4 and 37/255.
Small average image error does not prove acceptable shadow edges or temporal
stability. A moving camera, close contact shadows, sunlight, doors and foliage
need review before choosing a production quality policy. This is evidence for
adaptive shadow budgets, not a claim that this prototype guarantees 60 FPS.

The reflections-off result exposes an architectural boundary. The shared
material/lighting path and its shadow summaries currently require reflective
rain, HDR and single-sample rendering. Disabling reflections exits that path.
A private probe keeps the shared path while leaving reflections disabled:
corridor GPU time falls from 18.06 to 13.66 ms, garden from 21.61 to 21.08 ms.
Mean capture differences are below 0.007/255. Generalising this path should
precede adding further special cases. This improvement does not apply to the
user's current reflective-rain configuration, which already takes the fast path.

Dense retained blood remains a separate workload. With 4,096 projected stains
in the corridor, rain takes 16.89 ms GPU and snow takes 20.39 ms. With 1,600 valid
garden stains, rain takes 26.38 ms and snow 24.17 ms. The blood category costs
2.8–5.1 ms. Existing composition avoids repeated expensive shading on matching
flat receivers, but still rasterises coverage every frame and retains individual
shading on other receivers.

### Recommended implementation order

1. **Consolidate modern material and lighting evaluation.** Use one material
   description and shared lighting helpers across wet/dry surfaces, water and
   decals; remove incidental fast-path dependencies such as requiring rain
   reflections. Then measure a compact material buffer, targeting roughly half
   the current intermediate storage. A unified shading model can avoid keeping
   three normals solely to reproduce separate historical effects. Keep the
   Original presentation path and capability fallbacks explicit.
2. **Budget shadows by visible importance.** Keep detailed, frequently updated
   character/contact shadows. Give small or heavily softened shadows smaller
   maps and cheaper filtering, with stable allocation and gradual transitions.
   Keep lamps illuminated throughout. Use simpler foliage shadow casters or
   tolerate subpixel movement before invalidating their cached shadows, while
   retaining visible tree animation. Start here before building a complete
   virtual-shadow system. Deforming foliage invalidating cached shadows is a
   recognised problem in other engines too; Epic documents distance-dependent
   deformation and separate caching as remedies. Its full virtual-map system
   has different requirements and is not a drop-in recommendation.
   [Epic's shadow caching guidance](https://dev.epicgames.com/documentation/en-us/unreal-engine/virtual-shadow-maps-in-unreal-engine#caching).
3. **Separate ambient occlusion from the full-resolution finish pass.** Evaluate
   a depth-aware, spatially filtered AO buffer at reduced resolution, then
   composite it while keeping textures and UI at 4K. The measured AO portion is
   about 1.55 ms here. Preserve wall contacts and fine silhouettes; do not merely
   blur the finished image. GTAO/XeGTAO provide established references for depth
   preparation, sampling and edge-aware denoising; their published timings are
   for other hardware, not predictions for this machine.
   [XeGTAO implementation](https://github.com/GameTechDev/XeGTAO).
4. **Cache weather fields and make material detail scale with footprint.**
   Generate filtered ripple/noise fields once and sample them, instead of
   evaluating nine ripple cells independently at every nearby screen pixel.
   Cache slowly changing wetness/snow masks separately from lighting, invalidating
   affected regions for changing wind, shelter, accumulation and footprints.
   Preserve windward walls, roof shelter and snowfall edges. This needs visual
   comparisons; no speedup for the replacement algorithm has yet been measured.
5. **Make persistent stains part of the surface.** Investigate a bounded,
   incrementally updated surface-mask atlas for static terrain, applied before
   lighting. Keep fresh droplets, runoff and moving-character stains separate.
   Preserve mark metadata for fading, removal, styles and rewind, with the atlas
   as derived state. This replaces repeated coverage drawing with local updates;
   it is more substantial work than reducing a retention slider.

For particle-heavy waterfall/smoke views, a separate lower-resolution soft-particle
buffer is also worth testing. It should target soft spray/haze, while leaving
sharp sparks, droplets and debris at full resolution. Depth-aware composition
must avoid halos at geometry intersections; NVIDIA's treatment explains both
the savings and those failure modes.
[Off-screen particle rendering](https://developer.nvidia.com/gpugems/gpugems3/part-iv-image-effects/chapter-23-high-speed-screen-particles).

Temporal shadow filtering is a later candidate if spatial methods do not meet
the budget. It requires reliable reprojection, rejection of newly revealed
surfaces and reset rules for cuts, teleporting, rewind, resizing and lighting
changes. A few cheap samples with unmanaged history would trade frame time for
ghosting. Alternative filter families such as variance shadows also deserve
comparison rather than automatic adoption: their filtering savings can come
with light leaks. Filament maintains several shadow methods rather than treating
one as universally best. [Filament's rendering features](https://github.com/google/filament).

Judge each candidate with paired moving clips and actual displayed p95/p99 on the
village, garden, courtyard, waterfall and dense rain/snow/blood workloads. Keep
nearby contact detail, smooth transitions and stable thin geometry as quality
criteria. Target enough GPU headroom for 60 FPS rather than exactly 16.67 ms in
one fixed view. These techniques fit native and WebGPU rendering; WebGL2 can
retain simpler capability fallbacks. Dropping browser support would not eliminate
the native GPU work measured above. Gameplay, AI, collision and RNG remain a
separate behavioural specification throughout the rendering redesign.


## November 5: perceptual rendering trials

**November 6 update:** after reviewing the comparisons, all four retained trials
were adopted as automatic rendering behavior. The old menu switches and
`shadow-sampling` toggle demo are removed. Reference paths remain in GPU tests;
capability, material and receiver-boundary fallbacks remain in the runtime. The
measurements below record the comparison period, when the options were opt-in.
Modern presets now receive these savings automatically without changing their
artistic settings. Original still disables the corresponding modern effects.

The first retained change removes the reflection-strength requirement from
shared weather lighting and admits accumulated snow. It retains the original
single-pass dry path. At 4K, the same corridor with rain reflections disabled
improves from 18.01 to 13.59 ms GPU; snow improves from 17.39 to 13.36 ms. Garden
improvements are smaller, 21.40 to 20.99 ms and 20.77 to 20.49 ms respectively.
Reflective rain is unchanged, with pixel-identical paired captures. Other final
captures differ by mean channel values below 0.011/255 (maximum 19/255); the
expanded GPU regression covers cutouts, coplanar order, water, actors and sky.

A separate 24-byte material representation was tried against the existing
32-byte representation. It quantises octahedral normals and packs HDR emission,
preserving full-precision depth. Although the images are close, rainy views are
approximately 0.5–0.7 ms slower on the GTX 1080 Ti. Extra encoding/decoding costs
outweigh the saved bandwidth. This prototype was removed; captures, timings and
its patch remain in the private comparison evidence. Extending shared materials
to all dry scenes also regressed the garden, so that extension was not retained.
Evidence: `private/optimisation-loop/2026-11-05/perceptual-trials/`.


The ambient-occlusion trial originally appeared under F1 → Rendering trials. It shades
at half width/height, stores AO with linear distance, and reconstructs with
bilinear weights rejected across depth boundaries. Unsupported edge samples
fall back to the full-resolution calculation; there is no history to ghost on
cuts or rewind. Native 4K GPU time falls by 0.46–0.66 ms across the two views
and rain, reflections-off rain, dry and snow. Rain changes from 13.90 to 13.30 ms
in the corridor and 21.26 to 20.72 ms in the garden. Paired captures differ by
mean channel values 0.017–0.038/255; maximum differences are 8–17/255 at edges.
Full-resolution textures, HDR finishing and UI are unchanged. The extended GPU
regression exercises full/half/full toggles, actual contact occlusion, all
supported MSAA counts, UI isolation and restoring Original rendering. This
was opt-in; Modern presets were unchanged during these comparison trials.


The adaptive-shadow trial was also opt-in during comparison. Nearby lamps keep
configured map resolution; farther lamps use 1024/512 viewports with hysteresis.
Soft filtering uses 16 rather than 32 samples. Distant foliage updates at 30 or
15 Hz, staggered across lamp slots; actors and nearby foliage retain normal
updates. Physical array allocation remains unchanged, so this is a raster/filter
saving, not a shadow-memory reduction. The moving-region GPU regression matches
full redraws with wind, moving/disappearing casters, resized maps and indexed
geometry in both modes.

Across eight fixed-view animated 4K pairs, the garden improves by 4.5–5.0 ms:
reflective rain 21.44→16.90 ms mean and 22.19→17.98 ms p99; dry 17.43→12.40 ms
mean and 18.36→13.36 ms p99. The corridor's dry path improves 15.55→10.69 ms;
its already optimised rainy/snowy path changes by at most 0.18 ms. Mean RGB
channel differences are 0.002–0.115/255; the largest garden shadow-edge
pixel difference is 67/255. These are fixed-view GPU timings, not displayed FPS.
Camera-motion and transition comparisons remain part of the final trial report.


Two weather-field caches were tested and rejected on this machine. A 1024²
world-space ripple-gradient texture kept the original procedural seed/time,
with interpolation near the camera and procedural fallback outside its area.
Generating it every frame cost slightly more than it saved: the four rain pairs
were 0.04–0.14 ms slower, with almost identical captures. A second prototype
cached the four stable wetness/snow noise lattices in a 64³ RGBA16F volume,
regenerated only when the camera crossed a lattice region. It retained the same
world coordinates and interpolation, with procedural fallback outside the
volume. Mean GPU differences across rain/snow and both views were within 0.08 ms;
there was no repeatable benefit to justify the extra path and memory. Their
patches, binaries, screenshots and timings remain in the comparison evidence.
Neither cache changes wind/shelter or accumulation logic; neither is shipped.


The optional persistent-blood cache stores modern coverage per flat terrain
receiver in a bounded 32 MiB R16F array. Changed/removed/fading pages rebuild
only affected masks; original mark metadata remains authoritative for rewind.
Mask projection extends beyond receiver edges before sampling to avoid seams.
Current lighting, wetness and snow still shade each receiver. Slopes, ambiguous
overlapping floors, untracked receivers and other blood styles fall back to the
existing renderer. Sparse receivers also retain it. More than 256 eligible
receivers use the fallback rather than grow the atlas. Cached receivers are
batched, and uncached marks retain the existing overlap-composition heuristic.

That last detail mattered: an earlier prototype disabled overlap composition
for remaining marks, making the corridor slower despite improving the garden.
The retained version fixes it. Dense rainy corridor GPU time is 16.84→16.29 ms;
dense rainy garden is 26.42→24.01 ms. These synthetic workloads use 4096 and
about 1600 projected stains respectively. The GPU regression verifies actual
cache use, removing half/all marks and restoring them, rain/snow, cutout
materials, unknown receivers, slopes, foliage and overlapping floors. Cache
allocation does not change source state or the savestate format.

### Combined movement and desktop results

The final moving-camera probes advance precipitation and actor/foliage poses,
using identical camera paths and inputs in each pair. At 4K, all three accepted
options together change rainy corridor GPU mean/p99 from 14.08/15.32 to
13.32/14.44 ms, and rainy garden from 21.35/22.60 to 16.43/17.78 ms. Snow changes
the corridor from 13.55/14.61 to 12.79/13.75 ms and garden from 20.66/21.88 to
15.52/16.74 ms. These particular combined runs contain no synthetic blood;
separate dense-blood cases exercise that option.

Changing a blood page every 15 frames retains the cache's GPU benefit. Rainy
corridor mean/p99 is 17.06/17.98 → 16.51/17.39 ms; garden is 26.50/27.25 →
24.07/24.89 ms. Mask preparation adds CPU work on those updates: submission
p99 changes from 1.40 to 2.00 ms in the corridor and 1.37 to 2.26 ms in the
garden. Snowy update-heavy GPU means improve 15.74 → 15.31 and 24.36 → 22.52 ms.
This is a bounded tradeoff for dense stains, not an optimisation for every
sparse blood mark. The cache remains optional.

A separate full-application benchmark compares the frozen v0.15.6 executable
against the retained changes with all three trial options enabled, using the
reported graphics, HD pack and a 3840×2160 Wayland window. Each of four routes
has 60 warm-up frames and 1140 measured frames. No compilation, capture,
encoding or other test renderer runs concurrently. FPS below is the reciprocal
of the measured mean application frame interval, not the monitor refresh rate.

| Route | Mean FPS before → after | Frame p95, ms | Frame p99, ms |
| --- | ---: | ---: | ---: |
| Village lights | 54.7 → 78.6 | 19.50 → 13.75 | 19.81 → 13.99 |
| Snow courtyard | 62.9 → 101.3 | 16.81 → 10.65 | 17.42 → 10.95 |
| Garden and water | 46.0 → 59.8 | 22.80 → 17.62 | 23.28 → 17.96 |
| Waterfall and spray | 85.4 → 100.0 | 12.47 → 10.79 | 12.91 → 11.10 |

The garden is close to 60 FPS on average but still lacks consistent headroom.
The trials were kept opt-in under **F1 → Rendering trials** while their visual
compromises were reviewed. They were adopted automatically on November 6.
The private comparison page contains full-resolution before/after sliders,
individual timing files and paired 60 FPS motion clips. Clip export advances
at a fixed cadence and is not evidence of achieved rendering speed.

Native GPU checks cover moving shadow regions against full redraws, AO toggles
and resizing/MSAA/UI isolation, wet materials, and blood cache changes/fallbacks.
Native all-target and WASM library Clippy pass. WebGPU and WebGL2 play checks
load a current native blood fixture, toggle each option, resize and resume
without GPU errors. The WebGPU trace confirms mask generation; WebGL2 retains
the fallback. The comparison page verifies 46 image pairs and four paired motion
clips, with links to each original 4K clip. All image/video evidence stays outside
Git at `private/optimisation-loop/2026-11-05/perceptual-trials/comparison/`.

### Combat and accumulated blood follow-up (#68)

A matched Mission 1 sword fight uses Modern High at 3840×2160, 2× MSAA,
HD artwork, maximum cosmetic wound spurts/weapon flow and no fog. A stress
variant adds 16 projected stains per logic tick, accumulating up to 8192
retained stains (about 409,000 decal vertices). Each run has 1200 presented
frames, excluding the first 60 from distributions. The opponent receives
the same damage in each run. Native Vulkan uses the GTX 1080 Ti and Ryzen
9950X3D. These offscreen runs wait for submitted GPU work, so their frame
intervals are workload comparisons, not predictions of interactive FPS.

The blood cache now supports MSAA. Previously, one mark without receiver
metadata invalidated every cached receiver, causing intermittent full-cost
frames. Only receivers overlapping such a mark now fall back; ambiguous
overlapping receivers still all use the conservative path. `perf` also found
repeated hashing of complete receiver vertices and rescanning immutable stamp
opacity. Both are now computed when that immutable data is created. Full
bitwise receiver equality, saved bytes and original mark metadata are retained.

| Accumulation workload | Mean frame, ms | Frame p95, ms | Frame p99, ms | GPU p99, ms |
| --- | ---: | ---: | ---: | ---: |
| Cache off → updated cache on | 44.18 → 20.56 | 66.54 → 25.86 | 68.75 → 27.07 | 64.33 → 19.11 |
| Same, rewind recording on | 44.77 → 21.38 | 67.89 → 27.57 | 70.71 → 29.42 | 64.19 → 19.10 |

Ordinary combat without synthetic accumulation measured 16.65 ms mean frame
and 18.86 ms p99 with the cache off. Recording rewind raised those to
17.22/20.29 ms; rewind capture itself averaged 0.49 ms. Heavy accumulation
with the updated cache and rewind costs 0.67 ms mean / 2.09 ms p99 for rewind
capture. This does not establish the cause of every reported combat slowdown:
the original report still needs an equivalent current diagnostic/reproduction.

Paired final 4K captures differ by mean RGB values 0.018/0.018/0.007 out of
255, with sparse edge differences up to 127/255. This is still an optional
coverage approximation, not pixel-identical rendering. GPU regression checks
exercise every supported MSAA count, changing/removing/reusing masks, varying
opacity, rain/snow, cutouts, slopes, foliage, overlapping floors and both nearby
and distant unknown receivers. Private measurements and captures are in
`private/issue-68/`; the synthetic stress hooks are not shipped.

The native blood-heavy snapshot also loads in WebGPU and WebGL2; the F1 cache
toggle and resumed combat produce no runtime/GPU errors. WebGPU uses 4× MSAA
for this check; WebGL2 keeps its existing fallback. Saved-state continuation
and cached rewind serialization checks pass without format changes.

### Weather incidence follow-up

The shared terrain material pass now skips shelter-map sampling when a surface
faces away from precipitation, as the forward renderer already did. Eight paired
4K captures across rain/snow, two Mission 1 views and daytime/nighttime are
pixel-identical. No weather coverage, filter or sample count changes.

The GTX 1080 Ti repeat uses the reported graphics, HD artwork, all three rendering
trials and 25% flame motion/heat shimmer. Each 4K native route has 1200 frames,
with 60 warm-up frames; compilation and capture run separately. The repeated
comparison reverses execution order. Snow results from that repeat:

| Route | Mean GPU, ms before → after | Frame p95, ms | Frame p99, ms |
| --- | ---: | ---: | ---: |
| Village lights | 11.54 → 11.27 | 15.07 → 14.91 | 15.40 → 15.18 |
| Snow courtyard | 8.97 → 8.91 | 10.94 → 10.90 | 11.39 → 11.34 |
| Garden and water | 15.13 → 14.69 | 18.01 → 17.64 | 18.51 → 17.91 |
| Waterfall and spray | 6.81 → 6.81 | 9.94 → 9.94 | 10.56 → 10.31 |

The first order also improved garden snow GPU time by 0.56 ms. Rain gains did
not repeat consistently, and the waterfall uses the unchanged forward path;
its frame-time variation is not a gain from this change. The garden still
exceeds the 16.67 ms budget at p99. Private evidence is under
`private/optimisation-loop/2026-11-06/advanced/`.

### Quarter-resolution local-shadow trial

The optional terrain shadow-visibility pass samples soft local shadows at one
quarter of the display width/height, storing eight light visibilities in two
RGBA8 targets (about 4 MiB at 4K). Reconstruction accepts neighboring samples
only on a matching receiver plane and normal; otherwise it runs the existing
full-resolution filter. Material textures, normals, reflections, sunlight and
character shading keep their resolution. The shadow maps themselves retain
their detail. This applies to the shared wet/snow terrain pass on native/WebGPU
with HDR, soft shadows and no MSAA. Other configurations use the existing path.

An initial half-width/height RGBA16 implementation did not meaningfully improve
moving scenes and was discarded. The compact quarter-size variant reduces GPU
time by 0.7–1.3 ms in the fixed 4K report/garden rain/snow scenes, including
animated characters. Twelve paired uncompressed captures have mean RGB changes
of roughly 0.001–0.058 out of 255; sparse shadow-edge differences reach 54.
This is a visual approximation, not pixel-identical shading.

The full native application uses the same reported settings, HD artwork, three
earlier trials and 25% flame motion/shimmer as the incidence follow-up. Runs use
1200 frames per route, excluding 60 warm-up frames, with the new option on then
off in the same binary. No build, capture or encoder runs concurrently.

| Weather / route | Frame mean, ms before → after | Frame p95, ms | Frame p99, ms |
| --- | ---: | ---: | ---: |
| Rain / village | 12.82 → 12.81 | 13.85 → 13.80 | 14.16 → 14.20 |
| Rain / courtyard | 9.97 → 9.40 | 10.77 → 10.09 | 11.10 → 10.64 |
| Rain / garden | 16.49 → 15.58 | 17.28 → 16.27 | 17.63 → 16.90 |
| Rain / waterfall | 10.00 → 10.11 | 10.83 → 10.84 | 11.24 → 11.01 |
| Snow / village | 13.48 → 13.56 | 14.99 → 15.02 | 15.28 → 15.40 |
| Snow / courtyard | 10.07 → 9.65 | 10.93 → 10.62 | 11.48 → 10.91 |
| Snow / garden | 16.72 → 15.83 | 17.81 → 16.90 | 18.02 → 17.06 |
| Snow / waterfall | 9.36 → 9.31 | 10.07 → 10.02 | 10.76 → 10.69 |

The garden reaches 64.2 FPS mean in rain and 63.2 in snow, but p99 still exceeds
16.67 ms. The village has no useful gain; the waterfall uses the unchanged
forward path, so its small differences are noise. During review, built-in presets left
the trial off and `demo --clips shadow-sampling` alternated it in a rainy garden.
Paired 4K60 motion clips and raw evidence stay outside Git under
`private/optimisation-loop/2026-11-06/screen-shadows/`; export cadence is not a
measurement of achieved FPS.

The integrated setting matches all 24 prototype before/after captures exactly.
GPU regression checks cover thin receivers, slopes, moving actors, changing
light slots, odd window sizes and exact restoration when disabled. Native and
WASM strict Clippy pass. WebGPU/WebGL2 checks exercise the F1 toggle/reset,
resizing and resumed movement/combat; only WebGPU uses the extra pass. The
private comparison page verifies twelve image pairs and three paired motion
clips; the built-in demonstration also exports 720 consecutive 4K60 frames.

### Rain lighting with MSAA

Rainy terrain with MSAA previously evaluated diffuse light and wet reflections
separately, repeating local and sun shadow lookups. The forward renderer now
shares that work using the existing terrain material/light calculations. Existing
material batches select the shader without changing triangle order; water and
special materials retain their usual path. This adds no render targets or passes
and keeps texture, weather and shadow detail. It applies automatically to HDR
rain/rainstorm scenes with MSAA on native/WebGPU. Snow did not benefit, so it
retains the previous path, as do WebGL2 and non-HDR configurations.

Matched native 4K Modern High routes use HD artwork, 2× MSAA, six local shadow
lights and soft shadows on the GTX 1080 Ti / Ryzen 9950X3D. Each route contains
1200 frames, excluding 60 warm-up frames. Builds and image capture run separately
from timings. Final comparison against v0.16.0:

| Rain route | Mean frame, ms before → after | Frame p95, ms | Frame p99, ms |
| --- | ---: | ---: | ---: |
| Village lights | 15.21 → 13.08 | 16.71 → 14.14 | 17.00 → 14.46 |
| Snow courtyard | 11.50 → 10.15 | 12.28 → 11.18 | 12.88 → 11.78 |
| Garden and water | 19.55 → 16.34 | 20.96 → 17.64 | 21.24 → 17.91 |
| Waterfall and spray | 9.86 → 9.84 | 10.70 → 10.72 | 11.14 → 11.10 |

A preceding run measured 16.38 ms mean / 18.92 ms p99 in the garden. Average
performance improves from about 51 to 61 FPS, but p99 remains over the 16.67 ms
budget. The waterfall uses the unchanged path; its small differences are noise.
Eight representative rain/snow day/night captures differ by at most 1/255 in
sparse channels. GPU checks cover palette transparency, cutouts, coplanar order,
indexed geometry, moving actors and MSAA. Native, Android and WASM Clippy pass;
WebGPU/WebGL2 gameplay checks exercise lighting changes, camera movement and
pause/resume without GPU errors. Evidence stays outside Git under
`private/optimisation-loop/2026-11-06/resumed/`.

### Share identical blood receivers

Blood cache preparation now reuses the existing renderer identity table's `Arc`
for equal receivers. Subsequent grouping can use pointer equality instead of
repeatedly comparing full triangle attributes. The initial identity lookup still
compares all attributes; separate UVs, normals and surface heights stay distinct.
This adds no cache or saved-state fields and changes no blood geometry or shading.

The same 4K Modern High accumulation fixture adds sixteen stains per logic tick,
up to 8192 retained marks, with moving actors. Runs contain 1200 frames and
exclude 60 warm-up frames. The reverse-order repeat on the GTX 1080 Ti /
Ryzen 9950X3D measures:

| Workload | Frame mean, ms before → after | Frame p95, ms | Frame p99, ms | CPU submission p99, ms |
| --- | ---: | ---: | ---: | ---: |
| Accumulating blood | 18.11 → 17.80 | 22.72 → 21.39 | 23.97 → 22.51 | 4.18 → 2.67 |
| With rewind recording | 19.06 → 18.74 | 24.60 → 23.69 | 25.80 → 24.89 | 4.10 → 2.79 |

The first order also improves whole-frame p99 by 0.87/1.05 ms. Settled marks
with moving actors remain approximately 16 ms; their cache does not need this
repeated grouping. Paired final captures are byte-identical. These offscreen
runs wait for the GPU, so frame intervals compare workloads rather than predict
interactive FPS. Stress hooks and raw evidence remain outside Git under
`private/optimisation-loop/2026-11-06/resumed/canonical-*`.
The receiver/fallback GPU regression passes across all supported MSAA counts;
147 ordinary tests and strict native, WASM and Android Clippy also pass.

### Keep partial blood pages out of rewind frame deltas

Growing and trimmed blood pages now use the same immutable resource storage as
full pages. Previously their variable-length geometry and receiver metadata were
inserted into the main XOR delta, shifting later data whenever a mark arrived or
expired. Compressing those pages separately reduces both recording work and
history storage. Serialized game states, the recording format and the 96 MiB
memory limit remain unchanged.

A steady-play `perf` capture excludes loading and shutdown; LZ4 compression
accounts for about 10.6% of sampled CPU cycles in the dense-blood/rewind fixture.
Reusing the compression output buffer did not help and was discarded. Separating
partial pages gives these repeated 4K Modern High accumulation results:

| Metric | Before | After |
| --- | ---: | ---: |
| Mean rewind work per presented frame | 0.713 ms | 0.641 ms |
| Rewind p95 / p99 | 1.830 / 2.194 ms | 1.577 / 1.871 ms |
| Whole-frame mean / p99 | 18.646 / 24.853 ms | 18.560 / 24.384 ms |
| Retained history at the end of the stress run | 167 ticks (5.5 s span) | 300 ticks (10.0 s span) |

An earlier pair measures rewind means of 0.711 → 0.637 ms and p99 of
2.265 → 1.869 ms. Settled marks with moving actors show little difference
(0.444 → 0.433 ms mean rewind work); their whole-frame tails vary in both
directions. This is a CPU/history improvement, not a GPU-speed claim.

Both complete retained intervals export/import successfully. Seeks to their
first, middle and final states reproduce the original bytes; both builds end
with the same complete-state hash. Captures are byte-identical. The existing
blood expiry/style-change regression also checks exported recordings. The same
1200-frame offscreen methodology and hardware as above apply. Private evidence
is in `private/optimisation-loop/2026-11-06/resumed/rewind-{pages,control}-*`.
Strict native/WASM/Android Clippy, ordinary tests, full-versus-cached capture,
visual-effect restoration, exported timeline continuation and timed reverse
playback checks pass.


### Traverse presentation surfaces without temporary stacks

The presentation geometry index now stores nodes in traversal order with an
index past each subtree. Rain shelter/landing queries, blood projection and
footprints can skip a subtree without allocating a temporary stack. Triangle
visit order is preserved, including equal-distance hits; the index is 48 rather
than 64 bytes per node on this 64-bit build. Gameplay collision and AI do not use
this index, and saved states do not contain it.

Steady dense-blood profiling attributes about 3% of sampled CPU time to surface
casts, including their allocations. Repeated isolated CPU tests use actual HD
geometry from missions 1, 4, 6 and 9 on the Ryzen 9950X3D. Each cast/occlusion
batch contains 2048 rays; weather measurements run 1200 ticks with maximum
particle detail, moving centers and changing wind, excluding 120 warm-up ticks.
Two reverse-order repeats give these mean CPU times:

| Mission | Ray-cast batch, ms before → after | Rainstorm tick, ms | Blizzard tick, ms |
| --- | ---: | ---: | ---: |
| 1 | 1.779 → 1.533 | 0.0747 → 0.0596 | 0.0352 → 0.0312 |
| 4 | 1.037 → 0.834 | 0.1102 → 0.0904 | 0.0392 → 0.0342 |
| 6 | 2.505 → 2.205 | 0.1554 → 0.1328 | 0.0746 → 0.0640 |
| 9 | 2.196 → 1.923 | 0.0834 → 0.0646 | 0.0831 → 0.0700 |

Early-exit shelter checks improve by 22–30%. Batches of projected footprints,
blood and pools improve by approximately 2–7%; clipping still dominates those
operations. All 28 scenario output hashes match across builds, covering full
weather states, hit metadata and complete ordered projected vertices. These
are small CPU savings, not equivalent percentage gains in displayed FPS.
The existing exhaustive intersection and weather/projection tests exercise
empty geometry, parallel rays, excluded water, shelter and wind changes.
Evidence stays outside Git under
`private/optimisation-loop/2026-11-06/resumed/surface-traversal/`.
Native workspace/all-target, WASM and Android strict Clippy pass, as do 147
ordinary tests and disc-backed persistent-blood/animated-footprint checks.

Projection follow-up: stop clipping a triangle once a stamp plane has removed
it completely. Later planes cannot add vertices, so this skips unused work
without changing output or introducing storage. Two alternating repeat pairs
on the same fixture reduce blood projection by 5–8% and pool projection by
3–6%; footprint gains are smaller (1–4%). Mission 6's 512-stamp blood batch
falls from 2.976 to 2.738 ms and its pool batch from 18.055 to 16.972 ms. All
28 output hashes still match. Reusing two heap clipping buffers was also tested
but regressed most cases and was discarded. These remain CPU-only measurements.

### Reuse terrain shelter during storm flashes

Wet/snow terrain already evaluates precipitation shelter. Flash illumination now
reuses that result instead of sampling the same shelter map again. Surfaces that
did not need weather coverage still evaluate shelter for the flash. Filter taps,
coverage, flash timing and brightness are unchanged; no new render pass or cache
is needed.

Paired 4K Modern High GPU fixtures use HD Mission 1 geometry, animated characters,
2× MSAA and rain on the GTX 1080 Ti. The reverse-order repeat contains 1200 frames
per case, excluding 30 warm-up frames. Builds and other GPU tests run separately.
The flash is held at 30% to isolate its cost, rather than waiting for the storm
schedule. These are fenced offscreen GPU measurements, not interactive FPS.

| View / weather state | GPU mean, ms before → after | GPU p99, ms |
| --- | ---: | ---: |
| Garden / between flashes | 17.71 → 17.51 | 18.29 → 18.09 |
| Garden / flash | 19.64 → 18.74 | 20.26 → 19.30 |
| Village / between flashes | 14.15 → 14.10 | 14.81 → 14.72 |
| Village / flash | 15.32 → 14.76 | 16.02 → 15.38 |

The first order saves 0.75/0.59 ms during flashes; ordinary-frame differences
are small and vary in direction. Twelve paired captures are byte-identical,
including rain haze enabled/disabled. Evidence stays outside Git under
`private/optimisation-loop/2026-11-06/resumed/storm-sharing/`.

Single-sample rain/snow checks also improve flash frames by 0.35–1.22 ms;
ordinary frames differ by 0.05–0.14 ms and eight more captures match exactly.
The existing shared-versus-forward lighting regression now includes flashes
across special materials, sloped terrain, indexed geometry and MSAA counts.
Shelter/wind GPU checks, 147 ordinary tests, strict native/WASM/Android Clippy
and WebGPU/WebGL2 gameplay checks pass.

### Skip unused puddle basin noise

Puddle coverage is zero on steep surfaces and when its accumulated amount is
zero. Wet shading now skips the second noise evaluation in those cases, retaining
the original calculation and multiplication order where puddles can form. No
filter, sample count or weather setting changes.

On the same 4K GTX 1080 Ti fixtures, two orderings reduce rainy garden GPU time
by 0.13–0.26 ms and the village view by 0.08–0.13 ms. The 1200-frame reverse
repeat gives garden GPU mean/p99 of 17.61/18.22 → 17.35/17.92 ms; six paired
captures match exactly. A short moving-camera comparison is mixed, so it is
not evidence of a general FPS increase. The longer reverse native comparison
(1200 frames per route, 60 warm-up frames excluded) improves garden frame
mean/p95/p99 from 18.30/19.82/20.17 → 17.89/19.46/19.88 ms and village mean
from 13.73 → 13.53 ms. This is a small saving, still short of the garden's
16.67 ms target.

Rain/snow native routes, wet-material/shelter GPU regressions, 147 ordinary
tests, strict native/WASM/Android Clippy and WebGPU/WebGL2 gameplay checks pass.
Evidence is under `private/optimisation-loop/2026-11-06/resumed/weather-cost/`
and `resumed/puddle-*.json`, outside Git.

### Share rain haze through a view volume

Rain haze previously integrated six samples along the camera-to-surface ray
for every shaded fragment, including overlapping materials and MSAA samples.
Native/WebGPU now evaluate that same shelter integral in a 128×72×64 RGBA16F
view volume (4.5 MiB), with logarithmic depth spacing and trilinear lookup.
The volume is rebuilt each active frame, so camera motion, changing shelter,
wind and settings need no history rejection. Lightning still colors the haze
in the surface shader. WebGL2, orthographic views and points beyond the volume
retain the ray calculation; no precipitation or gameplay state changes.

The first paired 4K fixed views improve GPU mean from 17.36→16.04 ms in the
rainy garden and 14.05→13.13 ms in the village. Lightning views improve
18.48→17.10 and 14.79→13.90 ms. The full native comparison runs the candidate
first, then the preceding build, using Modern High, HD artwork, 2× MSAA and
1200 frames per route with 60 warm-up frames excluded:

| Rain route | Frame mean, ms before → after | Frame p95, ms | Frame p99, ms |
| --- | ---: | ---: | ---: |
| Village lights | 13.42 → 12.90 | 14.84 → 14.03 | 15.27 → 14.56 |
| Snow courtyard | 10.46 → 9.80 | 11.70 → 10.85 | 12.19 → 11.30 |
| Garden and water | 17.81 → 15.62 | 19.28 → 16.93 | 19.74 → 17.60 |
| Waterfall and spray | 10.27 → 9.58 | 11.23 → 10.65 | 11.83 → 11.25 |

The rainy garden averages approximately 64 FPS, but p99 still exceeds the
16.67 ms budget. Fixed-view checks at 720p/1080p also improve mean GPU time
by 0.10–0.30/0.29–0.59 ms. Short 720p runs contain tail outliers, so they do
not establish a p99 improvement. Measurements include volume preparation;
no build, capture or second GPU workload runs alongside native timing.

This is a presentation approximation. Ordinary paired captures differ by
mean RGB channel values 0.05–0.14/255, with maxima of 6–7. Strong 100% haze
with 75% wind, daytime and lightning differs by mean 0.30–1.30/255; at least
99% of pixels differ by no more than 8 in any RGB channel. Sparse differences
reach 59 at fine distant geometry. Texture detail and lighting are unchanged;
the interpolated haze softens some of the original integral's transitions.

A GPU regression compares moving views against direct integration while changing
roof coverage, wind direction and weather quality, including an exactly clear
enclosed room. Existing rain/snow shelter, windward walls and shared-lighting
regressions pass, along with 147 ordinary tests and native/WASM/Android strict
Clippy. Fresh WebGPU/WebGL2 builds pass rainstorm play, camera movement,
lighting changes and pause/resume without GPU errors. Instrumentation confirms
the volume runs only on WebGPU; WebGL2 retains direct integration.
Evidence is outside Git under
`private/optimisation-loop/2026-11-06/resumed/rain-volume/` and
`resumed/haze-{before,after}-high-rain.json`.

## HD shop menu preparation (2026-11-07)

The shop uses the selected sky, weather and lamp settings in a painted-room
shader; it does not select a higher gameplay quality preset. Profiling found
repeated SHA-256 hashing of replacement images for each background section and
item sprite on every animated frame. Replacement identities now reuse the
original lookup key, with GPU images invalidated when the loaded pack changes.
Original pixels, replacement pixels and all shading remain unchanged.

Release-build CPU frame preparation on the Ryzen 9 9950X3D, 120 samples after
10 warm-up frames, installed artwork-v3, English Mission 1 shop at 1920×1080:

| Configuration | Before p50 / p99 | After p50 / p99 |
| --- | ---: | ---: |
| Original artwork | 1.10 / 1.15 ms | 1.15 / 1.19 ms |
| HD artwork | 6.60 / 6.95 ms | 1.34 / 1.37 ms |
| HD artwork + room lighting | 6.58 / 6.63 ms | 1.35 / 1.37 ms |

These measure CPU menu preparation, not GPU time or Android FPS. Reproduce with
`TENCHU_DISC=… ./dev.sh cargo test --release -p tenchu-pc shop_frame_preparation_profile -- --ignored --nocapture`.
GPU regressions cover pack reloads, retro toggles, 4K replacement detail, and shop
day/night/weather changes without changing gameplay state.
