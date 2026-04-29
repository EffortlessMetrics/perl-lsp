//! Neutral semantic fact vocabulary for Perl analysis layers.
//!
//! This crate defines strongly-typed IDs and serializable fact records that can be shared
//! between parser-derived semantics, semantic analyzer synthesis, and workspace indexing.
//!
//! It intentionally does **not** parse Perl, implement LSP providers, or own workspace
//! storage backends.

use serde::{Deserialize, Serialize};
use perl_symbol::{SymbolRef, SymbolRefKind};

macro_rules! id_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(pub u64);
    };
}

id_newtype!(FileId);
id_newtype!(ScopeId);
id_newtype!(EntityId);
id_newtype!(AnchorId);
id_newtype!(OccurrenceId);
id_newtype!(EdgeId);
id_newtype!(DiagnosticId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorFact {
    pub id: AnchorId,
    pub file_id: FileId,
    pub span_start_byte: u32,
    pub span_end_byte: u32,
    pub scope_id: Option<ScopeId>,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityFact {
    pub id: EntityId,
    pub kind: EntityKind,
    pub canonical_name: String,
    pub anchor_id: Option<AnchorId>,
    pub scope_id: Option<ScopeId>,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OccurrenceFact {
    pub id: OccurrenceId,
    pub kind: OccurrenceKind,
    pub entity_id: Option<EntityId>,
    pub anchor_id: AnchorId,
    pub scope_id: Option<ScopeId>,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeFact {
    pub id: EdgeId,
    pub kind: EdgeKind,
    pub from_entity_id: EntityId,
    pub to_entity_id: EntityId,
    pub via_occurrence_id: Option<OccurrenceId>,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticFact {
    pub id: DiagnosticId,
    pub code: Option<String>,
    pub message: String,
    pub primary_anchor_id: AnchorId,
    pub related_anchor_ids: Vec<AnchorId>,
    pub scope_id: Option<ScopeId>,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

/// Adapter input for converting a `SymbolRef` into canonical semantic facts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SymbolRefOccurrenceInput<'a> {
    pub symbol_ref: &'a SymbolRef,
    /// Optional entity that owns the source occurrence context.
    pub source_entity_id: Option<EntityId>,
    /// Optional entity resolved as the target of this symbol reference.
    pub target_entity_id: Option<EntityId>,
}

/// Deterministic fact output from adapting `SymbolRef` records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRefFactSet {
    pub anchors: Vec<AnchorFact>,
    pub occurrences: Vec<OccurrenceFact>,
    pub edges: Vec<EdgeFact>,
}

/// Convert phase-1 `SymbolRef` projection records into semantic facts.
///
/// This adapter intentionally covers only the phase-1 `SymbolRef` families
/// (variable references and subroutine-call references). Excluded families such
/// as method calls, coderef calls, indirect calls, and typeglobs are already
/// omitted by `SymbolRef` extraction and therefore do not appear here.
pub fn symbol_ref_occurrence_facts(
    file_id: FileId,
    scope_id: Option<ScopeId>,
    refs: &[SymbolRefOccurrenceInput<'_>],
) -> SymbolRefFactSet {
    let mut anchors = Vec::with_capacity(refs.len());
    let mut occurrences = Vec::with_capacity(refs.len());
    let mut edges = Vec::new();

    for (index, input) in refs.iter().enumerate() {
        let anchor_id = AnchorId(index as u64 + 1);
        let occurrence_id = OccurrenceId(index as u64 + 1);

        let (start, end) = input
            .symbol_ref
            .anchor_span
            .unwrap_or(input.symbol_ref.full_span);

        anchors.push(AnchorFact {
            id: anchor_id,
            file_id,
            span_start_byte: start as u32,
            span_end_byte: end as u32,
            scope_id,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });

        let occurrence_kind = match input.symbol_ref.kind {
            SymbolRefKind::Variable(_) => OccurrenceKind::Reference,
            SymbolRefKind::SubroutineCall => OccurrenceKind::Call,
        };

        occurrences.push(OccurrenceFact {
            id: occurrence_id,
            kind: occurrence_kind,
            entity_id: input.target_entity_id,
            anchor_id,
            scope_id,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });

        if let (Some(from_entity_id), Some(to_entity_id)) =
            (input.source_entity_id, input.target_entity_id)
        {
            edges.push(EdgeFact {
                id: EdgeId(edges.len() as u64 + 1),
                kind: EdgeKind::References,
                from_entity_id,
                to_entity_id,
                via_occurrence_id: Some(occurrence_id),
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            });
        }
    }

    SymbolRefFactSet {
        anchors,
        occurrences,
        edges,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_symbol::VarKind;

    #[test]
    fn entity_fact_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let fact = EntityFact {
            id: EntityId(100),
            kind: EntityKind::Method,
            canonical_name: "Foo::bar".to_string(),
            anchor_id: Some(AnchorId(12)),
            scope_id: Some(ScopeId(3)),
            provenance: Provenance::SemanticAnalyzer,
            confidence: Confidence::High,
        };

        let serialized = serde_json::to_string(&fact)?;
        let decoded: EntityFact = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, fact);
        Ok(())
    }

    #[test]
    fn deterministic_debug_for_edge_fact() {
        let fact = EdgeFact {
            id: EdgeId(7),
            kind: EdgeKind::Calls,
            from_entity_id: EntityId(11),
            to_entity_id: EntityId(22),
            via_occurrence_id: Some(OccurrenceId(33)),
            provenance: Provenance::ExactAst,
            confidence: Confidence::Medium,
        };

        assert_eq!(
            format!("{fact:?}"),
            "EdgeFact { id: EdgeId(7), kind: Calls, from_entity_id: EntityId(11), to_entity_id: EntityId(22), via_occurrence_id: Some(OccurrenceId(33)), provenance: ExactAst, confidence: Medium }"
        );
    }

    #[test]
    fn pretty_json_for_anchor_fact_is_stable() -> Result<(), serde_json::Error> {
        let fact = AnchorFact {
            id: AnchorId(5),
            file_id: FileId(1),
            span_start_byte: 10,
            span_end_byte: 15,
            scope_id: None,
            provenance: Provenance::DesugaredAst,
            confidence: Confidence::Low,
        };

        let json = serde_json::to_string_pretty(&fact)?;
        assert_eq!(
            json,
            "{\n  \"id\": 5,\n  \"file_id\": 1,\n  \"span_start_byte\": 10,\n  \"span_end_byte\": 15,\n  \"scope_id\": null,\n  \"provenance\": \"DesugaredAst\",\n  \"confidence\": \"Low\"\n}"
        );
        Ok(())
    }

    /// Verify OccurrenceFact with a None entity_id round-trips correctly — this
    /// exercises the optional foreign-key path that EntityFact's test does not cover.
    #[test]
    fn occurrence_fact_with_null_entity_id_roundtrips() -> Result<(), serde_json::Error> {
        let fact = OccurrenceFact {
            id: OccurrenceId(42),
            kind: OccurrenceKind::Call,
            entity_id: None,
            anchor_id: AnchorId(10),
            scope_id: Some(ScopeId(2)),
            provenance: Provenance::NameHeuristic,
            confidence: Confidence::Low,
        };
        let serialized = serde_json::to_string(&fact)?;
        let decoded: OccurrenceFact = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, fact);
        // entity_id: None must serialize as JSON null, not be omitted.
        assert!(serialized.contains("\"entity_id\":null"), "entity_id null must be explicit in JSON");
        Ok(())
    }

    /// Verify that u64::MAX is preserved through JSON serialization without
    /// truncation — serde_json serializes u64 as a JSON number, which can
    /// exceed JS safe-integer range but round-trips correctly in Rust.
    #[test]
    fn id_u64_max_roundtrips() -> Result<(), serde_json::Error> {
        let id = EntityId(u64::MAX);
        let serialized = serde_json::to_string(&id)?;
        let decoded: EntityId = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, id);
        Ok(())
    }

    #[test]
    fn symbol_ref_occurrence_adapter_emits_deterministic_facts() {
        let refs = [
            SymbolRef {
                kind: SymbolRefKind::Variable(VarKind::Scalar),
                name: "x".to_string(),
                qualified_name: "x".to_string(),
                sigil: Some("$".to_string()),
                package_qualifier: None,
                full_span: (10, 12),
                anchor_span: Some((11, 12)),
            },
            SymbolRef {
                kind: SymbolRefKind::SubroutineCall,
                name: "foo".to_string(),
                qualified_name: "Pkg::foo".to_string(),
                sigil: None,
                package_qualifier: Some("Pkg".to_string()),
                full_span: (20, 30),
                anchor_span: None,
            },
        ];
        let facts = symbol_ref_occurrence_facts(
            FileId(7),
            Some(ScopeId(9)),
            &[
                SymbolRefOccurrenceInput {
                    symbol_ref: &refs[0],
                    source_entity_id: Some(EntityId(100)),
                    target_entity_id: Some(EntityId(200)),
                },
                SymbolRefOccurrenceInput {
                    symbol_ref: &refs[1],
                    source_entity_id: None,
                    target_entity_id: None,
                },
            ],
        );

        assert_eq!(facts.anchors.len(), 2);
        assert_eq!(facts.anchors[0].id, AnchorId(1));
        assert_eq!(facts.anchors[0].span_start_byte, 11);
        assert_eq!(facts.anchors[0].span_end_byte, 12);
        assert_eq!(facts.anchors[1].id, AnchorId(2));
        assert_eq!(facts.anchors[1].span_start_byte, 20);
        assert_eq!(facts.anchors[1].span_end_byte, 30);

        assert_eq!(facts.occurrences.len(), 2);
        assert_eq!(facts.occurrences[0].kind, OccurrenceKind::Reference);
        assert_eq!(facts.occurrences[0].entity_id, Some(EntityId(200)));
        assert_eq!(facts.occurrences[1].kind, OccurrenceKind::Call);
        assert_eq!(facts.occurrences[1].entity_id, None);

        assert_eq!(facts.edges.len(), 1);
        assert_eq!(facts.edges[0].id, EdgeId(1));
        assert_eq!(facts.edges[0].kind, EdgeKind::References);
        assert_eq!(facts.edges[0].from_entity_id, EntityId(100));
        assert_eq!(facts.edges[0].to_entity_id, EntityId(200));
        assert_eq!(facts.edges[0].via_occurrence_id, Some(OccurrenceId(1)));
    }
}
