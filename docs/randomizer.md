# Randomizer

Open **Randomizer** from the title / quick-launch screen, **F1 → General**, or
press **F7**. Choose a seed phrase and rules, inspect **Preview**, then start.
This is an opt-in mode; normal play is unchanged.

- **Setup:** Balanced, Wild, Boss patrols and Scavenger starting points; language,
  difficulty, layout, guard detection and always-active enemies.
- **Player:** Rikimaru/Ayame or any disc model, maximum HP, native model movesets,
  player movement overrides and idle-taunt suppression.
- **Enemies:** independent rolls or one type per mission, HP range, facing,
  optional civilian replacement. Models, motions, weapons, textures and voices
  come from the full disc, including bosses and animals. In T1, called
  reinforcements follow the same enemy pool and HP rules.
- **Stages:** a chosen mission, random missions, or a shuffled tour without repeats.
  Training can be included too.
- **Items:** number of item types, quantities, unlimited supplies, a guaranteed
  grapple, armour and randomized ordinary ground pickups.

The Player and Enemies tabs have separate class selections and searchable model
blacklists. Uncheck entries to exclude them. Stages and items have their own
blacklists. Searching only filters the list; it never changes eligibility.
Empty pools and contradictory rules produce an explanation before launch.

**Share seed → Export setup / copy** produces a JSON document containing the seed,
all rules and exclusions, the algorithm version and the zero-based roll index.
Paste it into the same tab and click **Import**, or use the native file buttons.
The text box also works when browser clipboard access is unavailable. During a
run, **Copy current run** exports the active rules, even if the editor has changed.
The short fingerprint identifies the complete seed/rules combination.

Retrying recreates the same start, including enemies, equipment and mission RNG.
Changing one randomization category does not reshuffle the others. Completing a
mission advances to the next roll; **F7** also offers retry, next and end controls.
The latest run is remembered in a separate `randomizer.json`; **Resume mission
from start** recreates that roll after restarting the game. This is not a mid-mission
save state. Normal campaign progress, records, stock and saved games are untouched.
For isolated experiments, `TENCHU_RANDOMIZER` overrides this file's native path.

While playing native T2 JP or T1 stages with T2 rules, **F1 → General →
Randomizer** and **F7** open the native editor. It has all three story campaigns
and adds T1 stages and models when that disc is connected. Native missions keep
their authored story hero and area order; the selected player body and moveset
are independent. The dialogue-language and layout choices apply to imported T1
stages. Starting NPC replacements keep their authored slots, patrols and mission
roles. T2 reinforcement calls retain the caller's selected body and native rules.
Ground-pickup rolls cover starting loose pickups on imported stages; native item
drops and corpse loot retain their source rules.

Native seeds use the version-one `tenchu2-jp-randomizer` format, distinct from
T1's `tenchu-randomizer`. Their setup/current run is stored in
`tenchu2-randomizer.json` (`TENCHU_T2_RANDOMIZER` overrides the path). T2 uses its
own item numbering, including resurrection leaves; its unused slot is excluded.
Spent supplies and rolled health up to 999 persist across areas, including when
the grapple is excluded. Savestates and rewind retain the roll and continuation.
Start a native randomized run before loading a randomized state. End the run
before loading a normal campaign state. **End randomizer** returns to the normal
session that was active when the run began.

Story actors and mission objectives are protected by default. Disabling that
protection is experimental and can prevent completion. Positions and patrol
routes stay authored; large bodies can have trouble in small spaces. Resource-only
models without authored AI, such as the crow, borrow animal behavior. Player
swaps keep a protagonist's story identity and retain fallback actions for moves
the selected model lacks. Strange body/animation combinations are intentional.

Graphics remain independently configurable. Randomized player bodies take
precedence over the normal model-swapper selection until the run ends; that
selection is kept. Ordinary input-tape recording is disabled during randomized
runs because its format does not describe these changes. Share the game version
alongside the seed document when comparing gameplay, since future logic fixes can
change input replays. Unsupported seed-format versions are rejected.

The browser seed-sharing check runs against a prepared, served build:
`TENCHU_WEB_URL=http://127.0.0.1:8000 TENCHU_HEADLESS=1 ./dev.sh node tools/test-web-randomizer.mjs`.
It uses an isolated browser profile and checks clipboard import/export, invalid
imports, language loading, resume and campaign save isolation.
