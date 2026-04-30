use crate::surface::r#ref::{SymbolRef, SymbolRefKind};
use perl_semantic_facts::{
    AnchorFact, AnchorId, Confidence, EdgeFact, EdgeId, EdgeKind, EntityId, FileId, OccurrenceFact,
    OccurrenceId, OccurrenceKind, Provenance,
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnsupportedRefFact {
    pub qualified_name: String,
    pub reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SymbolRefSemanticFacts {
    pub anchors: Vec<AnchorFact>,
    pub occurrences: Vec<OccurrenceFact>,
    pub reference_edges: Vec<EdgeFact>,
    pub unsupported: Vec<UnsupportedRefFact>,
}

pub fn symbol_refs_to_semantic_facts(
    refs: &[SymbolRef],
    file_id: FileId,
    entity_by_name: &BTreeMap<String, EntityId>,
) -> SymbolRefSemanticFacts {
    let mut anchors = Vec::with_capacity(refs.len());
    let mut occurrences = Vec::with_capacity(refs.len());
    let mut reference_edges = Vec::new();
    let mut unsupported = Vec::new();

    for symbol_ref in refs {
        let anchor_span = symbol_ref.anchor_span.unwrap_or(symbol_ref.full_span);
        let anchor_id = AnchorId(stable_id(
            "ref-anchor",
            &symbol_ref.qualified_name,
            anchor_span.0,
            anchor_span.1,
        ));
        anchors.push(AnchorFact {
            id: anchor_id,
            file_id,
            span_start_byte: anchor_span.0 as u32,
            span_end_byte: anchor_span.1 as u32,
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });

        let occurrence_id = OccurrenceId(stable_id(
            "occurrence",
            &symbol_ref.qualified_name,
            symbol_ref.full_span.0,
            symbol_ref.full_span.1,
        ));
        let occurrence_kind = match symbol_ref.kind {
            SymbolRefKind::Variable(_) => OccurrenceKind::Reference,
            SymbolRefKind::SubroutineCall => OccurrenceKind::Call,
        };
        let target_entity_id = entity_by_name.get(&symbol_ref.qualified_name).copied();
        occurrences.push(OccurrenceFact {
            id: occurrence_id,
            kind: occurrence_kind,
            entity_id: target_entity_id,
            anchor_id,
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });

        if let Some(entity_id) = target_entity_id {
            let edge_id = EdgeId(stable_id(
                "ref-edge",
                &symbol_ref.qualified_name,
                occurrence_id.0 as usize,
                entity_id.0 as usize,
            ));
            reference_edges.push(EdgeFact {
                id: edge_id,
                kind: match symbol_ref.kind {
                    SymbolRefKind::Variable(_) => EdgeKind::References,
                    SymbolRefKind::SubroutineCall => EdgeKind::Calls,
                },
                from_entity_id: entity_id,
                to_entity_id: entity_id,
                via_occurrence_id: Some(occurrence_id),
                provenance: Provenance::NameHeuristic,
                confidence: Confidence::Low,
            });
        } else {
            unsupported.push(UnsupportedRefFact {
                qualified_name: symbol_ref.qualified_name.clone(),
                reason: "symbol reference could not be linked to a known entity",
            });
        }
    }

    SymbolRefSemanticFacts { anchors, occurrences, reference_edges, unsupported }
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
    fn symbol_ref_adapter_is_deterministic() {
        let refs = vec![SymbolRef {
            kind: SymbolRefKind::Variable(VarKind::Scalar),
            name: "x".to_string(),
            qualified_name: "Foo::x".to_string(),
            sigil: Some("$".to_string()),
            package_qualifier: Some("Foo".to_string()),
            full_span: (10, 16),
            anchor_span: Some((11, 15)),
        }];
        let mut entity_by_name = BTreeMap::new();
        entity_by_name.insert("Foo::x".to_string(), EntityId(5));

        let facts = symbol_refs_to_semantic_facts(&refs, FileId(1), &entity_by_name);
        let again = symbol_refs_to_semantic_facts(&refs, FileId(1), &entity_by_name);
        assert_eq!(facts, again);
        assert_eq!(facts.anchors.len(), 1);
        assert_eq!(facts.occurrences.len(), 1);
        assert_eq!(facts.reference_edges.len(), 1);
        assert!(facts.unsupported.is_empty());
    }
}
