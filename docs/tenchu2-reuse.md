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
