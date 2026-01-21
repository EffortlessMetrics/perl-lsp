# perl-corpus

Reusable generators for Perl test corpora: proptest strategies, fixtures, and edge cases.

## Usage

```rust
use perl_corpus::{
    complex_data_structure_cases, generate_perl_code, generate_perl_code_with_options,
    get_all_test_files, get_corpus_files, CodegenOptions, CorpusLayer, EdgeCaseGenerator,
    StatementKind,
};

// Generate random valid Perl code
let code = generate_perl_code();

// Customize code generation coverage
let mut options = CodegenOptions::default();
options.statements = 50;
options.ensure_coverage = true;
options.kinds = vec![
    StatementKind::Expressions,
    StatementKind::ListOps,
    StatementKind::Builtins,
    StatementKind::Regex,
];
let code = generate_perl_code_with_options(options);

// Generate edge cases for testing
let edge_cases = EdgeCaseGenerator::all_cases();
let regex_or_heredoc = EdgeCaseGenerator::by_tags_any(&["regex", "heredoc"]);
let regex_code = EdgeCaseGenerator::by_tags_all(&["regex", "regex-code"]);

// Discover local corpus files for integration testing
let files = get_all_test_files();

// Inspect corpus files with layer metadata
let layered = get_corpus_files();
let fuzz_only: Vec<_> = layered
    .iter()
    .filter(|file| file.layer == CorpusLayer::Fuzz)
    .collect();

// Retrieve complex data structure samples for DAP variable rendering
let cases = complex_data_structure_cases();
```

## Features

- Property-based testing strategies via proptest
- Edge case fixtures with tags and IDs
- Random code generation helpers
- Local corpus file discovery (test_corpus + fuzz fixtures)
- Layered corpus file metadata (test corpus vs fuzz)
- Generators for heredoc, quote-like, regex (advanced patterns), expressions, whitespace, loop control, format, glob, tie, I/O, declarations, phaser blocks
- List operators (map/grep/sort) including empty-block coverage
- Filetest operator coverage (stacked and handle-based checks)
- Built-in call coverage (pack/unpack, split/join, printf/system, substr/index/length, bless/ref, caller/wantarray)
- Sigil-heavy variable and dereference generator
- Expanded edge cases: POD, v-strings, prototypes, postfix control flow, goto labels, flip-flop operators, AUTOLOAD/DESTROY, overload, symbolic references, DATA/END sections, source filters, Inline::C/XS/FFI heredocs, bareword filehandles, lvalue substr assignments, SUPER:: dispatch, mro pragmas, y/// transliteration, variable attributes, utf8/unicode escapes, and state/local/our declarations

## CLI

```bash
# Lint and index corpus metadata
perl-corpus lint --corpus tree-sitter-perl/test/corpus
perl-corpus index --corpus tree-sitter-perl/test/corpus

# Show corpus statistics
perl-corpus stats --corpus tree-sitter-perl/test/corpus --detailed

# Generate targeted samples
perl-corpus gen --generator list-ops --count 5
perl-corpus gen --generator filetest --count 5
perl-corpus gen --generator builtins --count 5
```

## License

Apache 2.0 OR MIT
