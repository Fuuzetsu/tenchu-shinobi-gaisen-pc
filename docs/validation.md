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
  it does not establish every rendered wall/terrain boundary on every map.
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
