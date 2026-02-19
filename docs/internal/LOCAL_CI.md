# Running CI Locally

This project follows a **local-first** development philosophy. All CI gates run locally before pushing, and GitHub Actions serves only as confirmation.

---

## Quick Start

**Am I ready to push?**

```bash
nix develop -c just ci-gate
```

That's it. If this passes, you're good to push.

---

## Prerequisites

### Option 1: Nix (Recommended)

Nix provides a fully reproducible environment with all tools pre-configured.

```bash
# Install Nix with flakes enabled
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install

# Enter the development shell
nix develop

# You now have: Rust 1.92.0, just, cargo-nextest, cargo-audit, gh, jq, python3
```

### Option 2: Manual Installation

If you prefer not to use Nix:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install just (command runner)
cargo install just

# Pin to MSRV
rustup install 1.92.0
rustup override set 1.92.0

# Optional but recommended
cargo install cargo-nextest cargo-audit
```

**System dependencies (for OpenSSL):**

```bash
# macOS
brew install openssl pkg-config

# Ubuntu/Debian
sudo apt-get install libssl-dev pkg-config

# Fedora
sudo dnf install openssl-devel pkg-config

# Arch
sudo pacman -S openssl pkg-config
```

### Editor Setup (Optional)

For the best experience, configure your editor to use rust-analyzer with the workspace's rust-toolchain.toml. The project MSRV is Rust 1.92.0.

---

## Understanding the Tiers

The CI system has three tiers, each building on the previous:

### PR-Fast (~1-2 minutes)

Quick validation for every code iteration.

```bash
just pr-fast
```

**What it checks:**
- Code formatting (`cargo fmt --check`)
- Clippy on core crates (perl-parser, perl-lexer)
- Core crate tests

**When to use:** While actively developing, for quick feedback.

### Merge-Gate (~2-5 minutes)

Required before every push to master.

```bash
just ci-gate
# or with Nix:
nix develop -c just ci-gate
```

**What it checks:**
- Everything in pr-fast, plus:
- Full workspace clippy
- Full workspace tests
- LSP smoke tests
- Security audit
- Policy checks (no unwrap/expect in production)
- LSP semantic definition tests
- Parser feature coverage baseline
- Features.toml invariants

**When to use:** Before pushing. The pre-push hook runs this automatically.

### Nightly (~15-30 minutes)

Comprehensive testing, typically run on schedule.

```bash
just nightly
```

**What it checks:**
- Everything in merge-gate, plus:
- Mutation testing subset
- Bounded fuzz testing (placeholder)
- Benchmarks

**When to use:** Before releases, after major refactorings, or when investigating subtle issues.

---

## Common Workflows

### "I'm iterating on code"

Quick feedback loop while developing:

```bash
just pr-fast  # ~1-2 minutes
```

Or run specific checks:

```bash
just fmt-check      # Format only
just clippy-core    # Clippy on core crates only
just test-core      # Tests on core crates only
```

### "I'm ready to push"

Run the full merge gate:

```bash
just ci-gate        # Without Nix
nix develop -c just ci-gate  # With Nix (recommended)
```

### "I want to run specific checks"

Individual gate targets are available:

```bash
# Formatting
just fmt-check          # Check formatting
just fmt                # Fix formatting

# Linting
just clippy-core        # Core crates only (fast)
just clippy-full        # Full workspace
just clippy-prod-no-unwrap  # No unwrap/expect in production

# Testing
just test-core          # Core crate tests
just test-full          # Full workspace tests
just lsp-smoke          # LSP smoke tests
just ci-lsp-def         # LSP semantic definition tests
just ci-test-lsp        # LSP integration tests

# Security
just security-audit     # cargo-audit scan

# Policy
just ci-policy          # All policy checks
just ci-docs-check      # Missing docs baseline
just ci-parser-features-check  # Parser feature baseline
just ci-features-invariants    # features.toml validation
```

### "I want to see what commands will run"

The justfile is your source of truth. View available commands:

```bash
just --list            # List all available recipes
just --show ci-gate    # Show what ci-gate does
```

### "I'm on a low-memory system (WSL, limited RAM)"

Use the low-memory gate variant:

```bash
just ci-gate-low-mem   # Forces sequential, single-threaded builds
```

This uses `CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1`, and unsets `RUSTC_WRAPPER`.

### "I want to validate MSRV compliance"

Ensure your code works on the Minimum Supported Rust Version (1.92.0):

```bash
just ci-gate-msrv      # Fast gate on MSRV
just ci-full-msrv      # Full CI on MSRV
```

---

## Troubleshooting

### Common Failures and Fixes

#### Format check fails

```bash
# Fix formatting automatically
just fmt
```

#### Clippy fails with warnings

Read the warnings, fix the code. Common issues:
- Use `.first()` instead of `.get(0)`
- Use `.push(char)` instead of `.push_str("x")` for single chars
- Use `or_default()` instead of `or_insert_with(Vec::new)`
- Avoid unnecessary `.clone()` on Copy types

#### "No unwrap/expect" check fails

Production code cannot use `.unwrap()` or `.expect()`. Options:
- Use `?` with proper error propagation
- Use `.ok_or_else()` to convert to Result
- Pattern match on Option/Result
- In tests: add `#[allow(clippy::unwrap_used)]` to the test module

#### Tests fail with "Address already in use"

LSP tests may conflict. Use thread constraints:

```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
```

The justfile recipes already handle this.

#### Out of memory / build killed

Use the low-memory variant:

```bash
just ci-gate-low-mem
```

Or set constraints manually:

```bash
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 just ci-gate
```

#### Nested Cargo.lock detected

You ran cargo from a subdirectory. Fix:

```bash
find . -name 'Cargo.lock' -not -path './Cargo.lock' -delete
cd /path/to/repo/root
just ci-gate
```

### "CI passed locally but failed in GitHub"

This usually means:

1. **Environment differences**: Use `nix develop -c just ci-gate` for closest parity
2. **Platform-specific issues**: Local Linux vs CI Ubuntu, or Windows runners
3. **Timing/race conditions**: LSP tests use thread constraints for stability
4. **Cache issues**: CI starts fresh. Try `cargo clean && just ci-gate` locally

To reproduce the exact CI environment:

```bash
# Use Nix for reproducibility
nix develop -c just ci-gate

# Or simulate CI threading limits
RUST_TEST_THREADS=2 CARGO_BUILD_JOBS=2 just ci-gate
```

---

## Receipts and Debugging

### Generating Receipts

Receipts capture detailed information about gate runs:

```bash
# Run gates with receipt output
just gates
```

This creates:
- `target/receipts/receipt.json` - Summary of all gates
- `target/receipts/logs/*.log` - Individual gate logs
- `target/receipts/artifacts/*` - Test artifacts

### Reading Receipts

```bash
# View receipt summary
cat target/receipts/receipt.json | jq .

# Check a specific gate's log
cat target/receipts/logs/ci-gate.log
```

Receipt JSON structure:

```json
{
  "schema": 1,
  "generated_at": "2025-01-24T10:00:00Z",
  "commit": "abc123...",
  "rustc": "rustc 1.92.0",
  "cargo": "cargo 1.92.0",
  "gates": [
    {
      "name": "ci-gate",
      "command": "just ci-gate",
      "required": true,
      "status": "success",
      "exit_code": 0,
      "duration_seconds": 180,
      "log_path": "target/receipts/logs/ci-gate.log"
    }
  ]
}
```

### Comparing Local vs CI Receipts

CI uploads receipts as artifacts. To compare:

1. Download the CI receipt from GitHub Actions artifacts
2. Compare with local: `diff target/receipts/receipt.json ci-receipt.json`
3. Check duration differences for performance issues
4. Check toolchain versions for compatibility issues

### Debugging Gate Failures

```bash
# Get verbose output for ignored test breakdown
VERBOSE=1 bash scripts/ignored-test-count.sh

# Trace memory usage of specific commands
just trace 'cargo clippy -p perl-parser --no-deps -j1'

# Trace each low-memory step individually
just trace-lowmem-steps
```

---

## Pre-push Hook

### Installing the Hook

```bash
bash scripts/install-githooks.sh
```

This creates `.git/hooks/pre-push` which runs:

```bash
nix develop -c just ci-gate
# Falls back to: just ci-gate (if Nix unavailable)
```

### What It Runs

The hook runs the full merge gate before every push:

```
Running local gate before push: nix develop -c just ci-gate
   (Skip with: git push --no-verify)

Checking code formatting...
Running clippy (libraries only)...
Running library tests...
Enforcing no unwrap/expect in production code...
Running policy checks...
Running LSP semantic definition tests...
Checking parser features baseline...

Merge gate passed! (total: 180s)
```

### Bypassing the Hook (Emergencies Only)

```bash
git push --no-verify
```

**Warning:** Only use this for genuine emergencies. Broken commits on master affect everyone. Document why in your commit message.

Valid reasons to bypass:
- Emergency hotfix that will be followed up
- Pushing to a WIP branch that intentionally breaks tests
- Infrastructure changes being tested in CI

---

## Cost Awareness

### Why Run Locally?

| Action | Cost |
|--------|------|
| Local `just ci-gate` | $0.00 |
| GitHub Actions per PR | ~$0.06-0.08 |
| Failed CI retry | +$0.06-0.08 |
| 5 pushes without concurrency | ~$0.30-0.40 |
| 5 pushes with concurrency | ~$0.06-0.08 |

Running locally before pushing prevents wasted CI runs.

### When to Use CI vs Local

| Scenario | Run Locally | Use CI |
|----------|-------------|--------|
| Active development iteration | Yes | No |
| Before pushing | Yes (required) | Automatic |
| Cross-platform validation | Yes (Nix) | Yes (matrix) |
| Windows-specific testing | No | Yes |
| Stress/mutation testing | Optional | With labels |

### Label Conventions for Heavy Jobs

Some CI jobs are gated behind labels to control costs:

| Label | Purpose | Cost |
|-------|---------|------|
| `ci:stress` | Long-running stability tests | High |
| `ci:extras` | Optional LSP features | Medium |
| `ci:mutation` | Mutation testing (~15-30 min) | Very High |
| `ci:bench` | Performance benchmarks | High |
| `ci:coverage` | Code coverage analysis | High |

Only add these labels when you specifically need those checks.

---

## Quick Reference Card

```bash
# Quick feedback loop
just pr-fast

# Before pushing (required)
nix develop -c just ci-gate

# Full validation (large changes)
just ci-full

# Low-memory systems
just ci-gate-low-mem

# MSRV validation
just ci-gate-msrv

# Generate receipts
just gates

# Install pre-push hook
bash scripts/install-githooks.sh

# Bypass hook (emergency)
git push --no-verify

# Fix formatting
just fmt

# Check health metrics
just health

# Show ignored test breakdown
bash scripts/ignored-test-count.sh
```

---

## Related Documentation

- **[CI_LOCAL_VALIDATION.md](CI_LOCAL_VALIDATION.md)** - Detailed local validation philosophy
- **[CI_README.md](CI_README.md)** - CI documentation index
- **[CI.md](CI.md)** - GitHub Actions architecture
- **[CI_TEST_LANES.md](CI_TEST_LANES.md)** - Test lane organization
- **[CI_COST_TRACKING.md](CI_COST_TRACKING.md)** - Budget and cost management
- **[THREADING_CONFIGURATION_GUIDE.md](THREADING_CONFIGURATION_GUIDE.md)** - Thread safety details

---

**Last Updated:** 2025-01-24
