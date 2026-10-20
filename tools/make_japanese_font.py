"""Build the bundled subtitle font from unifont_jp-16.0.04.hex.gz."""

import gzip
import hashlib
from pathlib import Path
import sys

source, destination = map(Path, sys.argv[1:])
data = source.read_bytes()
assert hashlib.sha256(data).hexdigest() == (
    "5ba84e901b9f7fad3bce0571c7e4b4b0aef4ce0acc4ee6622ef0cfa2eef38a6f"
)
glyphs = {
    int(code, 16): bytes.fromhex(bitmap)
    for code, bitmap in (line.split(":") for line in gzip.decompress(data).decode().splitlines())
}
output = bytearray()
for code in range(0x8140, 0xF000):
    try:
        text = code.to_bytes(2, "big").decode("shift_jis")
    except UnicodeDecodeError:
        continue
    if len(text) != 1:
        continue
    bitmap = glyphs[ord(text)]
    if len(bitmap) == 16:
        rows = [value << 4 for value in bitmap]
    else:
        assert len(bitmap) == 32
        rows = [int.from_bytes(bitmap[i : i + 2], "big") for i in range(0, 32, 2)]
    if rows[-1] and not rows[0]:
        rows = rows[1:]
    else:
        rows[14] |= rows[15]
        rows = rows[:15]
    output.extend(code.to_bytes(2, "big"))
    for row in rows:
        output.extend(row.to_bytes(2, "big"))
destination.write_bytes(output)
print(f"{len(output) // 32} glyphs, {len(output)} bytes")
