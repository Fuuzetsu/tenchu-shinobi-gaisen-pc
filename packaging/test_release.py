import hashlib
import io
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path

import release


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "Cargo.toml").write_text('[workspace.package]\nversion = "0.1.0"\n')
        (self.root / "artwork").mkdir()
        (self.root / "artwork/release.json").write_text('{"tag":"artwork-v1"}')

    def notes(self, changelog, ref="refs/tags/v0.1.0"):
        (self.root / "CHANGELOG.md").write_text(changelog)
        return release.release_notes(self.root, ref)

    def test_tagged_notes_exclude_unreleased_and_other_versions(self):
        notes = self.notes("## Unreleased\n- Future.\n## 0.1.0 — 2026-01-01\n- Current.\n## 0.0.1 — 2025-01-01\n- Old.\n")
        self.assertIn("Current.", notes)
        self.assertNotIn("Future.", notes)
        self.assertNotIn("Old.", notes)

    def test_unprepared_tags_fail(self):
        for changelog, ref in [
            ("## Unreleased\n- Feature.\n", "refs/tags/v0.1.0"),
            ("## Unreleased\n## 0.1.0 — 2026-01-01\n", "refs/tags/v0.1.0"),
            ("## Unreleased\n", "refs/tags/v1.0.0"),
            ("## Unreleased\n", "refs/tags/v0.1.0;echo"),
            ("## Unreleased\n## Unreleased\n", "refs/heads/master"),
            ("## Unreleased\n## 0.1.0 — 9999-01-01\n- Future.\n", "refs/tags/v0.1.0"),
        ]:
            with self.subTest(ref=ref, changelog=changelog), self.assertRaises(ValueError):
                self.notes(changelog, ref)

    def test_head_after_tag_can_have_empty_unreleased(self):
        self.assertIn("No user-facing changes", self.notes("## Unreleased\n", "refs/heads/master"))

    def archives(self, omit_source=False, unshipped_source=False):
        for i, name in enumerate(release.ARCHIVES):
            directory = self.root / "artifacts" / str(i)
            directory.mkdir(parents=True)
            path = directory / name
            info = json.dumps({"revision": "commit", "version": "0.1.0"}).encode()
            if name.endswith(".zip"):
                with zipfile.ZipFile(path, "w") as archive:
                    archive.writestr("bundle/build-info.json", info)
            else:
                with tarfile.open(path, "w:gz") as archive:
                    def add(name, data):
                        member = tarfile.TarInfo(f"bundle/{name}")
                        member.size = len(data)
                        archive.addfile(member, io.BytesIO(data))
                    add("build-info.json", info)
                    if name.startswith("tenchu-linux-"):
                        add("bundled-libraries.json", json.dumps({"lib/libexample.so": {
                            "component": "example", "source_archive": "upstream/example",
                        }}).encode())
                    if name == "tenchu-dependency-sources.tar.gz":
                        components = [{"name": "example", "source": "upstream/example",
                                       "recipe": "recipes/example.nix", "patches": []}]
                        if unshipped_source:
                            components.append({"name": "unshipped", "source": "upstream/unshipped",
                                               "recipe": "recipes/unshipped.nix", "patches": []})
                        add("source-packages.json", json.dumps({"components": components}).encode())
                        if not omit_source:
                            add("upstream/example", b"library source")
                        add("recipes/example.nix", b"build recipe")
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
            (directory / "SHA256SUMS").write_text(f"{digest}  {name}\n")

    def test_complete_set_has_combined_checksums(self):
        self.archives()
        release.collect(self.root / "artifacts", self.root / "dist", "commit", "0.1.0")
        self.assertEqual(len((self.root / "dist/SHA256SUMS").read_text().splitlines()), len(release.ARCHIVES))

    def test_missing_corrupt_or_wrong_commit_never_creates_distribution(self):
        self.archives()
        with self.assertRaises(ValueError):
            release.collect(self.root / "artifacts", self.root / "dist", "other", "0.1.0")
        path = self.root / "artifacts/0" / release.ARCHIVES[0]
        path.write_bytes(b"corrupted")
        with self.assertRaises(ValueError):
            release.collect(self.root / "artifacts", self.root / "dist", "commit", "0.1.0")
        path.unlink()
        with self.assertRaises(ValueError):
            release.collect(self.root / "artifacts", self.root / "dist", "commit", "0.1.0")
        self.assertFalse((self.root / "dist").exists())

    def test_missing_library_source_prevents_publication(self):
        self.archives(omit_source=True)
        with self.assertRaisesRegex(ValueError, "Missing dependency source"):
            release.collect(self.root / "artifacts", self.root / "dist", "commit", "0.1.0")
        self.assertFalse((self.root / "dist").exists())

    def test_unshipped_library_source_prevents_publication(self):
        self.archives(unshipped_source=True)
        with self.assertRaisesRegex(ValueError, "Sources for libraries absent"):
            release.collect(self.root / "artifacts", self.root / "dist", "commit", "0.1.0")
        self.assertFalse((self.root / "dist").exists())

    def publish(self, ref="refs/heads/master", **environment):
        self.archives()
        (self.root / "CHANGELOG.md").write_text("## Unreleased\n- New.\n## 0.1.0 — 2026-01-01\n- First.\n")
        scripts = self.root / "packaging"
        scripts.mkdir()
        for name in ("release.py", "publish.sh"):
            shutil.copyfile(Path(__file__).with_name(name), scripts / name)
        gh = self.root / "gh"
        gh.write_text(f"#!{sys.executable}\n" + '''
import json, os, sys
args = sys.argv[1:]
with open("calls.jsonl", "a") as log:
    log.write(json.dumps(args) + "\\n")
if args[:2] == ["release", "upload"] and os.environ.get("FAIL_UPLOAD"):
    sys.exit(1)
if any("releases?" in a for a in args):
    if os.environ.get("FAIL_NETWORK"):
        sys.exit(1)
    print(os.environ.get("EXISTING", ""))
elif any("git/ref/heads/" in a for a in args):
    print(os.environ.get("CURRENT", "commit"))
elif args[:2] == ["release", "view"]:
    print("1")
''')
        gh.chmod(0o755)
        result = subprocess.run(
            ["bash", "packaging/publish.sh"], cwd=self.root, capture_output=True, text=True,
            env={**os.environ, "PATH": f"{self.root}:{os.environ['PATH']}", "GH_REPO": "test/test",
                 "GITHUB_SHA": "commit", "GITHUB_REF": ref, "DEFAULT_BRANCH": "master", **environment},
        )
        self.assertTrue((self.root / "calls.jsonl").exists(), result.stderr)
        calls = [json.loads(line) for line in (self.root / "calls.jsonl").read_text().splitlines()]
        return result, calls

    def test_stale_head_cannot_mutate_releases(self):
        result, calls = self.publish(CURRENT="newer-commit")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(calls), 1)
        self.assertNotIn("--method", calls[0])

    def test_published_version_cannot_be_overwritten(self):
        result, calls = self.publish("refs/tags/v0.1.0", EXISTING="1\tfalse\tfalse")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(calls), 1)

    def test_network_error_is_not_treated_as_missing_release(self):
        result, calls = self.publish("refs/tags/v0.1.0", FAIL_NETWORK="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(calls), 1)

    def test_failed_upload_leaves_draft(self):
        result, calls = self.publish(FAIL_UPLOAD="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue(any(c[:2] == ["release", "create"] and "--draft" in c for c in calls))
        self.assertFalse(any("draft=false" in c for c in calls))

    def test_head_is_prerelease_and_published_only_after_upload(self):
        result, calls = self.publish()
        self.assertEqual(result.returncode, 0, result.stderr)
        create = next(c for c in calls if c[:2] == ["release", "create"])
        self.assertIn("--prerelease=true", create)
        self.assertIn("make_latest=false", calls[-1])
        upload = next(i for i, c in enumerate(calls) if c[:2] == ["release", "upload"])
        self.assertGreater(next(i for i, c in enumerate(calls) if "draft=false" in c), upload)


if __name__ == "__main__":
    unittest.main()
