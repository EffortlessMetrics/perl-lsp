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
//! - **Bridge Mode**: Proxy to existing Perl::LanguageServer DAP implementation
//! - **Launch Debugging**: Start and debug Perl processes with full control
//! - **Attach Debugging**: Attach to running Perl processes via TCP
//! - **AST-Based Validation**: Breakpoint validation using parsed syntax trees
//! - **Cross-Platform**: Windows, macOS, and Linux support with path normalization
//! - **Configuration Snippets**: VSCode launch.json generation
//!
//! # Quick Start
//!
//! ## Bridge Mode (Phase 1 - Implemented)
//!
//! The bridge adapter proxies DAP messages to Perl::LanguageServer:
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
//! The DAP adapter follows a phased implementation approach:
//!
//! ## Phase 1: Bridge Adapter (Implemented)
//!
//! **Acceptance Criteria: AC1-AC4**
//!
//! - **[`BridgeAdapter`]**: Message proxy between VSCode and Perl::LanguageServer
//! - **[`LaunchConfiguration`]**: Launch debugging configuration and validation
//! - **[`AttachConfiguration`]**: Attach debugging configuration for TCP connections
//! - **[`platform`]**: Cross-platform path resolution and environment setup
//!
//! Phase 1 provides immediate debugging support by bridging to the mature
//! Perl::LanguageServer implementation while the native adapter is developed.
//!
//! ## Phase 2: Native Adapter (Planned)
//!
//! **Acceptance Criteria: AC5-AC12**
//!
//! - **[`protocol`]**: DAP protocol types and message definitions
//! - **[`dispatcher`]**: Request routing and method dispatch
//! - **[`breakpoints`]**: Breakpoint management with AST validation
//! - **Session Management**: Debug session lifecycle and state tracking
//! - **Variable Renderer**: Lazy variable expansion for complex data structures
//! - **Stack Trace Provider**: Call stack navigation with source mapping
//! - **Control Flow**: Step, continue, pause, and breakpoint control
//! - **Safe Evaluation**: Expression evaluation in debug context
//!
//! Phase 2 will provide a native Rust DAP implementation with tighter integration
//! to `perl_parser` for enhanced validation and performance.
//!
//! ## Phase 3: Production Hardening (Planned)
//!
//! **Acceptance Criteria: AC13-AC19**
//!
//! - **Security Validation**: Input sanitization and command injection prevention
//! - **Performance Optimization**: Efficient variable inspection and stepping
//! - **Packaging**: Distribution via cargo, VSCode marketplace, and package managers
//! - **Documentation**: Comprehensive usage guides and troubleshooting
//! - **Testing**: End-to-end integration tests with real debugging scenarios
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

// Phase 1 modules (AC1-AC4) - IMPLEMENTED
pub mod bridge_adapter;
pub mod configuration;
/// Debug Adapter Protocol (DAP) implementation for Perl debugging.
pub mod debug_adapter;
pub mod feature_catalog;
pub mod platform;

// Phase 2 modules (AC5-AC12) - IN PROGRESS
pub mod breakpoints;
pub mod dispatcher; // AC5: Message dispatcher
pub mod inline_values; // Inline value extraction for debug sessions
pub mod protocol; // AC5: DAP protocol types // AC7: Breakpoint manager
pub mod tcp_attach; // TCP attach functionality for connecting to running Perl debugger

// Phase 2 modules (AC5-AC12) - Tracked in GitHub issues
// See #449: Implement session management (AC5)
// See #450: Implement AST-based breakpoint validation (AC7)
// See #452: Implement variable renderer with lazy expansion (AC8)
// See #453: Implement stack trace provider (AC8)
// See #454: Implement control flow handlers (AC9)
// See #455: Implement safe evaluation (AC10)

// Phase 3 modules (AC13-AC19) - Tracked in GitHub issues
pub mod security; // #358: Security validation and hardening (AC16)

// Re-export Phase 1 public types
pub use bridge_adapter::BridgeAdapter;
pub use configuration::{
    AttachConfiguration, LaunchConfiguration, create_attach_json_snippet,
    create_launch_json_snippet,
};
pub use debug_adapter::{DapMessage, DebugAdapter};

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

/// Debug adapter operating mode
///
/// Controls whether the DAP server uses its native `perl -d` adapter
/// or proxies to Perl::LanguageServer's DAP implementation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum DapMode {
    /// Native adapter using `perl -d` directly
    #[default]
    Native,
    /// Bridge adapter proxying to Perl::LanguageServer
    Bridge,
}

/// DAP server configuration
///
/// Controls the operating mode, logging, and workspace context for the DAP server.
pub struct DapConfig {
    /// Logging level for DAP operations
    pub log_level: String,
    /// Operating mode (native or bridge)
    pub mode: DapMode,
    /// Workspace root directory
    pub workspace_root: Option<std::path::PathBuf>,
}

/// DAP server
///
/// Supports two operating modes:
/// - **Native** (default): Uses the built-in [`DebugAdapter`] with `perl -d`
/// - **Bridge**: Proxies DAP messages to Perl::LanguageServer via [`BridgeAdapter`]
pub struct DapServer {
    /// Server configuration
    pub config: DapConfig,
    /// The underlying debug adapter (used in Native mode)
    adapter: DebugAdapter,
}

impl DapServer {
    /// Create a new DAP server instance
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration including operating mode
    ///
    /// # Errors
    ///
    /// Currently always succeeds. Phase 2 will add validation and initialization errors.
    pub fn new(config: DapConfig) -> anyhow::Result<Self> {
        Ok(Self { config, adapter: DebugAdapter::new() })
    }

    /// Run the DAP server
    ///
    /// Dispatches to the appropriate transport based on the configured [`DapMode`]:
    /// - [`DapMode::Native`]: Starts the stdio transport loop via [`DebugAdapter::run`]
    /// - [`DapMode::Bridge`]: Spawns Perl::LanguageServer and proxies DAP messages
    ///   via [`BridgeAdapter`] using a tokio async runtime
    pub fn run(&mut self) -> anyhow::Result<()> {
        match self.config.mode {
            DapMode::Native => self.adapter.run().map_err(Into::into),
            DapMode::Bridge => {
                tracing::info!("Starting DAP server in bridge mode");
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async {
                    let mut bridge = BridgeAdapter::new();
                    bridge.spawn_pls_dap().await?;
                    bridge.proxy_messages().await?;
                    bridge.shutdown().await?;
                    Ok(())
                })
            }
        }
    }

    /// Run the DAP server over TCP socket transport.
    ///
    /// This binds to `127.0.0.1:<port>` and serves one DAP client session.
    ///
    /// # Errors
    ///
    /// Returns an error if bridge mode is selected, since socket transport
    /// is only supported for native mode.
    pub fn run_socket(&mut self, port: u16) -> anyhow::Result<()> {
        if self.config.mode == DapMode::Bridge {
            anyhow::bail!("Socket transport is not supported in bridge mode");
        }
        self.adapter.run_socket(port).map_err(Into::into)
    }
}
