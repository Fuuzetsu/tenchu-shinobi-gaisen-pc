"""Check a 4K/60 demo export, every timestamp, chapters and full decoding."""

import argparse
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import struct
import subprocess


def require(condition, message):
    if not condition:
        raise ValueError(message)


def probe(video, *args):
    return json.loads(subprocess.check_output(
        ["ffprobe", "-v", "error", *args, "-of", "json", str(video)], text=True
    ))


def fast_start(video):
    atoms = []
    length = video.stat().st_size
    with video.open("rb") as source:
        while source.tell() < length:
            offset = source.tell()
            size, kind = struct.unpack(">I4s", source.read(8))
            if size == 1:
                size = struct.unpack(">Q", source.read(8))[0]
            elif size == 0:
                size = length - offset
            require(8 <= size <= length - offset, "Invalid MP4 atom size")
            atoms.append(kind)
            source.seek(offset + size)
    return atoms.index(b"moov") < atoms.index(b"mdat")


def verify(video, catalog=None, youtube=False, sound=False):
    metadata = probe(video, "-show_streams", "-show_chapters")
    streams = metadata["streams"]
    videos = [stream for stream in streams if stream["codec_type"] == "video"]
    require(len(videos) == 1, "Expected one video stream")
    audio = [s for s in streams if s["codec_type"] == "audio"]
    require(len(audio) == int(sound), "Unexpected number of audio streams; use --sound for game audio")
    stream = videos[0]
    expected = {
        "width": 3840, "height": 2160, "codec_name": "h264", "profile": "High",
        "pix_fmt": "yuv420p", "field_order": "progressive", "sample_aspect_ratio": "1:1",
        "r_frame_rate": "60/1", "avg_frame_rate": "60/1", "color_range": "tv",
        "color_space": "bt709", "color_transfer": "bt709", "color_primaries": "bt709",
    }
    for key, value in expected.items():
        require(stream.get(key) == value, f"{key}: expected {value}, got {stream.get(key)}")
    frames = int(stream["nb_frames"])
    duration = Fraction(stream["duration"])
    if sound:
        require(audio[0]["codec_name"] == "aac" and audio[0]["sample_rate"] == "48000"
                and audio[0]["channels"] == 2, "Expected 48 kHz stereo AAC")
        require(abs(Fraction(audio[0]["duration"]) - duration) < Fraction(1, 60),
                "Audio duration differs from the video")
        require(Fraction(audio[0].get("start_time", "0")) == 0, "Audio starts late")
    require(duration * 60 == frames, "Duration does not match frame count")
    packets = probe(video, "-select_streams", "v:0", "-show_packets",
                    "-show_entries", "packet=pts,duration")["packets"]
    tick = Fraction(stream["time_base"])
    indices = sorted(Fraction(p["pts"]) * tick * 60 for p in packets)
    require(indices == list(range(frames)), "Missing, duplicate or offset presentation timestamp")
    require(all(Fraction(p["duration"]) * tick == Fraction(1, 60) for p in packets),
            "A packet does not span exactly one 60 Hz frame")
    chapters = metadata["chapters"]
    end = Fraction(0)
    for chapter in chapters:
        start, finish = map(Fraction, (chapter["start_time"], chapter["end_time"]))
        require(start == end and finish > start, "Chapters overlap or have gaps")
        if youtube:
            require(finish - start >= 10, "YouTube chapters must last at least ten seconds")
        end = finish
    require(chapters and end == duration, "Chapters do not cover the video")
    if youtube:
        require(len(chapters) >= 3, "YouTube needs at least three chapters")
    if catalog:
        clips = json.loads(catalog.read_text())
        require(len(clips) == len(chapters), "Catalog and chapter counts differ")
        for clip, chapter in zip(clips, chapters):
            require(clip["title"] == chapter["tags"]["title"], "Chapter title differs from catalog")
            length = Fraction(chapter["end_time"]) - Fraction(chapter["start_time"])
            require(length == clip["seconds"], "Chapter duration differs from catalog")
    require(fast_start(video), "MP4 metadata must precede video data for fast start")
    print(f"Verified {frames:,} consecutive timestamps and {len(chapters)} chapters; decoding…",
          flush=True)
    decoded = subprocess.run(
        ["ffmpeg", "-nostdin", "-v", "error", "-xerror", "-threads", "8", "-i", str(video),
         "-map", "0:v:0", "-map", "0:a?", "-f", "null", "-"], capture_output=True, text=True, check=True
    )
    require(not decoded.stderr.strip(), decoded.stderr)
    with video.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    return {
        "file": video.name, "duration_seconds": float(duration), "frames": frames,
        "size_bytes": video.stat().st_size, "sha256": digest, "video": expected,
        "audio_streams": len(audio), "fast_start": True, "complete_60_hz_timestamps": True,
        "full_decode_errors": 0, "youtube_chapters": youtube,
        "chapters": [{"start": float(c["start_time"]), "end": float(c["end_time"]),
                      "title": c["tags"]["title"]} for c in chapters],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("video", type=Path)
    parser.add_argument("--catalog", type=Path)
    parser.add_argument("--youtube", action="store_true")
    parser.add_argument("--sound", action="store_true")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    result = verify(args.video, args.catalog, args.youtube, args.sound)
    output = args.output or args.video.with_suffix(".validation.json")
    output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"Passed: {args.video} ({result['frames']:,} frames). Wrote {output}.")


if __name__ == "__main__":
    main()
