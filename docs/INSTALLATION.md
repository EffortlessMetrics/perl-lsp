# Installation Guide

Perl Language Server (perl-lsp) v0.9.0 provides a high-performance Language Server Protocol implementation for Perl with ~100% syntax coverage.

## Install from crates.io

```bash
cargo install perl-lsp
```

## Manual Installation

### Pre-compiled Binaries

1. Download the appropriate binary for your system from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases)
2. Extract the archive
3. Move the `perl-lsp` binary to a directory in your PATH

#### Linux x86_64
```bash
wget https://github.com/EffortlessMetrics/perl-lsp/releases/download/v0.9.0/perl-lsp-0.9.0-x86_64-unknown-linux-gnu.tar.gz
tar xzf perl-lsp-0.9.0-x86_64-unknown-linux-gnu.tar.gz
sudo cp perl-lsp-0.9.0-x86_64-unknown-linux-gnu/perl-lsp /usr/local/bin/
chmod +x /usr/local/bin/perl-lsp
```

#### Linux aarch64
```bash
wget https://github.com/EffortlessMetrics/perl-lsp/releases/download/v0.9.0/perl-lsp-0.9.0-aarch64-unknown-linux-gnu.tar.gz
tar xzf perl-lsp-0.9.0-aarch64-unknown-linux-gnu.tar.gz
sudo cp perl-lsp-0.9.0-aarch64-unknown-linux-gnu/perl-lsp /usr/local/bin/
chmod +x /usr/local/bin/perl-lsp
```

#### macOS x86_64
```bash
wget https://github.com/EffortlessMetrics/perl-lsp/releases/download/v0.9.0/perl-lsp-0.9.0-x86_64-apple-darwin.tar.gz
tar xzf perl-lsp-0.9.0-x86_64-apple-darwin.tar.gz
sudo cp perl-lsp-0.9.0-x86_64-apple-darwin/perl-lsp /usr/local/bin/
chmod +x /usr/local/bin/perl-lsp
```

#### macOS aarch64 (Apple Silicon)
```bash
wget https://github.com/EffortlessMetrics/perl-lsp/releases/download/v0.9.0/perl-lsp-0.9.0-aarch64-apple-darwin.tar.gz
tar xzf perl-lsp-0.9.0-aarch64-apple-darwin.tar.gz
sudo cp perl-lsp-0.9.0-aarch64-apple-darwin/perl-lsp /usr/local/bin/
chmod +x /usr/local/bin/perl-lsp
```

#### Windows x86_64
```powershell
wget https://github.com/EffortlessMetrics/perl-lsp/releases/download/v0.9.0/perl-lsp-0.9.0-x86_64-pc-windows-msvc.zip
Expand-Archive perl-lsp-0.9.0-x86_64-pc-windows-msvc.zip
Copy-Item perl-lsp-0.9.0-x86_64-pc-windows-msvc\perl-lsp.exe C:\Program Files\perl-lsp\
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
perl-lsp 0.9.0
```

## Editor Configuration

### VS Code
1. Install the [Perl LSP extension](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp)
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
1. Ensure you're using the latest version (v0.9.0)
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
- Security vulnerabilities should be reported privately to security@effortlessmetrics.com
