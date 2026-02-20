# API Stability & Version Policy - v0.9.x (Production-Ready) GA-Lock

**MSRV:** 1.89 • **Edition:** 2024 • **Status:** General Availability (GA)

## Executive Summary

This document defines the **GA-lock** stability guarantees for Perl LSP v0.9.x (Production-Ready) and beyond. GA-lock establishes formal contracts for API stability, wire protocol invariants, versioning policy, and platform support that users can depend on for production deployments.

**What "GA-lock" means:**
- Published crate APIs are stable under semantic versioning
- LSP wire protocol capabilities are locked and versioned
- Breaking changes follow strict deprecation cycles
- Minimum supported platforms are explicitly documented
- Binary and source distribution commitments are defined

---

## Table of Contents

1. [Published Artifacts](#published-artifacts)
2. [GA-Lock Guarantees](#ga-lock-guarantees)
3. [Semantic Versioning Policy](#semantic-versioning-policy)
4. [Wire Protocol Invariants](#wire-protocol-invariants)
5. [Platform Support Matrix](#platform-support-matrix)
6. [API Surface Stability](#api-surface-stability)
7. [Breaking Changes Policy](#breaking-changes-policy)
8. [Deprecation Process](#deprecation-process)
9. [Feature Flags](#feature-flags)
10. [Performance Guarantees](#performance-guarantees)
11. [Security Support](#security-support)
12. [Migration Guides](#migration-guides)

---

## Published Artifacts

### What We Ship (v0.9.x (Production-Ready)+)

| Distribution | Format | Support Level | Update Cadence |
|--------------|--------|---------------|----------------|
| **Binaries** | GitHub Releases (tar.gz, zip) | Production | Every release |
| **Crates** | crates.io | Production | Every release |
| **VS Code Extension** | VS Marketplace | Production | Every release |
| **Homebrew** | Formula (macOS/Linux) | Production | Automated on release |
| **Source** | GitHub releases + tags | Production | Every release |

### Published Crates (v0.9.x (Production-Ready) GA)

| Crate | Version | Purpose | Stability | SemVer Commitment |
|-------|---------|---------|-----------|-------------------|
| [perl-parser](https://crates.io/crates/perl-parser) | 0.9.x (Production-Ready) | Parser & AST | **GA-locked** | Yes - strict SemVer |
| [perl-lexer](https://crates.io/crates/perl-lexer) | 0.9.x (Production-Ready) | Tokenizer | **GA-locked** | Yes - strict SemVer |
| [perl-lsp](https://crates.io/crates/perl-lsp) | 0.9.x (Production-Ready) | LSP Server Binary | **GA-locked** | Yes - strict SemVer |
| [perl-corpus](https://crates.io/crates/perl-corpus) | 0.9.x (Production-Ready) | Test corpus | **GA-locked** | Yes - best-effort API |
| [perl-dap](https://crates.io/crates/perl-dap) | 0.2.0 | Debug Adapter | **Preview** | No - pre-1.0 |
| [perl-parser-pest](https://crates.io/crates/perl-parser-pest) | 0.9.x (Production-Ready) | Legacy parser | **Deprecated** | Maintenance only |

**Note:** Only crates at 1.0+ provide GA-lock guarantees. `perl-dap` remains in preview with no stability guarantees until 1.0.

---

## GA-Lock Guarantees

### What GA-Lock Means

**GA-lock** is a formal stability contract that guarantees:

1. **API Stability**: Public APIs in GA-locked crates will not break without major version bump
2. **Wire Protocol Stability**: LSP capabilities advertised in v0.9.x (Production-Ready) will remain compatible through all v1.x releases
3. **Behavioral Compatibility**: Bug-fix releases will not change semantics of correct programs
4. **MSRV Stability**: Minimum Supported Rust Version only increases in minor releases with 6-month notice
5. **Data Format Stability**: Serialization formats (S-expressions, JSON-RPC) remain parseable by older clients
6. **Error Model Stability**: Error types and recovery behavior remain consistent

### What GA-Lock Does NOT Guarantee

- **Performance**: Performance improvements may change timing characteristics
- **Diagnostics**: Warning messages and diagnostic text may change in patch releases
- **Internal APIs**: `#[doc(hidden)]`, `pub(crate)`, and test-only APIs are unstable
- **Experimental Features**: Features marked `experimental` or behind unstable feature flags
- **Output Formatting**: Pretty-printed output (debug formatting, error displays) may change
- **Resource Usage**: Memory/CPU usage patterns may change with optimizations

---

## Semantic Versioning Policy

Perl LSP strictly follows [Semantic Versioning 2.0.0](https://semver.org/) with the following interpretation:

### Major Releases (X.0.0) - Breaking Changes Allowed

**Breaking changes include:**
- Removing public API functions, types, or modules
- Changing function signatures in incompatible ways
- Changing behavior of existing APIs in ways that break correct programs
- Removing or renaming LSP capabilities advertised to clients
- Changing wire protocol message formats
- Bumping MSRV beyond current stable Rust
- Removing feature flags or changing their semantics

**Major release cadence:** Approximately every 18-24 months, aligned with significant Rust ecosystem changes.

**Example breaking changes:**
```rust
// v1.x.x
pub fn parse(source: &str) -> Result<Node, ParseError>

// v2.0.0 - signature change (BREAKING)
pub fn parse(source: &str, config: &ParseConfig) -> Result<Ast, Error>
```

### Minor Releases (1.Y.0) - Additive Changes Only

**Allowed changes:**
- Adding new public API functions, types, or modules
- Adding new LSP capabilities (backward compatible)
- Adding new `NodeKind` variants or token types
- Deprecating APIs (with warnings and migration paths)
- Adding optional parameters with defaults
- Bumping MSRV with 6-month deprecation notice
- Performance improvements that don't change semantics
- Bug fixes that may change behavior of incorrect programs

**Minor release cadence:** Approximately every 6-8 weeks, driven by feature development.

**Example additive changes:**
```rust
// v0.9.x (Production-Ready)
pub enum NodeKind {
    Sub,
    Package,
}

// v1.1.0 - new variant (ALLOWED in minor)
pub enum NodeKind {
    Sub,
    Package,
    Async,  // New in 1.1.0
}
```

**MSRV Policy for Minor Releases:**
- MSRV may increase in minor releases ONLY
- 6-month deprecation notice required before MSRV bump
- Notice appears in release notes and `Cargo.toml` comments
- Users can pin to older minor versions if MSRV is a concern

### Patch Releases (1.2.Z) - Bug Fixes Only

**Allowed changes:**
- Bug fixes that restore documented behavior
- Security fixes (may change behavior of vulnerable code)
- Documentation corrections
- Internal refactoring with no observable effects
- Diagnostic message improvements
- Performance optimizations that don't change semantics

**Patch release cadence:** As needed for critical bugs and security issues.

**MSRV guarantee:** Patch releases NEVER increase MSRV.

---

## Wire Protocol Invariants

### LSP Protocol Stability

The LSP server advertises capabilities via the `initialize` handshake. **GA-lock guarantees:**

1. **Capability Stability**: Once advertised in v0.9.x (Production-Ready), capabilities remain available through all v1.x releases
2. **Request/Response Compatibility**: Message formats remain backward compatible
3. **Method Names**: LSP method names (e.g., `textDocument/completion`) never change in v1.x
4. **Capability Negotiation**: Servers honor client capabilities for conditional feature support
5. **Error Codes**: LSP error codes remain stable (may add new codes, never remove)

### Capability Snapshot (v0.9.x (Production-Ready))

The canonical capability set is defined in [`features.toml`](../features.toml). As of v0.9.x (Production-Ready):

| Area | Capabilities | GA Status |
|------|-------------|-----------|
| **Text Document** | 41 features | ✅ GA-locked |
| **Workspace** | 26 features | ✅ GA-locked |
| **Window** | 9 features | ✅ GA-locked |
| **Protocol** | 9 features | ✅ GA-locked |
| **Notebook** | 2 features | ⚠️ Preview (unstable) |
| **Debug** | 2 features | ⚠️ Preview (unstable) |

**Total GA-locked capabilities:** 88/89 (99% of LSP 3.18 protocol)

**Verification:**
```bash
# Validate capability snapshot against implementation
just status-check

# View current advertised capabilities
cargo run -p perl-lsp -- --stdio < /dev/null 2>&1 | grep -A 100 "capabilities"
```

### Wire Protocol Testing

All LSP capabilities have integration tests that verify wire protocol compatibility:

```bash
# Run LSP protocol compliance tests
cargo test -p perl-lsp --test lsp_comprehensive_3_17_test
cargo test -p perl-lsp --test lsp_*_tests

# Verify capability advertise/implement alignment
just ci-gate  # Includes LSP semantic tests
```

**Test coverage requirement:** All GA-locked capabilities must have integration tests covering:
- Initialize handshake advertising the capability
- Successful request/response round-trips
- Error handling and edge cases
- Client capability negotiation

---

## Platform Support Matrix

### Tier 1 Platforms (Guaranteed Support)

Tier 1 platforms receive:
- Pre-built binaries for every release
- CI testing on every commit
- Bug fixes within 7 days of confirmed issue
- Performance optimization attention
- Security patches within 24 hours of disclosure

| Platform | Architecture | Toolchain | Binary Format | Support Level |
|----------|-------------|-----------|---------------|---------------|
| **Linux (GNU)** | x86_64 | stable | ELF (dynamic) | ✅ Tier 1 |
| **Linux (musl)** | x86_64 | stable | ELF (static) | ✅ Tier 1 |
| **Linux (GNU)** | aarch64 | stable | ELF (dynamic) | ✅ Tier 1 |
| **macOS** | x86_64 | stable | Mach-O | ✅ Tier 1 |
| **macOS** | aarch64 | stable | Mach-O | ✅ Tier 1 |
| **Windows** | x86_64 | stable-msvc | PE (MSVC) | ✅ Tier 1 |

**Build targets:**
```bash
# Tier 1 platforms
x86_64-unknown-linux-gnu      # Ubuntu 22.04+ (GLIBC 2.35+)
x86_64-unknown-linux-musl     # Alpine Linux, static binary
aarch64-unknown-linux-gnu     # ARM64 Linux (Raspberry Pi 4+)
x86_64-apple-darwin           # Intel Mac (macOS 10.15+)
aarch64-apple-darwin          # Apple Silicon (macOS 11.0+)
x86_64-pc-windows-msvc        # Windows 10+ (x64)
```

### Tier 2 Platforms (Best-Effort Support)

Tier 2 platforms receive:
- Source code builds documented
- Community-contributed bug fixes accepted
- No pre-built binaries (users build from source)
- No CI coverage guarantee

| Platform | Architecture | Build Method | Notes |
|----------|-------------|--------------|-------|
| Linux (GNU) | i686 | cargo build | 32-bit x86 |
| Linux (musl) | aarch64 | cargo build | ARM64 Alpine |
| FreeBSD | x86_64 | cargo build | Community supported |
| NetBSD | x86_64 | cargo build | Community supported |
| OpenBSD | x86_64 | cargo build | Community supported |
| Windows (GNU) | x86_64 | cargo build | MinGW toolchain |

### Unsupported Platforms

Platforms without Rust compiler support are not supported:
- 32-bit ARM (armv7)
- MIPS architectures
- RISC-V (planned Tier 2 when Rust stabilizes)
- PowerPC architectures
- WASM (parser may work, LSP server does not)

### Platform Testing Policy

**Tier 1 platforms:**
- All releases tested on CI before release
- Gate checks (`just ci-gate`) must pass on all Tier 1 platforms
- Performance benchmarks tracked for Linux x86_64 and macOS aarch64

**Tier 2 platforms:**
- Community contributors verify builds
- Issues accepted but prioritized below Tier 1
- May be promoted to Tier 1 with sustained community support

---

## API Surface Stability

### perl-parser Stable APIs

**Core parsing API (GA-locked since v0.9.x (Production-Ready)):**

```rust
// Parser construction and execution
pub struct Parser { /* ... */ }
impl Parser {
    pub fn new() -> Self;
    pub fn parse(&mut self, source: &str) -> Result<Node, ParseError>;
}

// AST representation
#[non_exhaustive]
pub enum NodeKind {
    // Core variants (stable)
    Sub, Package, Use, If, While, For, Foreach,
    BinaryOp, UnaryOp, Assign, Call, Var, Literal,
    Block, Statement, Expression,
    // ... additional variants may be added in minor releases
}

pub struct Node {
    pub kind: NodeKind,
    pub children: Vec<Node>,
    pub source_location: SourceLocation,
    // ... additional fields may be added (non-breaking)
}

// Error handling (stable)
pub struct ParseError {
    pub message: String,
    pub location: SourceLocation,
}

// Position conversion utilities (stable)
pub fn offset_to_position(source: &str, offset: usize) -> Position;
pub fn position_to_offset(source: &str, position: Position) -> Option<usize>;

// S-expression serialization (stable)
impl Node {
    pub fn to_sexp(&self) -> String;
}
```

**Stability guarantees:**
- `Parser::parse()` signature never changes in v1.x
- `NodeKind` is `#[non_exhaustive]` - new variants may be added
- `Node` structure fields are public and stable
- `to_sexp()` output format remains parseable (modulo whitespace)
- Position helpers maintain UTF-8 ↔ UTF-16 conversion accuracy

**Breaking change examples (v2.0.0 only):**
```rust
// BREAKING: Changing parse signature
pub fn parse(&mut self, source: &str, config: &ParseConfig) -> Result<Ast, Error>

// BREAKING: Removing NodeKind variant
pub enum NodeKind {
    Sub,
    // Package removed (BREAKING)
}

// BREAKING: Changing Node field types
pub struct Node {
    pub kind: NodeKind,
    pub children: Arc<[Node]>,  // Changed from Vec<Node> (BREAKING)
}
```

### perl-lexer Stable APIs

**Tokenization API (GA-locked since v0.9.x (Production-Ready)):**

```rust
pub struct PerlLexer<'a> { /* ... */ }

impl<'a> PerlLexer<'a> {
    pub fn new(source: &'a str) -> Self;
    pub fn next_token(&mut self) -> Option<Token<'a>>;
}

pub struct Token<'a> {
    pub kind: TokenType,
    pub span: &'a str,
    pub offset: usize,
}

#[non_exhaustive]
pub enum TokenType {
    // Core token types (stable)
    Identifier, Keyword, Operator, Literal,
    Comment, Whitespace, Sigil, Arrow,
    // ... additional types may be added in minor releases
}
```

**Stability guarantees:**
- Token iteration interface (`next_token()`) remains stable
- `TokenType` is `#[non_exhaustive]` - new types may be added
- Token span lifetime tied to source string (lifetime stability)

### perl-lsp Binary Interface

**Command-line interface (GA-locked since v0.9.x (Production-Ready)):**

```bash
# Stdio mode (stable)
perl-lsp --stdio

# TCP socket mode (added in v0.9.1, stable in v0.9.x (Production-Ready))
perl-lsp --tcp 127.0.0.1:9257

# Version/help (stable)
perl-lsp --version
perl-lsp --help

# LSP capabilities (stable)
perl-lsp --capabilities  # JSON output of advertised capabilities
```

**Stability guarantees:**
- `--stdio` flag and behavior remain stable
- Exit codes follow LSP specification (0 = success, 1 = error)
- JSON-RPC wire format follows LSP 3.18 specification
- Configuration options loaded from `workspace/configuration`

**Configuration schema stability:**
```json
{
  "perl-lsp": {
    "trace.server": "off|messages|verbose",
    "diagnostics.enable": true,
    "completion.enable": true,
    // ... additional options may be added (non-breaking)
  }
}
```

---

## Breaking Changes Policy

### Pre-1.0 Policy (Historical)

**v0.x releases (before v0.9.x (Production-Ready)):**
- Breaking changes allowed in minor releases (0.Y.0)
- Deprecation warnings provided for at least one minor release
- CHANGELOG documents all breaking changes
- Migration guides provided for major API shifts

**Example from v0.9.0:**
```markdown
### Breaking Changes
- `Parser::parse()` now returns `Result<Node, ParseError>` instead of `Option<Node>`
- Migration: Replace `.unwrap()` with `.expect("parse error")` or proper error handling
```

### Post-1.0 Policy (Current)

**v1.x releases:**
- Breaking changes ONLY allowed in major releases (v2.0.0, v3.0.0, etc.)
- Minimum 6-month deprecation cycle required
- Deprecated items issue warnings pointing to migration paths
- `#[deprecated]` attribute used with clear `since` and `note` values

**Deprecation example:**
```rust
#[deprecated(since = "1.2.0", note = "use `Parser::parse_with_config()` instead")]
pub fn parse_legacy(source: &str) -> Result<Node, ParseError> {
    // Still works, but warns at compile time
}
```

**Deprecation timeline:**
1. **Release N (e.g., v1.2.0):** Deprecation warning added, alternative API introduced
2. **Release N+1 (e.g., v1.3.0, 6+ months later):** Deprecation still present, docs updated
3. **Release M (v2.0.0, 12+ months after N):** Deprecated API removed

---

## Deprecation Process

### How Deprecations Work

1. **Announcement (release notes):**
   ```markdown
   ### Deprecated
   - `Parser::parse_legacy()` deprecated in favor of `Parser::parse_with_config()`
   - Migration guide: https://example.com/migration/v1.2-parse-config
   ```

2. **Compiler warnings:**
   ```rust
   #[deprecated(since = "1.2.0", note = "use `parse_with_config()` instead")]
   pub fn parse_legacy(source: &str) -> Result<Node, ParseError>
   ```

3. **Documentation updates:**
   ```rust
   /// **DEPRECATED:** This function is deprecated since v1.2.0.
   /// Use [`parse_with_config`](Self::parse_with_config) instead.
   ///
   /// This function will be removed in v2.0.0.
   ```

4. **Migration guides:**
   - Dedicated section in `docs/MIGRATION.md`
   - Code examples showing before/after
   - Automated migration tools where feasible (cargo-fix compatible)

5. **Removal (major version only):**
   - Removed in next major release (v2.0.0)
   - CHANGELOG documents removal with migration path

### Deprecation Policy for LSP Capabilities

**Capability deprecation follows a stricter process:**

1. **Announcement (6 months before):** Capability marked deprecated in `initialize` response
2. **Documentation:** Alternative capability documented in LSP implementation guide
3. **Support period:** Deprecated capability remains functional for 12+ months
4. **Removal:** Capability removed only in major version (v2.0.0)

**Example capability deprecation:**
```rust
// v1.5.0 - deprecation announced
server_capabilities.deprecated_capabilities = Some(vec![
    "textDocument/oldFeature".to_string()
]);

// v2.0.0 - capability removed
// Capability no longer advertised or implemented
```

---

## Feature Flags

### Stable Feature Flags (v0.9.x (Production-Ready)+)

| Flag | Purpose | Stability | Default |
|------|---------|-----------|---------|
| `pure-rust` | Pest-based parser (v2) | **Deprecated** | No |
| `ts-compat` | Tree-sitter compatibility mode | **Stable** | No |
| `cli` | Build command-line binaries | **Stable** | Yes |

### Unstable Feature Flags

| Flag | Purpose | Stability | Default |
|------|---------|-----------|---------|
| `workspace` | Cross-file analysis (experimental) | **Unstable** | No |
| `expose_lsp_test_api` | Test-only LSP internals | **Test-only** | No |

**Stability guarantees:**
- **Stable flags:** Follow semantic versioning, may not be removed in v1.x
- **Unstable flags:** May change behavior or be removed in minor releases
- **Test-only flags:** No stability guarantees, internal use only
- **Deprecated flags:** Supported for 12+ months before removal

**Feature flag usage:**
```toml
# Cargo.toml
[dependencies]
perl-parser = { version = "1.0", features = ["ts-compat"] }
```

**Deprecation path:**
```rust
// v0.9.x (Production-Ready) - stable feature
#[cfg(feature = "ts-compat")]
pub fn tree_sitter_mode() -> bool { true }

// v1.5 - deprecation warning
#[deprecated(since = "1.5.0", note = "ts-compat is deprecated, use default parser")]
#[cfg(feature = "ts-compat")]
pub fn tree_sitter_mode() -> bool { true }

// v2.0 - feature removed
// #[cfg(feature = "ts-compat")] - removed entirely
```

---

## Performance Guarantees

### Complexity Guarantees (GA-locked)

**Parser performance (v0.9.x (Production-Ready)+):**
- **Time complexity:** O(n) in input size for valid Perl code
- **Space complexity:** O(n) for AST construction (no exponential blowups)
- **Streaming:** Parser does not require entire file in memory (incremental parsing)

**LSP response times (v0.9.x (Production-Ready)+):**
- **Single-file operations:** <50ms for files <10K lines (p95)
- **Workspace indexing:** <100ms initial index, <10ms incremental (p95)
- **Completion:** <20ms typical, <50ms worst-case (p95)
- **Go-to-definition:** <10ms single-file, <30ms cross-file (p95)

**Measured performance (v0.9.x (Production-Ready) baseline):**

| File Size | Parse Time (p50) | Parse Time (p95) |
|-----------|------------------|------------------|
| 100 lines | 6-8µs | 12µs |
| 1K lines | 12-18µs | 25µs |
| 10K lines | 150-200µs | 300µs |

**Verification:**
```bash
# Run performance benchmarks
cargo bench -p perl-parser

# LSP performance tests
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_performance_tests
```

### Performance Regression Policy

**Performance is not API:**
- Performance improvements are allowed in patch releases
- Performance regressions >20% are considered bugs
- Benchmarks tracked in CI for Tier 1 platforms (Linux x86_64, macOS aarch64)

**Performance bug reporting:**
- File issue with reproducible benchmark
- Include platform, Rust version, and file size
- P0 if regression >50%, P1 if regression >20%

---

## Security Support

### Security Policy (v0.9.x (Production-Ready)+)

**Supported versions:**
| Version | Security Fixes | Support Duration |
|---------|----------------|------------------|
| 1.x (latest) | ✅ Yes | Ongoing |
| 1.x (previous minor) | ✅ Yes | 6 months after next minor |
| 0.9.x | ⚠️ Critical only | Until 2027-01-01 |
| 0.8.x | ❌ No | End of life |

**Security response times:**
- **Critical (RCE, privilege escalation):** Patch within 24 hours, emergency release
- **High (data exposure, DoS):** Patch within 7 days, expedited release
- **Medium (info disclosure):** Patch within 30 days, regular release cycle
- **Low (minor issues):** Patch in next scheduled release

### Vulnerability Disclosure

**Report security issues to:** security@perl-lsp.org (or GitHub Security Advisories)

**Disclosure process:**
1. Reporter sends encrypted email (PGP key on website)
2. Maintainers acknowledge within 24 hours
3. Investigation and patch development (private repository)
4. Coordinated disclosure 7-14 days after patch release
5. CVE assignment and public advisory

### Security Hardening (v0.9.x (Production-Ready)+)

**Production hardening commitments:**
- **No panics on invalid input:** Parser returns `Result` for all malformed input
- **No unwrap/expect in production code:** Enforced by CI (see Issue #143)
- **Path traversal protection:** All file operations validate paths (PR #388)
- **Command injection hardening:** No shell interpolation (PR #332)
- **Memory safety:** Rust memory safety + additional bounds checking
- **Resource limits:** Configurable limits on recursion depth, file size, workspace size

---

## Migration Guides

### Upgrade Paths

**From v0.9.x to v0.9.x (Production-Ready):**
- See [`docs/MIGRATION.md`](MIGRATION.md) for detailed migration guide
- **Breaking changes:** MSRV bumped to 1.89 (Rust 2024 edition)
- **API changes:** None - v0.9.x (Production-Ready) is a stability release
- **Configuration:** `perl-lsp` config schema unchanged

**From v0.8.x to v0.9.x (Production-Ready):**
- **Position helpers:** `offset_to_position()` signature changed (v0.8.0)
- **Error types:** `ParseError` structure changed (v0.8.5)
- **LSP capabilities:** 8 capabilities promoted to GA (v0.9.0)

**From tree-sitter-perl C to perl-parser:**
- See [`docs/MIGRATION.md`](MIGRATION.md) section "Migrating from tree-sitter-perl C"
- **Parser API:** Completely different API surface (not compatible)
- **Performance:** 4-19x faster with maintained correctness
- **S-expression output:** Compatible format for test automation

### Version Compatibility Matrix

| perl-parser | perl-lexer | perl-corpus | perl-lsp | MSRV | Notes |
|-------------|------------|-------------|----------|------|-------|
| 1.0.x | 1.0.x | 1.0.x | 1.0.x | 1.89 | Current GA (2024 edition) |
| 0.9.x | 0.9.x | 0.9.x | 0.9.x | 1.89 | Previous stable (semantic analyzer) |
| 0.8.x | 0.8.x | 0.8.x | 0.8.x | 1.85 | Previous stable (workspace config) |
| 0.7.x | 0.7.x | 0.7.x | - | 1.80 | Security fixes only until 2026-04-01 |

**Cross-version compatibility:**
- Patch versions (`1.0.Z`) are fully compatible
- Minor versions (`1.Y.0`) are backward compatible (additive changes only)
- Major versions (`X.0.0`) may break compatibility

---

## Error Model Stability

### Parser Error Handling (GA-locked)

**Guarantees:**
- `Parser::parse()` **never panics** on malformed input
- All errors return `Result<Node, ParseError>` with source location
- Error recovery attempts to continue parsing after errors
- Error messages may change (not part of stable API)

**Error structure (stable):**
```rust
pub struct ParseError {
    pub message: String,         // Human-readable (may change)
    pub location: SourceLocation, // Source position (stable)
}

pub struct SourceLocation {
    pub start: usize,  // UTF-8 byte offset (stable)
    pub end: usize,    // UTF-8 byte offset (stable)
}
```

### LSP Server Error Handling (GA-locked)

**Guarantees:**
- LSP server **never panics** on invalid requests
- Unknown methods return JSON-RPC error with code `-32601` (method not found)
- Malformed requests return JSON-RPC error with code `-32700` (parse error)
- All errors include structured error information per LSP specification

**Error codes (stable per LSP spec):**
```rust
const PARSE_ERROR: i32 = -32700;
const INVALID_REQUEST: i32 = -32600;
const METHOD_NOT_FOUND: i32 = -32601;
const INVALID_PARAMS: i32 = -32602;
const INTERNAL_ERROR: i32 = -32603;
```

---

## Position Encoding Stability

### Position Encoding (GA-locked)

**Guarantees (v0.9.x (Production-Ready)+):**
- **Parser positions:** UTF-8 byte offsets (0-based)
- **LSP positions:** UTF-16 code unit offsets with 0-based lines/columns
- **Converters stable:** `offset_to_position()`, `position_to_offset()`
- **Line endings:** CRLF and LF both supported transparently

**Position conversion accuracy:**
```rust
// Stable API (v0.9.x (Production-Ready)+)
pub fn offset_to_position(source: &str, offset: usize) -> Position;
pub fn position_to_offset(source: &str, position: Position) -> Option<usize>;

// Guarantees:
// - Handles Unicode grapheme clusters correctly
// - CRLF treated as single line ending
// - UTF-16 surrogate pairs handled correctly
// - Round-trip safety: position_to_offset(offset_to_position(src, off)) == off
```

---

## Support Lifecycle

### Version Support Timeline (v0.9.x (Production-Ready)+)

**Long-Term Support (LTS):**
- **v0.9.x LTS:** First LTS release, supported until 2028-01-01
- **LTS policy:** 24-month support window from release date
- **LTS updates:** Security fixes + critical bugs only

**Standard Support:**
- **Current stable (1.x latest):** Full support (features, bugs, security)
- **Previous minor (1.x-1):** Bug fixes + security for 6 months
- **Older minors:** Security fixes only for 12 months

**Timeline visualization:**
```
2026-01 |---- v0.9.x (Production-Ready) LTS ------------------------------------> 2028-01
2026-07 |---- v1.1.0 -----> 2027-01
2027-01 |---- v1.2.0 -----> 2027-07
2027-07 |---- v1.3.0 -----> 2028-01
2028-01 |---- v2.0.0 LTS ------------------------------------> 2030-01

Legend:
|-----> Full support (features, bugs, security)
----> Bug fixes + security
...> Security fixes only
```

### End of Life Policy

**When versions reach end of life:**
- Documented in release notes 3 months before EOL
- Security advisory posted if critical vulnerabilities exist
- Users encouraged to upgrade to supported version
- Source code remains available on GitHub (archived releases)

---

## How to Report Stability Issues

### Stability Bug Reports

**What constitutes a stability issue:**
1. API breakage in patch or minor release (violates SemVer)
2. LSP capability removed or broken without major version bump
3. Wire protocol incompatibility with documented behavior
4. Performance regression >20% in stable API
5. MSRV increase in patch release

**How to report:**
1. Check this document for guarantees
2. File issue with label `api-stability`
3. Include minimal reproduction
4. Specify version numbers and feature flags used
5. Reference relevant section of STABILITY.md

**Example report:**
```markdown
Title: [Stability] Parser::parse() signature changed in patch release

Version: v0.9.x (Production-Ready).3 (previous: v0.9.x (Production-Ready).2)
Component: perl-parser

Description:
The signature of `Parser::parse()` changed from:
  v0.9.x (Production-Ready).2: `pub fn parse(&mut self, source: &str) -> Result<Node, ParseError>`
  v0.9.x (Production-Ready).3: `pub fn parse(&mut self, source: &str, config: &Config) -> Result<Node, ParseError>`

This breaks compilation of code using v0.9.x (Production-Ready).2 API.

Violates: Section "Semantic Versioning Policy" - signature changes require major version.

Reproduction: [link to minimal code sample]
```

---

## Verification and Enforcement

### Automated Stability Checks

**CI enforcement (v0.9.x (Production-Ready)+):**
```bash
# Semantic versioning compliance
cargo semver-checks check-release

# API surface documentation coverage
cargo test --doc
cargo test -p perl-parser --test missing_docs_ac_tests

# LSP capability snapshot validation
just status-check

# Wire protocol compatibility tests
cargo test -p perl-lsp --test lsp_comprehensive_3_17_test
```

**Pre-release checklist:**
- [ ] `cargo semver-checks` passes (no unexpected breaking changes)
- [ ] `just ci-gate` green on all Tier 1 platforms
- [ ] LSP capability snapshot matches `features.toml`
- [ ] CHANGELOG documents all changes with SemVer classification
- [ ] Migration guide updated for breaking changes (major releases only)
- [ ] Performance benchmarks within 20% of previous release

---

## Document Maintenance

**This document is authoritative for API stability questions.**

**Last updated:** 2026-01-22 (v0.9.x (Production-Ready) GA-Lock Release)

**Document history:**
- 2026-01-22: v0.9.x (Production-Ready) GA-Lock comprehensive stability documentation
- 2025-09-05: v0.8.8 GA production workspace configuration
- 2025-06-01: v0.8.0 initial stability statement

**Feedback:** File issues with label `documentation` for corrections or clarifications.

---

## Summary Checklist: What v0.9.x (Production-Ready) GA-Lock Guarantees

✅ **API Stability:** Public APIs stable under SemVer (breaking changes only in major releases)
✅ **Wire Protocol:** LSP capabilities locked, backward compatible through v1.x
✅ **Platform Support:** 6 Tier 1 platforms with pre-built binaries and CI testing
✅ **Versioning Policy:** Strict SemVer with 6-month deprecation cycles
✅ **Performance:** O(n) parsing, <50ms LSP responses, no exponential blowups
✅ **Security:** 24-hour critical patches, coordinated disclosure, memory safety
✅ **Error Handling:** No panics on invalid input, structured errors with source locations
✅ **MSRV Policy:** Rust 1.89+ (2024 edition), increases only in minor releases with 6-month notice
✅ **Documentation:** Comprehensive migration guides and API documentation
✅ **Testing:** 535+ tests, 100% LSP coverage, integration tests for all capabilities

**Verification command:**
```bash
nix develop -c just ci-gate
```

This command validates all GA-lock guarantees before every release.
