# Research Findings — work-24c7cfb5

## Issue Summary
GitHub issue #4184 reported that `nix develop` was missing optional Rust tools (cargo-llvm-cov, cargo-machete, cargo-semver-checks, git-cliff, bacon), lacked a Perl interpreter for CPAN corpus tasks, and had a version mismatch (0.9.1 vs 0.12.3). The issue affects contributors running `just ci-full`, coverage gates, semver checks, or CPAN corpus tasks via Nix.

## Relevant Codebase Areas
- `flake.nix` — primary fix target; declares devShell packages and version
- `CLAUDE.md` — declares `**Latest Release**: 0.12.4` (source of truth for version)
- `xtask/src/tasks/cpan_corpus.rs` — runs `perl` (line 476) and bootstraps cpanm from cpanmin.us
- `justfile` — references all missing tools in coverage/semver/changelog/dev-watch/cpan-corpus targets
- `rust-toolchain.toml` — MSRV 1.92.0, matches flake's rustVersion

## Key Findings
1. **Rust tools ALREADY FIXED** by PR #4261: cargo-llvm-cov, cargo-machete, cargo-semver-checks, git-cliff, bacon are all in `ciTools` (flake.nix lines 38-52); cargo-mutants is in `optionalCiTools`
2. **Version DRIFTED AGAIN**: flake.nix has `"0.12.3"` (from PR #4261) but CLAUDE.md now says `"0.12.4"` (from PR #4272 which bumped after #4261 merged). The flake is 1 patch behind.
3. **Perl STILL MISSING**: `buildInputs` (line 28) and `ciTools` (line 38) have no `perl`. The xtask bootstraps cpanm but requires `perl` to run the bootstrap script. Without `perl`, `just cpan-corpus-*` targets fail immediately.
4. **Root cause of version drift**: `crates/perl-ci-hygiene/src/version_sync.rs` does not include `flake.nix` in its collectors — this is tracked in issue #4357.

## Proposed Approach
Two targeted changes to `flake.nix`:
1. Bump `version = "0.12.3"` → `"0.12.4"` at line 205 to match CLAUDE.md
2. Add `perl` to the `buildInputs` list (line 28) so the xtask can bootstrap cpanm and run CPAN corpus tasks

Do NOT add `cpanm` itself to the flake — the xtask bootstrap handles it by downloading from https://cpanmin.us.

## Top Risks
1. **Version drift recurs** after next release — flake.nix is not in version_sync collectors (structural issue, tracked separately as #4357)
2. **Perl closure size** (~200MB+) — adds weight to the dev shell but is required for CPAN corpus workflow
3. **Darwin perl path differences** — low risk; nixpkgs `perl` should work generically

## Scope
- Covers: version bump to 0.12.4, adding `perl` to flake.nix buildInputs
- Does NOT cover: structural version_sync fix (#4357), adding cpanm to flake, changes to CI gates
