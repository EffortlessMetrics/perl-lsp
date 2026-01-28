# CI Local Validation

*Part of Issue #211: CI Pipeline Cleanup*

This guide documents the **local-first validation philosophy** for perl-lsp development. All CI gates run locally before pushing to prevent cost overruns and ensure fast feedback loops.

---

## Overview

### Local-First Philosophy

The perl-lsp project is **local-first by design**. CI is a confirmation step, not your iteration loop. All validation gates run locally before pushing:

- **Fast feedback**: Catch issues in seconds, not minutes
- **Cost awareness**: Avoid burning GitHub Actions minutes on known failures
- **Developer productivity**: Iterate without waiting for CI
- **Deterministic builds**: Same results locally and in CI

### CI Cost Awareness

**Why validate locally?**
- GitHub Actions minutes cost money (~$0.06-0.08 per PR for essential jobs)
- Untested CI pipelines can burn hundreds of dollars in compute costs
- Flaky tests multiply costs through retries and re-runs
- Local validation is **free** and **instant**

**Budget discipline:**
- Issue #211 targets $720/year savings through CI optimization
- Pre-push validation prevents wasted CI runs
- Label-gated expensive jobs (mutation, benchmarks) are opt-in only

---

## Quick Start

### Prerequisites

**Option 1: Nix (Recommended - Deterministic, Reproducible)**

```bash
# Install Nix (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install

# Enter development shell
nix develop

# You now have:
# - Rust 1.89.0 (MSRV) with wasm32-unknown-unknown target
# - cargo, rustfmt, clippy
# - just (command runner)
# - cargo-nextest (fast test runner)
# - cargo-audit (security scanner)
# - cargo-mutants (mutation testing)
# - gh (GitHub CLI)
# - jq, python3 (for CI scripts)
```

**Option 2: Standard Rust Toolchain**

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install just command runner
cargo install just

# Install cargo-nextest (optional but recommended)
cargo install cargo-nextest

# Ensure MSRV compliance (Rust 1.89)
rustup install 1.89.0
rustup override set 1.89.0
```

---

## Nix-Based CI Workflow

### Why Nix?

Nix provides **reproducible builds** - the exact same tools and versions on every machine:

1. **Pinned toolchain**: Rust 1.89.0 (MSRV) is locked via `flake.lock`
2. **All CI tools included**: just, cargo-nextest, cargo-audit, gh, etc.
3. **Cross-platform**: Works on Linux, macOS, and WSL
4. **No system pollution**: Tools don't affect your global environment

### Available Nix Commands

```bash
# Enter development shell (primary workflow)
nix develop

# Run checks in Nix sandbox (fast, no network)
nix flake check

# Build the perl-lsp binary
nix build

# Run the LSP server
nix run

# Enter minimal CI shell (for CI runners)
nix develop .#ci
```

### Nix Flake Checks

The `nix flake check` command runs these gates in the Nix sandbox:

| Check | Description | Duration |
|-------|-------------|----------|
| `format` | `cargo fmt --check` | ~5s |
| `clippy-lib` | Clippy on libraries | ~30-60s |
| `clippy-prod-no-unwrap` | No panic-prone code | ~20-30s |
| `test-lib` | Library tests | ~1-2min |
| `wasm-check` | WASM32 compilation | ~30s |
| `policy` | ExitStatus policy | ~5s |
| `no-nested-lock` | Lockfile hygiene | ~2s |

**Note**: `nix flake check` runs in a sandbox without network access. For full CI simulation (including tests that need network), use:

```bash
nix develop -c just ci-gate
```

### Canonical Pre-Push Command

The **single source of truth** for local validation:

```bash
nix develop -c just ci-gate
```

This mirrors what CI runs and is the REQUIRED command before pushing.

### Basic Validation

**Canonical local gate (REQUIRED before push):**

```bash
# With Nix (recommended)
nix develop -c just ci-gate

# Without Nix
just ci-gate
```

This runs the **fast merge gate** (~2-5 minutes) that validates:
- Code formatting
- Clippy lints (library only)
- Core library tests
- Policy compliance
- LSP semantic definition tests
- Parser feature coverage

**Full validation (RECOMMENDED for large changes):**

```bash
# With Nix
nix develop -c just ci-full

# Without Nix
just ci-full
```

This runs the **full CI pipeline** (~10-20 minutes) including:
- All clippy lints (all targets)
- Core tests (libraries + binaries)
- LSP integration tests (thread-constrained)
- Documentation build

---

## Gate Tiers

### Tier A: Merge Gate (REQUIRED)

**Command:** `just ci-gate`
**Duration:** ~2-5 minutes
**Purpose:** Fast feedback for every commit

**What it checks:**

1. **Code formatting** (`cargo fmt --check`)
   - Ensures consistent style
   - Fast fail: runs in <5 seconds

2. **Clippy lints - Libraries** (`cargo clippy --workspace --lib`)
   - Catches common mistakes
   - Runs on library code only (faster than full check)
   - Enforces `-D warnings` (all warnings are errors)
   - Allows `-A missing_docs` during systematic resolution

3. **Library tests** (`cargo test --workspace --lib`)
   - Fast, essential tests
   - Runs in ~1-2 minutes
   - Uses `--locked` to ensure Cargo.lock consistency

4. **Production panic safety** (`clippy-prod-no-unwrap`)
   - Enforces no `.unwrap()` or `.expect()` in production code
   - Prevents panic-prone code in shipped binaries (Issue #143)
   - Only checks `--lib` and `--bins`, excludes tests
   - Uses `--no-deps` to check workspace code only

5. **Policy compliance**
   - Checks for direct `ExitStatus::from_raw()` usage
   - Validates CURRENT_STATUS.md metrics are up-to-date
   - Enforces missing docs baseline (ratcheting down)

6. **LSP semantic definition tests** (`just ci-lsp-def`)
   - Semantic-aware go-to-definition validation
   - Resource-efficient mode: `RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1`
   - Critical for LSP correctness

7. **Parser feature coverage** (`just ci-parser-features-check`)
   - Baseline enforcement for parse error count
   - Prevents parser regressions
   - Validates against corpus

**Why Tier A matters:**
- Blocks all merges to main/master
- Must pass before creating pull requests
- Pre-push hook runs this automatically

### Tier B: Release Confidence (RECOMMENDED)

**Command:** `just ci-full`
**Duration:** ~10-20 minutes
**Purpose:** Comprehensive validation before releases

**Additional checks beyond Tier A:**

1. **Full clippy** (`cargo clippy --workspace --all-targets`)
   - Includes tests, benches, examples
   - More thorough than library-only check

2. **Core tests** (`cargo test --workspace --lib --bins`)
   - Libraries + binaries
   - More comprehensive than library-only

3. **LSP integration tests** (`just ci-test-lsp`)
   - Full LSP protocol validation
   - Adaptive threading: `RUST_TEST_THREADS=2 --test-threads=2`
   - Tests workspace navigation, cross-file features
   - Validates incremental parsing (<1ms updates)

4. **Documentation build** (`just ci-docs`)
   - Ensures docs compile without errors
   - Catches broken doc links
   - Validates examples in doc comments

**When to use Tier B:**
- Large refactorings
- Parser changes affecting LSP
- Before creating release tags
- After enabling ignored tests

### Tier C: Manual Smoke Test (As Needed)

**Duration:** ~5-10 minutes
**Purpose:** Real-world editor integration verification

**Manual testing checklist:**

```bash
# 1. Build release binary
cargo build -p perl-lsp --release

# 2. Test LSP server health
./target/release/perl-lsp --version

# 3. Editor integration (choose your editor)
# - VS Code: Open a Perl file, verify:
#   - Syntax highlighting works
#   - Go-to-definition (Ctrl+Click)
#   - Hover shows documentation
#   - Completion suggestions appear
# - Neovim: Similar verification
# - Helix: Similar verification

# 4. Corpus parsing validation
cargo run -p xtask -- corpus-audit --fresh

# 5. Benchmark smoke test (optional)
cargo bench -p perl-parser --bench parse_benchmark -- --quick
```

**When to use Tier C:**
- Before releases
- After LSP protocol changes
- After parser architecture changes
- Suspected editor-specific issues

---

## Pre-Push Hook

### Installation

```bash
# Install the pre-push hook
bash scripts/install-githooks.sh
```

This creates `.git/hooks/pre-push` that runs `nix develop -c just ci-gate` (or `just ci-gate` if Nix is unavailable).

### What It Checks

The pre-push hook automatically runs Tier A (merge gate) before every push:

```
🚪 Running local gate before push: nix develop -c just ci-gate
   (Skip with: git push --no-verify)

📝 Checking code formatting...
✅ Format check passed

🔍 Running clippy (libraries only)...
✅ Clippy (lib) passed

🧪 Running library tests...
✅ Library tests passed

🔒 Enforcing no unwrap/expect in production code...
✅ Production code is panic-safe (no unwrap/expect)

📋 Running policy checks...
✅ Policy checks passed

🔎 Running LSP semantic definition tests...
✅ LSP semantic definition tests passed

🔍 Checking parser features baseline...
✅ Parser features baseline maintained

✅ Merge gate passed!
```

### Bypassing the Hook

**When to bypass:**
- Emergency hotfixes (with caution)
- WIP branches that intentionally break tests
- Documentation-only changes (though hook is fast enough to run anyway)

**How to bypass:**

```bash
# Skip pre-push hook
git push --no-verify

# Or use the environment variable
SKIP_PREPUSH=1 git push
```

**Warning:** Only bypass if you understand the risks. Broken commits on main/master affect everyone.

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
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
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
cargo test -p perl-lsp

# Adaptive threading (recommended)
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
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

**Problem:** First build takes a long time as Nix builds everything from source.

**Solution:** This is expected for first build. Subsequent builds are cached:

```bash
# First build: ~10-20 minutes
nix develop -c just ci-gate

# Subsequent builds: ~2-5 minutes (cached)
nix develop -c just ci-gate
```

To speed up development, use `just ci-gate` outside Nix shell for iteration.

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

Validate against Minimum Supported Rust Version (1.89.0):

```bash
# Fast merge gate on MSRV
just ci-gate-msrv

# Full CI on MSRV
just ci-full-msrv

# Or manually
RUSTUP_TOOLCHAIN=1.89.0 just ci-gate
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

```
┌─────────────────────────────────────────┐
│  Developer Workstation (Local-First)    │
└────────────┬────────────────────────────┘
             │
             ├─► just ci-gate (~2-5 min)
             │   ├─ Format check
             │   ├─ Clippy (lib)
             │   ├─ Library tests
             │   ├─ Panic safety check
             │   ├─ Policy checks
             │   ├─ LSP semantic tests
             │   └─ Parser features check
             │
             ├─► just ci-full (~10-20 min)
             │   ├─ All ci-gate checks
             │   ├─ Full clippy
             │   ├─ Core tests (lib + bins)
             │   ├─ LSP integration tests
             │   └─ Documentation build
             │
             └─► git push
                 ├─ Pre-push hook runs ci-gate
                 └─ GitHub Actions (confirmation)
                     ├─ Default lane (every PR)
                     └─ Label-gated lanes (opt-in)
```

### Test Lanes

| Lane | Trigger | Cost | Purpose |
|------|---------|------|---------|
| **Core** | Every PR | Low | Format, clippy, essential tests |
| **LSP** | Code changes | Medium | LSP integration tests |
| **Stress** | `ci:stress` label | High | Long-running stability tests |
| **Extras** | `ci:extras` label | Medium | Optional LSP features |
| **Mutation** | `ci:mutation` label | Very High | Mutation testing (~15-30 min) |
| **Benchmark** | `ci:bench` label | High | Performance benchmarks |
| **Coverage** | `ci:coverage` label | High | Code coverage analysis |

### Concurrency Controls

All workflows include concurrency cancellation to prevent wasted CI runs:

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

**Impact:** Push 5 times in a row, only pay for the last run.

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
   - MSRV 1.89.0 is specified in `flake.nix`
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

```bash
# Recommended workflow
1. Make changes
2. Run: just ci-full
3. Run manual smoke test (Tier C)
4. If passing, commit and push
5. Add ci:stress label for stress tests
6. Monitor GitHub Actions for cross-platform issues
```

### Before Releases

```bash
# Release validation
1. Run: just ci-full
2. Run: just ci-full-msrv (validate MSRV)
3. Run manual smoke test with editors
4. Run: cargo run -p xtask -- corpus-audit --fresh
5. Check: bash scripts/ignored-test-count.sh
6. Verify: just status-check
7. Tag release
```

### Cost-Conscious Development

```bash
# Minimize CI costs
1. Always run ci-gate locally before pushing
2. Use pre-push hook (automatic)
3. Only add expensive labels when needed
4. Iterate locally, confirm on GitHub Actions
5. Use concurrency cancellation (automatic)
6. Skip CI on docs-only PRs (automatic)
```

---

## Performance Benchmarks

### Typical Local Gate Times

| Stage | Duration | Notes |
|-------|----------|-------|
| Format check | <5 seconds | Fast fail |
| Clippy (lib) | ~30-60 seconds | Cached after first run |
| Library tests | ~1-2 minutes | Fast, essential |
| Panic safety | ~20-30 seconds | Production code only |
| Policy checks | ~5-10 seconds | Lightweight |
| LSP semantic | ~30-60 seconds | Resource-constrained |
| Parser features | ~10-20 seconds | Baseline validation |
| **Total** | **~2-5 minutes** | Full merge gate |

### Typical Full Pipeline Times

| Stage | Duration | Notes |
|-------|----------|-------|
| ci-gate | ~2-5 minutes | See above |
| Full clippy | ~1-2 minutes | All targets |
| Core tests | ~2-3 minutes | Lib + bins |
| LSP integration | ~2-4 minutes | Thread-constrained |
| Documentation | ~1-2 minutes | No deps |
| **Total** | **~10-20 minutes** | Full CI pipeline |

### CI Cost Estimates

Based on GitHub Actions pricing (as of 2025):

```
Essential jobs per PR (Ubuntu + Windows):
- Format check:     ~$0.01
- Clippy:           ~$0.02
- Core tests:       ~$0.03
- LSP tests:        ~$0.02
─────────────────────────────
Total:              ~$0.06-0.08 per PR

With concurrency cancellation:
- 5 pushes = 1 billable run = $0.06-0.08
- Without cancellation: $0.30-0.40 (5x more!)

Annual savings (Issue #211 target):
- CI optimization: $720/year
```

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

# Correct (uses Nix Rust 1.89.0):
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
- **[CLAUDE.md](../CLAUDE.md)** - Project guidance (includes local workflow)
- **[COMMANDS_REFERENCE.md](COMMANDS_REFERENCE.md)** - Full command catalog
- **[COMPREHENSIVE_TESTING_GUIDE.md](COMPREHENSIVE_TESTING_GUIDE.md)** - Testing framework
- **[THREADING_CONFIGURATION_GUIDE.md](THREADING_CONFIGURATION_GUIDE.md)** - Thread safety

---

**Last Updated:** 2026-01-24
**Issue:** #211 (CI Pipeline Cleanup)
**Status:** Phase 3 - Nix-Based CI Infrastructure
