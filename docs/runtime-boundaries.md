# Shared runtime tools and presentation

Gaisen and native Tenchu 2 JP have separate simulation rules. Presentation and
inspection should consume their facts, without advancing either simulation or
maintaining a second live `Game` just to satisfy a renderer API.

| Concern | Shared code | Source adapter |
| --- | --- | --- |
| Camera and inspection commands | `debug/inspection.rs`, `debug/camera.rs` | `debug/gaisen.rs`; native `tools.rs` and `inspection.rs` |
| Overlay geometry and readable labels | `debug/overlay.rs` | `debug/gaisen/overlay.rs`; native `inspection.rs` |
| Settings edits and availability | `settings/edit.rs`, `settings/runtime.rs` | Runtime-specific command handlers for gameplay changes |
| Light selection, priorities and budgets | `lighting.rs` | `lighting/gaisen.rs`; native `render/lighting.rs` |
| Texture-based emitter discovery | `visuals/lights.rs` accepts a model catalog, VRAM and rules | `visuals/lights/gaisen.rs` supplies source meshes and instances |
| GPU shading, shadows, materials and post effects | `renderer` | Source scene preparation, including native palette/fog decoding |
| Weather and footprints | `visuals/weather`, `visuals/footprints`, `visuals/surfaces` | Source terrain and sole-contact adapters |

Paths above are relative to `crates/tenchu-pc/src`. The small inspection interface
borrows live actor facts in Gaisen. Native UI retains only camera and actor facts,
not a cloned world. The overlay builder accepts positions, bounds and labels;
source adapters retain collision filters and event arithmetic. Native T2's sight
diagnostics call its own perception routine on local query state; eye-height
envelopes are illustrative and separate from the actual target ray. Teleport UI
is shared, with a checked native command that repairs pose, collision and camera
state without advancing simulation or RNG.

Both runtimes use the same light selection and GPU shadow code. Native T2 maps
its explicit static flame emitters; its texture-based lamp metadata and transient
combat emitters are still missing. Static shadow/shelter geometry is retained by
the native environment cache. Source simulation visibility does not switch these
lights on/off as the camera turns. Original keeps dynamic lighting disabled.

For additional F1 features, connect an adapter to an existing shared component
before adding another renderer or menu implementation. Enable a control only when
its edit, reset and live application all work. Unsupported values must survive a
round trip between games without being applied to the wrong engine.

Remaining work includes further checked gameplay commands, more effect adapters,
and character resources with source-qualified
weapons and motion audio. The existing cross-game melee bridge still converts
motion data through Gaisen's representation; this refactor does not make every
move, item or weapon interchangeable. Preserve original action semantics at that
boundary rather than making a universal AI/controller or silently substituting
host-game effects. The existing disc-backed tests verify state preservation;
interactive and rendered checks are also needed for each connected feature.
