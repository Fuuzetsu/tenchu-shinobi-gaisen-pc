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

## Native 4K recording

Create a compact replay only after the complete RAM comparison passes:

```sh
./dev.sh cargo run --release -p tenchu-pc -- \
  prepare-retail DISC UPDATES/updates.ram.gz ARCHIVE/final-campaign.json REPLAY.json \
  > validation.json
TENCHU_ASSET_PACK=PACK ./dev.sh target/release/tenchu-pc \
  record-retail DISC REPLAY.json SILENT.mp4 [START_FRAME [END_FRAME]]
```

`REPLAY.json` contains mission initialization and each consumed input, source
skip branch, display swap, completion field, RNG and clock. It carries hashes of
the input tape and reference observations. Initialization includes loader caches
and ambient sound timers as well as RNG; RNG alone is insufficient. The recording
checks every update's clock/RNG again and never imports subsequent console state.
Frame ranges always replay their entire prefix, including cosmetic ticks.

The recorder uses Modern Ultra, sharp fonts and the specified artwork pack. It
does not load/write normal preferences or saves, and does not apply gameplay
modifiers. Presentation interpolates models with one source update of visual
latency. The original NTSC field rate is converted rationally to exactly 60 fps:
88,075 fields produce 88,164 video frames, rather than accelerating the source
timeline by treating console fields as 60 Hz. Every frame is rendered offline;
GPU speed affects export time only.

Boot/loading intervals retain their duration but display the port's native title,
mission-selection and results screens. They are a presentation of the original
timeline, **not a validated MENU.EXE replay or a BIOS simulation**. Gameplay and
in-engine cutscenes use the verified source camera and controller stream. The
archived full run ends at the start of the ending movie; this export holds its
final native results screen through that last loading interval.

For a resumable full export with the matching console soundtrack:

```sh
./dev.sh python3 tools/retail-capture/record.py DISC REPLAY.json \
  MATCHING-LOSSLESS-AUDIO.wav OUTPUT.mp4 --asset-pack PACK
```

This renders separate chapter parts, verifies their format/frame counts, then
joins them without re-encoding and adds 48 kHz stereo AAC audio. Audio must come
from the **identical continuous tape starting at field zero**, not an unrelated
playthrough. It preserves source speed and pads only the fractional final video
frame. The soundtrack is console audio, not output from the native sound mixer.

The script freezes binary/tape/audio/artwork hashes, refuses mixed-input resumes,
checks every video timestamp and decodes the entire result before publishing.
It writes YouTube chapter text and a verification report beside the MP4. Retained
parts and their logs allow a failed render to resume without starting over. Keep
all recordings and extracted replay data outside Git, normally under `private/`.
