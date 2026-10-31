import contextlib
import datetime
import io
import json
import os
import subprocess
import tempfile
import tomllib
import unittest
from pathlib import Path
from unittest import mock

import release_local


class LocalReleaseTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name) / "checkout"
        self.root.mkdir()
        previous = Path.cwd()
        os.chdir(self.root)
        self.addCleanup(os.chdir, previous)
        self.environment = mock.patch.dict(os.environ, {
            "GH_REPO": "test/project", "CARGO_HOME": str(Path(temporary.name) / "cargo"),
            "CARGO_TARGET_DIR": str(self.root / "target"),
        })
        self.environment.start()
        self.addCleanup(self.environment.stop)
        self.git("init", "--initial-branch=master")
        self.git("config", "user.name", "Release test")
        self.git("config", "user.email", "release@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        self.git("config", "tag.gpgsign", "false")
        self.git("config", "core.hooksPath", "/dev/null")
        (self.root / "Cargo.toml").write_text('[workspace]\nmembers = ["game", "core"]\nresolver = "3"\n[workspace.package]\nversion = "0.1.0"\n')
        for name in ("game", "core"):
            package = self.root / name
            (package / "src").mkdir(parents=True)
            (package / "src/lib.rs").write_text("pub fn example() {}\n")
            manifest = f'[package]\nname = "example-{name}"\nversion.workspace = true\nedition = "2024"\n'
            if name == "game":
                manifest += '[dependencies]\nexample-core = { path = "../core" }\n'
            (package / "Cargo.toml").write_text(manifest)
        subprocess.run(["cargo", "generate-lockfile", "--offline"], check=True, capture_output=True)
        (self.root / "CHANGELOG.md").write_text("# Changes\n\n## Unreleased\n\n### Fixed\n\n- Sound fix.\n\n## 0.1.0 — 2026-01-01\n\n- First release.\n")
        (self.root / ".gitignore").write_text("/private/\n/target/\n")
        (self.root / "artwork").mkdir()
        (self.root / "artwork/release.json").write_text(json.dumps({
            "tag": "artwork-v1", "archive": "tenchu-hd.tar.zst", "bytes": 10, "sha256": "artwork-hash",
        }))
        self.git("add", ".")
        self.git("commit", "-m", "Existing feature")
        self.before = self.git("rev-parse", "HEAD")
        remote = Path(temporary.name) / "origin.git"
        self.git("init", "--bare", "--initial-branch=master", str(remote))
        self.git("remote", "add", "origin", str(remote))
        self.git("push", "-u", "origin", "master")
        self.calls = []
        self.fail_check = False
        self.fail_commit = False
        self.fail_watch = False
        self.failed_previous_run = False
        patch = mock.patch.object(release_local, "run", side_effect=self.command)
        patch.start()
        self.addCleanup(patch.stop)
        self.verify = mock.Mock()
        patch = mock.patch.object(release_local, "verify_published", self.verify)
        patch.start()
        self.addCleanup(patch.stop)

    def git(self, *args):
        return subprocess.check_output(["git", *args], stderr=subprocess.PIPE, text=True).strip()

    def command(self, *args, capture=False):
        self.calls.append(args)
        if args[0] == "git":
            if args[1] == "commit" and self.fail_commit:
                raise subprocess.CalledProcessError(1, args)
            return self.git(*args[1:])
        if args == ("cargo", "check", "--workspace"):
            return subprocess.run(args, check=True, capture_output=True, text=True).stdout
        if args[0] == "nix":
            if self.fail_check:
                raise subprocess.CalledProcessError(1, args)
            return None
        if args[:3] == ("gh", "repo", "view"):
            return json.dumps({"nameWithOwner": "test/project", "defaultBranchRef": {"name": "master"}})
        if args[:2] == ("gh", "api") and args[2].endswith("/releases/tags/artwork-v1"):
            return json.dumps({"draft": False, "assets": [{"name": "tenchu-hd.tar.zst", "size": 10, "digest": "sha256:artwork-hash"}]})
        if args[:3] == ("gh", "run", "list"):
            return json.dumps([{"databaseId": 42, "url": "https://example.invalid/run/42", "status": "completed",
                                "conclusion": "failure" if self.failed_previous_run else "success"}])
        if args[:3] == ("gh", "run", "watch"):
            if self.fail_watch:
                raise subprocess.CalledProcessError(1, args)
            return None
        if args[:3] == ("gh", "run", "rerun"):
            self.failed_previous_run = False
            return None
        self.fail(f"Unexpected command: {args}")

    def execute(self, version="0.2.0", prepare_only=False):
        with contextlib.redirect_stdout(io.StringIO()):
            release_local.execute(self.root, version, prepare_only)

    def test_prepare_then_publish_and_repeat_preserve_one_commit_and_tag(self):
        self.execute(prepare_only=True)
        prepared = self.git("rev-parse", "HEAD")
        self.assertNotEqual(prepared, self.before)
        self.assertEqual(self.git("rev-parse", "origin/master"), self.before)
        self.assertEqual(self.git("tag", "--list"), "")
        self.assertEqual(self.git("status", "--porcelain"), "")
        packages = tomllib.loads((self.root / "Cargo.lock").read_text())["package"]
        self.assertEqual({p["version"] for p in packages}, {"0.2.0"})
        text = (self.root / "CHANGELOG.md").read_text()
        self.assertEqual(text.count("- Sound fix."), 1)
        self.assertIn("## Unreleased\n\n## 0.2.0 — ", text)
        self.assertIn("## 0.1.0 — 2026-01-01\n\n- First release.", text)
        self.verify.assert_not_called()
        self.execute()
        tag = self.git("rev-parse", "v0.2.0")
        self.assertEqual(self.git("cat-file", "-t", "v0.2.0"), "tag")
        self.assertEqual(self.git("rev-parse", "v0.2.0^{commit}"), prepared)
        self.assertEqual(self.git("rev-parse", "origin/master"), prepared)
        self.assertIn(("git", "push", "--atomic", "origin", "HEAD:refs/heads/master", "refs/tags/v0.2.0"), self.calls)
        self.execute()
        self.assertEqual(self.git("rev-parse", "HEAD"), prepared)
        self.assertEqual(self.git("rev-parse", "v0.2.0"), tag)
        self.assertEqual(self.verify.call_count, 2)

    def test_failed_checks_restore_files_and_never_commit_tag_or_push(self):
        original = {name: (self.root / name).read_bytes() for name in release_local.VERSION_FILES}
        for failure in ("fail_check", "fail_commit"):
            with self.subTest(failure=failure):
                setattr(self, failure, True)
                with self.assertRaises(subprocess.CalledProcessError):
                    self.execute()
                self.assertEqual(self.git("status", "--porcelain"), "")
                self.assertEqual(self.git("rev-parse", "HEAD"), self.before)
                self.assertEqual(self.git("tag", "--list"), "")
                self.assertFalse(any(c[:2] == ("git", "push") for c in self.calls))
                for name, data in original.items():
                    self.assertEqual((self.root / name).read_bytes(), data)
                setattr(self, failure, False)
        self.execute(prepare_only=True)

    def test_preparation_and_validation_share_utc_date_across_midnight(self):
        original = {name: (self.root / name).read_bytes() for name in release_local.VERSION_FILES}
        for hour, offset in [(15, 9), (2, -8)]:
            with self.subTest(offset=offset):
                for name, content in original.items():
                    (self.root / name).write_bytes(content)
                instant = datetime.datetime(2026, 10, 31, hour, tzinfo=datetime.UTC)
                local = instant.astimezone(datetime.timezone(datetime.timedelta(hours=offset)))
                with (
                    mock.patch.object(release_local.datetime, "datetime", wraps=datetime.datetime) as clock,
                    mock.patch.object(release_local.datetime, "date", wraps=datetime.date) as date,
                ):
                    clock.now.side_effect = lambda zone=None: instant.astimezone(zone) if zone else local.replace(tzinfo=None)
                    date.today.return_value = local.date()
                    release_local.prepare_version(self.root, "0.2.0")
                    changelog = self.root / "CHANGELOG.md"
                    text = changelog.read_text()
                    self.assertIn("## 0.2.0 — 2026-10-31", text)
                    changelog.write_text(text.replace("## 0.2.0 — 2026-10-31", "## 0.2.0 — 2026-11-01"))
                    with self.assertRaisesRegex(ValueError, "Release date is in the future"):
                        release_local.release.release_notes(self.root, "refs/tags/v0.2.0")

    def test_existing_tag_with_wrong_version_cannot_be_prepared(self):
        self.git("tag", "-a", "v0.2.0", "-m", "Mistaken tag")
        with self.assertRaisesRegex(ValueError, "does not match Cargo.toml"):
            self.execute()
        self.assertEqual(self.git("rev-parse", "HEAD"), self.before)
        self.assertEqual(self.git("status", "--porcelain"), "")

    def test_failed_hosted_build_can_resume_without_retagging(self):
        self.fail_watch = True
        with self.assertRaises(subprocess.CalledProcessError):
            self.execute()
        tag = self.git("rev-parse", "v0.2.0")
        self.verify.assert_not_called()
        self.fail_watch = False
        self.failed_previous_run = True
        self.execute()
        self.assertIn(("gh", "run", "rerun", "42", "--failed"), self.calls)
        self.assertEqual(self.git("rev-parse", "v0.2.0"), tag)
        self.verify.assert_called_once()

    def test_conflicting_remote_tag_is_never_moved(self):
        self.git("tag", "-a", "v0.2.0", "-m", "Existing release")
        self.git("push", "origin", "v0.2.0")
        tagged = self.git("ls-remote", "--tags", "origin")
        self.git("tag", "-d", "v0.2.0")
        self.git("commit", "--allow-empty", "-m", "Later work")
        with self.assertRaisesRegex(ValueError, "Remote tag.*another commit"):
            self.execute()
        self.assertEqual(self.git("ls-remote", "--tags", "origin"), tagged)
        self.assertEqual(self.git("tag", "--list"), "")
        self.assertEqual(self.git("status", "--porcelain"), "")

    def test_dirty_checkout_and_empty_notes_cannot_publish(self):
        changelog = self.root / "CHANGELOG.md"
        changelog.write_text("## Unreleased\n")
        with self.assertRaisesRegex(ValueError, "checkout must be clean"):
            self.execute()
        self.git("add", "CHANGELOG.md")
        self.git("commit", "-m", "No pending changes")
        with self.assertRaisesRegex(ValueError, "user-facing changes"):
            self.execute()
        self.assertFalse(any(c[:2] == ("git", "push") for c in self.calls))
        self.assertEqual(self.git("status", "--porcelain"), "")

    def test_version_order_handles_prereleases_and_rejects_old_versions(self):
        versions = ["0.1.0-alpha.1", "0.1.0-alpha.2", "0.1.0-beta.1", "0.1.0-rc.1", "0.1.0", "0.1.1-alpha.1"]
        self.assertEqual(sorted(versions, key=release_local.version_key), versions)
        with self.assertRaisesRegex(ValueError, "newer than"):
            self.execute("0.1.0-rc.1")
        self.assertEqual(self.git("status", "--porcelain"), "")


if __name__ == "__main__":
    unittest.main()
