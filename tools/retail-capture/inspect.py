"""Inspect exact MAIN update observations; optionally compare two captures."""
import argparse
from collections import Counter
import gzip
import hashlib
import json
from pathlib import Path
import struct

RAM_SIZE = 0x200000


def records(path):
    ram = bytearray(RAM_SIZE)
    initialized = False
    with gzip.open(path, "rb") as source:
        magic = source.read(8)
        if magic not in (b"TNRUN001", b"TNRUN002"):
            raise ValueError("Unsupported observation format")
        while header := source.read(56):
            if len(header) != 56:
                raise ValueError("Incomplete observation header")
            size, pc, field, skip, cycle, clock, stage, pad, seed, epoch, caller, v0, a0 = struct.unpack("<4IQ8I", header)
            if not 56 <= size <= RAM_SIZE + 65592:
                raise ValueError("Invalid observation size")
            payload = source.read(size - 56)
            if len(payload) != size - 56:
                raise ValueError("Incomplete observation payload")
            snapshot = size > 56
            if snapshot:
                if magic == b"TNRUN001":
                    if len(payload) != RAM_SIZE:
                        raise ValueError("Invalid full RAM snapshot")
                    ram[:] = payload
                    initialized = True
                else:
                    count, = struct.unpack_from("<I", payload)
                    offset, previous_end = 4, 0
                    for _ in range(count):
                        address, length = struct.unpack_from("<II", payload, offset)
                        offset += 8
                        if address < previous_end or address + length > RAM_SIZE or not length or offset + length > len(payload):
                            raise ValueError("Invalid RAM patch")
                        if not initialized and (count != 1 or address != 0 or length != RAM_SIZE):
                            raise ValueError("First snapshot must cover all RAM")
                        ram[address:address + length] = payload[offset:offset + length]
                        offset += length
                        previous_end = address + length
                    if offset != len(payload) or not initialized and not count:
                        raise ValueError("Invalid RAM patch length")
                    initialized = True
                if struct.unpack_from("<I", ram, 0x976b0)[0] != clock or struct.unpack_from("<I", ram, 0xc79e0)[0] != seed:
                    raise ValueError("Snapshot disagrees with event header")
            row = dict(pc=pc, field=field, skip=skip, cycle=cycle, clock=clock, stage=stage,
                       pad=pad, seed=seed, epoch=epoch, caller=caller, v0=v0, a0=a0)
            yield row, ram if snapshot else None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("trace", type=Path)
    parser.add_argument("--compare", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    other = iter(records(args.compare)) if args.compare else None
    phases, skips, stages = Counter(), Counter(), Counter()
    snapshots, count, first, last, previous_cycle = 0, 0, None, None, -1
    digest = hashlib.sha256()
    for row, ram in records(args.trace):
        count += 1
        if row["cycle"] < previous_cycle:
            raise ValueError("Observation time moved backwards")
        previous_cycle = row["cycle"]
        if other:
            expected, expected_ram = next(other)
            if row != expected or ram != expected_ram:
                raise ValueError(f"Capture differs at record {count}: {row} / {expected}")
        digest.update(json.dumps(row, sort_keys=True).encode())
        if ram is not None:
            snapshots += 1
            digest.update(ram)
        phases[f'{row["pc"]:08x}'] += 1
        if row["pc"] == 0x800170c8:
            skips[str(row["skip"])] += 1
            stages[str(row["stage"])] += 1
        first = row if first is None else first
        last = row
    if other and next(other, None) is not None:
        raise ValueError("Comparison contains more observations")
    result = dict(records=count, snapshots=snapshots, phases=phases, skips=skips,
                  stages=stages, first=first, last=last, decoded_sha256=digest.hexdigest(),
                  compared_equal=args.compare is not None)
    if args.output:
        args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
