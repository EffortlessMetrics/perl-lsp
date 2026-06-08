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

    def test_mixed_completion_general_rust_and_non_lcov_selects_fallback(self) -> None:
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

        paths = [
            "crates/perl-lsp-rs-core/src/providers/completion/completion.rs",
            "crates/perl-module/src/import/mod.rs",
            "scripts/ci/route-codecov-packs.py",
        ]

        self.assertEqual(
            ["patch-coverage-completion-core", router.FALLBACK_PACK_ID],
            [pack["id"] for pack in router.selected_packs(packs, paths)],
        )

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

    def test_ci_audit_workflows_shim_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-ci-audit-workflows-shim",
                "lcov": False,
                "files": [
                    "scripts/ci-audit-workflows.py",
                    "scripts/tests/test-ci-audit-workflows-shim.py",
                ],
                "commands": ["python scripts/tests/test-ci-audit-workflows-shim.py"],
                "coverage_filters": ["ci-audit-workflows-shim"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/ci-audit-workflows.py"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-ci-audit-workflows-shim"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_doc_claims_shim_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-doc-claims-shim",
                "lcov": False,
                "files": [
                    "scripts/check-doc-claims.py",
                    "scripts/tests/test-check-doc-claims-shim.py",
                ],
                "commands": ["python scripts/tests/test-check-doc-claims-shim.py"],
                "coverage_filters": ["doc-claims-shim"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/check-doc-claims.py"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-doc-claims-shim"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_features_invariants_shim_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-features-invariants-shim",
                "lcov": False,
                "files": [
                    "scripts/check_features_invariants.py",
                    "scripts/tests/test-check-features-invariants-shim.py",
                ],
                "commands": ["python scripts/tests/test-check-features-invariants-shim.py"],
                "coverage_filters": ["features-invariants-shim"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/check_features_invariants.py"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-features-invariants-shim"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_debt_report_shim_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-debt-report-shim",
                "lcov": False,
                "files": [
                    "scripts/debt-report.py",
                    "scripts/tests/test-debt-report-shim.py",
                ],
                "commands": ["python scripts/tests/test-debt-report-shim.py"],
                "coverage_filters": ["debt-report-shim"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/debt-report.py"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-debt-report-shim"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_debt_pr_summary_shim_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-debt-pr-summary-shim",
                "lcov": False,
                "files": [
                    "scripts/debt-pr-summary.py",
                    "scripts/tests/test-debt-pr-summary-shim.py",
                ],
                "commands": ["python scripts/tests/test-debt-pr-summary-shim.py"],
                "coverage_filters": ["debt-pr-summary-shim"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/debt-pr-summary.py"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-debt-pr-summary-shim"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_update_current_status_shim_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-update-current-status-shim",
                "lcov": False,
                "files": [
                    "scripts/update-current-status.py",
                    "scripts/tests/test-update-current-status-shim.py",
                ],
                "commands": ["python scripts/tests/test-update-current-status-shim.py"],
                "coverage_filters": ["update-current-status-shim"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/update-current-status.py"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-update-current-status-shim"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_update_parser_matrix_shim_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-update-parser-matrix-shim",
                "lcov": False,
                "files": [
                    "scripts/update-parser-matrix.py",
                    "scripts/tests/test-update-parser-matrix-shim.py",
                ],
                "commands": ["python scripts/tests/test-update-parser-matrix-shim.py"],
                "coverage_filters": ["update-parser-matrix-shim"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/update-parser-matrix.py"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-update-parser-matrix-shim"],
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

    def test_ci_cost_monitor_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-ci-cost-monitor-wrapper",
                "lcov": False,
                "files": [
                    "scripts/ci-cost-monitor.sh",
                    "scripts/tests/test-ci-cost-monitor-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-ci-cost-monitor-wrapper.sh"],
                "coverage_filters": ["ci-cost-monitor-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/ci-cost-monitor.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-ci-cost-monitor-wrapper"],
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

    def test_swarm_summary_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-swarm-summary-wrapper",
                "lcov": False,
                "files": [
                    "scripts/swarm-summary.sh",
                    "scripts/tests/test-swarm-summary-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-swarm-summary-wrapper.sh"],
                "coverage_filters": ["swarm-summary-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/swarm-summary.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-swarm-summary-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_workspace_exclusions_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-workspace-exclusions-wrapper",
                "lcov": False,
                "files": [
                    "scripts/validate-workspace-exclusions.sh",
                    "scripts/tests/test-validate-workspace-exclusions-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-validate-workspace-exclusions-wrapper.sh"],
                "coverage_filters": ["workspace-exclusions-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/validate-workspace-exclusions.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-workspace-exclusions-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_worktree_cleanup_wrapper_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-worktree-cleanup-wrapper",
                "lcov": False,
                "files": [
                    "scripts/cleanup-worktrees.sh",
                    "scripts/tests/test-cleanup-worktrees-wrapper.sh",
                ],
                "commands": ["bash scripts/tests/test-cleanup-worktrees-wrapper.sh"],
                "coverage_filters": ["worktree-cleanup-wrapper"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/cleanup-worktrees.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-worktree-cleanup-wrapper"],
            [pack["id"] for pack in router.non_lcov_matches(packs, paths)],
        )

    def test_cleanup_completed_worktrees_change_is_non_lcov_focused_proof(self) -> None:
        packs = [
            {
                "id": "patch-coverage-cleanup-completed-worktrees",
                "lcov": False,
                "files": [
                    "scripts/cleanup-completed-worktrees.sh",
                    "scripts/tests/test-cleanup-completed-worktrees.sh",
                ],
                "commands": ["bash scripts/tests/test-cleanup-completed-worktrees.sh"],
                "coverage_filters": ["cleanup-completed-worktrees"],
            },
            {
                "id": router.FALLBACK_PACK_ID,
                "files": ["*.rs"],
                "commands": ["cargo test --workspace --lib"],
                "coverage_filters": ["workspace-lib"],
            },
        ]

        paths = ["scripts/cleanup-completed-worktrees.sh"]

        self.assertEqual([], router.selected_packs(packs, paths))
        self.assertEqual(
            ["patch-coverage-cleanup-completed-worktrees"],
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


class IntegrationTestAugmentationTests(unittest.TestCase):
    """Regression guard for PR #1212 / #1217.

    DAP-style crates prove patch coverage through integration tests, not
    lib tests.  The rust-focused fallback pack must include a per-crate
    ``--tests`` command for every crate that owns a changed source file.
    """

    def _fallback_pack(self) -> dict:
        return {
            "id": router.FALLBACK_PACK_ID,
            "files": ["*.rs"],
            "commands": [
                "cargo test --workspace --lib --profile agent --locked",
                "cargo check --workspace --all-targets --profile agent --locked",
            ],
            "coverage_filters": ["workspace-lib"],
        }

    def test_dap_src_change_injects_per_crate_tests_command(self) -> None:
        """Regression: PR #1212 — frames.rs change → 0 % because only --lib ran."""
        packs = [self._fallback_pack()]
        paths = [
            "crates/perl-dap/src/debug_adapter/frames.rs",
            "crates/perl-dap/tests/dap_adapter_tests.rs",
        ]

        selected = router.selected_packs(packs, paths)
        self.assertEqual([router.FALLBACK_PACK_ID], [p["id"] for p in selected])

        normalized = [router.normalize_pack(p, paths) for p in selected]
        rust_pack = normalized[0]

        # Must include per-crate integration-test invocation with single-threaded flag.
        # --test-threads=1 prevents parallel-unsafe tests from flaking in the coverage lane.
        integration_cmds = [
            cmd for cmd in rust_pack["commands"]
            if "cargo test -p perl-dap" in cmd and "--tests" in cmd
        ]
        self.assertTrue(
            integration_cmds,
            f"Expected a 'cargo test -p perl-dap --tests' command; got: {rust_pack['commands']}",
        )
        # Every integration-test command must carry --test-threads=1.
        for cmd in integration_cmds:
            self.assertIn(
                "--test-threads=1",
                cmd,
                f"coverage-lane integration test must be single-threaded; got: {cmd}",
            )

        # Original lib command must not be removed.
        lib_cmds = [cmd for cmd in rust_pack["commands"] if "--lib" in cmd]
        self.assertTrue(
            lib_cmds,
            f"lib command must remain; got: {rust_pack['commands']}",
        )

    def test_multiple_changed_crates_each_get_integration_test_command(self) -> None:
        packs = [self._fallback_pack()]
        paths = [
            "crates/perl-dap/src/debug_adapter/frames.rs",
            "crates/perl-parser/src/lib.rs",
        ]

        selected = router.selected_packs(packs, paths)
        normalized = [router.normalize_pack(p, paths) for p in selected]
        rust_pack = normalized[0]

        dap_cmds = [
            cmd for cmd in rust_pack["commands"]
            if "cargo test -p perl-dap" in cmd and "--tests" in cmd
        ]
        parser_cmds = [
            cmd for cmd in rust_pack["commands"]
            if "cargo test -p perl-parser" in cmd and "--tests" in cmd
        ]
        self.assertTrue(dap_cmds, f"expected perl-dap --tests command; got: {rust_pack['commands']}")
        self.assertTrue(parser_cmds, f"expected perl-parser --tests command; got: {rust_pack['commands']}")
        # All injected integration-test commands must be single-threaded.
        for cmd in dap_cmds + parser_cmds:
            self.assertIn(
                "--test-threads=1",
                cmd,
                f"coverage-lane integration test must be single-threaded; got: {cmd}",
            )

    def test_integration_test_command_not_duplicated(self) -> None:
        packs = [self._fallback_pack()]
        paths = [
            "crates/perl-dap/src/debug_adapter/frames.rs",
            "crates/perl-dap/src/debug_adapter/variables.rs",
        ]

        selected = router.selected_packs(packs, paths)
        normalized = [router.normalize_pack(p, paths) for p in selected]
        rust_pack = normalized[0]

        dap_cmds = [
            cmd for cmd in rust_pack["commands"]
            if "cargo test -p perl-dap" in cmd and "--tests" in cmd
        ]
        # Only one command for perl-dap even if multiple files changed.
        self.assertEqual(1, len(dap_cmds), f"expected exactly one perl-dap --tests command; got: {dap_cmds}")
        # The deduplicated command must be single-threaded.
        self.assertIn(
            "--test-threads=1",
            dap_cmds[0],
            f"coverage-lane integration test must be single-threaded; got: {dap_cmds[0]}",
        )

    def test_no_crate_injection_for_xtask_src_change(self) -> None:
        """xtask/src/ changes have no crates/ crate; no --tests injection."""
        packs = [self._fallback_pack()]
        paths = ["xtask/src/tasks/ci_route.rs"]

        selected = router.selected_packs(packs, paths)
        normalized = [router.normalize_pack(p, paths) for p in selected]
        if normalized:
            rust_pack = normalized[0]
            spurious = [
                cmd for cmd in rust_pack["commands"]
                if "-p xtask --tests" in cmd
            ]
            self.assertEqual(
                [], spurious,
                f"xtask is not a crates/ crate; must not inject --tests; got: {spurious}"
            )

    def test_crate_name_from_source_path(self) -> None:
        self.assertEqual("perl-dap", router.crate_name_from_source_path(
            "crates/perl-dap/src/debug_adapter/frames.rs"
        ))
        self.assertEqual("perl-parser", router.crate_name_from_source_path(
            "crates/perl-parser/src/lib.rs"
        ))
        self.assertIsNone(router.crate_name_from_source_path("xtask/src/tasks/ci_route.rs"))
        self.assertIsNone(router.crate_name_from_source_path("docs/reference/API.md"))


if __name__ == "__main__":
    unittest.main()
