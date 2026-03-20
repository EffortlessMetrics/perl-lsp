# CLAUDE.md

**Latest Release**: 0.12.0 | **Metrics**: [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) | **API Stability**: [STABILITY.md](docs/reference/STABILITY.md)

## Orchestration Model

The orchestrator routes work to agents, never writes code directly.

- **Code change** -> worktree agent: `Agent(isolation: "worktree", prompt: "Goal: ... Crate: ... Verify: cargo fmt && cargo clippy -p <crate> --tests && cargo test -p <crate>. Commit and create PR.")`
  - Scout before building new features. Use 3:1 scout:builder ratio. Scout output = builder spec (exact files, functions, verify commands).
- **Research** -> explore agent: `Agent(subagent_type: "Explore", prompt: "Find ... in crates/...")`
- **Multiple changes** -> parallel worktree agents, one per crate. Microcrate architecture prevents conflicts.
  - Reserve 10 agent slots for late-cycle routing. Use SendMessage to repurpose idle agents instead of spawning new ones when roster is full.

### Merge Queue Protocol

- Don't rebase PRs unless merge conflicts exist
- Merge in batches of 3 (CI cancellation cascade -- rapid merges cancel each other's CI runs)
- Run `just cpan-corpus-ratchet` after parser fix merges
- CURRENT_STATUS.md is regenerated automatically post-merge (no manual step needed)

## Quick Reference

```bash
nix develop -c just ci-gate           # Canonical local gate (REQUIRED before push)
cargo build -p perl-lsp --release     # Build LSP server
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

128 workspace members across 129 crate directories (see `cargo metadata --no-deps`). Key crates:

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
cargo build -p perl-lsp --release     # LSP server
cargo build -p perl-parser --release  # Parser library
cargo test                            # All tests
cargo test -p perl-parser             # Parser tests
cargo test -p perl-lsp                # LSP tests
cargo test -p perl-parser -- test_name --exact  # Exact test in crate
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2  # LSP threading
just ci-lsp-def                       # Semantic definition tests
```

### Lint, Format, Quality

```bash
cargo fmt --all                       # Format code
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
| **A (PR-fast)** | `just pr-fast` | ~1-2 min | Quick iteration |
| **B (Merge gate)** | `just ci-gate` | ~3-5 min | Before pushing (required) |
| **C (Nightly)** | `just ci-full` | ~15-30 min | Mutation, fuzzing, benchmarks |

## Parser Versions

- **v3 (Native)**: Current recursive descent parser
- **v2 (Pest)**: Legacy, kept out of default gate
- **v1 (C-based)**: Benchmarking only

## Workspace Exclusions

`crates/tree-sitter-perl-c/` (needs libclang), `tree-sitter-perl/` (legacy C), `fuzz/` (fuzz builds), `archive/` (archived).

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
| Build tooling | `xtask/` |
| Slash commands | `.claude/commands/` |
| Swarm ops | `.ops-perl-lsp/` |

## Architecture Patterns

**Dual indexing**: Index workspace symbols under both qualified and bare names (see PR #122).

**LSP threading**: `RUST_TEST_THREADS=2`, `CARGO_BUILD_JOBS=1`, `RUSTC_WRAPPER=""`.

## Truth Sources

Metrics are **computed, not hand-edited**:
- `CURRENT_STATUS.md` auto-generated via `scripts/update-current-status.py`
- `features.toml` is the canonical LSP capability definition
- Test output and CI receipts are evidence for all claims
- `README.md` must not contain volatile metrics -- link to CURRENT_STATUS.md

## Coding Standards

Invoke `/coding-standards` for full detail.

- Run `cargo fmt` and `cargo clippy --workspace` before committing
- **Banned in production code**: `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, `std::process::abort()`, `dbg!()`
  - Use `?`, `.ok_or_else()`, pattern matching, `Result`/`Option` instead
  - `std::process::exit()` only in `bin/` and `lifecycle.rs`
  - Exception: `#[allow(clippy::expect_used)]` in `crates/perl-lsp/src/util/uri.rs`
  - Tests: `Result<()>` returns or `perl_tdd_support::must`/`must_some`
- **Prefer**: `.first()` over `.get(0)`, `.push(char)` over `.push_str("x")`, `or_default()` over `or_insert_with(Vec::new)`
- **Avoid**: unnecessary `.clone()` on Copy types
- **Regex**: `Option<Regex>` with `.ok()` for graceful degradation
- After adding tests, no manual status update needed — CURRENT_STATUS.md is auto-regenerated post-merge

## Documentation

[CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) | [ROADMAP.md](docs/project/ROADMAP.md) | [COMMANDS_REFERENCE.md](docs/reference/COMMANDS_REFERENCE.md) | [LSP_IMPLEMENTATION_GUIDE.md](docs/reference/LSP_IMPLEMENTATION_GUIDE.md) | [features.toml](features.toml)

## Contributing

Run `nix develop -c just ci-gate` before pushing. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Continuous Swarm Development

**Session start**: Run `just clean-worktrees` to prune stale agent worktrees before spawning new ones.

Start with `/swarm all`. Orchestrator spawns scoped agents from the catalog in worktree isolation. ~20% capacity reserved for background improvement.

**Key commands**: `/swarm` (start), `/swarm-protocol` (rules), `/coding-standards` (standards), `/verify` (crate gate), `/parser-fix` (TDD fix).

**PR lifecycle**: Draft PR -> reviewer agent -> `/pr-ready` -> CI -> ops agent merges.

**Files**: `.ops-perl-lsp/` (metrics), `.claude/agents/` (agent defs and catalog), `.claude/commands/` (step skills and shared ops).
