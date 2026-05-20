# Scripts Directory

Helper scripts for development, CI, release, and workspace management. Scripts here are invoked by `just` recipes, CI workflows, or directly.

## Development Helpers

Run these locally during normal contribution work.

| Script | Purpose |
|--------|---------|
| `install-githooks.sh` | Install the pre-push hook (run once after cloning) |
| `check-rust-toolchain.sh` | Legacy wrapper to `cargo xtask check-toolchain` (MSRV compatibility check) |
| `devex-doctor.sh` | Environment diagnostics (tool availability, paths) |
| `devex-targeted-checks.sh` | Targeted subset of diagnostics for a specific area |
| `preflight.sh` | Quick pre-flight checks before a PR |
| `pre-merge-check.sh` | Verify the working tree is ready to merge |
| `forbid-fatal-constructs.sh` | Grep for banned patterns (`unwrap`, `panic!`, etc.) |
| `dead-code-check.sh` | Report dead code across the workspace |

## Worktree & Workspace Management

| Script | Purpose |
|--------|---------|
| `cleanup-worktrees.sh` | Remove stale git worktrees |
| `cleanup-completed-worktrees.sh` | Remove worktrees whose branches are merged |
| `worktree-manager.py` | Python interface to create and track named worktrees |
| `validate-workspace-exclusions.sh` | Ensure excluded paths aren't accidentally included |
| `gen-xlarge-workspace.sh` | Generate a large synthetic workspace for scale testing |

## CI Gates

Scripts invoked by `just` CI tiers or GitHub Actions workflows.

| Script | Purpose |
|--------|---------|
| `gate-local.sh` | Run the local merge gate (same as `just ci-gate`) |
| `execute-gate.sh` | Compatibility shim for `cargo xtask gates --gate <name>` |
| `run-gates.sh` | Compatibility shim for `cargo xtask gates --tier merge-gate --receipt` |
| `e2e-gate.sh` | End-to-end gate (full pipeline validation) |
| `e2e-validation.sh` | Extended E2E validation with real Perl files |
| `production-gates-validation.sh` | Validate all production gates pass |
| `test-capped.sh` | Run tests with parallelism cap (low-memory environments) |
| `test-e2e-capped.sh` | E2E tests with parallelism cap |
| `performance-hardening.sh` | Performance regression checks |
| `security-hardening.sh` | Security-related checks |
| `lsp-smoke.sh` | Smoke test the LSP server binary |
| `smoke-test.sh` | General smoke tests |
| `smoke-test-release.sh` | Smoke test a release binary |
| `check-coverage-baseline.sh` | Compare coverage against baseline |
| `update-coverage-baseline.sh` | Ratchet the coverage baseline up |
| `ignored-test-count.sh` | Report number of `#[ignore]`-tagged tests |

## Release Workflow

Run in order for a release. See [CONTRIBUTING.md](../CONTRIBUTING.md#release-workflow) for the full sequence.

| Script | Order | Purpose |
|--------|-------|---------|
| `prepare-release.sh` | 1 | Bump versions, generate changelog entry |
| `release-turnkey-pr.sh` | 2 | Open release PR with all artifacts |
| `publish-topo.py` | 3 | Publish crates in topological order |
| `publish-release.sh` | — | Full publish pipeline (wraps publish-topo.py) |
| `publish-new-crates-manually.sh` | — | Publish a single new crate by name |
| `publish-receipts.sh` | — | Generate publish receipts for audit trail |
| `check-version-sync.sh` | — | Verify all version sites agree |
| `prep-crates-io-launch.sh` | — | Pre-launch checklist for crates.io |
| `post-publish-smoke.sh` | — | Verify published crates install correctly |
| `update-homebrew.sh` | — | Update the Homebrew formula after release |
| `verify-publication-facts.sh` | — | Cross-check publication receipts |

## Corpus & Parser Testing

| Script | Purpose |
|--------|---------|
| `run_parser_comparison.sh` | Compare v2 vs v3 parser output on test corpus |
| `test_edge_cases.sh` | Run edge case tests for the parser |
| `test_iterative_parser.sh` | Test the iterative parser path |
| `update-parser-matrix.py` | Regenerate the parser feature matrix |

## Documentation & Metrics

| Script | Purpose |
|--------|---------|
| `update-current-status.py` | Regenerate docs/project/CURRENT_STATUS.md metrics |
| `check-doc-claims.py` | Verify claims in docs against actual behavior |
| `check_features_invariants.py` | Validate features.toml is internally consistent |
| `check_release_history.sh` | Check release history for consistency |
| `generate-badges.sh` | Regenerate README status badges |
| `render-docs.sh` | Build the documentation site |
| `verify-docs-rs.sh` | Verify docs.rs links are valid |
| `populate-book.sh` | Populate the mdBook documentation |

## Diagnostics & Forensics

| Script | Purpose |
|--------|---------|
| `build-timing-receipt.sh` | Record build timing for comparison |
| `compare-build-timing.sh` | Compare two timing receipts |
| `ci-audit-workflows.py` | Audit CI workflow definitions |
| `ci-cost-monitor.sh` | Report CI resource usage |
| `debt-report.py` | Technical debt report |
| `debt-pr-summary.py` | Summarize debt across open PRs |
| `check-v2-bundle-sync.sh` | Check tree-sitter bundle is in sync |
| `check-windows-distribution.ps1` | Verify Windows distribution artifacts |
| `forensics/` | One-off forensic scripts (not for regular use) |

## Swarm & Agent Operations

| Script | Purpose |
|--------|---------|
| `agents/` | Agent definition fragments used by the swarm |
| `agent-preflight.sh` | Agent safety preflight checks |
| `test-agent-preflight.sh` | Test the preflight script itself |
| `control-plane-lock.sh` | Advisory single-writer lock for swarm operations |
| `test-control-plane-lock.sh` | Test the lock implementation |
| `swarm-summary.sh` | Summarize current swarm state |
| `cargo xtask validate-swarm-agent-roster` | Validate agent roster completeness |
| `validate_swarm_findings.py` | Validate swarm-discovered findings |
| `bulk-label-issues.sh` | Bulk-label GitHub issues by query |
| `close-duplicate-prs.sh` | Close duplicate PRs by title |

## Security & Miscellaneous

| Script | Purpose |
|--------|---------|
| `gh/` | GitHub CLI helper wrappers |
| `marketing/` | GIF rendering and demo scripts (see [docs/assets/gifs/README.md](../docs/assets/gifs/README.md)) |
| `tests/` | Shell-level integration tests for scripts |
| `requirements.txt` | Python dependencies for scripts that need them |
| `safe-pull.sh` | Pull with automatic conflict detection |
| `inject-sha-assets.sh` | Inject SHA checksums into release assets |
| `generate-receipt.sh` | Generate a single artifact receipt |
| `generate-receipts.sh` | Generate all artifact receipts |
| `quick-receipts.sh` | Fast receipt generation for CI |
| `llvm.sh` | LLVM toolchain helpers (coverage, profiling) |
| `install.sh` | Install `perllsp` from a local build |
| `list-gates.py` | Compatibility shim for `cargo xtask gates --list` |
| `verify_stacker.sh` | Verify stacker (stack-size extension) is working |
| `test-lsp-cancellation.sh` | Test LSP request cancellation behavior |
| `render-linux-packages.py` | Build Linux distribution packages |
| `DEPRECATED_RELEASE_SCRIPTS.md` | Index of scripts removed from the release workflow |
