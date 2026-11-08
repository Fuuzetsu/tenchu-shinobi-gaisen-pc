# Japanese Tenchu 2 integration preview

Connect your Japanese disc through **F6 → Connect another disc…**, select
**Tenchu 2 Japan**, an area and a sequence, then **Start mission**. JP selections
now use the native JP runtime. The sequence determines the original player and
cast. EU selections still use the Gaisen scenery sandbox; JP can also opt into
that mode with **Use Gaisen scenery sandbox**.

The same route is available from the command line:

```sh
./dev.sh target/debug/tenchu-pc play-t2 /path/to/gaisen.cue /path/to/tenchu2-japan.cue --sequence 9
```

Saved controller/keyboard bindings and Android touch controls apply. F1 opens
the currently supported JP settings, F3 pauses, F4 advances one logic tick, and
F6 returns to the quick launcher. JP play does not write Gaisen campaign saves.
Graphics changes in this F1 menu are shared with the main game.

For development, the standalone preview is also available:

```sh
./dev.sh cargo run -p tenchu-pc --bin tenchu-pc -- play-t2-jp /path/to/tenchu2-japan.cue --sequence 9
```

Disc folders and supported archives work here too. Use `inspect-t2 DISC` to
list sequences and their areas. The sequence selects its campaign hero.
Keyboard/controller bindings are shared with the main game. F1 shows actors,
health, water state and unconnected events; F3 pauses, F4 advances one logic
tick, and Escape closes the preview. This mode does not write campaign saves
or change saved graphics settings.

Add `--loadout 4:20` for twenty shuriken. The normal item-selection and item-use
bindings apply; the preview overlay shows the selected item ID and quantity.
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

This preview uses JP actors, controls, AI, collision, water interaction, camera,
scenery, grapple, shuriken/kunai, caltrops, bows, fire arrows, firearms, grenades, mines,
smoke/sleeping bombs, poison darts, confusion powder, invisibility, teleport, fire breath, coloured-rice markers, resurrection, poisoned rice with guard luring,
weapon/projectile trails and particles.
Mission objectives now use the JP sequence descriptors, including exit regions,
unseen requirements, boss-health thresholds and elimination quotas. The preview
continues through story, area, boss, results and retry sequences. Health,
inventory, RNG and score totals carry across areas. Results calculate JP scores,
ranks, best records, supply rewards and unlocks. Chapter titles, narrated text,
movies, story camera/actors/faces and dialogue load from the user's disc.
Missed shuriken can be retrieved; projectile impacts use the native damage,
deflection and alert rules. It loads the hero/area VAB from the supplied disc
for voices and ambience. Its minimal renderer does not yet expose all of the
port's visual enhancements. Timed 3D story effects, original HUD, equipment,
results artwork and persistent JP campaign saves remain integration work.
Component comparisons and snapshot continuation do not establish full campaign
parity. EU assets remain supported without an EU-native AI runtime.
