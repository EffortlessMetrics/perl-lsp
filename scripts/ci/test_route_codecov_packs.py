#!/usr/bin/env python3
"""Focused tests for the Codecov coverage-pack router."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest


SCRIPT_PATH = Path(__file__).with_name("route-codecov-packs.py")
SPEC = importlib.util.spec_from_file_location("route_codecov_packs", SCRIPT_PATH)
assert SPEC is not None
router = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(router)


class RouteCodecovPacksTests(unittest.TestCase):
    def test_non_lcov_router_script_change_is_skipped_by_policy(self) -> None:
        packs = [
            {
                "id": "patch-coverage-ci-route",
                "lcov": False,
                "files": [
                    "scripts/ci/route-codecov-packs.py",
                    "scripts/ci/test_route_codecov_packs.py",
                ],
                "commands": ["python -m unittest scripts/ci/test_route_codecov_packs.py"],
                "coverage_filters": ["ci_route"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/ci/route-codecov-packs.py"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-ci-route"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_lcov_pack_matching_only_test_files_is_reported_not_selected(self) -> None:
        packs = [
            {
                "id": "patch-coverage-xtask-semantic-inline",
                "files": [
                    "xtask/src/tasks/semantic_inline_receipts.rs",
                    "xtask/tests/semantic_inline_receipts_cli.rs",
                ],
                "commands": ["cargo test -p xtask semantic_inline_receipts"],
                "coverage_filters": ["semantic_inline_receipts"],
            },
        ]

        paths = ["xtask/tests/semantic_inline_receipts_cli.rs"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-xtask-semantic-inline"],
            [pack["id"] for pack in router.lcov_matches_without_source(packs, paths)],
        )

    def test_lcov_source_change_selects_matching_pack(self) -> None:
        packs = [
            {
                "id": "patch-coverage-xtask-semantic-inline",
                "files": [
                    "xtask/src/tasks/semantic_inline_receipts.rs",
                    "xtask/tests/semantic_inline_receipts_cli.rs",
                ],
                "commands": ["cargo test -p xtask semantic_inline_receipts"],
                "coverage_filters": ["semantic_inline_receipts"],
            },
        ]

        paths = ["xtask/src/tasks/semantic_inline_receipts.rs"]

        self.assertEqual(
            ["patch-coverage-xtask-semantic-inline"],
            [pack["id"] for pack in router.selected_packs(packs, paths)],
        )
        self.assertEqual([], router.lcov_matches_without_source(packs, paths))

    def test_completion_provider_change_selects_completion_pack(self) -> None:
        packs = [
            {
                "id": "patch-coverage-completion-core",
                "files": [
                    "crates/perl-lsp-rs-core/src/providers/completion/",
                ],
                "commands": [
                    "cargo test -p perl-lsp-rs-core --lib completion::completion",
                ],
                "coverage_filters": ["completion::completion"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = [
            "crates/perl-lsp-rs-core/src/providers/completion/completion/import_map/used_modules.rs",
        ]

        self.assertEqual(
            ["patch-coverage-completion-core"],
            [pack["id"] for pack in router.selected_packs(packs, paths)],
        )
        self.assertEqual([], router.lcov_matches_without_source(packs, paths))


if __name__ == "__main__":
    unittest.main()
