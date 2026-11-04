# Performance measurements

F1 → General → **Benchmarks…** runs four fixed scenes using the
current graphics: village lanterns/characters, Mission 5's courtyard, a garden
with vegetation/water, and a waterfall. Each scene advances the same 60 Hz
presentation timeline and original 30 Hz simulation. Loading and the first 60
frames are excluded. Escape cancels. The current game, preferences and input
recording are restored afterwards; temporary presentation particles are rebuilt.
Changing the window size or losing focus cancels a windowed measurement.

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
