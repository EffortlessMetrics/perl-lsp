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
- **Opt-in CI labels** for heavy jobs (`ci:bench`, `ci:mutation`, `ci:strict`, `ci:mac`)
- **Build optimizations** (lean flags, nextest configuration)
- **Troubleshooting tips** for common CI issues

### Quick CI Tips

- All PRs run **format checks**, **clippy**, and **core tests** automatically
- Tests use **nextest** with lean build flags for faster, reliable execution
- Add `ci:bench` label to run performance benchmarks
- Add `ci:strict` label for pedantic clippy checks
- Add `ci:mac` label if your changes affect macOS

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

## Documentation

- **Public APIs** must have documentation comments (`///`)
- **Modules** should have module-level documentation (`//!`)
- **Complex functions** should include examples in doc comments
- Run `cargo doc --no-deps --open` to view generated docs

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

Thank you for contributing to Perl LSP! =€
