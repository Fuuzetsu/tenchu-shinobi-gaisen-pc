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
The resizable F1 panel uses the same categories and controls as Gaisen. Tab /
Shift+Tab (or controller shoulders) changes category; arrows / D-pad selects and
changes values. R resets the hovered or selected value. General contains the
preset library, keyboard/controller remapping with JP action labels, and touch
layout settings. Graphics, Visual assets and Image effects include the working
native renderer controls. Visual assets shows the **Tenchu 2** pack and download
link independently of the Gaisen pack.

Unavailable controls stay visible but disabled. Hover or select a row to see
why; keyboard/controller shortcuts and resets cannot change disabled values.
Presets remain shared, with only the supported graphics applied in native JP.
Texture-based lamp discovery, combat-effect lights, blood/water enhancements, hair/cloth physics,
camera-relative controls, rewind, randomizer, enemy-behaviour cheats
and benchmarks are **not connected to native JP yet**. Gaisen and its scenery
sandbox still provide those features.

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
and source draw/holster attachment rules. Scripted/projectile-only moves and custom-folder
bodies are not connected to the native runtime yet. A body without a supported
moveset keeps the host's moves. The standalone `play-t2-jp` preview needs a
remembered Gaisen disc; open it once through `play` to connect the gallery.

**F1 → Lighting** uses the shared dynamic-light and shadow renderer for native
stage flames, with the same intensity, ambient, radius, light/shadow counts,
resolution and softness controls. Modern High/Ultra use their existing settings;
Original keeps these effects off. These sources are independent of the source
camera's draw culling. **Sky lighting** provides manual sun/moon direction,
colour, intensity and shadow quality. **Time of day** can instead drive the light
and ambient colour through a day/night cycle. Both work in gameplay and 3D
cutscenes, use the shared terrain/model shadows, and remain presentation-only.
The visual clock follows simulation time, including pause and fast-forward;
changing models or artwork preserves it. Other combat/lamp emitters remain pending.

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
wetness/snow but rebuilds the surface cache and clears existing footprint decals.

**F1 → Camera** supports free flight (F2), speed/FOV, player or actor focus,
overhead views and a camera bookmark. WASD/QE move, Shift boosts and RMB looks;
controller/touch sticks work too. F2 or Back returns to the game view. This also
works while paused and in 3D cutscenes; gameplay input is withheld during free
flight. F10 hides the gameplay HUD and camera help. During missions, **Follow Actor**
tracks the selected actor, including after the player dies. RMB/right stick orbits;
distance, sensitivity, inversion and recentering use the shared camera controls.
Terrain and props keep the camera out of walls. Following does not steer the actor
or change the simulation camera; F2/Back restores the original view.

**F1 → Tenchu 2** contains the original JP mission selector, integration
diagnostics and return to the quick launcher. These controls are disabled with
an explanation outside native JP play. **F1 → Simulation** contains pause,
single-step and automatic cutscene speed.

Hold Backspace during a cutscene to fast-forward, or choose an automatic speed
in F1 (2×–16× or Unlimited). This includes prologues, chapter narration, disc
movies and in-engine scenes. Acceleration mutes audio; releasing it resumes
dialogue at the current position. An on-screen Fast-forward button is also
available for mouse/touch. Menus and gameplay remain at normal speed.
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
