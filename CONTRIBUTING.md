# Contributing to tree-sitter-perl-rs

Thank you for your interest in contributing to tree-sitter-perl-rs! This document provides guidelines and information for contributors.

## 🚀 Quick Start

### Prerequisites

- **Rust 1.70+** (stable channel)
- **Git**
- **Cargo** (comes with Rust)

### Development Setup

```bash
# Fork and clone the repository
git clone https://github.com/YOUR_USERNAME/tree-sitter-perl-rs.git
cd tree-sitter-perl-rs

# Install dependencies
cargo build

# Run tests to ensure everything works
cargo xtask test
```

## 🏗️ Project Structure

```
tree-sitter-perl-rs/
├── crates/tree-sitter-perl-rs/     # Main Rust implementation
│   ├── src/
│   │   ├── lib.rs                  # Public API
│   │   ├── scanner/                # Rust scanner implementation
│   │   │   ├── mod.rs              # Scanner module
│   │   │   ├── rust_scanner.rs     # Rust-native scanner
│   │   │   └── types.rs            # Scanner types
│   │   ├── unicode.rs              # Unicode utilities
│   │   └── tests.rs                # Test suite
│   └── Cargo.toml
├── xtask/                          # Build automation
├── tree-sitter-perl/               # Legacy C implementation
├── benches/                        # Performance benchmarks
└── .github/workflows/              # CI/CD pipelines
```

## 🔧 Development Workflow

### 1. Create a Feature Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/issue-description
```

### 2. Make Your Changes

Follow these guidelines:

- **Write tests first** (TDD approach)
- **Keep changes focused** - one feature/fix per branch
- **Follow Rust conventions** - use `cargo fmt` and `cargo clippy`
- **Update documentation** for API changes

### 3. Test Your Changes

```bash
# Run all tests
cargo xtask test

# Run specific test categories
cargo test --lib                    # Unit tests
cargo xtask corpus                  # Corpus tests
cargo xtask highlight               # Highlight tests

# Check code quality
cargo xtask check --all
cargo xtask fmt --check

# Run benchmarks (if applicable)
cargo xtask bench
```

### 4. Commit Your Changes

Use conventional commit messages:

```bash
git commit -m "feat: add new scanner feature"
git commit -m "fix: resolve parsing issue with heredoc"
git commit -m "docs: update API documentation"
git commit -m "test: add test for edge case"
```

### 5. Push and Create Pull Request

```bash
git push origin feature/your-feature-name
```

Then create a pull request on GitHub.

## 📝 Code Style Guidelines

### Rust Code

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Address all `cargo clippy` warnings
- Write comprehensive documentation

### Test Code

- Use descriptive test names
- Follow AAA pattern (Arrange, Act, Assert)
- Add property-based tests for complex logic
- Include edge cases and error conditions

### Documentation

- Document all public APIs
- Include usage examples
- Update README.md for user-facing changes
- Keep CHANGELOG.md updated

## 🧪 Testing Guidelines

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_name() {
        // Arrange
        let input = "test input";
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert!(result.is_ok());
    }
}
```

### Property Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_property_name(input in "[a-zA-Z0-9_]+") {
        // Property-based test
        assert!(function_under_test(&input).is_ok());
    }
}
```

### Integration Tests

```rust
#[test]
fn test_corpus_file() {
    let test_cases = parse_corpus_file("test/corpus/feature").unwrap();
    
    for test_case in test_cases {
        let result = run_corpus_test_case(&test_case);
        assert!(result.is_ok(), "Test case failed: {}", test_case.name);
    }
}
```

## 🔍 Code Review Process

### Before Submitting

1. **Self-review** your changes
2. **Run all tests** locally
3. **Check code quality** with clippy and fmt
4. **Update documentation** if needed
5. **Test with real Perl code** if applicable

### Pull Request Guidelines

- **Clear title** describing the change
- **Detailed description** of what and why
- **Link to issues** if applicable
- **Include test results** if relevant
- **Screenshots** for UI changes (if applicable)

### Review Checklist

- [ ] Code follows Rust conventions
- [ ] Tests are comprehensive and pass
- [ ] Documentation is updated
- [ ] No performance regressions
- [ ] Error handling is appropriate
- [ ] Security considerations addressed

## 🐛 Bug Reports

### Before Reporting

1. **Check existing issues** for duplicates
2. **Try the latest version** from main branch
3. **Reproduce the issue** with minimal example

### Bug Report Template

```markdown
**Description**
Brief description of the issue

**Steps to Reproduce**
1. Step 1
2. Step 2
3. Step 3

**Expected Behavior**
What should happen

**Actual Behavior**
What actually happens

**Environment**
- OS: [e.g., Ubuntu 22.04]
- Rust version: [e.g., 1.75.0]
- tree-sitter-perl-rs version: [e.g., 0.1.0]

**Additional Context**
Any other relevant information
```

## 💡 Feature Requests

### Before Requesting

1. **Check existing features** and roadmap
2. **Consider use cases** and impact
3. **Think about implementation** complexity

### Feature Request Template

```markdown
**Problem Statement**
What problem does this feature solve?

**Proposed Solution**
How should this feature work?

**Use Cases**
Specific examples of how this would be used

**Implementation Considerations**
Any technical considerations or alternatives
```

## 🚀 Release Process

### Version Bumping

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

### Release Checklist

- [ ] All tests pass
- [ ] Documentation is updated
- [ ] CHANGELOG.md is updated
- [ ] Version is bumped in Cargo.toml
- [ ] Release notes are prepared
- [ ] CI/CD pipeline passes

## 🤝 Community Guidelines

### Communication

- **Be respectful** and inclusive
- **Ask questions** when unsure
- **Provide constructive feedback**
- **Help others** when possible

### Recognition

Contributors will be recognized in:
- **README.md** acknowledgments
- **CHANGELOG.md** for significant contributions
- **GitHub contributors** page

## 📚 Resources

### Documentation

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Tree-sitter Documentation](https://tree-sitter.github.io/tree-sitter/)

### Tools

- [cargo-edit](https://github.com/killercup/cargo-edit) - Edit Cargo.toml
- [cargo-watch](https://github.com/watchexec/cargo-watch) - Watch for changes
- [cargo-audit](https://github.com/RustSec/cargo-audit) - Security audit

## 🆘 Getting Help

- **GitHub Issues**: For bugs and feature requests
- **GitHub Discussions**: For questions and general discussion
- **Code Review**: Ask questions in pull request comments

## 📄 License

By contributing to tree-sitter-perl-rs, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to tree-sitter-perl-rs! 🎉 