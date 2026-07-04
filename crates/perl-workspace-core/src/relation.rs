//! Relation facts: a uniform cross-entity edge model.
//!
//! Relations are a **projection / synthesis** over facts the substrate already
//! has — package inheritance (`use parent`/`use base`) and module loads
//! (`use`/`require`) — into one edge shape. It performs no new parsing, so it
//! is deterministic and cheap. Edges that need call-site resolution
//! (caller→callee) are a documented follow-up, not attempted here.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::file::FileRole;
use crate::id::FileId;
use crate::import::{ImportFact, ImportKind};
use crate::provenance::Confidence;

/// The kind of a relation edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    /// A package inherits from a parent (`use parent`/`use base`/`@ISA`).
    Inherits,
    /// A source file loads a module (`use`/`require`).
    Uses,
    /// A test file loads a module under test.
    Tests,
}

impl RelationKind {
    /// A short, stable tag used for deterministic ordering.
    #[must_use]
    pub fn tag(self) -> &'static str {
        match self {
            Self::Inherits => "inherits",
            Self::Uses => "uses",
            Self::Tests => "tests",
        }
    }
}

/// A relation edge between two entities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationFact {
    /// The edge kind.
    pub kind: RelationKind,
    /// The source entity: a package name (`inherits`) or a repo-relative file
    /// path (`uses`/`tests`).
    pub source: String,
    /// The target entity: a parent package (`inherits`) or a module name
    /// (`uses`/`tests`).
    pub target: String,
    /// The file the relation is declared in.
    pub file_id: FileId,
    /// Confidence in the edge.
    pub confidence: Confidence,
}

/// Synthesize relation edges for one file from its already-extracted imports and
/// inheritance map.
#[must_use]
pub fn synthesize_relations(
    file_id: &FileId,
    relative_path: &str,
    role: FileRole,
    imports: &[ImportFact],
    parents_by_package: &BTreeMap<String, Vec<String>>,
) -> Vec<RelationFact> {
    let mut out = Vec::new();

    // Inherits: package -> each parent.
    for (package, parents) in parents_by_package {
        for parent in parents {
            out.push(RelationFact {
                kind: RelationKind::Inherits,
                source: package.clone(),
                target: parent.clone(),
                file_id: file_id.clone(),
                confidence: Confidence::High,
            });
        }
    }

    // Uses / Tests: file -> loaded non-pragma module. Pragmas (strict/warnings/
    // parent/base/...) are skipped — parent/base are already inherits edges.
    for import in imports {
        let is_load = matches!(import.kind, ImportKind::Use | ImportKind::Require);
        if is_load && !import.is_pragma && !import.module.is_empty() {
            let kind =
                if role == FileRole::Test { RelationKind::Tests } else { RelationKind::Uses };
            out.push(RelationFact {
                kind,
                source: relative_path.to_string(),
                target: import.module.clone(),
                file_id: file_id.clone(),
                confidence: Confidence::High,
            });
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::Digest;
    use crate::import::ImportFact;
    use crate::range::SourceRange;

    fn zero_range() -> SourceRange {
        SourceRange {
            start_byte: 0,
            end_byte: 0,
            start_line: 0,
            start_column_utf8: 0,
            end_line: 0,
            end_column_utf8: 0,
        }
    }

    fn import(module: &str, is_pragma: bool) -> ImportFact {
        ImportFact {
            file_id: FileId::new("lib/A.pm", &Digest::of("x")),
            kind: ImportKind::Use,
            module: module.to_string(),
            version: None,
            imports: Vec::new(),
            is_pragma,
            range: zero_range(),
            confidence: Confidence::High,
        }
    }

    #[test]
    fn derives_inherits_and_uses() {
        let file_id = FileId::new("lib/Child.pm", &Digest::of("x"));
        let mut parents = BTreeMap::new();
        parents.insert("Child".to_string(), vec!["Base".to_string()]);
        let imports = vec![import("strict", true), import("Moo", false)];

        let rels =
            synthesize_relations(&file_id, "lib/Child.pm", FileRole::Lib, &imports, &parents);

        assert!(
            rels.iter().any(|r| r.kind == RelationKind::Inherits
                && r.source == "Child"
                && r.target == "Base"),
            "inherits edge"
        );
        assert!(
            rels.iter().any(|r| r.kind == RelationKind::Uses && r.target == "Moo"),
            "uses edge for a non-pragma module"
        );
        assert!(!rels.iter().any(|r| r.target == "strict"), "pragma is not a uses edge");
    }

    #[test]
    fn test_files_produce_tests_edges() {
        let file_id = FileId::new("t/x.t", &Digest::of("x"));
        let imports = vec![import("My::Module", false)];
        let rels =
            synthesize_relations(&file_id, "t/x.t", FileRole::Test, &imports, &BTreeMap::new());
        assert!(
            rels.iter().any(|r| r.kind == RelationKind::Tests && r.target == "My::Module"),
            "test file → Tests edge"
        );
    }
}
