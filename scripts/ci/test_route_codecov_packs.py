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

    def test_preflight_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-preflight-wrapper",
                "lcov": False,
                "files": [
                    "scripts/preflight.sh",
                    "scripts/tests/test-preflight-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-preflight-wrapper.sh"],
                "coverage_filters": ["preflight-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/preflight.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-preflight-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_install_githooks_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-install-githooks-wrapper",
                "lcov": False,
                "files": [
                    "scripts/install-githooks.sh",
                    "scripts/tests/test-install-githooks-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-install-githooks-wrapper.sh"],
                "coverage_filters": ["install-githooks-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/install-githooks.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-install-githooks-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_e2e_gate_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-e2e-gate-wrapper",
                "lcov": False,
                "files": [
                    "scripts/e2e-gate.sh",
                    "scripts/tests/test-e2e-gate-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-e2e-gate-wrapper.sh"],
                "coverage_filters": ["e2e-gate-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/e2e-gate.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-e2e-gate-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_execute_gate_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-execute-gate-wrapper",
                "lcov": False,
                "files": [
                    "scripts/execute-gate.sh",
                    "scripts/tests/test-execute-gate-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-execute-gate-wrapper.sh"],
                "coverage_filters": ["execute-gate-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/execute-gate.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-execute-gate-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_run_gates_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-run-gates-wrapper",
                "lcov": False,
                "files": [
                    "scripts/run-gates.sh",
                    "scripts/tests/test-run-gates-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-run-gates-wrapper.sh"],
                "coverage_filters": ["run-gates-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/run-gates.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-run-gates-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_gate_local_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-gate-local-wrapper",
                "lcov": False,
                "files": [
                    "scripts/gate-local.sh",
                    "scripts/tests/test-gate-local-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-gate-local-wrapper.sh"],
                "coverage_filters": ["gate-local-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/gate-local.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-gate-local-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_list_gates_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-list-gates-wrapper",
                "lcov": False,
                "files": [
                    "scripts/list-gates.py",
                    "scripts/tests/test-list-gates-wrapper.py",
                ],
                "commands": ["python scripts/tests/test-list-gates-wrapper.py"],
                "coverage_filters": ["list-gates-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/list-gates.py"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-list-gates-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_forbid_fatal_constructs_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-forbid-fatal-constructs-wrapper",
                "lcov": False,
                "files": [
                    "scripts/forbid-fatal-constructs.sh",
                    "scripts/tests/test-forbid-fatal-constructs-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-forbid-fatal-constructs-wrapper.sh"],
                "coverage_filters": ["forbid-fatal-constructs-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/forbid-fatal-constructs.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-forbid-fatal-constructs-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_dead_code_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-dead-code-wrapper",
                "lcov": False,
                "files": [
                    "scripts/dead-code-check.sh",
                    "scripts/tests/test-dead-code-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-dead-code-wrapper.sh"],
                "coverage_filters": ["dead-code-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/dead-code-check.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-dead-code-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_check_toolchain_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-check-toolchain-wrapper",
                "lcov": False,
                "files": [
                    "scripts/check-rust-toolchain.sh",
                    "scripts/tests/test-check-rust-toolchain-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-check-rust-toolchain-wrapper.sh"],
                "coverage_filters": ["check-toolchain-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/check-rust-toolchain.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-check-toolchain-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_devex_doctor_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-devex-doctor-wrapper",
                "lcov": False,
                "files": [
                    "scripts/devex-doctor.sh",
                    "scripts/tests/test-devex-doctor-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-devex-doctor-wrapper.sh"],
                "coverage_filters": ["devex-doctor-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/devex-doctor.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-devex-doctor-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_devex_targeted_checks_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-devex-targeted-checks-wrapper",
                "lcov": False,
                "files": [
                    "scripts/devex-targeted-checks.sh",
                    "scripts/tests/test-devex-targeted-checks-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-devex-targeted-checks-wrapper.sh"],
                "coverage_filters": ["devex-targeted-checks-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/devex-targeted-checks.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-devex-targeted-checks-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_lsp_cancellation_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-lsp-cancellation-wrapper",
                "lcov": False,
                "files": [
                    "scripts/test-lsp-cancellation.sh",
                    "scripts/tests/test-lsp-cancellation-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-lsp-cancellation-wrapper.sh"],
                "coverage_filters": ["lsp-cancellation-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/test-lsp-cancellation.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-lsp-cancellation-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_test_capped_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-test-capped-wrapper",
                "lcov": False,
                "files": [
                    "scripts/test-capped.sh",
                    "scripts/tests/test-test-capped-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-test-capped-wrapper.sh"],
                "coverage_filters": ["test-capped-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/test-capped.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-test-capped-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_test_e2e_capped_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-test-e2e-capped-wrapper",
                "lcov": False,
                "files": [
                    "scripts/test-e2e-capped.sh",
                    "scripts/tests/test-test-e2e-capped-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-test-e2e-capped-wrapper.sh"],
                "coverage_filters": ["test-e2e-capped-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/test-e2e-capped.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-test-e2e-capped-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_build_timing_receipt_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-build-timing-receipt-wrapper",
                "lcov": False,
                "files": [
                    "scripts/build-timing-receipt.sh",
                    "scripts/tests/test-build-timing-receipt-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-build-timing-receipt-wrapper.sh"],
                "coverage_filters": ["build-timing-receipt-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/build-timing-receipt.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-build-timing-receipt-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_compare_build_timing_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-compare-build-timing-wrapper",
                "lcov": False,
                "files": [
                    "scripts/compare-build-timing.sh",
                    "scripts/tests/test-compare-build-timing-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-compare-build-timing-wrapper.sh"],
                "coverage_filters": ["compare-build-timing-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/compare-build-timing.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-compare-build-timing-wrapper"],
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

    def test_update_coverage_baseline_script_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-update-baseline-script",
                "lcov": False,
                "files": [
                    "scripts/update-coverage-baseline.sh",
                    "scripts/tests/test-update-coverage-baseline.sh",
                ],
                "commands": ["bash scripts/tests/test-update-coverage-baseline.sh"],
                "coverage_filters": ["update-coverage-baseline"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/update-coverage-baseline.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-update-baseline-script"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_generate_receipt_script_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-generate-receipt-script",
                "lcov": False,
                "files": [
                    "scripts/generate-receipt.sh",
                    "scripts/tests/test-generate-receipt.sh",
                ],
                "commands": ["bash scripts/tests/test-generate-receipt.sh"],
                "coverage_filters": ["generate-receipt"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/generate-receipt.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-generate-receipt-script"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_quick_receipts_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-quick-receipts-wrapper",
                "lcov": False,
                "files": [
                    "scripts/quick-receipts.sh",
                    "scripts/tests/test-quick-receipts-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-quick-receipts-wrapper.sh"],
                "coverage_filters": ["quick-receipts"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/quick-receipts.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-quick-receipts-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_publish_receipts_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-publish-receipts-wrapper",
                "lcov": False,
                "files": [
                    "scripts/publish-receipts.sh",
                    "scripts/tests/test-publish-receipts-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-publish-receipts-wrapper.sh"],
                "coverage_filters": ["publish-receipts-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/publish-receipts.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-publish-receipts-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_generate_badges_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-generate-badges-wrapper",
                "lcov": False,
                "files": [
                    "scripts/generate-badges.sh",
                    "scripts/tests/test-generate-badges-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-generate-badges-wrapper.sh"],
                "coverage_filters": ["generate-badges"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/generate-badges.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-generate-badges-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_ignored_test_count_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-ignored-test-count-wrapper",
                "lcov": False,
                "files": [
                    "scripts/ignored-test-count.sh",
                    "scripts/tests/test-ignored-test-count-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-ignored-test-count-wrapper.sh"],
                "coverage_filters": ["ignored-test-count"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/ignored-test-count.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-ignored-test-count-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_remaining_script_helper_changes_are_non_lcov_focused_proof(self) -> None:
        helper_packs = [
            (
                "patch-coverage-clean-tmp-targets",
                ["scripts/clean-tmp-targets.sh", "scripts/tests/test-clean-tmp-targets.sh"],
                ["bash scripts/tests/test-clean-tmp-targets.sh"],
                ["clean-tmp-targets"],
                "scripts/clean-tmp-targets.sh",
            ),
            (
                "patch-coverage-swarm-cleanup",
                [
                    "scripts/swarm-clean",
                    "scripts/swarm-doctor",
                    "scripts/tests/test_swarm_clean.sh",
                    "scripts/tests/test_swarm_doctor.sh",
                ],
                [
                    "bash scripts/tests/test_swarm_clean.sh",
                    "bash scripts/tests/test_swarm_doctor.sh",
                ],
                ["swarm-cleanup"],
                "scripts/swarm-clean",
            ),
            (
                "patch-coverage-pre-merge-check",
                ["scripts/pre-merge-check.sh", "scripts/tests/test-pre-merge-check.sh"],
                ["bash scripts/tests/test-pre-merge-check.sh"],
                ["pre-merge-check"],
                "scripts/pre-merge-check.sh",
            ),
        ]

        for pack_id, files, commands, coverage_filters, changed_file in helper_packs:
            with self.subTest(pack_id=pack_id):
                packs = [
                    {
                        "id": pack_id,
                        "lcov": False,
                        "files": files,
                        "commands": commands,
                        "coverage_filters": coverage_filters,
                    },
                    {
                        "id": router.FALLBACK_PACK_ID,
                        "files": ["*.rs"],
                        "commands": ["cargo test --workspace --lib"],
                        "coverage_filters": ["workspace-lib"],
                    },
                ]

                paths = [changed_file]

                self.assertEqual([], router.selected_packs(packs, paths))
                self.assertEqual(
                    [pack_id],
                    [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
                )


if __name__ == "__main__":
    unittest.main()
