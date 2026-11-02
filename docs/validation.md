# Gameplay validation — 2026-10-30

The PS1 emulator is the reference for original gameplay. Optional settings such
as **Keep all enemies active** need separate tests: those modes do not exist in
the original game. A successful route is evidence for the updates it exercises,
not proof that every possible interaction works.

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
display boundary if the recording stops during the PS1 display wait. It cannot
export a completed-run movie. Custom experiments can use different pre-mission
loadouts and text input segments; see the capture tool's instructions.

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
  a separate comparison runs 24,576 rays through actual Mission 1, 3 and 10
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

The broader `controller_route` test filter has five pre-existing failures: the
two whole-campaign routes and the independent Rikimaru Mission 3, 7 and 10
routes. They fail identically on the unmodified v0.9.10 tag and this batch.
Rikimaru's campaign reaches Mission 6 with one fewer grenade than its old stock
assertion expects; the other routes miss their endpoints or die. The causes of
these older divergences have not been isolated. Their original expectations
are retained pending a separate fixture audit;
do not count this suite as passing or change gameplay to force those paths.
Standalone Ayame Mission 4/10 and both Mission 7 campaign tapes also fail to
complete, and are excluded from the verified examples.

The independent PS1 references still match all 33,404 full-campaign updates,
19,075 equipped Mission 1–7 updates, and the checked Mission 1 stealth-kill,
Mission 3 item-use and mixed-input scenarios. These comparisons import initial
state once and apply no subsequent state corrections. This supports the
unchanged default gameplay on those paths, not every possible route.

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
