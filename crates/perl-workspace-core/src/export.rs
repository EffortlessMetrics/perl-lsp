//! Export facts: what a package advertises via Exporter (`@EXPORT` /
//! `@EXPORT_OK`).
//!
//! Together with [`ImportFact`](crate::import::ImportFact), these give the
//! module *interface* — what a package brings in and what it makes available.
//! This completes the "Exporter basics" follow-up noted on PLSP-ADR-0006 PR 4.

use serde::{Deserialize, Serialize};

use crate::id::FileId;
use crate::provenance::Confidence;
use crate::range::SourceRange;

/// Which Exporter list a fact came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportKind {
    /// `@EXPORT` — exported by default.
    Default,
    /// `@EXPORT_OK` — exported on request.
    Optional,
}

/// An export fact: the symbols a package advertises through one Exporter list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportFact {
    /// The file declaring the export list.
    pub file_id: FileId,
    /// The enclosing package, if known.
    pub package: Option<String>,
    /// Default (`@EXPORT`) vs optional (`@EXPORT_OK`).
    pub kind: ExportKind,
    /// The exported symbol names (sigils preserved as written; `qw()`/quotes
    /// normalized away).
    pub symbols: Vec<String>,
    /// Span of the export declaration.
    pub range: SourceRange,
    /// Confidence in the fact.
    pub confidence: Confidence,
}

/// Classify an `@`-array variable name as an Exporter list, if it is one.
#[must_use]
pub fn export_kind_for(array_name: &str) -> Option<ExportKind> {
    match array_name {
        "EXPORT" => Some(ExportKind::Default),
        "EXPORT_OK" => Some(ExportKind::Optional),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_exporter_arrays() {
        assert_eq!(export_kind_for("EXPORT"), Some(ExportKind::Default));
        assert_eq!(export_kind_for("EXPORT_OK"), Some(ExportKind::Optional));
        assert_eq!(export_kind_for("ISA"), None);
        assert_eq!(export_kind_for("EXPORT_TAGS"), None);
    }

    #[test]
    fn kind_serializes_snake_case() {
        assert_eq!(serde_json::to_string(&ExportKind::Optional).unwrap(), "\"optional\"");
    }
}
