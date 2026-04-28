# Codebase History: perl-lsp

> Source material for the launch article series. All claims are backed by git
> history, computed metrics, and CI receipts.

---

## Executive Summary

perl-lsp began life on **July 17, 2022** as `tree-sitter-perl-better`, a
JavaScript/C tree-sitter grammar for Perl authored by Veesh Goldman. Paul
"LeoNerd" Evans joined in January 2023 and contributed the external scanner
(C) and significant grammar coverage. The project sat in a steady-state
tree-sitter phase through early 2025.

In **July 2025**, Steven Zimmerman forked the tree-sitter grammar, introduced
a Pest (PEG) parser, then rapidly replaced it with a hand-written
recursive-descent parser in pure Rust. Within weeks an LSP server, VSCode
extension, and semantic analyzer were added. A Debug Adapter Protocol (DAP)
server followed in October 2025.

Starting in **January 2026**, the codebase underwent an aggressive
modularization into Single Responsibility Principle (SRP) microcrates,
growing from ~9 crates to **121 crates** by March 2026. All 97 LSP protocol
features reached GA maturity, and the project shipped its v0.10.0 Initial
Public Alpha in late February 2026.

### Key Numbers (as of March 2026)

| Metric | Value |
|--------|-------|
| Total commits | ~5,400 |
| Contributors | 15 human + 3 bots |
| Workspace crates | 121 |
| Rust lines of code | ~365,000 (code only, in `crates/`) |
| Total lines (all languages) | ~675,000 |
| Rust source files | 1,318 (in `crates/`) |
| Tests passing | ~4,400+ across the workspace |
| LSP features at GA | 97/97 (100%) |
| Fuzz targets | 9 |
| Benchmark suites | 5 |
| CI workflows | 13 |
| Releases (tagged) | 11 |

---

## Timeline

```
2022-07  First commit: tree-sitter grammar (JavaScript/C)
2023-01  Paul "LeoNerd" Evans joins; external scanner, string parsing
2023-02  Continued grammar expansion (heredocs, regex, OOP)
2023-03  through
2025-07  Steady-state tree-sitter development (community PRs)
         -------------------------------------------------------
2025-07-15  Steven Zimmerman forks; begins rapid development
2025-07-16  Pest (PEG) parser introduced
2025-07-20  v0.1.0-pest tagged
2025-07-21  Native Rust parser (perl-parser, perl-lexer) created
2025-07-24  First LSP support code
2025-08-03  VSCode extension; v0.5.0
2025-08-04  LSP v2 with modular features
2025-08-11  v0.8.0 (breaking: production-hardened position helpers)
2025-08-22  Pest parser designated legacy; v3 preference finalized
2025-08-23  v0.8.3 (LSP capabilities locked to GA contract)
2025-08-23  v0.8.5 (typed capabilities, pull diagnostics)
2025-08-27  perl-lsp split into dedicated crate
2025-10-04  perl-dap (Debug Adapter Protocol) introduced
         -------------------------------------------------------
2026-01-16  SRP microcrate extraction begins (37 crates in January)
2026-01-25  No-unwrap/no-panic coding standard enforced
2026-01-26  First microcrate batch (perl-error, perl-tokenizer, etc.)
2026-01-28  Supply chain security (SBOM, SLSA provenance)
2026-02-16  CI expansion: nightly, security, Docker, release orchestration
2026-02-20  v0.9.1 (public alpha alignment)
2026-02-22  Feature governance extracted into 9 microcrates
2026-02-28  v0.10.0 (Initial Public Alpha)
2026-03-04  Massive test campaign: ~60 crates get comprehensive tests
2026-03-05  SRP explosion: 37 more crates extracted in one week
2026-03-11  121 crates; ongoing SRP extractions
```

---

## Phase-by-Phase Evolution

### Phase 1: Tree-sitter Grammar (July 2022 -- July 2025)

**Origin.** The project started as `tree-sitter-perl-better`, a JavaScript-based
tree-sitter grammar with a C external scanner. The very first commit
(2022-07-17) by Veesh Goldman created `grammar.js` with rules for statements,
declarations, subroutines, and phasers.

**Key contributors.**
- **Veesh Goldman** (182 commits): Original author. Grammar foundations,
  numeric parsing, math operators, CI setup.
- **Paul "LeoNerd" Evans** (162 commits across two email aliases): External
  scanner in C for string literals, `qq()` strings, `qw()` lists, heredocs,
  regex. Followed `perly.y` closely. Major grammar expansion.
- Community contributors: Amaan Qureshi, Harald Jorg, Lorenzo Cian, Michael
  Budde, Olaf Alders, Trevor Gross, and others.

**What it covered.** By mid-2025 the tree-sitter grammar handled basic Perl
syntax: variables, operators, control flow, subroutines, packages, regex,
string interpolation, heredocs, and some OOP constructs. The grammar had
~150+ PRs merged over 3 years.

**Limitations.** Tree-sitter grammars are context-free, but Perl parsing is
notoriously context-sensitive (the well-known "only Perl can parse Perl"
problem). The C external scanner grew complex trying to handle string
delimiters, heredocs, and regex vs. division disambiguation. Coverage
plateaued around ~85%.

### Phase 2: Pest Parser (July 16--20, 2025)

**Duration: 4 days.** The Pest (PEG) parser was introduced on 2025-07-16 and
tagged as `v0.1.0-pest` on 2025-07-20.

**What drove it.** Steven Zimmerman joined the project with the goal of building
a full Perl language server. Tree-sitter's context-free grammar could not
adequately handle Perl's context sensitivity. A PEG (Parsing Expression
Grammar) approach via the Pest crate offered:
- Pure Rust (no C dependency)
- Declarative grammar files (`.pest`)
- Rich error messages
- Ordered choice for disambiguation

**What it achieved.** In 4 days, the Pest parser reached coverage of basic Perl
constructs including list literals, command options, and enhanced AST
structure. It handled ~99.995% of edge cases according to the project's own
analysis.

**Why it was superseded.** Despite excellent correctness, PEG backtracking
introduced performance overhead (~200-450 microseconds parse time). The
grammar file approach also limited control over incremental parsing and
error recovery strategies needed for IDE use.

The Pest parser remains in the codebase as `perl-parser-pest`, designated as
a legacy crate. It is maintained but excluded from the default CI gate.

### Phase 3: Native Rust Recursive Descent (July 21, 2025 onward)

**The turning point.** On 2025-07-21, the commit "Implement a modern two-crate
architecture for Perl parsing" created `perl-parser` and `perl-lexer` as the
foundation for the v3 parser: a hand-written recursive-descent parser in pure
Rust.

**Architecture.** The two-crate split (lexer + parser) was deliberate:
- `perl-lexer`: Context-aware tokenizer. Handles the context sensitivity that
  makes Perl hard to parse (regex vs. division, heredoc boundaries, quote-like
  operators with arbitrary delimiters).
- `perl-parser`: Recursive descent parser that consumes the token stream and
  builds a concrete syntax tree.

**Performance.** The native parser achieved 1-150 microseconds parse time with
931 nanosecond incremental updates -- a significant improvement over both the
Pest parser (~200-450 microseconds) and the C tree-sitter parser (~12-68
microseconds for simple cases, but without full coverage).

**Key milestones within this phase:**
- 2025-07-25: Error recovery and incremental parsing infrastructure
- 2025-07-26: First LSP support integration
- 2025-08-03: v0.5.0 with examples for dereferencing, formatting, statement
  modifiers
- 2025-08-11: v0.8.0 with production-hardened position helpers (breaking API)
- 2025-08-22: Official designation as the preferred parser, with Pest marked
  legacy

### Phase 4: LSP Server (July 24, 2025 onward)

**First LSP code.** On 2025-07-24, the commit "Implement incremental parsing,
LSP support, and language bindings for Perl" introduced the initial LSP
infrastructure.

**Rapid feature development.** Within 10 days (July 24 -- August 4, 2025):
- Basic LSP server with JSON-RPC transport
- Code formatting support
- VSCode extension (2025-08-03)
- LSP v2 with modular feature support (2025-08-04)
- Advanced features: Call Hierarchy, Inlay Hints, Test Runner (2025-08-04)
- Code Lens provider (2025-08-04)

**Dedicated crate.** On 2025-08-27, `perl-lsp` was split into its own crate,
separating the server binary from the parser library.

**Feature governance.** The project introduced `features.toml` as the canonical
single source of truth for all LSP capabilities. Each feature tracks:
- LSP specification version
- Maturity level (planned, preview, ga, production)
- Whether it is advertised to clients
- Associated test files

By March 2026, all **97 features** in the catalog reached GA maturity,
achieving 100% LSP 3.18 protocol compliance. The 97 features span:
- 41 text document features (completion, hover, diagnostics, etc.)
- 26 workspace features (symbols, configuration, etc.)
- 10 debug features
- 9 window features
- 9 protocol features
- 2 notebook features

### Phase 5: Debug Adapter Protocol (October 2025)

**Introduction.** The `perl-dap` crate was created on 2025-10-04 with the
commit "test: add comprehensive DAP test scaffolding for Issue #207."

**Architecture.** The DAP implementation uses a dual-mode approach:
- **Native adapter**: Built from scratch in Rust with AST-validated breakpoints,
  step/pause/continue handlers, safe-eval guards, and stdio+socket transport.
- **Bridge adapter**: Maintains compatibility with Perl::LanguageServer for
  interoperability.

**Microcrate decomposition.** The DAP subsystem was later decomposed into 9
focused crates: `perl-dap-breakpoint`, `perl-dap-command-args`, `perl-dap-eval`,
`perl-dap-platform`, `perl-dap-security`, `perl-dap-shell`, `perl-dap-stack`,
`perl-dap-value`, `perl-dap-variables`.

### Phase 6: SRP Microcrate Architecture (January 2026 onward)

**The modularization wave.** Starting 2026-01-16, the codebase underwent
aggressive decomposition following the Single Responsibility Principle. The
growth trajectory:

| Month | Crates created | Running total |
|-------|---------------|---------------|
| 2025-07 | 5 | 5 |
| 2025-08 | 3 | 8 |
| 2025-10 | 1 | 9 |
| 2026-01 | 37 | 46 |
| 2026-02 | 38 | 84 |
| 2026-03 | 37 | 121 |

**What drove it.** The project document `AGENTIC_DEV.md` describes the AI-native
development model: high-throughput changes are verified by mechanical gates
rather than manual inspection. Small, focused crates with clear boundaries
make it possible for AI agents and automated tools to:
- Understand scope quickly (each crate has a single responsibility)
- Run targeted tests (no need to rebuild the world)
- Enforce contracts at crate boundaries
- Publish independently on crates.io

**Crate families (March 2026):**

| Family | Count | Purpose |
|--------|-------|---------|
| `perl-lsp-*` | 41 | LSP feature providers and infrastructure |
| `perl-module-*` | 13 | Module resolution, imports, naming |
| `perl-dap-*` | 9 | Debug adapter components |
| `perl-workspace-*` | 6 | Workspace discovery, indexing, SLOs |
| `perl-ts-*` | 5 | Tree-sitter integration |
| `tree-sitter-*` | 3 | Tree-sitter grammars (JS, C, Rust) |
| Core leaf crates | 44 | Tokens, AST, quote, regex, heredoc, etc. |

**Tiered dependency structure.** Crates are organized in 7 tiers from leaf
crates with zero internal dependencies (Tier 1) up to application binaries
(Tier 6) and legacy crates (Tier 7). This enforces a DAG and prevents
circular dependencies.

---

## Key Technical Decisions

### 1. Tree-sitter to Pest: Escaping C

**Decision:** Replace the JavaScript/C tree-sitter grammar with a Pest PEG
grammar in pure Rust.

**Rationale:**
- Tree-sitter's C external scanner was becoming hard to maintain
- The context-free grammar could not handle Perl's context sensitivity
- C code introduced memory safety concerns
- Pest offered a declarative, pure-Rust alternative

**Outcome:** Achieved in 4 days (2025-07-16 to 2025-07-20). Demonstrated that
a Rust-native approach was viable for Perl parsing.

### 2. Pest to Recursive Descent: Performance for IDE Use

**Decision:** Replace the Pest PEG parser with a hand-written recursive-descent
parser.

**Rationale:**
- PEG backtracking caused ~200-450 microsecond parse times
- IDE use requires sub-millisecond incremental parsing
- Hand-written parsers offer full control over error recovery
- Incremental parsing is easier to implement with explicit state management

**Outcome:** Parse times dropped to 1-150 microseconds with 931-nanosecond
incremental updates. The parser achieved ~100% Perl 5 syntax coverage.

### 3. Dual Indexing (PR #122 pattern)

**Decision:** Index workspace symbols under both qualified and bare forms.

**Rationale:** Users navigate code using both `Foo::Bar::method` (qualified)
and `method` (bare). Indexing under both forms provides instant lookups
regardless of how the user references a symbol.

**Implementation:**
```rust
// Index under bare name
file_index.references.entry(bare_name.to_string())
    .or_default().push(symbol_ref.clone());
// Index under qualified name
file_index.references.entry(qualified)
    .or_default().push(symbol_ref);
```

### 4. Feature Governance Subsystem

**Decision:** Implement a formal feature governance system with 9 dedicated
microcrates.

**Rationale:** With 97 LSP features, the project needed:
- Compile-time feature IDs (`perl-lsp-feature-ids`)
- Feature flag evaluation (`perl-lsp-feature-flags`)
- Policy enforcement (`perl-lsp-feature-policy`)
- Runtime profiles (`perl-lsp-feature-profile`)
- Governance contracts (`perl-lsp-feature-governance`)

**Outcome:** Features can be individually toggled, profiled, and governed. The
CLI exposes feature profiles via `perl-lsp-feature-profile-cli`.

### 5. No-unwrap / No-panic Coding Standard

**Decision:** Ban `unwrap()`, `expect()`, `panic!()`, `todo!()`,
`unimplemented!()`, `dbg!()`, and `std::process::abort()` from production
code.

**Introduced:** January 25, 2026, with a series of commits eliminating
hundreds of `unwrap`/`expect` calls across the codebase.

**Rationale:** A language server must never crash. Every code path must degrade
gracefully. The project uses:
- `?` operator and `Result`/`Option` propagation
- `.ok_or_else()` for explicit error conversion
- `Option<Regex>` with `.ok()` for graceful regex compilation failure
- `perl-tdd-support::must`/`must_some` helpers in tests
- Safety ratchets in CI: baseline counts of `unwrap=0`, `panic!=0`, `unsafe=0`

**Exception:** One centralized `#[allow(clippy::expect_used)]` for
`lsp_types::Uri` fallback in `crates/perl-lsp-rs/src/util/uri.rs`.

### 6. SRP Microcrate Architecture

**Decision:** Decompose the codebase into 121+ single-responsibility crates.

**Rationale:**
- AI-native development requires small, well-bounded units of code
- Compile-time enforcement of module boundaries
- Independent crates.io publishing
- Targeted testing (run only affected crate tests)
- Reduced rebuild times during development

**Trade-offs acknowledged:** More `Cargo.toml` files to manage, more complex
dependency graph, potential for over-decomposition. Mitigated by the tiered
dependency structure and automated tooling.

### 7. Supply Chain Security

**Decision:** Implement comprehensive supply chain security measures.

**Introduced:** 2026-01-28 with the PR "feat(security): Add Supply Chain
Security - SBOM & SLSA Provenance (#281)."

**Components:**
- **SBOM generation** in both SPDX and CycloneDX formats
- **cargo-deny** (`deny.toml`) for license, advisory, and source auditing
- **SLSA provenance** attestations on release artifacts
- **Dependabot** for automated dependency updates (48 bot commits)
- **Security scanning** CI workflow (`ci-security.yml`)
- **Path traversal and injection hardening** across LSP and DAP servers

---

## Metrics Trajectory

### Commit Velocity

| Period | Commits | Avg/month |
|--------|---------|-----------|
| 2022 (6 months) | 10 | 1.7 |
| 2023 (12 months) | 284 | 23.7 |
| 2024 (12 months) | 60 | 5.0 |
| 2025 (12 months) | 3,352 | 279.3 |
| 2026 (3 months, partial) | 1,706 | 568.7 |

The 50x increase in commit velocity from 2024 to 2025 marks Steven
Zimmerman's entry and the shift to an AI-native development model.

### Crate Count Growth

| Date | Crates |
|------|--------|
| 2025-07 | 5 |
| 2025-08 | 8 |
| 2025-10 | 9 |
| 2026-01 | 46 |
| 2026-02 | 84 |
| 2026-03 | 121 |

### LSP Feature Coverage

| Version | Date | LSP Coverage |
|---------|------|-------------|
| v0.8.3 | 2025-08-23 | GA contract locked |
| v0.8.5 | 2025-08-23 | Typed capabilities, pull diagnostics |
| v0.10.0 | 2026-02-28 | 97/97 features at GA (100%) |

### Test Count

| Date | Approximate tests |
|------|-------------------|
| 2025-08 (v0.8.x) | ~hundreds (pre-microcrate) |
| 2026-02 (v0.10.0) | 1,543 lib tests (Tier A) |
| 2026-03 (current) | 4,400+ across workspace |

The March 2026 test campaign added comprehensive unit tests to ~60 crates
in a single week.

### Parser Performance

| Parser | Parse Time | Incremental |
|--------|------------|-------------|
| Tree-sitter (C) | ~12-68 us | N/A |
| Pest (v2) | ~200-450 us | N/A |
| Native (v3) | ~1-150 us | ~931 ns |

### Code Size

| Component | Lines (code only) |
|-----------|-------------------|
| Rust (crates/) | 364,600 |
| Total Rust | 374,179 |
| Perl (test corpus) | 31,822 |
| Total (all languages) | 476,360 |

---

## Contributor Analysis

### Human Contributors

| Contributor | Commits | Period | Focus |
|-------------|---------|--------|-------|
| Steven Zimmerman | ~4,770* | 2025-07 -- present | Parser v2/v3, LSP, DAP, architecture |
| Veesh Goldman | 182 | 2022-07 -- 2024-11 | Tree-sitter grammar, original author |
| Paul "LeoNerd" Evans | 162 | 2023-01 -- 2024-10 | External scanner, grammar coverage |
| Amaan Qureshi | 4 | 2023 | Grammar contributions |
| 9 other contributors | 2 each | Various | Community PRs |

*Steven Zimmerman committed under three aliases: "Steven Zimmerman" (3,955),
"Steven Zimmerman, CPA" (633), and "EffortlessSteven" (184).

### Bot Contributors

| Bot | Commits | Purpose |
|-----|---------|---------|
| google-labs-jules[bot] | 231 | Security fixes, performance optimization, VSCode UX |
| dependabot[bot] | 48 | Automated dependency updates |
| copilot-swe-agent[bot] | 1 | Code contribution |

### Development Model

The project is explicitly **AI-native** (documented in `AGENTIC_DEV.md`):
- Human role: reviews and accepts/rejects
- Quality gate: mechanical checks (CI, ratchets, mutation testing)
- Claims: receipt-based, not trust-based
- High throughput enabled by small crates and automated gates

Google's Jules agent contributed security hardening (argument injection
fixes, path traversal fixes, safe evaluation guards) and VSCode UX
improvements. Dependabot maintains dependency freshness.

---

## Commit Message Conventions

The project uses **Conventional Commits** with scope notation:

| Prefix | Count | Examples |
|--------|-------|---------|
| `feat:` | 942 | New features |
| `fix:` | 517 | Bug fixes |
| `docs:` | 285 | Documentation |
| `refactor:` | 268 | Code restructuring |
| `chore:` | 246 | Maintenance tasks |
| `ci:` | 91 | CI/CD changes |
| `test:` | 74 | Test additions |
| `style:` | 66 | Formatting |
| `perf:` | 44 | Performance improvements |
| `release:` | 11 | Version releases |

Common scopes: `(parser)`, `(lsp)`, `(dap)`, `(vscode)`, `(security)`,
`(ci)`, `(tests)`, `(docs)`, `(deps)`.

---

## Release History

| Version | Date | Highlights |
|---------|------|-----------|
| v0.1.0-pest | 2025-07-20 | First Pest-based parser release |
| v0.5.0 | 2025-08-03 | Native parser examples, version bump |
| v0.7.2 | 2025-08-06 | Operator precedence, division parsing fixes |
| v0.7.3 | 2025-08-06 | Return/die statements with modifiers |
| v0.8.0 | 2025-08-11 | Production-hardened position helpers (breaking API change) |
| v0.8.2 | 2025-08-12 | Comprehensive clippy cleanup |
| v0.8.3-rc1 | 2025-08-15 | Release candidate (ARM64 Linux issues) |
| v0.8.3 | 2025-08-23 | LSP capabilities locked to GA contract |
| v0.8.5 | 2025-08-23 | Typed capabilities, pull diagnostics, stable codes |
| v0.9.1 | 2026-02-20 | Public alpha alignment |
| v0.10.0 | 2026-02-28 | **Initial Public Alpha** -- current release |

**Distribution channels (v0.10.0):**
- GitHub Releases (prebuilt binaries)
- crates.io (library crates)
- VS Code Marketplace (extension)
- Homebrew formula
- Docker images
- Scoop/Chocolatey (Windows)

---

## Interesting Facts and Statistics

1. **4 days from Pest to Legacy.** The Pest parser was introduced, reached
   v0.1.0, and was already being superseded within 4 days (July 16--20, 2025).

2. **Tree-sitter heritage preserved.** The original tree-sitter grammar
   remains in the repository as `tree-sitter-perl/` with its test corpus of
   32 corpus files used for regression testing. The test corpus has grown to
   73 `.pl` files.

3. **Paul "LeoNerd" Evans connection.** Paul Evans, who wrote the C external
   scanner and major grammar portions, is the author of numerous CPAN modules
   and a Perl core contributor. His `perly.y`-following approach grounded the
   grammar in Perl's actual parser.

4. **The project name evolved.** `tree-sitter-perl-better` (2022) became the
   foundation for what is now `perl-lsp` (2025), reflecting the shift from
   a grammar project to a full language server.

5. **121 crates, one workspace.** The workspace has more crates than some
   organizations have repositories. Each crate averages ~3,000 lines of Rust
   code.

6. **Zero unsafe, zero panics.** The production code maintains baseline
   counts of `unwrap/expect=0`, `panic!/todo!/unimplemented!/unreachable!=0`,
   and `unsafe=0`, enforced by CI ratchets.

7. **Sub-microsecond incremental parsing.** At 931 nanoseconds for incremental
   updates, the parser can re-parse between every keystroke without any
   perceptible delay.

8. **AI-native by design.** With ~231 commits from Google Jules and the
   explicit AI-native development model documentation, this is one of the
   most openly AI-assisted Rust projects in the ecosystem.

9. **97 LSP features at GA.** The project implements every feature in the
   LSP 3.18 specification that it advertises, from basic completion and
   hover to notebook support, pull diagnostics, and type hierarchy.

10. **Rust Edition 2024.** The project uses the newest Rust edition (2024) with
    MSRV 1.92, adopting the latest language features.

11. **The commit velocity chart is a hockey stick.** From ~5 commits/month in
    2024 to ~569 commits/month in early 2026 -- a 100x increase.

12. **13 CI workflows.** From basic CI (`ci.yml`, 2023) to a full release
    orchestration pipeline including nightly builds, security scanning,
    Docker publishing, crate publishing, extension publishing, and package
    manager bumps across Homebrew, Scoop, and Chocolatey.

---

## Appendix: Crate Creation Timeline

Full list of 121 crates ordered by creation date:

| Date | Crate |
|------|-------|
| 2025-07-16 | tree-sitter-perl |
| 2025-07-16 | tree-sitter-perl-rs |
| 2025-07-16 | tree-sitter-perl-c |
| 2025-07-21 | perl-lexer |
| 2025-07-21 | perl-parser |
| 2025-08-21 | perl-corpus |
| 2025-08-22 | perl-parser-pest |
| 2025-08-27 | perl-lsp |
| 2025-10-04 | perl-dap |
| 2026-01-16 | perl-incremental-parsing, perl-lsp-providers, perl-parser-core, perl-position-tracking, perl-refactoring, perl-semantic-analyzer, perl-tdd-support, perl-workspace-index |
| 2026-01-16 | perl-lsp-protocol, perl-lsp-transport |
| 2026-01-21 | perl-symbol-types, perl-diagnostics-codes, perl-symbol-table, perl-uri |
| 2026-01-26 | perl-ast, perl-builtins, perl-edit, perl-pragma, perl-quote, perl-regex, perl-token, perl-heredoc, perl-error, perl-tokenizer |
| 2026-01-27 | perl-lsp-tooling, perl-lsp-formatting, perl-lsp-diagnostics |
| 2026-01-28 | perl-lsp-code-actions, perl-lsp-completion, perl-lsp-inlay-hints, perl-lsp-navigation, perl-lsp-semantic-tokens, perl-lsp-rename |
| 2026-02 | perl-module-*, perl-workspace-*, perl-dap-*, perl-lsp-feature-* (38 crates) |
| 2026-03 | perl-lsp-workspace-symbols, perl-lsp-folding, perl-lsp-completion-item, perl-ci-hygiene, perl-workspace-index-state-machine, and 32 more (37 crates) |
