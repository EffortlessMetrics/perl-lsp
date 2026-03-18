#![deny(unsafe_code)]
#![warn(missing_docs)]
//! Shared symbol-domain types for workspace indexing and navigation.
//!
//! This crate extracts the symbol model that was previously embedded in
//! `perl-workspace-index` so indexing logic can focus on orchestration while
//! downstream consumers share a single, stable representation.

use perl_position_tracking::{Range, WireLocation, WirePosition, WireRange};
pub use perl_symbol_types::{SymbolKind, VarKind};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Symbol kinds for cross-file indexing during Index/Navigate workflows.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SymKind {
    /// Variable symbol ($, @, or % sigil).
    Var,
    /// Subroutine definition (sub foo).
    Sub,
    /// Package declaration (package Foo).
    Pack,
}

/// A normalized symbol key for cross-file lookups in Index/Navigate workflows.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SymbolKey {
    /// Package name containing this symbol.
    pub pkg: Arc<str>,
    /// Bare name without sigil prefix.
    pub name: Arc<str>,
    /// Variable sigil ($, @, or %) if applicable.
    pub sigil: Option<char>,
    /// Kind of symbol (variable, subroutine, package).
    pub kind: SymKind,
}

/// Normalize a Perl variable name for Index/Analyze workflows.
#[must_use]
pub fn normalize_var(name: &str) -> (Option<char>, &str) {
    if name.is_empty() {
        return (None, "");
    }

    let Some(first_char) = name.chars().next() else {
        return (None, name);
    };

    match first_char {
        '$' | '@' | '%' => {
            if name.len() > 1 {
                (Some(first_char), &name[1..])
            } else {
                (Some(first_char), "")
            }
        }
        _ => (None, name),
    }
}

/// Internal location type used during Navigate/Analyze workflows.
#[derive(Debug, Clone)]
pub struct Location {
    /// File URI where the symbol is located.
    pub uri: String,
    /// Line and character range within the file.
    pub range: Range,
}

/// A symbol in the workspace for Index/Navigate workflows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSymbol {
    /// Symbol name without package qualification.
    pub name: String,
    /// Type of symbol (subroutine, variable, package, etc.).
    pub kind: SymbolKind,
    /// File URI where the symbol is defined.
    pub uri: String,
    /// Line and character range of the symbol definition.
    pub range: Range,
    /// Fully qualified name including package (e.g., "Package::function").
    pub qualified_name: Option<String>,
    /// POD documentation associated with the symbol.
    pub documentation: Option<String>,
    /// Name of the containing package or class.
    pub container_name: Option<String>,
    /// Whether this symbol has a body (false for forward declarations).
    #[serde(default = "default_has_body")]
    pub has_body: bool,
}

fn default_has_body() -> bool {
    true
}

/// Helper function to convert sigil to [`VarKind`].
#[must_use]
pub fn sigil_to_var_kind(sigil: &str) -> VarKind {
    match sigil {
        "@" => VarKind::Array,
        "%" => VarKind::Hash,
        _ => VarKind::Scalar,
    }
}

/// Reference to a symbol for Navigate/Analyze workflows.
#[derive(Debug, Clone)]
pub struct SymbolReference {
    /// File URI where the reference occurs.
    pub uri: String,
    /// Line and character range of the reference.
    pub range: Range,
    /// How the symbol is being referenced (definition, usage, etc.).
    pub kind: ReferenceKind,
}

/// Classification of how a symbol is referenced in Navigate/Analyze workflows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceKind {
    /// Symbol definition site (sub declaration, variable declaration).
    Definition,
    /// General usage of the symbol (function call, method call).
    Usage,
    /// Import via use statement.
    Import,
    /// Variable read access.
    Read,
    /// Variable write access (assignment target).
    Write,
}

/// LSP-compliant workspace symbol for wire format in Navigate/Analyze workflows.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspWorkspaceSymbol {
    /// Symbol name as displayed to the user.
    pub name: String,
    /// LSP symbol kind number (see `lsp_types::SymbolKind`).
    pub kind: u32,
    /// Location of the symbol definition.
    pub location: WireLocation,
    /// Name of the containing symbol (package, class).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
}

impl From<&WorkspaceSymbol> for LspWorkspaceSymbol {
    fn from(sym: &WorkspaceSymbol) -> Self {
        let range = WireRange {
            start: WirePosition { line: sym.range.start.line, character: sym.range.start.column },
            end: WirePosition { line: sym.range.end.line, character: sym.range.end.column },
        };

        Self {
            name: sym.name.clone(),
            kind: sym.kind.to_lsp_kind(),
            location: WireLocation { uri: sym.uri.clone(), range },
            container_name: sym.container_name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ReferenceKind, SymKind, normalize_var, sigil_to_var_kind};
    use perl_symbol_types::VarKind;

    #[test]
    fn normalize_var_strips_sigil() {
        assert_eq!(normalize_var("$count"), (Some('$'), "count"));
        assert_eq!(normalize_var("items"), (None, "items"));
        assert_eq!(normalize_var("%"), (Some('%'), ""));
    }

    #[test]
    fn sigil_maps_to_expected_var_kind() {
        assert_eq!(sigil_to_var_kind("@"), VarKind::Array);
        assert_eq!(sigil_to_var_kind("%"), VarKind::Hash);
        assert_eq!(sigil_to_var_kind("$"), VarKind::Scalar);
    }

    #[test]
    fn core_enums_are_equatable() {
        assert_eq!(SymKind::Sub, SymKind::Sub);
        assert_eq!(ReferenceKind::Usage, ReferenceKind::Usage);
    }
}
