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
