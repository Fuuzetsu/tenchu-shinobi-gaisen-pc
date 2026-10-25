# Native feature showcase

Run the full sequence using your own disc:

```sh
./dev.sh cargo run --release -- demo '/path/to/game.cue'
```

List the named clips, then select any subset in the order you want:

```sh
./dev.sh cargo run --release -- demo --list
./dev.sh cargo run --release -- demo '/path/to/game.cue' --clips moon,shadows,motion,bodies --loop
```

Each clip reloads its own mission, character and staged scene. Its animated
controls, camera and explanation are separate from the ordinary game host.
The mode does not read or write campaign progress and never saves preferences,
including changes made through F1. The current audio volume and configured
model folder are used; `TENCHU_ASSET_PACK` and `TENCHU_MODEL_DIR` work normally.

## Controls and recording

| Control | Action |
|---|---|
| Space / P / F3 | Pause or resume the clip and simulation |
| Left / Right | Previous / next clip |
| Home | Reload and restart the current clip |
| Tab | Open the mouse-driven clip picker; playback pauses |
| H | Toggle captions, animated value and progress |
| V | Toggle the small transport panel |
| F1 | Inspect/change temporary settings; normal menu pause applies |
| F9 | Save a screenshot using the existing screenshot path |
| Esc | Exit demo mode (close F1 first if it is open) |

The picker can play one clip alone or a checked selection. Command-line clip
order is preserved, including repeated IDs. Checked selections use catalog
order. The full sequence holds on its last shot; use `--loop` to repeat or
`--exit` to close automatically at the end.

For a clean view use `--clean` to hide captions and transport, or press V
to keep the explanations while hiding controls. `--mute` silences live playback.
F9 produces a PNG with the clip ID and timeline frame in its filename.
The profiler and inspection overlays remain visible in their clips
because they are the feature being demonstrated. Hide or omit those clips for
unannotated scenery footage.

`--duration 8` makes every selected clip eight seconds long; the default lengths
allow each comparison a short hold. Loading a new mission is outside its clip
timeline. On a slow machine the showcase slows instead of discarding a whole
comparison. The timeline schedules presentation at 60 Hz and gameplay at its
original 30 Hz, independent of display cadence; explicit speed/single-step clips
change tick pacing. Acceleration executes every game tick. This is staged game
playback, not an input recording or a campaign/fidelity regression route.

## 4K video export and chapters

```sh
./dev.sh cargo run --release -- demo '/path/to/game.cue' --video "$HOME/Videos/tenchu-showcase.mp4"
```

This renders every presentation frame at **3840×2160, 60 fps** and streams it to
FFmpeg. It produces a silent H.264 High Profile MP4 (CRF 16, up to 68 Mbps,
BT.709 limited-range 4:2:0, fast start). The Nix package and shell supply FFmpeg;
`TENCHU_FFMPEG` can select another installation. The destination directory must
exist, and existing video/chapter files are never overwritten.

The full catalog is 706 seconds / 42,360 frames. No display capture is involved:
the renderer waits for GPU readback and encoding before advancing the exact
60 Hz timeline. Host speed changes completion time, never output cadence.
Mission loading adds no video time. The window is only a host for the renderer;
it need not be 4K and does not display the exported frames. Esc or closing it
cancels the export and removes its temporary files.

Captions are scaled for 4K; transport controls are hidden. `--clean` also hides
captions. Character closeups place captions above the action, with fixed daylight
for the refinement and secondary-motion comparisons. `--clips`, `--catalog` and
`--duration` work as in live playback;
`--loop` is rejected. Interactive input is ignored and preferences are never saved.
Profiling/FPS demonstrations still measure real host work, including readback
and encoder waits, and their captions identify this. Those numbers are not a
real-time gameplay benchmark. The video still plays at 60 fps.

Each selected clip becomes an embedded MP4 chapter. The adjacent
`tenchu-showcase.chapters.txt` contains the same titles/timestamps for the
YouTube description. Paste that file into the video's description when
uploading: [YouTube reads chapters from description timestamps](https://support.google.com/youtube/answer/9884579),
starting at 00:00, with at least three chapters and at least ten seconds per
chapter. The default catalog meets these requirements; short custom selections
may not. The encoding follows the [YouTube SDR upload recommendations](https://support.google.com/youtube/answer/1722171).
Nothing is uploaded automatically.

Verify an export using the same Nix environment:

```sh
./dev.sh python3 tools/verify-demo-video.py '/path/to/showcase.mp4' --catalog '/path/to/catalog.json' --youtube
```

The verifier checks 4K/60 encoding, every presentation timestamp, chapter titles
and durations, absence of audio and fast-start metadata, then decodes the
complete video. It writes an adjacent `.validation.json`. Omit `--catalog` when
no exact catalog is available; omit `--youtube` for clips shorter than ten seconds.

## Authoring clips

The built-in catalog is [demos.json](crates/tenchu-pc/data/demos.json).
Copy it outside the repository, keep/edit the clips you want, and launch with
`--catalog /path/to/my-demos.json`. IDs must be unique; descriptions, titles,
duration (2–120 seconds), mission (0–10), character (0/1), preset (0–6), shot and
feature are validated. `--clips` also works with a custom catalog.

A shot chooses a player, lamp, fire, guard, wide or cinematic anchor from the actual
stage data. Features are typed Rust programs in `src/demo/presentation.rs`;
adding a genuinely new animated control belongs there. Playback, transport,
scene preparation and overlay rendering are separate modules.

Custom clips can set `artwork: false` or `true` to hold a before/after state
throughout the artwork feature, and `fog: false` for clear distant scenery.
An optional `camera` object accepts `eye` and `target` world-coordinate triples,
optional `end_eye`/`end_target`, and `fov` (55 degrees by default). It moves
smoothly between those endpoints on the same offline timeline.

`weather_warmup: 35` settles 35 seconds of cosmetic weather before the clip starts
(0–60 seconds, default 0). Gameplay does not advance during this pre-roll. The
particles and accumulation then continue on the normal game clock; this keeps
snow/footprint demonstrations short without substituting a painted snow mask.

For matched gameplay comparisons, use `feature: "artwork"`, `shot: "gameplay"`
and `recording: {"path": "route.inputs", "start_tick": 200}`. Paths are relative
to the catalog. The self-contained recording supplies the seed, inventory,
difficulty and language; its character/mission must match the clip. Inputs are
replayed from the start to the chosen tick with normal event handling, then
continue at 30 Hz. The file must contain enough inputs for the whole clip.
Omit `camera` to retain the gameplay/cinematic camera, or provide it for an
independent presentation view. Paired clips with identical paths, offsets,
presets and cameras differ only in artwork. Fixed-artwork captions sit at the
top, leaving the player and item effects visible. Recording files currently
require the native host; the simulation and camera presentation remain portable.

The original local Mission 1 comparison uses
`private/m1-before-after-video/showcase.json`: 12 original/replacement pairs,
ten seconds each, with self-contained input excerpts beside the catalog.
Its matching pack is archived at `private/m1-boundaries/previous-pack`; set
`TENCHU_ASSET_PACK` to that directory to reproduce the earlier artwork.
The completed 240-second MP4, 24 embedded/description chapters and validation
are in that directory. Every one of its 14,400 video timestamps was verified,
and detail/end frames from each clip were inspected. These private assets and
recordings are not bundled with the repository.

The faster follow-up uses `private/m1-showcase-v2/showcase.json`: **26 pairs,
three seconds per side, 156 seconds total**, including the later terrain/eaves/
roof refinements. New views cover path corners, garden ground, blossoms, stone
lanterns, shoji, carvings, gold screens, props and character details. Recorded
actions start near their useful moments so item effects remain visible in the
shorter shots. Gameplay still advances at 30 Hz with 60 Hz presentation.
`private/m1-showcase-v2/render.sh DISC OUTPUT.mp4` exports it using the frozen
`capture-pack` beside the catalog. The MP4 has 52 per-shot chapters; its separate
`youtube-description.txt` groups these into 13 twelve-second chapters to satisfy
YouTube's ten-second minimum. The local watch page is `index.html` in the same
directory; the earlier recording remains available.

The normal scene clips disable mission event triggers so the shot is not
interrupted by a story or result transition. The cinematic clip replays Mission 2's opening at each speed, keeping the
original script and camera. Inputs for character demonstrations are a small,
repeatable attack/crouch sequence. Restarts use the same initial RNG seed.

## Menu and text clips

Artwork clips also accept `hud: true` and `sharp_text: true` to include the
in-game HUD and outline lettering. Both default off, preserving existing catalogs.
For an animated original-menu clip, use `shot: "wide"`, `feature: "artwork"`,
a mission from 1–10, no camera/recording, and for example:

```json
"frontend": {"page": "missions", "language": "ja"}
```

Pages are `title`, `character`, `missions`, `options`, `controls`, `volume` and
`language`; language codes are `en` (default), `ja`, `fr` and `it`. The clip's
mission and character select the preview and ninja. Each clip starts from the
same settled page and animation state, then runs the original menu timing on the
60 Hz presentation timeline. Gameplay and campaign progress stay frozen, and
menu interaction is disabled. Captions use the pillarbox margin.

## Campaign artwork recording

The full-campaign recording is
`private/campaign-showcase/tenchu-campaign-hd-showcase-4k60.mp4`: **11:42,
4K/60, silent**. It contains 95 matched original/HD comparisons, three seconds
per side, plus short gameplay passages. Distinct views cover all ten missions,
both players, bosses, weapons, item effects, HUD and four-language menus/fonts.
Each pair shares its camera, inputs and Crisp classic settings. HUD/menu pairs
also compare the optional outline text; gameplay runs at the original 30 Hz.

The private directory holds the frozen pack, input recordings, per-chapter
catalogs, `capture-inputs.json` hashes and `visual-review.json`. `edit.json`
defines 13 grouped chapters. `render.sh` renders missing chapter MP4s through
Nix; `assemble.py` joins them without re-encoding. `verify.py` checks every
presentation timestamp, dimensions, color metadata, chapter bounds and input
hash, then decodes the entire result. `validation.json` records the outcome.

`index.html` provides playback and chapter seeking. Run
`./dev.sh python3 private/campaign-showcase/serve.py` to serve it on
`http://127.0.0.1:44113/`. The adjacent `youtube-description.txt` includes the
same 13 chapter timestamps for pasting into an upload description. Original
geometry, UV mappings and some lighting joins remain visible; movies retain
their original artwork. These private assets and videos are not included in Git.

## Effects showcase recording

The focused effects reel is
`private/effects-showcase/tenchu-effects-showcase-4k60.mp4`: **3:06, 4K/60,
silent, 11 chapters**. It covers pond water, waterfall bends, a full day/night
cycle, rain/runoff, snow coating, vegetation, wind, character refinement, both
players' secondary motion and persistent blood. Short off/on comparisons share
the same camera; weather and combat get longer holds. Existing HD artwork stays
enabled throughout.

Run the curated sequence live, or export another copy:

```sh
./private/effects-showcase/play.sh
./private/effects-showcase/render.sh /path/to/new-video.mp4
```

The live controls are the same as the main demo: Left/Right changes chapters,
Space pauses, Tab opens the picker, F1 opens temporary settings, H hides captions
and V hides transport. `--clips rain,snow` selects just those clips; `--loop`
repeats live playback. `TENCHU_DISC` overrides the local disc path.

The adjacent `capture-inputs.json` records hashes for the frozen executable,
catalog, isolated preferences and artwork. `validation.json` records every video
timestamp, complete decoding and chapter checks. These private artifacts are
excluded from Git. Run `./dev.sh python3 private/effects-showcase/serve.py` for the
watch page at `http://127.0.0.1:44115/`; `youtube-description.txt` contains chapter
timestamps for uploading. Nothing is uploaded automatically.

The same feature clips work from a normal checkout with the user's own assets:

```sh
./dev.sh cargo run --release -- demo '/path/to/game.cue' --clips water,waterfalls,day-cycle,rain,snow,vegetation,breeze,character-refinement,secondary-motion,equipment-motion,blood
```

## F1 coverage

| F1 page | Clips / scope |
|---|---|
| General | `interface`, `profiler`: HUD, clear FPS text, live CPU/GPU history. Focus/menu pause, volume, fullscreen and vsync remain usable preferences; they are not repeatedly changed during recording. |
| Graphics | `presets`, `surfaces`, `textures`, `filtering`, `edges`, `motion`, `atmosphere`: all visual controls, with supported GPU MSAA levels only. |
| Overlays | `inspection`, `sight-distance`, `collision`, `teleport`: labels, hitboxes, sight/stance bounds, patrols, event regions, terrain, x-ray and trail. |
| Camera | `camera`, `photo`: independent camera, framing, FOV, overhead and clean photo view. Bookmark/flight controls remain available in F1. |
| Teleport / actor | `inspection`, `enemies`, `teleport`: actor selection/state, explicit position changes and collision-floor placement. |
| Simulation | `speed`, `step`, `sight-distance`, `enemies`, `cinematics`: pacing, single ticks, detection range, activation override and cinematic acceleration. Unlimited pacing remains available in normal play; timed clips use bounded rates for repeatability. |
| Lighting | `lanterns`, `fires`, `light-range`, `light-budget`, `shadows`, `shadow-quality`, `movable-light`: emitters, markers, intensity, ambient, range, light/shadow budgets, resolution, softness, original blobs and a moving world-space light. |
| Visual assets | `artwork`, `reconstruction`, `models`: pack fallback, reconstruction, normals and contours. |
| Sky lighting | `moon`, `sky-direction`, `sky-quality`: strength, direction, color, sky tint, coverage and resolution. |
| Time of day | `day-cycle`: sunrise, daylight, sunset and moonlight with continuous shadow movement and local lanterns after dark. |
| Weather | `rain`, `snow`, `rain-reflections`, `rain-puddles`, `rain-haze`, `roof-drips`, `rain-wind`, `snow-wind`, `snow-footprints`: coatings, wet highlights, pooling/ripples, visibility, eave streams, directional shelter and fresh-snow track burial. |
| Water | `water` and `waterfalls`: matching views of pond ripples/sky sheen and curved falls with restrained spray. |
| Model refinement | `vegetation` and `character-refinement`: fuller cutout clusters, refined trunks and character surfaces with original joints/UVs. |
| Secondary motion | `secondary-motion`, `equipment-motion`, `breeze`: Ayame's hair/bow, Rikimaru's garment/equipment flex and a wind sweep through the garden with matching shadows. |
| Image effects | `contact`, `bloom`, `atmosphere`, `grading`, `finish`: contact strength/radius, bloom strength/threshold, haze/tint, temperature, saturation, vignette, filmic mapping, contrast, gamma and sharpening. |
| Player model | `bodies`: compatible disc bodies and configured folder bodies, with original animation and weapons. |
| Persistent effects | `blood`, `blood-snow`: a staged guard encounter produces attached wounds and ground stains through ordinary combat, including blood staining the snow. |
| Experiment | `experiment`: refill/edit mission equipment, god/stealth toggles and two real collision-floor clicks to teleport. Only this staged clip enables these gameplay tools. |

`grenade-light` and `kaen-light` use each item twice, once with Fire / explosion
lights off and once on. Both use the original item inputs and effect lifetimes;
the full campaign artwork remains constant. Very short `--duration` overrides
can end before a grenade's fuse expires; use the authored lengths for recording.

Reload/export/reset actions and external-file selection are workflows rather
than visual comparisons. The demo does not export disc assets, invent replacement
art, alter monitor modes or repeatedly write screenshots. `artwork` explicitly
reports the original fallback if no pack is configured; it does not claim to
show authored HD content that is not installed.

## Verification

Timeline/speed tests compare the same sequence at 24, 30, 60, 75, 144 and 240 Hz.
Transport tests cover ordered selections, repeated clips, restart/end/loop and
invalid options/catalogs. Private-disc tests compare the full gameplay state
under scripted demo input at different display rates and verify temporary F1
edits. Native X11/Vulkan runs exercised the catalog, longer model/cinematic
comparisons, mouse playlist selection, keyboard navigation, pause, clean view,
PNG capture and repeated Escape exit. Private save/settings sentinels remained
byte-for-byte unchanged. Desktop regression checks, Clippy, formatting and Nix
native-package/portable-WASM checks pass. Captures and disc data stay outside Git.

Offline exports were checked at 4K, including the full default 502-second
sequence: 30,120 video packets, unique consecutive 1/60-second timestamps, no
audio stream, BT.709 color tags, square pixels, progressive H.264 High, fast-start
MP4 and all 37 chapter/title boundaries. Decoded previews from every clip and
full-resolution caption crops were inspected. Native cancellation removes
partial output, existing output is preserved, and ordinary play still resizes
at 640×480 and 1920×1080. A regression also prevents sliders from changing
between-step preset values just because their category is opened.

The showcase and video export currently use the native window host. No browser
demo host or authored HD pack is bundled. Rendered scene/timeline state is
repeatable; performance measurements still depend on the host. Other operating
systems were not exercised.
