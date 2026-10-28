"""Preserve upstream license/copyright texts for the bundled native components."""

import json
import re
import sys
import tarfile
from pathlib import Path, PurePosixPath


def notice(path):
    return re.match(r"^(copying|copyright|licen[sc]e|notice|authors)([._-].*)?$", path.name, re.I) or any(
        part.lower() in ("licenses", "licences") for part in path.parts[:-1]
    )


manifest, destination = map(Path, sys.argv[1:])
for component in json.loads(manifest.read_text()):
    print(f"Collecting notices for {component['name']}", flush=True)
    source = Path(component["source"])
    output = destination / component["name"]
    output.mkdir(parents=True)
    count = 0

    def save(path, data):
        global count
        # Flatten paths to retain nested notices without extracting upstream archives.
        target = output / "__".join(path.parts)
        target.write_bytes(data)
        count += 1

    if source.is_dir():
        for path in sorted(source.rglob("*")):
            if path.is_file() and notice(path):
                save(path.relative_to(source), path.read_bytes())
    else:
        with tarfile.open(source, mode="r:*") as archive:
            for member in archive:
                path = PurePosixPath(member.name)
                if member.isfile() and notice(path):
                    save(path, archive.extractfile(member).read())
    if count == 0:
        raise RuntimeError(f"No license notices found in {component['name']}")
