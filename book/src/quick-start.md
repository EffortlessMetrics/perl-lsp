# Quick Start

Get perl-lsp up and running in 5 minutes!

## Prerequisites

- Rust toolchain (1.89+)
- Perl 5 installation (for testing)
- Your favorite code editor with LSP support

## Installation

### From crates.io (Recommended)

```bash
cargo install perl-lsp
```

### From Source

```bash
git clone https://github.com/EffortlessMetrics/tree-sitter-perl.git
cd tree-sitter-perl
cargo build --release -p perl-lsp
```

The binary will be at `target/release/perl-lsp`.

## Verify Installation

```bash
perl-lsp --version
```

You should see the version information displayed.

## Basic Usage

### Standalone Mode

Run the LSP server in stdio mode:

```bash
perl-lsp --stdio
```

### Editor Integration

Configure your editor to use `perl-lsp` as the Perl language server. See [Editor Setup](./getting-started/editor-setup.md) for detailed instructions for:

- Visual Studio Code
- Neovim
- Emacs
- Sublime Text
- Other LSP-compatible editors

## Quick Test

Create a simple Perl file:

```perl
#!/usr/bin/env perl
use strict;
use warnings;

sub greet {
    my ($name) = @_;
    print "Hello, $name!\n";
}

greet("World");
```

Open this file in your configured editor. You should see:

- Syntax highlighting
- Autocompletion for built-in functions
- Go-to-definition for the `greet` function
- Hover documentation

## Common Commands

```bash
# Build all crates
cargo build --workspace

# Run tests
cargo test

# Build LSP server only
cargo build -p perl-lsp --release

# Build parser library
cargo build -p perl-parser --release

# Run specific tests
cargo test -p perl-parser
cargo test -p perl-lsp
```

## Troubleshooting

### Server Not Starting

Check the log output:

```bash
RUST_LOG=debug perl-lsp --stdio
```

### Editor Not Connecting

Ensure your editor's LSP client is properly configured. Check:

1. The path to the `perl-lsp` binary
2. The command-line arguments (`--stdio`)
3. File type associations (`.pl`, `.pm`, `.t`)

### Performance Issues

For large workspaces, consider:

1. Adjusting thread settings
2. Excluding large directories from indexing
3. See [Performance Guide](./advanced/performance-guide.md) for optimization tips

## Next Steps

Now that you have perl-lsp running:

1. **Explore Features**: Read about [LSP Features](./user-guides/lsp-features.md)
2. **Configure**: Learn about [Configuration Options](./getting-started/configuration.md)
3. **Debugging**: Set up the [Debug Adapter](./user-guides/debugging.md)
4. **Contribute**: Check out the [Contributing Guide](./developer/contributing.md)

## Get Help

- [Troubleshooting Guide](./user-guides/troubleshooting.md)
- [Known Limitations](./user-guides/known-limitations.md)
- [GitHub Issues](https://github.com/EffortlessMetrics/tree-sitter-perl/issues)

## Useful Resources

- [LSP Specification](https://microsoft.github.io/language-server-protocol/)
- [Perl Documentation](https://perldoc.perl.org/)
- [Project README](https://github.com/EffortlessMetrics/tree-sitter-perl)

Happy coding with perl-lsp!
