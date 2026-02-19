# Upgrading to perl-lsp v1.0

This guide provides comprehensive upgrade instructions from v0.8.x to v1.0.

**Quick Summary:**
- **MSRV bumped**: Rust 1.92+ required (was 1.92+ in v0.8.x, unchanged)
- **Rust Edition**: Rust 2024 Edition (was 2021 in v0.8.x)
- **Breaking Changes**: Minimal - primarily internal API refinements
- **New Features**: Semantic analyzer, refactoring engine, performance optimizations
- **Security Hardening**: Path traversal and command injection protections

---

## Table of Contents

1. [Breaking Changes](#breaking-changes)
2. [New Features](#new-features)
3. [Performance Improvements](#performance-improvements)
4. [Security Enhancements](#security-enhancements)
5. [Deprecated Features](#deprecated-features)
6. [Migration Steps](#migration-steps)
7. [LSP Client Changes](#lsp-client-changes)
8. [Library API Changes](#library-api-changes)
9. [VS Code Extension Changes](#vs-code-extension-changes)
10. [DAP Changes](#dap-changes)
11. [Testing Changes](#testing-changes)
12. [Troubleshooting](#troubleshooting)

---

## Breaking Changes

### 1. Rust 2024 Edition (PR #175)

**Impact:** All crates now use Rust 2024 Edition

**Action Required:**
```toml
# Update your Cargo.toml if depending on perl-lsp crates
[package]
edition = "2024"
rust-version = "1.92"
```

**Implications:**
- Edition 2024 introduces new keyword reservations and syntax changes
- See [Rust Edition Guide](https://doc.rust-lang.org/edition-guide/rust-2024/) for migration details
- All perl-lsp crates are compatible with edition 2024 consumers

### 2. Strict Error Handling (Issue #143, #292)

**Impact:** APIs that previously panicked now return `Result` types

**Before (v0.8.x):**
```rust
// Could panic on malformed input
let ast = parser.parse_unchecked(source);
```

**After (v1.0):**
```rust
// Returns Result for safe error handling
let ast = parser.parse(source)?;
```

**Action Required:**
- Replace `.unwrap()` and `.expect()` calls with proper error handling
- Use `?` operator or `match` for Result types
- Review code for panic-prone operations

**Affected APIs:**
- Parser core: All parsing methods now return `Result<Node, ParseError>`
- LSP handlers: Internal handlers use `Result` for error propagation
- Lexer: Tokenization methods return `Result` for malformed input

### 3. VS Code Extension Configuration

**Impact:** Removed `perl-lsp.downloadBaseUrl` configuration setting

**Before (v0.8.x):**
```json
{
  "perl-lsp.downloadBaseUrl": "https://custom-mirror.example.com/"
}
```

**After (v1.0):**
```json
// Setting removed - use standard installation methods
// Install via: cargo install perl-lsp
```

**Action Required:**
- Remove `downloadBaseUrl` from VS Code settings
- Use standard installation: `cargo install perl-lsp`
- Internal archive hosting no longer supported

### 4. UTF-16 Position Encoding

**Impact:** Stricter UTF-16 compliance for LSP protocol

**Before (v0.8.x):**
```rust
// Mixed UTF-8 and UTF-16 positions
let position = Position { line: 0, character: offset };
```

**After (v1.0):**
```rust
// Strict UTF-16 code unit offsets
let position = offset_to_utf16_position(source, offset)?;
```

**Action Required:**
- Use provided conversion helpers for position calculations
- Ensure all LSP positions use UTF-16 code units (not UTF-8 bytes)
- Test with multi-byte Unicode characters

---

## New Features

### 1. Semantic Analyzer (PR #389, #234)

**Complete semantic analysis pipeline with all NodeKind handlers**

```rust
use perl_parser::semantic::SemanticModel;

// Build semantic model from AST
let model = SemanticModel::build(&root, source)?;

// Query symbol definitions
if let Some(def) = model.definition_at(position) {
    println!("Symbol defined at: {:?}", def.location);
}

// Access symbol table
let symbols = model.symbol_table();
for (name, symbol) in symbols {
    println!("{}: {:?}", name, symbol.kind);
}
```

**Features:**
- Complete NodeKind coverage (all AST nodes handled)
- Precise symbol resolution across scopes
- Package-qualified call resolution
- Uninitialized variable detection (PR #396)
- Type inference improvements

**LSP Integration:**
- `textDocument/definition` uses semantic analysis (not text search)
- Improved hover information with type hints
- Better completion context awareness

### 2. Refactoring Engine (PR #387, #391, #392)

**Extract Method, Inline Variable, and Move Code refactorings**

```perl
# Before: Inline code
my $total = $price * $quantity * (1 - $discount);

# After: Extract method refactoring
sub calculate_total {
    my ($price, $quantity, $discount) = @_;
    return $price * $quantity * (1 - $discount);
}
my $total = calculate_total($price, $quantity, $discount);
```

**Available Refactorings:**
- **Extract Method** (PR #315): Extract code blocks into new subroutines with parameter detection
- **Inline Variable** (PR #391): Inline variable definitions into usage sites
- **Move Code** (PR #392): Relocate code blocks within files
- **Rename Symbol**: Enhanced with sigil consistency validation

**Safety Features:**
- Transactional rollback with `create_backup` (PR #298)
- Comprehensive validation prevents invalid code generation
- Automatic parameter detection and signature generation

### 3. TCP Socket Mode (PR #370)

**LSP server can now listen on TCP sockets**

```bash
# Start LSP server on TCP port
perl-lsp --tcp 9257

# Connect from LSP client
{
  "serverConfig": {
    "host": "localhost",
    "port": 9257
  }
}
```

**Use Cases:**
- Remote development workflows
- Docker/container deployments
- Network-based LSP clients
- Debugging LSP communication

### 4. Advanced LSP Features

#### Inlay Hints (LSP 3.18)
```perl
# Displays parameter names and type hints inline
my $result = calculate($price, $quantity, $discount);
#                      ^^^^^^  ^^^^^^^^^  ^^^^^^^^^
#                      price:  quantity:  discount:
```

#### Call Hierarchy
```perl
# Navigate call chains
sub main {
    process_data();  # Jump to definition or find callers
}
```

#### Workspace Symbols
```bash
# Search symbols across entire workspace
# Query: "UserController"
# Results: All matching symbols in all files
```

### 5. Debug Adapter Protocol (DAP) - Phase 1 (PR #369, #374, #330)

**Native debugger adapter with bridge mode**

```bash
# Install DAP server
cargo install perl-dap

# Launch debugger
perl-dap --stdio

# Debug configuration (launch.json)
{
  "type": "perl",
  "request": "launch",
  "program": "${file}",
  "cwd": "${workspaceFolder}"
}
```

**Features:**
- Launch mode: Start new Perl process with debugging
- Breakpoints: Set/remove breakpoints with validation
- Step operations: Step over, step into, step out
- Stack traces: Full call stack inspection
- CLI argument parsing with clap (PR #374)
- Async BridgeAdapter with graceful shutdown (PR #369)
- Stdio transport (PR #330)

**Limitations (Phase 1):**
- Attach mode: Not yet implemented
- Variable inspection: Placeholder implementation
- Expression evaluation: Limited support
- Native adapter: Bridge to Perl::LanguageServer

---

## Performance Improvements

### 1. O(1) Symbol Lookups (PR #336)

**Before (v0.8.x):**
- Linear scan: O(n) for symbol resolution
- 10,000 symbols: ~10ms lookup time

**After (v1.0):**
- Hash-based lookup: O(1) for symbol resolution
- 10,000 symbols: ~50μs lookup time

**Impact:** 200x faster symbol resolution

### 2. Zero-Allocation Variable Lookup (PR #473)

**Before (v0.8.x):**
```rust
// Allocated String for every lookup
let var_name = format!("${}", name);
scope.find_variable(&var_name)
```

**After (v1.0):**
```rust
// Zero allocations with Cow<str>
scope.find_variable_borrowed(name)
```

**Impact:** 50% reduction in heap allocations during scope analysis

### 3. Stack-Based ScopeAnalyzer (PR #383)

**Before (v0.8.x):**
- Recursive parent map traversal
- Multiple HashMap lookups per scope resolution

**After (v1.0):**
- Stack-based ancestor tracking
- Single traversal for scope chain

**Impact:** 3x faster scope resolution in deeply nested code

### 4. Reduced String Allocations in Parser (PR #367, #372, #368)

**Before (v0.8.x):**
```rust
// Cloned strings for every AST node
Node::new(token.text().to_string())
```

**After (v1.0):**
```rust
// Arc<str> for shared string references
Node::new(Arc::clone(&token.text))
```

**Impact:** 40% reduction in parse time for large files

### 5. Built-in Function Signature Caching (PR #467)

**Before (v0.8.x):**
- Rebuilt signature objects for every completion request
- 150+ function signatures reconstructed each time

**After (v1.0):**
- Lazy static signature cache
- One-time initialization per function

**Impact:** 10x faster completion for built-in functions

### Summary: Overall Performance

| Metric | v0.8.x | v1.0 | Improvement |
|--------|--------|------|-------------|
| Symbol lookup (10K symbols) | ~10ms | ~50μs | 200x |
| Scope resolution (deep nesting) | 300μs | 100μs | 3x |
| Parser (large files) | 250μs | 150μs | 1.7x |
| Built-in completion | 5ms | 500μs | 10x |
| LSP test suite | 60s+ | <10s | 6x |

---

## Security Enhancements

### 1. Path Traversal Protection (PR #388)

**Before (v0.8.x):**
```perl
# Vulnerable to path traversal
executeCommand("perl", "../../../etc/passwd")
```

**After (v1.0):**
```rust
// Validates all paths before execution
let safe_path = validate_path(input)?;
if !safe_path.starts_with(workspace_root) {
    return Err(SecurityError::PathTraversal);
}
```

**Protected Operations:**
- `workspace/executeCommand` - All execute commands validated
- `perl.runCritic` - Path normalization and validation
- `perl.formatDocument` - Temporary file path sanitization
- `perl.runTests` - Test file path validation

### 2. Command Injection Hardening (PR #332, #463, #475, #469, #466)

**Before (v0.8.x):**
```rust
// Vulnerable to shell injection
Command::new("sh")
    .arg("-c")
    .arg(format!("perl {}", user_input))
```

**After (v1.0):**
```rust
// Direct command invocation without shell
Command::new("perl")
    .arg(validated_input)
    .spawn()?
```

**Hardened Components:**
- DAP `evaluate` request (PR #475) - Prevents code injection in debug expressions
- DAP `launch_debugger` (PR #463) - Argument validation and sanitization
- `perlcritic` integration (PR #469) - Prevents argument injection
- `perltidy` integration (PR #469) - Safe argument passing
- `perldoc` lookup (PR #466) - Module name validation

**Security Best Practices:**
- Never use shell expansion for user input
- Validate all arguments before command execution
- Use direct command invocation (not `sh -c`)
- Sanitize file paths and module names

### 3. Argument Injection Prevention (PR #469)

**Before (v0.8.x):**
```rust
// Vulnerable: user input could inject flags
Command::new("perlcritic").args(user_args.split_whitespace())
```

**After (v1.0):**
```rust
// Safe: explicit argument validation
let validated = validate_critic_args(user_args)?;
Command::new("perlcritic")
    .arg("--single-file")
    .arg(validated_file)
```

**Protected Tools:**
- Perl::Critic integration
- Perl::Tidy formatting
- perldoc documentation
- Test execution commands

---

## Deprecated Features

### 1. Legacy Parser (perl-parser-pest)

**Status:** Maintained but not in default gate

**Migration:**
```toml
# Before (v0.8.x) - Pest parser
[dependencies]
perl-parser-pest = "0.8"

# After (v1.0) - Native parser (recommended)
[dependencies]
perl-parser = "1.0"
```

**Rationale:**
- Native parser (v3) is production-ready with ~100% syntax coverage
- 4-19x faster than Pest-based implementation
- Better error recovery and incremental parsing

**Timeline:**
- v1.0: Legacy parser remains available but not recommended
- v2.0: Legacy parser may be archived or removed

### 2. Internal APIs

**Status:** Semi-internal APIs marked with `#[doc(hidden)]`

**Examples:**
```rust
// These are now hidden from public docs
#[doc(hidden)]
pub mod positions;  // Use high-level SemanticModel instead

#[doc(hidden)]
pub mod internal_utils;  // Internal parser utilities
```

**Action Required:**
- Avoid depending on `#[doc(hidden)]` APIs
- Use public stable APIs: `Parser`, `SemanticModel`, `Node`, `NodeKind`
- File an issue if you need a hidden API made public

---

## Migration Steps

### Step 1: Update Dependencies

```toml
# Cargo.toml - Update all perl-lsp crates
[dependencies]
perl-parser = "1.0"
perl-lexer = "1.0"
perl-lsp = "1.0"

# Update Rust edition
[package]
edition = "2024"
rust-version = "1.92"
```

### Step 2: Update Rust Toolchain

```bash
# Install Rust 1.92+ if needed
rustup update stable

# Verify version
rustc --version  # Should show 1.92 or higher
```

### Step 3: Fix Breaking Changes

```bash
# Run cargo check to identify breaking changes
cargo check

# Common fixes needed:
# 1. Add ? operator for Result returns
# 2. Update edition in Cargo.toml
# 3. Handle new error types
```

### Step 4: Test Your Code

```bash
# Run full test suite
cargo test

# Run LSP tests if using LSP features
RUST_TEST_THREADS=2 cargo test -p perl-lsp

# Check for warnings
cargo clippy --workspace
```

### Step 5: Update VS Code Extension

```bash
# Uninstall old extension
code --uninstall-extension perl-language-server

# Install new extension
code --install-extension perl-language-server@1.0.0

# Remove deprecated settings from settings.json
# Delete: "perl-lsp.downloadBaseUrl"
```

### Step 6: Update Documentation

```bash
# Generate new documentation
cargo doc --open

# Review API changes
# Check migration guide for your specific use case
```

---

## LSP Client Changes

### Updated Capabilities

**v1.0 Capability Negotiation:**

```json
{
  "capabilities": {
    "textDocument": {
      "definition": { "dynamicRegistration": true, "linkSupport": true },
      "semanticTokens": { "requests": { "full": true, "range": true } },
      "inlayHint": { "dynamicRegistration": true },
      "codeAction": { "resolveSupport": { "properties": ["edit"] } },
      "completion": { "completionItem": { "resolveSupport": true } }
    },
    "workspace": {
      "workspaceEdit": { "documentChanges": true },
      "symbol": { "resolveProvider": true },
      "fileOperations": {
        "willRename": true,
        "didRename": true,
        "didDelete": true
      }
    }
  }
}
```

### New Features Available to Clients

| Feature | LSP Method | Status | Notes |
|---------|------------|--------|-------|
| Semantic Definitions | `textDocument/definition` | Enhanced | Uses semantic analysis |
| Inlay Hints | `textDocument/inlayHint` | GA | Parameter names, type hints |
| Completion Resolve | `completionItem/resolve` | GA | Lazy documentation loading |
| Code Action Resolve | `codeAction/resolve` | GA | Deferred edit computation |
| Workspace Symbols | `workspace/symbol` | GA | Cross-file search |
| File Operations | `workspace/willRename` | GA | Refactoring support |
| Call Hierarchy | `textDocument/prepareCallHierarchy` | GA | Navigate call chains |

### Protocol Compliance

**v1.0 Compliance:**
- **LSP Coverage**: 100% (53/53 advertised features)
- **Protocol Compliance**: 100% (88/88 including plumbing)
- **LSP Version**: 3.18

See [features.toml](../features.toml) for complete capability catalog.

---

## Library API Changes

### Parser API

**Stable APIs (unchanged):**
```rust
use perl_parser::{Parser, Node, NodeKind, ParseError};

// Core parsing API unchanged
let parser = Parser::new();
let result = parser.parse(source)?;

// AST traversal unchanged
for child in result.children() {
    match child.kind() {
        NodeKind::Subroutine => { /* handle */ },
        _ => {}
    }
}
```

**New APIs (additive):**
```rust
use perl_parser::semantic::SemanticModel;

// New semantic analysis API
let model = SemanticModel::build(&root, source)?;
let definition = model.definition_at(byte_offset)?;
let symbols = model.symbol_table();
let hover = model.hover_info_at(position)?;
```

### Lexer API

**Stable APIs (unchanged):**
```rust
use perl_lexer::{PerlLexer, Token, TokenType};

// Tokenization API unchanged
let lexer = PerlLexer::new(source);
for token in lexer {
    match token.token_type {
        TokenType::Identifier => { /* handle */ },
        _ => {}
    }
}
```

### Error Handling

**New Result Types:**
```rust
// Before (v0.8.x) - could panic
let ast = parser.parse_unchecked(source);

// After (v1.0) - returns Result
let ast = parser.parse(source)
    .map_err(|e| format!("Parse error at {}: {}", e.location, e.message))?;
```

**Error Types:**
- `ParseError`: Parsing failures with location info
- `SemanticError`: Semantic analysis errors
- `RefactoringError`: Refactoring validation failures
- `SecurityError`: Path/command validation failures

---

## VS Code Extension Changes

### Configuration Updates

**Removed Settings:**
```json
// REMOVED in v1.0
{
  "perl-lsp.downloadBaseUrl": "..."  // No longer supported
}
```

**New Settings:**
```json
{
  // Enhanced diagnostics configuration
  "perl-lsp.diagnostics.enableUninitialized": true,

  // TCP socket mode (optional)
  "perl-lsp.server.tcp": {
    "enabled": false,
    "port": 9257
  },

  // Refactoring options
  "perl-lsp.refactoring.enableBackup": true
}
```

### Command Palette Updates

**New Commands:**
- **Perl: Extract Method** - Extract selected code into subroutine
- **Perl: Inline Variable** - Inline variable into usage sites
- **Perl: Show Call Hierarchy** - Navigate call chains
- **Perl: Toggle Inlay Hints** - Show/hide parameter hints

**Enhanced Commands:**
- **Perl: Go to Definition** - Now uses semantic analysis
- **Perl: Find References** - Improved cross-file accuracy
- **Perl: Rename Symbol** - Validates sigil consistency

### UI Improvements

**Product Icons (PR #384):**
- All commands now have visual icons in menus
- Improved context menu organization
- File-specific command filtering (PR #470)

**Markdown Descriptions (PR #474):**
- Rich formatting in configuration descriptions
- Code examples in setting hover tooltips
- Better documentation links

**Silent Startup (PR #474):**
- No notification spam on LSP server start
- Progress indicators for long operations
- Cleaner startup experience

---

## DAP Changes

### New DAP Server Binary

**Installation:**
```bash
# Install DAP server separately
cargo install perl-dap

# Verify installation
perl-dap --version  # perl-dap 0.1.0
```

**Launch Configuration:**
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "perl",
      "request": "launch",
      "name": "Debug Perl Script",
      "program": "${file}",
      "args": [],
      "cwd": "${workspaceFolder}",
      "stopOnEntry": false
    }
  ]
}
```

### DAP Features (Phase 1)

**Supported:**
- ✅ Launch mode (start new process)
- ✅ Breakpoints (set/remove/list)
- ✅ Step operations (over/into/out)
- ✅ Stack traces
- ✅ Continue/pause execution
- ✅ Stdio transport (PR #330)
- ✅ CLI argument parsing (PR #374)
- ✅ Async BridgeAdapter (PR #369)

**Not Yet Supported:**
- ❌ Attach mode (connect to running process)
- ❌ Variable inspection (placeholder only)
- ❌ Expression evaluation (limited)
- ❌ Conditional breakpoints

**Roadmap:**
- Phase 2 (planned): Attach mode, variable/evaluate work
- Phase 3 (planned): Native adapter completeness

---

## Testing Changes

### Test Infrastructure

**Adaptive Threading (PR #140):**
```bash
# LSP tests now support adaptive threading
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2

# Resource-efficient semantic tests
just ci-lsp-def
```

**Test Harness Improvements (PR #253, #251):**
- JSON-RPC compliance fixes
- BrokenPipe elimination (123 tests unignored)
- Robust cleanup and process management
- Better timeout handling

### Test Performance

| Test Suite | v0.8.x | v1.0 | Improvement |
|------------|--------|------|-------------|
| LSP Behavioral Tests | 1560s+ | 0.31s | 5000x |
| User Story Tests | 1500s+ | 0.32s | 4700x |
| Workspace Tests | 60s+ | 0.26s | 230x |
| Overall Suite | 60s+ | <10s | 6x |

### Running Tests

```bash
# All tests
cargo test --workspace

# Parser tests only
cargo test -p perl-parser

# LSP tests with threading constraints
RUST_TEST_THREADS=2 cargo test -p perl-lsp

# Semantic definition tests (resource-efficient)
just ci-lsp-def

# Full CI gate (local-first)
nix develop -c just ci-gate
```

---

## Troubleshooting

### Issue: "edition '2024' is unstable" Error

**Problem:**
```
error: edition '2024' is unstable and only available with -Z unstable-options
```

**Solution:**
```bash
# Update Rust to 1.92+
rustup update stable

# Verify version
rustc --version  # Must be 1.92 or higher
```

### Issue: "unknown field `downloadBaseUrl`" in VS Code

**Problem:**
```
Unknown configuration setting: perl-lsp.downloadBaseUrl
```

**Solution:**
```json
// Remove from settings.json
{
  // Delete this line:
  // "perl-lsp.downloadBaseUrl": "...",
}
```

### Issue: Parser Returns Errors Instead of AST

**Problem:**
```rust
// This now returns Result instead of panicking
let ast = parser.parse(source);  // Error: mismatched types
```

**Solution:**
```rust
// Use ? operator for error propagation
let ast = parser.parse(source)?;

// Or handle explicitly
let ast = match parser.parse(source) {
    Ok(ast) => ast,
    Err(e) => {
        eprintln!("Parse error: {}", e);
        return;
    }
};
```

### Issue: LSP Tests Timeout

**Problem:**
```
test lsp_comprehensive_test ... timeout after 30s
```

**Solution:**
```bash
# Use adaptive threading
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2

# Or resource-efficient mode
just ci-lsp-def
```

### Issue: Missing Symbol Lookups

**Problem:**
Symbols defined in other files not found by go-to-definition.

**Solution:**
```bash
# Ensure workspace indexing completes
# Check LSP server logs for indexing status

# Trigger manual re-index (VS Code)
# Command Palette: "Perl: Restart Language Server"
```

### Issue: Refactoring Fails with Validation Error

**Problem:**
```
Refactoring failed: Invalid sigil in rename operation
```

**Solution:**
- Ensure symbol renames maintain sigil consistency
- Scalar `$var` must stay scalar (can't rename to `@var`)
- Use LSP rename command (respects sigils)
- Manual refactoring may bypass validation

### Issue: DAP Debugger Won't Start

**Problem:**
```
Debug adapter executable 'perl-dap' not found
```

**Solution:**
```bash
# Install DAP server separately
cargo install perl-dap

# Verify installation
which perl-dap  # Should show path

# Update launch.json if needed
{
  "debugServer": 4711,  // Or omit for stdio
}
```

### Issue: Security Validation Rejects Valid Paths

**Problem:**
```
SecurityError: Path traversal attempt detected
```

**Solution:**
- Ensure paths are within workspace root
- Use absolute paths resolved from workspace root
- Avoid `../` in user-provided paths
- Check LSP server logs for validation details

### Issue: Performance Regression After Upgrade

**Problem:**
LSP operations slower than v0.8.x

**Solution:**
```bash
# Check if incremental parsing is enabled
# Should see fast re-parse after edits

# Verify symbol cache is working
# First lookup slow, subsequent fast

# Check server logs for performance warnings
# Look for "indexing took >100ms" messages

# Report performance regression with:
# - File size
# - Operation type (completion/hover/etc)
# - LSP server logs
```

---

## Getting Help

### Documentation

- **Upgrade Guide**: This document
- **Migration Guide**: [docs/MIGRATION.md](MIGRATION.md) (v0.7.x → v0.8.x)
- **API Docs**: `cargo doc --open`
- **LSP Status**: [docs/LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md)
- **Roadmap**: [docs/ROADMAP.md](ROADMAP.md)

### Support Channels

- **GitHub Issues**: [tree-sitter-perl-rs/issues](https://github.com/EffortlessMetrics/perl-lsp/issues)
  - Tag with `upgrade`, `v1.0`, or `migration`
  - Include version numbers and error messages
  - Provide minimal reproduction if possible

- **Discussions**: Use for questions, not bug reports

### Reporting Issues

When reporting upgrade issues:

1. **Version info**: Include v0.8.x version and v1.0 version
2. **Error messages**: Full error output with stack traces
3. **Minimal reproduction**: Smallest code that shows the issue
4. **Platform**: OS, Rust version, LSP client (if applicable)
5. **Steps taken**: What you tried from this guide

### Example Issue Report

```markdown
**Title:** Parser API change breaks my code after v1.0 upgrade

**Environment:**
- perl-parser: 0.8.9 → 1.0.0
- Rust: 1.91.0
- OS: Ubuntu 22.04

**Problem:**
After upgrading to v1.0, parser.parse() now returns Result but my code expects direct AST.

**Error:**
```
error[E0308]: mismatched types
  --> src/main.rs:10:13
   |
10 |     let ast = parser.parse(source);
   |               ^^^^^^^^^^^^^^^^^^^^ expected `Node`, found `Result<Node, ParseError>`
```

**Code:**
```rust
let parser = Parser::new();
let ast = parser.parse(source);  // This used to work
```

**Expected:**
Clear upgrade path documented

**Actual:**
Compilation error
```

---

## Summary

**v1.0 brings significant improvements:**

✅ **Semantic Analyzer**: Precise symbol resolution and type inference
✅ **Refactoring Engine**: Extract method, inline variable, move code
✅ **Performance**: 3-200x faster symbol lookups and parsing
✅ **Security**: Path traversal and command injection protection
✅ **LSP 3.18**: 100% protocol compliance, inlay hints, call hierarchy
✅ **DAP Phase 1**: Native debugger with launch/breakpoints/step
✅ **Test Infrastructure**: 5000x faster test suite, adaptive threading

**Breaking changes are minimal:**
- Rust 2024 Edition (standard upgrade path)
- Strict error handling (improves robustness)
- VS Code config cleanup (one setting removed)

**Follow the migration steps** in order, test thoroughly, and report any issues.

**Most users should experience a smooth upgrade** with significant performance and feature improvements.

---

*Last Updated: 2026-01-22*
*For latest updates, see: [CHANGELOG.md](../CHANGELOG.md)*
