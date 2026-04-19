//! Comprehensive unit tests for `perl-lsp-feature-flags`.
//!
//! Covers `BuildFlags`, `AdvertisedFeatures`, profile constructors,
//! `to_advertised_features()`, `to_feature_ids()`, trait derivations,
//! and cross-profile invariants.
#![allow(clippy::type_complexity)]

use perl_lsp_feature_flags::{AdvertisedFeatures, BuildFlags};
use perl_lsp_feature_ids::*;

// ---------------------------------------------------------------------------
// AdvertisedFeatures — trait derivations
// ---------------------------------------------------------------------------

#[test]
fn advertised_features_default_is_all_false() -> Result<(), Box<dyn std::error::Error>> {
    let af = AdvertisedFeatures::default();
    assert!(!af.completion);
    assert!(!af.hover);
    assert!(!af.definition);
    assert!(!af.references);
    assert!(!af.document_symbol);
    assert!(!af.workspace_symbol);
    assert!(!af.code_action);
    assert!(!af.code_lens);
    assert!(!af.formatting);
    assert!(!af.range_formatting);
    assert!(!af.rename);
    assert!(!af.folding_range);
    assert!(!af.selection_range);
    assert!(!af.linked_editing);
    assert!(!af.inlay_hints);
    assert!(!af.semantic_tokens);
    assert!(!af.call_hierarchy);
    assert!(!af.type_hierarchy);
    assert!(!af.diagnostic_provider);
    assert!(!af.document_color);
    assert!(!af.notebook_document_sync);
    assert!(!af.notebook_cell_execution);
    assert!(!af.signature_help);
    assert!(!af.document_highlight);
    assert!(!af.declaration);
    Ok(())
}

#[test]
fn advertised_features_clone_produces_equal_values() -> Result<(), Box<dyn std::error::Error>> {
    let af = AdvertisedFeatures {
        completion: true,
        hover: true,
        ..Default::default()
    };
    let cloned = af.clone();
    assert_eq!(cloned.completion, af.completion);
    assert_eq!(cloned.hover, af.hover);
    assert_eq!(cloned.definition, af.definition);
    Ok(())
}

#[test]
fn advertised_features_debug_is_non_empty() -> Result<(), Box<dyn std::error::Error>> {
    let af = AdvertisedFeatures::default();
    let debug = format!("{af:?}");
    assert!(!debug.is_empty());
    assert!(debug.contains("AdvertisedFeatures"));
    Ok(())
}

// ---------------------------------------------------------------------------
// BuildFlags — trait derivations
// ---------------------------------------------------------------------------

#[test]
fn build_flags_default_is_all_false() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags::default();
    assert!(!bf.completion);
    assert!(!bf.hover);
    assert!(!bf.definition);
    assert!(!bf.type_definition);
    assert!(!bf.implementation);
    assert!(!bf.references);
    assert!(!bf.document_symbol);
    assert!(!bf.workspace_symbol);
    assert!(!bf.inlay_hints);
    assert!(!bf.pull_diagnostics);
    assert!(!bf.workspace_symbol_resolve);
    assert!(!bf.semantic_tokens);
    assert!(!bf.code_actions);
    assert!(!bf.execute_command);
    assert!(!bf.rename);
    assert!(!bf.document_links);
    assert!(!bf.selection_ranges);
    assert!(!bf.on_type_formatting);
    assert!(!bf.code_lens);
    assert!(!bf.call_hierarchy);
    assert!(!bf.type_hierarchy);
    assert!(!bf.linked_editing);
    assert!(!bf.inline_completion);
    assert!(!bf.inline_values);
    assert!(!bf.notebook_document_sync);
    assert!(!bf.notebook_cell_execution);
    assert!(!bf.moniker);
    assert!(!bf.document_color);
    assert!(!bf.source_organize_imports);
    assert!(!bf.formatting);
    assert!(!bf.range_formatting);
    assert!(!bf.folding_range);
    assert!(!bf.signature_help);
    assert!(!bf.document_highlight);
    assert!(!bf.declaration);
    Ok(())
}

#[test]
fn build_flags_partial_eq_and_eq() -> Result<(), Box<dyn std::error::Error>> {
    let a = BuildFlags::default();
    let b = BuildFlags::default();
    assert_eq!(a, b);

    let c = BuildFlags {
        completion: true,
        ..Default::default()
    };
    assert_ne!(a, c);
    Ok(())
}

#[test]
fn build_flags_clone_preserves_equality() -> Result<(), Box<dyn std::error::Error>> {
    let original = BuildFlags::production();
    let cloned = original.clone();
    assert_eq!(original, cloned);
    Ok(())
}

#[test]
fn build_flags_debug_is_non_empty() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags::default();
    let debug = format!("{bf:?}");
    assert!(!debug.is_empty());
    assert!(debug.contains("BuildFlags"));
    Ok(())
}

// ---------------------------------------------------------------------------
// BuildFlags::default() -> to_feature_ids() yields empty
// ---------------------------------------------------------------------------

#[test]
fn default_flags_produce_no_feature_ids() -> Result<(), Box<dyn std::error::Error>> {
    let ids = BuildFlags::default().to_feature_ids();
    assert!(ids.is_empty());
    Ok(())
}

#[test]
fn default_flags_advertised_features_are_all_false() -> Result<(), Box<dyn std::error::Error>> {
    let af = BuildFlags::default().to_advertised_features();
    assert!(!af.completion);
    assert!(!af.hover);
    assert!(!af.definition);
    assert!(!af.references);
    assert!(!af.document_symbol);
    assert!(!af.workspace_symbol);
    assert!(!af.code_action);
    assert!(!af.code_lens);
    assert!(!af.formatting);
    assert!(!af.range_formatting);
    assert!(!af.rename);
    assert!(!af.folding_range);
    assert!(!af.selection_range);
    assert!(!af.linked_editing);
    assert!(!af.inlay_hints);
    assert!(!af.semantic_tokens);
    assert!(!af.call_hierarchy);
    assert!(!af.type_hierarchy);
    assert!(!af.diagnostic_provider);
    assert!(!af.document_color);
    assert!(!af.notebook_document_sync);
    assert!(!af.notebook_cell_execution);
    assert!(!af.signature_help);
    assert!(!af.document_highlight);
    assert!(!af.declaration);
    Ok(())
}

// ---------------------------------------------------------------------------
// BuildFlags::all()
// ---------------------------------------------------------------------------

#[test]
fn all_flags_are_all_true() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags::all();
    assert!(bf.completion);
    assert!(bf.hover);
    assert!(bf.definition);
    assert!(bf.type_definition);
    assert!(bf.implementation);
    assert!(bf.references);
    assert!(bf.document_symbol);
    assert!(bf.workspace_symbol);
    assert!(bf.inlay_hints);
    assert!(bf.pull_diagnostics);
    assert!(bf.workspace_symbol_resolve);
    assert!(bf.semantic_tokens);
    assert!(bf.code_actions);
    assert!(bf.execute_command);
    assert!(bf.rename);
    assert!(bf.document_links);
    assert!(bf.selection_ranges);
    assert!(bf.on_type_formatting);
    assert!(bf.code_lens);
    assert!(bf.call_hierarchy);
    assert!(bf.type_hierarchy);
    assert!(bf.linked_editing);
    assert!(bf.inline_completion);
    assert!(bf.inline_values);
    assert!(bf.notebook_document_sync);
    assert!(bf.notebook_cell_execution);
    assert!(bf.moniker);
    assert!(bf.document_color);
    assert!(bf.source_organize_imports);
    assert!(bf.formatting);
    assert!(bf.range_formatting);
    assert!(bf.folding_range);
    assert!(bf.signature_help);
    assert!(bf.document_highlight);
    assert!(bf.declaration);
    Ok(())
}

#[test]
fn all_advertised_features_are_all_true() -> Result<(), Box<dyn std::error::Error>> {
    let af = BuildFlags::all().to_advertised_features();
    assert!(af.completion);
    assert!(af.hover);
    assert!(af.definition);
    assert!(af.references);
    assert!(af.document_symbol);
    assert!(af.workspace_symbol);
    assert!(af.code_action);
    assert!(af.code_lens);
    assert!(af.formatting);
    assert!(af.range_formatting);
    assert!(af.rename);
    assert!(af.folding_range);
    assert!(af.selection_range);
    assert!(af.linked_editing);
    assert!(af.inlay_hints);
    assert!(af.semantic_tokens);
    assert!(af.call_hierarchy);
    assert!(af.type_hierarchy);
    assert!(af.diagnostic_provider);
    assert!(af.document_color);
    assert!(af.notebook_document_sync);
    assert!(af.notebook_cell_execution);
    assert!(af.signature_help);
    assert!(af.document_highlight);
    assert!(af.declaration);
    Ok(())
}

#[test]
fn all_feature_ids_count_matches_all_flags() -> Result<(), Box<dyn std::error::Error>> {
    let ids = BuildFlags::all().to_feature_ids();
    // BuildFlags has 35 bool fields but only 34 emit feature IDs
    // (workspace_symbol_resolve and source_organize_imports have no ID mapping).
    // range_formatting emits both LSP_RANGE_FORMATTING and LSP_RANGES_FORMATTING.
    assert_eq!(ids.len(), 34);
    Ok(())
}

// ---------------------------------------------------------------------------
// BuildFlags::production()
// ---------------------------------------------------------------------------

#[test]
fn production_enables_formatting_and_range_formatting() -> Result<(), Box<dyn std::error::Error>> {
    let p = BuildFlags::production();
    assert!(p.formatting);
    assert!(p.range_formatting);
    Ok(())
}

#[test]
fn production_enables_core_capabilities() -> Result<(), Box<dyn std::error::Error>> {
    let p = BuildFlags::production();
    assert!(p.completion);
    assert!(p.hover);
    assert!(p.definition);
    assert!(p.references);
    assert!(p.document_symbol);
    assert!(p.workspace_symbol);
    assert!(p.semantic_tokens);
    assert!(p.code_actions);
    assert!(p.rename);
    assert!(p.folding_range);
    assert!(p.signature_help);
    assert!(p.document_highlight);
    assert!(p.declaration);
    Ok(())
}

#[test]
fn production_advertised_features_reflect_formatting_on() -> Result<(), Box<dyn std::error::Error>>
{
    let af = BuildFlags::production().to_advertised_features();
    assert!(af.formatting);
    assert!(af.range_formatting);
    assert!(af.completion);
    assert!(af.hover);
    Ok(())
}

#[test]
fn production_feature_ids_include_formatting() -> Result<(), Box<dyn std::error::Error>> {
    let ids = BuildFlags::production().to_feature_ids();
    assert!(ids.contains(&LSP_FORMATTING));
    assert!(ids.contains(&LSP_RANGE_FORMATTING));
    assert!(ids.contains(&LSP_COMPLETION));
    assert!(ids.contains(&LSP_HOVER));
    Ok(())
}

// ---------------------------------------------------------------------------
// BuildFlags::ga_lock()
// ---------------------------------------------------------------------------

#[test]
fn ga_lock_disables_inline_values() -> Result<(), Box<dyn std::error::Error>> {
    let ga = BuildFlags::ga_lock();
    assert!(!ga.inline_values);
    Ok(())
}

#[test]
fn ga_lock_enables_formatting() -> Result<(), Box<dyn std::error::Error>> {
    let ga = BuildFlags::ga_lock();
    assert!(ga.formatting);
    assert!(ga.range_formatting);
    Ok(())
}

#[test]
fn ga_lock_enables_core_capabilities() -> Result<(), Box<dyn std::error::Error>> {
    let ga = BuildFlags::ga_lock();
    assert!(ga.completion);
    assert!(ga.hover);
    assert!(ga.definition);
    assert!(ga.references);
    assert!(ga.document_symbol);
    assert!(ga.workspace_symbol);
    assert!(ga.semantic_tokens);
    assert!(ga.code_actions);
    assert!(ga.rename);
    assert!(ga.folding_range);
    assert!(ga.signature_help);
    assert!(ga.document_highlight);
    assert!(ga.declaration);
    Ok(())
}

#[test]
fn ga_lock_feature_ids_exclude_inline_value() -> Result<(), Box<dyn std::error::Error>> {
    let ids = BuildFlags::ga_lock().to_feature_ids();
    assert!(!ids.contains(&LSP_INLINE_VALUE));
    assert!(ids.contains(&LSP_FORMATTING));
    assert!(ids.contains(&LSP_RANGE_FORMATTING));
    Ok(())
}

// ---------------------------------------------------------------------------
// Cross-profile comparisons
// ---------------------------------------------------------------------------

#[test]
fn all_is_superset_of_production() -> Result<(), Box<dyn std::error::Error>> {
    let all_ids = BuildFlags::all().to_feature_ids();
    let prod_ids = BuildFlags::production().to_feature_ids();
    for id in &prod_ids {
        assert!(
            all_ids.contains(id),
            "all() should contain production id: {id}"
        );
    }
    Ok(())
}

#[test]
fn all_is_superset_of_ga_lock() -> Result<(), Box<dyn std::error::Error>> {
    let all_ids = BuildFlags::all().to_feature_ids();
    let ga_ids = BuildFlags::ga_lock().to_feature_ids();
    for id in &ga_ids {
        assert!(
            all_ids.contains(id),
            "all() should contain ga_lock id: {id}"
        );
    }
    Ok(())
}

#[test]
fn production_and_ga_lock_differ() -> Result<(), Box<dyn std::error::Error>> {
    let prod = BuildFlags::production();
    let ga = BuildFlags::ga_lock();
    assert_ne!(prod, ga);
    Ok(())
}

#[test]
fn all_matches_production() -> Result<(), Box<dyn std::error::Error>> {
    let all = BuildFlags::all();
    let prod = BuildFlags::production();
    assert_eq!(all, prod);
    Ok(())
}

#[test]
fn all_differs_from_ga_lock() -> Result<(), Box<dyn std::error::Error>> {
    let all = BuildFlags::all();
    let ga = BuildFlags::ga_lock();
    assert_ne!(all, ga);
    Ok(())
}

// ---------------------------------------------------------------------------
// to_feature_ids() — sorting and deduplication
// ---------------------------------------------------------------------------

#[test]
fn feature_ids_are_sorted_for_all_profiles() -> Result<(), Box<dyn std::error::Error>> {
    for (label, flags) in [
        ("all", BuildFlags::all()),
        ("production", BuildFlags::production()),
        ("ga_lock", BuildFlags::ga_lock()),
    ] {
        let ids = flags.to_feature_ids();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted, "feature ids for {label} must be sorted");
    }
    Ok(())
}

#[test]
fn feature_ids_have_no_duplicates_for_all_profiles() -> Result<(), Box<dyn std::error::Error>> {
    for (label, flags) in [
        ("all", BuildFlags::all()),
        ("production", BuildFlags::production()),
        ("ga_lock", BuildFlags::ga_lock()),
    ] {
        let ids = flags.to_feature_ids();
        let mut deduped = ids.clone();
        deduped.dedup();
        assert_eq!(
            ids, deduped,
            "feature ids for {label} must have no duplicates"
        );
    }
    Ok(())
}

#[test]
fn feature_ids_are_strictly_increasing() -> Result<(), Box<dyn std::error::Error>> {
    let ids = BuildFlags::all().to_feature_ids();
    assert!(
        ids.windows(2).all(|w| w[0] < w[1]),
        "feature ids must be in strictly increasing order",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// to_feature_ids() — single-flag isolation
// ---------------------------------------------------------------------------

#[test]
fn single_flag_completion_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        completion: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_COMPLETION]);
    Ok(())
}

#[test]
fn single_flag_hover_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        hover: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_HOVER]);
    Ok(())
}

#[test]
fn single_flag_definition_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        definition: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_DEFINITION]);
    Ok(())
}

#[test]
fn single_flag_type_definition_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        type_definition: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_TYPE_DEFINITION]);
    Ok(())
}

#[test]
fn single_flag_implementation_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        implementation: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_IMPLEMENTATION]);
    Ok(())
}

#[test]
fn single_flag_references_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        references: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_REFERENCES]);
    Ok(())
}

#[test]
fn single_flag_document_symbol_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        document_symbol: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_DOCUMENT_SYMBOL]);
    Ok(())
}

#[test]
fn single_flag_workspace_symbol_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        workspace_symbol: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_WORKSPACE_SYMBOL]);
    Ok(())
}

#[test]
fn single_flag_inlay_hints_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        inlay_hints: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_INLAY_HINT]);
    Ok(())
}

#[test]
fn single_flag_pull_diagnostics_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        pull_diagnostics: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_PULL_DIAGNOSTICS]);
    Ok(())
}

#[test]
fn single_flag_semantic_tokens_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        semantic_tokens: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_SEMANTIC_TOKENS]);
    Ok(())
}

#[test]
fn single_flag_code_actions_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        code_actions: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_CODE_ACTION]);
    Ok(())
}

#[test]
fn single_flag_execute_command_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        execute_command: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_EXECUTE_COMMAND]);
    Ok(())
}

#[test]
fn single_flag_rename_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        rename: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_RENAME]);
    Ok(())
}

#[test]
fn single_flag_document_links_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        document_links: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_DOCUMENT_LINK]);
    Ok(())
}

#[test]
fn single_flag_selection_ranges_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        selection_ranges: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_SELECTION_RANGE]);
    Ok(())
}

#[test]
fn single_flag_on_type_formatting_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        on_type_formatting: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_ON_TYPE_FORMATTING]);
    Ok(())
}

#[test]
fn single_flag_code_lens_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        code_lens: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_CODE_LENS]);
    Ok(())
}

#[test]
fn single_flag_call_hierarchy_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        call_hierarchy: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_CALL_HIERARCHY]);
    Ok(())
}

#[test]
fn single_flag_type_hierarchy_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        type_hierarchy: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_TYPE_HIERARCHY]);
    Ok(())
}

#[test]
fn single_flag_linked_editing_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        linked_editing: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_LINKED_EDITING_RANGE]);
    Ok(())
}

#[test]
fn single_flag_inline_completion_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        inline_completion: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_INLINE_COMPLETION]);
    Ok(())
}

#[test]
fn single_flag_inline_values_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        inline_values: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_INLINE_VALUE]);
    Ok(())
}

#[test]
fn single_flag_notebook_document_sync_produces_correct_id() -> Result<(), Box<dyn std::error::Error>>
{
    let flags = BuildFlags {
        notebook_document_sync: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_NOTEBOOK_DOCUMENT_SYNC]);
    Ok(())
}

#[test]
fn single_flag_notebook_cell_execution_produces_correct_id()
-> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        notebook_cell_execution: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_NOTEBOOK_CELL_EXECUTION]);
    Ok(())
}

#[test]
fn single_flag_moniker_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        moniker: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_MONIKER]);
    Ok(())
}

#[test]
fn single_flag_document_color_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        document_color: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_DOCUMENT_COLOR]);
    Ok(())
}

#[test]
fn single_flag_formatting_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        formatting: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_FORMATTING]);
    Ok(())
}

#[test]
fn single_flag_range_formatting_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        range_formatting: true,
        ..Default::default()
    };
    // range_formatting gates both single-range and multi-range formatting because
    // both features require perltidy and the rangesFormatting handler already exists.
    let ids = flags.to_feature_ids();
    assert!(
        ids.contains(&LSP_RANGE_FORMATTING),
        "must contain LSP_RANGE_FORMATTING"
    );
    assert!(
        ids.contains(&LSP_RANGES_FORMATTING),
        "must contain LSP_RANGES_FORMATTING"
    );
    assert_eq!(ids.len(), 2);
    Ok(())
}

#[test]
fn single_flag_folding_range_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        folding_range: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_FOLDING_RANGE]);
    Ok(())
}

#[test]
fn single_flag_signature_help_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        signature_help: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_SIGNATURE_HELP]);
    Ok(())
}

#[test]
fn single_flag_document_highlight_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        document_highlight: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_DOCUMENT_HIGHLIGHT]);
    Ok(())
}

#[test]
fn single_flag_declaration_produces_correct_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        declaration: true,
        ..Default::default()
    };
    assert_eq!(flags.to_feature_ids(), vec![LSP_DECLARATION]);
    Ok(())
}

// ---------------------------------------------------------------------------
// Flags with no feature-id mapping emit nothing
// ---------------------------------------------------------------------------

#[test]
fn workspace_symbol_resolve_flag_has_no_feature_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        workspace_symbol_resolve: true,
        ..Default::default()
    };
    assert!(flags.to_feature_ids().is_empty());
    Ok(())
}

#[test]
fn source_organize_imports_flag_has_no_feature_id() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        source_organize_imports: true,
        ..Default::default()
    };
    assert!(flags.to_feature_ids().is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// to_advertised_features() — field mapping correctness
// ---------------------------------------------------------------------------

#[test]
fn to_advertised_maps_code_actions_to_code_action() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        code_actions: true,
        ..Default::default()
    };
    let af = flags.to_advertised_features();
    assert!(af.code_action);
    Ok(())
}

#[test]
fn to_advertised_maps_selection_ranges_to_selection_range() -> Result<(), Box<dyn std::error::Error>>
{
    let flags = BuildFlags {
        selection_ranges: true,
        ..Default::default()
    };
    let af = flags.to_advertised_features();
    assert!(af.selection_range);
    Ok(())
}

#[test]
fn to_advertised_maps_pull_diagnostics_to_diagnostic_provider()
-> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        pull_diagnostics: true,
        ..Default::default()
    };
    let af = flags.to_advertised_features();
    assert!(af.diagnostic_provider);
    Ok(())
}

#[test]
fn to_advertised_does_not_expose_build_only_fields() -> Result<(), Box<dyn std::error::Error>> {
    // Fields like type_definition, implementation, execute_command, document_links,
    // on_type_formatting, inline_completion, inline_values, moniker, source_organize_imports
    // have no corresponding AdvertisedFeatures field.
    let flags = BuildFlags {
        type_definition: true,
        implementation: true,
        execute_command: true,
        document_links: true,
        on_type_formatting: true,
        inline_completion: true,
        inline_values: true,
        moniker: true,
        workspace_symbol_resolve: true,
        source_organize_imports: true,
        ..Default::default()
    };
    let af = flags.to_advertised_features();
    // All advertised fields should be false since none of those build flags
    // map to an advertised feature.
    assert!(!af.completion);
    assert!(!af.hover);
    assert!(!af.definition);
    assert!(!af.references);
    assert!(!af.document_symbol);
    assert!(!af.workspace_symbol);
    assert!(!af.code_action);
    assert!(!af.code_lens);
    assert!(!af.formatting);
    assert!(!af.range_formatting);
    assert!(!af.rename);
    assert!(!af.folding_range);
    assert!(!af.selection_range);
    assert!(!af.linked_editing);
    assert!(!af.inlay_hints);
    assert!(!af.semantic_tokens);
    assert!(!af.call_hierarchy);
    assert!(!af.type_hierarchy);
    assert!(!af.diagnostic_provider);
    assert!(!af.document_color);
    assert!(!af.notebook_document_sync);
    assert!(!af.notebook_cell_execution);
    assert!(!af.signature_help);
    assert!(!af.document_highlight);
    assert!(!af.declaration);
    Ok(())
}

// ---------------------------------------------------------------------------
// to_advertised_features() — each directly-mapped field
// ---------------------------------------------------------------------------

#[test]
fn to_advertised_maps_each_direct_field() -> Result<(), Box<dyn std::error::Error>> {
    // Test each BuildFlags field that maps 1:1 to an AdvertisedFeatures field.
    let direct_mappings: Vec<(BuildFlags, Box<dyn Fn(&AdvertisedFeatures) -> bool>)> = vec![
        (
            BuildFlags {
                completion: true,
                ..Default::default()
            },
            Box::new(|af| af.completion),
        ),
        (
            BuildFlags {
                hover: true,
                ..Default::default()
            },
            Box::new(|af| af.hover),
        ),
        (
            BuildFlags {
                definition: true,
                ..Default::default()
            },
            Box::new(|af| af.definition),
        ),
        (
            BuildFlags {
                references: true,
                ..Default::default()
            },
            Box::new(|af| af.references),
        ),
        (
            BuildFlags {
                document_symbol: true,
                ..Default::default()
            },
            Box::new(|af| af.document_symbol),
        ),
        (
            BuildFlags {
                workspace_symbol: true,
                ..Default::default()
            },
            Box::new(|af| af.workspace_symbol),
        ),
        (
            BuildFlags {
                code_lens: true,
                ..Default::default()
            },
            Box::new(|af| af.code_lens),
        ),
        (
            BuildFlags {
                formatting: true,
                ..Default::default()
            },
            Box::new(|af| af.formatting),
        ),
        (
            BuildFlags {
                range_formatting: true,
                ..Default::default()
            },
            Box::new(|af| af.range_formatting),
        ),
        (
            BuildFlags {
                rename: true,
                ..Default::default()
            },
            Box::new(|af| af.rename),
        ),
        (
            BuildFlags {
                folding_range: true,
                ..Default::default()
            },
            Box::new(|af| af.folding_range),
        ),
        (
            BuildFlags {
                linked_editing: true,
                ..Default::default()
            },
            Box::new(|af| af.linked_editing),
        ),
        (
            BuildFlags {
                inlay_hints: true,
                ..Default::default()
            },
            Box::new(|af| af.inlay_hints),
        ),
        (
            BuildFlags {
                semantic_tokens: true,
                ..Default::default()
            },
            Box::new(|af| af.semantic_tokens),
        ),
        (
            BuildFlags {
                call_hierarchy: true,
                ..Default::default()
            },
            Box::new(|af| af.call_hierarchy),
        ),
        (
            BuildFlags {
                type_hierarchy: true,
                ..Default::default()
            },
            Box::new(|af| af.type_hierarchy),
        ),
        (
            BuildFlags {
                document_color: true,
                ..Default::default()
            },
            Box::new(|af| af.document_color),
        ),
        (
            BuildFlags {
                notebook_document_sync: true,
                ..Default::default()
            },
            Box::new(|af| af.notebook_document_sync),
        ),
        (
            BuildFlags {
                notebook_cell_execution: true,
                ..Default::default()
            },
            Box::new(|af| af.notebook_cell_execution),
        ),
        (
            BuildFlags {
                signature_help: true,
                ..Default::default()
            },
            Box::new(|af| af.signature_help),
        ),
        (
            BuildFlags {
                document_highlight: true,
                ..Default::default()
            },
            Box::new(|af| af.document_highlight),
        ),
        (
            BuildFlags {
                declaration: true,
                ..Default::default()
            },
            Box::new(|af| af.declaration),
        ),
    ];

    for (i, (flags, check)) in direct_mappings.iter().enumerate() {
        let af = flags.to_advertised_features();
        assert!(
            check(&af),
            "direct mapping {i} should be true in advertised features"
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Multi-flag combinations
// ---------------------------------------------------------------------------

#[test]
fn two_flags_produce_two_sorted_ids() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        hover: true,
        completion: true,
        ..Default::default()
    };
    let ids = flags.to_feature_ids();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&LSP_COMPLETION));
    assert!(ids.contains(&LSP_HOVER));
    // Must still be sorted
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted);
    Ok(())
}

#[test]
fn mixed_build_and_advertised_flags() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        completion: true,
        execute_command: true, // build-only (no advertised mapping)
        document_color: true,
        ..Default::default()
    };
    let af = flags.to_advertised_features();
    assert!(af.completion);
    assert!(af.document_color);
    // execute_command has no advertised mapping so all other fields stay false
    assert!(!af.hover);

    let ids = flags.to_feature_ids();
    assert!(ids.contains(&LSP_COMPLETION));
    assert!(ids.contains(&LSP_EXECUTE_COMMAND));
    assert!(ids.contains(&LSP_DOCUMENT_COLOR));
    assert_eq!(ids.len(), 3);
    Ok(())
}

// ---------------------------------------------------------------------------
// Profile identity (self-equality)
// ---------------------------------------------------------------------------

#[test]
fn production_equals_itself() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(BuildFlags::production(), BuildFlags::production());
    Ok(())
}

#[test]
fn all_equals_itself() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(BuildFlags::all(), BuildFlags::all());
    Ok(())
}

#[test]
fn ga_lock_equals_itself() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(BuildFlags::ga_lock(), BuildFlags::ga_lock());
    Ok(())
}

#[test]
fn default_equals_itself() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(BuildFlags::default(), BuildFlags::default());
    Ok(())
}

// ---------------------------------------------------------------------------
// Feature ID string values (spot checks)
// ---------------------------------------------------------------------------

#[test]
fn feature_id_strings_use_lsp_dot_prefix() -> Result<(), Box<dyn std::error::Error>> {
    let ids = BuildFlags::all().to_feature_ids();
    for id in &ids {
        assert!(
            id.starts_with("lsp."),
            "feature id {id} should start with 'lsp.'"
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ga_lock vs production: structural delta
// ---------------------------------------------------------------------------

#[test]
fn ga_lock_has_fewer_feature_ids_than_production() -> Result<(), Box<dyn std::error::Error>> {
    let ga_count = BuildFlags::ga_lock().to_feature_ids().len();
    let prod_count = BuildFlags::production().to_feature_ids().len();
    // ga_lock and production both have formatting+range_formatting enabled
    // ga_lock disables inline_values, so production has 1 more feature
    assert!(
        ga_count < prod_count,
        "ga_lock ({ga_count}) should have fewer feature ids than production ({prod_count})",
    );
    Ok(())
}
