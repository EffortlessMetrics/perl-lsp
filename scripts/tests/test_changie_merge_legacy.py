from __future__ import annotations

from pathlib import Path
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from changie_merge_legacy import ChangelogMergeError, merge_version  # noqa: E402


class ChangieLegacyMergeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        self.changes = self.root / ".changes"
        self.changes.mkdir()
        self.changelog = self.root / "CHANGELOG.md"
        self.legacy_tail = (
            "## [0.17.0] - 2026-06-28\n\n"
            "### Fixed\n\n"
            "- Historical text remains unchanged.\n"
        )
        self.changelog.write_text(
            "# Changelog\n\n## [Unreleased]\n\n" + self.legacy_tail,
            encoding="utf-8",
        )

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def write_release(self, version: str = "v0.18.0") -> None:
        (self.changes / f"{version}.md").write_text(
            "## [0.18.0] - 2026-07-12\n\n"
            "### Changed\n\n"
            "- Changie now owns future release fragments.\n",
            encoding="utf-8",
        )

    def test_inserts_release_before_legacy_tail(self) -> None:
        self.write_release()

        merge_version(
            changelog_path=self.changelog,
            changes_dir=self.changes,
            raw_version="v0.18.0",
        )

        rendered = self.changelog.read_text(encoding="utf-8")
        self.assertLess(rendered.index("## [0.18.0]"), rendered.index("## [0.17.0]"))
        self.assertTrue(rendered.endswith(self.legacy_tail))

    def test_preserves_crlf_legacy_tail_byte_for_byte(self) -> None:
        legacy_tail = (
            "## [0.17.0] - 2026-06-28\r\n\r\n"
            "### Fixed\r\n\r\n"
            "- Historical café text remains unchanged.\r\n"
        ).encode("utf-8")
        self.changelog.write_bytes(
            b"# Changelog\r\n\r\n## [Unreleased]\r\n\r\n" + legacy_tail
        )
        self.write_release()

        merge_version(
            changelog_path=self.changelog,
            changes_dir=self.changes,
            raw_version="v0.18.0",
        )

        rendered = self.changelog.read_bytes()
        self.assertTrue(rendered.endswith(legacy_tail))
        self.assertIn(
            b"## [0.18.0] - 2026-07-12\r\n\r\n### Changed",
            rendered,
        )
        self.assertEqual(rendered.count(b"\n"), rendered.count(b"\r\n"))

    def test_rejects_duplicate_release(self) -> None:
        self.write_release()
        merge_version(
            changelog_path=self.changelog,
            changes_dir=self.changes,
            raw_version="0.18.0",
        )

        with self.assertRaisesRegex(ChangelogMergeError, "already contains"):
            merge_version(
                changelog_path=self.changelog,
                changes_dir=self.changes,
                raw_version="0.18.0",
            )

    def test_rejects_mismatched_version_file_heading(self) -> None:
        (self.changes / "v0.18.0.md").write_text(
            "## [0.19.0] - 2026-07-12\n",
            encoding="utf-8",
        )

        with self.assertRaisesRegex(ChangelogMergeError, "must begin"):
            merge_version(
                changelog_path=self.changelog,
                changes_dir=self.changes,
                raw_version="v0.18.0",
            )


if __name__ == "__main__":
    unittest.main()
