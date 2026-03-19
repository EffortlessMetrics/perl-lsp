# Maintainer Vision

> perl-lsp exists to give Perl the language tooling it deserves.

---

## Why This Project Exists

Perl powers critical infrastructure across finance, biotech, government, and operations. Millions of lines of Perl run in production today. Yet Perl developers have been left behind by the language tooling revolution that TypeScript, Rust, Go, and Python developers now take for granted.

The existing Perl language servers require a Perl runtime, offer partial LSP coverage, and cannot provide the sub-millisecond responsiveness that modern editors demand. perl-lsp closes that gap: a native Rust binary that delivers completions, diagnostics, hover, navigation, refactoring, and debugging — with no Perl runtime required.

**The goal is simple**: make Perl development as productive and pleasant as working in any modern language.

---

## Architecture Philosophy

### Parser First

Everything flows from accurate parsing. If the parser gets it wrong, completions are wrong, diagnostics are misleading, and navigation breaks. That is why the v3 recursive-descent parser is the foundation of the entire project.

The parser handles the full breadth of Perl 5 syntax (5.8 through 5.40): heredocs, regex, quoting constructs, formats, context-sensitive slash disambiguation, and the many other constructs that make Perl notoriously difficult to parse. It is tested continuously against real-world Perl code from CPAN and system installations, with ratcheting CI gates that ensure coverage can only go up.

### Microcrate Architecture

The workspace contains 128+ focused Rust crates, each with a single responsibility. This is not accidental complexity — it is a deliberate design choice with concrete benefits:

- **Parallel development**: Multiple contributors can work on different crates simultaneously without merge conflicts. The crate boundary is the coordination boundary.
- **Fast compilation**: Changing one crate only recompiles that crate and its dependents, not the entire workspace.
- **Clear ownership**: Every piece of functionality has exactly one home. When something breaks, you know where to look.
- **Independent testing**: Each crate has its own test suite. You can run `cargo test -p perl-lsp-completion` without touching the parser.
- **Enforced dependency discipline**: The tiered dependency structure (leaf crates at Tier 1 through application crates at Tier 6) prevents circular dependencies and keeps the architecture clean.

### Dual Indexing

The workspace index stores every symbol under both its qualified name (`Package::function`) and its bare name (`function`). This enables fast lookup in both contexts — when a user types a fully qualified call and when they rely on imports. The cost is modest extra memory; the benefit is instant navigation and completion regardless of how the user writes their code.

### Single Responsibility, Everywhere

Every crate does one thing well. `perl-lexer` tokenizes. `perl-parser-core` provides parsing infrastructure. `perl-lsp-completion` handles completions. `perl-lsp-hover` handles hover. This principle extends beyond crates: each LSP provider is a self-contained unit that can be understood, tested, and improved in isolation.

---

## Quality Standards

### No Panics in Production

Production code must never call `unwrap()`, `expect()`, `panic!()`, `todo!()`, or `unimplemented!()`. Every fallible operation returns `Result` or `Option`. This is not a guideline — it is enforced by CI ratchets that scan the codebase and fail the build on violations.

The reasoning is straightforward: a language server runs inside an editor. A panic kills the process. A killed process means the user loses completions, diagnostics, and navigation until they restart. That is unacceptable.

### CPAN Corpus as Ground Truth

Parser quality is measured against real Perl code, not synthetic tests. The CPAN top-1000 corpus and the system Perl installation provide thousands of `.pm` and `.pl` files that the parser must handle without errors. Ratcheting baselines ensure that coverage can only increase — every merged PR either maintains or improves the parse rate.

### Automated Gates

Every merge must pass through automated quality gates:

- **Format**: `cargo fmt` ensures consistent style.
- **Lint**: `cargo clippy --workspace` catches common mistakes and enforces idioms.
- **Tests**: The full test suite must pass. No exceptions.
- **Corpus ratchet**: Parser coverage baselines can only go up.
- **Safety ratchet**: Zero `unwrap`, zero `panic!`-family macros, zero `unsafe` in production code.
- **SemVer check**: Published crate APIs are checked for breaking changes.

The canonical gate command is `nix develop -c just ci-gate`. If it passes, you can push. If it does not, fix the issue before pushing.

### SemVer Compliance

Published crates (`perl-lsp`, `perl-parser`, `perl-lexer`, `perl-corpus`) follow strict Semantic Versioning. Breaking changes are detected automatically by `cargo-semver-checks` and require explicit justification and migration guides.

---

## Release Strategy

### v0.12.0 — Public Alpha (Current)

The first release that is ready for early adopters. Core LSP features work well: completions, diagnostics, hover, go-to-definition, references, rename, code actions, formatting, and workspace symbols. The debug adapter provides breakpoint and stepping support via a DAP bridge with a native adapter in preview.

**What "alpha" means**: The feature set is substantially complete, but APIs are still evolving. We value early adopter feedback and use it to refine the project toward stability. Expect some rough edges, especially with unusual Perl constructs.

**Exit criteria**: 90%+ CPAN top-1000 clean parse rate. Committed baselines for all corpus gates. Documentation and install story polished for first-time users.

### v0.13.0 — Refactoring Suite

Safe, reliable automated code modification. Extract method, extract variable, inline, and workspace-wide rename with boundary detection. Automated modernization of older Perl constructs to 5.38+ syntax where applicable.

### v0.14.0 — Native Debugging

A first-class debugging experience without the bridge adapter. Conditional breakpoints, logpoints, rich variable inspection for complex data structures (blessed references, tied variables), and fork-aware debugging.

### v0.15.0 — Stability Contract

The milestone where perl-lsp commits to formal API stability:

- Public APIs in published crates follow strict SemVer with contract-locked guarantees.
- LSP capabilities are frozen for reliable client integration.
- Formal deprecation cycles (N-2 release minimum) for any breaking changes.
- Guaranteed platform support tiers for Linux, macOS, and Windows.

This is the release where enterprises can depend on perl-lsp without worrying about upstream churn.

---

## Contribution Model

### Issue-Driven Workflow

All work flows through GitHub issues. Scouts discover problems and file structured issues. Builders pick up issues and submit PRs. Reviewers validate correctness, style, and scope. This pipeline works for both human contributors and agent-assisted development.

Browse issues labeled `good first issue` for entry points. These are scoped, well-documented tasks that introduce you to the codebase without requiring deep context.

### Agent-Assisted Development

This project uses AI agents as a force multiplier, not a replacement for human judgment. Agents run in isolated worktrees, one per crate, and submit draft PRs that go through the same review process as human contributions. The microcrate architecture makes this safe: agents working on `perl-lsp-completion` cannot interfere with agents working on `perl-parser`.

The swarm methodology (scouts, builders, reviewers running in parallel) has produced hundreds of merged PRs and driven the CPAN corpus from 51% to 72%+ in a matter of weeks. But every PR is reviewed. Every merge is gated. The agents accelerate; the quality standards do not change.

### How to Add a New LSP Feature

The microcrate pattern makes adding new LSP features straightforward:

1. Create a new crate under `crates/perl-lsp-<feature>/` following the structure of an existing provider (e.g., `perl-lsp-completion`, `perl-lsp-hover`).
2. Implement the LSP handler in the new crate.
3. Wire it into the server dispatch in `crates/perl-lsp/`.
4. Add the feature to `features.toml` (the canonical capability catalog).
5. Add tests and run `just ci-gate`.

Each provider crate is self-contained. You can develop and test it without understanding the full workspace.

### Getting Started

```bash
# Clone and build
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo build -p perl-lsp --release

# Run the gate
nix develop -c just ci-gate

# Quick environment check
just devex
```

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for complete guidelines, [CODE_OF_CONDUCT.md](../../CODE_OF_CONDUCT.md) for community standards.

---

## What Makes This Project Unique

### A Parser Built for Perl's Complexity

Perl is one of the hardest languages to parse. Context-sensitive operators (`/` as division or regex), heredocs that interrupt the token stream, quoting constructs with arbitrary delimiters, and formats that change the parsing grammar entirely — these are not edge cases, they are everyday Perl. The v3 recursive-descent parser handles all of them natively, without delegating to a Perl runtime for disambiguation.

### Microcrate Scale

128+ independently testable crates in a single workspace. Each crate compiles in seconds. Each has its own test suite. This is not a monolith with module boundaries drawn on a whiteboard — it is a workspace where the compiler enforces the architecture.

### Continuous Corpus Validation

Most language servers are tested against curated examples. perl-lsp is tested against thousands of real-world Perl files from CPAN and system installations, with ratcheting gates that make regressions impossible to merge. The parser does not just handle textbook Perl — it handles the Perl that people actually write.

### Built for the Long Term

The safety ratchets (no panics, no unsafe, SemVer compliance), the corpus gates (coverage can only go up), and the microcrate architecture (clear boundaries, independent evolution) are all designed for a project that will be maintained for years, not months. Every decision trades short-term convenience for long-term reliability.

---

## The Road Ahead

Perl is not going away. The codebases that run on it are too large, too critical, and too embedded in organizational infrastructure to rewrite on a whim. The developers who maintain those codebases deserve tooling that respects their time and helps them work effectively.

perl-lsp is that tooling. It is fast, it is accurate, it is open source, and it is built to last.

If you write Perl, try it. If you find a rough edge, file an issue. If you want to help make Perl development better, we would be glad to have you.

---

*See also: [ROADMAP.md](ROADMAP.md) | [CURRENT_STATUS.md](CURRENT_STATUS.md) | [CONTRIBUTING.md](../../CONTRIBUTING.md)*
