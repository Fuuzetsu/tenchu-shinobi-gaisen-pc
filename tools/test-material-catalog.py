"""Check that shared texture usage survives merging different actor outfits."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from PIL import Image


class OutfitUsage(unittest.TestCase):
    def test_shared_identity_retains_each_outfit_and_its_face_count(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            exports = []
            for name, label, triangles in [("normal", "Normal", 7), ("armour", "Armour", 3)]:
                export = root / name
                (export / "textures").mkdir(parents=True)
                Image.new("RGBA", (4, 4), (60, 40, 20, 255)).save(export / "textures/shared.png")
                (export / "catalog.json").write_text(json.dumps({
                    "models": [{"id": "actors/0000/2", "label": label}],
                    "textures": ["shared"],
                    "texture_details": {"shared": {
                        "size": [4, 4], "alpha": {"255": 16}, "scrolling": False,
                        "terrain_attributes": [], "map_samples": [],
                        "models": {"actors/0000/2": {"triangles": triangles, "blends": [0]}},
                    }},
                }))
                exports.append(str(export))
            command = [sys.executable, str(Path(__file__).with_name("material-catalog.py")),
                       *exports, "--output", str(root / "merged")]
            result = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            catalogue = json.loads((root / "merged/catalog.json").read_text())
            self.assertEqual(catalogue["textures"], ["shared"])
            usage = catalogue["entries"][0]["models"]["actors/0000/2"]
            self.assertEqual(usage["labels"], ["Armour", "Normal"])
            self.assertEqual(usage["variants"]["normal"]["triangles"], 7)
            self.assertEqual(usage["variants"]["armour"]["triangles"], 3)
            self.assertEqual(usage["triangles"], 7)


if __name__ == "__main__":
    unittest.main()
