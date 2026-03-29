# Installation Guide

Perl Language Server (perl-lsp) is a high-performance Language Server Protocol
implementation for Perl 5. The repository currently tracks the `v0.12.0`
release target, while crates.io and GitHub Releases provide the latest
published version. Always check the releases page before copying a version
number from `main`.

## Install from crates.io

```bash
cargo install perl-lsp
```

If you already have an older version installed, upgrade in place:

```bash
cargo install perl-lsp --force
```

## Manual Installation

### Pre-compiled Binaries

1. Go to [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases) and download the latest release for your platform.
2. Extract the archive.
3. Move the `perl-lsp` binary to a directory in your PATH.

Binaries are provided for the following platforms:

| Platform | Binary suffix |
|----------|--------------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` |
| Linux aarch64 | `aarch64-unknown-linux-gnu` |
| macOS Intel | `x86_64-apple-darwin` |
| macOS Apple Silicon | `aarch64-apple-darwin` |
| Windows x86_64 | `x86_64-pc-windows-msvc` |

#### Linux / macOS (general)
```bash
# Replace VERSION and ARCH with values from the releases page
# e.g. VERSION=<published-version> ARCH=x86_64-unknown-linux-gnu
wget https://github.com/EffortlessMetrics/perl-lsp/releases/download/v${VERSION}/perl-lsp-${VERSION}-${ARCH}.tar.gz
tar xzf perl-lsp-${VERSION}-${ARCH}.tar.gz
sudo cp perl-lsp-${VERSION}-${ARCH}/perl-lsp /usr/local/bin/
chmod +x /usr/local/bin/perl-lsp
```

#### Windows x86_64
```powershell
# Replace VERSION with the version from the releases page
$VERSION = "<published-version>"
Invoke-WebRequest "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v$VERSION/perl-lsp-$VERSION-x86_64-pc-windows-msvc.zip" -OutFile perl-lsp.zip
Expand-Archive perl-lsp.zip -DestinationPath perl-lsp
Copy-Item perl-lsp\perl-lsp.exe "C:\Program Files\perl-lsp\"
```

### Windows Package Managers

The release automation keeps the Windows package-manager metadata in sync with
each GitHub release, but only the repo-owned manifest refresh is automated.
Upstream package submission and the final user-machine install checks remain
manual.

- Scoop: `scoop install perl-lsp`
- Chocolatey: `choco install perl-lsp`
- Winget: the repo tracks a local manifest in `distribution/winget/` and the
  release workflow refreshes it; upstream `winget-pkgs` submission is still a
  manual follow-up

#### Verification Boundary

Repo-local checks can verify that:

- the release workflow publishes the Windows zip and consolidated `SHA256SUMS`
- the Scoop and Chocolatey bump workflows download that release asset, compute
  the checksum, and call `distribution/windows/update-manifests.ps1`
- placeholder guards fail if `__RELEASE_VERSION__`, `__RELEASE_HASH__`, or
  other release tokens remain in the manifests after the update step

Still manual:

- upstream PR acceptance or merge in the Scoop and Chocolatey package repos
- running `scoop install perl-lsp` or `choco install perl-lsp` on a Windows
  machine
- confirming `perl-lsp --health` and PATH discovery after installation
- checking that VS Code or another editor can find the installed binary

To smoke the repo-side story locally, run:

```powershell
powershell -NoLogo -NoProfile -File scripts/check-windows-distribution.ps1
```

For the latest version number, always check [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases).

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
perl-lsp --health
perl-lsp --info
```

Expected output:

- `--version` prints the installed package version.
- `--health` prints `ok <version>`.
- `--info` prints version, build metadata, feature profile, and coverage summary.

## Editor Configuration

### VS Code
1. Install the [Perl LSP extension](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs)
2. Open a Perl file (.pl or .pm)
3. The language server will start automatically

### Neovim
Add to your `init.lua`:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.perl_lsp then
  configs.perl_lsp = {
    default_config = {
      cmd = { 'perl-lsp', '--stdio' },
      filetypes = { 'perl' },
      root_dir = lspconfig.util.root_pattern('.git', 'Makefile.PL', 'cpanfile', 'dist.ini'),
      single_file_support = true,
    },
  }
end

lspconfig.perl_lsp.setup({})
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

- **Broad Perl Syntax Coverage**: Handles Perl 5.8 through 5.40 syntax including modern constructs
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

## Useful CLI Checks

These commands are helpful both after installation and when debugging editor integration:

```bash
perl-lsp --version                 # Confirm the binary resolves on PATH
perl-lsp --health                  # Fast readiness check
perl-lsp --info                    # Print build and feature-profile information
perl-lsp --check lib/My/Module.pm  # Validate a file without starting an editor
perl-lsp --completion bash         # Generate shell completions
```

## Runtime Issues

#### LSP server not starting
1. Verify the binary is executable: `perl-lsp --version`
2. Check your editor's LSP configuration
3. Look for error messages in your editor's LSP logs

#### Slow performance
1. Ensure you're using the [latest release](https://github.com/EffortlessMetrics/perl-lsp/releases/latest)
2. Check if your workspace has very large Perl files (>100KB)
3. Consider using `.perl-lspignore` to exclude unnecessary files

#### Incomplete syntax coverage
1. Verify you're using a supported Perl version (5.10+)
2. Check for syntax errors in your Perl files
3. Report issues at [GitHub Issues](https://github.com/EffortlessMetrics/perl-lsp/issues)

## Getting Help

- **Documentation Index**: [docs/INDEX.md](../INDEX.md)
- **Full Documentation**: [Repository Docs](https://github.com/EffortlessMetrics/perl-lsp/tree/master/docs)
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
