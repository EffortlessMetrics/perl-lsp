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

    def test_inline_provider_change_selects_provider_pack_without_quality_pack(self) -> None:
        packs = [
            {
                "id": "patch-coverage-inline-provider-core",
                "files": [
                    "crates/perl-lsp-rs-core/src/providers/inline_completion/",
                ],
                "commands": [
                    "cargo test -p perl-lsp-rs-core --lib inline_completion",
                ],
                "coverage_filters": ["inline_completion"],
            },
            {
                "id": "patch-coverage-xtask-inline-quality",
                "files": [
                    "xtask/src/tasks/inline_completion_quality.rs",
                ],
                "commands": [
                    "cargo run -p xtask -- inline-completion-quality",
                ],
                "coverage_filters": ["inline_completion_quality"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = [
            "crates/perl-lsp-rs-core/src/providers/inline_completion/mod.rs",
        ]

        self.assertEqual(
            ["patch-coverage-inline-provider-core"],
            [pack["id"] for pack in router.selected_packs(packs, paths)],
        )

    def test_inline_quality_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-inline-provider-core",
                "files": [
                    "crates/perl-lsp-rs-core/src/providers/inline_completion/",
                ],
                "commands": [
                    "cargo test -p perl-lsp-rs-core --lib inline_completion",
                ],
                "coverage_filters": ["inline_completion"],
            },
            {
                "id": "patch-coverage-xtask-inline-quality",
                "lcov": False,
                "files": [
                    "xtask/src/tasks/inline_completion_quality.rs",
                ],
                "commands": [
                    "cargo run -p xtask -- inline-completion-quality",
                ],
                "coverage_filters": ["inline_completion_quality"],
            },
        ]

        paths = ["xtask/src/tasks/inline_completion_quality.rs"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-xtask-inline-quality"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_pr_overlap_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-pr-overlap",
                "lcov": False,
                "files": [
                    "scripts/pr_overlap.py",
                    "scripts/tests/test_pr_overlap.py",
                ],
                "commands": ["python scripts/tests/test_pr_overlap.py"],
                "coverage_filters": ["pr_overlap"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/pr_overlap.py"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-pr-overlap"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_control_plane_lock_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-control-plane-lock",
                "lcov": False,
                "files": [
                    "scripts/control-plane-lock.sh",
                    "scripts/test-control-plane-lock.sh",
                ],
                "commands": ["bash scripts/test-control-plane-lock.sh"],
                "coverage_filters": ["control-plane-lock"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/control-plane-lock.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-control-plane-lock"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_agent_preflight_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-agent-preflight",
                "lcov": False,
                "files": [
                    "scripts/agent-preflight.sh",
                    "scripts/test-agent-preflight.sh",
                ],
                "commands": ["bash scripts/test-agent-preflight.sh"],
                "coverage_filters": ["agent-preflight"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/agent-preflight.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-agent-preflight"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_coverage_baseline_script_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-baseline-script",
                "lcov": False,
                "files": [
                    "scripts/check-coverage-baseline.sh",
                    "scripts/tests/test-check-coverage-baseline.sh",
                ],
                "commands": ["bash scripts/tests/test-check-coverage-baseline.sh"],
                "coverage_filters": ["check-coverage-baseline"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/check-coverage-baseline.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-baseline-script"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )


if __name__ == "__main__":
    unittest.main()
