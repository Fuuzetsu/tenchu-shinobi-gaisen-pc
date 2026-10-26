# Retail campaign reference

This observer replays an archived `tenchu-search` controller stream in its pinned
BizHawk/Nymashock runtime. It reads MAIN instruction boundaries and RAM without
writing emulated game memory. The archived runtime verifies the exact core hash
and CPU ABI; the native observer also distinguishes MAIN from overlapping MENU
and ENDING code.

```sh
bash tools/retail-capture/capture.sh ARCHIVE --boot INPUT.segments NEW_OUTPUT
# Or replace --boot with an archived .State.
./dev.sh cargo run --release -p tenchu-pc -- \
  check-retail DISC NEW_OUTPUT/updates.ram.gz > native-validation.json
```

`TENCHU_SETUP_CHECKPOINTS` accepts the archived setup runner's checkpoint list.
Compare those RAM dumps against the archive before using an observed replay as
evidence. Output paths must be new. The capture retains its compiled observer,
source and hashes in `NEW_OUTPUT.runtime`; copyrighted captures belong outside
Git. `TENCHU_RETAIL_PHASE_FIELDS=START:END` additionally snapshots every selected
phase and every effect-slot return in that inclusive field range for diagnosis.

`TNRUN002` is a gzip stream with 56-byte little-endian event headers:
size, PC, field, skip, 64-bit CPU cycle, game clock, stage, latched pad, RNG,
display epoch, caller, v0, a0. Snapshots contain a patch count followed by
(offset, length, bytes) runs over 2 MiB RAM. The first snapshot covers all RAM.
Later patches replace changed 256-byte pages, coalescing adjacent pages. This is
lossless; ordinary events have no payload. `inspect.py` validates the stream and
can compare its decoded events/RAM against another capture.

The native checker initializes at the first StageSequence of each mission,
executes the recorded controller/cadence stream, and stops at the first
disagreement. It inspects state before EndDrawing, then applies the observed
display swap and cinematic continuation. It does not refresh native state from
later snapshots. An optional fourth CLI argument limits the checked update count
for diagnosis; such a prefix report does not claim mission completion.

The October 2026 Ayame/Normal campaign under `~/tenchu-searches/FINAL-RUN.txt`
matched all 77 console checkpoint RAMs and all 33,404 native gameplay/cinematic
updates, including 349 skipped draws. Boot/menu rendering is outside this core
comparison; video capture must document its presentation of those intervals.
