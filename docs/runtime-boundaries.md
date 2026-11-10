# Shared runtime tools and presentation

Gaisen and native Tenchu 2 JP have separate simulation rules. Presentation and
inspection should consume their facts, without advancing either simulation or
maintaining a second live `Game` just to satisfy a renderer API.

| Concern | Shared code | Source adapter |
| --- | --- | --- |
| Camera and inspection commands | `debug/inspection.rs`, `debug/camera.rs` | `debug/gaisen.rs`; native `tools.rs` and `inspection.rs` |
| Follow/orbit camera and terrain avoidance | `navigation.rs` consumes an actor target and retained surfaces | Gaisen/native adapters supply actor identity, position, facing and simulation tick; Gaisen movement steering stays in `navigation/gaisen.rs` |
| Pose interpolation | `visuals/interpolation.rs` blends keyed matrices and detects teleports/camera cuts | Gaisen snapshots live in `visuals/interpolation/gaisen.rs`; native gameplay/story snapshots live in `tenchu2/native/interpolation.rs` |
| Overlay geometry and readable labels | `debug/overlay.rs` | `debug/gaisen/overlay.rs`; native `inspection.rs` |
| Settings edits and availability | `settings/edit.rs`, `settings/runtime.rs` | Runtime-specific command handlers for gameplay changes |
| Experiment inventory UI | `settings/menu.rs` uses named, numbered inventory slots | Gaisen/native adapters supply their own items and quantity rules; queued native commands edit actual player state |
| Light selection, priorities and budgets | `lighting.rs` | `lighting/gaisen.rs`; native `render/lighting.rs` |
| Sun/moon and visual time of day | `visuals/sky.rs`, shared renderer | Each runtime supplies its simulation tick and retained shadow geometry |
| Texture-based emitter discovery | `visuals/lights.rs` accepts a model catalog, VRAM and rules | `visuals/lights/gaisen.rs` supplies source meshes and instances |
| GPU shading, shadows, materials and post effects | `renderer` | Source scene preparation, including native palette/fog decoding |
| Weather and footprints | `visuals/weather`, `visuals/footprints`, `visuals/surfaces` | Source terrain and sole-contact adapters |
| Imported weapon resources | `tenchu-assets::weapon::Equipment` and `ImportedModel`; shared model/material preparation | T2 disc tables supply attachments, contact eligibility and draw rules; each runtime maps its rig slots |

Paths above are relative to `crates/tenchu-pc/src`. The small inspection interface
borrows live actor facts in Gaisen. Native UI retains only camera and actor facts,
not a cloned world. The overlay builder accepts positions, bounds and labels;
source adapters retain collision filters and event arithmetic. Native T2's sight
diagnostics call its own perception routine on local query state; eye-height
envelopes are illustrative and separate from the actual target ray. Teleport UI
is shared, with a checked native command that repairs pose, collision and camera
state without advancing simulation or RNG.

Native interpolation captures poses only when enabled, after each original update,
and samples them at presentation time. Bodies, equipment and held items use the
same pose, including shadow geometry. Follow cameras sample the same tick history
even when multiple updates occur between display frames. Camera-mode changes and
large jumps snap; story cameras retain their authored cuts. Swaps and teleports
reset the pose history. Capturing a native pose reads a copy of its small scene
graph so presentation never refreshes simulation coordinate caches.

Both runtimes use the same light selection and GPU shadow code. Native T2 maps
its explicit static flame emitters; its texture-based lamp metadata and transient
combat emitters are still missing. Static shadow/shelter geometry is retained by
the native environment cache. Source simulation visibility does not switch these
lights on/off as the camera turns. Both also drive the shared directional light
and sky clock; repeated presentation frames do not advance time, and fast-forward
accounts for every elapsed simulation tick. Original keeps these effects disabled.

For additional F1 features, connect an adapter to an existing shared component
before adding another renderer or menu implementation. Enable a control only when
its edit, reset and live application all work. Unsupported values must survive a
round trip between games without being applied to the wrong engine.

Weapon imports retain disc-qualified identities, source VRAM and artwork IDs.
They reuse the existing moveset rig pool and shared mesh/material pipeline, rather
than baking source pixels into host texture pages. The T2 weapon-table parser is
the attachment specification; Gaisen's original slots and equip semantics remain
as specified by `tenchu-decomp/src/main.exe/APPEAR.C`, with the additional T2
accessory slot used only by imports. `tenchu-viewer/src/weapons.rs` also provides
the Gaisen attachment reference. No original assets are bundled.

Character-local audio follows the selected moveset. The library extracts the
character program from its source disc into an owned VoiceSet; edition and cast
selection happen at load time, never in the audio thread. Both hosts translate
local cue meanings and retain their own spatialization, voice allocation and
scheduling. Sourced sounds carry optional bank ownership to the common mixer,
so imported voices coexist with mission sounds and cannot replace stage ambience
or other actors' samples. The mixer retains the bank for each active voice.
Gaisen stores selected voice resources with its shared mission assets so rewind
checkpoints do not serialize PCM every tick, and saves restore them independently
of the currently selected model. No additional gameplay RNG is consumed.

Remaining work includes further checked gameplay commands, more effect adapters,
and complete source motion-event timing. The existing cross-game melee bridge still converts
motion data through Gaisen's representation; this refactor does not make every
move, item or weapon interchangeable. Preserve original action semantics at that
boundary rather than making a universal AI/controller or silently substituting
host-game effects. The existing disc-backed tests verify state preservation;
interactive and rendered checks are also needed for each connected feature.
