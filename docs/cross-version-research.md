# Stealth Assassins and Tenchu 2

The optional dismemberment setting follows **Stealth Assassins USA Rev 1**.
Enable it in F1 → Persistent effects. It is off by default and is a gameplay setting: visual
presets and Retro mode do not change it. It changes RNG consumption when enabled;
it is unsuitable for replaying an unmodified Shinobi Gaisen input tape.

| Event | USA default |
| --- | --- |
| Rikimaru rear stealth death, frame 68 | 4% head (`rand % 25 == 0`) |
| Ayame rear stealth death, frame 36 | 4% head |
| Ordinary death, player performing a dive attack or landing | 4% head |
| Ordinary death, if no head was selected | 50% forearm |
| Selected forearm | 50% either side |
| Kataoka's execution actor, first ordinary death frame | Head, guaranteed |

The ordinary-death roll occurs at frame 5 for the actor owning the pending death
sound. It is not a roll on every hit. Animals are excluded; players and bosses
are excluded after selection, except the execution actor. Side/front stealth
kills do not roll for dismemberment. Custom sliders keep these selection rules.
At the default percentages the original modulo/low-bit tests and RNG order are
preserved at selection. Arm velocity rolls and blood emitted on landing also
come from SA, but the port uses mesh-aware ground contact so detached limbs
remain above the floor rather than burying their joint origin.

The detached head or forearm is an existing model part, moved into world space.
Its motion and both weapon-anchor tracks stop updating; attached geometry follows
the severed part. USA gravity is 20 per update, initial upward speed 100; bounces
halve velocity and settle below 20. Body-animation completion settles a part
still moving. The setting does not introduce additional enemies or collision
bodies. Reviving/repositioning an actor restores its model hierarchy.

Player dismemberment is a separate, off-by-default port extension. It rolls on
an ordinary combat death: 4% head, then 50% forearm if the head stays attached,
with either arm equally likely. All three chances are configurable. Drowning
and special death sequences are excluded. Alternate player death poses preserve
the detached hierarchy. The optional **Severed wound bleeding** visual effect
adds weakening body jets and small drips from the detached part for three seconds;
ground stains follow the independently selected ground-blood style.

PAL SA is not merely a different display rate: its stealth roll is `rand % 20`,
its event frames are converted by `frame * 25 / 30`, and detached-part gravity is
24. This is why the USA executable is the specification for the port's 30 Hz mode.

Local asset comparison found all 14 shared hero/common/stage AMD motion banks
byte-identical between PAL SA and Gaisen. Of 57 shared HUMAN MAD models, 54 had
identical decoded geometry after ignoring texture addresses/UVs, including both
heroes and Kataoka. Detached parts therefore require no SA assets at runtime.

**SA Kataoka execution scene** is a separate, off-by-default option on the same
page. Enable it before Rikimaru's Mission 5 ending to restore SA's actor placement,
execution shots and guaranteed head detachment. It needs no SA disc: the motion
data already exists in Gaisen. The port adapts the SA camera units by a factor of
ten and retains Gaisen's captions, voice/music schedule and scene duration in all
four languages. It does not alter Ayame's ending or TRIAL missions. Random enemy
and player dismemberment can remain disabled.

The optional Tatsumaru importer supports the supplied **Tenchu 2 Europe
SLES-02452** disc. Its VOLUME.DAT contains 4,730 entries in 62 hashed directories;
`T_00.cab` supplies the base motion bank and `weapon/atktbl.dat` supplies damage,
steering/combo frames, active hit frames and a bitmask of striking model parts.
Both hands and feet are used, unlike Onikage's two-shin hitboxes. Root height is
normalized to 4096 in both engines. Bone order differs, so the importer remaps
rotations onto Gaisen's skeleton and keeps the hero's own proportions. Weapon
ribbons are omitted, contact spheres are centered on the striking hand/foot,
and Gaisen supplies its normal collision radius, reactions and combo input rules.
Authored motion speed and frames run at the port's 30 Hz rate; PAL's 6/5 speed
correction is not applied. No Tenchu 2 character model or audio is imported.

The normal combo has five strikes. Its two directional second-strike variants
occupy the next two registrations in the source table; treating that table as
a linear combo made the third/fourth attacks turn sideways or backward. The
import now follows the source action transitions and preserves the authored
root motion, without a rotation correction.

F1 → Movement imports the user's disc locally; `.t2moves` import/export allows
the same animations to be used on the user's Android and browser installations.
The port ships only the reader and adaptation code. Onikage remains the default
unarmed style and does not need an additional disc.

Research binaries, decoded resource files and captures remain outside Git.
