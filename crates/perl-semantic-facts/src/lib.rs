//! Neutral semantic fact vocabulary for Perl analysis layers.
//!
//! This crate defines typed identifiers and fact records that can be shared between parser,
//! semantic analyzer, indexing, and export/import inference components.
//!
//! ## Non-goals
//! - Parsing Perl source.
//! - Owning workspace storage and indexing.
//! - Providing LSP protocol behavior.

use serde::{Deserialize, Serialize};

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub u64);
    };
}

id_type!(FileId);
id_type!(ScopeId);
id_type!(EntityId);
id_type!(AnchorId);
id_type!(OccurrenceId);
id_type!(EdgeId);
id_type!(DiagnosticId);

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
    Certain,
    High,
    Medium,
    Low,
    Speculative,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AnchorFact {
    pub id: AnchorId,
    pub file_id: FileId,
    pub start_byte: u32,
    pub end_byte: u32,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EntityFact {
    pub id: EntityId,
    pub scope_id: Option<ScopeId>,
    pub anchor_id: Option<AnchorId>,
    pub canonical_name: String,
    pub kind: EntityKind,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OccurrenceFact {
    pub id: OccurrenceId,
    pub entity_id: EntityId,
    pub anchor_id: AnchorId,
    pub kind: OccurrenceKind,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EdgeFact {
    pub id: EdgeId,
    pub from_entity_id: EntityId,
    pub to_entity_id: EntityId,
    pub kind: EdgeKind,
    pub anchor_id: Option<AnchorId>,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DiagnosticFact {
    pub id: DiagnosticId,
    pub anchor_id: Option<AnchorId>,
    pub related_entity_id: Option<EntityId>,
    pub code: String,
    pub message: String,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip_fact_bundle_is_stable() -> Result<(), serde_json::Error> {
        let bundle = vec![
            EntityFact {
                id: EntityId(11),
                scope_id: Some(ScopeId(7)),
                anchor_id: Some(AnchorId(3)),
                canonical_name: "My::Package::method".to_owned(),
                kind: EntityKind::Method,
                provenance: Provenance::SemanticAnalyzer,
                confidence: Confidence::High,
            },
            EntityFact {
                id: EntityId(99),
                scope_id: None,
                anchor_id: None,
                canonical_name: "strict".to_owned(),
                kind: EntityKind::Module,
                provenance: Provenance::ImportExportInference,
                confidence: Confidence::Medium,
            },
        ];

        let json = serde_json::to_string_pretty(&bundle)?;
        let decoded: Vec<EntityFact> = serde_json::from_str(&json)?;

        assert_eq!(decoded, bundle);
        Ok(())
    }

    #[test]
    fn deterministic_debug_output_for_snapshot_style_assertions() {
        let edge = EdgeFact {
            id: EdgeId(1),
            from_entity_id: EntityId(2),
            to_entity_id: EntityId(3),
            kind: EdgeKind::DependsOn,
            anchor_id: Some(AnchorId(4)),
            provenance: Provenance::ExactAst,
            confidence: Confidence::Certain,
        };

        let expected = "EdgeFact { id: EdgeId(1), from_entity_id: EntityId(2), to_entity_id: EntityId(3), kind: DependsOn, anchor_id: Some(AnchorId(4)), provenance: ExactAst, confidence: Certain }";
        assert_eq!(format!("{edge:?}"), expected);
    }

    #[test]
    fn ids_are_typed_and_not_interchangeable() {
        let file = FileId(42);
        let anchor = AnchorId(42);

        assert_eq!(file.0, 42);
        assert_eq!(anchor.0, 42);
    }
}
