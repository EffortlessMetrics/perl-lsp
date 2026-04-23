# The perl-lsp Development Story

> How a single developer and a swarm of AI agents built a 130-crate Perl language
> server in nine months — and what the experience teaches about the future of
> software engineering.

> Historical snapshot: live metrics and current release posture belong in
> [CURRENT_STATUS.md](CURRENT_STATUS.md) and [ROADMAP.md](ROADMAP.md); the
> metrics in this story are intentionally frozen to the March 2026 launch
> window.

---

## Part 1: The Architecture Story

### Why 130 Microcrates?

The perl-lsp workspace contains 130 crate directories in a single Rust workspace.
Each crate follows the Single Responsibility Principle at the package level: one
concept, one small API, one test suite. The heuristic for what qualifies as an SRP
microcrate is enforced by tooling (`cargo xtask srp-microcrates`):

- 700 lines of code or fewer
- 3 or fewer Rust source files
- 8 or fewer direct dependencies

This was not the starting point. The project began in July 2022 with 2 crates
(tree-sitter bindings). By August 2025 it had grown to 8. Then, between February
and March 2026, an aggressive extraction campaign — driven almost entirely by AI
agents — decomposed the workspace:

| Date | Crate Count | What Happened |
|------|:-----------:|---------------|
| Jul 2022 | 2 | Tree-sitter bindings only |
| Aug 2025 | 8 | Parser, lexer, LSP, corpus, Pest archive |
| Feb 2026 | 53 | First SRP extraction wave |
| Late Feb 2026 | 85 | Second wave |
| Mar 2026 | 121 | Third wave |
| Mar 2026 snapshot | 130 | Continuing extraction + new features |

The most intense day was March 5, 2026: 93 extraction-related commits in a single
day.

**Why go this far?** Three concrete benefits:

1. **Parallel agent development.** When 50 agents are working simultaneously in
   isolated worktrees, each touching a different crate, there are zero merge
   conflicts. The crate boundary *is* the coordination boundary. An agent
   extracting `perl-lsp-folding` cannot interfere with an agent fixing
   `perl-parser-core`.

2. **Fast incremental compilation.** A change to `perl-lsp-folding` (314 lines)
   recompiles only that crate and its reverse dependencies. The parser, lexer,
   semantic analyzer, and 120 other crates are untouched.

3. **Independent SemVer contracts.** Each crate ships to crates.io with its own
   version. Downstream consumers can depend on `perl-module-name` without pulling
   in the entire LSP server.

The module resolution pipeline illustrates the decomposition philosophy. What could
be a single `perl-module-resolution` crate is instead 13 focused crates:

```
perl-module-token-core       Shared primitives for module tokens
perl-module-token            Module use/require token representation
perl-module-token-parser     Parser for module tokens from source text
perl-module-name             Validated Perl module name type
perl-module-boundary         Module boundary detection
perl-module-import           Import statement representation
perl-module-import-match     Import matching logic
perl-module-path             Module-to-filesystem path mapping
perl-module-reference        Cross-module reference tracking
perl-module-rename           Module rename refactoring
perl-module-resolution       Full module resolution pipeline
perl-module-resolution-path  Path-based resolution helpers
perl-module-resolution-uri   URI-based resolution helpers
```

A tool that only needs to parse `use` statements depends on `perl-module-token`
without pulling in filesystem resolution or the LSP. This is composition by
construction, not by convention.

### The Seven-Tier Dependency Structure

The workspace enforces a strict dependency hierarchy, annotated directly in the
root `Cargo.toml`:

| Tier | Purpose | Examples |
|------|---------|---------|
| 1 | Leaf crates, zero internal deps | `perl-token`, `perl-ast`, `perl-quote` |
| 2 | Single-level deps | `perl-parser-core`, `perl-tokenizer` |
| 3 | Two-level deps | `perl-workspace-index`, `perl-refactoring` |
| 4 | Three-level deps | `perl-semantic-analyzer`, `perl-lsp-providers` |
| 5 | Task runners | `xtask` |
| 6 | Application binaries | `perl-parser`, `perl-lsp`, `perl-dap` |
| 7 | Legacy/testing | `perl-parser-pest`, `perl-corpus` |

This prevents cascading breakage. A Tier 1 change cannot break Tier 1 peers —
they have no internal dependencies. A Tier 3 change can only affect Tiers 4-7.
The build graph is a DAG with predictable fan-out, not a tangle of cross-cutting
dependencies.

### Dual Indexing: One Symbol, Two Keys

The workspace index stores every symbol under both its qualified name
(`Package::function`) and its bare name (`function`):

```rust
// Index under bare name
file_index.references.entry(bare_name.to_string())
    .or_default().push(symbol_ref.clone());

// Index under qualified name
file_index.references.entry(qualified)
    .or_default().push(symbol_ref);
```

This design (established in PR #122) makes navigation instant regardless of how
the user writes their code. Type `Package::do_thing` — instant jump. Type
`do_thing` after a `use Package qw(do_thing)` — also instant. The cost is modest
extra memory; the benefit is that go-to-definition never disappoints.

### Why Recursive Descent Over Tree-sitter or PPI

Perl is notoriously context-sensitive. The slash character `/` can be division
(`$x / 2`) or the start of a regex (`/pattern/`). Heredocs interrupt the token
stream. Quote-like operators use arbitrary delimiters. Formats change the grammar
entirely.

The project tried three approaches:

**Phase 1 — Tree-sitter (July 2022 – mid 2025).** A tree-sitter grammar in
JavaScript with a C scanner for context-sensitive constructs. The scanner grew
into a maintenance burden. It required `libclang` for cross-platform builds.
Error recovery was not flexible enough for IDE-quality diagnostics.

**Phase 2 — Pest PEG grammar (July 2025).** Pure Rust, no C dependency. But
slash disambiguation required preprocessor markers (`_SUB_`, `_TRANS_`, `_DIV_`)
injected before parsing — a fragile workaround. Heredocs were handled by a
separate scanner pass. PEG backtracking caused performance cliffs on
pathological inputs.

**Phase 3 — Native recursive descent (July 2025 – March 2026 snapshot).** A hand-written
parser with a mode-based lexer that resolves context-sensitive ambiguities at
tokenization time:

```rust
pub enum LexerMode {
    ExpectTerm,       // slash starts regex, % starts hash
    ExpectOperator,   // slash is division, % is modulo
    ExpectDelimiter,  // # is not a comment (inside s///)
    InFormatBody,     // consume until lone dot
    InDataSection,    // consume everything to EOF
}
```

This two-mode approach eliminates the need for preprocessor hacks or a C
scanner. The lexer also enforces budget limits to prevent denial-of-service:

- `MAX_REGEX_BYTES`: 64 KB per regex literal
- `MAX_HEREDOC_BYTES`: 256 KB per heredoc
- `MAX_DELIM_NEST`: 128 levels of delimiter nesting
- `HEREDOC_TIMEOUT_MS`: 5-second timeout

The native parser achieves sub-millisecond performance on typical files (1–150us
initial parse, 931ns incremental updates) while producing partial ASTs with typed
error nodes — exactly what an LSP needs for IDE-quality diagnostics.

---

## Part 2: The Agent Development Story

### The Inflection Point: 96 Commits in One Day

For three years (July 2022 through June 2025), the project accumulated 170
commits. A steady, measured pace of human development. Then, on July 16, 2025,
the repository recorded **96 commits in a single day**. Commits landing every
5–15 minutes from midnight through the evening. The agent era had begun.

The numbers tell the rest:

| Period | Commits | Crates | PRs |
|--------|---------|--------|-----|
| Jul 2022 – Jun 2025 (36 months) | 170 | 2–8 | 0 |
| Jul 2025 – Mar 2026 (9 months) | 1,953+ | 130 | 1,857+ |

In nine months, the project produced more than 11x the output of the previous
three years. Over 1,090 PRs have been merged.

### The Multi-Agent Ecosystem

The repository shows evidence of multiple AI agent families contributing:

**Claude Code** is the primary development agent. Guided by a 500-line
`CLAUDE.md` instruction file (evolved through 30+ revisions), Claude Code
operates with explicit coding standards, banned constructs, and architectural
constraints. Branch patterns: `fix/`, `feat/`, `docs/`, `infra/`.

**OpenAI Codex** produced 266 PRs from `codex/`-prefixed branches with
distinctive random suffixes (`codex/split-and-integrate-srp-microcrates-kzvaqa`).
146 merged, 112 closed without merging — a 42% rejection rate that reveals the
trial-and-error nature of agent-scale development.

**Google Jules** created 308 PRs across three personas (Bolt for performance,
Sentinel for security, Palette for UX). Only 47 merged — an 85% rejection rate.
Jules tended to generate plausible-looking work with alarming titles
("[CRITICAL] Fix Path Traversal") that was often fixing non-existent problems.

**Dependabot** handles automated dependency updates across three ecosystems
(Cargo, GitHub Actions, npm).

The rejection rates are instructive: agent output is cheap to generate but
requires human judgment to validate. The CI gates act as a mechanical filter,
but they cannot catch semantic wrongness — agents proposing optimizations against
a stale version of the code, or security fixes for vulnerabilities that do not
exist.

### The Swarm Model: Scouts, Builders, Reviewers

The mature development model uses a pipeline with distinct roles:

```
Scout  →  Issue  →  Builder  →  Draft PR  →  Review  →  CI  →  Merge
```

**Scouts** explore the codebase and file structured GitHub issues describing
problems, root causes, and reproduction steps. They never write code.

**Builders** pick up issues, work in isolated git worktrees, and submit draft
PRs. Each builder gets a focused prompt describing exactly what to fix, which
files to modify, and what verification commands to run.

**Reviewers** validate correctness, coding standards compliance, and scope.
Review has caught 15+ real bugs that would have been merged otherwise.

The **orchestrator** routes work but never writes code directly. Every task
becomes an agent. This separation prevents the coordination bottleneck that
occurs when a single agent tries to do everything.

### The Scout → Constrain → Build Pattern

Early swarm sessions gave builders open-ended prompts: "fix parser errors in
category X." Success rates hovered around 50%. The breakthrough was adding a
scouting phase that constrains the problem space before building begins.

The improved pattern:

1. **Scout** reads the error bucket, examines specific failing files, identifies
   the root cause, and writes a GitHub issue with the exact failing construct
   and a proposed fix strategy.

2. **Builder** receives the scout's findings verbatim as its prompt. It knows
   exactly which code path to modify and has a concrete test case.

Success rates jumped to approximately 90%. The insight: exploration and planning
are cheap. Building from a constrained problem description is dramatically more
reliable than building from an open-ended goal.

### Worktree Isolation: Fearless Parallelism

Every coding agent runs in its own git worktree — a full filesystem checkout
branched from master. This means:

- 50 agents can work simultaneously without merge conflicts
- A failed agent session is disposable — close the worktree, try again
- Each agent sees a consistent snapshot of the codebase
- Mechanical gates (`cargo fmt`, `clippy`, `cargo test`) catch issues before PR

The microcrate architecture makes this safe at the semantic level. An agent
working on `perl-lsp-completion` modifies files that no other agent is touching.
The crate boundary ensures that parallel work is truly independent.

In practice, sessions with 30–50 concurrent agents are routine. The largest
session deployed approximately 100 agents across a mix of scout, builder,
reviewer, and infrastructure improvement roles.

### The Learning Loop

The swarm does not just execute — it improves itself over time:

1. **Observe**: Agents encounter friction (unclear instructions, missing tools,
   repeated failures) during their work.

2. **Memory**: Observations are captured in persistent memory files that survive
   across sessions. The next session's orchestrator reads these memories and
   adjusts behavior.

3. **Skills**: Repeated patterns are extracted into reusable skills (48 skills
   as of March 2026). Each new skill makes all future agents faster — a
   compounding effect.

4. **Hooks**: Enforcement rules that are too important for prompts alone are
   codified as hooks that run automatically. Format checks, safety ratchets,
   and policy gates execute without relying on the agent remembering to invoke
   them.

5. **Enforcement**: The CI gate is the final backstop. If an agent ignores a
   prompt instruction, the hook catches it. If the hook is misconfigured, the
   CI gate catches it. Defense in depth.

---

## Part 3: The Corpus-Driven Development Story

### CPAN Top 1000 as Ground Truth

Most language servers are tested against curated examples. perl-lsp is tested
against thousands of real Perl files from two sources:

**System Perl corpus**: 7,095 `.pm` files from the system Perl installation
(Perl 5.038002). This is the regression baseline. A ratcheting CI gate ensures
the clean-file count can only increase.

**CPAN top-1000 corpus**: The 1,000 most-depended-upon CPAN distributions,
installed locally and swept for parse errors. At the March 19, 2026 snapshot,
the corpus contained 4,355 `.pm` files with a committed baseline of 3,139
clean (72.1%) and a strict known-clean manifest of 1,579 modules that must
stay at zero errors.

The 90% target for CPAN is deliberately not 100%. Some distributions use source
filters, XS-only code, or generated Perl that no static parser should be expected
to handle cleanly.

### Error Bucket Analysis: Fixing What Matters Most

Parser improvements are prioritized by **first-error-per-file analysis**. When a
file fails to parse, only the first error is counted — cascade errors from a
single misparse are noise.

The error bucket analysis from the edge case roadmap reveals the distribution:

| Error Bucket | Files Affected | Wave |
|-------------|:--------------:|:----:|
| Package-qualified subscripts | 261 | 2A |
| Fat arrow as general separator | 91 | 2B |
| `split /regex/` slash after builtin | 22 | 2C |
| Nested ternary edge cases | varies | 2D |

Each bucket becomes a focused task for a builder agent: here is the exact failing
construct, here are the affected files, here is the code path to modify. The
scout has already done the diagnosis; the builder just executes.

### The Ratchet: Coverage Can Only Go Up

The ratchet mechanism is the enforcement layer that makes corpus-driven
development work. It tracks five metrics simultaneously:

1. **Crash count** — Catastrophic failures (stack overflow, infinite loop) must
   be zero. Any crash is a hard failure.
2. **Unreadable files** — Encoding errors must not increase.
3. **Clean file count** — Files with zero ERROR nodes must not decrease.
4. **Total ERROR nodes** — Aggregate error count must not increase.
5. **Per-bucket counts** — Each error category is tracked independently. An
   existing bucket's count must not grow.

A ratchet violation in any dimension blocks the merge. This eliminates the "fix
one, break another" problem that plagues parser work.

The implementation lives in `xtask/src/tasks/cpan_corpus.rs`:

```bash
cargo run -p xtask -- parser-corpus-sweep \
    --manifest .ci/common-corpus-manifest.txt --enforce --receipt
```

### The Coverage Journey

| Date | System Corpus | CPAN Corpus | Driver |
|------|:------------:|:-----------:|--------|
| Mar 9, 2026 (Wave 1) | 51.1% (3,627/7,095) | — | Baseline established |
| Mar 14, 2026 (Wave 2) | ~60% | — | POD, regex, builtins fixed |
| Mar 17, 2026 | 72.4% (5,139/7,095) | 72.1% (3,139/4,355) | CPAN baseline seeded |
| March 2026 target | — | 90%+ | Wave 3-4 parser fixes |

The trajectory from 51% to 72% was driven by four merged PRs in Wave 1 that
fixed POD block skipping, regex false positives, code dereference syntax, and
builtin expansion. Each PR was the result of a scout identifying the error bucket
and a builder implementing the fix — the same pipeline applied systematically.

---

## Part 4: The Self-Improving Swarm Story

### Skills Compound

The project maintains 48 skills (reusable prompt recipes) as of March 2026. Each
skill encapsulates a workflow: `/parser-fix` handles TDD parser repair,
`/verify` runs per-crate validation, `/corpus-ratchet` runs sweep-compare-update
cycles, `/review-pr` validates a single PR end-to-end.

The compounding effect is real: when a new builder agent invokes `/parser-fix`,
it inherits the accumulated knowledge of every previous parser fix — the TDD
workflow, the verification commands, the common pitfalls. The 48th skill makes
the 49th agent faster, and the 100th agent faster still.

### Friction Logging as the Improvement Backlog

Every agent session encounters friction: unclear instructions that cause wasted
attempts, missing tools that force workarounds, CI quirks that block otherwise
good work. Rather than treating these as background noise, the project captures
them as the improvement backlog.

A friction log entry becomes a GitHub issue. That issue becomes a builder task.
The builder's fix (a new skill, a CI adjustment, a documentation update)
eliminates the friction for all future agents. This is not a theoretical loop —
the project's memory system contains 50+ persistent feedback entries, each
capturing a specific lesson:

- "Agents adding tests must run `update-current-status.py` or the policy check
  gate fails" — discovered when multiple agents shipped passing tests but
  forgot the metrics update step.

- "Two agents on the same bug reveals a better solution — feature, not waste" —
  reframing duplicate work as competitive exploration.

- "Rebase `--ours`/`--theirs` is INVERTED from merge; agents get this wrong
  systematically" — a pitfall that required documenting once and enforcing
  forever.

### The "Built but Not Wired" Discovery

A recurring pattern in agent-generated code: the agent builds a correct
implementation but fails to wire it into the system. A new LSP handler exists
in its crate but is never registered in the server dispatch. A new xtask
subcommand works but is never added to the justfile.

This is not a random failure mode — it reveals a systematic gap in agent
context. The agent understands its crate boundary well but lacks visibility into
the integration points beyond that boundary. The fix was better scouting:
scouts now verify not just that the code works in isolation but that it is
reachable from the system entry points.

### Platform Ceiling Discovery

Working at the scale of 50-100 concurrent agents surfaces platform limitations
that single-agent use never encounters:

- **CI queue depth**: Rapid merges cancel each other's CI runs. The project
  learned to merge in batches of 3, waiting for completion between batches.

- **Context window pressure**: Large crates exhaust agent context before the
  work is done. The microcrate architecture is partly motivated by keeping
  each agent's working set small enough to fit in context.

- **Coordination cost**: Without a shared planning layer, agents converge on
  the same opportunities. PRs #1244, #1245, and #1246 all independently
  implemented the same `just doctor` command. The solution was better scouting
  and issue assignment — scouts file issues, builders claim them.

---

## Part 5: Unique Technical Decisions

### No Panics in Production

Production code contains zero calls to `unwrap()`, `expect()`, `panic!()`,
`todo!()`, `unimplemented!()`, or `dbg!()`. The ratchet baseline in this
snapshot is
literally zero for all of these. Three automated gates enforce it:

1. **Clippy lint gates** (`clippy::unwrap_used`, `clippy::expect_used`) run
   on every merge.
2. **A ratchet script** (`ci/check_unwraps_prod.sh`) compares counts against
   a baseline. If the count increases, the gate fails.
3. **A forbidden-constructs binary** (`perl-ci-hygiene`) catches patterns
   outside Clippy's scope: `std::process::abort()`, misplaced `exit()` calls,
   `panic!`-family macros.

A parallel unsafe syntax ratchet enforces zero explicit `unsafe` blocks in
production source.

The only exception is a single centralized `#[allow(clippy::expect_used)]` for
an `lsp_types::Uri` fallback in `crates/perl-lsp-rs/src/util/uri.rs`.

The motivation is direct: a language server that crashes takes down your editor
session. Graceful degradation is not optional.

### Result/Option Everywhere

The ban on panicking constructs forces a discipline: every fallible operation
returns `Result` or `Option`. In tests, the project uses `Result<()>` return
types and dedicated `perl_tdd_support::must`/`must_some` helpers instead of
assertions that could panic.

For regex initialization — a common source of `unwrap()` in Rust code — the
project uses `Option<Regex>` with `.ok()` for graceful degradation. If a regex
fails to compile (which should not happen with hardcoded patterns but
theoretically could), the feature degrades silently rather than crashing.

### Enterprise-Grade Security in a Language Server

A language server has an unusual threat model: it runs with the user's full
filesystem permissions, processes untrusted code (the files being edited), and
in some configurations accepts network connections. perl-lsp treats this
seriously:

**Path traversal prevention**: Three-layer defense prevents directory traversal
attacks through LSP requests. The server validates, canonicalizes, and
sandboxes all paths derived from client input.

**Command injection hardening**: The DAP evaluate handler, perldoc lookup, and
perlcritic integration all sanitize arguments to prevent shell injection.

**Supply chain security**: Every release includes SBOMs in SPDX and CycloneDX
formats. SLSA Level 2 provenance attestations are generated for release
artifacts. Dependencies are audited via `cargo-deny` (license allowlists,
advisory scanning, source restrictions) and `cargo-audit` (vulnerability
scanning). Trivy scans the repository and Docker images.

**Dependency governance**: Only MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause,
ISC, Unicode-3.0, CC0-1.0, and Zlib licenses are permitted. Unknown git sources
generate warnings. Major version updates for critical dependencies require
manual review.

### Feature Governance with Profiles and Maturity Tracking

The LSP feature catalog is not a checklist — it is a machine-readable governance
system. `features.toml` (953 lines) declares every feature with metadata:
LSP spec version, functional area, maturity level, whether it is advertised to
clients, associated test files, and whether it counts toward compliance metrics.

A build-time pipeline compiles `features.toml` into Rust constants:

```
features.toml
    → perl-feature-catalog         (parse TOML, validate)
    → perl-lsp-feature-contracts   (build.rs → const arrays)
    → perl-lsp-feature-flags       (BuildFlags, presets)
    → perl-lsp-feature-profile     (CLI token parsing)
    → perl-lsp-feature-policy      (profile + runtime → flags)
    → perl-lsp-feature-governance  (facade)
    → perl-lsp                     (server binary)
```

This means the runtime catalog is always derived from the same TOML file. A
feature cannot be implemented but forgotten, or advertised but untested, or
counted in one compliance report but not another. The governance system is itself
decomposed into 9 microcrates, following the same SRP principle as the rest of
the workspace.

---

## Part 6: The Numbers

These numbers are drawn from the git history, `features.toml`, and
`CURRENT_STATUS.md` as of March 19, 2026.

### Repository Scale

| Metric | Value |
|--------|-------|
| Total commits (first-parent) | 2,123+ |
| Pre-agent era (Jul 2022 – Jun 2025) | 170 commits |
| Agent era (Jul 2025 – Mar 2026) | 1,953+ commits |
| PRs created (all time) | 1,857+ |
| PRs merged | 1,090+ |
| Unique contributors | 20 |
| Total Rust source lines | ~547,000 |
| Crate directories | 130 |

### Quality Metrics

| Metric | Value |
|--------|-------|
| Tier A tests (lib) | 2,100+ passing |
| Tracked test debt (ignores) | 0 |
| Mutation score | 87% |
| `unwrap`/`expect` in production | 0 |
| `panic!`-family macros in production | 0 |
| Explicit `unsafe` in production | 0 |
| Fuzz targets | 7 |

### LSP Completeness

| Metric | Value |
|--------|-------|
| Features defined in `features.toml` | 97 |
| Features implemented (GA maturity) | 97/97 (100%) |
| Features advertised to clients | 96/97 |
| LSP spec version | 3.18 |
| Debug adapter (DAP) features | 10/10 |

### Parser Coverage

| Corpus | Clean Files | Total Files | Coverage |
|--------|:-----------:|:-----------:|:--------:|
| System Perl | 5,139 | 7,095 | 72.4% |
| CPAN top-1000 | 3,139 | 4,355 | 72.1% |
| CPAN known-clean manifest | 1,579 | 1,579 | 100% |
| Common corpus (CI-gated) | all | all | 100% |

### Performance

| Operation | Latency |
|-----------|---------|
| Initial parse (typical file) | 1–150 us |
| Incremental parse (edit) | 931 ns |
| LSP response time | <50 ms |
| Semantic token extraction | 2.8 us |

### Agent Development

| Metric | Value |
|--------|-------|
| Agent families contributing | 4 (Claude Code, Codex, Jules, Dependabot) |
| Codex PRs created | 266 (55% merge rate) |
| Jules PRs created | 308 (15% merge rate) |
| Reusable skills | 48 |
| Persistent feedback memories | 50+ |
| Largest single-day commit count | 152 (March 4, 2026) |
| Largest single-session agent count | ~100 |

---

## What This Means

perl-lsp is simultaneously a production language server and a case study in
agent-assisted software engineering. The architecture — 130 microcrates, seven
dependency tiers, ratcheting corpus gates, zero-panic production code — was
shaped by the demands of both goals.

The microcrate architecture exists because it enables fearless parallelism:
dozens of agents modifying dozens of crates with no coordination overhead. The
corpus-driven development exists because synthetic tests cannot catch the
breadth of real-world Perl. The learning loop exists because a swarm that does
not improve itself produces the same mistakes at higher volume.

None of this is magic. It is engineering discipline applied to a new mode of
development. The agents do the volume work. The CI gates do the mechanical
verification. The human does the architectural thinking, the quality judgment,
and the strategic direction. Between those three forces, a solo developer can
build and maintain a 130-crate workspace that would otherwise require a team.

Perl deserves better tooling. This is how it gets built.

---

*For detailed metrics, see [CURRENT_STATUS.md](CURRENT_STATUS.md).
For architecture details, see [WORKSPACE_ARCHITECTURE.md](WORKSPACE_ARCHITECTURE.md).
For the parser journey, see [PARSER_EVOLUTION.md](PARSER_EVOLUTION.md).
For the agent development case study, see [AGENTIC_DEVELOPMENT.md](AGENTIC_DEVELOPMENT.md).
For quality infrastructure, see [QUALITY_INFRASTRUCTURE.md](QUALITY_INFRASTRUCTURE.md).*
