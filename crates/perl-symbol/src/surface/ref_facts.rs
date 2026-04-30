use crate::surface::r#ref::{SymbolRef, SymbolRefKind};
use perl_semantic_facts::{
    AnchorFact, AnchorId, Confidence, EdgeFact, EdgeId, EdgeKind, EntityId, FileId, OccurrenceFact,
    OccurrenceId, OccurrenceKind, Provenance,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRefSemanticFacts {
    pub anchors: Vec<AnchorFact>,
    pub occurrences: Vec<OccurrenceFact>,
    pub references_edges: Vec<EdgeFact>,
}

pub fn symbol_refs_to_semantic_facts(
    refs: &[SymbolRef],
    file_id: FileId,
    resolve_entity_id: impl Fn(&SymbolRef) -> Option<EntityId>,
) -> SymbolRefSemanticFacts {
    let mut anchors = Vec::with_capacity(refs.len());
    let mut occurrences = Vec::with_capacity(refs.len());
    let mut references_edges = Vec::new();

    for symbol_ref in refs {
        let anchor_span = symbol_ref.anchor_span.unwrap_or(symbol_ref.full_span);
        let anchor_id = AnchorId(stable_id(
            "ref_anchor",
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

        let entity_id = resolve_entity_id(symbol_ref);
        let occurrence_id = OccurrenceId(stable_id(
            "ref_occurrence",
            &symbol_ref.qualified_name,
            symbol_ref.full_span.0,
            symbol_ref.full_span.1,
        ));
        occurrences.push(OccurrenceFact {
            id: occurrence_id,
            kind: occurrence_kind(symbol_ref),
            entity_id,
            anchor_id,
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });

        if let Some(to_entity_id) = entity_id {
            let from_entity_id = EntityId(stable_id(
                "ref_source_entity",
                &symbol_ref.qualified_name,
                symbol_ref.full_span.0,
                symbol_ref.full_span.1,
            ));
            references_edges.push(EdgeFact {
                id: EdgeId(stable_id(
                    "references_edge",
                    &symbol_ref.qualified_name,
                    from_entity_id.0 as usize,
                    to_entity_id.0 as usize,
                )),
                kind: EdgeKind::References,
                from_entity_id,
                to_entity_id,
                via_occurrence_id: Some(occurrence_id),
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            });
        }
    }

    SymbolRefSemanticFacts { anchors, occurrences, references_edges }
}

fn occurrence_kind(symbol_ref: &SymbolRef) -> OccurrenceKind {
    match symbol_ref.kind {
        SymbolRefKind::Variable(_) => OccurrenceKind::Reference,
        SymbolRefKind::SubroutineCall => OccurrenceKind::Call,
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
    fn symbol_refs_emit_occurrences_and_optional_reference_edges() {
        let refs = vec![
            SymbolRef {
                kind: SymbolRefKind::SubroutineCall,
                name: "run".into(),
                qualified_name: "Foo::run".into(),
                sigil: None,
                package_qualifier: Some("Foo".into()),
                full_span: (4, 12),
                anchor_span: Some((4, 7)),
            },
            SymbolRef {
                kind: SymbolRefKind::Variable(VarKind::Scalar),
                name: "x".into(),
                qualified_name: "x".into(),
                sigil: Some("$".into()),
                package_qualifier: None,
                full_span: (13, 15),
                anchor_span: Some((13, 15)),
            },
        ];

        let facts = symbol_refs_to_semantic_facts(&refs, FileId(1), |r| {
            (r.qualified_name == "Foo::run").then_some(EntityId(7))
        });
        assert_eq!(facts.anchors.len(), 2);
        assert_eq!(facts.occurrences.len(), 2);
        assert_eq!(facts.references_edges.len(), 1);
        assert_eq!(facts.occurrences[0].kind, OccurrenceKind::Call);
        assert_eq!(facts.occurrences[1].kind, OccurrenceKind::Reference);
    }
}
