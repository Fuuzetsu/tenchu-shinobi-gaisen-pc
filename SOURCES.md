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
