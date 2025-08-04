# Perl Language Server for Visual Studio Code

Advanced Perl language support powered by the tree-sitter-perl v3 parser with full Language Server Protocol (LSP) features.

## Features

### 🚀 Core Features

- **Syntax Highlighting** - Enhanced TextMate grammar for accurate highlighting
- **Real-time Diagnostics** - Instant syntax error detection and reporting
- **Code Formatting** - Format your code with Perl::Tidy integration
- **Auto-completion** - Context-aware completions for variables, functions, and keywords
- **Signature Help** - Parameter hints while typing function calls
- **Go to Definition** - Jump to variable and subroutine definitions
- **Find References** - Find all usages of variables and subroutines
- **Document Symbols** - Outline view of packages, subroutines, and variables
- **Code Actions** - Quick fixes for common issues
- **Hover Information** - Documentation on hover
- **Rename Symbol** - Safely rename variables across your codebase

### 🎯 Parser Features

This extension uses the **tree-sitter-perl v3 parser** which provides:

- **100% Perl 5 syntax coverage** including all edge cases
- **4-19x faster** than traditional parsers
- Support for modern Perl features (signatures, try/catch, class/method)
- Accurate handling of complex syntax (regex delimiters, indirect object notation)

## Requirements

### Required
- Visual Studio Code 1.74.0 or higher

### Optional (for formatting)
- Perl::Tidy for code formatting support
  ```bash
  # Install via CPAN
  cpan Perl::Tidy
  
  # Or via system package manager
  apt-get install perltidy    # Debian/Ubuntu
  yum install perltidy         # RedHat/Fedora
  brew install perltidy        # macOS
  ```

## Installation

1. Install from the Visual Studio Code Marketplace:
   - Open VS Code
   - Go to Extensions (Ctrl+Shift+X)
   - Search for "Perl Language Server"
   - Click Install

2. Or install from command line:
   ```bash
   code --install-extension tree-sitter-perl.perl-language-server
   ```

## Configuration

### Basic Settings

```json
{
  // Path to perl-lsp executable (optional - uses bundled by default)
  "perl.lsp.path": "",
  
  // Trace server communication for debugging
  "perl.lsp.trace.server": "off",
  
  // Path to perltidy for formatting
  "perl.formatting.perltidyPath": "perltidy",
  
  // Additional perltidy arguments
  "perl.formatting.perltidyArgs": []
}
```

### Using .perltidyrc

The extension automatically discovers and uses `.perltidyrc` files in your workspace or home directory for consistent formatting.

## Usage

### Code Formatting
- **Format Document**: `Shift+Alt+F`
- **Format Selection**: Select code and `Shift+Alt+F`

### Navigation
- **Go to Definition**: `F12` or `Ctrl+Click`
- **Find All References**: `Shift+F12`
- **Symbol Search**: `Ctrl+Shift+O`

### Code Intelligence
- **Hover**: Hover over any symbol for information
- **Signature Help**: Automatically shows while typing function calls
- **Auto-completion**: Triggered automatically or with `Ctrl+Space`

### Diagnostics
- Syntax errors appear instantly as you type
- Problems panel shows all diagnostics (`Ctrl+Shift+M`)

## Commands

- `Perl: Restart Language Server` - Restart the language server
- `Perl: Show Language Server Output` - Show debug output

## Troubleshooting

### Language server not starting
1. Check the output panel: `Perl: Show Language Server Output`
2. Ensure the bundled binary has execute permissions
3. Try setting `perl.lsp.path` to a manual installation

### Formatting not working
1. Install perltidy: `cpan Perl::Tidy`
2. Check perltidy is in PATH: `which perltidy`
3. Set `perl.formatting.perltidyPath` if needed

### Performance issues
- The bundled parser is highly optimized
- Large files (10K+ lines) should parse in under 100ms
- Report any performance issues on GitHub

## Contributing

This extension is part of the tree-sitter-perl project. Contributions are welcome!

- GitHub: https://github.com/tree-sitter-perl/tree-sitter-perl
- Issues: https://github.com/tree-sitter-perl/tree-sitter-perl/issues

## License

MIT License - see LICENSE file for details.

## Acknowledgments

- Built on the tree-sitter-perl v3 parser
- Uses Perl::Tidy for code formatting
- Powered by the Language Server Protocol