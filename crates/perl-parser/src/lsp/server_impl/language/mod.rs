//! Language feature handlers
//!
//! This module organizes LSP language features into focused submodules:
//! - hover: Hover information and signature help
//! - completion: Code completion with cancellation support
//! - (future) navigation: Go-to-definition, references, etc.
//! - (future) symbols: Document and workspace symbols

mod completion;
mod hover;
