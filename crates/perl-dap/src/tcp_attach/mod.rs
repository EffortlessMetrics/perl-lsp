//! TCP Attach Module for DAP
//!
//! This module provides TCP-based attachment to running Perl debugger processes.
//! It enables connecting to Perl::LanguageServer DAP instances via TCP sockets.
//!
//! # Architecture
//!
//! ```text
//! VS Code <-> Native DAP Adapter <-> TCP Socket <-> Perl::LanguageServer DAP
//! ```
//!
//! # Features
//!
//! - TCP socket connection with configurable timeout
//! - Bidirectional message proxying
//! - Connection state management
//! - Error recovery and reconnection
//! - Cross-platform support (Windows, macOS, Linux)

mod config;
mod event;
mod reader;
mod session;

pub use config::TcpAttachConfig;
pub use event::DapEvent;
pub use session::TcpAttachSession;
