"""Run with the Nix development shell's Python; all fixtures are synthetic."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from PIL import Image


class CutoutCoverage(unittest.TestCase):
    def test_reject_misalignment_and_preserve_original_alpha_with_color_bleed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "textures").mkdir()
            (root / "catalog.json").write_text(json.dumps({"textures": ["frame"]}))
            recipe = {"frame": {"color": "color.png", "size": 32}}
            (root / "recipe.json").write_text(json.dumps(recipe))
            original = Image.new("RGBA", (4, 4))
            for y in range(4):
                original.putpixel((0, y), (50, 40, 30, 255))
                original.putpixel((3, y), (50, 40, 30, 128))
            original.save(root / "textures/frame.png")
            color = Image.new("RGBA", (32, 32))
            color.save(root / "color.png")
            command = [sys.executable, str(Path(__file__).with_name("material-pack.py")),
                       str(root), str(root / "recipe.json"), str(root / "pack")]
            failed = subprocess.run(command, capture_output=True, text=True)
            self.assertNotEqual(failed.returncode, 0)
            self.assertIn("authored color misses 100.0%", failed.stderr)
            self.assertFalse((root / "pack").exists())

            color.paste((75, 55, 35, 255), (0, 0, 32, 32))
            color.save(root / "color.png")
            succeeded = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(succeeded.returncode, 0, succeeded.stderr)
            with Image.open(root / "pack/textures/frame.png") as packed:
                expected = original.getchannel("A").resize((32, 32), Image.Resampling.NEAREST)
                self.assertEqual(packed.getchannel("A").tobytes(), expected.tobytes())
                self.assertEqual(packed.getpixel((0, 0)), (75, 55, 35, 255))


if __name__ == "__main__":
    unittest.main()
