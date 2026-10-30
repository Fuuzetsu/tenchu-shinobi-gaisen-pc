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
