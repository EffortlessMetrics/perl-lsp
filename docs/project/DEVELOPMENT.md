# Development Guide

> **Pure Rust Perl Parser Development Guide**

This document provides guidelines and instructions for contributors working on the Pure Rust Perl Parser built with Pest.

---

## 🚀 Quick Start

### Prerequisites
- **Rust**: 1.92+ (stable)
- **Cargo**: Latest stable

### Development Setup
```bash
# Clone the repository
git clone <repository-url>
cd tree-sitter-perl

# Build the Pure Rust parser (default)
cargo build

# Run tests
cargo test --features pure-rust

# Run benchmarks
cargo bench --features pure-rust
```

---

## 📁 Project Structure

```
tree-sitter-perl/
├── crates/tree-sitter-perl-rs/   # Pure Rust Perl Parser
│   ├── src/
│   │   ├── grammar.pest          # Pest PEG grammar for Perl 5
│   │   ├── pure_rust_parser.rs   # Main parser implementation
│   │   ├── edge_case_handler.rs  # Edge case handling system
│   │   ├── tree_sitter_adapter.rs # S-expression output
│   │   └── lib.rs                # Public API
│   └── tests/                    # Integration tests
├── benchmarks/                   # Performance benchmarks
├── xtask/                        # Development automation
├── docs/                         # Architecture documentation
└── tree-sitter-perl/             # Legacy C implementation (reference only)
```

---

## 🔧 Common Development Tasks

### Running the Parser
```bash
# Parse a Perl file and output S-expression
cargo run --features pure-rust --bin parse-rust -- script.pl

# Parse from stdin
echo 'print "Hello"' | cargo run --features pure-rust --bin parse-rust -- -
```

### Testing
```bash
# Run all tests
cargo xtask test

# Run corpus tests
cargo xtask corpus

# Run edge case tests
cargo xtask test-edge-cases

# Run specific test
cargo test test_heredoc_parsing
```

### Benchmarking
```bash
# Run benchmarks
cargo bench --features pure-rust

# Compare with legacy implementation
cargo xtask compare
```

---

## 🛠 Development Workflow

### 1. Grammar Changes
To modify the Perl grammar:
1. Edit `crates/tree-sitter-perl-rs/src/grammar.pest`
2. Update corresponding AST nodes in `pure_rust_parser.rs`
3. Update the `build_node()` method
4. Add tests for new constructs

### 2. Adding Features
```rust
// 1. Add new rule to grammar.pest
new_feature = { "keyword" ~ expression }

// 2. Add AST node
#[derive(Debug, Clone)]
pub struct NewFeature {
    pub keyword: String,
    pub expr: Box<AstNode>,
}

// 3. Update build_node() in pure_rust_parser.rs
Rule::new_feature => {
    // Build AST node
}

// 4. Add tests
#[test]
fn test_new_feature() {
    let parser = PureRustPerlParser::new();
    let ast = parser.parse("keyword expression").unwrap();
    // Assert expectations
}
```

### 3. Edge Case Handling
For complex Perl edge cases:
1. Add detection in `edge_case_handler.rs`
2. Implement recovery strategy
3. Add diagnostic information
4. Update documentation in `docs/reference/EDGE_CASES.md`

---

## Agent-Driven Development Workflow

The project uses an orchestrator-agent pattern for large improvements. Multiple
agents work in parallel across isolated worktrees, each tackling a focused task,
while a central orchestrator plans, monitors, and coordinates merges.

### Roles

| Role | Scope | Typical work |
|------|-------|-------------|
| **Orchestrator** (main thread) | Plans work, launches agents, monitors PRs, coordinates merges | Reading baselines, triaging errors, routing swarm slices |
| **Worker agents** (worktrees) | Focused work in isolation | Parser fix, test addition, doc update, cleanup |
| **PR agents** | Validate and publish each worktree's changes | Running `ci-gate`, creating PRs, fixing CI failures |

### Daily Development Pattern

#### 1. Check current state
```bash
just health
just corpus-sweep  # parser coverage baseline
```

#### 2. Identify work items
```bash
# Read .ci/parser-corpus-baseline.json for error buckets
# Read docs/project/PARSER_EDGE_CASE_ROADMAP.md for known issues
```

#### 3. Start a focused swarm lane
```bash
/swarm parser        # or: tests, cleanup, improve
```

Each swarm slice creates one worktree per PR-shaped task and starts a focused
worker inside it. `/wave` still exists as a compatibility shim for older docs,
but it is no longer the primary entrypoint.

#### 4. Monitor and PR
```bash
gh pr list --state open --json number,title,statusCheckRollup
```

Let the reviewer/ops flow drain PRs. `/bulk-pr` is now a legacy/manual sweep
tool for older wave-style batches, not the default publishing path.

#### 5. Fix CI failures
```bash
# Check open PR status:
gh pr list --state open --json number,statusCheckRollup

# Launch an agent per failing PR to diagnose and fix
```

#### 6. Merge and ratchet
Merge PRs sequentially, then lock the new baselines:
```bash
/corpus-ratchet      # update baselines
```

### Task Lists for Agent Coordination

Use TodoWrite/TaskCreate to track:
- Which agents are launched and their worktree paths
- Which PRs are created and their CI status
- Which worktrees still need PRing
- Post-merge ratchet tasks

### Merge Order

Parser fix PRs should merge in dependency order:

1. **Infrastructure** PRs (xtask, CI tooling) first
2. **Parser fixes** (may unlock each other)
3. **Test additions** (no conflicts between test-only PRs)
4. **Documentation** (no code conflicts)
5. **Cleanup** (after all code changes merged)

After each parser fix merge, re-measure and lock the new baseline:
```bash
just corpus-sweep        # measure improvement
just corpus-sweep-update # lock new baseline
just common-corpus-check # verify pinned modules still clean
```

---

## 📝 Code Style

### Rust Guidelines
- Use `rustfmt` for formatting: `cargo fmt`
- Run `clippy` for lints: `cargo clippy`
- Write doc comments for public APIs
- Use descriptive variable names
- Prefer `Result<T, E>` for error handling

### Grammar Guidelines
- Keep grammar rules simple and composable
- Use meaningful rule names
- Add comments for complex patterns
- Test each rule independently

---

## 🧪 Testing Strategy

### Unit Tests
Test individual parser components:
```rust
#[test]
fn test_variable_parsing() {
    let result = parse_variable("$foo");
    assert_eq!(result.name, "foo");
}
```

### Integration Tests
Test complete parsing scenarios:
```rust
#[test]
fn test_complex_script() {
    let script = include_str!("../test/complex.pl");
    let ast = parser.parse(script).unwrap();
    verify_ast_structure(&ast);
}
```

### Edge Case Tests
Test Perl's tricky syntax:
```rust
#[test]
fn test_heredoc_in_eval() {
    let code = r#"eval "print <<EOF\nHello\nEOF\n""#;
    let result = parser.parse(code);
    assert!(result.is_ok());
}
```

---

## 🐛 Debugging

### Parser Debugging
```bash
# Enable debug output
RUST_LOG=debug cargo run --features pure-rust --bin parse-rust -- script.pl

# Use AST output for debugging
cargo run --features pure-rust --bin parse-rust -- --ast script.pl
```

### Common Issues
1. **Stack overflow**: Use iterative parser for deeply nested code
2. **Performance**: Check for backtracking in grammar rules
3. **Edge cases**: Use edge case handler for diagnostics

---

## 📚 Resources

### Documentation
- [Pest Documentation](https://pest.rs/book/)
- [Perl Language Reference](https://perldoc.perl.org/perlsyn)
- [Tree-sitter Docs](https://tree-sitter.github.io/)

### Architecture
- `ARCHITECTURE.md`: System design
- `docs/reference/EDGE_CASES.md`: Edge case handling
- `CLAUDE.md`: AI assistant guidance

---

## 🤝 Contributing

### Pull Request Process
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run `cargo test` and `cargo fmt`
6. Submit PR with clear description

### Code Review Checklist
- [ ] Tests pass
- [ ] Code is formatted
- [ ] Documentation updated
- [ ] No performance regressions
- [ ] Edge cases handled

---

## 🚀 Advanced Topics

### Performance Optimization
- Use `cargo bench` to measure impact
- Profile with `perf` or `flamegraph`
- Minimize allocations in hot paths
- Consider caching for repeated patterns

### Grammar Optimization
- Avoid left recursion
- Use atomic rules for common patterns
- Order alternatives by frequency
- Minimize backtracking

---

*For questions or discussions, please open an issue on GitHub.*
