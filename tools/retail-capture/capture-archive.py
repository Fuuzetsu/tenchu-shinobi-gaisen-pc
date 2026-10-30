#!/usr/bin/env python3
"""Capture a proven receipt chain from its menu/root state, including item setup."""

import argparse
import hashlib
import itertools
import json
import os
from pathlib import Path
import struct
import subprocess


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def ram_bytes(path):
    data = path.read_bytes()
    if data[:8] != b"ONKRAM01" or len(data) != 80 + 2 * 1024 * 1024:
        raise ValueError(f"Invalid RAM snapshot: {path}")
    return data[80:]


def prepare(campaign, tip):
    chain, seen = [], set()
    while True:
        if len(tip) != 64 or any(c not in "0123456789abcdef" for c in tip) or tip in seen:
            raise ValueError("Invalid or cyclic receipt chain")
        seen.add(tip)
        root = campaign / "receipts" / tip
        receipt = json.loads((root / "receipt.json").read_text())
        data = (root / "input.bin").read_bytes()
        if hashlib.sha256(data).hexdigest() != receipt["input_sha256"]:
            raise ValueError(f"Input hash mismatch: {tip}")
        if data[:8] != b"SCNPAD01" or len(data) < 12:
            raise ValueError(f"Unsupported controller tape: {tip}")
        count, = struct.unpack_from("<I", data, 8)
        if count != receipt["fields"] or len(data) != 12 + count * 2:
            raise ValueError(f"Controller tape length mismatch: {tip}")
        if hashlib.sha256(ram_bytes(root / "endpoint.ram")).hexdigest() != receipt["endpoint"]["ram_sha256"]:
            raise ValueError(f"Reference RAM hash mismatch: {tip}")
        chain.append((root, receipt, [pad for pad, in struct.iter_unpack("<H", data[12:])]))
        origin = receipt["origin"]
        if origin["kind"] != "predecessor":
            break
        tip = origin["receipt"]
    chain.reverse()
    state = chain[0][0] / "root.State"
    if not state.is_file() or sha256(state) != chain[0][1]["start_state_sha256"]:
        raise ValueError("Missing or mismatched archived root savestate")
    for (parent, before, _), (_, after, _) in itertools.pairwise(chain):
        if before["endpoint"]["state_sha256"] != after["start_state_sha256"]:
            raise ValueError(f"Broken savestate chain after {parent.name}")
    return chain, state


def read_segments(path):
    pads = []
    for line in path.read_text().splitlines():
        fields = line.partition("#")[0].split()
        if not fields:
            continue
        if len(fields) != 2:
            raise ValueError(f"Expected field count and hex buttons: {line}")
        count, pad = int(fields[0]), int(fields[1], 16)
        if not 0 < count <= 1_000_000 or not 0 <= pad <= 0xffff or len(pads) + count > 1_000_000:
            raise ValueError("Invalid or excessive input segment")
        pads.extend([pad] * count)
    if not pads:
        raise ValueError("Scenario has no inputs")
    return pads


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path, help="Pinned runtime/dependencies archive")
    parser.add_argument("output", type=Path, help="New capture directory")
    parser.add_argument("--campaign", type=Path, help="Receipt store (defaults to ARCHIVE/campaign)")
    parser.add_argument("--tip", help="Proven receipt (defaults to ARCHIVE/result.json tip)")
    parser.add_argument("--tail", type=Path, help="Append custom console-field inputs after the proven receipt")
    args = parser.parse_args()
    archive, output = args.archive.resolve(), args.output.resolve()
    if any(Path(str(output) + suffix).exists() for suffix in ("", ".runtime", ".json", ".segments")):
        parser.error("Output already exists")
    campaign = (args.campaign or archive / "campaign").resolve()
    tip = args.tip or json.loads((archive / "result.json").read_text())["tip"]
    chain, state = prepare(campaign, tip)
    output.parent.mkdir(parents=True, exist_ok=True)
    pads = list(itertools.chain.from_iterable(c[2] for c in chain))
    if args.tail:
        pads.extend(read_segments(args.tail))
    segments = Path(str(output) + ".segments")
    with segments.open("x") as stream:
        stream.write("# Exact archived SCNPAD01 inputs from root/menu through the proven tip.\n")
        for pad, repeats in itertools.groupby(pads):
            stream.write(f"{sum(1 for _ in repeats)} {pad:04x}\n")
    checkpoints, field = [], 0
    for root, receipt, inputs in chain:
        field += len(inputs)
        checkpoints.append(dict(field=field, receipt=root.name,
                                target=receipt["target"], ram_sha256=receipt["endpoint"]["ram_sha256"]))
    manifest = dict(version=1, archive=str(archive), campaign=str(campaign), tip=tip,
                    state_sha256=sha256(state), input_sha256=sha256(segments), checkpoints=checkpoints)
    if args.tail:
        manifest["scenario"] = dict(input_sha256=sha256(args.tail), start_field=field, end_field=len(pads))
    command = ["bash", str(Path(__file__).with_name("capture.sh")), str(archive),
               str(state), str(segments), str(output)]
    setup = chain[0][0] / "item-setup.json"
    if setup.exists():
        command.append(str(setup))
        manifest["menu_item_setup_sha256"] = sha256(setup)
    witness = Path(str(output) + ".json")
    witness.write_text(json.dumps(manifest, indent=2) + "\n")
    fields = sorted({c["field"] for c in checkpoints} | {len(pads)})
    env = dict(os.environ, TENCHU_SETUP_CHECKPOINTS=",".join(map(str, fields)))
    subprocess.run(command, env=env, check=True)
    for checkpoint, (root, _, _) in zip(checkpoints, chain, strict=True):
        actual = ram_bytes(output / f'{checkpoint["field"]:06}.ram')
        expected = ram_bytes(root / "endpoint.ram")
        if actual != expected:
            raise ValueError(f'Reference replay differs at field {checkpoint["field"]}: {root.name}')
        checkpoint["matched_all_ram"] = True
    manifest["updates_sha256"] = sha256(output / "updates.ram.gz")
    witness.write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"PASS: all RAM matches at {len(checkpoints)} archived checkpoints; {len(pads)} console fields captured")


if __name__ == "__main__":
    main()
