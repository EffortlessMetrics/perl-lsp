//! Property-based tests for adapter determinism (Property 4).
//!
//! **Validates: Requirements 3.2, 3.3**
//!
//! **Property 4: Adapter Determinism** — For any set of SymbolDecls and a FileId,
//! running `symbol_decls_to_semantic_facts` twice with the same inputs produces
//! identical output. Same for `symbol_refs_to_semantic_facts`.

use perl_semantic_facts::{EntityId, FileId};
use perl_symbol::{
    SymbolDecl, SymbolKind, SymbolRef, SymbolRefKind, VarKind, symbol_decls_to_semantic_facts,
    symbol_refs_to_semantic_facts,
};
use proptest::prelude::*;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Strategies
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
// Property tests
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// **Validates: Requirements 3.2, 3.3**
    ///
    /// Property 4 (decls): Running `symbol_decls_to_semantic_facts` twice with
    /// the same SymbolDecl list and FileId produces identical output — same
    /// AnchorIds, EntityIds, EdgeIds, and ordering.
    #[test]
    fn symbol_decls_adapter_is_deterministic(
        decls in prop::collection::vec(arb_symbol_decl(), 0..20),
        file_id in arb_file_id(),
    ) {
        let first = symbol_decls_to_semantic_facts(&decls, file_id);
        let second = symbol_decls_to_semantic_facts(&decls, file_id);
        prop_assert_eq!(&first, &second, "SymbolDeclSemanticFacts differed across two runs");
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// **Validates: Requirements 3.2, 3.3**
    ///
    /// Property 4 (refs): Running `symbol_refs_to_semantic_facts` twice with
    /// the same SymbolRef list, FileId, and entity map produces identical
    /// output — same AnchorIds, OccurrenceIds, EdgeIds, and ordering.
    #[test]
    fn symbol_refs_adapter_is_deterministic(
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

        let first = symbol_refs_to_semantic_facts(&refs, file_id, &entity_map);
        let second = symbol_refs_to_semantic_facts(&refs, file_id, &entity_map);
        prop_assert_eq!(&first, &second, "SymbolRefSemanticFacts differed across two runs");
    }
}
