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

## Optional walking, sheathing and unarmed controls (2026-11-06)

These are new gameplay options, not claims about retail player controls. Runtime
imports read the user's Gaisen disc: NinjaA/Kunoichi walk and draw/stow motions
from `HUMAN/MOTION/STAGE2.AMD`, and Onikage's grounded attacks from
`HUMAN/MOTION/STAGE4.AMD`. Character registrations and battle/hitbox data come
from the supported executable. The existing `MOTION.C`-based actor logic supplies
weapon swaps, animation timing, combo windows and limb hitboxes. No motion data
is checked in. The separate Tatsumaru importer is described below.

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

## TRIAL mission resources and terrain (2026-10-31)

`~/programming/tenchu-decomp` at `6421c970e3eaf24b19462e785ebcbfcd5bc070a7`:

| Source | SHA-256 | Use |
| --- | --- | --- |
| `src/trial.exe/mission.h` | `fdfa11e1dcee4eb079ffe400ee658574427f150735c15bc48ddc219f541adebc` | Mission records, tile map and patrol serialization |
| `src/trial.exe/MEMCARD.C` | `05d0acd6dc281dc6eef830aea5cc9d5b422bd2b401003ad3166d9433c71a2755` | Mission checksum; excludes time records |
| `src/trial.exe/BuildTrialCollisionMap.c` | `844ff8a04c6bc7140d95866a22b3bec8c084375ac642bbc8d2ddd2c4061baa22` | Collision surfaces, masks and merging |
| `src/trial.exe/BuildTrialTerrainModel.c` | `988db9e146df865cfd02775e035151d7e02809f7a4281937e98ea6097dbbf3d2` | Exposed faces, corner shading and ornament placement |

The accompanying `GetTrialTerrainHeight`, `FindOrAddTrialTerrainVertex`,
`SetupTrialTerrainPrimitive`, `UpdateTrialTerrainChunks`, `LoadTrialEditorTheme`,
`EDITORUTIL.C`, `MENULIST.C` and `SetupTrialObjectivePlacements` specify geometry
and mission initialization. The implementation uses owned Rust structures and
checked input; it does not reproduce out-of-bounds editor reads. Character,
motion and face tables are read from the user's verified TRIAL.EXE (SHA-256
`f236f35ad8d4bed8c70f6cc1e69b62e7dfd27469e6bf1a838aa65932335d7230`).
No original tables, models, missions or textures are embedded in the source.

The trial runtime follows `SetupTrialMission`, `RunTrialMission`,
`ActivateTrialHumans`, `InitTrialTrapMap`, `UpdateTrialArrowTraps`,
`UpdateTrialCollapsingFloors`, `DrawTrialMissionTimer`, `DrawTrialEnemyCandles`,
`APPEAR.C` and `SEMNG.C` at the same revision. It shares the scalar actor, item,
collision and audio engines with campaign play, with separate trial timing,
objectives and activation. TRIAL weapon/motion/death-event tables and the older
FLOWER.MAD revision are loaded from the disc.

The editor uses `RunMissionEditor`, `EDITORUTIL.C`, `EditTrialMission`,
`MENULIST.C`, `MEMCARD.C` and `SetupTrialObjectivePlacements` as the behavior and
file-format specification. The host uses a native/browser egui interface and
ordinary owned missions, rather than emulating memory cards or the original
screen layout. Imported file bounds, supporting tiles and adjacent patrol
segments are checked before edits are accepted. All sixteen challenges are
available without the original unlock gate. Campaign saves remain independent.

Validation includes decoding/encoding every disc preset, geometry and collision
for every theme, both heroes through all presets and five outcomes, arrow and
collapsing-floor traps, all-theme combat/audio, editor undo and save roundtrips,
and host return without campaign mutation. These source-backed tests and native/
browser playtests do not claim exhaustive TRIAL emulator parity.

### Optional player death variety

The presentation-only alternate deaths select NINJAB's `MOT_DEAD` and KUNOITI's
`MOT_DEAD_ALT` from the user's MAIN character registrations and `STAGE2.AMD`.
Both use the protagonists' fifteen-part hierarchy; KUNOITI's normal death is
already Ayame's motion. The shorter collapses hold their final poses while the
original simulation completes its death sequence. No original animation data
is embedded, and the selector never advances gameplay RNG.

### Optional guard behavior profiles

`tenchu-core::ai::profiles` implements the opt-in proposal in issue #58; it is
new gameplay, not a claim about differences between PlayStation editions.
Profiles reuse the scalar `THINK.C`-derived controllers above for patrol, watch,
trace, attack cooldown/combo inputs, evasion and covered reinforcement calls.
The original transition, sight and collision engines remain authoritative.
Disabled profiles take the original dispatch path without assignment RNG or AI
state resets. No code, motion tables or assets from another game are imported.

## Optional Stealth Assassins dismemberment

The user-supplied USA Rev 1 `MAIN.EXE` (SHA-256
`8ae3cbddd4b110ca5e4b6a4e7d3dd7ed6c649dbea7b92cceb218563662a2708b`),
ActDEAD at `0x80025d10`–`0x80026a38`, specifies the optional selection and
severed-part movement in `tenchu-core/src/game/dismemberment.rs` and
`actor/death/dismemberment.rs`. Inspected locally with Capstone; no executable,
model or animation data is included. Existing Gaisen models supply the parts.
See [cross-version research](docs/cross-version-research.md) for regional
probabilities, timing and asset comparisons.

The optional Rikimaru/Kataoka execution sequence uses locally compared
`ANIM/STAGE11R.CAD` from USA Rev 1 and PAL SA. Their non-caption commands match;
sequences 20 and 100 differ from Gaisen in actor placement, execution framing
and scripted blood. `game/cinematic/execution.rs` applies those differences to
the user's Gaisen command stream, converting camera coordinates by ten and
retaining each language's captions, waits and music. No CAD, dialogue or sound
data is included. Disc-backed validation compares the complete on/off caption
and music schedules in English, French, Italian and Japanese, and resumes a
serialized state during the execution.

## Optional Tatsumaru moves (2026-11-07)

Local analysis of Tenchu 2 Europe SLES-02452, GAME.EXE SHA-256
`55151e486984c1794ce4f48ab7e91b2d3a5b1abeb7a30ab8d1b326b2606d5422`:

- Resource name hashing at `0x800154c0` and VOLUME directory/file records;
  `o:\cdimage\game\cab\game\T_00.cab` contains `t_00.mot`.
- Motion relocation at `0x8001a488..0x8001a578`, lookup at
  `0x8001a588..0x8001a660`, and root-height scaling at
  `0x8001af9c..0x8001b024`. Sparse xyz/time keys are compatible with the existing
  Hermite sampler after remapping T2's interleaved arm/hand/leg bone order.
- Tatsumaru action/clip registrations in `datamng.bin` at file offset `0x2af4`.
  The normal combo is `0x900 → 0x901 → 0x904 → 0x905 → 0x906`, using clips
  `0xb8, 0xb9, 0xba, 0xbb, 0x4bf`; the transitions are at
  `0x8002a148..0x8002a16c` and `0x8002a358..0x8002a384`. Actions `0x902/0x903`
  (clips `0x4bd/0x4be`) are the directional second-strike variants, not the
  third/fourth attacks. Both variants continue into `0x904`. Root tracks remain
  unchanged. The extra final strike uses a port-only action ID.
- Grounded attack selection at `0x80022e8c..0x80022ef8` supplies the right/left
  strikes (`0x907/0x90a`), dash lunge (`0x910`), back strike (`0x913`) and
  crouched strike (`0x914`). Gaisen's backward lunge uses the back strike;
  unsupported Gaisen side-combo continuations are disabled for this moveset.
- `weapon/atktbl.dat` stores 16-byte rows: clip, power, steering, combo, start/end,
  ribbon timing and the striking-part bitmask. `ActAttack` at
  `0x80029924..0x8002aef8` starts/ends hits at row bytes 5/6;
  `0x8001f604..0x8001f794` enables the parts selected by the mask at byte 12.

This is a port-side combat option using the user's imported animation and timing
resources, not a Tenchu 2 simulation. Gaisen supplies collision/reactions and
input chaining; its fist sphere is centered on each selected hand or foot.
PAL's movement-speed 6/5 correction is omitted at the port's nominal 30 Hz.
No T2 motion, model, texture, audio, executable or extracted pack is checked in.

## Experimental Tenchu 2 scenery (2026-11-07)

The same user-supplied Europe GAME.EXE identified above was inspected locally
with Capstone. `tenchu-assets/src/tenchu2/` reads the user's VOLUME resources;
no executable tables, extracted models, textures or captures are included.

- MSD cell coordinates and map-4 vertical spacing: `0x80046800..0x800468cc`,
  loading at `0x800472f8..0x800473b4`. Cells use X/Z/Y ordering; MCD uses Z/X.
- MDL relocation and global vertex identities: `0x800180f4..0x8001830c`.
  Scenery has one root, eight-byte vertices and 28-byte indexed textured faces.
  `0x80018f98` onward supplies corner intensity indices and blend-mode handling.
  Opaque texture regions clear the blend flag when no texel uses PS1 STP.
- MCD cell loading at `0x8002cc94`; floor query at `0x8002cd88..0x8002d2e0`.
  The adapter keeps exclusive bounds, division masks and the two slope formulas.
  The grid query also preserves source list order, the `0x100` blocked material
  (`attribute & 0xf00`), and the terminating masked holes on flat `0xc000 == 0`
  entries. Gaisen's own ACM query remains separate. Verified 57,500 queries over
  all 23 populated maps against this executable routine running in Unicorn,
  including 20,949 blocked results. This tests the floor routine, not the whole
  Tenchu 2 movement/AI simulation. Other material flags remain sandbox limitations.
- OSD placement at `0x8004ef08..0x8004ef28` supplies pitch/yaw. Type-zero objects
  dispatch procedural effects (`0x8004e9c0`, including indices 16/17), rather than
  indexing mapOBJ.arc; these and scripted effects are counted but not imported.
- LAY actor positions and patrol points: `0x8002ea84..0x8002eeac`. Sequence
  descriptors at `0x80082bec`, read by `0x80060c04..0x80060c60`, map each layout
  to its scenery. GAME.EXE is hash-checked before reading this table.

`tenchu-pc::tenchu2` is a new, explicitly approximate gameplay adapter: Gaisen
heroes and optional guards explore imported scenery, with separate texture
storage. It does not reproduce T2 AI, scripts, water, hazards or mission outcomes.
Source-formula collision tests and native play checks are not emulator parity.
See [usage and boundaries](docs/tenchu2-sandbox.md).

### Tenchu 2 character models (2026-11-07)

The optional importer reads the user's SLES-02452 disc. Its `datamng.bin`
(SHA-256 `089bdad062d085d8692399c9379bfb62633d81ac17c75c49543aae3ec5ff5773`)
is loaded at `0x800ce598`; the character descriptors at file offset `0x6958`
have 24-byte rows containing kind, model-name and motion-registry pointers.
These supply 78 distinct models; the executable's actor resource names also
identify `rikia`, `ayamea` and `tatua` alternate costumes. The importer reads
MDL/ARC pairs under `o:\cdimage\game\actor`, retaining global vertex identities
and each vertex's source joint across faces. It maps the fifteen body joints to
Gaisen's order, attaching additional accessory joints to their nearest supported
ancestor. Source idle tracks are read from the character-specific motion CABs;
Gaisen gameplay and weapons remain in use outside the separate Tatsumaru import.
No source models, textures, animations or executable bytes are distributed.

For dismemberment, vertices on detached meshes follow the detached part rigidly;
vertices belonging to the corpse keep the missing joint's attached animation
transform. This separates the two sides of source joint seams without changing
Gaisen's severing decisions, animation, or detached-part physics. Disc-backed
regressions move heads and both arms away from eight imported bodies and verify
that neither side stretches across the gap or mutates gameplay state.

Sandbox voice selection is a presentation adaptation: imported female bodies
use Gaisen's KUNOITI combat bank, and imported Ayame/Rikimaru use their Gaisen
hero banks. Donor programs are resolved from Gaisen's appearance tables rather
than assuming a fixed mission slot. This does not import Tenchu 2 voice events
or change the selected Gaisen guard AI.

### Tenchu 2 sandbox music (2026-11-07)

The hash-checked Europe GAME.EXE supplies the music archive name at `0x8001323c`
and the eight-byte cue table at `0x8007f354`. XA setup (`0x80016ae0`), cue lookup
(`0x80016bac`) and the loop callback (`0x80016ec0..0x80016ed0`) establish the
row format: cue ID, repeat flag, XA channel, BCD start and end times. Times use
150 sectors per second; XA file number is 1. There are 19 entries, mixing songs,
short cues and character voices. Eight full-length themes use IDs 1–3 and 9–13;
their decoded lengths range from 55.79 to 93.65 seconds. The selector names their
menu, battle and editor roles, keeping short cues and voices out of the repeating
sandbox playlist. The existing XA decoder reads their timing/channel from the
user's executable, not a bundled audio catalogue.

**The repeat flag does not identify all background music.** MENU.EXE's sound-test
lookup at `0x8010ba8c..0x8010baa4` exposes nine cues, including IDs 9 and 11–13
whose repeat flags are zero. TRIAL.EXE initially selects the editor's scenery
theme at `0x80050ea0..0x80050ebc`. Its update at `0x80052170..0x800521cc` checks
for completed XA playback and starts the same song again: the theme index at
`0x8008972e` indexes a halfword table at `0x8008024c`, selecting IDs 11, 12 or 13.
The initial three-song importer incorrectly discarded these higher-level loops.

The optional sandbox selector repeats one chosen track; it neither cycles the
whole catalogue nor claims retail story/boss cue scheduling. Event narration
uses separate archives (GAME.EXE `0x80061870..0x80061888`; event.bin loaded at
`0x800ce598`, file offsets `0xb24..0xb34`), and movies have their own audio. Eight
sandbox themes therefore do not represent the entire game's soundtrack. The
disc-backed playback test decodes every choice and checks distinct non-silent
audio, pause/save/retry and return to Gaisen. Decoding and playback use the user's
disc at runtime. No XA data, decoded audio or extracted tables are distributed.

### Tenchu 2 retail stage ambience (2026-11-07)

A further audit and actual SLPS-03040 play in BizHawk/Nymashock found the missing
background layer. GAME.EXE schedules per-map VAB samples independently of XA:
Europe `0x800648f8..0x80064b50`, Japan `0x80066008..0x80066240`. The interval
tables are at `0x80083d24` and `0x80085c78`. Positive intervals repeat tone 10,
negative intervals try it with probability 1/10 once per second, and zero skips
the layer. Other updates have a 1/(10 × nominal tick rate) chance of tones 11/12,
with a random 0–3 semitone reduction. Japan's ship maps 21/23 instead have a
one-third chance of tone 15 and otherwise unshifted tones 11/12. PAL converts
the table by integer `value * 5 / 6`, including truncating negative entries to
zero; Japan also changes map 14's interval. The importer retains these edition
differences rather than silently giving them the same soundscape.

The bank loader (Europe `0x800167ec`, Japan `0x80016834`) reads
`o:\cdimage\game\sound/{hero}_E_{map:02}.vab` in **both** editions. Ambient sounds
are program 2, tones 10–12/15. Their samples and tone metadata match across the
three heroes' banks in every map of each supported edition. Reserved SPU voices
0/1 alternate the main sample; 2/3 alternate variations. The port reads that
program from the user's R bank and keeps four independent ambient voices, so
Gaisen effects neither overwrite it nor inherit Tenchu 2 sound IDs. The existing
ADPCM/envelope/pitch code is shared. Scheduling uses a presentation-only RNG;
this is source-derived background audio, not full T2 gameplay/RNG parity.

Verification used recorded controller input from boot, with no RAM writes or
injected state: one-minute stationary captures in Japanese training (map 0),
bandit mission (map 2) and Tengu mission (map 9), plus ordinary movement. XA is
idle in those quiet captures. Sample correlation identifies each map's VAB
background cues and the pitch-shifted Tengu variations. Moving into alert starts
the short XA cue 6. Private input/RAM/audio evidence is kept outside Git. These
captures do not prove every mission's event/boss music schedule. The source
mission loop still selects boss cues 9/10 and alert stingers separately; those
and event/cutscene audio remain distinct from a sandbox music override.

### Sword sheathing correction (2026-11-07)

Gaisen's NINJAA (`0x20`) uses weapon 10 (claws), while NINJAB (`0x21`)
uses weapon 28 (ninja sword), also identified by the original `game_types.h`
in `tenchu-decomp`. Rikimaru's optional imported transitions now use NINJAB's
draw clip 258 and sheath clip 265 from `HUMAN/MOTION/STAGE2.AMD`, with that
actor's sound bank. Ayame continues to use KUNOITI (`0x22`). The optional
stay-sheathed injury recovery is a port feature; its default retains the source
automatic draw. Disc-backed tests exercise incoming damage, recovery and
subsequent movement for both heroes and both weapon states.

### Japanese Tenchu 2 import (2026-11-07)

The user's SLPS-03040 disc has the same container/geometry/layout structures as
SLES-02452, with different identifiers and relocated tables. The importer checks
the edition and executable/module hashes before using addresses:

| Resource | Europe | Japan |
|---|---|---|
| VOLUME.DAT identifier | `a0704707` | `a0a28433` |
| GAME.EXE SHA-256 | `55151e486984c1794ce4f48ab7e91b2d3a5b1abeb7a30ab8d1b326b2606d5422` | `3bbc636c1d18dba2827b61dab84ae247218d7073eab7482ce382d15402dd829b` |
| datamng.bin SHA-256 | `089bdad062d085d8692399c9379bfb62633d81ac17c75c49543aae3ec5ff5773` | `e4c5075b7757c1649077c159bcae06f8c22935a23720c967a273511a021e281b` |
| Data module base / character table offset | `800ce598` / `6958` | `800d0610` / `6ed4` |
| Gameplay sequence descriptors | `80082bec` | `80084b2c` |
| Music path / cue table | `8001323c` / `8007f354` | `80013260` / `80081244` |

JP has 23 nonempty ordinary map resources and 56 gameplay layouts, all read
through the shared sandbox adapter. Its music table points to MUSIC_J.XA. Its
78 character descriptors plus five named alternate/masked resource variants
supply 83 selectable bodies. `tatua_m` and `tatu2_m`, named alongside the other
costumes in GAME.EXE, were missing from the initial Europe catalogue too.
Editor-built stages use separate TRIAL resources and are not ordinary MSD/LAY
maps; these and the campaign scripts remain outside the scenery sandbox.

The Japanese `atktbl.dat` differs from Europe in several timings, and clip
`0xbd` uses two contact bits (`0x820`, both hands) rather than Europe's `0x20`.
The optional Tatsumaru importer reads both contacts and the edition's original
hit windows. Existing one-contact `.t2moves` files remain readable. These are
Gaisen combat adaptations, not a Tenchu 2 AI/combat port. All retail data remains
on the user's disc or in their local extraction cache.

Motion selection at Europe GAME.EXE `0x8001a8b4..0x8001a8c0` negates the
16-bit blend duration at MOT header offset 6 before storing the animation
clock. Gaisen's Clip already stores a signed starting clock. The importer now
converts that sign; copying positive values had skipped the frame-one attack
setup and early hit frames for non-combo attacks. The adapter also normalizes
positive blend durations from earlier personal `.t2moves` exports.

Tatsumaru's attack recovery uses the resting hip yaw from his source idle clip
`0xa2`, rather than the borrowed Gaisen stance. Both supported discs store 450
there; the Gaisen Ninja B/Kunoichi stance stores -266. Comparing attack endpoints
with that unrelated stance added 716 angle units of rotation. The adapter keeps
the source endpoint-minus-rest delta (including intentional turning strikes),
then resets the pose to the destination stance. Legacy moveset exports recover
the same resting angle from the first attack's initial hip key. Disc-backed
checks cover all twelve attacks, both player skeletons and both disc editions.

### Native Tenchu 2 actor and motion catalogues (2026-11-08)

`tenchu-assets::tenchu2::{registry,motions}` reads each verified edition's
DATAMNG and all three heroes' gameplay MOT bundles from the user's disc.
Actor identity remains per kind: 130 definitions share 78 model names, with
separate weapon and motion registrations. These readers do not imply that
Tenchu 2 actor controllers or campaign progression have been ported.

Source routines inspected from the exact EU/JP GAME.EXE binaries listed above:
EU `8001a484` / JP `8001a534` relocate MOT tracks and install common motions;
EU `8001a588`, `8001a774` / JP `8001a638`, `8001a824` resolve character-local
registrations before common registrations and negate the initial blend duration.
EU `8001b4fc` / JP `8001b5ac` construct actors from the 24-byte definitions.
The reader preserves signed 16-bit motion speeds and every source joint, without
forcing them into Gaisen's signed-byte speeds or fifteen-part skeleton. Only
model-picker previews use the existing Gaisen bone mapping.

Independent private MIPS execution checked 3,063 EU and 3,025 JP motion selections,
including local overrides, common fallbacks and missing actions. The comparison
hook is `TENCHU2_MOTION_ORACLE`; traces and source data stay outside Git. All 72
campaign motion bundles per edition were decoded and duplicate clip IDs checked
for identical tracks. EU supplies 1,208 distinct clips and JP 1,191; JP's campaign
bundles omit nineteen horse/hawk/crow clips still named in its actor registry.
Those missing resources must not be silently relabelled as Gaisen motions or
replaced by another edition when native T2 behavior is requested.

The native rig reader retains all MDL joints; the older fifteen-joint projection
remains available for Gaisen model swaps. `tenchu-core::tenchu2::motion` follows
EU `8001a774`–`8001b4fc` / JP `8001a824`–`8001b5ac` for selection, blending,
partial-body overlays, held poses and logical advancement without posing. It
shares the verified GTE Hermite evaluation with Gaisen, reading each edition's
basis from its executable. An overlay preserves the original timeline and clip
header but replaces selected tracks; the source also replaces the last joint
regardless of the overlay mask.

Private MIPS execution compared 13,385 EU and 13,213 JP pose states across every
campaign clip, including masks, blending, restarts, loops and skipped poses.
The oracle supplies COP2 MVMVA arithmetic and stubs matrix publication only;
selection, tangent calculation, track sampling and state advancement execute
the original instructions. `TENCHU2_POSE_ORACLE` reads those private traces.

Weapon attachments come from EU `8007ff2c` / JP `80081e5c` and their hit timing
from `game/weapon/atktbl.dat`: 152 parts in 58 weapon groups, 231 attack records.
EU `8001b4fc`, `8001f358`, `8001f604` / corresponding JP routines establish
model slots, joint anchors, hit spheres and attack masks. Native skeleton checks
cover every registered actor's weapon joints in both editions. These facilities
are the native animation/combat foundation; game actor controllers remain a
separate unfinished port.

EU gameplay additionally calls DATAMNG `800cf848` from `80063388` after actors
are loaded. It scales clip durations and key times by 5/6, compacts coincident
keys, and retimes attack windows (including the special one-tick start offset).
The raw readers remain usable at 30 Hz for imports; `at_gameplay_rate` applies
this conversion when running native EU gameplay at 25 Hz. JP retains its source
30 Hz data. Blends and stored speeds are unchanged by this routine; EU's actor
velocity helper applies its separate 6/5 speed adjustment. Private execution of
the actual conversion routine matches all 1,208 EU clips and 231 attack windows
(`TENCHU2_PAL_ORACLE`).

Native command input follows EU `80016348`, `80016410`, `800164ec` / JP
`80016390`, `80016458`, `80016534`, with command tables read independently from
each executable. Its seven-entry history expires on the eighth repeated/released
update; AI command injection goes through that same history. Private MIPS traces
check 77,545 transitions per edition, including command consumption, injection,
release and signed timer wrap (`TENCHU2_INPUT_ORACLE`).

Actor definitions also retain sight penalties and combat skill. DATAMNG's
per-map/per-hero cast lists (EU `800d5cb8`, JP `800d82ac`) determine which layout
slots the source spawns; layouts preserve those slots and their packed AI
assignments. Typed AI assignments follow EU `8002eeb0` / JP `8002fb20`, including
the map/difficulty-dependent sleeping-sentry RNG call. Independent execution of
both original routines matches 31,104 assignments per edition, preserving RNG
state and the untouched non-idle programs when assigning a controller directly
(`TENCHU2_AI_ORACLE`). All campaign layouts' selected actors resolve to native
definitions and supported assignments. This is configuration, not the completed
Tenchu 2 AI update loop.

Native actor motion transitions follow EU `8001ee98`, `8001effc`, `8001f0f8`,
`8001f1c8`, `8001f218` / JP `8001f304`, `8001f468`, `8001f564`, `8001f5d8`,
`8001f628`. These retain unsupported-action fallback, attack-contact cleanup,
early motion overlays, root-motion recovery and the independent vertical
velocity. Source execution matches 12,288 EU and 12,132 JP transition/overlay
cases (`TENCHU2_ACTOR_ORACLE`); animation tangent initialization and effect
destruction are stubbed in this controller-level oracle and covered separately
by the pose checks and contact-cleanup assertions.

Actor sizes use native joint positions and source type-specific rules, not the
Gaisen hero skeleton. The original bound routine and size-selection instructions
from the actor constructor match all 130 definitions in both editions
(`TENCHU2_DIMENSIONS_ORACLE`); only collision-slot allocation is stubbed.
Native math reads each edition's PsyQ tables: 75,549 original-instruction
trigonometry, angle and square-root cases per edition match the shared scalar
math implementation (`TENCHU2_MATH_ORACLE`). The actor control foundation remains
separate from the still-incomplete native AI, collision update and campaign loop.

Native sentry turning and route following follow EU `80031ed4`, `800316a4`
/ JP `80032b54`, `800322ac`. Route construction follows EU `800388a4`, including
the return-to-home waypoint. JP pauses these behaviors during action `707` and
can randomly request it while waiting or patrolling; EU omits those RNG calls.
Source execution matches 320,000 sentry updates and 100,000 patrol updates per
edition (`TENCHU2_SENTRY_ORACLE`, `TENCHU2_PATROL_ORACLE`), including obstacle
responses, terrain probes, route timers and RNG state. Only the motion-request
helper is stubbed in these checks; actor transition semantics are checked above.

Wandering follows EU `80032014` / JP `80032cfc`, steering EU `8003126c` /
JP `80031e74`, and timed input overrides the block at EU `80030600` /
JP `8003113c`. The native port preserves the original signed-counter packing
and the JP-only random idle action. Independent instruction traces match
60,000 wandering, steering and override cases per edition
(`TENCHU2_IDLE_ORACLE`), including negative turn rates and counter wrap.

Native terrain retains the MCD materials, height indices and source surface
cache. Queries follow EU `8002cd88` / JP `8002d960`; passage checks follow EU
`8002da30` / JP `8002e654`. JP bypasses cached surfaces for ray steps smaller
than 128 in both horizontal axes, unlike EU. Private original-instruction
checks cover 46,000 floor queries and 11,500 passage checks in each edition,
across all 23 maps (`TENCHU2_TERRAIN_ORACLE`), comparing returned positions,
surface identity, material and cache state. The only supplied instructions are
the SDK's COP2 arithmetic. Gaisen's sandbox conversion remains separate.

Native target perception follows EU `80030c08` / JP `80031748`. JP uses narrower
standing/crouched cones, higher eye rays, skips checks while the observer hangs
from a ledge, and gives archers separate vertical and recognition-distance
rules. Both retain staggered 32-tick checks, target-state exclusions, cover
range changes and remembered target positions. Original execution matches
34,500 cases per edition across all maps (`TENCHU2_SIGHT_ORACLE`). That oracle
executes the actual terrain rays and supplies only SDK COP2 arithmetic and a
controlled return from the nearby-object query; object-query scheduling and the
outer alert/AI loop are not covered by this component check.

Native body terrain probes follow EU `8002d2f8` / JP `8002ded0`. Surface
transition rules, corner ordering, wall angles and escape directions are read
from each executable (EU `8008164c`, JP `8008357c`). Probes preserve the previous
surface category, water exclusions, step checks and asymmetric height masks.
The terrain oracle additionally checks 17,250 four-corner probes per edition
across all maps, including retained cache state and both query options.

Native object/body/weapon overlap tracking follows EU `8002dc88`–`8002e294` /
JP `8002e8f8`–`8002ef04`. It preserves deferred removal, tail compaction,
same-owner exclusions, disabled slots' retained contacts, signed contact deltas
and the 64-tick nearby-cover query. Private instruction execution matches 6,000
collision updates and 107,806 contact-iteration/cover checks in each edition
(`TENCHU2_CONTACT_ORACLE`), including explicit contact lookup, rotated local
offsets, full capacity and arithmetic edges. That oracle supplies world matrices
and SDK transformation at their boundary; collision and cover run original code.

Terrain movement follows the first part of EU `8001cfb0` / JP `8001d1d8`:
safe-position recovery, source gravity, water depth, ceiling/step response,
wall turning and root-motion suppression. JP retains its additional long-fall
death check. Private original execution matches 17,250 updates per edition
across all maps (`TENCHU2_PHYSICS_ORACLE`), including probe/cache state, velocity,
turning, water and death outcomes. The oracle runs native terrain queries and
speed calculations, supplies posed-root positions and surface-effect lookup,
and suppresses object contacts/audio/scoring to isolate terrain movement.

The remainder of those movement routines supplies actor/prop contact response,
weapon-hit selection and prop standing. JP additionally propagates certain
knockback reactions between actors; its reuse of the prop-response flag for a
knockback angle is retained. Private original-code comparisons match 20,000
ordered contact-response sequences per edition (`TENCHU2_BODY_ORACLE`), including
1,323 JP knockbacks, RNG, damage, motion requests and velocity changes. Terrain
and contact enumeration execute original code; the oracle supplies an ordered
contact set and stubs motion selection/audio, checked independently above.

Native weapon and body registration follows EU `8001e774`, `8001f358`,
`8001f604`, `8001f798` / JP `8001eb44`, `8001f768`, `8001fa14`, `8001fba8`.
Full-joint bindings retain attachment-slot swaps, the separate held/drawn flags,
attack bone masks, deferred collider removal and trail cleanup. Source execution
matches 23,530 operations per edition across all 130 actor definitions
(`TENCHU2_WEAPON_ORACLE`). The oracle runs the original allocator/compactor and
weapon helpers with source weapon/attack tables, supplying SDK transforms and
audio/effect destruction at their boundaries. Per-actor rig/weapon-joint validity
is checked independently against actual model assets as described above.

Native status interrupts follow EU `8001f914` / JP `8001fd24`; target selection
follows EU `8001c364` / JP `8001c4c8`. Both preserve source actor filtering,
target order, signed distance arithmetic, poison damage and RNG consumption,
and animal/boss reaction overrides. Original instruction checks match 18,000
status updates and 6,000 target queries per edition (`TENCHU2_STATUS_ORACLE`,
`TENCHU2_TARGET_ORACLE`). Scoring, audio, health-bar and vibration calls are
captured at their boundaries; the SDK square-root instructions run with supplied
COP2 leading-bit arithmetic. These components do not constitute a complete
native combat controller or campaign.

Jump entry follows EU `800226f0` / JP `80022de4`; attack entry follows EU
`80022950` / JP `80023010`. These retain motion fallback, directional/airborne
selection, stealth-kill angle and victim restrictions, JP's backward-dodge
attack rule, and edition-specific jump speeds/voices. Original instruction
execution matches 15,600 jump / 31,200 attack cases in EU and 15,240 / 30,480
in JP (`TENCHU2_JUMP_ENTRY_ORACLE`, `TENCHU2_ATTACK_ENTRY_ORACLE`). The JP
comparison excludes the three animal definitions with unavailable campaign
clips. Motion selection, input consumption and velocity run original code;
root publication, candidate lookup and audio are isolated, and the stealth
path stops before victim/world mutation. The full attack timeline, damage and
campaign integration remain separate work.

The airborne controller follows EU `8002771c` / JP `800280cc`, preserving
animation-driven vertical speed, air steering, wall kicks, ceiling cancellation,
steep-slope stopping and somersault landing orientation. Actual terrain/probe
instructions match 13,800 updates per edition across all 23 maps
(`TENCHU2_JUMP_ORACLE`), including scratch/cache state. Motion changes and attack
entry are supplied at their independently checked boundaries; audio/rumble calls
are recorded. PAL acceleration, steering limits, rebound speed and voice differ
from JP and are kept separate.

Per-clip attack sequences follow the action switches in EU `80029924` / JP
`8002a450`: directional combos, attachment grips, bow/gun/ice attack cues,
landing attacks, backward turns and recoil damping. Source instruction blocks
match 30,867 EU and 30,744 JP steps over all 281/280 registered attack/clip
pairs (`TENCHU2_SEQUENCE_ORACLE`), with PAL attack windows converted as above.
The oracle captures ordered calls and attachment visibility while supplying
projectile creation, sound, dust/splash, steering and death-dispatch boundaries.
Projectile simulation, contact resolution and the enclosing attack controller
are not established by this timing check.

Native root recovery follows EU `8001f1c8`, `8001f218` / JP `8001f5d8`,
`8001f628`, querying published joint coordinates instead of recomputing their
rotations. It shares the source-compatible scene cache: translation writes do
not invalidate matrices, and recovery reads filtered controller input rather
than the raw pad. Original SDK `GsGetLw`, matrix composition and recovery code
match 7,000 mixed coordinate/cache/recovery operations per edition
(`TENCHU2_SCENE_ORACLE`). Only COP2 arithmetic is supplied in that oracle.
Jump/attack-entry comparisons also verify when root queries occur.

The enclosing ordinary-attack paths in EU `80029924` / JP `8002a450` now
retain status interruption, attack-table selection, target steering, contact
windows, weapon parries, trail lifetime and root/yaw recovery. Source execution
matches 30,000 EU and 29,400 JP updates across all 100/98 definitions with
registered attack clips, including 271/250 parries (`TENCHU2_ATTACK_ORACLE`).
This oracle bypasses the independently checked per-clip sequence switch, supplies
target/root queries, and records audio/effect calls; selection, motion changes,
collision iteration and RNG execute original instructions. Paired stealth kills,
damage, projectile simulation and world orchestration remain separate work.

Standing/ready movement follows EU `80023198`, `80023750` / JP `800238a4`,
`80023f98`. Shared movement retains the distinctions between drawn and sheathed
walking, guard/dodge input, carry-state turns, character-specific sheathing,
footsteps and command consumption. JP-only random footsteps and hidden-stage
triggers remain edition-specific. Original execution matches 40,000 updates per
edition over all 130 definitions (`TENCHU2_STANDING_ORACLE`). The oracle supplies
ledge/attack/jump/item dispatch and audio/effects, while status, commands, turning,
velocity and recovery execute original code; nested jump dispatch deliberately
changes motion state to check subsequent control flow. Native terrain entry,
loot/carry targeting and world event integration remain separate boundaries.

Crouching, rolls, corpse searches, body-carry eligibility and wall entry follow
EU `800241c8`, `8001fd40`, `80022108` / JP `80024b40`, `80020150`,
`800227d8`. Loot retains the source byte arithmetic and unlock checks; JP alone
exempts item 15 from the unlock check. Carrying requires a settled corpse with
its joints attached and no blocked/uneven/water contact. Original execution
matches 20,700 checks per edition across all 23 maps (`TENCHU2_CROUCH_ORACLE`),
including inventory transfers, terrain/cache state, command history and filtered
input. The oracle supplies nearest-body/root queries and attack/item entry while
recording sound, pickup and vibration calls. World target links, HUD pickup
notifications and item use remain integration work.

Ledge entry, hanging, sideways movement, dropping and climbing follow EU
`80021924`, `80025780` / JP `80021f5c`, `80026100`. The port retains the
source's three-point ledge probes, diagonal fallback, slope restrictions and
source-specific PAL/JP climb timing. JP's map-23 secret remains an explicit
world-event cue. Original execution matches 32,200 cases per edition over all
23 maps (`TENCHU2_LEDGE_ORACLE`), including 73 successful entry checks and
33 EU / 42 JP new grabs. Terrain, movement, input filtering and motion selection
run the original instructions; animation tangent setup, audio/rumble and root
publication are isolated at their previously checked boundaries. The ledge
material table at EU `80081600` / JP `80083530` is identical to each edition's
body-probe transition table, so both use the same parsed rules.

Native inventory consumption and item-use entry follow EU `80054a88`,
`80022f40` / JP `80051aec`, `8002364c`. Inventory IDs are bounded disc-resource
indices, separate from action states. Grapple use preserves its count; other
uses consume stock before motion selection, including source refunds and the
moving shuriken/caltrop overlay. JP item 15 uses a different action from EU.
Original execution matches 26,000 EU / 25,400 JP calls over 130/127 actor
definitions (`TENCHU2_ITEM_ENTRY_ORACLE`). The unavailable JP animal clips are
excluded as above. Consumption, selection, overlays and velocity execute source
instructions; tangent initialization and audio remain isolated. Projectile and
item-effect simulation are separate unfinished work.

Wall alignment, sideways movement, peeking and item throws follow EU `80024ac0`
/ JP `80025440`. Shared animation-key adjustments are retained, as are camera
dependent exits, item consumption/refunds and the horizontal-velocity helper's
preservation of vertical speed. Original execution matches 18,400 updates per
edition over all 23 maps (`TENCHU2_WALL_ORACLE`), including 2,488 EU / 1,912 JP
motion-key changes and 300/383 throws. Terrain, inventory, commands and motion
selection execute original instructions; animation tangent setup, audio, root
publication and projectile creation are controlled boundaries. World integration
and projectile simulation remain separate work.

Weapon draw/sheath, falling, landing and taunt controllers follow EU `80027d54`
/ JP `80028704`; terrain pitch alignment follows EU `8002c7e0` / JP
`8002d354`. Draw/sheath frames are read from each disc's character module and
PAL-scaled as in `800cf848`. Original execution matches 23,000 updates per
edition across all 23 maps (`TENCHU2_SPECIAL_ORACLE`), including 191 EU / 201 JP
weapon transitions, 823/862 floor alignments and 60 JP-only damage changes.
The oracle executes PAL timing conversion, terrain, selection and RNG while
supplying published joint positions and recording weapon, attack, projectile
and audio callbacks. Those callbacks still require native-world integration.

Swimming follows EU `80025c64` / JP `800265e4`: wading, surface/underwater
steering, stroke history, diving, shore exits, breathing tubes, air refills and
waterborne grapple input. Native terrain and original controller execution match
23,000 updates per edition (`TENCHU2_SWIMMING_ORACLE`), including 204 EU / 202 JP
grapple launches, 1,072/1,065 shore-climb requests and 87/84 air refills. The
oracle supplies audio, projectile state, weapon changes and published root/bounds
queries, and emulates the SDK square-root leading-bit operation. Splash effects
receive the shared RNG so their later implementation can retain source call
order. The swim-corner tables at EU `8008162c` / JP `8008355c` are identical to
the existing probe corners. Initial breath follows EU `8001b4fc`; air refills
follow EU `80058834` / JP `80055a7c`. Full native-world integration remains open.

Native item aiming and release controllers follow EU `800286b4` / JP
`800290d4`. These preserve throwable release frames, aiming/cancel refunds,
grapple flight/attachment/release transitions, health item effects, held models,
NPC target normalization and JP's map-18 reward. Original instructions match
52,000 EU / 50,800 JP updates (`TENCHU2_ITEM_CONTROL_ORACLE`), including
2,318/6,115 uses and 83/4,036 healing effects. SDK VectorNormal executes with
its GTE signed-short inputs; the existing short normalizer shares that arithmetic.
The oracle records projectile creation, grapple state, published root and audio
at explicit boundaries; actual projectiles/effects and native world integration
remain unfinished. Swimming regressions still match both editions.

Hit reactions and knockdown recovery follow EU `8002aef8` / JP `8002ba28`.
Source execution matches 52,000 EU / 50,800 JP updates over 130/127 actor
registrations (`TENCHU2_HURT_ORACLE`), including 1,234/1,001 blood cues and
555/542 transitions into death handling. Knockback, water reactions, affliction
recovery, signed timers, HP-dependent get-up delays and JP's long-fall behavior
are retained. Selection, velocity and RNG execute source instructions; audio,
particles, death dispatch and previously verified floor alignment are controlled
boundaries. Weapon mode 3 removes weapon contacts without toggling or swapping
attachments; expanded weapon tests match 23,920 operations per edition.

Death, body carrying and stealth-event playback follow EU `8002b820` / JP
`8002c394`. The character module supplies all three heroes' seven event sequences;
PAL frame conversion follows `800cf848`, including shared Tatsumaru tables.
Source execution matches 27,600 updates per edition over all 23 maps
(`TENCHU2_DEATH_ORACLE`): 1,385/1,396 carrying updates, 2,702/2,683 stealth events
and 4,022/4,023 corpse completions. Original terrain, motion selection, velocity
and RNG execute; joint positions, collision mutations, floor alignment, particles
and audio are recorded boundaries. Dropping, water movement, corpse dimensions,
weapon events and interrupted kills retain source ordering. Paired-stealth setup,
actual effects and native-world integration remain unfinished.

Hit resolution follows EU `8001feac` / JP `800202bc`, including the original
terrain ray, parry selection, armor/skill adjustments, contact disabling,
projectile afflictions, resurrection and death transitions. Each edition matches
41,400 original-code updates over all 23 maps (`TENCHU2_DAMAGE_ORACLE`): 232/292
parries, 5,710/5,636 lethal hits and 2,305/2,311 resurrection dispatches. JP keeps
its distinct guarding odds, knockback, projectile immunity, burning reset and
map-19 inventory reward. Selection, terrain, direction, velocity and RNG execute
original instructions. Published roots, contact displacement, motion advancement,
weapon/effect/audio operations and score dispatch are controlled boundaries.
The passive resurrection count at actor offset `0x164` also becomes the searched
marker on corpses; the earlier boolean representation is now a byte. Both
20,700-case corpse-search regressions still pass. Actual effect/projectile and
world integration remain separate work.

Paired stealth setup follows the remainder of EU `80022950` / JP `80023010`;
attacker interruption/completion follows the first branch of `80029924` /
`8002a450`. Victims use the attacking hero's registered paired clip, not their own
ordinary death clip. Raw PAL victim speed, conditional first-voice omission,
penalty correction, JP story damage, cancellation position and final root-yaw
recovery are preserved. Original execution matches 5,180 EU / 5,176 JP complete
starts and 12,600 attacker updates per edition (`TENCHU2_STEALTH_ORACLE`,
`TENCHU2_STEALTH_UPDATE_ORACLE`), including 10,282/10,287 interruptions and
7,187/7,177 completions. Selection, normal recovery and velocity execute source
instructions; published roots, weapons, contacts, audio, score and screen tint
remain controlled boundaries. These controllers still need native-world wiring.

Sleeping guards follow EU `8003241c` / JP `80033168`; corpse perception follows
`80032648` / `80033394`. Original execution matches 26,000 EU / 25,400 JP sleep
updates (`TENCHU2_SLEEP_ORACLE`), including 2,617/2,555 sleeping and 3,770/3,664
waking results. Selection, velocity and sentry RNG execute original instructions.
Corpse scans match 32,200 cases per edition across all 23 maps
(`TENCHU2_CORPSE_ORACLE`), including 289/268 discoveries and terrain-cache state.
Discovery returns the witness; alert/audio, observer-state reset and corpse marking
belong to the unfinished world dispatcher. Shared short-vector ray stepping also
passes both editions' existing sight and damage comparisons. These checks do not
yet demonstrate a playable native T2 mission.

Combat-combo continuation, combat-item selection and ranged decisions follow EU
`80034998`, `80034acc`, `80037040` and JP `80035740`, `80035874`, `80037e70`.
Difficulty probabilities and spacing tables are read from the selected disc.
Each edition matches 36,000 original-code decisions (`TENCHU2_COMBAT_ORACLE`),
including 968/1,519 item selections and 3,986/3,985 attack inputs. Source RNG,
steering, command selection and attack tables execute directly. JP retains its
extra item-15 healing check and reduced firing odds against swimming targets.
Melee programs, enclosing perception/dispatch and native-world integration remain
unfinished; this does not enable a native T2 gameplay mode yet.

Blade-fighter decisions follow EU `80034dbc` / JP `80035c28`, with target-offset
approach from `80031460` / `80032068`. Each edition matches 40,000 source decisions
(`TENCHU2_BLADE_ORACLE`), including 617/674 item selections, 4,770/4,621 attack
inputs and 40/41 taunts. RNG, steering, command-pad history and offset arithmetic
execute original instructions; taunt selection is recorded at the previously
verified motion-controller boundary. JP's different counterattack condition is
preserved. Ranged/combo/item regression checks still pass for both editions.
Native world dispatch and the remaining AI programs are not connected yet.

Heavy-weapon and unarmed decisions follow EU `800366f4` / `80035af4` and JP
`80037538` / `8003694c`; dogs use `80037484` / `800382e8`. Each program matches
40,000 original-code decisions per edition (`TENCHU2_HEAVY_ORACLE`,
`TENCHU2_UNARMED_ORACLE`, `TENCHU2_DOG_ORACLE`). Heavy fighters produce 3,423/3,158
attack inputs and 643/685 item selections; unarmed fighters produce 5,534/5,415
attacks and 444/459 items; dogs produce 2,451 attacks in each edition. Short-range
spacing, counterattacks, boss-specific moves, command-pad changes and wrapped
counters retain source order. Shared melee helpers also pass the existing blade
comparisons. Motion-change requests remain recorded boundaries; enclosing AI and
native-world integration still need implementation.

Combat dispatch, cautious approach, home defense and flanking follow EU
`80033a98`, `80033bcc`, `80034354`, `80034740` and JP `80034840`, `80034974`,
`800350fc`, `800354e8`. Each edition matches 120,000 source decisions
(`TENCHU2_TACTICS_ORACLE`), including 1,222/1,367 item selections and 6,825/6,620
attack inputs. Weapon-specific strategies run through typed fighter state;
attack cooldowns, repeated-button suppression, home-return counters, approach
positions and the global rotation phase are preserved. RNG, SDK square root,
steering, command input and nested combat programs execute original instructions.
Motion changes are synchronous callbacks, recorded separately in these comparisons
so the already verified selection controller can be applied at the correct point.
Outer perception, search transitions and native-world wiring remain unfinished.

Watching, investigating, fleeing, confused attacks and corpse investigation follow
EU `80033230`, `800333f4`, `800337e8`, `80033948`, `800334f0` and JP `80033fd8`,
`8003419c`, `80034590`, `800346f0`, `80034298`. Each edition matches 100,000 source
decisions (`TENCHU2_ALERT_ORACLE`), including 2,896 corpse reports and 4,191 item
inputs. Source RNG, steering, sentry and combat-combo routines execute directly;
motion selection, alert propagation and voice playback are recorded synchronous
boundaries. The shared watch/throw phase, forced turns and corpse-home updates
retain source ordering. Lost-target searches, enclosing perception and native-world
integration remain unfinished.

Lost-target traversal follows EU `8002d5fc` / JP `8002e1d4`, distinct from ordinary
sight-ray stepping. It retains cell-boundary skips, material transitions, cursor
invalidation and JP's small-step cache bypass. Source execution matches 11,500
traversals per edition across all 23 maps, alongside 46,000 floor queries, 11,500
ordinary rays and 17,250 movement probes (`TENCHU2_TERRAIN_ORACLE`). Normalization
and square-root SDK instructions execute with the original GTE arithmetic.

Lost-target waiting and pursuit follow EU `80037598` / `800379e0` and JP
`800383fc` / `80038844`. Each edition matches 41,400 original-code decisions
across all 23 maps (`TENCHU2_SEARCH_ORACLE`), including 367/369 detours and
2,291/2,304 completed searches. Following, looking and waiting use typed states;
original RNG, terrain, SDK math and steering execute directly in the comparisons.
Motion requests are recorded boundaries. JP's shorter pursuit timeout, immediate
facing changes, forced movement, cached perception flags and terrain-cursor state
are preserved. These components still require enclosing AI/world integration.

Guard navigation follows the terrain samples at EU `8002f994..8002fb04` / JP
`8003a2ac` and movement constraints at EU `800305e8..800309d4` / JP
`80031124..80031518`. Original instruction execution matches 13,800 terrain
samples and 23,000 movement decisions per edition over all 23 maps
(`TENCHU2_NAVIGATION_ORACLE`), including 508/500 jump inputs. Alternating forward/
side probes, forced-turn expiration, ledge movement, step jumping and water/drop
avoidance retain source arithmetic and RNG order. Terrain, trigonometry and RNG
execute original instructions; entry registers supply the preceding AI decision.

Shared alerts, radar conversion and guards' post-victory reactions follow EU
`80030a34`, `80030a7c`, `800315fc`, `80031cac` and JP `80031574`, `800315bc`,
`80032204`, `8003292c`. Alert durations load from each executable. The timer and
script/alert visibility branches of the enclosing AI are also preserved,
including retaining the shared position when a positive timer reaches zero.
Each edition matches 60,000 source executions (`TENCHU2_STATUS_ORACLE`), covering
timer boundaries, all difficulty levels, radar thresholds and victory phases.
Motion requests are recorded at the selection boundary; native-world dispatch
remains unfinished.

Enclosing guard-perception transitions follow EU `8002f8cc..80030600` and JP
`80030524..8003113c`. Each edition matches 40,000 original-code transitions
(`TENCHU2_PERCEPTION_ORACLE`), including 296 player-spotted events. Calm,
suspicious, fighting and searching transitions preserve routine dispatch, reset
and memory semantics, radar/alert updates, motion/voice requests and input
suppression. EU/JP recognition differs in voice RNG and missing-animation
handling. Program results and motion/voice calls are controlled boundaries;
source alert timers, steering, command input and RNG execute directly. This
verifies transition policy, not yet a complete native AI tick or mission.

Outer AI entry follows EU `8002f074..8002f8cc` and JP `8002fce4..80030524`,
including the early-return paths through pad updates. Each edition matches
50,000 original-code cases (`TENCHU2_ENTRY_ORACLE`): 10,909 EU / 9,678 JP reach
normal perception. Ally target changes, hurt/corpse handling, recovery from
confusion, medicine selection, direct input, radar reset and alert expiry retain
source ordering. Nested behavior, target lookup, sight and motion calls are
recorded boundaries. JP's additional 1000-unit direct-input terrain samples also
match original code over all 23 maps, alongside the 1500-unit guard samples and
movement-constraint comparisons. Native behavior-state/world wiring remains.

Persistent behavior dispatch now connects the perception transitions to the
actual idle, patrol, combat, search and corpse programs. Each edition matches
48,000 decisions in 80 continuous 600-step sequences
(`TENCHU2_BEHAVIOR_ORACLE`), including 50/53 item inputs and 144/136 attacks.
The patrol wrapper follows EU `80032b04` / JP `80033850`; awareness resets
preserve waypoint index/wait, while clearing the shared turn counter. Sleep
motion changes execute synchronously. Original nested routines, terrain and RNG
run in the comparison; sight inputs, motion selection and voices remain
controlled boundaries. These sequences cover ordinary perception transitions,
not arbitrary scripted reassignment between stateful programs or complete AI
ticks. Native world, effects and campaign integration are still unfinished.

Complete AI dispatch now joins entry, sight, persistent behavior and final
navigation, with synchronous corpse-witness updates and shared regional state.
EU `8002f074` and JP `8002fce4` each match 48,000 full actor-AI calls in 40
four-actor sequences across five maps (`TENCHU2_RUNTIME_ORACLE`). The comparison
executes original target selection, sight rays, corpse scans, behavior, terrain,
pad history and RNG; it checks every actor and the shared alert/radar after each
round. Controller sampling, motion changes and voices remain explicit boundaries;
movement/postures are controlled inputs. Native rendering, actor controllers,
effects and campaign still need to be joined into a playable runtime.

Native AI now follows the Japanese executable only, following the revised project
scope. Regional AI branches have been removed, and native combat-rule loading
requires the JP disc rather than mixing EU tables with JP behavior. The source
addresses and dual-edition measurements above document the earlier research;
they are not a commitment to support European gameplay. European maps, models,
motions and other asset imports remain supported separately. After the change,
JP comparisons still pass 48,000 complete actor-AI calls, 48,000 persistent
behavior steps, sight checks and terrain queries across all 23 maps. Formatting,
strict workspace Clippy and the portable wasm core build pass.

Native actor control and physics now likewise use only JP behavior: PAL speed/
frame conversions and European damage/item branches have been removed from the
core. Actor construction requires Japanese definitions, while shared EU motion,
model and map decoders remain intact for resource imports. The JP movement,
controller, damage, paired-stealth and attack-timeline comparisons all pass after
this simplification (19 focused tests), along with strict Clippy, formatting and
wasm core compilation. This remains internal integration work, not a playable
native T2 campaign.

The JP outer controller follows `8001ee58`, with falling entry at `80021de8`
and water entry at `80022998`. It preserves damage/fall/water precedence, early
water transitions, breathing/drowning, camera input suppression, item selection
and final motion requests. It matches 24,000 source calls
(`TENCHU2_DISPATCH_ORACLE`), including 16,277 action transitions and 135 drowning
ticks. The comparison records state-controller, motion-selection and effect calls
as synchronous boundaries; their integration into a complete actor tick remains.
Fmt, strict workspace Clippy and wasm core checks pass. EU resource compatibility
also passes: every model's rig/textures/idle decodes, and both Gaisen heroes can
move through all 23 imported maps, including snapshot/restore comparisons.

Native JP world creation follows `8001b5ac` (actors/weapon attachments),
`8002f6f4` (hero/cast filtering), `80039358` (layout actor initialization), and
`8001bcd4` (NPC inventories). Loot parameters and item slots come from the user's
JP executable/module; no table bytes or models are distributed. Inventory/RNG
results match 13,000 original-code calls over all 130 actor kinds
(`TENCHU2_LOOT_ORACLE`). All 56 gameplay layouts create their source cast with
native skeletons, body/weapon collision and serializable world state. These are
initialization checks; continuous controller/effect/campaign integration remains.

The world now joins terrain/body movement and AI to the shared actor, scene and
collision state. Its post-controller update follows JP `8001cbb8`: travel/action
effort, offscreen motion policy, visible flags, foot probes and head tracking.
The post-controller behavior matches 10,000 source cases
(`TENCHU2_ACTOR_TAIL_ORACLE`); movement, controls, pose evaluation and spatial
queries are explicit fixture boundaries in this particular comparison. The
existing 24,000 outer-control comparisons still pass after allowing state
controllers to edit shared wall-hug motion keys. This does not yet establish
complete native mission playback.

Native mission startup also reads stage view distances from the executable's
24-entry stage table (JP `80083950`, EU resource table `80081a20`) and loads
`game/weapon/weapon.arc`. JP `80064a34` selects late-story Rikimaru's weapon and
Tatsumaru's map-dependent models; native assets now follow those selections.
All 56 gameplay layouts initialize with these sequence-specific resources.

The native world controller now connects JP locomotion, weapon drawing, attack
contacts and paired stealth entry to its live rigs and other actors. World calls
reuse the verified state routines, with synchronous geometry/contact changes and
explicit event callbacks for effects still to be integrated. Item cycling follows
`800518cc` / `800517c8`, including the passive resurrection slot. Model bounds
follow `8002f11c` and are retained at initialization rather than recomputed every
tick. Continuous movement/draw/attack input sequences pass for all three heroes,
including exact snapshot continuation of world state and emitted events; effect
callbacks are recorded boundaries in that check. Actor, wall, attack, dispatch,
world-setup and wasm checks also pass. Swimming, item, injury/death drivers and
concrete effects/campaign integration remain unfinished.

The remaining JP state controllers now connect to world state: swimming, items,
damage, hurt and death. Contact owners receive hit pause/health updates;
paired deaths restore body collision and advance their source effect sequence;
corpse carrying uses the player's actual rig. Kill counters and script-reset
rules follow `8002d4f8`; alert reset/raise follows `80031574` / `800315bc`.
Terrain-query policy survives movement into the controller pass. Concrete effect
and projectile spawning remains an explicit synchronous callback boundary.
World integration checks exercise a complete paired kill/corpse transition,
weapon damage, medicine consumption/healing, and drowning. Source checks still
pass 27,600 death updates, 41,400 damage updates and 24,000 outer controls, plus
world setup/continuation, strict Clippy, formatting and wasm core compilation.
These checks do not establish full campaign or ordinary-play parity.

Native JP actor activation follows `GAME.EXE` `8001e318`, including its two-ended
live/dead partition, initial active-count accounting, camera annulus, offscreen
budget retirement, home/patrol resets and collision/weapon restoration. Initial
sleeping follows `80064a34`; horizontal distance follows `8002f428`. A caller
supplies the 4–8 actor budget as a simulation input; native render time never
substitutes for the original hardware-cost measurement. A private original-code
oracle checks 1,000 twelve-actor cases, including vertical and distance boundaries,
visibility, dead actors, exemptions and non-update ticks. This still does not
expose native T2 campaign play.

The native JP actor loop now joins `8001c8f0` / `8001cbb8`: active actors run
movement/contact resolution, fall-death credit and cues, AI, state controls, then
animation/visibility/foot/head updates in source order. Water avoidance is cleared
between movement and AI. Effects and visibility are explicit caller boundaries;
AI voices are delivered before controls. A disc-backed integration check runs all
56 campaign casts for 180 updates each with movement, attacks and jumps. This
checks the joined actor logic, not complete items, camera, effects or campaigns.

JP scene resource selection follows `8004ce78`: sequence/hidden-scene variants
choose the matching OSD and object archive, while retaining the base terrain and
texture archive. The OSD reader retains model bounds, static/procedural placements,
interactive objects and trigger records. Disc checks cover 41 distinct JP scene
variants (2,692 interactive objects, 413 triggers, 623 static effects), with all
56 campaign casts still passing continuous actor updates. The imported Gaisen
collision adapter passes 57,500 original-code floor queries across all 23 EU maps,
including 20,949 blocked queries. Its obsolete nearest-surface-only test was
removed because that expectation ignored ordered blockers and masked holes.
Native object behavior and campaign rendering remain to be connected.

Native JP object state follows `8004ce78` / `8004dea0`; contacts and trigger
updates follow `8004e124`, `8005e1b8`, and `8005d78c`. This includes sliding/paired
doors, breakable scenery, hazard contacts, collapsing platforms, rolling rocks
and projectile trap cooldowns. Surface activation now runs during the actor's
movement pass. Mutable terrain lives in serialized world state rather than shared
assets; scenery initialization consumes the source static-effect phase RNG after
actor inventories. Shared object alerts update the native AI state.

Private JP executable comparisons pass 10,000 surface/trigger cases and 6,000
contact cases (`TENCHU2_OBJECTS_ORACLE`, `TENCHU2_OBJECT_CONTACTS_ORACLE`). They
record projectile, explosion, pickup, debris, audio and vibration creation as
explicit boundaries; they do not prove those effects are implemented. The world
check exercises opening a sliding door, destroying a collapsing floor and exact
snapshot continuation midway through the collapse. All 56 casts still complete
continuous actor updates; movement, body response, actor-tail and player-control
regressions pass, with strict Clippy/fmt and wasm core compilation. View-driven
object activation/rendering, the special falling prop, concrete effects/items,
camera and campaign integration remain unfinished.

Native JP camera state and terrain queries follow `8005e858`–`80061a1c`:
follow/aim/swim/wall/stealth/death modes, source pose tables, ceiling and wall
avoidance, smoothing history, snapshot state and underwater audio selection.
Tables are read from the user's JP executable. The camera shares the existing
scalar smoothing and view-matrix arithmetic where the algorithms agree; its
vector-length scaling retains T2's different threshold and signed shifts. The
item controller's reset field is camera yaw (`8008ebce`), not pitch.

Private original-instruction checks pass 5,520 camera updates on all 23 maps,
including terrain state and RNG (`TENCHU2_CAMERA_ORACLE`), and 2,000 complete SDK
view matrices including roll (`TENCHU2_CAMERA_VIEW_ORACLE`). SDK setup initializes
both identity matrices; no game resources are committed. All 56 campaign casts
run with the camera and activation together, and the three-hero control check
includes exact saved-camera continuation. Concrete visibility, object rendering,
effects/items and campaign presentation remain integration work; this is not a
claim that native JP missions are playable yet.

Native JP visibility now follows `80018030`, `8004f168`, `8004fb80` and
`80050db8`: actor projection/depth, scenery height and camera-direction gates,
static effect cones, interactive-object collision activation/removal and clocks.
MSD cell identity and the model half-height from `8004ce78` are retained. Assets
are resident in native memory; empty-cell fallback is preserved, but PS1 CD load
latency is not simulated. Actor visibility is computed after publishing its root
transform and before advancing its pose, rather than supplied by a test callback.

Private original-instruction checks cover 12,000 actor projections and 4,000
scenery passes with 20 placements each (`TENCHU2_ACTOR_VISIBILITY_ORACLE`,
`TENCHU2_SCENERY_VISIBILITY_ORACLE`). The latter records draw/effect creation as
boundaries; it does not verify the unfinished procedural scenery effects. All 56
campaign casts run the native camera, actor, scenery/contact and activation loops.
The existing actor-tail, object contacts/triggers and snapshot-continuation
checks pass. Rendering the selected scenery, falling props, concrete items and
effects, and the complete native campaign remain integration work.

Native procedural scenery now follows JP `8004f27c`, `8004f57c`, `8004f750`,
`8004fa90` and `80051410`: smoke/ripples/flames, shared fire collision, ambient
cues, and their phase/RNG order. Effect textures come from the user’s
`effect.arc` / `objectef.arc`; the four-frame flame UV cycle follows `8003b134`.
Falling props follow `800503b0` / `800509bc`, retaining the shortened stump
geometry, attacking actor’s direction and authored falling transform. If a prop
first becomes visible after its initial falling tick, native code captures its
direction safely instead of interpreting an actor pointer as a cloned model.

Private original-instruction comparisons cover 4,000 procedural scenery cases
and 1,000 falling-prop transforms (`TENCHU2_SCENERY_EFFECTS_ORACLE`,
`TENCHU2_FALLING_PROP_ORACLE`). Projection and model allocation/drawing are fixture
boundaries; the checks establish phases, RNG, sprites, sounds, collider creation,
transforms and removal, not final pixel output or the stump geometry itself.
All 56 campaign casts, prior visibility checks, strict Clippy and wasm core pass.
Actor-attached burning remains an event boundary until the concrete effect
system is connected; native rendering, items and campaign progression remain.

Native JP particles follow `8003b2bc` / `8003b354` / `8003b390`, `8003d708`,
`8003e260`, `8003e480`, `8003e7c8`, `8003f7dc` and the dust, smoke, flame,
healing, flash and explosion constructors at `8003db48`–`8003f580`. Ordinary
owned sprite groups retain particle phases, authored offsets, attached bones,
visibility-dependent image advancement and source RNG order. The source allocates
64 records but its free-pointer threshold replaces an old priority >=4 effect
at 62 active records; native eviction retains that threshold and insertion order.
Healing point particles use `8003d344`; body attachment tables are read from the
user’s executable, not embedded resources.

`TENCHU2_SPRITE_ORACLE` / `TENCHU2_EFFECT_TABLES` check 600 private mixed original
executable sequences, including all these constructors, overflow replacement,
RNG, expiry, sprite output and point-particle state. Projection is a controlled
boundary in this fixture, and point drawing is excluded; the native projection
has its separate source checks. These particle components still need the world
event adapter and native renderer. Terrain-dependent blood/water, item effects
and campaign integration remain unfinished.

Native JP audio follows `80016834` (hero/area VAB selection), `8001ec68`,
`8001ecb4`, `80016a14` (cast voice IDs, positional attenuation/panning, cooldowns
and fixed voice slots) and `80066008` (ambient scheduling). The latter consumes
the gameplay RNG even without an audio device. Private original-instruction
checks compare 8,000 direct, positional and ambient cases including RNG and
voice-slot state; the SPU key-on call is recorded rather than emulated. The
comparison uses unity effects gain; user volume remains a mixer setting.
Footstep surface triggers and quiet player steps follow `800238a4`, `80023f98`
and `8005e1b8`. Samples come only from the supplied disc.

The internal native preview orders camera, actors, scenery, items and particles
according to `80064a34`, followed by the scene epoch, `8001e318` activation and
`8002e9e8` collision update. It presents a copy of the scene's coordinate cache
so rendering does not alter simulation queries. Three-hero frame/sound snapshot
continuation and rendered captures exercise this adapter. The full original
frame still includes HUD/story/procedural-stage paths that are not connected;
this is not an assertion of whole-frame or campaign equivalence.

Native JP water and blood use `8003cbc4`, `8003ce44`, `80040150`–`80042130`,
`80044828`, `80044f98` and `8004518c`: weighted point bursts, impact sparks,
splashes, delayed ripples, airborne blood, wall runoff, sloped stains, water
spread and corpse pools. Point weights/shifts are loaded from the user's
executable. Blood alternation, particle activation, original contact-sound
positions and effect replacement share the native RNG/effect state.

`TENCHU2_SURFACE_EFFECTS_ORACLE` compares 1,200 private original-executable
sequences on JP map collision data, including spawning, RNG, terrain contacts,
sound positions and expiry. The generator excludes 99 candidate sequences where
the original dereferences a null surface pointer; native code expires that drop
instead. `TENCHU2_SURFACE_DRAWING_ORACLE` checks 1,000 original sloped quad,
wall-drip, ripple and fade outputs. Projection is isolated in these checks;
surface visibility belongs to the native renderer. The prior 600 sprite cases
also pass. These checks do not establish playable native mission/campaign parity.

Native JP actor/environment cues now create the owned effects at the source
call site, preserving RNG ordering: `8002d63c` splash kinds, dust/trails,
`8002ba28` underwater bubbles, block flashes, attached fire and healing. Death
pools use joint 1's world position minus 1024 (`8004192c`); scenery explosions
and burning contacts share this same effect state. The renderer does not own or
advance these effects. Four connected-world checks cover all 56 casts, paired
kills, blocks, healing, drowning and scenery contacts; all three heroes retain
exact world/effect/event continuation after a snapshot. Strict Clippy and wasm
core pass. Items, weapon trails/clashes, audio delivery, native presentation and
the complete campaign still require integration; this is not playable parity.

Cross-build comparison with `../tenchu-decomp` is recorded in
[`docs/tenchu2-reuse.md`](docs/tenchu2-reuse.md). The shared ancestry provides
function names and algorithm references, but checked movement, angle, animation
and combo routines have observable T2 differences. Only verified-equivalent
operations should share the Gaisen implementation.

Native JP item aiming, flight and grapple follow `80052628`, `800529f4`,
`80054404`–`80054ea8`, `8005ad28`, `8005b054` and `8005bed0`. They retain the
source terrain traversal, short-vector narrowing, water contacts, hook phases,
rope subdivision and pull/release timing. Item models and texture archives are
read from the user's disc according to `80052084`; no item artwork is embedded.

Private original-instruction checks compare 1,000 aims, 1,941 flight updates and
400 complete grapple trajectories (15,445 updates). The flight generator excludes
59 cases where the source dereferences a missing terrain node. Projection and
final rope pixels are not compared. A connected-world check exercises launching,
pulling and releasing across a saved-state round trip, and all three heroes pass
the movement/combat snapshot continuation. Other projectiles, native presentation
and campaign integration remain unfinished.

Imported water scrolling follows JP `8003ab60`, `8003c320` and `8003c418`
(EU setup `80039a2c` uses the same map speeds). Terrain textures already contain
the visible water; the omitted VRAM copies are its animation scratch images.
Presentation retains the initial image instead of displaying two uninitialized
startup copies, then uses the source's wrapped texture offsets. A private
original-instruction fixture checks 1,128 copy positions across six maps, and
the same checks pass with EU assets. GPU checks exercise both original and
replacement images, shared static artwork and backward seeking. Native swimming
also enters the swimming controller on Area 0's actual deep-water collision;
this is not a complete interactive water-gameplay comparison.

Native character rendering now uses every source joint, including the 23 JP
bodies/costumes with more than 15 joints. A disc-driven check builds and poses
every catalog rig; Gaisen's blended-joint seam regression still passes.

Native JP thrown blades follow `8005356c` / `80053770` (shuriken), `80053d64`
/ `80053f6c` (kunai), and the resting/falling pickup paths `8005baa0` /
`8005bdec`. They share the source's 30-active-item replacement threshold
(`800533ac`–`800534e4`) with the grapple and reuse coordinates. The allocator's
uncleared floor value survives record reuse; fresh owned slots start at zero.
Projectile damage decodes the collider's low six bits (`800202bc`), separately
from its weapon/contact flags. Blood contact audio uses ID29 (`8004028c`).

`TENCHU2_PROJECTILE_ORACLE` compares 1,600 original constructors and updates on
actual JP collision maps, including deflections, object/body contacts, sounds,
alerts and water entry. Projection/trail drawing and contact enumeration are
controlled fixture boundaries; they are not full-frame comparisons. A connected
world check throws through actor controls, damages a guard, recovers a missed
shuriken, stresses bounded coordinate reuse and restores in-flight items from a
snapshot. Other item types, trail presentation and campaign logic remain open.

Native JP arrows follow `8005c304`, `8005c658` and `8005ca54`; fire arrows use
`80056b10` / `80057000`, and firearms use `8005d1d4` / `8005d480`. Their native
actor-controller call sites retain bow nocking, map/kind-specific fire-arrow
odds, aimed trajectories and sound order. Auto-aim follows `8001c738`, including
`VectorNormal` returning the squared length of signed-halfword inputs.

`TENCHU2_RANGED_ORACLE` compares 2,600 private original constructors and updates
on JP maps, including parent-bone embedding, water extinguishing, deflection,
explosion phases, firearm damage windows and source RNG. Projection, drawing,
contacts and effect creation are controlled boundaries. `TENCHU2_AIM_TARGET_ORACLE`
checks 1,000 target selections. Connected-world checks exercise guard bow,
fire-arrow and firearm hits, the player's fire-arrow inventory/damage path,
and in-flight snapshot continuation. All 56 mission casts and three-hero frame/
audio continuation still pass; these are not complete mission parity checks.

Those connected checks exposed two setup gaps: `80039358` enables assigned AI
with `8002fb20`'s `0x10` return flag, and the zeroed actor created by `8001b5ac`
initially references attack row zero. Missing melee entries later select the
all-zero terminator; `80035740` still reads its zero combo timing rather than
failing. Both loaded editions' terminators have been inspected. The 36,000 JP
combat decision comparison now includes that terminator, and the 29,400 attack
controller and attack-sequence regressions pass.


Native JP grenades follow `80055a8c` / `80055d90`, mines `800560b0` / `800562b0`,
and smoke bombs `80057444` / `80057870`. Throws from cover retain the sequential
camera-direction adjustments and hand origins; terrain probes, bounce damping,
wet duds, smoke expansion and explosion collision windows are original logic.
The shared pickup callbacks `8005baa0` / `8005bdec` also recover grenade duds;
explosions can ignite them again. Fuse sparks use `80041f58`, and positional
explosion sound 51 follows `8003ec74`. Particle creation runs at the item call
site so its RNG consumption precedes the grenade's detonation roll.

`TENCHU2_EXPLOSIVE_ORACLE` compares 2,937 original constructors/updates across JP
maps, with 63 source null-terrain dereferences excluded. Projection, contact
lists and non-fuse effect calls are controlled boundaries. The 600 mixed
particle sequences now include actual fuse allocation, particle geometry,
RNG, expiry and pool replacement. Connected-world checks cover player inventory
use, blast damage, smoke blindness and recovering a quenched grenade across a
snapshot. Mines/smoke store yaw in an unused source root-angle field; their draw
routine uses the separate, zeroed item angles. No game assets or captures are
included in these implementations or tests.

Native effect blending follows the sprite attributes initialized by `8003a44c`
through `80043154` / `8004324c` / `800433c4`, not the TIM page's default blend
bits. The original initializer has been executed with controlled archive/image
inputs to inspect all 26 GsSPRITE records; smoke uses additive blending and
explosion smoke subtractive blending. Scenery supplies a raw ABR value, while
point effects use `80043518` / `8004368c`'s enable bit plus ABR. The renderer
now translates each representation separately. Native effect and item texture
loading also follows `80017600`'s palette normalization: when no nonblack entry
has an explicit STP bit, every nonzero entry gets that bit. Mixed palettes and
transparent black are preserved.

Native JP caltrops use `80055084` / `800553d0`. The four scatter directions,
wall-cover hand/origin rules, RNG speeds, landing sound 53, owner clearing and
body/contact flags follow those routines. `TENCHU2_CALTROP_ORACLE` checks 1,000
original-executable scatters and terrain/contact updates. A connected-world test
uses the actual item animation/inventory, waits for landing, then steps both a
guard and the player onto separate traps. Contact lists are controlled in the
source fixture; this is not a whole-mission comparison.


Native JP poisoned rice follows `80057b00` / `80057fe4`, and target selection
follows `80057e3c`. Flight, settling, owner clearing, consumption restrictions,
recovery and map 12's blindness exception are retained. Its separate lure AI
uses `80032aa0`, `80032ad4` and `80033928`: planar sight distances, its own eye
height and terrain ray, 32-tick checks, alert interruption and layout-program
restoration. Recognition checks the actor's current target after the idle
callback, as `8002fce4` does, so spotting while leaving a lure still counts.

`TENCHU2_RICE_ORACLE` checks 1,500 original throws and terrain updates;
`TENCHU2_RICE_TARGET_ORACLE` checks 1,000 searches through real JP map terrain;
`TENCHU2_BAIT_AI_ORACLE` checks 3,000 original lure decisions. Drawing, contact
lists, steering and fallback sentry calls are isolated where appropriate; the
existing steering/sentry checks cover those routines separately. Connected
checks exercise actual item controls, bait attraction, eating, owner recovery,
alert/recognition interruption and snapshot continuation. Rice's 16-tick scan
phase retains stable source pool slots and a replayable pool-address phase.
The native preview starts that phase at zero; recovering the original heap
phase during complete mission initialization remains a full-frame parity task.

Native JP weapon/projectile ribbons use `8003c848`, `8003c888` and `8003c90c`:
the 16-sample ring, 15-sample capacity, interval-based retirement, narrowing and
per-vertex colors follow the original instructions. They share the 62-entry
effect pool with particles and retain priority zero. Weapon sampling follows
`8001c8f0` and `8002f0b4`; shuriken/kunai sampling follows `80053770` / `80053f6c`.
Weapon clashes now call the `8003eb88` flash at the original attack callback.
The native driver samples once per logic tick, independent of display refresh.
An item discarded outside the map retires its ribbon rather than leaking the
original orphaned effect entry.

`TENCHU2_RIBBON_ORACLE` compares 1,200 original-executable sample/update sequences,
including ring wraparound, fading, narrowing, colors and removal. Drawing is
captured at the primitive submission boundary, not compared as pixels. Existing
blade-flight fixtures isolate projection; connected-world checks additionally
exercise actual melee/throw controls, rendered trail output and snapshots.

Native JP poison darts follow `800566ec` / `80056898`: hand origin, normalized
700-unit ballistic velocity, distance accumulation, terrain and owner/contact
checks, source range limit and ribbon width. They use the existing item-8 damage
decoder in `800202bc`, including the unaware/armed distinction. The item model is
not drawn; the blowgun is held by the actor and the flying dart uses its ribbon.
`TENCHU2_DART_ORACLE` checks 1,524 original constructor/update cases across real
JP terrain; 76 original null-terrain reads are excluded. Projection and contact
lists are controlled boundaries. Connected checks fire through ordinary item
controls at both unaware and armed guards, assert inventory/poison/damage and
compare an in-flight snapshot's continuation. No mission-level parity is implied.

Native JP sleeping bombs follow `80059c3c` / `80059f40`, with gas sprites from
`80045694` / `80045800`. The port preserves reduced throwing gravity, bouncing,
dry-ground arming, the expanding contact volume, emission cadence, underwater
quenching and expiry. The gas uses the shared smoke image's last rotation,
matching the original callback's omission of a rotation write. Sprite rotations
are owned by the effect runtime and survive snapshots.

1,600 original-executable constructor/terrain/contact cases pass alongside the
2,937 grenade/mine/smoke cases (`TENCHU2_EXPLOSIVE_ORACLE`); 600 mixed original
particle sequences now include sleeping gas (`TENCHU2_SPRITE_ORACLE`). Each
particle sequence resets its sprite descriptors, while preserving shared-image
writes within the sequence. Projection remains isolated. A connected actual-use
test throws the item, puts a guard to sleep without health damage, checks gas and
inventory consumption, and resumes the landed bomb from a serialized snapshot.
These checks establish the component connections, not full campaign parity.

Native JP confusion powder follows `80058918` / `80058a90`: head-origin placement,
surface query, forward contact offset, owner exclusion and the original 32-update
lifetime. Its particles follow `8004547c` / `8003cd78`; the five colors are read
from `GAME.EXE` at `80012a18`. Original execution matches 1,000 placement/expiry
cases (`TENCHU2_POWDER_ORACLE`) and 600 mixed particle sequences containing powder,
sleeping gas and prior effects. The connected actual-use check verifies guard
confusion without health damage, particles, item consumption and effect expiry.

Native JP invisibility follows `800583f4` / `800584b0`: the shared `8008e974`
concealment state, 60-update activation, 70-update trail cutoff, 900-update
previous-age expiry and cancellation (including the source's continued update
after removal). Transformation corners use joints 11/5/14/8 and additive effect
image 0. Body faces enable quarter-add blending; held weapons remain separate.
Character TIM loading now applies `80017600` as called by `8001bed4`. Authored
palette colors remain available for stable HD identities, and replacements
inherit loader-enabled blend classes. Verification includes 1,200 independent
original-executable activation/cancellation cases (`TENCHU2_INVISIBILITY_ORACLE`)
and actual item use, detection suppression, inventory restrictions, expiry and
snapshot continuation. This does not establish full mission/frame parity.

Native JP fire-breath scrolls follow `80059934` / `80059a34`: floor-relative
placement from joint 1, four triplets following the owner's current yaw, and
source burst heights/timing. The generated blast colliders follow `800564f4` /
`80056638`, with owner exclusion, the original grenade damage kind and a four-update
expansion window. Original-executable comparisons cover 600 launch/update cases
and 120 blast collision/expiry boundaries (`TENCHU2_FIRE_BREATH_ORACLE`,
`TENCHU2_FIRE_BLAST_ORACLE`). Connected item-use checks cover guard damage, owner
safety, inventory, particles, positional sound and expiry.

Native JP teleport follows `800594dc`, `800595f0` and `800597c4`: source visibility
flag eligibility and list-order ties, full-word squared distance, the 96-update delay,
cancellation and unsnapped placement behind the target's current pose. The shared
paper effects follow `80043fe8`, `80043dc0` and the four motion callbacks
`800443e8`, `800444c0`, `800445f8`, `80044740`, preserving their RNG calls, pool
priority, delayed appearance and motion transitions. Their disc image is
`item.arc` entry 64, configured by `80052084`; rotations use `RotMatrixYXZ`.
Original-instruction comparisons cover 1,600 target sets, 600 placement and expiry
cases, and 240 particle sequences including pool eviction
(`TENCHU2_TELEPORT_TARGET_ORACLE`, `TENCHU2_TELEPORT_UPDATE_ORACLE`,
`TENCHU2_PAPER_ORACLE`). Projection is excluded from the particle oracle. A
connected item-use check covers delayed teleportation to a moving target and
snapshot continuation. Full campaign parity remains unverified.

Native JP coloured-rice markers follow `80055528` / `8005577c`: five return-point
slots, the hand-relative offset, uncached terrain queries, water rejection and
inventory refund, the source collision box, owner-only retrieval and pickup
restrictions. The source clears a touched marker's return-point slot before
checking afflictions; removal by water/eviction does not clear it. Slots reference
reusable scene models to retain that distinction. Models 32–36 come from the
user's item archive. 1,200 original-instruction terrain cases cover 773 accepted
and 427 rejected placements (`TENCHU2_MARKER_ORACLE`); a connected use/restore
check covers other-actor contacts, afflicted-owner contacts and retrieval.
`80051bb0` queues a pickup notice; it does not unlock campaign equipment. Native
projectile, poisoned-rice and marker pickups now emit that notice accordingly.

Native JP resurrection follows `8005a21c` / `8005a5f0`: unsigned horizontal
return-point ranking, home fallback, healing/affliction clearing, the substitute
log's fall and roll, half-health floor, shared counters, control suppression and
camera return. The renderer uses the uniform additive-white pattern from
`8003ba58` / `8003bf20`. Attacker voice 9 follows `8005d5ec` / `8005d638` with
the log as its positional reference; this is audio, not an AI command. Smoke
follows `80046630` / `800467d0`, including all 24 constructor RNG calls for the
single-particle variant. `TENCHU2_RESURRECTION_ORACLE` covers 1,000 original
instruction sequences with terrain, marker ties, phase boundaries and blocked
return transitions. The 600 mixed sprite oracle sequences include dispersing
smoke. A connected fatal-hit check exercises both return destinations, inventory,
health, effects, fade, snapshot continuation and resumed movement. The standalone
preview has no scene loader; `Controls::transition_ready` remains true there.
Campaign loading and XA streaming must supply the original readiness condition
when connected (boss objectives bypass that wait).

Native JP campaign descriptors and mission starts come from `80084b2c` and
`80085b5c` in the user's verified executable. Objective evaluation, target and
population initialization, input suppression, attack-completion wait and exit
countdowns follow the gameplay loop `80064a34`. This includes source ordering
when an unseen objective is both completed and failed in the same update, signed
coordinate truncation, the 99-minute limit, the shorter fade into results, and
last-20-update item suppression. `TENCHU2_MISSION_ORACLE` compares 1,800 initial
conditions and 18,144 countdown updates against original instructions;
`TENCHU2_POPULATION_ORACLE` covers 240 target/population cases. Music scheduling
and camera internals are excluded from these block comparisons. A connected
native-frame test reaches a real exit, blocks input and restores mid-fade.
The regular launcher still uses the Gaisen sandbox; native story dispatch and
campaign carry-over remain integration work.

Native JP result calculation follows `8006343c`: score narrowing, seven ranks,
special results 207/211, best-record ordering, supply awards and returns, and
grand-master/Tatsumaru unlocks. Reward thresholds and item IDs are read from
`80085bd4`; starting stock and record sentinels follow boot `80014374` in
`SLPS_030.40`. `TENCHU2_RESULTS_ORACLE` compares 600 original-instruction cases,
including complete stock/record tables and final RNG. UI drawing is excluded.
`TENCHU2_PROGRESS_INITIAL` checks initialization against the original boot code.
A native-frame check completes sequence 134, computes its result once and
restores the awarded progress from a snapshot. These checks do not establish
cross-area statistics, story progression or the original results presentation.

JP story animation reads the user's `cab/event/E*.CAB` and `event/E*.EVT`
resources using the layout in `EVENT.BIN` routine `800d1c78`. This is a separate
format from Gaisen's CAD scripts. Root movement and joint sampling follow
`800d3554`, `800d38d0`, `800d3be8`, `800d3f5c` and `800d4104`, including relative
translation, packed scale/linear flags, interval boundaries and stopped tracks.
Spline evaluation reuses the port's Gaisen `eval_spline_gte_` implementation;
the JP call is `8002f628`. All 142 campaign event resources parse, and
`TENCHU2_STORY_MOTION_ORACLE` compares 6,009 original instruction states across
145 available event files, including restoration between samples. The oracle
executes the animation code and emulates its spline COP2 operations; matrix
publication is excluded. Story rendering, dialogue, effects and campaign
dispatch remain unconnected at this boundary.
