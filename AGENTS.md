# AGENTS.md

Guidance for coding agents working in this repository.

## Start From Current Truth

Use these files as the primary sources of truth before you restate project facts:

- [`Cargo.toml`](Cargo.toml): workspace members, package version, published crate line
- [`docs/project/CURRENT_STATUS.md`](docs/project/CURRENT_STATUS.md): evidence-backed metrics and receipts
- [`docs/project/ROADMAP.md`](docs/project/ROADMAP.md): canonical roadmap and active milestone
- [`features.toml`](features.toml): LSP capability catalog
- [`docs/README.md`](docs/README.md): documentation index

Do not hardcode workspace counts, release numbers, or performance metrics in new docs unless you regenerated or verified them in the same change.

## Project Shape

This repository is a large Rust workspace that ships editor tooling, parser infrastructure, and supporting libraries for Perl 5.

### Main entry points

| Path | Purpose |
| --- | --- |
| `/crates/perl-lsp-rs/` | LSP binary and server host |
| `/crates/perl-dap/` | Debug Adapter Protocol server |
| `/crates/perl-parser/` | Native recursive-descent parser |
| `/crates/perl-lexer/` | Context-aware tokenizer |
| `/crates/perl-parser-core/` | Shared parser infrastructure |
| `/crates/perl-semantic-analyzer/` | Semantic analysis and resolution |
| `/crates/perl-workspace-index/` | Cross-file indexing and lookup |
| `/crates/perl-corpus/` | Corpus and regression fixtures |

### Where to work

1. Parser behavior: `/crates/perl-parser/`, `/crates/perl-parser-core/`, `/crates/perl-lexer/`
2. LSP features: `/crates/perl-lsp-rs/` and `/crates/perl-lsp-*/`
3. DAP features: `/crates/perl-dap/` and `/crates/perl-dap-*/`
4. Semantic resolution: `/crates/perl-semantic-analyzer/`, `/crates/perl-workspace-index/`
5. Corpus and regressions: `/crates/perl-corpus/`, `/crates/*/tests/`, `/test_corpus/`

## Validation Commands

AI tools can run bare `cargo build` and `cargo test`; `.cargo/config.toml` handles the repo defaults.

### Fast feedback

```bash
just devex
just pr-fast
```

### Canonical local gate

```bash
nix develop -c just ci-gate
```

### Status and documentation drift

```bash
just status-update
just status-check
```

Run `just status-update` and `just status-check` when you change capability docs, generated status sections, or other docs that depend on computed project metrics.

### Verification Ladder

Prefer cheap, orthogonal passes over one broad gate:

1. Check recent merged/open PRs and issues for the area before scoping new work.
2. Use the narrowest truth-check first for claims: targeted repro/test for behavior, history/doc verification for attribution or status claims.
3. Escalate to `just pr-fast` or `nix develop -c just ci-gate` only after the narrow pass is green.

## PR Hygiene

- Use isolated worktrees and focused branches for agent-driven PR work.
- Give PRs a CI-compliant title before opening them: `type(scope): summary (#1234)`.
- The `validate-title` check rejects titles that do not include an issue reference.
- Put the issue reference in the PR title, not just in the body.

## Documentation Discipline

- [`docs/project/CURRENT_STATUS.md`](docs/project/CURRENT_STATUS.md) is the evidence document.
- [`docs/project/ROADMAP.md`](docs/project/ROADMAP.md) is the planning document.
- [`ROADMAP.md`](ROADMAP.md) and [`NOW_NEXT_LATER.md`](NOW_NEXT_LATER.md) should stay short and point back to the canonical project docs.
- Keep the current release line separate from the next milestone. Do not blur “shipped” and “targeted”.
- Prefer links to canonical docs over copying the same table into multiple places.

## Coding Expectations

- Run `cargo xtask fmt` for formatting (per-crate invocation, Windows-safe).
- Run the narrowest relevant tests first, then the broader gate when the change is ready.
- `unwrap()`, `expect()`, `panic!()`, `todo!()`, and `unimplemented!()` are banned in production code.
- In tests, prefer `Result<()>` and helpers such as `perl_tdd_support::must` and `must_some`.
- Prefer `.first()` over `.get(0)`.
- Use `.push(char)` instead of `.push_str("x")` for single characters.
- Use `or_default()` instead of `or_insert_with(Vec::new)`.
- Avoid unnecessary `.clone()` on `Copy` types.

## Useful References

- [`CONTRIBUTING.md`](CONTRIBUTING.md)
- [`docs/reference/COMMANDS_REFERENCE.md`](docs/reference/COMMANDS_REFERENCE.md)
- [`docs/reference/LSP_IMPLEMENTATION_GUIDE.md`](docs/reference/LSP_IMPLEMENTATION_GUIDE.md)
- [`docs/reference/CRATE_ARCHITECTURE_GUIDE.md`](docs/reference/CRATE_ARCHITECTURE_GUIDE.md)
- [`docs/tutorials/DAP_USER_GUIDE.md`](docs/tutorials/DAP_USER_GUIDE.md)
