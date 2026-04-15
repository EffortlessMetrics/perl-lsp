# Architecture Decision Records (ADRs)

This directory contains Architecture Decision Records (ADRs) for significant design decisions in the Perl LSP project.

## ADR Index

### Legacy Series (0001–0002)

| ADR | Status | Date | Title | Description |
|-----|--------|------|-------|-------------|
| [ADR-0001](0001-substitution-operator-parsing-architecture.md) | Accepted | 2025-01-20 | Substitution Operator Parsing | Comprehensive s/// parsing with all modifiers and delimiter styles |
| [ADR-0002](0002-api-documentation-infrastructure.md) | Accepted | 2025-09-20 | API Documentation Strategy | Enterprise-grade documentation with `#![warn(missing_docs)]` enforcement |

### Current Series (ADR_001+)

| ADR | Status | Ref/Date | Title | Description |
|-----|--------|----------|-------|-------------|
| [ADR-001](ADR_001_AGENT_ARCHITECTURE.md) | Superseded | PR #153 | Agent Architecture | Historical `.claude/agents2/` specialization phase; see ADR-0032/0033 for the current swarm model |
| [ADR-002](ADR_002_API_DOCUMENTATION_INFRASTRUCTURE.md) | Accepted | PR #160 | API Documentation (SPEC-149) | Documentation enforcement with acceptance criteria and quality gates |
| [ADR-003](ADR_003_MISSING_DOCUMENTATION_INFRASTRUCTURE.md) | Accepted | PR #159 | Missing Docs Infrastructure | Documentation enforcement validation framework |
| [ADR-004](ADR_004_EXECUTE_COMMAND_CODE_ACTIONS.md) | Draft | 2025-01-15 | Execute Command & Code Actions | LSP executeCommand integration with perlcritic |
| [ADR-005](ADR_005_HEREDOC_MANUAL_PARSING.md) | Proposed | 2025-11-05 | Manual Heredoc Parsing | Character-by-character state machine parser |
| [ADR-006](ADR_006_LSP_CANCELLATION_INFRASTRUCTURE.md) | Draft | 2026-01-28 | LSP Cancellation | Cancellation infrastructure for responsive editor interactions |
| [ADR-007](ADR_007_SUBSTITUTION_OPERATOR_PARSING.md) | Accepted | 2025-01-20 | Substitution Parsing | Comprehensive s/// parsing with all modifiers |

### Architecture Series (0008–0040)

| ADR | Status | Date | Title | Description |
|-----|--------|------|-------|-------------|
| [ADR-0008](0008-microcrate-architecture.md) | Accepted | 2025-01-15 | Microcrate Architecture (SRP) | 80+ small crates following Single Responsibility Principle for parallel compilation |
| [ADR-0009](0009-dual-indexing-strategy.md) | Accepted | 2025-02-15 | Dual Indexing Strategy | Index functions under both qualified and bare names for 98% reference coverage |
| [ADR-0010](0010-incremental-parsing-architecture.md) | Accepted | 2025-03-01 | Incremental Parsing | Node reuse strategy with less than 1ms update target for responsive LSP |
| [ADR-0011](0011-dap-bridge-mode-architecture.md) | Accepted | 2025-06-15 | DAP Bridge Mode | Debug Adapter Protocol bridge translating DAP to Perl debugger |
| [ADR-0012](0012-error-handling-strategy.md) | Accepted | 2025-01-10 | Error Handling (No Panics) | Ban unwrap/expect/panic in production code for server reliability |
| [ADR-0013](0013-utf16-position-tracking.md) | Accepted | 2025-01-20 | UTF-16 Position Tracking | Symmetric conversion with boundary validation for LSP protocol compliance |
| [ADR-0014](0014-mode-aware-lexer.md) | Accepted | 2025-01-15 | Mode-Aware Lexer | State machine for slash disambiguation enabling pure Rust parsing |
| [ADR-0015](0015-supply-chain-security.md) | Accepted | 2025-02-01 | Supply Chain Security | SBOM generation and SLSA Level 2 provenance for artifact verification |
| [ADR-0016](0016-feature-governance.md) | Accepted | 2025-02-15 | Feature Governance | 8-crate subsystem for enterprise-grade LSP capability management |
| [ADR-0017](0017-workspace-exclusion-strategy.md) | Accepted | 2025-02-15 | Workspace Exclusion Strategy | Exclude crates with C dependencies from main workspace for cross-platform builds |
| [ADR-0018](0018-adaptive-threading-tests.md) | Accepted | 2025-02-15 | Adaptive Threading for Tests | Thread-aware timeout scaling with environment validation for reliable CI |
| [ADR-0019](0019-security-first-dap.md) | Accepted | 2025-06-15 | Security-First DAP | Enterprise-grade security with path traversal prevention and safe evaluation |
| [ADR-0020](0020-rope-document-management.md) | Accepted | 2025-01-20 | Rope Document Management | O(log n) position lookups with ropey::Rope for sub-millisecond conversions |
| [ADR-0021](0021-lsp-capability-contract.md) | Accepted | 2025-02-15 | LSP Capability Contract | Contract-driven capability advertisement with lsp-ga-lock feature option |
| [ADR-0022](0022-scope-analyzer-hash-context.md) | Accepted | 2025-02-15 | Scope Analyzer Hash Context | Pointer-based AST traversal for accurate hash key context detection |
| [ADR-0023](0023-include-macro-architecture.md) | Accepted | 2025-02-20 | include! Macro Architecture | Rust include! macro for parser composition with tight coupling |
| [ADR-0024](0024-fifo-heredoc-queue.md) | Accepted | 2025-02-20 | FIFO Heredoc Queue | VecDeque queue for single-pass heredoc content collection |
| [ADR-0025](0025-dual-document-representation.md) | Accepted | 2025-02-20 | Dual Document Representation | Rope + String for O(log n) edits with O(1) access |
| [ADR-0026](0026-lifecycle-index-routing.md) | Accepted | 2025-02-20 | Lifecycle Index Routing | State machine with Building/Ready/Degraded for graceful degradation |
| [ADR-0027](0027-dap-bridge-native.md) | Accepted | 2025-02-20 | DAP Bridge vs Native | Phased approach with bridge mode for immediate value |
| [ADR-0028](0028-safe-eval-timeout.md) | Accepted | 2025-02-20 | Safe Eval Timeout | 5-second default and 300-second max for DoS prevention |
| [ADR-0029](0029-mutation-sentinel-values.md) | Accepted | 2025-02-20 | Mutation Sentinel Values | Sentinel values like xyzzy for mutation testing detection |
| [ADR-0030](0030-receipt-gate-system.md) | Accepted | 2025-02-20 | Receipt Gate System | Machine-readable receipts for CI gate verification |
| [ADR-0031](0031-async-runtime-concurrent-dispatch.md) | Accepted | 2026-03-16 | Async Runtime with Concurrent Dispatch | Two-lane scheduler (exclusive + 4-worker read pool) for concurrent LSP request handling |
| [ADR-0032](0032-skill-scoping-and-hook-enforcement.md) | Accepted | 2026-03-16 | Skill Scoping and Hook Enforcement | Frontmatter-based skill access control plus hook-enforced swarm coordination |
| [ADR-0033](0033-worktree-first-disposable-workers.md) | Accepted | 2026-03-16 | Worktree-First Disposable Worker Execution | Small persistent coordinators, fresh workers per context shift, and worktree isolation for PR-shaped changes |
| [ADR-0034](0034-custom-lsp-runtime.md) | Accepted | 2026-03-18 | Custom LSP Runtime over Framework Adoption | Bespoke protocol/transport/runtime stack kept to support governance, transport reuse, and explicit dispatch control |
| [ADR-0035](0035-deterministic-module-resolution.md) | Accepted | 2026-03-18 | Deterministic Module Resolution | Canonicalized names, explicit precedence, workspace-safe paths, and lib/ fallback for module lookup |
| [ADR-0036](0036-marker-framed-debugger-queries.md) | Accepted | 2026-03-18 | Marker-Framed Debugger Queries | Unique debugger output markers plus poison-safe shared state for resilient native DAP queries |
| [ADR-0038](0038-session-economics.md) | Accepted | 2026-03-19 | Session Economics | Agent lifecycle cost model and swarm wind-down policy to preserve context budget |
| [ADR-0039](0039-raw-pointer-parent-map.md) | Accepted | 2026-03-18 | Raw-Pointer Parent Map | Sidecar parent cache using raw pointers for efficient upward AST traversal without tree-sitter API changes |
| [ADR-0040](0040-generated-feature-catalog-contracts.md) | Accepted | 2026-03-18 | Generated Feature Catalog Contracts | Build-time compilation of `features.toml` into generated Rust contracts via perl-feature-catalog |
| [ADR-0041](0041-microcrate-collapse.md) | Accepted | 2026-04-14 | Microcrate Collapse | Collapse from 132 published crates to ~30; ~100 internal microcrates become subfolder modules |

## About ADRs

Architecture Decision Records (ADRs) capture important architectural decisions along with their context and consequences. Each ADR includes:

- **Context**: The situation that led to the decision
- **Decision**: The architectural choice made
- **Consequences**: The results of the decision, both positive and negative

## ADR Process

1. **Identify Decision**: When facing a significant architectural choice
2. **Document Options**: Record all considered alternatives with pros/cons
3. **Make Decision**: Choose the best option based on decision drivers
4. **Record ADR**: Document the decision with full context
5. **Update Index**: Add the new ADR to this index
6. **Link Documentation**: Cross-reference with relevant implementation docs

## Status Definitions

- **Proposed**: Under consideration
- **Accepted**: Decision made and implemented
- **Deprecated**: No longer current but kept for historical context
- **Superseded**: Replaced by a newer decision

## Cross-References

- [CLAUDE.md](../CLAUDE.md) - Project overview and capabilities
- [CRATE_ARCHITECTURE_GUIDE.md](../reference/CRATE_ARCHITECTURE_GUIDE.md) - System architecture
- [PARSER_COMPARISON.md](../reference/PARSER_COMPARISON.md) - Parser implementation details
