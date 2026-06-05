//! Debug Adapter Protocol Implementation for Perl
//!
//! This crate provides a production-grade Debug Adapter Protocol (DAP) server for Perl,
//! enabling debugging support in VSCode, Neovim, Emacs, and other DAP-compatible editors.
//!
//! The adapter integrates with `perl_parser` for AST-based breakpoint validation and
//! leverages existing LSP infrastructure for position mapping and workspace navigation.
//!
//! # Features
//!
//! - **Native Runtime**: Current launch/attach implementation for Perl debugging sessions
//! - **Launch Debugging**: Start and debug Perl processes with full control
//! - **Attach Debugging**: Attach to running Perl processes via TCP
//! - **Legacy Bridge Mode**: Compatibility proxy for Perl::LanguageServer installations
//! - **AST-Based Validation**: Breakpoint validation using parsed syntax trees
//! - **Cross-Platform**: Windows, macOS, and Linux support with path normalization
//! - **Configuration Snippets**: VSCode launch.json generation
//!
//! # Quick Start
//!
//! ## Native and Bridge Modes
//!
//! Native launch and attach are the current default runtime paths for new sessions.
//! The legacy bridge adapter remains available when an installation still needs to
//! proxy DAP messages to Perl::LanguageServer:
//!
//! ```no_run
//! use perl_dap::BridgeAdapter;
//!
//! # #[tokio::main]
//! # async fn main() -> anyhow::Result<()> {
//! let mut adapter = BridgeAdapter::new();
//!
//! // Start Perl::LanguageServer DAP backend
//! adapter.spawn_pls_dap().await?;
//!
//! // Proxy messages between VSCode and PLS
//! adapter.proxy_messages().await?;
//!
//! // Cleanup on shutdown
//! adapter.shutdown().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Launch Configuration
//!
//! Create debugging configurations for launching Perl scripts:
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use perl_dap::LaunchConfiguration;
//! use std::path::PathBuf;
//! use std::collections::HashMap;
//!
//! let config = LaunchConfiguration {
//!     program: PathBuf::from("script.pl"),
//!     args: vec!["--verbose".to_string()],
//!     cwd: Some(PathBuf::from("/workspace")),
//!     env: HashMap::new(),
//!     perl_path: None,
//!     include_paths: vec![PathBuf::from("lib")],
//! };
//!
//! // Validate configuration before launching
//! config.validate()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! # }
//! ```
//!
//! ## Attach Configuration
//!
//! Attach to running Perl processes via TCP:
//!
//! ```rust
//! use perl_dap::AttachConfiguration;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = AttachConfiguration {
//!     host: "localhost".to_string(),
//!     port: 13603,
//!     timeout_ms: Some(5000),
//!     stop_on_entry: None,
//! };
//!
//! config.validate()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## VSCode Integration
//!
//! Generate launch.json snippets for VSCode:
//!
//! ```rust
//! use perl_dap::{create_launch_json_snippet, create_attach_json_snippet};
//!
//! // Generate launch configuration snippet
//! let launch_snippet = create_launch_json_snippet();
//! println!("{}", launch_snippet);
//!
//! // Generate attach configuration snippet
//! let attach_snippet = create_attach_json_snippet();
//! println!("{}", attach_snippet);
//! ```
//!
//! # Architecture
//!
//! `perl-dap` exposes a dual architecture that supports both current runtime use
//! and compatibility migration:
//!
//! - **Native runtime (`DapServer` + `DebugAdapter`)** handles launch/attach
//!   flows, request dispatch, breakpoint/state management, and variable/evaluate
//!   inspection paths.
//! - **Legacy bridge (`BridgeAdapter`)** proxies traffic to
//!   `Perl::LanguageServer` for compatibility when teams are still migrating.
//! - **Shared foundations** (`protocol`, `dispatcher`, `breakpoints`,
//!   `platform`) provide message contracts, routing, validation, and
//!   cross-platform process setup.
//!
//! This keeps one crate boundary for editor integrations while allowing
//! per-session selection between native execution and bridge fallback behavior.
//!
//! # Protocol Support
//!
//! The adapter implements DAP 1.51+ specification features:
//!
//! ## Initialization
//!
//! - `initialize` - Capability negotiation
//! - `attach` / `launch` - Debug session start
//! - `configurationDone` - Initialization complete
//! - `disconnect` - Session termination
//!
//! ## Breakpoints
//!
//! - `setBreakpoints` - Set breakpoints with AST validation
//! - `setFunctionBreakpoints` - Break on function entry
//! - `setExceptionBreakpoints` - Break on exceptions
//!
//! ## Execution Control
//!
//! - `continue` - Resume execution
//! - `next` - Step over
//! - `stepIn` - Step into
//! - `stepOut` - Step out
//! - `pause` - Pause execution
//!
//! ## Inspection
//!
//! - `threads` - List active threads
//! - `stackTrace` - Get call stack
//! - `scopes` - Get variable scopes
//! - `variables` - Inspect variables with lazy loading
//! - `evaluate` - Evaluate expressions in context
//!
//! # Breakpoint Validation
//!
//! The [`breakpoints`] module provides AST-based validation:
//!
//! ```rust,ignore
//! use perl_dap::{BreakpointStore, SourceBreakpoint};
//! use perl_parser::Parser;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let code = "sub foo {\n    my $x = 1;\n    return $x;\n}";
//! let mut parser = Parser::new(code);
//! let ast = parser.parse()?;
//!
//! let mut store = BreakpointStore::new();
//! let bp = SourceBreakpoint {
//!     line: 2,
//!     column: None,
//!     condition: None,
//!     hit_condition: None,
//!     log_message: None,
//! };
//!
//! // Validate breakpoint is on executable line
//! let validated = store.add_breakpoint("script.pl", bp, &ast);
//! # Ok(())
//! # }
//! ```
//!
//! # Platform Support
//!
//! The [`platform`] module handles cross-platform concerns:
//!
//! - **Path Resolution**: Normalize paths for Windows/Unix
//! - **Perl Discovery**: Find Perl interpreter in PATH
//! - **Environment Setup**: Configure @INC and environment variables
//! - **Process Spawning**: Launch Perl processes with proper stdio handling
//!
//! ```rust,ignore
//! use perl_dap::platform::{find_perl, normalize_path};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let perl = find_perl().unwrap_or_else(|| std::path::PathBuf::from("/usr/bin/perl"));
//! let normalized = normalize_path("/workspace/lib/Foo.pm");
//! # Ok(())
//! # }
//! ```
//!
//! # Configuration Examples
//!
//! ## VSCode launch.json
//!
//! ```json
//! {
//!   "version": "0.2.0",
//!   "configurations": [
//!     {
//!       "type": "perl",
//!       "request": "launch",
//!       "name": "Debug Perl Script",
//!       "program": "${workspaceFolder}/script.pl",
//!       "args": ["--verbose"],
//!       "cwd": "${workspaceFolder}",
//!       "includePaths": ["lib", "local/lib/perl5"]
//!     },
//!     {
//!       "type": "perl",
//!       "request": "attach",
//!       "name": "Attach to Perl",
//!       "host": "localhost",
//!       "port": 13603
//!     }
//!   ]
//! }
//! ```
//!
//! ## Programmatic Configuration
//!
//! ```rust
//! use perl_dap::LaunchConfiguration;
//! use std::path::PathBuf;
//! use std::collections::HashMap;
//!
//! let mut env = HashMap::new();
//! env.insert("PERL5LIB".to_string(), "lib:local/lib/perl5".to_string());
//!
//! let config = LaunchConfiguration {
//!     program: PathBuf::from("${workspaceFolder}/script.pl"),
//!     args: vec!["--debug".to_string()],
//!     cwd: Some(PathBuf::from("${workspaceFolder}")),
//!     env,
//!     perl_path: Some(PathBuf::from("/usr/bin/perl")),
//!     include_paths: vec![
//!         PathBuf::from("lib"),
//!         PathBuf::from("local/lib/perl5"),
//!     ],
//! };
//! ```
//!
//! # Testing
//!
//! The adapter includes comprehensive test coverage:
//!
//! ```bash
//! # Run all DAP tests
//! cargo test -p perl-dap
//!
//! # Test specific phase
//! cargo test -p perl-dap bridge_adapter
//!
//! # Integration tests
//! cargo test -p perl-dap --test integration_tests
//! ```
//!
//! All tests are tagged with acceptance criteria (AC1-AC19) for traceability:
//!
//! ```rust,ignore
//! #[test]
//! fn test_launch_config_validation() {
//!     // AC:2 - Launch configuration validation
//!     let config = LaunchConfiguration { /* ... */ };
//!     assert!(config.validate().is_ok());
//! }
//! ```
//!
//! # Security Considerations
//!
//! - **Command Injection**: All paths and arguments are sanitized
//! - **Arbitrary Execution**: Evaluation restricted to debug context
//! - **Resource Limits**: Memory and time budgets for operations
//! - **Path Validation**: Prevent directory traversal and unauthorized access
//!
//! # Error Handling
//!
//! The adapter uses `anyhow::Result` for comprehensive error reporting:
//!
//! ```rust,ignore
//! use perl_dap::{BridgeAdapter, DapError};
//!
//! async fn run_adapter() -> anyhow::Result<()> {
//!     let mut adapter = BridgeAdapter::new();
//!     adapter.spawn_pls_dap().await?;
//!     adapter.proxy_messages().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Integration with perl-parser
//!
//! The DAP adapter leverages `perl_parser` for:
//!
//! - **Breakpoint Validation**: Verify breakpoints on executable lines
//! - **Variable Inspection**: Type-aware variable rendering
//! - **Expression Evaluation**: Parse and evaluate debug expressions
//! - **Source Mapping**: Map positions between editor and runtime
//!
//! # Migration Path
//!
//! Phase 1 provides immediate value via bridging, while Phase 2 and 3 will gradually
//! migrate functionality to native Rust implementation for better performance and
//! integration.
//!
//! Users can start with bridge mode today and transparently upgrade to native mode
//! when Phase 2 is complete.
//!
//! # Related Crates
//!
//! - `perl_parser`: Parsing engine and AST analysis
//! - `perl_lsp`: Language Server Protocol implementation
//! - `perl_lexer`: Context-aware Perl tokenizer
//!
//! # Documentation
//!
//! - **DAP Specification**: [Debug Adapter Protocol](https://microsoft.github.io/debug-adapter-protocol/)
//! - **Implementation Guide**: See `docs/DAP_IMPLEMENTATION_GUIDE.md`
//! - **Issue Tracking**: See GitHub issue #207 for acceptance criteria
//!
//! # Test-Driven Development
//!
//! This crate follows TDD principles with acceptance criteria from Issue #207.
//! All tests are tagged with `// AC:ID` comments for traceability to specifications.

#![warn(missing_docs)]
#![cfg_attr(test, allow(clippy::print_stderr, clippy::print_stdout))]

// Phase 1 modules (AC1-AC4) - IMPLEMENTED
/// Bridge adapter for communicating with Perl::LanguageServer's DAP implementation.
pub mod bridge_adapter;
/// Launch and attach configuration structures for DAP debugging sessions.
pub mod configuration;
/// Debug Adapter Protocol (DAP) implementation for Perl debugging.
pub mod debug_adapter;
/// DAP feature catalog and capability gating helpers.
pub mod feature_catalog;

// Wave H collapsed modules (in DAG order — command_args before platform, platform before shell, value before variables)
/// Explicit public API re-exports from all collapsed satellite modules.
pub mod api;
/// AST-based breakpoint validation (from perl-dap-breakpoint).
pub mod breakpoint;
/// Platform-aware shell argument formatting (from perl-dap-command-args).
pub mod command_args;
/// DAP launch and attach configuration types (from perl-dap-config).
pub mod config;
/// Safe expression evaluation validation (from perl-dap-eval).
pub mod eval;
/// Cross-platform utilities for Perl path resolution and environment setup (from perl-dap-platform).
pub mod platform;
/// Security validation and hardening (from perl-dap-security).
pub mod security;
/// Shell-specific helpers for Perl DAP process launch (from perl-dap-shell).
pub mod shell;
/// Stack trace parsing and frame classification (from perl-dap-stack).
pub mod stack;
/// Shared DAP session model types (from perl-dap-types).
pub mod types;
/// Shared Perl value model for DAP parser and renderer (from perl-dap-value).
pub mod value;
/// Variable parsing and rendering for Perl DAP (from perl-dap-variables).
pub mod variables;

// Phase 2 modules (AC5-AC12) - IN PROGRESS
/// Breakpoint storage and management for the DAP adapter.
pub mod breakpoints;
/// Message dispatcher for routing incoming DAP requests to handlers.
pub mod dispatcher;
/// Inline value extraction for DAP `inlineValues` requests.
pub mod inline_values;
/// DAP protocol types following the JSON-RPC 2.0 message format.
pub mod protocol;
/// DAP server lifecycle, configuration, and operating mode.
pub mod server;
/// TCP-based attachment to running Perl debugger processes.
pub mod tcp_attach;

// Phase 2 modules (AC5-AC12) - Tracked in GitHub issues
// See #449: Implement session management (AC5)
// See #450: Implement AST-based breakpoint validation (AC7)
// See #452: Implement variable renderer with lazy expansion (AC8)
// See #453: Implement stack trace provider (AC8)
// See #454: Implement control flow handlers (AC9)
// See #455: Implement safe evaluation (AC10)

// Re-export Phase 1 public types
pub use bridge_adapter::{BridgeAdapter, DapBridgeEnvConfig};
pub use configuration::{
    AttachConfiguration, LaunchConfiguration, create_attach_json_snippet,
    create_launch_json_snippet,
};
pub use debug_adapter::{DapMessage, DebugAdapter};
pub use server::{DapConfig, DapMode, DapServer};

// Re-export Phase 2 public types
pub use breakpoints::{BreakpointRecord, BreakpointStore};
#[allow(deprecated)]
pub use dispatcher::{DapDispatcher, DispatchResult};
pub use protocol::{
    AttachRequestArguments, Breakpoint, BreakpointLocation, BreakpointLocationsArguments,
    BreakpointLocationsResponseBody, CancelArguments, Capabilities, CompletionItem,
    CompletionsArguments, CompletionsResponseBody, ContinueArguments, ContinueResponseBody,
    DataBreakpoint, DataBreakpointInfoArguments, DataBreakpointInfoResponseBody,
    DisconnectArguments, EvaluateArguments, EvaluateResponseBody, Event, ExceptionBreakpointFilter,
    ExceptionDetails, ExceptionFilterOption, ExceptionInfoArguments, ExceptionInfoResponseBody,
    FunctionBreakpoint, GotoArguments, GotoTarget, GotoTargetsArguments, GotoTargetsResponseBody,
    InitializeRequestArguments, LaunchRequestArguments, LoadedSourcesResponseBody, Module,
    ModulesArguments, ModulesResponseBody, NextArguments, PauseArguments, ProtocolStackFrame,
    ProtocolVariable, Request, Response, RestartArguments, RestartFrameArguments, Scope,
    ScopesArguments, ScopesResponseBody, SetBreakpointsArguments, SetBreakpointsResponseBody,
    SetDataBreakpointsArguments, SetDataBreakpointsResponseBody, SetExceptionBreakpointsArguments,
    SetExpressionArguments, SetExpressionResponseBody, SetFunctionBreakpointsArguments,
    SetVariableArguments, SetVariableResponseBody, Source, SourceArguments, SourceBreakpoint,
    SourceResponseBody, StackTraceArguments, StackTraceResponseBody, StepInArguments, StepInTarget,
    StepInTargetsArguments, StepInTargetsResponseBody, StepOutArguments, TerminateArguments,
    TerminateThreadsArguments, Thread, ThreadsResponseBody, VariablesArguments,
    VariablesResponseBody,
};
