# Controller setup and troubleshooting

Open **F1 → General → Controls → Controller connection and input test**.
The backend selector applies immediately and remembers the choice for the next
launch. **gilrs** remains the default. The same panel is available in native T2.

| Backend | Purpose |
| --- | --- |
| gilrs | Existing input path; Windows uses Windows Gaming Input (WGI). |
| SDL automatic | SDL's normal device selection, including HIDAPI and Windows XInput. Start here when gilrs fails. |
| SDL XInput only (Windows) | Isolates the Xbox-compatible input path used by Steam Input gamepad emulation. Direct non-XInput controllers may disappear. |
| SDL WGI only (Windows) | Compares SDL's WGI implementation with gilrs; other physical Windows drivers are disabled. |

Only one backend runs at a time. Switching releases controller holds and stops
vibration. Keyboard bindings remain independent. **Controller** chooses a
specific device for this session; **Automatic** waits for a button press or a
deliberate stick movement and retains that device until disconnect. Device IDs
are local to the current backend attempt and are not saved across launches.
**Retry connection** reinitializes the selected backend.
**Release stuck input** suppresses current controller holds until they release.

The live readings show normalized inputs before gameplay/overlay suppression.
They continue while Controls pauses gameplay. Positive stick Y means up; trigger
readings run from 0 to 1. Existing bindings retain their physical positions.
L2/R2 item cycling follows the original game's order; do not infer a swapped
trigger from item names alone. Check the L/R readings and bindings first.

For Steam Controller use, add the executable as a non-Steam game and enable an
ordinary **Gamepad** Steam Input layout. Keep the standard gamepad outputs
enabled; assign extra/back buttons to the port's keyboard shortcuts. Unbinding
every output can remove the emulated controller input. The game does not need a
Steamworks AppID or a dedicated Steam Controller integration for this path.
SDL automatic preserves Steam's device-exclusion and mapping hints. Launching
outside Steam may leave a Steam Controller using its desktop layout.

## One testing session, enough evidence for follow-up

1. Use the new release's complete extracted folder. In the input-test notes,
   enter the controller model/generation, USB/dongle/Bluetooth connection,
   whether this is a Steam shortcut or a direct executable launch, Steam Input
   enabled/disabled, layout name/template, extra-button keyboard assignments,
   and other remappers or virtual-controller software in use. State whether
   Steam's overlay actually opens. The report includes port/build/OS details.
2. With the failing launch configuration, test **gilrs**, **SDL automatic**,
   **SDL XInput only**, and **SDL WGI only** in that order on Windows. Keep the
   same Steam layout throughout this comparison. Note failures as well as
   successes. If two devices appear, try each explicitly and record which works.
3. In each mode, press/release each D-pad direction, Start, crouch, movement,
   item/harpoon, L2 and R2 individually. Move each stick fully and release it.
   Check for a missing press, a missing release, a stuck axis, or the wrong
   device becoming active. Test a keyboard-mapped back button and look for its
   bound action mask. Click outside the notes field before testing keyboard
   mappings. Then close Controls for a short gameplay check; test losing and
   regaining window focus too.
4. Reopen Controls and describe that mode's result in its notes **before**
   switching. Save a diagnostic report after the comparison: the last eight
   backend attempts and their notes/activity are retained together. Reports are
   local JSON files; saving does not send them anywhere.
5. If practical, restart with a different launch mode (Steam/direct) or
   connection (USB/Bluetooth), repeat the comparison and save a separately
   named report. History is limited to the current process. For a direct
   Steam Controller launch, describe any desktop-layout behavior rather than
   assuming it presents a gamepad.
6. Attach the report(s) and say which backend/device worked, whether actions
   remain held after release, and whether keyboard-bound back buttons work.
   For a crash, distinguish a game exit, graphics-driver reset, blue screen,
   and full system lockup; include any visible Windows error/stop code and its
   time. Performance hitches need a separate report with Performance charts
   enabled during reproduction.

If the game cannot be opened, run this from a terminal in the extracted folder:

```powershell
.\tenchu-pc.exe check-controllers > controllers.json
```

Linux: `./tenchu-pc check-controllers > controllers.json`.
Append `gilrs`, `sdl`, `sdl-xinput` or `sdl-wgi` to check one available
backend in isolation if another crashes or hangs. To launch directly with SDL,
set `$env:TENCHU_CONTROLLER_DRIVER = 'sdl'` in PowerShell before launching the
executable (Linux: `TENCHU_CONTROLLER_DRIVER=sdl ./tenchu-pc`). This launch
override takes precedence over the saved choice until removed.

This disc-free check initializes, polls and closes each available backend twice,
without changing saved preferences. It writes a standard diagnostic report;
a nonzero exit indicates an initialization failure whose details remain in the
report. It does not check physical input correctness. The Windows release smoke
test runs it to cover SDL loading and backend reinitialization without hardware.

These options are diagnostic and compatibility paths, not confirmation that
the reported Windows/Steam hardware combinations are fixed. Review notes and
error text before sharing. See [diagnostic report contents](diagnostics.md).
