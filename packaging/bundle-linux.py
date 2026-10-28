import re
import json
import shutil
import subprocess
import sys
from pathlib import Path

binary, destination, manifest, *libraries = map(Path, sys.argv[1:])
components = json.loads(manifest.read_text())
lib = destination / "lib"
lib.mkdir(parents=True)
(destination / "libexec").mkdir()
program = destination / "libexec/tenchu-pc"
shutil.copyfile(binary, program)
program.chmod(0o755)

roots = [binary]
for directory in libraries:
    roots.extend(sorted((directory / "lib").glob("lib*.so*")))

dependencies = {}
for root in roots:
    with root.open("rb") as file:
        if file.read(4) != b"\x7fELF":
            continue
    resolved = subprocess.check_output(["ldd", str(root)], text=True)
    if "not found" in resolved:
        raise RuntimeError(f"Unresolved dependency for {root}:\n{resolved}")
    paths = [root] if root != binary else []
    paths.extend(Path(path) for path in re.findall(r"(?:=>\s+|^\s+)(/\S+)", resolved, re.M))
    for path in paths:
        previous = dependencies.setdefault(path.name, path.resolve())
        if previous != path.resolve() and previous.read_bytes() != path.read_bytes():
            raise RuntimeError(f"Conflicting libraries named {path.name}: {previous}, {path}")

uncovered = {
    path for path in dependencies.values()
    if not any(path.is_relative_to(output) for c in components for output in c["outputs"])
}
if uncovered:
    raise RuntimeError(f"No dependency source/notice entries for {sorted(map(str, uncovered))}")

for name, path in sorted(dependencies.items()):
    copied = lib / name
    shutil.copyfile(path, copied)
    copied.chmod(0o755)
    if not name.startswith("ld-linux-"):
        subprocess.run(["patchelf", "--set-rpath", "$ORIGIN", str(copied)], check=True)

inventory = {}
for name, path in sorted(dependencies.items()):
    component = next(c for c in components if any(path.is_relative_to(output) for output in c["outputs"]))
    inventory[f"lib/{name}"] = {
        "component": component["name"],
        "source_archive": component["source_archive"],
    }
(destination / "bundled-libraries.json").write_text(json.dumps(inventory, indent=2) + "\n")

subprocess.run(["patchelf", "--set-rpath", "$ORIGIN/../lib", str(program)], check=True)
