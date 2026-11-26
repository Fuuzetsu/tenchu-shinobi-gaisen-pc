# gilrs 0.11.2: worker shutdown backport

Source: the published `gilrs-0.11.2.crate` from
https://crates.io/crates/gilrs/0.11.2, upstream revision
`07e286e24b046cf39e5c367daa2770b805a64692`.
Published archive SHA-256:
`902fb00d3f6398e635be22e5c837b303c501835cca7ac11a47bba138f7aafdd8`.
The license files were retrieved from the same upstream revision's root
(the published archive omits them).
MIT or Apache-2.0; both upstream license files and file notices are retained.
The SDL mapping database retains its own notices.

Backport only upstream commit
[`5c38dfd3684114dba10ad281e6b3714e8334bf56`](https://gitlab.com/gilrs-project/gilrs/-/commit/5c38dfd3684114dba10ad281e6b3714e8334bf56),
“Shut down the force feedback worker when Gilrs is dropped”, to the published
0.11.2 source. Changed runtime files: `src/gamepad.rs` and `src/ff/server.rs`.
No intervening mapping, battery polling, platform-driver or Windows enumeration
changes are included. `gilrs-core` remains the registry version 0.6.8.

The released worker does not exit when its channel disconnects, and retains
device handles while continuing to submit vibration state. Live switching to
SDL requires that worker to shut down, stop active motors, close its handles and
join before opening SDL devices. Dropping only our active effect is insufficient.

Local worker tests cover explicit shutdown and channel-disconnection exit.
Formatting normalizes the backported functions and whitespace/heading markup in copied metadata. Examples and their unused development dependencies are omitted from the local
Cargo manifest. The copied runtime source is otherwise unchanged; Cargo registry bookkeeping and the package's independent
lockfile are omitted. This path patch applies identically to native, Windows,
Android and WASM builds through the root Cargo manifest and pinned Nix source.
Remove it once a reviewed published gilrs release includes the shutdown fix.
