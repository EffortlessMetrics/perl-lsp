# Installation

## From crates.io

The recommended way to install perl-lsp is via crates.io:

```bash
cargo install perl-lsp
```

## From Source

Clone the repository and build:

```bash
git clone https://github.com/EffortlessMetrics/tree-sitter-perl.git
cd tree-sitter-perl
cargo build --release -p perl-lsp
```

The binary will be located at `target/release/perl-lsp`.

## System Requirements

- Rust toolchain 1.70 or later
- Perl 5 installation (for testing)
- Sufficient memory for workspace indexing (typically 512MB+)

## Verifying Installation

Check that perl-lsp is properly installed:

```bash
perl-lsp --version
```

## Next Steps

- [Editor Setup](./editor-setup.md)
- [Configuration](./configuration.md)
- [First Steps](./first-steps.md)
