#!/usr/bin/env python3
"""Tests for scripts/publication_sync_check.py — Publication Sync Contract.

Covers the shift-left falsifiers and positive controls of
EffortlessMetrics/perl-lsp-swarm#12231 against synthetic Git graphs:

Positive controls:
  1. ordinary PR returns deterministic ``not_applicable`` success;
  2. exact synthetic [R,S] join and projected tree pass;
  3. simulated wrapper M preserving J passes post-merge verification.

Falsifiers (each must reject):
  - sync mode without a packet;
  - committed packet without the marker field;
  - wrong PR base;
  - head not equal to declared J (core-join re-derivation mismatch);
  - one-parent or reversed-parent join;
  - correct parents with wrong tree;
  - correct tree without reachable S ancestry (shallow clone);
  - one unlisted path differing from S;
  - reconciliation with a blocking decision or unmatched digest;
  - required NOT_PROVEN represented as pass;
  - ordinary PR accidentally entering sync mode from its title;
  - simulated GitHub squash wrapper;
  - any public mutation permission or command in the sync check.

Plus a permission/context ratchet on .github/workflows/publication-sync-contract.yml
and drift guards binding the validator to the committed schema and PR template.

Run with: python3 scripts/tests/test-publication-sync-contract.py
Exit code 0 on all-pass, 1 on any failure.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPTS_DIR.parent
CHECK = SCRIPTS_DIR / "publication_sync_check.py"
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "publication-sync-contract.yml"
SCHEMA = REPO_ROOT / "schemas" / "publication_sync.v2.schema.json"
PR_TEMPLATE = REPO_ROOT / ".github" / "PULL_REQUEST_TEMPLATE.md"

PACKET_PATH = ".github/publication-sync/packet.yaml"
LEDGER_PATH = ".github/publication-sync/reconciliation-ledger.json"
MANIFEST_PATH = ".github/publication-sync/projection-manifest.json"

RELEASE = "0.18.0-rc.1"
FIXED_ENV = {
    "GIT_AUTHOR_NAME": "Sync Fixture",
    "GIT_AUTHOR_EMAIL": "sync-fixture@example.invalid",
    "GIT_AUTHOR_DATE": "1700000000 +0000",
    "GIT_COMMITTER_NAME": "Sync Fixture",
    "GIT_COMMITTER_EMAIL": "sync-fixture@example.invalid",
    "GIT_COMMITTER_DATE": "1700000000 +0000",
}
JOIN_MESSAGE = "join 0.18.0-rc.1 audited projection\n"

BODY_YES = "## Publication Sync\n\n- Publication-sync PR (yes/no): yes\n"
BODY_NO = "## Publication Sync\n\n- Publication-sync PR (yes/no): no\n"
BODY_ABSENT = "ordinary pull request body\n"
# Title-like prose is not authority: mentions the sync everywhere except the
# dedicated marker field.
BODY_TITLE_LIKE = (
    "publication sync: land the audited 0.18.0-rc.1 join\n\n"
    "This PR is the publication-sync join for R/S/J.\n\n"
    "- Publication-sync PR (yes/no): no\n"
)


def json_bytes(payload: dict) -> bytes:
    return (json.dumps(payload, indent=2, sort_keys=True) + "\n").encode("utf-8")


class Repo:
    """Minimal plumbing-only synthetic repo builder."""

    def __init__(self, path: Path):
        self.path = path
        path.mkdir(parents=True, exist_ok=True)
        self.git("init", "-q")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "user.name", "Fixture")

    def git(self, *args: str, input_bytes: bytes | None = None) -> str:
        env = dict(os.environ)
        env.update(FIXED_ENV)
        proc = subprocess.run(
            ["git", "-C", str(self.path), *args],
            input=input_bytes,
            capture_output=True,
            env=env,
            check=False,
        )
        if proc.returncode != 0:
            raise AssertionError(
                f"git {' '.join(args)} failed: {proc.stderr.decode(errors='replace')}"
            )
        return proc.stdout.decode().strip()

    def make_tree(self, files: dict[str, bytes]) -> str:
        self.git("read-tree", "--empty")
        for path, content in sorted(files.items()):
            blob = self.git("hash-object", "-w", "--stdin", input_bytes=content)
            self.git("update-index", "--add", "--cacheinfo", f"100644,{blob},{path}")
        return self.git("write-tree")

    def make_commit(self, files: dict[str, bytes], parents: list[str], message: str) -> str:
        tree = self.make_tree(files)
        args = ["commit-tree", tree]
        for parent in parents:
            args += ["-p", parent]
        return self.git(*args, input_bytes=message.encode())


def ledger_payload(status: str = "resolved") -> dict:
    return {
        "schema_version": "v0.18_reconciliation.v1",
        "release": RELEASE,
        "decisions": [{"id": "d1", "status": status, "summary": "fixture decision"}],
        "blockers": [],
    }


def invariant_payload(status: str = "pass") -> list[dict]:
    return [
        {"id": inv, "status": status, "evidence": "synthetic fixture"}
        for inv in (
            "destination-context",
            "version",
            "topology",
            "artifact-reachability",
            "effective-identity",
        )
    ]


def manifest_payload(status: str = "pass") -> dict:
    return {
        "schema_version": "v0.18_publication_sync_manifest.v1",
        "release": RELEASE,
        "translations": [{"from": "docs/old.md", "to": "docs/new.md"}],
        "exclusions": ["tmp/skip.txt"],
        "required_invariants": invariant_payload(status),
    }


S_CONTENT = {
    "app/main.pl": b"use strict; use warnings;\n",
    "docs/old.md": b"old docs\n",
    "tmp/skip.txt": b"scratch\n",
    # A populated .github directory exercises the projection-tree subtree
    # rebuild (the real J keeps .github/workflows alongside the control dir).
    ".github/workflows/ci.yml": b"name: CI\n",
}
# Projection: docs/old.md translated to docs/new.md, tmp/skip.txt excluded.
PROJECTION_CONTENT = {
    "app/main.pl": S_CONTENT["app/main.pl"],
    "docs/new.md": b"new docs\n",
    ".github/workflows/ci.yml": S_CONTENT[".github/workflows/ci.yml"],
}


class SyncFixture:
    """Producer-side builder mirroring the future sibling-tooling output.

    Builds R, S, the core join J0 (projection tree, parents [R, S]), and the
    PR head J (projection plus the control-plane directory, identical message
    and identity) exactly as scripts/publication_sync_check.py documents.

    Tamper hooks:
      packet_override   — merged into the packet before it is committed;
      ledger / manifest — replace the artifact payloads;
      projection_extra  — extra files in the projection AND the packet's
                          expected tree (consistent producer change);
      head_extra        — extra files only in J's tree (post-packet tree
                          drift; may target the control directory);
      head_parents      — actual parents of J (default [R, S]).
    """

    def __init__(self, repo: Repo, *, packet_override=None, ledger=None, manifest=None,
                 projection_extra: dict[str, bytes] | None = None,
                 head_extra: dict[str, bytes] | None = None,
                 head_parents: list[str] | None = None):
        self.repo = repo
        self.r = repo.make_commit({"README.md": b"perl-lsp base\n"}, [], "R: base\n")
        self.s = repo.make_commit(dict(S_CONTENT), [], "S: prepared swarm\n")

        ledger_bytes = json_bytes(ledger if ledger is not None else ledger_payload())
        manifest_bytes = json_bytes(manifest if manifest is not None else manifest_payload())

        projection = dict(PROJECTION_CONTENT)
        if projection_extra:
            projection.update(projection_extra)
        projection_tree = repo.make_tree(projection)

        self.core_join = repo.git(
            "commit-tree", projection_tree, "-p", self.r, "-p", self.s,
            input_bytes=JOIN_MESSAGE.encode(),
        )
        packet = {
            "schema_version": "v0.18_publication_sync.v2",
            "release": RELEASE,
            "release_base_sha": self.r,
            "prepared_swarm_sha": self.s,
            "sync_join_sha": self.core_join,
            "expected_join_parents": [self.r, self.s],
            "reconciliation_digest": "sha256:" + hashlib.sha256(ledger_bytes).hexdigest(),
            "publication_sync_manifest_digest": "sha256:"
            + hashlib.sha256(manifest_bytes).hexdigest(),
            "expected_projected_tree": projection_tree,
            "published_channels": [],
            "release_cut": False,
        }
        if packet_override:
            packet.update(packet_override)

        head_files = dict(projection)
        if head_extra:
            head_files.update(head_extra)
        head_files[PACKET_PATH] = json_bytes(packet)
        head_files[LEDGER_PATH] = ledger_bytes
        head_files[MANIFEST_PATH] = manifest_bytes
        actual_parents = head_parents if head_parents is not None else [self.r, self.s]
        self.j = repo.make_commit(head_files, actual_parents, JOIN_MESSAGE)

    def wrapper_merge(self) -> str:
        """GitHub-style wrapper M: parents [master tip, J], tree == tree(J)."""
        tree = self.repo.git("rev-parse", f"{self.j}^{{tree}}")
        return self.repo.git(
            "commit-tree", tree, "-p", self.r, "-p", self.j,
            input_bytes=b"merge publication sync\n",
        )


def run_check(args: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(CHECK), *args],
        capture_output=True,
        text=True,
        check=False,
    )


class PublicationSyncContractTest(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="pubsync-test-")
        self.addCleanup(self._tmp.cleanup)
        self.repo = Repo(Path(self._tmp.name) / "repo")
        self.body_file = Path(self._tmp.name) / "body.md"

    def write_body(self, body: str) -> str:
        self.body_file.write_text(body, encoding="utf-8")
        return str(self.body_file)

    def run_pr(self, fixture: SyncFixture, body: str, *, base=None, head=None):
        return run_check([
            "pr",
            "--repo", str(self.repo.path),
            "--base-sha", base or fixture.r,
            "--head-sha", head or fixture.j,
            "--pr-body-file", self.write_body(body),
        ])

    def run_post_merge(self, merge_sha: str, *, master_at: str):
        self.repo.git("update-ref", "refs/heads/master", master_at)
        return run_check([
            "post-merge",
            "--repo", str(self.repo.path),
            "--merge-sha", merge_sha,
        ])

    # --- positive controls ---------------------------------------------------

    def test_ordinary_pr_not_applicable(self) -> None:
        commit = self.repo.make_commit({"src/lib.rs": b"fn main() {}\n"}, [], "ordinary\n")
        for body in (BODY_NO, BODY_ABSENT, BODY_TITLE_LIKE):
            proc = run_check([
                "pr", "--repo", str(self.repo.path),
                "--base-sha", commit, "--head-sha", commit,
                "--pr-body-file", self.write_body(body),
            ])
            self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
            self.assertIn("publication-sync: not_applicable", proc.stdout)

    def test_not_applicable_is_deterministic(self) -> None:
        commit = self.repo.make_commit({"src/lib.rs": b"fn main() {}\n"}, [], "ordinary\n")
        outputs = []
        for _ in range(2):
            proc = run_check([
                "pr", "--repo", str(self.repo.path),
                "--base-sha", commit, "--head-sha", commit,
                "--pr-body-file", self.write_body(BODY_NO),
            ])
            self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
            outputs.append(proc.stdout)
        self.assertEqual(outputs[0], outputs[1])

    def test_exact_synthetic_join_passes(self) -> None:
        fixture = SyncFixture(self.repo)
        proc = self.run_pr(fixture, BODY_YES)
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        for proof in range(1, 12):
            self.assertIn(f"PASS proof {proof}:", proc.stdout)
        self.assertIn("publication-sync: pass", proc.stdout)

    def test_wrapper_merge_passes_post_merge(self) -> None:
        fixture = SyncFixture(self.repo)
        merge_m = fixture.wrapper_merge()
        proc = self.run_post_merge(merge_m, master_at=merge_m)
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn("publication-sync post-merge: pass", proc.stdout)

    # --- falsifiers ------------------------------------------------------------

    def test_declared_sync_without_packet_fails(self) -> None:
        commit = self.repo.make_commit({"src/lib.rs": b"x\n"}, [], "c\n")
        proc = run_check([
            "pr", "--repo", str(self.repo.path),
            "--base-sha", commit, "--head-sha", commit,
            "--pr-body-file", self.write_body(BODY_YES),
        ])
        self.assertEqual(proc.returncode, 1)
        self.assertIn("without a committed packet", proc.stdout)

    def test_packet_without_marker_fails(self) -> None:
        fixture = SyncFixture(self.repo)
        for body in (BODY_NO, BODY_ABSENT):
            proc = self.run_pr(fixture, body)
            self.assertEqual(proc.returncode, 1, body)
            self.assertIn("may not arrive undeclared", proc.stdout)

    def test_wrong_pr_base_fails(self) -> None:
        fixture = SyncFixture(self.repo)
        wrong_base = self.repo.make_commit({"other.txt": b"x\n"}, [], "wrong base\n")
        proc = self.run_pr(fixture, BODY_YES, base=wrong_base)
        self.assertEqual(proc.returncode, 1)
        self.assertIn("proof 1", proc.stdout)

    def test_head_not_declared_join_fails(self) -> None:
        fixture = SyncFixture(self.repo, packet_override={"sync_join_sha": "0" * 40})
        proc = self.run_pr(fixture, BODY_YES)
        self.assertEqual(proc.returncode, 1)
        self.assertIn("proof 2", proc.stdout)

    def test_head_tree_drift_after_packet_fails(self) -> None:
        fixture = SyncFixture(self.repo, head_extra={"drift.txt": b"late change\n"})
        proc = self.run_pr(fixture, BODY_YES)
        self.assertEqual(proc.returncode, 1)
        self.assertIn("proof 2", proc.stdout)

    def test_one_parent_join_fails(self) -> None:
        fixture = SyncFixture(self.repo)
        solo = self.repo.make_commit({"solo.txt": b"x\n"}, [], "solo parent\n")
        one_parent = SyncFixture(self.repo, head_parents=[solo])
        self.assertEqual(fixture.r, one_parent.r)  # deterministic fixture identity
        proc = self.run_pr(one_parent, BODY_YES)
        self.assertEqual(proc.returncode, 1)
        self.assertIn("proof 3", proc.stdout)

    def test_reversed_parents_fail(self) -> None:
        fixture = SyncFixture(self.repo)
        reversed_head = SyncFixture(self.repo, head_parents=[fixture.s, fixture.r])
        proc = self.run_pr(reversed_head, BODY_YES)
        self.assertEqual(proc.returncode, 1)
        self.assertIn("proof 3", proc.stdout)

    def test_correct_parents_wrong_tree_fails(self) -> None:
        fixture = SyncFixture(
            self.repo,
            packet_override={"expected_projected_tree": "0" * 40},
        )
        proc = self.run_pr(fixture, BODY_YES)
        self.assertEqual(proc.returncode, 1)
        self.assertIn("proof 7", proc.stdout)

    def test_unreachable_s_fails_closed(self) -> None:
        fixture = SyncFixture(self.repo)
        # Shallow clone at depth 1: R and S are beyond the boundary, so
        # ancestry/parent evidence is missing and must fail, never pass.
        self.repo.git("update-ref", "refs/heads/master", fixture.j)
        shallow = Path(self._tmp.name) / "shallow"
        subprocess.run(
            ["git", "clone", "-q", "--depth", "1", f"file://{self.repo.path}", str(shallow)],
            check=True,
            capture_output=True,
        )
        proc = run_check([
            "pr", "--repo", str(shallow),
            "--base-sha", fixture.r, "--head-sha", fixture.j,
            "--pr-body-file", self.write_body(BODY_YES),
        ])
        self.assertEqual(proc.returncode, 1)
        self.assertIn("Publication Sync Contract FAILED", proc.stdout)

    def test_unlisted_path_diff_fails(self) -> None:
        fixture = SyncFixture(
            self.repo,
            projection_extra={"unlisted/evil.txt": b"not in manifest\n"},
        )
        # The projection tree changed consistently with the packet; only the
        # manifest fails to list the extra path.
        proc = self.run_pr(fixture, BODY_YES)
        self.assertEqual(proc.returncode, 1)
        self.assertIn("proof 8", proc.stdout)
        self.assertIn("unlisted/evil.txt", proc.stdout)

    def test_blocking_reconciliation_decision_fails(self) -> None:
        fixture = SyncFixture(self.repo, ledger=ledger_payload(status="blocked"))
        proc = self.run_pr(fixture, BODY_YES)
        self.assertEqual(proc.returncode, 1)
        self.assertIn("proof 5", proc.stdout)
        self.assertIn("blocked", proc.stdout)

    def test_ledger_digest_mismatch_fails(self) -> None:
        fixture = SyncFixture(
            self.repo,
            packet_override={"reconciliation_digest": "sha256:" + "0" * 64},
        )
        proc = self.run_pr(fixture, BODY_YES)
        self.assertEqual(proc.returncode, 1)
        self.assertIn("proof 5", proc.stdout)

    def test_not_proven_invariant_fails(self) -> None:
        fixture = SyncFixture(self.repo, manifest=manifest_payload(status="not_proven"))
        proc = self.run_pr(fixture, BODY_YES)
        self.assertEqual(proc.returncode, 1)
        self.assertIn("not_proven", proc.stdout)

    def test_missing_required_invariant_fails(self) -> None:
        manifest = manifest_payload()
        manifest["required_invariants"] = [
            inv for inv in manifest["required_invariants"] if inv["id"] != "topology"
        ]
        fixture = SyncFixture(self.repo, manifest=manifest)
        proc = self.run_pr(fixture, BODY_YES)
        self.assertEqual(proc.returncode, 1)
        self.assertIn("topology", proc.stdout)

    def test_squash_wrapper_fails_post_merge(self) -> None:
        fixture = SyncFixture(self.repo)
        # Simulated GitHub squash: same tree as J, single parent, J lost.
        tree = self.repo.git("rev-parse", f"{fixture.j}^{{tree}}")
        squash_m = self.repo.git(
            "commit-tree", tree, "-p", fixture.r, input_bytes=b"squashed sync\n"
        )
        proc = self.run_post_merge(squash_m, master_at=squash_m)
        self.assertEqual(proc.returncode, 1)
        self.assertIn("squash/rebase", proc.stdout)

    def test_post_merge_master_mismatch_fails(self) -> None:
        fixture = SyncFixture(self.repo)
        merge_m = fixture.wrapper_merge()
        other = self.repo.make_commit({"x.txt": b"x\n"}, [], "other tip\n")
        proc = self.run_post_merge(merge_m, master_at=other)
        self.assertEqual(proc.returncode, 1)
        self.assertIn("refs/heads/master", proc.stdout)

    def test_publish_fields_fail_closed(self) -> None:
        for override in (
            {"published_channels": ["crates.io"]},
            {"release_cut": True},
        ):
            fixture = SyncFixture(self.repo, packet_override=override)
            proc = self.run_pr(fixture, BODY_YES)
            self.assertEqual(proc.returncode, 1, override)
            self.assertIn("no-publish contract", proc.stdout)

    def test_packet_schema_ratchet(self) -> None:
        for override in (
            {"unexpected": "key"},
            {"release_base_sha": "not-a-sha"},
            {"schema_version": "v0.18_publication_sync.v1"},
        ):
            fixture = SyncFixture(self.repo, packet_override=override)
            proc = self.run_pr(fixture, BODY_YES)
            self.assertEqual(proc.returncode, 1, override)

    def test_conflicting_marker_values_fail(self) -> None:
        fixture = SyncFixture(self.repo)
        body = BODY_YES + "\n- Publication-sync PR (yes/no): no\n"
        proc = self.run_pr(fixture, body)
        self.assertEqual(proc.returncode, 1)
        self.assertIn("conflicting", proc.stdout)

    def test_crlf_body_still_enters_sync_mode(self) -> None:
        # GitHub may deliver PR bodies with CRLF endings; the marker must not
        # silently read as absent (which would false-red a valid sync PR).
        fixture = SyncFixture(self.repo)
        proc = self.run_pr(fixture, BODY_YES.replace("\n", "\r\n"))
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn("publication-sync: pass", proc.stdout)

    def ordinary_commit_on(self, parent: str) -> str:
        """Ordinary follow-up commit reusing the parent's exact tree."""
        tree = self.repo.git("rev-parse", f"{parent}^{{tree}}")
        return self.repo.git("commit-tree", tree, "-p", parent, input_bytes=b"ordinary\n")

    def test_inherited_packet_is_not_applicable(self) -> None:
        # After a sync lands, master carries its packet (tree(M) == tree(J)).
        # An ordinary PR whose head inherits that packet unchanged must NOT
        # enter sync mode, or every later PR would fail closed forever.
        fixture = SyncFixture(self.repo)
        head = self.ordinary_commit_on(fixture.j)
        proc = self.run_pr(fixture, BODY_NO, base=fixture.j, head=head)
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn("publication-sync: not_applicable", proc.stdout)

    def test_declared_sync_with_stale_packet_fails(self) -> None:
        # Marker yes but the packet is byte-identical to the base: a stale
        # packet cannot re-prove an old join.
        fixture = SyncFixture(self.repo)
        head = self.ordinary_commit_on(fixture.j)
        proc = self.run_pr(fixture, BODY_YES, base=fixture.j, head=head)
        self.assertEqual(proc.returncode, 1)
        self.assertIn("stale packet", proc.stdout)

    def test_extra_control_dir_file_fails(self) -> None:
        # Anything beyond packet/ledger/manifest in the control directory is
        # invisible to the tree and diff proofs, so it must fail the gate.
        fixture = SyncFixture(
            self.repo,
            head_extra={".github/publication-sync/unreviewed.txt": b"smuggled\n"},
        )
        proc = self.run_pr(fixture, BODY_YES)
        self.assertEqual(proc.returncode, 1)
        self.assertIn("unreviewed files", proc.stdout)


class ValidatorUnitTest(unittest.TestCase):
    """Pure-function guards that need no git fixture."""

    @classmethod
    def setUpClass(cls) -> None:
        sys.path.insert(0, str(SCRIPTS_DIR))
        try:
            import publication_sync_check as check
        finally:
            sys.path.remove(str(SCRIPTS_DIR))
        cls.check = check

    def test_packet_shape_rejects_missing_keys(self) -> None:
        with self.assertRaises(self.check.ContractFailure):
            self.check.validate_packet_shape({})

    def test_marker_field_parsing(self) -> None:
        self.assertIs(self.check.check_marker_field(BODY_ABSENT), None)
        self.assertIs(self.check.check_marker_field(BODY_NO), False)
        self.assertIs(self.check.check_marker_field(BODY_YES), True)
        with self.assertRaises(self.check.ContractFailure):
            self.check.check_marker_field(BODY_YES + BODY_NO)


class RatchetTest(unittest.TestCase):
    """Workflow permission/context ratchet and schema/template drift guards."""

    def test_workflow_has_no_publication_authority(self) -> None:
        text = WORKFLOW.read_text(encoding="utf-8")
        self.assertNotIn("write-all", text)
        self.assertNotIn(": write", text)
        self.assertNotIn("secrets.", text)
        self.assertIn("contents: read", text)
        forbidden_commands = (
            "git push", "gh release", "cargo publish", "npm publish",
            "docker push", "gh api", "winget", "scoop", "brew bump",
            "chocolatey", "ovsx", "vsce publish",
        )
        for token in forbidden_commands:
            self.assertNotIn(token, text, f"workflow must not contain {token!r}")

    def test_validator_has_no_publication_command(self) -> None:
        text = CHECK.read_text(encoding="utf-8")
        forbidden_commands = (
            "git push", "gh release", "cargo publish", "npm publish",
            "docker push", "gh api", "vsce publish",
        )
        for token in forbidden_commands:
            self.assertNotIn(token, text, f"validator must not contain {token!r}")

    def test_git_plumbing_preserves_commit_message_bytes(self) -> None:
        text = CHECK.read_text(encoding="utf-8")
        run_git = text[text.index("def run_git"):text.index("def git_ok")]
        self.assertIn("input_bytes: Optional[bytes] = None", text)
        self.assertIn("capture_output=True,\n        input=input_bytes", run_git)
        self.assertIn("input_bytes=message.encode(\"utf-8\")", text)
        self.assertNotIn("text=True", run_git)

    def test_exact_check_context_names(self) -> None:
        text = WORKFLOW.read_text(encoding="utf-8")
        # The required status context is the job name; both workflow and job
        # are pinned to the exact contract name so the ruleset context cannot
        # drift silently.
        self.assertTrue(text.startswith("name: Publication Sync Contract\n"))
        self.assertIn("\n    name: Publication Sync Contract\n", text)

    def test_workflow_reruns_on_body_edits(self) -> None:
        # The PR body carries one sync-mode authority (the marker field), so
        # the workflow must trigger on `edited` or a body flip would leave a
        # stale success on an unchanged head.
        text = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn("types: [opened, edited, reopened, synchronize]", text)

    def test_schema_file_matches_validator(self) -> None:
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        sys.path.insert(0, str(SCRIPTS_DIR))
        try:
            import publication_sync_check as check
        finally:
            sys.path.remove(str(SCRIPTS_DIR))
        self.assertEqual(set(schema["required"]), check.PACKET_KEYS)
        self.assertEqual(set(schema["properties"]), check.PACKET_KEYS)
        self.assertEqual(schema["properties"]["schema_version"]["const"], check.SCHEMA_VERSION)
        self.assertEqual(schema["properties"]["release"]["const"], check.RELEASE)

    def test_pr_template_marker_line_matches_validator_regex(self) -> None:
        template = PR_TEMPLATE.read_text(encoding="utf-8")
        sys.path.insert(0, str(SCRIPTS_DIR))
        try:
            import publication_sync_check as check
        finally:
            sys.path.remove(str(SCRIPTS_DIR))
        matches = list(check.MARKER_FIELD.finditer(template))
        self.assertEqual(len(matches), 1, "template must carry exactly one marker field line")
        self.assertEqual(matches[0].group(1).lower(), "no", "template default must be 'no'")


if __name__ == "__main__":
    unittest.main(verbosity=2)
