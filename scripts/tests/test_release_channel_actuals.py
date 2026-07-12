from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from check_release_channel_actuals import (  # noqa: E402
    load_manifest,
    validate_manifest,
    validate_notes,
)


SHA_150 = "a" * 40
SHA_152 = "b" * 40


def valid_manifest() -> dict:
    return {
        "schema_version": 1,
        "audited_at": "2026-07-12",
        "repository": "EffortlessMetrics/perl-lsp",
        "github_releases": [
            {
                "version": "0.15.0",
                "tag": "v0.15.0",
                "tag_commit": SHA_150,
                "published_date_utc": "2026-05-22",
                "release_url": "https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.15.0",
            },
            {
                "version": "0.15.2",
                "tag": "v0.15.2",
                "tag_commit": SHA_152,
                "published_date_utc": "2026-05-26",
                "release_url": "https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.15.2",
                "closeout_receipt": "docs/releases/0.15.2-closeout-audit.md",
            },
        ],
    }


def write_note(root: Path, record: dict, *, github_status: str = "published") -> Path:
    releases = root / "docs" / "releases"
    releases.mkdir(parents=True, exist_ok=True)
    version = record["version"]
    status = "canonical" if record.get("closeout_receipt") else "draft"
    path = releases / f"v{version}.md"
    path.write_text(
        "\n".join(
            [
                "---",
                f'version: "{version}"',
                f'tag: "{record["tag"]}"',
                f'tag_commit: "{record["tag_commit"]}"',
                f'release_date_utc: "{record["published_date_utc"]}"',
                f'github_release: "{record["release_url"]}"',
                f"notes_status: {status}",
                "channels:",
                f'  github_release: "{github_status}"',
                "  crates_io: pending",
                "---",
                "",
                f"# v{version}",
                "",
            ]
        ),
        encoding="utf-8",
    )
    return path


def write_valid_tree(root: Path, manifest: dict) -> None:
    for record in manifest["github_releases"]:
        write_note(root, record)
        receipt = record.get("closeout_receipt")
        if receipt:
            receipt_path = root / receipt
            receipt_path.parent.mkdir(parents=True, exist_ok=True)
            receipt_path.write_text("# Closeout\n", encoding="utf-8")


class ManifestValidationTests(unittest.TestCase):
    def test_valid_manifest(self) -> None:
        self.assertEqual([], validate_manifest(valid_manifest()))

    def test_duplicate_version_is_rejected(self) -> None:
        manifest = valid_manifest()
        duplicate = deepcopy(manifest["github_releases"][0])
        duplicate["tag"] = "v0.15.1"
        duplicate["version"] = "0.15.0"
        manifest["github_releases"].append(duplicate)
        errors = validate_manifest(manifest)
        self.assertTrue(any("duplicate release version" in error for error in errors))

    def test_malformed_sha_is_rejected(self) -> None:
        manifest = valid_manifest()
        manifest["github_releases"][0]["tag_commit"] = "abc123"
        errors = validate_manifest(manifest)
        self.assertTrue(any("40-hex" in error for error in errors))

    def test_noncanonical_release_url_is_rejected(self) -> None:
        manifest = valid_manifest()
        manifest["github_releases"][0]["release_url"] = "https://example.invalid"
        errors = validate_manifest(manifest)
        self.assertTrue(any("canonical tag release URL" in error for error in errors))

    def test_load_manifest_rejects_invalid_json(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "actuals.json"
            path.write_text("{not json", encoding="utf-8")
            with self.assertRaises(Exception):
                load_manifest(path)


class NoteValidationTests(unittest.TestCase):
    def test_valid_notes_and_receipt(self) -> None:
        manifest = valid_manifest()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            write_valid_tree(root, manifest)
            self.assertEqual([], validate_notes(manifest, root))

    def test_pending_github_release_is_rejected(self) -> None:
        manifest = valid_manifest()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            write_valid_tree(root, manifest)
            write_note(root, manifest["github_releases"][0], github_status="pending")
            errors = validate_notes(manifest, root)
            self.assertTrue(any("regressed verified GitHub Release" in error for error in errors))

    def test_wrong_tag_commit_is_rejected(self) -> None:
        manifest = valid_manifest()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            write_valid_tree(root, manifest)
            path = root / "docs" / "releases" / "v0.15.0.md"
            text = path.read_text(encoding="utf-8")
            path.write_text(text.replace(SHA_150, "f" * 40), encoding="utf-8")
            errors = validate_notes(manifest, root)
            self.assertTrue(any("tag_commit mismatch" in error for error in errors))

    def test_missing_closeout_receipt_is_rejected(self) -> None:
        manifest = valid_manifest()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            write_valid_tree(root, manifest)
            (root / "docs" / "releases" / "0.15.2-closeout-audit.md").unlink()
            errors = validate_notes(manifest, root)
            self.assertTrue(any("closeout receipt missing" in error for error in errors))

    def test_closed_note_must_be_canonical(self) -> None:
        manifest = valid_manifest()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            write_valid_tree(root, manifest)
            path = root / "docs" / "releases" / "v0.15.2.md"
            text = path.read_text(encoding="utf-8")
            path.write_text(
                text.replace("notes_status: canonical", "notes_status: draft"),
                encoding="utf-8",
            )
            errors = validate_notes(manifest, root)
            self.assertTrue(any("notes_status: canonical" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
