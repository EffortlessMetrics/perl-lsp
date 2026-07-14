//! Symbol facts: the packages, subs, methods, and other declarations a file
//! defines.

use serde::{Deserialize, Serialize};

use crate::id::{FileId, SymbolId};
use crate::provenance::Confidence;
use crate::range::SourceRange;

/// The kind of a declared symbol, in the substrate's own vocabulary.
///
/// This mirrors [`perl_symbol::SymbolKind`] but is a stable, serializable
/// vocabulary owned by the substrate (so a consumer never has to depend on
/// `perl-symbol` to read facts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolFactKind {
    /// `package Foo;`
    Package,
    /// `class Foo { ... }` (Moose/Moo/`feature 'class'`).
    Class,
    /// `role Foo { ... }`.
    Role,
    /// `sub name { ... }`.
    Sub,
    /// A method (`method name { ... }`, or a sub in OO context).
    Method,
    /// A variable declaration.
    Variable,
    /// `use constant NAME => ...`.
    Constant,
    /// A symbol imported from another module.
    Import,
    /// A symbol exported via Exporter.
    Export,
    /// A loop/block label.
    Label,
    /// A `format` declaration.
    Format,
}

impl SymbolFactKind {
    /// A short, stable tag used in [`SymbolId`] derivation and serialization.
    #[must_use]
    pub fn tag(self) -> &'static str {
        match self {
            Self::Package => "package",
            Self::Class => "class",
            Self::Role => "role",
            Self::Sub => "sub",
            Self::Method => "method",
            Self::Variable => "variable",
            Self::Constant => "constant",
            Self::Import => "import",
            Self::Export => "export",
            Self::Label => "label",
            Self::Format => "format",
        }
    }

    /// Map a [`perl_symbol::SymbolKind`] into the substrate vocabulary.
    #[must_use]
    pub fn from_perl_symbol(kind: perl_symbol::SymbolKind) -> Self {
        use perl_symbol::SymbolKind as K;
        match kind {
            K::Package => Self::Package,
            K::Class => Self::Class,
            K::Role => Self::Role,
            K::Subroutine => Self::Sub,
            K::Method => Self::Method,
            K::Variable(_) => Self::Variable,
            K::Constant => Self::Constant,
            K::Import => Self::Import,
            K::Export => Self::Export,
            K::Label => Self::Label,
            K::Format => Self::Format,
        }
    }
}

/// Whether a symbol is reachable outside its declaring scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    /// Package-scoped or exported — reachable from other files.
    Public,
    /// Lexically scoped (`my`) — invisible outside its block.
    Private,
    /// Visibility could not be determined.
    Unknown,
}

/// A symbol fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolRecord {
    /// Stable identity.
    pub symbol_id: SymbolId,
    /// The file that declares this symbol.
    pub file_id: FileId,
    /// Symbol classification.
    pub kind: SymbolFactKind,
    /// Enclosing package, if the declaration is inside one.
    pub package: Option<String>,
    /// Unqualified name.
    pub name: String,
    /// Package-qualified name (`Foo::bar`) or bare name at top level.
    pub qualified_name: String,
    /// Span of the full declaration.
    pub declaration_range: SourceRange,
    /// Reachability.
    pub visibility: Visibility,
    /// How confident we are in this fact.
    pub confidence: Confidence,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_stable_and_snake_case_serialization() {
        assert_eq!(SymbolFactKind::Sub.tag(), "sub");
        assert_eq!(serde_json::to_string(&SymbolFactKind::Package).unwrap(), "\"package\"");
    }

    #[test]
    fn maps_every_perl_symbol_kind() {
        use perl_symbol::{SymbolKind as K, VarKind};
        assert_eq!(SymbolFactKind::from_perl_symbol(K::Package), SymbolFactKind::Package);
        assert_eq!(SymbolFactKind::from_perl_symbol(K::Subroutine), SymbolFactKind::Sub);
        assert_eq!(SymbolFactKind::from_perl_symbol(K::Method), SymbolFactKind::Method);
        assert_eq!(
            SymbolFactKind::from_perl_symbol(K::Variable(VarKind::Scalar)),
            SymbolFactKind::Variable
        );
        assert_eq!(SymbolFactKind::from_perl_symbol(K::Constant), SymbolFactKind::Constant);
    }

    #[test]
    fn visibility_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&Visibility::Public).unwrap(), "\"public\"");
    }
}
