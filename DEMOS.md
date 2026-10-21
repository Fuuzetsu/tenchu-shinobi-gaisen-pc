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

For a clean recording use `--clean` to hide captions and transport, or press V
to keep the explanations while hiding controls. `--mute` silences the demo.
Capture the native window with your desktop recording tool. F9 produces a PNG with the clip ID and timeline frame in its filename,
not a video. The profiler and inspection overlays remain visible in their clips
because they are the feature being demonstrated. Hide or omit those clips for
unannotated scenery footage.

`--duration 8` makes every selected clip eight seconds long; the default lengths
allow each comparison a short hold. Loading a new mission is outside its clip
timeline. On a slow machine the showcase slows instead of discarding a whole
comparison. The timeline schedules presentation at 60 Hz and gameplay at its
original 30 Hz, independent of display cadence; explicit speed/single-step clips
change tick pacing. Acceleration executes every game tick. This is staged game
playback, not an input recording or a campaign/fidelity regression route.

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

The normal scene clips disable mission event triggers so the shot is not
interrupted by a story or result transition. The cinematic clip replays Mission 2's opening at each speed, keeping the
original script and camera. Inputs for character demonstrations are a small,
repeatable attack/crouch sequence. Restarts use the same initial RNG seed.

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
| Image effects | `contact`, `bloom`, `atmosphere`, `grading`, `finish`: contact strength/radius, bloom strength/threshold, haze/tint, temperature, saturation, vignette, filmic mapping, contrast, gamma and sharpening. |
| Player model | `bodies`: compatible disc bodies and configured folder bodies, with original animation and weapons. |

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
byte-for-byte unchanged. All 79 desktop checks, Clippy, formatting and Nix
native-package/portable-WASM checks pass. Captures and disc data stay outside Git.

The showcase is currently a native window mode. It does not encode video, and
no browser demo host or authored HD pack is bundled. Performance charts show
real host timings, so those measurements and accelerated audio are not suitable
as deterministic media exports. Other operating systems were not exercised.
