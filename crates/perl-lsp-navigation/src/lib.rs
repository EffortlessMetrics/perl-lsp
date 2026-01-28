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
//! ```rust
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
mod document_links;
mod references;
mod type_definition;
mod type_hierarchy;
mod workspace_symbols;

// Re-export key types and functions
pub use self::document_links::compute_links;
pub use self::references::find_references_single_file;
pub use self::type_definition::TypeDefinitionProvider;
pub use self::type_hierarchy::{TypeHierarchyItem, TypeHierarchyProvider, TypeHierarchySymbolKind};
pub use self::workspace_symbols::{WorkspaceSymbol, WorkspaceSymbolsProvider};

// Re-export Location type for convenience
pub use lsp_types::Location;
