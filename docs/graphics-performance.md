# Graphics setting costs

Measured on 2026-10-28 with a Ryzen 9 9950X3D, GTX 1080 Ti and NVIDIA
580.126.09, using Linux/Vulkan and release revision `af886a6`. These are costs
of the current implementation, not requirements for other hardware or predictions
for WebGL/WebGPU.

The reference is **Modern Ultra at 3840×2160**, with the complete campaign
replacement artwork and clear HUD text. Each comparison changes only the named
option, unless explicitly described as a group. The four benchmark cameras cover
village lanterns/characters, Mission 5's courtyard, a garden with vegetation/water,
and Mission 4's waterfall. All reports verify the requested settings and loaded
artwork in each mission.

Each scene renders the same three-second presentation path: 60 warm-up frames
are discarded, followed by 120 CPU and 119 GPU samples. Loading, shader creation
and preference-change hitches are outside the measured interval. Offscreen GPU
completion avoids virtual-display presentation waits. Its overall FPS is not
interactive FPS; the tables use GPU timestamp durations instead.

Other TAS work and the normal desktop remained running. CPU wall times varied
substantially, so small CPU differences are not reliable. GPU baselines are
repeated to check drift; small differences should not be interpreted as precise
speedups. These short scenes are not sustained combat or fully accumulated
weather stress tests.

## Measurements

55 complete runs, 220 scene measurements. Reference GPU means
are averaged across 3 Ultra runs. Reference run-to-run spread is
0.18 ms, 0.07 ms, 0.20 ms, 0.06 ms in scene order.

| Reference | Village | Courtyard | Garden | Waterfall |
| --- | ---: | ---: | ---: | ---: |
| Ultra GPU time | 30.7 ms | 14.4 ms | 46.8 ms | 18.1 ms |

**GPU milliseconds saved per frame** below: positive is faster; negative is
slower. Repeat checks of a variant are averaged. Each column is a separate
scene, not an uncertainty interval. Savings are conditional on Ultra and cannot
be added together. A 60 FPS
frame has a 16.67 ms budget; CPU/presentation also need to keep up.

| Change from Ultra | Village | Courtyard | Garden | Waterfall |
| --- | ---: | ---: | ---: | ---: |
| 4× MSAA → Off | +4.8 | +3.8 | +5.2 | +4.0 |
| 4× MSAA → 2× | +3.6 | +2.4 | +3.4 | +2.7 |
| Local shadow budget 8 → 0; keep local lighting | +15.5 | +5.1 | +25.9 | +6.3 |
| All dynamic shadows → Off; keep lighting | +18.4 | +5.9 | +29.8 | +7.9 |
| Local shadow budget 8 → 2 | +7.8 | +0.7 | +19.7 | +2.0 |
| Local shadow resolution 2048 → 1024 | +9.4 | +3.3 | +13.2 | +3.8 |
| Local shadow resolution 2048 → 512 | +11.8 | +4.2 | +17.1 | +4.8 |
| Emitter size 100 → 0 | +3.1 | +2.4 | +4.5 | +0.8 |
| Shadow filtering radius 3 → 0; keep emitter size 100 | +0.4 | +0.1 | +0.7 | +0.1 |
| Maximum local lights 8 → 2 | +8.0 | +0.6 | +20.3 | +2.0 |
| Local dynamic lighting → Off; keep moon | +15.8 | +5.1 | +26.9 | +6.6 |
| Sun/moon cycle → Off; keep local lights | +4.8 | +2.5 | +5.7 | +3.0 |
| Directional shadow resolution 4096 → 2048 | +1.5 | +0.5 | +2.0 | +0.9 |
| Surface depth Parallax → Off | +1.6 | +0.5 | +1.4 | +0.1 |
| Surface depth Parallax → Bump | +0.4 | 0.0 | +0.2 | -0.1 |
| Vegetation Botanical dense → Original | +0.4 | +0.2 | +1.6 | +0.2 |
| Vegetation Botanical dense → Botanical | +0.1 | +0.1 | +0.6 | +0.2 |
| Vegetation Botanical dense → Fuller | -0.5 | -0.6 | -7.6 | -0.2 |
| Character surfaces Fine → Original | +1.0 | +0.5 | +1.4 | -0.1 |
| Character surfaces Fine → Refined | +0.1 | 0.0 | +0.1 | 0.0 |
| Character contours Subtle → Original (Fine surfaces retained) | 0.0 | 0.0 | -0.1 | 0.0 |
| Smooth model shading → Off (Fine character surfaces retained) | +0.1 | 0.0 | -0.1 | -0.1 |
| Modern water → Off | +0.1 | 0.0 | +0.3 | +0.3 |
| Waterfall Fine curves → Original | 0.0 | 0.0 | +0.2 | +0.3 |
| Waterfall spray 60 → 0 | 0.0 | 0.0 | -0.1 | 0.0 |
| Image effects → Off (whole HDR/finishing group) | +5.4 | +3.7 | +5.4 | +4.9 |
| Contact shading 25 → 0 | +2.6 | +1.1 | +2.6 | +2.5 |
| Bloom 12 → 0 | +0.8 | +0.6 | +0.6 | +0.5 |
| Sharpen 10 → 0 | +0.2 | +0.1 | +0.1 | +0.2 |
| Replacement artwork → Off (whole pack) | +7.6 | +1.8 | +10.9 | +3.4 |
| Anisotropy 16× → 8× | +0.2 | +0.1 | +0.5 | +0.1 |
| Anisotropy 16× → Trilinear | +1.4 | +0.7 | +2.5 | +0.6 |
| Close-up textures Crisp → Smooth | +0.9 | 0.0 | -0.2 | -0.3 |
| Close-up textures Crisp → Cubic | -1.0 | -0.1 | -4.2 | -1.9 |
| Hair/cloth/equipment motion → Off together | 0.0 | 0.0 | +0.4 | 0.0 |
| Breeze 20 → 0 | +0.1 | 0.0 | 0.0 | +0.1 |
| Smooth model motion → Off | -0.1 | 0.0 | 0.0 | +0.2 |
| Falling petals/leaves → Off | 0.0 | -0.1 | +0.4 | 0.0 |
| Black distance fog → On at 100% | -0.1 | 0.0 | 0.0 | 0.0 |
| Output 4K → 2560×1440 | +8.4 | +5.5 | +12.6 | +6.0 |
| Output 4K → 1920×1080 | +11.7 | +7.5 | +17.3 | +8.1 |
| Varied tree shapes → Off | -0.1 | 0.0 | -0.1 | -0.1 |

**CPU observations for character refinement** (median milliseconds per frame).
The reference range spans the repeated Ultra runs. Concurrent TAS work makes
these indicative rather than clean isolated CPU estimates; other small CPU
differences are deliberately not ranked.

| Character setting | Village | Courtyard | Garden | Waterfall |
| --- | ---: | ---: | ---: | ---: |
| Fine reference range | 4.0–4.2 | 3.3–3.7 | 3.2–3.9 | 8.6–31.0 |
| Refined | 2.6 | 2.1 | 2.6 | 5.3 |
| Original | 2.4 | 2.1 | 2.2 | 4.6 |

The later waterfall reference rose to 31 ms on the CPU while its GPU time
stayed at 18.1 ms, illustrating the CPU variability with identical graphics.

Weather comparisons below use **added GPU milliseconds**, so a positive
number is an extra cost. Rain/snow start clear and run for three seconds;
the first second is excluded. These do not measure a saturated footprint,
runoff or accumulated-puddle workload.

| Weather comparison | Village | Courtyard | Garden | Waterfall |
| --- | ---: | ---: | ---: | ---: |
| Rain, High detail vs original weather | +8.4 | +4.2 | +16.0 | +1.9 |
| Snow, High detail vs original weather | +16.9 | +7.4 | +36.1 | +1.5 |
| Rain Low vs High detail | -0.3 | +0.1 | +0.1 | -0.2 |
| Snow Low vs High detail | -0.5 | -0.2 | -0.6 | -0.2 |
| Rain reflections Off vs 100 | -1.9 | -1.1 | -4.9 | 0.0 |
| Rain haze Off vs 25 | -1.8 | -1.0 | +3.7 | -0.5 |
| Snow drift softness Off vs 65 | -2.7 | -1.6 | -6.0 | -0.2 |

## Practical interpretation

Local shadow maps are the largest cost in the normal-weather reference. Start
with **2048→1024 local shadow resolution**, then **4×→2× MSAA** if needed. This
keeps all lights, shadow sources, artwork, model motion and water enhancements.
These are trade-offs, not a promise that either change alone reaches 60 FPS.

The unchanged shadow pass explains why output resolution alone does not solve
everything: at 1080p the garden still takes about 29.4 ms on the GPU with the
remaining Ultra settings. The same map sizes are rendered at 4K and 1080p.
Much of the replacement-pack difference is also inside the shadow pass: in the
garden, about 8 ms of its roughly 11 ms extra GPU time is shadow rendering.
The pack changes material/model behavior together, so this does not isolate
texture resolution alone.

Keep **Crisp** close-up filtering for a reasonable cost/appearance balance.
Cubic adds up to about 4 ms in these views. Reducing anisotropy from 16× to 8×
saves very little here; going to Trilinear is a more visible quality trade-off
and saves up to about 2.5 ms. Parallax→Bump likewise shows little difference in
these cameras; a close roof/floor view may exercise more parallax work.

Hair/cloth/equipment motion, interpolation, breeze and water have small GPU
differences in this sweep. Character subdivision is more clearly a CPU-side
trade-off, because it expands per-frame vertex processing; the busy machine
does not support precise small CPU savings. Retained blood, tracks and particles
still need an active-effects stress test before assigning them a per-frame cost.

The vegetation modes are different algorithms, not a strictly ordered cost
ladder. Fuller duplicates curved foliage cards; Botanical builds leaf sprays.
Fuller takes 54.3/54.4 ms in two garden runs, versus roughly 46.8 ms for Botanical
dense. Most of the extra time is in scene shading, consistent with the added
overlapping cards. An appearance label alone is not a reliable performance rank.

Custom weather is a substantial additional expense. The short High-detail rain
test adds about 2–16 ms; snow adds about 2–36 ms. Low particle/shelter detail
recovers less than 1 ms in these views. Disabling wet reflections saves up to
5 ms in rain, and disabling snow drift softness saves up to 6 ms in snow, but
both change the appearance. Most of the extra GPU time is in the scene shader,
so reducing only particle counts cannot address it. This is a strong target
for further shader profiling; the modern presets do not enable custom weather
unless the player selects it.

Rain haze has a repeatable exception: disabling it saves about 0.5–1.8 ms in
three views but makes the garden about 3.7 ms slower in both checks. The cause
has not been isolated; it is not a reliable blanket performance adjustment.
The table retains that result rather than assuming fewer shader operations
must always mean a faster frame.

## Reading the settings

The following explains the cost model from the implementation. It supplements
the measurements; it does not assign invented timings to untested situations.

### Lighting and shadows

| Controls | What affects performance |
| --- | --- |
| Dynamic lighting; lantern/prop lights; fire lights | Add/remove local light sources. An unshadowed light adds per-pixel distance/facing/lighting work. Turning local lighting off leaves the separately enabled sun/moon cycle active. Fire, explosion and spark lights depend on the current action. |
| Maximum lights | Caps active local lights. Raising it can also admit more shadow casters up to the separate shadow budget. A scene with fewer available sources cannot use the whole budget. |
| Dynamic shadows | Controls both local and directional shadows. Disabling them also avoids preparing extra actor shadow geometry. |
| Shadowed point lights | Each source requires six shadow views. Keeping many shadowed lamps is substantially more expensive than keeping the same number of unshadowed lamps. |
| Local shadow resolution | Resolution is per face. At 2048, one six-face Depth32Float map uses about 96 MiB; 1024 uses 24 MiB and 512 uses 6 MiB. Eight allocated sources therefore use about 768/192/48 MiB for these maps alone. Lower resolution saves rendering work and memory while retaining lights and shadows. |
| Light source size | Nonzero size enables blocker sampling and distance-dependent soft shadows, with up to 32 filter taps. Zero removes that work but changes the appearance to a point emitter. |
| Shadow filtering radius | Zero can use a cheaper comparison, but a nonzero source size can still select the wider area-light filter. Reducing radius alone does not necessarily remove the expensive filtering path. Positive radii generally use the same number of taps. |
| Light radius | Larger radii can illuminate more pixels and include more shadow geometry; cost depends on light placement and scenery. |
| Light intensity, ambient brightness, sky tint | Mostly shader parameters, not quality/work budgets. Changing a positive intensity does not reduce the number of shadow passes. |
| Manual sky strength, color, heading, elevation, range | An active directional light adds one shadow map and per-pixel lighting. Heading/elevation/range change which geometry it covers. Strength and color mainly change arithmetic inputs. Manual controls are superseded while the sun/moon cycle is enabled. |
| Directional shadow resolution | Larger maps increase memory and shadow raster work. 4096→2048 reduces this map from 64 to 16 MiB, independently of output resolution. |
| Sun/moon cycle, start time, time lapse, sun/moon strength | Activates sky lighting and its directional shadow. Time changes light direction, and therefore the shadow workload. The clock itself is cheap; a stationary clock still renders the shadow each frame. |
| Day/night ambient and relighting | Color/brightness controls with roughly the same shader work. These are not substitutes for reducing shadow quality. |
| Original round shadows | Extra original sprite geometry/blending. Separate from model-cast shadows; enabling both draws both. Not measured separately here. |
| Movable light and light markers | A movable light uses the existing light/shadow budgets. Marker text is diagnostic overlay work. |

### Textures, models and motion

| Controls | What affects performance |
| --- | --- |
| Replacement artwork | Changes textures, some models and material metadata together. Includes loading/RAM/VRAM cost. Its benchmark is the whole checkbox, not a pure texture-resolution comparison. |
| Texture filtering | This renderer implements atlas-aware anisotropy in the shader. The maximum sample count rises with the chosen level, but actual work depends on viewing angle and texture footprint. Trilinear keeps mipmaps; nearest/bilinear lose their distance filtering. |
| Close-up textures: Smooth/Crisp/Cubic/Pixel | Cubic performs a 4×4 reconstruction for magnified texels; Smooth, Crisp and Pixel avoid that 16-tap reconstruction. Distant surfaces use the selected mip/aniso filtering instead. |
| Texture reconstruction / upscale | Scale2x is performed when preparing original images, not every frame. It increases texture memory and can affect cache/sampling behavior. Authored replacement textures are not scaled again. This is not the window's render resolution. |
| Surface depth: Off/Bump/Parallax | Bump samples the height/gradient material; Parallax can additionally trace up to eight height samples at close range. Both fade with distance and apply only to eligible materials. Off removes the height layer and shader path. |
| Depth strength | Mostly changes displacement magnitude; zero bypasses surface detail. Smaller nonzero values generally keep the same sampling work. |
| Textures, vertex shading, exposure | Shading/exposure are inexpensive parameter changes. Turning textures off skips color filtering, but textures are still consulted for cutout/transparency coverage. |
| Character surfaces | Subdivision increases vertices to transform, upload and render, including shadow geometry. It can matter more with many visible/shadow-casting actors. Custom models are retained. |
| Character contours; smooth model shading | Normals/contours are prepared on reload. Character-surface refinement takes precedence over the older contour path; lowering contours while refinement is active is not a useful performance adjustment. Smooth shading alone does not add a shadow pass. |
| Vegetation volume | Adds foliage geometry. Cost depends on plants in the view and shadow maps; original cards can themselves have substantial texture/overdraw cost. |
| Varied tree shapes | Per-instance shape transforms, mostly for prepared scenery, plus any per-frame geometry that uses those instances. Does not increase triangle count on its own. |
| Smooth model motion | Interpolates presentation poses between original simulation ticks. Adds pose/transform work, not extra gameplay updates. |
| Hair, loose cloth, equipment flex | Small spring simulations plus vertex deformation of authored regions. Cost scales with supported actors and their refined vertex counts. Tested together in the sweep, not independently. |
| Motion strength, spring stiffness | Mostly arithmetic parameters. Zero strength bypasses actor secondary motion; smaller positive values do not simplify the mesh. |
| Breeze | Vertex deformation for tagged vegetation is repeated in scene and shadow passes; also drives enabled secondary motion. Zero skips vegetation wind math. |
| Detailed rope/hook | Extra geometry while the grapple is visible. Idle camera scenes do not measure the active grapple. |

### Water, weather and persistent/contact effects

| Controls | What affects performance |
| --- | --- |
| Modern water | Enables specialized surface shading and waterfall preparation/spray. The water fixture exercises this, but other stages can have different coverage. |
| Waterfall curves | Subdivides waterfall geometry: Curved uses 4 subdivisions per edge, Fine curves 8. This can multiply both triangle work and CPU processing of transparent surfaces. |
| Water ripples, reflections, flow, foam | Mostly shader/animation strengths within the enabled water path. These reflections are sky/light highlights, not a second rendering of the entire scene. Setting an individual strength to zero does not necessarily eliminate its calculations. |
| Waterfall spray | Extra transparent particles near waterfall edges. Curvature/detail also controls the particle budget. Its cost depends on waterfalls in view. |
| Weather mode | Rain/snow add particles, surface collision queries, shelter maps and wet/snow surface shading. Clear can retain wetness or snow while it dries/melts; Original bypasses the custom weather presentation. |
| Weather detail | Low/Medium/High allow 384/896/1792 particles at full precipitation and use 256/512/1024 shelter maps. These are separate from lamp shadow maps. |
| Precipitation amount | Scales the particle budget and accumulation rate. Zero stops new precipitation but can leave accumulated surface effects to render. |
| Particle distance | Redistributes a bounded particle budget rather than multiplying it by map area. Screen coverage and collision work can still change. |
| Weather wind strength/compass | Changes particle paths and which surfaces are exposed. Editing direction can require refreshing shelter/collision information; steady-state cost depends on the resulting paths. |
| Surface wetness, puddle coverage, pooling speed | Control the accumulated surface state and where reflection/ripple shading applies. Faster pooling is not extra gameplay simulation, but can expose the expensive wet-surface path sooner. |
| Wet reflections | Adds sky/local-light highlights and ripple shading to wet surfaces. Zero skips that reflection branch; higher positive strengths largely change brightness rather than sample count. |
| Rain haze | Samples shelter along the viewing ray as well as applying fog. Zero skips those haze samples. This is more work than the simple black fog. |
| Rain streak visibility | Changes particle width/brightness, not their count. Wider streaks can increase blended pixel coverage. |
| Roof runoff; roof drip amount | Additional roof/eave queries and particles. Bounded to 192 runoff particles and 96 eave sources; depends on wetness and nearby roofs. |
| Snow cover | Controls surface snow shading. This needs shelter/variation/lighting work even when the visible accumulation is still shallow. |
| Snow drift softness | Nonzero softness takes extra shelter-map samples around boundaries. Reducing it to zero removes this smoothing work but restores sharper shelter contours. Positive strengths keep the sampling pattern. |
| Snow footprints; fresh-snow burial time | Footstep projection and retained decals. Longer retention can leave more geometry to draw. Bounded to 512 marks/65,536 vertices. No sustained footprint stress test was performed. |
| Persistent blood; amount; wound duration; maximum marks | Work grows with active wounds, droplets and retained projected marks. Marks are capped by both the selected count and a 98,304-vertex budget. These idle scenes do not measure a heavily bloodied battlefield. |
| Grapple sparks; spark density | Short particle/light bursts at hook impact. Density scales sparks; light/shadow budgets still apply. Not exercised by these camera sweeps. |
| Enhanced splashes; contact scatter | Event-driven particles, ripples and ground marks when moving through water/snow/leaves. Idle scenes do not establish their peak cost. |
| Falling petals/leaves; amount | Adds bounded particles, collision/settling and translucent drawing near tagged trees. The short sweep measures early falling particles, not a fully populated ground layer. |
| Clear weather/blood/footprints | One-shot removal of accumulated effects. Can reduce subsequent work by removing retained particles/decals. |

### Image finishing and display

| Controls | What affects performance |
| --- | --- |
| HDR / image effects | Enables the HDR intermediate target and full-screen finishing pass, plus whichever effects below are active. Disabling it removes these together. |
| Contact shading / radius | Screen-space ambient occlusion: depth reconstruction plus neighborhood samples. Zero strength disables it; reducing a positive strength mostly changes darkness. MSAA increases the depth samples it must read. |
| Bloom / threshold | Nonzero bloom adds four downsample stages and compositing samples. Zero removes them. Threshold and positive intensity mainly change the appearance. |
| Sharpen | Nonzero sharpening samples four neighbors; zero skips those samples. |
| Filmic tone mapping, contrast, saturation, temperature, gamma, vignette | Arithmetic in the existing finishing pass. Generally minor compared with shadows/texture sampling; turning down a strength does not remove the pass. |
| Haze and fog tint | Cheap distance/color calculations in the scene shader. |
| Black distance fog; fog distance | Currently a color blend, not geometry/draw-distance culling. Restoring black fog is not expected to recover the cost of drawing distant scenery. |
| Edge anti-aliasing | MSAA increases color/depth storage, raster work, resolves and post-process depth reads. Only supported sample counts are selectable. Higher than 4× was not tested. |
| Window/render resolution | Affects main-scene and post-process pixels, but shadow-map dimensions are independent. 1080p has one quarter of 4K's output pixels; the whole renderer therefore does not become exactly four times faster. |
| Fullscreen | Performance depends on the resulting pixel dimensions and platform presentation path. Fullscreen itself is not a visual-quality tier. |
| Vsync | Caps/paces presentation. Disabling it can remove a display cap without reducing rendering work. Offscreen measurements do not predict compositor behavior. |
| Clear text, HUD, FPS counter | UI/text geometry; normally a small cost. These were not independently timed. |
| Performance charts, actor labels, hitboxes, sight bounds, collision overlays | Diagnostic CPU/UI/geometry work. Sight bounds include wall-clipping ray queries per guard; showing all collision geometry can be heavier than simple labels. Not included in quality-preset timings. |
| Field of view/free camera | Changes visible and shadow-relevant geometry/pixel coverage; can change cost substantially without changing a quality setting. |

Gameplay speed, enemy activation, detection distance, god mode and other
simulation options are outside this graphics comparison. In particular, keeping
every enemy active can increase CPU and actor rendering work independently of
the chosen visual preset.

## Reproduce a comparison

Use the [benchmark instructions](performance.md) for the fixed scene suite.
To isolate one setting, use the same build, artwork, resolution and duration;
copy a complete settings JSON, edit just that field, then run:

```sh
TENCHU_SETTINGS=/path/to/variant-settings.json \
TENCHU_ASSET_PACK=/path/to/artwork-pack \
./dev.sh target/release/tenchu-pc benchmark '/path/to/game.cue' \
  --size 3840x2160 --seconds 3 --offscreen --output variant.json
```

Omit `--preset`, which would replace the edited graphics. Repeat the reference
before/after a batch; compare the same scene's `gpu.mean_ms` and GPU passes.
The report also records actual settings, artwork status and sample counts.
For clean CPU comparisons, repeat when other CPU-intensive work is idle, and
increase the measured duration for small effects or accumulated particles.
