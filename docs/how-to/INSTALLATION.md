# Installation Guide

Perl Language Server (`perl-lsp`) provides a high-performance Language Server Protocol implementation for Perl with broad Perl 5 syntax coverage.

## Install from crates.io

```bash
cargo install perl-lsp
```

## Manual Installation

### Pre-compiled Binaries

1. Download the archive for your platform from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases/latest)
2. Extract the archive
3. Move `perl-lsp` (or `perl-lsp.exe` on Windows) to a directory on your `PATH`
4. Verify the install with `perl-lsp --health`

#### Linux / macOS
```bash
# Example flow after downloading a release archive
mkdir -p "$HOME/.local/bin"
tar xzf perl-lsp-*.tar.gz
find . -type f -name perl-lsp -exec cp {} "$HOME/.local/bin/perl-lsp" \;
chmod +x "$HOME/.local/bin/perl-lsp"
export PATH="$HOME/.local/bin:$PATH"
perl-lsp --health
```

#### Windows (PowerShell)
```powershell
# Example flow after downloading a release zip
Expand-Archive .\perl-lsp-*.zip -DestinationPath .\perl-lsp-release
New-Item -ItemType Directory -Force "$HOME\bin" | Out-Null
Copy-Item .\perl-lsp-release\**\perl-lsp.exe "$HOME\bin\perl-lsp.exe"
$env:Path = "$HOME\bin;" + $env:Path
perl-lsp.exe --health
```

### Build from Source

1. Install Rust (minimum version 1.92)
2. Clone the repository
3. Build the release binary

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo build --release --bin perl-lsp -p perl-lsp
cp target/release/perl-lsp ~/.local/bin/
```

## Verification

After installation, verify that perl-lsp is working:

```bash
perl-lsp --version
```

Expected output:
```
perl-lsp <version>
```

## First-Run Checklist

Before debugging editor integration, confirm the standalone binary works:

- `perl-lsp --version` prints a version
- `perl-lsp --health` prints `ok ...`
- your install location is on `PATH`
- your editor is configured to launch `perl-lsp --stdio`

If you cloned this repository for development, run `just doctor` for a guided environment check.

## Editor Configuration

### VS Code
1. Install the [Perl LSP extension](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs)
2. Open a Perl file (.pl or .pm)
3. The language server will start automatically

### Neovim
Add to your `init.lua` or `init.vim`:

```lua
require'lspconfig'.perllsp.setup{
  cmd = {"perl-lsp", "--stdio"},
  filetypes = {"perl", "perl6"},
  root_dir = function(fname)
    return require'lspconfig'.util.find_git_ancestor(fname) or vim.fn.getcwd()
  end,
}
```

### Emacs
Add to your configuration:

```elisp
(use-package lsp-mode
  :config
  (add-to-list 'lsp-language-id-configuration '(perl-mode . "perl"))
  (lsp-register-client
    :make-interactive
    :new-connection (lambda (&rest _) (list (cons "stdio" (start-process "perl-lsp" nil "perl-lsp" "--stdio"))))
    :activation-fn (lsp-activate-on "perl-mode")
    :server-id 'perllsp))

(add-hook 'perl-mode-hook #'lsp)
```

### Other Editors
Configure your editor to use the command:
```
perl-lsp --stdio
```

### API Access Patterns

- **Direct Rust integration**: add `perl-lsp`, `perl-parser`, `perl-lexer`, and `perl-dap` via your normal Cargo workflow for library and binary usage.
- **DAP / debugging clients**: run `perl-dap` in native or bridge mode from any DAP-compatible editor.
- **FFI / non-Rust integration**: use the `tree-sitter-perl-rs` crate with its optional `c-parser` feature for C-oriented Tree-sitter integration where needed.

## Features

- **~100% Perl Syntax Coverage**: Handles all modern Perl constructs
- **Real-time Syntax Checking**: Instant feedback on code issues
- **Code Completion**: Intelligent autocomplete with type inference
- **Go-to-Definition**: Navigate to symbol definitions
- **Find References**: Locate all usages of a symbol
- **Symbol Search**: Search across workspace files
- **Refactoring Support**: Advanced code transformation operations
- **Incremental Parsing**: <1ms updates for large files
- **Cross-file Navigation**: Dual indexing for comprehensive workspace analysis
- **Import Optimization**: Automatic import management

## Troubleshooting

### Installation Issues

#### "Permission denied" error
Ensure you have permission to write to the installation directory:
```bash
# For system-wide installation
sudo chown $USER:$USER /usr/local/bin

# Or install to user directory
mkdir -p ~/.local/bin
export PATH="$HOME/.local/bin:$PATH"
```

#### "Command not found" after installation
Add the installation directory to your PATH:

**Bash (~/.bashrc):**
```bash
export PATH="$PATH:$HOME/.local/bin"
```

**Zsh (~/.zshrc):**
```bash
export PATH="$PATH:$HOME/.local/bin"
```

**Windows:**
```powershell
[Environment]::SetEnvironmentVariable('Path', "$env:Path;$HOME\.local\bin", 'User')
```

### Runtime Issues

#### LSP server not starting
1. Verify the binary is executable: `perl-lsp --version`
2. Check your editor's LSP configuration
3. Look for error messages in your editor's LSP logs

#### Slow performance
1. Ensure you're using the latest released version
2. Check if your workspace has very large Perl files (>100KB)
3. Consider using `.perl-lspignore` to exclude unnecessary files

#### Incomplete syntax coverage
1. Verify you're using a supported Perl version (5.10+)
2. Check for syntax errors in your Perl files
3. Report issues at [GitHub Issues](https://github.com/EffortlessMetrics/perl-lsp/issues)

## Getting Help

- **Documentation**: [Full Documentation](https://github.com/EffortlessMetrics/perl-lsp)
- **Issues**: [GitHub Issues](https://github.com/EffortlessMetrics/perl-lsp/issues)
- **Discussions**: [GitHub Discussions](https://github.com/EffortlessMetrics/perl-lsp/discussions)
- **Changelog**: [Release Notes](https://github.com/EffortlessMetrics/perl-lsp/releases)

## Supported Platforms

| Platform | Architecture | Status |
|----------|-------------|--------|
| Linux | x86_64 | Tested |
| Linux | aarch64 | Tested |
| macOS | x86_64 | Tested |
| macOS | aarch64 | Tested |
| Windows | x86_64 | Tested |

## Minimum Requirements

- **Rust**: 1.92+ (for building from source)
- **Perl**: 5.10+ (for parsing)
- **Memory**: 50MB base usage
- **Disk**: 10MB for installation

## Security Notes

- perl-lsp only reads files in your workspace
- No network access is required during normal operation
- All dependencies are statically linked in release builds
- Security vulnerabilities should be reported privately via SECURITY.md
