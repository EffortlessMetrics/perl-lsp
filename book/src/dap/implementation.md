# DAP Implementation Specification
<!-- Labels: spec:dap, implementation:greenfield, phase:bridge-to-native -->

**Issue**: #207 - Debug Adapter Protocol Support
**Status**: Specification Complete
**Version**: 0.9.x (Production-Ready)
**Date**: 2025-10-04

---

## Executive Summary

This specification defines the comprehensive implementation strategy for adding Debug Adapter Protocol (DAP) support to the Perl LSP ecosystem. The implementation follows a **phased Bridge-to-Native approach** delivering immediate user value while building production-grade infrastructure.

**Implementation Strategy**: Greenfield implementation leveraging existing Perl LSP infrastructure (AST integration, incremental parsing, workspace navigation, security framework).

**Key Deliverables**:
- Phase 1 (Week 1-2): Bridge to Perl::LanguageServer for immediate debugging capability
- Phase 2 (Week 3-6): Native Rust adapter (perl-dap) + CPAN Perl shim (Devel::TSPerlDAP)
- Phase 3 (Week 7-8): Production hardening with comprehensive testing and security validation

**Performance Targets**:
- Breakpoint operations: <50ms latency
- Step/continue: <100ms p95 response time
- Variable expansion: <200ms initial + <100ms per child
- Incremental breakpoint updates: <1ms (leveraging existing parser)

**Success Metrics**:
- 19/19 acceptance criteria validated
- >95% test coverage for DAP adapter
- >80% test coverage for Perl shim
- Zero security findings
- 100% LSP test pass rate with DAP active

---

## 1. Architecture Overview

### 1.1 System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    VS Code Extension                        │
│  - contributes.debuggers configuration                      │
│  - Launch.json snippet management                           │
│  - Platform binary distribution (6 targets)                 │
└───────────────────────────┬─────────────────────────────────┘
                            │ DAP over stdio (JSON-RPC 2.0)
                            ↓
┌─────────────────────────────────────────────────────────────┐
│              perl-dap Rust Adapter (NEW CRATE)              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Protocol Layer                                        │  │
│  │  - JSON-RPC DAP server (tokio async)                 │  │
│  │  - Request routing: initialize, launch, attach, etc. │  │
│  │  - Session state management (Arc<Mutex<>>)           │  │
│  └───────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Integration Layer                                     │  │
│  │  - Breakpoint Manager (AST-based validation)         │  │
│  │  - Position Mapper (UTF-16 ↔ UTF-8 conversion)      │  │
│  │  - Workspace Navigator (dual indexing strategy)      │  │
│  │  - Security Manager (path validation, safe eval)     │  │
│  └───────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Shim Communication Layer                              │  │
│  │  - JSON protocol over stdio/TCP                      │  │
│  │  - Process lifecycle management                      │  │
│  │  - Error handling and recovery                       │  │
│  └───────────────────────────────────────────────────────┘  │
└───────────────────────────┬─────────────────────────────────┘
                            │ JSON over stdio/TCP
                            ↓
┌─────────────────────────────────────────────────────────────┐
│         Devel::TSPerlDAP Perl Shim (NEW CPAN MODULE)        │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Protocol Server                                       │  │
│  │  - JSON-RPC server (stdio or TCP configurable)       │  │
│  │  - Command routing: set_breakpoints, continue, etc.  │  │
│  └───────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Debugger Integration                                  │  │
│  │  - Breakpoint management ($DB::single hooks)         │  │
│  │  - Stack introspection (caller() + %DB::sub)         │  │
│  │  - Variable inspection (PadWalker for lexicals)      │  │
│  │  - Code evaluation (safe eval wrapper)               │  │
│  └───────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Rendering Layer                                       │  │
│  │  - Variable serialization (B::Deparse for coderefs)  │  │
│  │  - Lazy expansion (large arrays/hashes)              │  │
│  │  - Truncation (1KB preview max)                      │  │
│  └───────────────────────────────────────────────────────┘  │
└───────────────────────────┬─────────────────────────────────┘
                            │ Perl Debugger API
                            ↓
┌─────────────────────────────────────────────────────────────┐
│              perl -d (Perl Debugger Runtime)                │
│  - $DB::single, $DB::trace, $DB::sub hooks                  │
│  - caller() stack introspection                             │
│  - Package symbol table access (%::)                        │
│  - PadWalker lexical inspection                             │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 Component Responsibilities

#### perl-dap Rust Adapter
**Location**: `/crates/perl-dap/`
**Purpose**: Production-grade DAP protocol implementation with LSP infrastructure integration

**Responsibilities**:
- DAP 1.x protocol compliance (JSON-RPC 2.0)
- AST-based breakpoint validation (reuse perl-parser)
- Position mapping (UTF-16 ↔ UTF-8 symmetric conversion)
- Workspace navigation (dual indexing for stack frames)
- Security enforcement (path traversal, safe eval, timeouts)
- Shim lifecycle management (spawn, monitor, restart)
- Performance optimization (<50ms breakpoints, <100ms step/continue)

#### Devel::TSPerlDAP Perl Shim
**Location**: CPAN module (bundled fallback in extension)
**Purpose**: Machine-readable JSON bridge to Perl debugger runtime

**Responsibilities**:
- Debugger integration ($DB::single, caller(), %DB::sub)
- Variable inspection (PadWalker for lexicals, symbol table for package vars)
- Code evaluation (safe eval wrapper with timeout enforcement)
- Variable rendering (B::Deparse for coderefs, lazy expansion)
- JSON protocol server (stdio or TCP configurable)
- Cross-platform compatibility (Perl 5.16+ required, 5.30+ recommended)

#### VS Code Extension Integration
**Location**: `/vscode-extension/`
**Purpose**: DAP debugger contribution and platform binary management

**Responsibilities**:
- `contributes.debuggers` configuration (types: "perl", "perl-rs")
- Launch.json snippet generation (launch and attach configurations)
- Platform binary selection (6 targets: Linux/macOS/Windows x86_64/aarch64)
- Auto-download fallback (GitHub Releases integration)
- First-time setup (<30 seconds target for shim installation)

---

## 2. DAP Protocol Specification

### 2.1 Protocol Overview

**Standard**: Debug Adapter Protocol 1.x
**Transport**: JSON-RPC 2.0 over stdio (Content-Length headers)
**Message Types**: Request, Response, Event

**Core Request Types Implemented**:
- `initialize`: Capability negotiation
- `launch`: Start debugging session (program execution)
- `attach`: Attach to running Perl process
- `setBreakpoints`: Manage source breakpoints
- `continue`: Resume execution until next breakpoint
- `next`: Step over (execute next line)
- `stepIn`: Step into subroutine
- `stepOut`: Step out of subroutine
- `pause`: Interrupt execution
- `threads`: List debugger threads (single "Main Thread" for Perl)
- `stackTrace`: Retrieve call stack
- `scopes`: Get variable scopes (Locals, Package, Globals)
- `variables`: Retrieve variable values with lazy expansion
- `evaluate`: Evaluate expression in stack frame context
- `disconnect`: Terminate debugging session

### 2.2 Request/Response Sequencing

#### 2.2.1 Initialization Sequence

```json
// Client → Adapter
{
  "seq": 1,
  "type": "request",
  "command": "initialize",
  "arguments": {
    "clientID": "vscode",
    "adapterID": "perl-rs",
    "linesStartAt1": true,
    "columnsStartAt1": true,
    "pathFormat": "path",
    "supportsVariableType": true,
    "supportsVariablePaging": false
  }
}

// Adapter → Client
{
  "seq": 1,
  "type": "response",
  "request_seq": 1,
  "success": true,
  "command": "initialize",
  "body": {
    "supportsConfigurationDoneRequest": true,
    "supportsEvaluateForHovers": true,
    "supportsStepInTargetsRequest": false,
    "supportsSetVariable": false,
    "supportsConditionalBreakpoints": false,  // Phase 2 enhancement
    "supportsExceptionBreakpoints": false     // Post-MVP
  }
}

// Adapter → Client (Event)
{
  "seq": 2,
  "type": "event",
  "event": "initialized"
}
```

#### 2.2.2 Launch Sequence

```json
// Client → Adapter
{
  "seq": 3,
  "type": "request",
  "command": "launch",
  "arguments": {
    "program": "/workspace/script.pl",
    "args": ["--verbose", "input.txt"],
    "perlPath": "/usr/bin/perl",
    "includePaths": ["/workspace/lib"],
    "env": { "PERL5LIB": "/custom/lib" },
    "cwd": "/workspace",
    "stopOnEntry": false
  }
}

// Adapter → Client
{
  "seq": 3,
  "type": "response",
  "request_seq": 3,
  "success": true,
  "command": "launch"
}

// Adapter spawns: perl -d:TSPerlDAP script.pl --verbose input.txt
// With environment: PERL5LIB=/workspace/lib:/custom/lib
```

#### 2.2.3 Breakpoint Management Sequence

```json
// Client → Adapter
{
  "seq": 4,
  "type": "request",
  "command": "setBreakpoints",
  "arguments": {
    "source": {
      "path": "/workspace/lib/Module.pm",
      "name": "Module.pm"
    },
    "breakpoints": [
      { "line": 10 },
      { "line": 25 },
      { "line": 100 }
    ]
  }
}

// Adapter validates breakpoints using AST (<50ms target)
// Adapter → Client
{
  "seq": 4,
  "type": "response",
  "request_seq": 4,
  "success": true,
  "command": "setBreakpoints",
  "body": {
    "breakpoints": [
      { "id": 1, "verified": true, "line": 10 },
      { "id": 2, "verified": true, "line": 25 },
      { "id": 3, "verified": false, "line": 100, "message": "Line contains only comments" }
    ]
  }
}
```

#### 2.2.4 Execution Control Sequence

```json
// Client → Adapter
{
  "seq": 5,
  "type": "request",
  "command": "continue",
  "arguments": {
    "threadId": 1
  }
}

// Adapter → Client (Response <100ms p95)
{
  "seq": 5,
  "type": "response",
  "request_seq": 5,
  "success": true,
  "command": "continue",
  "body": {
    "allThreadsContinued": true
  }
}

// Adapter → Client (Event when breakpoint hit)
{
  "seq": 6,
  "type": "event",
  "event": "stopped",
  "body": {
    "reason": "breakpoint",
    "threadId": 1,
    "preserveFocusHint": false,
    "allThreadsStopped": true
  }
}
```

#### 2.2.5 Variable Inspection Sequence

```json
// Client → Adapter: Get stack trace
{
  "seq": 7,
  "type": "request",
  "command": "stackTrace",
  "arguments": {
    "threadId": 1,
    "startFrame": 0,
    "levels": 20
  }
}

// Adapter → Client
{
  "seq": 7,
  "type": "response",
  "request_seq": 7,
  "success": true,
  "command": "stackTrace",
  "body": {
    "stackFrames": [
      {
        "id": 1001,
        "name": "Package::subroutine",
        "source": { "path": "/workspace/lib/Package.pm", "name": "Package.pm" },
        "line": 42,
        "column": 0
      },
      {
        "id": 1002,
        "name": "main::run",
        "source": { "path": "/workspace/script.pl", "name": "script.pl" },
        "line": 10,
        "column": 0
      }
    ],
    "totalFrames": 2
  }
}

// Client → Adapter: Get scopes for frame
{
  "seq": 8,
  "type": "request",
  "command": "scopes",
  "arguments": {
    "frameId": 1001
  }
}

// Adapter → Client
{
  "seq": 8,
  "type": "response",
  "request_seq": 8,
  "success": true,
  "command": "scopes",
  "body": {
    "scopes": [
      {
        "name": "Locals",
        "variablesReference": 2001,
        "expensive": false
      },
      {
        "name": "Package",
        "variablesReference": 2002,
        "expensive": false
      }
    ]
  }
}

// Client → Adapter: Get variables
{
  "seq": 9,
  "type": "request",
  "command": "variables",
  "arguments": {
    "variablesReference": 2001,
    "start": 0,
    "count": 100
  }
}

// Adapter → Client (<200ms initial expansion)
{
  "seq": 9,
  "type": "response",
  "request_seq": 9,
  "success": true,
  "command": "variables",
  "body": {
    "variables": [
      {
        "name": "$x",
        "value": "42",
        "type": "scalar",
        "variablesReference": 0
      },
      {
        "name": "@array",
        "value": "[10 items]",
        "type": "array",
        "variablesReference": 3001
      },
      {
        "name": "%hash",
        "value": "{5 keys}",
        "type": "hash",
        "variablesReference": 3002
      }
    ]
  }
}
```

### 2.3 Error Handling

#### 2.3.1 Structured Error Responses

```json
// Breakpoint validation failure
{
  "seq": 10,
  "type": "response",
  "request_seq": 10,
  "success": false,
  "command": "setBreakpoints",
  "message": "Path traversal detected in breakpoint path",
  "body": {
    "error": {
      "id": 1001,
      "format": "Security violation: attempted path traversal in '{path}'",
      "variables": {
        "path": "/workspace/../../../etc/passwd"
      },
      "showUser": true
    }
  }
}

// Evaluation timeout
{
  "seq": 11,
  "type": "response",
  "request_seq": 11,
  "success": false,
  "command": "evaluate",
  "message": "Evaluation timed out after 5 seconds",
  "body": {
    "error": {
      "id": 1002,
      "format": "Expression evaluation exceeded {timeout}s timeout",
      "variables": {
        "timeout": "5"
      },
      "showUser": true
    }
  }
}
```

#### 2.3.2 Event Sequencing for Errors

```json
// Output event for stderr
{
  "seq": 12,
  "type": "event",
  "event": "output",
  "body": {
    "category": "stderr",
    "output": "Global symbol \"$undefined\" requires explicit package name at script.pl line 10.\n",
    "source": { "path": "/workspace/script.pl" },
    "line": 10,
    "column": 0
  }
}

// Terminated event on fatal error
{
  "seq": 13,
  "type": "event",
  "event": "terminated"
}
```

---

## 3. LSP Integration Points

### 3.1 AST-Based Breakpoint Validation

**Requirement**: Leverage existing ~100% Perl syntax coverage for accurate breakpoint placement

**Integration Pattern**:
```rust
// crates/perl-dap/src/breakpoints.rs
use perl_parser::{Parser, ast::Node};
use ropey::Rope;

pub struct BreakpointManager {
    parser: Arc<Parser>,
    workspace: Arc<WorkspaceIndex>,
}

impl BreakpointManager {
    pub fn verify_breakpoint(&self, uri: &str, line: u32, rope: &Rope) -> BreakpointVerification {
        // Parse source text (using existing parser infrastructure)
        // NOTE: Parser::parse() returns AST Node, not file-based parsing
        let source = rope.to_string();
        let mut parser = Parser::new(&source);
        let ast = parser.parse()?;

        // Get byte offset for the line (using Rope)
        let line_start = rope.line_to_byte(line as usize);
        let line_end = if (line as usize) < rope.len_lines() - 1 {
            rope.line_to_byte(line as usize + 1)
        } else {
            rope.len_bytes()
        };

        // Validate line contains executable code using AST traversal
        // These helper functions will be implemented in perl-dap crate (see DAP_BREAKPOINT_VALIDATION_GUIDE.md)
        if is_comment_or_blank_line(&ast, line_start, line_end, &source) {
            return BreakpointVerification::Invalid {
                reason: "Line contains only comments or whitespace"
            };
        }

        // Validate not inside string literal or heredoc using AST node type analysis
        if is_inside_string_literal(&ast, line_start) {
            return BreakpointVerification::Invalid {
                reason: "Line is inside string literal or heredoc"
            };
        }

        // Validate not inside POD documentation using AST traversal
        if is_inside_pod(&source, line_start) {
            return BreakpointVerification::Invalid {
                reason: "Line is inside POD documentation"
            };
        }

        BreakpointVerification::Verified {
            line: self.adjust_to_executable_line(&ast, line, rope)
        }
    }

    // Adjust breakpoint to nearest executable line
    fn adjust_to_executable_line(&self, ast: &Node, line: u32, rope: &Rope) -> u32 {
        // Search forward for next executable line (max 5 lines)
        for offset in 0..5 {
            let candidate = line + offset;
            if (candidate as usize) >= rope.len_lines() {
                break;
            }

            let line_start = rope.line_to_byte(candidate as usize);
            let line_end = rope.line_to_byte(candidate as usize + 1);

            if is_executable_line(ast, line_start, line_end) {
                return candidate;
            }
        }

        line // Fallback to original line
    }
}

// DAP-specific AST validation utilities (crates/perl-dap/src/breakpoints/ast_utils.rs)
// These functions analyze the AST Node structure from perl_parser::ast::Node

fn is_comment_or_blank_line(ast: &Node, line_start: usize, line_end: usize, source: &str) -> bool {
    // Extract line text
    let line_text = &source[line_start..line_end.min(source.len())];

    // Check if blank (only whitespace)
    if line_text.trim().is_empty() {
        return true;
    }

    // Check if comment (starts with # after whitespace)
    if line_text.trim_start().starts_with('#') {
        return true;
    }

    // TODO AC7: Implement AST-based comment detection by traversing nodes
    // and checking NodeKind for comment types
    false
}

fn is_inside_string_literal(ast: &Node, byte_offset: usize) -> bool {
    // TODO AC7: Traverse AST to find node containing byte_offset
    // Check if NodeKind is StringLiteral or Heredoc
    // This will be implemented using perl_parser::ast::Node traversal
    false
}

fn is_inside_pod(source: &str, byte_offset: usize) -> bool {
    // POD detection via text scanning (POD markers: =pod, =head1, etc.)
    // Scan backwards from byte_offset to detect POD block markers
    let before = &source[..byte_offset];
    let after = &source[byte_offset..];

    // Check if we're between =pod and =cut markers
    let in_pod = before.rfind("=pod").is_some() &&
                 after.find("=cut").is_some() &&
                 before.rfind("=cut").map_or(true, |cut| before.rfind("=pod").unwrap() > cut);

    in_pod
}

fn is_executable_line(ast: &Node, line_start: usize, line_end: usize) -> bool {
    // TODO AC7: Check if line range contains executable AST nodes
    // (statements, expressions, not just comments/POD/whitespace)
    true // Conservative default - will be refined in implementation
}
```

**Performance Target**: <50ms breakpoint verification leveraging Rope and AST analysis

**Implementation Notes**:
- AST validation utilities will be implemented in `perl-dap` crate (not in `perl-parser`)
- See `docs/DAP_BREAKPOINT_VALIDATION_GUIDE.md` for detailed AST traversal patterns
- Uses existing `Parser::parse()` API which returns `ast::Node` structure
- Rope-based line-to-byte conversion for efficient position mapping

### 3.2 Incremental Parsing Integration

**Requirement**: Live breakpoint adjustment as code changes without full re-parse

**Integration Pattern**:
```rust
// crates/perl-dap/src/session.rs
use perl_parser::incremental_v2::IncrementalParserV2;

pub struct DapSession {
    parser: IncrementalParserV2,
    breakpoints: HashMap<String, Vec<Breakpoint>>,
    active_session: bool,
}

impl DapSession {
    pub fn on_text_change(&mut self, uri: &str, changes: Vec<TextEdit>) -> Result<()> {
        // Apply incremental parsing (<1ms target)
        self.parser.apply_edits(uri, &changes)?;

        // Calculate affected line range
        let affected_lines = self.calculate_affected_lines(&changes);

        // Re-verify breakpoints in affected range
        let breakpoints = self.breakpoints.get(uri).cloned().unwrap_or_default();
        for bp in breakpoints {
            if affected_lines.contains(&bp.line) {
                let verification = self.breakpoint_manager.verify_breakpoint(uri, bp.line)?;

                // Send breakpoint event if verification status changed
                if verification != bp.verification {
                    self.send_breakpoint_event(bp.id, verification)?;
                }
            }
        }

        Ok(())
    }

    fn calculate_affected_lines(&self, changes: &[TextEdit]) -> Range<u32> {
        let mut min_line = u32::MAX;
        let mut max_line = 0;

        for change in changes {
            min_line = min_line.min(change.range.start.line);
            max_line = max_line.max(change.range.end.line);
        }

        // Add buffer for multi-line statements
        min_line.saturating_sub(5)..max_line.saturating_add(5)
    }
}
```

**Performance Target**: <1ms incremental breakpoint updates

### 3.3 Workspace Navigation for Stack Frames

**Requirement**: Dual indexing strategy for accurate stack frame source resolution

**Integration Pattern**:
```rust
// crates/perl-dap/src/stack.rs
use perl_parser::workspace_index::WorkspaceIndex;

pub struct StackTraceProvider {
    workspace: Arc<WorkspaceIndex>,
}

impl StackTraceProvider {
    pub fn resolve_frame_location(
        &self,
        package: &str,
        subroutine: &str
    ) -> Option<Location> {
        // Use dual pattern matching (98% reference coverage)
        let qualified = format!("{}::{}", package, subroutine);

        // Search exact qualified match first
        if let Some(def) = self.workspace.get_definition(&qualified) {
            return Some(def.location);
        }

        // Fallback to bare name search (dual indexing)
        if let Some(def) = self.workspace.get_definition(subroutine) {
            return Some(def.location);
        }

        // Fallback to text search across workspace
        self.workspace.text_search(&qualified)
            .or_else(|| self.workspace.text_search(subroutine))
    }

    pub fn build_stack_frames(&self, perl_stack: Vec<PerlFrame>) -> Vec<DapStackFrame> {
        perl_stack.iter().enumerate().map(|(idx, frame)| {
            let location = self.resolve_frame_location(&frame.package, &frame.subroutine);

            DapStackFrame {
                id: 1000 + idx as i64,
                name: format!("{}::{}", frame.package, frame.subroutine),
                source: location.as_ref().map(|loc| DapSource {
                    path: loc.uri.to_file_path().ok(),
                    name: loc.uri.path().split('/').last().map(String::from),
                }),
                line: location.as_ref().map(|loc| loc.range.start.line).unwrap_or(0),
                column: location.as_ref().map(|loc| loc.range.start.character).unwrap_or(0),
                presentationHint: Some("normal"),
            }
        }).collect()
    }
}
```

**Coverage Target**: 98% stack frame resolution success rate via dual indexing

### 3.4 UTF-16 Position Mapping

**Requirement**: Symmetric position conversion for accurate breakpoint placement

**Integration Pattern**:
```rust
// crates/perl-dap/src/protocol.rs
use perl_lsp::textdoc::{lsp_pos_to_byte, byte_to_lsp_pos, PosEnc};

pub fn dap_breakpoint_to_byte_offset(
    rope: &Rope,
    line: u32,
    column: u32,
) -> Result<usize> {
    // DAP uses 0-based line/column (same as LSP)
    let pos = Position { line, character: column };

    // Reuse LSP infrastructure (PR #153 symmetric conversion)
    lsp_pos_to_byte(rope, pos, PosEnc::Utf16)
}

pub fn byte_offset_to_dap_position(
    rope: &Rope,
    byte_offset: usize,
) -> Result<(u32, u32)> {
    // Convert byte offset to LSP position
    let pos = byte_to_lsp_pos(rope, byte_offset, PosEnc::Utf16)?;

    Ok((pos.line, pos.character))
}

// Variable rendering with UTF-16 safety
pub fn render_variable_value(value: &str, rope: &Rope) -> String {
    // Truncate large values (security: prevent memory exhaustion)
    if value.len() > 1024 {
        let truncated = &value[..1024];

        // UTF-16 safe truncation (reuse PR #153 infrastructure)
        let safe_truncate = ensure_utf16_boundary(truncated, rope);
        format!("{}…", safe_truncate)
    } else {
        value.to_string()
    }
}
```

**Security Requirement**: Symmetric conversion prevents UTF-16 boundary vulnerabilities

---

## 4. Phased Implementation Plan

### 4.1 Phase 1: Bridge Implementation (Week 1-2)

**Goal**: Deliver immediate debugging capability for users

**Deliverables**: AC1-AC4
- AC1: VS Code extension contributes "perl" debugger type
- AC2: Launch.json snippets (launch and attach configurations)
- AC3: Bridge setup documentation
- AC4: Basic debugging workflow (set breakpoints, step, inspect variables)

**Implementation Tasks**:

#### Task 1.1: VS Code Extension Configuration (AC1)
**Duration**: 0.5 days
**Files Modified**:
- `vscode-extension/package.json`: Add `contributes.debuggers` section
- `vscode-extension/src/debugAdapter.ts`: Implement bridge proxy to Perl::LanguageServer

**Code Template**:
```json
{
  "contributes": {
    "debuggers": [
      {
        "type": "perl",
        "label": "Perl Debug (Bridge)",
        "program": "./out/debugAdapter.js",
        "runtime": "node",
        "configurationAttributes": {
          "launch": {
            "required": ["program"],
            "properties": {
              "program": {
                "type": "string",
                "description": "Absolute path to Perl script"
              },
              "args": {
                "type": "array",
                "description": "Command line arguments",
                "default": []
              },
              "perlPath": {
                "type": "string",
                "description": "Path to Perl executable",
                "default": "perl"
              },
              "includePaths": {
                "type": "array",
                "description": "Additional @INC paths",
                "default": []
              }
            }
          }
        }
      }
    ]
  }
}
```

**Test Validation**: `cd vscode-extension && npm test`

#### Task 1.2: Launch.json Snippets (AC2)
**Duration**: 0.5 days
**Files Created**: `vscode-extension/snippets/launch.json`

**Test Validation**: `cargo test --test dap_launch_snippets -- windows macos linux`

#### Task 1.3: Bridge Documentation (AC3)
**Duration**: 0.5 days
**Files Created**: `docs/DAP_BRIDGE_SETUP_GUIDE.md`

**Content Requirements**:
- Perl::LanguageServer installation instructions
- Configuration examples
- Platform-specific troubleshooting (Windows UNC paths, macOS symlinks, WSL)

#### Task 1.4: Basic Workflow Validation (AC4)
**Duration**: 0.5 days
**Files Created**: `crates/perl-dap/tests/fixtures/bridge_basic.pl`

**Test Suite**: Golden transcript validation for bridge protocol

**Success Criteria**: All Phase 1 ACs passing, users can debug Perl code via bridge

---

### 4.2 Phase 2: Native Infrastructure (Week 3-6)

**Goal**: Production-grade DAP adapter owned by perl-lsp

**Deliverables**: AC5-AC12
- AC5: perl-dap Rust crate scaffolding
- AC6: Devel::TSPerlDAP CPAN module
- AC7: Breakpoint management with AST validation
- AC8: Stack traces, scopes, variables with lazy expansion
- AC9: Stepping and control flow (<100ms p95)
- AC10: Evaluate and REPL with safe eval
- AC11: VS Code native integration
- AC12: Cross-platform compatibility (6 platforms)

**Critical Path**: AC6 (Perl shim) requires 2 weeks

#### Task 2.1: perl-dap Crate Scaffolding (AC5)
**Duration**: 1 week
**Files Created**:
- `crates/perl-dap/Cargo.toml`: Dependencies (tokio, serde_json, anyhow, tracing)
- `crates/perl-dap/src/main.rs`: Adapter entry point
- `crates/perl-dap/src/protocol.rs`: DAP message types
- `crates/perl-dap/src/session.rs`: Session state management

**Code Template**:
```rust
// crates/perl-dap/src/protocol.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DapRequest {
    pub seq: i64,
    #[serde(rename = "type")]
    pub type_: String,
    pub command: String,
    pub arguments: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DapResponse {
    pub seq: i64,
    #[serde(rename = "type")]
    pub type_: String,
    pub request_seq: i64,
    pub success: bool,
    pub command: String,
    pub message: Option<String>,
    pub body: Option<serde_json::Value>,
}

pub struct DapServer {
    seq: AtomicI64,
    session: Arc<Mutex<Option<DapSession>>>,
}

impl DapServer {
    pub async fn handle_request(&self, request: DapRequest) -> DapResponse {
        match request.command.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "launch" => self.handle_launch(request).await,
            "setBreakpoints" => self.handle_set_breakpoints(request).await,
            "continue" => self.handle_continue(request).await,
            // ... other commands
            _ => self.handle_unknown_command(request),
        }
    }
}
```

**Test Validation**: `cargo test -p perl-dap --test protocol_compliance`

#### Task 2.2: Devel::TSPerlDAP CPAN Module (AC6) - **CRITICAL PATH**
**Duration**: 2 weeks
**Files Created**:
- `Devel-TSPerlDAP/lib/Devel/TSPerlDAP.pm`: Core shim implementation
- `Devel-TSPerlDAP/t/*.t`: Test suite (>80% coverage)
- `Devel-TSPerlDAP/META.json`: CPAN metadata

**Perl Shim Architecture**:
```perl
# Devel/TSPerlDAP.pm
package Devel::TSPerlDAP;
use strict;
use warnings;
use JSON::PP;
use PadWalker qw(peek_my);
use B::Deparse;

our $VERSION = '0.1.0';

sub import {
    my ($class, %opts) = @_;

    my $daemon = $opts{daemon} // 0;

    if ($daemon) {
        start_tcp_server($opts{host}, $opts{port});
    } else {
        start_stdio_server();
    }
}

sub handle_command {
    my ($request) = @_;

    my $command = $request->{command};

    return set_breakpoints($request->{arguments})   if $command eq 'set_breakpoints';
    return continue_execution()                     if $command eq 'continue';
    return step_next()                              if $command eq 'next';
    return step_in()                                if $command eq 'step_in';
    return step_out()                               if $command eq 'step_out';
    return get_stack_trace()                        if $command eq 'stack';
    return get_scopes($request->{arguments})        if $command eq 'scopes';
    return get_variables($request->{arguments})     if $command eq 'variables';
    return evaluate_expression($request->{arguments}) if $command eq 'evaluate';

    return { success => 0, message => "Unknown command: $command" };
}

sub set_breakpoints {
    my ($args) = @_;
    my $file = $args->{source}{path};
    my @breakpoints = @{$args->{breakpoints}};

    foreach my $bp (@breakpoints) {
        my $line = $bp->{line};
        $DB::single{$file}{$line} = 1;
    }

    return { success => 1, breakpoints => \@breakpoints };
}

sub get_stack_trace {
    my @frames;
    my $i = 0;

    while (my ($package, $file, $line, $sub) = caller($i++)) {
        push @frames, {
            name => $sub,
            source => { path => $file },
            line => $line,
            column => 0,
        };
    }

    return { stackFrames => \@frames };
}

sub get_variables {
    my ($args) = @_;
    my $frame_id = $args->{frameId};

    # Use PadWalker to inspect lexical variables
    my $vars = peek_my($frame_id);

    my @variables;
    foreach my $name (sort keys %$vars) {
        my $value = $vars->{$name};

        push @variables, {
            name => $name,
            value => render_value($value),
            type => ref($value) || 'scalar',
            variablesReference => is_expandable($value) ? allocate_ref($value) : 0,
        };
    }

    return { variables => \@variables };
}

sub render_value {
    my ($value) = @_;

    if (ref($value) eq 'CODE') {
        my $deparse = B::Deparse->new();
        return $deparse->coderef2text($value);
    } elsif (ref($value) eq 'ARRAY') {
        return "[" . @$value . " items]";
    } elsif (ref($value) eq 'HASH') {
        return "{" . (scalar keys %$value) . " keys}";
    } else {
        my $str = "$value";
        return length($str) > 1024 ? substr($str, 0, 1024) . "…" : $str;
    }
}

1;
```

**Test Validation**: `cd Devel-TSPerlDAP && prove -lv t/`

**CPAN Publication Requirements**:
- >80% test coverage
- Perl 5.16+ compatibility
- Dependencies: JSON::PP, PadWalker, B::Deparse
- Documentation: POD for all public functions

#### Task 2.3: Breakpoint Management (AC7)
**Duration**: 0.5 weeks
**Integration**: AST-based validation + path mapping

**Test Validation**: `cargo test -p perl-dap --test breakpoint_validation`

#### Task 2.4: Stack/Scopes/Variables (AC8)
**Duration**: 1 week
**Features**: Lazy expansion, variable rendering, Unicode safety

**Test Validation**: `cargo test -p perl-dap --test variable_rendering`

#### Task 2.5: Stepping and Control Flow (AC9)
**Duration**: 0.5 weeks
**Performance Target**: <100ms p95 latency

**Test Validation**: `cargo test -p perl-dap --test control_flow_performance`

#### Task 2.6: Evaluate and REPL (AC10)
**Duration**: 0.5 weeks
**Security**: Safe eval default, timeout enforcement

**Test Validation**: `cargo test -p perl-dap --test eval_security`

#### Task 2.7: VS Code Native Integration (AC11)
**Duration**: 0.5 weeks
**Deliverable**: "perl-rs" debugger type with native adapter

**Test Validation**: `cd vscode-extension && npm test -- native`

#### Task 2.8: Cross-Platform Compatibility (AC12)
**Duration**: 0.5 weeks
**Platforms**: Linux/macOS/Windows x86_64/aarch64

**Test Validation**: `cargo test -p perl-dap --test cross_platform_validation`

**Success Criteria**: All Phase 2 ACs passing, native adapter functional

---

### 4.3 Phase 3: Production Hardening (Week 7-8)

**Goal**: Enterprise-ready debugging with comprehensive testing

**Deliverables**: AC13-AC19
- AC13: Comprehensive integration tests (golden transcripts, breakpoint matrix, variable rendering)
- AC14: Performance benchmarks with regression detection
- AC15: Documentation complete (Tutorial, Reference, Architecture, Troubleshooting)
- AC16: Security validation (path traversal, safe eval, timeout, Unicode safety)
- AC17: LSP integration non-regression (100% LSP test pass rate)
- AC18: Dependency management (auto-install, bundled fallback, versioning)
- AC19: Binary packaging (6 platforms, GitHub Releases, auto-download)

#### Task 3.1: Comprehensive Integration Tests (AC13)
**Duration**: 0.5 weeks
**Files Created**:
- `crates/perl-dap/tests/integration_tests.rs`: Golden transcript validation
- `crates/perl-dap/tests/fixtures/*.pl`: Test scripts (hello, args, eval, loops)

**Test Coverage Target**: >95% for DAP adapter

**Test Validation**: `cargo test -p perl-dap --test integration_tests`

#### Task 3.2: Performance Benchmarks (AC14)
**Duration**: 0.5 weeks
**Files Created**: `crates/perl-dap/benches/dap_benchmarks.rs`

**Baselines**:
- Breakpoint verification: <50ms
- Step/continue: <100ms p95
- Variable expansion: <200ms initial + <100ms per child
- Memory overhead: <1MB adapter state

**Test Validation**: `cargo bench -p perl-dap`

#### Task 3.3: Documentation Complete (AC15)
**Duration**: 0.5 weeks
**Files Created**:
- `docs/DAP_GETTING_STARTED_TUTORIAL.md`: Step-by-step tutorial with screenshots
- `docs/DAP_CONFIGURATION_REFERENCE.md`: All launch.json parameters documented
- `docs/DAP_TROUBLESHOOTING_GUIDE.md`: Common issues and solutions

**Diátaxis Framework Compliance**: Tutorial, How-to, Reference, Explanation

**Test Validation**: `cargo test --test dap_documentation_complete`

#### Task 3.4: Security Validation (AC16)
**Duration**: 0.5 weeks
**Files Created**: `crates/perl-dap/tests/security_validation.rs`

**Security Requirements**:
- Path traversal prevention (reuse enterprise framework)
- Safe eval enforcement (non-mutating default)
- Timeout enforcement (5s default)
- Unicode boundary safety (PR #153 symmetric conversion)

**Test Validation**: `cargo test -p perl-dap --test security_validation`

#### Task 3.5: LSP Non-Regression (AC17)
**Duration**: 0.5 weeks
**Files Created**: `crates/perl-lsp/tests/lsp_dap_non_regression.rs`

**Requirements**:
- 100% LSP test pass rate with DAP active
- <50ms LSP response time maintained
- No memory leaks or resource contention

**Test Validation**: `cargo test -p perl-lsp --test lsp_dap_non_regression`

#### Task 3.6: Dependency Management (AC18)
**Duration**: 0.5 weeks
**Files Created**: `docs/DAP_DEPENDENCY_MANAGEMENT.md`

**Installation Strategies**:
1. Auto-install via cpanm (recommended)
2. Bundled fallback (extension bundles Perl shim)
3. System package (future enhancement)

**Versioning Strategy**: Adapter ↔ shim protocol versioning with feature detection

**Test Validation**: `cargo test --test dap_dependency_installation`

#### Task 3.7: Binary Packaging (AC19)
**Duration**: 0.5 weeks
**Deliverable**: Platform binaries for 6 targets

**Platforms**:
- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`
- `aarch64-pc-windows-msvc`

**Distribution**: GitHub Releases with auto-download fallback

**Test Validation**: `cargo test --test dap_binary_packaging`

**Success Criteria**: All Phase 3 ACs passing, production-ready DAP adapter

---

## 5. Performance Specifications

### 5.1 Latency Targets

| Operation | Target Latency | Measurement Method |
|-----------|---------------|-------------------|
| Breakpoint verification | <50ms | `cargo bench -p perl-dap -- verify_breakpoint` |
| setBreakpoints request | <100ms p95 | Golden transcript timing validation |
| continue/next/stepIn/stepOut | <100ms p95 | Control flow performance benchmark |
| Variable scope retrieval | <200ms initial | Variable rendering benchmark |
| Child variable expansion | <100ms per child | Lazy expansion benchmark |
| Incremental breakpoint update | <1ms | Incremental parsing integration test |

### 5.2 Memory Targets

| Component | Target | Measurement Method |
|-----------|--------|-------------------|
| Adapter state overhead | <1MB per session | Memory profiling in integration tests |
| Perl shim process | <5MB | Process memory monitoring |
| Variable rendering buffer | <1KB preview | Truncation enforcement test |
| Total session overhead | <10MB | End-to-end memory validation |

### 5.3 Scalability Targets

| Scenario | Target | Validation |
|----------|--------|-----------|
| Large codebase (100K LOC) | <50ms breakpoint verification | Large file benchmark |
| Deep call stack (50+ frames) | <200ms stack trace retrieval | Stack depth stress test |
| Large variable (10MB+ array) | Lazy expansion only | Variable truncation test |
| Concurrent sessions | No resource contention | Multi-session isolation test |

---

## 6. Security Specifications

### 6.1 Path Traversal Prevention

**Requirement**: All file paths validated through existing enterprise security framework

**Implementation**:
```rust
// crates/perl-dap/src/security.rs
use perl_parser::security::validate_workspace_path;

pub fn validate_breakpoint_path(uri: &str, workspace_root: &Path) -> Result<PathBuf> {
    // Convert URI to filesystem path
    let path = uri_to_path(uri)?;

    // Validate path is within workspace boundaries
    let canonical = validate_workspace_path(&path, workspace_root)?;

    // Prevent directory traversal attacks
    if canonical.components().any(|c| c == Component::ParentDir) {
        return Err(SecurityError::PathTraversalAttempt(uri.to_string()));
    }

    Ok(canonical)
}
```

**Test Coverage**:
- Valid workspace paths
- Path traversal attempts (`../../../etc/passwd`)
- UNC path validation (Windows)
- Symlink resolution (macOS/Linux)

### 6.2 Safe Evaluation Mode

**Requirement**: Non-mutating eval default with explicit opt-in for side effects

**Implementation**:
```rust
// crates/perl-dap/src/eval.rs
pub async fn evaluate_expression(
    expr: &str,
    context: &StackFrame,
    allow_side_effects: bool,
) -> Result<Value> {
    // Input validation: prevent code injection
    validate_expression_safety(expr)?;

    // Timeout enforcement: prevent DoS (5s default, configurable)
    let timeout = Duration::from_secs(5);

    let result = tokio::time::timeout(timeout, async {
        if allow_side_effects {
            // Full evaluation with write access
            context.eval_with_side_effects(expr).await
        } else {
            // Safe evaluation: read-only mode
            context.eval_readonly(expr).await
        }
    }).await??;

    Ok(result)
}
```

**Test Coverage**:
- Read-only expressions (no opt-in)
- Side effect prevention ($var = 42 fails without opt-in)
- Explicit opt-in validation
- Timeout enforcement (infinite loops)

### 6.3 Timeout Enforcement

**Requirement**: Hard timeouts on evaluate requests (<5s default) to prevent DoS

**Configuration**:
```json
{
  "perl.dap.evaluateTimeout": 5,  // seconds
  "perl.dap.evaluateMaxDepth": 10  // recursion depth limit
}
```

**Test Coverage**:
- Infinite loop detection
- Recursive function timeout
- Configurable timeout override

### 6.4 Unicode Boundary Safety

**Requirement**: Reuse PR #153 symmetric position conversion for variable rendering

**Implementation**:
```rust
// crates/perl-dap/src/variables.rs
use perl_lsp::textdoc::ensure_utf16_boundary;

pub fn render_variable_value(value: &str, rope: &Rope) -> String {
    if value.len() > 1024 {
        let truncated = &value[..1024];

        // UTF-16 safe truncation (PR #153 infrastructure)
        let safe_truncate = ensure_utf16_boundary(truncated, rope);
        format!("{}…", safe_truncate)
    } else {
        value.to_string()
    }
}
```

**Test Coverage**:
- Multi-byte character truncation (emoji, CJK)
- UTF-16 surrogate pair safety
- Variable rendering with Unicode content

---

## 7. Cross-Platform Strategy

### 7.1 Platform-Specific Binaries

**Targets**:
1. `x86_64-unknown-linux-gnu` (Linux x86_64)
2. `aarch64-unknown-linux-gnu` (Linux ARM64)
3. `x86_64-apple-darwin` (macOS Intel)
4. `aarch64-apple-darwin` (macOS Apple Silicon)
5. `x86_64-pc-windows-msvc` (Windows x86_64)
6. `aarch64-pc-windows-msvc` (Windows ARM64)

**Build Strategy**:
```yaml
# .github/workflows/release-dap-binaries.yml
name: Release DAP Binaries

on:
  release:
    types: [created]

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
          - os: windows-latest
            target: aarch64-pc-windows-msvc

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: ${{ matrix.target }}

      - name: Build perl-dap binary
        run: cargo build -p perl-dap --release --target ${{ matrix.target }}

      - name: Upload artifact
        uses: actions/upload-release-asset@v1
        with:
          upload_url: ${{ github.event.release.upload_url }}
          asset_path: target/${{ matrix.target }}/release/perl-dap${{ matrix.os == 'windows-latest' && '.exe' || '' }}
          asset_name: perl-dap-${{ matrix.target }}${{ matrix.os == 'windows-latest' && '.exe' || '' }}
```

### 7.2 WSL Support Requirements

**Challenges**:
- WSL1 vs WSL2 path translation differences
- Windows ↔ Linux path mapping (`/mnt/c` vs `C:\`)
- Performance implications of WSL filesystem access

**Implementation**:
```rust
// crates/perl-dap/src/platform.rs
#[cfg(target_os = "linux")]
pub fn normalize_wsl_path(path: &Path) -> Result<PathBuf> {
    let path_str = path.to_str().ok_or(PathError::InvalidUtf8)?;

    // Detect WSL mount point (/mnt/c, /mnt/d, etc.)
    if let Some(drive) = path_str.strip_prefix("/mnt/") {
        let drive_letter = drive.chars().next().ok_or(PathError::InvalidPath)?;
        let rest = &drive[1..];

        // Convert /mnt/c/Users/... to C:\Users\...
        let windows_path = format!("{}:{}", drive_letter.to_uppercase(), rest.replace('/', "\\"));
        return Ok(PathBuf::from(windows_path));
    }

    Ok(path.to_path_buf())
}
```

**Test Coverage**:
- WSL1 path translation
- WSL2 path translation
- Performance benchmarks for WSL filesystem access

### 7.3 Path Normalization

**Windows-Specific**:
- Drive letter normalization (`C:` vs `c:`)
- UNC path support (`\\server\share\file.pl`)
- CRLF line ending handling

**macOS/Linux-Specific**:
- Symlink resolution (`/tmp`, `~/Library`)
- Case-sensitive filesystem handling
- UNIX signal handling (SIGINT for pause)

**Implementation**:
```rust
// crates/perl-dap/src/platform.rs
pub fn normalize_path(path: &Path) -> Result<PathBuf> {
    #[cfg(windows)]
    {
        // Normalize drive letter to uppercase
        let path_str = path.to_str().ok_or(PathError::InvalidUtf8)?;
        if let Some((drive, rest)) = path_str.split_once(':') {
            let normalized = format!("{}:{}", drive.to_uppercase(), rest);
            return Ok(PathBuf::from(normalized));
        }
    }

    // Canonicalize path (resolves symlinks, normalizes separators)
    path.canonicalize().map_err(|e| PathError::Canonicalization(e))
}
```

---

## 8. Test Strategy

### 8.1 Golden Transcript Tests

**Purpose**: Validate DAP protocol compliance with expected message sequences

**Test Structure**:
```rust
// crates/perl-dap/tests/integration_tests.rs
#[test] // AC13
fn test_hello_world_golden_transcript() {
    let transcript = load_golden_transcript("hello.json");
    let adapter = DapAdapter::new();

    for message in transcript.messages {
        if message.type_ == "request" {
            let response = adapter.handle_request(message.request)?;
            assert_eq!(response, message.expected_response,
                       "Transcript mismatch at seq {}", message.seq);
        }
    }
}
```

**Golden Transcript Files**:
- `hello.json`: Basic launch → breakpoint → continue → terminate
- `args.json`: Command-line argument passing
- `eval.json`: Expression evaluation with side effects
- `loops.json`: Step through loop iterations

### 8.2 Breakpoint Matrix Tests

**Purpose**: Validate breakpoint verification edge cases

**Test Matrix**:
```rust
#[test] // AC13
fn test_breakpoint_edge_cases() {
    let fixtures = vec![
        ("file_start.pl", 1, true),       // First line
        ("file_end.pl", 100, true),       // Last line
        ("blank_line.pl", 10, false),     // Blank line
        ("comment_line.pl", 5, false),    // Comment line
        ("heredoc.pl", 15, false),        // Inside heredoc
        ("pod_doc.pl", 20, false),        // Inside POD
        ("begin_block.pl", 3, true),      // BEGIN block
        ("end_block.pl", 50, true),       // END block
        ("multiline_stmt.pl", 12, true),  // Multi-line statement
    ];

    for (fixture, line, should_verify) in fixtures {
        let result = verify_breakpoint(fixture, line);
        assert_eq!(result.verified, should_verify,
                   "Breakpoint verification mismatch for {}:{}",
                   fixture, line);
    }
}
```

### 8.3 Variable Rendering Tests

**Purpose**: Validate variable serialization and truncation

**Test Cases**:
```rust
#[test] // AC13
fn test_variable_rendering_edge_cases() {
    let test_cases = vec![
        ("$scalar", "42", "scalar", 0),
        ("@array", "[10 items]", "array", 3001),  // variablesReference for expansion
        ("%hash", "{5 keys}", "hash", 3002),
        ("$unicode", "Hello 😀 World", "scalar", 0),
        ("$large", "data…", "scalar", 0),  // >1KB truncated
        ("&coderef", "sub { ... }", "code", 0),
    ];

    for (name, expected_value, expected_type, expected_ref) in test_cases {
        let var = render_variable(name)?;
        assert_eq!(var.value, expected_value);
        assert_eq!(var.type_, expected_type);
        assert_eq!(var.variablesReference, expected_ref);
    }
}
```

### 8.4 Security Validation Tests

**Purpose**: Validate enterprise security framework compliance

**Test Coverage**:
```rust
// crates/perl-dap/tests/security_validation.rs

#[test] // AC16
fn test_path_traversal_prevention() {
    let adapter = DapAdapter::new();

    // Valid workspace path
    let result = adapter.set_breakpoints("file:///workspace/lib/Module.pm", vec![]);
    assert!(result.is_ok());

    // Path traversal attack
    let result = adapter.set_breakpoints("file:///workspace/../../../etc/passwd", vec![]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("traversal"));
}

#[test] // AC16
fn test_safe_eval_prevents_side_effects() {
    let frame = create_test_frame();

    // Read-only expression OK
    let result = evaluate_expression("$var + 10", &frame, false);
    assert!(result.is_ok());

    // Side effect without opt-in fails
    let result = evaluate_expression("$var = 42", &frame, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("side effect"));

    // Explicit opt-in succeeds
    let result = evaluate_expression("$var = 42", &frame, true);
    assert!(result.is_ok());
}

#[test] // AC16
fn test_eval_timeout_prevents_dos() {
    let frame = create_test_frame();

    // Infinite loop should timeout
    let result = evaluate_expression("while(1) {}", &frame, true);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("timeout"));
}

#[test] // AC16
fn test_unicode_boundary_safety() {
    let rope = Rope::from_str("my $emoji = '😀👨‍👩‍👧‍👦🎉';");

    // Large unicode value should truncate safely
    let large_value = "😀".repeat(500); // 2000 bytes
    let rendered = render_variable_value(&large_value, &rope);

    assert!(rendered.len() <= 1024 + 1); // +1 for '…'
    assert!(rendered.ends_with('…'));
    assert!(is_valid_utf8(&rendered));
}
```

### 8.5 Cross-Platform CI Validation

**Purpose**: Ensure compatibility across all 6 platform targets

**GitHub Actions**:
```yaml
name: DAP Cross-Platform Tests

on: [push, pull_request]

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        perl: ['5.16', '5.30', '5.38']

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v3
      - uses: shogo82148/actions-setup-perl@v1
        with:
          perl-version: ${{ matrix.perl }}

      - name: Install Devel::TSPerlDAP
        run: cpanm Devel::TSPerlDAP

      - name: Build perl-dap
        run: cargo build -p perl-dap --release

      - name: Run DAP tests
        run: cargo test -p perl-dap

      - name: Run cross-platform validation
        run: cargo test -p perl-dap --test cross_platform_validation
```

---

## 9. Documentation Requirements

### 9.1 Diátaxis Framework Compliance

**Tutorial**: `docs/DAP_GETTING_STARTED_TUTORIAL.md`
- Step-by-step debugging workflow with screenshots
- VS Code setup and configuration
- First debugging session walkthrough

**How-To Guide**: `docs/DAP_CONFIGURATION_REFERENCE.md`
- Launch.json parameter reference
- Attach configuration for remote debugging
- Platform-specific setup (Windows, macOS, Linux, WSL)

**Reference**: `docs/DAP_PROTOCOL_SCHEMA.md` (this document)
- DAP protocol message schemas
- Request/response formats
- Error codes and handling

**Explanation**: `docs/CRATE_ARCHITECTURE_DAP.md`
- Architecture decision rationale
- Rust adapter + Perl shim design
- LSP integration patterns

### 9.2 Troubleshooting Guide

**File**: `docs/DAP_TROUBLESHOOTING_GUIDE.md`

**Common Issues**:
1. **Breakpoints not hitting**
   - Cause: Path mapping mismatch
   - Fix: Verify pathMapping in launch.json

2. **Windows UNC path failures**
   - Cause: Network share path normalization
   - Fix: Use drive letter mapping (Z:\ instead of \\server\share)

3. **WSL path translation errors**
   - Cause: WSL1 vs WSL2 differences
   - Fix: Use WSL2 for best compatibility

4. **Variable expansion timeout**
   - Cause: Large data structure rendering
   - Fix: Increase evaluateTimeout configuration

---

## 10. Success Metrics

### 10.1 Functional Metrics

| Metric | Target | Validation |
|--------|--------|-----------|
| Acceptance criteria passing | 19/19 (100%) | All `// AC:ID` tests passing |
| DAP protocol compliance | 100% | Golden transcript validation |
| Breakpoint verification accuracy | >95% | Breakpoint matrix tests |
| Stack frame resolution | 98% | Workspace navigation dual indexing |

### 10.2 Performance Metrics

| Metric | Target | Validation |
|--------|--------|-----------|
| Breakpoint operations | <50ms | `cargo bench -p perl-dap -- verify_breakpoint` |
| Step/continue operations | <100ms p95 | Control flow performance benchmark |
| Variable expansion | <200ms initial, <100ms per child | Variable rendering benchmark |
| Incremental breakpoint update | <1ms | Incremental parsing integration test |
| Memory overhead | <1MB adapter + <5MB shim | Memory profiling tests |

### 10.3 Quality Metrics

| Metric | Target | Validation |
|--------|--------|-----------|
| Test coverage (adapter) | >95% | `cargo tarpaulin -p perl-dap` |
| Test coverage (shim) | >80% | `cover -test` (Devel::TSPerlDAP) |
| Security findings | 0 | `cargo test -p perl-dap --test security_validation` |
| LSP non-regression | 100% pass rate | `cargo test -p perl-lsp --test lsp_dap_non_regression` |
| Documentation completeness | 100% | `cargo test --test dap_documentation_complete` |

### 10.4 Cross-Platform Metrics

| Metric | Target | Validation |
|--------|--------|-----------|
| Platform binaries | 6 targets | GitHub Actions build matrix |
| Cross-platform tests | 100% pass rate | CI validation on Linux/macOS/Windows |
| WSL compatibility | Full support | WSL-specific test suite |
| Perl version compatibility | 5.16+ | Test matrix with 5.16, 5.30, 5.38 |

---

## 11. References

### 11.1 Related Specifications

- [DAP Protocol Schema](DAP_PROTOCOL_SCHEMA.md): JSON-RPC message schemas
- [DAP Crate Architecture](CRATE_ARCHITECTURE_DAP.md): Component design
- [DAP Security Specification](DAP_SECURITY_SPECIFICATION.md): Security requirements

### 11.2 Existing Perl LSP Documentation

- [LSP Implementation Guide](LSP_IMPLEMENTATION_GUIDE.md): LSP server architecture
- [Security Development Guide](SECURITY_DEVELOPMENT_GUIDE.md): Enterprise security practices
- [Incremental Parsing Guide](INCREMENTAL_PARSING_GUIDE.md): <1ms parser updates
- [Workspace Navigation Guide](WORKSPACE_NAVIGATION_GUIDE.md): Dual indexing strategy
- [Position Tracking Guide](POSITION_TRACKING_GUIDE.md): UTF-16 ↔ UTF-8 conversion

### 11.3 External Standards

- [Debug Adapter Protocol Specification](https://microsoft.github.io/debug-adapter-protocol/)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [CPAN Module Best Practices](https://metacpan.org/pod/distribution/Module-Starter/lib/Module/Starter.pm)

---

## 12. Appendix: Validation Commands

### 12.1 Phase 1 (Bridge) Commands

```bash
# AC1: Extension debugger contribution
cd vscode-extension && npm test

# AC2: Launch.json snippets
cargo test --test dap_launch_snippets -- windows macos linux

# AC3: Documentation completeness
cargo test --test dap_documentation_coverage -- AC3

# AC4: Basic workflow validation
cargo test --test bridge_workflow_tests
```

### 12.2 Phase 2 (Native) Commands

```bash
# AC5: Protocol scaffolding
cargo test -p perl-dap --test protocol_compliance

# AC6: Perl shim tests
cd Devel-TSPerlDAP && prove -lv t/

# AC7: Breakpoint management
cargo test -p perl-dap --test breakpoint_validation

# AC8: Variables and scopes
cargo test -p perl-dap --test variable_rendering

# AC9: Stepping and control flow
cargo test -p perl-dap --test control_flow_performance

# AC10: Evaluate and REPL
cargo test -p perl-dap --test eval_security

# AC11: VS Code native integration
cd vscode-extension && npm test -- native

# AC12: Cross-platform compatibility
cargo test -p perl-dap --test cross_platform_validation
```

### 12.3 Phase 3 (Hardening) Commands

```bash
# AC13: Integration tests
cargo test -p perl-dap --test integration_tests

# AC14: Performance benchmarks
cargo bench -p perl-dap

# AC15: Documentation completeness
cargo test --test dap_documentation_complete

# AC16: Security validation
cargo test -p perl-dap --test security_validation

# AC17: LSP non-regression
cargo test -p perl-lsp --test lsp_dap_non_regression

# AC18: Dependency management
cargo test --test dap_dependency_installation

# AC19: Binary packaging
cargo test --test dap_binary_packaging
```

---

**End of DAP Implementation Specification**
