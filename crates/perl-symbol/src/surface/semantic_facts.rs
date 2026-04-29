use crate::surface::decl::SymbolDecl;
use crate::types::SymbolKind;
use perl_semantic_facts::{
    AnchorFact, AnchorId, Confidence, EdgeFact, EdgeId, EdgeKind, EntityFact, EntityId, EntityKind,
    FileId, Provenance,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolDeclSemanticFacts {
    pub anchors: Vec<AnchorFact>,
    pub entities: Vec<EntityFact>,
    pub edges: Vec<EdgeFact>,
    pub unsupported: Vec<UnsupportedSymbolDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedSymbolDecl {
    pub symbol_name: String,
    pub symbol_kind: SymbolKind,
}

pub fn symbol_decls_to_semantic_facts(
    decls: &[SymbolDecl],
    file_id: FileId,
    scope_id: Option<perl_semantic_facts::ScopeId>,
) -> SymbolDeclSemanticFacts {
    let mut anchors = Vec::new();
    let mut entities = Vec::new();
    let mut edges = Vec::new();
    let mut unsupported = Vec::new();

    for decl in decls {
        let Some(entity_kind) = map_entity_kind(decl.kind) else {
            unsupported.push(UnsupportedSymbolDecl {
                symbol_name: decl.qualified_name.clone(),
                symbol_kind: decl.kind,
            });
            continue;
        };

        let entity_id = EntityId(stable_id("entity", file_id.0, decl));
        let synthetic_span = decl.anchor_span.unwrap_or(decl.full_span);
        let anchor_id = AnchorId(stable_id_with_span("anchor", file_id.0, decl, synthetic_span));

        anchors.push(AnchorFact {
            id: anchor_id,
            file_id,
            span_start_byte: synthetic_span.0 as u32,
            span_end_byte: synthetic_span.1 as u32,
            scope_id,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });

        entities.push(EntityFact {
            id: entity_id,
            kind: entity_kind,
            canonical_name: decl.qualified_name.clone(),
            anchor_id: Some(anchor_id),
            scope_id,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });

        edges.push(EdgeFact {
            id: EdgeId(stable_id("edge_defines", file_id.0, decl)),
            kind: EdgeKind::Defines,
            from_entity_id: entity_id,
            to_entity_id: entity_id,
            via_occurrence_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });
    }

    SymbolDeclSemanticFacts { anchors, entities, edges, unsupported }
}

fn map_entity_kind(kind: SymbolKind) -> Option<EntityKind> {
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

fn stable_id(label: &str, file_id: u64, decl: &SymbolDecl) -> u64 {
    let mut out = 0xcbf29ce484222325u64;
    hash_mix(&mut out, label.as_bytes());
    hash_mix(&mut out, &file_id.to_le_bytes());
    hash_mix(&mut out, decl.name.as_bytes());
    hash_mix(&mut out, decl.qualified_name.as_bytes());
    hash_mix(&mut out, &(decl.full_span.0 as u64).to_le_bytes());
    hash_mix(&mut out, &(decl.full_span.1 as u64).to_le_bytes());
    out
}

fn stable_id_with_span(label: &str, file_id: u64, decl: &SymbolDecl, span: (usize, usize)) -> u64 {
    let mut out = stable_id(label, file_id, decl);
    hash_mix(&mut out, &(span.0 as u64).to_le_bytes());
    hash_mix(&mut out, &(span.1 as u64).to_le_bytes());
    out
}

fn hash_mix(state: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *state ^= u64::from(*b);
        *state = state.wrapping_mul(0x100000001b3);
    }
}
