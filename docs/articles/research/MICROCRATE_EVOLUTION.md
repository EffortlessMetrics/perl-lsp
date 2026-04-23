# Microcrate Evolution: How 2 Crates Became 133

*The perl-lsp workspace grew from a two-crate monolith to 133 microcrates in nine months. This was not planned. It emerged from the operational needs of swarm development — each extraction solved a specific conflict, and the pattern compounded.*

---

## The Five Phases

```
Phase 0:  1 crate    (tree-sitter grammar, pre-project)
Phase 1:  2 crates   (parser + LSP, July 2025)
Phase 2:  ~8 crates  (LSP extraction, August 2025)
Phase 3:  ~30 crates (foundation microcrates, January 2026)
Phase 4:  ~62 crates (32 crates in ONE DAY, March 5, 2026)
Phase 5:  133 crates (provider cascade, March 2026)
```

---

## Phase 0: Tree-Sitter Grammar (Pre-Project)

**Crate count**: 1
**Period**: Before July 2025

The project began as a tree-sitter grammar for Perl — a single `grammar.js` file compiled to C by tree-sitter's generator. The grammar, scanner, and Rust bindings lived in one directory (`tree-sitter-perl/`).

This was not a Rust workspace. It was a tree-sitter project with Rust bindings added for integration testing. The `Cargo.toml` defined a single crate that wrapped the C parser.

The tree-sitter approach was abandoned after the seven parsing failures documented in `TREE_SITTER_BREAKAGE.md`. But the directory remains in the repo for benchmark comparison.

---

## Phase 1: Two-Crate Architecture (July -- August 2025)

**Crate count**: 2
**Commits**: ~382 (Era 1)
**Active days**: 42

The v3 recursive descent parser was built as two crates:
- `perl-parser` — the parser library (lexer, parser, AST types)
- `perl-lsp` — the LSP server binary

This is the classic library-plus-binary split. The parser crate contained everything: tokenization, parsing, AST node types, error recovery, scope analysis, position tracking. The LSP crate consumed the parser API and implemented the Language Server Protocol.

### Why Two Crates

The initial development was a single developer with Claude Opus. One conversation, one context, one codebase. There was no need for more granularity because there was no parallelism — everything happened sequentially in one dialogue.

The two-crate split existed purely for separation of concerns: the parser is a library that could theoretically be used outside the LSP (as a standalone Perl parser), and the LSP is a binary that consumes the library.

### What This Looked Like

```
crates/
  perl-parser/
    src/
      lib.rs           # Everything: lexer, parser, AST, scope, errors
      lexer.rs          # Tokenization
      parser.rs         # Parsing (eventually grew to ~5,000 lines)
      ast.rs            # AST node types
      scope.rs          # Scope analysis
  perl-lsp/
    src/
      main.rs           # LSP server binary
      providers.rs      # All LSP feature providers in one file
```

The parser's `lib.rs` and `parser.rs` were already showing god-file tendencies at ~3,000 lines each by the end of Era 1.

---

## Phase 2: LSP Extraction (August -- October 2025)

**Crate count**: ~8
**Commits**: ~800 (Era 2)
**Trigger**: First parallel agents

The first swarm experiments (Era 2) introduced parallel agents. Two agents working on different LSP features — completion and hover — kept conflicting on `providers.rs` because all providers lived in one file.

The solution: extract each major subsystem into its own crate.

### Crates Created

| Crate | Extracted From | Reason |
|-------|---------------|--------|
| `perl-lexer` | `perl-parser/lexer.rs` | Lexer needed independent testing |
| `perl-ast` | `perl-parser/ast.rs` | AST types shared across crates |
| `perl-scope-analyzer` | `perl-parser/scope.rs` | Scope analysis independent of parsing |
| `perl-semantic-analyzer` | New | Semantic tokens for IDE highlighting |
| `perl-dap` | New | Debug Adapter Protocol server |
| `perl-parser-core` | `perl-parser` | Core parsing engine (parser.rs became its own crate) |

### The Key Discovery

**Crate boundaries are conflict boundaries.** When two agents work in different crates, they touch different `Cargo.toml` files, different `src/` directories, and different test files. Git merge conflicts become nearly impossible.

This was discovered empirically, not designed. The extraction happened because of merge conflicts, and the absence of conflicts after extraction was noticed and recorded.

### Cost

Each extraction required:
1. Moving source files to a new directory
2. Creating a new `Cargo.toml` with correct dependencies
3. Updating `pub use` paths in the parent crate
4. Fixing import paths in all consumer crates
5. Adding the new crate to the workspace `Cargo.toml`

At 8 crates, this was manageable. Each extraction took 30-60 minutes of agent time.

---

## Phase 3: Foundation Microcrates (January 2026)

**Crate count**: ~30
**Commits**: ~350 (Era 3)
**Trigger**: Architectural review and quality investment

Era 3 was the "architectural sidechain" — a deliberate slowdown to invest in structure. The crate extractions in this phase were driven by design rather than conflict:

### Crates Created

| Category | Crates | Rationale |
|----------|--------|-----------|
| Parser internals | `perl-heredoc`, `perl-quote`, `perl-regex` | Isolate complex parsing subsystems |
| Type system | `perl-token`, `perl-error`, `perl-ast-v2` | Share types across parser and LSP |
| Workspace | `perl-workspace-discovery`, `perl-workspace-index` | Separate workspace management from LSP |
| Refactoring | `perl-refactoring` | Extract refactoring engine |
| Module resolution | `perl-module-resolver`, `perl-module-path` | Module `use`/`require` handling |
| Testing | `perl-tdd-support` | Shared test utilities |
| Position tracking | `perl-position-tracking` | Byte-offset-to-line-column mapping |
| Incremental | `perl-incremental-parsing` | Incremental re-parsing engine |

### Observation: perl-tdd-support

`perl-tdd-support` was created as a shared test utilities crate. By this phase, it already had 15+ reverse dependencies. By Phase 5, it would grow to 62 reverse dependencies and become the project's most coupled crate — a pattern documented in `HINDSIGHT_FINDINGS.md`.

### Cost and Benefit

Each extraction was more expensive than in Phase 2 because the codebase was larger and the dependency graph more complex. But the benefit was clear: each new crate was an independent compilation unit that could be tested, benchmarked, and developed in isolation.

---

## Phase 4: 32 Crates in ONE DAY (March 5, 2026)

**Crate count**: ~30 to ~62
**Date**: 2026-03-05
**Commits**: ~30 merge commits
**Trigger**: Codex batch extraction

This was the most dramatic phase. On a single day, 32 new crates were extracted from existing code — primarily using Codex batch operations that generated near-duplicate extraction PRs.

### Git Evidence

The March 5, 2026 git log shows:
```
codex/split-and-integrate-srp-microcrates-o2sot7
codex/split-and-integrate-srp-microcrates-ad24o6
codex/split-and-integrate-srp-microcrates-5mkjii
codex/split-out-srp-microcrates-for-integration-syg085
```

The `codex/split-and-integrate-srp-microcrates-*` branch pattern appears 4+ times — Codex generated near-duplicate PRs for the same extraction work. The triage pattern (cluster, compare, keep best, close rest) was applied to sort through them.

### Crates Created

| Crate | Extracted From | Lines |
|-------|---------------|-------|
| `perl-path-security` | `perl-lsp` | ~200 |
| `perl-path-normalize` | `perl-lsp` | ~150 |
| `perl-lsp-text-utils` | `perl-lsp` | ~100 |
| `perl-dap-security` | `perl-dap` | ~150 |
| `perl-dap-config` | `perl-dap` | ~300 |
| `perl-dap-session-model` | `perl-dap` | ~250 |
| `perl-workspace-index-monitoring` | `perl-workspace-index` | ~200 |
| `perl-diagnostic-type` | `perl-lsp` | ~100 |
| `perl-perlcritic-output` | `perl-lsp` | ~150 |
| `perl-completion-path` | `perl-lsp-completion` | ~200 |
| `perl-ts-statement-tracker` | `perl-ts-*` | ~200 |
| `perl-ast-v2` | `perl-ast` | ~300 |
| `perl-lsp-type-hierarchy` | `perl-lsp` | ~400 |
| ... | ... | ... |

Each crate was tiny — 100-400 lines. The average was approximately 200 lines of source code.

### Why It Worked

The microcrate architecture had reached a tipping point: the extraction pattern was so well-established that batch operations could apply it mechanically. Each extraction followed the same template:
1. Identify a cohesive set of functions in a large file
2. Create a new crate directory with `Cargo.toml`
3. Move the functions to `src/lib.rs`
4. Update the parent crate to `pub use` from the new crate
5. Update all consumers to import from the new crate

Codex's batch generation was imperfect (2-5 duplicates per extraction), but the volume was unprecedented: more crate extractions in one day than in the previous six months combined.

---

## Phase 5: Provider Cascade (March 6-19, 2026)

**Crate count**: ~62 to 133
**Commits**: ~721 (Era 5)
**Trigger**: Swarm operations at scale

The final phase saw the extraction of LSP providers into individual crates. This was driven directly by the swarm methodology: when 50+ agents needed to work on LSP features simultaneously, the remaining shared files became bottlenecks.

### Provider Crates

| Crate | Source | Purpose |
|-------|--------|---------|
| `perl-lsp-completion` | `perl-lsp/providers.rs` | Completion provider |
| `perl-lsp-hover` | `perl-lsp/providers.rs` | Hover provider |
| `perl-lsp-definition` | `perl-lsp/providers.rs` | Go-to-definition |
| `perl-lsp-references` | `perl-lsp/providers.rs` | Find references |
| `perl-lsp-rename` | `perl-lsp/providers.rs` | Rename provider |
| `perl-lsp-code-actions` | `perl-lsp/providers.rs` | Code action provider |
| `perl-lsp-diagnostics` | `perl-lsp/providers.rs` | Diagnostic provider |
| `perl-lsp-folding` | `perl-lsp/providers.rs` | Folding range provider |
| `perl-lsp-selection-range` | `perl-lsp/providers.rs` | Selection range |
| `perl-lsp-document-highlight` | `perl-lsp/providers.rs` | Document highlight |
| `perl-lsp-inline-completion` | `perl-lsp/providers.rs` | Inline completion |
| `perl-lsp-color-provider` | `perl-lsp/providers.rs` | Color provider |
| `perl-lsp-code-lens` | `perl-lsp/providers.rs` | Code lens |
| `perl-lsp-signature-help` | `perl-lsp/providers.rs` | Signature help |
| `perl-lsp-semantic-tokens` | `perl-lsp/providers.rs` | Semantic tokens |
| `perl-lsp-navigation` | `perl-lsp/` | Navigation provider |
| `perl-lsp-perltidy` | `perl-lsp/` | Perltidy integration |

### Feature Governance Crates

An additional layer of crates was created for feature governance:
- `perl-lsp-feature-*` — Feature flag and maturity tracking per LSP capability
- These crates are thin wrappers that control whether a feature is advertised to the client

### Why This Phase Was Different

Phases 1-4 extracted **infrastructure** (parser subsystems, type definitions, utility functions). Phase 5 extracted **features** (LSP providers, each with its own business logic).

Feature extraction has a qualitative difference: each provider crate has a well-defined input (the LSP request) and output (the LSP response). The interface is the LSP protocol itself. This makes the crate boundary extremely clean — the only coupling is through shared types (`perl-ast`, `perl-token`, `perl-diagnostic-type`).

---

## The Emergent Property

The microcrate architecture was not planned. It emerged from operational pressure:

```
Conflict → Extract → No conflict → More agents → More conflicts → More extraction
```

Each extraction reduced the surface area for merge conflicts. Each reduction allowed more parallel agents. More agents discovered more areas of conflict. The cycle continued until the natural stopping point: each LSP feature, each parser subsystem, and each utility function lived in its own crate.

### The Cost-Benefit Curve

```
Crates:     2       8       30      62      133
Conflicts:  High    Medium  Low     Rare    Zero
Build time: Fast    Fast    Medium  Medium  Medium+
Navigation: Easy    Easy    Medium  Hard    Hard
Agents:     1-2     3-5     10-20   50      100
```

At 133 crates:
- **Conflicts**: Zero. 100 agents work simultaneously without touching the same files.
- **Build time**: Incremental builds are fast (change one crate, rebuild one crate). Full builds are slower due to crate overhead.
- **Navigation**: Challenging for newcomers. The organization is logical but the sheer number is intimidating.
- **Agent capacity**: Unlimited. The architecture IS the parallelism enabler.

### Would You Do It Again?

Yes, but with different timing:
1. Phase 1-2 extractions (parser internals, LSP subsystems) were correct and timely
2. Phase 3 extractions (foundation microcrates) were premature for some crates — `perl-position-tracking` didn't need to be its own crate
3. Phase 4 (batch extraction day) was the right idea but produced too many tiny crates — some 100-line crates could have stayed as modules
4. Phase 5 (provider extraction) was essential for swarm scale and should have happened earlier

The ideal path would have been: extract providers first (Phase 5 work done in Phase 2), then extract parser subsystems as needed, and skip the tiny utility crate extractions unless they cause specific conflict.

---

## Impact on Development Methodology

The microcrate architecture fundamentally shapes how the swarm operates:

### Agent Assignment

Each agent is assigned to one or two crates. The agent's worktree isolation plus the crate boundary creates a double isolation layer:
- **Git isolation**: The worktree has its own working tree and index
- **Crate isolation**: The agent only modifies files in its assigned crate directory

### Verification

Each crate has its own verification command: `cargo clippy -p <crate> --tests && cargo test -p <crate>`. An agent can verify its work without building the entire workspace.

### Dependency Management

Crate dependencies are explicit in `Cargo.toml`. When an agent modifies a crate's public API, all downstream crates fail to compile — providing immediate feedback about breaking changes.

### Compilation Granularity

Incremental compilation operates at crate granularity. Changing one function in `perl-lsp-hover` only recompiles that crate and its downstream dependents (primarily `perl-lsp`). This makes agent verification fast: ~2-5 seconds for a typical microcrate.

---

## Crate Count Over Time

| Date | Crates | Event |
|------|--------|-------|
| Before Jul 2025 | 1 | Tree-sitter grammar |
| Jul 2025 | 2 | v3 parser + LSP |
| Aug 2025 | ~5 | First extractions |
| Oct 2025 | ~8 | Lexer, AST, scope |
| Jan 2026 | ~30 | Foundation microcrates |
| Mar 5, 2026 | ~62 | 32 in one day (Codex batch) |
| Mar 10, 2026 | ~90 | Provider cascade begins |
| Mar 15, 2026 | ~115 | Provider + feature governance |
| Mar 19, 2026 | 133 | Current state |

The growth is exponential through Phase 4, then linear through Phase 5 as the remaining extraction targets are exhausted. The curve is flattening — there are few remaining extractions that would provide meaningful conflict reduction.

---

## Lessons

1. **Architecture follows workflow**: The microcrate structure was shaped by swarm operations, not by upfront design. The architecture serves the development methodology.

2. **Small crates compound**: Each new microcrate makes the next extraction easier (smaller source crates, cleaner dependency graph).

3. **The 100-line crate is fine**: Traditional wisdom says "don't create tiny crates." For swarm development, a 100-line crate that enables conflict-free parallel work is more valuable than a 1,000-line file that blocks concurrent agents.

4. **Batch extraction works**: The Phase 4 single-day extraction of 32 crates was messy (duplicates, conflicts) but effective. The triage cost was lower than the conflict cost of leaving the code in shared files.

5. **The stopping point is real**: At 133 crates, further extraction provides diminishing returns. The conflict surface is already near zero. Additional splits would add overhead without reducing conflicts.
