#!/usr/bin/env python3
"""Measure soundtrack correlation and lag against the matching lossless run."""

import argparse
from array import array
import json
import math
import subprocess

RATE = 12000
MARGIN = RATE // 5


def samples(path, time):
    data = subprocess.check_output([
        "ffmpeg", "-hide_banner", "-v", "error", "-ss", str(time - 0.2), "-i", path,
        "-t", "1.4", "-map", "0:a:0", "-ac", "1", "-ar", str(RATE),
        "-f", "f32le", "-",
    ])
    result = array("f")
    result.frombytes(data)
    if len(result) < RATE + 2 * MARGIN:
        raise ValueError(f"Truncated audio at {time}s")
    return result


def compare(reference, encoded, time):
    a = samples(reference, time)
    b = samples(encoded, time)

    def correlation(lag, stride):
        indices = range(MARGIN, MARGIN + RATE, stride)
        aa = sum(a[i] * a[i] for i in indices)
        bb = sum(b[i + lag] * b[i + lag] for i in indices)
        ab = sum(a[i] * b[i + lag] for i in indices)
        return ab / math.sqrt(aa * bb) if aa * bb > 1e-12 else 0.0

    coarse = max(range(-1800, 1801, 12), key=lambda lag: correlation(lag, 12))
    lag = max(range(coarse - 12, coarse + 13), key=lambda lag: correlation(lag, 1))
    return {"time": time, "lag_samples_at_12khz": lag, "lag_ms": lag * 1000 / RATE,
            "correlation": correlation(lag, 1), "zero_lag_correlation": correlation(0, 1)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference")
    parser.add_argument("encoded")
    parser.add_argument("times", type=float, nargs="+")
    args = parser.parse_args()
    results = [compare(args.reference, args.encoded, time) for time in args.times]
    print(json.dumps(results, indent=2))
    if any(abs(row["lag_samples_at_12khz"]) > 1 or row["zero_lag_correlation"] < 0.995 for row in results):
        raise SystemExit("Soundtrack differs or is misaligned")


if __name__ == "__main__":
    main()
