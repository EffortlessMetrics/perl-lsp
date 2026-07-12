from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from check_release_container_actuals import (  # noqa: E402
    ContainerActualsError,
    load_manifest,
    validate_manifest,
    validate_notes,
)


NOTE_VALUE = (
    "Docker Hub amd64/arm64 verified; "
    "GHCR builder/runtime tags exist but are arm64-only"
)
DOCKER_DIGEST = "sha256:" + "a" * 64
RUNTIME_DIGEST = "sha256:" + "b" * 64


def valid_manifest() -> dict:
    return {
        "schema_version": 1,
        "audited_at": "2026-07-12",
        "repository": "EffortlessMetrics/perl-lsp",
        "evidence_runs": [29192188862, 29192323459],
        "releases": [
            {
                "version": "0.15.2",
                "note_channel_value": NOTE_VALUE,
                "docker_hub": {
                    "builder": {
                        "tag": "0.15.2",
                        "pushed_at": "2026-05-26T22:51:53.017230Z",
                        "digest": DOCKER_DIGEST,
                        "platforms": ["linux/amd64", "linux/arm64"],
                    },
                    "runtime": {
                        "tag": "0.15.2-perl",
                        "pushed_at": "2026-05-26T22:05:51.263719Z",
                        "digest": RUNTIME_DIGEST,
                        "platforms": ["linux/amd64", "linux/arm64"],
                    },
                },
                "ghcr": {
                    "builder": {
                        "package": "perl-lsp",
                        "created_at": "2026-05-26T22:59:06Z",
                        "platforms": ["linux/arm64"],
                    },
                    "runtime": {
                        "package": "perl-lsp-perl",
                        "created_at": "2026-05-26T21:55:00Z",
                        "platforms": ["linux/arm64"],
                    },
                },
            }
        ],
    }


def write_note(root: Path, *, version: str = "0.15.2", docker: str = NOTE_VALUE) -> None:
    releases = root / "docs" / "releases"
    releases.mkdir(parents=True, exist_ok=True)
    (releases / "v0.15.2.md").write_text(
        "\n".join(
            [
                "---",
                f'version: "{version}"',
                "channels:",
                f'  docker: "{docker}"',
                "---",
                "",
                "# v0.15.2",
                "",
            ]
        ),
        encoding="utf-8",
    )


class ManifestValidationTests(unittest.TestCase):
    def test_valid_manifest(self) -> None:
        self.assertEqual([], validate_manifest(valid_manifest()))

    def test_duplicate_version_is_rejected(self) -> None:
        manifest = valid_manifest()
        manifest["releases"].append(deepcopy(manifest["releases"][0]))
        errors = validate_manifest(manifest)
        self.assertTrue(any("duplicate release version" in error for error in errors))

    def test_invalid_digest_is_rejected(self) -> None:
        manifest = valid_manifest()
        manifest["releases"][0]["docker_hub"]["builder"]["digest"] = "sha256:short"
        errors = validate_manifest(manifest)
        self.assertTrue(any("must be a sha256 digest" in error for error in errors))

    def test_incomplete_docker_hub_platforms_are_rejected(self) -> None:
        manifest = valid_manifest()
        manifest["releases"][0]["docker_hub"]["runtime"]["platforms"] = [
            "linux/arm64"
        ]
        errors = validate_manifest(manifest)
        self.assertTrue(any("linux/amd64" in error for error in errors))

    def test_ghcr_amd64_claim_is_rejected_for_historical_actuals(self) -> None:
        manifest = valid_manifest()
        manifest["releases"][0]["ghcr"]["builder"]["platforms"] = [
            "linux/amd64",
            "linux/arm64",
        ]
        errors = validate_manifest(manifest)
        self.assertTrue(any("ghcr.builder.platforms" in error for error in errors))

    def test_pending_note_value_is_rejected(self) -> None:
        manifest = valid_manifest()
        manifest["releases"][0]["note_channel_value"] = "pending"
        errors = validate_manifest(manifest)
        self.assertTrue(any("resolved string" in error for error in errors))

    def test_load_manifest_rejects_invalid_json(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "actuals.json"
            path.write_text("{not json", encoding="utf-8")
            with self.assertRaises(ContainerActualsError):
                load_manifest(path)


class NoteValidationTests(unittest.TestCase):
    def test_valid_note(self) -> None:
        manifest = valid_manifest()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            write_note(root)
            self.assertEqual([], validate_notes(manifest, root))

    def test_pending_note_is_rejected(self) -> None:
        manifest = valid_manifest()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            write_note(root, docker="pending")
            errors = validate_notes(manifest, root)
            self.assertTrue(any("docker channel mismatch" in error for error in errors))

    def test_wrong_note_version_is_rejected(self) -> None:
        manifest = valid_manifest()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            write_note(root, version="0.15.1")
            errors = validate_notes(manifest, root)
            self.assertTrue(any("version mismatch" in error for error in errors))

    def test_missing_note_is_rejected(self) -> None:
        manifest = valid_manifest()
        with tempfile.TemporaryDirectory() as temp:
            errors = validate_notes(manifest, Path(temp))
            self.assertTrue(any("cannot read" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
