# Crate Dependency Tiers

The perl-lsp workspace contains 116 workspace members (plus `xtask` and 4
path-dependency crates resolved via `[workspace.dependencies]`), organized into
a strict tiered dependency structure. Each tier builds only on crates from lower
tiers, which guarantees acyclic dependencies, maximizes build parallelism, and
isolates API surface changes.

This document is the canonical reference for tier assignments. The root
`Cargo.toml` encodes the same ordering in its `[workspace.dependencies]` tier
comments and `[workspace.metadata.publish]` topological allowlist.

## Why Tiers Matter

**Build parallelism.** Cargo compiles independent crates in parallel. All Tier 1
crates compile simultaneously at the start of a build, forming a wide wavefront
that saturates available CPU cores. Each subsequent tier must wait only for its
direct dependencies, not the entire previous tier.

**API stability.** Leaf crates (Tier 1) have no internal dependencies, so their
APIs change infrequently. Higher-tier crates absorb more churn. Downstream
consumers can pin to a leaf crate without transitively pulling the whole
workspace.

**Testing isolation.** A crate at Tier N can be tested in isolation by building
only Tiers 1 through N. Failures are confined to a narrow dependency cone, which
simplifies bisection and reduces CI resource usage.

**Dependency hygiene.** The tier rule (a crate may depend only on strictly lower
tiers) makes circular dependencies impossible by construction. The
`[workspace.metadata.publish]` section in `Cargo.toml` enforces topological
order for crates.io publishing.

## Tier Diagram

```
Tier 7  Legacy / Testing / Tree-sitter  perl-parser-pest, perl-corpus,
  ^                                     tree-sitter-perl-rs, perl-ts-*
  |
Tier 6  Application / Executables       perl-parser, perl-lsp, perl-dap
  ^
  |
Tier 5  Task Runner                     xtask
  ^
  |
Tier 4  Analysis & Provider             perl-semantic-analyzer,
  ^     Aggregation                     perl-lsp-providers
  |
Tier 3  Indexing & Multi-dep            perl-workspace-index, perl-refactoring,
  ^     Coordination                    perl-incremental-parsing
  |
Tier 2  Single-level Dependencies       perl-parser-core, perl-tdd-support,
  ^                                     perl-lsp-transport, perl-lsp-formatting, ...
  |
Tier 1  Leaf Crates (no workspace deps) perl-token, perl-ast, perl-keywords,
                                        perl-lsp-feature-ids, perl-module-name, ...
```

## Tier 1 -- Leaf Crates (no workspace dependencies)

Crates with **no workspace production dependencies** (external-only or zero
deps). These compile first and in parallel, forming the foundation for
everything above. Note: some of these have workspace dev-dependencies (typically
`perl-tdd-support`) which do not affect production build order.

| Crate | Description |
|-------|-------------|
| `perl-token` | Token definitions for Perl parser |
| `perl-quote` | Perl quote-like operator parsing helpers |
| `perl-ast` | AST node definitions for Perl parsing (deps: `perl-token`, `perl-position-tracking`) |
| `perl-pragma` | Perl pragma extraction and analysis primitives (deps: `perl-ast`) |
| `perl-edit` | Text edit representation for incremental parsing (deps: `perl-position-tracking`) |
| `perl-builtins` | Builtin symbol metadata for parser and LSP tooling (deps: `perl-builtins-phf`) |
| `perl-builtins-phf` | PHF-backed Perl builtin signature tables for O(1) lookup |
| `perl-regex` | Regex parsing and validation helpers for Perl syntax |
| `perl-heredoc` | Heredoc collector and processor for Perl (deps: `perl-position-tracking`) |
| `perl-error` | Error types, classification, and recovery strategies (deps: `perl-ast`, `perl-regex`, `perl-lexer`) |
| `perl-tokenizer` | Token stream and utilities for Perl parser (deps: `perl-lexer`, `perl-token`, `perl-error`, `perl-ast`) |
| `perl-lexer` | High-performance context-aware Perl tokenizer (deps: `perl-position-tracking`, `perl-keywords`) |
| `perl-keywords` | Canonical Perl keyword inventories and classification helpers |
| `perl-position-tracking` | UTF-8/UTF-16 position tracking and conversion for LSP |
| `perl-symbol-types` | Unified Perl symbol taxonomy for LSP tooling |
| `perl-symbol-cursor` | Cursor-based Perl symbol extraction helpers |
| `perl-symbol-index` | Trie + inverted-index symbol search for Perl tooling |
| `perl-module-boundary` | Module-token boundary matching for single-line scanners (deps: `perl-module-token-core`) |
| `perl-module-token-core` | Shared parser and boundary primitives for Perl module tokens |
| `perl-module-token` | Boundary-safe module token replacement and variant helpers (deps: `perl-module-boundary`, `perl-module-name`, `perl-module-path`) |
| `perl-module-token-parser` | Module token parsing for import/reference workflows (deps: `perl-module-token-core`) |
| `perl-module-name` | Perl module-name separator normalization and canonical/legacy variant helpers |
| `perl-qualified-name` | Perl qualified-name parsing, splitting, and validation helpers |
| `perl-module-reference` | Cursor-aware module reference extraction (deps: `perl-module-name`, `perl-module-token-parser`, `perl-text-line`) |
| `perl-module-import` | Single-line use/require import head parsing (deps: `perl-module-path`, `perl-module-token`) |
| `perl-module-import-match` | Import-line module match predicates (deps: `perl-module-import`, `perl-module-boundary`, `perl-module-path`, `perl-module-token`) |
| `perl-module-path` | Perl module name/path conversion utilities (deps: `perl-module-name`) |
| `perl-module-rename` | Deterministic module-import line edit planning (deps: `perl-module-token`, `perl-module-import-match`, `perl-module-path`) |
| `perl-module-resolution` | Deterministic Perl module resolution (deps: `perl-module-resolution-path`, `perl-module-resolution-uri`) |
| `perl-module-resolution-path` | Perl module path resolution within workspace roots (deps: `perl-module-path`, `perl-path-security`) |
| `perl-module-resolution-uri` | Module URI resolution with workspace-safe search (deps: `perl-module-path`, `perl-path-security`, `perl-workspace-folder`) |
| `perl-text-line` | Text-line cursor and boundary helpers (deps: `perl-module-token-parser`) |
| `perl-line-index` | Byte-oriented line/column index for incremental parsing |
| `perl-source-file` | Shared Perl source file classification helpers |
| `perl-content-length-framing` | Shared Content-Length frame parsing and encoding for LSP and DAP |
| `perl-uri-classify` | URI classification and key normalization helpers |
| `perl-uri` | URI/filesystem path conversion and normalization (deps: `perl-uri-classify`) |
| `perl-path-normalize` | Secure workspace-relative path normalization |
| `perl-path-security` | Workspace-bound path validation and traversal prevention (deps: `perl-path-normalize`) |
| `perl-percentile` | Nearest-rank percentile helpers for integer latency samples |
| `perl-workspace-ignore` | Shared ignore rules for workspace traversal and filtering |
| `perl-workspace-folder` | Parse workspace folder declarations into filesystem paths (deps: `perl-uri`) |
| `perl-workspace-discovery` | Git-aware workspace file discovery (deps: `perl-source-file`, `perl-workspace-ignore`) |
| `perl-workspace-index-state-machine` | Index lifecycle state machine primitives |
| `perl-workspace-index-slo` | SLO tracking primitives for workspace index operations (deps: `perl-percentile`) |
| `perl-subprocess-runtime` | Shared subprocess execution abstraction with OS and mock runtimes |
| `perl-lsp-protocol` | JSON-RPC/LSP protocol types and capability configuration (deps: `perl-lsp-feature-flags`) |
| `perl-lsp-symbol-query` | Workspace symbol query matching and ranking helpers |
| `perl-lsp-feature-ids` | Canonical feature identifiers for LSP/DAP capability interoperability |
| `perl-lsp-capability-map` | Translating LSP ServerCapabilities to feature IDs (deps: `perl-lsp-feature-ids`) |
| `perl-lsp-feature-contracts` | Shared LSP feature contract model (deps: `perl-feature-catalog`, `perl-lsp-feature-ids`, `perl-lsp-capability-map`) |
| `perl-lsp-feature-flags` | Feature-flag and advertised-feature models (deps: `perl-lsp-feature-contracts`, `perl-lsp-feature-ids`) |
| `perl-lsp-feature-profile` | Canonical feature profile contract and parsing (deps: `perl-lsp-feature-contracts`) |
| `perl-lsp-feature-profile-cli` | CLI profile token parsing for feature profiles (deps: `perl-lsp-feature-profile`) |
| `perl-lsp-feature-policy` | Policy and profile helpers for capability selection (deps: `perl-lsp-feature-contracts`, `perl-lsp-feature-profile`, `perl-lsp-feature-flags`) |
| `perl-lsp-feature-grid` | BDD-aware feature-grid payload and profile-aware APIs (deps: `perl-lsp-feature-contracts`, `perl-lsp-feature-policy`) |
| `perl-lsp-feature-governance` | Feature profile governance facade (deps: `perl-lsp-feature-contracts`, `perl-lsp-feature-grid`, `perl-lsp-feature-policy`, `perl-lsp-feature-profile`, `perl-lsp-feature-profile-cli`) |
| `perl-lsp-on-type-formatting` | On-type formatting edit computation for Perl LSP |
| `perl-lsp-formatting-types` | Shared formatting DTOs for perl-lsp formatting workflows |
| `perl-lsp-diagnostic-types` | Shared diagnostic data types for Perl LSP crates |
| `perl-lsp-text-utils` | Text-manipulation and insertion-point helpers for LSP refactoring |
| `perl-lsp-import-management` | Perl import statement collection, ordering, and range detection |
| `perl-lsp-critic-parser` | Parsing Perl::Critic output lines |
| `perl-lsp-cancellation` | Enhanced LSP cancellation infrastructure with token/registry support |
| `perl-lsp-limits` | Bounded LSP limits and deadline policy |
| `perl-lsp-uri` | Typed URI parsing helpers for perl-lsp |
| `perl-lsp-config` | Configuration models for perl-lsp server and workspace behavior |
| `perl-lsp-input-validation` | LSP request and file-input validation helpers (deps: `perl-path-security`) |
| `perl-lsp-diagnostic-catalog` | Stable LSP diagnostic metadata catalog (deps: `perl-diagnostics-codes`) |
| `perl-diagnostics-codes` | Stable diagnostic codes and severity levels for Perl LSP |
| `perl-dap-command-args` | Shell-safe command argument formatting for perl-dap |
| `perl-dap-shell` | Shell argument and environment helpers for perl-dap |
| `perl-dap-value` | Perl DAP value model types |
| `perl-dap-eval` | Safe expression evaluation validation for Perl DAP |
| `perl-dap-stack` | Stack trace parsing and frame classification |
| `perl-dap-variables` | Variable rendering for Perl DAP (deps: `perl-dap-value`) |
| `perl-dap-platform` | Cross-platform runtime utilities for perl-dap (deps: `perl-dap-shell`) |
| `perl-dap-security` | Security validation primitives for perl-dap (deps: `perl-path-security`) |
| `perl-feature-catalog` | Shared feature catalog model and code-generation helpers |
| `perl-ci-hygiene` | Native Rust replacements for shell-based CI hygiene scripts |

The `[workspace.dependencies]` section of `Cargo.toml` classifies all of these
as "Tier 1: Leaf crates" since their internal dependencies are confined to other
Tier 1 crates. The crates listed above form a directed acyclic graph within
Tier 1, but they all resolve before any Tier 2 crate begins compilation.

**Count: 82 crates**

## Tier 2 -- Single-level Dependencies (Core Infrastructure)

Crates with **single-level** workspace dependencies above the Tier 1 leaf layer.
This tier contains the parser core engine and LSP transport layer.

| Crate | Key workspace deps | Description |
|-------|-------------------|-------------|
| `perl-parser-core` | `perl-lexer`, `perl-ast`, `perl-quote`, `perl-pragma`, `perl-edit`, `perl-builtins`, `perl-regex`, `perl-heredoc`, `perl-error`, `perl-tokenizer` | Core parser engine for perl-parser |
| `perl-tdd-support` | `perl-parser-core` | Test-driven development helpers for the Perl LSP ecosystem |
| `perl-lsp-transport` | `perl-lsp-protocol`, `perl-content-length-framing` | LSP transport layer with Content-Length message framing |
| `perl-lsp-tooling` | `perl-subprocess-runtime`, `perl-parser-core`, `perl-symbol-index`, `perl-lsp-performance`, `perl-lsp-critic-parser` | Tooling integration (perltidy, perlcritic, subprocess) |
| `perl-lsp-formatting` | `perl-lsp-tooling`, `perl-lsp-formatting-types` | LSP formatting provider with perltidy integration |
| `perl-lsp-diagnostics` | `perl-lsp-diagnostic-types`, `perl-parser-core`, `perl-semantic-analyzer`, `perl-workspace-index`, `perl-pragma` | LSP diagnostics provider |
| `perl-lsp-semantic-tokens` | `perl-parser-core`, `perl-lexer`, `perl-semantic-analyzer` | LSP semantic tokens provider |
| `perl-lsp-inlay-hints` | `perl-semantic-analyzer`, `perl-position-tracking`, `perl-parser-core` | LSP inlay hints provider |
| `perl-lsp-launcher` | `perl-lsp-feature-governance` | Typed CLI launch configuration |
| `perl-lsp-performance` | `perl-parser-core`, `perl-symbol-index` | Performance utilities |
| `perl-lsp-completion-item` | `perl-parser-core` | LSP completion item types and deterministic sorting |
| `perl-ast-utils` | `perl-ast` | AST range and insertion helpers |
| `perl-lsp-folding` | `perl-lexer`, `perl-parser-core` | Perl LSP folding range extraction |
| `perl-lsp-document-links` | `perl-module-import`, `perl-module-path` | Document-link extraction for use/require |
| `perl-lsp-workspace-symbols` | `perl-lsp-symbol-query`, `perl-module-path`, `perl-parser-core`, `perl-qualified-name`, `perl-semantic-analyzer` | Workspace symbol provider |
| `perl-lsp-rename` | `perl-parser-core`, `perl-semantic-analyzer`, `perl-keywords`, `perl-symbol-cursor` | LSP rename provider |
| `perl-lsp-completion` | `perl-lsp-completion-item`, `perl-parser-core`, `perl-semantic-analyzer`, `perl-workspace-index`, `perl-keywords`, `perl-path-security` | LSP completion engine |
| `perl-lsp-code-actions` | `perl-parser-core`, `perl-ast-utils`, `perl-lsp-text-utils`, `perl-lsp-rename`, `perl-lsp-import-management`, `perl-lsp-diagnostics` | LSP code actions provider |
| `perl-lsp-navigation` | `perl-parser-core`, `perl-module-path`, `perl-module-import`, `perl-qualified-name`, `perl-lsp-document-links`, `perl-lsp-symbol-query`, `perl-lsp-workspace-symbols` | LSP navigation providers |

**Count: 19 crates**

> **Note:** Several crates listed above as "Tier 2" in `[workspace.dependencies]`
> actually depend on Tier 3/4 crates (e.g., `perl-lsp-diagnostics` depends on
> `perl-semantic-analyzer`). The `[workspace.dependencies]` tier comments are
> approximate groupings; the `[workspace.metadata.publish]` section provides the
> exact topological order.

## Tier 3 -- Two-level Dependencies (Indexing & Coordination)

Crates with **two levels** of workspace dependencies. This tier handles
workspace indexing, incremental parsing, and cross-cutting refactoring.

| Crate | Key workspace deps | Description |
|-------|-------------------|-------------|
| `perl-workspace-index` | `perl-parser-core`, `perl-symbol-types`, `perl-uri`, `perl-workspace-index-slo`, `perl-workspace-index-state-machine` | Workspace indexing and refactoring orchestration |
| `perl-incremental-parsing` | `perl-parser-core`, `perl-edit`, `perl-lexer`, `perl-line-index` | Incremental parsing with subtree reuse |
| `perl-refactoring` | `perl-parser-core`, `perl-workspace-index`, `perl-module-path`, `perl-qualified-name` | Refactoring and modernization utilities |

**Count: 3 crates**

## Tier 4 -- Three-level Dependencies (Analysis & Provider Aggregation)

Crates with **three levels** of workspace dependencies. This tier contains the
semantic analyzer and the LSP provider aggregation layer.

| Crate | Key workspace deps | Description |
|-------|-------------------|-------------|
| `perl-semantic-analyzer` | `perl-parser-core`, `perl-workspace-index`, `perl-symbol-types` | Semantic analysis and symbol extraction |
| `perl-lsp-providers` | `perl-parser-core`, `perl-semantic-analyzer`, `perl-lsp-tooling`, `perl-lsp-formatting`, `perl-lsp-diagnostics`, `perl-lsp-semantic-tokens`, `perl-lsp-inlay-hints`, `perl-lsp-rename`, `perl-lsp-navigation`, `perl-lsp-completion`, `perl-lsp-code-actions`, `perl-lsp-folding`, `perl-lsp-uri` | LSP provider aggregation and tooling integrations |

**Count: 2 crates**

## Tier 5 -- Task Runner

| Crate | Key workspace deps | Description |
|-------|-------------------|-------------|
| `xtask` | `perl-feature-catalog`, `perl-parser`, `perl-parser-pest` (optional), `tree-sitter-perl-rs` (optional) | Task runner for the workspace |

**Count: 1 crate**

## Tier 6 -- Application / Executable Crates

Top-level application binaries that compose the full stack: parser library, LSP
server, and DAP server.

| Crate | Key workspace deps | Description |
|-------|-------------------|-------------|
| `perl-parser` | `perl-parser-core`, `perl-semantic-analyzer`, `perl-dead-code`, `perl-workspace-index`, `perl-refactoring`, `perl-lsp-providers`, `perl-keywords` | Native Perl parser (v3) with semantic analysis and LSP providers |
| `perl-lsp` | `perl-parser`, `perl-lsp-providers`, `perl-lsp-feature-governance`, `perl-lsp-transport`, `perl-module-resolution`, `perl-workspace-discovery`, `perl-dap` (optional) | Perl Language Server -- the top-level binary |
| `perl-dap` | `perl-dap-breakpoint`, `perl-dap-eval`, `perl-dap-platform`, `perl-dap-command-args`, `perl-dap-variables`, `perl-dap-stack`, `perl-dap-security`, `perl-module-path`, `perl-lsp-launcher` | Debug Adapter Protocol server for Perl |
| `perl-dap-breakpoint` | `perl-parser` | Breakpoint validation for Perl DAP |
| `perl-dead-code` | `perl-workspace-index` | Dead code detection for Perl codebases |

**Count: 5 crates**

## Tier 7 -- Legacy, Testing, and Tree-sitter Crates

Crates kept for compatibility, test corpus management, or the tree-sitter
integration layer. These sit outside the main LSP/DAP critical build path.

| Crate | Key workspace deps | Description |
|-------|-------------------|-------------|
| `perl-parser-pest` | *(external only)* | Legacy Pest-based Perl parser (v2) |
| `perl-corpus` | `perl-tdd-support` | Test corpus management and generators |
| `tree-sitter-perl-rs` | `perl-lexer`, `perl-parser`, `perl-ts-*` (optional) | Pure-Rust parser with tree-sitter S-expression output |
| `perl-ts-heredoc-analysis` | *(external only)* | Standalone heredoc analysis tools |
| `perl-ts-logos-lexer` | `perl-parser-pest` | Logos-based token parser |
| `perl-ts-heredoc-parser` | `perl-ts-heredoc-analysis`, `perl-parser-pest` | Heredoc parsing pipeline |
| `perl-ts-partial-ast` | `perl-ts-heredoc-analysis`, `perl-parser-pest` | Partial parse and anti-pattern AST |
| `perl-ts-advanced-parsers` | `perl-ts-heredoc-analysis`, `perl-ts-heredoc-parser`, `perl-ts-partial-ast`, `perl-parser-pest` | Composed parser experiments |

**Count: 8 crates**

## Summary by Tier

| Tier | Name | Count | Role |
|------|------|------:|------|
| 1 | Leaf crates | 82 | Foundation -- no workspace deps above Tier 1 |
| 2 | Single-level deps | 19 | Parser core, LSP transport, feature providers |
| 3 | Two-level deps | 3 | Workspace indexing, incremental parsing |
| 4 | Three-level deps | 2 | Semantic analysis, provider aggregation |
| 5 | Task runner | 1 | `xtask` |
| 6 | Applications | 5 | `perl-parser`, `perl-lsp`, `perl-dap` |
| 7 | Legacy / testing | 8 | `perl-parser-pest`, tree-sitter integration |
| | **Total** | **120** | |

> The total exceeds 117 workspace `members` (116 crates + `xtask`) because 4
> crates (`perl-dap-stack`, `perl-lsp-feature-policy`, `perl-lsp-formatting-types`,
> `perl-workspace-folder`) exist as path-dependency crates in
> `[workspace.dependencies]` and `[workspace.metadata.publish]` but are not
> listed in the `members` array. They are compiled transitively and published
> alongside the rest of the workspace. Additionally, 2 crates
> (`perl-lsp-providers` and `perl-lsp-protocol`) have complex dependency
> positions -- `perl-lsp-protocol` is classified as Tier 1 in
> `[workspace.dependencies]` because its only workspace deps are other Tier 1
> crates, while `perl-lsp-providers` sits in Tier 4 as the top-level aggregation
> crate.

## Crate Families

The workspace members are organized into naming-convention families. Each
family covers a single domain concern.

### `perl-module-*` -- Module Resolution (13 crates)

Microcrates for Perl module name parsing, path conversion, import analysis,
and `@INC`-based resolution.

| Crate | Tier |
|-------|------|
| `perl-module-token-core` | 1 |
| `perl-module-name` | 1 |
| `perl-module-path` | 1 |
| `perl-module-boundary` | 1 |
| `perl-module-token-parser` | 1 |
| `perl-module-token` | 1 |
| `perl-module-import` | 1 |
| `perl-module-import-match` | 1 |
| `perl-module-reference` | 1 |
| `perl-module-rename` | 1 |
| `perl-module-resolution-path` | 1 |
| `perl-module-resolution-uri` | 1 |
| `perl-module-resolution` | 1 |

### `perl-lsp-*` -- LSP Feature Providers (41 crates)

All crates providing LSP server features, from transport to individual
capability providers.

| Crate | Tier |
|-------|------|
| `perl-lsp-feature-ids` | 1 |
| `perl-lsp-symbol-query` | 1 |
| `perl-lsp-on-type-formatting` | 1 |
| `perl-lsp-formatting-types` | 1 |
| `perl-lsp-diagnostic-types` | 1 |
| `perl-lsp-text-utils` | 1 |
| `perl-lsp-import-management` | 1 |
| `perl-lsp-critic-parser` | 1 |
| `perl-lsp-cancellation` | 1 |
| `perl-lsp-limits` | 1 |
| `perl-lsp-uri` | 1 |
| `perl-lsp-config` | 1 |
| `perl-lsp-capability-map` | 1 |
| `perl-lsp-diagnostic-catalog` | 1 |
| `perl-lsp-input-validation` | 1 |
| `perl-lsp-feature-contracts` | 1 |
| `perl-lsp-feature-flags` | 1 |
| `perl-lsp-feature-profile` | 1 |
| `perl-lsp-feature-profile-cli` | 1 |
| `perl-lsp-feature-policy` | 1 |
| `perl-lsp-feature-grid` | 1 |
| `perl-lsp-feature-governance` | 1 |
| `perl-lsp-transport` | 2 |
| `perl-lsp-tooling` | 2 |
| `perl-lsp-formatting` | 2 |
| `perl-lsp-performance` | 2 |
| `perl-lsp-completion-item` | 2 |
| `perl-ast-utils` | 2 |
| `perl-lsp-folding` | 2 |
| `perl-lsp-document-links` | 2 |
| `perl-lsp-workspace-symbols` | 2 |
| `perl-lsp-rename` | 2 |
| `perl-lsp-completion` | 2 |
| `perl-lsp-code-actions` | 2 |
| `perl-lsp-navigation` | 2 |
| `perl-lsp-diagnostics` | 2 |
| `perl-lsp-semantic-tokens` | 2 |
| `perl-lsp-inlay-hints` | 2 |
| `perl-lsp-launcher` | 2 |
| `perl-lsp-protocol` | 1 |
| `perl-lsp-providers` | 4 |

### `perl-lsp-feature-*` -- Feature Governance Subsystem (8 crates)

A subset of `perl-lsp-*` that implements the feature-flag, profile, and
governance layer for capability negotiation.

| Crate | Tier |
|-------|------|
| `perl-lsp-feature-ids` | 1 |
| `perl-lsp-feature-contracts` | 1 |
| `perl-lsp-feature-flags` | 1 |
| `perl-lsp-feature-profile` | 1 |
| `perl-lsp-feature-profile-cli` | 1 |
| `perl-lsp-feature-policy` | 1 |
| `perl-lsp-feature-grid` | 1 |
| `perl-lsp-feature-governance` | 1 |

### `perl-dap-*` -- Debug Adapter Protocol (10 crates)

Components for the Perl Debug Adapter Protocol (DAP) server, from value types
to the top-level aggregation crate.

| Crate | Tier |
|-------|------|
| `perl-dap-command-args` | 1 |
| `perl-dap-shell` | 1 |
| `perl-dap-value` | 1 |
| `perl-dap-eval` | 1 |
| `perl-dap-stack` | 1 |
| `perl-dap-variables` | 1 |
| `perl-dap-platform` | 1 |
| `perl-dap-security` | 1 |
| `perl-dap-breakpoint` | 6 |
| `perl-dap` | 6 |

### `perl-ts-*` -- Tree-sitter Integration (5 crates)

Experimental and legacy parsers built on the tree-sitter grammar.

| Crate | Tier |
|-------|------|
| `perl-ts-heredoc-analysis` | 7 |
| `perl-ts-logos-lexer` | 7 |
| `perl-ts-heredoc-parser` | 7 |
| `perl-ts-partial-ast` | 7 |
| `perl-ts-advanced-parsers` | 7 |

### `perl-workspace-*` -- Workspace Discovery and Indexing (6 crates)

Crates for workspace file discovery, ignore rules, symbol indexing, and SLO
tracking.

| Crate | Tier |
|-------|------|
| `perl-workspace-ignore` | 1 |
| `perl-workspace-index-state-machine` | 1 |
| `perl-workspace-index-slo` | 1 |
| `perl-workspace-folder` | 1 |
| `perl-workspace-discovery` | 1 |
| `perl-workspace-index` | 3 |

### Core Leaf Crates

Foundational crates for tokens, AST, quoting, regex, heredocs, errors, and
symbols.

| Crate | Tier |
|-------|------|
| `perl-token` | 1 |
| `perl-quote` | 1 |
| `perl-keywords` | 1 |
| `perl-position-tracking` | 1 |
| `perl-regex` | 1 |
| `perl-symbol-types` | 1 |
| `perl-symbol-index` | 1 |
| `perl-ast` | 1 |
| `perl-lexer` | 1 |
| `perl-heredoc` | 1 |
| `perl-edit` | 1 |
| `perl-symbol-cursor` | 1 |
| `perl-pragma` | 1 |
| `perl-error` | 1 |
| `perl-tokenizer` | 1 |

## Build Time Impact

The tier structure directly affects how `cargo build` schedules compilation:

```
Time -->
|========= Tier 1 (82 crates, all in parallel) =========|
   |===== Tier 2 (19 crates, parallel) =====|
      |== Tier 3 (3 crates) ==|
         |= Tier 4 (2) =|
            | Tier 5 (1) |
               | Tier 6 (5) |
                  | Tier 7 (8) |
```

**Key observations:**

1. **Wide base.** 82 crates (Tier 1) compile simultaneously at the start of a
   build. Within Tier 1, Cargo still respects internal ordering (e.g.,
   `perl-token` before `perl-ast`) but the wavefront is very wide, saturating
   all available cores.

2. **Narrow top.** Only 5 crates sit at Tier 6 (`perl-parser`, `perl-lsp`,
   `perl-dap`, `perl-dap-breakpoint`, `perl-dead-code`), so the critical path
   through the dependency graph is short.

3. **Incremental builds.** Editing a Tier 1 leaf crate triggers recompilation
   of its transitive dependents. Editing a Tier 4+ crate only recompiles that
   crate and its few direct consumers. Most day-to-day changes happen in the
   higher tiers (LSP features, semantic analysis), keeping rebuild times low.

4. **Feature-gated optional deps.** Several crates use Cargo features to gate
   heavy dependencies (e.g., `perl-parser`'s `incremental` feature, `perl-lsp`'s
   optional `perl-dap` dep). This keeps the default build path lean.

5. **`perl-tdd-support` as dev-dependency.** Nearly every crate depends on
   `perl-tdd-support` for testing, but it is a `[dev-dependencies]` entry in
   most cases, so it does not affect the production build graph.

## Verification

The tier assignments in this document can be verified against the source of
truth:

```bash
# The [workspace.metadata.publish] section in Cargo.toml lists all
# publishable crates in topological dependency order:
grep -A 300 '\[workspace.metadata.publish\]' Cargo.toml

# The [workspace.dependencies] section has tier comments:
grep -B1 'Tier' Cargo.toml

# To inspect a specific crate's workspace dependencies:
grep 'perl-' crates/<crate-name>/Cargo.toml | grep workspace
```

The publish allowlist enforces topological order: `cargo publish` will fail if a
crate is listed before one of its dependencies, providing a machine-checked
guarantee that tier assignments are acyclic.
