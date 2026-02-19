# Perl Language Server Protocol (LSP) Documentation

The Perl LSP server (`perl-lsp`) provides professional IDE features for Perl development in any LSP-compatible editor.

## Table of Contents

1. [Overview](#overview)
2. [Features](#features)
3. [Installation](#installation)
4. [Editor Configuration](#editor-configuration)
5. [Protocol Support](#protocol-support)
6. [Architecture](#architecture)
7. [Development](#development)
8. [Troubleshooting](#troubleshooting)

## Overview

The Perl LSP server is built on top of the v3 native parser (perl-lexer + perl-parser), providing:
- **100% Perl 5 syntax coverage** including all edge cases
- **Sub-millisecond response times** for most operations
- **Incremental parsing** for efficient document updates
- **Zero false positives** in error detection

## Features

### 1. Syntax Diagnostics
Real-time error detection as you type:
- Syntax errors with precise locations
- Undefined variable warnings (when strict is enabled)
- Missing semicolons and brackets
- Invalid syntax constructs

### 2. Document Symbols
Hierarchical outline of your Perl code:
- Package declarations
- Subroutine definitions
- Variable declarations (my, our, local, state)
- Constants and use statements

### 3. Symbol Navigation
- **Go to Definition**: Jump to where symbols are defined.
- **Find References**: Find all uses of a variable or function.
- **Workspace Symbols**: Fuzzy search for symbols across all files in the project.

### 4. Signature Help (**Enhanced in v0.8.8+**)
Advanced function parameter hints while typing:
- **Built-in Perl functions**: Complete coverage for print, substr, splice, split, push, pop, etc.
- **User-defined subroutines**: Full support for modern Perl signature syntax (`sub add($x, $y)`)
- **Active parameter highlighting**: Real-time tracking of which parameter you're currently entering.
- **Parameter documentation**: Comprehensive parameter information and type hints.
- **Nested call support**: Accurate parameter tracking in complex nested function calls.
- **Context-aware parsing**: Handles method calls (`->method()`) and regular function calls.

#### Example Usage:
```perl
# Built-in function signature help
substr($str, 5, |)  # Shows: EXPR, OFFSET, LENGTH, REPLACEMENT
                    # Highlights LENGTH parameter (index 2)

# User-defined function with signature
sub calculate($base, $rate, $time = 1) {
    return $base * $rate * $time;
}

calculate(1000, 0.05, |)  # Shows: $base, $rate, $time = 1
                          # Highlights $time parameter
```

### 5. Semantic Tokens
Enhanced syntax highlighting beyond regex-based:
- Different token types for variables based on context
- Distinguishes between function calls and barewords
- Proper highlighting of interpolated strings
- Context-aware operator highlighting

### 6. Code Completion
- **Variable Completion**: Suggests variables in the current scope.
- **Built-in Completion**: Suggests all 150+ built-in functions.
- **Keyword Completion**: Suggests Perl keywords.
- **File Path Completion**: Securely completes file paths in `use` and `require` statements.

### 7. Code Actions
- **Add Pragmas**: Add missing `use strict;` and `use warnings;`.
- **Perltidy**: Format the current file with `perltidy` (if installed).
- **Import Optimization**: Detects and removes unused imports, and consolidates duplicate imports.

### 8. Refactoring
- **Rename**: Cross-file rename for `our` variables and local rename for `my` variables.

### 9. Other Features
- **Inlay Hints** (**Enhanced in v0.8.8+**): Shows parameter names and inferred types with improved positioning accuracy.
  - **Enhanced Parameter Positioning**: Accurate positioning for parenthesized function calls (e.g., `push(@arr, "x")` shows hint at `@arr`, not at `(`)
  - **Consistent Parameter Labels**: Standardized parameter signatures with consistent case (`ARRAY`, `FILEHANDLE` for built-in functions)
  - **Built-in Function Support**: Comprehensive parameter hints for all major Perl built-ins including `push`, `open`, `print`, `printf`
- **Document Links**: Creates links from `use`/`require` statements to the corresponding file or MetaCPAN page.
- **Selection Ranges**: Allows for smart, hierarchical selection of code blocks.
- **On-Type Formatting**: Automatically adjusts indentation when typing `{`, `}`, `;`, etc.
- **Code Lens**: Shows reference counts above subroutines, and provides "Run Test" lenses.

### 10. Incremental Parsing
- Only re-parses changed portions of documents.
- Maintains full AST for accurate analysis.
- Sub-millisecond update times, with 96.8% - 99.7% of the AST reused on typical edits.

## Installation

The recommended way to install is to use the pre-built binaries or a package manager.

### Quick Install (Linux/macOS)
```bash
# One-liner installer
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/main/install.sh | bash
```

### Quick Install (Windows PowerShell)
```powershell
irm https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/main/install.ps1 | iex
```

### Homebrew (macOS/Linux)
```bash
brew tap effortlesssteven/tap
brew install perl-lsp
```

### Build from Source
```bash
# Install the perl-lsp binary from crates.io
cargo install perl-lsp

# Or, build from this repository
git clone https://github.com/EffortlessMetrics/perl-lsp
cd tree-sitter-perl
cargo build --release -p perl-lsp
# The binary will be in target/release/perl-lsp
```

### System Requirements
- Rust 1.92 or later
- No runtime dependencies
- Works on Linux, macOS, and Windows

## Editor Configuration

### Visual Studio Code

1. Install a generic LSP client extension (e.g., "Generic LSP Client")
2. Add to your `settings.json`:

```json
{
  "genericLSP.servers": {
    "perl": {
      "command": "perl-lsp",
      "args": ["--stdio"],
      "rootIndicators": ["Makefile.PL", "Build.PL", "cpanfile", ".git"],
      "fileEvents": ["**/*.{pl,pm,pod,t}"],
      "languageIds": ["perl"]
    }
  }
}
```

### Neovim

Add to your Neovim configuration:

```lua
-- Using nvim-lspconfig
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Define the Perl LSP configuration
if not configs.perl_lsp then
  configs.perl_lsp = {
    default_config = {
      cmd = {'perl-lsp', '--stdio'},
      filetypes = {'perl'},
      root_dir = lspconfig.util.root_pattern('Makefile.PL', 'Build.PL', 'cpanfile', '.git'),
      settings = {},
    },
  }
end

-- Enable the Perl LSP
lspconfig.perl_lsp.setup{}
```

### Emacs

Using `lsp-mode`:

```elisp
(require 'lsp-mode)

(add-to-list 'lsp-language-id-configuration '(perl-mode . "perl"))

(lsp-register-client
 (make-lsp-client
  :new-connection (lsp-stdio-connection "perl-lsp --stdio")
  :major-modes '(perl-mode cperl-mode)
  :server-id 'perl-lsp))

(add-hook 'perl-mode-hook #'lsp)
```

### Sublime Text

1. Install the LSP package
2. Add to LSP settings:

```json
{
  "clients": {
    "perl-lsp": {
      "enabled": true,
      "command": ["perl-lsp", "--stdio"],
      "selector": "source.perl",
      "initializationOptions": {}
    }
  }
}
```

## Protocol Support

### Supported LSP Methods

#### Lifecycle
- [x] `initialize`
- [x] `initialized`
- [x] `shutdown`
- [x] `exit`

#### Document Synchronization
- [x] `textDocument/didOpen`
- [x] `textDocument/didChange` (incremental)
- [x] `textDocument/didClose`
- [x] `textDocument/didSave`

#### Language Features
- [x] `textDocument/publishDiagnostics`
- [x] `textDocument/documentSymbol`
- [x] `textDocument/definition`
- [x] `textDocument/references`
- [x] `textDocument/signatureHelp`
- [x] `textDocument/semanticTokens/full`
- [x] `textDocument/completion`
- [x] `textDocument/hover`
- [x] `textDocument/codeAction`
- [x] `textDocument/formatting`
- [x] `textDocument/rename`
- [x] `textDocument/codeLens`
- [x] `textDocument/documentLink`
- [x] `textDocument/selectionRange`
- [x] `textDocument/onTypeFormatting`
- [x] `workspace/symbol`

### Client Capabilities

The server adapts its behavior based on client capabilities:
- Hierarchical document symbols (if supported)
- Semantic token support with custom token types
- Incremental text synchronization
- Dynamic registration (planned)

## Architecture

### Components

The LSP is composed of two main crates:
- **`perl-lsp`**: The binary that runs the server.
- **`perl-parser`**: The library that contains all the logic.

```
perl-parser crate
├── LSP Server Logic
│   ├── JSON-RPC message handling
│   ├── Request routing
│   └── Response serialization
├── Document Manager
│   ├── Document state tracking
│   ├── Change management
│   └── AST caching
├── Language Services
│   ├── DiagnosticsProvider
│   ├── CompletionProvider
│   ├── HoverProvider
│   ├── SignatureHelpProvider
│   ├── DefinitionProvider
│   ├── ReferencesProvider
│   ├── ... and many more
└── Parser Integration
    ├── perl-lexer (tokenization)
    └── perl-parser (AST generation)
```

### Request Flow

1. **Client Request** → LSP server receives JSON-RPC message
2. **Parse Request** → Deserialize and validate request
3. **Document Lookup** → Find document state and cached AST
4. **Process Request** → Call appropriate language service
5. **Generate Response** → Create LSP-compliant response
6. **Send Response** → Serialize and send back to client

### Performance Optimizations

- **AST Caching**: Parse once, analyze many times
- **Incremental Updates**: Only re-parse changed regions
- **Lazy Analysis**: Compute only what's requested
- **Zero-Copy Strings**: Efficient memory usage

## Development

### Building from Source

```bash
# Debug build with logging
RUST_LOG=debug cargo build -p perl-parser --bin perl-lsp

# Run with logging
RUST_LOG=perl_parser=debug perl-lsp --stdio --log
```

### Running Tests

```bash
# Run all LSP tests
cargo test -p perl-parser lsp

# Run specific test
cargo test -p perl-parser lsp_server::tests::test_initialize

# Run integration tests
cargo test -p perl-parser --test lsp_integration_test
```

### Adding New Features

1. Implement the trait in `lsp.rs`:
   ```rust
   impl HoverProvider for LanguageService {
       fn hover(&self, params: HoverParams) -> Option<Hover> {
           // Implementation
       }
   }
   ```

2. Add handler in `lsp_server.rs`:
   ```rust
   "textDocument/hover" => {
       self.handle_hover(request.params)
   }
   ```

3. Update capabilities in `initialize`:
   ```rust
   hover_provider: Some(HoverProviderCapability::Simple(true)),
   ```

### Debugging

Enable debug logging:
```bash
RUST_LOG=debug perl-lsp --stdio --log 2>lsp.log
```

Use LSP inspector tools:
- [LSP Inspector](https://microsoft.github.io/language-server-protocol/inspector/)
- Editor's built-in LSP logging

## Troubleshooting

### Common Issues

#### Server doesn't start
- Check if `perl-lsp` is in PATH
- Verify with: `perl-lsp --version`
- Check permissions: `chmod +x perl-lsp`

#### No diagnostics appearing
- Ensure file has `.pl` or `.pm` extension
- Check if document was opened (not just visible)
- Verify client sends `textDocument/didOpen`

#### Features not working
- Check server capabilities in initialize response
- Verify client capabilities are being sent
- Enable debug logging to see requests

#### Performance issues
- Check document size (very large files may be slow)
- Monitor AST cache hits in debug logs
- Consider increasing parser timeout

### Debug Output

With `--log` flag, the server outputs:
- All incoming requests
- Processing times
- Error details
- AST parsing statistics

Example debug output:
```
[DEBUG] Received request: textDocument/didOpen
[DEBUG] Parsing document: file:///home/user/test.pl
[DEBUG] Parse completed in 0.5ms
[DEBUG] Found 3 diagnostics
[DEBUG] Sending diagnostics notification
```

## Future Enhancements

### Planned Features
1. **Code Completion**
   - Context-aware suggestions
   - Snippet support
   - Auto-import statements

2. **Code Actions**
   - Quick fixes for common errors
   - Refactoring operations
   - Code generation

3. **Enhanced Diagnostics**
   - Type checking (where possible)
   - Security warnings (taint checking)
   - Best practice suggestions

4. **Multi-file Support**
   - Cross-file symbol resolution
   - Project-wide search
   - Module dependency analysis

5. **Performance Improvements**
   - True incremental parsing
   - Parallel analysis
   - Background indexing

### Contributing

Contributions are welcome! See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

Key areas for contribution:
- Additional language features
- Editor-specific plugins
- Performance optimizations
- Documentation improvements

## Resources

- [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/)
- [LSP Tutorial](https://microsoft.github.io/language-server-protocol/overviews/lsp/overview/)
- [Perl Parser Documentation](../crates/perl-parser/README.md)
- [Project Repository](https://github.com/EffortlessMetrics/perl-lsp)