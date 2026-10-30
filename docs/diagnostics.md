# Diagnostic reports

When a bug happens, open **F1 → General → Save diagnostic report…**. Desktop
builds ask where to save; browsers download a `.json` file. Attach that file to
the bug report with a short description of what happened and what you expected.
For performance problems, enable **Performance charts** before reproducing the
issue to include GPU timestamps when the adapter supports them.

The report includes:

- Version, source revision (including local tracked changes), build target and
  compiler; OS/browser, GPU, driver, renderer, resolution and presentation mode.
- Current settings and controller/keyboard bindings, effective renderer settings,
  gameplay/debug overrides, model selection and loaded artwork counts/errors.
- Mission, character, difficulty, layout, randomizer seed/configuration if active,
  RNG, actors, AI/alert/radar, inventory, active items, camera and mission counters.
- Up to 600 recent gameplay inputs and their preceding player/alert state, plus
  the performance chart's most recent 300 frames. Retries clear input history.

It is UTF-8 JSON with `format: "tenchu-diagnostic-report"` and `schema_version: 1`.
Positions use the original game's integer coordinates; durations have explicit
units. IDs are game indices, never process pointers. Windows, Linux, macOS and
browser reports use the same structure. A report can be opened in a text editor
or inspected **without a disc** on any native platform:

```sh
tenchu-pc inspect-report tenchu-report-0.3.2-m3-tick1234.json
```

The command validates the format and prints a short summary; the JSON retains
all settings and actor details. Unknown future schema versions are rejected
explicitly rather than interpreted as the current format.

This is diagnostic context, not a complete savestate or replay. It includes no
disc assets, textures, raw process memory, campaign save, username/hostname query,
or environment dump. Model directory values and known personal path roots in
errors are redacted. Review custom names and error text before sharing. Saving
does not upload anything, modify settings, or change the simulation.

See [gameplay validation](validation.md) for the current emulator comparisons and
the separate audit of optional enemy simulation settings.
