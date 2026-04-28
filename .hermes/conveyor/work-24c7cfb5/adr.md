# ADR-2024-04-17-001: Fix Nix Flake Version Drift and Add Perl to Dev Shell

## Status

Accepted

## Context

GitHub issue #4184 (`nix flake: missing optional tools and version mismatch`) identified two problems with the `flake.nix` development environment:

1. **Version mismatch**: The `packages.perl-lsp` derivation declared `version = "0.12.3"` while `CLAUDE.md` line 3 declared `**Latest Release**: 0.12.4`. This drift was introduced because PR #4261 bumped the flake to 0.12.3, then PR #4272 bumped Cargo.toml/CLAUDE.md to 0.12.4 without updating the flake.

2. **Missing Perl**: The `flake.nix` `buildInputs` did not include `perl`, causing `just cpan-corpus-*` targets to fail immediately since `xtask/src/tasks/cpan_corpus.rs:476` calls `Command::new("perl")` directly. The cpanm binary is bootstrapped from `https://cpanmin.us` by the xtask — only the Perl runtime is needed.

The `flake.nix` is the canonical local development environment for this project (used by `nix develop -c just ci-gate` in CI on Linux). Any gap in the flake directly degrades contributor experience.

## Decision

We will make two targeted changes to `flake.nix`:

1. **Bump the version** in `packages.perl-lsp` (line 205) from `"0.12.3"` to `"0.12.4"` to match `CLAUDE.md`.

2. **Add `perl` to the shared `buildInputs`** (line 28). This makes Perl available in both `devShells.default` and `devShells.ci` without duplicating it. We do NOT add `cpanm` — the xtask bootstraps it from cpanmin.us.

A comment will be added to line 205 noting the manual sync requirement and referencing issue #4357 (the structural version_sync fix).

## Consequences

### Benefits

- `just cpan-corpus-*` targets now work in `nix develop` shells, enabling CPAN corpus testing for contributors using Nix
- Version metadata in the flake matches the declared latest release
- The two changes are mechanically simple, low-risk, and independently reversible

### Trade-offs / Risks

- **Perl closure size**: Adding `perl` increases the Nix closure by ~200MB. This is a one-time download cost per machine, not per build. The `cpan-corpus-*` targets are opt-in dev tools, not CI gates, so the ongoing CI impact is zero.
- **Version drift will recur**: The `version_sync.rs` collectors do not include `flake.nix`. Without the structural fix (tracked in issue #4357), the flake will drift again after the next release. This ADR addresses the immediate drift only.
- **Darwin Perl path**: nixpkgs' `perl` may behave differently on macOS. The flake already handles Darwin conditionally for other packages, establishing precedent. This risk is low but should be verified on Darwin before merging.

## Alternatives Considered

### Alternative 1: Document that Perl is an external requirement
- **Decision**: Rejected. The issue explicitly asks for Perl to be available in the flake. The project uses `nix develop` as the canonical dev environment — requiring contributors to install Perl externally undermines that goal.
- **Trade-off**: Adds ~200MB to the closure. Acceptable for an opt-in dev tool.

### Alternative 2: Add `cpanm` directly to the flake instead of `perl`
- **Decision**: Rejected. The xtask already bootstraps cpanm from cpanmin.us. Adding cpanm to the flake would be redundant and would not solve the actual problem (the xtask still calls `perl` to execute the bootstrap script).

### Alternative 3: Gate Perl behind `lib.optionals stdenv.isLinux`
- **Decision**: Deferred. If Darwin testing reveals path issues, gating is a viable mitigation. For now, we add Perl unconditionally and adjust if Darwin problems emerge.

## Notes

- Rust tools (cargo-llvm-cov, cargo-machete, cargo-semver-checks, git-cliff, bacon, cargo-mutants) were already added by PR #4261 — they are out of scope for this ADR.
- The structural fix for version drift (adding `flake.nix` to `version_sync::collect_sites()`) is tracked in issue #4357 and is out of scope for this work item.