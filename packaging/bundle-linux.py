import re
import shutil
import subprocess
import sys
from pathlib import Path

binary, destination, *libraries = map(Path, sys.argv[1:])
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

for name, path in sorted(dependencies.items()):
    copied = lib / name
    shutil.copyfile(path, copied)
    copied.chmod(0o755)
    if not name.startswith("ld-linux-"):
        subprocess.run(["patchelf", "--set-rpath", "$ORIGIN", str(copied)], check=True)

subprocess.run(["patchelf", "--set-rpath", "$ORIGIN/../lib", str(program)], check=True)
