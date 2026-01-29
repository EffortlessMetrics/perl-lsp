# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-corpus` is a **Tier 7 testing/benchmark resource crate** providing test corpus management and generators for Perl parsers.

**Purpose**: Test corpus management and generators for Perl parsers — provides test data, property-based testing, and benchmark utilities.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-corpus               # Build this crate
cargo test -p perl-corpus                # Run tests
cargo run -p perl-corpus -- --help       # Run CLI tool
cargo clippy -p perl-corpus              # Lint
cargo doc -p perl-corpus --open          # View documentation
```

### CLI Tool

```bash
# Generate test corpus
perl-corpus generate --output corpus/

# Validate corpus
perl-corpus validate corpus/

# Run property tests
perl-corpus proptest --iterations 1000
```

## Architecture

### Dependencies

- `proptest` - Property-based testing
- `rand` - Random generation
- `serde`, `serde_json` - Corpus serialization
- `regex` - Pattern matching
- `glob` - File discovery
- `chrono` - Timestamps
- `clap` - CLI parsing

### Features

| Feature | Purpose |
|---------|---------|
| `ci-fast` | Skip slow tests in CI |

### Key Components

| Component | Purpose |
|-----------|---------|
| `CorpusManager` | Manage test corpus files |
| `Generator` | Generate random Perl code |
| `Validator` | Validate corpus files |
| `Proptest` | Property-based test strategies |

### Corpus Structure

```
test_corpus/
├── valid/           # Valid Perl files
│   ├── basic/       # Basic constructs
│   ├── complex/     # Complex cases
│   └── edge/        # Edge cases
├── invalid/         # Intentionally invalid Perl
├── generated/       # Property-test generated
└── manifest.json    # Corpus metadata
```

## Usage

### Property-Based Testing

```rust
use perl_corpus::proptest::{perl_expression, perl_statement};
use proptest::prelude::*;

proptest! {
    #[test]
    fn parse_any_expression(expr in perl_expression()) {
        // Parser should handle any generated expression
        let _ = parse(&expr);
    }

    #[test]
    fn parse_any_statement(stmt in perl_statement()) {
        let result = parse(&stmt);
        // Should either parse successfully or report clean error
        assert!(result.is_ok() || result.error().is_recoverable());
    }
}
```

### Corpus Validation

```rust
use perl_corpus::CorpusManager;

let corpus = CorpusManager::load("test_corpus/")?;

for file in corpus.valid_files() {
    let result = parse(&file.content);
    assert!(result.is_ok(), "Valid file should parse: {}", file.path);
}
```

## Important Notes

- Large test corpus — use `ci-fast` feature in CI to skip slow tests
- Property-based tests can find edge cases
- Generated corpus helps test parser robustness
- See `test_corpus/` and `tree-sitter-perl/test/corpus/` for test data
