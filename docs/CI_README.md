# CI Documentation Index

This directory contains all CI/CD documentation for perl-lsp. The project follows a **local-first** development philosophy where all gates run locally before pushing.

## Quick Start

```bash
# Run the canonical local gate (required before push)
nix develop -c just ci-gate
```

## Documentation Overview

| Document | Purpose | Audience |
|----------|---------|----------|
| [CI.md](CI.md) | GitHub Actions architecture, runner versions | DevOps, Contributors |
| [CI_LOCAL_VALIDATION.md](CI_LOCAL_VALIDATION.md) | Local-first philosophy, `just ci-gate` workflow | All developers |
| [CI_TEST_LANES.md](CI_TEST_LANES.md) | Test categorization (Core, LSP, etc.) | Contributors, Test authors |
| [CI_HARDENING.md](CI_HARDENING.md) | Quality enforcement rules, Rust contracts | Contributors |
| [CI_COST_TRACKING.md](CI_COST_TRACKING.md) | GitHub Actions budget, optimization | Maintainers |
| [CI_AUDIT.md](CI_AUDIT.md) | Automated parser/feature validation | Maintainers |

## Reading Order

**For contributors:**
1. [CI_LOCAL_VALIDATION.md](CI_LOCAL_VALIDATION.md) - Understand local-first workflow
2. [CI_TEST_LANES.md](CI_TEST_LANES.md) - Know which tests run when
3. [CI_HARDENING.md](CI_HARDENING.md) - Understand quality gates

**For maintainers:**
1. [CI.md](CI.md) - GitHub Actions configuration
2. [CI_COST_TRACKING.md](CI_COST_TRACKING.md) - Budget management
3. [CI_AUDIT.md](CI_AUDIT.md) - Automated validation

## Key Concepts

### Local-First Development

All CI gates can (and should) run locally:

```bash
# Full gate check
nix develop -c just ci-gate

# Individual checks
cargo fmt --check         # Format check
cargo clippy --workspace  # Linting
cargo test --workspace    # Tests
```

### Test Lanes

Tests are categorized into lanes for budget efficiency:

| Lane | Runs On | Purpose |
|------|---------|---------|
| Core | Every PR | Essential parsing tests |
| LSP | Every PR | Language server tests |
| Extended | Merge to main | Comprehensive coverage |

### Quality Contracts

Enforced via Rust attributes:

- `#![deny(unsafe_code)]` - No unsafe code
- `#![deny(unreachable_pub)]` - Clean public APIs
- `#![warn(missing_docs)]` - Documentation required

## Archived Documents

Historical CI status documents (like `CI_STATUS_214.md`) are kept for reference but should not be used for current development guidance.

## See Also

- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contribution guidelines
- [COMMANDS_REFERENCE.md](COMMANDS_REFERENCE.md) - Full command catalog
