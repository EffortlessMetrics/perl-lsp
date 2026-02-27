# SemVer Breaking Change Detection Workflow

**Status:** ✅ Automated (Issue #277)
**Tools:** cargo-semver-checks, justfile recipes, CI integration
**Scope:** Published crates (perl-parser, perl-lexer, perl-parser-core, perl-lsp)

---

## Overview

Perl LSP uses automated semantic versioning validation to prevent accidental breaking changes in public APIs. This document describes the complete workflow for checking, documenting, and managing API changes.

## Quick Start

### For Contributors

**Before submitting a PR with API changes:**

```bash
# Check for breaking changes locally
just semver-check

# Check specific package
just semver-check-package perl-parser

# View detailed API diff
just semver-diff perl-parser
```

**In your PR:**
- Add `ci:semver` label to run automated checks
- Document any breaking changes in PR description
- Follow migration guide template (see below)

### For Maintainers

**Before cutting a release:**

```bash
# Generate comprehensive report
just semver-report

# Review breaking changes
cat target/semver-reports/breaking-changes.json

# Verify version bump is appropriate
# - Breaking changes → major version (0.9 → 1.0)
# - Additive changes → minor version (0.9 → 0.10)
# - Bug fixes → patch version (0.9.x → 0.9.2)
```

---

## How It Works

### Baseline Comparison

SemVer checks compare the current codebase against the last release tag:

```
Current HEAD
    ↓
   diff
    ↓
Last release tag (e.g., v0.8.5)
```

The tool analyzes:
- Public API surface (all `pub` items)
- Function signatures
- Type definitions
- Trait implementations
- Module structure

### Detection Rules

**Breaking changes detected:**
- ❌ Removing public functions, types, or modules
- ❌ Changing function signatures (parameters, return types)
- ❌ Removing or renaming struct fields
- ❌ Changing enum variants (unless `#[non_exhaustive]`)
- ❌ Changing trait bounds or implementations
- ❌ Tightening visibility (pub → pub(crate))

**Non-breaking changes allowed:**
- ✅ Adding new public functions or types
- ✅ Adding new enum variants (with `#[non_exhaustive]`)
- ✅ Adding optional trait implementations
- ✅ Relaxing trait bounds
- ✅ Expanding visibility (pub(crate) → pub)
- ✅ Deprecating items (with `#[deprecated]`)

---

## Local Workflow

### 1. Install Tool (if needed)

```bash
# cargo-semver-checks is required
cargo install cargo-semver-checks --locked
```

### 2. Check for Breaking Changes

```bash
# Check all published packages
just semver-check

# Example output:
# 🔍 Checking for SemVer breaking changes...
# Using baseline: v0.8.5
#
# Checking perl-parser...
# ✅ No breaking changes detected
#
# Checking perl-lexer...
# ❌ Breaking changes detected:
# - Removed pub function: PerlLexer::tokenize_legacy
# - Changed signature: Token::new (added parameter)
```

### 3. Review Specific Changes

```bash
# View detailed diff for a package
just semver-diff perl-parser

# Example output:
# 📝 Public API changes in perl-parser since last release:
#
# Breaking changes:
# - pub fn parse(&mut self, source: &str) -> Result<Node, ParseError>
# + pub fn parse(&mut self, source: &str, config: &ParseConfig) -> Result<Node, ParseError>
#
# Additions:
# + pub fn parse_with_defaults(&mut self, source: &str) -> Result<Node, ParseError>
```

### 4. Fix or Document

**Option A: Make non-breaking (preferred)**

```rust
// Before (breaking)
pub fn parse(&mut self, source: &str) -> Result<Node, ParseError>

// After (non-breaking - add new function, keep old)
#[deprecated(since = "1.2.0", note = "use `parse_with_config()` instead")]
pub fn parse(&mut self, source: &str) -> Result<Node, ParseError> {
    self.parse_with_config(source, &ParseConfig::default())
}

pub fn parse_with_config(&mut self, source: &str, config: &ParseConfig) -> Result<Node, ParseError>
```

**Option B: Document breaking change**

If the breaking change is intentional (for major version):

```markdown
## Breaking Changes

### Parser API Signature Change

**What changed:**
- `Parser::parse()` now requires a `ParseConfig` parameter

**Migration:**
```rust
// Old code (v1.x)
let result = parser.parse(source)?;

// New code (v2.0)
let config = ParseConfig::default();
let result = parser.parse(source, &config)?;
```

**Reason:** Enables incremental parsing and position tracking configuration
```

---

## CI Integration

### Triggering CI Checks

Add the `ci:semver` label to your PR:

```
Labels: ci:semver
```

GitHub Actions will:
1. Checkout code with full git history
2. Install cargo-semver-checks
3. Determine baseline tag (last release)
4. Run checks on all published packages
5. Generate breaking changes report
6. Upload report as artifact

### Reading CI Output

**No breaking changes:**
```
✅ Check perl-parser API compatibility
   No breaking changes detected
```

**Breaking changes detected:**
```
⚠️ Check perl-parser API compatibility
   Breaking changes detected:
   - Removed: pub fn parse_legacy
   - Changed: pub fn parse (signature)
```

### Downloading Reports

1. Go to PR → Actions tab
2. Find "API Compatibility" workflow run
3. Download "semver-breaking-changes-report" artifact
4. Unzip and view `breaking-changes.json`

---

## Configuration

### .cargo-semver-checks.toml

Project-wide configuration for SemVer checking:

```toml
# Baseline strategy
[baseline]
default-strategy = "git-tag"
tag-pattern = "v{major}.{minor}.{patch}"

# Packages to check
[[packages]]
name = "perl-parser"
check = true  # Strict semver enforcement

[[packages]]
name = "perl-parser-pest"
check = false  # Excluded: deprecated
reason = "deprecated legacy parser"

# Lints
[lints]
breaking = "deny"           # Fail on breaking changes
deprecated-removals = "warn"  # Warn on deprecated item removal
additions = "allow"          # Allow new items
```

### Excluding Crates

Some crates are excluded from SemVer checking:

| Crate | Reason |
|-------|--------|
| `xtask` | Internal build tooling |
| `perl-tdd-support` | Test-only utilities |
| `perl-parser-pest` | Deprecated legacy parser |
| `perl-dap` | Pre-1.0 (no stability guarantees) |

To exclude additional crates, update `.cargo-semver-checks.toml`.

---

## Common Scenarios

### Scenario 1: Adding New Public Function

**Change:**
```rust
// Add new function
pub fn parse_with_config(source: &str, config: &Config) -> Result<Node>
```

**SemVer check:** ✅ Passes (additive change)
**Version bump:** Minor (0.9 → 0.10)
**Action:** None required

---

### Scenario 2: Changing Function Signature

**Change:**
```rust
// Old
pub fn parse(source: &str) -> Result<Node>

// New
pub fn parse(source: &str, config: &Config) -> Result<Node>
```

**SemVer check:** ❌ Fails (breaking change)
**Version bump:** Major (0.9 → 1.0)
**Action:**
1. Document breaking change in PR
2. Add to CHANGELOG under "Breaking Changes"
3. Provide migration guide
4. Coordinate with maintainers for major release

**Alternative (non-breaking):**
```rust
// Keep old function, add new one
pub fn parse(source: &str) -> Result<Node> {
    parse_with_config(source, &Config::default())
}

pub fn parse_with_config(source: &str, config: &Config) -> Result<Node>
```

---

### Scenario 3: Removing Deprecated Function

**Change:**
```rust
// Remove previously deprecated function
// (was: #[deprecated(since = "1.2.0")])
// pub fn parse_legacy(source: &str) -> Node
```

**SemVer check:** ❌ Fails (breaking change)
**Version bump:** Major (0.9 → 1.0)
**Action:**
1. Ensure deprecation was present for ≥6 months
2. Document removal in CHANGELOG
3. Reference migration path
4. Only remove in major version release

---

### Scenario 4: Adding Enum Variant

**Change:**
```rust
#[non_exhaustive]
pub enum NodeKind {
    Sub,
    Package,
    Async,  // New variant
}
```

**SemVer check:** ✅ Passes (if `#[non_exhaustive]`)
**Version bump:** Minor (0.9 → 0.10)
**Action:** Ensure enum has `#[non_exhaustive]` attribute

**Without `#[non_exhaustive]`:**
```rust
pub enum NodeKind {  // Missing #[non_exhaustive]
    Sub,
    Package,
    Async,  // ❌ Breaking change
}
```

This breaks exhaustive pattern matching:
```rust
// User code breaks
match kind {
    NodeKind::Sub => {},
    NodeKind::Package => {},
    // Missing Async case
}
```

---

## SemVer Policy Reference

### Version Bumping Rules

| Change Type | Example | Version Bump |
|-------------|---------|--------------|
| **Breaking** | Remove pub fn | 0.9.x → 2.0.0 |
| **Breaking** | Change signature | 0.9.x → 2.0.0 |
| **Breaking** | Remove enum variant | 0.9.x → 2.0.0 |
| **Additive** | Add pub fn | 0.9.x → 0.10.0 |
| **Additive** | Add enum variant (`#[non_exhaustive]`) | 0.9.x → 0.10.0 |
| **Additive** | Deprecate item | 0.9.x → 0.10.0 |
| **Patch** | Fix bug (same behavior) | 0.9.x → 0.9.2 |
| **Patch** | Documentation only | 0.9.x → 0.9.2 |
| **Patch** | Internal refactoring | 0.9.x → 0.9.2 |

### Deprecation Timeline

1. **Release N (e.g., 1.2.0):**
   - Add `#[deprecated]` attribute
   - Provide alternative API
   - Document in CHANGELOG

2. **Release N+1 (e.g., 1.3.0, +6 months):**
   - Deprecation warning remains
   - Update documentation

3. **Release M (e.g., 2.0.0, +12 months):**
   - Remove deprecated item
   - Document removal in CHANGELOG

Minimum deprecation period: **6 months**

---

## Troubleshooting

### Issue: "Baseline tag not found"

**Error:**
```
error: baseline tag v0.8.5 not found
```

**Solution:**
```bash
# List available tags
just semver-list-baselines

# Use specific tag
git tag | grep -E '^v[0-9]' | sort -V | tail -1
```

---

### Issue: "Unsupported rustdoc format"

**Error:**
```
error: unsupported rustdoc format v57
(supported formats are v53, v55, v56)
```

**Cause:** Rustdoc version too new for cargo-semver-checks

**Solution:**
```bash
# Update cargo-semver-checks
cargo install cargo-semver-checks --locked --force

# Or use older Rust toolchain for check
rustup install 1.92.0
rustup run 1.92.0 cargo semver-checks
```

---

### Issue: "Crate not published to crates.io"

**Error:**
```
error: perl-parser not found in registry (crates.io)
```

**Solution:** Use git baseline (already configured):
```bash
# Uses git tags instead of crates.io
cargo semver-checks check-release -p perl-parser --baseline-rev v0.8.5
```

---

### Issue: False Positive Breaking Change

**Problem:** SemVer check reports breaking change for internal change

**Solution:**
1. Verify item is truly public (`pub` without `pub(crate)`)
2. If internal, use `pub(crate)` or `pub(super)`
3. If false positive, report issue to cargo-semver-checks

---

## Integration with Release Process

### Pre-Release Checklist

Before cutting a release:

- [ ] Run `just semver-check` locally
- [ ] Review `target/semver-reports/breaking-changes.json`
- [ ] Verify version bump matches change type:
  - Breaking → major
  - Additive → minor
  - Bug fix → patch
- [ ] Update CHANGELOG with API changes
- [ ] Document breaking changes with migration guide
- [ ] Ensure deprecations followed timeline
- [ ] Run full gate: `just ci-gate`

### Release Tagging

After release:
```bash
# Tag release
git tag -a v0.10.0 -m "Release v0.10.0"
git push origin v0.10.0

# This tag becomes the baseline for next semver check
```

---

## Resources

### Documentation

- **SemVer Spec:** https://semver.org/
- **Rust API Guidelines:** https://rust-lang.github.io/api-guidelines/
- **cargo-semver-checks:** https://github.com/obi1kenobi/cargo-semver-checks
- **Project Stability Policy:** `docs/STABILITY.md`

### Justfile Commands

```bash
just semver-check              # Check all packages
just semver-check-package PKG  # Check specific package
just semver-check-all          # Check all published packages
just semver-report             # Generate JSON report
just semver-diff PKG           # View API diff for package
just semver-list-baselines     # List available baseline tags
```

### CI Labels

- `ci:semver` - Run SemVer checks in CI
- `breaking-change` - Mark PR as containing breaking changes

---

## Feedback

File issues with label `tooling` for:
- False positives/negatives in SemVer detection
- Configuration improvements
- Documentation clarifications
- Integration suggestions

---

**Last updated:** 2026-01-28
**Related:** Issue #277, `docs/STABILITY.md`, `CONTRIBUTING.md`
