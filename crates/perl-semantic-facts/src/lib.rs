//! Neutral semantic fact vocabulary for Perl analysis layers.
//!
//! This crate defines strongly-typed IDs and serializable fact records that can be shared
//! between parser-derived semantics, semantic analyzer synthesis, and workspace indexing.
//!
//! It intentionally does **not** parse Perl, implement LSP providers, or own workspace
//! storage backends.

use serde::{Deserialize, Serialize};

macro_rules! id_newtype {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
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
pub enum ImportKind {
    Use,
    UseEmpty,
    UseExplicitList,
    UseTag,
    Require,
    RequireThenImport,
    UseConstant,
    DynamicRequire,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportSymbols {
    Default,
    None,
    Explicit(Vec<String>),
    Tags(Vec<String>),
    Mixed { tags: Vec<String>, names: Vec<String> },
    Dynamic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSpec {
    pub importer_entity_id: Option<EntityId>,
    pub module_name: String,
    pub kind: ImportKind,
    pub symbols: ImportSymbols,
    pub anchor_id: Option<AnchorId>,
    pub scope_id: Option<ScopeId>,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum VisibleSymbolSource {
    LocalLexical,
    LocalPackage,
    ExplicitImport,
    DefaultExport,
    ExportTag,
    Constant,
    Generated,
    External,
    DynamicUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleSymbol {
    pub name: String,
    pub entity_id: Option<EntityId>,
    pub source: VisibleSymbolSource,
    pub import_anchor_id: Option<AnchorId>,
    pub scope_id: Option<ScopeId>,
    pub provenance: Provenance,
    pub confidence: Confidence,
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

/// Canonical export facts inferred for a Perl package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportSet {
    /// Package symbols exported by default (`@EXPORT`).
    pub default_exports: Vec<String>,
    /// Package symbols exported on request (`@EXPORT_OK`).
    pub optional_exports: Vec<String>,
    /// Named export groups (`%EXPORT_TAGS`).
    pub tags: Vec<ExportTag>,
    /// How this export set was inferred.
    pub provenance: Provenance,
    /// Confidence for the inferred export set.
    pub confidence: Confidence,
}

/// Named `%EXPORT_TAGS` entry and its members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportTag {
    /// Tag name (for example `all` from `:all`).
    pub name: String,
    /// Symbols in this tag.
    pub members: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(
            serialized.contains("\"entity_id\":null"),
            "entity_id null must be explicit in JSON"
        );
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
    fn import_spec_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let spec = ImportSpec {
            importer_entity_id: Some(EntityId(50)),
            module_name: "Exporter".to_string(),
            kind: ImportKind::UseExplicitList,
            symbols: ImportSymbols::Mixed {
                tags: vec!["all".to_string()],
                names: vec!["foo".to_string(), "bar".to_string()],
            },
            anchor_id: Some(AnchorId(9)),
            scope_id: Some(ScopeId(4)),
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        };

        let serialized = serde_json::to_string(&spec)?;
        let decoded: ImportSpec = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, spec);
        Ok(())
    }

    #[test]
    fn visible_symbol_pretty_json_is_stable() -> Result<(), serde_json::Error> {
        let symbol = VisibleSymbol {
            name: "foo".to_string(),
            entity_id: None,
            source: VisibleSymbolSource::DynamicUnknown,
            import_anchor_id: None,
            scope_id: Some(ScopeId(99)),
            provenance: Provenance::ImportExportInference,
            confidence: Confidence::Medium,
        };

        let json = serde_json::to_string_pretty(&symbol)?;
        assert_eq!(
            json,
            "{\n  \"name\": \"foo\",\n  \"entity_id\": null,\n  \"source\": \"DynamicUnknown\",\n  \"import_anchor_id\": null,\n  \"scope_id\": 99,\n  \"provenance\": \"ImportExportInference\",\n  \"confidence\": \"Medium\"\n}"
        );
        Ok(())
    }
}
