# Initial Plan — work-24c7cfb5

## Approach

Two targeted, low-risk changes to `flake.nix`:

### Change 1: Version bump (line 205)

Update `version = "0.12.3"` → `version = "0.12.4"` in the `packages.perl-lsp` derivation to match CLAUDE.md's declared latest release.

**Why**: CLAUDE.md line 3 says `**Latest Release**: 0.12.4`. The flake is stale at 0.12.3. This is purely a metadata fix.

### Change 2: Add `perl` to devShell buildInputs (lines 28–35)

Add `perl` to the `buildInputs` list (the `with pkgs; [...]` block). This makes `perl` available in both `devShells.default` and `devShells.ci` since they both inherit from `buildInputs`.

**Why**: The xtask cpan_corpus task (line 476 of `xtask/src/tasks/cpan_corpus.rs`) calls `Command::new("perl")` directly. Without perl in PATH, `just cpan-corpus-*` targets fail immediately. Adding perl allows the cpanm bootstrap (downloaded from cpanmin.us) to execute. We do NOT need to add `cpanm` itself to the flake — the xtask already bootstraps it.

**NOTE on cpanm vs perl**: The issue title mentions both "perl" and "cpanm". The correct fix is to add `perl` only. The xtask bootstrap handles cpanm by downloading from https://cpanmin.us — it just needs perl to run that bootstrap script.

---

## Risks

### Risk 1: Perl closure size bloat
- **Severity**: Medium
- **Description**: Adding `perl` to `buildInputs` increases the Nix closure by ~200MB+.
- **Mitigation**: `perl` is a standard Nixpkgs package and is already needed for the CPAN corpus workflow. The lean alternative (Option B from issue #4184: document external Perl requirement) was weighed — but the explicit ask in the issue is to add it to the flake, and the xtask already has robust bootstrap logic that requires it. The closure cost is a one-time download, not per-build.

### Risk 2: macOS-specific Perl path/naming
- **Severity**: Low
- **Description**: nixpkgs `perl` may behave differently on Darwin (e.g., Perl framework paths). The xtask calls `perl` generically — if nixpkgs provides a `perl` binary that works, it should be fine.
- **Mitigation**: The flake already handles Darwin conditionally for other packages (Security, SystemConfiguration frameworks). If `perl` has issues on Darwin, it can be gated with `lib.optionals stdenv.isLinux`.

### Risk 3: Version drift recurs after next release
- **Severity**: Medium (structural)
- **Description**: Without adding `flake.nix` to the version_sync collectors, the version will drift again after the next release.
- **Mitigation**: This is tracked separately in issue #4357. This work item fixes the immediate drift; the structural fix is a follow-up.

---

## Task Breakdown

1. **Verify current state**: Confirm `grep -n "version = " flake.nix"` shows `"0.12.3"` at line 205 and no `perl` in `buildInputs`
2. **Update version**: Patch `flake.nix` line 205: `"0.12.3"` → `"0.12.4"`
3. **Add perl to buildInputs**: Patch `flake.nix` lines 28–35 to include `perl` in the buildInputs list
4. **Verify Nix evaluates**: Run `nix flake check --no-build` or `nix develop . --ignore-environment -c echo "flake ok"` to confirm the flake still evaluates
5. **Verify the change**: Confirm `nix develop -c perl --version` would work (no network needed for this check)
6. **Create a verification commit** on the branch

---

## What This Plan Does NOT Cover

- Adding `cpanm` to the flake (the xtask bootstraps it — no need)
- Adding `perl` to the `checks` derivations (CPAN corpus is not part of the Nix sandbox CI gates)
- Fixing structural version_sync (issue #4357 — separate work item)
- Any Cargo.toml / workspace version changes (those are updated by release automation)
