# Source provenance

No retail assets are included. The supported executable is identified at runtime
by SHA-256 `0690a5c14ff3e975ebcb3de26e196a4dbb6afc992677d0de2cbcf86af9993558`.

The following local source files informed the first implementation. SHA-256
values identify the actual working-tree contents, including any local changes;
the listed Git revisions alone need not describe those contents.

| Repository | Revision | File | SHA-256 | Use |
| --- | --- | --- | --- | --- |
| `~/programming/tenchu-viewer` | `677cdc1f37710eec17b0479e7714dc3be09f2003` | `src/archive.rs` | `3d77b288c88ca1acd7257214402eeb54244e756fd993d8e435c2155cf4773d8f` | AFS format and logical asset paths; rewritten as a checked on-demand index |
| `~/programming/tenchu-viewer` | same | `src/bin/export.rs` | `b2c804ed5fbb4d7cddbf063881453121209eff1dbedfdf0e6d2c30936d50a7fd` | StageConfig layout and executable addresses |
| `~/programming/tenchu-decomp` | `cd3a505559bc748ba7be0082ccea58c1d4908407` | `src/main.exe/START.C` | `41eb63500c09fa33d8938a9e8acc541789038d9df8971c3ee14e5d835f2591e3` | Main-loop ordering and nominal presentation boundary |
| `~/programming/tenchu-decomp` | same | `src/main.exe/WORLD.C` | `f7999b02640fcdaecc8ee005941e3b7afb4108fbe598df6b8d8c8c19581e460f` | Mission construction sequence |
| `~/programming/tenchu-decomp` | same | `config/symbols.main.exe.txt` | `2e72575d7c08c455a1212ce9b461ac84d51107c5e043b13b03c39fea6510ebd8` | Retail symbol addresses |

The portable clock, byte-source interface, ISO/CUE parsing and host executable
are new code. No sibling checkout is a build or runtime dependency.

## Asset, scalar and renderer checkpoint (2026-10-20)

The asset decoders and texture shader adapt the viewer’s format knowledge.
Scalar math, motion, input and terrain follow the matching routines and the
research model’s regression cases. No batched simulator code is included.

`~/programming/tenchu-viewer` at `677cdc1f37710eec17b0479e7714dc3be09f2003`:

| File | Working-tree SHA-256 |
| --- | --- |
| `src/mesh.rs` | `753ebe63e612c8a1f5b90c3ba7049688b28fdcd97e0567a06c4a270aa766da15` |
| `src/motion.rs` | `124849546ac1135c963f6e9cd6c1e2077d06b2aecc6315d9a3c491ca411949a7` |
| `src/collision.rs` | `738fcee9ad44e304ddfb017eb4633e0b6b1ef788bff2f552d5f7012c7e20ffa1` |
| `src/scene.rs` | `7de326a13573430e02478bddadacdde14371b8f24566efa20f831d450a525c4c` |
| `src/renderer.rs` | `aa50e228ffe51c24af55a29d9fd7eec5bdfc79912db0c3aa8efba9a5d8e2d092` |

`~/programming/tenchu-decomp-glitch-research` at `9ac830e2234f1217daa096198cebadd3f80c0b02`:

| File | Working-tree SHA-256 |
| --- | --- |
| `tools/tenchu-sim/src/geometry.rs` | `a2b03900a979336300d77f47ad4fc2ebdae8f9817577107c8fb04d188889ce13` |
| `tools/tenchu-sim/src/spline.rs` | `e275c72aada8c2bd5bff0303c90e74c3cc3b0e10badb597e2b40cbc8a1972fe0` |
| `tools/tenchu-sim/src/command.rs` | `387a8aa2c254f0cebb023749d15d2316ad742c54e4dfc0a682d4b59b804eed54` |
| `tools/tenchu-sim/src/random.rs` | `3990b504daf0099284e96b6dc515e693bfe9492fbd979503595bb83b92f430ab` |
| `tools/tenchu-sim/src/terrain.rs` | `4098c4db4323e2ca04f6f011af855e16746095dcf81386b0cb64744f53cdd47f` |

`~/programming/tenchu-decomp` at `cd3a505559bc748ba7be0082ccea58c1d4908407`:

| File | Working-tree SHA-256 |
| --- | --- |
| `src/main.exe/ACTION.C` | `8599e6f2b0c7c15723d3c5f82fa6823f0b1f53e896b4ff83a4bc8e1fb927f8cd` |
| `src/main.exe/APPEAR.C` | `6626d5d5491a74dd826fa6a6e9fb0599b3ba8b5846e2e7377ef1b0a3e79ef86b` |
| `src/main.exe/STAGE.C` | `dc1573257be1b02eeaaa96cd1e3c6ce8b952da3dbd4a122e669a29e4d819a391` |
| `src/main.exe/PADCMD.C` | `570e8ab66ddfe86b6a69271093c0c3ba187304985c252ee79bfa3a459d415f78` |
| `src/main.exe/CONFLICT.C` | `4fd248c53cd709febd10f8f748dabe32e7844f1b5b91ee845343f57a7b7df4c0` |
| `src/main.exe/game_types.h` | `27e346901f78a9664385cd4110ca089dd45fc9b5ac8d6e883cacd08ed2e1cb07` |
| `include/psxsdk/libetc.h` | `2d8dd8ba91ba482f6eaf7eff9270853c737f0f5729289b44c7f5b8a131b61e3c` |

## Scalar movement and desktop input (2026-10-20)

Movement was rewritten as ordinary owned actor state, following these matching
C routines and the research model’s source-order regression evidence. The native
window, CLI replay and fixed-step host integration are new code.

`~/programming/tenchu-decomp` at the revision above:

| File | Working-tree SHA-256 |
| --- | --- |
| `src/main.exe/HUMAN.C` | `9138dbd55667b99dec7b7703ac58a6325dc70654bda3f0612b2af33340892158` |
| `src/main.exe/MOTION.C` | `7d5cddc279921e20a9e1e0066370d08b1f0dd8e81a2b047d46e0995ef5df7a82` |
| `src/main.exe/EFFECT.C` | `a50b850d80090eda8cc39770d9d3026dc6888936eb2b1324d6c2821348d4da17` |
| `src/main.exe/humanoid.h` | `e10827d154b8a55e40e4a740ef8225fe2f0ea742586fc80a5df520a5977b9022` |
| `src/main.exe/tuning.h` | `d24d0528aafd617a572eb1201edb3518d8ff497adaae82e44a9695e7c4d7897e` |

`~/programming/tenchu-decomp-glitch-research` at the revision above:

| File | Working-tree SHA-256 |
| --- | --- |
| `tools/tenchu-sim/src/body.rs` | `980adfaefcbe33af67fb5b36b881f5f747bedbed2d70b04e1eaccf398267dac4` |
| `tools/tenchu-sim/src/ground.rs` | `79df01195397c37371ce07dbebb1af0e7cd5d5efa34ec053f4c1b23bd0a86559` |
| `tools/tenchu-sim/src/locomotion.rs` | `f16dbc13959c5788dbd6aa8b82733fac09875bc590f6e5fa20752c65efd402b3` |
| `tools/tenchu-sim/src/hang.rs` | `80d92736fa26d2b6209e3b0c652759f8a8bcebaea6872078108fcf523ddf2bea` |
| `tools/tenchu-sim/src/crouch.rs` | `6e4160e91edeac53a19b4ebac567ce430aa30a28e576d27cf80dc4fb53ecc988` |
| `tools/tenchu-sim/src/effects.rs` | `7d6de4c9a97f8f50f1e8bf8b09ca698f94525b716ffbfe9815777679fd046a6b` |
| `tools/tenchu-sim/src/controller.rs` | `adaf0e5d7c8c46616fe9eba793d898f626064739cbbb3900b7b141782d30ac7b` |

## Coordinate caches and ordered contacts (2026-10-20)

The scalar scene/cache walk follows the SDK behavior documented and tested in
`tools/tenchu-sim/src/geometry.rs` above. The collision pool and body resolver
follow the matching `CONFLICT.C` and `HUMAN.C`, including retained tail slots.
Model construction and draw visits also reference this file in `tenchu-decomp`:

| File | Working-tree SHA-256 |
| --- | --- |
| `src/main.exe/3DCTRL.C` | `1175982b0890852071a4b0f581ab637b24258d9fe2153b94d958d0edd337c45a` |

## Source camera and view transform (2026-10-20)

The camera uses scalar owned state and tables read from the supported executable.
Terrain rays also follow `EFFECT.C` above; head tracking follows `HUMAN.C`.
The SDK view transform was checked against generated retail assembly and one
private capture. Smoothing has a private-fixture comparison against matching C;
neither the C nor captured RAM is needed by the game.

| Repository | File | Working-tree SHA-256 |
| --- | --- | --- |
| `tenchu-decomp` (revision above) | `src/main.exe/CAMERA.C` | `759da5c93aaa780ec4dc113bc0b49b18eeee7bc27f63c4b2024a5b520b4c5e86` |
| `tenchu-decomp` (generated retail assembly) | `.shake/gen/main.exe/asm/SDK_TEXT_55714.s` | `85568476bf02dc47e015f0fd7f90e9b3909e5009f326f190090561bd5e35ccc8` |
| `tenchu-decomp-glitch-research` (revision above) | `tools/tenchu-sim/src/camera.rs` | `e7a0277b94ad4d9d98853f364ea0816b1a1b85a6cb0c6af6955d9344b455c34d` |

Weapon metadata, attachment construction and equipment swaps follow
`APPEAR.C` and `game_types.h` already recorded above. Weapon geometry uses the
same checked TMD reader as the original character models.

## Stage roster construction (2026-10-20)

Construction, layout installation and story ordering follow `WORLD.C`,
`STAGE.C`, `APPEAR.C` and `HUMAN.C` recorded above. Patrol control retains
`ControlTraceLine`'s narrow counters and direction fold. Additional matching
sources at the same decomp revision:

| File | Working-tree SHA-256 | Use |
| --- | --- | --- |
| `src/main.exe/ITEM.C` | `fd5a6d31ef3e5628d792a4f05604d7e40ff588663c339651858e161bee57e78a` | Temporary dog/disguise construction |
| `src/main.exe/THINK.C` | `65ddde563a175579f83c1f26e00cd63848b9abe2fb82a326097a42a7d987b23a` | Think-program selection |
| `src/main.exe/CHRANIM.C` | `5e8c2a7548b441bdb68b16b54b7ac2fb6c9dd5f791868295d625c3fb2297c825` | Identifying CVA-mutated homes excluded from the roster fixture check |

## Visibility and sight checks (2026-10-20)

`3DCTRL.C`, `HUMAN.C` and `CONFLICT.C` above define root clipping, sight guards
and terrain passage. These research files supplied scalar arithmetic and
regression evidence; the new runtime has no parallel projection/continuation
machinery. Revisions are the ones recorded above.

| File in `tenchu-decomp-glitch-research` | Working-tree SHA-256 |
| --- | --- |
| `tools/tenchu-sim/src/projection.rs` | `f6c4385b5317290e3bba067f1275421784bc1b306cac3c1f576df722c0c27b7e` |
| `tools/tenchu-sim/src/allocation.rs` | `2fa9018eb96dc7c427dc8b1177120e6d2d6a2c0ca7722e63f58ae51ff364ed7b` |
| `tools/tenchu-sim/src/sensing.rs` | `c9f2ffba7d2d73d7c589316941213398ba3e5824d3a7a1aa1a0ae23265a569f6` |
| `tools/tenchu-sim/native/sensing.c` | `d46bf244fbf33a085e24324041b26b3a9a4fed49d992664e743cf5d20c8ca73c` |

## Scalar AI and ordered actor control (2026-10-20)

Controllers follow `THINK.C`, `HUMAN.C`, `APPEAR.C` and `tuning.h` recorded
above. The reinforcement type table is read from `MAIN.EXE` at `0x80089e14`.
Research code clarified the freed-caller continuation and test-oracle boundary;
no batched actor storage or source-C runtime code was imported. The combat oracle
extracts actual decision bodies and supplies native test structs/side effects.

| Repository | File | Working-tree SHA-256 |
| --- | --- | --- |
| `tenchu-decomp` | `src/main.exe/item.h` | `99109f2c30cc5b661e52225d14d14da287dc48045246e8657b9d29eefee0017c` |
| `tenchu-decomp` | `src/main.exe/sound.h` | `405f3e09c98bcdea57852ec640f1249975e27f9bf44edf242084c2ef1e36159b` |
| `tenchu-decomp-glitch-research` | `tools/tenchu-sim/src/ai.rs` | `c2f07eede86335dfb3047dd641eb42e90cc17ceb156e05325f05311091341439` |
| `tenchu-decomp-glitch-research` | `tools/tenchu-sim/src/birth/replacement.rs` | `ebf4c3fc4f247284af7b3aae621ca21e44b18f3bf9df665c636f4c43afc0aaec` |
| `tenchu-decomp-glitch-research` | `tools/tenchu-sim/native/ai.c` | `8c783ef32ee1cc58fff24227118fdb84fc42f0df3ebcb0a277813599cf8429f2` |

Shared effect allocation, `DrawShadow`, splashes, droplets and attached flashes
follow `EFFECT.C` above and this layout/allocator header at the same revision:

| File in `tenchu-decomp` | Working-tree SHA-256 |
| --- | --- |
| `src/main.exe/effect.h` | `b274c297c776620cfa28ef26a0d0f3637f233e9fb3317a5d76ffd18a932c94f5` |


## Activation and logical drawing workload (2026-10-20)

`WORLD.C`, `3DCTRL.C`, `HUMAN.C` and `EFFECT.C` above define the activation
period, previous sampled budget, construction order and drawing side effects.
The scalar reference sections of the research counters supplied the integer
geometry algorithms; no SIMD slabs, search bounds, imported-RAM runtime or GPU
timing/rasterizer was adopted. Optional tests use matching activation C and a
separate BizHawk/Mednafen GTE executable; neither is part of the runtime.

| Repository | File | Working-tree SHA-256 |
| --- | --- | --- |
| `tenchu-decomp` | `src/main.exe/IMAGES.C` | `4ad4a53038c560c889c0057b6c1b7d3903da2585dbbd0e86682af24e94d71799` |
| `tenchu-decomp` | `src/main.exe/model.h` | `1a390b1d673df773a43dcfb58e341af9d3f99c5a5a9135ec450c358d4bd59c00` |
| `tenchu-decomp-glitch-research` | `tools/tenchu-sim/src/construction.rs` | `8ffca448d7c39a8edc3864fff5ee418438cc42a2309fdfecd695573d6790651d` |
| `tenchu-decomp-glitch-research` | `tools/tenchu-sim/src/render.rs` | `3a98f8aca8602071ae4e5908e8c29ad7743be32fcbf9bfe0d65add6d670d2e34` |
| `tenchu-decomp-glitch-research` | `tools/tenchu-sim/src/near_packets.rs` | `bbd545b68642dd4159bd05a76c9af5f3193f71d9b3c29728a15e5c0aefff5786` |
| `tenchu-decomp-glitch-research` | `tools/tenchu-sim/src/sprite_packets.rs` | `2b70e70d368cd98a43aaefdf827da6186c486a32c9a00ae461c6808981cf9e65` |
| `tenchu-decomp-glitch-research` | `tools/tenchu-sim/native/gte_oracle.cpp` | `11b23899158103d574de865abbbd388bd4f8ec851cb4fa33d4700f8454639ee5` |


## Melee, damage and death (2026-10-20)

The scalar attack, guard, damage and death paths follow `MOTION.C`, `HUMAN.C`,
`THINK.C`, `CONFLICT.C`, `EFFECT.C` and the headers recorded above, at the same
working-tree hashes. Stealth death event lists are read from the executable's
`DeadEvents` pointer table at `0x80086b0c`. Blood/gore, impacts, blood pools and
weapon trails retain the source RNG, shared-pool and coordinate visits.

The optional damage comparison extracts the original DamageControl and
cleanup/scoring helpers at test time. Its native test shim supplies open terrain,
real-disc motion descriptors, arithmetic movement and effect-call RNG costs.
It checks decisions and ordering, not complete native SDK/rendering equivalence.
No matching C, executable tables or disc assets are bundled in the runtime.


Effect presentation reads all 62 entries of `IMAGE/IMAGES.ARC` after stage and
character texture uploads, following `START.C`, `IMAGES.C` and `EFFECT.C` above.
The wgpu vertex/batch conversion is new code; it reads prepared core graphics.
Additional decomp headers at the revision above:

| File | Working-tree SHA-256 |
| --- | --- |
| `src/main.exe/main.exe.h` | `511108e5103d791e0457cfbf209efa4dd59f95c8a475edea11a7370ff422d7df` |
| `src/main.exe/images.h` | `415c9dfb272a098b396451fd26f639b32c9f4b9e167d7f40ac1b416d712de1ca` |

## Scalar dropped items and pickups (2026-10-20)

The owned 30-slot item pool, rolling physics, layout placement, inventory drops
and pickup transfer follow the matching C, including allocation and update order.
The research implementation informed edge cases; its slot bytes, imported state
and batched storage were not reused. Original-C tests use recorded terrain
responses to isolate item arithmetic and branching.

| Repository | File | Working-tree SHA-256 |
| --- | --- | --- |
| `tenchu-decomp` (revision above) | `src/main.exe/ITEM.C` | `fd5a6d31ef3e5628d792a4f05604d7e40ff588663c339651858e161bee57e78a` |
| `tenchu-decomp` (revision above) | `src/main.exe/item.h` | `99109f2c30cc5b661e52225d14d14da287dc48045246e8657b9d29eefee0017c` |
| `tenchu-decomp` (revision above) | `src/main.exe/INFOVIEW.C` | `e4aef1960f32907c3217bbdb3552204f99693124261ede8e9aaa3b6deba15af0` |
| `tenchu-decomp` (revision above) | `src/main.exe/3DCTRL.C` | `1175982b0890852071a4b0f581ab637b24258d9fe2153b94d958d0edd337c45a` |
| `tenchu-decomp` (revision above) | `src/main.exe/layout_save.h` | `31531d6c0942c77ebe82132f7f86de543178bd6ae6b51119d8b676ea346cd8b9` |
| `tenchu-decomp-glitch-research` (revision above) | `tools/tenchu-sim/src/items/rolling.rs` | `c780da63cacd9f2ccf4b6ef6fd1d78871cb948bf0a90d23d2d05a6dbce147665` |

Stage loading restores the shared image archive after the stage/common/player
texture packs, matching `CreateStage` → `InitializeImage`. Loading it only at
startup lets the stage pack overwrite item-icon texels and palettes.

`tenchu-decomp` (revision above), `src/main.exe/IMAGES.C`: `4ad4a53038c560c889c0057b6c1b7d3903da2585dbbd0e86682af24e94d71799`.

## Scalar projectiles and item damage (2026-10-20)

Flight arcs, targeting, arrows, gunshots, shuriken, item damage, ranged animations
and scatter particles follow `ITEM.C`, `MOTION.C`, `EFFECT.C` and headers at the
hashes recorded above. Bow timing is read from `0x80097714` in the executable.
The ordinary scalar implementation owns projectile state alongside the existing
item pool. Runtime code does not execute original instructions or depend on C.

The optional XYZ-rotation comparison executes only the retail SDK RotMatrix
instruction range in the existing test oracle. This supplements the source-C
flight/damage checks and ordinary-tick ranged scenarios.

| Repository | File | Working-tree SHA-256 |
| --- | --- | --- |
| `tenchu-decomp-glitch-research` (revision above) | `tools/tenchu-sim/native/sdk_rotation_oracle.py` | `9f1fe49761cf8119b26b839380732ea542046eebcef7178c83ca142075420bdf` |

## Inventory and HUD (2026-10-20)

`INFOVIEW.C`, `START.C`, `game_types.h` and `main.exe.h` at the hashes above
define item selection order, reserved-slot behavior, life-bar allocation and
expiry, proximity feedback and the fallback launch loadout. Life-bar style
parameters and image IDs come from `0x8008e414` and `0x8008e4b4` in `MAIN.EXE`.
The HUD owns scalar state and prepared sprites; presentation cannot advance it.

## Player aiming and consumable items (2026-10-20)

`MOTION.C`/`ITEM.C` supply quick/aimed shuriken, cancellation/refunds,
interruption drops, caltrop scatter/arming and medicine animation/healing.
`THINK.C` supplies enemy medicine requests. The working-tree hashes above still
apply. Item cleanup can access the actor currently executing control as well as
the roster, preserving immediate owner-flag writes during pool eviction.
Shared icon scales and reused item rotations remain live scalar state.

## Grappling and cable effects (2026-10-20)

`ActKAGI`, `ProcKaginawa`, `ReqItemUse`, `SetFlyWire`, `DrawFlyWire` and
`SetWire` in the matching files above govern aim/release, camera, motion masks,
pull/contact transitions, cable RNG and integer curve sampling. The runtime uses
ordinary scalar actor/item/effect state. The original empty-curve hook draw uses
undefined stack coordinates; the port explicitly omits that draw.

Additional behavioral references in the research repository at the revision above:

| File | Working-tree SHA-256 |
| --- | --- |
| `tools/tenchu-sim/src/grapple.rs` | `b3f7b34a5b1baa42201c02929355a9f040cb37843e0a827ea058222de9541a87` |
| `tools/tenchu-sim/src/wire_packets.rs` | `e67a1e26aeceb714f88bd0adbc4af080f84146c41ecd09cbf49c935feceef05b` |

The new private test extracts the actual C hook setup/update routines. It compares
2,048 flights with supplied SDK square-root results and draw/sound/particle stubs.
No sibling source or C toolchain is required by the game.

## Smoke bombs, grenades and mines (2026-10-20)

The matching `ITEM.C` routines `ProcItemSmoke`, `ProcItemFire`, `ProcItemJirai`,
`FindItemTarget` and their request functions define throws, filters, timers,
collision changes, dud recovery and refunds. `EFFECT.C` defines the new smoke,
explosion and ember requests and updates. These are the same working-tree
versions hashed above. Shared scalar helpers retain request/update ordering;
there is no source-C runtime dependency.

A private comparison extracts `DrawSmoke`, `DrawExplosion` and `DrawHinoko`
from the actual source and checks 384 complete particle lifetimes, including
RNG, velocity, scale, color, sprite choice and disposal. Coordinate/graphics
submission is stubbed in that comparison. Controlled gameplay and window checks
exercise actual item use and rendering separately.

## Swimming and water transitions (2026-10-20)

`SwimCheck`, `ActSWIM`, `FallCheck` and body visibility in the matching
`MOTION.C` above supply entry/exit, motion and camera changes, steering, RNG,
attack cleanup and hook interaction. The runtime uses scalar actor state and
the existing ordered effects pool. Additional behavioral reference at the
recorded research revision:

| Repository | File | Working-tree SHA-256 |
| --- | --- | --- |
| `tenchu-decomp-glitch-research` | `tools/tenchu-sim/src/swim.rs` | `3ebfca456308dc54e65f1e7cf695f19e13ba7f4de41e85a10b2e864cda2cb5c1` |

Private-disc tests exercise both players through water input, hook cancellation
and shore exit, and a non-swimmer through drowning/collision removal. Targeted
checks cover entry splash RNG, one-time cleanup and falling attack cancellation.
These scenarios do not establish complete mission or water-presentation parity.

## Wall hugging and corner throws (2026-10-20)

`StickonCheck`/`ActSTICKON` in the matching `MOTION.C` above govern entry,
shared static pose-key edits, alignment, sliding, rolls and delayed throws.
`ACTION.C` defines motion update/hold behavior. `ITEM.C` supplies the raw
caltrop, grenade and smoke requests, now shared with ordinary item use. The
empty `ActCEILHANG` dispatch remains empty.

| Repository | File | Working-tree SHA-256 |
| --- | --- | --- |
| `tenchu-decomp-glitch-research` | `tools/tenchu-sim/src/wallstick.rs` | `6f2e224a48c642b6061007b3f053544f394b84d34f89f71648d5cfa32c14440f` |

The research implementation supplied behavioral context, not storage or copied
architecture. Matching C takes precedence, including `UpdateMotion`'s same-ID
early return. Mission data is owned and mutable by game control so edits to the
wall clip persist across users/re-entry. A private-disc check confirms the
modified tracks have distinct storage in the original Rikimaru and Ayame AMDs.

Retail wall sliding dereferences a null map at RAM `0x0000000c` when a probe
fails. Its vector byte is zero in the available retail capture. The port uses
`RefrectVector[0]` explicitly here; it neither reads a native null pointer nor
requires BIOS/RAM emulation. This behavior is checked for both advancing away
from the wall and reverting a sideways slide. Different low-RAM contents are
not part of this claim.

## Poisoned food and lure cleanup (2026-10-20)

`ProcItemDokudango`, `ReqItemDokudango`, `FindItemTarget`,
`restore_dokudango_target` and `apply_dokudango_reaction` in `ITEM.C` above
supply rolling, search order, owner recovery, AI redirection, eating, interrupted
drops and poison expiry. `MOTION.C` supplies the return-to-normal behavior.
The source stores a function pointer for the original AI; the scalar port uses
`Option<u8>` so program index zero remains distinct from no saved callback.

| Repository | File | Working-tree SHA-256 |
| --- | --- | --- |
| `tenchu-decomp-glitch-research` | `tools/tenchu-sim/src/items/poison.rs` | `893c917374d28d9c4d220ce54dd7656578c677028f2516ec0e89403813bffdd9` |

The research file is a behavioral reference only. Private-disc scenarios exercise
ordinary input, enemy following, eating, reaction/expiry, interrupted-drop RNG,
cleanup, and the source search-order quirk that can suppress owner recovery.

## Stage event and cinematic assets (2026-10-20)

`CHRANIM.C`, `STAGE.C` and their recovered structures define the ESD event
records and CAD command stream. The parser retains the original byte offsets
for subtitle strings and loads the chosen character's localized script. These
are the matching decompilation working-tree versions at the revision above:

| File | Working-tree SHA-256 |
| --- | --- |
| `src/main.exe/CHRANIM.C` | `5e8c2a7548b441bdb68b16b54b7ac2fb6c9dd5f791868295d625c3fb2297c825` |
| `src/main.exe/STAGE.C` | `dc1573257be1b02eeaaa96cd1e3c6ce8b952da3dbd4a122e669a29e4d819a391` |

The private-disc check decodes all 88 stage/character/language CAD files (502
sequences, 15,689 commands), checks event successors, and decodes all subtitle
strings without replacement characters. Synthetic tests cover truncated records,
signed fields, sentinel sizes and out-of-bounds text. No retail script bytes are
stored in the repository.

## Event execution, cinematics, fades and subtitles (2026-10-20)

The `STAGE.C` and `CHRANIM.C` versions above govern routing, cached targets,
clock restoration, CVA motion ownership, actor placement, camera cuts/pans,
cumulative waits and end cleanup. `MOTION.C` supplies guarded motion updates and
the global cinematic damage gate. `PADCMD.C`/`CHRANIM.C` show that controller
sampling stays at the entry poll during CVA; an item clearing ActionHalt does not
refresh that sample. `EFFECT.C` supplies fade allocation and clock
arithmetic. `CAMERA.C` and `WORLD.C` confirm that normal and cinematic control
share `ViewInfo`; the port keeps a separate immutable view of the rendered frame,
not a separate normal-camera position. Source score packing and rank computation
are used for training's sequence selection and subsequent results work.

Additional matching source files at the decompilation revision above:

| File | Working-tree SHA-256 |
| --- | --- |
| `src/main.exe/draw_glyph_.c` | `b53f92efcc7181b741888788edc10be93ca4a9f7e8b10620bf811e7fb39d9dd6` |
| `src/main.exe/telop_text_width_.c` | `ff94949d90d52753b09ac4f2cfaa23e3fc1dd88a12c835c8eaec836422fbf047` |
| `src/main.exe/draw_telop_line_.c` | `77fa51787c93e7595d353eb9ad885ce2b7ae55048c96a4cc825fd964db95eb92` |
| `src/main.exe/initialise_font.c` | `13bc65c8f9a759d9494b96b3eafcaeccd9716b699ca8c048f21fb10dbd2e2373` |
| `src/main.exe/SetupTelop.c` | `cf3b80b2a8fda7c57f5138d89de6473214abc7fbcc36c5bc4fb6c043efc041c8` |

Latin glyph pixels and widths come from the supplied disc/executable. The Shift-JIS
KROM rasterization path is identified but not implemented; no BIOS font is copied.
Widescreen bars/fades extend to the host viewport. The source's intentional
one-second post-music hold preserves a frozen world independently of host refresh.
CD startup latency and PS1 frame-cost stalls are not simulated.

Private source-C harnesses compare event routing/death returns (with a controlled
CVA callback), camera cuts/pans (with supplied SDK square-root/direction results),
fade state/colors and score components. Separate live-world Mission 1 script and
window checks cover integration. A campaign diagnostic attempts all 126 English
sequence entries in isolation, including source-defined early returns. This evidence does not establish complete
campaign, audio or uninterrupted retail-gameplay parity.

## Results, records and inventory rules (2026-10-20)

The portable progression state follows the original stage-end bookkeeping,
per-character stock and per-layout score records. Training uses its separate
five-entry grade/time leaderboard and prize path. Original byte overflow,
locked/infinite markers, random award retries and leftover-item ordering remain
intact. Shop limits and stage prizes are read from the supplied executable.

| File | Working-tree SHA-256 |
| --- | --- |
| `src/main.exe/StageEndScreen.c` | `20251d89fe30220963dcb11471b2bd6e7908d5a7893a7c8eeb7f05ba70b3e5c0` |
| `src/main.exe/award_stage_items_.c` | `ad8bce152cc5a32036d932f31207c2bc3349e29dcf8c5e176de5540391cf308f` |
| `src/main.exe/mission_score_screen.c` | `e3b8ab0e195477eae678e0e2cb099409ef3c11556b8217279b0f7bcc408689e8` |
| `src/main.exe/BriefingAndInventorySelectionScreen.c` | `bab7073d72addfc57e69a789e42650a64cdf27bca3efa4588fdfb8fbc16a7d66` |
| `src/main.exe/START.C` | `41eb63500c09fa33d8938a9e8acc541789038d9df8971c3ee14e5d835f2591e3` |
| `src/main.exe/game_over_screen_.c` | `7e292e6ea605fa1b6ac809b4628094d807565765af7884c4f9abbcb7590b1005` |
| `src/main.exe/clamp_shop_stock_.c` | `6ff4e219bc87507b533af959b5e05c20ae595d089e3d4c653483bbb0307a35e5` |

An independent C harness checks 2,048 reward/leftover cases and exact RNG state.
Portable integration tests cover best-record ties, unplayed-layout preference,
character isolation, training ranking, loadout limits and stock retained on retry.
The original score oracle still passes after moving score calculation onto the
packed record. This commit supplies rules; host screens and saves follow.

## Campaign host and portable saves (2026-10-20)

The host uses the progression rules above and replaces memory-card UI with a
portable checked save and atomic native file replacement. Menu graphics/timing
are intentionally simple. The stage loader retains all three layout candidates
for random selection, then chooses at WORLD.C's original boundary after player
and helper construction. APPEAR.C defines armour model substitutions, consumption
of the carried armour, and the already ported damage flag. 3DCTRL.C documents the
original VSync-derived startup seed; the PC host instead has an explicit,
repeatable seed lifecycle independent of menu/display duration.

| File | Working-tree SHA-256 |
| --- | --- |
| `src/main.exe/APPEAR.C` | `6626d5d5491a74dd826fa6a6e9fb0599b3ba8b5846e2e7377ef1b0a3e79ef86b` |
| `src/main.exe/WORLD.C` | `f7999b02640fcdaecc8ee005941e3b7afb4108fbe598df6b8d8c8c19581e460f` |
| `src/main.exe/3DCTRL.C` | `1175982b0890852071a4b0f581ab637b24258d9fe2153b94d958d0edd337c45a` |

Campaign tests inject terminal outcomes to test result/retry/advance/persistence;
they do not claim to beat the missions. Separate existing event tests exercise
actual authored event chains. Original-disc checks load both characters' armour
variants and random layouts; private cadence and roster checks remain passing.

## Original sound assets and XA streams (2026-10-20)

The VAB parser, SPU/XA ADPCM decoders and cue resolver are new portable Rust.
The sound bank, note, channel and cue choices follow the decompilation at
`cd3a505559bc748ba7be0082ccea58c1d4908407`, including these working-tree files:

| File | Working-tree SHA-256 |
| --- | --- |
| `src/main.exe/AUDIO.C` | `cfef41d067b7d1099e5d64cfc4fcced2d4c0c8f85479ae79f8f482fe6f3537cc` |
| `src/main.exe/SEMNG.C` | `2acf89e99c288fa90d767245eda9c888f4ae9ea95688f332fadc0fbb02f32a10` |
| `src/main.exe/IMAGES.C` | `4ad4a53038c560c889c0057b6c1b7d3903da2585dbbd0e86682af24e94d71799` |
| `src/main.exe/OPAUDIO.C` | `8365aa533aba84ba110af4decfb66f65a806a8deade3c0de2c9f20d76e0ada64` |
| `src/main.exe/sound.h` | `405f3e09c98bcdea57852ec640f1249975e27f9bf44edf242084c2ef1e36159b` |
| `src/main.exe/music.h` | `ba4db842b2a1200f95750fabb18747bf54ed235d756640a364261a337345bccc` |

Format references: [PSX-SPX SPU](https://psx-spx.consoledev.net/soundprocessingunitspu/)
and [CD-ROM XA](https://psx-spx.consoledev.net/cdromformat/). No emulator source or
runtime is imported. The portable reader retains the raw Form 2 payload, checks
duplicated subheaders and selects the original file/channel. Cue offsets use
the game's double-speed CD conversion. Language-specific dialogue, shared
fallbacks and Japanese silent entries come from the supplied executable.

Private checks decode 104 non-trial VAB banks (9,139 samples), all 88 localized
stage bank selections and 419 distinct XA cues (399,853,440 stereo frames).
Another check compares 10,604,160 decoded samples across music and four dialogue
languages with [FFmpeg](https://github.com/FFmpeg/FFmpeg) 8.1.2. FFmpeg is a
test-only external oracle, not a runtime dependency. Synthetic checks cover
ADPCM saturation/loops, mono/stereo unit order, eight-bit units and raw sectors
with CUE pregaps. These checks establish decoding, not completed audio playback.

## Portable mixing and desktop playback (2026-10-20)

`tenchu-audio` mixes decoded samples independently of game state, clocks and
device APIs. It retains the 24-voice replacement cycle, bank/program/tone gains,
note 36 relative to each sample's tuning, loop boundaries and ADS envelope
progression at 44.1 kHz. Output uses linear resampling at the device rate. SPU
register emulation, reverb, Gaussian interpolation and CD seek timing are outside
this implementation. The desktop boundary uses [CPAL 0.18.2](https://docs.rs/cpal/0.18.2/cpal/),
with Nix providing ALSA development libraries and portable Rust dependencies.

The spatial conversion follows `SEMNG.C::SoundEx` and `AUDIO.C::PlaySE`, including
the player run-step attenuation, original angle folding and packed direction/pan
arithmetic. Program selection follows `APPEAR.C`'s stage appearance table.
`MOTION.C` supplies restored walk/run/wood/turn/crouch footsteps, dashes, landing,
ledge and jump sounds. Actor requests and position requests are currently
presented after each completed logic tick; their listener position is sampled at
that boundary and actor requests precede positional effects. Sub-tick SPU ordering
is not claimed. Voice categories are suppressed during cinematics and suspension.

The host dispatches every completed tick, including catch-up ticks. XA decoding
runs on a separate file-owning worker with cancellation and a 64 MiB cache; it
does not stall or advance gameplay. Pausing/focus loss freezes mixer playback.
Private C comparisons cover 4,096 spatial cases with supplied SDK math results.
Portable tests cover sample order across buffer sizes, pause/resume, loop/expiry,
voice replacement and envelopes. An asynchronous replacement test rejects stale
XA completions. Both characters' localized Mission 2 openings return to the
authored background cue. Isolated window captures exercise music/effects and
French dialogue, without sending sound to the user's desktop speakers.


## Environmental construction and fixed-tick updates (2026-10-20)

New scalar `misc` state follows `MISC.C`: source construction ordering, the
200-object limit, activation/deactivation before all active callbacks, door and
pitfall collisions/hinges, shared template coordinates, fire/puff/bonfire/snow
producers and ambient schedules. Asset tables and scrolling TIM names come from
the user executable. The persistent texture-scroll effect and snow particle
follow `EFFECT.C`; `3DCTRL.C` supplies model/sprite initialization and drawing.

| File | Working-tree SHA-256 |
| --- | --- |
| `src/main.exe/MISC.C` | `6b46b6d6ac8ae87c9871b3109fedd47172f5f0fa25476aaf873c3e41a894a19c` |
| `src/main.exe/misc.h` | `0579a437b71549205a5b0a70b7504ec17d794b41cd7086bba2c77eaf6f7f158e` |
| `src/main.exe/EFFECT.C` | `a50b850d80090eda8cc39770d9d3026dc6888936eb2b1324d6c2821348d4da17` |
| `src/main.exe/3DCTRL.C` | `1175982b0890852071a4b0f581ab637b24258d9fe2153b94d958d0edd337c45a` |
| `src/main.exe/CONFLICT.C` | `4fd248c53cd709febd10f8f748dabe32e7844f1b5b91ee845343f57a7b7df4c0` |
| `src/main.exe/AfsGetEntry.c` | `27ca3bd6f9e4c713185a67fff8a7b9baf3994340511ee2faf686d48c2c2b0f0c` |

The research model's `misc/door_underflow.rs` establishes that an exhausted
collision index aliases `systemAFS.maxElementArea` and two 24-bit ordering-table
links. The owner alias is not the door sentinel on this disc. The port proves a
single opening direction over all possible link values, by bounding the original
atan table quotient for positive coordinate ranges. All 43 authored doors pass.
It diagnoses unsupported oblique/out-of-range overrides instead of silently
substituting a direction. This is a bounded gameplay proof, not a GPU/memory model.

Research references at `9ac830e2234f1217daa096198cebadd3f80c0b02` (working-tree
content hashes; no imported RAM storage or simulation batching):

| File | Working-tree SHA-256 |
| --- | --- |
| `tools/tenchu-sim/src/misc/door_underflow.rs` | `fc4966d7d64e9e7589a6dc75d0a38e41429a7c34e08650ca7906a5a65c87b0d4` |
| `tools/tenchu-sim/src/misc.rs` | `e72556f510b8909c9592fa9c88cac90f2bc825a2d755f714d8f3d28e26db2fb8` |
| `tools/tenchu-sim/src/effects/snow.rs` | `a393686aa21eba156eef7d573f8cfe2a5342ad1cdd95210c7b4919287b925d37` |

Original-C tests cover 4,096 emitter cases using previously verified particle RNG
cost stubs; 4,096 hinged-object cases with supplied SDK angle results; 2,048 snow
trajectories with flat terrain and omitted projection; and 40,960 scroll updates
with graphics submission stubbed. Separate checks cover activation/resume,
persistent effect allocation, overflow snow and texture pixels/palettes. Cadence
snapshots now include misc state and texture revision. Relocated native-window
starts exercise door opening, swimming over water textures, snow and flames;
they do not establish completed missions or continuous retail parity.


## Napalm and fire attacks (2026-10-20)

`ITEM.C::ReqItemUse`, `ProcItemNapalm`, `ReqItemKaengeki` and
`ProcItemKaengeki` supply allocation, arithmetic, collision, rendering and cast
ordering. `EFFECT.C::SpawnNapalmDamage` and the Manji branch of
`MOTION.C::ActATTACK` supply the two additional producers. Their source hashes
are recorded above. Runtime code uses typed scalar item fields and ordinary
scene/effect state, without RAM or batch storage.

Research references at the recorded research revision:

| File | Working-tree SHA-256 |
| --- | --- |
| `tools/tenchu-sim/src/items/napalm.rs` | `fa2fb97a75afb6008cb71fd5da33e04be7c51b9b688e84b03bf36f8af728714e` |
| `tools/tenchu-sim/src/items/breath.rs` | `7b71390084ffc3496dd4fcea10b174bb0d2fc9f97ae67011ebc91a439ed461d2` |

Private differential checks extract the actual C callbacks: 4,096 napalm cases
with recorded terrain responses and stubbed projection, and 3,072 fire-breath
cases with previously verified motion/rotation calls stubbed. Additional checks
cover full-pool even-tick rejection, both playable casts, a Manji attack and the
retail lava-floor emitter through ordinary updates. Native QA uses temporary
loadout/position setup only; those hooks are not part of the committed runtime.


## Mesh visibility and transparency (2026-10-20)

The existing TMD decoder provides primitive winding, double-sided flags, texture
STP and texture-page blend modes. The host now uses those fields for stage,
placement, character and weapon faces as well as effects. This is a modern
GPU depth-buffer renderer: no ordering-table addresses, raster warping or PS1
frame-cost emulation. Logical packet counting remains in the core unchanged.
An offscreen GPU regression verifies positive source-screen winding, reversed
and double-sided faces, opaque/transparent texels, STP black and all four
original blend equations. Shared face traversal keeps opaque and transparent
passes consistent. Native screenshots cover Mission 1 fire breath, Mission 4
scenery and a relocated final-stage lava start.


## Sleeping gas (2026-10-20)

`ITEM.C::ReqItemUse`, `ReqItemNemuri` and `ProcItemNemuri` provide the cast,
release, moving soft collision, RNG/effect calls, state changes and cleanup.
Existing `THINK.C::Think1sleep` control is selected by the item. Motion entry
uses the original direct `SetNowMotion` rules rather than the guarded medicine
and fire-breath entry blocks. The shared `param_napalm` velocity/count layout is
represented by the same small owned Rust record for both moving sprite items.

Research reference (revision recorded above):

| File | Working-tree SHA-256 |
| --- | --- |
| `tools/tenchu-sim/src/items/sleep.rs` | `c89c230e1c622c6ca3a237094b394929a649239b9f2a4373e8decf5f307b8f21` |

The differential test extracts the original C callback and exercises 4,096
flight/contact cases, with supplied sine/terrain values and stubbed, separately
verified particle RNG costs and projection. Integration checks cover both player
throws and a sleeping guard, plus cache preservation and interruption cleanup.


## Lure flute and resurrection leaves (2026-10-20)

`ITEM.C::ProcItemManebue`, `ReqItemManebue`, `ProcItemKawarimi` and
`ReqItemKawarimi` define initialization, the thirty-tick flute flag and
thirty-one recovery-particle bursts. `MOTION.C::DamageControl` defines leaf
consumption and health restoration after damage feedback, with launch, swimming
and scripted-motion exceptions. The flute uses the existing source AI response
to `active_item == 18`; its forced-disposal flag leak is retained. Medicine and
resurrection share the identical particle loop with different original colors.

Research references (revision recorded above):

| File | Working-tree SHA-256 |
| --- | --- |
| `tools/tenchu-sim/src/items/flute.rs` | `c59c3e5f6ff595ea42495a9461074ecc4f367180385a21263b518a50129e51c7` |
| `tools/tenchu-sim/src/items/leaf.rs` | `65d31e97c4544ebeb0ad0591c304b8701ccbb756ee3afd6274ca2d2673160536` |

Differential checks extract both item callbacks and the original damage tail.
The callback comparison includes all emitted particle fields and RNG; the damage
comparison stubs item requests and final motion entry while retaining their
stock/script guards. Controlled gameplay checks exercise actual lethal arrows,
revival effects, local flute sound and forced eviction.
