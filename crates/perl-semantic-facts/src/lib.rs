//! Neutral semantic fact vocabulary shared across Perl analysis layers.
//!
//! This crate defines stable typed IDs and fact records that can be emitted by parsing,
//! semantic analysis, import/export inference, and indexing subsystems.
//!
//! It intentionally does **not** parse Perl source, implement LSP providers, or own
//! workspace-scale storage.

use serde::{Deserialize, Serialize};

macro_rules! semantic_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub u64);
    };
}

semantic_id!(FileId);
semantic_id!(ScopeId);
semantic_id!(EntityId);
semantic_id!(AnchorId);
semantic_id!(OccurrenceId);
semantic_id!(EdgeId);
semantic_id!(DiagnosticId);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorFact {
    pub id: AnchorId,
    pub file_id: FileId,
    pub start_byte: u32,
    pub end_byte: u32,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityFact {
    pub id: EntityId,
    pub kind: EntityKind,
    pub canonical_name: String,
    pub scope_id: Option<ScopeId>,
    pub anchor_id: Option<AnchorId>,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OccurrenceFact {
    pub id: OccurrenceId,
    pub entity_id: EntityId,
    pub anchor_id: AnchorId,
    pub kind: OccurrenceKind,
    pub file_id: FileId,
    pub scope_id: Option<ScopeId>,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeFact {
    pub id: EdgeId,
    pub source: EntityId,
    pub target: EntityId,
    pub kind: EdgeKind,
    pub anchor_id: Option<AnchorId>,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticFact {
    pub id: DiagnosticId,
    pub file_id: FileId,
    pub anchor_id: Option<AnchorId>,
    pub message: String,
    pub code: Option<String>,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EntityKind {
    Package,
    Class,
    Role,
    Subroutine,
    Method,
    Variable,
    Constant,
    Field,
    Label,
    Format,
    Module,
    GeneratedMember,
    ExternalSymbol,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[non_exhaustive]
pub enum OccurrenceKind {
    Definition,
    Reference,
    Read,
    Write,
    Call,
    MethodCall,
    StaticMethodCall,
    Import,
    Export,
    Inheritance,
    RoleComposition,
    GeneratedUse,
    DynamicBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EdgeKind {
    Defines,
    References,
    Reads,
    Writes,
    Calls,
    ImportsModule,
    ImportsSymbol,
    ExportsSymbol,
    ExportsGroup,
    Inherits,
    ComposesRole,
    MemberOf,
    GeneratedFrom,
    AliasOf,
    DependsOn,
    DynamicBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Provenance {
    ExactAst,
    DesugaredAst,
    SemanticAnalyzer,
    FrameworkSynthesis,
    ImportExportInference,
    PragmaInference,
    NameHeuristic,
    SearchFallback,
    DynamicBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Confidence {
    Confirmed,
    High,
    Medium,
    Low,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn typed_ids_are_distinct_and_ordered() -> TestResult {
        let file = FileId(10);
        let entity = EntityId(10);
        assert_eq!(file.0, 10);
        assert_eq!(entity.0, 10);
        assert!(EdgeId(2) > EdgeId(1));
        Ok(())
    }

    #[test]
    fn entity_fact_roundtrips_through_json() -> TestResult {
        let fact = EntityFact {
            id: EntityId(42),
            kind: EntityKind::Method,
            canonical_name: "My::Class::run".to_string(),
            scope_id: Some(ScopeId(7)),
            anchor_id: Some(AnchorId(11)),
            provenance: Provenance::SemanticAnalyzer,
            confidence: Confidence::High,
        };

        let json = serde_json::to_string(&fact)?;
        let restored: EntityFact = serde_json::from_str(&json)?;
        assert_eq!(restored, fact);
        Ok(())
    }

    #[test]
    fn deterministic_debug_output_for_snapshots() -> TestResult {
        let edge = EdgeFact {
            id: EdgeId(3),
            source: EntityId(1),
            target: EntityId(2),
            kind: EdgeKind::Calls,
            anchor_id: Some(AnchorId(5)),
            provenance: Provenance::ExactAst,
            confidence: Confidence::Confirmed,
        };

        let debug = format!("{edge:?}");
        assert_eq!(
            debug,
            "EdgeFact { id: EdgeId(3), source: EntityId(1), target: EntityId(2), kind: Calls, anchor_id: Some(AnchorId(5)), provenance: ExactAst, confidence: Confirmed }"
        );
        Ok(())
    }

    #[test]
    fn all_kinds_serialize_with_stable_names() -> TestResult {
        assert_eq!(serde_json::to_string(&EntityKind::GeneratedMember)?, "\"GeneratedMember\"");
        assert_eq!(serde_json::to_string(&OccurrenceKind::StaticMethodCall)?, "\"StaticMethodCall\"");
        assert_eq!(serde_json::to_string(&EdgeKind::ImportsSymbol)?, "\"ImportsSymbol\"");
        assert_eq!(serde_json::to_string(&Provenance::ImportExportInference)?, "\"ImportExportInference\"");
        assert_eq!(serde_json::to_string(&Confidence::Medium)?, "\"Medium\"");
        Ok(())
    }
}
