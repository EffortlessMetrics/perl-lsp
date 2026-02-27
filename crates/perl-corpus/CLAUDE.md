# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

- **Tier**: 7 (testing/legacy crate)
- **Version**: 0.9.1
- **Purpose**: Test corpus management, property-based generators, and edge case fixtures for Perl parsers and LSP/DAP testing.

## Commands

```bash
cargo build -p perl-corpus               # Build library and binary
cargo test -p perl-corpus                # Run tests
cargo test -p perl-corpus --features ci-fast  # Run tests (skip slow property tests)
cargo run -p perl-corpus -- --help       # Show CLI help
cargo run -p perl-corpus -- lint --corpus test_corpus  # Lint corpus metadata
cargo run -p perl-corpus -- index --corpus test_corpus # Build index files
cargo run -p perl-corpus -- stats --corpus test_corpus --detailed # Show statistics
cargo run -p perl-corpus -- gen program --count 10 --seed 42  # Generate test code
cargo clippy -p perl-corpus              # Lint
cargo doc -p perl-corpus --open          # View documentation
```

## Architecture

### Dependencies

- `proptest` - Property-based testing strategies and test runners
- `rand` - Seeded random generation for deterministic codegen
- `serde`, `serde_json` - Corpus index serialization
- `regex` - Section delimiter and metadata parsing
- `glob` - Corpus file discovery
- `clap` - CLI argument parsing
- `chrono` - Timestamps in coverage reports
- `once_cell` - Lazy-initialized regex patterns
- `anyhow` - Error handling

### Key Types and Modules

| Type/Module | Location | Purpose |
|-------------|----------|---------|
| `Section` | `meta.rs` | Parsed corpus section with id, title, tags, flags, body, line number |
| `CorpusPaths` / `CorpusFile` / `CorpusLayer` | `files.rs` | Corpus file discovery with layer classification (TestCorpus, Fuzz) |
| `EdgeCase` / `EdgeCaseGenerator` | `cases.rs` | 100 static edge case fixtures with tag filtering and deterministic sampling |
| `ComplexDataStructureCase` | `cases.rs` | 32 static complex data structure samples for DAP variable inspection |
| `ContinueRedoCase` | `continue_redo.rs` | Continue/redo loop control fixtures with parse expectation flags |
| `FormatStatementCase` / `FormatStatementGenerator` | `format_statements.rs` | Format/formline statement fixtures |
| `GlobExpressionCase` / `GlobExpressionGenerator` | `glob_expressions.rs` | Glob and diamond operator fixtures |
| `TieInterfaceCase` | `tie_interface.rs` | Tie/untie/tied mechanism fixtures |
| `CodegenOptions` / `StatementKind` | `codegen.rs` | Randomized Perl code generation with 21 statement categories |
| `LintConfig` / `LintResult` | `lint.rs` | Corpus validation (duplicate IDs, unknown tags/flags, section limits) |
| `gen::*` | `gen/` | 21 proptest strategy modules (regex, heredoc, qw, quote_like, whitespace, control_flow, format_statements, glob, tie, io, filetest, builtins, list_ops, declarations, object_oriented, expressions, ambiguity, sigils, phasers, special_vars, program) |

### Public API (lib.rs)

- `parse_file(path)` / `parse_dir(dir)` - Parse corpus `.txt` files into `Section` vectors
- `find_by_tag(sections, tag)` / `find_by_flag(sections, flag)` - Filter sections
- `generate_perl_code()` / `generate_perl_code_with_seed(n, seed)` / `generate_perl_code_with_options(opts)` - Randomized codegen
- `edge_cases()` / `complex_data_structure_cases()` - Static fixture accessors
- `get_corpus_files()` / `get_all_test_files()` / `get_test_files()` / `get_fuzz_files()` - File discovery
- Specialized fixture accessors: `continue_redo_cases()`, `format_statement_cases()`, `glob_expression_cases()`, `tie_interface_cases()`

### CLI Subcommands (bin/main.rs)

| Subcommand | Purpose |
|------------|---------|
| `lint` | Validate corpus metadata (IDs, tags, flags, section limits) |
| `index` | Generate `_index.json`, `_tags.json`, `COVERAGE_SUMMARY.md` |
| `stats` | Print corpus statistics (files, sections, tags, flags) |
| `gen <generator>` | Generate test cases using proptest strategies (21 generators) |

### Features

| Feature | Purpose |
|---------|---------|
| `ci-fast` | Skip slow/flaky property tests in CI |

## Usage Examples

```rust
use perl_corpus::{parse_dir, find_by_tag, EdgeCaseGenerator, generate_perl_code_with_seed};
use perl_corpus::{CodegenOptions, StatementKind};

// Parse and query corpus files
let sections = parse_dir(std::path::Path::new("test_corpus")).unwrap();
let regex_tests = find_by_tag(&sections, "regex");

// Generate deterministic Perl code
let code = generate_perl_code_with_seed(10, 42);

// Customize codegen
let options = CodegenOptions {
    statements: 50,
    seed: 42,
    ensure_coverage: true,
    kinds: vec![StatementKind::Regex, StatementKind::Heredoc],
    ..Default::default()
};

// Edge case fixtures
let all = EdgeCaseGenerator::all_cases();            // 100 cases
let by_tag = EdgeCaseGenerator::by_tag("heredoc");
let sample = EdgeCaseGenerator::sample(42);           // deterministic
```

## Important Notes

- The `gen` module is accessed as `r#gen` in Rust source (reserved keyword)
- `PERL_CORPUS_ROOT` env var overrides corpus root discovery
- Regex patterns use `Option<Regex>` with `once_cell::Lazy` for graceful degradation
- Edge case arrays are fixed-size (`[EdgeCase; 100]`, `[ComplexDataStructureCase; 32]`) for compile-time guarantees
- Dev-dependencies include `perl-parser` and `perl-tdd-support` for integration tests
