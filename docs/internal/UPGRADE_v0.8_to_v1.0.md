# Upgrade Guide: Perl LSP v0.8.x to v1.0.0

> **Status**: Production Ready
> **Last Updated**: 2026-01-24
> **Applies To**: perl-parser, perl-lsp, perl-lexer, perl-dap

This guide documents how to migrate from Perl LSP v0.8.x to v1.0.0. The v1.0 release marks the first GA (Generally Available) release with formal stability guarantees.

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Breaking Changes](#breaking-changes)
3. [New Features](#new-features)
4. [Deprecations](#deprecations)
5. [Migration Steps](#migration-steps)
6. [Rollback Procedure](#rollback-procedure)
7. [Known Issues](#known-issues)
8. [Platform-Specific Notes](#platform-specific-notes)
9. [Getting Help](#getting-help)

---

## Quick Start

For most users, upgrading is straightforward:

```bash
# Binary users: Download new version
# GitHub releases: https://github.com/EffortlessMetrics/tree-sitter-perl/releases

# Cargo users: Update dependencies
cargo install perl-lsp --force

# VS Code users: Extension updates automatically
# Or manually: Extensions > Perl Language Server > Update
```

**No configuration changes required for most users.** v1.0.0 maintains backward compatibility with v0.8.x configurations.

---

## Breaking Changes

### 1. Minimum Supported Rust Version (MSRV)

| Version | MSRV | Edition |
|---------|------|---------|
| v0.8.x | 1.85 | 2021 |
| v0.9.x | 1.92 | 2024 |
| v1.0.0 | 1.92 | 2024 |

**Impact**: If building from source, ensure Rust 1.92 or later is installed.

```bash
# Check your Rust version
rustc --version

# Update if needed
rustup update stable
```

### 2. DeclarationProvider API Change (Library Users Only)

If you use `perl-parser` as a library, the `find_declaration()` method signature changed in v0.8.0:

**Before (v0.7.x)**:
```rust
let provider = DeclarationProvider::new(&tree);
let location = provider.find_declaration(offset, col);
```

**After (v0.8.0+)**:
```rust
let provider = DeclarationProvider::new(&tree);
let current_version = provider.version();
let location = provider.find_declaration(offset, col, current_version);
```

Or use the convenience method:
```rust
let provider = DeclarationProvider::new(&tree);
let location = provider.with_doc_version(doc_version)
    .find_declaration(offset, col, doc_version);
```

**Reason**: Version tracking prevents stale AST usage after document edits, improving reliability.

### 3. Error Type Changes (Library Users Only)

Parser error handling changed from `Option<Node>` to `Result<Node, ParseError>`:

**Before (v0.7.x)**:
```rust
let node = parser.parse(source).unwrap();
```

**After (v0.8.0+)**:
```rust
let node = parser.parse(source)?;
// Or with explicit error handling:
match parser.parse(source) {
    Ok(node) => { /* use node */ }
    Err(e) => { eprintln!("Parse error at {:?}: {}", e.location, e.message); }
}
```

### 4. VS Code Extension: Removed `downloadBaseUrl` Setting

The `perl-lsp.downloadBaseUrl` configuration option has been removed.

**Before (v0.8.x)**:
```json
{
  "perl-lsp.downloadBaseUrl": "https://internal.example.com/perl-lsp/"
}
```

**After (v1.0.0)**:
- Use `perl-lsp.serverPath` to specify a local binary path
- Or rely on automatic GitHub releases download

**Migration**:
```json
{
  "perl-lsp.serverPath": "/path/to/perl-lsp",
  "perl-lsp.autoDownload": false
}
```

### 5. NodeKind Enum is Non-Exhaustive

The `NodeKind` enum is now marked `#[non_exhaustive]`, which means match statements must include a wildcard arm:

**Before**:
```rust
match node.kind {
    NodeKind::Sub => { /* ... */ }
    NodeKind::Package => { /* ... */ }
    // Compile error in v1.0 - missing wildcard arm
}
```

**After**:
```rust
match node.kind {
    NodeKind::Sub => { /* ... */ }
    NodeKind::Package => { /* ... */ }
    _ => { /* handle other/new variants */ }
}
```

**Reason**: Allows adding new AST node types in minor releases without breaking existing code.

---

## New Features

### Major New Capabilities in v0.9/v1.0

#### 1. Semantic Analyzer (Complete)

Full semantic analysis pipeline with all NodeKind handlers:

- **Uninitialized variable detection**: Warns when variables are used before assignment
- **Lexical scoping**: Proper handling of nested scopes and shadowed variables
- **Type inference**: Basic type tracking for scalars, arrays, hashes

#### 2. Refactoring Engine

New refactoring capabilities:

- **Extract Subroutine**: Select code, extract to new sub with parameters
- **Extract Variable**: Extract expressions to variables
- **Inline Variable**: Inline variable usages back to expressions
- **Move Code**: Relocate code blocks between files

```
VS Code: Right-click > Perl Refactor > Extract Subroutine
Keyboard: Shift+Alt+O (Organize Imports)
```

#### 3. TCP Socket Mode

LSP server now supports TCP connections in addition to stdio:

```bash
# Start in socket mode
perl-lsp --socket --port 9257

# Connect from editor or tool
```

#### 4. Security Hardening

Comprehensive security improvements:

- **Path traversal protection**: All file operations validate paths (PR #388)
- **Command injection hardening**: No shell interpolation in commands (PR #332)
- **DAP security**: Expanded blocklist for dangerous operations (PR #521)
- **Panic elimination**: No more `unwrap()`/`expect()` in production code

#### 5. Performance Optimizations

Significant performance improvements:

| Metric | v0.8.x | v1.0.0 | Improvement |
|--------|--------|--------|-------------|
| Symbol lookups | O(n) | O(1) | ~10-100x |
| Variable lookup | Allocations | Zero-allocation | Memory reduction |
| Token storage | String | Arc<str> | Memory reduction |
| LSP tests | 1560s | 0.31s | 5000x faster |

#### 6. DAP Improvements

Debug Adapter Protocol enhancements:

- **Async BridgeAdapter** with graceful shutdown
- **CLI argument parsing** with clap
- **Stdio transport** support
- **Enhanced breakpoint handling**

#### 7. New LSP Capabilities (8 Features Promoted to GA)

- `completionItem/resolve`: Additional completion details
- `codeAction/resolve`: Lazy code action resolution
- `codeLens/resolve`: Lazy code lens command resolution
- `workspaceSymbol/resolve`: Additional symbol info
- `workspace/willRenameFiles`: Pre-rename hooks
- `workspace/didRenameFiles`: Post-rename notifications
- `workspace/didDeleteFiles`: Delete tracking
- `workspace/edit`: Multi-file edits

---

## Deprecations

### 1. `perl-parser-pest` (v2 Parser)

The Pest-based parser is deprecated. While still available, it is in maintenance mode:

- **Status**: Deprecated (v1.0.0)
- **Removal**: Planned for v2.0.0
- **Migration**: Use the default v3 native parser (no action needed if using defaults)

```toml
# Remove explicit pest feature if present
# Before:
perl-parser = { version = "0.8", features = ["pure-rust"] }

# After (v1.0):
perl-parser = "1.0"  # Uses v3 native parser by default
```

### 2. Feature Flag: `pure-rust`

The `pure-rust` feature flag is deprecated:

- **Status**: Deprecated (v1.0.0)
- **Migration**: Simply remove the feature flag; v3 parser is now default

### 3. Position Module Visibility

The `positions` module is now `#[doc(hidden)]`:

- Still accessible but not guaranteed stable
- Use public position conversion functions instead

**Before**:
```rust
use perl_parser::positions::*;
```

**After**:
```rust
use perl_parser::{offset_to_position, position_to_offset};
```

---

## Migration Steps

### Step 1: Backup Current Configuration

```bash
# VS Code settings
cp ~/.config/Code/User/settings.json ~/.config/Code/User/settings.json.backup

# Workspace settings (if any)
cp .vscode/settings.json .vscode/settings.json.backup
```

### Step 2: Update the LSP Server

#### Option A: VS Code Extension (Recommended)

1. Open VS Code
2. Go to Extensions (Ctrl+Shift+X)
3. Find "Perl Language Server"
4. Click Update (or it updates automatically)
5. Reload window (Ctrl+Shift+P > "Reload Window")

#### Option B: Cargo Install

```bash
cargo install perl-lsp --force
```

#### Option C: From Source

```bash
git clone https://github.com/EffortlessMetrics/tree-sitter-perl
cd tree-sitter-perl
cargo install --path crates/perl-lsp
```

#### Option D: Pre-built Binary

Download from [GitHub Releases](https://github.com/EffortlessMetrics/tree-sitter-perl/releases):

```bash
# Linux x86_64
curl -LO https://github.com/.../perl-lsp-x86_64-unknown-linux-gnu.tar.gz
tar xzf perl-lsp-*.tar.gz
sudo mv perl-lsp /usr/local/bin/

# Verify
perl-lsp --version
```

### Step 3: Update Library Dependencies (If Applicable)

```toml
# Cargo.toml
[dependencies]
perl-parser = "1.0"
perl-lexer = "1.0"
perl-lsp = "1.0"  # If using as library
```

### Step 4: Address Breaking Changes

1. **MSRV**: Ensure Rust 1.92+ if building from source
2. **DeclarationProvider**: Add version parameter if using library API
3. **Error handling**: Update `Option` to `Result` handling
4. **NodeKind matches**: Add wildcard arms

### Step 5: Verify the Upgrade

```bash
# Check version
perl-lsp --version
# Expected: perl-lsp 1.0.0

# Health check
perl-lsp --health
# Expected: ok 1.0.0

# Test in editor
# 1. Open a Perl file
# 2. Verify syntax highlighting
# 3. Test completion (Ctrl+Space)
# 4. Test go-to-definition (F12)
# 5. Test hover (hover over symbol)
```

### Step 6: Clean Up Deprecated Settings

Remove deprecated settings from VS Code configuration:

```json
{
  // Remove if present:
  // "perl-lsp.downloadBaseUrl": "..."
}
```

---

## Rollback Procedure

If issues occur after upgrading, you can safely rollback:

### VS Code Extension

1. Open Extensions (Ctrl+Shift+X)
2. Click gear icon on "Perl Language Server"
3. Select "Install Another Version..."
4. Choose v0.8.x version

### Cargo Install

```bash
cargo install perl-lsp --version 0.8.9 --force
```

### Pre-built Binary

Download the previous version from GitHub releases:

```bash
curl -LO https://github.com/.../releases/download/v0.8.9/perl-lsp-*.tar.gz
tar xzf perl-lsp-*.tar.gz
sudo mv perl-lsp /usr/local/bin/
```

### Library Rollback

```toml
# Cargo.toml
[dependencies]
perl-parser = "0.8.9"
perl-lexer = "0.8.9"
```

### Restore Configuration Backup

```bash
cp ~/.config/Code/User/settings.json.backup ~/.config/Code/User/settings.json
```

---

## Known Issues

### 1. CRLF Line Ending Edge Case

**Issue**: Position conversions may not round-trip perfectly with CRLF line endings.

**Workaround**: Use LF line endings where possible, or accept minor column offset variations.

**Status**: Documented limitation, not planned for immediate fix.

### 2. DAP Variables/Evaluate Placeholders

**Issue**: The native DAP adapter has placeholder implementations for variables and evaluate requests.

**Workaround**: Use the BridgeAdapter mode with Perl::LanguageServer for full debugging:

```json
// launch.json
{
  "type": "perl",
  "request": "launch",
  "program": "${file}",
  "adapter": "bridge"
}
```

**Status**: Full implementation planned for future release.

### 3. Large File Memory Usage

**Issue**: Files over 10,000 lines may consume significant memory.

**Workaround**:
- Split large files if possible
- Increase available memory for VS Code
- Consider using `perl-lsp.diagnostics.enable: false` for very large files

### 4. Socket Mode for DAP

**Issue**: DAP socket transport is not yet implemented.

**Workaround**: Use stdio mode (default):

```bash
perl-dap --stdio
```

---

## Platform-Specific Notes

### Linux

**Tier 1 Support** (x86_64 glibc, x86_64 musl, aarch64 glibc)

No special considerations. All pre-built binaries work on modern distributions.

**Minimum glibc**: 2.35 (Ubuntu 22.04+, Fedora 36+)

For older distributions, use the musl static binary:
```bash
curl -LO .../perl-lsp-x86_64-unknown-linux-musl.tar.gz
```

### macOS

**Tier 1 Support** (x86_64 Intel, aarch64 Apple Silicon)

- Intel Macs: Requires macOS 10.15 (Catalina) or later
- Apple Silicon: Requires macOS 11.0 (Big Sur) or later

If you get a Gatekeeper warning:
```bash
xattr -d com.apple.quarantine perl-lsp
```

### Windows

**Tier 1 Support** (x86_64 MSVC)

- Requires Windows 10 or later
- Uses MSVC toolchain (not MinGW)

**WSL**: Fully supported. Use the Linux binary within WSL.

### Editor/IDE Integration Changes

#### VS Code

New keyboard shortcuts added:
- `Shift+Alt+O`: Organize imports
- `Shift+Alt+T`: Run tests
- `Shift+Alt+R`: Restart server

New commands in command palette (filtered for Perl files only):
- "Perl: Extract Subroutine"
- "Perl: Extract Variable"
- "Perl: Inline Variable"

New status bar item shows LSP server state.

#### Neovim

Configuration remains the same. Update the server path or let it auto-download:

```lua
require('lspconfig').perl_lsp.setup{
  cmd = { "perl-lsp", "--stdio" },
}
```

#### Emacs

Update lsp-mode configuration:

```elisp
(use-package lsp-mode
  :config
  (lsp-register-client
    (make-lsp-client :new-connection (lsp-stdio-connection '("perl-lsp" "--stdio"))
                     :major-modes '(perl-mode cperl-mode)
                     :server-id 'perl-lsp)))
```

---

## Getting Help

### Resources

- **Documentation**: [docs/](https://github.com/EffortlessMetrics/tree-sitter-perl/tree/master/docs)
- **Issues**: [GitHub Issues](https://github.com/EffortlessMetrics/tree-sitter-perl/issues)
- **CHANGELOG**: [CHANGELOG.md](../CHANGELOG.md)
- **Stability Policy**: [STABILITY.md](STABILITY.md)

### Reporting Issues

When reporting upgrade issues, include:

1. Previous version (e.g., v0.8.9)
2. New version (e.g., v1.0.0)
3. Platform (e.g., Linux x86_64, macOS aarch64)
4. Editor (e.g., VS Code 1.88, Neovim 0.10)
5. Error messages or unexpected behavior
6. Minimal reproduction steps

### Security Issues

Report security issues to: **security@perl-lsp.org** (or GitHub Security Advisories)

Do NOT file public issues for security vulnerabilities.

---

## Version Compatibility Matrix

| Component | v0.8.x | v0.9.x | v1.0.x |
|-----------|--------|--------|--------|
| perl-parser | 0.8.x | 0.9.x | 1.0.x |
| perl-lexer | 0.8.x | 0.9.x | 1.0.x |
| perl-corpus | 0.8.x | 0.9.x | 1.0.x |
| perl-lsp | 0.8.x | 0.9.x | 1.0.x |
| perl-dap | 0.1.x | 0.1.x | 0.2.x |
| MSRV | 1.85 | 1.92 | 1.92 |
| Rust Edition | 2021 | 2024 | 2024 |

**Note**: `perl-dap` remains at 0.2.x in v1.0.0 as it is still in preview.

---

## Summary Checklist

Before upgrading:
- [ ] Backup VS Code settings
- [ ] Check Rust version if building from source

During upgrade:
- [ ] Update server binary or extension
- [ ] Update library dependencies if applicable

After upgrade:
- [ ] Verify `perl-lsp --version` shows 1.0.0
- [ ] Test basic LSP features in editor
- [ ] Remove deprecated settings

If issues occur:
- [ ] Check Known Issues section
- [ ] Try rollback procedure
- [ ] File GitHub issue with details

---

*Last Updated: 2026-01-24*
