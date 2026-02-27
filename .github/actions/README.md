# GitHub Actions Components

This directory contains reusable composite actions for perl-lsp CI workflows.

## Composite Actions

### `setup-rust/`

Sets up Rust toolchain with caching optimized for perl-lsp.

**Usage:**
```yaml
- uses: ./.github/actions/setup-rust
  with:
    toolchain: '1.92.0'        # or 'stable', 'nightly'
    components: rustfmt, clippy
    cache: true
    sccache: false
    install-just: true
    install-nextest: false
```

**Inputs:**
| Input | Default | Description |
|-------|---------|-------------|
| `toolchain` | `stable` | Rust toolchain version |
| `components` | `rustfmt, clippy` | Toolchain components |
| `cache` | `true` | Enable Swatinem/rust-cache |
| `cache-key-suffix` | `''` | Extra cache key differentiation |
| `sccache` | `false` | Enable Mozilla sccache |
| `install-just` | `false` | Install just command runner |
| `install-nextest` | `false` | Install cargo-nextest |

### `rust-checks/`

Runs standard Rust format, lint, and test checks.

**Usage:**
```yaml
- uses: ./.github/actions/rust-checks
  with:
    check-fmt: true
    check-clippy: true
    clippy-args: '--workspace --lib --locked -- -D warnings'
    clippy-prod: true
    test: true
    test-args: '--workspace --lib --locked'
    test-threads: '2'
```

**Inputs:**
| Input | Default | Description |
|-------|---------|-------------|
| `check-fmt` | `true` | Run `cargo fmt --check` |
| `check-clippy` | `true` | Run clippy |
| `clippy-args` | `--workspace --lib --locked -- -D warnings -A missing_docs` | Clippy arguments |
| `clippy-prod` | `false` | Run production clippy (deny unwrap/expect) |
| `test` | `true` | Run tests |
| `test-args` | `--workspace --lib --locked` | Test arguments |
| `test-threads` | `''` | `RUST_TEST_THREADS` value |
| `build-jobs` | `''` | `CARGO_BUILD_JOBS` value |

### `upload-receipt/`

Uploads gate receipts with GitHub step summary generation.

**Usage:**
```yaml
- uses: ./.github/actions/upload-receipt
  if: always()
  with:
    receipt-path: target/receipts/receipt.json
    retention-days: 14
    generate-summary: true
```

**Inputs:**
| Input | Default | Description |
|-------|---------|-------------|
| `receipt-path` | `target/receipts/receipt.json` | Path to receipt file |
| `logs-path` | `target/receipts/logs` | Path to logs directory |
| `artifacts-path` | `target/receipts/artifacts` | Path to artifacts |
| `artifact-name` | `gate-receipt` | Artifact name |
| `retention-days` | `14` | Days to retain artifact |
| `generate-summary` | `true` | Generate step summary |

## Reusable Workflows

### `_rust-tier.yml`

Reusable workflow for running different tiers of checks. The underscore prefix indicates this is a called workflow, not triggered directly.

**Usage:**
```yaml
jobs:
  fast-checks:
    uses: ./.github/workflows/_rust-tier.yml
    with:
      tier: pr-fast
      os: '["ubuntu-latest"]'
      toolchain: '1.92.0'
    secrets: inherit

  full-checks:
    uses: ./.github/workflows/_rust-tier.yml
    with:
      tier: merge-gate
      os: '["ubuntu-latest", "windows-latest"]'
    secrets: inherit
```

**Tiers:**

| Tier | Duration | Checks |
|------|----------|--------|
| `pr-fast` | ~1-2 min | Format, core clippy, core tests |
| `merge-gate` | ~3-5 min | Full clippy, all tests, policy, LSP smoke |
| `nightly` | ~15-30 min | Everything + security audit, docs, benchmarks |

**Inputs:**
| Input | Default | Description |
|-------|---------|-------------|
| `tier` | (required) | `pr-fast`, `merge-gate`, or `nightly` |
| `os` | `["ubuntu-latest"]` | JSON array of OS runners |
| `toolchain` | `1.92.0` | Rust toolchain |
| `timeout-minutes` | `30` | Job timeout |
| `fail-fast` | `true` | Stop on first failure |
| `upload-receipt` | `true` | Upload gate receipt |

## Shared Configuration Patterns

### Environment Variables

Common environment variables used across workflows:

```yaml
env:
  # Terminal and output
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: '1'

  # Build optimization
  CARGO_INCREMENTAL: '0'
  CARGO_BUILD_JOBS: '2'      # Limit parallelism in CI

  # Test configuration
  RUST_TEST_THREADS: '2'     # Adaptive threading

  # Network resilience
  CARGO_NET_RETRY: '4'
  CARGO_HTTP_MULTIPLEXING: 'false'
```

### Cache Key Patterns

Standard cache key pattern:
```yaml
key: ${{ runner.os }}-${{ inputs.toolchain }}-${{ inputs.tier }}-${{ hashFiles('Cargo.lock') }}
```

### Timeout Defaults

| Context | Timeout |
|---------|---------|
| PR fast checks | 10 minutes |
| Merge gate | 30 minutes |
| Nightly comprehensive | 60 minutes |
| LSP tests | 30 minutes |

### Concurrency Groups

Prevent redundant runs:
```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

## Example: Consolidated CI Workflow

```yaml
name: CI

on:
  pull_request:
    branches: [master, main]

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

jobs:
  # Fast checks on every PR push
  pr-fast:
    uses: ./.github/workflows/_rust-tier.yml
    with:
      tier: pr-fast
    secrets: inherit

  # Full checks before merge (when labeled or on master)
  merge-gate:
    if: contains(github.event.pull_request.labels.*.name, 'ready-to-merge')
    needs: pr-fast
    uses: ./.github/workflows/_rust-tier.yml
    with:
      tier: merge-gate
      os: '["ubuntu-latest", "windows-latest"]'
    secrets: inherit
```

## Migration Guide

To migrate an existing workflow to use these components:

1. Replace toolchain setup:
   ```yaml
   # Before
   - uses: dtolnay/rust-toolchain@stable
     with:
       components: rustfmt, clippy
   - uses: Swatinem/rust-cache@v2

   # After
   - uses: ./.github/actions/setup-rust
     with:
       toolchain: stable
       cache: true
   ```

2. Replace check steps:
   ```yaml
   # Before
   - run: cargo fmt --check
   - run: cargo clippy -- -D warnings
   - run: cargo test

   # After
   - uses: ./.github/actions/rust-checks
     with:
       check-fmt: true
       check-clippy: true
       test: true
   ```

3. Replace receipt upload:
   ```yaml
   # Before
   - uses: actions/upload-artifact@v4
     with:
       name: receipt
       path: target/receipts/

   # After
   - uses: ./.github/actions/upload-receipt
     with:
       receipt-path: target/receipts/receipt.json
   ```
