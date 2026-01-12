//! Language feature handlers
//!
//! This module organizes LSP language features into focused submodules:
//! - hover: Hover information and signature help
//! - completion: Code completion with cancellation support
//! - navigation: Go-to-definition, declaration, type definition, implementation
//! - references: Find references and document highlights
//! - symbols: Document symbols and folding ranges
//! - formatting: Document and range formatting
//! - code_actions: Code actions and quick fixes
//! - rename: Symbol renaming (single file and workspace)
//! - hierarchy: Type hierarchy and call hierarchy
//! - semantic_tokens: Semantic tokens for syntax highlighting
//! - colors: Document color detection and presentation
//! - virtual_content: Virtual document content for perldoc:// URIs
//! - misc: Inlay hints, document links, code lens, and other features

mod code_actions;
mod colors;
mod completion;
mod formatting;
mod hierarchy;
mod hover;
mod misc;
mod navigation;
mod references;
mod rename;
mod semantic_tokens;
mod symbols;
mod virtual_content;
