//! Property-based tests for export set completeness (Property 8).
//!
//! **Validates: Requirements 6.5**
//!
//! **Property 8: Export Set Completeness** — For any module with @EXPORT,
//! @EXPORT_OK, or %EXPORT_TAGS, the ExportSet contains all declared symbols,
//! sorted and deduplicated.
//!
//! Since we cannot easily generate random ASTs, this test constructs
//! `ExportInfo` directly with random symbol sets and verifies that
//! `to_export_set()` produces a sorted, deduplicated result containing
//! all input symbols.

use perl_semantic_analyzer::analysis::export_analyzer::ExportInfo;
use perl_semantic_facts::{AnchorId, Confidence};
use proptest::prelude::*;
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Generate a Perl-like symbol name (e.g. `foo`, `bar_baz`, `_qux`).
fn arb_symbol_name() -> impl Strategy<Value = String> {
    "[a-z_][a-z0-9_]{0,15}".prop_map(String::from)
}

/// Generate a set of symbol names (0..20 elements).
fn arb_symbol_set() -> impl Strategy<Value = HashSet<String>> {
    prop::collection::hash_set(arb_symbol_name(), 0..20)
}

/// Generate a tag name (e.g. `all`, `utils`, `io_funcs`).
fn arb_tag_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,10}".prop_map(String::from)
}

/// Generate export tags: a map of tag name -> list of symbols.
/// Symbols may contain duplicates within a tag to test dedup.
fn arb_export_tags() -> impl Strategy<Value = HashMap<String, Vec<String>>> {
    prop::collection::hash_map(
        arb_tag_name(),
        prop::collection::vec(arb_symbol_name(), 0..15),
        0..5,
    )
}

/// Generate an optional module name.
fn arb_module_name() -> impl Strategy<Value = Option<String>> {
    prop::option::of("[A-Z][a-z]{1,8}(::[A-Z][a-z]{1,8}){0,2}".prop_map(String::from))
}

/// Generate an optional AnchorId.
fn arb_anchor_id() -> impl Strategy<Value = Option<AnchorId>> {
    prop::option::of(any::<u64>().prop_map(AnchorId))
}

/// Generate a complete `ExportInfo` with random contents.
fn arb_export_info() -> impl Strategy<Value = ExportInfo> {
    (arb_symbol_set(), arb_symbol_set(), arb_export_tags(), arb_module_name(), arb_anchor_id())
        .prop_map(|(default_export, optional_export, export_tags, module_name, anchor_id)| {
            ExportInfo {
                default_export,
                optional_export,
                export_tags,
                module_name,
                anchor_id,
                ..ExportInfo::default()
            }
        })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Assert that a `Vec<String>` is sorted and contains no duplicates.
fn assert_sorted_and_deduped(items: &[String], label: &str) -> Result<(), TestCaseError> {
    for window in items.windows(2) {
        prop_assert!(
            window[0] < window[1],
            "{label} is not strictly sorted/deduplicated: found {:?} followed by {:?}",
            window[0],
            window[1],
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Property tests
// ---------------------------------------------------------------------------

#[test]
fn export_info_default_custom_import_keeps_high_confidence() {
    let info =
        ExportInfo { module_name: Some("Static::Exporter".to_string()), ..ExportInfo::default() };

    let export_set = info.to_export_set();

    assert_eq!(export_set.confidence, Confidence::High);
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// **Validates: Requirements 6.5**
    ///
    /// Property 8: The ExportSet produced by `to_export_set()` contains all
    /// symbols from the input `ExportInfo`, sorted and deduplicated.
    #[test]
    fn export_set_contains_all_symbols_sorted_and_deduped(
        info in arb_export_info(),
    ) {
        let export_set = info.to_export_set();

        // Generated ExportInfo values model static Exporter-style facts, not dynamic custom import.
        prop_assert_eq!(
            export_set.confidence,
            Confidence::High,
            "generated ExportInfo should keep static-export confidence",
        );

        // -- default_exports: sorted, deduplicated, and complete --
        assert_sorted_and_deduped(&export_set.default_exports, "default_exports")?;

        // Every input default symbol must appear in the output.
        for sym in &info.default_export {
            prop_assert!(
                export_set.default_exports.contains(sym),
                "default_exports missing input symbol: {sym:?}",
            );
        }
        // Output must not contain symbols not in the input.
        let input_defaults: HashSet<&String> = info.default_export.iter().collect();
        for sym in &export_set.default_exports {
            prop_assert!(
                input_defaults.contains(sym),
                "default_exports contains unexpected symbol: {sym:?}",
            );
        }

        // -- optional_exports: sorted, deduplicated, and complete --
        assert_sorted_and_deduped(&export_set.optional_exports, "optional_exports")?;

        for sym in &info.optional_export {
            prop_assert!(
                export_set.optional_exports.contains(sym),
                "optional_exports missing input symbol: {sym:?}",
            );
        }
        let input_optionals: HashSet<&String> = info.optional_export.iter().collect();
        for sym in &export_set.optional_exports {
            prop_assert!(
                input_optionals.contains(sym),
                "optional_exports contains unexpected symbol: {sym:?}",
            );
        }

        // -- tags: sorted by name, each tag's members sorted and deduplicated --
        // Tag names must be sorted.
        let tag_names: Vec<&str> = export_set.tags.iter().map(|t| t.name.as_str()).collect();
        for window in tag_names.windows(2) {
            prop_assert!(
                window[0] < window[1],
                "tags not sorted by name: found {:?} followed by {:?}",
                window[0],
                window[1],
            );
        }

        // Each tag's members must be sorted and deduplicated.
        for tag in &export_set.tags {
            assert_sorted_and_deduped(&tag.members, &format!("tag '{}'", tag.name))?;
        }

        // Every input tag must appear in the output with all its unique symbols.
        for (tag_name, tag_symbols) in &info.export_tags {
            let output_tag = export_set.tags.iter().find(|t| &t.name == tag_name);
            prop_assert!(
                output_tag.is_some(),
                "tags missing input tag: {tag_name:?}",
            );
            let output_tag = output_tag.ok_or_else(|| {
                TestCaseError::Fail(format!("tag {tag_name:?} not found").into())
            })?;

            let unique_input: HashSet<&String> = tag_symbols.iter().collect();
            for sym in &unique_input {
                prop_assert!(
                    output_tag.members.contains(sym),
                    "tag {tag_name:?} missing input symbol: {sym:?}",
                );
            }
            // Output tag must not contain symbols not in the input.
            for sym in &output_tag.members {
                prop_assert!(
                    unique_input.contains(sym),
                    "tag {tag_name:?} contains unexpected symbol: {sym:?}",
                );
            }
        }
        // Output must not contain tags not in the input.
        for tag in &export_set.tags {
            prop_assert!(
                info.export_tags.contains_key(&tag.name),
                "tags contains unexpected tag: {:?}",
                tag.name,
            );
        }

        // -- module_name and anchor_id are preserved --
        prop_assert_eq!(
            &export_set.module_name, &info.module_name,
            "module_name not preserved",
        );
        prop_assert_eq!(
            export_set.anchor_id, info.anchor_id,
            "anchor_id not preserved",
        );
    }
}
