# Developer Friction Points

> This document catalogs known developer friction points in the Perl LSP project, their impacts, mitigations, and proposed long-term solutions.

**Last Updated**: 2026-03-13
**Status**: Living Document

---

## Table of Contents

- [Overview](#overview)
- [Setup and Installation Friction](#1-setup-and-installation-friction)
- [Testing Friction](#2-testing-friction)
- [Code Quality Friction](#3-code-quality-friction)
- [Workflow Friction](#4-workflow-friction)
- [Documentation Friction](#5-documentation-friction)
- [Quick Reference](#quick-reference)

---

## Overview

The Perl LSP project maintains high quality standards that inevitably create some developer friction. This document aims to:

1. **Acknowledge** friction points explicitly
2. **Explain** why they exist
3. **Provide** current workarounds
4. **Propose** long-term solutions

### Friction Philosophy

We accept some friction as a trade-off for:

- **Reliability**: No-panic policy prevents server crashes
- **Security**: Supply chain security and safe evaluation
- **Performance**: Sub-millisecond LSP responses
- **Maintainability**: 80+ crate architecture with clear boundaries

---

## 1. Setup and Installation Friction

### 1.1 Nix Flake Requirements

#### Description
The canonical local CI gate requires Nix with flakes enabled:

```bash
nix develop -c just ci-gate
```

#### Impact
- Developers without Nix must use fallback commands
- Nix installation adds ~15-30 minutes to onboarding
- Some corporate environments block Nix installation

#### Mitigation
Multiple fallback paths exist:

```bash
# Option 1: Use just directly (requires manual tool installation)
just ci-gate

# Option 2: Rust-native local mirror
cargo run -p perl-ci-hygiene -- check-local

# Option 3: Run individual gates manually
cargo fmt --all -- --check
cargo clippy --workspace --lib
cargo test --workspace --lib
```

#### Proposed Solutions
| Solution | Effort | Status |
|----------|--------|--------|
| Container-based CI gate | Medium | Proposed |
| Install script for CI tools | Low | Proposed |
| VS Code dev container | Medium | Proposed |

#### Related Documentation
- [`flake.nix`](../../flake.nix) - Nix configuration
- [`docs/ci/LOCAL_CI_SUMMARY.md`](../ci/LOCAL_CI_SUMMARY.md) - Local CI overview

---

### 1.2 Rust Toolchain Version Requirements

#### Description
The project pins to a specific MSRV (Minimum Supported Rust Version):

```toml
# rust-toolchain.toml
[toolchain]
channel = "1.92.0"
```

#### Impact
- Developers with older toolchains must upgrade
- Some Linux distributions have outdated Rust packages
- CI runners must match the pinned version

#### Mitigation
```bash
# Check current version
rustc --version

# Update via rustup
rustup update stable

# Use exact version
rustup override set 1.92.0

# Or let repo tooling validate it for you
just pr-fast
just doctor-env
```

#### Proposed Solutions
| Solution | Effort | Status |
|----------|--------|--------|
| Automatic version check in justfile | Low | Implemented via `scripts/check-rust-toolchain.sh` |
| Better error messages for version mismatch | Low | Proposed |

#### Related Documentation
- [`rust-toolchain.toml`](../../rust-toolchain.toml) - Toolchain specification
- [`AGENTS.md`](../../AGENTS.md) - Quick start guide

---

### 1.3 VS Code Extension Setup

#### Description
Setting up the Perl LSP extension requires manual configuration:

```json
// .vscode/settings.json
{
  "perl-lsp.serverPath": "",
  "perl-lsp.autoDownload": true,
  "perl-lsp.trace.server": "off",
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true
}
```

#### Impact
- Multiple configuration options can be overwhelming
- Different setup paths for generic LSP vs official extension
- Debug logging requires manual enabling

#### Mitigation
Use the official extension with auto-download:

```bash
code --install-extension EffortlessMetrics.perl-lsp-rs
```

#### Proposed Solutions
| Solution | Effort | Status |
|----------|--------|--------|
| Zero-config extension | Medium | Proposed |
| Setup wizard | Medium | Proposed |
| Better default settings | Low | Proposed |

#### Related Documentation
- [`docs/how-to/EDITOR_SETUP.md`](../how-to/EDITOR_SETUP.md) - Editor configuration

---

### 1.4 DAP Bridge Mode Setup

#### Description
The Debug Adapter Protocol requires Perl::LanguageServer CPAN module:

```bash
# Required for bridge mode
cpanm Perl::LanguageServer

# Verify installation
perl -e "use Perl::LanguageServer::DebuggerInterface; print qq{OK\n};"
```

#### Impact
- CPAN module installation can fail on some systems
- Additional dependency beyond Rust toolchain
- Bridge mode adds complexity vs native adapter

#### Mitigation
Use the native adapter CLI when possible:

```bash
# Native adapter (no Perl dependencies)
perl-dap

# Bridge mode only if needed
cpanm Perl::LanguageServer
```

#### Proposed Solutions
| Solution | Effort | Status |
|----------|--------|--------|
| Full native DAP implementation | High | In Progress |
| Better CPAN installation docs | Low | Proposed |

#### Related Documentation
- [`docs/tutorials/DAP_USER_GUIDE.md`](../tutorials/DAP_USER_GUIDE.md) - DAP setup guide
- [`docs/adr/0011-dap-bridge-mode-architecture.md`](../adr/0011-dap-bridge-mode-architecture.md) - Architecture decision

---

## 2. Testing Friction

### 2.1 Flaky Tests Requiring Special Configuration

#### Description
LSP tests require thread-constrained execution to avoid flakiness:

```bash
# Required for LSP tests
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2
```

#### Impact
- Default `cargo test` may fail on LSP tests
- Developers must remember special flags
- CI requires explicit configuration

#### Mitigation
Use the justfile targets which handle threading:

```bash
# Handles threading automatically
just ci-lsp-def

# Or use nextest with proper config
cargo nextest run -p perl-lsp-rs
```

#### Proposed Solutions
| Solution | Effort | Status |
|----------|--------|--------|
| Adaptive threading auto-detection | Medium | Implemented |
| Better test isolation | High | Proposed |
| Remove global state | High | Proposed |

#### Related Documentation
- [`docs/how-to/THREADING_CONFIGURATION_GUIDE.md`](../how-to/THREADING_CONFIGURATION_GUIDE.md) - Threading guide
- [`docs/adr/0018-adaptive-threading-tests.md`](../adr/0018-adaptive-threading-tests.md) - ADR

---

### 2.2 CI Resource Constraints

#### Description
CI builds use constrained resources to prevent OOM:

```bash
RUSTFLAGS="-Cdebuginfo=0 -Copt-level=1 --cfg ci"
CARGO_BUILD_JOBS=2
```

#### Impact
- CI builds slower than local builds
- Some tests skipped in CI via `#[cfg_attr(ci, ignore)]`
- Different behavior between CI and local

#### Mitigation
Use lean flags locally if experiencing resource issues:

```bash
# Lean build for resource-constrained environments
CARGO_BUILD_JOBS=2 cargo build --workspace
```

#### Proposed Solutions
| Solution | Effort | Status |
|----------|--------|--------|
| Larger CI runners | Low | Proposed |
| Better test partitioning | Medium | Proposed |

#### Related Documentation
- [`docs/project/CI.md`](../project/CI.md) - CI configuration

---

### 2.3 Unicode Processing Overhead

#### Description
UTF-16 position conversion for LSP protocol adds complexity:

```rust
// Required for LSP protocol compliance
pub fn convert_utf8_to_utf16_position(text: &str, utf8_offset: usize) -> u32 {
    if utf8_offset > text.len() {
        return text.chars().count() as u32;
    }
    text[..utf8_offset].encode_utf16().count() as u32
}
```

#### Impact
- Additional complexity in position handling
- Potential for boundary violations if not careful
- Performance overhead on large files

#### Mitigation
Use provided conversion utilities:

```rust
// Use rope for O(log n) conversions
use ropey::Rope;

let rope = Rope::from_str(text);
let utf16_pos = rope.char_to_utf16_cu(utf8_pos);
```

#### Proposed Solutions
| Solution | Effort | Status |
|----------|--------|--------|
| Rope-based document management | Medium | Implemented |
| Cached position conversion | Medium | Proposed |

#### Related Documentation
- [`docs/adr/0013-utf16-position-tracking.md`](../adr/0013-utf16-position-tracking.md) - UTF-16 ADR
- [`docs/adr/0020-rope-document-management.md`](../adr/0020-rope-document-management.md) - Rope ADR

---

## 3. Code Quality Friction

### 3.1 No-Panic Policy Enforcement

#### Description
Production code cannot use fatal constructs:

```rust
// BANNED in production code:
unwrap()      // ❌
expect()      // ❌
panic!()      // ❌
todo!()       // ❌
unimplemented!()  // ❌

// REQUIRED patterns:
?             // ✅
.ok_or_else() // ✅
match         // ✅
```

#### Impact
- More verbose error handling
- Learning curve for Rust beginners
- Requires explicit error types

#### Mitigation
Use test helpers for test code:

```rust
// In tests, use Result or helpers
#[test]
fn my_test() -> Result<()> {
    let value = some_fn()?;  // Works in tests
    Ok(())
}

// Or use perl_tdd_support helpers
use perl_tdd_support::{must, must_some};

let value = must(some_fn());      // Panics only in tests
let value = must_some(option);    // Panics only in tests
```

#### Proposed Solutions
| Solution | Effort | Status |
|----------|--------|--------|
| Custom lint rules | Medium | Proposed |
| Better error types | Medium | Ongoing |

#### Related Documentation
- [`docs/adr/0012-error-handling-strategy.md`](../adr/0012-error-handling-strategy.md) - Error handling ADR
- [`ci/check_unwraps_prod.sh`](../../ci/check_unwraps_prod.sh) - Enforcement script

---

### 3.2 Mutation Testing Requirements

#### Description
Mutation testing is required for quality validation:

```bash
# Run mutation testing
cargo mutants --in-place -- --test-threads=2
```

#### Impact
- Long-running tests (15-30 minutes)
- CI opt-in via `ci:mutation` label
- Requires understanding mutation operators

#### Mitigation
Run mutation testing selectively:

```bash
# Test specific modules
cargo mutants -p perl-parser --file src/utf16.rs

# Use baseline for comparison
cargo mutants --baseline
```

#### Proposed Solutions
| Solution | Effort | Status |
|----------|--------|--------|
| Incremental mutation testing | Medium | Proposed |
| Pre-computed mutation baselines | Low | Proposed |

#### Related Documentation
- [`docs/reference/MUTATION_TESTING_METHODOLOGY.md`](../reference/MUTATION_TESTING_METHODOLOGY.md) - Methodology guide
- [`docs/adr/0029-mutation-sentinel-values.md`](../adr/0029-mutation-sentinel-values.md) - Sentinel values ADR

---

### 3.3 Clippy Configuration

#### Description
Strict Clippy lints are enforced:

```bash
# Core crates require -D warnings
cargo clippy -p perl-parser -p perl-lexer -- -D warnings -A missing_docs

# Full workspace with stricter lints
cargo clippy --workspace -- -D warnings
```

#### Impact
- Some idioms are disallowed
- Additional refactoring may be needed
- `missing_docs` warnings require documentation

#### Mitigation
Use lib-only clippy for faster iteration:

```bash
# Faster clippy on libraries only
just clippy-core

# Or manually
cargo clippy --workspace --lib -- -D warnings -A missing_docs
```

#### Proposed Solutions
| Solution | Effort | Status |
|----------|--------|--------|
| Gradual lint tightening | Low | Ongoing |
| Better lint documentation | Low | Proposed |

#### Related Documentation
- [`clippy.toml`](../../clippy.toml) - Clippy configuration
- [`justfile`](../../justfile) - Clippy targets

---

### 3.4 Documentation Requirements

#### Description
Public APIs require documentation:

```rust
// This generates a warning:
pub fn my_function() -> i32 { 0 }

// Required:
/// Returns the default value.
///
/// # Examples
/// ```
/// use my_module::my_function;
/// assert_eq!(my_function(), 0);
/// ```
pub fn my_function() -> i32 { 0 }
```

#### Impact
- 605+ documentation warnings baseline
- Documentation must be maintained
- Examples must compile as doctests

#### Mitigation
Use phased approach:

```bash
# Check documentation warnings
cargo doc -p perl-parser 2>&1 | grep "missing documentation"

# Focus on public APIs first
cargo doc --document-private-items
```

#### Proposed Solutions
| Solution | Effort | Status |
|----------|--------|--------|
| Documentation generation | Medium | Proposed |
| Template-based docs | Low | Proposed |

#### Related Documentation
- [`docs/reference/MISSING_DOCUMENTATION_GUIDE.md`](../reference/MISSING_DOCUMENTATION_GUIDE.md) - Documentation guide
- [`docs/adr/0002-api-documentation-infrastructure.md`](../adr/0002-api-documentation-infrastructure.md) - Documentation ADR

---

## 4. Workflow Friction

### 4.1 Pre-Push Hook Requirements

#### Description
Pre-push hooks run the CI gate automatically:

```bash
# Install hooks
bash scripts/install-githooks.sh

# Hooks run on every push
git push  # Runs: nix develop -c just ci-gate
```

#### Impact
- Pushes take 3-5 minutes
- Failed gates block pushes
- Must remember to install hooks

#### Mitigation
Bypass in emergencies (not recommended):

```bash
# Skip hooks (use sparingly)
git push --no-verify
```

#### Proposed Solutions
| Solution | Effort | Status |
|----------|--------|--------|
| Faster pre-push gate | Medium | Proposed |
| Incremental gate caching | High | Proposed |

#### Related Documentation
- [`scripts/install-githooks.sh`](../../scripts/install-githooks.sh) - Hook installation
- [`hooks/pre-push`](../../hooks/pre-push) - Pre-push hook

---

### 4.2 CI Gate Tiers

#### Description
Multiple CI tiers with different scopes:

| Tier | Command | Duration | When to Use |
|------|---------|----------|-------------|
| A (PR-fast) | `just pr-fast` | ~1-2 min | Every PR iteration |
| B (Merge-gate) | `just ci-gate` | ~3-5 min | Before push |
| C (Nightly) | `just ci-full` | ~15-30 min | Scheduled |

#### Impact
- Confusion about which tier to use
- Merge gate required but slow
- Nightly tests may fail unexpectedly

#### Mitigation
Use tier-appropriate commands:

```bash
# During active development
just pr-fast

# Before pushing
just ci-gate

# For major changes
just ci-full
```

#### Proposed Solutions
| Solution | Effort | Status |
|----------|--------|--------|
| Smart tier detection | Medium | Proposed |
| Parallel gate execution | High | Proposed |

#### Related Documentation
- [`docs/ci/LOCAL_CI_SUMMARY.md`](../ci/LOCAL_CI_SUMMARY.md) - CI summary
- [`docs/ci/LOCAL_CI_PROTOCOL.md`](../ci/LOCAL_CI_PROTOCOL.md) - CI protocol

---

### 4.3 Opt-In CI Labels

#### Description
Heavy CI jobs require explicit labels:

| Label | Purpose |
|-------|---------|
| `ci:bench` | Performance benchmarks |
| `ci:mutation` | Mutation testing |
| `ci:strict` | Pedantic clippy |
| `ci:mac` | macOS builds |
| `ci:semver` | API compatibility |

#### Impact
- Must remember to add labels
- Some validation skipped by default
- Label-specific failures may surprise

#### Mitigation
Add labels proactively:

```bash
# Via GitHub CLI
gh pr edit --add-label "ci:bench,ci:mutation"

# Via web UI
# Labels sidebar → Add labels
```

#### Proposed Solutions
| Solution | Effort | Status |
|----------|--------|--------|
| Auto-label based on changes | Medium | Proposed |
| Label suggestions in PR template | Low | Proposed |

#### Related Documentation
- [`docs/project/CI.md`](../project/CI.md) - CI documentation

---

## 5. Documentation Friction

### 5.1 ADR Requirements

#### Description
Architecture decisions require ADRs:

```markdown
# ADR-00XX: Title

## Status
Accepted | Proposed | Deprecated

## Context
Why this decision was needed.

## Decision
What was decided.

## Consequences
Impact of the decision.
```

#### Impact
- 30+ ADRs to understand
- New decisions require ADR creation
- ADRs must be kept updated

#### Mitigation
Review ADR index first:

```bash
# Read ADR index
cat docs/adr/README.md

# Search for relevant ADRs
grep -r "topic" docs/adr/
```

#### Proposed Solutions
| Solution | Effort | Status |
|----------|--------|--------|
| ADR template generator | Low | Proposed |
| ADR search tool | Medium | Proposed |

#### Related Documentation
- [`docs/adr/README.md`](../adr/README.md) - ADR index
- [`docs/adr/ADR_001_AGENT_ARCHITECTURE.md`](../adr/ADR_001_AGENT_ARCHITECTURE.md) - ADR template

---

### 5.2 Missing Documentation Baselines

#### Description
Baselines track known issues:

```
# ci/missing_docs_baseline.txt
# Contains 605+ known documentation gaps
```

#### Impact
- Baselines must be maintained
- New violations must be justified
- Reducing baseline is slow progress

#### Mitigation
Check baselines before changes:

```bash
# Check if changes affect baseline
ci/check_missing_docs.sh

# View current baseline
wc -l ci/*.txt
```

#### Proposed Solutions
| Solution | Effort | Status |
|----------|--------|--------|
| Automated baseline reduction | Medium | Proposed |
| Baseline metrics tracking | Low | Proposed |

#### Related Documentation
- [`ci/missing_docs_baseline.txt`](../../ci/missing_docs_baseline.txt) - Documentation baseline
- [`ci/parse_errors_baseline.txt`](../../ci/parse_errors_baseline.txt) - Parser baseline

---

### 5.3 Cross-Reference Requirements

#### Description
Documentation must cross-reference properly:

```markdown
<!-- Good -->
See [COMMANDS_REFERENCE.md](../reference/COMMANDS_REFERENCE.md)

<!-- Bad -->
See the commands reference
```

#### Impact
- Relative paths can break
- Must update links on file moves
- Multiple link formats exist

#### Mitigation
Use link checker:

```bash
# Check documentation links
ci/check_doc_hygiene.sh
```

#### Proposed Solutions
| Solution | Effort | Status |
|----------|--------|--------|
| Automated link checking | Low | Implemented |
| Link linting in CI | Low | Implemented |

#### Related Documentation
- [`ci/check_doc_hygiene.sh`](../../ci/check_doc_hygiene.sh) - Link checker
- [`ci/check_doc_paths.sh`](../../ci/check_doc_paths.sh) - Path validator

---

## Quick Reference

### Common Commands

```bash
# Setup
nix develop                           # Enter dev shell
bash scripts/install-githooks.sh      # Install hooks

# Development
just pr-fast                          # Quick validation (~1-2 min)
just ci-gate                          # Full gate (~3-5 min)
just ci-lsp-def                       # LSP semantic tests

# Testing
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2
cargo nextest run                     # Fast test runner

# Quality
cargo fmt --all                       # Format
cargo clippy --workspace --lib        # Lint
cargo doc -p perl-parser              # Check docs

# Debugging
just doctor                           # Workspace health check (state corruption)
just doctor-env                       # Environment diagnostics (tools, components)
perl-lsp --health                     # Server health
RUST_LOG=perl_lsp=debug perl-lsp --stdio  # Debug logging
```

### Friction Summary Table

| Category | Friction Point | Severity | Mitigation Available |
|----------|---------------|----------|---------------------|
| Setup | Nix requirement | Medium | Fallback commands |
| Setup | Rust version | Low | rustup |
| Setup | VS Code config | Low | Official extension |
| Setup | DAP bridge | Medium | Native adapter |
| Testing | Thread constraints | Medium | just targets |
| Testing | CI resources | Low | Lean flags |
| Testing | Unicode overhead | Low | Rope utilities |
| Quality | No-panic policy | High | Test helpers |
| Quality | Mutation testing | Medium | Selective runs |
| Quality | Clippy strictness | Medium | lib-only option |
| Quality | Documentation | Medium | Phased approach |
| Workflow | Pre-push hooks | Medium | --no-verify |
| Workflow | CI tiers | Low | Tier-appropriate |
| Workflow | CI labels | Low | gh CLI |
| Docs | ADR requirements | Low | ADR index |
| Docs | Baselines | Low | Baseline scripts |
| Docs | Cross-references | Low | Link checker |

---

## Contributing

To add a new friction point:

1. Create an issue with label `documentation`
2. Include description, impact, and proposed solutions
3. Submit PR updating this document

---

## Related Documentation

- [`AGENTS.md`](../../AGENTS.md) - Project overview
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md) - Contribution guide
- [`docs/project/CI.md`](../project/CI.md) - CI documentation
- [`docs/how-to/TROUBLESHOOTING.md`](../how-to/TROUBLESHOOTING.md) - Troubleshooting guide
