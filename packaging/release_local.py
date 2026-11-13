"""Prepare, push and verify a release using the existing GitHub build workflow."""

import argparse
import datetime
import json
import os
import re
import subprocess
import tempfile
import time
import tomllib
from pathlib import Path

import release


VERSION_FILES = ("Cargo.toml", "Cargo.lock", "CHANGELOG.md")
WORKFLOW_START_TIMEOUT = 5 * 60


class ReleaseProgress:
    def __init__(self, root, version):
        self.path = root / "private/releases" / f"v{version}" / "status.json"
        self.report = {"version": version, "status": "running"}

    def update(self, phase, message, **details):
        self.report.update(details, phase=phase, message=message,
                           updated_at=datetime.datetime.now(datetime.UTC).isoformat())
        self.path.parent.mkdir(parents=True, exist_ok=True)
        temporary = self.path.with_suffix(".tmp")
        temporary.write_text(json.dumps(self.report, indent=2) + "\n")
        temporary.replace(self.path)
        print(message, flush=True)

    def __enter__(self):
        self.update("preflight", f"Preparing v{self.report['version']}; status: {self.path}")
        return self

    def __exit__(self, kind, error, traceback):
        if error is not None:
            status = "interrupted" if isinstance(error, KeyboardInterrupt) else "failed"
            self.update(self.report["phase"], f"Release {status}: {error or 'interrupted'}",
                        status=status, error=str(error))


def run(*args, capture=False):
    if not capture:
        print("+ " + " ".join(map(str, args)), flush=True)
    result = subprocess.run(args, check=True, text=True, stdout=subprocess.PIPE if capture else None)
    return result.stdout.strip() if capture else None


def github(*args):
    return json.loads(run("gh", *args, capture=True))


def version_key(version):
    if not re.fullmatch(release.VERSION, version):
        raise ValueError("Use MAJOR.MINOR.PATCH, optionally followed by -alpha.N, -beta.N or -rc.N")
    numbers, _, suffix = version.partition("-")
    kind, _, number = suffix.partition(".")
    return (*map(int, numbers.split(".")), ("alpha", "beta", "rc", "").index(kind), int(number or 0))


def prepare_version(root, version):
    manifest = root / "Cargo.toml"
    text = manifest.read_text()
    current = tomllib.loads(text)["workspace"]["package"]["version"]
    release.release_notes(root, "refs/heads/master")
    changelog = (root / "CHANGELOG.md").read_text()
    section = re.search(r"^## Unreleased\n(.*?)(?=^## |\Z)", changelog, flags=re.M | re.S)
    if section is None:
        raise ValueError("CHANGELOG.md needs an Unreleased section")
    body = section[1].strip()
    if version == current:
        release.release_notes(root, f"refs/tags/v{version}")
        if body:
            raise ValueError("Choose a new version for the Unreleased changes")
        return False
    if version_key(version) <= version_key(current):
        raise ValueError(f"Release version must be newer than {current}")
    if not re.search(r"^- \S", body, flags=re.M):
        raise ValueError("Add user-facing changes under Unreleased in CHANGELOG.md first")
    if re.search(rf"^## {re.escape(version)} — ", changelog, flags=re.M):
        raise ValueError(f"CHANGELOG.md already contains version {version}")
    updated, count = re.subn(
        r'(^\[workspace\.package\]\n(?:(?!^\[).)*?^version\s*=\s*)"[^"]+"',
        lambda match: f'{match[1]}"{version}"', text, count=1, flags=re.M | re.S,
    )
    if count != 1:
        raise ValueError("Cannot locate workspace.package.version in Cargo.toml")
    date = datetime.datetime.now(datetime.UTC).date().isoformat()
    replacement = f"## Unreleased\n\n## {version} — {date}\n\n{body}\n\n"
    manifest.write_text(updated)
    (root / "CHANGELOG.md").write_text(changelog[:section.start()] + replacement + changelog[section.end():])
    run("cargo", "check", "--workspace")
    release.release_notes(root, f"refs/tags/v{version}")
    return True


def check_tag(tag, revision):
    local = bool(run("git", "tag", "--list", tag, capture=True))
    if local:
        if run("git", "rev-parse", f"{tag}^{{commit}}", capture=True) != revision:
            raise ValueError(f"Local tag {tag} already identifies another commit; it will not be moved")
        if run("git", "cat-file", "-t", f"refs/tags/{tag}", capture=True) != "tag":
            raise ValueError(f"{tag} must be an annotated tag")
    refs = dict(line.split()[::-1] for line in run(
        "git", "ls-remote", "--tags", "origin", f"refs/tags/{tag}", f"refs/tags/{tag}^{{}}", capture=True,
    ).splitlines())
    remote = refs.get(f"refs/tags/{tag}^{{}}", refs.get(f"refs/tags/{tag}"))
    if remote is not None and remote != revision:
        raise ValueError(f"Remote tag {tag} already identifies another commit; it will not be moved")
    if remote is not None:
        run("git", "fetch", "--no-tags", "origin", f"refs/tags/{tag}:refs/tags/{tag}")
    return local or remote is not None


def check_artwork(root):
    for artwork in release.artwork_releases(root):
        published = github("api", f"repos/{os.environ['GH_REPO']}/releases/tags/{artwork['tag']}")
        assets = [a for a in published["assets"] if a["name"] == artwork["archive"]]
        if published["draft"] or len(assets) != 1:
            raise ValueError(f"Publish {artwork['tag']} before releasing the game")
        if assets[0]["size"] != artwork["bytes"] or assets[0]["digest"] != "sha256:" + artwork["sha256"]:
            raise ValueError(f"Published {artwork['tag']} does not match its artwork/release.json")


def build_summary(branch, build):
    jobs = build["jobs"]
    finished = sum(job["status"] == "completed" for job in jobs)
    message = f"{branch}: {build['conclusion'] or build['status']}; {finished}/{len(jobs)} jobs finished"
    for label, selected in (
        ("running", [job["name"] for job in jobs if job["status"] == "in_progress"]),
        ("failed", [job["name"] for job in jobs if job["conclusion"] not in (None, "", "success", "skipped", "neutral")]),
    ):
        if selected:
            message += f"; {label}: {', '.join(selected)}"
    return message


def wait_for_builds(branches, revision, progress):
    builds = {}
    snapshots = {}
    discovery_deadline = time.monotonic() + WORKFLOW_START_TIMEOUT
    while True:
        if len(builds) != len(branches):
            runs = github(
                "run", "list", "--workflow", "build.yml", "--event", "push", "--commit", revision,
                "--limit", "20", "--json", "databaseId,headBranch,url",
            )
            for candidate in runs:
                branch = candidate["headBranch"]
                if branch in branches and branch not in builds:
                    builds[branch] = {"id": candidate["databaseId"], "observed": False}
                    progress.update("builds", f"{branch}: {candidate['url']}", builds=snapshots)
            missing = [branch for branch in branches if branch not in builds]
            if missing and time.monotonic() >= discovery_deadline:
                raise ValueError(f"No push workflow appeared for {', '.join(missing)} at {revision}; rerun to resume")
        for branch, state in builds.items():
            previous = snapshots.get(branch, {})
            if previous.get("status") == "completed" and previous.get("conclusion") == "success":
                continue
            build = github("run", "view", str(state["id"]), "--json", "attempt,status,conclusion,jobs,url")
            if not state["observed"]:
                state["observed"] = True
                if build["status"] == "completed" and build["conclusion"] != "success":
                    # Resume an earlier failure once; a new failure stops this invocation.
                    flags = ("--failed",) if build["conclusion"] == "failure" else ()
                    run("gh", "run", "rerun", str(state["id"]), *flags)
                    state.update(minimum_attempt=build["attempt"] + 1,
                                 deadline=time.monotonic() + WORKFLOW_START_TIMEOUT)
                    progress.update("builds", f"{branch}: retry requested; waiting for the next attempt", builds=snapshots)
                    continue
            if build["attempt"] < state.get("minimum_attempt", 0):
                if time.monotonic() >= state["deadline"]:
                    raise ValueError(f"Retry did not start for {branch}: {build['url']}; rerun to resume")
                continue
            snapshot = {key: build[key] for key in ("attempt", "status", "conclusion", "url")}
            snapshot["id"] = state["id"]
            snapshot["jobs"] = sorted(
                ({key: job[key] for key in ("name", "status", "conclusion")} for job in build["jobs"]),
                key=lambda job: job["name"],
            )
            if snapshot != snapshots.get(branch):
                snapshots[branch] = snapshot
                progress.update("builds", build_summary(branch, snapshot), builds=snapshots)
            if build["status"] == "completed" and build["conclusion"] != "success":
                raise ValueError(f"{build_summary(branch, snapshot)}; {build['url']}; rerun to resume")
        if len(snapshots) == len(branches) and all(
            build["status"] == "completed" and build["conclusion"] == "success" for build in snapshots.values()
        ):
            return
        time.sleep(5 if len(builds) != len(branches) else 30)


def verify_published(root, tag, revision, version):
    repository = os.environ["GH_REPO"]
    published = github("api", f"repos/{repository}/releases/tags/{tag}")
    if published["draft"] or published["prerelease"] != ("-" in version):
        raise ValueError("Release is still a draft or has the wrong prerelease status")
    if "-" not in version and github("api", f"repos/{repository}/releases/latest")["tag_name"] != tag:
        raise ValueError("The stable release is not marked Latest")
    nixpkgs = json.loads((root / "flake.lock").read_text())["nodes"]["nixpkgs"]["locked"]["rev"]
    with tempfile.TemporaryDirectory(prefix=f"tenchu-{tag}-") as temporary:
        downloads = Path(temporary)
        run("gh", "release", "download", tag, "--dir", str(downloads))
        release.verify_downloads(downloads, published["assets"], revision, version, nixpkgs)
    current = github("api", f"repos/{repository}/git/ref/heads/master")["object"]["sha"]
    if current == revision:
        head = github("api", f"repos/{repository}/releases/tags/head")
        head_commit = github("api", f"repos/{repository}/git/ref/tags/head")["object"]["sha"]
        if head["draft"] or not head["prerelease"] or head_commit != revision:
            raise ValueError("HEAD has not published this commit yet")
        if {a["name"] for a in head["assets"]} != set(release.ARCHIVES) | {"SHA256SUMS"}:
            raise ValueError("HEAD has an incomplete download set")
    else:
        print("master has advanced; leaving the newer HEAD release alone.")
    report = {"url": published["html_url"], "revision": revision, "version": version, "assets": published["assets"]}
    destination = root / "private/releases" / tag
    destination.mkdir(parents=True, exist_ok=True)
    (destination / "verified.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"Published and verified {published['html_url']}\nLinked artwork packs were verified separately.")


def execute(root, version, prepare_only=False):
    version_key(version)
    with ReleaseProgress(root, version) as progress:
        execute_release(root, version, prepare_only, progress)


def execute_release(root, version, prepare_only, progress):
    tag = f"v{version}"
    if run("git", "branch", "--show-current", capture=True) != "master":
        raise ValueError("Release from master")
    if run("git", "status", "--porcelain", capture=True):
        raise ValueError("Commit or stash your changes before releasing; the checkout must be clean")
    revision = run("git", "rev-parse", "HEAD", capture=True)
    if check_tag(tag, revision):
        release.release_notes(root, f"refs/tags/{tag}")
    remote = run("git", "remote", "get-url", "origin", capture=True)
    repo = github("repo", "view", remote, "--json", "nameWithOwner,defaultBranchRef")
    if repo["defaultBranchRef"]["name"] != "master":
        raise ValueError("The GitHub default branch must remain master")
    os.environ["GH_REPO"] = repo["nameWithOwner"]
    run("git", "fetch", "--no-tags", "origin", "master")
    run("git", "merge-base", "--is-ancestor", "origin/master", "HEAD")
    check_artwork(root)
    original = {name: (root / name).read_bytes() for name in VERSION_FILES}
    try:
        progress.update("checks", "Checking the release version, local builds and supported systems.")
        changed = prepare_version(root, version)
        run("nix", "flake", "check", "--print-build-logs")
        run("nix", "flake", "check", "--all-systems", "--no-build")
        if changed:
            run("git", "diff", "--check")
            run("git", "add", "--", *VERSION_FILES)
            run("git", "commit", "-m", f"Prepare v{version} release")
    except BaseException:
        if run("git", "rev-parse", "HEAD", capture=True) == revision:
            run("git", "restore", "--staged", "--", *VERSION_FILES)
            for name, content in original.items():
                (root / name).write_bytes(content)
        raise
    revision = run("git", "rev-parse", "HEAD", capture=True)
    if run("git", "status", "--porcelain", capture=True):
        raise ValueError("Checks left uncommitted changes; review them before publishing")
    if prepare_only:
        progress.update("prepared", f"Prepared {tag} locally. Review/play-test it, then run ./release.sh {version} to publish.",
                        status="prepared", revision=revision)
        return
    progress.update("push", f"Publishing {tag} at {revision}.", revision=revision)
    check_tag(tag, revision)
    if not run("git", "tag", "--list", tag, capture=True):
        run("git", "tag", "-a", tag, "-m", f"Tenchu PC {version}")
    run("git", "push", "--atomic", "origin", "HEAD:refs/heads/master", f"refs/tags/{tag}")
    progress.update("builds", "Waiting for the tagged and HEAD workflows; progress is reported only when it changes.")
    wait_for_builds((tag, "master"), revision, progress)
    progress.update("downloads", "Both workflows passed; downloading and verifying the published release.")
    verify_published(root, tag, revision, version)
    receipt = progress.path.with_name("verified.json")
    progress.update("verified", f"Release {tag} is published and verified; receipt: {receipt}",
                    status="succeeded", receipt=str(receipt))


def main():
    parser = argparse.ArgumentParser(description=__doc__, epilog=(
        "Requires a clean master checkout, user-facing Unreleased changelog entries and GitHub authentication. "
        "By default this commits, tags and pushes to origin, then waits for CI and verifies downloads. "
        "Rerun the same command to resume; published tags are never moved."
    ))
    parser.add_argument("version", help="release version, e.g. 0.3.3 or v0.3.3")
    parser.add_argument("--prepare-only", action="store_true", help="check and commit the version/notes locally; do not tag or push")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    os.chdir(root)
    try:
        execute(root, args.version.removeprefix("v"), args.prepare_only)
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Release stopped: {error}\nFix the problem and rerun the same command.\n")
    except KeyboardInterrupt:
        parser.exit(130, "Release interrupted. Rerun the same command to resume.\n")


if __name__ == "__main__":
    main()
