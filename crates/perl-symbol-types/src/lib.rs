//! Unified Perl symbol taxonomy for LSP tooling.
//!
//! This crate provides a single, authoritative definition of Perl symbol kinds
//! used across the parser, semantic analyzer, workspace index, and LSP providers.
//!
//! # Design Goals
//!
//! - **Single source of truth**: All symbol classification flows through this crate
//! - **Perl semantics**: Distinguishes variables by sigil type (scalar/array/hash)
//! - **LSP compatibility**: Direct mapping to LSP protocol symbol kinds
//! - **Zero-cost abstractions**: Enum variants are `Copy` types with inline methods

use serde::{Deserialize, Serialize};

/// Variable sigil classification for Perl's three primary container types.
///
/// Perl distinguishes variables by their sigil prefix, which determines
/// the container type and access semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VarKind {
    /// Scalar variable (`$foo`) - holds a single value
    Scalar,
    /// Array variable (`@foo`) - holds an ordered list
    Array,
    /// Hash variable (`%foo`) - holds key-value pairs
    Hash,
}

impl VarKind {
    /// Returns the sigil character for this variable kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use perl_symbol_types::VarKind;
    ///
    /// assert_eq!(VarKind::Scalar.sigil(), "$");
    /// assert_eq!(VarKind::Array.sigil(), "@");
    /// assert_eq!(VarKind::Hash.sigil(), "%");
    /// ```
    #[inline]
    pub const fn sigil(self) -> &'static str {
        match self {
            VarKind::Scalar => "$",
            VarKind::Array => "@",
            VarKind::Hash => "%",
        }
    }
}

/// Unified Perl symbol classification for LSP tooling.
///
/// This enum represents all meaningful symbol types in Perl code, designed
/// to be the canonical taxonomy across all crates in the perl-lsp ecosystem.
///
/// # LSP Protocol Mapping
///
/// Each variant maps to an LSP `SymbolKind` number via [`Self::to_lsp_kind()`]:
///
/// | Variant | LSP Kind | Number | Description |
/// |---------|----------|--------|-------------|
/// | `Package` | Module | 2 | Package declaration |
/// | `Class` | Class | 5 | OO class (Moose, Moo, class keyword) |
/// | `Role` | Interface | 8 | Role definition (Moose::Role) |
/// | `Subroutine` | Function | 12 | Standalone subroutine |
/// | `Method` | Method | 6 | OO method |
/// | `Variable(_)` | Variable | 13 | Variables (scalar, array, hash) |
/// | `Constant` | Constant | 14 | use constant or Readonly |
/// | `Import` | Module | 2 | Imported symbol |
/// | `Export` | Function | 12 | Exported symbol |
/// | `Label` | Key | 20 | Loop/block label |
/// | `Format` | Struct | 23 | format declaration |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    // -------------------------------------------------------------------------
    // Package/namespace types
    // -------------------------------------------------------------------------
    /// Package declaration (`package Foo;`)
    Package,
    /// OO class declaration (class keyword, Moose, Moo)
    Class,
    /// Role definition (role keyword, Moose::Role)
    Role,

    // -------------------------------------------------------------------------
    // Callable types
    // -------------------------------------------------------------------------
    /// Subroutine definition (`sub name { }`)
    Subroutine,
    /// Method definition in OO context
    Method,

    // -------------------------------------------------------------------------
    // Variable types
    // -------------------------------------------------------------------------
    /// Variable declaration with sigil-based container type
    Variable(VarKind),

    // -------------------------------------------------------------------------
    // Value/reference types
    // -------------------------------------------------------------------------
    /// Constant value (`use constant NAME => value`)
    Constant,
    /// Imported symbol from `use` statement
    Import,
    /// Exported symbol via Exporter
    Export,

    // -------------------------------------------------------------------------
    // Control flow and special types
    // -------------------------------------------------------------------------
    /// Loop/block label (`LABEL: while ...`)
    Label,
    /// Format declaration (`format STDOUT =`)
    Format,
}

impl SymbolKind {
    /// Convert to LSP-compliant symbol kind number.
    ///
    /// Maps Perl symbol types to the closest LSP protocol equivalents.
    /// See the enum documentation for the full mapping table.
    #[inline]
    pub const fn to_lsp_kind(self) -> u32 {
        match self {
            SymbolKind::Package => 2,      // Module
            SymbolKind::Class => 5,        // Class
            SymbolKind::Role => 8,         // Interface
            SymbolKind::Subroutine => 12,  // Function
            SymbolKind::Method => 6,       // Method
            SymbolKind::Variable(_) => 13, // Variable
            SymbolKind::Constant => 14,    // Constant
            SymbolKind::Import => 2,       // Module
            SymbolKind::Export => 12,      // Function
            SymbolKind::Label => 20,       // Key
            SymbolKind::Format => 23,      // Struct
        }
    }

    /// Returns the sigil for this symbol kind if applicable.
    ///
    /// Only variable symbols have sigils; all other symbols return `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use perl_symbol_types::{SymbolKind, VarKind};
    ///
    /// assert_eq!(SymbolKind::Variable(VarKind::Scalar).sigil(), Some("$"));
    /// assert_eq!(SymbolKind::Variable(VarKind::Array).sigil(), Some("@"));
    /// assert_eq!(SymbolKind::Subroutine.sigil(), None);
    /// ```
    #[inline]
    pub const fn sigil(self) -> Option<&'static str> {
        match self {
            SymbolKind::Variable(vk) => Some(vk.sigil()),
            _ => None,
        }
    }

    /// Returns true if this is any variable type.
    #[inline]
    pub const fn is_variable(self) -> bool {
        matches!(self, SymbolKind::Variable(_))
    }

    /// Returns true if this is a callable type (subroutine or method).
    #[inline]
    pub const fn is_callable(self) -> bool {
        matches!(self, SymbolKind::Subroutine | SymbolKind::Method)
    }

    /// Returns true if this is a namespace type (package, class, or role).
    #[inline]
    pub const fn is_namespace(self) -> bool {
        matches!(self, SymbolKind::Package | SymbolKind::Class | SymbolKind::Role)
    }

    /// Create a scalar variable symbol kind.
    #[inline]
    pub const fn scalar() -> Self {
        SymbolKind::Variable(VarKind::Scalar)
    }

    /// Create an array variable symbol kind.
    #[inline]
    pub const fn array() -> Self {
        SymbolKind::Variable(VarKind::Array)
    }

    /// Create a hash variable symbol kind.
    #[inline]
    pub const fn hash() -> Self {
        SymbolKind::Variable(VarKind::Hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_kind_sigils() {
        assert_eq!(VarKind::Scalar.sigil(), "$");
        assert_eq!(VarKind::Array.sigil(), "@");
        assert_eq!(VarKind::Hash.sigil(), "%");
    }

    #[test]
    fn test_symbol_kind_sigils() {
        assert_eq!(SymbolKind::Variable(VarKind::Scalar).sigil(), Some("$"));
        assert_eq!(SymbolKind::Variable(VarKind::Array).sigil(), Some("@"));
        assert_eq!(SymbolKind::Variable(VarKind::Hash).sigil(), Some("%"));
        assert_eq!(SymbolKind::Subroutine.sigil(), None);
        assert_eq!(SymbolKind::Package.sigil(), None);
    }

    #[test]
    fn test_lsp_kind_mapping() {
        assert_eq!(SymbolKind::Package.to_lsp_kind(), 2);
        assert_eq!(SymbolKind::Class.to_lsp_kind(), 5);
        assert_eq!(SymbolKind::Method.to_lsp_kind(), 6);
        assert_eq!(SymbolKind::Role.to_lsp_kind(), 8);
        assert_eq!(SymbolKind::Subroutine.to_lsp_kind(), 12);
        assert_eq!(SymbolKind::Variable(VarKind::Scalar).to_lsp_kind(), 13);
        assert_eq!(SymbolKind::Constant.to_lsp_kind(), 14);
        assert_eq!(SymbolKind::Label.to_lsp_kind(), 20);
        assert_eq!(SymbolKind::Format.to_lsp_kind(), 23);
    }

    #[test]
    fn test_convenience_constructors() {
        assert_eq!(SymbolKind::scalar(), SymbolKind::Variable(VarKind::Scalar));
        assert_eq!(SymbolKind::array(), SymbolKind::Variable(VarKind::Array));
        assert_eq!(SymbolKind::hash(), SymbolKind::Variable(VarKind::Hash));
    }

    #[test]
    fn test_category_predicates() {
        assert!(SymbolKind::Variable(VarKind::Scalar).is_variable());
        assert!(!SymbolKind::Subroutine.is_variable());

        assert!(SymbolKind::Subroutine.is_callable());
        assert!(SymbolKind::Method.is_callable());
        assert!(!SymbolKind::Variable(VarKind::Scalar).is_callable());

        assert!(SymbolKind::Package.is_namespace());
        assert!(SymbolKind::Class.is_namespace());
        assert!(SymbolKind::Role.is_namespace());
        assert!(!SymbolKind::Subroutine.is_namespace());
    }
}
