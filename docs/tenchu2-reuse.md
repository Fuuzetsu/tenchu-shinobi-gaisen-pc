# Reusing the Shinobi Gaisen decomp for Tenchu 2 JP

The existing `../tenchu-decomp` is a useful reference for naming and understanding
Tenchu 2 routines. It is not a substitute for checking their behavior against the
Japanese T2 executable (`SLPS-030.40`). The native T2 implementation already shares
scalar math, scene transforms and camera smoothing where their behavior agrees.

## Checked examples

Addresses refer to the loaded executables, not disc offsets.

| Shinobi Gaisen | Tenchu 2 JP | Finding |
| --- | --- | --- |
| `RotateVectorS`, `80039a64`, `EFFECT.C` | `80060268` | Same 43 instructions except the three call targets. They still call `memset`, `RotMatrixYXZ` and `ApplyMatrixSV`. The signed-short rotation operation can be reused. |
| `GetMoveSpeed`, `80029660`, `HUMAN.C` | `8002f2ac` | Same horizontal trigonometric calculation, with reordered parameters. T2 does **not** zero vertical velocity; calling the T1 helper unchanged would change movement. |
| `GetDirection`, `8002972c`, `HUMAN.C` | `8002f4ac` | Related relative-angle calculation, but T2 uses `ratan2(dx, dz) + 2048` and subtracts 4096 above the positive half-turn. T1 uses `ratan2(-dx, -dz)` and its positive overflow branch returns `4096 - difference`. Preserve the build-specific arithmetic and edge cases. |
| `PlayMotion`, `8001c584`, `ACTION.C` | `8001acdc` | Same animation-manager ancestry. T2 updates the pose when mode is zero, then advances its counter in either mode; T1 branches differently. A stopped T2 motion returns -1 rather than 0. |
| `SuccessionAttack`, `8002fabc`, `THINK.C` | `80035740` | Same combo-frame/range/angle/RNG decision structure. T2 additionally clears the generated attack bit when attack is already held. The native T2 combo routine retains that check. |

These are manual comparisons of the decomp, disassembly and call targets. The
native implementation has separate original-instruction checks for its motion,
movement, camera and AI behavior; this table alone is not a parity test.

## How the candidates were found

A local comparison used 1,618 Gaisen and 1,368 JP T2 function ranges. Unique
equal-sized instruction bodies produced 94 exact matches, all SDK routines.
Ignoring jump targets and selected relocated addresses produced 196 candidates,
including `RotateVectorS`. Ignoring all non-branch immediate operands produced
377 candidates, mostly SDK code. Duplicate wrappers were excluded from those
counts. Function-boundary differences and compiler changes also prevent matches,
so these counts are **not** a measure of how much source code was reused.

Mnemonic-sequence similarity also identifies candidates such as `MakeDifSub`,
`SweepMotion`, `UpdateSplineControl`, `Think3area` and `AttackAnimal`. Those names
are leads, not evidence that their complete behavior is interchangeable. Consult
the corresponding T1 source first, then check T2's fields, constants, operation
order, branches and RNG calls before sharing an implementation.

No usable T2 debug-symbol release has been verified. The known demo `PSX.SYM`
belongs to Tenchu 1 and is already used by `../tenchu-decomp`. This comparison does
not depend on finding further symbols and commits no executable or captured data.

## Imported T1 cinematic cast

The cinematic bridge follows `Director::native_actor` and the bound native
actor's current model and weapon definition. It reuses the player override's
shared retargeting, material and equipment paths; original T1 scripts, visibility,
camera and poses remain the scene source. Actors without a native binding or a
compatible humanoid rig retain their original presentation.

T2 costume joints outside the shared 15-part rig follow their nearest retained
ancestor, preserving their geometry and bind positions. This is the same rule as
`tenchu-assets/src/tenchu2/model.rs::parts`, verified against that parser's separate
disc import, including Suzaku's extra joint. The source Onikage identifiers
`NINJA_0`/`NINJA_1` (`0x87`/`0x8a`) come from the existing Gaisen decomp enum;
the existing native mapping selects `0xf6` (`suzaku`). No gameplay mapping changes.

Disc-backed checks cover M1 and the actual **Free the Princess** M10 encounter
for both heroes, drawn/sheathed equipment, player overrides, source visibility,
and unchanged serialized World/Director state. All 22 mission/hero combinations
load their converted cast; 24 distinct bodies match the separate disc importer's
geometry and joint attachments. Internal stage ID 7 is M10;
stage ID 9 is M4, despite earlier fixture filenames. Before/after captures and
measurements are outside Git in `private/t2-performance-20261113`.

## Native T2 title menu

The title state follows JP `MENU.EXE` `8010fa3c`, called by `8010f87c`.
The four `title.arc` images use layout descriptors at `80130e0c`; shared sprite
parsing and the existing frontend Canvas/GPU renderer supply presentation.
The port preserves release-before-press input, the initial menu reveal,
non-wrapping selection, signed copyright-color increment, subtractive fade and
1,801-tick idle boundary at nominal 60 Hz. Host rendering cadence does not drive
the source's slow-frame counter branch. The PC's Settings entry opens F1;
T2 `TRIAL.EXE` and the original options submenus remain separate boundaries.

The local oracle executes the original title instructions with pad, sound and
sprite submission hooks: 66 traces / 59,151 updates cover held and simultaneous
buttons, navigation limits, confirmation/fade boundaries and attract timeout.
Evidence: `private/render-consolidation-20261112/title-oracle.py` and its JSONL;
neither executable bytes nor oracle data are committed. The `OP_MOV.STR` and
`TITLE.STR` cutoffs are 2,690 and 213, and both files carry stereo XA on channel 1.
Campaign movies retain their existing three-frame tail cutoff. Native title
integration checks cover disc import, save isolation, mission selection/back,
the editor boundary, settings, movie skip edges and rendered control prompts.

## Training sound inspection

The overlay reads the existing JP `DATAMNG.BIN` lesson table at `800d8d68`,
course starts at `800d8f18` and course bounds at `800d8f24`, used by handler
`800d202c`. Entry and reach boxes preserve its division by 1000 toward zero,
strict horizontal radius and exact vertical cell. Nonspatial action checks
remain in the lesson controller. The narration comes from the existing
`NARAT_J.XA` track parser; scenery markers use the native environment trigger
kinds and their sound IDs. Preview mixing and picking are PC presentation only.
No core behavior or sibling-repository code was changed for this overlay.

## Native T2 campaign return

The JP `GAME.EXE` campaign dispatcher at `80062310` returns after its terminal
sequence or cinematic quit. Its caller at `80066378` then cleans up and executes
`\TENCHU2\MENU.EXE;1` at `80066388`–`80066394` (path at `80013368`). The menu
entry calls `8010f87c`, which enters the title at `8010f9bc` and mission selection
at `8010fa04`. The PC therefore returns menu-launched campaigns to the native
title after endings/credits and Start+Select cancellation during narration or
movies. Direct developer previews retain their existing host-exit behavior.

Campaign selection uses the existing source-derived unlock rules and saved
progress; F6 explicitly selects unrestricted access. The existing mission-menu,
results/reward and campaign-dispatch oracles under
`private/tenchu2-campaign-20261108` were rerun for this boundary. The Gaisen
research, decomp and viewer repositories were consulted read-only; none supplies
the T2 campaign frontend. No sibling code or game assets were imported. Audit
logs and disassembly remain in `private/t2-campaign-audit-20261113`.

## Native T2 story effects

The 2026-11-12 completion uses JP `EVENT.BIN` command dispatch `800d42b4`,
actor/sprite drawing `800d2e6c`, sprite motion `800d3794`, resource loading
`800d6178` / `800d67e8`, and per-joint ignition `800d66bc`. `GAME.EXE` supplies
story sparks `80041e60`, water spray `80045008`, dust `8003db48`, dispersing
smoke `80046630`, explosion `8003ec74`, smoke `8003ddec`, leaves `80046cfc` /
`8004711c`, ripple/splash `80044f98` / `8004518c`, and sprite projection
`80046b94`. Common particle updates reuse the already checked native routines.
Leaf quads share the paper renderer, retaining their distinct source motion.
SPR files are ordinary TIMs with the same palette blending normalization;
models 285 and 289 intentionally have no drawable resource.

Frame feedback (`8003b3dc`, invoked by `800d4c20`) blends the preceding
frame using either half-current/half-previous or current + 3/4 previous, then
draws the closer overlays. The authored phase word is not a geometric warp.
The PC renderer retains a fixed previous frame for repeated draws of a logic
tick and clears history after scene changes, seeks, resize or long gaps.
Source cutscene initialization `800d1150` clears the positional-audio listener;
the explosion's call to `8001ecb4` therefore produces no sound in that mode.

The local original-instruction fixture has 180 timed-command cases and 30
per-bone ignition cases, including RNG and particle lifetime comparisons.
The disc-backed event audit runs all 141 reachable resources (146 sequence
placements, 178,133 updates). This is scripted-cutscene validation, not a new
ordinary-play recording. Executables, extracted data, fixture outputs and
review captures remain outside Git under `private/story-completion-20261112`.
The sibling Gaisen research repositories were consulted read-only; these T2
behaviors were checked directly against the JP executable instead of inferred
from the related T1 routines.

## HD interface (v0.33.0)

T2 menus reuse `frontend::draw::Frame`, its content identities, filtered artwork
and GPU cache; HUD sprites bind through the shared material renderer and pack
recipes. T2 resolves its own archive, including on Gaisen stages under T2 rules.
The existing Japanese resource readers provide layout/UVs and item identities;
health fills and digit slices retain source texture coordinates. Only presentation
changes. The source blend classes (opaque black Ki strokes versus additive colour)
remain separate. Button-legend swapping applies the same coordinates to original
and replacement images, scaled to the authored resolution.

The read-only sibling viewer's `src/items.rs` and Gaisen decomp's
`src/main.exe/INFOVIEW.C` confirm the shared item-sprite/layout convention; their
T1 item IDs and viewer alpha simplification are not imported into T2. Existing
T2 asset/HUD source comparisons remain authoritative. No sibling code or retail
art was copied. Artwork provenance is tracked in `artwork/tenchu2/`; generated
paintings and authored type are separate from runtime disc-derived lettering.

## Invisibility and teleport presentation

JP `GAME.EXE` initializes the shared trail GT4 template at `800c5410` from
`EFFECT.ARC` image 0 (`8003a810`–`8003a828`). Both the ribbon renderer
`8003c90c` and invisibility handler `800584b0` submit that textured primitive
through `80043a68`; textured RGB uses 128 as unity. At `80058858`, the cloth
uses half the first joint's ordering depth, preserving its screen coordinates
while placing it in front of the body. The PC renderer retains that deliberate
depth placement rather than treating the cloth as an ordinary world surface.
Imported-stage sprite retention uses the existing independent T2 material cache.

The body transition in `800584b0` sets page bits `0x60` on the actor's MDL faces,
selecting background + foreground/4; it does not change the separate weapon
models. The TIM loader at `80017600` prepares eligible T2 palettes for blending.
T1 and custom body materials bypass that loader and can retain opaque texels.
Their concealment therefore blends every covered texel at draw time, preserving
transparent cutouts and the original material when the effect ends. Filtered
sampling combines both source texel classes for this case. The shared shader's
flag excludes foliage, whose wind weights occupy the same high bits. Native T2
palette masks and stage shading retain their original behavior.

Retail BizHawk captures confirm the white cloth, subsequent translucent body
with visible sword, and teleport's leaf burst, delayed relocation and failed
use without a target. Controlled capture setup supplied inventory; one teleport
fixture placed the player near an existing guard. Exact writes, controller
fields, original states, audio and video remain private under
`private/t2-item-review-20261113`. These captures are demonstrations, not a full
ordinary-play or pixel-for-pixel rendering comparison. The existing instruction
oracles cover item timing, cancellation, targeting, placement and leaf motion.
Sibling repositories were consulted read-only; no sibling code was imported.
