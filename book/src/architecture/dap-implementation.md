# DAP Crate Architecture Specification
<!-- Labels: architecture:dap, crate:perl-dap, integration:lsp -->

**Issue**: #207 - Debug Adapter Protocol Support
**Status**: Architecture Design Complete
**Version**: 0.9.x
**Date**: 2025-10-04

---

## Executive Summary

This specification defines the crate-level architecture for the perl-dap implementation, integrating with the existing Perl LSP ecosystem. The design follows separation-of-concerns principles while maximizing reuse of existing infrastructure (AST integration, incremental parsing, workspace navigation, security framework).

**Key Components**:
- **perl-dap** (new crate): Standalone DAP adapter binary
- **Devel::TSPerlDAP** (CPAN module): Perl runtime shim
- **perl-parser** (integration): AST-based breakpoint validation
- **perl-lsp** (integration): LSP ↔ DAP coordination

**Design Principles**:
- Clean separation between LSP and DAP protocols
- Optional dependency: LSP server can run without DAP
- Focused testing: DAP-specific test infrastructure
- Independent versioning: DAP features evolve separately from LSP

---

## 1. Workspace Structure

### 1.1 Crate Organization

```
crates/
├── perl-parser/          # Core parser (unchanged)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── parser.rs
│   │   ├── workspace_index.rs  # Used by DAP for stack frame resolution
│   │   ├── textdoc.rs          # Used by DAP for position mapping
│   │   └── ...
│   └── Cargo.toml
│
├── perl-lsp/             # LSP server binary (unchanged)
│   ├── src/
│   │   ├── main.rs
│   │   └── ...
│   └── Cargo.toml
│
├── perl-dap/             # NEW - DAP adapter binary
│   ├── src/
│   │   ├── main.rs       # Adapter entry point
│   │   ├── lib.rs        # Public API for integration testing
│   │   ├── protocol/
│   │   │   ├── mod.rs    # DAP protocol types and serialization
│   │   │   ├── request.rs
│   │   │   ├── response.rs
│   │   │   └── event.rs
│   │   ├── session/
│   │   │   ├── mod.rs    # Session state management
│   │   │   ├── lifecycle.rs
│   │   │   └── state.rs
│   │   ├── breakpoints/
│   │   │   ├── mod.rs    # Breakpoint manager with AST validation
│   │   │   └── validator.rs
│   │   ├── variables/
│   │   │   ├── mod.rs    # Variable rendering and lazy expansion
│   │   │   └── renderer.rs
│   │   ├── stack/
│   │   │   ├── mod.rs    # Stack trace provider
│   │   │   └── resolver.rs
│   │   ├── shim/
│   │   │   ├── mod.rs    # Perl shim communication
│   │   │   ├── protocol.rs
│   │   │   └── process.rs
│   │   ├── security/
│   │   │   ├── mod.rs    # Security validation (path, eval, timeout)
│   │   │   └── validator.rs
│   │   └── platform/
│   │       ├── mod.rs    # Platform-specific code
│   │       ├── unix.rs
│   │       └── windows.rs
│   ├── tests/
│   │   ├── integration_tests.rs      # Golden transcript validation (AC13)
│   │   ├── breakpoint_validation.rs  # Breakpoint matrix tests (AC7)
│   │   ├── variable_rendering.rs     # Variable rendering tests (AC8)
│   │   ├── control_flow_performance.rs  # Stepping performance (AC9)
│   │   ├── eval_security.rs          # Safe eval validation (AC10)
│   │   ├── security_validation.rs    # Enterprise security (AC16)
│   │   ├── cross_platform_validation.rs  # Platform compatibility (AC12)
│   │   └── fixtures/
│   │       ├── hello.pl
│   │       ├── args.pl
│   │       ├── eval.pl
│   │       └── loops.pl
│   ├── benches/
│   │   └── dap_benchmarks.rs         # Performance benchmarks (AC14)
│   └── Cargo.toml
│
├── perl-lexer/           # Tokenizer (unchanged)
└── perl-corpus/          # Test corpus (unchanged)

vscode-extension/
├── package.json          # Add contributes.debuggers (AC1, AC11)
├── snippets/
│   └── launch.json       # Launch.json snippets (AC2)
├── src/
│   ├── debugAdapter.ts   # Bridge adapter (Phase 1)
│   ├── nativeDapAdapter.ts  # Native adapter interface (Phase 2)
│   └── dapBinaryManager.ts  # Platform binary management (AC19)
└── resources/
    ├── dap-binaries/     # Platform binaries (AC19)
    │   ├── linux-x64/
    │   ├── linux-arm64/
    │   ├── darwin-x64/
    │   ├── darwin-arm64/
    │   ├── win32-x64/
    │   └── win32-arm64/
    └── perl-shim/        # Bundled fallback (AC18)
        └── Devel/
            └── TSPerlDAP.pm

Devel-TSPerlDAP/          # NEW - CPAN module (separate repo or subdir)
├── lib/
│   └── Devel/
│       └── TSPerlDAP.pm  # Perl shim implementation
├── t/
│   ├── 01-set-breakpoints.t
│   ├── 02-stack-trace.t
│   ├── 03-variables.t
│   ├── 04-evaluate.t
│   └── 05-control-flow.t
├── META.json
├── Makefile.PL
└── README.pod
```

---

## 2. perl-dap Crate Design

### 2.1 Dependencies

```toml
# crates/perl-dap/Cargo.toml
[package]
name = "perl-dap"
version = "0.1.0"
edition = "2024"
authors = ["Tree-sitter Perl Contributors"]
description = "Debug Adapter Protocol server for Perl"
license = "MIT OR Apache-2.0"

[[bin]]
name = "perl-dap"
path = "src/main.rs"

[lib]
name = "perl_dap"
path = "src/lib.rs"

[dependencies]
# Core parser integration
perl-parser = { path = "../perl-parser", version = "0.8.9" }

# LSP types reuse (Position, Range, Location, etc.)
lsp-types = "0.97.0"

# JSON serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
anyhow = "1.0"
thiserror = "2.0"

# Async runtime
tokio = { version = "1.0", features = ["full"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Rope for position mapping (reuse from perl-parser)
ropey = "1.6"

# Process management
tokio-process = "0.2"

[dev-dependencies]
# Property-based testing (AC13)
proptest = "1.0"

# Performance benchmarking (AC14)
criterion = "0.5"

# Test fixtures
tempfile = "3.0"

# Golden transcript validation
serde_yaml = "0.9"

[target.'cfg(unix)'.dependencies]
nix = "0.28"  # For SIGINT handling

[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["processthreadsapi"] }  # For Ctrl+C
```

### 2.2 Module Structure

#### 2.2.1 Main Entry Point

```rust
// crates/perl-dap/src/main.rs
use perl_dap::{DapServer, DapConfig};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Parse command-line arguments
    let config = DapConfig::from_args()?;

    info!("Starting perl-dap adapter version {}", env!("CARGO_PKG_VERSION"));

    // Create DAP server
    let mut server = DapServer::new(config)?;

    // Run stdio transport
    server.run_stdio().await?;

    Ok(())
}
```

#### 2.2.2 Library API

```rust
// crates/perl-dap/src/lib.rs
//! Debug Adapter Protocol server for Perl
//!
//! This crate provides a robust DAP adapter for debugging Perl code.
//! It integrates with the perl-parser crate for AST-based breakpoint validation
//! and leverages existing LSP infrastructure for position mapping and workspace navigation.

mod protocol;
mod session;
mod breakpoints;
mod variables;
mod stack;
mod shim;
mod security;
mod platform;

pub use protocol::{DapRequest, DapResponse, DapEvent};
pub use session::{DapSession, SessionState};
pub use breakpoints::{BreakpointManager, BreakpointVerification};

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use std::sync::Arc;

/// DAP server configuration
pub struct DapConfig {
    pub log_level: String,
    pub install_shim: bool,  // For --install-shim command (AC18)
}

impl DapConfig {
    pub fn from_args() -> Result<Self> {
        // Parse command-line arguments
        let args: Vec<String> = std::env::args().collect();

        Ok(Self {
            log_level: "info".to_string(),
            install_shim: args.contains(&"--install-shim".to_string()),
        })
    }
}

/// Main DAP server
pub struct DapServer {
    config: DapConfig,
    session: Arc<Mutex<Option<DapSession>>>,
    seq: Arc<std::sync::atomic::AtomicI64>,
}

impl DapServer {
    pub fn new(config: DapConfig) -> Result<Self> {
        Ok(Self {
            config,
            session: Arc::new(Mutex::new(None)),
            seq: Arc::new(std::sync::atomic::AtomicI64::new(1)),
        })
    }

    /// Run DAP server over stdio transport
    pub async fn run_stdio(&mut self) -> Result<()> {
        use tokio::io::{stdin, stdout};

        let stdin = BufReader::new(stdin());
        let mut stdout = stdout();

        let mut lines = stdin.lines();

        while let Some(line) = lines.next_line().await? {
            // Parse Content-Length header
            if line.starts_with("Content-Length:") {
                let content_length: usize = line
                    .split(':')
                    .nth(1)
                    .ok_or_else(|| anyhow::anyhow!("Invalid Content-Length header"))?
                    .trim()
                    .parse()?;

                // Skip blank line
                lines.next_line().await?;

                // Read message body
                let mut buffer = vec![0u8; content_length];
                // Note: This simplified example needs proper async reading
                // In production, use tokio::io::AsyncReadExt::read_exact

                let request: DapRequest = serde_json::from_slice(&buffer)?;

                // Handle request
                let response = self.handle_request(request).await?;

                // Serialize response
                let response_json = serde_json::to_string(&response)?;

                // Write response with Content-Length header
                let header = format!("Content-Length: {}\r\n\r\n", response_json.len());
                stdout.write_all(header.as_bytes()).await?;
                stdout.write_all(response_json.as_bytes()).await?;
                stdout.flush().await?;
            }
        }

        Ok(())
    }

    /// Handle DAP request
    async fn handle_request(&self, request: DapRequest) -> Result<DapResponse> {
        match request.command.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "launch" => self.handle_launch(request).await,
            "attach" => self.handle_attach(request).await,
            "setBreakpoints" => self.handle_set_breakpoints(request).await,
            "continue" => self.handle_continue(request).await,
            "next" => self.handle_next(request).await,
            "stepIn" => self.handle_step_in(request).await,
            "stepOut" => self.handle_step_out(request).await,
            "pause" => self.handle_pause(request).await,
            "threads" => self.handle_threads(request).await,
            "stackTrace" => self.handle_stack_trace(request).await,
            "scopes" => self.handle_scopes(request).await,
            "variables" => self.handle_variables(request).await,
            "evaluate" => self.handle_evaluate(request).await,
            "disconnect" => self.handle_disconnect(request).await,
            _ => self.handle_unknown_command(request),
        }
    }

    async fn handle_initialize(&self, request: DapRequest) -> Result<DapResponse> {
        Ok(DapResponse {
            seq: self.next_seq(),
            type_: "response".to_string(),
            request_seq: request.seq,
            success: true,
            command: "initialize".to_string(),
            message: None,
            body: Some(serde_json::json!({
                "supportsConfigurationDoneRequest": true,
                "supportsEvaluateForHovers": true,
                "supportsStepInTargetsRequest": false,
                "supportsSetVariable": false,
                "supportsConditionalBreakpoints": false,
                "supportsExceptionBreakpoints": false,
            })),
        })
    }

    // Additional handlers omitted for brevity...

    fn next_seq(&self) -> i64 {
        self.seq.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    fn handle_unknown_command(&self, request: DapRequest) -> Result<DapResponse> {
        Ok(DapResponse {
            seq: self.next_seq(),
            type_: "response".to_string(),
            request_seq: request.seq,
            success: false,
            command: request.command.clone(),
            message: Some(format!("Unknown command: {}", request.command)),
            body: None,
        })
    }
}
```

#### 2.2.3 Protocol Module

```rust
// crates/perl-dap/src/protocol/mod.rs
use serde::{Deserialize, Serialize};

/// DAP request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapRequest {
    pub seq: i64,
    #[serde(rename = "type")]
    pub type_: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

/// DAP response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapResponse {
    pub seq: i64,
    #[serde(rename = "type")]
    pub type_: String,
    pub request_seq: i64,
    pub success: bool,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

/// DAP event message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapEvent {
    pub seq: i64,
    #[serde(rename = "type")]
    pub type_: String,
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

// Request-specific argument types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchRequestArguments {
    pub program: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "perlPath")]
    pub perl_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "includePaths")]
    pub include_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "stopOnEntry")]
    pub stop_on_entry: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetBreakpointsArguments {
    pub source: DapSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breakpoints: Option<Vec<SourceBreakpoint>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceBreakpoint {
    pub line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
}

// Response-specific body types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    pub id: i64,
    pub verified: bool,
    pub line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub id: i64,
    pub name: String,
    pub source: DapSource,
    pub line: u32,
    pub column: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "presentationHint")]
    pub presentation_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    pub name: String,
    #[serde(rename = "variablesReference")]
    pub variables_reference: i64,
    pub expensive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub type_: Option<String>,
    #[serde(rename = "variablesReference")]
    pub variables_reference: i64,
}
```

#### 2.2.4 Breakpoint Manager

```rust
// crates/perl-dap/src/breakpoints/mod.rs
use perl_parser::{Parser, ast::Node};
use ropey::Rope;
use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;

pub struct BreakpointManager {
    parser: Arc<Parser>,
    breakpoints: HashMap<String, Vec<Breakpoint>>,
    next_id: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BreakpointVerification {
    Verified { line: u32 },
    Invalid { reason: String },
}

impl BreakpointManager {
    pub fn new(parser: Arc<Parser>) -> Self {
        Self {
            parser,
            breakpoints: HashMap::new(),
            next_id: 1,
        }
    }

    /// Verify breakpoint line using AST analysis (AC7)
    /// Performance target: <50ms
    ///
    /// Implementation Note: Uses perl_parser::Parser::parse() which returns ast::Node.
    /// AST validation utilities (is_comment_or_blank_line, is_inside_string_literal, etc.)
    /// are implemented in perl-dap crate (see DAP_BREAKPOINT_VALIDATION_GUIDE.md).
    pub fn verify_breakpoint(&self, uri: &str, line: u32, rope: &Rope) -> Result<BreakpointVerification> {
        // Parse source text using existing Parser::new() and parse() API
        let source = rope.to_string();
        let mut parser = Parser::new(&source);
        let ast = parser.parse()?;

        // Get byte offsets for the line using Rope
        let line_start = rope.line_to_byte(line as usize);
        let line_end = if (line as usize) < rope.len_lines() - 1 {
            rope.line_to_byte(line as usize + 1)
        } else {
            rope.len_bytes()
        };

        // Validate line contains executable code using DAP AST utilities
        if is_comment_or_blank_line(&ast, line_start, line_end, &source) {
            return Ok(BreakpointVerification::Invalid {
                reason: "Line contains only comments or whitespace".to_string()
            });
        }

        // Validate not inside string literal or heredoc using AST node type analysis
        if is_inside_string_literal(&ast, line_start) {
            return Ok(BreakpointVerification::Invalid {
                reason: "Line is inside string literal or heredoc".to_string()
            });
        }

        // Validate not inside POD documentation using text scanning
        if is_inside_pod(&source, line_start) {
            return Ok(BreakpointVerification::Invalid {
                reason: "Line is inside POD documentation".to_string()
            });
        }

        // Adjust to nearest executable line if needed
        let adjusted_line = self.adjust_to_executable_line(&ast, line, rope);

        Ok(BreakpointVerification::Verified { line: adjusted_line })
    }

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

    /// Set breakpoints for a source file
    pub fn set_breakpoints(
        &mut self,
        uri: &str,
        requested: Vec<(u32, Option<u32>)>, // (line, column)
    ) -> Result<Vec<Breakpoint>> {
        let mut breakpoints = Vec::new();

        for (line, column) in requested {
            let verification = self.verify_breakpoint(uri, line)?;

            let (verified, actual_line, message) = match verification {
                BreakpointVerification::Verified { line } => (true, line, None),
                BreakpointVerification::Invalid { reason } => (false, line, Some(reason)),
            };

            let breakpoint = Breakpoint {
                id: self.next_id,
                verified,
                line: actual_line,
                column,
                message,
            };

            self.next_id += 1;
            breakpoints.push(breakpoint.clone());
        }

        self.breakpoints.insert(uri.to_string(), breakpoints.clone());

        Ok(breakpoints)
    }
}
```

#### 2.2.5 Variable Renderer

```rust
// crates/perl-dap/src/variables/mod.rs
use ropey::Rope;
use anyhow::Result;

pub struct VariableRenderer {
    // Variable expansion references
    expansion_refs: HashMap<i64, ExpandableValue>,
    next_ref: i64,
}

enum ExpandableValue {
    Array(Vec<String>),
    Hash(HashMap<String, String>),
}

impl VariableRenderer {
    pub fn new() -> Self {
        Self {
            expansion_refs: HashMap::new(),
            next_ref: 1000,
        }
    }

    /// Render variable value with truncation and lazy expansion
    /// Performance target: <200ms initial, <100ms per child expansion
    pub fn render_value(&mut self, value: &PerlValue, rope: &Rope) -> Variable {
        match value {
            PerlValue::Scalar(s) => Variable {
                name: value.name(),
                value: self.render_scalar(s, rope),
                type_: Some("scalar".to_string()),
                variables_reference: 0,
            },
            PerlValue::Array(arr) => {
                let ref_id = self.allocate_expansion_ref(ExpandableValue::Array(arr.clone()));
                Variable {
                    name: value.name(),
                    value: format!("[{} items]", arr.len()),
                    type_: Some("array".to_string()),
                    variables_reference: ref_id,
                }
            },
            PerlValue::Hash(hash) => {
                let ref_id = self.allocate_expansion_ref(ExpandableValue::Hash(hash.clone()));
                Variable {
                    name: value.name(),
                    value: format!("{{{} keys}}", hash.len()),
                    type_: Some("hash".to_string()),
                    variables_reference: ref_id,
                }
            },
            PerlValue::CodeRef(code) => Variable {
                name: value.name(),
                value: self.render_coderef(code),
                type_: Some("code".to_string()),
                variables_reference: 0,
            },
        }
    }

    fn render_scalar(&self, value: &str, rope: &Rope) -> String {
        // Truncate large values (AC8: 1KB preview max)
        if value.len() > 1024 {
            let truncated = &value[..1024];

            // UTF-16 safe truncation (PR #153 infrastructure)
            let safe_truncate = ensure_utf16_boundary(truncated, rope);
            format!("{}…", safe_truncate)
        } else {
            value.to_string()
        }
    }

    fn render_coderef(&self, code: &str) -> String {
        // Use B::Deparse representation from Perl shim
        format!("sub {{ {} }}", code)
    }

    fn allocate_expansion_ref(&mut self, value: ExpandableValue) -> i64 {
        let ref_id = self.next_ref;
        self.next_ref += 1;
        self.expansion_refs.insert(ref_id, value);
        ref_id
    }

    /// Expand variable reference to children
    pub fn expand_variable(&self, ref_id: i64) -> Result<Vec<Variable>> {
        let expandable = self.expansion_refs.get(&ref_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid variable reference: {}", ref_id))?;

        match expandable {
            ExpandableValue::Array(arr) => {
                Ok(arr.iter().enumerate().map(|(idx, val)| Variable {
                    name: format!("[{}]", idx),
                    value: val.clone(),
                    type_: Some("scalar".to_string()),
                    variables_reference: 0,
                }).collect())
            },
            ExpandableValue::Hash(hash) => {
                Ok(hash.iter().map(|(key, val)| Variable {
                    name: format!("{{{}}}", key),
                    value: val.clone(),
                    type_: Some("scalar".to_string()),
                    variables_reference: 0,
                }).collect())
            },
        }
    }
}

// Helper for UTF-16 safe truncation
fn ensure_utf16_boundary(s: &str, _rope: &Rope) -> String {
    // Reuse perl-parser UTF-16 boundary validation (PR #153)
    // This is a simplified example; actual implementation would use
    // perl_lsp::textdoc::ensure_utf16_boundary

    s.to_string() // Placeholder
}
```

---

## 3. Devel::TSPerlDAP CPAN Module

### 3.1 Module Structure

```
Devel-TSPerlDAP/
├── lib/
│   └── Devel/
│       └── TSPerlDAP.pm
├── t/
│   ├── 01-set-breakpoints.t
│   ├── 02-stack-trace.t
│   ├── 03-variables.t
│   ├── 04-evaluate.t
│   └── 05-control-flow.t
├── META.json
├── Makefile.PL
└── README.pod
```

### 3.2 Core Implementation

```perl
# lib/Devel/TSPerlDAP.pm
package Devel::TSPerlDAP;

use strict;
use warnings;
use JSON::PP;
use IO::Socket::INET;
use PadWalker qw(peek_my peek_our);
use B::Deparse;

our $VERSION = '0.1.0';

# Global state
our %BREAKPOINTS;     # file => { line => 1 }
our $SERVER_SOCKET;
our $CLIENT_SOCKET;
our $JSON = JSON::PP->new->utf8;

sub import {
    my ($class, %opts) = @_;

    my $daemon = $opts{daemon} // 0;
    my $host = $opts{host} // '127.0.0.1';
    my $port = $opts{port} // 0;  # 0 = random port

    if ($daemon) {
        start_tcp_server($host, $port);
    } else {
        start_stdio_server();
    }
}

sub start_stdio_server {
    # Read JSON commands from STDIN, write responses to STDOUT
    while (my $line = <STDIN>) {
        chomp $line;
        my $request = eval { $JSON->decode($line) };

        if ($@) {
            print STDERR "JSON parse error: $@\n";
            next;
        }

        my $response = handle_command($request);
        print $JSON->encode($response), "\n";
    }
}

sub start_tcp_server {
    my ($host, $port) = @_;

    $SERVER_SOCKET = IO::Socket::INET->new(
        LocalAddr => $host,
        LocalPort => $port,
        Proto     => 'tcp',
        Listen    => 1,
        Reuse     => 1,
    ) or die "Cannot start TCP server: $!\n";

    # Print actual port for client discovery
    my $actual_port = $SERVER_SOCKET->sockport();
    print STDERR "TSPerlDAP listening on $host:$actual_port\n";

    # Accept single client connection
    $CLIENT_SOCKET = $SERVER_SOCKET->accept();

    # Read JSON commands from socket
    while (my $line = <$CLIENT_SOCKET>) {
        chomp $line;
        my $request = eval { $JSON->decode($line) };

        if ($@) {
            print STDERR "JSON parse error: $@\n";
            next;
        }

        my $response = handle_command($request);
        print $CLIENT_SOCKET $JSON->encode($response), "\n";
    }
}

sub handle_command {
    my ($request) = @_;

    my $command = $request->{command};

    # Dispatch to command handlers
    return set_breakpoints($request->{arguments})   if $command eq 'set_breakpoints';
    return continue_execution()                     if $command eq 'continue';
    return step_next()                              if $command eq 'next';
    return step_in()                                if $command eq 'step_in';
    return step_out()                               if $command eq 'step_out';
    return pause_execution()                        if $command eq 'pause';
    return get_stack_trace()                        if $command eq 'stack';
    return get_scopes($request->{arguments})        if $command eq 'scopes';
    return get_variables($request->{arguments})     if $command eq 'variables';
    return evaluate_expression($request->{arguments}) if $command eq 'evaluate';

    return { success => 0, message => "Unknown command: $command" };
}

sub set_breakpoints {
    my ($args) = @_;

    my $file = $args->{source}{path};
    my @breakpoints = @{$args->{breakpoints} // []};

    # Clear existing breakpoints for this file
    delete $BREAKPOINTS{$file};

    # Set new breakpoints using Perl debugger API
    foreach my $bp (@breakpoints) {
        my $line = $bp->{line};
        $BREAKPOINTS{$file}{$line} = 1;

        # Set debugger breakpoint
        $DB::single{$file}{$line} = 1;
    }

    return {
        success => 1,
        breakpoints => [
            map { { id => $_, verified => 1, line => $_ } } @breakpoints
        ]
    };
}

sub get_stack_trace {
    my @frames;
    my $i = 0;

    # Walk call stack using caller()
    while (my ($package, $file, $line, $sub) = caller($i++)) {
        # Skip internal frames (debugger, shim infrastructure)
        next if $package =~ /^(DB|Devel::TSPerlDAP)/;

        push @frames, {
            name => $sub,
            source => { path => $file },
            line => $line,
            column => 0,
        };
    }

    return {
        stackFrames => \@frames,
        totalFrames => scalar(@frames),
    };
}

sub get_scopes {
    my ($args) = @_;
    my $frame_id = $args->{frameId};

    # Return standard scopes: Locals, Package, Globals
    return {
        scopes => [
            {
                name => 'Locals',
                variablesReference => $frame_id * 1000 + 1,
                expensive => 0,
            },
            {
                name => 'Package',
                variablesReference => $frame_id * 1000 + 2,
                expensive => 0,
            },
        ]
    };
}

sub get_variables {
    my ($args) = @_;
    my $ref = $args->{variablesReference};

    # Decode scope type from reference
    my $frame_id = int($ref / 1000);
    my $scope_type = $ref % 1000;

    my @variables;

    if ($scope_type == 1) {
        # Locals: Use PadWalker to inspect lexical variables
        my $vars = peek_my($frame_id);

        foreach my $name (sort keys %$vars) {
            my $value = $vars->{$name};

            push @variables, {
                name => $name,
                value => render_value($value),
                type => ref($value) || 'scalar',
                variablesReference => is_expandable($value) ? allocate_ref($value) : 0,
            };
        }
    } elsif ($scope_type == 2) {
        # Package: Use peek_our for package variables
        my $vars = peek_our($frame_id);

        foreach my $name (sort keys %$vars) {
            my $value = $vars->{$name};

            push @variables, {
                name => $name,
                value => render_value($value),
                type => ref($value) || 'scalar',
                variablesReference => is_expandable($value) ? allocate_ref($value) : 0,
            };
        }
    }

    return { variables => \@variables };
}

sub render_value {
    my ($value) = @_;

    if (ref($value) eq 'CODE') {
        # Use B::Deparse for code refs
        my $deparse = B::Deparse->new();
        return $deparse->coderef2text($value);
    } elsif (ref($value) eq 'ARRAY') {
        return "[" . scalar(@$value) . " items]";
    } elsif (ref($value) eq 'HASH') {
        return "{" . scalar(keys %$value) . " keys}";
    } else {
        # Truncate large values (AC8: 1KB max)
        my $str = "$value";
        return length($str) > 1024 ? substr($str, 0, 1024) . "…" : $str;
    }
}

sub is_expandable {
    my ($value) = @_;
    return ref($value) =~ /^(ARRAY|HASH)$/;
}

our %EXPANSION_REFS;
our $NEXT_REF = 3000;

sub allocate_ref {
    my ($value) = @_;

    my $ref_id = $NEXT_REF++;
    $EXPANSION_REFS{$ref_id} = $value;

    return $ref_id;
}

sub evaluate_expression {
    my ($args) = @_;
    my $expr = $args->{expression};
    my $frame_id = $args->{frameId};
    my $allow_side_effects = $args->{allowSideEffects} // 0;

    # Safe evaluation (AC10)
    my $result;
    eval {
        local $SIG{ALRM} = sub { die "timeout\n" };
        alarm(5);  # 5 second timeout

        if ($allow_side_effects) {
            # Full evaluation with write access
            $result = eval $expr;
        } else {
            # Safe evaluation: read-only mode
            # Note: This is a simplified check; production would use Safe.pm
            if ($expr =~ /=(?![=~])/) {
                die "Side effects not allowed without allowSideEffects flag\n";
            }
            $result = eval $expr;
        }

        alarm(0);
    };

    if ($@) {
        return {
            success => 0,
            message => "Evaluation failed: $@",
        };
    }

    return {
        success => 1,
        result => render_value($result),
        type => ref($result) || 'scalar',
        variablesReference => is_expandable($result) ? allocate_ref($result) : 0,
    };
}

# Control flow commands
sub continue_execution {
    $DB::single = 0;
    return { success => 1 };
}

sub step_next {
    $DB::single = 1;
    return { success => 1 };
}

sub step_in {
    $DB::single = 1;
    $DB::step = 1;
    return { success => 1 };
}

sub step_out {
    $DB::single = 0;
    $DB::trace = 1;
    return { success => 1 };
}

sub pause_execution {
    $DB::signal = 1;
    return { success => 1 };
}

1;

__END__

=head1 NAME

Devel::TSPerlDAP - Debug Adapter Protocol shim for Perl debugger

=head1 SYNOPSIS

    # Launch debugger with stdio protocol
    perl -d:TSPerlDAP script.pl

    # Launch debugger with TCP protocol
    perl -d:TSPerlDAP=daemon,host=127.0.0.1,port=5000 script.pl

=head1 DESCRIPTION

Devel::TSPerlDAP provides a machine-readable JSON protocol bridge to the Perl
debugger (perl -d). It is designed for integration with the perl-dap Debug
Adapter Protocol server.

=head1 REQUIREMENTS

=over 4

=item * Perl 5.16 or higher (5.30+ recommended)

=item * JSON::PP (core module)

=item * PadWalker 2.0+

=item * B::Deparse (core module)

=back

=head1 AUTHOR

Tree-sitter Perl Contributors

=head1 LICENSE

MIT OR Apache-2.0

=cut
```

### 3.3 Test Suite

```perl
# t/01-set-breakpoints.t
use strict;
use warnings;
use Test::More tests => 5;

use Devel::TSPerlDAP;

my $result = Devel::TSPerlDAP::set_breakpoints({
    source => { path => 'test.pl' },
    breakpoints => [ { line => 10 }, { line => 20 } ]
});

ok($result->{success}, "Set breakpoints succeeded");
is(scalar @{$result->{breakpoints}}, 2, "Two breakpoints set");
is($result->{breakpoints}[0]{line}, 10, "First breakpoint at line 10");
is($result->{breakpoints}[1]{line}, 20, "Second breakpoint at line 20");
ok($result->{breakpoints}[0]{verified}, "Breakpoint verified");

# t/02-stack-trace.t
use strict;
use warnings;
use Test::More tests => 3;

use Devel::TSPerlDAP;

sub outer { inner() }
sub inner {
    my $stack = Devel::TSPerlDAP::get_stack_trace();
    return $stack;
}

my $result = outer();
ok(scalar @{$result->{stackFrames}} >= 2, "Stack has at least 2 frames");
like($result->{stackFrames}[0]{name}, qr/inner/, "Top frame is 'inner'");
is($result->{totalFrames}, scalar @{$result->{stackFrames}}, "Total frames matches array size");
```

---

## 4. Integration with perl-parser

### 4.1 AST Integration

**Purpose**: Breakpoint validation using existing ~100% Perl syntax coverage

**Integration Points**:
- `perl_parser::Parser::parse_file()`: Parse source for AST
- `perl_parser::AstNode::line_to_span()`: Convert line number to span
- `perl_parser::AstNode::is_comment_or_blank_line()`: Validate executable code
- `perl_parser::AstNode::is_inside_string_literal()`: Prevent breakpoints in strings
- `perl_parser::AstNode::is_inside_pod()`: Prevent breakpoints in documentation

**Usage Example**:
```rust
// crates/perl-dap/src/breakpoints/validator.rs
use perl_parser::{Parser, AstNode};

pub fn validate_breakpoint_line(parser: &Parser, uri: &str, line: u32) -> Result<bool> {
    let ast = parser.parse_file(uri)?;
    let span = ast.line_to_span(line)?;

    Ok(!ast.is_comment_or_blank_line(span) &&
       !ast.is_inside_string_literal(span) &&
       !ast.is_inside_pod(span))
}
```

### 4.2 Incremental Parsing Integration

**Purpose**: Live breakpoint adjustment as code changes

**Integration Points**:
- `perl_parser::incremental_v2::IncrementalParserV2::apply_edits()`: <1ms updates
- `perl_parser::TextEdit`: Document change representation

**Usage Example**:
```rust
// crates/perl-dap/src/session.rs
use perl_parser::incremental_v2::IncrementalParserV2;

impl DapSession {
    pub fn on_text_change(&mut self, uri: &str, changes: Vec<TextEdit>) -> Result<()> {
        // Apply incremental parsing (<1ms target)
        self.parser.apply_edits(uri, &changes)?;

        // Re-verify affected breakpoints
        let affected_lines = calculate_affected_lines(&changes);
        for bp in self.get_breakpoints_in_range(uri, &affected_lines) {
            let verification = self.verify_breakpoint(uri, bp.line)?;
            if verification != bp.verification {
                self.send_breakpoint_event(bp.id, verification)?;
            }
        }

        Ok(())
    }
}
```

### 4.3 Workspace Navigation Integration

**Purpose**: Stack frame source resolution via dual indexing

**Integration Points**:
- `perl_parser::workspace_index::WorkspaceIndex::get_definition()`: Symbol lookup
- `perl_parser::workspace_index::Location`: Source location representation

**Usage Example**:
```rust
// crates/perl-dap/src/stack/resolver.rs
use perl_parser::workspace_index::WorkspaceIndex;

pub fn resolve_stack_frame(
    workspace: &WorkspaceIndex,
    package: &str,
    subroutine: &str
) -> Option<Location> {
    // Dual pattern matching (98% coverage)
    let qualified = format!("{}::{}", package, subroutine);

    workspace.get_definition(&qualified)
        .or_else(|| workspace.get_definition(subroutine))
}
```

---

## 5. Integration with perl-lsp

### 5.1 Protocol Separation

**Requirement**: Clean routing between LSP and DAP without performance degradation

**Design**: Separate binaries, optional integration

```rust
// crates/perl-lsp/src/main.rs (unchanged)
// LSP server runs independently, no DAP dependency

// crates/perl-dap/src/main.rs (new)
// DAP adapter runs as separate process
```

**Future Enhancement** (optional): Dual-protocol server
```rust
// Future: Combined LSP + DAP server
fn main() {
    let mode = std::env::var("DAP_MODE").unwrap_or_default();

    if mode == "dap" {
        run_dap_server();
    } else {
        run_lsp_server();
    }
}
```

### 5.2 Shared Infrastructure

**Reusable Components**:
- JSON-RPC message framing (`Content-Length` header parsing)
- Position mapping (UTF-16 ↔ UTF-8 conversion)
- Error handling patterns
- Logging infrastructure

**Integration Example**:
```rust
// Both crates can reuse common protocol handling
use perl_lsp::textdoc::{lsp_pos_to_byte, byte_to_lsp_pos, PosEnc};

// LSP server uses this for textDocument/* requests
let byte_offset = lsp_pos_to_byte(rope, lsp_position, PosEnc::Utf16)?;

// DAP adapter uses same infrastructure for breakpoint positions
let byte_offset = lsp_pos_to_byte(rope, dap_position, PosEnc::Utf16)?;
```

---

## 6. VS Code Extension Integration

### 6.1 Debugger Contribution

```json
// vscode-extension/package.json
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
              }
            }
          }
        },
        "configurationSnippets": [
          {
            "label": "Perl: Launch",
            "body": {
              "type": "perl",
              "request": "launch",
              "name": "Launch Perl Script",
              "program": "^\"\\${workspaceFolder}/\\${1:script.pl}\"",
              "args": [],
              "perlPath": "perl"
            }
          }
        ]
      },
      {
        "type": "perl-rs",
        "label": "Perl Debug (Native)",
        "program": "./bin/perl-dap",
        "runtime": null,
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
              },
              "env": {
                "type": "object",
                "description": "Environment variables"
              },
              "cwd": {
                "type": "string",
                "description": "Working directory"
              },
              "stopOnEntry": {
                "type": "boolean",
                "description": "Stop on entry",
                "default": false
              }
            }
          }
        }
      }
    ]
  }
}
```

### 6.2 Binary Management

```typescript
// vscode-extension/src/dapBinaryManager.ts
import * as path from 'path';
import * as fs from 'fs';
import * as https from 'https';

export class DapBinaryManager {
    private extensionPath: string;

    constructor(extensionPath: string) {
        this.extensionPath = extensionPath;
    }

    async ensureBinary(): Promise<string> {
        const platform = process.platform;
        const arch = process.arch;

        // Determine binary name
        const binaryName = this.getBinaryName(platform, arch);
        const binaryPath = path.join(this.extensionPath, 'bin', binaryName);

        // Check if binary exists
        if (fs.existsSync(binaryPath)) {
            return binaryPath;
        }

        // Download from GitHub Releases
        const version = '0.1.0';
        const downloadUrl = `https://github.com/EffortlessMetrics/perl-lsp/releases/download/v${version}/${binaryName}`;

        console.log(`Downloading ${binaryName} from ${downloadUrl}...`);
        await this.downloadFile(downloadUrl, binaryPath);

        // Make executable (Unix)
        if (platform !== 'win32') {
            fs.chmodSync(binaryPath, 0o755);
        }

        return binaryPath;
    }

    private getBinaryName(platform: string, arch: string): string {
        const platformMap: Record<string, string> = {
            'linux': 'linux',
            'darwin': 'darwin',
            'win32': 'win32',
        };

        const archMap: Record<string, string> = {
            'x64': 'x64',
            'arm64': 'arm64',
        };

        const ext = platform === 'win32' ? '.exe' : '';
        return `perl-dap-${platformMap[platform]}-${archMap[arch]}${ext}`;
    }

    private async downloadFile(url: string, dest: string): Promise<void> {
        return new Promise((resolve, reject) => {
            const file = fs.createWriteStream(dest);

            https.get(url, (response) => {
                if (response.statusCode !== 200) {
                    reject(new Error(`Download failed: ${response.statusCode}`));
                    return;
                }

                response.pipe(file);

                file.on('finish', () => {
                    file.close();
                    resolve();
                });
            }).on('error', (err) => {
                fs.unlinkSync(dest);
                reject(err);
            });
        });
    }
}
```

---

## 7. Testing Infrastructure

### 7.1 Test Organization

**Location**: `crates/perl-dap/tests/`

**Test Categories**:
1. **Protocol Compliance**: Golden transcript validation (AC13)
2. **Breakpoint Validation**: AST-based edge cases (AC7, AC13)
3. **Variable Rendering**: Truncation, expansion, Unicode (AC8, AC13)
4. **Performance**: Benchmarks with regression detection (AC14)
5. **Security**: Path traversal, safe eval, timeout (AC16)
6. **Cross-Platform**: Platform-specific behavior (AC12)

### 7.2 Golden Transcript Tests

```rust
// crates/perl-dap/tests/integration_tests.rs
use perl_dap::{DapServer, DapRequest, DapResponse};
use serde_json::json;

#[tokio::test] // AC13
async fn test_hello_world_golden_transcript() {
    let transcript = load_golden_transcript("hello.json");
    let server = DapServer::new(Default::default()).unwrap();

    for message in transcript.messages {
        if message.type_ == "request" {
            let response = server.handle_request(message.request).await.unwrap();
            assert_eq!(
                response,
                message.expected_response,
                "Transcript mismatch at seq {}",
                message.seq
            );
        }
    }
}

struct GoldenTranscript {
    messages: Vec<TranscriptMessage>,
}

struct TranscriptMessage {
    type_: String,
    seq: i64,
    request: DapRequest,
    expected_response: DapResponse,
}

fn load_golden_transcript(filename: &str) -> GoldenTranscript {
    let path = format!("tests/fixtures/golden/{}", filename);
    let content = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&content).unwrap()
}
```

### 7.3 Benchmarking Infrastructure

```rust
// crates/perl-dap/benches/dap_benchmarks.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use perl_dap::{DapServer, BreakpointManager};

fn benchmark_breakpoint_verification(c: &mut Criterion) {
    let fixtures = vec![
        ("small.pl", 100),
        ("medium.pl", 1000),
        ("large.pl", 10000),
    ];

    for (fixture, _lines) in fixtures {
        c.bench_function(&format!("verify_breakpoint_{}", fixture), |b| {
            let manager = BreakpointManager::new(/* parser */);
            b.iter(|| {
                manager.verify_breakpoint(black_box(fixture), black_box(42))
            });
        });
    }
}

fn benchmark_variable_rendering(c: &mut Criterion) {
    c.bench_function("render_large_scalar", |b| {
        let large_value = "x".repeat(10000);
        b.iter(|| {
            render_variable_value(black_box(&large_value), /* rope */)
        });
    });
}

criterion_group!(benches, benchmark_breakpoint_verification, benchmark_variable_rendering);
criterion_main!(benches);
```

---

## 8. Deployment Strategy

### 8.1 Binary Distribution

**Platforms**: 6 targets (Linux/macOS/Windows x86_64/aarch64)

**GitHub Actions**:
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

### 8.2 CPAN Publication

**Devel::TSPerlDAP**:
```bash
# Publish to CPAN
cd Devel-TSPerlDAP
perl Makefile.PL
make test
make dist
cpan-upload Devel-TSPerlDAP-0.1.0.tar.gz
```

**Bundled Fallback**:
- Extension bundles `Devel/TSPerlDAP.pm` in `resources/perl-shim/`
- Auto-install via `cpanm` on first use
- Fallback to bundled version if CPAN install fails

---

## 9. Success Metrics

### 9.1 Build Metrics

| Metric | Target | Validation |
|--------|--------|-----------|
| Clean compilation | Zero warnings | `cargo build -p perl-dap --release` |
| Cross-platform builds | 6 platforms | GitHub Actions CI |
| Binary size | <5MB per platform | Binary size validation |

### 9.2 Test Metrics

| Metric | Target | Validation |
|--------|--------|-----------|
| Test coverage (adapter) | >95% | `cargo tarpaulin -p perl-dap` |
| Test coverage (shim) | >80% | `cover -test` |
| Integration tests | >95% pass rate | `cargo test -p perl-dap --test integration_tests` |

### 9.3 Performance Metrics

| Metric | Target | Validation |
|--------|--------|-----------|
| Breakpoint verification | <50ms | `cargo bench -p perl-dap -- verify_breakpoint` |
| Variable rendering | <200ms initial | `cargo bench -p perl-dap -- render_variable` |
| Memory overhead | <1MB adapter | Memory profiling tests |

---

## 10. References

- [DAP Implementation Specification](DAP_IMPLEMENTATION_SPECIFICATION.md): Primary technical specification
- [DAP Protocol Schema](DAP_PROTOCOL_SCHEMA.md): JSON-RPC message schemas
- [DAP Security Specification](DAP_SECURITY_SPECIFICATION.md): Security requirements
- [LSP Implementation Guide](LSP_IMPLEMENTATION_GUIDE.md): LSP server architecture patterns
- [Crate Architecture Guide](CRATE_ARCHITECTURE_GUIDE.md): Existing workspace structure

---

**End of DAP Crate Architecture Specification**
