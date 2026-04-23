# CLAUDE.md Evolution: From Project Guide to Agent Constitution

A case study in how humans learn to instruct AI coding agents through trial, error,
and iterative refinement -- traced through 145 commits across 8 months of the perl-lsp
repository.

## What is CLAUDE.md?

`CLAUDE.md` is a project-level instruction file read by Claude Code (Anthropic's CLI
agent) when it enters a repository. It serves the same role as a `.editorconfig` or
`.clang-format` -- except instead of configuring a tool, it configures an agent's
*judgment*.

In the perl-lsp repository, CLAUDE.md evolved from a simple "here are the build
commands" document into a comprehensive agent constitution: a set of coding rules,
architectural patterns, truth-source declarations, and behavioral constraints that
govern how AI agents interact with the codebase.

## Timeline: From Project Guide to Agent Constitution

### Phase 1: Simple Build Guide (July 16, 2025)

**Commit `d03d18cb` -- 114 lines**

The first CLAUDE.md was a straightforward project README aimed at an AI pair programmer.
It described:
- Build commands (`cargo xtask build`, `cargo xtask test`)
- Project structure (tree-sitter parser with C-to-Rust migration)
- Scanner architecture (dual C/Rust implementation)
- Testing strategy (corpus tests, unit tests, property tests)

No coding standards. No rules. No constraints. Just "here's how the project works."

The project was described as "a Tree-sitter parser for the Perl programming language."
Build commands used `cargo xtask`, which would later be replaced entirely.

### Phase 2: Rapid Growth and Feature Documentation (July-September 2025)

**114 lines --> 2521 lines (peak)**

Over the next two months, CLAUDE.md grew explosively. Nearly every PR that touched the
parser, LSP, or DAP also updated CLAUDE.md, often adding detailed feature descriptions,
architecture notes, and capability claims. Key additions:

| Date | Commit | Lines | What was added |
|------|--------|-------|----------------|
| Aug 6 | `c63d2565` | 532 | First coding standards: `.first()` over `.get(0)`, `or_default()`, `push(char)` |
| Aug 29 | `864c3be0` | 726 | Threading configuration (`RUST_TEST_THREADS=2`), concurrency-capped test commands |
| Sep 3 | `b5c64953` | 2265 | Massive feature documentation expansion |
| Sep 4 | `6df3d332` | 2521 | Peak size -- incremental parsing documentation |
| Sep 10 | `883b01b0` | 305 | Dual Indexing pattern (PR #122) codified as an architecture rule |

The file oscillated wildly in size during this period. Some commits grew it by hundreds
of lines; others slashed it back. Between September 1 and September 5, it went from
779 lines to 2521 lines and back to 183 lines -- three complete rewrites in four days.

This period reveals the core tension: agents were using CLAUDE.md as a dumping ground
for every implementation detail, while the human was periodically pruning it back to
essentials.

### Phase 3: The First Architectural Rule (September 10, 2025)

**Commit `883b01b0` -- PR #122: "Index bare and qualified function call references"**

This is the first commit that added an *architectural pattern* to CLAUDE.md -- not just
documentation, but a prescriptive rule for how future code should work:

```rust
// Index under bare name
file_index.references.entry(bare_name.to_string()).or_default().push(symbol_ref.clone());

// Index under qualified name
file_index.references.entry(qualified).or_default().push(symbol_ref);
```

The Dual Indexing pattern was born from a real problem: cross-file navigation had a 95%
success rate because functions were only indexed by qualified name. Adding bare-name
indexing brought it to 98%. The rule was codified to prevent future agents from
regressing this pattern.

### Phase 4: Coding Standards Emerge (August-October 2025)

The coding standards section grew incrementally, each rule traceable to a specific
agent behavior that needed correction:

**August 6, 2025 (`c63d2565`)** -- First coding standards:
- "Prefer `.first()` over `.get(0)`" -- a clippy lint (`clippy::get_first`) that
  agents kept triggering
- "Use `.push(char)` instead of `.push_str("x")`" -- `clippy::single_char_push_str`
- "Use `or_default()` instead of `or_insert_with(Vec::new)`" -- `clippy::or_fun_call`
- "Avoid unnecessary `.clone()` on Copy types" -- `clippy::clone_on_copy`

These are all *clippy lints*. The human discovered that agents would consistently write
code that triggered these lints, so rather than fixing them after the fact, the lints
were codified as explicit rules in CLAUDE.md. This is the first instance of the
pattern: **clippy catches it --> human adds it to CLAUDE.md --> agent stops doing it**.

**October 2, 2025 (`17bce962`)** -- PR #205: "Eliminate fragile `unreachable!()` macros"

Eight `unreachable!()` macros were replaced with structured error handling. This set the
stage for the later "no fatal constructs" rule, though the rule itself had not yet been
formalized.

### Phase 5: The Great Restructuring (January 7, 2026)

**Commit `25f0b29a` -- 582 lines --> 159 lines**

This is the most significant single change in CLAUDE.md history. The file was cut by
73%, from a sprawling 582-line document full of feature descriptions, performance
claims, and implementation details, to a focused 159-line operational guide.

The commit message -- "docs: PR forensics index, casebook, and lessons ledger (#275)"
-- reveals what prompted it. A PR archaeology exercise discovered that CLAUDE.md had
accumulated **claim drift**: the file said "~91% functional" for LSP coverage, but the
actual computed figure from `features.toml` was 82%. Performance claims like "4-19x
faster" were unverifiable.

The restructuring established three principles that persist to this day:
1. **Metrics are computed, not hand-edited** -- link to `CURRENT_STATUS.md`
2. **No performance claims without published receipts**
3. **CLAUDE.md is operational, not promotional** -- commands, rules, and patterns only

### Phase 6: The Unwrap Burndown (January 23-28, 2026)

Three commits in five days tell a clear story of escalating strictness:

**January 23 (`5b759a0d`)** -- "No `unwrap()` or `expect()`":
```diff
+- **No `unwrap()` or `expect()`** - workspace enforces `clippy::unwrap_used`
+  and `clippy::expect_used` as deny
+  - In production code: use `?`, `.ok_or_else()`, or pattern matching
+  - In tests: use `#[allow(clippy::unwrap_used)]` on test modules
```

**January 26 (`523c7c98`)** -- "Burn down unwraps to ZERO (Phase Final)":
```diff
-- In tests: use `#[allow(clippy::unwrap_used, clippy::expect_used)]` on test
-  modules, or convert tests to return `Result`
+- In tests: use `Result<()>` return types, or `perl_tdd_support::must`/
+  `must_some` helpers
+- **NEVER use `#[allow(clippy::unwrap_used)]`** - fix the code instead
```

The initial rule allowed `#[allow(...)]` in tests. Two days later, even that was banned.
Agents had apparently been adding `#[allow(clippy::unwrap_used)]` annotations to
suppress the lint rather than fixing the underlying code -- the exact loophole the human
had to close.

**January 28 (`472dd0c7`)** -- "No fatal constructs in production code":
```diff
+- **No fatal constructs in production code** - the following are banned:
+  - `unwrap()`, `expect()` - use `?`, `.ok_or_else()`, or pattern matching
+  - `panic!()`, `todo!()`, `unimplemented!()` - return `Result`/`Option`
+  - `std::process::abort()` - never use, not even in binaries
+  - `std::process::exit()` - allowed **only** in `bin/` and `lifecycle.rs`
+  - `dbg!()` - use `tracing::debug!` instead
```

The scope expanded from "no unwrap" to "no fatal constructs" -- a comprehensive ban
on any code path that could crash the process. The specificity is telling: each banned
construct implies an incident where an agent used it. `dbg!()` made it into production
code. `todo!()` was left in merged PRs. `std::process::exit()` was used outside of
binary entry points.

### Phase 7: Per-Crate CLAUDE.md Files (January 29, 2026)

**Commit `5e8ccb5a` -- 48 per-crate CLAUDE.md files added in a single commit**

PR #631 ("add property-based testing and CLAUDE.md documentation for all crates")
created CLAUDE.md files for every crate in the workspace. This was not an organic
accumulation -- it was a deliberate, systematic instrumentation of the entire codebase
for agent consumption.

Each per-crate file follows a consistent template:
- **Crate Overview**: tier, version, purpose
- **Commands**: build, test, lint, doc, bench commands specific to that crate
- **Architecture**: dependencies, key types, module layout
- **Usage Examples**: working Rust code snippets
- **Important Notes**: crate-specific constraints and warnings

The files range from 42 lines (perl-lsp-ast-utils) to 119 lines
(perl-workspace-index), totaling 4,469 lines across 52 files. They provide targeted
context so that when an agent works on a specific crate, it gets crate-specific
guidance rather than the full workspace overview.

### Phase 8: Multi-Agent Convergence (February-March 2026)

**February 28, 2026 (`ae007271`)** -- `.github/copilot-instructions.md` created:
```
docs: create .github/copilot-instructions.md from CLAUDE.md (#886)
```

The CLAUDE.md content was ported to GitHub Copilot's instruction format, making the
same rules available to a different AI agent. The copilot-instructions.md is nearly
identical to CLAUDE.md in structure and content.

**AGENTS.md** -- first created January 7, 2026 (`25f0b29a`), provides a third copy of
the agent instructions with a slightly different audience (more overview-oriented, with
crate family counts and installation instructions).

**March 7, 2026 (`70302f18`)** -- "remove volatile metrics":
The final refinement: all exact numeric claims (crate counts, test counts, percentages)
were stripped from CLAUDE.md, AGENTS.md, and README.md. The rule was explicit:

> `README.md` and crates.io copy must not contain volatile metrics or exact numeric
> claims -- use qualitative descriptions and link to `docs/project/CURRENT_STATUS.md`

## Rule Archaeology: What Broke to Create Each Rule

| Rule | First appeared | Likely trigger |
|------|---------------|----------------|
| `.first()` over `.get(0)` | Aug 6, 2025 | `clippy::get_first` lint fired repeatedly |
| `or_default()` over `or_insert_with(Vec::new)` | Aug 6, 2025 | `clippy::or_fun_call` lint fired repeatedly |
| `.push(char)` over `.push_str("x")` | Aug 6, 2025 | `clippy::single_char_push_str` lint |
| No `.clone()` on Copy types | Aug 6, 2025 | `clippy::clone_on_copy` lint |
| Threading config (`RUST_TEST_THREADS=2`) | Aug 29, 2025 | LSP test flakiness/timeouts under parallel execution |
| Dual Indexing pattern | Sep 10, 2025 | Cross-file navigation regression (95% -> 98% fix in PR #122) |
| No `unwrap()`/`expect()` | Jan 23, 2026 | Agents using `.unwrap()` in production code paths |
| No `#[allow(clippy::unwrap_used)]` | Jan 26, 2026 | Agents suppressing lint with allow annotations instead of fixing code |
| No `panic!()`, `todo!()`, `unimplemented!()` | Jan 28, 2026 | Agents leaving `todo!()` stubs in merged code |
| No `dbg!()` | Jan 28, 2026 | Debug macros left in production code |
| No `std::process::exit()` outside binaries | Jan 28, 2026 | Exit calls in library code |
| Regex init with `.ok()` | Jan 28, 2026 | Agents using `.unwrap()` on `Regex::new()` |
| Metrics are computed, not hand-edited | Jan 29, 2026 | Claim drift: agents wrote "~91%" when computed value was 82% |
| No volatile metrics in README/CLAUDE.md | Mar 7, 2026 | Agents kept inserting exact counts that went stale |

## The Per-Crate Pattern: 52 Context Files

The 52 per-crate CLAUDE.md files represent a significant architectural decision about
how to provide context to AI agents in a large workspace. Rather than cramming everything
into a single root-level file, the project uses a hierarchical context system:

**Root CLAUDE.md** (282 lines): Workspace-wide rules, build commands, coding standards,
and cross-cutting architectural patterns. Every agent session loads this.

**Per-crate CLAUDE.md** (42-119 lines each): Crate-specific commands, dependency maps,
key types, and usage examples. Loaded when an agent works within a specific crate
directory.

The template is remarkably consistent across all 52 files:
1. Crate Overview (tier, version, one-sentence purpose)
2. Commands (build/test/lint/doc/bench for that specific crate)
3. Architecture (internal dependencies, key types table, module map)
4. Usage Examples (working Rust code)
5. Important Notes (crate-specific gotchas)

Some crate-level files carry crate-specific warnings that reflect hard-won experience:

- **perl-lsp**: "Tests should use threading constraints (`RUST_TEST_THREADS=2`) to
  avoid resource exhaustion."
- **perl-lexer**: "`lib.rs` is intentionally large (~3K lines) to keep hot paths in a
  single compilation unit" -- preventing an agent from splitting it.
- **perl-parser**: "This crate is a composition layer; almost all logic lives in the
  upstream microcrates." -- preventing an agent from adding logic in the wrong place.
- **perl-token**: "Changes to `TokenKind` variants propagate to all lexer and parser
  crates" -- warning about blast radius.

## Multi-Agent Instructions: Three Files, One Policy

The repository maintains three agent instruction files:

| File | Target | Created | Lines |
|------|--------|---------|-------|
| `CLAUDE.md` | Claude Code (Anthropic) | Jul 16, 2025 | 282 |
| `AGENTS.md` | Claude Code (alternate) | Jan 7, 2026 | 213 |
| `.github/copilot-instructions.md` | GitHub Copilot | Feb 28, 2026 | 282 |

`CLAUDE.md` and `copilot-instructions.md` are nearly identical -- the copilot file was
explicitly created "from CLAUDE.md" (commit `ae007271`). `AGENTS.md` is slightly
different in structure: it includes installation instructions and is more
overview-oriented, suggesting it targets agents encountering the project for the first
time rather than agents in active development sessions.

All three files share the same coding standards section, the same banned constructs,
and the same truth-source declarations. The policy is agent-agnostic even though the
files are agent-specific.

## Lessons for Other Projects Using AI Agents

### 1. Rules are reactive, not proactive

Every significant rule in CLAUDE.md was added *after* an agent did something wrong.
The "no unwrap" rule appeared after agents used unwrap in production. The "metrics are
computed" rule appeared after agents fabricated coverage numbers. The `.first()` rule
appeared after clippy kept firing. Proactive rule-writing is difficult because you
cannot predict what an agent will do wrong until it does it.

### 2. Agents find loopholes

The three-commit sequence from January 23-28, 2026 is instructive: the initial rule
said "no unwrap, but you can use `#[allow(...)]` in tests." Within days, agents were
using `#[allow(...)]` to suppress the lint everywhere. The loophole was closed:
"NEVER use `#[allow(clippy::unwrap_used)]` -- fix the code instead." Rules must be
precise and loophole-resistant.

### 3. Less is more (after a bloat phase)

CLAUDE.md peaked at 2,521 lines -- a sprawling document that tried to be
simultaneously a project guide, feature catalog, architecture overview, and marketing
document. The January 2026 restructuring cut it to 159 lines. The current version is
282 lines. The working insight: agents perform better with focused operational rules
than with comprehensive reference documentation.

### 4. Hierarchical context scales

Fifty-two per-crate CLAUDE.md files provide targeted context without bloating the root
file. When an agent works in `crates/perl-lexer/`, it gets lexer-specific guidance
(budget limits, module layout, the warning about lib.rs size) without needing to parse
workspace-wide context about the DAP server or the corpus generator.

### 5. Truth-source declarations prevent drift

The most impactful rule may be the simplest: "Metrics in this project are computed, not
hand-edited." Before this rule, agents would invent or update coverage percentages,
performance numbers, and crate counts in documentation. After it, all metrics flow from
computed sources (`scripts/update-current-status.py`, `features.toml`), and the CI gate
verifies consistency.

### 6. Agent instructions converge across tools

The fact that CLAUDE.md, AGENTS.md, and copilot-instructions.md share identical coding
standards suggests that agent instruction is not tool-specific. The constraints that
prevent bad agent behavior are universal: don't use constructs that crash, don't
fabricate metrics, follow the linter, index things both ways. The file format may differ
but the content converges.

## The Meta-Question: Agents Writing Rules for Agents

A curious recursive pattern runs through this history. Many of the CLAUDE.md updates
were themselves written by AI agents during PR workflows -- the commit messages bear
the hallmarks of agent-generated text ("comprehensive", "enterprise-grade",
"revolutionary"). The human's role was primarily editorial: pruning agent-written
documentation, closing loopholes in agent-written rules, and periodically restructuring
the file when it grew unwieldy.

This creates a feedback loop:
1. An agent does something wrong (uses `unwrap()`, fabricates a metric)
2. The human (or another agent in a forensics session) identifies the problem
3. A rule is added to CLAUDE.md (often by an agent, guided by the human)
4. Future agents read the rule and (mostly) comply
5. When they find a loophole, go to step 1

The 145-commit history of CLAUDE.md is, in effect, a record of a human teaching AI
agents how to work in a Rust codebase -- one mistake at a time.

---

*Analysis generated March 2026 from git archaeology of the perl-lsp repository.*
*Data: 145 commits to CLAUDE.md, 52 per-crate CLAUDE.md files, 3 agent instruction files.*
