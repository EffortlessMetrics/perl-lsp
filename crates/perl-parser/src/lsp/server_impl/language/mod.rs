//! Language feature handlers
//!
//! This module organizes LSP language features into focused submodules:
//! - hover: Hover information and signature help
//! - completion: Code completion with cancellation support
//! - navigation: Go-to-definition, declaration, type definition, implementation
//! - references: Find references and document highlights
//! - symbols: Document symbols and folding ranges

mod completion;
mod hover;
mod navigation;
mod references;
mod symbols;
