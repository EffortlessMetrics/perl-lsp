# CLAUDE.md

**Latest Release**: 0.12.4 | **Metrics**: [status/index.md](docs/project/status/index.md) | **API Stability**: [STABILITY.md](docs/reference/STABILITY.md)

## Orchestration Model

The orchestrator routes work to agents, never writes code directly.

### Pipeline: Scout → Accuracy-Scout → Plan-Review → Build → Review → Green → Merge → Wisdom

Every change flows through this pipeline. Each stage is a cheap pass that catches what the previous one missed.

| Stage | Model | Purpose | Fix forward? |
|-------|-------|---------|-------------|
| **Scout** (haiku) | Broad discovery | Find the problem, file roughly-right spec | N/A — files issues |
| **Accuracy-scout** (haiku) | Mechanical fact check | Verify file paths, function names, issue status against master | No — corrects facts, not plans |
| **Plan-review** (sonnet) | Improve the plan | Fill gaps, correct root cause, add edge cases | Yes — complete the spec yourself |
| **Build** (sonnet) | Execute the spec | TDD: test → implement → verify → PR | Yes — adapt if plan-reviewed; bump back if not |
| **Review** (haiku/sonnet) | Improve the PR | Push fixes directly to the branch | Yes — always fix forward |
| **Green** | CI gate | SHA-verified, merge-time fresh check | N/A |
| **Merge** | Ops | Batch of 3, wait for green, ratchet corpus | N/A |
| **Wisdom** | Learning | Retrospective, update memory, log patterns | N/A |

**Key principles:**
- The orchestrator routes, it doesn't execute. Never poll CI, read diffs, or check PR state in loops. Launch an agent with the full job and move to the next routing decision.
- One status check to inform routing, then delegate. When the orchestrator has context (exact edits, file contents), pass it to the agent — don't make agents re-research what you already know.
- Scouts are honest about uncertainty — plan-reviewers correct. Being roughly right > confidently wrong.
- Accuracy-scouts verify mechanical facts only (file paths, function names, issue status). They do not redesign the spec or suggest approaches.
- Plan-reviewers improve plans, never punt "needs more scout work." They're enhanced scouts with sonnet.
- Builders execute the spec as given. Fix forward on small gaps, bump back if structural.
- Reviewers push improvements directly to PR branches. Every PR gets improved, no LGTM-only.
- Every agent recommends next steps for the orchestrator.
- Learning is continuous — every agent-wrapup captures what was learned.

### Pipeline State Labels

Labels are the authoritative state for every issue and PR. The orchestrator reads them; agents write them.

| Label | Set by | Means |
|-------|--------|-------|
| `needs-plan-review` | scout (/scout-report) | Awaiting plan-reviewer |
| `plan-reviewed` | plan-reviewer (/plan-review-improve) | Spec verified |
| `builder-ready` | plan-reviewer (/plan-review-improve) | Ready for builder pickup |
| `in-build` | builder (/builder-read-spec) | Builder claimed this issue |
| `in-review` | reviewer (/reviewer-read-handoff) | PR actively in review — set at review start |
| `merge-ready` | reviewer (/pr-ready) | Ready for ops merge; docs-only PRs may reach this without `reviewed-deep` |
| `structural-blocker` | any agent | Architecture issue; blocks parallel work |
| `needs-deep-review` | reviewer (/reviewer-decide) | Standards review done, awaiting deep correctness review |
| `reviewed-deep` | reviewer-deep (/reviewer-deep-decide) | Deep correctness review complete — required before merge for non-docs PRs |
| `follow-up-recommended` | wisdom or reviewer | Related follow-up issue needed |
| `already-fixed` | plan-reviewer or scout | Close without build |

Labels gate entry, not skip execution. Multiple passes of the same agent are normal. Query examples:
```bash
gh issue list --label "builder-ready" --state open   # ready to build
gh issue list --label "in-build" --state open        # builder assigned
gh issue list --label "structural-blocker" --state open  # blocked work
```

Note: `needs-accuracy-scout` and `accuracy-reviewed` are reserved for the accuracy-scout agent (issue #2628).

### Routing patterns

- **Code change** -> worktree agent: `Agent(isolation: "worktree", prompt: "...")`
- **Research** -> explore agent: `Agent(subagent_type: "Explore", prompt: "...")`
- **Multiple changes** -> parallel worktree agents, one per crate. Microcrate architecture prevents conflicts.
  - Reserve 10 agent slots for late-cycle routing. Use SendMessage to repurpose idle agents.

### Merge Queue Protocol

- Don't rebase PRs unless merge conflicts exist
- Merge in batches of 3 (CI cancellation cascade -- rapid merges cancel each other's CI runs)
- Run `just cpan-corpus-ratchet` after parser fix merges
- `docs/project/status/*.md` subsystem files are regenerated automatically post-merge (no manual step needed)

## Quick Reference

```bash
just doctor                           # Workspace health check (run before any agent-spawning session)
just pr-fast                          # Canonical fast push guard
nix develop -c just ci-gate           # Canonical local merge gate (before merge)
cargo build -p perl-lsp-rs --release     # Build LSP server
cargo test --workspace --lib          # Run all tests
```

| Task | Pattern |
|------|---------|
| Code change | `Agent(isolation: "worktree", ...)` |
| Research | `Agent(subagent_type: "Explore", ...)` |
| Parser fix | `/parser-fix` |
| Swarm cycle | `/swarm all` |
| Crate verification | `/verify <crate>` |

## Crate Structure

134 workspace members across 135 crate directories (see `cargo metadata --no-deps`). Key crates:

| Crate | Path | Purpose |
|-------|------|---------|
| **perl-parser** | `crates/perl-parser/` | Main parser (v3 recursive descent) |
| **perl-lsp** | `crates/perl-lsp/` | LSP server binary |
| **perl-dap** | `crates/perl-dap/` | Debug Adapter Protocol |
| **perl-lexer** | `crates/perl-lexer/` | Context-aware tokenizer |
| **perl-parser-core** | `crates/perl-parser-core/` | Core parsing infrastructure |
| **perl-workspace-index** | `crates/perl-workspace-index/` | Workspace symbol indexing |
| **perl-semantic-analyzer** | `crates/perl-semantic-analyzer/` | Semantic analysis |

Families: `perl-module-*` (module resolution), `perl-lsp-*` (LSP providers), `perl-lsp-feature-*` (feature governance), `perl-dap-*` (DAP), `perl-ts-*` (tree-sitter), `perl-workspace-*` (workspace discovery), core leaf crates (token, AST, quote, regex, heredoc, error).

## Essential Commands

### Build & Test

```bash
cargo build -p perl-lsp-rs --release     # LSP server
cargo build -p perl-parser --release  # Parser library
cargo test                            # All tests
cargo test -p perl-parser             # Parser tests
cargo test -p perl-lsp-rs                # LSP tests
cargo test -p perl-parser -- test_name --exact  # Exact test in crate
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2  # LSP threading
just ci-lsp-def                       # Semantic definition tests
```

### Lint, Format, Quality

```bash
cargo xtask fmt                       # Format code (per-crate, Windows-safe)
cargo clippy --workspace              # Lint all crates
cargo clippy --workspace --lib        # Lint libraries only (faster)
just dead-code                        # Dead code report
cargo machete                         # Unused dependencies
just security-audit                   # Security audit
just semver-check                     # SemVer check all published packages
```

### Benchmarks, Fuzzing, Coverage

```bash
just benchmarks                       # Run all benchmarks
just fuzz-bounded                     # Bounded fuzz run (60s per target)
just mutation-subset                  # Mutation testing subset
just coverage                         # HTML coverage report
just coverage-lcov                    # lcov.info for CI
```

### Health & Status

```bash
just health                           # Codebase metrics
just status-check                     # Verify computed metrics
just debt-report                      # Technical debt status
just debt-check                       # Debt budget compliance
```

### CPAN Corpus

```bash
just cpan-corpus-sweep                # Sweep and report
just cpan-corpus-check                # Enforce manifest (fails on regression)
just cpan-corpus-ratchet              # Auto-add clean modules to manifest
```

## Development Workflow

**Local-first** -- all gates run locally before CI. Install hook: `bash scripts/install-githooks.sh`

### CI Gate Tiers

| Tier | Command | Time | When |
|------|---------|------|------|
| **A (PR-fast)** | `just pr-fast` | ~1-2 min | Quick iteration and pre-push hook |
| **B (Merge gate)** | `just ci-gate` | ~3-5 min | Before merge |
| **C (Nightly)** | `just ci-full` | ~15-30 min | Mutation, fuzzing, benchmarks |

## Parser Versions

- **v3 (Native)**: Current recursive descent parser
- **v2 (Pest)**: Legacy, kept out of default gate
- **v1 (C-based)**: Benchmarking only

## Workspace Exclusions

`tree-sitter-perl/` (legacy C), `fuzz/` (fuzz builds), `archive/` (archived).

## Key Paths

| What | Where |
|------|-------|
| Parser source | `crates/perl-parser/src/` |
| LSP providers | `crates/perl-lsp-*/src/` |
| LSP server binary | `crates/perl-lsp/src/` |
| DAP server | `crates/perl-dap/src/` |
| Tests | `crates/*/tests/` |
| Test corpus | `test_corpus/`, `tree-sitter-perl/test/corpus/` |
| VSCode extension | `vscode-extension/` |
| Documentation | `docs/` |
| Features catalog | `features.toml` |
| CI config | `.ci/` |
| Known blockers | `.ci/blockers.yaml` |
| Build tooling | `xtask/` |
| Slash commands | `.claude/commands/` |
| Swarm ops | `.ops-perl-lsp/` |

## Architecture Patterns

**Dual indexing**: Index workspace symbols under both qualified and bare names (see PR #122).

**LSP threading**: `RUST_TEST_THREADS=2`, `CARGO_BUILD_JOBS=1`, `RUSTC_WRAPPER=""`.

**Worktree stash prohibition**: Never use `git stash` in a worktree agent. The stash list is shared across all worktrees and the main checkout — `git stash pop` may silently restore another agent's changes. Use `git restore <file>` to discard changes, or `git commit -m "wip"` to save work in progress.

## Truth Sources

Metrics are **computed, not hand-edited**:
- `docs/project/status/*.md` subsystem files auto-generated via `just status-update` (writes lsp.md, tests.md, parser.md, quality.md)
- `docs/project/CURRENT_STATUS.md` is now a stable stub linking to the subsystem files (no `<!-- BEGIN: -->` markers)
- `features.toml` is the canonical LSP capability definition
- Test output and CI receipts are evidence for all claims
- `README.md` must not contain volatile metrics -- link to `docs/project/status/index.md`
- `.ci/blockers.yaml` is manually maintained — verify counts against `parser-corpus-baseline.json` before trusting `affected_files` values

## Coding Standards

Invoke `/coding-standards` for full detail.

- Run `cargo fmt` and `cargo clippy --workspace` before committing
- **Banned in production code**: `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, `std::process::abort()`, `dbg!()`
  - Use `?`, `.ok_or_else()`, pattern matching, `Result`/`Option` instead
  - `std::process::exit()` only in `bin/` and `lifecycle.rs`
  - Exception: `#[allow(clippy::expect_used)]` in `crates/perl-lsp/src/util/uri.rs`
  - Exception: `bin/` targets may use `#[allow(clippy::expect_used)]` for profiling / CLI entry points, including `crates/perl-workspace-index/src/bin/workspace_memory_profile.rs`
  - Exception: static `LazyLock<Regex>` initializers may use `unreachable!()`/`expect()` for known-good patterns, including `crates/perl-heredoc-anti-patterns/src/lib.rs`
  - Tests: `Result<()>` returns or `perl_tdd_support::must`/`must_some`
- **Prefer**: `.first()` over `.get(0)`, `.push(char)` over `.push_str("x")`, `or_default()` over `or_insert_with(Vec::new)`
- **Avoid**: unnecessary `.clone()` on Copy types
- **Regex**: `Option<Regex>` with `.ok()` for graceful degradation
- After adding tests, no manual status update needed — `docs/project/status/*.md` files are auto-regenerated post-merge

## Documentation

[Status Overview](docs/project/status/index.md) | [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) (stub) | [ROADMAP.md](docs/project/ROADMAP.md) | [COMMANDS_REFERENCE.md](docs/reference/COMMANDS_REFERENCE.md) | [LSP_IMPLEMENTATION_GUIDE.md](docs/reference/LSP_IMPLEMENTATION_GUIDE.md) | [features.toml](features.toml)

## Contributing

Run `just pr-fast` while iterating and `nix develop -c just ci-gate` before merge. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Continuous Swarm Development

**Session start**: Run `just clean-worktrees` to prune stale agent worktrees before spawning new ones.

Start with `/swarm all`. Orchestrator spawns scoped agents from the catalog in worktree isolation. ~20% capacity reserved for background improvement.

**Key commands**: `/swarm` (start), `/swarm-protocol` (rules), `/coding-standards` (standards), `/verify` (crate gate), `/parser-fix` (TDD fix).

**PR lifecycle**: Draft PR -> reviewer agent -> `/pr-ready` -> CI -> ops agent merges.

**Files**: `.ops-perl-lsp/` (metrics), `.claude/agents/` (agent defs and catalog), `.claude/commands/` (step skills and shared ops).
