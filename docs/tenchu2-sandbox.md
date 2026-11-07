# Tenchu 2 scenery sandbox

Experimental work on the `tenchu2-import` branch, after v0.19.0. This loads the
user's **Tenchu 2 Europe (SLES-02452)** disc and uses **Gaisen gameplay**. It is
not a port of Tenchu 2's missions. Both discs remain necessary; no retail assets
are bundled or downloaded.

On desktop, use **Tenchu 2…** at the top left of the title or quick-launch
menu. Select the CUE, then choose an area, placement/patrol layout, player and
optional Gaisen guards. The disc path is remembered for the next launch.
On Android, **Browse…** imports a folder containing the CUE and all its BINs;
the Android launcher also has **Import Tenchu 2 disc folder…**. Keep both games
in separate folders so the importer can identify the CUE unambiguously.

Alternatively, build and explore a map from the command line:

```sh
./dev.sh cargo build --release -p tenchu-pc
./dev.sh target/release/tenchu-pc play-t2 '/path/Gaisen.cue' '/path/Tenchu 2.cue' --map 0 --character ayame
```

Normal keyboard/controller bindings, F1 settings, free camera and camera-relative
controls work. F5 restarts this sandbox; Esc returns to the ordinary launcher.
Sandbox play does not award campaign progress. Savestates and rewind work within
the sandbox; reopen the same map and sequence before loading its file save.

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
The guards follow the source X/Z patrol points using Gaisen's AI; Tenchu 2 waits,
route actions, character identities and behavior are not reproduced.

Imported terrain, ordinary static props, textures, floor heights, slopes and
holes are supported. Gaisen's player/HUD textures are kept separate from the
Tenchu 2 texture pages. Original T2 textures are used: the Gaisen HD pack does not
provide replacements for them. Native desktop and Android have disc/map menus;
browser level import is not yet available.

Unported elements include doors and other scripted/moving objects, procedural
lights/effects, sky/background setup, animated water, water/material/hazard
semantics, music/voices, objectives, cutscenes and Tenchu 2 character models.
The sandbox uses Gaisen's audio assets and treats collision surfaces as ordinary
solid terrain. Water surfaces and invisible bottom planes can consequently act
as floors. Some routes can lead into unimplemented obstacles. F1 free camera
helps explore those areas; this is not yet a completable Tenchu 2 campaign.

Validation covers decoding all 23 maps, source-formula collision samples in both
travel directions, both heroes starting on visible floors, movement/jumping,
patrols and combat, save/rewind, and leaving/restarting without campaign changes.
These checks do not establish Tenchu 2 emulator parity.

See [source provenance](../SOURCES.md#experimental-tenchu-2-scenery-2026-11-07)
for the locally examined executable and resource readers.
