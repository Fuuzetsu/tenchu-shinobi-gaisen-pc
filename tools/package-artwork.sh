#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 2 ] || [ "$#" -gt 3 ]; then
    echo "Usage: ./dev.sh bash tools/package-artwork.sh PACK_DIRECTORY OUTPUT_DIRECTORY [ARCHIVE_NAME]" >&2
    exit 2
fi

project=$(cd -- "$(dirname -- "$0")/.." && pwd)
archive=${3:-tenchu-hd.tar.zst}
if [[ ! "$archive" =~ ^[a-z0-9][a-z0-9._-]*\.tar\.zst$ ]]; then
    echo "Archive name must be a simple .tar.zst filename" >&2
    exit 1
fi
pack=$(cd -- "$1" && pwd)
mkdir -p -- "$2"
output=$(cd -- "$2" && pwd)
if [[ "$output/" == "$pack/"* ]]; then
    echo "Output must be outside the pack directory" >&2
    exit 1
fi
if [ -e "$output/$archive" ]; then
    echo "Output already contains $archive; use a new output directory" >&2
    exit 1
fi

python3 "$project/packaging/check-artwork.py" "$pack"
temporary=$(mktemp "$output/.tenchu-hd.XXXXXXXX")
trap 'rm -f -- "$temporary"' EXIT
tar --sort=name --mtime=@1 --owner=0 --group=0 --numeric-owner --format=ustar \
    -cf - -C "$pack" . | zstd -10 -T1 --check -q -o "$temporary" -f
mv -- "$temporary" "$output/$archive"
(cd -- "$output" && sha256sum "$archive" > SHA256SUMS)
cat "$output/SHA256SUMS"
