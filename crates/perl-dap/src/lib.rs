//! Debug Adapter Protocol server for Perl
//!
//! This crate provides a production-grade DAP adapter for debugging Perl code.
//! It integrates with the perl-parser crate for AST-based breakpoint validation
//! and leverages existing LSP infrastructure for position mapping and workspace navigation.
//!
//! # Architecture
//!
//! The DAP adapter follows a phased implementation approach:
//!
//! - **Phase 1 (AC1-AC4)**: Bridge to Perl::LanguageServer DAP - **IMPLEMENTED**
//!   - [`BridgeAdapter`]: Proxy VS Code ↔ Perl::LanguageServer DAP messages
//!   - [`LaunchConfiguration`]: Launch configuration for starting new Perl processes
//!   - [`AttachConfiguration`]: Attach configuration for TCP connections
//!   - [`platform`]: Cross-platform utilities for path resolution and environment setup
//!
//! - **Phase 2 (AC5-AC12)**: Native Rust adapter + Perl shim - **TODO**
//!   - DAP protocol types
//!   - Session management
//!   - Breakpoint manager with AST validation
//!   - Variable renderer with lazy expansion
//!   - Stack trace provider
//!   - Control flow handlers
//!   - Safe evaluation
//!
//! - **Phase 3 (AC13-AC19)**: Production hardening - **TODO**
//!   - Security validation
//!   - Performance optimization
//!   - Packaging and distribution
//!
//! # Examples
//!
//! ## Using the Bridge Adapter (Phase 1)
//!
//! ```no_run
//! use perl_dap::BridgeAdapter;
//!
//! # fn main() -> anyhow::Result<()> {
//! let mut adapter = BridgeAdapter::new();
//! adapter.spawn_pls_dap()?;
//! adapter.proxy_messages()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Creating Launch Configuration
//!
//! ```no_run
//! use perl_dap::LaunchConfiguration;
//! use std::path::PathBuf;
//!
//! let mut config = LaunchConfiguration {
//!     program: PathBuf::from("script.pl"),
//!     args: vec!["--verbose".to_string()],
//!     cwd: Some(PathBuf::from("/workspace")),
//!     env: std::collections::HashMap::new(),
//!     perl_path: None,
//!     include_paths: vec![PathBuf::from("lib")],
//! };
//!
//! config.validate().expect("Valid configuration");
//! ```
//!
//! ## Generating launch.json Snippets
//!
//! ```
//! use perl_dap::{create_launch_json_snippet, create_attach_json_snippet};
//!
//! let launch_snippet = create_launch_json_snippet();
//! let attach_snippet = create_attach_json_snippet();
//! ```
//!
//! # Test-Driven Development Approach
//!
//! This scaffolding supports 19 acceptance criteria (AC1-AC19) from Issue #207.
//! All tests are tagged with `// AC:ID` for traceability to specifications.

// Phase 1 modules (AC1-AC4) - IMPLEMENTED
pub mod bridge_adapter;
pub mod configuration;
/// Debug Adapter Protocol (DAP) implementation for Perl debugging.
pub mod debug_adapter;
pub mod platform;

// Phase 2 modules (AC5-AC12) - IN PROGRESS
pub mod breakpoints;
pub mod dispatcher; // AC5: Message dispatcher
pub mod protocol; // AC5: DAP protocol types // AC7: Breakpoint manager

// Phase 2 modules (AC5-AC12) - TODO
// TODO: Implement session management (AC5)
// TODO: Implement AST-based breakpoint validation (AC7)
// TODO: Implement variable renderer with lazy expansion (AC8)
// TODO: Implement stack trace provider (AC8)
// TODO: Implement control flow handlers (AC9)
// TODO: Implement safe evaluation (AC10)

// Phase 3 modules (AC13-AC19) - TODO
// TODO: Implement security validation (AC16)

// Re-export Phase 1 public types
pub use bridge_adapter::BridgeAdapter;
pub use configuration::{
    AttachConfiguration, LaunchConfiguration, create_attach_json_snippet,
    create_launch_json_snippet,
};
pub use debug_adapter::{DapMessage, DebugAdapter};

// Re-export Phase 2 public types
pub use breakpoints::{BreakpointRecord, BreakpointStore};
pub use dispatcher::{DapDispatcher, DispatchResult};
pub use protocol::{
    Breakpoint, Capabilities, Event, InitializeRequestArguments, LaunchRequestArguments, Request,
    Response, SetBreakpointsArguments, SetBreakpointsResponseBody, Source, SourceBreakpoint,
};

/// DAP server configuration (Phase 2 placeholder)
///
/// This configuration structure will be enhanced in Phase 2 (AC5-AC12) to support:
/// - Session management settings
/// - Breakpoint validation options
/// - Performance tuning parameters
/// - Security constraints
pub struct DapConfig {
    /// Logging level for DAP operations
    pub log_level: String,
}

/// DAP server (Phase 2 placeholder)
///
/// This server will be fully implemented in Phase 2 (AC5-AC12) with:
/// - Native Rust DAP protocol implementation
/// - AST-based breakpoint validation
/// - Lazy variable expansion
/// - Safe evaluation context
pub struct DapServer {
    /// Server configuration
    pub config: DapConfig,
    /// The underlying debug adapter
    adapter: DebugAdapter,
}

impl DapServer {
    /// Create a new DAP server instance
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration
    ///
    /// # Errors
    ///
    /// Currently always succeeds. Phase 2 will add validation and initialization errors.
    pub fn new(config: DapConfig) -> anyhow::Result<Self> {
        Ok(Self {
            config,
            adapter: DebugAdapter::new(),
        })
    }

    /// Run the DAP server
    ///
    /// This method starts the stdio transport loop and blocks until the session ends.
    pub fn run(&mut self) -> anyhow::Result<()> {
        self.adapter.run().map_err(|e| anyhow::anyhow!(e))
    }
}
