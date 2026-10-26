#!/usr/bin/env python3
"""Shorten campaign loading intervals without re-encoding or cutting gameplay."""

import argparse
import json
import os
from pathlib import Path
import subprocess

from record import check_video, digest, frame, probe, run


def keyframes(path):
    data = json.loads(subprocess.check_output([
        "ffprobe", "-v", "error", "-select_streams", "v:0", "-show_packets",
        "-show_entries", "packet=pts,flags", "-of", "json", str(path),
    ]))
    return sorted(int(p["pts"]) // 1000 for p in data["packets"] if "K" in p["flags"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tape", type=Path)
    parser.add_argument("audio", type=Path)
    parser.add_argument("parts", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    for key, value in vars(args).items():
        setattr(args, key, value.resolve())
    if args.output.exists():
        raise SystemExit(f"Refusing to overwrite {args.output}")
    tape = json.loads(args.tape.read_text())
    starts = [0] + [frame(m["updates"][0]["field"]) for m in tape["missions"]]
    ends = starts[1:] + [frame(tape["fields"])]
    sources = [args.parts / f"{i:02d}.mp4" for i in range(len(starts))]
    for source, start, end in zip(sources, starts, ends):
        check_video(source, end - start)
    work = args.output.with_suffix(".edit")
    work.mkdir(exist_ok=True)
    inputs = {"parts": {str(p): digest(p) for p in sources},
              "audio_sha256": digest(args.audio), "tape_sha256": digest(args.tape),
              "boot_frames": 180, "loading_frames": 60, "final_results_frames": 240}
    manifest = work / "inputs.json"
    if manifest.exists():
        assert json.loads(manifest.read_text()) == inputs, "Edit inputs changed"
    else:
        manifest.write_text(json.dumps(inputs, indent=2) + "\n")
    segments = []
    total = 0
    for i, (source, start, end) in enumerate(zip(sources, starts, ends)):
        keys = keyframes(source)
        assert keys[0] == 0
        if i == 0:
            first = max(k for k in keys if k <= max(0, end - start - 180))
            last = end - start
        else:
            mission = tape["missions"][i - 1]
            first = 0
            gameplay_end = frame(mission["result_field"]) - start
            target = (frame(mission["result_end"]) + 60 if i < len(starts) - 1
                      else frame(mission["result_field"]) + 240) - start
            last = max(k for k in keys if gameplay_end < k <= min(target, end - start))
            assert all(start <= frame(u["field"]) < start + last for u in mission["updates"])
        count = last - first
        target = work / f"{i:02d}.mp4"
        if not target.exists():
            temporary = work / f"{i:02d}.tmp.mp4"
            temporary.unlink(missing_ok=True)
            run("ffmpeg", "-hide_banner", "-v", "error", "-nostdin",
                "-ss", f"{first / 60:.9f}", "-i", str(source), "-map", "0:v:0",
                "-frames:v", str(count), "-c:v", "copy", "-an",
                "-movflags", "+faststart+negative_cts_offsets", "-use_editlist", "0",
                "-video_track_timescale", "60000", str(temporary))
            check_video(temporary, count)
            os.rename(temporary, target)
        check_video(target, count)
        segments.append({"part": i, "source_start_frame": start + first,
                         "source_end_frame": start + last, "output_start_frame": total,
                         "output_end_frame": total + count})
        total += count
    edit = {"segments": segments, "frames": total, "duration": total / 60,
            "source_frames": ends[-1], "removed_frames": ends[-1] - total,
            "all_gameplay_updates_retained": sum(len(m["updates"]) for m in tape["missions"]),
            "video_reencoded": False, "audio_boundary_fade_ms": 10}
    args.output.with_suffix(".edit.json").write_text(json.dumps(edit, indent=2) + "\n")
    rate = int(next(s for s in probe(args.audio)["streams"] if s["codec_type"] == "audio")["sample_rate"])
    assert rate % 60 == 0, "Source sample rate must divide exactly into 60 Hz frames"
    samples_per_frame = rate // 60
    fade = rate // 100
    branches = "".join(f"[a{i}]" for i in range(len(segments)))
    filters = [f"[0:a]asplit={len(segments)}{branches}"]
    for i, segment in enumerate(segments):
        first = segment["source_start_frame"] * samples_per_frame
        last = segment["source_end_frame"] * samples_per_frame
        filters.append(f"[a{i}]atrim=start_sample={first}:end_sample={last},asetpts=PTS-STARTPTS,"
                       f"afade=t=in:ns={fade},afade=t=out:ss={last-first-fade}:ns={fade}[s{i}]")
    filters.append("".join(f"[s{i}]" for i in range(len(segments))) + f"concat=n={len(segments)}:v=0:a=1[audio]")
    edited_audio = work / "audio.wav"
    if not edited_audio.exists():
        temporary_audio = work / "audio.tmp.wav"
        temporary_audio.unlink(missing_ok=True)
        run("ffmpeg", "-hide_banner", "-v", "error", "-nostdin", "-i", str(args.audio),
            "-filter_complex", ";".join(filters), "-map", "[audio]", "-c:a", "pcm_s16le", str(temporary_audio))
        os.rename(temporary_audio, edited_audio)
    assert abs(float(probe(edited_audio)["format"]["duration"]) - total / 60) < 1 / rate
    chapters = ";FFMETADATA1\ntitle=Tenchu Shinobi Gaisen - Ayame native campaign\n"
    description = []
    # The short title belongs to Mission 1's chapter, keeping every YouTube chapter over ten seconds.
    chapter_starts = [0] + [s["output_start_frame"] for s in segments[2:]]
    for i, (start, end) in enumerate(zip(chapter_starts, chapter_starts[1:] + [total])):
        title = f"Mission {i + 1}"
        chapters += f"\n[CHAPTER]\nTIMEBASE=1/60\nSTART={start}\nEND={end}\ntitle={title}\n"
        seconds = start // 60
        description.append(f"{seconds // 60:02d}:{seconds % 60:02d} {title}")
    metadata = work / "chapters.ffmetadata"
    metadata.write_text(chapters)
    args.output.with_suffix(".chapters.txt").write_text("\n".join(description) + "\n")
    concat = work / "concat.txt"
    concat.write_text("".join(
        f"file '{i:02d}.mp4'\nduration {(s['output_end_frame'] - s['output_start_frame']) / 60:.9f}\n"
        for i, s in enumerate(segments)))
    temporary = work / "assembled.mp4"
    temporary.unlink(missing_ok=True)
    run("ffmpeg", "-hide_banner", "-v", "warning", "-nostdin", "-f", "concat", "-safe", "1",
        "-i", str(concat), "-i", str(edited_audio), "-f", "ffmetadata", "-i", str(metadata),
        "-map", "0:v:0", "-map", "1:a:0", "-map_metadata", "2", "-map_chapters", "2",
        "-c:v", "copy", "-c:a", "aac", "-b:a", "320k", "-ar", "48000",
        "-movflags", "+faststart", "-video_track_timescale", "60000", str(temporary))
    check_video(temporary, total)
    details = probe(temporary)
    audio = next(s for s in details["streams"] if s["codec_type"] == "audio")
    assert audio["codec_name"] == "aac" and audio["sample_rate"] == "48000"
    assert audio["channels"] == 2
    assert abs(float(audio["duration"]) - total / 60) < 0.025
    packets = json.loads(subprocess.check_output([
        "ffprobe", "-v", "error", "-select_streams", "v:0", "-show_packets",
        "-show_entries", "packet=pts", "-of", "json", str(temporary),
    ]))["packets"]
    assert sorted(int(p["pts"]) for p in packets) == list(range(0, total * 1000, 1000))
    print(f"Verifying shortened cut: {total} frames / {total / 60:.3f} seconds", flush=True)
    run("ffmpeg", "-hide_banner", "-v", "error", "-xerror", "-nostdin", "-threads", "16",
        "-i", str(temporary), "-map", "0:v:0", "-map", "0:a:0", "-f", "null", "-")
    os.link(temporary, args.output)
    report = {**edit, "all_timestamps_verified": True, "full_decode_passed": True,
              "inputs": inputs, "sha256": digest(args.output), "probe": details}
    args.output.with_suffix(".verification.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"Saved {args.output}", flush=True)


if __name__ == "__main__":
    main()
