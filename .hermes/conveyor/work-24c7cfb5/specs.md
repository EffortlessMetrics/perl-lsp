# Specifications — work-24c7cfb5

## Feature/Behavior Description

Fix two deficiencies in `flake.nix` that prevent full contributor parity with the standard development environment:

1. **Version metadata sync**: The `packages.perl-lsp` derivation's `version` field must match the latest release declared in `CLAUDE.md`.
2. **Perl runtime availability**: The `devShells.default` and `devShells.ci` shells must include `perl` so that `just cpan-corpus-*` targets can bootstrap cpanm and execute.

## Acceptance Criteria

### AC-1: Version field matches CLAUDE.md

- **Given** the current latest release is `0.12.4` as declared in `CLAUDE.md` line 3
- **When** a contributor runs `grep 'version = ' flake.nix` or inspects the `packages.perl-lsp` derivation
- **Then** the version field shows `"0.12.4"` (not `"0.12.3"`)

### AC-2: Perl is available in nix develop shells

- **Given** a contributor runs `nix develop -c perl --version`
- **Then** the command succeeds and prints the Perl version
- **And** `Command::new("perl")` in the xtask can execute without "command not found"

### AC-3: Nix flake evaluates without errors

- **Given** `nix flake check --no-build` is run
- **Then** the flake evaluates successfully with no Nix errors
- **And** all sandboxed checks (format, clippy-lib, no-unwrap, test-lib, wasm, policy, no-nested-lock) continue to pass

### AC-4: cpanm bootstrap works

- **Given** a contributor runs `just cpan-corpus-install` inside `nix develop`
- **Then** the xtask can bootstrap cpanm from `https://cpanmin.us` using `curl` and execute it with `perl`
- **And** the command does not fail with "perl: command not found"

## Non-Goals

- This spec does NOT add `cpanm` to the flake — the xtask bootstraps it
- This spec does NOT add Perl to the sandboxed `checks` derivations (CPAN corpus is an opt-in dev workflow, not a CI gate)
- This spec does NOT fix the structural version_sync gap (tracked in issue #4357)
- This spec does NOT modify any Cargo.toml or workspace version files

## Dependencies

- `nixpkgs-unstable` provides the `perl` package
- `xtask/src/tasks/cpan_corpus.rs` provides the bootstrap logic for cpanm (no changes to xtask are made by this spec)
- The `just cpan-corpus-*` targets in the justfile must not be broken by this change (they should continue to work as before)