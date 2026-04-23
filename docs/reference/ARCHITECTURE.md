# Architecture Overview for Contributors

This document explains the codebase structure, data flow, and key design decisions
for new contributors. Read this before diving into source files.

## Why 128 crates?

The workspace uses a **microcrate architecture** to enforce single-responsibility
boundaries. Each crate owns one concern. This has two practical benefits:

1. **Parallel development** -- worktree agents or human contributors can edit
   separate crates at the same time with zero merge conflicts.
2. **Dependency clarity** -- circular imports between subsystems become compiler
   errors, not just style violations.

The cost is navigation overhead. Use the tier map below to orient quickly.

## Crate families

| Family prefix | Count | Role |
|---|---|---|
| `perl-lsp` | ~30 | LSP server binary, protocol runtime, feature providers |
| `perl-lsp-feature-*` | 7 | Feature governance (flags, profiles, policy, grid) |
| `perl-dap-*` | 10 | Debug Adapter Protocol |
| `perl-workspace-*` | 6 | Workspace discovery, indexing, and state machine |
| `perl-module-*` | 12 | Module resolution and import tracking |
| `perl-ts-*` | 5 | Tree-sitter integration utilities |
| core leaf | ~20 | Token, AST, quote, regex, heredoc, error |

## Tier dependency graph

The workspace is organized into tiers. Higher tiers depend on lower tiers;
the reverse direction is forbidden.

```
Tier 0 (leaf crates -- no internal deps)
  perl-token          Raw token type
  perl-ast            AST node definitions (NodeKind, SourceLocation)
  perl-ast-v2         Next-generation AST (ParseOutput, DiagnosticId)
  perl-error          Parse error types and budget tracking
  perl-heredoc        Heredoc content collector
  perl-quote          Quote-like operator parsing primitives
  perl-regex          Regex literal parsing
  perl-edit           Edit delta types for incremental parsing
  perl-keywords       Perl keyword sets
  perl-builtins       Builtin function signatures (including PHF variant)
  perl-diagnostics-codes  Stable diagnostic code definitions
  perl-symbol-types   Shared SymbolKind / VarKind enums

Tier 1 (single internal dep)
  perl-lexer          Context-aware tokenizer; reads perl-token
  perl-tokenizer      TokenStream; bridges lexer output to parser
  perl-position-tracking  Line-index, byte-offset/line-column mapping
  perl-line-index     Fast byte-offset to line-number lookup
  perl-qualified-name Package-qualified name utilities

Tier 2 (two-level deps)
  perl-parser-core    Recursive-descent parser engine (see below)
  perl-pragma         PragmaTracker for use/no directives
  perl-pod            POD parser

Tier 3 (three-level deps)
  perl-workspace-index  Workspace symbol index with dual indexing (see below)
  perl-lsp-diagnostic-types  Diagnostic wire types

Tier 4 (four-level deps)
  perl-semantic-analyzer  Semantic analysis, scope analysis, type inference
  perl-lsp-diagnostics    Diagnostic generation from AST + scope issues

Tier 5+ (LSP providers)
  perl-lsp-completion   Completion provider
  perl-lsp-navigation   Go-to-definition, references
  perl-lsp-rename       Symbol rename
  perl-lsp-semantic-tokens  Semantic highlighting
  perl-lsp-diagnostics  Diagnostic delivery (re-uses Tier 4 generator)
  ...and 20+ other perl-lsp-* providers

Tier top
  perl-lsp            LSP server binary (depends on all providers)
```

## Parser pipeline

```
Perl source (&str)
    |
    v
perl-lexer::PerlLexer
    Context-aware tokenizer
    Modes: ExpectTerm / ExpectOperator (slash disambiguation)
    Outputs: Token stream with type, text, start, end
    |
    v
perl-tokenizer::TokenStream
    Wraps the lexer into an iterator consumed by the parser
    Trivia (whitespace, comments) tracked separately
    |
    v
perl-parser-core::Parser  (crates/perl-parser-core)
    Recursive-descent, ~130 parse functions
    Error recovery: ERROR nodes for unrecognized input
    Output: Node tree (NodeKind enum with ~80 variants)
    Non-fatal parse errors collected in ParseOutput::diagnostics
    |
    v
perl-parser-core::Node / NodeKind
    All nodes carry a SourceLocation (byte-offset span)
    Program node is always the root
    ERROR nodes mark recovery boundaries
```

Key files in `crates/perl-parser-core/src/engine/parser/`:

| File | Purpose |
|---|---|
| `mod.rs` | `Parser` struct, `parse()` / `parse_with_recovery()` entry points |
| `statements.rs` | Statement-level parsing |
| `expressions/` | Expression parsing (precedence climbing) |
| `declarations.rs` | `use`, `my`, `sub`, package declarations |
| `control_flow.rs` | `if`, `while`, `for`, `foreach`, `do` |
| `heredoc.rs` | Heredoc content collection |
| `variables.rs` | Sigil-aware variable parsing |

## Semantic analysis pipeline

```
Node (AST root)
    |
    +---> perl-semantic-analyzer::analysis::symbol::SymbolExtractor
    |         Builds SymbolTable: all declared symbols with scopes
    |
    +---> perl-semantic-analyzer::analysis::scope_analyzer::ScopeAnalyzer
    |         Detects scope issues: unused vars, undeclared vars, shadowing
    |
    +---> perl-semantic-analyzer::analysis::semantic::SemanticAnalyzer
    |         Classifies tokens for LSP semantic highlighting
    |         Produces HoverInfo for LSP hover
    |
    +---> perl-semantic-analyzer::analysis::type_inference::TypeInferenceEngine
              Lightweight type inference for completion context
```

## Workspace index

`perl-workspace-index::WorkspaceIndex` maintains an in-memory symbol index
across all files in the workspace.

**Dual indexing pattern**: every function call is indexed under both its
fully-qualified form (`Package::name`) and its bare form (`name`). This
achieves ~98% reference coverage across all Perl calling styles
(`Utils::helper()`, `helper()`, `&helper()`).

```
WorkspaceIndex
  |-- definitions: HashMap<String, Vec<Location>>   qualified+bare
  |-- references:  HashMap<String, Vec<Location>>   qualified+bare
  |-- DocumentStore                                  open file cache
  |-- IndexStateMachine                              lifecycle states
  |     Idle -> Initializing -> Building -> Ready
  |     (also: Updating, Invalidating, Degraded, Error)
  `-- ProductionIndexCoordinator
        integrates index + BoundedLruCache + SloTracker
```

The index is updated incrementally: each `textDocument/didChange` triggers
re-indexing of the affected file only.

## LSP request flow

```
Editor (VS Code / Neovim / Emacs)
    |  stdin (JSON-RPC, Content-Length framing)
    v
perl-lsp/src/main.rs
    spawn_reader_thread: blocking reads from stdin
    forwards JsonRpcRequest to tokio mpsc channel
    |
    v
LspServer::serve_async (crates/perl-lsp-rs/src/runtime/serving.rs)
    Ingress loop -- no heavy work inline
    Calls scheduler::classify(method) -> RequestClass
    |
    +-- Control ($/cancelRequest) -----> processed inline (atomics only)
    |
    +-- Lifecycle (initialize, shutdown) -> exclusive mutation worker
    |                                        single sequential queue
    +-- Mutation (didOpen, didChange, ...) -> exclusive mutation worker
    |
    `-- ReadOnly (completion, hover, ...) -> bounded read pool
                                             N concurrent workers
    |
    v
LspServer::handle_request (crates/perl-lsp-rs/src/runtime/dispatch/)
    dispatch/lifecycle/  -- initialize, shutdown, exit
    dispatch/text_document/  -- per-method handlers
    dispatch/workspace/  -- workspace-level handlers
    |
    v
Feature provider (crates/perl-lsp-rs/src/features/)
    Calls into perl-semantic-analyzer, perl-workspace-index, providers
    Returns LSP-shaped response (serde_json::Value)
    |
    v
outbound::OutboundSender
    Writes Content-Length framed JSON to stdout
```

### Single-client model

Each server process serves exactly one editor connection. In stdio mode the
process is owned by the editor. In TCP mode (`--tcp <port>`) each accepted
connection spawns an independent `LspServer` with its own document map and
workspace index -- two editors on the same project get fully isolated views.

### Generation counter (stale-parse prevention)

Because parsing is async, a newer `didChange` can arrive while the previous
parse is still running. An `AtomicU32` generation counter on each
`DocumentState` prevents stale results from overwriting newer state:

1. `didChange` increments the counter, records `next_gen`.
2. Parser completes, checks current generation.
3. If generation advanced (another change arrived), the result is discarded.

## Diagnostic pipeline

```
textDocument/didChange or didOpen
    |
    v
LspServer (text_sync handler)
    Updates DocumentState, increments generation counter
    Schedules diagnostic re-run (debounced, ~200ms)
    |
    v
DiagnosticsProvider::get_diagnostics  (crates/perl-lsp-diagnostics)
    1. ParseOutput::diagnostics -> parse error diagnostics
    2. PragmaTracker::build     -> pragma context for scope analysis
    3. ScopeAnalyzer::analyze   -> scope issues (unused/undeclared/shadowed)
    4. scope_issues_to_diagnostics -> map issues to Diagnostic structs
    5. check_common_mistakes    -> assignment-in-condition, undef numeric
    6. check_strict_warnings    -> missing `use strict` / `use warnings`
    7. detect_heredoc_antipatterns -> heredoc pattern warnings
    8. detect_dead_code (workspace) -> unused symbols (non-WASM only)
    9. deduplicate_diagnostics  -> stable sort + dedup
    |
    v
textDocument/publishDiagnostics notification -> editor
```

Diagnostic codes are stable strings defined in `perl-diagnostics-codes`.
The full code table is in the `perl-lsp-diagnostics` CLAUDE.md.

## Feature governance

LSP features are governed through a multi-crate system under the
`perl-lsp-feature-*` family:

| Crate | Role |
|---|---|
| `perl-lsp-feature-ids` | Canonical feature ID constants (`"lsp.completion"`, etc.) |
| `perl-lsp-feature-flags` | Runtime feature flag booleans |
| `perl-lsp-feature-profile` | Profile enum: `Minimal`, `Standard`, `Full`, `Custom` |
| `perl-lsp-feature-policy` | Which features are on/off per profile |
| `perl-lsp-feature-contracts` | BDD acceptance criteria rows |
| `perl-lsp-feature-grid` | Compliance percentage reporting |
| `perl-lsp-feature-governance` | Facade crate -- single import for all of the above |

At startup `perl-lsp` resolves the active profile from the `--profile` CLI
flag (default: `Standard`). The profile determines which LSP capabilities
are advertised to the editor in the `initialize` response.

## DAP (Debug Adapter Protocol)

The `perl-dap-*` family (~10 crates) implements DAP for Perl debugging.
Current architecture is a **bridge**: the Rust adapter proxies DAP messages
to `Perl::LanguageServer` which drives `perl -d`.

```
VS Code (DAP client)
    |
    v
perl-dap  (bridge adapter)
    |
    v
Perl::LanguageServer (Perl process)
    |
    v
perl -d (Perl debugger)
```

Key microcrates: `perl-dap-config` (launch/attach config), `perl-dap-platform`
(cross-platform perl path resolution), `perl-dap-security` (path validation),
`perl-dap-eval` (safe eval), `perl-dap-types` (protocol wire types).

## Where to make changes

| What you want to change | Where to go |
|---|---|
| Parser bug (wrong AST for Perl construct) | `crates/perl-parser-core/src/engine/parser/` |
| Tokenizer / lexer bug | `crates/perl-lexer/src/` |
| Completion suggestions | `crates/perl-lsp-completion/src/` |
| Go-to-definition / references | `crates/perl-lsp-navigation/src/` |
| Diagnostics / lints | `crates/perl-lsp-diagnostics/src/lints/` |
| Scope analysis | `crates/perl-semantic-analyzer/src/analysis/scope_analyzer.rs` |
| Workspace symbol index | `crates/perl-workspace-index/src/workspace/workspace_index.rs` |
| LSP request routing | `crates/perl-lsp-rs/src/runtime/dispatch/` |
| LSP threading / scheduling | `crates/perl-lsp-rs/src/runtime/scheduler.rs` |
| Feature flags / profiles | `crates/perl-lsp-feature-policy/src/` |
| DAP debugging | `crates/perl-dap/src/` |
| Binary CLI / startup | `crates/perl-lsp-rs/src/main.rs` |

## Adding a new parser test

Tests live in individual files under `crates/perl-parser-core/tests/`.
Use the shared helpers:

```rust
// crates/perl-parser-core/tests/my_new_test.rs
mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_my_construct() {
    let source = r#"my $x = do { 1 };"#;
    assert_clean_parse(source);
}
```

Run with: `cargo test -p perl-parser-core`

After adding tests run `python3 scripts/update-current-status.py` to keep
the metrics pipeline accurate (required by CI).

## Adding a new LSP feature

1. Add the feature ID in `crates/perl-lsp-feature-ids/src/lib.rs`.
2. Wire the flag in `crates/perl-lsp-feature-policy/src/lib.rs` under the
   appropriate profile.
3. Create or extend a provider crate under `crates/perl-lsp-<name>/`.
4. Add the handler in `crates/perl-lsp-rs/src/runtime/dispatch/text_document/`
   or `workspace/`.
5. Add a BDD acceptance criterion in `crates/perl-lsp-feature-contracts/`.

## Performance expectations

| Operation | Target | Notes |
|---|---|---|
| Parse a typical file (2-5 KB) | 150-500 µs | Recursive descent, no allocation hot path |
| Workspace symbol lookup | < 1 ms | Dual-indexed HashMap |
| LSP completion response | < 50 ms | Includes parse + semantic analysis |
| Diagnostic re-run (debounced) | < 200 ms after edit | Scope analysis is the slow path |
| Request cancellation | < 50 ms | Atomic flag check in long-running ops |

## Related documents

- `CLAUDE.md` -- commands, CI gate tiers, workspace exclusions
- `docs/reference/LSP_IMPLEMENTATION_GUIDE.md` -- LSP protocol details, UTF-16 position handling
- `docs/reference/LSP_PROVIDERS_REFERENCE.md` -- per-provider API reference
- `docs/reference/SCOPE_ANALYZER_REFERENCE.md` -- scope analysis internals
- `docs/reference/WORKSPACE_NAVIGATION_GUIDE.md` -- workspace index API
- `docs/adr/` -- Architecture Decision Records for major decisions
- Per-crate `CLAUDE.md` files -- each crate directory carries its own context
