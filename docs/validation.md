# Gameplay validation

The PS1 emulator is the reference for original gameplay. Optional settings such
as **Keep all enemies active** need separate tests: those modes do not exist in
the original game. A successful route is evidence for the updates it exercises,
not proof that every possible interaction works.

Ordinary-play validation uses recorded BizHawk controller inputs and the observed
retail draw-skip schedule, replayed through the port and compared at corresponding
source update boundaries. Reuse the capture/replay tools below; synthetic input
sent to the native window is not an ordinary-play comparison. Direct state
fixtures remain useful for isolated behavior and optional modes. Recorded retail
skips are reference inputs, never inferred from the PC renderer's performance.

[Level Studio](level-studio.md) has separate optional-mode coverage: all sixteen
retail challenges and twelve themes round-trip through its project format;
invalid imports, grouped undo/redo and deterministic event queues have portable
tests. Disc-backed fixtures exercise playtest completion, record eligibility,
exact campaign restoration and return to native T2's title. Mixed-game GPU
captures check scene visibility, source materials and animated weather while
serialized gameplay remains unchanged. These are controlled authoring tests,
not additional ordinary-play recordings. Private screenshots stay outside Git.

Custom-asset checks reopen exported maps without the source project, verify shared
immutable undo payloads, reject malformed files and dangling material/cue links,
and decode independent WAV, Ogg/Vorbis, MP3 and FLAC fixtures. Mixer tests cover
overlap, pan, paused audition, stopping and music independence. A disc-backed GPU
fixture renders an embedded textured OBJ with replacement artwork on and off,
then exercises its queued custom music/effect and campaign restoration. Native
strict Clippy and the WASM build pass; packaged notices and matching Symphonia
source inventories are verified. No retail-behavior claim follows from these tests.

Campaign Studio fixtures construct 178 source/hero/layout combinations: all T1
stages with both heroes and three layouts, and all available gameplay layouts
on the connected European and Japanese T2 discs. They check source cast mapping,
model imports and initial updates. Separate real-disc fixtures raise source
floors, compare rendered ray hits with Gaisen and native T2 collision queries,
and export/reopen the edits. Portable tests cover clipped floors, slopes, holes,
side walls, unchanged neighbours, moved collision layers and placement-following
undo. GPU before/after captures exercise edited T1/T2 maps, saved projects,
weather seeking, playtest/return and exact restoration of the previous game and
settings. These are editor/template checks, not campaign playthroughs or proof
of native T2 script compatibility. Original gameplay collision queries are unchanged.

`check-retail` supports T1's `MAIN.EXE` and bounded native Japanese T2 gameplay
through the same observer and TNRUN002 format. Both initialize once and stop at
the first divergence, without correcting state from later snapshots. T2 uses
recorded display decisions and activation budgets; normal PC play keeps nominal
ticks independent of renderer performance.

The T2 training receipt matches 2,320 completed updates, including idle patrol
activation, turning and walking, with 115 three-count retail display advances.
Checks include RNG, actor/motion/pad/inventory state, patrol counters, root and
bone transforms, terrain probes, collision slots/contacts and camera state.
Full RAM checkpoints also reproduce the original BizHawk recording at fields
12,436, 18,448 and 18,754. The last captured update is incomplete and is reported
separately. This is bounded gameplay coverage, not a campaign playthrough:
T2 menu/cinematic transitions, full AI/effect/item state and map-specific weather
producers are not yet covered by this comparer. Campaign progression has separate
controlled fixtures. T2 requires `--scenario` or an update limit; the video tape
export path remains T1-only.

The native JP campaign fixture starts each hero at the title with the same
persistent save and original unlocks. It traverses 231 sequence visits, 56
gameplay areas and 29 results screens (27 story missions plus both training
routes), letting prologues, chapter narration, 3D cutscenes, ending movies and
credits finish without skip input. It confirms equipment entry, records and
supplies saved once per result, Tatsumaru becoming available after Rikimaru and
Ayame, and return to the native title. Objectives use controlled exit positions,
boss HP and kill quotas; this is progression coverage, not a full ordinary-play
campaign recording. Separate fixtures exercise all 56 death routes and both
detection failures, unchanged progress on failure and the correct retry area.

The 2026-11-13 campaign audit also reruns the original-instruction references:
336 area entries, 600 area exits, 483 dispatcher routes, 25,348 mission-selection
updates and 600 results/reward cases. Menu checks cover locked Campaign versus
unrestricted F6 access, cinematic cancellation, checkpoint/rewind continuation
and the v0.32.0 quickstate menu-flag encoding. Logs remain outside Git in
`private/t2-campaign-audit-20261113`; source provenance is in
[T2 reuse notes](tenchu2-reuse.md#native-t2-campaign-return).

The November 13 item audit uses private BizHawk demonstrations for invisibility
and teleport, plus the original-instruction checks: 1,200 invisibility boundary
cases, 1,600 teleport targeting cases, 600 placement/cancellation cases, 240 leaf
sequences and 1,200 ribbon sequences. Disc-backed GPU fixtures cover activation,
expiry, failed use without a target and successful placement on a native T2
stage and a Gaisen stage under T2 rules. The success fixtures keep guard AI idle
to avoid combat interruption, and explicitly activate actors after their setup
teleport skips the source's distant activation band. Existing world tests cover
moving targets, guard perception, item restrictions and snapshot continuation.

Captured rendering leaves the complete serialized world unchanged. Before/after
item observations match across 5,700 ticks, including an interrupted scroll on
the imported stage. Reviewed captures confirm the cloth's source depth placement
and textured trails. Videos, exact retail setup writes, source disassembly and
logs remain in `private/t2-item-review-20261113`; they are controlled item evidence,
not a new ordinary-play route or a claim of pixel-perfect PS1 rendering.

A follow-up body comparison reproduced mostly opaque T1 Rikimaru/Ayame overrides
and incorrectly concealed imported weapons. Matched poses and cameras on native
T2 and imported Gaisen stages now check both bodies against dark and bright
backgrounds, with equipment rendered separately. Native T2 output is unchanged;
increasing the backdrop by 96 levels increases every native-body output channel
by exactly 96, confirming the same additive strength on both stages. Normal
T1 body/equipment images also remain unchanged. Private before/after captures
and observations are in `private/t2-invisibility-models-20261113`. The fixture
passes 96 captures with nearest filtering and filtered replacement artwork.
Existing GPU blend/cutout and shadow tests cover the shared shader changes;
the guard-perception/snapshot test and 1,200 source item boundaries still pass.
These are isolated fixtures; no new ordinary-play coverage is claimed.

The imported-moveset attack-window check compares a basic swing for all three
T2 heroes on both original T1 bodies with native JP T2. The active updates,
motion frames, local contact offsets and sizes match exactly: Rikimaru has
three active ticks (100 ms), Ayame four and Tatsumaru five. The existing
guard-contact fixture also confirms damage and cleanup after switching movesets.
Both hitbox overlays read the active collision slots. This does not establish
identical world-space reach for differently proportioned bodies, all combos or
every blocked/clashing contact; the reported shortened basic swing was not
reproduced, so its gameplay timing has not been changed.

Sound-overlay checks compare region boundaries with the lesson controller's
quantized position checks, including negative and zero coordinates. Click
previews preserve the complete serialized world, and cover pause, free camera,
normal camera, menu consumption and photo-mode suppression. Mixer checks retain
the campaign audio playhead and game effect voices through preview, pause,
cancellation and scene changes. GPU captures exercise overlay visibility and
photo-mode suppression. Evidence remains under
`private/t2-sound-hitbox-review-20261113`.

T2 scenery-sandbox guard checks (#98/#99) run separately on Europe and Japan.
An authored route-less slot from sequence 4 is placed on a flat arena to compare
640 updates with the unseen player on opposite sides. Sword, spear and archer
controllers, Gaisen/T2 bodies and both activation settings yield identical idle
turns and brain state; moving into view still triggers detection. The existing
sequence-4 patrol/combat fixture exercises authored routes. This is controlled
adapter coverage, not original T2 AI parity or an ordinary-play recording.

The health check loads sequence 4 with all three sandbox guard choices, both
heroes, all three difficulties and both body settings. Starting/max HP must match
the selected Gaisen definition. A separate 72-encounter fixture checks actual
melee contacts: T2 bodies retain the same damage, HUD bars report the real HP,
armed guards survive the first strike and unalerted guards retain source kills.
Gaisen's existing `DamageControl` comparison against original C provides the
damage-rule reference. No health increase or native T2 combat change is implied.

The large-group reinforcement regression (#77) uses all three Mission 2 layouts, both
heroes, Hard difficulty, whole-level activation and no population cap. Three
authored callers request groups of 1, 4 or 32. The maximum cases retain all 96
recruits (115, 121 and 117 total actors by layout), check separate starting body
bounds and run 2,400 updates; surviving recruits must finish on collision ground.
The scored enemy total also receives every recruit. Before correction,
stacking 32 guards at each caller pushed many out of bounds within the first few
updates. Calls are triggered directly at the original queued-call boundary;
subsequent movement, collisions and combat use normal updates. This is a
controlled stress fixture, not a recorded ordinary-play route or a guarantee
that source movement can never clip. Separate fixtures cover closed/open solid
barriers, disconnected ground, cover, capacity, random counts, nonrecursive
recruitment and the original single-guard placement.
Layout 1 produces 110 scored enemies, matching the report's results-screen total;
the actor roster also contains the player and non-enemy actors.

Native T2 vegetation and snow tracks have separate presentation fixtures. All
23 available Japanese maps are checked for classified vegetation, bounded
geometry and unchanged displayed texture pixels. An autumn-map fixture compares
180 effect updates with frequent and sparse redraws, then restores a checkpoint
and rebuilds presentation; particles and movement scatter continue identically
without changing source state. An imported T1 Mission 5 fixture verifies visible
snow tracks with Original weather, persistence and clearing. A GPU check gives
the imported texture and active VRAM contradictory snow masks to verify that
tracks sample the terrain actually being drawn. Before/after captures and logs
remain in `private/two-releases-20261110/native-vegetation`. These controlled
fixtures do not add ordinary-play or campaign coverage.

Optional native T2 walking has controlled fixtures for all three heroes, selected
Gaisen/T2 moveset overrides and an imported T1 stage after its cinematic. They
check actual walk speeds, restoration of each moveset's run, committed actions,
stick hysteresis, modifier input and suppression when controls are inactive.
Checkpoint continuation compares complete session bytes; the save/rewind tests
also retain the option and recorded walking input. Default-mode validation reuses
the existing BizHawk training capture and original standing-controller oracle.
These are optional-mode fixtures and regression checks of existing coverage,
not a new recorded ordinary-play route.

Native T2 manual sheathing fixtures cover original and imported source motion
events, audible weapon samples, committed actions, scripts, mission input locks
and water entry. Speed checks use actual running/walking velocities; damage checks
use the native melee/projectile resolver, including protection and resurrection.
Live guard sight and its overlay verify stowed detection scaling. Native injury
and knockdown recovery checks retain the original weapon state. Saves, rewind,
recordings and imported-stage continuation include the optional settings and input.
These are controlled feature checks; source-default regressions reuse the existing
damage/AI/controller oracles and BizHawk training recording.

T1's jump-over sheathing regression (#94) starts NinjaA's actual forward jump and
runs collision/recovery on a flat arena. Both heroes, front/back contact, a missed
jump, armed/stowed weapons, optional unarmed mode, recovery preservation and guard
drawing are checked in 96 combinations. Health remains unchanged; both source
hurt reactions occur. Recovery preservation prevents automatic drawing, while a
subsequent deliberate guard still obeys its own setting. This is an isolated
fixture, not an ordinary-play recording. The contact and default draw behavior
follow decomp `HUMAN.C` and `MOTION.C`'s `ActDAMAGE` completion.

Native optional unarmed checks exercise both styles on all three heroes, source
combo progression, striking-limb contacts, directional/crouch/dodge entries,
recovery heading, retained weapons and armed-moveset fallback. Controlled damage
fixtures place a native target in an active limb's contact volume on a flat floor
and use the original collision/damage path. Save/rewind, model overrides and an
imported T1 stage are checked separately. Presentation checks report hits from
both styles and require stowed blades to stay clean, then verify drawn attacks
still coat blades. These fixtures do not establish a new ordinary-play route;
source-default validation reuses the existing recordings.

Native dismemberment fixtures cover heads and both forearms on all three heroes
and a native guard, flat-floor support in three resting orientations, and exact
midair checkpoint continuation. Actual front/rear paired kills check the native
blood timing and direction exclusion. Disabled decisions preserve the rest of
the world byte-for-byte; removed/revived corpses restore their hierarchy, and
invalid detached checkpoints are rejected without mutation. The imported
Rikimaru Mission5 execution is exercised on/off with all three native heroes.
GPU fixtures use native weapon damage to cause death, then check native/imported/
custom bodies, finite blood, redraw independence, Clear blood and presentation
restore. Review captures use a plain receiver floor and live under
`private/two-releases-20261110/native-dismemberment`. These are controlled optional
feature tests, not additional ordinary-play or campaign coverage.

## Native T2 cutscene replacement contact

The #116 regression reproduces replacement knees/feet below the courtyard in
the three protagonists' opening scenes (events 201, 2011 and 102). Across 600
updates per scene, the previous renderer differed from the authored body's lower
edge by up to 167.3 source units. Matching captures show the replacement boots
and knees emerging from the ground after correction; all three Original-body
controls remain pixel-identical.

The expanded check covers those intros and both training trapdoor falls (2021
and 2031), with Original, T1 Rikimaru/Ayame, T2 Rikimaru/Ayame and an exported
custom body. Both refinement settings and current/half-interpolated poses pass
in all 60 combinations, with at most one source unit of contact-height error.
The fall fixtures require actual vertical motion exceeding 1,000 units, and
rendering preserves the serialized story state. The existing body/weapon override
check also retains the cast, attachments, playback and reset behavior. This is
controlled presentation verification, not an additional ordinary-play recording.
Captures and logs stay in `private/bug-fixes-20261112/116-*` outside Git.

## Emulator comparisons

The combat/item searches under `~/tenchu-searches/ayame-mission-*` provide
immutable input receipts, pre-menu states, loadouts and endpoint RAM hashes.
The [capture tools](../tools/retail-capture/README.md) replay these in BizHawk,
verify all RAM at the archived checkpoints and observe the real draw skips.
The native comparison initializes once at mission entry; it never corrects
native state using later emulator snapshots.

| Completed reference mission | Matching updates |
| --- | ---: |
| 1 | 3,199 |
| 2 | 1,323 |
| 3 | 3,121 |
| 4 | 3,770 |
| 5 | 4,078 |
| 6 | 2,568 |
| 7 | 1,016 |
| Total | 19,075 |

These comparisons include actors/poses/AI, collision slots, inventory, equipped
armour, active item processors and projectile state, RNG, camera, draw budgets,
alert/radar state, kills and mission results. The selected Mission 7 receipt was
`adc0ff53d80d201ac9a9b65327eafa5def4f67f87004b08b29e08d80022eaee8`;
the finished archive now lives under that search's `final/` directory.

Additional bounded experiments also match every observed update:

| Experiment | Matching updates | Actual coverage |
| --- | ---: | --- |
| Mission 3, unequipped route | 1,866 | Detection, repeated damage, player death and actor control afterward |
| Mission 3, equipment cycling | 782 | One use each from inventory slots 1–8 |
| Mission 3, seeded mixed inputs | 933 | Movement, jumping, crouching, attacks and detection |
| Mission 1, altered route | 922 | An actual stealth kill, without a detection increment |
| Mission 6, altered route | 555 | Close approach and an attack attempt; no stealth kill |
| Mission 7, altered route | 575 | Close approach and an attack attempt; no stealth kill |

Prefixes overlap the completed routes and are not additional unique coverage.
`--scenario` checks unfinished missions/deaths and explicitly reports a pending
display boundary if the recording stops during the PS1 display wait. Partial
scenario recordings omit mission results and retain only finished updates.
Custom experiments can use different pre-mission loadouts and text input
segments; see the capture tool's instructions.

This work corrected an equipped-armour replay initialization error and a real
gameplay divergence after death: the stage-end halt value and guard sheathing
transition. Original-source checks also cover sight thresholds/occlusion queries,
reinforcement replacement, all six protagonist stealth-kill pairs, and all four
Mei-Oh lightning attacks on both protagonists, including imported bodies.

Captures, RAM, disc data and review images stay in ignored local directories.
The local evidence for this pass is `private/parity-20261030/`, with input/RAM
hashes and checkpoint results beside each capture.

## Reported Mission 3 behavior

The reports came from ordinary play in a released build, with the exact version
and settings unknown. They have not been reproduced as default-mode divergences
in these captures. They remain unconfirmed, rather than assumed fixed or invalid.

- **Seeing through walls:** original sight uses the terrain passage query. The
  source comparison includes blocked queries and their exact step arguments;
  a separate comparison runs 32,772 rays through actual Mission 1, 3, 4 and 10
  collision maps against `CONFLICT.C`, including indexed rows and their inclusive
  edges. This does not establish that every visible wall has retail collision.
- **Repeated respawns:** the original AI deliberately creates reinforcements.
  Replacement/order tests pass; this does not identify what happened in the report.
- **Radar staying above minimum:** the original calm phase still contributes
  the nearest actor's distance to radar strain. Calm enemies nearby can therefore
  leave a proximity reading. A stuck alert indication is a separate question.
- **Spotted around a stealth kill:** the captured isolated kill did not increment
  detection. Other guards and corpse-witness behavior need the particular scene
  to judge; a stealth kill is not general immunity from those systems.

If one recurs, save a [diagnostic report](diagnostics.md) immediately. Its settings,
actor/AI state, mission/layout and recent inputs help distinguish these cases.

## Cross the Checkpoint body witnesses (#16, #17)

The November 3 follow-up identifies Mission 4, with group reactions during a
stealth kill past the bridge toward Senjuro. A controlled native setup near the
northern watchtowers reproduces body discovery during Ayame's execution in both
ordinary and all-active modes. This relocates one victim onto a flat, valid tile;
it is not a replay of the reporter's route.

With ordinary activation, guard 15 discovers the victim on update 45 while the
execution is still running. The shared alarm becomes 300, but the player-spotted
counter remains zero. Nearby guards become suspicious when the execution ends.
All-active mode lets guard 25 discover the body earlier, on update 27. Marking
the body already discovered as a control removes the alarm during the execution;
ordinary player detection still works afterward. The regression preserves this
distinction between body discovery, shared suspicion and spotting the player.

`THINK.C:register_character_death` uses a different ray from ordinary player
sight. It starts at the witness's feet and ends at the corpse's root position
minus the witness's standing height. It does not trace from the witness's eyes
to the animated body. At the ordinary-mode discovery above, that original ray
is clear while an eye-to-body ray is blocked by the same terrain collision map.
The default uses the original endpoints. **Strict corpse visibility** adds a
second ray from the witness's eye height to the animated body's root part, using
the same terrain collision map at smaller steps. The opt-in regression hides the
Mission 4 body behind cover and still discovers the same posed corpse on open
ground. It does not make decorative meshes into new collision obstacles or
suppress alerts from genuinely visible bodies. Body checks still run during the execution,
before the victim finishes falling, as in `HUMAN.C:ControlHumanoid`.

A local comparison ran 99 controlled cases through the original C corpse
routine **and** its original terrain functions on Mission 4's collision map.
All matched native discovery flags, alert duration, notice/draw calls, chase
coordinates and witness selection, including the two witness/body arrangements
from the execution. The direction and square-root inputs came from the separately
validated native PS1 math; sound and motion calls were recorded by stubs. This is
a host-C comparison, not a new emulator input replay. Evidence and review images
are retained outside Git in `private/issues/detection-16-17/`.

The passage regression now includes both ray variants from those arrangements
and Mission 10 explicitly. Earlier notes incorrectly called internal stage 9
"Mission 10"; it is Mission 4. Mission 10 is internal stage 7. The original 24,576
rays covered Missions 1, 3 and 4; the expanded check covers 32,772 rays.

### Controller-only BizHawk follow-up

Two new Mission 4 routes now reproduce corpse alarms in the actual PS1 runtime,
with ordinary actor activation, Ayame, Normal difficulty and layout 1. They
branch from the equipped search's verified receipt
`d2542abfb69f02cb987105fc5293bfedb60c69293834a03bb6fadf5550b8bc79` and retain its
approach before adding controller inputs around the northern watchtowers. All
four inherited checkpoints match the archived 2 MiB RAM exactly. The archived
pre-mission item setup supplies the loadout; neither experiment relocates actors
or patches in-mission RAM. Both initialize RNG to `0x38665927`.

| Route | Matched updates | Recorded draw skips | Observed behavior |
| --- | ---: | ---: | --- |
| Discovery during execution | 4,005 | 8 | Guard 15 discovers victim 25 at clock 2534, during Ayame's stealth kill. Alarm becomes 300; times spotted stays zero throughout the captured aftermath. |
| Discovery then direct sight | 4,078 | 11 | The same guard discovers the victim at clock 2677 after a different stealth-kill animation. Alarm becomes 300 with zero times spotted. At clock 2737, direct detection increments times spotted to one. |

The routes include the same approach; their update counts are not unique
coverage. The native checker matched every selected-state comparison from mission
entry, using recorded input consumption, draw skips and swaps, with no later
state corrections. Both captures end at a completed display boundary. The
discovery display fields are 9387 and 9672; direct sight in the second route is
field 9792. This distinguishes a shared corpse alarm from the separate detection
counter instead of inferring both from the radar icon.

The original `StateTransition` also explains guards waiting during an execution:
`ActionHalt` allows turning/backing away but bypasses normal attacks. Corpse
registration runs independently of that halt. These checks can therefore raise
an alarm while nearby guards wait for the kill animation to finish.

Reconstructing the second discovery's rays through the original terrain routine
finds a clear original feet-to-elevated-root ray, while an eye-to-root-minus-300
diagnostic ray is blocked. **This is not proof of a fully hidden corpse being
seen through a solid wall:** the free-camera witness-eye view still shows parts
of the corpse. The overlay is reconstructed from recorded coordinates, not a
ray drawn or logged by the emulator. These two routes do not reproduce a
spotted-count increment *during* the execution.

Local evidence is in `private/parity-film-16-17/`: `case-a3` and `case-b4` contain
input/provenance records, traces, validation reports and partial native tapes.
The 4K/60 side-by-side viewing edit uses Original graphics/artwork and labelled
analysis freezes. Its console soundtrack was captured separately, with the
decoded event/RAM streams verified identical to the corresponding video runs;
it does not validate native audio. Diagnostic views disable fog to expose the
geometry. Recordings, RAM and extracted artwork remain outside Git.

### Brief red indicator at the start of a stealth kill

The reporter subsequently described a very brief `!!` flash as the kill begins,
followed by `!?`. A third controller-only Mission 4 route (`case-c1`) reproduces
a **one-update red indicator** at kill start in BizHawk and the native port.
All 4,159 checked updates match, including 8 observed draw skips. The same four
archived checkpoints match all RAM; no in-mission memory is changed.

This route was selected by searching variations of movement and attack timing
for a red indicator at execution start. The search accepted that visible symptom
without requiring a successful sight check or an increased spotted counter.
The no-attack control below establishes that it selected a coincidence: the
archer's combat transition occurs independently of the execution. Consequently,
this case demonstrates a different retail behavior with a superficially similar
indicator; it is not a reproduction of a stealth kill causing detection, shared
alert or a scoring penalty, and must not be used as evidence that the complete
reported encounter has been explained.

| Gameplay clock | Radar strain | Times spotted | Stealth kills | Execution halt |
| ---: | ---: | ---: | ---: | --- |
| 2646 | 791: proximity | 0 | 0 | Off |
| 2647 | 0: red alert | 0 | 1 | Off |
| 2648 | `0x7fffffff`: no indicator | 0 | 1 | On |
| 2776 | `0x7fffffff`: no indicator | 0 | 1 | Clears late in update |
| 2777 | 0: red alert | 0 | 1 | Off |
| 2806 | -32768: searching | 0 | 1 | Off |

The weapon draw itself comes from idle navigation, not a successful sight check.
Guard 11's recorded idle function is `Think1chase` (`0x8002c39c`, native `hunt`).
When `GotoPosition` returns no movement from its blocked-terrain branch, this
routine issues Square and resets its idle counter. At clock 2634, the capture
shows the wall flag, a new Square input (`0x0080`), the counter resetting to zero,
and the walking motion (`0x200`) changing to weapon draw (`0x80e`). The AI phase
remains calm until the draw finishes. These observations and the original source
identify the blocked-navigation path, rather than an enemy witnessing the kill.

The initial retail layout assigns this archer think mix `0x4448` and no patrol
waypoints. Its idle function is already `Think1chase` at mission start and never
changes during this capture. Of the 25 placed actors in this layout, it alone
uses that routine; 15 watch, five follow patrol routes, three use the separate
random-wander routine, and one sleeps. Thus this is not a general response of
guards to blocked movement. In particular, `Think1random` resets its destination
counter on encountering the wall flag instead of issuing Square there.

At clock 2527 the archer has no other actor within the 5,000-unit acquisition
radius. Its destination `[-40654, 144844]` is its spawn X/Z `[-40072, 140245]`
plus random offsets `[-582, 4599]`; these match the next two draws from the
recorded RNG seed `0x3f0505b3` exactly. It walks toward that point, reaching the
edge of the raised ground before drawing at 2634. This is its first weapon draw
in the capture, about 55 seconds after activation at clock 990. The source does
not validate the random destination's reachability before choosing it. The
recorded activation history, random destination and approach explain this
particular occurrence; its frequency across ordinary play has not been measured.
Initial assignments and the complete archer timeline are retained privately as
`archer-initial-ai.json` and `archer-provenance.json`.

Temporary native branch logging, with all 4,159 updates still checked against the
console capture, confirms the navigation branch directly: clock 2634 has the
wall flag, direction 13, and a missing forward terrain level. Both side probes
return 260; no turn input is requested, so `GotoPosition` returns zero. This is
a deterministic collision/navigation result, not a random detection roll. The
archer's scheduled sight checks at clocks 2613 and 2645 both return `Gone`.

The red icon is not itself a reliable detection-counter readout. Here guard 11
finishes drawing its weapon on clock 2646 while its AI phase is calm. The retail
`ActSTATE` completion branch sets `ATTR_SEARCH | PHASE_ALERT`, without incrementing
`Findenemies` or setting the shared alarm. Its next AI update contributes zero
radar strain, selecting the red icon. The execution's action halt begins on the
next update and suppresses those normal AI contributions. The killed actor is
guard 25; the transient red indication comes from guard 11.

An empty radar does **not** mean the guard forgets the player. Each update resets
the radar aggregate to its no-indicator value; halted guards return before
contributing their normal readings. Guard 11 retains its combat phase and player
target throughout the execution. After the halt ends, the red indicator returns
at clock 2777, the guard starts an attack at 2790, and it changes to the search
phase at 2805 (the radar reflects that on 2806). The spotted counter and shared
alarm remain zero through the capture's final clock, 2849. Thus this guard is
hostile despite the absence of a scored spotting event; the execution temporarily
suppresses AI/radar processing, not the stored alert state.

Guard 11 is an **archer**, and its attack is a bow shot rather than an out-of-range
melee swing. At clock 2805, the next scheduled sight check returns `Gone`: the
player is roughly 3,760 units lower and the original sight rule, with the shared
alarm at zero, rejects height differences of at least 3,000. Changing the AI phase
does not cancel the ongoing bow animation. It releases an arrow at clock 2828,
after entering search.
The projectile target ray stops at its own launch point,
`[-40354, -12051, 143147]`, without selecting a character target. On clock 2829,
the arrow changes from flying to embedded/waiting with the original terrain
status `OutsideTerrain`; its position does not advance. Both the native and PS1
item states agree, and Ayame remains at 100 HP. Thus this shot fails through the
original terrain handling, not simply because the archer is too far away.

A console control (`case-c-control`) replaces only the attack-button interval
with crouch. It also matches 4,159 native updates and 8 draw skips. The indicator
still turns red at clock 2647, but stays red on subsequent updates because no
execution halt starts. Both the spotted counter and shared alarm remain zero
across this interval. This establishes that the red transition in this case
does not depend on the kill or corpse discovery.

The captured PS1 image shows red on console fields 9615–9616, about 33 ms: one
gameplay update, two video fields. The update's EndDrawing return is field 9613;
the console capture presents it two fields later than the native recorder.
`stealth-kill-red-flash-comparison.png` aligns the displayed images and labels
that offset. The HUD uses the Japanese red `殺` symbol in these captures.

This confirms a brief red flash can be original behavior without a spotting
penalty. It does **not** yet reproduce the complete reported red-to-`!?`
transition with group alert and a spotted-count increment. The third route and
control, radar timelines, screenshots and hashes are retained alongside the
first two routes in `private/parity-film-16-17/`. The extracted weapon-draw and
aftermath observations for both captures are in
`red-flash-draw-and-aftermath.json` there.

`tenchu-red-flash-explained-4k60.mp4` expands this third route with a location
overview, simultaneous player/archer views, normal-speed playback, slowed
sections, and labelled analysis freezes. Persistent labels distinguish Ayame,
victim 25, archer 11 and the background guard 15 (which stays calm). The cards
separate AI phase, animation, execution halt and the spotting/alarm counters.
Diagnostic terrain/actor views reconstruct presentation only; the arrow marker
comes from the matched item position. The closing PS1/native stills account for
the console presentation delay. Its edit, extraction, branch logs and checks are
retained in `private/parity-film-16-17/c1-explanation/`; no instrumentation remains
in gameplay code.

A further retail replay observes the archer without a nearby fight or execution.
After the approach, Ayame turns toward the ridge and holds crouch + L1 look.
From console field 7684 through 11523, every controller input is `0x0401`, and
her position remains `[-38335, -7000, 147755]`. The archer remains unharmed.
The approach is not pacifist: the reused route kills the entrance opponent with
five shuriken and uses three Lightfoot Scrolls before reaching the vantage point.
Those actions are included in the video, not cut out. There is no stealth kill,
additional dust/RNG steering or in-mission RAM patching in this follow-up.

| Gameplay clock | Archer state | Times spotted | Shared alarm |
| ---: | --- | ---: | --- |
| 2836 | Blocked movement requests Square; starts drawing, still calm | 0 | Off |
| 2848 | Draw completes; enters combat | 0 | Off |
| 2849 | Radar turns red | 0 | Off |
| 2869–2870 | Enters search; radar turns blue next update | 0 | Off |
| 3221 | Search ends; returns to calm | 0 | Off |
| 3317 | Later, a real sight check succeeds after coming down from the ridge | 1 | On |

`private/parity-film-16-17/tenchu-archer-observation-retail-4k60.mp4` shows the
continuous mission-entry replay with labelled fast-forward and slow motion,
game sound, the held inputs, an archer marker and read-only state annotations.
It includes the later genuine detection and attacks. This establishes that this
archer's unscored red transition can occur during stationary observation; it
does not establish its frequency or reproduce the reported stealth-kill penalty.

The standalone `watch/tape/tenchu-m4-observe-archer.bk2` embeds a mission-entry
state and contains 9,997 console fields. Ordinary Nymashock movie playback,
without tool-supplied inputs or game-memory writes, matches all 2 MiB of RAM at
four checkpoints, including the end. The entry state was recreated without CPU
debugger callbacks and matches the instrumented capture's RAM exactly. The
stocked menu setup is retained from the archived search. Tape instructions,
hashes, the raw console recording and the verification logs remain private.

Before the November 12 terrain correction, native comparison matched the first
4,743 updates through clock 3433, including the unscored red and later real
detection. Another spearguard's contact state diverged at 3434. The corrected
bounded lookup now matches **all 4,912 observed updates**, including eight
recorded skipped draws, through the end of this recording with no state
corrections or pending display return. The earlier differences remain under
`watch/`; this video uses console footage only.

The November 12 investigation identifies an original out-of-bounds **read** in
that terrain query. Read-only BizHawk instruction observations reproduce the
same 11,523-field input sequence and match all four archived checkpoints. Their
final 2 MiB RAM is identical to the earlier capture. All 1,910 visited node
addresses and Z-boundary decisions match an isolated execution of the original
MIPS routine using the captured RAM.

At clock 3434, the guard's corner probe is `(-36838, -7000, 151380)`. Integer
conversion gives ACM coordinates `(-3683, 15138)`. Row 42 includes its maximum Z,
15138, but divides the span into four buckets without clamping: the resulting
quadrant `(1,4)` produces flat index 17, beyond its sixteen-entry lookup table.

- The table starts at `801a6410`; the short read at `801a6436` is the high half
  of row 51's relocated list pointer, `801a7b68`.
- The original signed `LH` at `80019cac` interprets `801a` as **-32742**. Adding
  that index times 16 to row 42's list `801a75d8` points to `80127778`, inside
  Ayame's animation allocation (`AYAME.AMD`, offset 48392).
- The routine examines 1,844 unrelated node-shaped locations, making 3,897
  halfword loads. None contains the query. It stops at `8012eaa8` when the
  supposed node's Z minimum is read as 31064, greater than the query's 15138.
  That value is actually the `7958` texture-palette word in `NINKEN.MAD`, part 0,
  object 0, primitive 3, at file offset 354. The allocator links and disc bytes
  independently identify these unrelated allocations.
- The routine then finds the valid slope in row 51 and returns **-8000**. Its
  writes are confined to its stack and normal terrain-query cache globals;
  this case establishes a stray read, not corruption of those allocations.

The port's unrelocated file offset has a zero high half, so it starts at row 42's
node zero instead. It finds a different slope at **-8080**, then rejects that
height because the drop exceeds 1000 units. This sets the guard's wall bit and
changes its turn/input. Substituting zero for the original lookup reproduces
that result. Clamping the quadrant to `(1,3)` also rejects the floor; substituting
an empty lookup instead continues to row 51 and returns -8000.

The port now treats flat lookup indices outside `0..16` as empty and continues
the existing row search. Flattened X-edge aliases that remain inside that table
retain their source behavior. This explicit bounded policy removes dependence
on neighboring pointer bytes; it does not reproduce arbitrary PS1 heap reads or
promise their result in every possible case.

The recorded guard-contact regression fails before the correction and passes
after it. The whole watch recording, the new 360-update cultist recording and
seven earlier Mission 1–7 recordings (19,075 updates) all match their captured
states/draw cadence. Source-C movement probes, sight-passage checks and the
ordinary core tests pass. The sight harness explicitly supplies empty padding
past the valid lookup table; it does not claim original out-of-bounds parity.
Evidence and private observer sources are under
`private/bug-fixes-20261112/17-*`; no RAM or game data is committed.

The terrain correction does not establish the separately reported scored
stealth-kill detection. The sight checks establish matching corpse alarms for
these routes and the inherited ray geometry, not every reported encounter.
The scored-detection report (#17) remains open. A more natural corpse sight ray
would be a separate optional gameplay change; substituting rendered-triangle
visibility would also change which original scenery blocks sight.

## Mission 6 cultist death cry (#52)

The November 12 recording branches from the archived Ayame/Normal Mission 6
entry and uses controller inputs to cancel a speed scroll near the first cultist,
then kill him with a sword strike. It changes no in-mission memory. The native
checker matches all 360 observed updates, including one recorded skipped draw;
the final display return is pending, so the exported tape contains 359 finished
updates. The entry checkpoint and the two observer runs' endpoint RAM agree.

Read-only `PlaySE`/`SsUtKeyOnV` observations show the scream (`0x68`) starting at
clock 65 in voice 16. The 24th subsequent sound, ambient cue `0x44`, reuses that
voice at clock 149, about 2.818 seconds later. The isolated sample lasts about
4.003 seconds. The original therefore also interrupts this cultist's scream;
there is no voice-lifetime change to make for this reproduced case.

The audio regression replays the captured draw cadence through `Audio::tick`
and checks all 55 cue IDs, logic clocks, volumes and stereo positions against
the retail observation, plus the scream and its channel-reuse event. Existing
mixer checks cover 24-voice rotation. This is not a claim of sample-exact audio
parity. The reporter's original kill and comparison edition remain unspecified.

A separate sound-position bug preceded the kill. At clock 38, the cultist's
footstep uses emitter `(-792,4830,-35202)` and listener `(2698,7300,-44583)`,
with yaw 2221. Read-only instruction observations show the player's ordinary
control moving it to that listener position before the footstep. The speed
scroll then moves the player to `(2829,7060,-44101)` during item control.
Resolving the queued footstep after that movement produced volume/pan 44/38
instead of the original 41/39. Capturing emitter and listener coordinates at the
actor sound call corrects all 55 events without changing movement or audio math.
Both instrumented recaptures reproduce the entry checkpoint and final RAM.

The regression also restores the in-flight event through both savestate and
rewind paths, verifies that later actor/camera movement cannot change it, and
checks explicit listener overrides used by native T2 scenes. These counterfactual
checks operate on restored copies; the recorded playthrough receives no state
corrections. Positional-effect listener timing and sub-tick SPU playback remain
outside this fix.

Run `recorded_cultist_death_retains_retail_sound_sequence` with `TENCHU_DISC`
and `TENCHU_CULTIST_REPLAY` pointing to the private verified replay. Captures,
observer sources, sound-event comparison and checks are under
`private/bug-fixes-20261112/52-cultist-*`; no game assets or RAM are in Git.

## Separate always-active-enemy audit

Keeping distant actors active exposed assumptions normally hidden by PS1 actor
activation. Squared distances could overflow and make faraway actors look close;
the global alert could also promote remote patrols into suspicion and let them
affect the player's radar. This mode now uses wide distances where necessary,
and limits alarm/radar contributions to the player's neighborhood. Distant
patrols continue their AI instead of freezing under every local alarm.

The Mission 3 regression now exercises all seven distant guard patrols together
for 600 updates. They keep patrolling, do not enter suspicion, and do not prevent
the local alarm from expiring. A separate host regression flies F2 away and back
in Missions 1, 3 and 10, with ordinary/all-active enemies and camera-relative
controls enabled, comparing AI, HUD and final gameplay state with a stationary
camera reference. It does not replace GPU/display testing or identify an unknown
reporter's exact route.

Regressions cover distant target selection, extreme coordinates, local arithmetic,
remote patrol motion, alarm expiry, radar and population stability. Both heroes
also pass 300 updates on every mission/layout combination, including training.
The option remains separate from original gameplay, whose seven reference tapes
still match.

The later [reinforcement report (#42)](https://github.com/Fuuzetsu/tenchu-shinobi-gaisen-pc/issues/42)
has a confirmed all-active failure mode. In the [reported video from 4:34](https://www.youtube.com/watch?v=iPSbiNP1Ga4&t=274s),
successive callers accumulate near a wall. Retail's `think_alarm_reaction_`
returns both a caller and its new guard to the approach/search state. Reaching
the search point **or hitting a wall** can select another call: never on Easy,
half the time on Normal, always on Hard while capacity allows. New arrivals
need not see the player to repeat this search reaction. Calling also refreshes
the alarm, so a stuck group can sustain its own alert.

Retail stops new call decisions at 30 total actors, including the player,
civilians and corpses. This is not an exact spawn-time cap: already queued calls
can overshoot. `CreateHumanoid` separately rejects allocation at 40 actors.
`ActivateHumans` suspends distant actors beyond 17,000 units and uses a 3–6
thinking budget for other activation decisions. The call distance is 16,500
units. Fog hides distant activity; it does not itself authorize calls.

That 30-actor threshold covers the **whole mission**, including inactive and
story actors, not just the current fight. In the archived PS1 campaign's first
layouts, Mission 1 starts with 28 slots occupied, Mission 3 with 31 and Mission
10 with 32. Retail can therefore reach the reinforcement threshold after only
a few arrivals, or already be above it before combat; it does not require a
30-enemy crowd around the player. These counts come from the emulator replay's
mission-entry actor table, also matched by native loading. Later actor removals
can free capacity.

Nor does a successful call immediately request another. Both guards return
toward the reported disturbance; sighting the player changes the suspicious
search phase to combat, and expiry of the alarm can return it to calm. In an
open-ground Hard test, a caller at 16,600 units with a search destination at the
player summoned one guard, both reacquired the player within 32 updates, and
the population stayed at three (player plus two guards) for 900 updates. It also
stayed at three when all-active was enabled after that first call. A separate
blocked-guard fixture placed immediately before an activation check froze under
the original policy, but multiplied with all-active. The earlier explosive
fixture deliberately set the search destination at the distant blocked caller
and started just after an activation check; it is not a representative first
encounter. A blocked return path, sustained search state, activation timing and
available population capacity matter in addition to the difficulty setting.

Before the nonrecursive-call fix, all-active + Unlimited removed both the distant
suspension and the call decision limit. The extra terrain-cover check hid spawns but could not stop recruitment
feeding back into itself. A controlled, disc-backed wall fixture with one guard,
Hard difficulty and ordinary `Game::step` updates reached 570 total actors after
18 updates; the same fixture with the all-active limit at 30 stopped at 30.
This is a synthetic stress reproduction, not a replay of the reporter's route
or crash. The extracted retail alarm routine confirms the decision threshold
and queued-call overshoot; the existing compiled-C activation comparison passes.
YouTube preview frames were reviewed, but the full video stream was unavailable.

All-active now allows one call per placed guard per alert. A caller rearms only
after returning to calm with the shared alert ended; summoned guards permanently
retain their reinforcement role and cannot recruit, including after civilian
replacement or randomized actor imports. This leaves their investigation and
combat AI intact. F1 → Simulation configures a fixed 0–32 recruits per call or
a random 1–N count; zero disables guard calls. The optional population limit is
checked before every spawn, including within a group. Settings changes do not
reset caller eligibility. Original mode ignores this policy, count and randomness
and retains its original call/RNG order and population threshold.

The blocked-corner regression now stays at three actors (player, caller, recruit)
through 920 updates with Unlimited and the default count. Fixed groups, randomized
groups, zero calls, group clipping at a population limit, rearming after calm,
settings changes, civilian replacements and imported reinforcement attacks also
pass disc-backed checks. The archived PS1 campaign still matches all 33,404
updates; compiled-C activation and combat comparisons pass. These remain separate
from reproducing the reporter's exact route or Windows crash.

## Legacy PC route fixtures

The 2026-11-01 issue batch rechecked the public PC input examples as well as the
emulator references. Both training tapes now include machine-readable launch
metadata, fixing direct replay with the wrong fallback equipment. Both training
routes and the campaign tapes listed in the [guide](guide.md) complete through
the standalone replay command. These are useful port-integration examples, not
console recordings.

The 2026-11-12 audit ran all twelve route tests: six passed and six failed.
The passing `controller_route` filter now covers both training heroes, Ayame's
delivery, and Rikimaru's checkpoint, cult, mountain and castle routes. The six
failing tests live in `campaign::tests::routes::legacy`, with their tapes and
original assertions retained. Run that filter explicitly with `--ignored` to
reproduce the unresolved failures:

- Rikimaru campaign: Mission 6 starts with two grenades instead of the asserted three.
- Ayame campaign: dies in Mission 4 at tick 10,371.
- Rikimaru rescue: dies at tick 3,677.
- Punishment: dies at tick 6,472.
- Rikimaru pirates: reaches scenes 10 and 20, but not 100.
- Rikimaru final: dies at tick 4,567.

The previous audit reproduced five of these failures on v0.9.10; punishment is
an additional failure recorded in this audit, without a historical baseline
claim. Their causes remain unisolated. Separating the fixtures does not fix or
validate them; do not count them as passing or change gameplay to force these
PC-authored paths.
Standalone Ayame Mission 4/10 and both Mission 7 campaign tapes also fail to
complete, and are excluded from the verified examples.

The independent PS1 references still match all 33,404 full-campaign updates,
19,075 equipped Mission 1–7 updates, and the checked Mission 1 stealth-kill,
Mission 3 item-use and mixed-input scenarios. These comparisons import initial
state once and apply no subsequent state corrections. This supports the
unchanged default gameplay on those paths, not every possible route.

## Independent subtitle language

The subtitle regression compares four live Mission 1 scenes against their
authored caption tracks, including Japanese's different line splits, its
32-byte display limit and French's consecutive waits at the same timestamp.
Caption lookup accounts for the original text-before-script-advance order.
Changing the displayed language preserves the source scene and world geometry,
including when outline HUD text is disabled. Optional captures use
`TENCHU_FONT_CAPTURE_DIR`.

The movie regression checks all 80 movie/audio/subtitle combinations frame by
frame. Alternate caption commands cannot emit voice cues; switching languages
at a held frame catches up the caption track without restarting narration.
The imported T1 scene rendering check covers both subtitle modes and verifies
the return to native T2 materials. The asset parser covers all 88 stage scripts.

The default still links captions to game language. Alternate tracks retain
their own authored timing; they are not resynchronized to each spoken word.
Audio language controls scene selection and duration, so a locale-only scene
is not synthesized for another language. Native T2 JP has no translated caption
assets and remains Japanese. Campaign saves are unchanged; mid-mission states
include the caption tracks and use format 41.

## Persistent mid-mission states

The savestate regression compares every serialized simulation field before/after
loading and after matching inputs with explicit draw skips. It covers training
and Missions 1–10 with both heroes, live grapple/shuriken/explosive/disguise
items, cross-stage actors, randomizer context and TRIAL playtests. Additional
checks cover accumulated blood/snow/contact effects and alternate death poses,
corrupt/truncated/version-incompatible files, unchanged campaign save data, and
audio seeking across resampling, looping and pause.

Native M1 play exercised both keyboard and F1 save/load, multiple slots, moving
and attacking after resume, and loading from the launcher after closing the
process. `tools/test-web-savestate.mjs` checks browser movement, save/load and a
complete browser restart using an isolated persistent profile. Its optional
`TENCHU_NATIVE_STATE` imports a desktop slot for cross-platform checks. The
2026-11-01 run passed with WebGPU and loaded a 64-bit Linux Ayame state at tick152
in 32-bit WASM. These snapshots are private runtime data, not test assets in Git.

The four disc-backed savestate regressions,182 ordinary workspace tests and
strict native/WASM Clippy pass. This verifies the saved simulation and tested
continuations; audio playheads are approximate and one-shot audio tails and
cosmetic spring/waterfall motion are deliberately not serialized.

## Optional rewind

The disc-backed rewind check runs both heroes for 36 seconds with movement and
live grenades. Every captured `Game` equals a reference with capture disabled.
Per-update restores match every serialized simulation field; different inputs
replace the abandoned future. File loads and retries clear history. Delta tests
cover variable-length states, backward/random seeking, memory eviction and
branching across full checkpoints.

Timed playback checks exercise 2× and 0.5× reverse speed, loss of focus, stopping,
resuming and reaching the oldest state. Geometry allocation identities remain
unchanged on ordinary restores, preserving renderer caches. The visual-state
continuation check includes alternate deaths, blood, weather, footprints,
secondary motion and interpolation; hair/cloth pointers are rebound rather than
serialized as platform addresses.

Native M1 movement was viewed backward, stopped and resumed on a new branch.
Chromium WebGPU passed `tools/test-web-savestate.mjs`, including browser restart,
continuous backward movement, stopping, branching and history clearing, with no
JavaScript errors. The optional practice tool remains off in graphics presets
and excluded from deterministic demos/benchmarks. Strict native/WASM Clippy and
118 ordinary PC tests pass.

In the release-build M1 fixture with live grenades and no renderer, 30 seconds
occupied about 59–60 MiB of compressed per-update history. Sample captures took
2.4–3.3 ms on this host. This is not a performance guarantee for large missions
or accumulated effects. The 96 MiB history cap reserves room for decoded frames;
heavy states shorten the available history. Unchanged scene geometry, textures
and static GPU resources are reused; resource changes take the full reload path.

Native T2 uses the same recorder, bounded history and timeline with an independent
clock across source scene resets. Its `native_timeline_` disc/GPU fixtures check
gameplay-to-story transitions, exact seeks, timed forward/reverse playback,
branch continuation, selected recording exports, native input metadata and
recorded/current graphics and camera. Invalid imports preserve the current
source and history. Menus and movies are omitted from this history. A separate
host regression verifies that automatic restoration keeps elapsed restore time
in the next host interval and leaves an open settings panel open. These are
isolated tool fixtures; ordinary-play validation still uses the BizHawk workflow.

## Stealth-kill blood timing review

The 2026-11-01 review of #15 traced all six normal paired executions against a
Mission 1 guard, with original and modern spray captures. First gore emission
occurred at death-motion count 38/118/none for Rikimaru's back/front/side kills
and 38/22/100 for Ayame's. These follow the disc event tables and `ActDEAD`'s
emission cadence; Rikimaru's side kill has no gore event. Neither added wound
drips nor body stains appeared at the initial HP drop. All six modern corpse
pools began only after the victim's motion completed.

No early effect was reproduced on these six paths, so their timing was left
unchanged. This is a source/timing and rendered-frame review, not a replay of
the reporter's unidentified scene; #15 remains open for a clip or saved state
that identifies the affected animation and effect.

Weapon impact presentation: the `weapon-sparks` demo spars with a live Mission 2
swordsman. Its scripted demo inputs produce blocked hits in both comparison
halves (demo ticks 24 and 384). Modern trails consume the existing yellow impact
and blue weapon-clash particles; block/clash lights only observe effect births.
Focused disc-backed checks cover density limits, pause/render cadence, flash
expiry and clustering, discarded pool slots, blood/lightning/gunfire independence
and unchanged serialized game state. Native captures and logs are kept outside
Git in `private/issues/impact-jitter`.

Camera interpolation: Mission 1 forward/run-stop traces for both heroes measured
an interpolated eye displacement of up to 14.97 game units from independent
rotation/translation rounding. Recomputing translation from the rounded rotation
and interpolated eye lowers the maximum to 0.81 units. The regression also checks
views 250,000 units from the origin; stationary views, exact pose endpoints and
camera cuts remain unchanged. Native movement/turn/attack checks and the existing
body/weapon/shadow interpolation and secondary-motion tests pass. The external
jitter report lacked a clip, preset or affected body part, so this verifies these
specific camera defects rather than confirming that report's exact cause.

HIME cross-stage appearance: the library now examines both protagonists' story
resources. A disc-backed regression compares every HIME texture region with her
native Mission 8 Ayame scene; all four library/import tests pass. Native Mission 1
Ayame movement and attacks with the HIME swap show her kimono, hair and face
textures, with the player's weapons retained. Before/after images are kept in
`private/issues/hime`. No artwork replacement or gameplay changes are needed.

Jitter-report follow-up (`tenchu-report-0.10.0-m2-tick862.json`): the Windows
report predates the camera fix and records healthy frame times (about 6.94 ms
average, 10.98 ms p95). A separate Mission 2 hard/layout 2 run-stop fixture near
the reported location measured up to 24.46 units of eye error with the old view
rounding, versus 0.86 with the fix, across both heroes. Native movement, turning
and attacks were also exercised with the reported explicit Rikimaru swap, model
refinement, interpolation and secondary-motion settings. No additional rig or
spring defect was identified. This was a local Linux reproduction of the
configuration, not the reporter's Windows machine or exact input history: the
report contains only its last 600 updates and is not a restorable state. Evidence
is retained in `private/issues/report-jitter`; a remaining visual symptom still
needs a clip or current-version savestate to distinguish body motion from hair.

Spark/fire lighting follow-up: the native Mission 2 sparring demo now shows a
short sharp strike and persistent light from travelling metal fragments. A native
attached-flame fixture was exercised while moving/turning with point shadows
enabled; it exposed complete self-occlusion of flames inside the body. Burning
volumes now exclude close non-scenery shadow casters. The GPU regression verifies
that this exclusion admits the flame light while nearby terrain still occludes
it, and that changing the emission volume invalidates cached shadows. Lifecycle
tests cover moving/fading flames, thrown Kaen/explosions, bounded spark clusters
and zero-density/disabled options without changing game state. Native and WASM
library Clippy pass. Captures/logs are in `private/issues/effect-lights`.

Alternate-death timing: both heroes and all four available clips now use the
selected clip's ground speed and settle without the original clip's continued
sideways motion. Movement stays within supported nearby terrain; falls still
follow the gameplay body's vertical position until landing. Pool geometry is
checked on every update against visible completion, including short variants
that finish well before the underlying animation. Their hidden original gore
sounds are replaced by sounds from the visible particles; co-located sounds
from other actors remain intact. The initial death sample is about 0.52 seconds;
the prolonged noise was repeated original blood impacts, not that voice sample.

Four disc-backed death checks, the original music/death-hold regression, visual
savestate continuation and ordinary workspace tests pass, as do strict native
and WASM library Clippy. Native isolated death fixtures for both heroes were
inspected with the regular and free cameras, including the held corpse/pool.
Captures and sound/clip measurements remain in `private/issues/death-timing`.
These checks preserve the original simulation and do not assert a new PS1 replay.

Weapon finishes: ordinary Mission 2 melee hits coat both protagonists' and enemy
weapons, with retail meshes and the v3 HD weapon replacements. A deterministic
continuation check confirms that swings shed substantially more blood than idle
drips and that serialized coatings resume identically. GPU checks isolate the
metal response from matte surfaces and foliage flags, exercise moving lights,
and verify graduated coating coverage. Native sparring, the F1 controls and
preservation of existing ground stains were inspected; prepared weapon meshes
avoid a scene rebuild when toggling the options. Native and WASM library Clippy
pass. Evidence is retained in `private/issues/weapon-finishes`. Reflections use
the sky and scene lights, not reflected images of scene geometry.

High-detail heroes: all five hero/outfit combinations preserve their original
attachment corners, finite textured geometry and unchanged gameplay state through
walking, attacks and jumps. Prepared meshes contain 27,520–34,176 triangles.
Native comparisons cover Rikimaru, Ayame and alternate Ayame with v3 artwork;
the refinement retains the retail designs rather than supplying new sculpts.
Chromium WebGPU and WebGL2 checks enter gameplay, draw the refined hero with
weather and lighting, and reset the new model/weapon options through F1 without
changing paused game state or reporting rendering errors. Native and WASM library
Clippy and 118 ordinary PC tests pass. Captures and logs remain outside Git in
`private/issues/hero-models`; browser smoke checks are not performance guarantees.

Custom weighted player models: a full-detail external model was exercised on
both heroes through walking, turning, attacks and jumps, with interpolated poses,
shadows and the animated gallery. The disc-backed check verifies finite, bounded
geometry and byte-identical game state before and after rendering. Synthetic
checks cover blended joint seams, invalid weights and coincident vertices with
different influences; the existing rigid-body/weapon regression also passes.
The 2K texture loader preserves decoded pixels and rejects oversized images.
All 122 ordinary PC tests, native Clippy and WASM library Clippy pass; browser
export includes skin sidecars, but this external model was tested interactively
only on native. Its assets, conversion tools and captures remain private and
untracked; the game needs only OBJ/PNG/JSON files, with no Python runtime.

Rewind performance: a repeatable release-build fixture walks and turns for 300
updates in Missions 1, 2, 6 and 9, then rewinds 120 updates. On this machine,
recording (capture plus compression) falls from 2.27–2.41 ms/update to
0.27–0.33 ms/update, about 7–9 times faster. Stored ten-second histories fall from
16.5–19.4 MiB to 8.2–11.1 MiB. Reverse playback no longer rebuilds periodic
keyframes; the previous 17–18 ms spikes in Missions 1, 2 and 9 are absent from
this run, whose largest restore is 4.24 ms. A separate 4,096-splat fixture
(98,304 vertices) takes 0.043 ms/update for cached capture and history, versus
0.951 ms/update for full serialization alone. These are CPU fixture measurements,
not whole-game frame-rate guarantees; first capture and resource changes still
pay the cost of encoding those resources.

Byte-for-byte comparisons cover ordinary serialization and cached capture after
texture, terrain, animation, script and copy-on-write mesh edits. Both heroes
run 1,080 ticks with grenades without diverging from rewind-disabled gameplay;
checks cover history eviction, reverse speed, branching, scene allocation reuse,
and continuation of cosmetic effects. Persistent savestate encoding is unchanged.
All 191 ordinary workspace tests and strict native/WASM library Clippy pass.
Disc-backed saves continue identically across all missions, live items, imported
actors and TRIAL. Native movement/reverse/resume was inspected with an isolated
copy of the user's settings; Chromium WebGPU and WebGL2 both pass persistent
save/restart/rewind/branch checks. The Xvfb native run has substantial NVIDIA
presentation latency, so its displayed FPS is not used as a performance result.
The benchmark is available with `TENCHU_DISC` set through
`./dev.sh cargo test --release -p tenchu-pc --lib profile_rewind -- --ignored --nocapture --test-threads=1`.
Local logs and native/browser captures remain in
`private/issues/rewind-performance`.

## Ground pickup stability (#47)

The supplied v0.11.0 Mission 7 report has SHA-256
`0ef16bab6ab210a6cdb1e31b4fcf2f276557fe35359bff3e79d9bff007d4af18`.
Its settled medicine pickup stays at (-23092, -2000, -24472); item physics
does not move it. The renderer instead used last-tick integer sprite coordinates
while terrain used the interpolated camera. A disc-backed reproduction near the
reported player position measured up to 7.38 original-resolution pixels of
misalignment across camera interpolation samples before the fix; the new check
requires less than 0.003 pixels. Rendering leaves serialized gameplay unchanged.

The regular camera now projects inventory icons from their world anchors using
the presented view, preserving DrawSprite's five ordering-table-unit depth bias
and icon scale. Free-camera rendering and the core drawing/physics code stay
unchanged. The pickup-transfer and free-camera effect regressions pass, as do
strict native/WASM Clippy and formatting. The isolated native Mission 7 build
was checked with the report's graphics settings, a restored nearby pickup state,
and ordinary movement. Before/after GPU captures and logs are kept outside Git
in `private/issues/47`. The report is not a full replay, so this establishes a
reproducible rendering defect rather than an exact replay of the reporter's run.

## Seekable rewind timeline

Both heroes pass a 1,080-update movement/grenade comparison with rewind disabled,
followed by repeated backward/forward seeks against the recorded serialized
states. Seeking preserves entry count and stored bytes, including after opening
and closing F1. Resuming removes only the abandoned future; resuming at Latest
keeps all entries. Tests also cover timed reverse playback, endpoints, clamped
seeks, file-load/retry clearing and unchanged scene allocations. The additional
decoded seek cache is reserved within the existing 96 MiB budget and is discarded
when recording resumes, without extra encoding during ordinary gameplay.

The M1/2/6/9 fixture retains roughly 0.27–0.33 ms per-update capture/compression
on this host. Large jumps still decode intervening deltas on their first visit;
the ten-second seek fixture measures up to about 60 ms. Repeated endpoint jumps
can reuse the decoded position. These are isolated CPU measurements, not whole
frame-rate guarantees. Native mouse seeking, frame steps, latest/oldest, menu
access and branching were inspected with isolated user settings. Chromium WebGPU
and WebGL2 exercise persistent save/restart, timeline slider/buttons, exact
return to Latest, repeated seeks and branch/resume. Ordinary workspace tests,
disc-backed rewind tests, formatting and strict native/WASM Clippy pass.
Logs/captures remain in `private/issues/rewind-timeline`.

## Call-aid spawn bounds

The all-active cover test treated a failed terrain passage as occlusion, including
passages into missing terrain. `APPEAR.C::BreedLife` and its Rust constructor
assign the result of the STEP_DOWN floor query directly to the new actor's Y;
without a floor that is the `i32::MIN` sentinel. The port's all-active extension
now validates this floor before visibility testing and tests cover at that actual
spawn height. Original distance-only recruitment is unchanged, as specified by
`THINK.C::Think3alarm` / `Think3callaid`.

A disc-backed fixture failed before the fix by adding a guard outside its terrain
bounds. It now checks map bounds, an interior terrain hole, and a caller below
all floors through both alarm/call-aid paths. Rejected calls allocate no actor
or models, consume no RNG and retain recruitment eligibility; moving the caller
to valid hidden ground still produces a guard at the correct height. Existing
cover/population, nonrecursive-call, recruitment reset, civilian replacement and
original-C AI comparisons pass. This confirms an unsafe spawn condition; the
external report did not include a mission or replay to identify its exact route.
Logs remain in `private/issues/callaid-bounds`.

2026-11-02 hero refinement revision: replaced outward PN patches with bounded
Loop subdivision for the high-detail heroes. All five hero/outfit combinations
remain inside source bounds and retain their full joint-axis extents. The
asset-backed animation test checks finite posed geometry and byte-identical
simulation state over walking, jumping and attacks. Native six-angle previews
of Rikimaru and both Ayame outfits were inspected; captures remain private under
`private/issues/hero-revision`. This refines the original designs and rigs; it is
not a newly authored character replacement. Native and WASM strict Clippy pass.

2026-11-02 #33: optional witnessed-kill calls reuse the all-active covered-ground
recruitment path. A lethal sword-contact fixture verifies one nearest eligible
witness, facing/range/terrain checks, spent/recruited guards, disabled settings,
Original mode, zero call counts and full capacity. Witness selection consumes no
additional game RNG. A continuous AI fixture completes the three-guard request
and runs 300 further updates without recursion; disabling the option cancels a
pending request. All nine AI tests, the original C melee/item-damage oracle, all
six retail stealth pairs and native settings persistence/reset checks pass.
Native and WASM strict Clippy pass. Original defaults are unchanged.

2026-11-02 #46: native checks cover both heroes using independently selected
actor moves without changing their body, including player locomotion, taunt
suppression and restoring original moves. Chromium WebGPU and WebGL2 exercise
the F1 search field, changing between Rikimaru and Onikage sources, disabling
and restoring the override, persistence, walking and attacking. Paused tick,
RNG and position remain unchanged; no page or GPU errors were reported. The
browser check caught and now covers dropdown search focus and explicit closure
after choosing a result. Captures/logs are in `private/issues/batch-46`.

## Mission 9 guard repeatedly drawing a weapon

2026-11-04: the [reported castle footage around 6:39](https://www.youtube.com/watch?v=Zr6QOl0PA_4&t=399s)
shows an ONIKERAI polearm guard repeatedly readying/lowering his weapon near the
garden. The exact save, settings and earlier inputs were not supplied. Native
Mission 9 layout A reproduces a matching loop with **Keep all enemies active**;
this is a reproducible port defect, not a verified replay of that particular run.

The all-active extension limits reception of the shared alarm to 20,000 units
from the player. Drawing moves this guard's root away from the player, across
that boundary. The next sight check then abandons suspicion and requests
sheathing; patrol movement brings him back into range of the same alarm. No new
spotting is necessary. A stationary-player fixture using the real castle terrain,
guard motions and patrol produces 11 draw entries during a single 600-tick alarm.
Other guards' AI is disabled in this fixture to isolate that cause; the initial
positions/alarm are constructed, not claimed to come from a retail controller tape.

Each all-active guard now remembers an alarm actually heard until the shared
alarm expires or the guard returns to calm. A distant glimpse no longer borrows
an unheard global alarm to request drawing. The same fixture now draws once,
finishes the animation outside the radius, and sheathes once after expiration;
the spotting counter remains zero. A separate distant-patrol fixture confirms
that an unheard alarm does not spread to remote actors. Memory is part of the
saved/rewound actor state and diagnostic reports. The savestate envelope format
is advanced so older binary actor layouts are rejected rather than misread.

This hearing-radius extension has no counterpart in retail `THINK.C`; original
`WORLD.C::ActivateHumans` normally suspends actors beyond 17,000 units. Original
activation mode keeps the original alarm rules. This case is separate from the
Mission 4 archer's blocked-navigation Square input described above. Core tests,
all eleven disc-backed AI tests (including the original-C combat oracle), and
save/restore continuation checks cover the fix. Video crops, native captures,
transition traces and the private probe remain outside Git under
`private/issues/weapon-loop-20261104`.

## Mission 1 sleepers waking during stealth kills

2026-11-05: diagnostic `my-tenchu-report-0.15.5-m1-tick1837.json` has whole-level
AI enabled, zero sightings/alarms, one stealth kill, and two initially sleeping
guards searching toward the kill location. Their home positions are about
23,000 and 33,000 units away. Its 585 recent inputs start after mission entry,
so this report is not a complete replay.

Retail `THINK.C::StateTransition` handles `ActionHalt` before normal sight checks:
active guards turn toward their target and back away at close range, regardless
of awareness or terrain visibility. `MOTION.C::ActACTION` treats nonzero input
as a reason to leave the sleeping pose. Getting up leads to drawing a weapon;
the draw completion can start a personal search without `Found` or a shared
alarm. Awake guards also receive the turning input, but that alone does not
change their awareness phase or record a sighting.

A controlled BizHawk pair executes the unmodified PS1 game with and without a
stealth kill. Both runs share boot/menu inputs and two player-placement edits:
first to activate the local actors while hidden, then behind an existing patrol.
The second placement also resets the player's previous position/velocity and
copies the target's local terrain probe. Guard positions, motions, AI, RNG and
awareness are never written. These are controlled experiments, not controller-only
routes or a replay of the supplied report. The original archived menu loadout
setup is retained and documented separately.

| Logic tick | Kill case | No-kill control |
|---|---|---|
| 107 | Stealth kill starts; sightings and alarm remain zero | Patrol continues |
| 108 | Sleeping guard 11 starts getting up; awake archer 13 receives turn input | Sleeper remains asleep; archer remains idle |
| 143 | Archer has turned from yaw 779 to 2479 and is still calm; no alarm/sighting | Archer has not made that turn |
| 165 | Sleeper starts drawing; still no alarm/sighting | Sleeper remains asleep |
| 173 | Corpse discovery raises an alarm; sighting count stays zero | No alarm/sighting |
| 210 | Stealth-kill AI pause ends | No alarm/sighting |
| 215 | The woken guard actually sees the player; sighting count becomes one | No alarm/sighting |

Actor numbers in this table are the PS1 roster indices. The core's stable IDs
for these observers are 9 and 11. A nearby kill can therefore make subsequent
detection more likely, but turning is not an automatic sighting, and normal sight
and terrain checks resume afterward. Corpse discovery is a separate path that
can operate during the kill animation.

A second controlled pair kills the archer instead, across the building from the
sleeper. Again the sleeper gets up at tick 108 and draws at 165, but this time
draw completion puts him into phase 2 at tick 195 with both alarm and sightings
still zero. The radar becomes red when the pause ends at 210; the first corpse
alarm occurs only at 294 and the sighting counter stays zero through tick 359.
The no-kill control leaves him asleep with neither alarm nor sighting throughout.
This also confirms that the forced draw can produce an unscored alert state,
not just a blue search indicator. A zero sighting counter alone cannot establish
that every later sight test failed after a guard has already entered phase 2.

Whole-level AI had extended the same forced turn to remote sleepers that retail
activation would suspend. The port now leaves nonscripted remote actors on their
normal AI during this pause, using the existing local awareness radius. Nearby
reactions, original activation, scripted actors and stage-end control remain
unchanged. A disc-backed regression failed before the fix for a distant sleeper
with whole-level AI, then passed for both activation modes and nearby/distant
cases. It suppresses separate corpse discovery to isolate this wake-up chain.
Private inputs, placement audits, RAM observations and native traces are retained
under `private/issues/m1-blue-alert-20261104`; no game resources are committed.
Formatting, strict native/WASM Clippy, 41 ordinary core checks, eleven disc-backed
AI checks and both corpse fixtures pass. The archived Mission 1 stealth/item
routes still match all 4,121 observed PS1 updates, including 30 skipped draws,
without correcting native state after initialization. Those controller-only
parity routes are separate from the controlled placement experiments above.

## T2 training signs, blood overlap and report dialogs (2026-11-13)

The report at `/tmp/blood-flicker.json` locates native training near
`[59774, 500, 4035]`. It contains settings and input history, not a restorable
recording. Controlled GPU fixtures at that location reproduce speckled overlapping
floor stains: opaque contact quads wrote competing depth values. Rendering them
without depth writes and preserving source insertion order removes the pattern
as the camera moves. Double-sided wall contacts also receive separation on the
viewed side. The regression compares overlapping floor/wall marks in Original
and Original HD against independent ordered composition at three camera positions;
one-channel quantization differences of one are tolerated. Source world bytes
remain unchanged. The earlier single-mark floor regression remains separate.

Distant-scenery checks use native maps 0, 1, 4, 12 and 15, including dormant
training signs. Rendering does not activate collisions or alter source object
poses/destruction state. GPU toggles cover both signs and doors. Falling trees
reuse the same pose calculation for active and distant drawing; the existing
JP falling-prop oracle checks the source arithmetic and destruction timing.

Training narration is derived from JP `DATAMNG.BIN` handler `800d202c`, its 27
lesson records at `800d8d68`, course starts at `800d8f18` and bounds at `800d8f24`.
Cue metadata comes from `GAME.EXE` at `800812e4`; audio is decoded from the user's
`NARAT_J.XA`. The existing sibling research repositories were reviewed read-only;
none supplied a T2 lesson implementation. The local original-code Unicorn
harness was reused to execute the handler over 4,000 boundary/action cases.
`training_lessons_match_original_handler` consumes those private traces through
`TENCHU2_TUTORIAL_ORACLE`. Separate disc checks decode the narration, enter all
four courses, check retrigger behavior and restore lesson/audio state. These are
source and integration fixtures, not an additional recorded campaign playthrough.

The diagnostic save uses an asynchronous native dialog polled by the window
loop. Pending/completed/cancelled/failed-dialog checks cover pause lifetime;
a game fixture checks normal ticks, rewind and manual stepping while pending.
Evidence and screenshots remain outside Git in
`private/release-fixes-20261113`.

### T2 replacement UI (v0.33.0)

The separate `tenchu2-artwork-v3` pack adds 64 Japanese UI identities. Native
menu fixtures cover title, mission selection, equipment, results, prologue and
all six narrated briefing backgrounds, with both confirm/cancel conventions.
Paused original → HD → original rebuilds produce identical original PNGs and
leave serialized menu state unchanged. The HUD fixture covers all 21 item icons,
four Ki states, digit slices, reticle, selector and zero/partial health fills;
vertex placement, tint and blend mode remain unchanged. A full transition fade
is black, and Hide HUD matches an empty HUD. The displayed VRAM blend classes,
not the pre-upload palette copy, provide the same identities as equipment icons.

Commands use the user's disc and the published pack:

```sh
TENCHU2_DISC=/path/to/tenchu2-jp.cue \
TENCHU2_ASSET_PACK=/path/to/tenchu2-hd.tar.zst \
TENCHU2_UI_REVIEW=/private/review \
./dev.sh cargo test -p tenchu-pc --lib native_hd_interface_restores_original_and_preserves_state -- --ignored

TENCHU2_DISC=/path/to/tenchu2-jp.cue \
TENCHU2_ASSET_PACK=/path/to/tenchu2-hd.tar.zst \
TENCHU2_UI_REVIEW=/private/review \
./dev.sh cargo test -p tenchu-pc --lib hd_hud_keeps_layout_blending_and_game_state -- --ignored
```

The local review is `private/t2-hd-ui-20261113`; generated paintings, prompts,
source references and screenshots remain outside Git. Folder and archive loading
are compared using matching captures. These are presentation checks, not a new
ordinary-play input route or a campaign-completion claim. Workspace tests,
formatting, strict native Clippy, Android library and WASM checks also pass;
WASM retains its four existing renderer dead-code warnings.


Level Studio's expanded building checks cover atomic stamp rollback, source
patrol truncation, non-square rotation of both layers, backward project imports,
saved-view bounds, and source-surface grounding with undo and misses. The ignored
`studio_native_clipboard_events_copy_paste_and_rotate_terrain` fixture feeds egui's
actual Copy/Paste events, rather than assuming the backend supplies raw C/V keys.
The mixed-disc GPU fixture also captures the editable garden's three saved views.

`demo --studio` uses the existing nominal presentation scheduler, renderer,
chapters, offline mixer and video verifier. Non-playtest chapters execute zero
logic ticks; playtest chapters use ordinary 30 Hz game updates. Its projects are
held in memory and do not read/write the user's library. The staged video is
feature evidence, not ordinary-play or retail input replay validation.

Studio follow-up checks reproduce the viewport input deadlock with real egui
pointer events (the old code times out), then exercise left/right/middle input,
keyboard flight and scrolling. Disc fixtures check inherited graphics, saved-draft
environments, T1/T2 return-state restoration, all original challenges and mixed
assets. Weather seeks compare accumulation against nominal visual ticks and
verify paused native T2/Studio state invariance, dry resets and subsequent updates.
The TRIAL HUD fixture checks source glyphs/candle counts, timer visibility, removal
of old textured candles, and unchanged serialized gameplay before/after rendering.
Before/after visual evidence is kept outside Git; these optional-mode fixtures
are not ordinary-play retail-parity evidence.

`studio_browser_and_tile_edit_regressions` measures trap/pit placement, T1/T2
campaign terrain edits and mixed-disc scenery changes in an optimized build.
It compares the resulting geometry and serialized game with fresh playtest/editor
reloads and retains accumulated weather across cached edits. Set
`TENCHU_STUDIO_USER_SETTINGS=1` to read the current graphics preferences and
`TENCHU_STUDIO_PROFILE_FRAMES=1` to additionally collect CPU/GPU preview timings
with rain, lighting, shadows and the full editor UI. The fixture saves its own
library and captures under `TENCHU_STUDIO_CAPTURE`, leaving user data untouched.
See [performance measurements](performance.md) for the comparison and its scope.
