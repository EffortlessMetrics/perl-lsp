# Release Checklist for 0.12.0 Public Alpha

This checklist covers all verification steps required before cutting the 0.12.0
release. Items marked **[automated]** are enforced by `just release-check`.
Items marked **[manual]** require human verification.

## Prerequisites

```bash
# Install required tools (if not already available)
cargo install cargo-audit cargo-sbom cargo-semver-checks --locked
```

## Automated Gates

Run all automated checks at once:

```bash
just release-check
```

This composes the following gates:

### 1. CI Gate (`just ci-gate`) **[automated]**

All merge-blocking gates must pass:

- [ ] Code formatting (`cargo fmt --check --all`)
- [ ] Clippy lints -- core crates and full workspace
- [ ] Unit tests -- core crates and full workspace
- [ ] LSP smoke tests (Tier A) and core behavior tests (Tier B)
- [ ] LSP semantic definition tests
- [ ] Common corpus clean (pinned modules parse with zero errors)
- [ ] Security audit (`cargo audit`)
- [ ] Policy checks (ExitStatus helper, CURRENT_STATUS freshness, features invariants)
- [ ] Documentation build (`cargo doc`)
- [ ] v2 parity and bundle sync
- [ ] Workflow audit (no ungated expensive jobs)
- [ ] No nested lockfiles

### 2. Release Build **[automated]**

- [ ] `cargo build -p perl-lsp --release --locked` succeeds
- [ ] `cargo build -p perl-dap --release --locked` succeeds

### 3. Version Sync **[automated]**

- [ ] All version strings agree across `Cargo.toml` (workspace), `features.toml`,
      `package.json` (VSCode extension), and any `build.rs` references
- [ ] Verified by `just version-check`

### 4. SemVer Check **[automated]**

- [ ] No unintended breaking changes in public API
- [ ] Checked by `cargo-semver-checks` against the last release tag

### 5. SBOM Generation **[automated]**

- [ ] SPDX SBOM generates successfully
- [ ] CycloneDX SBOM generates successfully

### 6. No Panic Constructs **[automated]**

- [ ] No `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()` in
      production code (excluding allowed exceptions)

### 7. CHANGELOG **[automated]**

- [ ] `CHANGELOG.md` contains a `## [0.12.0]` section (not just `[Unreleased]`)

### 8. Cargo Publish Dry-Run **[automated]**

- [ ] `cargo publish --dry-run -p perl-parser` succeeds
- [ ] `cargo publish --dry-run -p perl-lsp` succeeds

## Manual Verification

These steps require human judgment and cannot be fully automated.

### 9. CPAN Corpus Coverage **[manual]**

- [ ] Common corpus manifest is up to date
- [ ] The manifest-shaped CPAN subset is the actionable release bar for 0.12.0
- [ ] Run `just cpan-corpus-sweep` against the manifest-shaped subset and verify no regressions
- [ ] If the broader 8k+ full installed sweep is red while the manifest-shaped subset stays clean, treat that as corpus-shape / baseline drift, not a silent product regression
- [ ] Track deeper parser reduction work in `#2971` if the broad sweep still needs tightening

### 10. Documentation Review **[manual]**

- [ ] README.md is current and accurate
- [ ] GETTING_STARTED.md installation instructions work
- [ ] SUPPORT.md contact information is correct
- [ ] docs/project/CURRENT_STATUS.md reflects reality (`just status-update`)
- [ ] docs/project/ROADMAP.md milestones are current

### 11. VSCode Extension **[manual]**

- [ ] Extension version matches workspace version
- [ ] Extension builds (`cd vscode-extension && npm run package`)
- [ ] Auto-download of language server binary works
- [ ] VSIX install / activation works end-to-end
- [ ] Moose / OO flow: hover, goto-definition, references, cross-file call hierarchy
- [ ] DBI flow: completion after `->`, hover docs, signature help, diagnostics
- [ ] Plain-script flow: startup, symbols, diagnostics, health widget
- [ ] Basic smoke test in VSCode: open a `.pl` file, verify diagnostics, completion, hover

### 12. Installation Paths **[manual]**

- [ ] `cargo install perl-lsp` (from local checkout) works
- [ ] `perl-lsp --version` prints correct version
- [ ] `perl-lsp --health` returns healthy status

### 13. Platform Builds **[manual, CI-assisted]**

Cross-platform builds are handled by the Release workflow, but verify:

- [ ] Linux x86_64 binary runs
- [ ] macOS arm64 binary runs (if available)
- [ ] Windows x86_64 binary runs (if available)

## Release Execution

Once all checks pass:

1. **Bump version**: `just release-turnkey 0.12.0`
   - Creates a version-bump PR with CHANGELOG updates
2. **Merge the version-bump PR** after review
3. **Trigger release**: Dispatch the Release Orchestration workflow
   - GitHub Actions: `.github/workflows/release-orchestration.yml`
   - Input: version=`0.12.0`, prerelease=`false`
4. **Monitor workflows**: Release, Publish Crates, Publish Extension, Docker
5. **Verify artifacts**:
   - GitHub Release page has binaries for all platforms
   - SHA256SUMS file attached
   - SBOM attached
   - crates.io page is live
   - VSCode Marketplace listing is updated
6. **Post-release**:
   - Update Homebrew formula (automated via `brew-bump.yml`)
   - Verify `cargo install perl-lsp` from crates.io
   - Announce release

## Automation Reference

| Recipe | What it checks |
|--------|---------------|
| `just release-check` | All automated gates (superset of `release-gate`) |
| `just release-gate` | ci-gate + release-build + sbom-verify + version-check |
| `just ci-gate` | All merge-blocking gates |
| `just ci-full` | Nightly-tier deep checks |
| `just semver-check` | SemVer breaking change detection |
| `just version-check` | Version string sync |
| `just sbom-verify` | SBOM generation and verification |
