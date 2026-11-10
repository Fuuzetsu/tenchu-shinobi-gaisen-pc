# Japanese Tenchu 2 integration preview

Connect your Japanese disc through **F6 → Connect another disc…**, select
**Tenchu 2 Japan**, an area and a sequence, then **Start mission**. JP selections
now use the native JP runtime. The sequence determines the original player and
cast. EU selections still use the Gaisen scenery sandbox; JP can also opt into
that mode with **Use Gaisen scenery sandbox**.

Choose **Original JP mission selection…** in F6 for the original mission screen,
with all 29 missions available across Rikimaru, Ayame and Tatsumaru. Left/right
changes hero; up/down chooses a mission; Circle or Enter starts and Cross returns.
The portraits, mission rows and arrows also accept mouse/touch input. Selection
starts at the mission's opening scene. F1 can return to this screen during JP
play; leaving equipment or results returns here when the mission began here.
Saved ranks and supplies are retained; making missions available does not grant
items or change your recorded results.
Until that hero's training is complete, Rikimaru and Ayame first play the original
narrated prologue. Start/Circle, Enter or the on-screen button skips it.

The same route is available from the command line:

```sh
./dev.sh target/debug/tenchu-pc play-t2 /path/to/gaisen.cue /path/to/tenchu2-japan.cue --sequence 9
```

Saved controller/keyboard bindings and Android touch controls apply. F1 opens
the shared settings menu, F3 pauses, F4 advances one logic tick, and
F6 returns to the quick launcher. JP play does not write Gaisen campaign saves.
Mission results automatically save JP ranks, supplies and unlocks to
`tenchu2-jp-progress.json` in the game's user-data directory. This is a portable
JSON file on desktop and Android. `TENCHU_T2_SAVE` can select another path on
desktop. A damaged or newer-format file reports an error instead of being reset.
The standalone developer preview below does not load or write this progress.
Graphics changes in this F1 menu are shared with the main game.
**General → Performance charts** shows the same CPU frame phases, GPU passes and
detailed scene categories as Gaisen. CPU timings include simulation/audio,
geometry/uploads, menu work, submission, presentation waits and unclassified time.
GPU samples require timestamp support and arrive asynchronously; they are not
added to CPU time. F1 allows freezing the charts; photo mode hides them.
Diagnostic reports include these bounded timing histories. The charts work in
the main launcher and standalone JP preview; automated benchmarks remain pending.
The resizable F1 panel uses the same categories and controls as Gaisen. Tab /
Shift+Tab (or controller shoulders) changes category; arrows / D-pad selects and
changes values. R resets the hovered or selected value. General contains the
preset library, keyboard/controller remapping with JP action labels, and touch
layout settings. Graphics, Visual assets and Image effects include the working
native renderer controls. Visual assets shows the **Tenchu 2** pack and download
link independently of the Gaisen pack.

Unavailable controls stay visible but disabled. Hover or select a row to see
why; keyboard/controller shortcuts and resets cannot change disabled values.
**F1 → Lighting** also supports movable lights, placement above the player or at
the camera, camera-follow lights, source markers and nearest-light inspection.
Flame flicker and heat shimmer use the shared renderer in gameplay and 3D
cutscenes. Heat shimmer requires Image effects; Modern High/Ultra select it
already. Source markers label shadow candidates because the renderer also checks
visibility and its shadow budget. Movable lights reset when the scene changes.

Presets remain shared, with only the supported graphics applied in native JP.
Body/weapon blood enhancements, hair/cloth physics,
rewind, randomizer, enemy-behaviour cheats
and automated benchmarks are **not connected to native JP yet**. Gaisen and its scenery
sandbox still provide those features.

**F1 → Lighting → Lantern / prop lights** uses the shared texture analysis and
shadow pipeline for T2 paper, stone, hanging and red lanterns, including 3D
cutscenes. Original material fingerprints identify the lit panels; unlit stone
lanterns and bright walls stay unlit. Their source positions remain available
outside the native draw list, so moving the camera does not activate them.
Fog-off and free-camera views also draw distant static props; the original
simulation still decides when interactive objects update. **Graphics → Distant
doors without fog** also draws dormant doors and collapsing platforms when fog
is off or extended, or a custom camera is active. Opening/destroyed objects keep
their current source poses; collision and triggers remain source-controlled.
Disable the option to restore source drawing. Moving props also cast dynamic
shadows in gameplay and 3D cutscenes.
Artwork packs can override the same light rules as Gaisen. Modern High/Ultra
already enable this setting; Original keeps dynamic lighting off.

**F1 → Lighting → Fire / explosion lights** also lights burning characters, explosions,
fire breath and embers. Their moving emitters share Gaisen's clustering, shadow
selection and surface lighting. Light follows the source effect's position and
lifetime even outside the original camera view; disabling it restores the source
particles without changing damage, AI, audio or RNG. Modern High/Ultra already
enable fire lighting; Original keeps it disabled.

**F1 → Contact effects → Modern impact sparks** enables the shared modern
sparks for grappling, projectile contacts and weapon blocks. **Spark amount**
controls their density; dynamic lighting illuminates nearby surfaces from the
impact and moving sparks. Modern High/Ultra enable this, while Original retains
the source particles. Presentation effects do not consume gameplay RNG, and
pause, fast-forward and artwork/model reloads do not duplicate impacts.

**F1 → Persistent effects → Blood spurts on hit** offers the same Off, Original,
Original HD and Modern styles as Gaisen. Native particles supply their own
motion and timing; Original HD smooths the disc artwork, and Modern uses the
shared droplet renderer. **Ground blood** independently offers the same four
styles for dry floors and walls. **Retain original blood** keeps the chosen
Original/Original HD splats; **Conform retained blood to surfaces** fits them to
visible terrain. Modern ground stains use the shared projection and cached decal
storage. The shared amount, retention limit, runoff and clear controls apply;
water blood retains its source effect and does not become a permanent stain.
Source contacts survive fast-forward, and pausing or changing model detail does
not duplicate or erase them. **Death pool / star** independently selects Off,
Original, Original HD or Modern for source death pools, with the shared size and
retention controls. Their appearance and growth follow the native death event;
Modern pools fit visible terrain and both original styles use T2 artwork. Body/weapon stains and wound bleeding remain pending.
**Death pool beneath player** adds a pool when the native death does not create
one. It waits for the finished, grounded corpse pose and uses the selected style,
size and retention. Normal T2 deaths already create pools: those remain controlled
by **Death pool / star**, and the extra option never duplicates them.
Modern High/Ultra already enable Modern hit, ground and death blood. Original
keeps source timing, growth and artwork.

**Detailed rope / hook** in Contact effects uses the shared round woven rope and
hook refinement. T2 supplies its original flight/pull path; interpolation and
custom bodies supply the displayed hand anchor. Original keeps the source line
and hook mesh, and the option never changes grapple reach, collision or timing.

**F1 → Water** controls the shared modern water shading, ripples, reflections,
flow, foam, waterfall smoothing and spray. Native gameplay and 3D scenes use the
level's authored water textures to identify surfaces, with original or HD artwork.
Modern High/Ultra already enable these effects. Original keeps source texture
scrolling, geometry and collision; these options only affect presentation.
**F1 → Contact effects → Water contact splashes** also replaces source water
contacts with the shared droplets and ripples. Droplets leave temporary wet marks
on nearby dry ground. Source swimming, landing and item contacts drive the effect;
turning it off restores the original sprites, points and rings.

**F1 → Movement → Camera-relative controls** enables movement relative to the view,
including full left-stick direction. Hold right mouse or use the right stick to
orbit, and click the right stick or choose **Recenter behind player** to recenter. T2's
bindings still apply: Circle guards and R1 crouches. Attacks keep their committed
facing; directional attack inputs are mapped into the character's local space.
Wall, ledge, underwater and item controls retain T2's original behavior.
**Start aiming toward camera** turns the player and seeds the vertical angle when
starting grapple, shuriken/blowgun aim or look. Subsequent aiming adjustments use
the source controls; both sticks can adjust aim. Disable camera-relative controls
to return to the original camera and tank movement. Free camera and menus suppress
movement input, and Follow Actor remains independent.

**F1 → Player model** uses the shared searchable gallery of Gaisen and connected
T2 discs. Choose a body, enable **Override moveset**, then leave **Moveset source**
on **Selected player body** to inherit its moves. You can also select an explicit
source independently of appearance. This includes supported guards and bosses,
not just heroes. **Use player movement** retains the mission protagonist's
locomotion with the chosen attacks; **Disable idle taunts** suppresses fidgets.
Changes apply while alive and standing/crouching outside cutscenes. Disable the
moveset override and select **Original** to return to the native hero.

Movement and melee use source registrations, timing, combos and contacts. The
host still supplies collision, damage rules, mission identity, inventory,
paired stealth kills and unmapped actions. Character-local voice/weapon cues use
the selected moveset’s own disc bank, with translations between the games’ cue
slots; global world effects and event scheduling still follow the host. Movesets carry their source
weapon models, textures, handles and sheaths, including T2 HD-pack replacements
and source draw/holster attachment rules. Scripted/projectile-only moves
are not connected to the native runtime yet. A body without a supported
moveset keeps the host's moves. The standalone `play-t2-jp` preview needs a
remembered Gaisen disc; open it once through `play` to connect the gallery.

The same **Model folder → Load / reload** control accepts Gaisen-format custom
model packs, including OBJ parts, textures and optional skin weights. Select a
body in **Custom folder**; an independent moveset source still works with it.
These swaps only change presentation. Weapons, held items and grapple ropes
follow the displayed hand positions while collision keeps the source rig.
**Visual assets → Reload artwork** rereads the installed packs during play.
Reloads preserve weather accumulation, footprints and the visual clock.

**F1 → Lighting** uses the shared dynamic-light and shadow renderer for native
stage flames, with the same intensity, ambient, radius, light/shadow counts,
resolution and softness controls. Modern High/Ultra use their existing settings;
Original keeps these effects off. These sources are independent of the source
camera's draw culling. **Sky lighting** provides manual sun/moon direction,
colour, intensity and shadow quality. **Time of day** can instead drive the light
and ambient colour through a day/night cycle. Both work in gameplay and 3D
cutscenes, use the shared terrain/model shadows, and remain presentation-only.
The visual clock follows simulation time, including pause and fast-forward;
changing models or artwork preserves it. Other combat emitters remain pending.

**F1 → Overlays** provides hitboxes, actor labels and the actor inspector, with
X-ray and photo-mode handling. The boxes come from native collision slots.
Terrain/collision, patrol routes, mission destinations and surface triggers also
use native data. Sight envelopes are clipped at eye height; actual target rays
use the JP sight routine and stop at its obstruction point. The selected guard's
label explains the result of its next perception update. These queries do not
advance AI or change the world. Free-camera clicks select actors. Both games share the
[inspection and presentation boundaries](runtime-boundaries.md).

**F1 → Teleport / actor** supports coordinate and click/tap destinations, optionally
snapping to a collision floor. **Experiment → Click to teleport** also works in
the normal camera. Escape / Back cancels picking. Teleports require a living player
outside attacks, damage, item use, paired kills and scripted transitions; blocked
or out-of-map destinations are rejected. Collision and camera state update together.

**F1 → Experiment** also provides god mode, armour protection, give/clear equipment
and individual quantities for T2's carried items. God mode blocks combat, poison,
drowning and fall damage; hit reactions still happen and enabling it does not
refill health or revive a dead player. Armour changes damage protection, not the
already loaded outfit. God mode and armour overrides reset with a new area. Equipment edits change
the current loadout without changing campaign stock or interrupting active items.
Unlike Gaisen, a T2 quantity of 255 is still consumed normally (the grappling hook
is never consumed). Armour has its own checkbox; unused item slots are omitted.

**F1 → Weather** uses the same rain/snow/storm renderer as Gaisen, including
wind-directed shelter on terrain and static props, accumulation, runoff, tracks,
and synthesized ambience/thunder. Weather remains optional in every preset.
Original leaves native weather unchanged. Baked-snow recognition on T2 artwork
and moving-prop shelter are still pending; the disabled baked-snow control does
not enable footprints on original snowy textures. Replacing artwork retains
wetness/snow, footprint decals and the visual sky clock while rebuilding the surface cache.

**F1 → Camera** supports free flight (F2), speed/FOV, player or actor focus,
overhead views and a camera bookmark. WASD/QE move, Shift boosts and RMB looks;
controller/touch sticks work too. F2 or Back returns to the game view. This also
works while paused and in 3D cutscenes; gameplay input is withheld during free
flight. F10 hides the gameplay HUD and camera help. During missions, **Follow Actor**
tracks the selected actor, including after the player dies. RMB/right stick orbits;
distance, sensitivity, inversion and recentering use the shared camera controls.
Terrain and props keep the camera out of walls. Following does not steer the actor
or change the simulation camera; F2/Back restores the original view.

**F1 → Visual assets / Model refinement** supports smooth model lighting,
character contours and character surface refinement through the shared mesh
pipeline. Native T2 bone weights follow the added vertices, including imported
models used in either game. These options also apply to 3D cutscene characters.
**Weapon reflections** uses the shared metal shader. Modern presets select their
usual quality levels; Original retains the source geometry. Live changes rebuild
only when mesh preparation settings change and preserve weather/footprints.
High-detail hero rebuilding, vegetation and secondary-motion adapters remain
separate pending work; their controls stay disabled.

**Smooth model motion** blends actor and weapon poses between the original 30 Hz
updates, including held equipment and cutscene characters. Gameplay and follow
cameras use the same visual timing; scripted camera cuts stay exact. Modern
presets enable it, Original leaves it off. This adds up to one logic tick of
display delay, without changing collision, AI, audio events or RNG. Pausing and
single-stepping show the exact current pose; model swaps and teleports reset it.

**F1 → Tenchu 2** contains the original JP mission selector, integration
diagnostics and return to the quick launcher. These controls are disabled with
an explanation outside native JP play. **F1 → Simulation** contains pause,
single-step, simulation speed (25%–400% or Unlimited) and automatic cutscene speed.
Simulation speed changes host pacing while retaining every source update. Unlimited
mutes audio and yields regularly to input/rendering; menu navigation stays at its
normal rate. Reset the value with R to return to 100%.

**F1 → General → Save diagnostic report** exports the same portable JSON report
as Gaisen, with native JP actor/AI/item state, mission progress, model/moveset
selection, the last 600 simulation inputs, settings and GPU details. Cutscene
reports include their current frame and actor/camera state. Input history resets
at scene boundaries; the report identifies the native pad encoding. No disc assets
or campaign save are included, and personal path roots are redacted. Reports from
Windows, Linux and Android can be read on any platform with `inspect-report`.

Hold Backspace during a cutscene to fast-forward, or choose an automatic speed
in F1 (2×–16× or Unlimited). This includes prologues, chapter narration, disc
movies and in-engine scenes. Acceleration mutes audio; releasing it resumes
dialogue at the current position. An on-screen Fast-forward button is also
available for mouse/touch. Cutscene acceleration does not change the chosen gameplay
speed or speed up menus.
The JP game uses Triangle for items and Circle for guarding (E and `/` with
default keyboard bindings); Z/C cycle inventory. These follow Tenchu 2's controls.
Before missions, the original equipment screen draws from your saved JP supplies.
Directions select an item, Circle adds one, Cross removes one, Start confirms,
and Start + Select cancels. Mouse/touch selection and buttons are also available;
Enter confirms. Training keeps its original fixed loadouts, and boss-area
transitions keep the inventory already carried. Armor selects the original
armored costume and damage protection.
At results, Circle continues, Start retries, and Cross leaves the mission. The
original score/rank artwork and dimmed scene accompany mouse/touch buttons;
Enter also continues. Results use the disc's menu sound bank.

For development, the standalone preview is also available:

```sh
./dev.sh cargo run -p tenchu-pc --bin tenchu-pc -- play-t2-jp /path/to/tenchu2-japan.cue --sequence 9
```

Add `--menu` to open the original JP mission screen in this standalone preview.
Disc folders and supported archives work here too. Use `inspect-t2 DISC` to
list sequences and their areas. The sequence selects its campaign hero.
Keyboard/controller bindings and the F1 settings panel are shared with the main
game. F1 → Tenchu 2 → Integration diagnostics shows actors, health, water state
and unconnected events;
F3 pauses, F4 advances one logic
tick, and Escape closes the preview. This mode does not write campaign saves;
it loads and saves the shared settings, including cutscene speed.

Add `--loadout 4:20` to bypass the equipment screen with twenty shuriken. The normal item-selection and item-use
bindings apply; the original item carousel shows the selected item and quantity.
Use `--loadout 4:20,5:5,6:10,7:5,8:10,9:10,10:5,11:5,12:5,13:5,14:5,16:5,17:5,18:5,20:1` to add caltrops (5), grenades (6), mines (7), poison darts (8), fire
arrows (9), smoke bombs (10), poisoned rice (11), coloured rice (12), invisibility (13), confusion powder (14), teleport (16), fire breath (17) sleeping bombs (18) and one passive resurrection leaf (20). Other inventory entries accept comma-separated
`ID:COUNT` pairs, but their effects are still being connected.

For a repeatable capture:

```sh
./dev.sh target/debug/tenchu-pc render-t2-jp /path/to/tenchu2-japan.cue /tmp/jp.png --sequence 9 --ticks 60
```

`--seed` sets the initial RNG state. Capture-only `--buttons HEX` holds a raw
Tenchu 2 pad mask for the requested ticks. `TENCHU_SETTINGS` selects a settings
file for rendering; the separate `tenchu2-hd.tar.zst` pack is discovered normally.

For JP HD mission textures, install [Tenchu 2 artwork v2](../artwork/tenchu2/README.md).
It covers the native terrain, props, character costumes, weapons and items.
The Gaisen `tenchu-hd.tar.zst` pack remains separate; installing that pack alone
does not replace T2 textures. F1 shows the T2 pack path independently.

This preview uses JP actors, controls, AI, collision, water interaction, camera,
scenery, grapple, shuriken/kunai, caltrops, bows, fire arrows, firearms, grenades, mines,
smoke/sleeping bombs, poison darts, confusion powder, invisibility, teleport, fire breath, coloured-rice markers, resurrection, poisoned rice with guard luring,
weapon/projectile trails, enemy status clouds, floor smoke/spear/arrow traps and particles.
Mission objectives now use the JP sequence descriptors, including exit regions,
unseen requirements, boss-health thresholds and elimination quotas. The preview
continues through story, area, boss, results and retry sequences. Health,
inventory, RNG and score totals carry across areas. Results calculate JP scores,
ranks, best records, supply rewards and unlocks. Chapter titles, narrated text,
movies, story camera/actors/faces and dialogue load from the user's disc.
Missed shuriken can be retrieved; projectile impacts use the native damage,
deflection and alert rules. It loads the hero/area VAB from the supplied disc
for voices and ambience. Its minimal renderer does not yet expose all of the
port's visual enhancements. The original HUD now shows health/enemy bars, Ki,
breath, the aiming reticle, item selection and pickup notices. Timed 3D story
effects remain integration work.
The native renderer also loads the original sky layers and distant scenery,
including the story sequences' alternate evening palettes.
Component comparisons and snapshot continuation do not establish full campaign
parity. EU assets remain supported without an EU-native AI runtime.
