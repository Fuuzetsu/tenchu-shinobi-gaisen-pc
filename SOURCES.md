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
