#!/usr/bin/env python3
"""Exercise coverage publication against real local bare Git repositories."""

import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).with_name("publish-coverage.py")
sys.dont_write_bytecode = True
SPEC = importlib.util.spec_from_file_location("publish_coverage", SCRIPT)
PUBLISHER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PUBLISHER)


class PublishIntegrationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="test-coverage-publisher-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.remote = self.root / "remote.git"
        self.source = self.root / "source"
        self.payload = self.root / "payload"
        self.payload.mkdir()
        self.git("init", "--quiet", "--bare", str(self.remote))
        self.git("init", "--quiet", "--initial-branch=master", str(self.source))
        self.git("-C", str(self.source), "config", "user.name", "Test Author")
        self.git("-C", str(self.source), "config", "user.email", "test@example.invalid")
        self.sha = self.add_source_commit("Initial source")
        self.write_payload(self.sha)

    def git(self, *arguments):
        environment = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
        environment.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull)
        result = subprocess.run(
            ["git", "-c", "core.hooksPath=" + os.devnull, "-c", "commit.gpgsign=false", "-c", "init.templateDir=", *arguments],
            cwd=self.root, env=environment, text=True, capture_output=True, check=True,
        )
        return result.stdout.strip()

    def add_source_commit(self, message):
        (self.source / "source.txt").write_text(message + "\n", encoding="utf-8")
        self.git("-C", str(self.source), "add", "source.txt")
        self.git("-C", str(self.source), "commit", "--quiet", "-m", message)
        self.git("-C", str(self.source), "push", "--quiet", str(self.remote), "master")
        return self.git("-C", str(self.source), "rev-parse", "HEAD")

    def write_payload(self, sha, percentage="88.9"):
        (self.payload / "coverage.svg").write_text(
            f'<svg xmlns="http://www.w3.org/2000/svg"><text>{percentage}%</text></svg>\n', encoding="utf-8"
        )
        (self.payload / "COVERAGE.md").write_text(f"# Hosted coverage\n\nSource: `{sha}`\n", encoding="utf-8")
        # Extra artifacts must never enter the public branch.
        (self.payload / "coverage.json").write_text('{"unpublished": true}\n', encoding="utf-8")
        (self.payload / "untrusted.py").write_text('raise RuntimeError("must never run")\n', encoding="utf-8")

    def publish(self, sha=None, run=100, attempt=None):
        url = f"https://github.com/example/project/actions/runs/{run}"
        if attempt is not None:
            url += f"/attempts/{attempt}"
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--payload", str(self.payload), "--remote", str(self.remote),
             "--source-sha", sha or self.sha, "--run-url", url],
            cwd=self.source, text=True, capture_output=True,
        )

    def data_head(self):
        refs = self.git("--git-dir", str(self.remote), "for-each-ref", "--format=%(objectname)", "refs/heads/coverage-data")
        return refs or None

    def read_data(self, filename):
        return self.git("--git-dir", str(self.remote), "show", f"coverage-data:{filename}")

    def test_first_publication_is_allowlisted_orphan_and_source_checkout_is_untouched(self):
        before = self.git("-C", str(self.source), "rev-parse", "HEAD")
        result = self.publish()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Published coverage", result.stdout)
        self.assertEqual(
            set(self.git("--git-dir", str(self.remote), "ls-tree", "--name-only", "coverage-data").splitlines()),
            {"coverage.svg", "COVERAGE.md", "metadata.json"},
        )
        self.assertEqual(self.git("--git-dir", str(self.remote), "rev-list", "--count", "coverage-data"), "1")
        metadata = json.loads(self.read_data("metadata.json"))
        self.assertEqual(metadata["source_sha"], self.sha)
        self.assertEqual(metadata["workflow_run_url"], "https://github.com/example/project/actions/runs/100")
        self.assertTrue(metadata["updated_at"].endswith("Z"))
        self.assertEqual(self.git("-C", str(self.source), "rev-parse", "HEAD"), before)
        self.assertEqual(self.git("-C", str(self.source), "status", "--porcelain"), "")
        self.assertEqual(self.git("--git-dir", str(self.remote), "rev-parse", "master"), self.sha)

    def test_update_preserves_parent_history(self):
        self.assertEqual(self.publish().returncode, 0)
        previous = self.data_head()
        self.sha = self.add_source_commit("Second source")
        self.write_payload(self.sha, "90.1")
        result = self.publish(run=101)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.git("--git-dir", str(self.remote), "rev-parse", "coverage-data^"), previous)
        self.assertIn("90.1%", self.read_data("coverage.svg"))
        self.assertEqual(json.loads(self.read_data("metadata.json"))["source_sha"], self.sha)

    def test_stale_source_skips_without_creating_publication_branch(self):
        self.add_source_commit("Newer source")
        result = self.publish()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Skipped stale coverage", result.stdout)
        self.assertIsNone(self.data_head())

    def test_same_source_run_and_attempt_never_move_backwards(self):
        self.assertEqual(self.publish(run=102, attempt=2).returncode, 0)
        head = self.data_head()
        for run, attempt in ((101, 9), (102, None), (102, 2)):
            with self.subTest(run=run, attempt=attempt):
                result = self.publish(run=run, attempt=attempt)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn("same or a newer", result.stdout)
                self.assertEqual(self.data_head(), head)
        self.assertEqual(self.publish(run=102, attempt=3).returncode, 0)
        self.assertEqual(self.git("--git-dir", str(self.remote), "rev-parse", "coverage-data^"), head)

    def test_unexpected_existing_paths_are_refused_without_overwrite(self):
        self.git("-C", str(self.source), "push", "--quiet", str(self.remote), "HEAD:coverage-data")
        head = self.data_head()
        result = self.publish()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unexpected tracked paths", result.stderr)
        self.assertEqual(self.data_head(), head)

    def test_missing_nonregular_and_invalid_payloads_are_rejected(self):
        mutations = {
            "missing": lambda: (self.payload / "coverage.svg").unlink(),
            "directory": lambda: ((self.payload / "coverage.svg").unlink(), (self.payload / "coverage.svg").mkdir()),
            "script": lambda: (self.payload / "coverage.svg").write_text('<svg xmlns="http://www.w3.org/2000/svg"><script/></svg>'),
            "foreignObject": lambda: (self.payload / "coverage.svg").write_text('<svg xmlns="http://www.w3.org/2000/svg"><foreignObject/></svg>'),
            "event_handler": lambda: (self.payload / "coverage.svg").write_text('<svg xmlns="http://www.w3.org/2000/svg" onload="alert(1)"/>'),
            "animation": lambda: (self.payload / "coverage.svg").write_text('<svg xmlns="http://www.w3.org/2000/svg"><animate attributeName="onload" to="alert(1)"/></svg>'),
            "external_style": lambda: (self.payload / "coverage.svg").write_text('<svg xmlns="http://www.w3.org/2000/svg"><style>@import url(https://example.invalid);</style></svg>'),
            "external_resource": lambda: (self.payload / "coverage.svg").write_text('<svg xmlns="http://www.w3.org/2000/svg"><path fill="url(https://example.invalid)"/></svg>'),
            "doctype": lambda: (self.payload / "coverage.svg").write_text('<!DOCTYPE svg><svg xmlns="http://www.w3.org/2000/svg"/>'),
            "wrong_sha": lambda: (self.payload / "COVERAGE.md").write_text("# Missing provenance\n"),
        }
        for name, mutate in mutations.items():
            with self.subTest(payload=name):
                svg = self.payload / "coverage.svg"
                if svg.is_dir():
                    svg.rmdir()
                self.write_payload(self.sha)
                mutate()
                result = self.publish()
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertIsNone(self.data_head())

    def test_symlink_payload_is_rejected(self):
        target = self.root / "outside.svg"
        (self.payload / "coverage.svg").rename(target)
        try:
            (self.payload / "coverage.svg").symlink_to(target)
        except OSError as error:
            self.skipTest(f"symlinks unavailable: {error}")
        result = self.publish()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not a symlink", result.stderr)
        self.assertIsNone(self.data_head())

    def test_source_change_during_preparation_is_skipped(self):
        original = PUBLISHER.Git.source_is_current
        calls = 0

        def change_master_before_second_check(git, source_sha):
            nonlocal calls
            calls += 1
            if calls == 2:
                self.add_source_commit("Master moved while coverage prepared")
            return original(git, source_sha)

        arguments = type("Arguments", (), {
            "payload": str(self.payload), "remote": str(self.remote), "source_sha": self.sha,
            "run_url": "https://github.com/example/project/actions/runs/100",
        })()
        with patch.object(PUBLISHER.Git, "source_is_current", change_master_before_second_check):
            PUBLISHER.publish(arguments)
        self.assertEqual(calls, 2)
        self.assertIsNone(self.data_head())

    def test_competing_publication_is_not_force_pushed(self):
        self.assertEqual(self.publish(run=100).returncode, 0)
        original = PUBLISHER.Git.run
        competing_head = None

        def publish_competitor_before_push(git, *arguments, **keywords):
            nonlocal competing_head
            if arguments[0] == "push":
                result = self.publish(run=102)
                self.assertEqual(result.returncode, 0, result.stderr)
                competing_head = self.data_head()
            return original(git, *arguments, **keywords)

        arguments = type("Arguments", (), {
            "payload": str(self.payload), "remote": str(self.remote), "source_sha": self.sha,
            "run_url": "https://github.com/example/project/actions/runs/101",
        })()
        with patch.object(PUBLISHER.Git, "run", publish_competitor_before_push):
            with self.assertRaisesRegex(PUBLISHER.PublishError, "competing publication was not overwritten"):
                PUBLISHER.publish(arguments)
        self.assertEqual(self.data_head(), competing_head)
        self.assertEqual(json.loads(self.read_data("metadata.json"))["workflow_run_url"], "https://github.com/example/project/actions/runs/102")


if __name__ == "__main__":
    unittest.main(verbosity=2)
