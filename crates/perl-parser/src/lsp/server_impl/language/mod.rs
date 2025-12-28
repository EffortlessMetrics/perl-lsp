//! Language feature handlers
//!
//! This module organizes LSP language features into focused submodules:
//! - hover: Hover information and signature help
//! - completion: Code completion with cancellation support
//! - navigation: Go-to-definition, declaration, type definition, implementation
//! - (future) references: Find references and document highlights
//! - (future) symbols: Document and workspace symbols

mod completion;
mod hover;
mod navigation;
