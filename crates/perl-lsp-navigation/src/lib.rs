//! LSP navigation providers for Perl
//!
//! This crate provides navigation functionality for Perl code.
//!
//! ## Features
//!
//! - Go to definition
//! - Find references
//! - Go to implementation
//! - Go to type definition
//! - Type hierarchy
//! - Call hierarchy
//! - Document links
//!
//! ## Usage
//!
//! ```rust,ignore
//! use perl_lsp_navigation::{TypeHierarchyProvider, WorkspaceSymbolsProvider};
//!
//! let type_hierarchy = TypeHierarchyProvider::new(workspace_index);
//! let workspace_symbols = WorkspaceSymbolsProvider::new(workspace_index);
//! ```

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

// Declare modules
mod references;
mod type_definition;

// Re-export key types and functions
pub use self::references::find_references_single_file;
pub use self::type_definition::TypeDefinitionProvider;
pub use perl_lsp_rs_core::providers::document_links::compute_links;
pub use perl_lsp_rs_core::providers::type_hierarchy::{
    TypeHierarchyItem, TypeHierarchyProvider, TypeHierarchySymbolKind,
};
pub use perl_lsp_rs_core::providers::workspace_symbols::{
    WorkspaceSymbol, WorkspaceSymbolsProvider,
};

// Re-export Location type for convenience
pub use lsp_types::Location;
