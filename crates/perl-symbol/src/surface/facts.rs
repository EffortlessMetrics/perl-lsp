use crate::surface::decl::SymbolDecl;
use crate::types::SymbolKind;
use perl_semantic_facts::{
    AnchorFact, AnchorId, Confidence, EdgeFact, EdgeId, EdgeKind, EntityFact, EntityId, EntityKind,
    FileId, Provenance,
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnsupportedDeclFact {
    pub qualified_name: String,
    pub kind: SymbolKind,
    pub reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SymbolDeclSemanticFacts {
    pub anchors: Vec<AnchorFact>,
    pub entities: Vec<EntityFact>,
    pub defines_edges: Vec<EdgeFact>,
    pub unsupported: Vec<UnsupportedDeclFact>,
}

pub fn symbol_decls_to_semantic_facts(decls: &[SymbolDecl], file_id: FileId) -> SymbolDeclSemanticFacts {
    let mut anchors = Vec::with_capacity(decls.len());
    let mut entities = Vec::with_capacity(decls.len());
    let mut unsupported = Vec::new();

    let mut entity_by_name = BTreeMap::new();
    for decl in decls {
        let entity_kind = match symbol_kind_to_entity_kind(decl.kind) {
            Some(kind) => kind,
            None => {
                unsupported.push(UnsupportedDeclFact {
                    qualified_name: decl.qualified_name.clone(),
                    kind: decl.kind,
                    reason: "symbol kind is not yet representable as EntityFact",
                });
                continue;
            }
        };

        let anchor_span = decl.anchor_span.unwrap_or(decl.full_span);
        let anchor_id = AnchorId(stable_id("anchor", &decl.qualified_name, anchor_span.0, anchor_span.1));
        anchors.push(AnchorFact {
            id: anchor_id,
            file_id,
            span_start_byte: anchor_span.0 as u32,
            span_end_byte: anchor_span.1 as u32,
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });

        let entity_id = EntityId(stable_id(
            "entity",
            &decl.qualified_name,
            decl.full_span.0,
            decl.full_span.1,
        ));
        entity_by_name.insert(decl.qualified_name.clone(), entity_id);
        entities.push(EntityFact {
            id: entity_id,
            kind: entity_kind,
            canonical_name: decl.qualified_name.clone(),
            anchor_id: Some(anchor_id),
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });
    }

    let mut defines_edges = Vec::new();
    for decl in decls {
        let Some(to_entity_id) = entity_by_name.get(&decl.qualified_name).copied() else {
            continue;
        };

        let Some(container) = &decl.container else {
            continue;
        };

        let from_candidate = if let Some((prefix, _)) = decl.qualified_name.rsplit_once("::") {
            if prefix.ends_with(container) {
                prefix.to_string()
            } else {
                container.clone()
            }
        } else {
            container.clone()
        };

        let Some(from_entity_id) = entity_by_name.get(&from_candidate).copied() else {
            unsupported.push(UnsupportedDeclFact {
                qualified_name: decl.qualified_name.clone(),
                kind: decl.kind,
                reason: "container declaration not present; Defines edge omitted",
            });
            continue;
        };

        let edge_id = EdgeId(stable_id("defines", &decl.qualified_name, from_entity_id.0 as usize, to_entity_id.0 as usize));
        defines_edges.push(EdgeFact {
            id: edge_id,
            kind: EdgeKind::Defines,
            from_entity_id,
            to_entity_id,
            via_occurrence_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });
    }

    SymbolDeclSemanticFacts { anchors, entities, defines_edges, unsupported }
}

fn symbol_kind_to_entity_kind(kind: SymbolKind) -> Option<EntityKind> {
    match kind {
        SymbolKind::Package => Some(EntityKind::Package),
        SymbolKind::Class => Some(EntityKind::Class),
        SymbolKind::Subroutine => Some(EntityKind::Subroutine),
        SymbolKind::Method => Some(EntityKind::Method),
        SymbolKind::Variable(_) => Some(EntityKind::Variable),
        SymbolKind::Constant => Some(EntityKind::Constant),
        SymbolKind::Label => Some(EntityKind::Label),
        SymbolKind::Format => Some(EntityKind::Format),
        SymbolKind::Role | SymbolKind::Import | SymbolKind::Export => None,
    }
}

fn stable_id(namespace: &str, name: &str, start: usize, end: usize) -> u64 {
    let mut hash = 14695981039346656037u64;
    for byte in namespace
        .as_bytes()
        .iter()
        .chain([0xff].iter())
        .chain(name.as_bytes().iter())
        .chain([0xff].iter())
        .chain(start.to_le_bytes().iter())
        .chain(end.to_le_bytes().iter())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::VarKind;

    #[test]
    fn adapter_is_deterministic_for_mixed_decls() {
        let decls = vec![
            SymbolDecl {
                kind: SymbolKind::Package,
                name: "Foo".to_string(),
                qualified_name: "Foo".to_string(),
                full_span: (0, 20),
                anchor_span: Some((8, 11)),
                container: None,
                declarator: None,
            },
            SymbolDecl {
                kind: SymbolKind::Subroutine,
                name: "run".to_string(),
                qualified_name: "Foo::run".to_string(),
                full_span: (21, 50),
                anchor_span: Some((25, 28)),
                container: Some("Foo".to_string()),
                declarator: None,
            },
            SymbolDecl {
                kind: SymbolKind::Variable(VarKind::Scalar),
                name: "value".to_string(),
                qualified_name: "Foo::value".to_string(),
                full_span: (51, 70),
                anchor_span: Some((54, 60)),
                container: Some("Foo".to_string()),
                declarator: Some("our".to_string()),
            },
            SymbolDecl {
                kind: SymbolKind::Label,
                name: "LOOP".to_string(),
                qualified_name: "LOOP".to_string(),
                full_span: (71, 95),
                anchor_span: None,
                container: Some("Foo".to_string()),
                declarator: None,
            },
        ];

        let actual = symbol_decls_to_semantic_facts(&decls, FileId(9));
        let again = symbol_decls_to_semantic_facts(&decls, FileId(9));
        assert_eq!(actual, again);
        assert_eq!(actual.anchors.len(), 4);
        assert_eq!(actual.entities.len(), 4);
        assert_eq!(actual.defines_edges.len(), 3);
        assert!(actual.unsupported.is_empty());
    }

    #[test]
    fn unsupported_kinds_are_reported_explicitly() {
        let decls = vec![SymbolDecl {
            kind: SymbolKind::Role,
            name: "MyRole".to_string(),
            qualified_name: "MyRole".to_string(),
            full_span: (0, 10),
            anchor_span: Some((5, 10)),
            container: None,
            declarator: None,
        }];

        let facts = symbol_decls_to_semantic_facts(&decls, FileId(1));
        assert!(facts.anchors.is_empty());
        assert!(facts.entities.is_empty());
        assert!(facts.defines_edges.is_empty());
        assert_eq!(facts.unsupported.len(), 1);
        assert_eq!(facts.unsupported[0].reason, "symbol kind is not yet representable as EntityFact");
    }
}
