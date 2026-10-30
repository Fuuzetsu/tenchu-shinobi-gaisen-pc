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

## Native inspection tools (2026-10-20)

`~/programming/tenchu-viewer`, revision
`677cdc1f37710eec17b0479e7714dc3be09f2003`, supplied the inspection reference:
ACM quarter-cell masks, slope surfaces and terrain colors; ConflictObject body
and attack boxes; standing/crouching sight envelopes; event zones, patrol paths,
free camera and picking. Its working tree contains unrelated research; the exact
consulted files, including the modified `web.rs`, are identified below.

| Source | SHA-256 |
| --- | --- |
| `src/collision.rs` | `738fcee9ad44e304ddfb017eb4633e0b6b1ef788bff2f552d5f7012c7e20ffa1` |
| `src/sight.rs` | `7a959df4291d24d72a686ee3d5091ceb9d8cde0b4ab68048409d414beb9841d6` |
| `src/spatial.rs` | `360d6bccc19b0511468af813a5adb55d302ceeb6698690b9eea2582f606e3f44` |
| `src/web.rs` | `046e76fb1fbe8e6bff9e42dd0e3016e7ea5a8c1938efffc85f1f3b1969c3f79d` |

The port adapts these ideas to owned live state without the emulator bridge.
Terrain picks use original integer `node_level` after triangle intersection.
Free-camera models derive from cloned poses; all overlays borrow the game
immutably. The original gameplay camera, visibility/packet accounting and
render-derived gameplay state continue to update normally. Sight bounds show
calm-state distance/angle limits, not an occlusion solution. The optional sight
multiplier changes distance thresholds only; the original C oracle still checks
100% behavior. Teleport is an explicit gameplay edit with pose/contact refresh,
not a claimed original function.
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


## Original map overlay and UI ordering (2026-10-20)

`INFOVIEW.C::ResetInfoview` and `PutMap` define the per-stage `CHIZU.TIM`,
slide state, ghost image, subtractive backdrop and local opening cue.
`ITEM.C::draw_map_items_` provides the divided/rotated world transform and
owner/type filtering; `EFFECT.C::DrawTargetS` supplies the crossed line markers,
including reversed RGB byte extraction. `MapPlacement` comes from the supported
executable at `0x8008e50c`, using the sixteen-byte `MapPlacementType` structure.
These source files and their current hashes are already recorded above.

`START.C` orders `DoInfoViewProc` before `DoMiscProc`; `CHRANIM.C::CVArun`
orders `DrawTelop` after `DoMiscProc`. The scalar update now distinguishes those
orders explicitly. No map texture, executable table or retail RAM is embedded.
The differential test compiles all three original map callbacks with supplied
trigonometric values and SDK submission collectors. Native presentation uses
thin modern GPU quads for the original one-pixel lines.


## Diagonal terrain probes (2026-10-20)

`CONFLICT.C::GetAreaMapVector` uses `direction` at executable address
`0x800867d4`: `[-1,-1], [1,-1], [-1,1], [1,1]`. These diagonals and their bit
order are required by `RefrectVector`/`RefrectMove` in actor, item and camera
responses. The first port's compass offsets were an implementation error;
the research model already reads the original table correctly. The source,
symbol-file and research-terrain hashes recorded above are unchanged.

The new differential check extracts the C movement probe, supplies the retail
direction table, and records five source-ordered terrain query responses from
actual mission geometry. It compares returned fields, cursor state and query
coordinates/modes. Terrain lookup itself is the supplied boundary of this check.
A separate ordinary-input integration test exercises the straight-wall trap
found during Mission 1 exploration, with both player characters.

The checked-in `crates/tenchu-pc/tests/data/mission1-rikimaru.inputs` was authored
through exploration in this port after that fix. It contains only controller
durations/masks, not retail recordings or game data. Its completion regression
uses ordinary new-game stock and actions; it is not a retail parity oracle.

## Colored rice (2026-10-20)

`ITEM.C::ProcItemGoshikimai`, `ReqItemGoshikimai` and its `ReqItemUse` branch
define the scalar planting callback and raw rotated release vector.
`MOTION.C::ActSTATE` and `game_types.h` establish the deliberate status 8
override and hand index 13. Using the animation's usual status 15 would enter
`ActITEM` and wrongly emit a mine at frame 5. The existing source hashes are
unchanged; the research `tools/tenchu-sim/src/items/rice.rs` reference has SHA-256
`0b74f40161ced0bf0bf9905d85780f246c5e87863b883aaf712002e6fca62b0b`.
The shared initializer now also applies `item.h::INITIALIZE_ITEM_FROM_REQUEST`'s
coordinate rotation refresh for every item, including reused slots; the stored
world matrix remains untouched while its validity flag is cleared.

Integration tests cover both characters, the persistent owner/map marker,
recollection, interrupted-cast restart and freeing the cast before allocating
its drop in a full pool. These are behavior regressions, not a new extracted-C
comparison or retail-state parity claim.

## Eight-shot shuriken (2026-10-20)

`ITEM.C::ReqItemHappou`, `ProcItemHappou`, `SetupFly` and `SearchItemTarget2`
define the eight-shot request, aim jitter, flight, trails, delayed hitboxes and
different humanoid/terrain impact behavior. The model is archive asset 28.
`MOTION.C::DamageControl` already supplies its thirty-point damage. The source
hashes recorded above are unchanged. The research reference
`tools/tenchu-sim/src/items/darts.rs` has SHA-256
`5493c40b19a220574da504ad1cca00776295e4afe86f800bac4464b660819a90`.

The request comparison extracts the actual C request, curve setup and initializer/
allocation macros. It supplies recorded target coordinates and SDK distances,
then checks the C aim angles through the native target lookup. Motion entry,
coordinate calculation, trail allocation and sound submission are stubbed at
their request boundaries; existing arithmetic/flight tests cover the shared
helpers. Integration checks exercise arming, actual damage, continued flight
after a humanoid hit and final terrain disposal.

## Speed scroll (2026-10-20)

`ITEM.C::ProcItemShinsoku` and `ReqItemShinsoku` define casting, the 75-tick boost,
terrain probes, trails, steering, cancellation and interruption refunds.
`EFFECT.C::RotateVectorS` uses the SDK's signed-short matrix result; steering
therefore clamps the transformed short vector. `MODEL_POSITION` aliases the
actor's position, while direct coordinate writes preserve the cached world matrix
and validity flag. `MOTION.C::ActITEM` already holds the finished cast animation.
The recorded source and tuning hashes are unchanged. The research reference
`tools/tenchu-sim/src/items/speed.rs` has SHA-256
`49b93440f71f8da3ae884652e9c6ced15d4a16cbc434112ee0fbe3be701e5502`.

The focused checks exercise ordinary casts for both characters and controlled
terrain/cache/interruption boundaries. They do not add a new extracted-C oracle
for this callback. Existing matrix, camera, terrain and particle comparisons
cover its shared helper boundaries.

## Protection charm (2026-10-20)

`ITEM.C::ProcItemGosin`, `ReqItemGosin`, `item.h::param_gosin`, `tuning.h` and
`effect.h` define the cast, unsigned sixteen-bit timer, attached pulse, refund
and unconditional active-item cleanup. Existing combat damage logic consumes
the active-item flag. The recorded source hashes are unchanged; the research
`tools/tenchu-sim/src/items/charm.rs` reference has SHA-256
`7853b2ff6872d04c04fc1b501714d71e26ff9fa4150bced4cc57d10256afd54b`.

An extracted-C callback check exercises active-phase timer edges and disposal,
including an unrelated active item. It compares RNG and every emitted pulse
field/parent; effect allocation is stubbed at the request boundary. Casting
and interrupted refunds are separate behavioral integration checks, not new
retail trace comparisons.

## Decoy doll (2026-10-20)

`ITEM.C::ReqItemNingyo` and `ProcItemNingyo`, `item.h::param_ningyo` and the
existing tuning constants define rolling, growth, target changes, collision,
the three-doll cap and cleanup. Model archive asset 27 is drawn through the
existing item mesh path. The original camera owner is assigned once per stage
in `WORLD.C`; the scalar request retains that player ID for target restoration.
The research `tools/tenchu-sim/src/items/decoy.rs` reference has SHA-256
`583e2d6fd26041656988e09f53441b3878e1c05f2cd6525b33e39c25d5d7f77f`.
The existing source and header hashes are unchanged.

Behavior checks cover ordinary throws and growth for both characters, target
eligibility/restoration, consumed contacts on retarget ticks, knockback edges,
fourth-hit break, overflow refunds and water cleanup. They add no new full C
callback or retail replay comparison; rolling, collisions, matrices and effects
use the shared previously checked routines.

## Disguise (2026-10-20)

`ITEM.C::create_ninken_character_`, `ProcItemHenshin`, `ReqItemHenshin`,
`item.h::CaptureHenshinModel` and `ApplyHenshinModel` define the two stage
snapshots, fifteen signed-short offsets, root-motion scale, casting, shared
600-tick timer, replacement and restoration. Existing `THINK.C` ports already
consume active item 11 and the negative alert timer. Recorded source hashes
are unchanged. The research `tools/tenchu-sim/src/items/disguise.rs` reference
has SHA-256 `be333adcff687760a341b17185271787e09b7a8d813283b4094b4fe0857f60a2`.

The port keeps actor identity and gameplay definitions separate from the chosen
body mesh. Model application preserves parent links, rotation matrices and
coordinate-cache validity; the animation state receives the same offsets and
scale. Rendering captures the chosen mesh at the existing body draw boundary.
Ninety-six extracted-C snapshot cases check signed-short offset truncation and
height; additional checks cover cache preservation, both players' cast/movement/
expiry, interruption, replacement, forced eviction and attack/damage/death
boundaries. These checks do not establish full retail replay parity.

## Ninja dog (2026-10-20)

`ITEM.C::ReqItemNinken`, `ProcItemNinken`, `create_ninken_character_`,
`HUMAN.C::GetNearestHumanoid`, the shared motion/weapon helpers and the recorded
item/tuning headers define summoning, the 1,800-tick lifetime, target changes and
parking. The research `tools/tenchu-sim/src/items/dog.rs` reference has SHA-256
`413c7e457236be57ab43f87d257e148cda790debadfb23d5d54c26bff3aaf7fd`.
The matching C hashes remain unchanged (`HUMAN.C`:
`9138dbd55667b99dec7b7703ac58a6325dc70654bda3f0612b2af33340892158`).

The stage retains the original dog slot reference across layout removal and
recreates the actor on demand under the source's two existence checks. Ordinary
input checks cover both players, landing attacks, expiry and reuse. Controlled
checks cover target filters/status gates, repeated idle sheathing, occupied-dog
retries, disposal-before-refund allocation and inventory cleanup. These are
behavioral checks using shared verified helpers, not a new full C callback or
retail trace comparison. A dangling summon reference to an already freed actor
produces an explicit diagnostic rather than reading freed memory.

## Lightning (2026-10-20)

`MOTION.C::launch_lightning_bolt_` and its four attack branches,
`ITEM.C::ReqItemLightningBolt`, `ProcItemLightningBolt`, `ReqItemUse`, and
`EFFECT.C::SetLightningI` define launch timing, rotation, repeated target lookup,
postdecrement lifetime, collision, particles and line generation. Their recorded
hashes are unchanged. The research reference `tools/tenchu-sim/src/items/lightning.rs`
has SHA-256 `9371073d6002eb0387c2ac685a04996412921fea87ce5bb71567df1c91e2bbee`.

Ninety-six extracted-C segment-generation cases compare coordinates and RNG with
the scalar loop, including sub-segment lengths and distance rescaling. SDK square
roots are supplied to C; its generated points are projected through the existing
separately checked native projection in both forward and reversed views. This
does not independently revalidate the SDK projection. No persistent effect slot
is allocated for bolt lines; visible segments contribute their original 20 bytes
to activation accounting, and hidden segments still consume jitter/branch RNG.
Native rendering shares the existing screen-line geometry with grappling cables,
using the bolt's source color and additive blend.

Ordinary-tick controlled fights exercise all four attacks against both characters.
A callback check covers suspended-target inclusion, clock-driven re-aiming,
persistent hitboxes between strikes, the final zero-count callback, forced disposal
and player-local casting sound. These are focused behavior checks rather than a
complete final-boss encounter or full retail replay comparison.

## Debug teleport and vector normalization (2026-10-20)

`ITEM.C::ProcItemTeleport`/`ReqItemUse` and
`CAMERA.C::SnapCameraTargetVector` define the original debug item, its camera-owner
movement, short ray, range check and raw translation write. `CAMERA.C` has SHA-256
`759da5c93aaa780ec4dc113bc0b49b18eeee7bc27f63c4b2024a5b520b4c5e86`.
`game_types.h` identifies item 24 as the debug menu's "the world"; normal equipment
selection remains unchanged.

The scalar `VectorNormalSS` implementation follows retail instructions at
`0x80077b04`–`0x80077bf0`, with the lookup based at `0x80090a2c`, including accesses
before that table for zero/overflowing squared lengths. The verified user
executable supplies the table at load time. `tools/sdk_normal_oracle.py` executes
only this original SDK instruction range and its two GTE commands for tests;
it is not part of the runtime. Its small interpreter structure was adapted from
the previously recorded research `native/sdk_rotation_oracle.py` reference.
No retail instructions, tables or captured RAM are bundled in the repository.

The 1,031-vector corpus compares the actual SDK result, including short extremes,
zero, signed overflow and variable-shift behavior. Teleport checks cover the
127-unit step produced for a 2,000-unit axial reference, passage/no-hit fallback,
quantized range acceptance, movement of the camera owner rather than item owner,
untouched coordinate caches, held sight and forced-disposal camera behavior.
The reticle drawing is shared with the existing hook and aimed shuriken paths.

## Native distance fog (2026-10-20)

`3DCTRL.C::InitGraphicsSystem` initializes the black far color, projection 300,
depth-cue slope `-0x7ef4` and offset `0x2f282e0`; `EndDrawing` clears to that black.
`drawzGT4` at `0x8005e658` applies the GTE depth cue to the original vertex colors.
The renderer uses the corresponding continuous camera-forward curve, sharing the
constants already used by scalar projection. It evaluates fog per fragment,
preserving modern perspective interpolation rather than copying original
per-polygon cue selection/quantization. HUD and sprite graphics remain separate;
plain shadow meshes are not fogged and the hook retains its selected drawing bank.
No host-renderer calculation changes logical packet counters or game state.

| Source | Working-tree SHA-256 |
| --- | --- |
| `tenchu-decomp/src/main.exe/3DCTRL.C` | `1175982b0890852071a4b0f581ab637b24258d9fe2153b94d958d0edd337c45a` |
| `tenchu-decomp/.shake/gen/main.exe/asm/nonmatchings/drawzGT4/drawzGT4.s` | `43aff0b0349765d41ea33a452aa7a218a4940f86971675f80ba7c1301a491df9` |
| `tenchu-viewer/src/fog.rs` (continuous curve reference) | `91412f619eb8b9365a3c70330d66b24aebbc9a0ae8630391bdc4e643e009dced` |

An asset audit across eleven stages and both character selections finds baked
vertex colors throughout, with one lit primitive in stage 9's actor `0x8b`, part 13.
The original `DrawTMD` stops at a lit primitive; the native opaque/transparent
builders now share that traversal rule. Its assembly listing at
`tenchu-decomp/.shake/gen/main.exe/asm/nonmatchings/DrawTMD/DrawTMD.s` has SHA-256
`305bc9883166ec07b69d4f81272c7d59c8e77ec523b5491ad0aaecb3c0586320`.
A private-disc regression preserves ANI's seven triangles and three quads,
omitting the final lit triangle and any subsequently appended faces. The logical
packet counter already had this stop rule. The audit reads private disc data and stores no game assets
in the repository. GPU checks cover near/far depths, translated/rotated cameras,
texture transparency/blending and unfogged screen graphics.

`MOTION.C::ActHANG` retains an independent height counter while `HangCheck`
adjusts the actor position. The scalar release loop was corrected after an
ordinary Mission 6 guard reached its diagnostic limit. The original executable
at `0x800247b8` loads the counter into `s0`; `0x800247c0` increments it by 100,
and the call's delay slot stores it before `HangCheck`, without reloading the
adjusted height. `HangCheck` also excludes `ACTIVE_ITEM_DISGUISE`; that guard is
now retained. The `MOTION.C` hash remains
`7d5cddc279921e20a9e1e0066370d08b1f0dd8e81a2b047d46e0995ef5df7a82`.
A private 144-case oracle compiles both original functions over a synthetic
two-level ledge, stubbing cardinal movement and terrain queries with the same
fixed geometry used by Rust. It compares release positions, results, motion
requests and velocity across dimensions, disguise and beast guards. This is an
isolated source comparison; the separate Mission 6 input regression covers the
originally failing guard within ordinary gameplay.


## Japanese subtitle rendering (2026-10-20)

`SetupTelop.c`, `telop_text_width_.c` and `draw_telop_line_.c` above define the
Shift-JIS detection, sixteen-character cap, fifteen-row strip at (768, 496),
outline and width-minus-one quad. The custom star is read from the supplied
executable at `0x8008f078`. The port now includes the original forty-byte textured
quad contribution in logical drawing costs. The prepared caption follows the
existing DrawTelop/CVA update order and stays cached between text changes.

The replacement glyphs come from GNU Unifont JP 16.0.04 under SIL OFL 1.1.
[Font provenance, conversion and license](crates/tenchu-assets/data/japanese-font/README.md)
are bundled with the bitmap and installed by Nix. Glyph shapes replace the BIOS
KROM; neither BIOS data nor a system font package is needed. All Japanese CAD
captions fit the bundled Shift-JIS glyph set. This is presentation substitution,
not a claim of pixel-identical console fonts.

## Continuous Castle-start comparison (2026-10-20)

`CAMERA.C::CameraType1` selects the swimming pose for `STAT_SWIM` (3), not
`STAT_CEILHANG` (13). `jt_init4` sets the adaptive drawing depth to `0x25a` and
both extents to `0x200`; the on-disc SDK globals precede that initialization.
These corrections were checked against `tenchu-decomp` revision
`47c1ee292a779704865963636fe5672b1f9455fb`, with these source SHA-256 hashes:

| Source | SHA-256 |
| --- | --- |
| `src/main.exe/CAMERA.C` | `759da5c93aaa780ec4dc113bc0b49b18eeee7bc27f63c4b2024a5b520b4c5e86` |
| `src/main.exe/jt_init4.c` | `6de1321f2d6b6f0a77a7c8cfcb0f5ebab176b15a8d743b3e194e2b248c195e20` |

The private `tenchu-castle-approach-validation-20260925` corpus supplies seven
hash-bound original recordings. The core test documents their precise horizons
and initial loading-pose and ambient-deadline alignment. It runs disc-constructed scalar
worlds with no subsequent state refresh and compares exact drawing totals in
addition to gameplay state. Neither captures nor the research runtime are
included in the port. The research allocation report helped localize the
subdivision mismatch; the original startup code establishes the correct values.

`MOTION.C::INSERT_WEAPON_CONFLICT` copies the complete `WeaponDB.confp`
SVECTOR, including its radius in `pad`, into the collision offset. That word
is the contact count until the next `ComputeAllConflict` pass. The longer
`wall-grapple-long.trace` exposed this difference at clock 1,254; all 4,000
updates match after preserving the assignment. Its SHA-256 is
`9486c05b11149c82acd093231bf4ae5d6e89e22f6586499e3a64b84331555859`.
`MISC.C::proc_misc_sound_` initializes deadlines from the pre-loading GameClock;
`STAGE.C::StartStageSequence` later resets the clock. The fixture retains 644,
which is supplied only to the test's initial conditions. Source hashes:

| Source | SHA-256 |
| --- | --- |
| `src/main.exe/MOTION.C` | `7d5cddc279921e20a9e1e0066370d08b1f0dd8e81a2b047d46e0995ef5df7a82` |
| `src/main.exe/MISC.C` | `6b46b6d6ac8ae87c9871b3109fedd47172f5f0fa25476aaf873c3e41a894a19c` |
| `src/main.exe/STAGE.C` | `dc1573257be1b02eeaaa96cd1e3c6ce8b952da3dbd4a122e669a29e4d819a391` |

## Original menu and ending (2026-10-21)

The matching MENU/ENDING decompilation at `~/programming/tenchu-decomp`,
revision `7e13c76e48a79ef8b187a838867670a945b789bd`, supplies the screen flow, archive indices,
positions, animation registrations, caption/voice scheduling and skip rules.
These are rewritten as owned Rust state and image composition. Disc movies
use the Nix-provided FFmpeg MDEC decoder at the native host boundary; XA and
VAB audio use the existing portable decoders. No SDK, emulator, sibling source
or extracted assets are required at runtime. Native saves replace memory cards.

| Consulted source | SHA-256 |
| --- | --- |
| `src/menu.exe/MAIN.C` | `5059dfd34680441e385b92cf187fb19e00764cdd02f5298b612afee5d65fdbf3` |
| `src/menu.exe/TITLE.C` | `c6a0f380051d0857c4671041f5ad10192c88c44842c2750d5f431bae65fa5cd6` |
| `src/menu.exe/TITLEMESH.C` | `8953014a46e45a8273a72bfbff4a556f46d1d3207d2d7a6c05502d8d73dd644a` |
| `src/menu.exe/CHARACTER.C` | `3ff34ca92a9aff208bc6383025a284f2954ae4219f3fc5fcfea08c5c670613a8` |
| `src/menu.exe/MISSION.C` | `e0d49f8827cf1f3429b96a76bbc099cf271c81322ce640d022d7398c1a29680f` |
| `src/menu.exe/OPTIONS.C` | `5313a6fbe64a3607230ee00da7ae0e575687245719dc375c4e1f29d73619950c` |
| `src/menu.exe/ACTION.C` | `33ce7904c2e31aa6cc98dd4de8e62f6578e84440c9a981bac1d6a60f8637285c` |
| `src/menu.exe/3DCTRL.C` | `1fa4a4b185d1f9bf9776fcd7c82bcda6a3460620ea4a6e120880f5fcecc64e60` |
| `src/menu.exe/OPMOVIE.C` | `9548653c76340a5e00e8b782d6835bfbccf0251dcc0f7cd5baf86d5907524e48` |
| `src/menu.exe/AUDIO.C` | `ac7678fe77676f9d9ddf3fec2042a89663f012ea44c332d901ab1b9befe0f26b` |
| `src/menu.exe/SEMNG.C` | `50d497dccc40cde4eecfbca7627efc9df228d0c3033999c855b94783790cca55` |
| `src/menu.exe/persistent.h` | `90a102bc17a8e77e59cabc4cc95ae6f0927ca1ad3833acb29a2b72dd184d6b4b` |
| `src/ending.exe/MAIN.C` | `9eb229c838e98cf7dafacb431d9cc0e473874719e7a004ff629c2551c56ae62f` |
| `src/ending.exe/SUBTITLE.C` | `b206a40f12dee52f3b1f3bb88f424efd34ff47c33049b22363394abdd97e7198` |
| `src/ending.exe/RESULTS.C` | `560d0fa8f915bca27db74e395c14b9166ccee2c0e48fa03df9ae1064a620389a` |
| `config/symbols.menu.exe.txt` | `c35e2755cdaa0e6a0e2e0276883f281ac292aecc19e10ded06505eb945f9a8b2` |
| `src/main.exe/PADCMD.C` | `e113fecf2c20e8e26140153569eeb9a746ef57f3ded1b4bb6b83fb36d117c7d0` |
| `src/main.exe/MOTION.C` | `7d5cddc279921e20a9e1e0066370d08b1f0dd8e81a2b047d46e0995ef5df7a82` |
| `src/main.exe/tuning.h` | `d24d0528aafd617a572eb1201edb3518d8ff497adaae82e44a9695e7c4d7897e` |
| `src/main.exe/main.exe.h` | `28ef0d40cd65e3815173ffe8a52a9daea57f44c0e226029a61ba6fd521656206` |

The original vibration option now controls native gamepad feedback. MOTION's
attack, landing, swimming, ledge, damage and authored stealth-death cues are
presentation events; the host filters for the player and sends finite effects
through gilrs. PadProc's envelope advances twice per 60 Hz callback. Its attack
and release durations are adapted to the host driver's timing resolution;
motor hardware behavior is not emulated. The original-C damage comparison also
checks the final vibration request without changing gameplay state or RNG.


## Browser host (2026-10-22)

Original Rust game/asset/frontend code is shared with native builds. Browser
platform adapters use [winit's Web API](https://docs.rs/winit/0.30.12/winit/platform/web/index.html),
[WebGPU](https://www.w3.org/TR/2026/CRD-webgpu-20260521/), wasm-bindgen and CPAL's
Web Audio backend. The native preparation tool uses the existing XA decoder
and pinned FFmpeg to encode Opus audio and VP9 movies; caption/voice timing still
uses the original disc commands. Sparse immutable disc pages are this port's
implementation, not an emulator or a copy of the parallel TAS execution model.
[Chromium's GPU testing notes](https://chromium.googlesource.com/chromium/src/%2B/HEAD/docs/gpu/using-gpu-hardware-in-headless-chrome.md)
informed browser validation; headed Chromium under an isolated X server was
used because this machine's headless compositor did not capture WebGPU reliably.

The WebGL2 fallback follows wgpu 24's
[adapter detection helper](https://docs.rs/wgpu/24.0.5/wgpu/util/fn.new_instance_with_webgpu_detection.html)
and downlevel limits. Its published `wgpu-hal-24.0.4/src/gles/mod.rs`
`Texture::get_info_from_desc` explains the square/multiple-of-six cubemap
heuristic; padding array textures avoids that inference without vendoring the
dependency. Metadata texture access and the host selector are new port code.
That backend's `ProgramCacheKey` also omits pipeline override constants;
specialized scene shaders use distinct modules so toggling effects cannot reuse
a program compiled with different feature values.
Its depth textures retain default filtering when fetched without a sampler.
Post-processing now supplies an explicit nearest sampler for non-MSAA depth;
this avoids incomplete textures returning zero under
[WebGL2's texel-fetch rules](https://registry.khronos.org/webgl/specs/latest/2.0/).
[Mozilla's experimental-feature documentation](https://developer.mozilla.org/en-US/docs/Mozilla/Firefox/Experimental_features)
identifies the `dom.webgpu.enabled` preference; enabling it is not required for
the WebGL2 backend.

Firefox presentation profiling used ordinary Wayland windows on a 60 Hz display,
GL completion waits, a diagnostic run that omitted presentation, and Mozilla's
[native profiler](https://firefox-source-docs.mozilla.org/tools/profiler/code-overview.html).
Firefox 147's `about:support` reports `DMABUF_WEBGL` blocked on this NVIDIA driver.
[Bug 1924578](https://bugzilla.mozilla.org/show_bug.cgi?id=1924578) records the
reason for that block; [bug 1981326](https://bugzilla.mozilla.org/show_bug.cgi?id=1981326)
allows newer NVIDIA drivers on Nightly only. An isolated Nix-packaged Nightly 160
test reached 60 FPS with interpolation at the same full resolution where 147
release measured about 24 FPS; a 3840×2160 fullscreen run also maintained 60 FPS
while moving the camera. Skipping the final blit was a diagnostic only and
is not an optimization: it stops displaying new images. Runtime changes instead
retain genuinely unchanged browser frames and cache unchanged egui shapes.

A captured Firefox content-process stack during an intermittent startup hang
stopped in `AudioContext_Binding::get_outputLatency`, waiting for synchronous
`audioipc2`/cubeb IPC. CPAL 0.18.2's Web Audio backend reads this property for each
buffer's callback timestamp; this port ignores that timestamp. The game output
context supplies zero for that unused estimate, without altering its audio clock,
buffer scheduling, movie context or the browser's global API. Browser tests check
that the hardware getter is never called and non-silent PCM is still scheduled.


## Surface relief (2026-10-22)

The new height inference, mip gradients and WGSL implementation are written for
this port. The bounded ray/height intersection and distance transition follow
the general method described by Natalya Tatarchuk in
[Practical Parallax Occlusion Mapping (SIGGRAPH 2006)](https://www.realtimerendering.com/advances/s2006/Chapter5-Parallax_Occlusion_Mapping_for_detailed_surface_rendering.pdf).
This implementation does not add the paper's geometric silhouettes or
light-ray self-shadowing. Brightness is explicitly treated as an estimate;
authored per-texture height maps override it without modifying the game models.
Actual disc roof/wood textures are used only in private validation captures.
Their identifiers in the diagnostic test select views and do not select runtime
material behavior.

## Optional effects and geometry refinement (2026-10-24)

Persistent stains, surface queries, weather particles/coatings, the sky clock,
water shading/curves and secondary-motion springs are new presentation code for
this port. They consume original game state without changing core simulation.
Character and shrub subdivision uses the general Loop subdivision rules described
in [Charles Loop's thesis](https://www.microsoft.com/en-us/research/publication/smooth-subdivision-surfaces-based-on-triangles/),
with local displacement bounds and pinned open/nonmanifold edges for these assets.
Connected foliage layers preserve source UV/material boundaries.

`data/water.json`, `data/vegetation.json` and `data/motion.json` record explicit
classifications and reviewed player regions from the user's disc. Fingerprints
identify source assets; no texture pixels, models or captured game memory are
included in those metadata files. The existing private campaign HD artwork is
reused without new image generation. Original/HD scene reviews, recorded combat
checks and browser results are under the ignored `private/effects-review` folder.

## Runtime texture recipes (2026-10-28)

Portable Rust recipes reconstruct mixed HD textures from the user's disc and
shipped generated layers at menu/mission load. Integer compositing and resampling
rounding follow Pillow/PIL; [the compatibility notice](crates/tenchu-pc/data/image-compat/README.md)
records the exact source files and MIT-CMU license. Synthetic golden tests and
private accepted-pack comparisons verify the result. No Python, Pillow, network
image generator or interpreter is needed by the game, including its WASM build.

## Retail mission preparation and results presentation (2026-10-30)

The narrated briefing, item-shop grid and ordinary/training score screens use
MAIN.EXE's localized asset pointers, narration cues, scrolling tables and item
positions. The UI reuses the stock/reward/record rules above. Rendering, input
adaptation and asynchronous audio integration are new portable Rust; no images
or narration are embedded. Japanese Ayame's castle/final briefings retain their
separate narration cues. The existing ending presentation remains after credits.

Reference: `~/programming/tenchu-decomp` at
`bd843abe11d555ca8c584b8b72537bb5efe2c773` (working-tree file hashes below).

| File | Working-tree SHA-256 |
| --- | --- |
| `src/main.exe/briefing_screen_.c` | `09fa08cc5cf0d745d2b75e2265c9cd890b6756b795c1ec883dc1aeaaa32f1d7f` |
| `src/main.exe/BriefingAndInventorySelectionScreen.c` | `bab7073d72addfc57e69a789e42650a64cdf27bca3efa4588fdfb8fbc16a7d66` |
| `src/main.exe/StageEndScreen.c` | `20251d89fe30220963dcb11471b2bd6e7908d5a7893a7c8eeb7f05ba70b3e5c0` |
| `src/main.exe/mission_score_screen.c` | `e3b8ab0e195477eae678e0e2cb099409ef3c11556b8217279b0f7bcc408689e8` |
| `src/main.exe/INFOVIEW.C` | `e4aef1960f32907c3217bbdb3552204f99693124261ede8e9aaa3b6deba15af0` |
| `config/symbols.main.exe.txt` | `2e72575d7c08c455a1212ce9b461ac84d51107c5e043b13b03c39fea6510ebd8` |
