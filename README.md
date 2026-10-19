# Tenchu: Shinobi Gaisen PC

A Rust reimplementation of the gameplay in the PlayStation release
**SLPS-019.01**, using assets from the user's game disc.

The target is faithful mechanics, including gameplay glitches, with a modern
renderer and portable runtime. Gameplay advances at the original nominal rate;
display refresh and graphics settings do not determine game speed. Reproducing
PS1 hardware slowdown is outside the target.

Development starts with Mission 1. The complete goal includes the functionality
of `MAIN.EXE`, all missions, both characters, cinematics, progression and
save/retry. A simple launcher replaces the original main menu.

See [PLAN.md](PLAN.md) for deliverables and acceptance criteria and
[AGENTS.md](AGENTS.md) for development conventions.

Game data, captured RAM and disc images must remain outside Git. No BIOS is
required by the intended native runtime. The initial implementation is in
progress; this repository is not yet a playable game.
