//! Neutral semantic fact vocabulary shared by perl-lsp analysis layers.
//!
//! This crate defines typed identifiers, fact records, and relationship enums used to
//! exchange semantic analysis information across crates.
//!
//! It intentionally does not parse Perl, implement LSP providers, or own workspace
//! persistence/indexing.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[non_exhaustive]
        pub struct $name(pub String);

        impl $name {
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<T> From<T> for $name
        where
            T: Into<String>,
        {
            fn from(value: T) -> Self {
                Self::new(value)
            }
        }
    };
}

typed_id!(FileId);
typed_id!(ScopeId);
typed_id!(EntityId);
typed_id!(AnchorId);
typed_id!(OccurrenceId);
typed_id!(EdgeId);
typed_id!(DiagnosticId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AnchorFact {
    pub id: AnchorId,
    pub file_id: FileId,
    pub scope_id: Option<ScopeId>,
    pub start_byte: u32,
    pub end_byte: u32,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EntityFact {
    pub id: EntityId,
    pub kind: EntityKind,
    pub canonical_name: String,
    pub display_name: String,
    pub defining_scope_id: Option<ScopeId>,
    pub anchor_id: Option<AnchorId>,
    pub provenance: Provenance,
    pub confidence: Confidence,
    pub attributes: BTreeMap<String, String>,
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
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DiagnosticFact {
    pub id: DiagnosticId,
    pub anchor_id: Option<AnchorId>,
    pub code: String,
    pub message: String,
    pub severity: String,
    pub provenance: Provenance,
    pub confidence: Confidence,
    pub attributes: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use anyhow::Result;

    use super::*;

    #[test]
    fn typed_ids_serialize_as_json_strings() -> Result<()> {
        let id = EntityId::new("entity:My::Class");
        let payload = serde_json::to_string(&id)?;

        assert_eq!(payload, "\"entity:My::Class\"");
        assert_eq!(id.as_str(), "entity:My::Class");

        Ok(())
    }

    #[test]
    fn entity_fact_roundtrips_with_deterministic_map_order() -> Result<()> {
        let mut attributes = BTreeMap::new();
        attributes.insert(String::from("zeta"), String::from("last"));
        attributes.insert(String::from("alpha"), String::from("first"));

        let fact = EntityFact {
            id: EntityId::from("entity:My::Role"),
            kind: EntityKind::Role,
            canonical_name: String::from("My::Role"),
            display_name: String::from("My::Role"),
            defining_scope_id: Some(ScopeId::from("scope:file:main")),
            anchor_id: Some(AnchorId::from("anchor:42")),
            provenance: Provenance::SemanticAnalyzer,
            confidence: Confidence::High,
            attributes,
        };

        let json = serde_json::to_string(&fact)?;
        let reparsed: EntityFact = serde_json::from_str(&json)?;

        assert_eq!(fact, reparsed);
        assert!(json.contains("\"alpha\":\"first\",\"zeta\":\"last\""));

        Ok(())
    }

    #[test]
    fn debug_output_is_stable_for_snapshot_style_assertions() {
        let mut attrs = BTreeMap::new();
        attrs.insert(String::from("framework"), String::from("moose"));

        let edge = EdgeFact {
            id: EdgeId::from("edge:1"),
            from_entity_id: EntityId::from("entity:A"),
            to_entity_id: EntityId::from("entity:B"),
            kind: EdgeKind::ComposesRole,
            anchor_id: None,
            provenance: Provenance::FrameworkSynthesis,
            confidence: Confidence::Medium,
            attributes: attrs,
        };

        let debug = format!("{edge:?}");

        assert!(debug.contains("ComposesRole"));
        assert!(debug.contains("framework"));
    }
}
