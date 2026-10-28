"""Validate release notes and assemble checked, complete download sets."""

import argparse
import datetime
import hashlib
import json
import os
import re
import shutil
import tarfile
import tomllib
import zipfile
from pathlib import Path


ARCHIVES = (
    "tenchu-linux-x86_64.tar.gz",
    "tenchu-linux-aarch64.tar.gz",
    "tenchu-windows-x86_64.zip",
    "tenchu-wasm32.tar.gz",
    "tenchu-dependency-sources.tar.gz",
)
VERSION = r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-(?:alpha|beta|rc)\.[1-9][0-9]*)?"


def release_notes(root, ref):
    version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    sections = re.split(r"^## (.+)$", (root / "CHANGELOG.md").read_text(), flags=re.M)
    headings = sections[1::2]
    if len(set(headings)) != len(headings):
        raise ValueError("Duplicate changelog headings")
    changes = dict(zip(headings, sections[2::2]))
    if "Unreleased" not in changes:
        raise ValueError("CHANGELOG.md needs an Unreleased section")
    if ref.startswith("refs/tags/"):
        tag = ref.removeprefix("refs/tags/")
        if not re.fullmatch("v" + VERSION, tag):
            raise ValueError("Use vMAJOR.MINOR.PATCH, optionally followed by -alpha.N, -beta.N or -rc.N")
        if tag != f"v{version}":
            raise ValueError(f"Tag {tag} does not match Cargo.toml version {version}")
        matching = [h for h in headings if h.startswith(f"{version} — ")]
        if len(matching) != 1:
            raise ValueError(f"Add exactly one '## {version} — YYYY-MM-DD' changelog section")
        date = datetime.date.fromisoformat(matching[0].split(" — ")[1])
        if date > datetime.date.today():
            raise ValueError("Release date is in the future")
        body = changes[matching[0]].strip()
        if not re.search(r"^- \S", body, flags=re.M):
            raise ValueError("A versioned release needs user-facing changelog entries")
    else:
        body = "Development build of the default branch. It may contain unfinished changes.\n\n"
        body += changes["Unreleased"].strip() or "No user-facing changes recorded since the last tagged release."
    artwork = json.loads((root / "artwork/release.json").read_text())
    tag = artwork["tag"]
    if not re.fullmatch(r"artwork-v[1-9][0-9]*", tag):
        raise ValueError("Invalid artwork release tag")
    repository = os.environ.get("GH_REPO", os.environ.get("GITHUB_REPOSITORY", ""))
    label = f"[{tag}](https://github.com/{repository}/releases/tag/{tag})" if re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository) else tag
    body += (
        "\n\nSupply your own Shinobi Gaisen SLPS-019.01 CUE and BIN files; original disc assets are not included. "
        "FFmpeg is needed separately for native movies/video export. "
        "The browser archive needs assets prepared with the matching native build.\n\n"
        "Download the archive for your platform and extract it completely. "
        "SHA256SUMS verifies the downloads. Dependency sources are an optional separate download.\n"
        f"\nOptional HD artwork: download `tenchu-hd.tar.zst` from the separate {label} release "
        "and place it beside the executable/launcher. Reuse it across game updates; "
        "unpacked custom packs are supported too.\n"
    )
    return body


def metadata(archive, filename):
    if archive.suffix == ".zip":
        with zipfile.ZipFile(archive) as bundle:
            paths = [n for n in bundle.namelist() if n.count("/") == 1 and n.endswith(f"/{filename}")]
            if len(paths) != 1:
                raise ValueError(f"Missing or ambiguous {filename} in {archive.name}")
            return json.loads(bundle.read(paths[0]))
    with tarfile.open(archive) as bundle:
        paths = [m for m in bundle if m.name.count("/") == 1 and m.name.endswith(f"/{filename}")]
        if len(paths) != 1:
            raise ValueError(f"Missing or ambiguous {filename} in {archive.name}")
        return json.load(bundle.extractfile(paths[0]))


def verify_library_sources(archives):
    source = archives["tenchu-dependency-sources.tar.gz"]
    components = metadata(source, "source-packages.json")["components"]
    indexed = {c["name"]: c for c in components}
    if len(indexed) != len(components):
        raise ValueError("Duplicate dependency source components")
    with tarfile.open(source) as bundle:
        members = {m.name.partition("/")[2]: m for m in bundle}
    required = set()
    for name in ("tenchu-linux-x86_64.tar.gz", "tenchu-linux-aarch64.tar.gz"):
        for library in metadata(archives[name], "bundled-libraries.json").values():
            path = library["source_archive"]
            if path is None:
                continue
            component = indexed.get(library["component"])
            if component is None or component["source"] != path:
                raise ValueError(f"Missing matching source for {library['component']} in {name}")
            member = members.get(path)
            if member is None or not (member.isfile() or member.isdir()):
                raise ValueError(f"Missing dependency source: {path}")
            for file in [component["recipe"], *component["patches"]]:
                if file not in members or not members[file].isfile():
                    raise ValueError(f"Missing dependency build file: {file}")
            required.add(component["name"])
    if required != set(indexed):
        raise ValueError(f"Sources for libraries absent from Linux bundles: {sorted(set(indexed) - required)}")


def collect(source, destination, revision, version):
    verified = []
    for name in ARCHIVES:
        paths = list(source.glob(f"*/{name}"))
        if len(paths) != 1:
            raise ValueError(f"Expected exactly one {name}, found {len(paths)}")
        archive = paths[0]
        with archive.open("rb") as file:
            digest = hashlib.file_digest(file, "sha256").hexdigest()
        checksums = (archive.parent / "SHA256SUMS").read_text().splitlines()
        if f"{digest}  {name}" not in checksums:
            raise ValueError(f"Checksum mismatch for {name}")
        info = metadata(archive, "build-info.json")
        if info["revision"] != revision or info["version"] != version:
            raise ValueError(f"Build identity mismatch for {name}")
        verified.append((archive, digest))
    verify_library_sources({archive.name: archive for archive, _ in verified})
    destination.mkdir(parents=True, exist_ok=False)
    for archive, _ in verified:
        shutil.copyfile(archive, destination / archive.name)
    (destination / "SHA256SUMS").write_text("".join(f"{digest}  {archive.name}\n" for archive, digest in verified))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    notes = commands.add_parser("notes")
    notes.add_argument("ref")
    notes.add_argument("output", type=Path)
    collect_args = commands.add_parser("collect")
    collect_args.add_argument("source", type=Path)
    collect_args.add_argument("destination", type=Path)
    collect_args.add_argument("revision")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    try:
        if args.command == "notes":
            args.output.write_text(release_notes(root, args.ref))
        else:
            version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
            collect(args.source, args.destination, args.revision, version)
    except (ValueError, KeyError, OSError) as error:
        parser.exit(1, f"Release validation failed: {error}\n")


if __name__ == "__main__":
    main()
