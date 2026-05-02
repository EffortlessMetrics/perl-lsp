//! Property-based tests for fact-to-anchor referential integrity (Property 2).
//!
//! **Validates: Requirements 2.2, 2.3**
//!
//! **Property 2: Fact-to-Anchor Referential Integrity** — Every emitted
//! EntityFact has an `anchor_id` referencing an AnchorFact in the same result
//! set. Every emitted OccurrenceFact has an `anchor_id` referencing an
//! AnchorFact in the same result set.

use perl_semantic_facts::{AnchorId, EntityId, FileId};
use perl_symbol::{
    SymbolDecl, SymbolKind, SymbolRef, SymbolRefKind, VarKind, symbol_decls_to_semantic_facts,
    symbol_refs_to_semantic_facts,
};
use proptest::prelude::*;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Strategies (reused from prop_adapter_determinism.rs)
// ---------------------------------------------------------------------------

/// Generate a random `VarKind`.
fn arb_var_kind() -> impl Strategy<Value = VarKind> {
    prop_oneof![Just(VarKind::Scalar), Just(VarKind::Array), Just(VarKind::Hash),]
}

/// Generate a `SymbolKind` that the adapter can convert to an `EntityKind`.
///
/// We include both supported kinds (Package, Class, Subroutine, Method,
/// Variable, Constant, Label, Format) and unsupported kinds (Role, Import,
/// Export) so the adapter exercises both paths.
fn arb_symbol_kind() -> impl Strategy<Value = SymbolKind> {
    prop_oneof![
        Just(SymbolKind::Package),
        Just(SymbolKind::Class),
        Just(SymbolKind::Subroutine),
        Just(SymbolKind::Method),
        arb_var_kind().prop_map(SymbolKind::Variable),
        Just(SymbolKind::Constant),
        Just(SymbolKind::Label),
        Just(SymbolKind::Format),
        Just(SymbolKind::Role),
        Just(SymbolKind::Import),
        Just(SymbolKind::Export),
    ]
}

/// Generate a Perl-like identifier segment.
fn arb_identifier() -> impl Strategy<Value = String> {
    "[A-Za-z_][A-Za-z0-9_]{0,12}".prop_map(String::from)
}

/// Generate a qualified name like `"Foo"` or `"Foo::Bar::baz"`.
fn arb_qualified_name() -> impl Strategy<Value = String> {
    prop::collection::vec(arb_identifier(), 1..=3).prop_map(|segments| segments.join("::"))
}

/// Generate a non-overlapping byte span pair `(full_span, anchor_span)`.
///
/// `full_span` is `(start, end)` where `start < end`.
/// `anchor_span` is `Some((a, b))` where `start <= a < b <= end`, or `None`.
fn arb_spans() -> impl Strategy<Value = ((usize, usize), Option<(usize, usize)>)> {
    (0usize..1000usize, 1usize..200usize).prop_flat_map(|(start, len)| {
        let end = start + len;
        let anchor_strat = if len >= 2 {
            prop_oneof![
                Just(None),
                (start..end)
                    .prop_flat_map(move |a| { ((a + 1)..=end).prop_map(move |b| Some((a, b))) }),
            ]
            .boxed()
        } else {
            // len == 1: anchor can only be the full span or None
            prop_oneof![Just(None), Just(Some((start, end))),].boxed()
        };
        (Just((start, end)), anchor_strat)
    })
}

/// Generate a random `SymbolDecl`.
fn arb_symbol_decl() -> impl Strategy<Value = SymbolDecl> {
    (
        arb_symbol_kind(),
        arb_identifier(),
        arb_qualified_name(),
        arb_spans(),
        prop::option::of(arb_identifier()),
        prop::option::of(prop_oneof![
            Just("my".to_string()),
            Just("our".to_string()),
            Just("local".to_string()),
            Just("state".to_string()),
        ]),
    )
        .prop_map(
            |(kind, name, qualified_name, (full_span, anchor_span), container, declarator)| {
                SymbolDecl {
                    kind,
                    name,
                    qualified_name,
                    full_span,
                    anchor_span,
                    container,
                    declarator,
                }
            },
        )
}

/// Generate a random `SymbolRefKind`.
fn arb_symbol_ref_kind() -> impl Strategy<Value = SymbolRefKind> {
    prop_oneof![
        arb_var_kind().prop_map(SymbolRefKind::Variable),
        Just(SymbolRefKind::SubroutineCall),
        Just(SymbolRefKind::MethodCall),
        Just(SymbolRefKind::StaticMethodCall),
        Just(SymbolRefKind::CoderefReference),
        Just(SymbolRefKind::TypeglobReference),
    ]
}

/// Generate a random `SymbolRef`.
fn arb_symbol_ref() -> impl Strategy<Value = SymbolRef> {
    (
        arb_symbol_ref_kind(),
        arb_identifier(),
        arb_qualified_name(),
        prop::option::of(prop_oneof![
            Just("$".to_string()),
            Just("@".to_string()),
            Just("%".to_string()),
            Just("&".to_string()),
            Just("*".to_string()),
        ]),
        prop::option::of(arb_qualified_name()),
        arb_spans(),
    )
        .prop_map(
            |(kind, name, qualified_name, sigil, package_qualifier, (full_span, anchor_span))| {
                SymbolRef {
                    kind,
                    name,
                    qualified_name,
                    sigil,
                    package_qualifier,
                    full_span,
                    anchor_span,
                }
            },
        )
}

/// Generate a random `FileId`.
fn arb_file_id() -> impl Strategy<Value = FileId> {
    any::<u64>().prop_map(FileId)
}

// ---------------------------------------------------------------------------
// Helper: collect all AnchorIds from a Vec<AnchorFact>
// ---------------------------------------------------------------------------

fn anchor_id_set(
    anchors: &[perl_semantic_facts::AnchorFact],
) -> std::collections::HashSet<AnchorId> {
    anchors.iter().map(|a| a.id).collect()
}

// ---------------------------------------------------------------------------
// Property tests
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// **Validates: Requirements 2.2**
    ///
    /// Property 2 (decls): Every EntityFact emitted by
    /// `symbol_decls_to_semantic_facts` that carries a `Some(anchor_id)` must
    /// reference an AnchorFact present in the same result set.
    #[test]
    fn decl_entity_anchor_ids_resolve(
        decls in prop::collection::vec(arb_symbol_decl(), 0..20),
        file_id in arb_file_id(),
    ) {
        let facts = symbol_decls_to_semantic_facts(&decls, file_id);
        let known_anchors = anchor_id_set(&facts.anchors);

        for entity in &facts.entities {
            if let Some(aid) = entity.anchor_id {
                prop_assert!(
                    known_anchors.contains(&aid),
                    "EntityFact {:?} (canonical_name={}) has anchor_id {:?} \
                     not found in anchors set ({} anchors)",
                    entity.id,
                    entity.canonical_name,
                    aid,
                    known_anchors.len(),
                );
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// **Validates: Requirements 2.3**
    ///
    /// Property 2 (refs): Every OccurrenceFact emitted by
    /// `symbol_refs_to_semantic_facts` has an `anchor_id` that references an
    /// AnchorFact present in the same result set.
    #[test]
    fn ref_occurrence_anchor_ids_resolve(
        refs in prop::collection::vec(arb_symbol_ref(), 0..20),
        file_id in arb_file_id(),
    ) {
        // Build entity map from the generated refs so some resolve and some don't.
        let entity_map: BTreeMap<String, EntityId> = refs
            .iter()
            .enumerate()
            .filter(|(i, _)| i % 2 == 0)
            .map(|(i, r)| (r.qualified_name.clone(), EntityId(i as u64 + 100)))
            .collect();

        let facts = symbol_refs_to_semantic_facts(&refs, file_id, &entity_map);
        let known_anchors = anchor_id_set(&facts.anchors);

        for occurrence in &facts.occurrences {
            prop_assert!(
                known_anchors.contains(&occurrence.anchor_id),
                "OccurrenceFact {:?} has anchor_id {:?} \
                 not found in anchors set ({} anchors)",
                occurrence.id,
                occurrence.anchor_id,
                known_anchors.len(),
            );
        }
    }
}
