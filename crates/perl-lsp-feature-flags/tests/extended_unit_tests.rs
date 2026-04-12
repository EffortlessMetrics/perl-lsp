//! Extended unit tests for `perl-lsp-feature-flags`.
//!
//! This module contains additional comprehensive test coverage for:
//! - AdvertisedFeatures struct construction and field mutations
//! - BuildFlags struct with all features and profile combinations
//! - Conversion methods (to_advertised_features, to_feature_ids)
//! - Profile constructors and invariants (production, all, ga_lock)
//! - Edge cases and cross-profile comparisons
//! - Feature flag toggling and querying patterns

use perl_lsp_feature_flags::{AdvertisedFeatures, BuildFlags};
use perl_lsp_feature_ids::*;

// ============================================================================
// AdvertisedFeatures — Construction and field manipulation
// ============================================================================

#[test]
fn advertised_features_completion_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.completion);
    af.completion = true;
    assert!(af.completion);
    af.completion = false;
    assert!(!af.completion);
    Ok(())
}

#[test]
fn advertised_features_hover_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.hover);
    af.hover = true;
    assert!(af.hover);
    Ok(())
}

#[test]
fn advertised_features_definition_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.definition);
    af.definition = true;
    assert!(af.definition);
    Ok(())
}

#[test]
fn advertised_features_references_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.references);
    af.references = true;
    assert!(af.references);
    Ok(())
}

#[test]
fn advertised_features_document_symbol_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.document_symbol);
    af.document_symbol = true;
    assert!(af.document_symbol);
    Ok(())
}

#[test]
fn advertised_features_workspace_symbol_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.workspace_symbol);
    af.workspace_symbol = true;
    assert!(af.workspace_symbol);
    Ok(())
}

#[test]
fn advertised_features_code_action_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.code_action);
    af.code_action = true;
    assert!(af.code_action);
    Ok(())
}

#[test]
fn advertised_features_code_lens_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.code_lens);
    af.code_lens = true;
    assert!(af.code_lens);
    Ok(())
}

#[test]
fn advertised_features_formatting_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.formatting);
    af.formatting = true;
    assert!(af.formatting);
    Ok(())
}

#[test]
fn advertised_features_range_formatting_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.range_formatting);
    af.range_formatting = true;
    assert!(af.range_formatting);
    Ok(())
}

#[test]
fn advertised_features_rename_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.rename);
    af.rename = true;
    assert!(af.rename);
    Ok(())
}

#[test]
fn advertised_features_folding_range_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.folding_range);
    af.folding_range = true;
    assert!(af.folding_range);
    Ok(())
}

#[test]
fn advertised_features_selection_range_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.selection_range);
    af.selection_range = true;
    assert!(af.selection_range);
    Ok(())
}

#[test]
fn advertised_features_linked_editing_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.linked_editing);
    af.linked_editing = true;
    assert!(af.linked_editing);
    Ok(())
}

#[test]
fn advertised_features_inlay_hints_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.inlay_hints);
    af.inlay_hints = true;
    assert!(af.inlay_hints);
    Ok(())
}

#[test]
fn advertised_features_semantic_tokens_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.semantic_tokens);
    af.semantic_tokens = true;
    assert!(af.semantic_tokens);
    Ok(())
}

#[test]
fn advertised_features_call_hierarchy_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.call_hierarchy);
    af.call_hierarchy = true;
    assert!(af.call_hierarchy);
    Ok(())
}

#[test]
fn advertised_features_type_hierarchy_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.type_hierarchy);
    af.type_hierarchy = true;
    assert!(af.type_hierarchy);
    Ok(())
}

#[test]
fn advertised_features_diagnostic_provider_can_be_toggled() -> Result<(), Box<dyn std::error::Error>>
{
    let mut af = AdvertisedFeatures::default();
    assert!(!af.diagnostic_provider);
    af.diagnostic_provider = true;
    assert!(af.diagnostic_provider);
    Ok(())
}

#[test]
fn advertised_features_document_color_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.document_color);
    af.document_color = true;
    assert!(af.document_color);
    Ok(())
}

#[test]
fn advertised_features_notebook_document_sync_can_be_toggled()
-> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.notebook_document_sync);
    af.notebook_document_sync = true;
    assert!(af.notebook_document_sync);
    Ok(())
}

#[test]
fn advertised_features_notebook_cell_execution_can_be_toggled()
-> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.notebook_cell_execution);
    af.notebook_cell_execution = true;
    assert!(af.notebook_cell_execution);
    Ok(())
}

#[test]
fn advertised_features_signature_help_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.signature_help);
    af.signature_help = true;
    assert!(af.signature_help);
    Ok(())
}

#[test]
fn advertised_features_document_highlight_can_be_toggled() -> Result<(), Box<dyn std::error::Error>>
{
    let mut af = AdvertisedFeatures::default();
    assert!(!af.document_highlight);
    af.document_highlight = true;
    assert!(af.document_highlight);
    Ok(())
}

#[test]
fn advertised_features_declaration_can_be_toggled() -> Result<(), Box<dyn std::error::Error>> {
    let mut af = AdvertisedFeatures::default();
    assert!(!af.declaration);
    af.declaration = true;
    assert!(af.declaration);
    Ok(())
}

#[test]
fn advertised_features_partial_initialization() -> Result<(), Box<dyn std::error::Error>> {
    let af = AdvertisedFeatures {
        completion: true,
        hover: true,
        definition: true,
        ..Default::default()
    };
    assert!(af.completion);
    assert!(af.hover);
    assert!(af.definition);
    assert!(!af.references);
    assert!(!af.code_action);
    Ok(())
}

#[test]
fn advertised_features_clone_independence() -> Result<(), Box<dyn std::error::Error>> {
    let af1 = AdvertisedFeatures { completion: true, ..Default::default() };
    let mut af2 = af1.clone();
    af2.completion = false;
    assert!(af1.completion);
    assert!(!af2.completion);
    Ok(())
}

// ============================================================================
// BuildFlags — Construction and field manipulation
// ============================================================================

#[test]
fn build_flags_all_features_enabled() -> Result<(), Box<dyn std::error::Error>> {
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
fn build_flags_production_profile() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags::production();
    assert!(bf.completion);
    assert!(bf.hover);
    assert!(bf.definition);
    assert!(bf.formatting);
    assert!(bf.range_formatting);
    assert!(bf.signature_help);
    Ok(())
}

#[test]
fn build_flags_ga_lock_profile() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags::ga_lock();
    assert!(bf.completion);
    assert!(bf.hover);
    assert!(!bf.inline_values);
    assert!(bf.formatting);
    assert!(bf.document_highlight);
    Ok(())
}

#[test]
fn build_flags_production_and_all_match_on_formatting() -> Result<(), Box<dyn std::error::Error>> {
    let prod = BuildFlags::production();
    let all = BuildFlags::all();
    assert!(prod.formatting);
    assert!(all.formatting);
    assert!(prod.range_formatting);
    assert!(all.range_formatting);
    Ok(())
}

#[test]
fn build_flags_ga_lock_and_all_differ_on_inline_values() -> Result<(), Box<dyn std::error::Error>> {
    let ga_lock = BuildFlags::ga_lock();
    let all = BuildFlags::all();
    assert!(!ga_lock.inline_values);
    assert!(all.inline_values);
    Ok(())
}

#[test]
fn build_flags_partial_initialization() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags { completion: true, hover: true, definition: true, ..Default::default() };
    assert!(bf.completion);
    assert!(bf.hover);
    assert!(bf.definition);
    assert!(!bf.references);
    assert!(!bf.code_actions);
    Ok(())
}

#[test]
fn build_flags_clone_independence() -> Result<(), Box<dyn std::error::Error>> {
    let bf1 = BuildFlags { completion: true, ..Default::default() };
    let mut bf2 = bf1.clone();
    bf2.completion = false;
    assert!(bf1.completion);
    assert!(!bf2.completion);
    Ok(())
}

#[test]
fn build_flags_equality() -> Result<(), Box<dyn std::error::Error>> {
    let bf1 = BuildFlags { completion: true, hover: true, ..Default::default() };
    let bf2 = BuildFlags { completion: true, hover: true, ..Default::default() };
    assert_eq!(bf1, bf2);
    Ok(())
}

#[test]
fn build_flags_inequality() -> Result<(), Box<dyn std::error::Error>> {
    let bf1 = BuildFlags { completion: true, ..Default::default() };
    let bf2 = BuildFlags { hover: true, ..Default::default() };
    assert_ne!(bf1, bf2);
    Ok(())
}

// ============================================================================
// Conversions: BuildFlags to AdvertisedFeatures
// ============================================================================

#[test]
fn build_flags_all_to_advertised_features() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags::all();
    let af = bf.to_advertised_features();
    assert!(af.completion);
    assert!(af.hover);
    assert!(af.definition);
    assert!(af.references);
    assert!(af.code_action);
    assert!(af.formatting);
    assert!(af.range_formatting);
    Ok(())
}

#[test]
fn build_flags_production_to_advertised_features() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags::production();
    let af = bf.to_advertised_features();
    assert!(af.completion);
    assert!(af.hover);
    assert!(af.definition);
    assert!(af.formatting);
    assert!(af.range_formatting);
    Ok(())
}

#[test]
fn build_flags_empty_to_advertised_features() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags::default();
    let af = bf.to_advertised_features();
    assert!(!af.completion);
    assert!(!af.hover);
    assert!(!af.definition);
    assert!(!af.formatting);
    Ok(())
}

#[test]
fn build_flags_code_actions_maps_to_code_action() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags { code_actions: true, ..Default::default() };
    let af = bf.to_advertised_features();
    assert!(af.code_action);
    Ok(())
}

#[test]
fn build_flags_selection_ranges_maps_to_selection_range() -> Result<(), Box<dyn std::error::Error>>
{
    let bf = BuildFlags { selection_ranges: true, ..Default::default() };
    let af = bf.to_advertised_features();
    assert!(af.selection_range);
    Ok(())
}

#[test]
fn build_flags_pull_diagnostics_maps_to_diagnostic_provider()
-> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags { pull_diagnostics: true, ..Default::default() };
    let af = bf.to_advertised_features();
    assert!(af.diagnostic_provider);
    Ok(())
}

// ============================================================================
// Conversions: BuildFlags to feature IDs
// ============================================================================

#[test]
fn build_flags_all_to_feature_ids_contains_completion() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags::all();
    let ids = bf.to_feature_ids();
    assert!(ids.contains(&LSP_COMPLETION));
    Ok(())
}

#[test]
fn build_flags_all_to_feature_ids_contains_hover() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags::all();
    let ids = bf.to_feature_ids();
    assert!(ids.contains(&LSP_HOVER));
    Ok(())
}

#[test]
fn build_flags_default_to_feature_ids_is_empty() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags::default();
    let ids = bf.to_feature_ids();
    assert!(ids.is_empty());
    Ok(())
}

#[test]
fn build_flags_single_feature_to_feature_ids() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags { completion: true, ..Default::default() };
    let ids = bf.to_feature_ids();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0], LSP_COMPLETION);
    Ok(())
}

#[test]
fn build_flags_completion_and_hover_to_feature_ids() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags { completion: true, hover: true, ..Default::default() };
    let ids = bf.to_feature_ids();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&LSP_COMPLETION));
    assert!(ids.contains(&LSP_HOVER));
    Ok(())
}

#[test]
fn build_flags_feature_ids_are_sorted() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags::all();
    let ids = bf.to_feature_ids();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted);
    Ok(())
}

#[test]
fn build_flags_feature_ids_are_deduplicated() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags::all();
    let ids = bf.to_feature_ids();
    let mut dedup_check = ids.clone();
    dedup_check.sort_unstable();
    let original_len = dedup_check.len();
    dedup_check.dedup();
    assert_eq!(original_len, dedup_check.len());
    Ok(())
}

#[test]
fn build_flags_production_to_feature_ids_includes_formatting()
-> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags::production();
    let ids = bf.to_feature_ids();
    assert!(ids.contains(&LSP_FORMATTING));
    assert!(ids.contains(&LSP_RANGE_FORMATTING));
    Ok(())
}

#[test]
fn build_flags_ga_lock_to_feature_ids() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags::ga_lock();
    let ids = bf.to_feature_ids();
    assert!(ids.contains(&LSP_FORMATTING));
    assert!(ids.contains(&LSP_SIGNATURE_HELP));
    Ok(())
}

// ============================================================================
// Edge cases and invariants
// ============================================================================

#[test]
fn build_flags_type_definition_to_feature_ids() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags { type_definition: true, ..Default::default() };
    let ids = bf.to_feature_ids();
    assert!(ids.contains(&LSP_TYPE_DEFINITION));
    Ok(())
}

#[test]
fn build_flags_implementation_to_feature_ids() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags { implementation: true, ..Default::default() };
    let ids = bf.to_feature_ids();
    assert!(ids.contains(&LSP_IMPLEMENTATION));
    Ok(())
}

#[test]
fn build_flags_workspace_symbol_resolve_excluded_from_feature_ids()
-> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags { workspace_symbol_resolve: true, ..Default::default() };
    let ids = bf.to_feature_ids();
    assert!(ids.is_empty());
    Ok(())
}

#[test]
fn build_flags_execute_command_to_feature_ids() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags { execute_command: true, ..Default::default() };
    let ids = bf.to_feature_ids();
    assert!(ids.contains(&LSP_EXECUTE_COMMAND));
    Ok(())
}

#[test]
fn build_flags_document_links_to_feature_ids() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags { document_links: true, ..Default::default() };
    let ids = bf.to_feature_ids();
    assert!(ids.contains(&LSP_DOCUMENT_LINK));
    Ok(())
}

#[test]
fn build_flags_on_type_formatting_to_feature_ids() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags { on_type_formatting: true, ..Default::default() };
    let ids = bf.to_feature_ids();
    assert!(ids.contains(&LSP_ON_TYPE_FORMATTING));
    Ok(())
}

#[test]
fn build_flags_inline_completion_to_feature_ids() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags { inline_completion: true, ..Default::default() };
    let ids = bf.to_feature_ids();
    assert!(ids.contains(&LSP_INLINE_COMPLETION));
    Ok(())
}

#[test]
fn build_flags_inline_values_to_feature_ids() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags { inline_values: true, ..Default::default() };
    let ids = bf.to_feature_ids();
    assert!(ids.contains(&LSP_INLINE_VALUE));
    Ok(())
}

#[test]
fn build_flags_moniker_to_feature_ids() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags { moniker: true, ..Default::default() };
    let ids = bf.to_feature_ids();
    assert!(ids.contains(&LSP_MONIKER));
    Ok(())
}

#[test]
fn build_flags_source_organize_imports_excluded_from_feature_ids()
-> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags { source_organize_imports: true, ..Default::default() };
    let ids = bf.to_feature_ids();
    assert!(ids.is_empty());
    Ok(())
}

#[test]
fn advertised_features_count_all_fields() -> Result<(), Box<dyn std::error::Error>> {
    let af = AdvertisedFeatures {
        completion: true,
        hover: true,
        definition: true,
        references: true,
        document_symbol: true,
        workspace_symbol: true,
        code_action: true,
        code_lens: true,
        formatting: true,
        range_formatting: true,
        rename: true,
        folding_range: true,
        selection_range: true,
        linked_editing: true,
        inlay_hints: true,
        semantic_tokens: true,
        call_hierarchy: true,
        type_hierarchy: true,
        diagnostic_provider: true,
        document_color: true,
        notebook_document_sync: true,
        notebook_cell_execution: true,
        signature_help: true,
        document_highlight: true,
        declaration: true,
    };
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
fn build_flags_count_all_fields() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags {
        completion: true,
        hover: true,
        definition: true,
        type_definition: true,
        implementation: true,
        references: true,
        document_symbol: true,
        workspace_symbol: true,
        inlay_hints: true,
        pull_diagnostics: true,
        workspace_symbol_resolve: true,
        semantic_tokens: true,
        code_actions: true,
        execute_command: true,
        rename: true,
        document_links: true,
        selection_ranges: true,
        on_type_formatting: true,
        code_lens: true,
        call_hierarchy: true,
        type_hierarchy: true,
        linked_editing: true,
        inline_completion: true,
        inline_values: true,
        notebook_document_sync: true,
        notebook_cell_execution: true,
        moniker: true,
        document_color: true,
        source_organize_imports: true,
        formatting: true,
        range_formatting: true,
        folding_range: true,
        signature_help: true,
        document_highlight: true,
        declaration: true,
    };
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
fn advertised_features_cloned_after_conversion() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags::production();
    let af1 = bf.to_advertised_features();
    let af2 = af1.clone();
    assert_eq!(af1.completion, af2.completion);
    assert_eq!(af1.hover, af2.hover);
    assert_eq!(af1.formatting, af2.formatting);
    Ok(())
}

#[test]
fn build_flags_debug_format_is_comprehensive() -> Result<(), Box<dyn std::error::Error>> {
    let bf = BuildFlags::all();
    let debug_str = format!("{bf:?}");
    assert!(debug_str.contains("BuildFlags"));
    assert!(debug_str.contains("completion"));
    Ok(())
}

#[test]
fn advertised_features_debug_format_is_comprehensive() -> Result<(), Box<dyn std::error::Error>> {
    let af = AdvertisedFeatures::default();
    let debug_str = format!("{af:?}");
    assert!(debug_str.contains("AdvertisedFeatures"));
    assert!(debug_str.contains("completion"));
    Ok(())
}
