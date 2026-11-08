# Japanese Tenchu 2 integration preview

The native JP runtime can be exercised separately while its remaining systems
are connected:

```sh
./dev.sh cargo run -p tenchu-pc --bin tenchu-pc -- play-t2-jp /path/to/tenchu2-japan.cue --sequence 9
```

Disc folders and supported archives work here too. Use `inspect-t2 DISC` to
list sequences and their areas. The sequence selects its campaign hero.
Keyboard/controller bindings are shared with the main game. F1 shows actors,
health, water state and unconnected events; F3 pauses, F4 advances one logic
tick, and Escape closes the preview. This mode does not write campaign saves
or change saved graphics settings.

For a repeatable capture:

```sh
./dev.sh target/debug/tenchu-pc render-t2-jp /path/to/tenchu2-japan.cue /tmp/jp.png --sequence 9 --ticks 60
```

`--seed` sets the initial RNG state. Capture-only `--buttons HEX` holds a raw
Tenchu 2 pad mask for the requested ticks. `TENCHU_SETTINGS` selects a settings
file for rendering; the separate `tenchu2-hd.tar.zst` pack is discovered normally.

This preview uses JP actors, controls, AI, collision, water interaction, camera,
scenery, grapple and particles. It loads the hero/area VAB from the supplied disc
for voices and ambience. Its minimal renderer does not yet expose all of the
port's visual enhancements. Projectiles, some procedural effects, story scripts,
the original HUD, menus, results and progression remain integration work.
Component comparisons and snapshot continuation do not establish full campaign
parity. F6 and `play-t2` retain the existing Gaisen sandbox until that boundary
is complete. EU assets remain supported without an EU-native AI runtime.
