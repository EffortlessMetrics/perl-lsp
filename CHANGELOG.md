# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- No unreleased changes have been recorded yet.

## [0.12.1] - 2026-03-30

`v0.12.1` is the fix-forward cut after the initial public alpha release. It does
not reopen the wider alpha scope; it closes the release-surface regressions that
slipped into the first `v0.12.0` tag and keeps the install and publish story
aligned.

### Fixed

- restored the top-level README and release-facing docs so the source snapshot
  no longer presents hook-test fixture content as the project front page
- hardened hook-test fixture setup so temporary repos must live outside the real
  checkout and seed commits no longer write placeholder git identities into repo
  config
- fixed local git-hook installation for worktrees and added pre-commit blocking
  for the known placeholder identities used by release and hook tests

### Changed

- workspace, feature-catalog, VS Code extension, and operator release surfaces
  now target `0.12.1`
- status and roadmap docs now treat `v0.12.0` as the latest published GitHub
  release and `v0.12.1` as the active fix-forward cut

## [0.12.0] - 2026-03-24

`v0.12.0` is the initial public alpha for the native Rust Perl 5 toolchain. The
headline change is not one feature in isolation; it is that the parser, language
server, debugger, install surface, and release process now line up well enough
for normal editor use.

### Highlights

#### Native editor path

- `perllsp` and `perl-dap` are now treated as first-class native binaries for editor integration and debugging.
- VS Code, manual binary install, and release surfaces were tightened for first-run setup, health checks, and issue reporting.
- `.perl-lsp.toml` gives teams a shared, editor-agnostic project configuration layer.

#### Better day-to-day language tooling

- Completion, hover, diagnostics, formatting, semantic tokens, workspace symbols, code lens, and code actions all received broad hardening.
- Hover and completion coverage expanded for Perl built-ins, special variables, module flows, and workspace-aware suggestions.
- Diagnostic wiring now consistently surfaces parser, project, and optional Perl::Critic signals through the LSP pipeline.

#### Better real-world Perl coverage

- The native recursive-descent parser was hardened against curated common-corpus and CPAN-facing receipts instead of toy examples alone.
- Semantic and workspace layers improved cross-file navigation, rename, inheritance-aware lookups, and framework-aware behavior for Moo and Moose patterns.
- Workspace indexing, cancellation, timeouts, and runtime concurrency all received reliability work aimed at larger real projects.

#### Release and contributor surface

- Release prep, package-manager manifests, docs, validation receipts, and status pages were aligned for the public-alpha launch.
- The workspace continued its crate-boundary cleanup so parser, runtime, LSP, DAP, and release tooling are easier to reason about independently.

### Notable user-facing additions

- project config via `.perl-lsp.toml`
- richer hover coverage for special variables, built-ins, and framework-aware symbols
- broader completion coverage and improved ranking
- native DAP improvements for stepping, variables, and editor integration
- stronger workspace symbol, formatting, code action, and code lens support

### Notable fixes

- parser recovery and disambiguation across real Perl edge cases such as quote operators, slash parsing, prototypes, and framework-heavy code
- deadlock, contention, and stale-state fixes in the LSP runtime and workspace index
- safer handling for empty files, binary files, Windows and macOS path quirks, and shell-launch edge cases
- stale capability drift, unwired command paths, and release-surface documentation mismatches

For the detailed receipts behind this release, see [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) and [docs/project/status/index.md](docs/project/status/index.md).

## [0.11.0] - 2026-03-12

This release finalizes the 0.11.0 distribution pipeline across GitHub releases,
crates.io, and the VS Code extension so the workspace can ship from a single,
repeatable release flow.

### Added
- **Turnkey Release Orchestration**: A PR-driven release path now covers version
  bumping, changelog generation, tagging, GitHub release creation, crates.io
  publishing, extension publishing, and downstream package manager automation.
- **Topological crates.io Publishing**: Workspace publish automation computes
  dependency order from `cargo metadata` and publishes only the crates in the
  workspace allowlist.
- **Release Guardrails**: Release helper scripts now validate semver inputs and
  align manual operator flows with the automated `0.11.0` release path.

### Changed
- **Workspace Release Alignment**: Workspace packages, extension metadata, and
  release workflows now target `0.11.0`.
- **Release Tooling**: Legacy release helper scripts now delegate to the current
  GitHub workflow-based release flow instead of relying on stale one-off cargo
  publish steps and outdated examples.
- **Operator Documentation in Scripts**: Manual publish and smoke-test helpers
  now accept an explicit version argument and default to the matching `vX.Y.Z`
  release ref when dispatching workflows or validating published artifacts.

### Fixed
- **Stale Release Examples**: Removed hardcoded `0.8.3` release references from
  publish and smoke-test scripts that could misdirect manual release operations.
- **Publish Version Safety**: crates.io publishing now fails early when the
  workflow target version does not match the versions resolved for workspace
  crates scheduled for publication.

## [0.10.0] - 2026-02-28

A major release campaign spanning 60+ PRs (#845-#911) focused on build reliability,
security hardening, crates.io publishing readiness, documentation, and code quality.

### Added
- **Document Highlight for Modern Perl**: try/catch parameters, method/sub signatures, and string interpolation (#882, #896).
- **Feature Governance Microcrates**: Extracted feature governance into 9 dedicated crates for modularity (#848).
- **Module Infrastructure Crates**: Content-Length framing and LSP transport hardening (#857).
- **Context-Aware Status Menu**: Perl LSP status menu with workspace-aware states (#646).
- **InlineValues Lifecycle Coverage**: Test coverage for inlineValues support (#729).
- **Tie-Interface Corpus Tests**: New corpus test fixtures for Perl tie interface syntax (#900).
- **Public API Documentation**: Comprehensive rustdoc for `perl-parser` (#904) and leaf crates (#903).
- **Copilot Instructions**: `.github/copilot-instructions.md` for AI-assisted development (#886).
- **Merge-Gate Commit Status**: CI now publishes merge-gate status checks (#880).
- **Benchmark Test Enablement**: Previously-ignored workspace benchmark test enabled with real assertions (#908).

### Changed
- **Version Bump to 0.10.0**: All 80+ workspace crates, documentation, VS Code extension, and feature catalogs updated (77+ files) (#879, #884).
- **crates.io Publishing Readiness**: All crate metadata verified, publish-ignore lists normalized, crate badges added, publish allowlist expanded (#865, #867, #871, #897).
- **VS Code Extension Polish**: Marketplace readiness with packaging fixes, runtime deps, npm lockfile (#863, #866, #869, #906).
- **Documentation Overhaul**: CONTRIBUTING.md polished for public release (#909), README.md and ROADMAP.md updated (#888), FrameworkKind/FrameworkFlags docs (#887), cargo doc warnings resolved (#894).
- **features.toml**: Version bumped to 0.10.0 with 100% LSP coverage maintained (53/53 user-visible, 97/97 protocol).
- **LSP Harness**: Replaced sleep-poll with condvar+drain-bytes pattern for deterministic testing (#846).
- **xtask Gates**: Fail closed for required timeout/error statuses (#868).
- **Unused Dependencies Removed**: cargo-machete sweep across workspace (#895).
- **Debt Ledger Updated**: Refreshed after cleanup campaign (#898).
- **Stale Files Cleaned**: Removed stale tracked files, hardened .gitignore (#889).
- **Semver-Aware Benchmark Sorting**: Correct version comparison for baseline selection (#885).

### Fixed
- **Build**: Resolved 4 compilation errors in the release candidate build (#881).
- **Clippy**: Resolved warnings across all targets (#901).
- **Document Highlight Regressions**: Fixed test regressions from modern syntax support (#896).
- **LSP Error Logging**: Improved error logging in LSP providers (#905).
- **Unresolved Review Comments**: Addressed outstanding comments from PRs #881 and #882 (#892).
- **Version Drift**: Fixed remaining v0.9.x references in satellite files (#884).
- **Checksum Verification**: Hardened verification and stabilized incremental parsing CI (#858).
- **Installer Scripts**: Hardened for security and reliability (#910).
- **Refactoring Test Isolation**: Isolated `cleanup_no_backups` backup root (#864).
- **CI Receipt Parsing**: Aligned receipt parsing and serialized BDD tests (#845).
- **CI BDD Gate**: Added `--locked` flag and timing receipts (#847).
- **CI Docs Deploy**: Skip when GitHub Pages is disabled (#859).
- **Release Workflow**: Asset naming alignment across chain (#890, #902), concurrency groups (#890).
- **Release Tooling**: git-cliff installation fixes (#873, #874, #875), cargo-release installs (#876, #877), PR-driven 0.x.y flow (#872).
- **Publish Workflow**: Dry-run quoting fix (#870), `--no-verify` for dev-dep cycles (#867).

### Security
- **[HIGH] Path Traversal in DAP Launch**: Fixed path traversal vulnerability in debug adapter (#640).
- **[HIGH] Argument Injection in TestRunner**: Fixed argument injection vulnerability (#633).
- **[MEDIUM] Safe Evaluation Bypass**: Fixed bypass for iterator/IO operations (#647).
- **GitHub Actions Hardening**: SHA-pinned all workflow action references (#911).
- **Installer Hardening**: Hardened install scripts for security and reliability (#910).
- **VS Code Extension**: Pinned minimatch to 10.2.3 to remediate CVEs (#861).

### Performance
- **Symbol Extraction**: Optimized regex compilation for faster workspace indexing (#645).
- **Semantic Analyzer**: Eliminated deep cloning of AST nodes in subroutine analysis (#632).
- **Scope Analyzer**: Optimized unused parameter detection, fixed double reporting (#638).

### Infrastructure
- **Nightly CI Stabilization**: Fuzz harness panic hardening, coverage test resilience, clippy cleanup (#860).
- **Release Orchestration**: Turnkey PR-driven 0.x.y release workflow (#872).
- **Release Tool Installs**: Deterministic git-cliff and cargo-release installation (#873-#877).
- **crates.io Dry-Run**: Unblocked dry-run packaging for all workspace crates (#865).
- **Lockfile Maintenance**: Refreshed lockfile for CI deny checks, fuzz lockfile exclusion (#885).

### Dependencies
- `rand` 0.9.2 -> 0.10.0 (#855).
- `serial_test` 3.3.1 -> 3.4.0 (#854).
- `uuid` 1.20.0 -> 1.21.0 (#856).
- `toml` 0.9.12 -> 1.0.3 (#853).
- `aquasecurity/trivy-action` 0.34.0 -> 0.34.1 (#851).
- `@types/node` 25.1.0 -> 25.3.0 (#849).
- `@types/tar` 6.1.13 -> 7.0.87 (#850).
- Additional dependency group updates (#852).

## [0.9.1] - 2026-02-20

### Added
- **Initial Public Alpha Release**: Substantially complete feature set for early testing.
- **Enhanced LSP Features**: 99% coverage of LSP 3.18 methods (alpha-validated).
- **Complete Semantic Analyzer**: All NodeKind handlers implemented (Phases 1, 2, 3) with 100% AST node coverage.
- **Debug Adapter Protocol (DAP) Support**: Phase 1 bridge to Perl::LanguageServer.
- **Enhanced LSP Cancellation System**: Thread-safe infrastructure for minimal latency.
- **Advanced Code Actions**: AST-aware refactoring including extraction and import optimization.
- **Security Hardening**: UTF-16 boundary fixes and path traversal prevention.
- **Comprehensive API Documentation**: Infrastructure for documentation enforcement.
- **Optimized Test Suite**: 0.31s full test suite execution via adaptive threading.

### Changed
- **Project Origins Documented**: Origins in Q2 2025, forked July 15, 2025 from `tree-sitter-perl-better`.
- **Stability Roadmap Refined**: Formal Stability Contract (contract-locked APIs) pushed to v0.15.0.
- **MSRV Updated**: Minimum Supported Rust Version bumped to 1.92 (Rust 2024 edition).
- **Parser Architecture**: Native recursive descent parser as the primary implementation.

### Fixed
- **v0.9.1 close-out receipts captured**: Workspace index state-machine transitions and early-exit behavior verified.
- **Security boundary fixes**: Resolved multi-root workspace path traversal issues.

## [0.9.0] - 2026-01-18

### Added
- **Semantic Analyzer Phase 1**: 12/12 critical node handlers implemented.
- **LSP textDocument/definition Integration**: Semantic-aware definition resolution.
- **Enhanced Cross-File Navigation**: Dual indexing strategy for improved reference coverage.

### Changed
- **LSP Coverage**: Increased to 82% of trackable features.

## [0.8.8] - 2025-12-01

### Added
- **Initial Workspace Configuration Support**.
- **Enhanced Formatting Fallback**: Always-available capabilities with perltidy integration.

---

## Future Milestones

### Next Release
- Enhanced DAP native implementation (Phase 2).
- Semantic depth improvements for Moo/Moose.

### v0.15.0 (Stability Contract Milestone)
- **Formal Stability Contract**: Contract-locked APIs and wire protocol invariants.
- Full protocol compliance audit.
- Multi-release deprecation cycles.

---

## Version Support Policy (Alpha Phase)

During the alpha phase (pre-v0.15.0):
- **Current Alpha (0.x.y)**: Active development and bug fixes.
- **Breaking Changes**: Allowed in minor (0.x) releases.
- **Security**: Critical patches prioritized for the latest alpha version.
