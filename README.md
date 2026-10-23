# Tenchu: Shinobi Gaisen PC

A Rust reimplementation of the gameplay in the PlayStation release
**SLPS-019.01**, using assets from the user's game disc.

The target is faithful mechanics, including gameplay glitches, with a modern
renderer and portable runtime. Gameplay advances at the original nominal rate;
display refresh and graphics settings do not determine game speed. Reproducing
PS1 hardware slowdown is outside the target.

All ten campaign missions and training are playable with both characters,
including gameplay cinematics, progression and save/retry. The original animated
title, character selection, mission map and options are ported from `MENU.EXE`.
The opening/training movies, both character epilogues, credits and final results
from `ENDING.EXE` play from the disc. **F6** opens the previous quick launcher;
**Esc** there returns to the title. **F1** still opens PC settings.

See [PLAN.md](PLAN.md) for deliverables and acceptance criteria and
[AGENTS.md](AGENTS.md) for development conventions.

Game data, captured RAM and disc images must remain outside Git. No BIOS is
required. This is an early PC port with full campaign and training regression
routes, native window checks and source/retail comparisons described below.
Those checks do not establish perfect parity for every possible playthrough.
A browser host is available alongside native play; editable texture/model
packs are supported. Authored HD content remains separate. The shared assets, audio and gameplay crates compile for WASM.

The visual-upgrade work is tracked in [CONFIGURATION.md](CONFIGURATION.md).
For a guided tour, run the modular [feature showcase](DEMOS.md):

```sh
./dev.sh cargo run --release -- demo '/path/to/game.cue'
./dev.sh cargo run --release -- demo '/path/to/game.cue' --clips moon,shadows,motion --loop
```

Its 37 clips stage visual, simulation and inspection comparisons with animated
controls and short captions. Tab opens the clip picker; Space pauses, arrows
change clip, Home restarts, H hides captions and V hides transport controls.
Use `demo --list` for clip IDs or `--clean` for uncaptioned recording. Each clip
resets independently; campaign saves and normal preferences stay unchanged.
Add `--video /path/to/showcase.mp4` for a silent 4K/60 fps offline export with
every frame preserved, embedded chapters and a YouTube description timestamp
file. See [DEMOS.md](DEMOS.md#4k-video-export-and-chapters).

For the optional upgraded look, open **F1 → Graphics → Visual preset**:

- **Original** restores the original presentation settings and is the factory default.
- **Enhanced** combines 4× MSAA, 8× anisotropic filtering, subtle character
  contours, smooth model lighting, lantern/sky shadows, contact shading,
  restrained bloom and blue distance haze.
- **Cinematic** increases anisotropy and point-shadow resolution, with
  stronger bloom/contact shading, haze and a slight vignette. It costs more GPU time.
- **Crisp classic** and **Soft classic** preserve original lighting/geometry, adding
  edge antialiasing, distant texture filtering and smooth motion, with distinct
  close-up texture sampling.
- **Lantern night** emphasizes warm local lights and dark surroundings.
- **Moonlit** adds cool outdoor lighting with gentler color and bloom.
- **Surface relief** adds inferred bump/parallax depth with cubic filtering and lower-angle sky light.

Each effect remains adjustable. Changes show the preset as **Custom**; R on the
preset row restores Original. Presets preserve the replacement-artwork switch
and never change gameplay or simulation settings. Source textures still limit
close-up detail; the editable pack workflow below supplies genuinely new art.

**Graphics → Close-up textures** separates enlargement from distance filtering:
Smooth is bilinear, Crisp preserves pixel interiors with narrow antialiased
transitions, Cubic interpolates curves with bounded Catmull–Rom filtering, and
Pixel uses nearest sampling up close. All four retain the selected mipmaps and
anisotropy in the distance. Enhanced selects Crisp; Cinematic selects Cubic.
These work with the optional Scale2x/4x reconstruction and replacement PNGs.
The Original preset keeps its original nearest sampling.

Both enhanced presets shadow all four active point lights. Ambient brightness
is lower so lamp occlusion remains visible on scenery, models and weapons.
Increasing the light budget beyond the shadow budget allows the extra lights
to shine through geometry; the Lighting page reports that difference. Previously
saved custom settings are preserved: reselect a preset to use the revised balance.

An asset export is available for reviewing and editing the original art:

```sh
./dev.sh cargo run --release -- export-assets '/path/to/game.cue' /tmp/tenchu-art --mission 1
```

Use an empty output directory. `catalog.json` records original prop/model names;
`textures/` contains decoded PNGs with content-based identities, and `models/`
contains textured OBJ geometry in original local units (negative Y is up).
Actor parts are separate files corresponding to the existing animation rig.
PNG alpha 0 means a hole, 128 preserves the original semi-transparency flag,
and 255 is opaque. The original face still decides whether that flag blends.
These are local exports of your disc, not assets bundled with the port.
Edit the PNGs with your preferred image editor, then launch with that folder:

```sh
TENCHU_ASSET_PACK=/tmp/tenchu-art ./dev.sh cargo run --release -- play '/path/to/game.cue'
```

F1 → **Visual assets → Reload asset pack** applies edits without restarting.
Unchanged exported textures are skipped, preserving the live original fallback.
The **Replacement artwork** switch compares against the originals. A failed
reload keeps the previous pack. `pack.json` maps content identities to relative
PNG paths; it can contain only the replacements you want to distribute.
Textures may be RGB/RGBA, grayscale or indexed PNG, up to 1024 pixels per side,
with a pack budget of 16 million decoded pixels. Alpha 0 is transparent,
1–191 selects semi-transparency and 192–255 is opaque; these are game blend
classes rather than continuous opacity. Higher resolutions retain the original
UV mapping. New cutouts also affect model shadows. World sprites support
replacements too; HUD/menu textures stay original, even when they share an image
with a world effect or item.
`render` and replay captures also accept `TENCHU_ASSET_PACK`.
Scrolling world textures retain their original animation with replacement art,
including when the surface first becomes visible after the animation has started.
The source game's texture-copy phase drives the replacement UVs; image data is
not rebuilt or uploaded each animation frame. Wrap filtering also joins opposite
edges. Use a seamless image for these materials.

For material authoring, `preview-materials DISC EMPTY_DIRECTORY ID,ID,...`
captures matching original/replacement views of selected scenery textures.
Use identities from `catalog.json`; the usual mission options apply. It finds
nearby surfaces, checks visibility and keeps inspection cameras in front of
obstructions. Front, angled and wider views share a frozen game state. Default
review settings disable fog and enable filtered mipmaps; `TENCHU_SETTINGS` can
provide a different lighting setup. `TENCHU_ASSET_PACK` selects the replacement
pack. This tool reviews texture/height changes on original scenery geometry.
`--ticks N` advances the game before freezing the view, useful for checking
animated replacements. The context view adds an oblique camera along the surface.

`preview-models DISC EMPTY_DIRECTORY actors/0000,weapons/0000` captures six
original/replacement angles of assembled actor and equipment meshes. Use model
IDs from the export, omitting the final part number for whole actors. Groups such
as `actors`, `weapons` and `common` select all matching models. Actors use their
idle pose after 30 animation updates; gameplay is not advanced. It accepts the
usual mission options and `--outfit normal|armour|alternate`. Review framing
fits each model automatically and preserves translucent face blends. These
isolated views supplement actual stage/gameplay review. Pack views include OBJ
model overrides as well as textures; both versions use a camera that fits their
combined bounds so geometry changes remain directly comparable.

`preview-models` accepts `--background grey` to inspect subtractive shadows and
dark translucent surfaces; black remains the default. For world sprite artwork,
`preview-sprites DISC EMPTY_DIRECTORY 0,6,14 [OPTIONS]` captures the selected
sprite archive indices on black, grey and white. Each image shows five columns:
opaque, average, additive, subtractive and quarter-additive blending. These are
blend diagnostics, not a claim that a sprite uses all five modes in gameplay.
The comparison-page tool also accepts these captures.

Fresh `export-assets` catalogues include each texture's model usage, blend modes,
alpha classes, scrolling status, map positions and matching terrain attributes.
To combine player/layout exports and browse labelled contact sheets, run
`./dev.sh python3 tools/material-catalog.py EXPORT1 EXPORT2 --output EMPTY_DIRECTORY`.
An optional `--recipe RECIPE.json` shows authoring status. The resulting local
HTML catalogue is searchable by model name or texture identity. Terrain flags
and animation are evidence to inspect, not an automatic material classification.
Use `--layout random` when exporting to include all enemy layouts.
`export-assets` also accepts `--outfit normal|armour|alternate`; the alternate
outfit is Ayame's. Exports include the 62 shared sprite images with their archive
indices. The catalogue's sprite category includes both world effects and interface
art; an exported image is not evidence that it is used in the selected mission.
Shared textures retain every outfit label and its per-export face usage when
catalogues are merged. `tools/test-material-catalog.py` checks this with synthetic
outfit fixtures.

The export's `mission_usage` records the loaded actors' weapon kinds and the
stage's environmental placements, including door/pitfall model indices and
environmental sprite indices. Merged catalogues retain these per export under
`missions`. Use this evidence when deciding coverage: the global weapon/common
archives include models unused in this mission, whose textures may not even be
loaded into its VRAM. Loaded archive entries alone are not a mission checklist.

`./dev.sh python3 tools/material-review.py CAPTURES --recipe RECIPE.json`
adds a local `index.html` with a before/after slider. The optional recipe gives
materials readable names. Captures remain private disc-derived files.

`./dev.sh python3 tools/material-pack.py EXPORT RECIPE.json EMPTY_PACK`
packages authored images at 512 pixels on their longest edge by default
(`--size` accepts 16–1024). It retains the original aspect ratio and transparency
classes and checks the runtime pixel budget. Pillow comes from the pinned Nix
development shell. It also checks that authored color covers the original cutout
before restoring its alpha classes. A rejected frame or leaf needs aligned
artwork or extra color bleed behind the mask. Run the synthetic cutout regression
with `./dev.sh python3 tools/test-material-pack.py`.

The default recipe setting `"alpha": "original"` keeps the original silhouette.
For finer foliage or glints, opt into `"alpha": "authored"` to use the new PNG's
cutout instead. Its alpha is thresholded at 128 and mapped to the original blend
class. This requires a single nonzero original alpha class; mixed opaque and
semi-transparent images need their original mask. Review authored silhouettes
in context, including shadows and animation.

For smoke, fire and other additive particles, use `"alpha": "additive"`.
This converts soft authored transparency into RGB intensity over black while
retaining the original semi-transparent blend flag. The runtime's PNG alpha
selects a blend class, not conventional opacity; thresholding a soft glow as
foliage would create a hard, overly bright edge. Use this mode only for sprites
whose game usage is additive or quarter-additive, and review their actual blend.

Recipe paths are relative to the recipe file:

```json
{
  "TEXTURE_ID_FROM_CATALOG": {
    "name": "roof",
    "color": "roof.png",
    "size": 512,
    "height": "roof-height.png",
    "strength": 0.35
  }
}
```

`height` is optional; without it the renderer uses its automatic estimate when
surface depth is enabled. Keep the original exported image and several stage
views as authoring references. Check pattern scale, repeated edges, palette and
lighting in the game; a plausible generated photograph is not necessarily a
usable material. The completed local Mission 1 texture pack lives in
`private/m1-hd-pilot`, with source artwork/prompts in `private/m1-hd-source`.
These local assets are not included in Git.
On the authoring workstation, `./private/m1-hd-pilot/play.sh` launches the pack
with the local disc (or a disc path supplied as its first argument). F1 → Visual
assets → Replacement artwork toggles it. The normal launcher stays unchanged.
The pilot's incorrectly classified masonry candidate was the canal water;
it has been replaced with scrolling ripple artwork. The pack covers 368 scoped
world texture identities: all scenery, props/foliage, both players and all five
outfits, enemies/civilians/animals, equipment/doors and 44 world/item/effect
sprites. It also includes optional replacement meshes for Rikimaru's ninjato
and Ayame's kodachi, adding beveled steel, wrapped grips and fittings within
their original local bounds. Animation, attachment transforms and gameplay
remain original. Other geometry and interface artwork are retained. Ambiguous tiny source details are
authored interpretations, not newly recovered original artwork.

`private/m1-review/index.html` links the scenery, assembled-model, sprite and
full-route comparisons. `private/m1-hd-source/coverage.json` accounts for all
484 loaded archive identities, including 98 unused model textures and 18
interface-only/unused/other-stage sprites excluded from Mission 1 world scope.
The 368 replacements use 15,738,240 decoded pixels, below the 16,777,216 limit.
Both complete Mission 1 controller routes retain identical gameplay traces;
native disable/reload restores identical paused images for both players.
The gable corrections were authored as an assembled attic surface before being
cut into UV pieces, retaining the wooden opening frame. Ayame's shared normal/
alternate face patch now depicts one closed mouth without invented nostrils.
The corrected source artwork, prompts and weapon generator are recorded in
`private/m1-hd-source/corrections-provenance.json`.

**Texture reconstruction** optionally applies
[Scale2x/Scale4x](https://www.scale2x.it/algorithm) to original world textures.
It smooths pixel stair-steps without inventing detail, and can change cutout edges.
It suits some patterns better than photographic textures; compare it in game.
Authored replacements take precedence and are not automatically enlarged.
To replace a model part, edit its OBJ and add its catalogue identity and file
path to the `models` object in `pack.json`, for example:

```json
"models": { "actors/0000/2": "models/actors/0000/2.obj" }
```

Reload from F1. Export triangulated faces, preserve local coordinates and the
part's origin, and retain its material names. The loader accepts OBJ positions,
UVs, normals and optional vertex RGB colors; it uses `pack.json` material
definitions rather than loading MTL files. The exported MTL is for editor
previews. Material `blend` values are 0 opaque, 1 half blend, 2 additive,
3 subtractive and 4 quarter additive; `double_sided` controls back faces.
New materials can reference additional PNG identities declared in `textures`.
UVs address each individual image, with clamp-to-edge sampling.

Character parts and weapons follow the original animation transforms; scenery
uses its original placements. The same edited geometry casts shadows. Collision,
AI and simulation remain original. This is a rigid-part replacement workflow,
not a new skeletal-animation importer. Empty source parts cannot be replaced.

**F1 → Player model** changes the player's body live. The built-in list contains
compatible humanoid models loaded for the current mission. **Original** follows
the game's equipment and disguise changes. A selected override changes only
appearance: the chosen character keeps their animations, weapons, inventory,
hitboxes and behavior. Weapons follow the new body's hands/back, including
draw/sheath transitions; interpolation and dynamic shadows use the same body.
Different proportions can visibly differ from the original collision bounds.

To use bodies from another character/mission or edit your own, export a folder:

```sh
./dev.sh cargo run --release -- export-assets '/path/to/game.cue' ~/tenchu-models --mission 1 --character ayame
```

Enter that folder under **Player model → Model folder**, click **Load / reload
folder**, then choose a body marked “folder”. The folder and selection are saved.
`TENCHU_MODEL_DIR=/path/to/folder` can set the folder at launch. Reload applies
OBJ/PNG edits live; a failed load retains the previous models. Returning to
Original restores the normal body immediately. Unavailable mission-specific
bodies fall back to Original when changing missions.

Exports include a `player_models` object in `pack.json`. Each named body contains
fifteen `parts` with an OBJ `file` (or `null` for an empty joint), a `parent` index
(`null` at the root) and local `position: [x, y, z]`. Keep the exported humanoid
hierarchy and part ordering; edit positions for different proportions. Each OBJ
uses the pack's existing material/PNG definitions. Bodies can be renamed or
removed from this object without changing ordinary world asset replacements.
Arbitrary skeletons and animal rigs need retargeting and are not accepted.

**Smooth model lighting** generates crease-aware normals for dynamic lighting.
**Character contours** optionally rounds original actor silhouettes, with
Subtle/Rounded settings. It uses
[curved PN triangles](https://www.cs.jhu.edu/~misha/ReadingSeminar/Papers/Vlachos00.pdf),
keeps original vertices and open boundaries fixed, and leaves scenery and
authored replacement geometry unrounded. These options start off; finer
geometry costs more to transform and shadow. They can soften the original
shapes but cannot invent new costume or facial detail.

## Build and inspect a disc

Nix supplies the pinned Rust toolchain and dependencies:

```sh
nix develop
cargo run -p tenchu-pc -- inspect '/path/to/game.cue'
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

`./dev.sh cargo ...` enters the same environment for one command. Use `nix build`
for the native package, `nix run -- inspect /path/to/game.cue` to run it, and
`nix flake check` for tests, formatting and a WASM build of the reusable crates.
ISO files and raw BIN data tracks also work. Disc data is read on demand; it is
never an input to the Nix package or copied into the Nix store.

```sh
./dev.sh cargo run --release -- audit '/path/to/game.cue'
./dev.sh cargo run --release -- render '/path/to/game.cue' /tmp/mission1.png
```

`audit` constructs every stage for both characters and samples each motion track
through its duration. `render` captures after 30 ordinary logic ticks, using the
same camera, textures, characters, effects and HUD as the window. It accepts the
same mission/character/layout options as `play`; Mission 1 is the default.
Captures use Original settings unless `TENCHU_SETTINGS` explicitly names a
settings file. That also works with `replay --capture`, allowing repeatable
Original/Enhanced comparisons of the same simulation state.

## Browser build

The browser host uses the same Rust gameplay, renderer and menus through WebGPU
or WebGL2. Automatic selection checks for a usable WebGPU adapter and falls back
to WebGL2, including browsers that expose WebGPU but cannot create an adapter.
Build a static site and prepare server assets from your own raw BIN/CUE disc:

```sh
./build-web.sh '/path/to/game.cue'
./dev.sh python3 tools/serve-web.py private/web
```

Open `http://127.0.0.1:8000` in a browser and click **Start game**
or **Quick launcher**. The first click also enables browser audio. F1 and F6
keep their native roles. The canvas follows the available browser area; use
**Fullscreen** below it to fill the display. Use localhost or HTTPS for browser APIs.
The welcome screen's **Graphics** choice can force **WebGL2 (compatibility)**;
`?renderer=webgl2` also bypasses WebGPU completely, including adapter probes.
Use this if experimental WebGPU support crashes your browser. Firefox 147 on
Linux runs through WebGL2 without enabling `dom.webgpu.enabled`; the reported
Firefox/NVIDIA/Wayland WebGPU crash has not been diagnosed or fixed inside Firefox.
The active backend appears below the game; hover it to see the adapter name.
WebGL2 supports the visual presets, lighting/shadows and surface depth, but does
not offer multisample edge anti-aliasing or GPU timestamp measurements.
Original uses a small shader variant with disabled effects compiled out. The
browser retains unchanged frames, including when paused, and reuses unchanged
overlay meshes. The FPS counter counts frames actually rendered. Original's
animation changes at 30 Hz; **F1 → Graphics → Smooth model motion** enables
interpolated poses and camera motion at the display rate, without changing logic.
The game audio output also avoids repeated hardware-latency queries that could
hang Firefox's main thread; audio scheduling and mixing remain unchanged.

On this GTX 1080 Ti/NVIDIA 580/Wayland desktop, Firefox 147 release still renders
about 24 FPS at 3840×2124. Its `about:support` lists `DMABUF_WEBGL` as blocked;
copying the canvas into the browser compositor dominates the frame time. The
earlier 42 FPS result was headless and overstates normal-window performance.
Firefox Nightly 160 in a separate profile reached 60 FPS at the same resolution
and at 3840×2160 fullscreen, including camera movement, with smooth model motion
enabled. Mozilla's
[NVIDIA WebGL sharing change](https://bugzilla.mozilla.org/show_bug.cgi?id=1981326)
enables this path on Nightly for newer drivers; it does not change release builds.
The port does not override browser blocklists or reduce resolution automatically.
Use a 60 Hz display for that comparison; this desktop also has 30 Hz outputs.
These are Mission 1 measurements, not a guarantee for every preset or device.

To try Nightly through Nix without changing your usual Firefox profile:

```sh
mkdir -p private/firefox-nightly
nix run github:nix-community/flake-firefox-nightly#firefox-nightly-bin -- \
  --no-remote --profile "$PWD/private/firefox-nightly" \
  'http://127.0.0.1:8000/?renderer=webgl2'
```

The complete host is also built by `nix build .#web`; it contains no game data.
`tenchu-pc prepare-web DISC DIRECTORY` prepares that site's `data/` directory.

The boot set contains the original frontend and initial scene. Remaining game
pages and audio for the selected language are downloaded in the background.
Starting a mission waits for those assets while keeping you in the menu; a
running mission performs no HTTP reads for game state/assets. Movies are also
cached before mission entry, including both endings and credits. Browser audio/movie exports are compressed for transfer;
native playback continues to decode the original disc. Keep generated `data/`
private unless you have permission to distribute its copyrighted contents.

Campaign saves and settings are local to this browser and site address. Use
**Export save** / **Import save** to back them up or transfer them. Browser storage
is separate from desktop saves. Closing the tab does not erase saved progress;
clearing site data does. F9 downloads a PNG. Input recordings, offline 4K exports
and local filesystem model/texture directories remain native tools; the browser
uses built-in models and settings. Browser gamepad input uses the Gamepad API;
controller vibration and physical-controller behavior have not been verified.

Validated in Chromium 150 with WebGPU/Vulkan: missions 1 and 5, both characters,
movement, F1, resize, opening playback/pause/skip, persistent saves, PNG downloads,
continued play after disconnecting the network, surface-depth modes at an unchanged
simulation tick, fullscreen transitions and exit/restart. This is a browser host check,
not a new full campaign playthrough. Run `./dev.sh node tools/test-web.mjs` against
the local server to repeat it (a display and working WebGPU adapter are needed).
Set `TENCHU_WEBGL=1` to exercise automatic fallback in Chromium. Run
`./dev.sh node tools/test-web-firefox.mjs` for Firefox, using an isolated temporary
profile and the explicit WebGL2 path. `FIREFOX_EXECUTABLE` can select an installed
Firefox binary. These tests check rendered screenshots as well as gameplay state;
the Firefox check verifies that explicit WebGL2 never probes WebGPU.
`TENCHU_WEB_BENCH=1` compares small-window/4K stationary play and moving-camera
views, reporting actual rendered frames, browser callbacks, simulation ticks and
CPU callback time separately. `TENCHU_WEB_INTERPOLATION=1` enables smooth motion
in the isolated test profile. `TENCHU_WEB_FULLSCREEN=1` measures only the current
display's fullscreen canvas instead of requesting two window sizes. Run it on a 60 Hz
display without other GPU workloads and inspect the actual reported canvas size;
tiling window managers may ignore requested sizes. Headless results omit the
normal desktop presentation path and cannot establish its performance.
Set `TENCHU_WEB_URL` to use another address. For deployment, serve the generated
site over HTTPS with `Cross-Origin-Opener-Policy: same-origin` and
`Cross-Origin-Embedder-Policy: require-corp`, as the local server does.

## Play

```sh
./dev.sh cargo run --release -- play '/path/to/game.cue'
./dev.sh cargo run --release -- play '/path/to/game.cue' --quick --character ayame --mission 1 --layout 1
```

Normal launch opens the original movie/title flow. Enter skips the opening,
then reveals the title choices; select **Begin mission**, choose a character and
use Up/Down for the mission and Left/Right for its layout. The map follows each
character's saved progress. **F6** opens the unrestricted development launcher
from any frontend screen, including movies. **`--quick`** starts there immediately.
Launch character/mission/layout arguments prefill that quick launcher.

Mission numbers follow campaign order (1–10); `--mission training` selects
training. Each mission has layouts 1–3; `--layout random` selects one at the
source construction boundary. `--difficulty easy|normal|hard` selects the source
AI difficulty; otherwise the saved choice is used, or Normal for a new campaign.
The original **Options → Language** and quick launcher's **Language** row select
English, French, Italian or Japanese dialogue and subtitles. All four remain
available immediately. Character/map/results artwork follows that choice;
the original release's options artwork is Japanese, with clear English
descriptions and controls below it. The choice is saved with campaign progress;
`--language english|french|italian|japanese` overrides the saved preference at launch.
Japanese subtitles use a bundled
[replacement font](crates/tenchu-assets/data/japanese-font/README.md); no BIOS is
needed. Latin subtitles retain the original disc font.

Original options include records, difficulty, stereo/mono, music/effects volume,
sound test, four controller layouts, vibration, load/save and automatic saving.
Load/save uses the existing native campaign file. Turning automatic saving off
does not disable explicit Save. Controller vibration uses the original gameplay
cues on devices supported by the host; motor timing follows the driver's
resolution; physical motor output still needs a hands-on controller check.
Original cheat-button sequences are not implemented; F6 provides unrestricted
mission/layout selection and all four languages are available immediately.
The separate **TRIAL.EXE mission editor** is not ported; its title entry explains
this and returns to the menu. Equipment and intermediate mission-results
screens retain the existing PC presentation.

Movies require raw Mode 2 BIN/CUE data and the Nix-provided FFmpeg decoder.
Cooked ISO images can still run gameplay; an unavailable movie shows a message
and Enter continues. Movies run at their authored 15 fps independently of the
display; menu animation uses 60 Hz and the mission selector 30 Hz. Opening,
training and credits can be skipped with Enter/Esc; the character epilogues
follow the original unskippable rule, with F6 still available for development.

The frontend regression checks load all four languages, exercise both character
animations and ending transitions, and decode all five movies to their source
endpoints with XA audio and caption/voice cues. They also check playback at
5/15/60 Hz display cadence and cancellation of movie workers:

```sh
TENCHU_DISC='/path/to/game.cue' ./dev.sh cargo test --release -p tenchu-pc frontend -- --include-ignored
```

| Action | Keyboard | Controller |
| --- | --- | --- |
| Move and turn | WASD / arrows | D-pad / left stick |
| Jump | Space | South (Cross / A) |
| Attack / draw weapon | J | West (Square / X) |
| Crouch | Shift | East (Circle / B), or R1 / RB |
| Use item | E | North (Triangle / Y) |
| Look / aim | Hold Q + directions | Hold L1 / LB + directions |
| Previous / next item | Z / C | L2 / R2 (LT / RT) |
| Map | Hold M / Tab | Hold Select / Back |
| Pause / resume | P | Start / Menu |

In menus, use the arrows, D-pad or stick to choose and change values; Enter or
the south face button confirms, Escape or east goes back, and R or west retries
from results. Original menus also accept mouse clicks; F1 captures interaction
while its settings overlay is open. The table above describes controller layout
1; changing the original controller layout remaps gameplay buttons, including
their keyboard equivalents. Host menu/navigation shortcuts keep their usual roles.
In the equipment screen, Up/Down selects an item and Left/Right
removes/adds one. Enter/south starts the mission; the hook is carried automatically.
In gameplay, F5 or Escape abandons the attempt and opens retry equipment. On a
controller, pause first, then use east/west to abandon/retry. Escape/east from
the quick launcher returns to the title; Escape on the title quits. Controller hints use physical face-button positions because
labels differ between controller brands.

With your weapon drawn, back away (S / Down) to guard frontal melee attacks.
Quick keyboard and controller taps are retained until the next gameplay tick.

**F1** opens a compact settings overlay from any screen. Click categories,
checkboxes and dropdowns; drag sliders or double-click numbers to type.
Drag the title bar to move the panel, resize its width, and scroll long pages.
Arrows/D-pad and Enter/South also work; F1/Escape/East closes it. The overlay
pauses the game by default. **General → Pause while settings are open** can keep
the simulation running while you tune settings; menu input stays out of gameplay,
and F3/focus-loss pauses still apply. It provides FPS
and tick display, pause on defocus, borderless fullscreen, VSync, HUD visibility
and master volume. Factory defaults preserve the preceding experience.
**General → Performance charts** opens live CPU/GPU stage plots that remain
visible after closing F1. Reopen F1 to move/resize the panel, freeze history or
clear it. The charts show the last 300 frames, average and p95 milliseconds,
simulation/audio, geometry/uploads, overlay work, CPU submission, presentation
wait, and separate GPU shadow/world/image-effect/HUD times. GPU results are
asynchronous and optional when the adapter lacks timestamp queries; they exclude
the egui overlay. CPU and GPU overlap, so their totals should not be added.
Photo mode hides the charts along with the other inspection overlays.

Preferences save separately from progress in
`$XDG_CONFIG_HOME/tenchu-shinobi-gaisen-pc/settings.json` (normally
`~/.config/tenchu-shinobi-gaisen-pc/settings.json` on Linux). `TENCHU_SETTINGS`
overrides the file. Disabling pause on defocus keeps simulation/audio running
but still releases controls; input from another application never controls the game.
Tab/Shift+Tab, Page Down/Page Up or controller R1/L1 changes settings pages.
Hover a row or select it with the arrows, then press R to reset only that value;
or right-click a control for its reset action. One-shot actions such as teleport
cannot be reset or triggered with R. Graphics
settings include black fog and its distance, textures, baked vertex shading,
exposure, nearest/bilinear/trilinear filtering, 2×–16× anisotropic filtering and
supported MSAA modes. MSAA smooths polygon edges. Mipmaps reduce distant texture
shimmer; anisotropic filtering retains detail on angled floors and roofs.
Each decoded texture has isolated mip levels and separate opaque/semitransparent
colors, so filtering does not borrow from neighboring textures or blend modes.
Animated VRAM textures remain live. HUD and menu textures remain sharp.

Actor/light labels, FPS and inspection readouts use a clear, resolution-independent
overlay font. Inspection pages include live body/attack hitboxes, sight bounds, masked terrain
surfaces, patrol routes, spatial event regions, actor labels/state, X-ray overlays
and a recent player trail. These tools start disabled each launch. Sight bounds
show calm-state ranges and angles, not walls/occlusion or a guarantee of detection.
Terrain overlays show ACM floors/slopes rather than inferred wall meshes.

**F2** toggles free camera. Hold right mouse to look, WASD to fly, Q/E down/up,
and Shift for a speed boost. Left-click selects an actor. F1's Camera page adds
focus, overhead view, FOV, speed and a mission-local bookmark. Free camera
suppresses player input; **F3** pauses/resumes while keeping the scene visible,
and **F4** runs exactly one logic tick. The gameplay camera still runs internally.
Some effects already projected by that camera (wire lines, weapon trails and
sprites) are omitted from the inspection view.

The Teleport page accepts exact X/Y/Z values (Enter to edit, Delete to clear,
Enter to apply), floor snapping or a click on a terrain surface. Negative Y is
up. Teleporting and changing the enemy sight-distance multiplier are explicit
gameplay edits: they end an active recording before the edit, leaving a replayable
prefix. A new attempt can record again with original sight distance. Sight
distance does not change hearing, actor activation, attack range or occlusion.

**Simulation → Keep all enemies active** disables distance/budget suspension for
spawned enemies and civilians, so distant patrols, AI and animation keep running. It is off
by default and saves as a preference across mission changes. Story-controlled
actors still follow their cutscenes and spawn rules. This changes gameplay/RNG;
it ends an active recording, and new recordings require original enemy settings.
Turning it off restores the normal activation policy at its next periodic check;
it does not undo movement or events that have already happened.

Hold **Backspace** during a cutscene to fast-forward at maximum bounded throughput.
The Simulation page offers automatic 2×/4×/8×/16×/Max cutscene speed and general
25–400% or Unlimited simulation speed. Unlimited runs across gameplay and
cutscenes, muting audio and yielding periodically for input/rendering. Every
logic tick runs; cutscene-only accelerated backlog is discarded
when control returns. Fast-forward mutes sound and resumes background music at
ordinary speed; releasing it mid-dialogue resumes speech at the next cue.
**F9** saves the current view as PNG under `screenshots/`, or `TENCHU_SCREENSHOTS`.
**F10** toggles a clean photo view, hiding HUD, captions, FPS and inspection
helpers while preserving world effects. Combine it with F2/F3 for a paused
free-camera shot; F1 remains available. It does not alter saved HUD preferences.

The Lighting page adds optional dynamic point lights and model/weapon shadows.
In a mission, select **Preview model shadows** to enable a light above/beside the
player, lower ambient brightness and hide the original blob shadows. Close F1
and move or attack to inspect the animated silhouette. This changes presentation
preferences; General → Restore defaults restores the original look.
For manual setup, enable **Dynamic lighting**. Authored fires and detected lanterns
supply lights; a movable light can also be placed at the camera or above the
player. The movable light lasts for the session and can follow free camera or
move along X/Y/Z. Radius, intensity, ambient brightness, shadow filtering and
256–2048 shadow resolution are adjustable.
Lighting starts off; changing it never affects AI, visibility, RNG or logic timing.

Lighting uses linear color, front-facing diffuse response and controlled highlights.
Lantern light fills directional shadows naturally; moon strength is calibrated
below local lamps. Emissive panels replace their surface lighting instead of
adding another full bright layer. Original baked light painted into textures
remains; replacement artwork can remove it for a fully relit look.

This is a new visual treatment over the original baked colors.
**Point-light budget** allows 0/1/2/4/8/16 simultaneous sources (default 2).
Nearest sources fill the budget; the movable light takes priority.
**Shadowed point lights** independently limits shadow rendering to the nearest
0–4 lights. Unshadowed lights are cheaper but can shine through walls.
The menu shows active lights, shadow views and depth-map memory. Each shadowed
point light costs six extra scene passes and about 6 MiB at 512 or 96 MiB at 2048.
Maps are allocated only for active shadowed lights; disabling shadows releases
them. Filtering radius 0 retains bilinear depth comparisons; 1–8 adds nine-tap
filtering. This softens edges, rather than simulating a full area light.

**Lantern / prop lights** recognizes names such as AKARI, TORO and ANDON from
the disc, then samples bright side surfaces to locate/color the emitter.
Placements and carried-weapon transforms supply world positions. Bright panels
glow and transmit light; their darker frames and solid caps still cast shadows.
Emission is scoped to those models, so shared wall textures and snowy roofs do
not become light sources. This is a
reusable heuristic, not recovered original lighting data. **Light source
markers** labels active sources; **Inspect nearest lantern** focuses free camera.

Exported `pack.json` includes editable `lights` rules. An omitted list uses the
built-in rules; `[]` disables detection. The last matching rule wins. Match with
case-insensitive `name_contains`, or an exact catalogue `model` identity. Rules
accept `enabled`, `threshold` (0.05–1 brightness), `emission` (0–10),
`radius_scale` (0.1–4), optional RGB `color` and a local-space `offset`.
`emit_caps: true` permits emission from upward/downward surfaces for custom
fixtures; the default excludes them.
Defaults derive color and position from the original mesh/texture; offsets let
replacement authors adjust them. Reload through Visual assets. No mission
coordinates are embedded in the renderer.

Actual live actors, held/sheathed weapons, scenery, doors and trapdoors cast
shadows even outside the gameplay camera. Other rendered world item meshes also
cast. Original texture holes and translucent pixels remain transparent to
shadows, including mixed opaque/translucent textured faces. Particles do not cast.
**Original blob shadows** controls the original ground decals independently.
Lighting already baked into textures/vertex colors remains.

F1 → **Sky lighting** adds a directional sun/moon source and optional cool ambient
tint. Its strength starts at zero. Enable Dynamic lighting and raise Sun / moon
strength, then choose neutral/cool/warm color, compass angle and elevation.
Its single shadow map has independent 512–4096 resolution and a 10k–60k coverage
radius around the view. The projection moves in texel increments to reduce
shimmer, with a gradual fade at its boundary. Wider coverage reduces detail;
2048 is a useful starting point. The shared shadow switch and filtering radius
also apply to sky shadows. A point-light budget of zero leaves just sky/ambient
lighting and glowing panels.

F1 → **Image effects** enables HDR finishing with bloom, contact shading,
filmic tone mapping, contrast, saturation, warm/cool balance, shadow brightness,
vignette and sharpening. Sky lighting also offers distance-fog colors and
atmospheric haze, which require Image effects. HUD/menu rendering bypasses
these effects. Contact shading uses visible depth, so off-screen objects cannot
contribute; turn its strength to zero to reduce GPU cost. The filmic curve
retains bright-light detail but is an optional artistic treatment of the original
baked colors. All these effects start disabled in Original.

**F1 → Graphics → Surface depth** offers **Off**, **Bump** and **Parallax**,
with a separate **Depth strength** slider. **Surface relief** is a ready-made
preset. All existing presets, including Original, keep depth off. Bump changes
how grain and seams catch dynamic/sky light; Parallax also shifts the texture
with the viewing angle. Lower strength reduces distortion. Close-up texture
filtering remains an independent choice (Crisp preserves pixels; Cubic smooths).

The port derives a softened height estimate from each opaque scenery texture,
removing broad painted brightness before generating mip-filtered gradients.
There are no mission/texture-name rules or new meshes. Cutout foliage,
translucent faces, emissive panels, UI and automatic character materials are
excluded. Detail fades at distance, region edges and grazing views; parallax
uses at most eight steps. Collision, silhouettes, gameplay and cast-shadow
geometry are unchanged. Small crevice shading is an approximation, not geometric
self-shadowing. Painted art does not uniquely describe height: a dark stain can
still look recessed, and this cannot invent authentic HD detail.

For precise results, add a `surfaces` section to a native asset pack's `pack.json`,
using texture identities from `export-assets`:

```json
"surfaces": {
  "TEXTURE_ID_FOR_ROOF": {"height": "heightmaps/roof.png", "strength": 1.0},
  "TEXTURE_ID_TO_LEAVE_FLAT": {"strength": 0.0}
}
```

Height PNGs use the red channel: black is low, white is high. Their resolution
can differ from the color texture (up to 1024×1024); strength is 0–2 and multiplies
the F1 slider. Omitting `height` uses the inferred map. Explicit maps may also
target model materials. Reload the pack through F1; no model or color-texture edit
is required. Browser play currently uses the automatic maps; filesystem asset
packs remain a native feature.

**F1 → Graphics → Smooth model motion** interpolates character parts, held
weapons, hinged doors/pitfalls and their dynamic shadows between consecutive
30 Hz updates. The gameplay camera follows the same timing, so movement can
look smooth at 60 Hz and higher. Original leaves this off; Enhanced/Cinematic
enable it. It adds up to one tick (33 ms at normal speed) of presentation delay.
Collision, hitboxes, AI, RNG and animation logic still use the exact current
game state. Pause/frame-step show that exact pose; teleports, new actors, mesh
changes and mission reloads do not blend from unrelated poses. Scripted camera
cuts remain exact. Particle sprites, trails, projectiles and original blob
shadows retain their authored 30 Hz presentation.

The export/edit/reload workflow above supports higher-resolution textures and
replacement models without changing collision or logic. Automatic processing
cannot supply bespoke new detail or remove lighting baked into the art.
Broader presentation upgrades are tracked in `CONFIGURATION.md`.

The first controller used is selected until disconnected. Keyboard and controller
can be used together. Disconnecting the active controller pauses gameplay.
Held controls must be released after menu or focus changes. Losing focus freezes
simulation and audio. Controller mappings use [gilrs](https://docs.rs/gilrs/0.11.2/gilrs/)
and accept its `SDL_GAMECONTROLLERCONFIG` environment override.

The game supports movement,
jumping, crouching, wall hugging/sliding, ledge control, swimming, melee combos, guard/damage reactions, stealth
kills, dropped inventory, pickups, enemy arrows/gunshots/shuriken, item damage
and the source follow/look camera. Grappling, shuriken, medicine, caltrops,
smoke bombs, grenades, mines, poisoned food, colored rice, sleeping gas, fire breath and the lure flute work through normal item selection.
Planted rice remains marked on the map and can be picked up again. The eight-shot
shuriken item uses its original spread, delayed hitboxes and independent trails.
Speed scrolls retain the original cast, steering, terrain checks and cancellation.
Protection charms apply their original melee damage modifiers for 450 logic ticks.
Decoy dolls grow, draw nearby enemies and break under attack; only three can be active.
Disguise changes the original body meshes and motion scale, with source alert,
expiry and ledge-grab restrictions.
Ninja dogs summon, select and attack enemies, then expire after 1,800 logic ticks.
Mei-Oh's four lightning attacks retain repeated targeting, damage and original bolt jitter.
Resurrection leaves restore health after a lethal hit. Damage floors and Manji flame attacks share the original napalm behavior. Mission 1 now runs its authored encounter scripts, camera cuts and subtitles,
then returns to gameplay. Results, stock rewards, retry and progression connect
to the original frontend and optional quick launcher. Unexpected or unsupported data pauses with a diagnostic. Caltrops,
grenades, smoke bombs and poisoned food also use the original corner-throw behavior.
Held and sheathed weapons use their original attachment rules. Original shadows,
weapon trails, blood and implemented particle effects are displayed with their
texture palettes and transparency modes. The HUD shows inventory selection and
counts, player/enemy health and the proximity indicator using original graphics.
The equipment screen shows both limits: 15 selected items and five item types.
The automatic hook uses one type, leaving four for selected equipment.
The launcher uses the source initial stock: choose equipment with Up/Down and
Left/Right, then Enter to start. The hook is always carried. Tap E for a quick
shuriken throw or hold it to aim,
then release to throw; J, Space or Shift cancels the held item. With the hook
selected, hold E, aim with the direction keys and release to grapple. Doors, pitfalls, animated water, fire emitters, snow and ambient sound now run
from the original construction data. Scene faces now use back-face culling and their original transparency flags and
blend modes, sorted together with particles. Meshes retain their baked vertex
colors and fade into black using the original camera-depth fog curve. The GPU
evaluates that curve smoothly per pixel; original per-polygon cue quantization
and distance/sector visibility are not yet reproduced. HUD, sprites and shadow
meshes retain their separate drawing rules.

Original sound effects, character voices, music and localized cinematic dialogue
play from the disc. Music/dialogue require a raw Mode 2 BIN/CUE; cooked 2048-byte
ISO images omit the XA payload. The mixer uses the original banks, pitch,
envelopes and spatial sound rules with modern resampling. It does not emulate
SPU reverb or CD seek timing. Pausing or losing focus freezes audio and gameplay.
An unavailable audio device is reported without preventing play. Linux uses
PulseAudio/PipeWire's PulseAudio service when available, with ALSA as fallback.
Set `TENCHU_AUDIO_TRACE=1` to log music/dialogue requests and decoded clips.

The quick launcher lets you choose character, difficulty, mission and layout. Results
show the original score/rank, award items and update each character's records.
Enter advances (or retries training), R replays, and Escape returns to the menu.
After Mission 10, the selected character's epilogue and credits precede the
original final-results screen. Enter returns to the title; R/F5 retries.
Failed or abandoned attempts lose their carried items. Training keeps its separate
ranking table and fixed equipment. Armour uses its original model and damage rule.
Training completion is checked for both
[Rikimaru](crates/tenchu-pc/tests/data/training-rikimaru.inputs) and
[Ayame](crates/tenchu-pc/tests/data/training-ayame.inputs), including all seven
opponents, the instructor's closing sequence, saved ranking and retry with
the fixed hook and five shuriken. These routes finish with 70 and 90 HP.

Progress saves between attempts to
`$XDG_DATA_HOME/tenchu-shinobi-gaisen-pc/progress.sav`, or
`~/.local/share/tenchu-shinobi-gaisen-pc/progress.sav` on Linux. macOS uses
`~/Library/Application Support`; Windows uses `%APPDATA%`. Set `TENCHU_SAVE` to
choose a different file. Saves retain both characters' stock/records, mission,
layout, language and difficulty. They are checked and replaced atomically;
The quick launcher's Continue resumes at equipment selection, not mid-mission. Closing during an
attempt retains the preceding checkpoint. Existing PS1 memory cards are not
imported. Initial gameplay uses seed 1, then subsequent attempts continue from
the preceding gameplay/reward RNG. Menu waiting and display refresh consume no
gameplay RNG; title palette variation has a separate cosmetic generator. PS1
startup VSync seeding is intentionally a host policy, not a hardware model.

Add `--record DIRECTORY` when launching to record normal play:

```sh
./dev.sh cargo run --release -- play '/path/to/game.cue' --record ./recordings
./dev.sh cargo run --release -- replay '/path/to/game.cue' ./recordings/mission1-EXAMPLE.inputs
```

The terminal prints a unique filename for each mission attempt. Recordings store
launch settings, starting RNG, equipment and inputs for each logic tick. Pause
and menu time add no inputs; retry and subsequent missions start new files.
Files contain no game assets or captured RAM. Replay supplies the recorded
settings automatically, including random-layout selection and armour; extra
launch options are rejected. Use the same port revision for deterministic
comparisons.

`replay DISC INPUT.txt [OPTIONS]` runs without a window and writes a CSV trace,
including health, game clock, cinematic activity and mission outcome. Add
`--capture OUTPUT.png` to render its final state through the ordinary renderer;
renderer messages go to stderr so stdout remains CSV.
Older input-only tapes use the diagnostic fallback seed 1 and loadout
(hook, 10 shuriken, 5 caltrops, 2 medicine), with launch options from the command line.
Each input line contains a decimal tick count and hexadecimal PS1 button mask;
`#` starts a comment. For example, `30 1000` holds forward for one second and
`1 1040` presses jump while holding forward. The replay and window use the same
scalar gameplay code.

Saved-campaign regressions complete all ten missions for both characters,
including the final bosses and results. Stock and RNG carry
through real encoded/decoded saves at every boundary. The recordings include
their starting seed and equipment, so
`replay` can run them directly.

| Character | Mission | Results tick | Health |
| --- | --- | ---: | ---: |
| Rikimaru | [1](crates/tenchu-pc/tests/data/mission1-rikimaru.inputs) | 5,667 | 64 |
| Ayame | [1](crates/tenchu-pc/tests/data/mission1-ayame.inputs) | 4,834 | 94 |
| Rikimaru | [3](crates/tenchu-pc/tests/data/mission3-rikimaru-campaign.inputs) | 5,952 | 7 |
| Ayame | [3](crates/tenchu-pc/tests/data/mission3-ayame-campaign.inputs) | 5,883 | 46 |
| Rikimaru | [4](crates/tenchu-pc/tests/data/mission4-rikimaru-campaign.inputs) | 9,128 | 57 |
| Ayame | [4](crates/tenchu-pc/tests/data/mission4-ayame-campaign.inputs) | 12,062 | 24 |
| Rikimaru | [5](crates/tenchu-pc/tests/data/mission5-rikimaru-campaign.inputs) | 8,034 | 100 |
| Ayame | [5](crates/tenchu-pc/tests/data/mission5-ayame-campaign.inputs) | 7,919 | 46 |
| Rikimaru | [6](crates/tenchu-pc/tests/data/mission6-rikimaru-campaign.inputs) | 6,139 | 50 |
| Ayame | [6](crates/tenchu-pc/tests/data/mission6-ayame-campaign.inputs) | 6,393 | 5 |
| Rikimaru | [7](crates/tenchu-pc/tests/data/mission7-rikimaru-campaign.inputs) | 3,744 | 100 |
| Ayame | [7](crates/tenchu-pc/tests/data/mission7-ayame-campaign.inputs) | 2,546 | 100 |
| Rikimaru | [8](crates/tenchu-pc/tests/data/mission8-rikimaru-campaign.inputs) | 8,448 | 74 |
| Ayame | [8](crates/tenchu-pc/tests/data/mission8-ayame-campaign.inputs) | 8,126 | 56 |
| Rikimaru | [9](crates/tenchu-pc/tests/data/mission9-rikimaru-campaign.inputs) | 6,292 | 3 |
| Ayame | [9](crates/tenchu-pc/tests/data/mission9-ayame-campaign.inputs) | 5,741 | 67 |
| Rikimaru | [10](crates/tenchu-pc/tests/data/mission10-rikimaru-campaign.inputs) | 15,082 | 30 |
| Ayame | [10](crates/tenchu-pc/tests/data/mission10-ayame-campaign.inputs) | 16,015 | 7 |

Both complete Mission 2 with 100 HP. The independent
[Mission 7 route](crates/tenchu-pc/tests/data/mission7-rikimaru.inputs)
also passes with the corrected swimming camera, finishing at tick 3,461 with
93 HP. [PLAN.md](PLAN.md) records the feature coverage, native play checks and
source/retail comparisons. These routes exercise ordinary controller inputs;
they do not prove exhaustive parity with retail.

```sh
TENCHU_DISC='/path/to/game.cue' \
  ./dev.sh cargo test --release -p tenchu-pc controller_route -- --ignored
```

Native window checks cover movement, combat/items, pause/resize, language
selection and death/retry. Arbitrary inputs and every difficulty/layout
combination have not been exhaustively compared with retail.

Optional GPU checks cover source blending/fog, filtering/MSAA, light occlusion
and actual character/weapon shadows. Run these sequentially to avoid concurrent
graphics-device initialization:

```sh
TENCHU_DISC='/path/to/game.cue' \
  ./dev.sh cargo test --release -p tenchu-pc renderer::tests -- --ignored --test-threads=1
```

The private-disc cadence test exercises both characters and compares player,
animation, effects, items, RNG, camera, collision, coordinate caches and activation at every tick
under 24 Hz and 144 Hz host frames:

```sh
TENCHU_DISC='/path/to/game.cue' \
  ./dev.sh cargo test -p tenchu-pc retail_movement -- --ignored
TENCHU_DISC='/path/to/game.cue' \
  ./dev.sh cargo test -p tenchu-pc all_retail_layouts -- --ignored
```

This checks display independence. A separate private comparison checks seven
original Castle-start recordings for 4,630 consecutive updates, including
swimming, grappling, climbing and enemy attacks that reduce health to 73:

```sh
TENCHU_DISC='/path/to/game.cue' \
TENCHU_CASTLE_CORPUS='/path/to/tenchu-castle-approach-validation-20260925' \
  ./dev.sh cargo test --release -p tenchu-core castle_start_matches_retail -- --ignored
```

It constructs the world from disc assets, supplies the recorded launch seed and
loadout, and aligns the initial player pose and ambient-sound deadlines once.
The original loading sequence left those deadlines at clock 644; the PC launcher
starts them at zero. All subsequent actor, camera, RNG, collision and drawing state advances
in the port without corrections from the recordings. This is finite coverage,
not a claim of full-campaign parity.

Terrain collision probes retain the original diagonal sample positions and mask
order. A private comparison runs the original C probe with the executable's
direction table and recorded terrain responses; an ordinary-input regression
checks wall pressure and retreat for both characters.

Every mission/layout constructs the source actor roster, collision slots,
weapons and patrol data. The ordered actor loop and AI controllers are implemented;
NPC activation runs at the source cadence. Activation workload counts use
original geometry, independent of PC rendering. The seven continuous recordings
check exact drawing counts as well as gameplay. Empty
cable curves omit the hook mesh: the original reads undefined stack coordinates
in that case, which this port does not reproduce.
An additional private check compares the roster, mission totals, weapon hitbox
indices and layout patrol points with the existing Reclaim the Castle capture:

```sh
TENCHU_DISC='/path/to/game.cue' TENCHU_ROSTER_TRACE='/path/to/kagi-aim-v1.trace' \
  ./dev.sh cargo test -p tenchu-pc roster_and_weapon -- --ignored
```

Optional core checks compare camera smoothing, enemy sight and combat decisions
with matching C, including 4,096 activation worlds, and compare the integer view
matrix and construction geometry with an existing retail trace. The
combat check covers 81,920 engaged-controller cases, including RNG and command
history. A further 8,192 human/item cases compare the original damage routine and collision
cleanup, with supplied terrain/motion/effect boundaries. Six controlled stealth
pairs exercise both characters through death and return to control. These checks
do not yet validate the entire AI/terrain/update pipeline. These private files remain outside
the repository and are never build or runtime dependencies:

```sh
TENCHU_RETAIL_EXE='/path/to/main.exe' TENCHU_DECOMP='/path/to/tenchu-decomp' \
TENCHU_CAMERA_TRACE='/path/to/kagi-aim-v1.trace' \
TENCHU_SCENE_TRACE='/path/to/kagi-aim-v1.trace' \
TENCHU_GTE_ORACLE='/path/to/standalone-gte-oracle' \
TENCHU_SDK_ROTATION_ORACLE='/path/to/tenchu-sim/native/sdk_rotation_oracle.py' \
TENCHU_DISC='/path/to/game.cue' \
  ./dev.sh cargo test -p tenchu-core -- --ignored
```

The optional projection check uses the standalone arithmetic oracle described in
`~/programming/tenchu-decomp-glitch-research/docs/tas-native-packets.md`. That
external test executable is not linked into this port or required to build it.

Projectile checks cover 2,048 complete flight curves against original C and
2,048 targeting rotations against the retail SDK. Controlled ordinary-tick
scenarios check bow, pistol, rifle and ninja attacks through launch, damage,
attachment and disposal. The SDK test invokes the research Python oracle above;
it is optional and has no role in the runtime.

The ignored `profile_mission_rendering` renderer check provides a repeatable
Mission 5 camera sweep, with 1×/4× MSAA and contact shading on/off. Run it with
`TENCHU_DISC` set and `./dev.sh cargo test --release -p tenchu-pc
profile_mission_rendering -- --ignored --nocapture` (one shell command).
Optional `TENCHU_PROFILE_SIZE=2560x1440` and `TENCHU_PROFILE_FAR=1` select a larger,
elevated view with fog disabled. It measures GPU work offscreen, independently
of VSync/window-system presentation waits.
