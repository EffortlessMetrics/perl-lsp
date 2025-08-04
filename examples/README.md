# Perl Language Server Examples

This directory contains example programs demonstrating how to use the Perl parser and Language Server.

## Examples

### parse_file.rs
A simple command-line tool that parses a Perl file and outputs the AST in S-expression format.

```bash
cargo run --example parse_file -- test.pl
```

### lsp_client.rs
Demonstrates how to interact with the Perl Language Server programmatically.

```bash
# First, ensure perl-lsp is in your PATH
cargo install --path crates/perl-parser

# Then run the example
cargo run --example lsp_client
```

## Test Files

The `perl/` subdirectory contains various Perl files for testing:

- `simple.pl` - Basic Perl syntax
- `complex.pl` - Advanced features and edge cases
- `unicode.pl` - Unicode identifiers and strings
- `regex.pl` - Regular expression examples
- `oop.pl` - Object-oriented Perl

## Running Tests

```bash
# Parse all test files
for file in examples/perl/*.pl; do
    echo "=== $file ==="
    cargo run --example parse_file -- "$file"
done
```