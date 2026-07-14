//! Package facts.

use serde::{Deserialize, Serialize};

use crate::id::{FileId, PackageId};
use crate::provenance::Confidence;
use crate::range::SourceRange;

/// A package declaration fact.
///
/// Inheritance (`parents`) and roles are populated by the module/import fact
/// pass (PLSP-ADR-0006 PR 4); until then they are empty and honestly so.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageRecord {
    /// Stable identity.
    pub package_id: PackageId,
    /// Fully-qualified package name.
    pub name: String,
    /// The file that declares this package.
    pub file_id: FileId,
    /// Span of the `package` declaration.
    pub declaration_range: SourceRange,
    /// Declared version (`package Foo 1.23;` / `our $VERSION`), if known.
    pub version: Option<String>,
    /// Parent packages (`use parent`/`use base`/`@ISA`). Populated in PR 4.
    pub parents: Vec<String>,
    /// Consumed roles. Populated in PR 4.
    pub roles: Vec<String>,
    /// How confident we are in this fact.
    pub confidence: Confidence,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::Digest;

    #[test]
    fn package_record_round_trips_json() {
        let file_id = FileId::new("lib/App.pm", &Digest::of("x"));
        let record = PackageRecord {
            package_id: PackageId::new(&file_id, "App", 0),
            name: "App".to_string(),
            file_id,
            declaration_range: SourceRange {
                start_byte: 0,
                end_byte: 11,
                start_line: 0,
                start_column_utf8: 0,
                end_line: 0,
                end_column_utf8: 11,
            },
            version: None,
            parents: Vec::new(),
            roles: Vec::new(),
            confidence: Confidence::High,
        };
        let json = serde_json::to_string(&record).unwrap();
        let back: PackageRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record, back);
    }
}
