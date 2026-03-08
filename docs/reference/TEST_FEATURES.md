# Test Feature Gates

This document describes the Cargo features used to gate different categories of tests in the Perl LSP workspace.

## Overview

The test suite uses feature gates to separate different types of tests:
- **Default tests** run without any features and should always pass
- **Feature-gated tests** require specific flags and may have special requirements

## Feature Flags

### perl-lsp crate (`crates/perl-lsp/Cargo.toml`)

| Feature | Description | When to Run |
|---------|-------------|-------------|
| `stress-tests` | Stress tests that are too slow for CI (>60s) | Local development, nightly CI |
| `strict-jsonrpc` | Strict JSON-RPC 2.0 protocol validation tests | Protocol compliance audits |
| `lsp-extras` | Quarantined tests for unimplemented/aspirational LSP features | Feature development |

### perl-parser crate (`crates/perl-parser/Cargo.toml`)

| Feature | Description | When to Run |
|---------|-------------|-------------|
| `parser-extras` | Enhanced parser tests | Development |
| `semantic-phase2` | Semantic analyzer Phase 2+ tests | Semantic feature development |
| `utf16-complete` | Complete UTF-16 position mapping tests | Position mapping work |

### perl-dap crate (`crates/perl-dap/Cargo.toml`)

| Feature | Description | When to Run |
|---------|-------------|-------------|
| `dap-phase1` | Phase 1 DAP tests | DAP bridge development |
| `dap-phase2` | Phase 2 DAP tests | Native DAP development |
| `dap-phase3` | Phase 3 DAP tests | Advanced DAP features |

## Running Feature-Gated Tests

```bash
# Run stress tests
cargo test -p perl-lsp --features stress-tests

# Run protocol compliance tests
cargo test -p perl-lsp --features strict-jsonrpc

# Run with multiple features
cargo test -p perl-lsp --features "stress-tests,strict-jsonrpc"
```

## Ignored Test Categories

The test suite uses standardized ignore reason prefixes for tracking:

| Category | Purpose | CI Policy |
|----------|---------|-----------|
| `BROKENPIPE:` | Transport/pipe flakes | **Must be 0** |
| `FEATURE:` | Feature-gated tests | **Must be 0** (use cfg_attr instead) |
| `STRESS:` | Stress tests | **Must be 0** (use cfg_attr instead) |
| `PROTOCOL:` | Protocol compliance | **Must be 0** (use cfg_attr instead) |
| `BARE` | No reason given | **Must be 0** |
| `OTHER` | Unrecognized | **Must be 0** |
| `MANUAL:` | Manual helper tests | Allowed |
| `BUG:` | Known bugs to fix | Allowed (tracked) |
| `INFRA:` | Infrastructure/TODO | Allowed (tracked) |
| `MUT_*:` | Mutation testing bugs | Allowed (tracked as BUG) |

## Checking Test Health

```bash
# Check current ignored test counts
bash scripts/ignored-test-count.sh

# Verbose output with details
VERBOSE=1 bash scripts/ignored-test-count.sh

# Update baseline after fixing tests
bash scripts/ignored-test-count.sh --update

# CI gate mode (fails if critical categories > 0)
bash scripts/ignored-test-count.sh --check
```

## Migration Pattern: ignore → cfg_attr

When converting a test from `#[ignore]` to feature-gated:

```rust
// Before (counted as INFRA/FEATURE/etc)
#[test]
#[ignore = "FEATURE: Requires semantic phase 2"]
fn test_advanced_semantic() { ... }

// After (not counted, properly gated)
#[test]
#[cfg_attr(not(feature = "semantic-phase2"), ignore = "FEATURE: Run with --features semantic-phase2")]
fn test_advanced_semantic() { ... }
```

## Best Practices

1. **Never leave bare `#[ignore]`** - always provide a categorized reason
2. **Use cfg_attr for feature-gated tests** - keeps counts accurate
3. **Fix BUG tests promptly** - they represent real issues
4. **Keep MANUAL tests minimal** - only snapshot regenerators and helpers
5. **Run the gate script before PRs** - `bash scripts/gate-local.sh`
