# Japanese Tenchu 2 native play

Press **Left/Right** on the Shinobi Gaisen title, or use its **Tenchu 2** button,
to open the original JP title menu. If the disc is missing, import it from the
prompt; the title opens immediately afterward. Either direction or the footer
button switches back to Shinobi Gaisen. Held controls must be released after
switching. This frontend is available on desktop and Android.

The title uses the original disc artwork, menu sounds, fades and idle movies.
Enter or the configured confirm button reveals its three choices; Up/Down
selects. **Campaign** opens the original mission screen, whose Back action
returns to the T2 title. **Settings** opens F1. The original **Mission editor**
entry explains that T2's editor has not yet been ported. Switching titles does
not change either campaign's saved progress. The PC enters the still title
directly; after 30 seconds without input, the original opening and title movies
play. Start/Circle skips each movie.

Connect your Japanese disc through **F6 → Connect another disc…**, select
**Tenchu 2 Japan**, an area and a sequence, then **Start mission**. JP selections
now use the native JP runtime. The sequence determines the original player and
cast. EU selections still use the Gaisen scenery sandbox; JP can also opt into
that mode with **Use Gaisen scenery sandbox**.

For T1 stages with native T2 gameplay, select **Shinobi Gaisen → Gameplay rules →
Tenchu 2 (JP)** in F6. Both discs must be connected. Choose the stage, placement
layout, difficulty and any of the three T2 heroes, then use the native equipment
screen. T2 supplies guards, AI, physics, inventory, HUD and results; original T1
objectives and cutscenes retain their cast, captions and audio. Tatsumaru follows
Rikimaru's story. Doors, pitfalls, damaging floors, pickups, scrolling textures
and ambient effects are supported. Neither game's saved campaign is changed.
This mode is experimental: T1 has no authored underwater floors or scenery.
Added collision supports native swimming, diving and breath, but submerged views
remain sparse. **Water depth** defaults to 4,000 game units; 0 disables it.

The standalone equivalent is:

```sh
./dev.sh target/debug/tenchu-pc play-t2-jp /path/to/tenchu2-japan.cue --gaisen /path/to/gaisen.cue --stage 0 --hero 0 --layout 1 --water-depth 4000
```

Choose **Original JP mission selection…** in F6 for the original mission screen,
with all 29 missions available across Rikimaru, Ayame and Tatsumaru. Left/right
changes hero; up/down chooses a mission; Enter or the configured menu-confirm
button starts, and the other face button returns.
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
**General → Circle confirms in menus** applies to F1, mission selection,
equipment, results and retry/death prompts in both launchers. Off uses
South/Cross to confirm and East/Circle to go back; on reverses them. In equipment,
confirm adds an item and back removes it; Start/Enter begins the mission.
Controller menu positions ignore gameplay remapping, and touch controls follow
the same convention. The source button legends and PC prompts update immediately.
Enter/Esc retain their keyboard actions. Gameplay and cutscene skip buttons keep
their original meanings, including Circle guard/dodge and R1 crouch in gameplay.
**Visual assets → High-definition text** sharpens native health/Ki/item digits,
equipment counts, results numbers and the disc's prologue/briefing calligraphy.
Story dialogue already uses outline text. Modern High/Ultra enable this option;
Original keeps the source glyphs. Authored title and rank artwork stay original.
**General → Wait after death** holds the finalized corpse before the failure
fade. **Keep world running after death** also keeps guards, scenery, items and
effects updating while you observe with the camera. With that option off, the
scene and effect audio freeze; **Music while paused** still applies. The held
scene cannot change the failure result, carried supplies or retry RNG. Press
Enter, the configured menu-confirm button, Start or the on-screen Continue button
to proceed. Disabling Wait after death also proceeds once simulation resumes.
Mission success and detection failures keep their normal transitions; both death
options are off by default.
**General → Performance charts** shows the same CPU frame phases, GPU passes and
detailed scene categories as Gaisen. CPU timings include simulation/audio,
geometry/uploads, menu work, submission, presentation waits and unclassified time.
GPU samples require timestamp support and arrive asynchronously; they are not
added to CPU time. F1 allows freezing the charts; photo mode hides them.
Diagnostic reports include these bounded timing histories. The charts work in
the main launcher and standalone JP preview. **General → Benchmarks** runs four
fixed native scenes and supports automatic graphics tuning. It keeps the current
campaign or randomizer untouched and restores its presentation afterwards.
See [performance measurements](performance.md) for sampling and tuning rules.
The resizable F1 panel uses the same categories and controls as Gaisen. Tab /
Shift+Tab (or controller shoulders) changes category; arrows / D-pad selects and
changes values. R resets the hovered or selected value. General contains the
preset library, keyboard/controller remapping with JP action labels, and touch
layout settings. Graphics, Visual assets and Image effects include the working
native renderer controls. Visual assets shows the **Tenchu 2** pack and download
link independently of the Gaisen pack.

Unavailable controls stay visible but disabled. Hover or select a row to see
why; keyboard/controller shortcuts and resets cannot change disabled values.
**F1 → Movement → Export Tatsumaru moves** reads the running JP disc and saves
a portable `.t2moves` file for personal use in T1 on other devices, including
the browser port. It works from native menus as well as gameplay and does not
require a previously imported moves file. Native T2 already reads Tatsumaru's
unarmed style from its disc, so the T1 import control does not apply here.
**F1 → Overlays → Player trail** shows recent movement through the same bounded
path recorder as Gaisen. Paused redraws do not add points, large teleports break
the line, and new areas clear it. X-ray also applies to the trail; Photo mode hides
it with the other overlays. All graphical presets leave debug overlays off.
**F1 → Lighting** also supports movable lights, placement above the player or at
the camera, camera-follow lights, source markers and nearest-light inspection.
Flame flicker and heat shimmer use the shared renderer in gameplay and 3D
cutscenes. Heat shimmer requires Image effects; Modern High/Ultra select it
already. Source markers label shadow candidates because the renderer also checks
visibility and its shadow budget. Movable lights reset when the scene changes.

Presets remain shared, with only the supported graphics applied in native JP.
**Simulation → Keep all enemies active** keeps the placed native cast running
outside the source distance and activation budget. Distant poses and foot contacts
also update. It preserves patrol routes, deaths and scripted holds, and never
spawns extra actors. This changes encounters and RNG usage and may use more CPU.
It persists across areas; switching it off restores the original activation check
on its normal one-second cadence. It is off by default and graphical presets do
not change it. Enabling it expands collision storage for larger casts until the
area reloads, including retained contacts after the option is switched off.
**General → Randomizer** or **F7** opens a separate native seed editor, including
native missions and connected T1 stages. Starting enemy replacements retain
authored slots and mission roles; retries keep the same roll and normal campaign
saves are isolated. **End randomizer** returns to the previous normal session.
Native seeds and setup files are separate from T1; see [Randomizer](randomizer.md)
for supported rolls, area carry, savestates and source-specific limits.

**Enemy behavior** enables optional Coward, Reckless, Cautious, Flanker and
Watchman policies, or a seeded mix. Native sight, navigation, weapon reach,
combo frames and hit rules remain in use. Profiles can follow the authored idle
routine, patrol an existing route, keep watch or face toward the player.
Sleeping, bait investigation, bosses, civilians, animals, story actors and
mission targets retain their original routines. Per-guard combat and idle
overrides use the selected actor. Optional progression starts at Mission 6 on
Easy, 4 on Normal or 2 on Hard; imported T1 stages use their original order.
Profiles stay off by default and graphical presets do not change them.

Cowards can call aid after reaching terrain cover more than 16,500 units from
the player. With **Keep all enemies active**, **Witnessed-kill reinforcements**
also lets the nearest eligible guard who sees an armed guard die seek aid.
Calls use the configured fixed or random group size and total actor limit;
normal activation retains a 30-actor limit. Corpses count toward capacity.
Kill-all missions also retain their native kill-quota limit.
Each guard can call once per alert; recruits cannot call more recruits.
Recruits retain the caller's guard appearance and voice. These are optional new
behaviors: original T2 has no reinforcement caller. Assignments, pending calls
and settings survive saves and rewind.

**F1 → Simulation → Savestate slot** selects one of five native T2 slots.
**Save state / F11** saves gameplay or a 3D cutscene; **Load state / F12** restores
it and pauses until **F3**. Load from another scene or mission selection after
opening native T2. For T1 stages running under T2, first open the same imported
stage, source character, layout and language. T2 files are
`states/t2-jp-slot-1.tstate` through `t2-jp-slot-5.tstate` in the usual data
directory (`TENCHU_STATES` can override that directory), separately from T1 slots.

States retain campaign/retry context, inventory, edited actors, model/moveset/
equipment/voice overrides, cutscene faces, blood, weather, footprints and secondary
motion. Folder-model resources are embedded. Gameplay choices follow the state;
current graphics and installed artwork remain in use. Loading cancels pending
click/editor actions and held physical inputs, returns to the player camera and
resynchronizes cutscene audio. Loading itself does not write campaign progress;
later normal mission completion can do so. State files require the same game
version and contain disc-derived runtime data, so keep them outside source and
asset releases. Native T2 also supports the shared rewind timeline, seeking,
forward playback, branching, and recording import/export through F1 or Ctrl+F12.

**F1 → Bleeding** also works in native JP, including imported T1 stages. It is
optional gameplay, separate from cosmetic wound drips: successful enemy melee,
shuriken/kunai, darts, ordinary arrows and gunshots can start nonfatal blood loss.
Chance, duration, damage and bandage supply use the shared settings. Select
Bandage with L2/R2 and use Triangle, or use the rebindable B shortcut; F1 can edit
the current supply or request treatment on the next game tick. Treatment requires
grounded, free movement and cannot interrupt attacks, rolls, injury, aiming,
swimming or scripted scenes. A potion cures only when its healing effect occurs
if enabled; resurrection always cures. HUD status and the bandage icon/count
respect Hide HUD, Photo mode and scene fades, with either original or HD text.

Wounds and bandages carry through areas and cutscenes in the same mission; retry
or a new mission starts a fresh supply. Changing the configured loadout does not
refill current stock, and bandages never consume the campaign's item stock.
Original stays unchanged, and Modern High/Ultra retain the separate gameplay
choice. Savestates preserve wounds and supplies. The existing `bleeding-bandages`
demo exercises Gaisen; native demo playback remains separate work. Native
rewind retains wounds and supplies.

**F1 → Persistent effects → Stain wounded models** attaches soft stains to the
body at native blood-emission positions after damage. It uses Gaisen's projector
and the same bounded blood store
as ground stains. Native bodies, imported bodies and custom player models follow
their displayed joints, interpolation and secondary motion. Body stains remain
separate from the hit-spurt and ground styles. Turning the option off or using
Clear blood removes them; rebuilding model geometry also clears body stains while
preserving ground blood and weather. Modern High/Ultra already enable this;
Original stays unchanged.

**Wound drips**, **Wound bleeding** and **Wound spurt strength** share Gaisen's
finite bleeding simulation. Original, Original HD and Modern affect the extra
drops independently of hit blood and body staining. Drops move on source ticks,
land on level surfaces using the ground-blood style/retention setting, and play
T2's spatial blood-impact sound. Pause freezes them and fast-forward advances
them without playing a backlog of sounds. Disabling drips or using Clear blood
removes active wound bleeding; a model rebuild clears body-local emitters while
already falling drops and ground stains remain. Modern High/Ultra enable wound
drips; Original remains unchanged.

**Death gush style**, **Death gush strength** and **Death gush duration** use the
shared optional upward bursts. A new death waits for its authored blood event,
then emits from the animated source body joint. Original, Original HD and Modern
styles are independent of hit spurts and wound drips. Drops stain the ground using
its selected style and play spatial T2 impact sounds. Death pools must not be Off;
**Death pool beneath player** also controls extra player gushes. Drowning remains
unchanged. Clear blood, disabling gushes and model reloads stop current emitters;
reenabling does not restart old corpses. The effect stays off by default, as in
Gaisen; native and shared graphical presets keep their existing values.

**Blood on weapons**, **Weapon blood on metal only**, **Weapon blood coverage**
and **Wet weapon blood flow** share the Gaisen coating and droplet simulation.
Successful native melee hits stain the attacking blades; paired kills wait for
source blood emission. Native and imported weapons on players and enemies use
their own geometry, including both blades of dual weapons. Sheaths and unarmed
accessories stay clean. Coatings follow animated hands and shed droplets during
swings; the ground-blood settings control their stains. Clear blood and disabling
the option remove coatings and loose weapon drops. Rebuilding models clears blade
coatings alongside body attachments. Modern High/Ultra enable the option;
Original stays unchanged.

**F1 → Simulation → Enemy sight distance** scales native notice, recognition,
forgetting and corpse-detection distances from 0–400%, including guards approaching
bait. Angles, terrain obstruction and actor activation remain original; 100% is
the source behavior. **Experiment → Never noticed** returns guards to their own
idle/patrol behavior and suppresses player/corpse alerts. Already committed attacks,
projectiles and traps can still hurt; use God mode for protection. Scripted
sequences and combat against other actors remain active. These experiments reset
on a new area, and do not erase previous detection penalties. Sight overlays show
the effective range and query result, with ignored-player cones dimmed. Diagnostic
reports include the active values.

**F1 → Teleport / actor → Place / edit mission actors** supports copying the
selected position, picking a floor point, moving with the chosen facing, and
removing a native actor. Moving clears its previous patrol route. Players,
story actors and mission targets are protected; removing living actors that
contribute to a kill quota is blocked. **Kill selected actor** remains available.
Removal gives no kill credit and keeps script references intact. Edits last
until retry or the next area, including in imported T1 stages. Native actor
spawning is still being connected.

**F1 → Secondary motion** uses the same bounded hair, cloth and equipment springs
as Gaisen, during gameplay and 3D cutscenes. Strength, response and breeze work
without changing animations, weapon contacts, collision or RNG. Motion advances
once per game update, including fast-forward, and interpolates for display;
pausing freezes it and disabling the options restores rigid geometry immediately.
The authored regions cover Rikimaru's hair and loose shoulder gear, Ayame's hair
and bow, and Tatsumaru's ponytail. Faces and rigid weapon attachments stay intact.
These rules also work when importing the T2 bodies into Gaisen. Other models need
matching material/part rules; custom folders can provide the same `motion` rules
in `pack.json`. Modern High/Ultra already enable these controls; Original keeps
them off.

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

**F1 → Lighting → Distance fog color / Atmospheric haze** adds the selected
atmosphere to T2's original palette shading, using the stage's own fog range.
These options affect gameplay and 3D cutscenes, including imported T1 stages;
disabling Image effects restores the original shading.
**F9 / F1 → Simulation → Screenshot** saves the current view as a PNG, including HD
text, the selected camera, visual effects and HUD/photo settings. It also works
in equipment, results and narration screens. Files are written to `screenshots/`
or the directory selected by `TENCHU_SCREENSHOTS`.

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
Modern pools fit visible terrain and both original styles use T2 artwork.
**Death pool beneath player** adds a pool when the native death does not create
one. It waits for the finished, grounded corpse pose and uses the selected style,
size and retention. Normal T2 deaths already create pools: those remain controlled
by **Death pool / star**, and the extra option never duplicates them.
Modern High/Ultra already enable Modern hit, ground and death blood. Original
keeps source timing, growth and artwork.
Scripted native T2 cutscene bursts, droplets and pools use these same styles.
Their original timing and surface contact continue through fast-forward; camera
movement, pausing and graphics changes do not advance the particle simulation.
Authored sparks, water spray, dust, leaves, smoke, explosions, animated sprite
tracks and joint-attached fire also run through the original effect routines.
The two frame-feedback commands blend preceding frames; they do not distort
geometry. Paused redraws do not accumulate that blend. Seeking, resizing or
changing scenes discards presentation history.

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
**Shift / Circle action** offers Follow moveset, Crouch and Guard / dash.
Native T2 defaults to Circle guard/dodge and R1 crouch for every body and moveset.
Crouch explicitly swaps those buttons. In T1 under Gaisen rules, Follow moveset
uses T2 guard/dodge with a T2 moveset and T1 crouching with a T1 moveset. The extra
T1 R1 guard controller and its automatic weapon drawing do not apply to native
T2; its source guard stance keeps the current weapon state.
**Start aiming toward camera** turns the player and seeds the vertical angle when
starting grapple, shuriken/blowgun aim or look. Subsequent aiming adjustments use
the source controls; both sticks can adjust aim. Disable camera-relative controls
to return to the original camera and tank movement. Free camera and menus suppress
movement input, and Follow Actor remains independent.

**Analog / modifier walking** uses T2's original ninja/kunoichi walk cycles and
speeds (38/31 units per tick) with light stick input or the rebindable Walk action.
New bindings use Left Alt or left-stick click; existing bindings can be assigned
under General → Controls. The threshold includes hysteresis to avoid jitter.
Classic controls walk forward; camera-relative movement can walk in any direction.
Full stick travel or disabling walking restores the selected moveset's run.
Attacks, guarding, crouching, swimming and collision retain their source behavior.
This optional gameplay edit stays off by default and is separate from graphical
presets. Save/rewind retains its motion state and recordings retain its input.

**Draw / sheathe weapons** enables the shared rebindable action (H by default).
It starts the active moveset's native draw/stow animation from grounded free
stances, with the original weapon-swap frame and sound. It cannot interrupt
attacks, injury, jumps, scripts or mission input locks. The movement-speed,
detection-range and damage options apply while this feature is enabled and the
weapon is stowed. Speed changes affect forward/backward running and optional
walking. Detection changes stack with the guard sight-distance setting. Damage
changes affect health loss from melee and item/projectile hits; recoil, falling,
ongoing poison, God mode and resurrection keep their original rules. Disabling
the feature removes its modifiers and preserves T2's current weapon state.
Native T2 already retains a stowed weapon after injury and knockdown, so the
T1 **Stay sheathed after damage** option is unnecessary here. Saves, rewind and
recordings retain the settings and pending manual input.

**Unarmed attacks while sheathed** uses Onikage's Gaisen kicks or Tatsumaru's JP
punches and kicks while Draw / sheathe is enabled and weapons are stowed. Select
the style in F1; Tatsumaru reads the running T2 disc, while Onikage needs your
connected Gaisen disc. Grounded directional attacks, crouching attacks and combos
use that style's available moves and hit contacts. Your armed moveset, weapons and
voice remain selected. Airborne, wall and stealth attacks keep T2's rules. Style
changes wait until the current attack finishes; disabling the option also lets
that attack finish. Saves and rewind retain the active attack. Portable `.t2moves`
import/export is still unavailable in native T2.

**Stealth Assassins dismemberment** adapts the shared optional head/forearm rules
to native T2 guards and civilians. Rear stealth kills roll at their first authored
blood event; dive kills can remove heads, and ordinary combat deaths can remove
forearms. Bosses and animals are excluded. **Player dismemberment** has separate
chances. The controls change gameplay RNG and stay off by default; graphical
presets do not enable them. Parts bounce against T2's terrain and retain their
detached position through animation, saves and rewind. **Severed head resting
pose** changes the next landing without consuming RNG.

**Severed blood style**, strength and duration use the shared finite droplets
and body-wound jets, including imported/custom player bodies. They advance on
logic ticks and respect Clear blood. Modern High/Ultra already enable this
presentation effect when a part exists; Original remains unchanged. The shared
dismemberment demo covers Gaisen; native demo playback is still pending.
**SA Kataoka execution scene** applies to Rikimaru's imported T1 Mission 5 story,
with any native hero, and retains Gaisen's localized dialogue and audio. Enable
it before the scene begins. Native T2 campaign scenes keep their source behavior.

**F1 → Player model** uses the shared searchable gallery of Gaisen and connected
T2 discs. Choose a body, enable **Override moveset**, then leave **Moveset source**
on **Selected player body** to inherit its moves. You can also select an explicit
source independently of appearance. This includes supported guards and bosses,
not just heroes. **Use player movement** retains the mission protagonist's
locomotion with the chosen attacks; **Disable idle taunts** suppresses fidgets.
Changes apply while alive and standing/crouching outside cutscenes. Disable the
moveset override and select **Original** to return to the native hero.

Selected bodies and the equipped weapon set also carry into imported T1
cutscenes, including custom bodies and independent weapon choices. Attachments
follow the displayed body and the scene's authored draw/sheath state. The
original script, other actors and gameplay state are unchanged.

Movement and melee use source registrations, timing, combos and contacts. The
host still supplies collision, damage rules, mission identity, inventory,
paired stealth kills and unmapped actions. Character-local voice/weapon cues use
the selected moveset’s own disc bank, with translations between the games’ cue
slots; global world effects and event scheduling still follow the host. Movesets carry their source
weapon models, textures, handles and sheaths, including T2 HD-pack replacements
and source draw/holster attachment rules.
Draw/sheath animations and weapon-event timing also follow the selected moveset
when it supplies those motions. Scripted/projectile-only moves
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
changing models or artwork preserves it.

**Original blob shadows** draws T2's original foot-contact shadow beneath living
characters, including on imported T1 stages, and follows the authored shadow
rules in 3D cutscenes. Original enables it; Modern High/Ultra disable it
in favor of their model shadows. The toggle changes drawing only.

**F1 → Overlays** provides hitboxes, actor labels and the actor inspector, with
X-ray and photo-mode handling. The boxes come from native collision slots.
Terrain/collision, patrol routes, mission destinations and surface triggers also
use native data. Sight envelopes are clipped at eye height; actual target rays
use the JP sight routine and stop at its obstruction point. The selected guard's
label explains the result of its next perception update. These queries do not
advance AI or change the world. Free-camera clicks select actors. Both games share the
[inspection and presentation boundaries](runtime-boundaries.md).

**F1 → Teleport / actor → Kill selected actor** forces a living NPC's death,
including bosses. **Click to kill**, also under Experiment, targets NPC body
hitboxes in the normal, follow or free camera; close F1 before clicking/tapping.
Esc/Back cancels, teleport picking disables it, and new areas reset the mode.
The command bypasses invulnerability and resurrection, credits the kill and runs
the native death animation and corpse handling. Mission objectives can advance.
Players, hidden story actors, active cutscenes and paired stealth kills are
protected. Photo mode and clicks over menus do not target actors. Gaisen has the
same controls; using them ends input recording and disables TRIAL records for
that run. These are gameplay experiments; graphical presets do not enable them.

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
Original leaves native weather unchanged. **Tracks on original snow** supports
recognized snowy surfaces, including imported T1 stages, without generated snowfall.
Native T2 artwork recognition and moving-prop shelter remain separate work.
Replacing artwork retains wetness/snow, footprint decals and the visual sky clock
while rebuilding the surface cache.

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
High-detail hero rebuilding, foliage refinement and vegetation variation use
the shared mesh pipeline. The native environment supplies foliage emitters for
leaves and contact scatter; these follow the corresponding F1 controls.

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
for voices and ambience. It uses the shared renderer and the F1 features described
above. The HUD includes health/enemy bars, Ki, breath, the aiming reticle, item
selection and pickup notices, with optional HD text. Story playback includes scripted blood and other timed particles, animated
sprites, joint-attached fire and the original frame-feedback overlays.
The native renderer also loads the original sky layers and distant scenery,
including the story sequences' alternate evening palettes.
The existing BizHawk observer/replay checker now compares bounded native JP
training play, including recorded display skips. Separate controlled fixtures
cover progression, failure and retry through all three campaigns. Neither is a
complete recorded campaign playthrough; see [validation coverage](validation.md).
EU assets remain supported without an EU-native AI runtime.
