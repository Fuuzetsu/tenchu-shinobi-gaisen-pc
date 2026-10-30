# Retail campaign reference

This observer replays an archived `tenchu-search` controller stream in its pinned
BizHawk/Nymashock runtime. It reads MAIN instruction boundaries and RAM without
writing emulated game memory. Optional archived item setup writes only the
pre-mission loadout through the setup runner; its original audit is retained.
The archived runtime verifies the exact core hash
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
lossless; ordinary events have no payload. `inspect_trace.py` validates the stream and
can compare its decoded events/RAM against another capture.

The native checker initializes at the first StageSequence of each mission,
executes the recorded controller/cadence stream, and stops at the first
disagreement. It inspects state before EndDrawing, then applies the observed
display swap and cinematic continuation. It does not refresh native state from
later snapshots. An optional fourth CLI argument limits the checked update count
for diagnosis; such a prefix report does not claim mission completion.

### Combat/item archives and custom scenarios

Capture the full immutable receipt chain, including its menu inputs and equipped
items, instead of starting from a clock-one gameplay savestate:

```sh
./dev.sh python3 tools/retail-capture/capture-archive.py ARCHIVE NEW_OUTPUT
# If the receipt store is separate, add --campaign STORE --tip RECEIPT_SHA256.
```

The helper verifies controller, RAM and savestate-chain hashes, captures each
receipt boundary, and requires byte-identical endpoint RAM. It only reads the
archive; an active search can keep running. `NEW_OUTPUT.json` records provenance,
checkpoint matches and the observation hash. The generated `.segments` is the
exact console-field input stream. `capture.sh` also accepts an optional fifth
argument pointing at the archived `item-setup.json`.

To try new inputs, choose a proven mission-entry receipt and append a text file
containing `field_count hex_buttons` lines (blank lines and `#` comments allowed):

```sh
./dev.sh python3 tools/retail-capture/capture-archive.py ARCHIVE NEW_OUTPUT \
  --tip ENTRY_RECEIPT --tail experiment.segments
./dev.sh cargo run --release -p tenchu-pc -- \
  check-retail DISC NEW_OUTPUT/updates.ram.gz --scenario
```

This still verifies the archived entry checkpoints. The appended inputs are a new
experiment, not a claim to reproduce an archived win. `--scenario` requires the
entire observation stream to match and end between updates, but permits an
unfinished mission or death; it cannot export a completed-run movie tape. No RNG,
actor or item state is repaired from later snapshots.

The checker covers clocks/RNG, actors and poses, AI, collision slots, inventory,
active item processors/coordinates and projectile parameters, armour, alert and
radar state, camera, rendering budgets and mission counters. This is a selected
state comparison, not proof that every byte of transient state is equivalent.

On 2026-10-30 the equipped Ayame mission 1–7 searches matched 19,075 native
updates (3,199 / 1,323 / 3,121 / 3,770 / 4,078 / 2,568 / 1,016). The Mission 7
snapshot was receipt `adc0ff53d80d201ac9a9b65327eafa5def4f67f87004b08b29e08d80022eaee8`;
the search continued independently. A further 1,866-update Mission 3 death
scenario exposed and verified a fix for stage-end actor control. These tapes
exercise item use and kills, but do not establish stealth-kill or every ordinary
AI path's correctness. Keep the local witnesses/captures outside Git.

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

The recorder uses Showcase, sharp fonts and the specified artwork pack. It
does not load/write normal preferences or saves, and does not apply gameplay
modifiers. Presentation interpolates models with one source update of visual
latency. The original NTSC field rate is converted rationally to exactly 60 fps:
88,075 fields produce 88,164 video frames, rather than accelerating the source
timeline by treating console fields as 60 Hz. Every frame is rendered offline;
GPU speed affects export time only.

Showcase enables local/sky shadows, refined models, hair/cloth motion, improved
water/waterfalls, grapple/contact effects, surface relief and post-processing.
Weather remains original and sky time is fixed. Actual applied GPU settings and
interpolation state are saved to `SILENT.settings.json`. Recorded draw skips still
affect the authoritative simulation; presentation reconstructs actor transforms
without mutating those caches, so a skipped console submission does not blank
models in the PC video.

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

`--jobs 2` renders two independent chapters concurrently, if GPU memory permits.
The default is one. Each chapter has its own log and actual-settings sidecar;
assembly refuses inconsistent settings. Keep a copy of the executable and pack
when exact reproduction matters. Artwork hashes include OBJ models as well as
PNG/JSON/binary assets.

This renders separate chapter parts, verifies their format/frame counts, then
joins them without re-encoding and adds 48 kHz stereo AAC audio. Audio must come
from the **identical continuous tape starting at field zero**, not an unrelated
playthrough. It preserves source speed and pads only the fractional final video
frame. The soundtrack is console audio, not output from the native sound mixer.
Joins use frame-count durations explicitly, since MP4 chapter tracks round to
milliseconds and can otherwise introduce small video timestamp gaps.

The script freezes binary/tape/audio/artwork hashes, refuses mixed-input resumes,
checks every video timestamp and decodes the entire result before publishing.
It writes YouTube chapter text and a verification report beside the MP4. Retained
parts and their logs allow a failed render to resume without starting over. Keep
all recordings and extracted replay data outside Git, normally under `private/`.

For the shorter viewing edit, reuse the rendered parts:

```sh
./dev.sh python3 tools/retail-capture/shorten.py REPLAY.json \
  MATCHING-LOSSLESS-AUDIO.wav OUTPUT.parts YOUTUBE.mp4
```

This keeps about three seconds of title, the complete gameplay and in-engine
cutscenes, the mission results, roughly one second of each loading transition,
and four seconds of the final result. Cuts use actual closed-GOP keyframes and
copy the video packets without re-encoding. Audio uses the identical retained
intervals, with 10 ms fades at joins to avoid clicks. A frame/sample edit map,
input hashes, exact timestamp checks and a complete decode accompany the MP4.
The short title shares Mission 1's chapter so every YouTube chapter exceeds ten
seconds. The uncut source parts remain available.

Measure soundtrack alignment at several non-silent points across the finished run:

```sh
./dev.sh python3 tools/retail-capture/check_audio.py \
  MATCHING-LOSSLESS-AUDIO.wav OUTPUT.mp4 100 750 1400 > audio-alignment.json
# For an edited video, choose sample times from its shorter timeline:
./dev.sh python3 tools/retail-capture/check_audio.py \
  MATCHING-LOSSLESS-AUDIO.wav YOUTUBE.mp4 30 140 --timeline YOUTUBE.edit.json
```

The check searches ±150 ms, then refines the best lag to one 12 kHz sample. It
requires near-zero lag and correlation above 0.995; silence is not accepted as
alignment evidence. Codec priming is handled by the standard AAC mux timestamps.
Sample extraction starts on the common 44.1/48/12 kHz grid, avoiding resampling
phase differences when testing fractional-second positions across an edit.
The export uses MP4/H.264 High, closed GOPs, 4:2:0 BT.709 and stereo AAC at 48 kHz,
following [YouTube's upload format recommendations](https://support.google.com/youtube/answer/1722171?hl=en).
