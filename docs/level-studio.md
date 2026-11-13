# Level Studio

Choose **Level Studio** on the Shinobi Gaisen or Tenchu 2 title, or press
**Shift+F10** in the menus. Studio is a separate workspace inside the game;
**Mission editor / F10** still opens the original editor. Studio uses your
Shinobi Gaisen disc. Connect a Tenchu 2 disc through the game launcher to add
its character, scenery and audio assets.

**Projects → Open T1 + T2 garden example** builds an editable courtyard from
both connected discs: T1 terrain, cherry trees, lanterns and props, T2 scenery
and Rikimaru, an original patrol, region messages and a delayed storm. Three
saved camera views frame the garden and characters. The example contains only
asset references and authored placements; it still needs both discs.

**Projects → Open lighting & events example** loads a T1-only editable example.
Press **F5** to play it: an arrival message appears immediately and a storm
arrives three seconds later. **Esc / Return to editing** returns to the project;
F5 during a playtest restarts it. Success and failure return automatically.
Exiting Studio restores the previous campaign or T2 title and its settings.
Playtests use the current gameplay preferences and never change campaign progress.

The built-in **demo --studio** tour demonstrates these features in fourteen
short chapters and can export a 4K/60 video with synchronized source sound cues.
See the [showcase commands](../DEMOS.md#brief-level-studio-showcase).

## Workspace and controls

The scene uses the game's renderer. **Lighting & weather** provides graphical
presets, time of day, sun direction and strength, ambient light, local lighting
and shadows, fog, rain, snow, wind, wetness and lightning. Changes preview in
the scene; weather and shader animation continue while gameplay stays at tick
zero. New projects start with **Modern High**. **Modern Ultra**, **Original**
and other presets are available per project without changing global presets.

| Control | Action |
| --- | --- |
| Right drag | Look around the scene |
| Hold right mouse + WASD / Q / E | Fly, descend and rise; Shift moves faster |
| Middle drag | Pan the scene or map |
| Wheel | Dolly in 3D, zoom around the pointer in Map view |
| F | Frame the selected character/scenery, or current terrain cell |
| G | Toggle editor guides for clean scene previews |
| Left click / drag | Paint; select and drag character/scenery markers with Select / move |
| R | Rotate the terrain brush or selected scenery by 90 degrees |
| Delete | Remove the selected removable character or scenery |
| Ctrl+D | Duplicate selected scenery |
| Arrows / Page Up / Page Down | Nudge selected scenery horizontally / vertically |
| End | Ground selected scenery on the solid terrain below it |
| Ctrl+C / Ctrl+V | Copy a selected terrain area / arm the stamp tool |
| I | Sample the terrain tile and rotation under the cursor |
| Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y | Undo / redo, with a brush stroke as one edit |
| Ctrl+S / Ctrl+Shift+S | Save / save a copy |
| Alt+1 through Alt+7 | Terrain, characters, scenery, events, environment, mission, projects |
| F5 | Start/restart a playtest |
| F9 | Capture a screenshot |

**Map view** supplies a zoomable 32×32 plan with tile thumbnails, character
markers and patrol paths. The resizable inspector holds palettes and properties.
Terrain strokes rebuild the 3D scene when released; environment controls update
live. Text fields commit on leaving the field. Keyboard editing shortcuts leave
text entry alone.

## Building tools

Choose **Area**, then drag a rectangle in Map or 3D view. **Terrain → Areas &
reusable stamps** copies both terrain layers, fills the rectangle with the current
brush, or creates an event region covering it. **Ctrl+V** arms placement; **R**
rotates the stamp and both layers' tile orientations. Click to place. The complete
stamp must fit and satisfy the original placement rules; a blocked fill or paste
leaves the map unchanged. Successful edits use the original patrol truncation
rules and can be undone as one operation.

Name a copied stamp and choose **Keep stamp** to save it in the project. Up to
32 stamps can travel with a project; stamps only paste into their source theme.
Copying another area replaces the temporary clipboard, not the saved stamps.
Terrain stamps also use the system clipboard, so they can be pasted into another
Studio session with the same terrain theme.

Scenery placement in 3D follows the surface under the mouse, with **Free**, 0.1 m,
0.5 m, 1 m or 2 m horizontal snapping. Arrow keys nudge X/Z and Page Up/Down change
height by the snap step (0.1 m in Free mode). **End / Ground on terrain** places
the object's origin on the highest solid terrain beneath its X/Z position.
Added scenery remains decorative and does not become a grounding surface.

**Camera views → Save current view** stores an editable name and camera angle.
Recall any of up to 16 views, or remove one. The first saved view opens when the
project is loaded. Views and stamps use the existing undo/redo history and keep
mission records intact; view changes never alter the playtest camera. Existing
version-one Studio files without these additions remain compatible.

## Missions and assets

All twelve original tile themes and sixteen challenges are available. Paint
ground, ramps, walls, pits, water, ornaments and traps using the source editor's
placement rules. Brushes support sizes from one to eight cells and four rotations.
Character placement retains supporting ground, ornament limits and original
route rules. Ten slots include the player and reserved objective characters;
patrols support up to 64 adjacent waypoints with original waits and facing.

The five objectives are reaching Lord Gohda, assassination, eliminating enemies,
protecting Princess Kiku and finding the herb. Any can require stealth. Mission
title, author, editing password, player character and best times are available
in **Mission**. Original gameplay and characters can set adjusted time records;
enabled events, imported character assignments or gameplay modifiers make a
practice run. Gameplay-affecting edits clear records; graphical edits keep them.

**Characters** offers the connected mixed-game appearance library. Enemy imports
reuse the existing Gaisen actor import behavior and keep their placement and
patrol. Lord Gohda, Princess Kiku and the objective herb retain their source
identity. A T2 body does not switch the mission to native T2 gameplay.

**Scenery** browses object meshes from T1 stages and connected T2 areas. Added
instances have editable position and yaw, can be duplicated, and use their source
textures and installed artwork overrides. They receive lighting and cast shadows.
They are decorative: solid geometry and interactive traps come from the terrain
palette. Studio does not import complete campaign maps as editable terrain or
port T2's original TRIAL editor. The mission grid retains Gaisen's original size
and collision behavior. Projects allow up to 512 added scenery instances.

## Events and cue queues

An event can trigger at mission start, after an elapsed time, when the player
enters a 3D region, when a selected character is defeated, or when a named flag
matches its value. Regions use game coordinates: 1000 units per metre, with Y
increasing downward. The viewport draws region bounds. Flags start false.

Each event contains an ordered queue. A cue's delay is measured from the previous
cue, or from the trigger for the first cue. Times show both nominal 30 Hz ticks
and seconds. Actions show a message, play a sound, start/stop looping music,
change weather or time of day, set a flag, or complete/fail the mission.
Sound cues select the source game, bank, sample, volume and pan. **Browse sounds
in this bank** lists populated samples; **Audition cue** previews audio while
editing. Music is selected by source track. Playtests prepare their cue audio
before starting; browser music preparation is asynchronous.

Events can be disabled or set to run once. Repeating region/flag triggers require
leaving and re-entering their condition, and do not overlap their own pending
queues. Equal-time cues use event order, then cue order. Flag changes can trigger
other events on the next logic tick. Rendering cadence never runs events or
advances gameplay. Limits are 256 events, 128 cues per event, one hour of queued
delay and eight distinct music tracks per project.

## Saving and sharing

**Save** updates the selected library project or creates one; **Save copy** keeps
both. Opening another project saves the current work. **Remove** removes a library
copy while keeping the current draft. The separate library holds up to 128
projects in `level-studio.json` in the game data directory, or at `TENCHU_STUDIO`.
Browsers use local storage. The original `trials.json` is untouched.

**Export project** writes a versioned `.tenchu` JSON project with all settings,
events and asset references. **Copy project** supports text sharing. These files
contain no textures, sounds or models; recipients need the corresponding discs
and any selected external model assets. Missing references produce an error.

**Import** accepts Studio projects, retail `.msn` missions, or the existing
single-block `.mcs` format. Retail share codes also work. **Export retail map
only** and **Copy retail share code** preserve original terrain, characters,
patrols, objectives, passwords and records. Studio scenery, character overrides,
events and graphics require a project and are excluded from retail exports.
Retail text must fit the original Shift-JIS fields; Studio project text is UTF-8.

The project data, editing rules and event runtime are portable. The browser has
file import/download and T1 editing; T2 scenery/audio browsing requires the native
disc connection. Android supports project export and pasted project/share-code
import. The desktop controls are designed for mouse and keyboard; playtests use
the game's existing keyboard/controller/touch bindings.

Verification includes project round trips and invalid-reference rejection,
stroke history, event ordering independent of render calls, source-bank audition
while paused, controlled disc-backed records/playtest/return fixtures, mixed T1/T2
GPU captures, and native/WASM compilation. These editor fixtures and screenshots
do not establish ordinary-play retail parity; that remains the existing
BizHawk capture/replay workflow described in [validation](validation.md).

New projects and imported retail missions start with your current graphics
settings. Saved projects retain their own environment. **Environment** contains
the graphics preset, lighting and weather controls; **Use my game settings**
reapplies the preferences you had when entering Studio. Leaving Studio restores
your main-game settings. **3D view / Map view** selects one view mode; **Guides**
independently controls editing markers. The workspace tabs remain above the
inspector, with grouped, aligned environment fields.
