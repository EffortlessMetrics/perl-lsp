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

// Declare modules
mod references;
mod type_definition;

// Re-export key types and functions
pub use self::references::find_references_single_file;
pub use self::type_definition::TypeDefinitionProvider;
pub use crate::providers::document_links::compute_links;
pub use crate::providers::type_hierarchy::{
    TypeHierarchyItem, TypeHierarchyProvider, TypeHierarchySymbolKind,
};
pub use crate::providers::workspace_symbols::{WorkspaceSymbol, WorkspaceSymbolsProvider};

// Re-export Location type for convenience
pub use lsp_types::Location;

/// Navigation provider wrapper for LSP navigation features.
///
/// Provides access to all navigation functionality (references, type definitions,
/// document links, type hierarchy, workspace symbols).
pub struct NavigationProvider;

impl NavigationProvider {
    /// Create a new navigation provider.
    pub fn new() -> Self {
        NavigationProvider
    }
}

impl Default for NavigationProvider {
    fn default() -> Self {
        Self::new()
    }
}
