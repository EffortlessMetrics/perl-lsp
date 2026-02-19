# Testing Heredoc Features

This document describes how to test the heredoc parsing features to ensure they don't regress.

## Quick Start

Run all heredoc tests:
```bash
cargo xtask test-heredoc --release
```

Or use cargo xtask:
```bash
cargo xtask test --suite heredoc
```

## Test Suites

We have three main test suites for heredocs:

### 1. `heredoc_missing_features_tests.rs`
Tests edge cases and previously missing features:
- Empty heredocs
- Heredocs in hash values (multi-line statements)
- Mixed quote types (single, double, backtick)
- Indented heredocs
- Special terminators (keywords, numbers, regex chars)
- And more...

### 2. `heredoc_integration_tests.rs`
Integration tests combining heredocs with other features:
- Basic heredocs
- Interpolated heredocs
- Multiple heredocs on one line
- Heredocs with division operator

### 3. `comprehensive_heredoc_tests.rs`
Comprehensive test suite ensuring all improvements work:
- Multi-line statement heredocs
- Statement boundary tracking
- Builtin list operators (print, say, warn, die)
- Complex nested structures

## Key Features Tested

### 1. Multi-line Statement Heredocs
```perl
my %config = (
    name => "Test",
    description => <<'DESC'
);
This is a long description
that spans multiple lines
DESC
```

### 2. Builtin List Operators
```perl
print $x;                    # Without parentheses
print $a, $b, $c;           # Multiple arguments
say "Hello, world!";        # say builtin
warn "Something wrong";     # warn builtin
die "Fatal error";          # die builtin
```

### 3. Statement Boundary Tracking
The parser correctly identifies where statements end, even in complex cases:
```perl
my $result = func(
    arg1,
    func2(
        <<'EOF'
    ),
    arg3
);
content
EOF
```

## Running Individual Tests

To run a specific test file:
```bash
cargo test --features pure-rust --release --test heredoc_missing_features_tests
```

To run a specific test function:
```bash
cargo test --features pure-rust --release test_heredoc_as_hash_value
```

## Debug vs Release

Note: Some tests may experience stack overflow in debug builds due to deep recursion in the parser. Always test in release mode for production validation:
```bash
cargo test --features pure-rust --release
```

## Adding New Tests

When adding new heredoc features, add tests to `comprehensive_heredoc_tests.rs` to ensure they're included in the regression suite.

## CI Integration

The heredoc tests should be run in CI to prevent regression. Add to your CI workflow:
```yaml
- name: Test Heredocs
  run: cargo xtask test-heredoc --release
```

Or use cargo xtask:
```yaml
- name: Test Heredocs
  run: cargo xtask test --suite heredoc
```