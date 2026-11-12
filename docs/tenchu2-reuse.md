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
