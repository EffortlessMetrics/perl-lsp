//! Property tests for the [`SymbolKind`] / [`VarKind`] taxonomy.
//!
//! These invariants protect the LSP-facing mappings: the workspace and
//! document-symbol providers fan out from `to_lsp_kind()` /
//! `to_lsp_kind_document_symbol()`, and the rename / completion features
//! rely on `sigil()` and the category predicates being consistent.

use perl_symbol::{SymbolKind, VarKind};
use proptest::prelude::*;

fn var_kind_strategy() -> impl Strategy<Value = VarKind> {
    prop_oneof![Just(VarKind::Scalar), Just(VarKind::Array), Just(VarKind::Hash)]
}

fn symbol_kind_strategy() -> impl Strategy<Value = SymbolKind> {
    prop_oneof![
        Just(SymbolKind::Package),
        Just(SymbolKind::Class),
        Just(SymbolKind::Role),
        Just(SymbolKind::Subroutine),
        Just(SymbolKind::Method),
        var_kind_strategy().prop_map(SymbolKind::Variable),
        Just(SymbolKind::Constant),
        Just(SymbolKind::Import),
        Just(SymbolKind::Export),
        Just(SymbolKind::Label),
        Just(SymbolKind::Format),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// Every variant maps to an LSP `SymbolKind` in the protocol range
    /// `1..=26`. Returning `0` or any out-of-range value would crash
    /// `lsp-types` decoding in editors.
    #[test]
    fn prop_to_lsp_kind_is_within_lsp_range(kind in symbol_kind_strategy()) {
        let lsp = kind.to_lsp_kind();
        prop_assert!((1..=26).contains(&lsp), "to_lsp_kind() returned {lsp} for {kind:?}");

        let doc_lsp = kind.to_lsp_kind_document_symbol();
        prop_assert!(
            (1..=26).contains(&doc_lsp),
            "to_lsp_kind_document_symbol() returned {doc_lsp} for {kind:?}"
        );
    }

    /// The document-symbol mapping only differs from the workspace mapping
    /// for variable kinds. Documented in `CLAUDE.md` and `test_lsp_kind_document_symbol_mapping`,
    /// but worth a property so adding a new non-variable variant can't silently
    /// drift the two mappings.
    #[test]
    fn prop_document_mapping_matches_workspace_for_non_variables(kind in symbol_kind_strategy()) {
        if !kind.is_variable() {
            prop_assert_eq!(
                kind.to_lsp_kind(),
                kind.to_lsp_kind_document_symbol(),
                "non-variable kind {:?} diverged between workspace and document mappings",
                kind
            );
        }
    }

    /// `sigil()` is `Some` if and only if the kind is a variable, and the
    /// returned sigil exactly matches the inner `VarKind`'s sigil.
    #[test]
    fn prop_sigil_iff_variable(kind in symbol_kind_strategy()) {
        match (kind.is_variable(), kind.sigil()) {
            (true, Some(s)) => {
                let SymbolKind::Variable(vk) = kind else {
                    return Err(proptest::test_runner::TestCaseError::fail(format!(
                        "is_variable() but not Variable variant: {kind:?}"
                    )));
                };
                prop_assert_eq!(s, vk.sigil(), "SymbolKind sigil disagrees with VarKind sigil");
            }
            (false, None) => {}
            (is_var, sig) => {
                return Err(proptest::test_runner::TestCaseError::fail(format!(
                    "is_variable={is_var} but sigil={sig:?} for {kind:?}"
                )));
            }
        }
    }

    /// The three category predicates (`is_variable`, `is_callable`,
    /// `is_namespace`) partition the taxonomy such that no kind matches more
    /// than one. Constants, imports, exports, labels, and formats are
    /// allowed to match none.
    #[test]
    fn prop_category_predicates_are_mutually_exclusive(kind in symbol_kind_strategy()) {
        let count = [kind.is_variable(), kind.is_callable(), kind.is_namespace()]
            .into_iter()
            .filter(|&b| b)
            .count();
        prop_assert!(count <= 1, "{kind:?} matches more than one category predicate");
    }

    /// Convenience constructors round-trip through `is_variable` / sigil.
    #[test]
    fn prop_variable_constructors_roundtrip(vk in var_kind_strategy()) {
        let kind = SymbolKind::Variable(vk);
        prop_assert!(kind.is_variable());
        prop_assert_eq!(kind.sigil(), Some(vk.sigil()));

        // Each constructor must match its tagged VarKind.
        let constructed = match vk {
            VarKind::Scalar => SymbolKind::scalar(),
            VarKind::Array => SymbolKind::array(),
            VarKind::Hash => SymbolKind::hash(),
        };
        prop_assert_eq!(kind, constructed);
    }

    /// The variable document-symbol mapping is a bijection on the three
    /// `VarKind` values. Different sigils must produce different LSP kinds
    /// so the editor's Outline view can render distinct icons.
    #[test]
    fn prop_variable_document_kinds_are_distinct(a in var_kind_strategy(), b in var_kind_strategy()) {
        let kind_a = SymbolKind::Variable(a).to_lsp_kind_document_symbol();
        let kind_b = SymbolKind::Variable(b).to_lsp_kind_document_symbol();
        prop_assert_eq!(a == b, kind_a == kind_b);
    }
}
