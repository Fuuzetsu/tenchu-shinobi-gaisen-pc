"""Run with the Nix development shell's Python; all fixtures are synthetic."""

import json
import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from PIL import Image

spec = importlib.util.spec_from_file_location("material_pack", Path(__file__).with_name("material-pack.py"))
material_pack = importlib.util.module_from_spec(spec)
spec.loader.exec_module(material_pack)


class CampaignBudget(unittest.TestCase):
    def test_shared_textures_count_in_each_using_mission_and_require_evidence(self):
        catalogue = {"entries": [
            {"id": "player", "exports": ["m1", "m2", "m1"]},
            {"id": "roof", "exports": ["m1"]},
            {"id": "snow", "exports": ["m2"]},
        ]}
        sizes = {"player": 2_000_000, "roof": 10_000_000, "snow": 12_000_000}
        self.assertEqual(material_pack.pixel_budgets(catalogue, sizes, True),
                         {"m1": 12_000_000, "m2": 14_000_000})
        self.assertEqual(material_pack.pixel_budgets(catalogue, sizes, False), {"pack": 24_000_000})
        with self.assertRaisesRegex(ValueError, "missing.*export usage"):
            material_pack.pixel_budgets(catalogue, {"missing": 100}, True)


class CutoutCoverage(unittest.TestCase):
    def test_reject_misalignment_and_preserve_original_alpha_with_color_bleed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "textures").mkdir()
            (root / "catalog.json").write_text(json.dumps({"textures": ["frame"]}))
            recipe = {"frame": {"color": "color.png", "size": 32,
                                "world_tile": [4000, 2000], "uv_tile": True}}
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
            manifest = json.loads((root / "pack/pack.json").read_text())
            self.assertEqual(manifest["surfaces"]["frame"]["world_tile"], [4000, 2000])
            self.assertTrue(manifest["surfaces"]["frame"]["uv_tile"])
            with Image.open(root / "pack/textures/frame.png") as packed:
                expected = original.getchannel("A").resize((32, 32), Image.Resampling.NEAREST)
                self.assertEqual(packed.getchannel("A").tobytes(), expected.tobytes())
                self.assertEqual(packed.getpixel((0, 0)), (75, 55, 35, 255))

            recipe["frame"]["alpha"] = "authored"
            (root / "recipe.json").write_text(json.dumps(recipe))
            command[-1] = str(root / "ambiguous")
            failed = subprocess.run(command, capture_output=True, text=True)
            self.assertIn("one original nonzero blend class", failed.stderr)
            original.putalpha(original.getchannel("A").point(lambda value: 128 if value else 0))
            original.save(root / "textures/frame.png")
            color.putalpha(Image.new("L", (32, 32), 0))
            color.putpixel((16, 16), (20, 40, 60, 200))
            color.putpixel((16, 17), (20, 40, 60, 100))
            color.save(root / "color.png")
            command[-1] = str(root / "authored")
            succeeded = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(succeeded.returncode, 0, succeeded.stderr)
            with Image.open(root / "authored/textures/frame.png") as packed:
                self.assertEqual(packed.getpixel((16, 16)), (20, 40, 60, 128))
                self.assertEqual(packed.getpixel((16, 17))[3], 0)
                self.assertEqual(packed.getpixel((0, 0))[3], 0)

            recipe["frame"]["alpha"] = "additive"
            (root / "recipe.json").write_text(json.dumps(recipe))
            command[-1] = str(root / "additive")
            succeeded = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(succeeded.returncode, 0, succeeded.stderr)
            with Image.open(root / "additive/textures/frame.png") as packed:
                self.assertEqual(packed.getpixel((16, 16)), (16, 31, 47, 128))
                self.assertEqual(packed.getpixel((16, 17)), (8, 16, 24, 128))
                self.assertEqual(packed.getpixel((0, 0)), (0, 0, 0, 0))


if __name__ == "__main__":
    unittest.main()
