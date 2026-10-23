"""Create the portable outline font from the pinned Noto Sans JP variable TTF."""

import argparse
import hashlib
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    digest = hashlib.sha256(args.source.read_bytes()).hexdigest()
    if digest != "c2f3b4d463500a2ddcd3849cded1fceeb9fd6d1c32e6cbecd568453ba50fc68f":
        parser.error("source does not match the pinned Noto Sans JP font")
    codepoints = set(range(0x20, 0x250)) | set(range(0x2000, 0x2070))
    for code in range(0x8140, 0xF000):
        try:
            text = code.to_bytes(2, "big").decode("shift_jis")
        except UnicodeDecodeError:
            continue
        if len(text) == 1:
            codepoints.add(ord(text))
    codepoints.update(range(0xFF61, 0xFFA0))
    font = TTFont(args.source, recalcTimestamp=False)
    font = instantiateVariableFont(font, {"wght": 400}, inplace=True)
    options = subset.Options()
    options.name_IDs = [0, 1, 2, 3, 4, 5, 6, 13, 14]
    options.name_languages = [0x409]
    options.recalc_timestamp = False
    subsetter = subset.Subsetter(options=options)
    subsetter.populate(unicodes=codepoints)
    subsetter.subset(font)
    names = {1: "Tenchu UI Sans", 2: "Regular", 3: "Tenchu UI Sans Regular", 4: "Tenchu UI Sans Regular",
             6: "TenchuUISans-Regular"}
    for name, value in names.items():
        font["name"].setName(value, name, 3, 1, 0x409)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    font.save(args.output)
    print(f"Saved {len(font.getBestCmap())} Unicode glyph mappings, {args.output.stat().st_size:,} bytes")


if __name__ == "__main__":
    main()
