#!/usr/bin/env python3
"""Resume a verified native campaign export and mux its original lossless audio."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
from concurrent.futures import ThreadPoolExecutor


def run(*args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def frame(field):
    return (field * 2097152 * 60 + 125703416) // 125703417


def probe(path):
    return json.loads(subprocess.check_output([
        "ffprobe", "-v", "error", "-show_streams", "-show_format", "-show_chapters",
        "-of", "json", str(path),
    ]))


def check_video(path, count):
    stream = next(s for s in probe(path)["streams"] if s["codec_type"] == "video")
    assert (stream["width"], stream["height"]) == (3840, 2160), path
    assert stream["avg_frame_rate"] == "60/1", path
    assert int(stream["nb_frames"]) == count, (path, stream["nb_frames"], count)
    assert stream["pix_fmt"] == "yuv420p", path
    assert stream["color_space"] == "bt709", path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("disc", type=Path)
    parser.add_argument("tape", type=Path)
    parser.add_argument("audio", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--binary", type=Path, default=Path("target/release/tenchu-pc"))
    parser.add_argument("--asset-pack", type=Path, required=True)
    parser.add_argument("--jobs", type=int, choices=range(1, 5), default=1,
                        help="Independent chapter renders; each needs its own GPU memory")
    args = parser.parse_args()
    for key, value in vars(args).items():
        if isinstance(value, Path):
            setattr(args, key, value.resolve())
    if args.output.exists():
        raise SystemExit(f"Refusing to overwrite {args.output}")
    parts = args.output.with_suffix(".parts")
    parts.mkdir(exist_ok=True)
    source = {
        "binary": str(args.binary), "binary_sha256": digest(args.binary),
        "disc": str(args.disc), "disc_descriptor_sha256": digest(args.disc),
        "tape": str(args.tape), "tape_sha256": digest(args.tape),
        "audio": str(args.audio), "audio_sha256": digest(args.audio),
        "asset_pack": str(args.asset_pack),
        "artwork": {str(p.relative_to(args.asset_pack)): digest(p)
                    for p in sorted(args.asset_pack.rglob("*"))
                    if p.is_file() and p.suffix in (".json", ".png", ".bin", ".obj")},
        "graphics": "Showcase; original weather; fixed sky; full artwork; sharp text; visual interpolation",
        "presentation": "Original field timing. Native title/mission/results screens replace boot and loading intervals. All gameplay and in-engine cutscenes replay the validated tape.",
        "audio_source": "Lossless console audio from the identical continuous boot tape; no speed change.",
    }
    manifest = parts / "inputs.json"
    if manifest.exists():
        assert json.loads(manifest.read_text()) == source, "Capture inputs changed; use a new output"
    else:
        manifest.write_text(json.dumps(source, indent=2) + "\n")
    tape = json.loads(args.tape.read_text())
    starts = [0] + [frame(m["updates"][0]["field"]) for m in tape["missions"]]
    total = frame(tape["fields"])
    titles = ["Boot and mission selection"] + [f"Mission {i + 1}" for i in range(len(tape["missions"]))]
    chapters = ";FFMETADATA1\ntitle=Tenchu Shinobi Gaisen - Ayame full campaign - Native PC\n"
    youtube = []
    clips = []
    applied_settings = None
    env = dict(os.environ, TENCHU_ASSET_PACK=str(args.asset_pack))

    def render(part):
        i, (start, end, title) = part
        path = parts / f"{i:02d}.mp4"
        if not path.exists():
            print(f"Rendering {title}: frames {start}–{end}", flush=True)
            with (parts / f"{i:02d}.log").open("w") as log:
                run(str(args.binary), "record-retail", str(args.disc), str(args.tape), str(path),
                    str(start), str(end), env=env, stdout=log, stderr=subprocess.STDOUT)
        check_video(path, end - start)
        print(f"Verified {title}: {end - start} frames", flush=True)

    ranges = list(zip(starts, starts[1:] + [total], titles))
    with ThreadPoolExecutor(max_workers=args.jobs) as workers:
        list(workers.map(render, enumerate(ranges)))
    for i, (start, end, title) in enumerate(ranges):
        path = parts / f"{i:02d}.mp4"
        clips.append(path)
        seconds = start // 60
        youtube.append(f"{seconds // 60:02d}:{seconds % 60:02d} {title}")
        chapters += f"\n[CHAPTER]\nTIMEBASE=1/60\nSTART={start}\nEND={end}\ntitle={title}\n"
        configured = json.loads(path.with_suffix(".settings.json").read_text())
        if applied_settings is None:
            applied_settings = configured
        else:
            assert configured == applied_settings, f"Renderer settings changed in {path}"
    metadata = parts / "chapters.ffmetadata"
    metadata.write_text(chapters)
    args.output.with_suffix(".chapters.txt").write_text("\n".join(youtube) + "\n")
    # These generated relative names contain no quoting-sensitive characters.
    concat = parts / "concat.txt"
    # MP4 chapter tracks round to milliseconds; use video frame counts for joins.
    concat.write_text("".join(
        f"file '{p.name}'\nduration {(end - start) / 60:.9f}\n"
        for p, start, end in zip(clips, starts, starts[1:] + [total])))
    temporary = parts / "assembled.mp4"
    temporary.unlink(missing_ok=True)
    run("ffmpeg", "-hide_banner", "-loglevel", "warning", "-nostdin",
        "-f", "concat", "-safe", "1", "-i", str(concat), "-i", str(args.audio),
        "-f", "ffmetadata", "-i", str(metadata), "-map", "0:v:0", "-map", "1:a:0",
        "-map_metadata", "2", "-map_chapters", "2", "-c:v", "copy",
        "-af", f"apad,atrim=end={total / 60:.9f}", "-c:a", "aac", "-b:a", "320k",
        "-ar", "48000", "-movflags", "+faststart", "-video_track_timescale", "60000",
        str(temporary))
    check_video(temporary, total)
    details = probe(temporary)
    audio = next(s for s in details["streams"] if s["codec_type"] == "audio")
    assert audio["codec_name"] == "aac" and audio["sample_rate"] == "48000"
    assert abs(float(audio["duration"]) - total / 60) < 0.025
    packets = subprocess.check_output([
        "ffprobe", "-v", "error", "-select_streams", "v:0", "-show_packets",
        "-show_entries", "packet=pts", "-of", "json", str(temporary),
    ])
    pts = sorted(int(p["pts"]) for p in json.loads(packets)["packets"])
    assert pts == list(range(0, total * 1000, 1000)), "Missing, duplicate or shifted video timestamps"
    print("Checking every decoded video and audio frame", flush=True)
    run("ffmpeg", "-hide_banner", "-v", "error", "-xerror", "-nostdin", "-threads", "16",
        "-i", str(temporary), "-map", "0:v:0", "-map", "0:a:0", "-f", "null", "-")
    os.link(temporary, args.output)
    report = {"frames": total, "duration": total / 60, "all_timestamps_verified": True,
              "full_decode_passed": True, "sha256": digest(args.output), "probe": details,
              "inputs": source, "applied_settings": applied_settings}
    args.output.with_suffix(".verification.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"Saved {args.output}: {total} frames / 4K60 / synchronized AAC audio", flush=True)


if __name__ == "__main__":
    main()
