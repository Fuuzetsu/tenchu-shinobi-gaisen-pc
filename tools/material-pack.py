"""Package authored color/height images against an export-assets catalogue."""

import argparse
import json
from pathlib import Path

from PIL import Image, ImageStat


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("export", type=Path)
    parser.add_argument("recipe", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--size", type=int, default=512)
    args = parser.parse_args()
    if not 16 <= args.size <= 1024:
        parser.error("--size must be between 16 and 1024")
    if args.output.exists() and any(args.output.iterdir()):
        parser.error("output must be empty; use a new directory to preserve edits")

    catalogue = json.loads((args.export / "catalog.json").read_text())
    recipe = json.loads(args.recipe.read_text())
    pack = {"version": 1, "textures": {}, "surfaces": {}}
    prepared = []
    pixels = 0
    for identity, entry in recipe.items():
        if identity not in catalogue["textures"]:
            parser.error(f"unknown texture identity: {identity}")
        with Image.open(args.export / "textures" / f"{identity}.png") as source:
            original = source.convert("RGBA")
        longest = entry.get("size", args.size)
        if not isinstance(longest, int) or not 16 <= longest <= 1024:
            parser.error(f"size for {identity} must be between 16 and 1024")
        size = tuple(max(1, round(n * longest / max(original.size))) for n in original.size)
        with Image.open(args.recipe.parent / entry["color"]) as source:
            color = source.convert("RGBA").resize(size, Image.Resampling.LANCZOS)
        alpha = original.getchannel("A").resize(size, Image.Resampling.NEAREST)
        mode = entry.get("alpha", "original")
        if mode == "original":
            visible = sum(mask != 0 for mask in alpha.get_flattened_data())
            missing = sum(mask != 0 and paint < 128 for mask, paint in zip(
                alpha.get_flattened_data(), color.getchannel("A").get_flattened_data()))
            if missing > visible * 0.01:
                parser.error(f"{identity}: authored color misses {missing / visible:.1%} "
                             "of the original mask; align the artwork or extend its color bleed")
        elif mode == "authored":
            classes = set(alpha.get_flattened_data()) - {0}
            if len(classes) != 1:
                parser.error(f"{identity}: authored alpha requires one original nonzero blend class")
            authored = color.getchannel("A")
            if alpha.getextrema()[0] == 0 and authored.getextrema()[0] != 0:
                parser.error(f"{identity}: authored cutout must contain transparent pixels")
            original_class = classes.pop()
            alpha = authored.point(lambda value: original_class if value >= 128 else 0)
        else:
            parser.error(f"{identity}: alpha must be original or authored")
        color.putalpha(alpha)
        name = f"textures/{identity}.png"
        prepared.append((name, color))
        pixels += size[0] * size[1]
        pack["textures"][identity] = name
        strength = float(entry.get("strength", 0.25))
        if not 0 <= strength <= 2:
            parser.error(f"height strength for {identity} must be between 0 and 2")
        rule = {"strength": strength}
        if "height" in entry:
            with Image.open(args.recipe.parent / entry["height"]) as source:
                height = source.convert("RGB").getchannel("R").resize(size, Image.Resampling.LANCZOS)
            name = f"heightmaps/{identity}.png"
            prepared.append((name, height))
            pixels += size[0] * size[1]
            rule["height"] = name
        pack["surfaces"][identity] = rule
        before = ImageStat.Stat(original.convert("RGB")).mean
        after = ImageStat.Stat(color.convert("RGB")).mean
        print(entry.get("name", identity), size, "mean RGB:",
              tuple(round(n) for n in before), "->", tuple(round(n) for n in after))
    if pixels > 16 * 1024 * 1024:
        parser.error("pack exceeds the runtime's 16 million decoded pixel budget")

    args.output.mkdir(parents=True, exist_ok=True)
    for name, image in prepared:
        path = args.output / name
        path.parent.mkdir(parents=True, exist_ok=True)
        image.save(path)
    (args.output / "pack.json").write_text(json.dumps(pack, indent=2) + "\n")
    print(f"Saved {len(recipe)} materials, {pixels:,} decoded pixels to {args.output}")


if __name__ == "__main__":
    main()
