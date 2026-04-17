# From Zero to 98 LSP Features: Building a Modern Language Server for Perl in Rust

> How the perl-lsp project achieved near-complete Language Server Protocol coverage
> for one of programming's most syntactically challenging languages.

---

## Introduction: Why Perl Needs a Modern LSP

Perl powers critical infrastructure across finance, bioinformatics, systems
administration, and web development. Yet for decades, Perl developers have worked
with minimal IDE support compared to languages like TypeScript, Python, or Go.
The reason is not a lack of interest -- it is that Perl's syntax is among the
hardest to analyze statically. Context-sensitive parsing, sigil-based variable
types, bareword ambiguity, heredocs, quoting operators, regular expression
delimiters, and the infamous "only perl can parse Perl" reputation have kept
tooling authors at bay.

The perl-lsp project set out to change that. Built from scratch in Rust, it now
implements 98 of the 98 trackable features in the LSP 3.18 specification -- 100%
protocol compliance -- along with a full Debug Adapter Protocol server, a
recursive descent parser, and a semantic analysis engine. The journey from an
initial tree-sitter grammar to a production LSP server spanning 121 crates and
over 5,400 commits is a story about architecture, discipline, and the
surprising depth of Perl's syntax.

---

## The LSP Protocol Challenge

The Language Server Protocol defines dozens of capabilities across several
categories: text document features (completion, hover, definition, references,
rename, diagnostics, formatting, folding, semantic tokens, inlay hints, code
actions, code lens, signature help, and more), workspace features (symbol search,
diagnostics, file operations, configuration), window features (progress,
messages, telemetry), and protocol lifecycle (initialization, shutdown,
cancellation, capability registration).

Most language servers implement a subset. TypeScript's server covers the core
features it invented the protocol for. Rust-analyzer is famously complete. But
reaching near-total coverage for a dynamically-typed language with Perl's
syntactic complexity is a different order of challenge.

The perl-lsp project tracks every LSP feature in a canonical `features.toml`
file at the repository root. Each feature entry records its LSP spec version,
area (text_document, workspace, window, protocol, notebook, debug), maturity
level, whether it is advertised to clients, and the test files that validate it.
As of v0.12.x, the catalog contains 98 trackable features, all at GA maturity:

- **53 user-visible features** (the ones that `counts_in_coverage` tracks)
- **44 protocol plumbing features** (lifecycle, sync, refresh routes,
  configuration, window management)
- **10+ DAP features** (breakpoints, watchpoints, exception handling, inline
  values, debug completions, module inspection)

The distinction between user-visible and plumbing features is important. A
language server can claim high coverage by counting `initialize` and `shutdown`
as features. The perl-lsp project tracks both numbers honestly, reporting 100%
user-visible coverage (53/53) and 100% protocol compliance (98/98) separately.

---

## Architecture: A Custom LSP Runtime

### No tower-lsp

Unlike many Rust LSP implementations that build on the `tower-lsp` framework,
perl-lsp implements its own protocol stack. The server is organized in layers:

- **Transport layer** (`perl-lsp-transport`): Message framing over stdio and TCP,
  using Content-Length framing per the LSP specification.
- **Protocol layer** (`perl-lsp-protocol`): JSON-RPC 2.0 request, response, and
  notification types with serde serialization.
- **Dispatch layer** (`dispatch.rs`, `runtime/routing.rs`): Method-based routing
  that maps incoming JSON-RPC methods to handler functions.
- **Handler layer** (`handlers/`): Individual request and notification handlers.
- **Feature layer** (`features/`): All LSP capability implementations -- 34
  feature modules covering everything from completion to type hierarchy.
- **State layer** (`state/`): Document store, server configuration, and resource
  limits.

This architecture gives the project full control over threading, cancellation,
and resource management. The server supports both stdio mode (for editor
integration) and TCP socket mode (for remote development scenarios).

### The Parser Foundation

The LSP server is built on a native recursive descent parser (v3) written
entirely in Rust. The project actually went through three parser generations:

1. **v1 (tree-sitter)**: The project began as a tree-sitter grammar for Perl,
   with early commits implementing basic grammar rules, string literals, and
   subroutine declarations. Tree-sitter provided a solid foundation for syntax
   highlighting but lacked the flexibility needed for deep semantic analysis.

2. **v2 (Pest)**: A PEG-based parser using the Pest library. This is preserved
   in `perl-parser-pest` as a legacy crate but is no longer part of the default
   build.

3. **v3 (Native recursive descent)**: The current parser, implemented from
   scratch in Rust. It achieves near-100% Perl 5 syntax coverage with parsing
   times of 1-150 microseconds and incremental update times of 931 nanoseconds.

The parser produces a full AST that feeds into semantic analysis, symbol
extraction, type inference, and scope analysis -- all of which power the LSP
features.

---

## Feature-by-Feature: The Implementation Journey

### Core Navigation (LSP 3.0)

The foundational features came first: **completion**, **hover**, **definition**,
**references**, **document symbols**, and **workspace symbols**. These require a
working parser, a symbol table, and cross-file indexing.

Completion alone demonstrates the depth of Perl-specific work required. The
`perl-lsp-completion` crate provides context-aware completion with:

- 130+ Perl built-in function completions with signatures
- Variable completion (scalars, arrays, hashes) from the symbol table
- Special variable completion (`$_`, `@ARGV`, `%ENV`, etc.)
- Method completion after `->`, with DBI type inference (`$dbh` maps to
  `DBI::db`, `$sth` maps to `DBI::st`)
- Package member completion after `::`
- Moo/Moose `has(...)` option-key completion
- Cross-file symbol completion from the workspace index
- Test::More/Test2::V0 function completions in test contexts
- Secure file-path completion inside string literals

Each completion source is implemented as a separate internal module, and results
are deduplicated and sorted deterministically.

### Semantic Intelligence (LSP 3.6-3.17)

As the project matured, more sophisticated features were added:

- **Type definition** and **implementation** (LSP 3.6): Navigate to where a
  type is defined or where an interface is implemented.
- **Call hierarchy** (LSP 3.16): Navigate caller/callee relationships across
  the workspace.
- **Type hierarchy** (LSP 3.17): Navigate Perl's class hierarchy built from
  `use parent`, `use base`, and `@ISA` declarations.
- **Semantic tokens** (LSP 3.16): Enhanced syntax highlighting that goes beyond
  tree-sitter patterns, using the semantic analyzer to classify tokens with
  context-aware precision.
- **Inlay hints** (LSP 3.17): Parameter names and type hints displayed inline.
- **Pull diagnostics** (LSP 3.17): Both document-level and workspace-wide pull
  model diagnostics.

### Code Intelligence

- **Code actions**: Quick fixes and refactoring suggestions, with import
  management delegated to a dedicated `perl-lsp-import-management` microcrate.
- **Code lens**: Reference counts and actionable information overlaid on
  subroutine declarations.
- **Rename**: Workspace-wide symbol renaming with conflict detection and a
  `prepareRename` validation step.
- **Linked editing** (LSP 3.16): Synchronized editing of related tokens.

### Formatting and Diagnostics

- **Document formatting**: Integration with Perl::Tidy via a subprocess runtime
  abstraction. The `FormattingProvider` wraps perltidy execution behind a generic
  `SubprocessRuntime` trait, supporting full-document formatting, range
  formatting, and the LSP 3.18 multi-range formatting proposal.
- **On-type formatting**: Auto-indentation on keystroke, extracted into its own
  `perl-lsp-on-type-formatting` SRP microcrate.
- **Diagnostics**: A multi-source diagnostic pipeline combining parse errors,
  scope analysis (unused/undeclared/shadowed variables), lint checks
  (assignment-in-condition, numeric comparison with undef), deprecated syntax
  detection, strict/warnings compliance, and workspace-wide dead code detection.
- **Perl::Critic integration**: Structured parsing of perlcritic verbose output
  into LSP diagnostics via `perl-lsp-critic-parser`.

### LSP 3.18 and Beyond

The project tracks the latest LSP specification proposals:

- **Inline completion** (LSP 3.18): AI-powered inline completion suggestions.
- **Multi-range formatting** (LSP 3.18): The `textDocument/rangesFormatting`
  proposed method.
- **Virtual document content** (LSP 3.18): `workspace/textDocumentContent` for
  virtual file providers.
- **Folding range refresh** (LSP 3.18): Server-initiated folding range
  refresh requests.

---

## Debug Adapter Protocol: Beyond Editing

The perl-lsp project does not stop at editing. The `perl-dap` crate implements a
full Debug Adapter Protocol server, enabling debugging support in VSCode, Neovim,
Emacs, and other DAP-compatible editors.

### Dual-Mode Architecture

The DAP server supports two operating modes:

- **Native mode** (default): Drives `perl -d` directly, with the adapter
  managing all communication with the Perl debugger. This provides tight
  integration with the parser for AST-based breakpoint validation.
- **Bridge mode**: Proxies DAP messages to Perl::LanguageServer's existing DAP
  implementation, providing immediate debugging support while the native adapter
  matures.

### Debugging Features

The DAP implementation covers the full debugging lifecycle:

- **Breakpoints**: Source breakpoints with AST validation (verifying breakpoints
  are on executable lines), hit-count breakpoints (`>= N`, `== N`, `%N`), and
  logpoint breakpoints that emit output without stopping.
- **Exception handling**: Break on `die`/uncaught exceptions and
  `warn`/`carp`/`cluck` warnings.
- **Watchpoints**: Data breakpoints via the Perl debugger's `w`/`W` commands.
- **Execution control**: Step over, step in, step out, continue, pause.
- **Inspection**: Stack traces, variable scopes with lazy loading, expression
  evaluation with safe-eval guards, and loaded module inspection via `%INC`.
- **Debug console**: Autocomplete with Perl keywords in the debug console.
- **Inline values**: Variable values displayed inline during debugging.

### DAP Microcrates

The DAP implementation follows the same SRP decomposition as the LSP server:

- `perl-dap-breakpoint`: AST-based breakpoint validation
- `perl-dap-eval`: Safe expression evaluation
- `perl-dap-stack`: Stack trace parsing from debugger output
- `perl-dap-variables`: Variable rendering and parsing
- `perl-dap-value`: Shared `PerlValue` model
- `perl-dap-command-args`: Command argument formatting
- `perl-dap-shell`: Shell and environment helpers
- `perl-dap-platform`: Cross-platform path resolution
- `perl-dap-security`: Security validation (path traversal prevention,
  command injection protection, expression sanitization)

---

## Perl-Specific Challenges

### Module Resolution Across @INC

Perl's module system maps package names like `Foo::Bar::Baz` to filesystem
paths like `Foo/Bar/Baz.pm`, searched across the `@INC` include path. The
perl-lsp project handles this through a family of module resolution microcrates:

- `perl-module-token-core` and `perl-module-token`: Tokenize module names
- `perl-module-name`: Module name normalization
- `perl-module-path`: Filesystem path construction from module names
- `perl-module-resolution-path`: Path-based resolution against include
  directories
- `perl-module-resolution-uri`: URI-based resolution for LSP
- `perl-module-resolution`: Unified resolution facade
- `perl-module-import` and `perl-module-import-match`: Import statement analysis
- `perl-module-boundary`: Module boundary detection
- `perl-module-reference`: Module cross-reference tracking
- `perl-module-rename`: Module rename operations

This family of 13 microcrates ensures that every aspect of Perl's module system
is handled correctly and testable in isolation.

### Moose and Moo Attribute Handling

Perl's object systems -- Moose, Moo, and Class::Accessor -- introduce
syntactic patterns that look like function calls but define class attributes:

```perl
has 'name' => (is => 'ro', isa => 'Str', default => 'World');
```

The completion provider detects `has(...)` contexts and offers option-key
completion (is, isa, default, builder, lazy, required, etc.). The type
hierarchy provider understands `use parent`, `use base`, and Moose's `extends`
to build class hierarchies.

### Hash Key Context and Bareword Analysis

One of the trickiest Perl analysis challenges is distinguishing hash keys from
barewords under `use strict`. In `$hash{key}`, the bareword `key` is a hash
key and should not trigger a strict violation. But in `print FOO`, the bareword
`FOO` might be a filehandle or an error.

The semantic analyzer implements a hash key context detector that traverses the
AST parent chain to identify hash subscripts, hash literals, hash slices, and
nested hash access patterns. This eliminates false positives that plague simpler
Perl analysis tools.

### Cross-File Navigation: The Dual Indexing Strategy

The workspace index uses a dual indexing strategy (introduced in PR #122) that
indexes symbols under both their qualified and bare names:

```rust
// Index under bare name
file_index.references.entry(bare_name.to_string())
    .or_default().push(symbol_ref.clone());
// Index under qualified name
file_index.references.entry(qualified)
    .or_default().push(symbol_ref);
```

This handles Perl's flexible calling conventions where `Utils::process_data()`,
`process_data()`, and `&process_data()` all reference the same function.
Reference searches check both name forms and deduplicate results.

### UTF-16 Position Security

LSP uses UTF-16 offsets (a legacy of VSCode's JavaScript internals), while Rust
uses UTF-8 byte offsets. Incorrect conversion can cause boundary violations with
multi-byte characters and emoji. The position mapping system provides symmetric,
bounds-checked conversion with overflow protection, ensuring round-trip accuracy
for all Unicode content.

---

## The Feature Governance Innovation

One of the most distinctive aspects of the perl-lsp architecture is its feature
governance subsystem -- a family of seven microcrates that manage which LSP
capabilities are advertised, compiled, and reported:

### The Governance Stack

1. **`perl-lsp-feature-ids`**: Canonical string constants for every feature
   (`lsp.completion`, `lsp.hover`, `dap.core`, etc.). These are the
   single source of truth that prevents identifier drift across the codebase.

2. **`perl-lsp-feature-flags`**: `BuildFlags` and `AdvertisedFeatures` structs
   that map feature identifiers to boolean toggles. `BuildFlags` controls
   compile-time inclusion; `AdvertisedFeatures` controls what the server
   announces during `initialize`.

3. **`perl-lsp-feature-contracts`**: The `FeatureProfileKind` enum
   (GaLock, Production, All) and `BddFeatureRow` type for BDD-style
   reporting. This crate also includes a build-time code generator that reads
   `features.toml` and produces a Rust module with all feature metadata.

4. **`perl-lsp-feature-policy`**: Maps high-level profile decisions to runtime
   `BuildFlags`. Handles runtime tooling checks (e.g., formatting is only
   advertised if perltidy is installed).

5. **`perl-lsp-feature-profile`**: Profile name parsing and normalization.
   Handles aliases (`ga`, `ga-lock`, `ga_lock` all resolve to `GaLock`).

6. **`perl-lsp-feature-profile-cli`**: CLI argument parsing for profile
   selection.

7. **`perl-lsp-feature-grid`**: BDD grid reporting that produces JSON
   payloads showing feature compliance per profile. The grid includes
   stable column definitions, per-profile compliance percentages, and
   multi-profile projections.

8. **`perl-lsp-feature-governance`**: The facade crate that re-exports
   the entire governance stack through a single stable API.

### Feature Profiles

The profile system allows different deployment scenarios:

- **ga-lock**: Conservative profile for environments that need maximum
  stability. Excludes experimental features.
- **production**: Standard runtime profile. Enables all GA features, with
  formatting conditional on perltidy availability.
- **all**: Every in-tree feature enabled. Used for test matrices, snapshots,
  and CI compliance reporting.

This governance system means that the server's capability advertisement is
derived mechanically from a TOML catalog, not hand-coded in the `initialize`
response. Adding a new LSP feature means adding an entry to `features.toml`
and implementing the handler -- the governance stack propagates the change
through profiles, flags, and reporting automatically.

---

## Workspace Indexing and Cross-File Intelligence

The `perl-workspace-index` crate is the backbone of cross-file features. It
maintains an in-memory index of all symbols, references, and module declarations
across the workspace.

### Index State Machine

The workspace index uses an eight-state state machine for lifecycle management:

- **Idle** -> **Initializing** -> **Building** -> **Ready**
- **Ready** -> **Updating** (incremental) -> **Ready**
- **Ready** -> **Invalidating** -> **Building** (full rebuild)
- Any state -> **Degraded** or **Error** on failure

Guarded transitions prevent invalid state changes, and the state machine
integrates with SLO tracking to monitor per-operation latency.

### Production Coordinator

The `ProductionIndexCoordinator` integrates the index, a bounded LRU cache
(with TTL-based eviction and memory size estimation), and an SLO tracker into
a single coordination layer. Performance benchmarks show:

- ~368 microseconds for initial small index
- ~721 microseconds for initial medium index
- ~213 microseconds for incremental updates

### Document Store

A thread-safe document store manages open files with URI normalization, version
tracking, and text caching. It uses `parking_lot::RwLock` for the index and
`std::sync::RwLock` for the store, chosen for their different performance
characteristics under the server's access patterns.

---

## The Microcrate Strategy

With 121 crates in the workspace, perl-lsp takes the microcrate strategy to an
extreme. The workspace is organized into seven dependency tiers:

| Tier | Description | Examples |
|------|-------------|---------|
| 1 | Leaf crates, no internal deps | `perl-token`, `perl-ast`, `perl-lsp-feature-ids` |
| 2 | Single-level deps | `perl-parser-core`, `perl-tokenizer`, `perl-module-name` |
| 3 | Two-level deps | `perl-workspace-index`, `perl-module-resolution` |
| 4 | Three-level deps | `perl-semantic-analyzer`, `perl-lsp-providers` |
| 5 | Task runner | `xtask` |
| 6 | Application crates | `perl-parser`, `perl-lsp`, `perl-dap` |
| 7 | Legacy/testing | `perl-parser-pest`, `perl-corpus` |

This structure has concrete benefits:

- **Compile-time parallelism**: Independent crates compile in parallel, reducing
  incremental build times.
- **Test isolation**: Each crate has focused tests. The workspace runs 1,543 lib
  tests with only 2 tracked ignores.
- **API boundaries**: Each crate has a small, documented public API. The project
  enforces `#![deny(unsafe_code)]`, `#![warn(missing_docs)]`, and bans
  `unwrap()`, `expect()`, `panic!()`, `todo!()`, and `unimplemented!()` in
  production code.
- **SRP extraction**: As modules grow, they are extracted into dedicated
  microcrates (recent examples: `perl-lsp-folding`, `perl-lsp-completion-item`,
  `perl-lsp-import-management`, `perl-lsp-on-type-formatting`).

---

## The Performance Story

Performance is not an afterthought. The project targets sub-50ms response times
for LSP operations and achieves this through several strategies:

- **Incremental parsing**: 931 nanosecond incremental updates mean that
  on-keystroke operations do not need to re-parse the entire file.
- **Bounded caching**: LRU caches with TTL and memory limits for parsed ASTs,
  symbol tables, and workspace data.
- **O(1) symbol lookups**: The workspace index uses hash maps for both qualified
  and bare name lookups.
- **Cancellation support**: Completion and other long-running requests accept an
  `is_cancelled` callback that is checked at multiple points, supporting LSP
  request cancellation (`$/cancelRequest`).
- **Resource limits**: Configurable caps on file counts, symbol counts, and
  indexing time prevent the server from consuming unbounded resources on large
  workspaces.
- **SLO monitoring**: The production coordinator tracks per-operation latency
  percentiles and SLO compliance, providing visibility into performance
  regressions.

---

## Quality and Safety Engineering

### Zero Fatal Constructs

The project bans all fatal constructs in production code: no `unwrap()`,
`expect()`, `panic!()`, `todo!()`, `unimplemented!()`, `std::process::abort()`,
or `dbg!()`. The only exception is a single `#[allow(clippy::expect_used)]` for
`lsp_types::Uri` fallback in `util/uri.rs`. Instead, the codebase uses `?`,
`.ok_or_else()`, pattern matching, and `Result`/`Option` throughout.

### Mutation Testing

The project maintains an 87% mutation score, meaning that 87% of code mutations
(introducing bugs) are caught by the test suite. This is tracked as a quality
ratchet -- the score must not decrease.

### Security Hardening

Security is comprehensive:

- **Path traversal prevention** in file completion and DAP operations
- **Command injection protection** in subprocess execution (perltidy,
  perlcritic, perl debugger)
- **Expression sanitization** in DAP evaluate requests
- **Null byte rejection** in file paths
- **Windows reserved name filtering**
- **Workspace boundary enforcement** in all file operations

### Supply Chain Security

The project generates SBOMs in both SPDX and CycloneDX formats, runs
`cargo-audit` for vulnerability scanning, and uses `deny.toml` for dependency
policy enforcement. Release artifacts include provenance attestations verifiable
via `gh attestation verify`.

---

## What's Next

The perl-lsp project is in Initial Public Alpha at v0.12.x. The path forward
includes:

- **v0.11.0**: Complete Moo/Moose/Class::Accessor attribute resolution,
  cross-file type inference via `use parent`/`use base`, and native DAP
  enhancements.
- **v0.15.0 (Stability Contract)**: Formal API stability guarantees, contract-
  locked wire protocol, multi-release deprecation cycles, and platform support
  tiers.
- **Full LSP 3.18 compliance**: As proposed features stabilize, the project will
  adopt them.
- **Distribution**: Package manager distribution via Homebrew, apt, and other
  channels beyond the current cargo install and VSCode extension.

The project demonstrates that even the most syntactically challenging languages
can have first-class IDE support. With 98 LSP features, a full DAP
implementation, a three-generation parser, and a feature governance system that
mechanically ensures compliance, perl-lsp makes a case that the "only perl can
parse Perl" era is over.

---

*The perl-lsp project is open source under Apache-2.0/MIT dual license. For
current metrics, see
[CURRENT_STATUS.md](https://github.com/EffortlessMetrics/perl-lsp/blob/master/docs/project/CURRENT_STATUS.md).
For the canonical feature catalog, see
[features.toml](https://github.com/EffortlessMetrics/perl-lsp/blob/master/features.toml).*
