use crate::surface::decl::SymbolDecl;
use crate::types::SymbolKind;
use perl_semantic_facts::{
    AnchorFact, AnchorId, Confidence, EdgeFact, EdgeId, EdgeKind, EntityFact, EntityId, EntityKind,
    FileId, Provenance,
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SymbolDeclFacts {
    pub anchors: Vec<AnchorFact>,
    pub entities: Vec<EntityFact>,
    pub edges: Vec<EdgeFact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolDeclFactError {
    UnsupportedSymbolKind(SymbolKind),
    SpanOutOfRange { start: usize, end: usize },
}

pub fn symbol_decls_to_facts(
    decls: &[SymbolDecl],
    file_id: FileId,
) -> Result<SymbolDeclFacts, SymbolDeclFactError> {
    let mut anchors = Vec::new();
    let mut entities = Vec::new();
    let mut edges = Vec::new();

    for (idx, decl) in decls.iter().enumerate() {
        let entity_id = EntityId((idx as u64) + 1);
        let anchor_id = AnchorId((idx as u64) + 1);
        let edge_id = EdgeId((idx as u64) + 1);

        let (start, end) = decl.anchor_span.unwrap_or(decl.full_span);
        let span_start_byte = u32::try_from(start)
            .map_err(|_| SymbolDeclFactError::SpanOutOfRange { start, end })?;
        let span_end_byte =
            u32::try_from(end).map_err(|_| SymbolDeclFactError::SpanOutOfRange { start, end })?;

        anchors.push(AnchorFact {
            id: anchor_id,
            file_id,
            span_start_byte,
            span_end_byte,
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });

        entities.push(EntityFact {
            id: entity_id,
            kind: entity_kind_from_symbol_kind(decl.kind)?,
            canonical_name: decl.qualified_name.clone(),
            anchor_id: Some(anchor_id),
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });

        edges.push(EdgeFact {
            id: edge_id,
            kind: EdgeKind::Defines,
            from_entity_id: entity_id,
            to_entity_id: entity_id,
            via_occurrence_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });
    }

    Ok(SymbolDeclFacts { anchors, entities, edges })
}

fn entity_kind_from_symbol_kind(kind: SymbolKind) -> Result<EntityKind, SymbolDeclFactError> {
    Ok(match kind {
        SymbolKind::Package => EntityKind::Package,
        SymbolKind::Class => EntityKind::Class,
        SymbolKind::Subroutine => EntityKind::Subroutine,
        SymbolKind::Method => EntityKind::Method,
        SymbolKind::Variable(_) => EntityKind::Variable,
        SymbolKind::Constant => EntityKind::Constant,
        SymbolKind::Label => EntityKind::Label,
        SymbolKind::Format => EntityKind::Format,
        SymbolKind::Role | SymbolKind::Import | SymbolKind::Export => {
            return Err(SymbolDeclFactError::UnsupportedSymbolKind(kind));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::VarKind;

    #[test]
    fn conversion_is_deterministic_and_stable() -> Result<(), serde_json::Error> {
        let decls = vec![
            SymbolDecl {
                kind: SymbolKind::Package,
                name: "Foo".to_string(),
                qualified_name: "Foo".to_string(),
                full_span: (0, 10),
                anchor_span: Some((8, 11)),
                container: None,
                declarator: None,
            },
            SymbolDecl {
                kind: SymbolKind::Subroutine,
                name: "greet".to_string(),
                qualified_name: "Foo::greet".to_string(),
                full_span: (12, 30),
                anchor_span: Some((16, 21)),
                container: Some("Foo".to_string()),
                declarator: None,
            },
            SymbolDecl {
                kind: SymbolKind::Variable(VarKind::Scalar),
                name: "count".to_string(),
                qualified_name: "Foo::count".to_string(),
                full_span: (31, 42),
                anchor_span: Some((34, 40)),
                container: Some("Foo".to_string()),
                declarator: Some("my".to_string()),
            },
        ];

        let facts = match symbol_decls_to_facts(&decls, FileId(9)) {
            Ok(facts) => facts,
            Err(err) => panic!("unexpected conversion error: {err:?}"),
        };
        let json = serde_json::to_string_pretty(&facts)?;

        assert_eq!(json, r#"{
  "anchors": [
    {
      "id": 1,
      "file_id": 9,
      "span_start_byte": 8,
      "span_end_byte": 11,
      "scope_id": null,
      "provenance": "ExactAst",
      "confidence": "High"
    },
    {
      "id": 2,
      "file_id": 9,
      "span_start_byte": 16,
      "span_end_byte": 21,
      "scope_id": null,
      "provenance": "ExactAst",
      "confidence": "High"
    },
    {
      "id": 3,
      "file_id": 9,
      "span_start_byte": 34,
      "span_end_byte": 40,
      "scope_id": null,
      "provenance": "ExactAst",
      "confidence": "High"
    }
  ],
  "entities": [
    {
      "id": 1,
      "kind": "Package",
      "canonical_name": "Foo",
      "anchor_id": 1,
      "scope_id": null,
      "provenance": "ExactAst",
      "confidence": "High"
    },
    {
      "id": 2,
      "kind": "Subroutine",
      "canonical_name": "Foo::greet",
      "anchor_id": 2,
      "scope_id": null,
      "provenance": "ExactAst",
      "confidence": "High"
    },
    {
      "id": 3,
      "kind": "Variable",
      "canonical_name": "Foo::count",
      "anchor_id": 3,
      "scope_id": null,
      "provenance": "ExactAst",
      "confidence": "High"
    }
  ],
  "edges": [
    {
      "id": 1,
      "kind": "Defines",
      "from_entity_id": 1,
      "to_entity_id": 1,
      "via_occurrence_id": null,
      "provenance": "ExactAst",
      "confidence": "High"
    },
    {
      "id": 2,
      "kind": "Defines",
      "from_entity_id": 2,
      "to_entity_id": 2,
      "via_occurrence_id": null,
      "provenance": "ExactAst",
      "confidence": "High"
    },
    {
      "id": 3,
      "kind": "Defines",
      "from_entity_id": 3,
      "to_entity_id": 3,
      "via_occurrence_id": null,
      "provenance": "ExactAst",
      "confidence": "High"
    }
  ]
}"#);

        Ok(())
    }

    #[test]
    fn unsupported_symbol_kind_is_explicit() {
        let decls = vec![SymbolDecl {
            kind: SymbolKind::Role,
            name: "R".to_string(),
            qualified_name: "R".to_string(),
            full_span: (0, 1),
            anchor_span: Some((0, 1)),
            container: None,
            declarator: None,
        }];

        let result = symbol_decls_to_facts(&decls, FileId(1));
        assert_eq!(result, Err(SymbolDeclFactError::UnsupportedSymbolKind(SymbolKind::Role)));
    }
}
