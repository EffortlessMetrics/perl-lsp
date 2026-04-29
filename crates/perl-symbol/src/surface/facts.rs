use crate::{SymbolDecl, SymbolKind};
use perl_semantic_facts::{
    AnchorFact, AnchorId, Confidence, EdgeFact, EdgeId, EdgeKind, EntityFact, EntityId, EntityKind,
    FileId, Provenance,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclFactSet {
    pub anchors: Vec<AnchorFact>,
    pub entities: Vec<EntityFact>,
    pub edges: Vec<EdgeFact>,
    pub unsupported: Vec<UnsupportedDeclKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedDeclKind {
    pub symbol_kind: SymbolKind,
    pub qualified_name: String,
}

pub fn symbol_decls_to_facts(decls: &[SymbolDecl], file_id: FileId) -> DeclFactSet {
    let mut anchors = Vec::new();
    let mut entities = Vec::new();
    let mut edges = Vec::new();
    let mut unsupported = Vec::new();

    for decl in decls {
        let Some(entity_kind) = symbol_kind_to_entity_kind(decl.kind) else {
            unsupported.push(UnsupportedDeclKind {
                symbol_kind: decl.kind,
                qualified_name: decl.qualified_name.clone(),
            });
            continue;
        };

        let entity_id = EntityId(stable_id("entity", &decl.qualified_name, decl.full_span));
        let anchor_id =
            decl.anchor_span.map(|span| AnchorId(stable_id("anchor", &decl.qualified_name, span)));

        if let Some((start, end)) = decl.anchor_span {
            anchors.push(AnchorFact {
                id: AnchorId(stable_id("anchor", &decl.qualified_name, (start, end))),
                file_id,
                span_start_byte: start as u32,
                span_end_byte: end as u32,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            });
        }

        entities.push(EntityFact {
            id: entity_id,
            kind: entity_kind,
            canonical_name: decl.qualified_name.clone(),
            anchor_id,
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });

        edges.push(EdgeFact {
            id: EdgeId(stable_id("defines", &decl.qualified_name, decl.full_span)),
            kind: EdgeKind::Defines,
            from_entity_id: entity_id,
            to_entity_id: entity_id,
            via_occurrence_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });
    }

    DeclFactSet { anchors, entities, edges, unsupported }
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

fn stable_id(kind: &str, name: &str, span: (usize, usize)) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in format!("{kind}|{name}|{}|{}", span.0, span.1).bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VarKind;

    #[test]
    fn adapter_marks_unsupported_kinds_explicitly() {
        let decl = SymbolDecl {
            kind: SymbolKind::Role,
            name: "RoleX".to_string(),
            qualified_name: "RoleX".to_string(),
            full_span: (1, 2),
            anchor_span: Some((1, 2)),
            container: None,
            declarator: None,
        };
        let facts = symbol_decls_to_facts(&[decl], FileId(1));
        assert_eq!(facts.entities.len(), 0);
        assert_eq!(facts.unsupported.len(), 1);
    }

    #[test]
    fn adapter_maps_supported_kinds() {
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
                kind: SymbolKind::Variable(VarKind::Scalar),
                name: "x".to_string(),
                qualified_name: "Foo::x".to_string(),
                full_span: (12, 20),
                anchor_span: Some((15, 17)),
                container: Some("Foo".to_string()),
                declarator: Some("my".to_string()),
            },
        ];

        let facts = symbol_decls_to_facts(&decls, FileId(9));
        assert_eq!(facts.entities.len(), 2);
        assert_eq!(facts.anchors.len(), 2);
        assert_eq!(facts.edges.len(), 2);
        assert!(facts.unsupported.is_empty());
    }
}
