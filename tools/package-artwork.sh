#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
    echo "Usage: ./dev.sh bash tools/package-artwork.sh PACK_DIRECTORY OUTPUT_DIRECTORY" >&2
    exit 2
fi

project=$(cd -- "$(dirname -- "$0")/.." && pwd)
pack=$(cd -- "$1" && pwd)
mkdir -p -- "$2"
output=$(cd -- "$2" && pwd)
if [[ "$output/" == "$pack/"* ]]; then
    echo "Output must be outside the pack directory" >&2
    exit 1
fi
if [ -e "$output/tenchu-hd.tar.zst" ]; then
    echo "Output already contains tenchu-hd.tar.zst; use a new output directory" >&2
    exit 1
fi

python3 "$project/packaging/check-artwork.py" "$pack"
temporary=$(mktemp "$output/.tenchu-hd.XXXXXXXX")
trap 'rm -f -- "$temporary"' EXIT
tar --sort=name --mtime=@1 --owner=0 --group=0 --numeric-owner --format=ustar \
    -cf - -C "$pack" . | zstd -10 -T1 --check -q -o "$temporary" -f
mv -- "$temporary" "$output/tenchu-hd.tar.zst"
(cd -- "$output" && sha256sum tenchu-hd.tar.zst > SHA256SUMS)
cat "$output/SHA256SUMS"
