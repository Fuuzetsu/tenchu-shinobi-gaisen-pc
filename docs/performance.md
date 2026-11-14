# Performance measurements

F1 → General → **Benchmarks…** runs five fixed scenes using the
current graphics: village lanterns/characters, Mission 5's courtyard, a garden
with vegetation/water, Mission 4's checkpoint guards, and a waterfall. Each scene advances the same 60 Hz
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

## Level Studio authoring (2026-11-14)

Texture preparation now resolves repeated face/image queries once per pass and
checks each model's texture set when selecting light rules. Artwork caches also
retain manifest selectors, so renumbering a prop with no override does not reopen
the archive. Geometry, texture identities, light-rule order and graphics settings
are unchanged. These paths are shared with scene loading in both games.

Matched release builds against v0.35.1 (`4f7f80d`) on GTX 1080 Ti/Vulkan, using
the reported saved graphics and installed T1/T2 artwork:

| Authoring operation | v0.35.1 | Optimized |
| --- | ---: | ---: |
| Lighting/events example: trap/pit placement, median of three edits | 63.6 ms | 28.2 ms |
| T1 campaign: terrain elevation edit | 153.4 ms | 88.3 ms |
| T2 campaign: terrain elevation edit | 112.0 ms | 94.8 ms |
| Mixed T1/T2 garden: ornament placement | 139.9 ms | 21.6 ms |

Campaign/mixed entries are individual edits, not percentile estimates. These
measure synchronous authoring work through scene refresh, not interactive FPS.
Idle refresh remains 0.26 ms. At 1440×1000 with rain accumulated for two minutes,
lighting, shadows and the full editor UI, 120 preview frames after 30 warm-up
frames measure CPU p50/p95 2.08/2.60 ms and GPU p50/p95 2.22/2.54 ms. CPU times
exclude presentation/completion waits; GPU timestamps are separate. The baseline
preview was similar (CPU 1.99/3.01 ms, GPU 2.22/2.50 ms): the gain is in editing.

Eight final before/after captures are pixel-identical; the T1 campaign differs at
seven of 1,440,000 pixels by one colour-channel unit. Repeating the baseline also
changes three pixels by one unit; an earlier nine-pair comparison was identical.
Edited geometry and serialized game state match fresh reloads, including imported
scenery and preserved weather.
Raw timings, settings and screenshots remain outside Git under
`private/studio-performance-20261114`. These are controlled authoring/presentation
fixtures, not ordinary-play retail comparisons.

Further preparation changes restrict mist setup to waterfall meshes, sort
references to shadow triangles before copying them once, transfer owned stage
geometry directly to upload, and skip curvature adjacency for lighting-only
smoothing. Imported terrain shares texture analysis with artwork selection;
original texture pixels are decoded only when new artwork needs loading.
Material binding retains shared source images so unchanged imports avoid copies.
A fresh v0.35.2 baseline on the same hardware/settings measures:

| Authoring operation | v0.35.2 | Further preparation changes |
| --- | ---: | ---: |
| Lighting/events trap/pit edit, median of three | 28.5 ms | 27.0 ms |
| T1 campaign terrain elevation edit | 86.7 ms | 70.7 ms |
| T2 campaign terrain elevation edit | 102.4 ms | 67.1 ms |
| Mixed garden ornament placement | 20.3 ms | 20.8 ms |

Campaign/mixed entries remain individual edits. Preview CPU/GPU p95 is
2.48/2.53 ms versus 2.73/2.53 ms before; the improvement is scene preparation.
Eight captures are identical; T1 differs at 26 pixels by one channel unit.
Filtered waterfall spray matches the full-scene calculation byte for byte on
the village and waterfall missions at every tested water-detail setting.
GPU atlas checks cover shared-image edits, artwork toggles, animated textures,
VRAM changes and refined blood. New model recipes still receive original disc
textures even when those textures were already selected by an earlier load.
Evidence remains outside Git in `private/studio-performance-continuous-20261114`.

Static scenery now uploads shared vertices and triangle indices. On the sampled
T1/T2 campaign edit states, visible terrain buffer data falls from approximately
20.5/13.6 MB to 11.9/6.9 MB, including indices. These figures exclude textures,
shadow geometry and other GPU resources. Unshared geometry retains its previous
upload path. Expanded triangles, attributes and source game state are unchanged.

A fresh v0.35.3 (`a2a9f727`) comparison on the same settings/hardware measured T1
terrain editing at 67.6 → 61.9 ms, T2 at 64.5 → 64.6 ms, mixed ornament placement
at 20.4 → 19.1 ms, and the three example edits at median 27.0 → 26.1 ms. Campaign
entries are single edits. Preview GPU p95 remains about 2.54 ms; CPU p95 is
2.53 → 2.40 ms, with variable medians. This is an upload/memory improvement, not
a claimed interactive frame-rate increase. Eight captures are identical; the
T1 view differs at 25 pixels by one channel unit.

Source and enhanced geometry comparisons cover Original, High and Ultra plus
mixed T1/T2 Studio maps. GPU comparisons cover original texture bindings,
weather, water, wind, cutouts, emission, HDR and MSAA, and both games' animated
character poses. Evidence: `private/studio-indexed-stage-20261114` outside Git.

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

Every candidate runs every scene, with 60 warm-up and 240 measured frames
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

## Shinobi Gaisen Mission 4 report (2026-11-13)

The catalog now includes a camera sweep around the first checkpoint patrol in
**Cross the Checkpoint** (internal stage 9). On a GTX 1080 Ti / Vulkan at
1280×720, an optimized build measured 420 CPU frames and 419 GPU frames after
60 warm-up frames, with no other benchmark or compilation running:

| Settings | CPU work p95 | Geometry/uploads p95 | GPU p95 |
| --- | ---: | ---: | ---: |
| Original, disc artwork | 0.31 ms | 0.15 ms | 0.08 ms |
| Modern High, disc artwork | 1.02 ms | 0.31 ms | 1.37 ms |
| Modern High, HD v3, all enemies active | 1.60 ms | 0.81 ms | 1.81 ms |

The last run loaded 311 replacement textures and two replacement models for the
checkpoint. The reported unstable frame rate was not reproduced on this path.
This offscreen measurement excludes loading, window/compositor presentation and
other routes through the mission; it cannot rule out a problem with a different
location, settings or device. No gameplay or rendering behavior was changed on
the strength of this report. Full JSON reports and settings are in
`private/t2-sound-hitbox-review-20261113/performance` outside Git.

## T1 stages under T2 rules (2026-11-12)

The T2 renderer rebuilt stationary terrain, recomputed foliage variants and
uploaded its vertex poses every frame, even with fog disabled. At the start of
Shinobi Gaisen Mission 1 this processed 319,887 terrain vertices with the reported
settings (fog off, Botanical foliage). The renderer now retains that scenery on
the GPU. Transparent faces still sort for the current camera, and doors and
actors remain dynamic. Fogged terrain keeps its original per-vertex shading path.

An isolated starting-area fixture measured the release build at 1280×720 on a
GTX 1080 Ti / Vulkan, with 20 warm-up frames and 80 measured frames:

| Graphics | Geometry + uploads before, mean | After, mean |
| --- | ---: | ---: |
| Original | 0.52 ms | 0.46 ms |
| Modern High | 11.44 ms | 2.14 ms |
| Reported settings | 13.78 ms | 0.87 ms |

These are CPU preparation measurements, not interactive FPS predictions. The
before/after captures match exactly for all three configurations. The disc-backed
`cached_terrain_matches_rebuilt_frames` check compares cached rendering with full
rebuilds through camera, scenery phase, atmosphere and cache invalidation changes,
including a transparent-face variant. Imported cinematic handoff also checks
restoration of previously cached terrain. Captures and raw local measurements
remain outside Git in `private/t2-gaisen-performance-20261112`.

## Weighted T2 characters (2026-11-13)

Native T2 bodies use per-vertex joint weights, which previously kept them on the
CPU even when the shared renderer could pose rigid T1 models on the GPU. The
shared posing path now handles those weights too. Original palette-fog drawing,
detached parts and devices without the required compute support retain their
CPU path. No simulation updates or graphical preset values change.

A release-build audit on GTX 1080 Ti / Vulkan at 1280×720 used 30 Hz logic and
60 Hz presentation, with 60 warm-up frames and 120 measured frames per scene:

| Scene | Modern High CPU preparation, before → after | Saved settings, before → after |
| --- | ---: | ---: |
| Native T2 training | 0.98 → 0.52 ms | 1.17 → 0.71 ms |
| Native T2 Mission 1 | 0.70 → 0.35 ms | 1.24 → 0.74 ms |
| Imported T1 Mission 1 | 0.76 → 0.29 ms | 1.99 → 1.21 ms |
| Imported T1 Mission 4 (Cross the Checkpoint) | 1.88 → 1.55 ms | 4.15 → 3.33 ms |

The saved configuration enables all enemies, detailed characters, Botanical
foliage and eight 2048-pixel point-shadow lights. Mission 4 includes nearby
combat. These preparation timings include geometry and uploads; they are not
interactive FPS estimates. GPU execution is measured with timestamps separately
from the serialized completion wait.

CPU/GPU checks compare 2.4 million weighted vertices, conservative shadow bounds,
secondary motion, constrained-buffer fallbacks and actual native/imported frames.
The moving vertex comparison has maximum errors of 0.0078125 world units and
0.0000066 in normals. The four fixed scene comparisons are pixel-identical;
the moving imported-stage captures differ at 39–71 of 921,600 pixels. Original
imported-stage captures remain identical. Native background corrections are
verified separately. Evidence is in ignored `private/t2-performance-20261113`.

## Historical measurements

The October 27–November 7 optimisation journal is preserved in
[the archived measurements at 7b6c42c](https://github.com/Fuuzetsu/tenchu-shinobi-gaisen-pc/blob/7b6c42cbe732af0f8b1e321fd92627e95fb21c81/docs/performance.md#interactive-wayland-follow-up-2026-10-29).
Those experiments compare particular builds, machines and scenes; their proposed
next steps and pending-work lists are not the current development plan.
The [per-control cost guide](graphics-performance.md) retains the useful comparison
method and measured tradeoffs. File a current performance report with the build,
hardware, settings, scene and benchmark JSON so it can be compared on that setup.

### Shared character and cinematic rendering

Imported T1 cinematics now use the same retained stage, indexed character meshes
and eligible GPU poses as ordinary T1 rendering. VRAM uploads follow its revision
instead of running on every presentation. Native T2 gameplay and story scenes
share static scenery caching, indexed actors and reusable geometry buffers.
T2 palette fog still uses its original triangle/depth rules; joint-weighted skins
and unsupported compute backends retain the scalar CPU path.

The first Ayame Mission 4 scene (sequence 10) was measured at 1280×720 on the same
GTX 1080 Ti / Vulkan setup, with 20 warm-up and 80 measured frames, one source
tick every two presentations. Geometry and upload preparation fell from
**29.10 ms to 1.82 ms** with the reported enhanced settings, and **2.18 ms to
1.12 ms** in Original. These are CPU preparation times, not total frame times.
The Original capture was byte-identical; enhanced captures differed in one
pixel by one colour-channel unit after GPU posing. Raw timings and captures
remain outside Git under `private/render-consolidation-20261112`.

The Mission 4 measurements above were originally labelled M10. Internal stage
ID 9 is Cross the Checkpoint; Free the Princess (Mission 10) is internal ID 7.
The archived fixture filenames retain their old labels.

The corrected Mission 10 Onikage fixture (Free the Princess, Ayame, sequence 10,
elapsed tick 31) was measured on November 13 with the same resolution, GPU and
20/80-frame sampling. With the mapped T2 cast, preparation averaged **0.15 ms
in Original** and **0.20 ms with the saved enhanced settings**. The original-cast
comparison measured 0.18/0.22 ms; this small difference establishes no regression,
not a separate performance gain. Captures and measurements are in ignored
`private/t2-performance-20261113/onikage-profile`.
