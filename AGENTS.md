# Development contract

- Keep working toward the complete `MAIN.EXE` game. Completing Mission 1 or
  another milestone is a reason to advance the plan, not declare the goal done.
- Read `PLAN.md` and current implementation before selecting work. Update the
  plan with concrete results and unresolved boundaries.
- Reuse and verify the existing work in `~/programming/tenchu-decomp-glitch-research`,
  `~/programming/tenchu-decomp` and `~/programming/tenchu-viewer`. Do not modify
  those repositories as part of this port. Record provenance for imported code.
- Use the TAS model as a behavior/algorithm reference, not as an architecture to
  copy. Write straightforward scalar code over ordinary owned game state. Do not
  import batched worlds, SoA slabs, SIMD ownership machinery, search continuations
  or speculative optimisation. Scalar loops may auto-vectorise normally.
- Preserve source arithmetic, operation order, collision, AI, RNG, animation and
  gameplay glitches. Do not substitute generic physics or approximate mechanics.
- Separate game updates from presentation. Run nominal original logic ticks;
  do not infer PS1 frame costs or trigger gameplay skip branches when the PC
  renderer drops a frame.
- Keep the core usable on native desktops and `wasm32-unknown-unknown`. Platform
  file access, clocks, windows and audio belong at the boundary. A portable
  implementation is required before optional CPU-specific optimisation.
- Use simple code and coherent modules. Avoid speculative abstraction, duplicate
  logic, obvious comments and tests that merely restate implementation.
- Useful tests include source arithmetic edges, recorded state comparisons,
  asset parsing, gameplay transitions and independence from rendering cadence.
- Run `cargo fmt --all` and appropriate focused checks before each implementation
  commit. Commit coherent increments; keep the tree reviewable.
- Never commit copyrighted game assets, disc images, BIOS files, captured RAM or
  generated build outputs. The runtime reads the user's disc; replacement assets
  will use explicit overrides with original assets as the fallback.
- A diagnostic checkpoint or scripted successful route does not prove a complete
  game. Report exactly what is supported and exercise ordinary interactive play.
