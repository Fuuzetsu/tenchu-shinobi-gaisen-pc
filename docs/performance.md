# Performance measurements

F1 → General → **Benchmarks…** runs four fixed scenes using the
current graphics: village lanterns/characters, Mission 5's courtyard, a garden
with vegetation/water, and a waterfall. Each scene advances the same 60 Hz
presentation timeline and original 30 Hz simulation. Loading and the first 60
frames are excluded. Escape cancels. The current game, preferences and input
recording are restored afterwards; temporary presentation particles are rebuilt.
Changing the window size or losing focus cancels a windowed measurement.

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

**Auto-optimise graphics** tests Ultra down through Low, with Original as a
fallback. Choose a 30/60/120 FPS target and keep the game focused at your intended
playing resolution. A tier must pass all four scenes: CPU/GPU p95 below 85% of
the frame budget and observed frame p95 within 12% of the target interval.
A failed scene immediately advances to the next tier. Each case has 60 warm-up
frames and 240 measured frames. The best passing tier is saved and applied as
**Auto-optimised** when the original session is restored. Cancellation changes
nothing. If even Original cannot meet the target, the result says so.

This is a short sample, not a guarantee for every scene or unusual weather/mod.
Re-run after changing resolution, artwork or hardware. Missing GPU timestamps
are reported; that selection uses observed frames and CPU time. Display refresh
and compositor limits can prevent a high target regardless of graphics quality.

User presets support new names, overwrite, apply, separate rename and delete.
Built-ins and Auto-optimised are protected. The library is `presets.json` next to
the preferences (`TENCHU_PRESETS` overrides it); browsers use local storage.
For a non-persistent automatic report, use `benchmark DISC --auto 60 --output
report.json`. CLI benchmarks never overwrite preferences or preset libraries.

The full-artwork 4K offscreen test on the hardware below selected Low at 60 FPS:
its four GPU p95 values were 5.47, 3.24, 9.21 and 4.08 ms. Medium failed the garden
at 16.74 ms GPU p95; High and Ultra failed the first scene. This describes that
machine and serialized offscreen test, not a universal default or FPS promise.

For reproducible native reports:

```sh
./dev.sh cargo run --release -- benchmark '/path/to/game.cue' \
  --preset 'Modern Ultra' --size 3840x2160 --seconds 20 --output report.json
```

Set `TENCHU_ASSET_PACK` to use replacement artwork. Reports contain the actual
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
