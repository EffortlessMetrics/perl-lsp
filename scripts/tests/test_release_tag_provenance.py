from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from check_release_tag_provenance import (  # noqa: E402
    validate_manifest,
    verify_git_refs,
)


SHA_A = "1" * 40
SHA_B = "2" * 40


def valid_manifest() -> dict:
    return {
        "schema_version": 1,
        "repository": "example/project",
        "audited_at": "2026-07-12",
        "tag": [
            {
                "name": "v0.1.0",
                "current_sha": SHA_A,
                "record_status": "match",
                "recorded_sha": SHA_A[:8],
                "recorded_reachable": True,
                "lineage": "root",
            },
            {
                "name": "v0.2.0",
                "current_sha": SHA_B,
                "record_status": "pending",
                "predecessor": "v0.1.0",
                "lineage": "linear",
            },
        ],
        "missing_tag": [
            {
                "version": "0.1.1",
                "status": "never-cut",
                "note": "fixture",
            }
        ],
    }


class ManifestValidationTests(unittest.TestCase):
    def test_valid_manifest(self) -> None:
        self.assertEqual([], validate_manifest(valid_manifest()))

    def test_duplicate_tag_is_rejected(self) -> None:
        manifest = valid_manifest()
        manifest["tag"].append(deepcopy(manifest["tag"][0]))
        errors = validate_manifest(manifest)
        self.assertTrue(any("duplicate tag record" in error for error in errors))

    def test_malformed_sha_is_rejected(self) -> None:
        manifest = valid_manifest()
        manifest["tag"][0]["current_sha"] = "abc123"
        errors = validate_manifest(manifest)
        self.assertTrue(any("40-hex" in error for error in errors))

    def test_unknown_predecessor_is_rejected(self) -> None:
        manifest = valid_manifest()
        manifest["tag"][1]["predecessor"] = "v9.9.9"
        errors = validate_manifest(manifest)
        self.assertTrue(any("unknown predecessor" in error for error in errors))

    def test_record_status_must_match_sha_relationship(self) -> None:
        manifest = valid_manifest()
        manifest["tag"][0]["record_status"] = "stale"
        manifest["tag"][0]["recorded_reachable"] = False
        errors = validate_manifest(manifest)
        self.assertTrue(any("marked stale" in error for error in errors))


class GitAvailabilityTests(unittest.TestCase):
    @patch("check_release_tag_provenance.shutil.which", return_value=None)
    def test_missing_git_returns_actionable_error(self, _which: object) -> None:
        self.assertEqual(
            ["git executable not found on PATH"],
            verify_git_refs({"tag": []}, Path(".")),
        )


@unittest.skipUnless(shutil.which("git"), "git executable not found on PATH")
class GitVerificationTests(unittest.TestCase):
    def run_git(self, root: Path, *args: str) -> str:
        result = subprocess.run(
            ["git", "-C", str(root), *args],
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        return result.stdout.strip()

    def write_commit(self, root: Path, value: str) -> str:
        (root / "value.txt").write_text(value, encoding="utf-8")
        self.run_git(root, "add", "value.txt")
        self.run_git(root, "commit", "-m", value)
        return self.run_git(root, "rev-parse", "HEAD")

    def init_repo(self, root: Path) -> None:
        self.run_git(root, "init")
        self.run_git(root, "config", "user.name", "Tag Provenance Test")
        self.run_git(root, "config", "user.email", "test@example.invalid")

    def test_local_ref_drift_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.init_repo(root)
            first = self.write_commit(root, "first")
            self.run_git(root, "tag", "v0.1.0")
            second = self.write_commit(root, "second")
            self.run_git(root, "tag", "v0.2.0")

            manifest = {
                "tag": [
                    {
                        "name": "v0.1.0",
                        "current_sha": first,
                        "lineage": "root",
                    },
                    {
                        "name": "v0.2.0",
                        "current_sha": "f" * 40,
                        "predecessor": "v0.1.0",
                        "lineage": "linear",
                    },
                ]
            }
            errors = verify_git_refs(manifest, root)
            self.assertTrue(any("v0.2.0 drifted" in error for error in errors))
            self.assertNotEqual(second, "f" * 40)

    def test_unlisted_local_release_tag_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.init_repo(root)
            first = self.write_commit(root, "first")
            self.run_git(root, "tag", "v0.1.0")
            second = self.write_commit(root, "second")
            self.run_git(root, "tag", "v0.2.0")
            self.write_commit(root, "third")
            self.run_git(root, "tag", "v0.3.0")

            manifest = {
                "tag": [
                    {
                        "name": "v0.1.0",
                        "current_sha": first,
                        "lineage": "root",
                    },
                    {
                        "name": "v0.2.0",
                        "current_sha": second,
                        "predecessor": "v0.1.0",
                        "lineage": "linear",
                    },
                ]
            }
            errors = verify_git_refs(manifest, root)
            self.assertIn(
                "local release tag is missing from manifest: v0.3.0",
                errors,
            )

    def test_diverged_lineage_is_accepted_when_neither_ref_is_ancestral(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.init_repo(root)
            base = self.write_commit(root, "base")

            self.run_git(root, "checkout", "-b", "old-line")
            old = self.write_commit(root, "old")
            self.run_git(root, "tag", "v0.1.0")

            self.run_git(root, "checkout", "--detach", base)
            self.run_git(root, "checkout", "-b", "new-line")
            new = self.write_commit(root, "new")
            self.run_git(root, "tag", "v0.2.0")

            manifest = {
                "tag": [
                    {
                        "name": "v0.1.0",
                        "current_sha": old,
                        "lineage": "root",
                    },
                    {
                        "name": "v0.2.0",
                        "current_sha": new,
                        "predecessor": "v0.1.0",
                        "lineage": "diverged",
                    },
                ]
            }
            self.assertEqual([], verify_git_refs(manifest, root))


if __name__ == "__main__":
    unittest.main()
