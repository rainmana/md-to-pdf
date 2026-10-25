#!/usr/bin/env python3
"""Publish an allowlisted coverage payload without changing the source checkout."""

import argparse
import datetime
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
from urllib.parse import urlsplit
import xml.etree.ElementTree as ET


SOURCE_REF = "refs/heads/master"
DATA_REF = "refs/heads/coverage-data"
EXPECTED_FILES = {"coverage.svg", "COVERAGE.md", "metadata.json"}
MAX_PAYLOAD_BYTES = 2_000_000
SAFE_SVG_TAGS = {
    "svg", "title", "desc", "g", "defs", "lineargradient", "stop", "clippath", "mask",
    "rect", "path", "text", "tspan",
}
SHA_PATTERN = re.compile(r"[0-9a-f]{40}")
RUN_PATTERN = re.compile(
    r"https://github\.com/([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)/actions/runs/"
    r"([1-9][0-9]*)(?:/attempts/([1-9][0-9]*))?/?"
)


class PublishError(Exception):
    pass


def run_identity(url):
    match = RUN_PATTERN.fullmatch(url)
    if not match:
        raise PublishError("run URL must identify a GitHub Actions run and optional attempt")
    owner, repository, run_id, attempt = match.groups()
    return (owner.lower(), repository.lower()), (int(run_id), int(attempt or "1"))


def validate_remote(value):
    if value.startswith("https://"):
        parsed = urlsplit(value)
        if not parsed.hostname or parsed.username or parsed.password or parsed.query or parsed.fragment:
            raise PublishError("remote must not contain credentials, a query, or a fragment")
        return value
    # Local bare repositories make the full publishing path testable without credentials.
    path = Path(value)
    if not path.is_dir():
        raise PublishError("remote must be an HTTPS URL or an existing local repository")
    return str(path.resolve())


class Git:
    def __init__(self, directory, remote):
        self.directory = directory
        self.remote = remote
        self.environment = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
        self.environment.update(
            GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull, GIT_TERMINAL_PROMPT="0"
        )

    def run(self, *arguments, check=True):
        command = [
            "git", "-c", "core.hooksPath=" + os.devnull,
            "-c", "commit.gpgsign=false", "-c", "init.templateDir=",
        ]
        if urlsplit(self.remote).hostname == "github.com":
            command += ["-c", "credential.helper=", "-c", "credential.helper=!gh auth git-credential"]
        result = subprocess.run(
            command + list(arguments), cwd=self.directory, env=self.environment,
            text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=120,
        )
        if check and result.returncode:
            raise PublishError(f"git {arguments[0]} failed: {result.stderr.strip() or result.stdout.strip()}")
        return result

    def refs(self):
        output = self.run("ls-remote", "--heads", self.remote, SOURCE_REF, DATA_REF).stdout
        refs = {}
        for line in output.splitlines():
            sha, ref = line.split("\t", 1)
            refs[ref] = sha
        return refs

    def source_is_current(self, source_sha):
        master = self.refs().get(SOURCE_REF)
        if master is None:
            raise PublishError("remote master branch does not exist")
        if master != source_sha:
            print(f"Skipped stale coverage: source {source_sha} is no longer remote master ({master}).")
            return False
        return True


def read_payload(directory, source_sha):
    if directory.is_symlink() or not directory.is_dir():
        raise PublishError("payload must be a real directory")
    payload = {}
    for filename in ("coverage.svg", "COVERAGE.md"):
        path = directory / filename
        if path.is_symlink() or not path.is_file():
            raise PublishError(f"payload {filename} must be a regular file, not a symlink")
        if path.stat().st_size > MAX_PAYLOAD_BYTES:
            raise PublishError(f"payload {filename} exceeds the size limit")
        payload[filename] = path.read_bytes()

    svg = payload["coverage.svg"]
    if b"<!DOCTYPE" in svg.upper() or b"<!ENTITY" in svg.upper():
        raise PublishError("coverage SVG must not declare a document type or entities")
    root = ET.fromstring(svg)
    if root.tag != "{http://www.w3.org/2000/svg}svg":
        raise PublishError("coverage badge must be an SVG document")
    for element in root.iter():
        name = element.tag.rsplit("}", 1)[-1].lower()
        if not element.tag.startswith("{http://www.w3.org/2000/svg}") or name not in SAFE_SVG_TAGS:
            raise PublishError(f"coverage SVG contains forbidden {name} content")
        for attribute, value in element.attrib.items():
            name = attribute.rsplit("}", 1)[-1].lower()
            if name.startswith("on") or name in {"href", "src"}:
                raise PublishError("coverage SVG must not contain event handlers or external resources")
            if "url(" in value.lower() and not re.fullmatch(r"url\(#[A-Za-z0-9_-]+\)", value):
                raise PublishError("coverage SVG must not contain external resource references")
    report = payload["COVERAGE.md"].decode("utf-8")
    if source_sha not in report:
        raise PublishError("coverage report must record the full source SHA")
    return payload


def prepare_existing_branch(git, source_sha, run_url):
    git.run("fetch", "--quiet", "--no-tags", "origin", DATA_REF)
    tracked = {}
    tree = git.run("ls-tree", "-r", "--full-tree", "-z", "FETCH_HEAD").stdout
    for entry in tree.split("\0"):
        if not entry:
            continue
        properties, filename = entry.split("\t", 1)
        mode, kind, _ = properties.split(" ")
        tracked[filename] = (mode, kind)
    if set(tracked) != EXPECTED_FILES or any(
        mode not in {"100644", "100755"} or kind != "blob" for mode, kind in tracked.values()
    ):
        raise PublishError("coverage-data has unexpected tracked paths or nonregular files; refusing to overwrite")
    metadata = json.loads(git.run("show", "FETCH_HEAD:metadata.json").stdout)
    if not isinstance(metadata, dict) or set(metadata) != {"source_sha", "workflow_run_url", "updated_at"}:
        raise PublishError("coverage-data has invalid metadata")
    if not isinstance(metadata["source_sha"], str) or not SHA_PATTERN.fullmatch(metadata["source_sha"]):
        raise PublishError("coverage-data has invalid source metadata")
    if not isinstance(metadata["workflow_run_url"], str):
        raise PublishError("coverage-data has invalid workflow metadata")
    old_repository, old_run = run_identity(metadata["workflow_run_url"])
    repository, run = run_identity(run_url)
    if old_repository != repository:
        raise PublishError("coverage-data metadata belongs to another repository")
    if metadata["source_sha"] == source_sha and old_run >= run:
        print("Skipped coverage: this source already has a report from the same or a newer workflow run/attempt.")
        return False
    git.run("checkout", "--quiet", "--detach", "FETCH_HEAD")
    return True


def publish(arguments):
    source_sha = arguments.source_sha
    if not SHA_PATTERN.fullmatch(source_sha):
        raise PublishError("source SHA must be a full lowercase 40-character Git SHA")
    run_identity(arguments.run_url)
    remote = validate_remote(arguments.remote)
    with tempfile.TemporaryDirectory(prefix="md-to-pdf-coverage-") as temporary:
        git = Git(temporary, remote)
        if not git.source_is_current(source_sha):
            return
        payload = read_payload(Path(arguments.payload), source_sha)
        git.run("init", "--quiet", "--initial-branch=coverage-data")
        git.run("remote", "add", "origin", remote)
        git.run("config", "user.name", "github-actions[bot]")
        git.run("config", "user.email", "41898282+github-actions[bot]@users.noreply.github.com")
        if DATA_REF in git.refs() and not prepare_existing_branch(git, source_sha, arguments.run_url):
            return
        metadata = {
            "source_sha": source_sha,
            "workflow_run_url": arguments.run_url,
            "updated_at": datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z"),
        }
        payload["metadata.json"] = (json.dumps(metadata, indent=2, sort_keys=True) + "\n").encode("utf-8")
        for filename, contents in payload.items():
            destination = Path(temporary) / filename
            destination.write_bytes(contents)
            destination.chmod(0o644)
        git.run("add", "--", *sorted(EXPECTED_FILES))
        git.run("commit", "--quiet", "-m", f"Update hosted coverage for {source_sha[:12]}")
        # Recheck after all local work, immediately before the non-forced push.
        if not git.source_is_current(source_sha):
            return
        result = git.run("push", "--porcelain", "origin", f"HEAD:{DATA_REF}", check=False)
        if result.returncode:
            if not git.source_is_current(source_sha):
                return
            raise PublishError("coverage push was rejected; a competing publication was not overwritten. " + result.stderr.strip())
        print(f"Published coverage for {source_sha} to coverage-data ({arguments.run_url}).")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--payload", required=True)
    parser.add_argument("--remote", required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--run-url", required=True)
    arguments = parser.parse_args()
    try:
        publish(arguments)
    except (PublishError, OSError, ValueError, ET.ParseError, subprocess.TimeoutExpired) as error:
        print(f"Coverage publication failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
