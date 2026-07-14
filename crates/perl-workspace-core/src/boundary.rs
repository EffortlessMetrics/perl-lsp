//! Dynamic-boundary reporting.
//!
//! Perl resolves a great deal at runtime. The substrate never pretends it
//! resolved something it did not: where static analysis stops, it emits an
//! explicit [`DynamicBoundary`] with a reason, so consumers (critic, DAP,
//! Kwalitee, RIPR) can degrade honestly rather than silently.

use serde::{Deserialize, Serialize};

use crate::id::FileId;
use crate::provenance::Confidence;
use crate::range::SourceRange;

/// The kind of dynamic construct that bounds static analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DynamicBoundaryKind {
    /// `eval "..."` — string eval.
    StringEval,
    /// `require $expr` — runtime require of a computed module.
    RuntimeRequire,
    /// `*foo = ...` — typeglob assignment.
    TypeglobAssignment,
    /// `$$name` / `&{$ref}` — symbolic reference.
    SymbolicRef,
    /// `AUTOLOAD`.
    Autoload,
    /// A method generated at runtime (accessor generation, etc.).
    GeneratedMethod,
    /// A crossing into XS code.
    XsBoundary,
    /// Inline C (`Inline::C` and similar).
    InlineCBoundary,
    /// A Moose/Moo-generated method or attribute.
    MooseGenerated,
    /// A dynamic Exporter interaction.
    ExporterDynamic,
    /// An `import` that installs symbols dynamically.
    ImportIntoDynamic,
    /// A pragma the substrate does not model.
    UnknownPragma,
}

/// A point where static analysis cannot see through to the runtime effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicBoundary {
    /// The file containing the boundary.
    pub file_id: FileId,
    /// Where the boundary is.
    pub range: SourceRange,
    /// What kind of dynamic construct it is.
    pub kind: DynamicBoundaryKind,
    /// A human-readable reason the analysis stops here.
    pub reason: String,
    /// How confident we are that this is a real boundary.
    pub confidence: Confidence,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&DynamicBoundaryKind::StringEval).unwrap(),
            "\"string_eval\""
        );
        assert_eq!(
            serde_json::to_string(&DynamicBoundaryKind::TypeglobAssignment).unwrap(),
            "\"typeglob_assignment\""
        );
    }
}
