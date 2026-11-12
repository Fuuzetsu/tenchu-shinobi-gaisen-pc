# Tenchu 2 scenery sandbox

Available from v0.20.0. This experimental sandbox loads the
user's **Tenchu 2 Europe (SLES-02452)** or **Japan (SLPS-03040)** disc and uses **Gaisen gameplay**. It is
not a port of Tenchu 2's missions. Both discs remain necessary; no retail assets
are bundled or downloaded.

Development builds now default Japanese selections to the
[native JP runtime](tenchu2-native.md). Enable **Use Gaisen scenery sandbox** in
F6, or add `--mode sandbox` below, to use the mode described on this page.

Use **F6 → Connect another disc…** (or **Quick launcher…** on the title screen).
Select the CUE, disc folder or archive, then choose the game, area, patrol layout,
player, music and optional Gaisen guards in F6. Both editions stay connected;
the **Game** selector switches their area lists. On Android, **Browse…** or the launcher's
**Import Tenchu 2 disc…** offers a CUE/BIN folder or an archive. Each folder or
archive should contain one disc. ZIP, TAR, TAR.GZ/TGZ, TAR.XZ/TXZ and
TAR.ZST/TZST are supported. Archives are extracted once into a local cache;
keep the original archive on desktop. Android copies and prepares it for you.
No external unpacking tools are needed. `TENCHU_DISC_CACHE` can override the
native extraction directory.

Alternatively, build and explore a map from the command line:

```sh
./dev.sh cargo build --release -p tenchu-pc
./dev.sh target/release/tenchu-pc play '/path/Gaisen.cue' --quick --tenchu2 '/path/Tenchu 2 Europe.zip' --tenchu2 '/path/Tenchu 2 Japan.zip'
./dev.sh target/release/tenchu-pc play-t2 '/path/Gaisen.cue' '/path/Tenchu 2.cue' --mode sandbox --map 0 --character ayame
```

Repeat `--tenchu2` for each edition. The paths are remembered, so subsequent
launches only need `play --quick` (and can reuse the last verified Gaisen disc).

Normal keyboard/controller bindings, F1 settings, free camera and camera-relative
controls work. F5 restarts this sandbox; Esc returns to the ordinary launcher.
Sandbox play does not award campaign progress. Savestates and rewind work within
the sandbox; reopen the same map and sequence before loading its file save.

**Stage ambience (retail)** is the default. Each map loads its own background
samples and repeat/random-variation timing from the Tenchu 2 disc. These are
separate from the XA songs and use the effects volume control. Pausing freezes
this layer; loading a save restarts its samples. Ambient variation uses a separate
presentation RNG, so it does not alter Gaisen gameplay.

The music selector also offers eight optional themes. It repeats your chosen
theme over the stage ambience; it does not cycle through the catalogue.
Use `--music retail` (or `--music off`) for ambience alone, or `--music N`:

| N | Theme |
|---|---|
| 1 | Character selection |
| 2 | Options |
| 3 | Editor menu |
| 4 | Boss battle |
| 5 | Final battle |
| 6 | Editor — landscape |
| 7 | Editor — settlement |
| 8 | Editor — cave / ship |

This is a sandbox soundtrack choice, not the original game's per-scene music
schedule. These are the eight long tracks in its music archive; shorter cues,
voices and the separate event/movie soundtracks are not background selections.
Only the first three set the XA driver's repeat flag; the editor's gameplay loop
restarts its themes separately. Music decodes in the background and uses the
normal music volume and pause settings. No audio files are bundled. Character voices and combat effects still use
Gaisen's resources; environmental ambience uses the selected Tenchu 2 map bank.

List all maps and source layouts, choose another starting point, or add guards:

```sh
./dev.sh target/release/tenchu-pc inspect-t2 '/path/Tenchu 2.cue'
./dev.sh target/release/tenchu-pc play-t2 '/path/Gaisen.cue' '/path/Tenchu 2.cue' --sequence 4 --guards spear
./dev.sh target/release/tenchu-pc render-t2 '/path/Gaisen.cue' '/path/Tenchu 2.cue' preview.png --map 6
```

Map IDs are resource IDs, not mission numbers. IDs 0–23 exist, but 7 is an empty
placeholder. By default the first gameplay layout for a map supplies its start;
`--sequence` selects a specific layout and automatically selects its map. Map 20
has no layout and starts on a visible open floor. An explicit mismatched map and
sequence is rejected. `--guards sword|spear|archer` replaces imported actor slots
with that Gaisen guard type, omitting placements without a nearby collision floor.
The guards follow the source X/Z patrol points using Gaisen's AI. Guards without
a route use Gaisen's stationary lookout routine, which looks around independently
of an unseen player's position. Tenchu 2 waits, route actions and behavior are
not reproduced. **Use Tenchu 2 character models
for guards** gives them their source bodies; all still use the selected Gaisen
guard's weapon, moves and AI, including source civilian/animal slots. Female
bodies use female Gaisen combat voices, and imported Ayame/Rikimaru use their
Gaisen voice banks. Other bodies retain the selected guard's voice; original
Tenchu 2 voice events are not implemented.

Health and damage also come from the selected Gaisen controller: sword and spear
guards have 35 HP, archers 30 HP. Difficulty changes damage and reactions, not
their starting HP. Changing the displayed body does not change those values.
Gaisen's unalerted one-hit kill rule still applies, including when the displayed
body belongs to a T2 boss. Native Japanese T2 uses its own actor definitions and
combat rules.

Connecting a disc adds its **Tenchu 2 (EU)** or **Tenchu 2 (JP)** tab to
**F1 → Player model** in any Gaisen mission or sandbox. Both can coexist with
**Shinobi Gaisen** and **Custom folder** (when a model directory is configured).
Each imported edition has 83 bodies, including alternate and masked costumes. Search by name to find the heroes, bosses, guards,
animals and alternate costumes. The visible preview tiles play source idle
motions. Swapping a body is appearance-only until **Override moveset** is enabled.
**Moveset source** can select imported heroes and supported melee guards/bosses
from either edition independently of the body; alternate costumes share their
character's moves. This imports standing/running, crouching/rolling, wall movement, jumping
and melee combos with source attack timing, striking joints and recovery turns.
**Use player movement** keeps Gaisen locomotion with the chosen T2 attacks.
The Gaisen controller, collision and damage rules remain in charge. Paired
stealth kills, items, grapple, ledges, special deaths and unmapped actions still
use Gaisen motions. Projectile-only attacks and extra source controllers are not
yet selectable. Imported movesets carry their own weapon models, textures,
handles and sheaths from the selected disc, with its HD pack when enabled.
Source draw/holster rules control the attachments; weapon blood and reflections
remain available. Voices still use the host game. This is an
optional moveset adapter, not a T2-native simulation in a Gaisen map.
The same gallery also supports Gaisen bodies and movesets in
[native Japanese T2 missions](tenchu2-native.md).
The separate Tatsumaru unarmed importer remains available in Movement.
Extra hair/accessory joints are attached to their nearest
supported body joint, so their independent source animation is not reproduced.

## Optional artwork

From v0.20.1, download the separate
[Tenchu 2 HD pack](../artwork/tenchu2/README.md). Put **tenchu2-hd.tar.zst** beside the game
executable/launcher, or unpack it into **assets/hd-tenchu2/**. An explicit
**TENCHU2_ASSET_PACK** environment variable can point to either the archive or
an unpacked folder. On Android, use **Import Tenchu 2 HD artwork** in the launcher,
then restart the game. The Gaisen pack remains independent; either or both may
be installed.

**F1 → Visual assets → Replacement artwork** toggles both installed packs.
Missing replacements fall back to the appropriate disc. Pack v2 retains the
Europe paintings and adds the Japanese mission terrain, props, bodies/costumes,
weapons and items, including area 0 and the masked costumes. Packs use the same
`pack.json` format described in [the guide](guide.md); texture
identities come from the original pixels, so matching materials are shared
across maps. Retail cutout masks are reconstructed from the user's disc at load
time. The download contains replacement paintings and recipes, not disc exports.

Imported terrain, ordinary static props, textures, floor heights, slopes and
holes are supported. Gaisen's player/HUD textures are kept separate from the
Tenchu 2 texture pages. The Gaisen HD pack does not provide T2 replacements.
Native desktop and Android have disc/map menus;
browser level import is not yet available.

Unported elements include TRIAL/editor-built stages, doors and other scripted/moving objects, procedural
lights/effects, sky/background setup, animated water, water/material/hazard
semantics, scripted music/voices, objectives and cutscenes.
Collision preserves the source's blocked wall regions, masked holes and ordered
floor layers. Water surfaces still act as floors until water behavior is ported.
Some routes can lead into unimplemented scripted obstacles. F1 free camera
helps explore those areas; this is not yet a completable Tenchu 2 campaign.

Validation covers decoding all 23 maps, source-formula collision samples in both
travel directions, both heroes starting on visible floors, movement/jumping,
patrols and combat, save/rewind, and leaving/restarting without campaign changes.
These checks do not establish Tenchu 2 emulator parity.

Native Tenchu 2 gameplay is being developed against the Japanese version only.
The normal JP launcher now selects that runtime; this sandbox is an explicit
alternative. See [native JP support and remaining work](tenchu2-native.md).
European resource imports remain supported; a separate European AI
implementation is outside the scope of that work.

See [source provenance](../SOURCES.md#experimental-tenchu-2-scenery-2026-11-07)
for the locally examined executable and resource readers.
