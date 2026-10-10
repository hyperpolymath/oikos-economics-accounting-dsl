#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Lock parser and trust-boundary regressions; run with python3 -m unittest discover -s tests."""

import os
from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[1]
VALIDATOR = (ROOT / "scripts/check-lock-sync.sh").read_text()
WORKFLOW = (ROOT / ".github/workflows/lock-sync-gate.yml").read_text()
RUN = textwrap.dedent(WORKFLOW.split("        run: |\n", 1)[1])
LOCK = """version: 'v0.0.2'
workflows:
    '.github/workflows/test.yml':
        - 'example/action@v1'
dependencies:
    'example/action@v1':
        uses:
"""
YAML = "jobs:\n  test:\n    steps:\n      - uses: example/action@v1\n"


class Fixture(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.wf = self.root / ".github/workflows"
        self.wf.mkdir(parents=True)
        (self.wf / "test.yml").write_text(YAML)
        (self.wf / "actions.lock").write_text(LOCK)


class LockSyncTests(Fixture):
    def validate(self):
        return subprocess.run(
            ["bash", str(ROOT / "scripts/check-lock-sync.sh"), str(self.wf)],
            text=True, capture_output=True,
        )

    def test_nested_string_forms_and_transitive_closure(self):
        for ref in ("'example/child@v2'", '"example/child@v2"', "example/child@v2"):
            with self.subTest(ref=ref):
                nested = LOCK + f"            - {ref} # nested dependency\n"
                (self.wf / "actions.lock").write_text(nested)
                result = self.validate()
                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertIn("DANGLING EDGES", result.stdout)
                self.assertIn("example/child@v2", result.stdout)
                self.assertIn("named by: dependencies:example/action@v1", result.stdout)
                closed = nested + "    'example/child@v2': {}\n"
                (self.wf / "actions.lock").write_text(closed)
                result = self.validate()
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                (self.wf / "actions.lock").write_text(
                    nested + "    'example/child@v2':\n        uses:\n"
                    '            - "example/grandchild@v3"\n'
                )
                result = self.validate()
                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertIn("named by: dependencies:example/child@v2", result.stdout)

    def test_existing_validation_clauses(self):
        cases = (
            (YAML.replace("@v1", "@v2"), LOCK, "refs missing from the lockfile"),
            ("jobs: {}\n", LOCK, "stale lockfile entries"),
            (YAML, LOCK.split("dependencies:")[0], "DANGLING EDGES"),
            ("jobs: {}\n", "workflows:\ndependencies:\n", "UNLISTED WORKFLOWS"),
        )
        for yaml, lock, diagnostic in cases:
            with self.subTest(diagnostic=diagnostic):
                (self.wf / "test.yml").write_text(yaml)
                (self.wf / "actions.lock").write_text(lock)
                result = self.validate()
                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertIn(diagnostic, result.stdout)


class GateTests(Fixture):
    def setUp(self):
        super().setUp()
        self.env = dict(os.environ, GIT_CONFIG_GLOBAL=os.devnull,
                        GIT_CONFIG_NOSYSTEM="1", GIT_AUTHOR_NAME="Test",
                        GIT_AUTHOR_EMAIL="test@example.invalid", GIT_COMMITTER_NAME="Test",
                        GIT_COMMITTER_EMAIL="test@example.invalid")
        self.git("init", "-q")
        scripts = self.root / "scripts"
        scripts.mkdir()
        (scripts / "check-lock-sync.sh").write_text(VALIDATOR)
        self.base = self.commit()
        self.marker = self.root / "pr-code-executed"
        (scripts / "check-lock-sync.sh").write_text(f"touch '{self.marker}'\nexit 0\n")
        self.head = self.commit()
        self.merge()

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, env=self.env,
                                       text=True, stderr=subprocess.PIPE).strip()

    def commit(self):
        self.git("add", ".")
        self.git("commit", "-qm", "fixture")
        return self.git("rev-parse", "HEAD")

    def merge(self):
        self.git("add", ".")
        tree = self.git("write-tree")
        sha = self.git("commit-tree", tree, "-p", self.base, "-p", self.head, "-m", "merge")
        self.git("update-ref", "refs/pull/1/merge", sha)

    def gate(self, *, pr=True, head=None):
        with tempfile.TemporaryDirectory() as runner:
            env = dict(self.env, RUNNER_TEMP=runner, REPO="fixture/repo", SHA=self.base,
                       PR_NUMBER="1" if pr else "", PR_HEAD=head or self.head, TOKEN="synthetic",
                       GIT_CONFIG_COUNT="1", GIT_CONFIG_KEY_0="url.file://" + str(self.root) + ".insteadOf",
                       GIT_CONFIG_VALUE_0="https://github.com/fixture/repo.git")
            result = subprocess.run(["bash", "-c", RUN], cwd=runner, env=env,
                                    capture_output=True, text=True)
        self.assertFalse(self.marker.exists(), "PR-controlled script was executed")
        return result

    def test_trusted_workflow_configuration(self):
        self.assertIn("\n  pull_request_target:\n", WORKFLOW)
        self.assertNotIn("\n  pull_request:\n", WORKFLOW)
        self.assertIn("github.event.pull_request.number || github.ref", WORKFLOW)
        self.assertNotIn("uses:", "\n".join(
            line for line in WORKFLOW.splitlines() if not line.lstrip().startswith("#")
        ))

    def test_valid_merge_ignores_replaced_validator(self):
        result = self.gate()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_merge_data_is_validated_even_when_head_is_clean(self):
        (self.wf / "test.yml").write_text(YAML.replace("@v1", "@v2"))
        self.merge()
        result = self.gate()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("refs missing from the lockfile", result.stdout)

    def test_deleted_pr_validator_is_ignored(self):
        (self.root / "scripts/check-lock-sync.sh").unlink()
        self.merge()
        result = self.gate()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_symlink_data_is_rejected(self):
        (self.wf / "test.yml").unlink()
        (self.wf / "test.yml").symlink_to("../../scripts/check-lock-sync.sh")
        self.merge()
        result = self.gate()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("must be regular files", result.stderr)

    def test_stale_merge_is_rejected(self):
        result = self.gate(head=self.base)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not match", result.stdout)

    def test_missing_merge_is_rejected(self):
        self.git("update-ref", "-d", "refs/pull/1/merge")
        self.assertNotEqual(self.gate().returncode, 0)

    def test_push_and_dispatch_use_event_sha(self):
        result = self.gate(pr=False)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
