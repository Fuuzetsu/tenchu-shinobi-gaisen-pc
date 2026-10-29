# Detailed guide

Start with the [quick-start README](../README.md). Commands below run from the
repository root. Fresh installs use **Modern Low**; option defaults described
in individual feature notes refer to **Original**. See
[quality presets and automatic tuning](performance.md) for current defaults.

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
**F7** opens the [randomizer](randomizer.md), with full-disc actor pools,
stage/item blacklists and shareable seeds. Runs keep separate progress.

See [AGENTS.md](../AGENTS.md) for development conventions.

Game data, captured RAM and disc images must remain outside Git. No BIOS is
required. This is an early PC port with full campaign and training regression
routes, native window checks and source/retail comparisons described below.
Those checks do not establish perfect parity for every possible playthrough.
A browser host is available alongside native play; editable texture/model
packs are supported. Authored HD content remains separate. The shared assets, audio and gameplay crates compile for WASM.

The visual-upgrade work is tracked in [CONFIGURATION.md](../CONFIGURATION.md).
For a guided tour, run the modular [feature showcase](../DEMOS.md):

```sh
./dev.sh cargo run --release -- demo '/path/to/game.cue'
./dev.sh cargo run --release -- demo '/path/to/game.cue' --clips moon,shadows,motion --loop
```

Its clips stage visual, simulation and inspection comparisons with animated
controls and short captions. Tab opens the clip picker; Space pauses, arrows
change clip, Home restarts, H hides captions and V hides transport controls.
Use `demo --list` for clip IDs or `--clean` for uncaptioned recording. Each clip
resets independently; campaign saves and normal preferences stay unchanged.
Add `--video /path/to/showcase.mp4` for a silent 4K/60 fps offline export with
every frame preserved, embedded chapters and a YouTube description timestamp
file. See [DEMOS.md](../DEMOS.md#4k-video-export-and-chapters).

**F1 → General → Gameplay preset** groups independent gameplay choices: Original,
Living world (all guards active), Keen guards (125% sight), Relaxed stealth (75%
sight), and Practice (god mode, never noticed, click teleport). The middle three
also accelerate cutscenes to 4x. Inventory and graphics are kept. Practice requires
a living player in a mission; cheats remain session-only. Sight range and guard
activation are saved preferences.

**F8**, or **F1 → Graphics → Retro comparison**, temporarily shows original
artwork, geometry, text and effects. Press it again to restore your selected
visual settings. Gameplay settings and simulation are unaffected. Modern High
and Modern Ultra enable the integrated visual improvements with 2x/4x MSAA,
respectively, and remove black sight fog and round blob shadows. They preserve
your weather choice and artwork-pack preference. Unsupported MSAA falls back to
the adapter's supported settings.

Point-light shadows now support up to eight sources. High/Ultra select six/eight,
with gradual shadow-budget handovers as you travel. **Lighting → Light source
size** controls finite-emitter softness (0 restores point-source shadows). Paper
lantern panels diffuse their own light; caps and surrounding objects still cast
shadows. Daylight reduces lantern fill on sunlit surfaces while sheltered lamps
remain useful. This approximates area lighting with shadow maps.

**Model refinement → Vegetation volume** now includes Botanical and Botanical
dense. These add folded leaf sprays and round bamboo stalks using existing
materials. **Varied tree shapes** gives repeated placements different crowns and
proportions while holding their roots fixed. Breeze bends trunks gently and
moves branches/leaves separately; matching shadows follow the deformation.
Autumn trees, snowy bushes and bamboo are included alongside cherries and pines.

For the optional upgraded look, open **F1 → Graphics → Visual preset**:

- **Original** restores the original presentation settings. New installs use Modern Low.
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

**F1 → Visual assets → High-definition text** draws HUD numbers, in-game
subtitles, quick launcher/equipment/pause panels and separate original-menu
labels with a portable outline font at the window's resolution. English,
Japanese, French and Italian are supported; the original options screens keep
their Japanese wording. It is off in Original and enabled in the modern quality tiers. It does not change text timing,
navigation or gameplay. It includes all ten mission titles and the five ranks.
Lettering baked into backgrounds needs replacement artwork; movie subtitles
retain their original presentation.
The [font and license](../crates/tenchu-pc/data/hd-font/README.md)
are bundled for both desktop and browser builds.

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
A `recipes` map can instead reconstruct selected textures from the supplied disc
and generated PNG layers at load/reload time. This runs in Rust, also in the
browser; it needs no Python, AI service or per-frame processing. Do not include
an identity in both `textures` and `recipes`. For example:

```json
"recipes": {
  "source-texture-id": {
    "steps": [
      {"op": "opaque"},
      {"op": "resize", "size": [128, 128], "filter": "bicubic"},
      {"op": "grain", "image": "layers/detail.png", "smooth": "layers/smooth.png", "gain": 0.7},
      {"op": "alpha", "mask": {}}
    ]
  }
}
```

The default base and mask source is `"original"`, meaning the decoded disc
texture with that identity; a base can also be `{"file": "generated.png"}`.
Masks can use `original`, `current` or a file, select `alpha`, `red` or `luma`,
and apply optional 256-entry `before`/`after` lookup tables around resampling.
`intermediate_size` preserves a deliberately staged mask resize. Filters are
`nearest` (default for masks), `bilinear`, `bicubic` and `lanczos`.
Other operations are `overlay` (RGBA composition), `blend` (weighted RGB),
`mask_blend` (two RGB layers), `matte` (flatten RGB while retaining alpha), and
`clear_alpha` (a rectangular cutout). Layers must match the current image size.
Recipes allow at most 16 operations and obey the same image/path limits as PNGs.
Only their final images enter the mission's decoded-image budget; temporary
layers are freed. Browser preparation exports and prefetches all recipe inputs.
The bundled pack's manifest contains working examples.

Textures may be RGB/RGBA, grayscale or indexed PNG, up to 1024 pixels per side,
with a budget of 16 million decoded pixels per loaded mission/outfit (including
height maps and replacement model textures). A campaign pack can contain more:
the loader selects current mission textures and model dependencies before
reading images, and refreshes that selection on mission/outfit changes.
Unrelated files consume no decoded-image memory. Alpha 0 is transparent,
1–191 selects semi-transparency and 192–255 is opaque; these are game blend
classes rather than continuous opacity. Higher resolutions retain the original
UV mapping. New cutouts also affect model shadows. World and HUD sprites support
explicit replacements too. Generic texture reconstruction still preserves the
original HUD, and HUD artwork bypasses world lighting and grading.
Original frontend images use the same content-identity
override format, with their own active-image budget. Only the current language's
menu images and character-selection textures are loaded. Replacements render
directly at window resolution, retaining original sprite blending and layout;
filtering stays inside each atlas row or region. F1 reload refreshes them too.
`render` and replay captures also accept `TENCHU_ASSET_PACK`.
`export-interface DISC DIRECTORY` exports title, character/mission selection,
options, results and shared HUD/font images, including all four language
variants. Its catalogue records each source archive/index and deduplicates
identical images; Latin font advances and nudges are included for text authoring.
Use these identities in `pack.json` to supply frontend replacements. Exporting
the unchanged images alone retains the originals.
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

For campaign artwork, merge all mission/outfit exports with `material-catalog.py`
and pass that catalogue to `material-pack.py --campaign`. The tool checks the
pixel budget for every exported mission/outfit, counting shared artwork in each
one that uses it. Keep all variants in the catalogue; an incomplete catalogue
cannot establish complete coverage or a campaign-wide memory bound.

The default recipe setting `"alpha": "original"` keeps the original silhouette.
For finer foliage or glints, opt into `"alpha": "authored"` to use the new PNG's
cutout instead. Its alpha is thresholded at 128 and mapped to the original blend
class. This requires a single nonzero original alpha class. Review authored silhouettes
in context, including shadows and animation.

For deliberately painted masks mixing opaque and semi-transparent pixels, use
`"alpha": "classes"`. The authored PNG must contain exactly the original set of
alpha values from 0, 128 and 255. These are blend flags, not opacity: the tool
resizes the class mask with nearest-neighbor sampling and the RGB separately.
Ordinary partially transparent artwork should use the other modes. This allows
sharp opaque lettering over a blended HUD glow without changing either blend.

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

The campaign pack is a separate **`tenchu-hd.tar.zst`** download in the
**artwork-v1** release. Place it beside the executable/launcher or in your working
directory. For editing, unpack it into `assets/hd/` or set `TENCHU_ASSET_PACK`
to another directory. The game caches archive extraction by content hash.
It covers 1,719 texture identities across Missions 1–10, character/outfit families,
equipment, world effects, HUD and menus, plus two authored weapon meshes.
Plain `play DISC` discovers it automatically. F1 → **Visual assets → Replacement
artwork** controls its use; Modern presets enable it. F1 reports the active
folder, loaded textures/models and any load error. Existing preferences are kept.
F6 opens the quick launcher; F8 compares original graphics.

The pack ships generated artwork and 301 Rust recipes. Mixed source colour,
cutout masks and snow coverage are reconstructed from your disc at asset load,
without Python or online generation. See [pack provenance/editing](../artwork/README.md).
To use a custom pack, set `TENCHU_ASSET_PACK` to its directory or `.tar.zst` file.
An explicitly empty value disables loading. With the download in place:

```sh
./dev.sh cargo run --release -- play '/path/to/game.cue'
```

Lantern emission, lighting and material shaders are separate settings, so they
can still improve original artwork with the replacement switch off.

Campaign references, prompts, assembled tile families, recipes and review
evidence live in `private/campaign-hd-source`. Its `coverage.json` records the
accepted pack and 22 intentional world fallbacks: invisible weapon faces and
ambiguous original UV spans. Original geometry, UV scale/mirroring and vertex
lighting can still expose some joins. Movies remain original. Menu/HUD artwork
and fonts are covered in all four supported languages; original options retain
their Japanese wording. The native and browser builds use the same pack format.

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
The subsequent map-boundary review revises 20 ground/path/eaves textures and
enables full-period UV mapping for two repeating roofs. Compare original,
previous and revised artwork in 100 matched views at
`private/m1-boundaries/index.html`; `provenance.json` alongside it records the
accepted atlases, exact prompts and crops. Original geometry and baked shading
still account for some visible joins. Restart the pack launcher to pick up the
renderer change; game logic and collision are unchanged.

The campaign showcase is
`private/campaign-showcase/tenchu-campaign-hd-showcase-4k60.mp4`: **11:42,
3840×2160, 60 fps, silent**, covering all ten missions, characters/bosses,
weapons, items, HUD and menus/fonts in four languages. Its 95 matched
comparisons last three seconds per side, with short gameplay sections between
them. The directory includes a watch page, 13 embedded/YouTube chapters,
reproducible capture inputs and frame/encoding verification. See
[the recording notes](../DEMOS.md#campaign-artwork-recording).

The earlier Mission 1 showcase is
`private/m1-showcase-v2/tenchu-mission1-showcase-v2-4k60.mp4`: 26 comparisons in
2:36 at 3840×2160 and 60 fps. Each side lasts three seconds, compared with ten
previously, while gameplay runs at normal speed. It includes the boundary
refinements and additional scenery, interiors, characters and props. The watch
page, frozen capture pack, reproducible catalog and grouped YouTube chapter
description are beside the video.

The earlier four-minute silent comparison is
`private/m1-before-after-video/tenchu-mission1-before-after-4k60.mp4`.
It contains 12 matched before/after pairs at 3840×2160, 60 fps, covering scenery,
both players and item use. The review hub links the video and its YouTube
description/chapters. The catalog, input excerpts and frame-timing validation
are stored beside it; generated footage remains outside Git.
This video predates the boundary refinement; its exact capture pack is archived
at `private/m1-boundaries/previous-pack`.

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
58 character, boss, animal and alternate-outfit resources from across the disc.
Search by name or resource ID, then click a tile. Only visible tiles load and
animate their idle previews; these previews use original artwork. The active
body uses your selected artwork/refinement settings and remains selected across
missions. **Original** follows the game's equipment and disguise changes.
By default an override changes only appearance: the protagonist keeps their
animations, weapons, inventory, hitboxes and behavior. Weapons follow the new body's hands/back, including
draw/sheath transitions; interpolation and dynamic shadows use the same body.
Different proportions can visibly differ from the original collision bounds;
animal bodies can look unusual with humanoid movements.

**Use model's moveset** also adopts the selected actor's registered animations,
attacks and weapon profile. For example, Onikage uses his kicks while keeping
the normal player controls. Missing player actions fall back to the protagonist,
so grappling/items remain available. This changes gameplay and resets the current
action; campaign identity, health and inventory stay with Ayame or Rikimaru.
Original and custom folders use the protagonist's moves. Enemy AI routines are
not additional player commands. Disable the checkbox to restore normal moves.

Two optional checkboxes customize that moveset: **Use player movement** restores
your protagonist's locomotion clips and speeds (walking, running, dashes,
crouching, jumping, climbing, swimming and grappling) while retaining the selected
actor's attacks. **Disable idle taunts** prevents random fidgets and combat taunts
from locking player input. Enabling it also ends a taunt already in progress.
Both are off by default, apply only with **Use model's moveset**, and are saved
across launches. They leave enemies and scripted animations unchanged.

To edit your own bodies, export a folder:

```sh
./dev.sh cargo run --release -- export-assets '/path/to/game.cue' ~/tenchu-models --mission 1 --character ayame
```

Enter that folder under **Player model → Model folder**, click **Load / reload
folder**, then choose a body marked “folder”. The folder and selection are saved.
`TENCHU_MODEL_DIR=/path/to/folder` can set the folder at launch. Reload applies
OBJ/PNG edits live; a failed load retains the previous models. Returning to
Original restores the normal body immediately. Missing custom bodies fall back
to Original.

Exports include a `player_models` object in `pack.json`. Each named body contains
fifteen `parts` with an OBJ `file` (or `null` for an empty joint), a `parent` index
(`null` at the root) and local `position: [x, y, z]`. Keep the exported humanoid
hierarchy and part ordering; edit positions for different proportions. Each OBJ
uses the pack's existing material/PNG definitions. Bodies can be renamed or
removed from this object without changing ordinary world asset replacements.
Custom folders require that humanoid hierarchy; the built-in animal choices use
the disc's own skeletons.

**Smooth model lighting** generates crease-aware normals for dynamic lighting.
**Character contours** optionally rounds original actor silhouettes, with
Subtle/Rounded settings. It uses
[curved PN triangles](https://www.cs.jhu.edu/~misha/ReadingSeminar/Papers/Vlachos00.pdf),
keeps original vertices and open boundaries fixed, and leaves scenery and
authored replacement geometry unrounded. These options start off; finer
geometry costs more to transform and shadow. They can soften the original
shapes but cannot invent new costume or facial detail.

## Contact effects and remake presets

F1 → **Contact effects** controls detailed grapple rope/hook geometry, impact
sparks and light, water splashes, footstep/landing scatter, and falling cherry
petals/autumn leaves. Splashes can leave small temporary wet marks on nearby
land. Fallen foliage settles on terrain, fades away, and scatters underfoot;
snow and puddles respond more strongly to running and landing than walking.
These effects use bounded cosmetic state and do not consume gameplay RNG.

**Modern High** and **Modern Ultra** enable these effects alongside botanical
foliage, varied tree shapes and anchored branch/leaf wind motion. Original stays
unchanged. F8 temporarily compares your current visual setup with Original;
press it again to restore it. Gameplay profiles in F1 remain independent.

The built-in `remake-*` demos cover this batch in short, separate clips. For example:

```sh
./dev.sh target/release/tenchu-pc demo '/path/to/game.cue' --clips remake-snow-tracks,remake-area-lamps,remake-petal-fall --mute
```

Use `demo --list` for all clip IDs. Modern High/Ultra are demanding settings;
WebGPU or the native renderer is preferable to WebGL2 for their multiple shadowed
lights and denser foliage. Shadow views cull distant geometry, and WebGL2 uses a
readable depth attachment for the same soft-light/weather calculations.

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
`nix flake check` for workspace tests, strict Clippy, formatting, the native
package and browser builds, plus Windows cross-compilation on x86-64 Linux.
Checks need no disc; retail and GPU regressions remain opt-in because they
require private game data or graphics hardware.
ISO files and raw BIN data tracks also work. Disc data is read on demand; it is
never an input to the Nix package or copied into the Nix store.

Starting without arguments, or using `play` without a disc path, reopens the
last verified disc when it is still available. Otherwise it opens a native
file picker. An explicitly supplied path always takes priority, including errors;
it never silently opens a different disc. Successful selections are saved as an
absolute path in `last-disc.json` beside settings (`TENCHU_LAST_DISC` overrides
that preference-file location). Delete that file to forget the selection.
Windows also accepts dragging a CUE onto the executable. Keep its
referenced BIN files beside it. Linux uses a desktop file-picker portal or
Zenity; the Nix package includes Zenity. If neither is available in a portable
Linux installation, pass the CUE path explicitly.

Native play verifies `MAIN.EXE`, `MENU.EXE`, `ENDING.EXE` and the entire
`DATA.VOL` against the original SLPS-019.01 SHA-256 hashes before starting.
`tenchu-pc verify DISC` runs the same check without opening a game window.
Browser asset preparation verifies the source disc once; the browser then
checks downloaded page hashes while streaming. These checks cover base game
logic/artwork, not a bit-for-bit checksum of the audio/movie tracks or optional
replacement packs. Custom artwork belongs in packs rather than patched discs.

On x86-64 Linux, `nix build .#windows` builds `result/bin/tenchu-pc.exe` with the
same pinned Rust compiler. Copy the executable to Windows and run it from
PowerShell:

```powershell
.\tenchu-pc.exe play 'C:\Games\Tenchu\game.cue'
```

Keep the CUE and its referenced BIN files together. Opening/ending movies and
video export also need a Windows `ffmpeg.exe` on `PATH`, or an explicit
`$env:TENCHU_FFMPEG = 'C:\Tools\ffmpeg\bin\ffmpeg.exe'`. FFmpeg links Windows
builds from its [download page](https://ffmpeg.org/download.html). `--quick`
opens the development launcher directly. Saves/settings use the existing
`TENCHU_SAVE`/`TENCHU_SETTINGS` overrides or `%APPDATA%\tenchu-shinobi-gaisen-pc`.

The Linux Windows-test shell includes Wine and the cross-linker:

```sh
WINEPREFIX="$PWD/private/windows-wine" nix develop .#windows --command cargo test --workspace
nix develop .#windows --command cargo clippy --workspace --all-targets -- -D warnings
```

Linux and Wine runs cover live gameplay, Vulkan rendering, audio, preferences,
saving and clean shutdown. ARM64 Linux unit tests run under QEMU. Wine/QEMU
coverage does not validate physical Windows drivers or ARM graphics hardware.
Apple Silicon macOS has Nix package/check definitions but still needs a real
Mac build and runtime test. The pinned Nixpkgs has removed Intel macOS support,
so this flake no longer advertises an unusable `x86_64-darwin` output.

### Automated downloads

[Build and release](../.github/workflows/build.yml) runs on pushes to `master` and
`main`, version tags, pull requests and **Run workflow** in GitHub Actions.
**Releases** contains versioned downloads and **HEAD (development)**, a rolling
prerelease of the latest successful default-branch build. These downloads include
`SHA256SUMS` and do not expire after 14 days. Per-run Actions artifacts remain
available for 14 days for diagnostics. See [releasing](releases.md).

| Artifact | Contents |
| --- | --- |
| `linux-x86_64` | Linux launcher, executable, loader and shared libraries |
| `linux-aarch64` | The same bundle built/tested on a native ARM64 Linux runner |
| `windows-x86_64` | Windows executable and dependency/font notices |
| `wasm32` | Browser website, WASM, JavaScript bindings and Python server |
| `dependency-sources` | Optional dependency sources, patches and pinned build recipes |

Extract Linux archives with `tar -xzf`, keep the whole directory together and
run `./tenchu-pc play /path/to/game.cue`. Nix is **not required** to run these
bundles. They use the installed GPU drivers; FFmpeg remains an external
dependency on both Linux and Windows. Browser downloads need a `data/` directory
prepared from your own disc with `tenchu-pc prepare-web`; see the included
`README.txt`. Archives include build provenance and never include disc assets.

The workflow runs the native package's unit tests on both Linux architectures,
checks formatting/Clippy and the portable WASM crates, and tests Linux startup
and dynamically loaded libraries with `/nix` hidden. A Windows runner starts
the downloaded executable. These checks do not exercise GPU gameplay on hosted
runners, which have no game disc.

The same downloads can be built locally with `nix build .#linux-archive`,
`.#windows-archive` or `.#web-archive`; files appear in `result/`.
The ARM64 archive uses `packages.aarch64-linux.linux-archive` on an ARM64 builder.

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

A downloaded HD pack is included automatically by `./build-web.sh`. Set
`TENCHU_ASSET_PACK` to override it (an empty value disables it). Existing
disc/audio/movie exports are reused; artwork is refreshed. For example:

```sh
./build-web.sh '/path/to/game.cue' private/web-hd
```

`prepare-web DISC DIRECTORY --artwork` refreshes only the configured pack in
an existing `data/` directory. Only manifest-referenced files are exported.
The menu artwork for all four languages arrives with the boot set. The remaining
PNG/model files download in the background before mission entry; only the active
mission's images are decoded. F1 → **Visual assets → Replacement artwork** and
**High-definition text** work in the browser as on desktop. The modern tiers
enable outline text and use the replacement-artwork preference (enabled by default). Browser pack reload uses the cached export; rebuild the site and reload
the page to pick up edited files.

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
and live local filesystem model/texture directories remain native tools; browser
replacements come from the site's exported pack. Browser gamepad input uses the Gamepad API;
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
[replacement font](../crates/tenchu-assets/data/japanese-font/README.md); no BIOS is
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

**F1 → General → Controls** remaps the keyboard and controller, including both
sticks, shoulder buttons, triggers and Pause. Select a primary or alternate
binding and press the new input. Assigning an input moves it from its previous
action; × clears it. Each device has a reset-to-defaults button. Changes save
automatically to `controls.json` beside settings (override with `TENCHU_CONTROLS`),
or to browser local storage. Graphics/gameplay presets do not change bindings.
The action labels describe original layout 1; selecting another original
controller layout still changes the meaning of those virtual PS1 buttons.
Escape cancels capture; Escape, Enter and F1–F12 retain menu/tool shortcuts,
and Backspace retains cutscene fast-forward. Bindings use physical key positions.
Free-camera/tool navigation keeps its separate shortcuts.

**F1 → Vibration** adjusts global intensity and each of the 17 gameplay cues
independently: swings, clashes, successful/blocked hits, blocking, melee and
item/trap damage, animal/target hits, body slams, stealth kills and their
uneven-ground interruption, jump collisions, normal/heavy/flip landings, ledge
grabs and entering water. Each slider spans 0–200%; 100% preserves the existing
strength, 0 disables it. Global and action strengths multiply, scaling both
motors up to their hardware maximum without changing timing or gameplay.
Changes save with settings and are independent of graphics/gameplay presets.
R or right-click resets a slider. The enable switch is shared with the original
menu; a force-feedback-capable controller and host driver are required.

**F1** opens a compact settings overlay from any screen. Click categories,
checkboxes and dropdowns; drag sliders or double-click numbers to type.
Drag the title bar to move the panel, resize its width, and scroll long pages.
Arrows/D-pad and Enter/South also work; F1/Escape/East closes it. The overlay
pauses the game by default. **General → Pause while settings are open** can keep
the simulation running while you tune settings; menu input stays out of gameplay,
and F3/focus-loss pauses still apply. It provides FPS
and tick display, pause on defocus, borderless fullscreen, VSync, HUD visibility
and master volume. Existing saved preferences are kept; new installs use Modern Low.
**General → Performance charts** opens live CPU/GPU stage plots that remain
visible after closing F1. Reopen F1 to move/resize the panel, freeze history or
clear it. The charts show the last 300 frames, average and p95 milliseconds,
simulation/audio, geometry/uploads, overlay work, CPU submission, presentation
wait, and separate GPU shadow/world/image-effect/HUD times. GPU results are
asynchronous and optional when the adapter lacks timestamp queries; they exclude
the egui overlay. CPU and GPU overlap, so their totals should not be added.
Photo mode hides the charts along with the other inspection overlays.

**F1 → Experiment** provides mission-only tools: god mode (health is preserved,
with normal hit reactions), never-noticed stealth, and click-to-teleport. Close
F1 and left-click a collision floor to teleport repeatedly; Escape cancels the
mode. It works with the normal camera or the free camera. Item sliders edit
carried quantities, including 0 to remove and 255 for unlimited; Give all
supplies ten of each and Clear empties the equipment. Armour protection can be
toggled independently of the outfit selected at mission load. Internal item
timers and campaign stock/unlocks are preserved. These edits end input recording;
a new mission restores ordinary gameplay. Cutscenes retain their scripted behavior.

**F1 → Persistent effects → Persistent blood** keeps cosmetic wound stains on
animated bodies and projects drops onto scenery, doors and trapdoors. Amount,
bleeding duration and retained-stain limit are adjustable; **Clear blood** removes
the cosmetic marks and drips. This is off by default. Damage, health, the original
blood effects and gameplay RNG are unchanged. Marks retain texture detail/cutouts
and follow interpolated poses and compatible replacement models. Wound placement
is inferred from original blood and damage events, rather than adding hit-location
gameplay. Stains are held in memory, bounded by both count and geometry; changing
mission, reloading models/artwork or disabling the feature clears them. They are
not written into campaign saves.
`demo --clips blood` stages an ordinary guard encounter to show the effect.

**F1 → Time of day → Sun / moon cycle** replaces the manual sky light with a
continuous day/night cycle, sky gradient and moving shadows. Set the start time
with the 24-hour slider (or type `12:30`), and use **Time lapse** for game minutes
per second; 0 holds that time. Sun/moon intensity, day/night ambient light and
path orientation are independent controls. The original 30 Hz simulation drives
the cosmetic clock, with interpolated lighting between ticks; pauses freeze it.
The clock is separate from mission scripts and the AI's lighting/detection rules.
The cycle is off by default. `demo --clips day-cycle` shows a day in twenty seconds.
**Daylight relighting** fades the original baked night shading on textured
scenery as daylight rises, so midday can look like daytime while preserving the
artwork. Set it to 0 to keep all original vertex shading throughout the cycle.

**F1 → Weather** selects Original, Clear, Rain or Snow. Original retains the
mission's weather; overrides change presentation only. Precipitation, wind,
detail, particle distance, wetness, snow cover and roof runoff are adjustable.
Wind carries particles under eaves, while walls shelter areas downwind. **Wind
compass** is the direction precipitation travels toward. Both rain and snow use
that direction for falling particles and surface exposure: windward walls get
wet or lightly dusted, leeward walls stay clear, and calm vertical precipitation
leaves vertical walls dry. Wind edits redirect airborne particles and recheck
their collisions without resetting accumulation. Rain splashes, runs down sloped
surfaces and drips over edges. Wetness builds gradually and dries in clear weather;
snow coats exposed upward surfaces more heavily and slowly melts. Coverage follows
the current wind and shelter; it does not retain separate weather histories for
individual surfaces.
Near and whole-stage shelter maps respect scenery cutouts. Moving actors cast
lighting shadows without erasing snow beneath them. Fog and particle distance do
not limit accumulated ground coverage. Model/artwork changes preserve accumulation.

**Clear weather now** removes accumulation, tracks, runoff and airborne particles
immediately and selects Clear. Select Rain or Snow again to restart precipitation.
**Snow drift softness** broadens shelter silhouettes in world units to approximate
varying wind, independently of weather detail. Settled snow forms a continuous
layer; grain is filtered at distance instead of becoming large square patches.

Snow footprints follow players' and humanoid enemies' animated feet and project
onto actual ground triangles. Fresh snowfall buries exposed tracks gradually;
turning the wind updates which tracks receive snow. Projected tracks use compact
geometry, and older tracks fade under budget pressure instead of disappearing.
F1 controls the burial time
and provides a clear action. Ground blood stains absorb the snow
material. Tracks, particles and decals are bounded cosmetic effects; collision
is unchanged and there is no fluid/flood simulation. Low/Medium/High weather
caps precipitation at 384/896/1792 particles, shared between near and distant
layers, with 256/512/1024 shelter maps. Pauses freeze weather; changing missions
or selecting Original clears it. Original weather RNG still updates when hidden.
`demo --clips rain,snow` reviews both effects on Mission 1's roofs.

Rain has additional controls on that same page:

- **Wet reflections** (0–300%) controls sky and shadowed sun/moon/lamp highlights.
  Lamp highlights need dynamic lighting. These are wet-surface highlights rather
  than mirror images of buildings or characters.
- **Puddle coverage** adds shallow reflective patches with animated raindrop
  ripples on nearly level, exposed surfaces. Their pattern uses world coordinates,
  so it continues across texture tiles. Coverage approximates small irregularities;
  it does not solve terrain drainage or fill real depressions with fluid.
- **Pooling speed** (0.25–10x) changes wetness and puddle build-up. At full rain,
  1x wets surfaces in about 20 seconds and builds puddles over a minute; 10x takes
  roughly 2 and 7 seconds. Drying is gradual and independent of that multiplier.
- **Rain haze** obscures distant views independently of black fog, with less haze
  along sheltered portions of the view. It does not change enemy detection.
- **Rain streak visibility** changes drop/splash visibility without increasing
  the particle count. **Roof drip amount** controls rivulets and recurring eave
  drips; runoff has its own bounded budget and still works with streak visibility
  set to zero. Roof catchment is inferred from exposed sloped scene triangles.

For a heavy downpour, try precipitation 100%, wetness 100%, reflections 180%,
puddles 75%, pooling 5x, haze 60%, streaks 130% and roof drips 100%.
The `rain`, `rain-reflections` and `rain-haze` demo clips show these separately:

```sh
./dev.sh cargo run --release -- demo '/path/to/game.cue' \
  --clips rain,rain-reflections,rain-haze
```

With dynamic lighting enabled, **F1 → Lighting → Fire / explosion lights** also
lights grenades, kaen and their burning embers. Flashes fade with the real effects,
cluster nearby particles, and share the existing light/shadow budgets.

**F1 → Water** adds optional moving ripple normals, angle-dependent sky sheen,
sun/moon highlights, faint foam and waterfall mist. It works with original or
replacement artwork. Flow, reflections, ripples, foam and spray have independent
controls; pauses freeze animation. These are sky reflections, not mirrors of
nearby objects. The waterfall detail choices retain original geometry or add
curved render patches with pinned attachment edges and at most 250 world units
of displacement. Collision, swimming and original texture animation are unchanged.
Fine curves cost more triangles; generated waterfall geometry and spray are capped.
`demo --clips water,waterfalls` compares the Mission 1 pond and Mission 4 falls.

Water classification uses reviewed original texture fingerprints in
`crates/tenchu-pc/data/water.json`, not a guess based on animation or color. Mixed
snow/rock/bank tiles and lava are deliberately excluded. An asset pack can override
this with a `"water"` map in `pack.json`, mapping texture IDs to `"surface"`,
`"waterfall"` or `"off"`. The metadata follows model material dependencies when
exported for the browser. Curves preserve UVs and existing texture boundaries;
this is not a fluid simulation or a replacement for a fully authored river mesh.

**F1 → Model refinement** offers Refined/Fine character surfaces and
Fuller/Dense/Botanical/Botanical dense vegetation. Character subdivision welds coincident positions across texture seams,
retains separate UVs, pins open/nonmanifold joints and limits contour changes per
pass. It replaces the older contour option while enabled; authored replacement
characters and weapon meshes remain as supplied. Existing animation, attachments,
collision and AI visibility are unchanged. Original is the default for both options.

Vegetation adds angled layers to connected cutout clusters and refines selected
trunks and dense shrubs, reusing the original or installed HD textures. Connected material tiles
share their pivot so branch/leaf textures remain joined. Explicit fingerprints
in `data/vegetation.json` cover reviewed cherry, pine, cedar, shrubs, grass and
snow-covered branches, autumn trees and bamboo. Botanical levels add folded leaf
sprays and rounded bamboo stalks; the lower levels retain the original stalk cards.
**Tree variation** gives repeated placements stable crown shapes and proportions.
Pack authors can provide a `"vegetation"` map from texture IDs to `"leaves"`,
`"blossoms"`, `"autumn"`, `"bamboo"`, `"grass"`, `"shrub"`, `"trunk"` or `"off"`;
these rules also travel with browser exports.
Refinement and foliage have independent geometry budgets and retain the original
mesh when a budget is exhausted. The new botanical geometry reuses the current
artwork; character refinement remains a bounded change to the supplied mesh.
`demo --clips vegetation,character-refinement` shows the choices.

**F1 → Secondary motion** independently enables hair, loose cloth and carried
equipment flex, with strength and spring stiffness controls. Reviewed regions of
Rikimaru and Ayame react to their original animations; attachment regions stay
fixed. Equipped weapon transforms, hitboxes and gameplay remain unchanged. These
are bounded spring deformations of existing surfaces, without strand simulation,
cloth collision or anatomical deformation. Original is the default: all three
switches are off. Changing bodies, teleporting or reloading resets the springs.
Attached blood stains and shadow geometry follow the same deformed surfaces.

**Breeze** keeps roots anchored, gives trunks restrained motion and adds separate
branch gusts and leaf flutter in both visible geometry and shadows. It also adds
a small force to enabled hair/cloth/equipment motion. It is independent of the
precipitation wind controls. Cosmetic motion advances with the fixed game clock,
interpolates between ticks and freezes when paused; rendering more frames does
not advance physics. `demo --clips secondary-motion,equipment-motion,breeze`
shows both characters and the Mission 1 garden.

The reviewed player regions live in `crates/tenchu-pc/data/motion.json`. Packs may
provide a `"motion"` map in `pack.json`, keyed by model-part ID such as
`"actors/0001/2"` or `"player/my-body/2"`. Each part supports up to four rules:
`kind` (`"hair"`, `"cloth"`, `"equipment"`), a unit `axis`, `start`/`end` coordinates
along it, a local `anchor`, a bounded `limit`, optional `absolute` axis distance,
and optional texture-ID `materials`. The influence ramps smoothly from fixed to
flexible across that interval; `limit` cannot exceed one quarter of its length.
Replacement geometry needs its own explicit rules; original-body masks are not
silently applied to custom meshes. Metadata is included in browser exports.

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
**Shadowed point lights** independently limits shadow rendering to 0–8 lights.
Stable slots and gradual handovers avoid abrupt nearby activation. Unshadowed
lights are cheaper but can shine through walls.
The menu shows active lights, shadow views and depth-map memory. Each shadowed
point light costs six extra scene passes and about 6 MiB at 512 or 96 MiB at 2048.
WebGL2 keeps an additional readable depth copy for soft-light filtering.
Maps are allocated only for active shadowed lights; disabling shadows releases
them. Each view culls geometry outside its frustum. **Source size** controls
blocker-dependent penumbrae: finite lanterns use a wider soft filter, while zero
retains point-source filtering. Nearby emitting paper panels diffuse their own
light instead of projecting their painted grating as hard shadows. Solid caps and
other objects still occlude it. Sun-exposed surfaces suppress local-light contrast
at midday; covered areas retain useful lantern fill. This approximates a diffuse
finite source without a ray-traced lighting simulation.

**Lantern / prop lights** recognizes names such as AKARI, TORO and ANDON from
the disc, then samples bright side surfaces to locate/color the emitter.
Placements and carried-weapon transforms supply world positions. Bright panels
glow and diffuse their own light; solid caps and other objects still cast shadows.
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
preset. Original keeps depth off; Modern High uses Bump and Modern Ultra uses
Parallax. Bump changes how grain and seams catch dynamic/sky light; Parallax also
shifts the texture with the viewing angle. Lower strength reduces distortion. Close-up texture
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
  "TEXTURE_ID_FOR_ROOF": {"height": "heightmaps/roof.png", "strength": 1.0, "uv_tile": true},
  "TEXTURE_ID_TO_LEAVE_FLAT": {"strength": 0.0},
  "TEXTURE_ID_FOR_WATER": {"strength": 0.0, "world_tile": [4000, 4000]}
}
```

Height PNGs use the red channel: black is low, white is high. Their resolution
can differ from the color texture (up to 1024×1024); strength is 0–2 and multiplies
the F1 slider. Omitting `height` uses the inferred map. Explicit maps may also
target model materials. Reload the pack through F1; no model or color-texture edit
is required. Browser play currently uses the automatic maps; filesystem asset
packs remain a native feature.

`world_tile` optionally maps replacement artwork across world X/Z coordinates,
with the two values giving its repeat periods in game units. It joins terrain
or water patterns across independently rotated/cropped mesh UVs. Use the same
image and periods for adjoining materials. Original texture scrolling still
advances at its original rate; color filtering, cutouts and cast shadows share
the mapping. This requires a replacement image and disables with the artwork;
it does not change the mesh, original UVs, collision or simulation. Use ordinary
UVs for characters and materials whose pattern must follow an object.

`uv_tile: true` keeps the original UV orientation but fits a complete repeating
replacement image between the original texel-center endpoints. It avoids losing
one original texel's worth of artwork at each join, useful for seamless roof or
ground tiles. Filtering wraps across the image edges, including mipmaps. Leave
it off for atlas pieces, framed panels and textures with a deliberate outer edge.
`world_tile` takes precedence when both are supplied. Both options require actual
replacement artwork; toggling it off restores the original mapping exactly.

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
[Rikimaru](../crates/tenchu-pc/tests/data/training-rikimaru.inputs) and
[Ayame](../crates/tenchu-pc/tests/data/training-ayame.inputs), including all seven
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
| Rikimaru | [1](../crates/tenchu-pc/tests/data/mission1-rikimaru.inputs) | 5,667 | 64 |
| Ayame | [1](../crates/tenchu-pc/tests/data/mission1-ayame.inputs) | 4,834 | 94 |
| Rikimaru | [3](../crates/tenchu-pc/tests/data/mission3-rikimaru-campaign.inputs) | 5,952 | 7 |
| Ayame | [3](../crates/tenchu-pc/tests/data/mission3-ayame-campaign.inputs) | 5,883 | 46 |
| Rikimaru | [4](../crates/tenchu-pc/tests/data/mission4-rikimaru-campaign.inputs) | 9,128 | 57 |
| Ayame | [4](../crates/tenchu-pc/tests/data/mission4-ayame-campaign.inputs) | 12,062 | 24 |
| Rikimaru | [5](../crates/tenchu-pc/tests/data/mission5-rikimaru-campaign.inputs) | 8,034 | 100 |
| Ayame | [5](../crates/tenchu-pc/tests/data/mission5-ayame-campaign.inputs) | 7,919 | 46 |
| Rikimaru | [6](../crates/tenchu-pc/tests/data/mission6-rikimaru-campaign.inputs) | 6,139 | 50 |
| Ayame | [6](../crates/tenchu-pc/tests/data/mission6-ayame-campaign.inputs) | 6,393 | 5 |
| Rikimaru | [7](../crates/tenchu-pc/tests/data/mission7-rikimaru-campaign.inputs) | 3,744 | 100 |
| Ayame | [7](../crates/tenchu-pc/tests/data/mission7-ayame-campaign.inputs) | 2,546 | 100 |
| Rikimaru | [8](../crates/tenchu-pc/tests/data/mission8-rikimaru-campaign.inputs) | 8,448 | 74 |
| Ayame | [8](../crates/tenchu-pc/tests/data/mission8-ayame-campaign.inputs) | 8,126 | 56 |
| Rikimaru | [9](../crates/tenchu-pc/tests/data/mission9-rikimaru-campaign.inputs) | 6,292 | 3 |
| Ayame | [9](../crates/tenchu-pc/tests/data/mission9-ayame-campaign.inputs) | 5,741 | 67 |
| Rikimaru | [10](../crates/tenchu-pc/tests/data/mission10-rikimaru-campaign.inputs) | 15,082 | 30 |
| Ayame | [10](../crates/tenchu-pc/tests/data/mission10-ayame-campaign.inputs) | 16,015 | 7 |

Both complete Mission 2 with 100 HP. The independent
[Mission 7 route](../crates/tenchu-pc/tests/data/mission7-rikimaru.inputs)
also passes with the corrected swimming camera, finishing at tick 3,461 with
93 HP. These routes exercise ordinary controller inputs;
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
and does not cover every possible playthrough.

The exact-boundary checker additionally validates the archived full Ayame/Normal
campaign: all 33,404 gameplay/cutscene updates, 349 skipped draws, ten completions
and the original result clocks/scores. Recorded skips are explicit replay input;
ordinary play remains independent of console or host rendering performance.
See [retail capture instructions](../tools/retail-capture/README.md) for capturing
and checking another reference. The checker imports initial mission state once
and never corrects later native state from the reference.

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
