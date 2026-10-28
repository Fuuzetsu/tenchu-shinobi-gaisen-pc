"""Check the reviewed pack inventory and hashes; this is not a provenance classifier."""

import hashlib
import json
from pathlib import Path, PurePosixPath
import struct
import sys


def check(root):
    manifest_bytes = (root / "pack.json").read_bytes()
    manifest = json.loads(manifest_bytes)
    provenance = json.loads((root / "provenance.json").read_text())
    if hashlib.sha256(manifest_bytes).hexdigest() != provenance["manifest_sha256"]:
        raise ValueError("pack.json changed without updating provenance")
    textures = manifest.get("textures", {})
    recipes = manifest.get("recipes", {})
    if textures.keys() & recipes.keys():
        raise ValueError("Texture declared as both PNG and recipe")
    if textures.keys() | recipes.keys() != provenance["entries"].keys():
        raise ValueError("Texture provenance does not match manifest")
    files = set(textures.values()) | set(manifest.get("models", {}).values())
    files.update(s["height"] for s in manifest.get("surfaces", {}).values() if s.get("height"))
    for body in manifest.get("player_models", {}).values():
        files.update(part["file"] for part in body["parts"] if part.get("file"))
    for recipe in recipes.values():
        if isinstance(recipe.get("base"), dict):
            files.add(recipe["base"]["file"])
        for step in recipe["steps"]:
            files.update(step[key] for key in ("image", "smooth") if key in step)
            source = step.get("mask", {}).get("source")
            if isinstance(source, dict):
                files.add(source["file"])
    if files != provenance["files"].keys():
        raise ValueError("Referenced files do not match provenance")
    total = 0
    for name in sorted(files):
        path = PurePosixPath(name)
        if path.is_absolute() or any(p in ("", ".", "..") for p in name.split("/")) or "\\" in name:
            raise ValueError(f"Invalid pack path: {name}")
        if (root / name).is_symlink() or not (root / name).resolve().is_relative_to(root.resolve()):
            raise ValueError(f"Pack path escapes: {name}")
        data = (root / name).read_bytes()
        if hashlib.sha256(data).hexdigest() != provenance["files"][name]:
            raise ValueError(f"File changed without provenance update: {name}")
        if path.suffix == ".png":
            if data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
                raise ValueError(f"Invalid PNG: {name}")
            if not all(0 < size <= 1024 for size in struct.unpack(">II", data[16:24])):
                raise ValueError(f"Oversized texture: {name}")
        elif path.suffix != ".obj":
            raise ValueError(f"Unexpected shipped format: {name}")
        total += len(data)
    actual = {p.relative_to(root).as_posix() for p in root.rglob("*") if p.is_file()}
    if actual != files | {"pack.json", "provenance.json", "README.md", "LICENSE"}:
        raise ValueError("Unreferenced or missing files in pack")
    print(f"HD pack: {len(textures)} PNG entries, {len(recipes)} recipes, {len(files)} inputs, {total / 1048576:.1f} MiB")


if __name__ == "__main__":
    check(Path(sys.argv[1]))
