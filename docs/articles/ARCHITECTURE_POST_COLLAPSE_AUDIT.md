# Architecture Post-Collapse Audit: State and Next Steps

*Status after the 135→31 microcrate collapse. What worked, what still needs surface
tightening, and why the remaining work is seam cleanup rather than another broad wave.*

---

## Current State

The collapse is operationally complete at the published crate level. As of 2026-04-24:

- **31 published crates** (from 135 at the project start)
- **All major collapse waves merged**: perl-module (Wave 1), perl-workspace (Wave A),
  perl-symbol, perl-diagnostics, perl-dap, perl-lexer, G1a/G1b providers, G2 runtime,
  G3 utilities, Wave D thin-facade pattern, Wave 4-Completion
- **Wave 4-Completion verified**: `perl-dead-code`, `perl-refactoring`, and
  `perl-incremental-parsing` absorbed into `perl-parser::dead_code`, `perl-parser::refactor`,
  and `perl-parser::incremental` respectively; all three marked `publish = false`

The remaining workspace members that are present but unpublished (keeping `publish = false`)
serve as implementation modules within their host crates. The directory count is higher than
the published count, but the Cargo API surface is controlled.

---

## What Is Working

### perl-parser-core as Kernel

`perl-parser-core` functions correctly as the parsing kernel — it owns the recursive descent
engine, AST nodes, token types, source position types, and error recovery infrastructure.
Crates that need parser types import from `perl-parser-core` directly (for core types) or
from `perl-parser` (for the full parser + analysis stack).

The layering is sound: `perl-lexer` → `perl-parser-core` → `perl-parser` is the canonical
direction. No upstream crate imports from a downstream neighbor.

### Domain-Separated Top-Level Crates

The four major domains have clear boundaries:

| Crate | Domain |
|-------|--------|
| `perl-parser` | Parsing, AST, analysis, incremental, refactoring |
| `perl-semantic-analyzer` | Scope resolution, symbol tables, type inference |
| `perl-workspace-index` | Cross-file indexing, workspace management, rename |
| `perl-dap` | Debug Adapter Protocol implementation |

These domains are separated by their dependency direction and their conceptual concerns.
`perl-semantic-analyzer` depends on `perl-parser-core` (for AST types) and
`perl-workspace-index` (for workspace symbol lookup). `perl-workspace-index` depends on
`perl-parser-core`. Neither depends on the other at the implementation level.

### LSP Thin-Facade Pattern

The `perllsp` published crate is a thin facade over `perl-lsp-rs` (the implementation
crate), which is itself a thin facade over the domain crates. Users install `perllsp`;
the Cargo API surface users see is `LspServer`, `run_stdio()`, and the protocol types.
The pattern is documented in `project_wave_d_facade_pattern.md` and is the established
house style for entry-point crates.

---

## Three Seams Needing Surface Tightening

The remaining work is not another broad collapse wave. It is surface tightening on three
specific seams where the public API currently exposes more than it should.

### Seam 1: `perl-lsp-rs` Module Surface

**Current state**: `perl-lsp-rs/src/lib.rs` exports the following as `pub mod`:

```
cancellation, cli, convert, diagnostics_catalog, dispatch, execute_command,
fallback, features, handlers, protocol, runtime, security, server, state,
textdoc, transport, util
```

Plus re-exports: `JsonRpcError`, `JsonRpcRequest`, `JsonRpcResponse`, `LspServer`,
`BridgeAdapter`, `capability_map`, `run_stdio()`.

**The problem**: The `pub mod` declarations expose the entire implementation hierarchy —
handler dispatch tables, transport internals, state machine internals — as part of the
crate's public Cargo surface. Any downstream crate that depends on `perl-lsp-rs` (rather
than `perllsp`) has access to all of these modules. This is a surface wider than intended
for a crate that is supposed to be an implementation detail.

**Target state**: The implementation modules should be `pub(crate)`. The re-exported types
(`LspServer`, `run_stdio()`, `JsonRpcError/Request/Response`) should remain public. The
`cli`, `server`, and `protocol` modules may warrant `pub use` re-exports at top level,
but `cancellation`, `transport`, `runtime`, `state`, and `dispatch` are implementation
concerns that should not be part of the public surface.

**Scope**: This is a 1-2 day refactor in `perl-lsp-rs/src/lib.rs` — changing `pub mod`
to `pub(crate) mod` for implementation modules. It requires verifying that no external
crate imports from the affected modules. Given the `perllsp` → `perl-lsp-rs` dependency
direction, this is likely safe.

### Seam 2: `perl-semantic-analyzer` Re-Export Sprawl

**Current state**: `perl-semantic-analyzer/src/lib.rs` re-exports:

```rust
pub use perl_parser_core::{Node, NodeKind, SourceLocation};
pub use perl_parser_core::{
    Parser, ast, edit, error, parser, parser_context, position,
    pragma_tracker, quote_parser, util,
};
pub use perl_workspace::workspace_index;
```

Plus its own analysis modules: `class_model`, `declaration`, `index`, `scope_analyzer`,
`semantic`, `symbol`, `type_inference`.

**The problem**: Re-exporting `edit`, `parser_context`, `pragma_tracker`, `quote_parser`,
and `util` from `perl-parser-core` through `perl-semantic-analyzer` creates a secondary
import path for these types. Consumers can now reach `perl_parser_core::edit` via either
`perl_parser_core::edit` or `perl_semantic_analyzer::edit`. This is confusing (which is
canonical?) and fragile (a refactor of the semantic analyzer's internals could break callers
who were using the re-exported path).

**Target state**: Re-export only the types that `perl-semantic-analyzer` consumers need
to interact with the semantic analyzer's own types: `Node`, `NodeKind`, `SourceLocation`
(because `semantic::Symbol` references them). Remove the module-level re-exports of
`edit`, `parser_context`, `pragma_tracker`, `quote_parser`, `util` — callers that need
these should import from `perl_parser_core` directly.

**Scope**: Verify that no downstream crate uses `perl_semantic_analyzer::edit` (or similar
re-exported path), then remove the re-exports. Lower risk than Seam 1 because semantic
analyzer consumers are more constrained.

### Seam 3: `perl-workspace-index` Parser Type Leakage

**Current state**: `perl-workspace-index/src/lib.rs` re-exports:

```rust
pub use perl_parser_core::line_index;
pub use perl_parser_core::{Node, NodeKind, SourceLocation};
pub use perl_parser_core::{Parser, ast, position};
```

**The problem**: `Parser`, `ast`, and `position` are parser-domain types being re-exported
from a workspace-domain crate. The workspace index crate owns workspace-level concerns
(file discovery, cross-file indexing, rename orchestration). It should not be a secondary
import path for parser types.

**Target state**: Re-export only `Node`, `NodeKind`, `SourceLocation` (needed because
workspace index APIs return these types in their signatures), and `line_index` (needed
for position mapping). Remove `Parser`, `ast`, `position` re-exports — callers should
get these from `perl_parser_core`.

**Scope**: Same as Seam 2 — verify no downstream caller uses the re-exported path, then
remove.

---

## `perl-parser-core` Public Re-Export Sprawl

`perl-parser-core/src/lib.rs` itself has a wide public surface:

```
engine::ast, engine::ast_v2, engine::parser_context, engine::pragma_tracker,
engine::quote_parser, engine::error, engine::parser, engine::position,
syntax::edit, syntax::heredoc, syntax::path_normalize, syntax::path_security,
syntax::percentile, syntax::qualified_name, syntax::source_file, syntax::text_line,
parser::Parser, error::classifier, error::recovery, builtins::builtin_signatures,
perl_lexer::tokenizer::util, line_index (inline mod)
```

This is a wide surface, but many of these are legitimately needed by multiple downstream
crates. The narrowing target is:

- **Core public surface** (narrow, stable, must stay pub): `Parser`, `Node`, `NodeKind`,
  `SourceLocation`, `ParseOutput`, `ParseError`, `ParseResult`, `ParseBudget`, `BudgetTracker`
- **Parser-internal utilities** (should be `pub(crate)` or accessed via `perl-parser`):
  `pragma_tracker`, `quote_parser`, `parser_context`, `ast_v2`, `heredoc_collector`,
  `path_normalize`, `path_security`, `percentile`, `source_file`, `text_line`, `qualified_name`
- **Questionable**: `edit`, `error_classifier`, `error_recovery` — these are needed by
  parser analysis code but may not need to be public at the `perl-parser-core` level

This narrowing is post-alpha work. Before narrowing `perl-parser-core`'s surface, all
downstream callers must be audited. The surface is wide precisely because it evolved
during a period when crate boundaries were being determined. Narrowing it risks breaking
callers that are legitimately using the wider surface for now.

---

## Parser-Family Crate Status

The "parser-family" crates are those that were absorbed into `perl-parser::*` during
collapse waves but remain as workspace members:

| Crate | Status | Location after absorption |
|-------|--------|--------------------------|
| `perl-dead-code` | `publish = false` | `perl-parser::dead_code` |
| `perl-refactoring` | `publish = false` | `perl-parser::refactor` |
| `perl-incremental-parsing` | `publish = false` | `perl-parser::incremental` |
| `perl-ast-v2` | Still present as workspace member | Used via `perl-parser-core::ast_v2` |

**The tracker-vs-manifest inconsistency**: The Wave 4-Completion spec described absorbing
`perl-dead-code`, `perl-refactoring`, and `perl-incremental-parsing` into
`perl-parser::*`, with the absorbed crates marked `publish = false`. This is done. The
workspace still contains the directories (and Cargo.toml files with `publish = false`),
but they are no longer separate published crates. The "published count" baseline reflects
this correctly (34 after Wave 4, from 37 pre-Wave-4).

Other parser-adjacent crates in the workspace:
- `perl-parser-bench`: Benchmarking crate for the parser; `publish = false`; correctly
  excluded from the published surface
- `perl-parser-pest`: Legacy Pest grammar; kept for benchmarking comparison; `publish = false`
- `perl-ast`: Original AST types; superseded by `perl-parser-core::ast`; should be verified
  `publish = false` and removed from doc references

---

## One Collapse to Consider Post-Alpha

The spec called out `perl-line-index` → `perl-position-tracking` as the one remaining
potential collapse. These two crates overlap conceptually: `perl-line-index` owns
`LineIndex` (line-ending-aware UTF-8/UTF-16 position mapping); `perl-position-tracking`
owns position coordinate types and UTF-16 offset utilities.

**Current separation logic** (from Amendment 4 in the collapse planning): They were kept
separate pre-alpha because merging them would require an API decision about the canonical
home for `PositionMapper`, `LineEnding`, and the UTF-16 surrogates logic. That decision
was correctly deferred to avoid blocking the alpha timeline.

**Post-alpha recommendation**: Audit actual callers of each crate. If `perl-line-index`
has fewer than 5 external callers and `perl-position-tracking` is mostly consumed by the
same callers, the merge is low-risk. If they have different caller sets, keep them separate.
The criterion is call-graph coupling, not crate count.

---

## Operational Advice

**Do not collapse before alpha.** The current 31 published crates is a clean, tested
boundary. Collapsing more before v0.13.0 introduces rebase risk (every merge touches
Cargo.toml and lib.rs of the host crate) and surface instability (callers may depend on
current module paths that will change in a collapse).

**Surface tightening is safe to do now.** Changing `pub mod` to `pub(crate) mod` for
implementation modules in `perl-lsp-rs`, removing excess re-exports from
`perl-semantic-analyzer` and `perl-workspace-index` — these changes do not affect the
Cargo dependency graph and do not require version bumps unless the removed paths were
part of the documented public API (they are not).

**Audit the publish surface before the v0.13.0 release.** Run `cargo xtask publish-closure`
to verify no absorbed crate name appears in the publish allowlist. Verify `perl-parser-bench`,
`perl-parser-pest`, `perl-ast-v2`, and `perl-ast` are marked `publish = false`. The
allowlist is hand-maintained and silently excludes new crates until a live publish fails.

---

_Related: `docs/articles/AGGREGATOR_ABSORPTION_PATTERN.md`, `memory/project_microcrate_collapse_v014.md`, `memory/project_wave_d_facade_pattern.md`_
