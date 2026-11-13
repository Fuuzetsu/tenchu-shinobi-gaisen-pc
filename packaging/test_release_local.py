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
        self.fail_build = False
        self.failed_previous_run = False
        self.build_views = {}
        self.retried = set()
        patch = mock.patch.object(release_local, "run", side_effect=self.command)
        patch.start()
        self.addCleanup(patch.stop)
        patch = mock.patch.object(release_local.time, "sleep")
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
            return json.dumps([{"databaseId": number, "headBranch": branch, "url": f"https://example.invalid/run/{number}"}
                               for number, branch in [(42, "v0.2.0"), (43, "master")]])
        if args[:3] == ("gh", "run", "view"):
            number = args[3]
            count = self.build_views.get(number, 0)
            self.build_views[number] = count + 1
            status = "in_progress" if self.fail_build and count == 0 else "completed"
            conclusion = ("failure" if self.fail_build or self.failed_previous_run else "success") if status == "completed" else ""
            return json.dumps({"attempt": 2 if number in self.retried else 1,
                               "url": f"https://example.invalid/run/{number}", "status": status, "conclusion": conclusion,
                               "jobs": [{"name": "build", "status": status, "conclusion": conclusion}]})
        if args[:3] == ("gh", "run", "rerun"):
            self.failed_previous_run = False
            self.retried.add(args[3])
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
        self.assertEqual(json.loads((self.root / "private/releases/v0.2.0/status.json").read_text())["status"], "prepared")
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
        report = json.loads((self.root / "private/releases/v0.2.0/status.json").read_text())
        self.assertEqual((report["status"], report["phase"], report["revision"]), ("succeeded", "verified", prepared))

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

    def test_both_artwork_releases_must_be_published_and_match(self):
        directory = self.root / "artwork/tenchu2"
        directory.mkdir()
        (directory / "release.json").write_text(json.dumps({
            "tag": "tenchu2-artwork-v1", "archive": "tenchu2-hd.tar.zst",
            "bytes": 20, "sha256": "t2-hash",
        }))
        published = [
            {"draft": False, "assets": [{"name": "tenchu-hd.tar.zst", "size": 10, "digest": "sha256:artwork-hash"}]},
            {"draft": False, "assets": [{"name": "tenchu2-hd.tar.zst", "size": 20, "digest": "sha256:t2-hash"}]},
        ]
        with mock.patch.object(release_local, "github", side_effect=published) as github:
            release_local.check_artwork(self.root)
            self.assertEqual(github.call_args.args[-1], "repos/test/project/releases/tags/tenchu2-artwork-v1")
        for broken in (
            {"draft": True, "assets": published[1]["assets"]},
            {"draft": False, "assets": []},
            {"draft": False, "assets": [{"name": "tenchu2-hd.tar.zst", "size": 20, "digest": "sha256:wrong"}]},
        ):
            with mock.patch.object(release_local, "github", side_effect=[published[0], broken]):
                with self.assertRaisesRegex(ValueError, "tenchu2-artwork-v1"):
                    release_local.check_artwork(self.root)

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
        self.fail_build = True
        with self.assertRaisesRegex(ValueError, "v0.2.0: failure"):
            self.execute()
        tag = self.git("rev-parse", "v0.2.0")
        self.verify.assert_not_called()
        report = json.loads((self.root / "private/releases/v0.2.0/status.json").read_text())
        self.assertEqual((report["status"], report["phase"]), ("failed", "builds"))
        self.assertIn("https://example.invalid/run/42", report["error"])
        self.fail_build = False
        self.failed_previous_run = True
        self.execute()
        self.assertIn(("gh", "run", "rerun", "42", "--failed"), self.calls)
        self.assertEqual(self.git("rev-parse", "v0.2.0"), tag)
        self.verify.assert_called_once()

    def test_failed_download_verification_is_not_reported_as_release_success(self):
        self.verify.side_effect = ValueError("Checksum mismatch")
        with self.assertRaisesRegex(ValueError, "Checksum mismatch"):
            self.execute()
        report = json.loads((self.root / "private/releases/v0.2.0/status.json").read_text())
        self.assertEqual((report["status"], report["phase"]), ("failed", "downloads"))
        self.assertTrue(all(build["conclusion"] == "success" for build in report["builds"].values()))

    def test_interrupt_records_incomplete_release_and_preserves_files(self):
        with mock.patch.object(release_local, "prepare_version", side_effect=KeyboardInterrupt):
            with self.assertRaises(KeyboardInterrupt):
                self.execute()
        report = json.loads((self.root / "private/releases/v0.2.0/status.json").read_text())
        self.assertEqual((report["status"], report["phase"]), ("interrupted", "checks"))
        self.assertEqual(self.git("status", "--porcelain"), "")
        self.assertEqual(self.git("rev-parse", "HEAD"), self.before)
        self.assertEqual(self.git("tag", "--list"), "")

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


class BuildMonitorTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.progress = release_local.ReleaseProgress(Path(temporary.name), "0.2.0")
        self.output = io.StringIO()
        self.now = 0
        self.poll = 0

    def sleep(self, seconds):
        self.now += seconds
        self.poll += 1

    @staticmethod
    def build(conclusion="", attempt=1):
        status = "completed" if conclusion else "in_progress"
        return {"attempt": attempt, "status": status, "conclusion": conclusion,
                "url": "https://example.invalid/run/42",
                "jobs": [{"name": "linux", "status": status, "conclusion": conclusion}]}

    def monitor(self, github):
        with (
            mock.patch.object(release_local, "github", side_effect=github),
            mock.patch.object(release_local, "run") as run,
            mock.patch.object(release_local.time, "sleep", side_effect=self.sleep),
            mock.patch.object(release_local.time, "monotonic", side_effect=lambda: self.now),
            contextlib.redirect_stdout(self.output),
            self.progress,
        ):
            release_local.wait_for_builds(("v0.2.0", "master"), "revision", self.progress)
        return run

    @staticmethod
    def runs():
        return [{"databaseId": number, "headBranch": branch, "url": f"https://example.invalid/run/{number}"}
                for number, branch in [(42, "v0.2.0"), (43, "master")]]

    def test_head_failure_is_reported_while_tag_builds_and_unchanged_polls_are_quiet(self):
        def github(*args):
            if args[:2] == ("run", "list"):
                return self.runs()
            self.assertLess(self.poll, 3)
            build = self.build("failure" if self.poll == 2 and args[2] == "43" else "")
            # Step-level churn must not repeat the unchanged job summary.
            build["jobs"][0]["steps"] = [{"name": str(self.poll)}]
            return build

        with self.assertRaisesRegex(ValueError, "master: failure.*failed: linux"):
            self.monitor(github)
        self.assertEqual(self.poll, 2)
        text = self.output.getvalue()
        self.assertEqual(text.count("v0.2.0: in_progress"), 1)
        self.assertEqual(text.count("master: in_progress"), 1)
        report = json.loads(self.progress.path.read_text())
        self.assertEqual(report["status"], "failed")
        self.assertEqual(report["builds"]["v0.2.0"]["status"], "in_progress")

    def test_retry_waits_for_new_attempt_and_does_not_accept_stale_success_or_failure(self):
        for conclusion in ("failure", "cancelled"):
            with self.subTest(conclusion=conclusion):
                self.poll = 0

                def github(*args):
                    if args[:2] == ("run", "list"):
                        return self.runs()
                    if args[2] == "43":
                        return self.build("success")
                    self.assertLess(self.poll, 4)
                    return [self.build(conclusion), self.build("success"),
                            self.build(attempt=2), self.build("success", attempt=2)][self.poll]

                run = self.monitor(github)
                flags = ("--failed",) if conclusion == "failure" else ()
                run.assert_called_once_with("gh", "run", "rerun", "42", *flags)
                self.assertEqual(self.poll, 3)
                self.assertEqual(self.progress.report["builds"]["v0.2.0"]["attempt"], 2)

    def test_missing_head_workflow_times_out_even_if_tag_succeeds(self):
        def github(*args):
            self.assertLessEqual(self.now, 300)
            return self.runs()[:1] if args[:2] == ("run", "list") else self.build("success")

        with self.assertRaisesRegex(ValueError, "No push workflow appeared for master"):
            self.monitor(github)
        self.assertEqual(self.now, 300)

    def test_retry_that_never_starts_times_out_instead_of_accepting_old_result(self):
        def github(*args):
            self.assertLessEqual(self.now, 300)
            if args[:2] == ("run", "list"):
                return self.runs()
            return self.build("failure" if args[2] == "42" else "success")

        with self.assertRaisesRegex(ValueError, "Retry did not start for v0.2.0"):
            self.monitor(github)
        self.assertEqual(self.now, 300)
        self.assertEqual(json.loads(self.progress.path.read_text())["status"], "failed")

    def test_delayed_workflows_complete_without_extra_watch_commands(self):
        def github(*args):
            self.assertLessEqual(self.poll, 1)
            if args[:2] == ("run", "list"):
                return [] if self.poll == 0 else self.runs()
            return self.build("success")

        run = self.monitor(github)
        run.assert_not_called()
        self.assertEqual(self.poll, 1)
        self.assertEqual(set(self.progress.report["builds"]), {"v0.2.0", "master"})


if __name__ == "__main__":
    unittest.main()
