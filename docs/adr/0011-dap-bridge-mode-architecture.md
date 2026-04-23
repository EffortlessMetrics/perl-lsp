# ADR-0011: DAP Bridge Mode Architecture

**Status**: Accepted
**Date**: 2025-06-15
**Decision Makers**: DAP Team, LSP Architecture Committee
**Related**: [DAP User Guide](../tutorials/DAP_USER_GUIDE.md), [DAP Security Specification](../DAP_SECURITY_SPECIFICATION.md)

## Context

The Debug Adapter Protocol (DAP) is the standard protocol for editor-agnostic debugging, used by VS Code, Neovim, and other editors. For Perl debugging support, there was an existing solution: Perl::LanguageServer, which provides both LSP and DAP functionality.

### Problem Statement

1. **Existing Investment**: Perl::LanguageServer already implements Perl debugging via the Perl debugger (`perl -d`)
2. **Protocol Compliance**: Editors expect standard DAP protocol, not custom implementations
3. **Maintenance Burden**: Building a complete native debugger from scratch is substantial
4. **Feature Parity**: Perl::LanguageServer has years of debugging feature development
5. **Integration Goal**: Provide seamless debugging experience within the perl-lsp ecosystem

### Design Challenge

How to provide DAP debugging support while leveraging existing Perl debugging tooling?

## Decision

**We implement a bridge mode architecture where the DAP adapter translates between DAP protocol and Perl::LanguageServer's debugging interface.**

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         Editor (VS Code)                         │
│                    DAP Protocol Client                           │
└─────────────────────────────────┬───────────────────────────────┘
                                  │ DAP Protocol
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│                      perl-dap (Rust)                             │
│                   DAP Bridge Adapter                             │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────────┐    ┌──────────────────┐                   │
│  │ DAP Protocol     │    │ Bridge           │                   │
│  │ Handler          │◄──►│ Translation      │                   │
│  │ (decode/encode)  │    │ Layer            │                   │
│  └──────────────────┘    └────────┬─────────┘                   │
│                                   │                              │
│  ┌──────────────────┐    ┌────────▼─────────┐                   │
│  │ Session          │    │ Native Adapter   │                   │
│  │ Management       │    │ (direct perl -d) │                   │
│  └──────────────────┘    └──────────────────┘                   │
│                                                              │
└─────────────────────────────────────────────────────────────────┘
                                  │
                    ┌─────────────┴─────────────┐
                    │                           │
                    ▼                           ▼
        ┌─────────────────────┐     ┌─────────────────────┐
        │ Perl::LanguageServer│     │ Native Perl Debug   │
        │ (Bridge Mode)       │     │ (Native Mode)       │
        │ - BridgeAdapter     │     │ - Direct perl -d    │
        └─────────────────────┘     └─────────────────────┘
```

### Adapter Modes

#### 1. Native Adapter Mode (Primary)
Direct integration with Perl's built-in debugger:

```rust
// Launch Perl with debugger attached
Command::new("perl")
    .arg("-d")
    .arg("-e")
    .arg("0")  // Start in debug mode
    .spawn()?
```

**Features**:
- Launch and attach support
- Breakpoint management
- Step execution (step over, step into, step out)
- Variable evaluation
- Call stack inspection

#### 2. Bridge Adapter Mode (Legacy Compatibility)
Translation layer to Perl::LanguageServer:

```rust
// Bridge to Perl::LanguageServer
let bridge = BridgeAdapter::new()
    .perl_ls_path(perl_ls_path)
    .log_level(log_level)
    .build()?;
```

**Use Cases**:
- Compatibility with existing Perl::LanguageServer setups
- Features not yet implemented in native adapter
- Transition path for existing users

### Protocol Translation

| DAP Request | Native Mode | Bridge Mode |
|-------------|-------------|-------------|
| `initialize` | Native handling | Forward to Perl::LS |
| `launch` | Spawn `perl -d` | Forward to Perl::LS |
| `attach` | Attach to PID | Forward to Perl::LS |
| `setBreakpoints` | Native handling | Forward to Perl::LS |
| `configurationDone` | Resume execution | Forward to Perl::LS |
| `threads` | Query debugger | Forward to Perl::LS |
| `stackTrace` | Native handling | Forward to Perl::LS |
| `scopes` | Native handling | Forward to Perl::LS |
| `variables` | Native handling | Forward to Perl::LS |
| `evaluate` | Native handling | Forward to Perl::LS |
| `next` | Debugger command `n` | Forward to Perl::LS |
| `stepIn` | Debugger command `s` | Forward to Perl::LS |
| `stepOut` | Debugger command `r` | Forward to Perl::LS |
| `continue` | Debugger command `c` | Forward to Perl::LS |
| `disconnect` | Cleanup session | Forward to Perl::LS |

### Session Management

```rust
pub struct DebugSession {
    /// Unique session identifier
    id: Uuid,
    /// Adapter mode (native or bridge)
    mode: AdapterMode,
    /// Debug target process
    target: Option<Child>,
    /// Breakpoint state
    breakpoints: HashMap<PathBuf, Vec<Breakpoint>>,
    /// Communication channels
    events_tx: mpsc::Sender<DapEvent>,
}
```

## Alternatives Considered

### Option 1: Pure Native Implementation
**Description**: Implement complete Perl debugger from scratch in Rust

**Pros**:
- Full control over debugging behavior
- No dependency on Perl modules
- Potentially better performance

**Cons**:
- Massive development effort
- Years of edge case handling to replicate
- Complex Perl internals knowledge required

**Decision**: Rejected - not feasible with current resources

### Option 2: Pure Bridge Mode
**Description**: Always delegate to Perl::LanguageServer

**Pros**:
- Leverages existing mature implementation
- Minimal Rust code needed
- Fast time to market

**Cons**:
- Dependency on external Perl module
- Limited control over debugging behavior
- Protocol version coupling

**Decision**: Rejected - limits flexibility and adds external dependency

### Option 3: Fork Perl::LanguageServer
**Description**: Fork and embed Perl::LanguageServer code

**Pros**:
- Full control over codebase
- Can modify as needed

**Cons**:
- Maintenance burden of Perl code
- Divergence from upstream
- Mixed language codebase

**Decision**: Rejected - prefer clean protocol boundary

## Consequences

### Positive

1. **Leveraged Investment**:
   - Uses mature Perl debugging implementation
   - Years of bug fixes and edge case handling
   - Community-tested debugging scenarios

2. **Protocol Compliance**:
   - Standard DAP protocol to editors
   - Compatible with VS Code, Neovim, etc.
   - Future-proof editor support

3. **Flexibility**:
   - Native mode for core features
   - Bridge mode for compatibility
   - Can migrate features over time

4. **Reduced Risk**:
   - Lower implementation risk
   - Proven debugging backend
   - Easier testing against real Perl code

### Negative

1. **Protocol Translation Complexity**:
   - Must maintain translation layer
   - DAP version compatibility
   - Edge cases in protocol mapping

2. **External Dependency**:
   - Bridge mode requires Perl::LanguageServer
   - Version compatibility concerns
   - Installation complexity for users

3. **Feature Lag**:
   - New DAP features require bridge updates
   - Dependent on Perl::LanguageServer updates
   - Potential feature gaps

4. **Debugging Complexity**:
   - Issues may span Rust and Perl code
   - Harder to diagnose problems
   - Two codebases to understand

### Mitigations

1. **Native Mode Priority**:
   - Core features in native adapter
   - Reduce bridge mode dependency over time
   - Clear migration path

2. **Version Pinning**:
   - Document compatible Perl::LS versions
   - Version checks at startup
   - Graceful degradation

3. **Comprehensive Logging**:
   - Protocol message logging
   - Translation step tracing
   - Error context preservation

4. **Testing Strategy**:
   - Integration tests with real Perl code
   - Protocol conformance tests
   - Bridge mode compatibility tests

## Security Considerations

See [DAP Security Specification](../DAP_SECURITY_SPECIFICATION.md) for detailed security requirements.

Key points:
- Path traversal prevention in file paths
- Command injection prevention in evaluate requests
- Process isolation for debug targets
- Secure handling of environment variables

## Configuration

### Native Mode Launch

```json
{
  "type": "perl",
  "request": "launch",
  "name": "Launch Perl Script",
  "program": "${file}",
  "adapterMode": "native"
}
```

### Bridge Mode Launch

```json
{
  "type": "perl",
  "request": "launch",
  "name": "Launch Perl Script (Bridge)",
  "program": "${file}",
  "adapterMode": "bridge",
  "perlLanguageServerPath": "/usr/local/bin/perl-language-server"
}
```

## References

- [Debug Adapter Protocol Specification](https://microsoft.github.io/debug-adapter-protocol/specification)
- [Perl::LanguageServer on CPAN](https://metacpan.org/pod/Perl::LanguageServer)
- [DAP User Guide](../tutorials/DAP_USER_GUIDE.md)
- [DAP Security Specification](../DAP_SECURITY_SPECIFICATION.md)
