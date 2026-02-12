# Contributing to Perl LSP

Thank you for your interest in contributing to Perl LSP! This guide will help you get started.

## Getting Started

1. **Fork and Clone**
   ```bash
   git clone https://github.com/your-username/perl-lsp.git
   cd perl-lsp
   ```

2. **Install Dependencies**
   ```bash
   # Rust toolchain (if not already installed)
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # Install nextest for faster testing
   cargo install cargo-nextest
   ```

3. **Build the Project**
   ```bash
   cargo build
   cargo test
   ```

## Development Workflow

### Making Changes

1. Create a feature branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes following our coding standards:
   - Run `cargo fmt` to format code
   - Run `cargo clippy` to check for common issues
   - Add tests for new functionality
   - Update documentation as needed

3. Test your changes:
   ```bash
   cargo nextest run          # Fast test execution
   cargo test                 # Traditional test runner
   cargo clippy --workspace   # Lint checks
   ```

4. Commit with clear messages:
   ```bash
   git commit -m "feat: add new feature X"
   git commit -m "fix: resolve issue #123"
   ```

### Pull Request Process

1. **Push your branch** and open a Pull Request
2. **Describe your changes** clearly in the PR description
3. **Link related issues** using GitHub keywords (e.g., "Fixes #123")
4. **Respond to review feedback** promptly

## Continuous Integration

See **[CI & Automation](./docs/CI.md)** for comprehensive details about our GitHub Actions setup, including:

- **Pinned runner versions** (`ubuntu-22.04`, `windows-2022`)
- **Default CI jobs** that run on every PR
- **Opt-in CI labels** for heavy jobs (`ci:bench`, `ci:mutation`, `ci:strict`, `ci:mac`, `ci:semver`)
- **Build optimizations** (lean flags, nextest configuration)
- **Troubleshooting tips** for common CI issues

### Quick CI Tips

- All PRs run **format checks**, **clippy**, and **core tests** automatically
- Tests use **nextest** with lean build flags for faster, reliable execution
- Add `ci:bench` label to run performance benchmarks
- Add `ci:strict` label for pedantic clippy checks
- Add `ci:mac` label if your changes affect macOS
- Add `ci:semver` label to check for breaking API changes

### Local CI Validation (While GitHub Actions Is Unavailable)

**⚠️ IMPORTANT**: GitHub Actions is currently unavailable due to billing issues. During this period:

- **REQUIRED**: Run `just ci-gate` before every merge
- **RECOMMENDED**: Run `just ci-full` for large/structural changes
- See **[Local CI Protocol](./docs/ci/LOCAL_CI_PROTOCOL.md)** for complete details

```bash
# Fast merge gate (~2-5 min, required for all merges)
just ci-gate

# Comprehensive validation (~10-20 min, for large changes)
just ci-full
```

**Note in PR descriptions**:
```markdown
## Local CI Validation
✅ `just ci-gate` passed
See: [Local CI Protocol](docs/ci/LOCAL_CI_PROTOCOL.md)
```

**Semantic & LSP Changes**:

If you modify `crates/perl-parser/src/semantic.rs` or any LSP handler (especially `textDocument/definition`):

```bash
# Run semantic-aware definition tests
just ci-lsp-def

# Or run the full gate (includes ci-lsp-def)
just ci-gate
```

The semantic tests validate that LSP definition resolution works correctly for:
- Scalar variable references → declarations
- Subroutine calls → sub definitions
- Lexical scope resolution
- Package-qualified symbol lookups

Once GitHub Actions is restored, this section will be archived and normal CI workflow will resume.

## SemVer Breaking Change Detection

Perl LSP follows strict [Semantic Versioning 2.0.0](https://semver.org/). We use automated tools to detect breaking changes in public APIs.

### When to Check for Breaking Changes

**Required for:**
- Changes to public API functions, types, or modules
- Changes to `pub` items in published crates (`perl-parser`, `perl-lexer`, `perl-parser-core`, `perl-lsp`)
- Signature changes to existing functions
- Removing or renaming public items
- Changes to error types or return values

**Not required for:**
- Internal (`pub(crate)`) changes
- Test-only code changes
- Documentation updates
- Performance improvements that don't change behavior

### Local SemVer Checking

Check for breaking changes locally before submitting a PR:

```bash
# Check all published packages for breaking changes
just semver-check

# Check a specific package
just semver-check-package perl-parser

# View detailed diff of API changes
just semver-diff perl-parser

# List available baseline tags
just semver-list-baselines
```

**Understanding the output:**

```rust
// Breaking change (requires major version bump)
- pub fn parse(&mut self, source: &str) -> Result<Node, ParseError>
+ pub fn parse(&mut self, source: &str, config: &Config) -> Result<Node, Error>

// Non-breaking change (allowed in minor version)
+ pub fn parse_with_config(&mut self, source: &str, config: &Config) -> Result<Node, Error>
```

### CI SemVer Validation

Add the `ci:semver` label to your PR to run automated breaking change detection:

1. **Add label:** `ci:semver` to your PR
2. **CI runs:** GitHub Actions compares your changes against the last release tag
3. **Review results:** Check the workflow output for breaking changes
4. **Download report:** Breaking changes report available as artifact

**CI checks:**
- Compares against baseline (last release tag, e.g., `v0.8.5`)
- Checks `perl-parser`, `perl-lexer`, `perl-parser-core`, `perl-lsp`
- Generates JSON report of all breaking changes
- Warns on breaking changes (doesn't fail the build)

### SemVer Policy Summary

| Change Type | Example | Version Bump | Allowed In |
|-------------|---------|--------------|------------|
| **Breaking** | Remove public function | Major (1.0 → 2.0) | Major releases only |
| **Breaking** | Change function signature | Major (1.0 → 2.0) | Major releases only |
| **Additive** | Add new public function | Minor (1.0 → 1.1) | Minor releases |
| **Additive** | Add new enum variant | Minor (1.0 → 1.1) | Minor releases (with `#[non_exhaustive]`) |
| **Patch** | Fix bug, same behavior | Patch (1.0.0 → 1.0.1) | Patch releases |
| **Patch** | Documentation update | Patch (1.0.0 → 1.0.1) | Patch releases |

### Breaking Change Workflow

If you need to make a breaking change:

1. **Document the breaking change:**
   ```markdown
   ## Breaking Changes
   - `Parser::parse()` signature changed to include `Config` parameter
   - Migration: Use `Parser::parse_with_config()` or pass default config
   ```

2. **Deprecate before removing (when possible):**
   ```rust
   #[deprecated(since = "1.2.0", note = "use `parse_with_config()` instead")]
   pub fn parse_legacy(source: &str) -> Result<Node, ParseError> {
       self.parse_with_config(source, &Config::default())
   }
   ```

3. **Add migration guide** to PR description
4. **Label PR with `breaking-change`**
5. **Coordinate with maintainers** for major version planning

### Configuration

SemVer checking is configured in `.cargo-semver-checks.toml`:

```toml
# Published crates checked for breaking changes
- perl-parser (strict)
- perl-lexer (strict)
- perl-parser-core (strict)
- perl-lsp (strict)

# Internal crates excluded
- xtask (build tooling)
- perl-tdd-support (test utilities)
- perl-parser-pest (deprecated)
```

### Resources

- **SemVer spec:** https://semver.org/
- **cargo-semver-checks:** https://github.com/obi1kenobi/cargo-semver-checks
- **Project stability policy:** `docs/STABILITY.md`
- **API stability guarantees:** `docs/STABILITY.md#api-surface-stability`

## Coding Standards

- **Formatting:** Use `cargo fmt --all` before committing
- **Linting:** Fix all `cargo clippy` warnings
- **Testing:** Maintain or improve test coverage
- **Documentation:** Update docs for public APIs and new features
- **Commits:** Use conventional commit format (feat:, fix:, docs:, etc.)

### Code Style Guidelines

- Prefer `.first()` over `.get(0)` for accessing first element
- Use `.push(char)` instead of `.push_str("x")` for single characters
- Use `or_default()` instead of `or_insert_with(Vec::new)` for default values
- Avoid unnecessary `.clone()` on types that implement Copy
- Add `#[allow(clippy::only_used_in_recursion)]` for recursive tree traversal functions

### Cross-Platform `ExitStatus` in Tests

On Unix, `ExitStatus::from_raw(1)` is **wrong** (needs high-byte encoding). On Windows, the signature doesn't exist. Always use the portable helpers from `crates/perl-parser/src/execute_command.rs`:

```rust
#[cfg(test)]
use crate::execute_command::mock_status;

#[test]
fn status_round_trip() {
    assert!(mock_status(0).success());
    assert_eq!(mock_status(1).code(), Some(1));
}
```

**Never use** `std::process::ExitStatus::from_raw(..)` directly in tests/benches - CI will reject it.

#### Pre-Commit Hook (Optional)

To catch policy violations before pushing, install the pre-commit hook:

```bash
# Option 1: Copy hook (manual updates needed)
cp .ci/hooks/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit

# Option 2: Symlink hook (auto-updates with git pull)
ln -sf ../../.ci/hooks/pre-commit .git/hooks/pre-commit
```

#### Manual Policy Check

Run the policy check locally anytime:

```bash
./.ci/scripts/check-from-raw.sh
```

## Project Structure

- **`crates/perl-parser/`** - Core parser implementation and LSP providers
- **`crates/perl-lsp/`** - LSP server binary and CLI
- **`crates/perl-dap/`** - Debug Adapter Protocol implementation
- **`crates/perl-lexer/`** - Tokenization and lexical analysis
- **`crates/perl-corpus/`** - Test corpus and property-based testing
- **`xtask/`** - Advanced testing and development tools
- **`docs/`** - Comprehensive project documentation

## Testing Guidelines

### Writing Tests

- Place tests in `tests/` directory or inline with `#[cfg(test)]`
- Use descriptive test names that explain what is being tested
- Test both success and failure cases
- Add edge case tests for parser improvements

### Running Tests

```bash
# Fast parallel testing with nextest
cargo nextest run

# Traditional test runner
cargo test

# Test specific crate
cargo test -p perl-parser

# Test with verbose output
cargo test -- --nocapture

# Run determinism checks
cargo test --test determinism_test
```

### Dead Code Detection

We use `cargo-machete` and `clippy` to identify unused dependencies and code.

#### Check for dead code locally
```bash
just dead-code
```

#### Handling False Positives
If a dependency is detected as unused but is actually required (e.g., used only via macros or in tests), add it to the ignore list in the crate's `Cargo.toml`:

```toml
[package.metadata.cargo-machete]
ignored = ["crate-name"]
```

For unreachable code warnings from clippy, use `#[allow(dead_code)]` with a comment explaining why it should be preserved.

### Documentation

- **Public APIs** must have documentation comments (`///`)
- **Modules** should have module-level documentation (`//!`)
- **Complex functions** should include examples in doc comments
- Run `cargo doc --no-deps --open` to view generated docs

## Dependency Management

The project uses **Dependabot** for automated dependency updates. Dependabot PRs are created weekly and should be reviewed according to the update type:

- **Patch updates (x.y.Z)** - Can be merged quickly if CI passes
- **Minor updates (x.Y.0)** - Require changelog review and testing
- **Major updates (X.0.0)** - Require deep review, migration planning, and comprehensive testing

For handling Dependabot PRs:

```bash
# View all dependency PRs
gh pr list --label "dependencies"

# Merge passing patch updates
gh pr list --author "app/dependabot" --search "status:success" --json number --jq '.[].number' | \
  xargs -I {} gh pr merge {} --auto --squash
```

See **[Dependency Management Guide](./docs/DEPENDENCY_MANAGEMENT.md)** for complete details on:
- Update strategy and grouping
- Review process by update type
- Auto-merge configuration
- Security update handling
- Troubleshooting common issues

For quick reference, see **[Dependency Quick Reference](./docs/DEPENDENCY_QUICK_REFERENCE.md)**.

## Getting Help

- **Issues:** Browse existing issues or create a new one
- **Discussions:** Use GitHub Discussions for questions and ideas
- **Documentation:** Check `docs/` for comprehensive guides
- **Code Examples:** See `examples/` and test files for usage patterns

## Code of Conduct

We follow the Rust Code of Conduct. Please be respectful and constructive in all interactions.

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (typically MIT or Apache-2.0).

---

Thank you for contributing to Perl LSP! =�
