# CI Local Validation

This guide is the local companion to [CI.md](./CI.md) and
[CI_TEST_LANES.md](./CI_TEST_LANES.md). It keeps the contributor flow aligned
with the actual gate order used in the repo.

## Canonical Flow

Run the commands in this order:

```bash
just devex
just pr-fast
nix develop -c just ci-gate
just ci-full
just status-update
just status-check
just release-check
```

If you do not use Nix, run the same `just ...` commands directly. The Nix
shell is only there to make the toolchain and external helpers reproducible.

## What Each Step Means

- `just devex` checks the local environment and highlights missing tools.
- `just pr-fast` is the quick edit loop for normal PR iteration.
- `nix develop -c just ci-gate` is the canonical full local merge gate.
- `just ci-full` is the broader validation pass for large refactors or release
  confidence.
- `just status-update` and `just status-check` keep generated status docs in
  sync.
- `just release-check` is the release-prep gate before tagging or publishing.

If a change is security-sensitive, also run `just security-audit`. If it touches
MSRV-sensitive code, use `just ci-gate-msrv` or `just ci-full-msrv`.

## Nix and Non-Nix Usage

Nix is recommended because it gives everyone the same toolchain and helper
commands. The plain `just` entry points stay available for contributors who
already have the Rust toolchain and project tools installed locally.

`nix flake check` is useful for a quick sandbox sanity pass, but it is not a
replacement for `nix develop -c just ci-gate`.

---

## Gate Tiers

### Tier A: PR-Fast / Push Guard

- **Command:** `just pr-fast`
- **Use when:** during normal iteration and before every push (the pre-push hook runs this tier)
- **Checks:** fast formatting/lint/test coverage meant to catch obvious regressions quickly

### Tier B: Merge Gate

- **Command:** `just ci-gate`
- **Use when:** before merge, or any time you need the full local merge receipt
- **Checks:** formatting, library Clippy, library tests, policy checks, LSP semantic definition tests, parser feature checks, and the hooks that CI relies on

### Tier C: Release Confidence

- **Command:** `just ci-full`
- **Use when:** the change is broad, touches several crates, or needs release-level confidence
- **Checks:** everything from Tier B, plus the broader Clippy/test/doc coverage used by the full local pipeline

### Tier D: Manual Smoke Test

- **Command:** build and run the release binary, then verify it in the editor you care about
- **Use when:** editor behavior changed, parser behavior changed, or you are validating release readiness
- **Typical checks:** build `perl-lsp`, confirm the binary starts, exercise hover/completion/definition, and run a corpus smoke test if the change touches parsing or indexing

---

## Pre-Push Hook

### Installation

```bash
# Install the pre-push hook
bash scripts/install-githooks.sh
```

The hook runs `nix develop -c just pr-fast` when Nix is available, or
`just pr-fast` otherwise. Use `git push --no-verify` only if you intentionally
need to bypass the fast local push guard.

---

## Troubleshooting

### Common Issues

#### Issue: `just: command not found`

**Problem:** The `just` command runner is not installed.

**Solution:**

```bash
# Install just
cargo install just

# Or via package manager
# macOS:
brew install just

# Arch Linux:
pacman -S just

# Ubuntu/Debian:
snap install --edge --classic just
```

#### Issue: `nix: command not found`

**Problem:** Nix is not installed, but you're trying to run `nix develop`.

**Solution 1:** Install Nix (recommended):

```bash
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install
```

**Solution 2:** Run without Nix:

```bash
just ci-gate  # Works without Nix if Rust toolchain is installed
```

#### Issue: `error: failed to run custom build command for perl-lsp`

**Problem:** Missing system dependencies (usually OpenSSL).

**Solution:**

```bash
# macOS:
brew install openssl pkg-config

# Ubuntu/Debian:
sudo apt-get install libssl-dev pkg-config

# Fedora:
sudo dnf install openssl-devel pkg-config

# Arch:
sudo pacman -S openssl pkg-config
```

#### Issue: Tests fail with "Address already in use"

**Problem:** LSP tests try to bind to the same port simultaneously.

**Solution:** This is why we use `RUST_TEST_THREADS=2`:

```bash
# LSP tests are already thread-constrained in justfile
just ci-test-lsp

# If running manually:
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2
```

#### Issue: `error: could not find Cargo.toml`

**Problem:** Running commands from wrong directory.

**Solution:**

```bash
# Always run from repository root
cd /path/to/perl-lsp
just ci-gate
```

#### Issue: Nested `Cargo.lock` detected

**Problem:** Running cargo commands from subdirectory created nested lockfile.

**Solution:**

```bash
# Remove nested lockfiles
find . -name 'Cargo.lock' -not -path './Cargo.lock' -delete

# Always run from repo root
cd /path/to/perl-lsp
just ci-gate
```

The merge gate includes `ci-check-no-nested-lock` to catch this automatically.

### Threading Configuration

LSP tests use **adaptive threading** to prevent resource exhaustion:

```bash
# Standard threading (may fail on CI runners)
cargo test -p perl-lsp-rs

# Adaptive threading (recommended)
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2
```

**Environment variables:**

| Variable | Value | Purpose |
|----------|-------|---------|
| `RUST_TEST_THREADS` | `1` | Semantic definition tests (memory-intensive) |
| `RUST_TEST_THREADS` | `2` | LSP integration tests (adaptive) |
| `CARGO_BUILD_JOBS` | `1` | Semantic definition build (reduces memory) |
| `RUSTC_WRAPPER` | `""` | Disable rustc wrapper for semantic tests |

These are already configured in `justfile` recipes.

### Nix Shell Issues

#### Issue: Nix flake evaluation fails

**Problem:** `nix flake check` fails due to sandbox blocking network access.

**Solution:** Use `nix develop -c just ci-gate` instead:

```bash
# DON'T use this (sandbox blocks Cargo network access)
nix flake check

# DO use this (runs commands in shell with network access)
nix develop -c just ci-gate
```

#### Issue: Nix builds are slow

**Problem:** The first Nix run is slower because it populates the store.

**Solution:** Reuse the same dev shell and run `just ci-gate` directly for
faster iteration once your local toolchain is set up.

---

## Advanced Usage

### Running Individual Gates

```bash
# Just format check
just ci-format

# Just clippy (libraries only)
just ci-clippy-lib

# Just clippy (all targets)
just ci-clippy

# Just library tests
just ci-test-lib

# Just LSP tests
just ci-test-lsp

# Just LSP semantic tests
just ci-lsp-def

# Just policy checks
just ci-policy

# Just panic safety check
just clippy-prod-no-unwrap

# Just parser features check
just ci-parser-features-check
```

### MSRV Validation

Validate against Minimum Supported Rust Version (1.92.0):

```bash
# Fast merge gate on MSRV
just ci-gate-msrv

# Full CI on MSRV
just ci-full-msrv

# Or manually
RUSTUP_TOOLCHAIN=1.92.0 just ci-gate
```

### Cost Estimation

Estimate GitHub Actions costs locally:

```bash
# Run full local pipeline and time it
time just ci-full

# GitHub Actions runner costs:
# - Ubuntu: ~$0.008/minute
# - Windows: ~$0.016/minute
# - macOS: ~$0.08/minute (10x more expensive!)

# Example: 10-minute CI run
# - Ubuntu: $0.08 per PR
# - Windows: $0.16 per PR
# Total: ~$0.24 per PR for essential jobs
```

### Checking Test Count Baseline

```bash
# Show ignored test breakdown by category
bash scripts/ignored-test-count.sh

# Expected output categories:
# - BUG: Parser bugs (target: 0)
# - IMPLEMENTATION: Features not yet implemented
# - FEATURE: Feature-gated tests (stress, extras)
# - MANUAL: Requires human intervention
# - SKIP: Tests to skip (performance, known issues)
```

### Health Metrics

```bash
# Quick health overview
just health

# Detailed file-by-file breakdown
just health-detail

# Status verification (CURRENT_STATUS.md consistency)
just status-check

# Update computed metrics
just status-update
```

### Workflow Audit

```bash
# Audit workflows for ungated expensive jobs
just ci-workflow-audit

# This checks for:
# - Missing concurrency cancellation
# - Missing path-ignore filters
# - Expensive jobs without label gates
# - Redundant test executions
```

---

## CI Pipeline Architecture

### Gate Flow

The practical order is:

1. `just pr-fast` while iterating.
2. Let the pre-push hook re-run `just pr-fast` on push, or run it manually first.
3. `nix develop -c just ci-gate` before merge.
4. `just ci-full` for large refactors or release confidence.
5. `just release-check` before tagging a release.

### Test Lanes

| Lane | Local command | Notes |
|------|---------------|-------|
| PR smoke | `just pr-fast` | Fast feedback for every PR |
| Merge gate | `nix develop -c just ci-gate` | Local equivalent of the merge gate |
| Full validation | `just ci-full` | Broader workspace confidence |
| Security | `just security-audit` | Use when security-sensitive changes land |

### Path Filters

Most workflows skip on documentation-only changes:

```yaml
paths-ignore:
  - 'docs/**'
  - '**/*.md'
  - '.claude/**'
```

**Impact:** Documentation updates don't burn CI minutes.

---

## Nix Configuration Architecture

### flake.nix Structure

The Nix flake provides reproducible development environments and CI checks:

```
flake.nix
├── inputs
│   ├── nixpkgs (pinned via flake.lock)
│   ├── rust-overlay (Rust toolchain management)
│   └── flake-utils (cross-platform helpers)
│
├── devShells
│   ├── default      # Full dev environment with all tools
│   └── ci           # Minimal CI shell (no optional tools)
│
├── checks           # Run via: nix flake check
│   ├── format               # cargo fmt --check
│   ├── clippy-lib           # Clippy on libraries
│   ├── clippy-prod-no-unwrap # No unwrap/expect in production
│   ├── test-lib             # Library tests
│   ├── wasm-check           # WASM32 compilation
│   ├── policy               # ExitStatus policy
│   └── no-nested-lock       # Lockfile hygiene
│
├── packages
│   └── perl-lsp     # Built LSP server binary
│
└── apps
    ├── default      # Run perl-lsp
    └── ci-simulate  # Run CI simulation
```

### Reproducibility Guarantees

1. **Rust Version Pinning**
   - MSRV 1.92.0 is specified in `flake.nix`
   - Also enforced via `rust-toolchain.toml`
   - CI workflows use the same version

2. **Dependency Pinning**
   - `flake.lock` pins nixpkgs and rust-overlay
   - `Cargo.lock` pins Rust dependencies
   - Together, these ensure identical builds

3. **Tool Versions**
   - All CI tools come from pinned nixpkgs
   - No system-installed tools are used
   - Same versions on Linux, macOS, WSL

### Updating Pinned Versions

```bash
# Update all flake inputs to latest
nix flake update

# Update only nixpkgs
nix flake update nixpkgs

# Update only rust-overlay
nix flake update rust-overlay

# After updating, run checks
nix flake check
nix develop -c just ci-gate
```

### Platform-Specific Considerations

| Platform | Notes |
|----------|-------|
| Linux | Full support, all features |
| macOS | Requires Darwin frameworks for OpenSSL |
| WSL | Use `nix develop` not native Windows |
| Windows | Not supported (use WSL) |

---

## Best Practices

### Daily Development

```bash
# Standard workflow
1. Make changes
2. Run: just ci-gate
3. If passing, commit and push
4. Pre-push hook validates again
5. GitHub Actions confirms
```

### Large Refactorings

- Use `just ci-full` and `just release-check` before you ask for review.
- If the change touches parsing or indexing, add a manual editor smoke test.
- If the change touches release packaging, also run the MSRV variants.

---

## Nix Troubleshooting

### Common Nix Issues

#### Issue: `nix flake check` fails with network errors

**Problem:** Nix sandbox blocks network access during checks.

**Solution:** Use the dev shell instead:

```bash
# DON'T use (sandbox blocks Cargo network):
nix flake check

# DO use (shell has network access):
nix develop -c just ci-gate
```

The `nix flake check` command is best for quick syntax validation. For full CI simulation, always use `nix develop -c just ci-gate`.

#### Issue: `error: experimental Nix feature 'flakes' is disabled`

**Problem:** Flakes are not enabled in your Nix configuration.

**Solution:**

```bash
# Option 1: Add to ~/.config/nix/nix.conf
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf

# Option 2: Use --experimental-features flag
nix --experimental-features 'nix-command flakes' develop
```

#### Issue: Rust version mismatch

**Problem:** Local rustc differs from Nix-provided version.

**Solution:** Always run commands inside `nix develop`:

```bash
# Wrong (uses system Rust):
just ci-gate

# Correct (uses Nix Rust 1.92.0):
nix develop -c just ci-gate
```

#### Issue: First `nix develop` is very slow

**Problem:** Nix is downloading and building dependencies from scratch.

**Solution:** This is expected for the first run. Subsequent runs use the cache:

```bash
# First run: ~5-15 minutes (downloads everything)
nix develop

# Subsequent runs: ~1-5 seconds (cached)
nix develop
```

To speed up initial setup, use the Determinate Systems installer which enables binary caches:

```bash
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install
```

#### Issue: `cargo-mutants` takes too long

**Problem:** Mutation testing is computationally expensive.

**Solution:** Only run mutation tests when needed (use CI label):

```bash
# Local quick test (skip mutation):
nix develop -c just ci-gate

# Mutation testing (only when reviewing test quality):
nix develop -c cargo mutants -p perl-parser --timeout 60
```

---

## Related Documentation

- **[CI.md](CI.md)** - GitHub Actions workflow architecture
- **[CI_TEST_LANES.md](CI_TEST_LANES.md)** - Test lane organization
- **[CLAUDE.md](../../CLAUDE.md)** - Project guidance (includes local workflow)
- **[COMMANDS_REFERENCE.md](../reference/COMMANDS_REFERENCE.md)** - Full command catalog
- **[COMPREHENSIVE_TESTING_GUIDE.md](../tutorials/COMPREHENSIVE_TESTING_GUIDE.md)** - Testing framework
- **[THREADING_CONFIGURATION_GUIDE.md](../how-to/THREADING_CONFIGURATION_GUIDE.md)** - Thread safety

---

**Last Updated:** 2026-03-29
**Status:** Local validation and CI command flow aligned with the current gate model
