# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Swarm v2 — SubagentStart hook**: Auto-injects `/coding-standards` into every builder subagent at spawn time so coding rules are enforced without inline repetition.
- **Swarm v2 — Stop hook**: Reminds agents to verify deliverables (branch + PR) before stopping.
- **Swarm v2 — PreToolUse hook**: Blocks destructive commands (`git push --force`, `rm -rf` of tracked directories, unauthorized `cargo publish`) at the platform level.
- **Swarm v2 — SessionStart/compact hook**: Re-injects critical context after context compaction to prevent agent drift.
- **Swarm v2 — `/plan-fix` skill**: Scouts write detailed implementation plans before handing off to builders.
- **Swarm v2 — `/verify-build` skill**: Builders verify branch, tests, and PR exist before marking tasks complete.
- **Swarm v2 — `/how-to-use-skills` skill**: Meta-documentation explaining skill invocation patterns to new agents.
- **Swarm v2 — Skill templates**: `test-template.rs`, `handoff-template.md`, `slice-template.md`, `issue-template.md` for consistent artifact creation.
- **Swarm v2 — Skill examples**: `undef-fix.md`, `parser-scout-output.md` illustrate expected output formats.
- **Swarm v2 — Dynamic context injection**: Status skills use `` !`command` `` syntax to embed live data (git log, cargo test output) inline.
- **Swarm v2 — agents7 generation**: Full coordinator team definitions for 5-coordinator architecture (scout, builder, reviewer, ops, improver).
- **Async Runtime with Concurrent Dispatch**: LSP server migrated to a two-lane scheduler — exclusive worker for mutations + 4-worker read pool for concurrent hover/goto-def/completion requests. `$/cancelRequest` is now processed inline before enqueuing (#1555).
- **Continuous Swarm Infrastructure**: 53 agents, 22 skills, 12 named teammates, GitHub-native tracking, handoff protocol, and portable agent pack for other repos (#1553).
- **Goto AST Node**: Dedicated `Goto` node with full `TokenKind::display_name` support (#1521).
- **Smarter Selection Range**: Expand/shrink selection chains with semantic awareness (#1545).
- **Cross-file Go-to-Definition for Methods and Modules**: Improved navigation for method calls and `use parent`/`base` statements (#1542, #1544).
- **Enhanced Diagnostics**: Added `suggestion` field to diagnostic messages (#1543).
- **Inlay Hints for Builtins**: Derive parameter names from builtin function signatures (#1541).
- **Semantic Tokens**: Comprehensive AST walker with new token types for broader coverage (#1540).
- **DAP Improvements**: POD detection, conditional expression validation in breakpoints (#1536), and improved variable inspection rendering (#1535).
- **Hover Enhancements**: Improved documentation quality in hover responses (#1537).
- **Cross-sigil Variable Highlighting**: Highlight `@foo` and `%foo` references when cursor is on `$foo` (#1538).
- **Extract Variable for Methods**: Code actions handle method calls and hash/array access (#1534).
- **Workspace Symbols Ranking**: Improved ranking with comprehensive tests (#1529).
- **Completion for Moo/Moose Accessors**: Show `isa` type in completion for accessor methods (#1525).
- **Signature Help Builtin Coverage**: Expanded coverage for common Perl builtins (#1532).

### Changed
- **Swarm v2 — Hooks read stdin JSON**: Hooks now parse the official Claude Code JSON payload via `jq` instead of reading empty environment variables (`$TASK_SUBJECT`, `$TEAMMATE_ID` were never populated).
- **Swarm v2 — Skills use `context: fork`**: Skills run in isolated subagents with `allowed-tools` enforcement; plain markdown files without frontmatter are no longer sufficient.
- **Swarm v2 — 5-coordinator team**: Team restructured from 12 flat teammates to 5 coordinators with subagent fanout, reducing idle token overhead by ~60%.
- **Swarm v2 — Thin agent definitions**: Agent definitions are ~50-line orchestration loops that invoke skills; inline workflow instructions moved to skills.
- **Swarm v2 — Skills directory**: Skills migrated to `.claude/skills/` with a `SKILL.md` index; `.claude/commands/` retained for backward compatibility.
- **Swarm v2 — Permission rule syntax**: Permission rules corrected from `Bash(gh:*)` (colon) to `Bash(gh *)` (space) throughout `settings.json`.
- **bypassPermissions**: Set as default permission mode (#1554).
- **Native Debt Report**: `xtask` now has a native `debt-report` subcommand (#1528).
- **Devex Targeted Checks**: Converted from shell script to native Rust xtask subcommand (#1527).

### Fixed
- **Swarm v2 — TaskCompleted hook**: Now verifies a non-master branch and an open PR exist before allowing task completion; previously read an empty env var and was a no-op.
- **Swarm v2 — TeammateIdle hook**: Now reads teammate ID from stdin JSON; previously used `$TEAMMATE_ID` which was never set.
- **Parser**: Statement modifiers after complex expressions (#1550), package-qualified variable subscripts (#1548), fat arrow as list separator in all builtin argument paths (#1549), split regex slash disambiguation in expression contexts (#1547).
- **Incremental Parsing**: Improved efficiency and fixed position underflow (#1539).
- **On-Type Formatting**: Heredoc suppression, string/comment-aware brace matching, correct trigger semantics (#1530).
- **Test Count Sync**: Updated test count to 1953, removed PerlIO from corpus manifest (#1552).

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

A major release campaign spanning 60+ PRs (#845–#911) focused on build reliability,
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
- **Release Tool Installs**: Deterministic git-cliff and cargo-release installation (#873–#877).
- **crates.io Dry-Run**: Unblocked dry-run packaging for all workspace crates (#865).
- **Lockfile Maintenance**: Refreshed lockfile for CI deny checks, fuzz lockfile exclusion (#885).

### Dependencies
- `rand` 0.9.2 → 0.10.0 (#855).
- `serial_test` 3.3.1 → 3.4.0 (#854).
- `uuid` 1.20.0 → 1.21.0 (#856).
- `toml` 0.9.12 → 1.0.3 (#853).
- `aquasecurity/trivy-action` 0.34.0 → 0.34.1 (#851).
- `@types/node` 25.1.0 → 25.3.0 (#849).
- `@types/tar` 6.1.13 → 7.0.87 (#850).
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
