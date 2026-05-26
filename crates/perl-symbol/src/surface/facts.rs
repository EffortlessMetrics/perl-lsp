use crate::surface::decl::SymbolDecl;
use crate::surface::r#ref::{SymbolRef, SymbolRefKind};
use crate::types::SymbolKind;
use perl_semantic_facts::{
    AnchorFact, AnchorId, Confidence, EdgeFact, EdgeId, EdgeKind, EntityFact, EntityId, EntityKind,
    FileId, OccurrenceFact, OccurrenceId, OccurrenceKind, Provenance,
};
use serde::Serialize;
use std::collections::BTreeMap;

/// Declaration that the adapter could not project into an [`EntityFact`].
///
/// Emitted when a symbol kind has no [`EntityKind`] mapping yet, or when a
/// declared container is missing from the input set so the `Defines` edge
/// cannot be resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnsupportedDeclFact {
    /// Fully-qualified name of the declaration that was skipped.
    pub qualified_name: String,
    /// Original [`SymbolKind`] reported by the surface extractor.
    pub kind: SymbolKind,
    /// Human-readable reason the declaration was not projected.
    pub reason: &'static str,
}

/// Output of [`symbol_decls_to_semantic_facts`]: the semantic-facts projection
/// of a set of [`SymbolDecl`]s for one file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SymbolDeclSemanticFacts {
    /// Source-span anchors, one per projected declaration.
    pub anchors: Vec<AnchorFact>,
    /// Entity records produced from supported declaration kinds.
    pub entities: Vec<EntityFact>,
    /// `Defines` edges connecting containers to the entities they declare.
    pub defines_edges: Vec<EdgeFact>,
    /// Declarations skipped because they could not be projected (see [`UnsupportedDeclFact`]).
    pub unsupported: Vec<UnsupportedDeclFact>,
}

/// Output of [`symbol_refs_to_semantic_facts`]: occurrences and reference edges
/// derived from [`SymbolRef`]s for one file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SymbolRefSemanticFacts {
    /// Source-span anchors, one per reference.
    pub anchors: Vec<AnchorFact>,
    /// Occurrence records (read / call / method-call / etc.) for each reference.
    pub occurrences: Vec<OccurrenceFact>,
    /// `References` edges, emitted only when the reference resolves to a known entity.
    pub reference_edges: Vec<EdgeFact>,
}

/// Convert extracted [`SymbolRef`]s into the semantic-facts graph for `file_id`.
///
/// `entity_ids_by_qualified_name` provides entity IDs already minted for
/// declarations in this or other files; unresolved references produce
/// occurrences without `entity_id` and emit no reference edge.
pub fn symbol_refs_to_semantic_facts(
    refs: &[SymbolRef],
    file_id: FileId,
    entity_ids_by_qualified_name: &BTreeMap<String, EntityId>,
) -> SymbolRefSemanticFacts {
    let mut anchors = Vec::with_capacity(refs.len());
    let mut occurrences = Vec::with_capacity(refs.len());
    let mut reference_edges = Vec::new();

    for symbol_ref in refs {
        let occurrence_kind = occurrence_kind(&symbol_ref.kind);
        let provenance = occurrence_provenance(&symbol_ref.kind);
        let confidence = occurrence_confidence(&symbol_ref.kind);
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
            provenance,
            confidence,
        });

        let entity_id = entity_ids_by_qualified_name.get(&symbol_ref.qualified_name).copied();
        let occurrence_id = OccurrenceId(stable_id(
            "occurrence",
            &symbol_ref.qualified_name,
            symbol_ref.full_span.0,
            symbol_ref.full_span.1,
        ));
        occurrences.push(OccurrenceFact {
            id: occurrence_id,
            kind: occurrence_kind,
            entity_id,
            anchor_id,
            scope_id: None,
            provenance,
            confidence,
        });

        if let Some(to_entity_id) = entity_id {
            let from_entity_id =
                EntityId(stable_id("ref-source", &symbol_ref.qualified_name, 0, 0));
            reference_edges.push(EdgeFact {
                id: EdgeId(stable_id(
                    "references",
                    &symbol_ref.qualified_name,
                    from_entity_id.0 as usize,
                    to_entity_id.0 as usize,
                )),
                kind: EdgeKind::References,
                from_entity_id,
                to_entity_id,
                via_occurrence_id: Some(occurrence_id),
                provenance: Provenance::NameHeuristic,
                confidence: Confidence::Low,
            });
        }
    }

    SymbolRefSemanticFacts { anchors, occurrences, reference_edges }
}

fn occurrence_kind(kind: &SymbolRefKind) -> OccurrenceKind {
    match kind {
        SymbolRefKind::Variable(_) => OccurrenceKind::Read,
        SymbolRefKind::SubroutineCall => OccurrenceKind::Call,
        SymbolRefKind::MethodCall => OccurrenceKind::MethodCall,
        SymbolRefKind::StaticMethodCall => OccurrenceKind::StaticMethodCall,
        SymbolRefKind::CoderefReference => OccurrenceKind::CoderefReference,
        SymbolRefKind::TypeglobReference => OccurrenceKind::TypeglobReference,
    }
}

fn occurrence_provenance(kind: &SymbolRefKind) -> Provenance {
    match kind {
        SymbolRefKind::TypeglobReference => Provenance::DynamicBoundary,
        _ => Provenance::ExactAst,
    }
}

fn occurrence_confidence(kind: &SymbolRefKind) -> Confidence {
    match kind {
        SymbolRefKind::MethodCall => Confidence::Medium,
        SymbolRefKind::TypeglobReference => Confidence::Low,
        _ => Confidence::High,
    }
}

/// Convert extracted [`SymbolDecl`]s into the semantic-facts graph for `file_id`.
///
/// Unsupported kinds and unresolved containers are reported via the
/// `unsupported` field on the returned [`SymbolDeclSemanticFacts`] rather than
/// silently dropped, so callers can diagnose missing coverage.
pub fn symbol_decls_to_semantic_facts(
    decls: &[SymbolDecl],
    file_id: FileId,
) -> SymbolDeclSemanticFacts {
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
        let anchor_id =
            AnchorId(stable_id("anchor", &decl.qualified_name, anchor_span.0, anchor_span.1));
        anchors.push(AnchorFact {
            id: anchor_id,
            file_id,
            span_start_byte: anchor_span.0 as u32,
            span_end_byte: anchor_span.1 as u32,
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });

        let entity_id =
            EntityId(stable_id("entity", &decl.qualified_name, decl.full_span.0, decl.full_span.1));
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

        // Resolve the from-entity for this Defines edge.
        //
        // `container` is the *bare* enclosing package name (e.g. "Bar").
        // `qualified_name` may be multi-level (e.g. "Foo::Bar::baz"), so we
        // strip the last segment to obtain the qualified prefix ("Foo::Bar")
        // and use that as the lookup key when it is a proper segment-boundary
        // match for `container`.
        //
        // The check must be a *segment-boundary* match, not a raw byte-suffix
        // match: "FooBar".ends_with("Bar") is true but "FooBar" != "Bar" and
        // is not a qualified suffix "::Bar".
        let from_candidate = if let Some((prefix, _)) = decl.qualified_name.rsplit_once("::") {
            let is_segment_match =
                prefix == container.as_str() || prefix.ends_with(&format!("::{container}"));
            if is_segment_match { prefix.to_string() } else { container.clone() }
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

        let edge_id = EdgeId(stable_id(
            "defines",
            &decl.qualified_name,
            from_entity_id.0 as usize,
            to_entity_id.0 as usize,
        ));
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
    use perl_tdd_support::must_some;

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

    /// Regression test: container name that is a byte-suffix of a sibling
    /// package name must NOT produce a wrong Defines edge.
    ///
    /// Before the fix, `prefix.ends_with(container)` matched "FooBar" against
    /// container "Bar" and resolved to the wrong entity "FooBar" instead of
    /// falling back to the bare container name "Bar".
    #[test]
    fn container_resolution_uses_segment_boundary_not_byte_suffix() {
        // "FooBar" is a top-level package.
        // "FooBar::baz" has container "Bar" — unusual but representable
        // (imagine `package Bar { }` block inside a FooBar file context).
        // The correct from-entity key is "Bar", not "FooBar".
        let decls = vec![
            SymbolDecl {
                kind: SymbolKind::Package,
                name: "FooBar".to_string(),
                qualified_name: "FooBar".to_string(),
                full_span: (0, 10),
                anchor_span: None,
                container: None,
                declarator: None,
            },
            SymbolDecl {
                kind: SymbolKind::Package,
                name: "Bar".to_string(),
                qualified_name: "Bar".to_string(),
                full_span: (11, 20),
                anchor_span: None,
                container: None,
                declarator: None,
            },
            SymbolDecl {
                kind: SymbolKind::Subroutine,
                name: "baz".to_string(),
                qualified_name: "FooBar::baz".to_string(),
                full_span: (21, 40),
                anchor_span: Some((25, 28)),
                // container is "Bar" (bare), not "FooBar"
                container: Some("Bar".to_string()),
                declarator: None,
            },
        ];

        let facts = symbol_decls_to_semantic_facts(&decls, FileId(42));
        // "Bar" is present in entity_by_name, so the edge should resolve to Bar.
        // "FooBar" would be a wrong resolution.
        assert_eq!(facts.defines_edges.len(), 1, "should have exactly one Defines edge");
        let edge = &facts.defines_edges[0];
        let bar_entity = must_some(facts.entities.iter().find(|e| e.canonical_name == "Bar"));
        assert_eq!(
            edge.from_entity_id, bar_entity.id,
            "Defines edge must point FROM Bar (not FooBar)"
        );
        let baz_entity =
            must_some(facts.entities.iter().find(|e| e.canonical_name == "FooBar::baz"));
        assert_eq!(edge.to_entity_id, baz_entity.id, "Defines edge must point TO FooBar::baz");
        // No false unsupported reports — "Bar" container was found.
        assert!(
            facts.unsupported.is_empty(),
            "unsupported should be empty; got: {:?}",
            facts.unsupported
        );
    }

    /// When a decl's container is not itself present as a declaration, the
    /// adapter must emit an `UnsupportedDeclFact` explaining the missing edge
    /// rather than silently dropping the Defines record.
    #[test]
    fn missing_container_produces_unsupported_entry() {
        let decls = vec![SymbolDecl {
            kind: SymbolKind::Subroutine,
            name: "orphan".to_string(),
            qualified_name: "Missing::orphan".to_string(),
            full_span: (0, 30),
            anchor_span: Some((10, 16)),
            container: Some("Missing".to_string()),
            declarator: None,
        }];

        let facts = symbol_decls_to_semantic_facts(&decls, FileId(3));
        assert_eq!(facts.entities.len(), 1, "entity is still emitted");
        assert_eq!(facts.defines_edges.len(), 0, "no edge without container entity");
        assert_eq!(facts.unsupported.len(), 1, "one unsupported entry for missing container");
        assert_eq!(
            facts.unsupported[0].reason,
            "container declaration not present; Defines edge omitted",
        );
        assert_eq!(facts.unsupported[0].qualified_name, "Missing::orphan");
    }

    /// Deeply nested qualified names must resolve the container using a full
    /// segment-boundary match, not just the last segment.
    ///
    /// "A::B::C::D" with container "C" should produce a Defines edge
    /// FROM "A::B::C" (the qualified prefix that ends_with "::C") TO "A::B::C::D".
    #[test]
    fn deeply_nested_qualified_name_resolves_correct_container() {
        let decls = vec![
            SymbolDecl {
                kind: SymbolKind::Package,
                name: "A".to_string(),
                qualified_name: "A".to_string(),
                full_span: (0, 5),
                anchor_span: None,
                container: None,
                declarator: None,
            },
            SymbolDecl {
                kind: SymbolKind::Package,
                name: "B".to_string(),
                qualified_name: "A::B".to_string(),
                full_span: (6, 15),
                anchor_span: None,
                container: Some("A".to_string()),
                declarator: None,
            },
            SymbolDecl {
                kind: SymbolKind::Package,
                name: "C".to_string(),
                qualified_name: "A::B::C".to_string(),
                full_span: (16, 30),
                anchor_span: None,
                container: Some("B".to_string()),
                declarator: None,
            },
            SymbolDecl {
                kind: SymbolKind::Subroutine,
                name: "D".to_string(),
                qualified_name: "A::B::C::D".to_string(),
                full_span: (31, 55),
                anchor_span: Some((35, 36)),
                container: Some("C".to_string()),
                declarator: None,
            },
        ];

        let facts = symbol_decls_to_semantic_facts(&decls, FileId(5));
        assert_eq!(facts.entities.len(), 4);
        // A has no container -> no edge
        // A::B has container "A" -> edge A -> A::B
        // A::B::C has container "B" -> prefix "A::B", ends_with("::B") -> edge A::B -> A::B::C
        // A::B::C::D has container "C" -> prefix "A::B::C", ends_with("::C") -> edge A::B::C -> A::B::C::D
        assert_eq!(facts.defines_edges.len(), 3, "should have 3 Defines edges");
        assert!(facts.unsupported.is_empty(), "no unsupported entries");

        let c_entity = must_some(facts.entities.iter().find(|e| e.canonical_name == "A::B::C"));
        let d_entity = must_some(facts.entities.iter().find(|e| e.canonical_name == "A::B::C::D"));
        let d_edge = must_some(facts.defines_edges.iter().find(|e| e.to_entity_id == d_entity.id));
        assert_eq!(
            d_edge.from_entity_id, c_entity.id,
            "Defines edge for A::B::C::D must come FROM A::B::C"
        );
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
        assert_eq!(
            facts.unsupported[0].reason,
            "symbol kind is not yet representable as EntityFact"
        );
    }

    #[test]
    fn symbol_ref_adapter_emits_occurrences_and_optional_reference_edges() {
        use crate::types::VarKind;

        let refs = vec![
            SymbolRef {
                kind: SymbolRefKind::SubroutineCall,
                name: "run".to_string(),
                qualified_name: "Foo::run".to_string(),
                sigil: None,
                package_qualifier: Some("Foo".to_string()),
                full_span: (10, 20),
                anchor_span: Some((12, 15)),
            },
            SymbolRef {
                kind: SymbolRefKind::Variable(VarKind::Scalar),
                name: "x".to_string(),
                qualified_name: "x".to_string(),
                sigil: Some("$".to_string()),
                package_qualifier: None,
                full_span: (21, 23),
                anchor_span: None,
            },
            SymbolRef {
                kind: SymbolRefKind::StaticMethodCall,
                name: "new".to_string(),
                qualified_name: "Foo::new".to_string(),
                sigil: None,
                package_qualifier: Some("Foo".to_string()),
                full_span: (24, 32),
                anchor_span: None,
            },
            SymbolRef {
                kind: SymbolRefKind::MethodCall,
                name: "save".to_string(),
                qualified_name: "save".to_string(),
                sigil: None,
                package_qualifier: None,
                full_span: (33, 45),
                anchor_span: None,
            },
            SymbolRef {
                kind: SymbolRefKind::CoderefReference,
                name: "callback".to_string(),
                qualified_name: "Foo::callback".to_string(),
                sigil: Some("&".to_string()),
                package_qualifier: Some("Foo".to_string()),
                full_span: (46, 59),
                anchor_span: Some((47, 59)),
            },
            SymbolRef {
                kind: SymbolRefKind::TypeglobReference,
                name: "alias".to_string(),
                qualified_name: "alias".to_string(),
                sigil: Some("*".to_string()),
                package_qualifier: None,
                full_span: (60, 66),
                anchor_span: None,
            },
        ];
        let mut entity_map = BTreeMap::new();
        entity_map.insert("Foo::run".to_string(), EntityId(42));
        entity_map.insert("Foo::new".to_string(), EntityId(43));
        entity_map.insert("Foo::callback".to_string(), EntityId(44));

        let facts = symbol_refs_to_semantic_facts(&refs, FileId(7), &entity_map);
        assert_eq!(facts.anchors.len(), 6);
        assert_eq!(facts.occurrences.len(), 6);
        assert_eq!(facts.reference_edges.len(), 3);
        assert_eq!(facts.occurrences[0].kind, OccurrenceKind::Call);
        assert_eq!(facts.occurrences[1].kind, OccurrenceKind::Read);
        assert_eq!(facts.occurrences[2].kind, OccurrenceKind::StaticMethodCall);
        assert_eq!(facts.occurrences[3].kind, OccurrenceKind::MethodCall);
        assert_eq!(facts.occurrences[4].kind, OccurrenceKind::CoderefReference);
        assert_eq!(facts.occurrences[5].kind, OccurrenceKind::TypeglobReference);
        assert_eq!(facts.occurrences[0].entity_id, Some(EntityId(42)));
        assert_eq!(facts.occurrences[1].entity_id, None);
        assert_eq!(facts.occurrences[2].entity_id, Some(EntityId(43)));
        assert_eq!(facts.occurrences[3].entity_id, None);
        assert_eq!(facts.occurrences[4].entity_id, Some(EntityId(44)));
        assert_eq!(facts.occurrences[5].entity_id, None);
        assert_eq!(facts.occurrences[2].confidence, Confidence::High);
        assert_eq!(facts.occurrences[3].confidence, Confidence::Medium);
        assert_eq!(facts.occurrences[4].confidence, Confidence::High);
        assert_eq!(facts.occurrences[5].confidence, Confidence::Low);
        assert_eq!(facts.occurrences[5].provenance, Provenance::DynamicBoundary);
    }
}
