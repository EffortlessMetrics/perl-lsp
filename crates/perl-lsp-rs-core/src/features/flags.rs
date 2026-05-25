#![warn(missing_docs)]
//! Shared feature flag types for LSP profile-driven capability selection.
//!
//! The `BuildFlags` shape is the single source of truth for runtime-constructed
//! capability toggles, while `AdvertisedFeatures` is the runtime-facing projection
//! used by server startup and protocol responses.
use crate::features::ids::*;

/// LSP features advertised to clients for Perl script development
///
/// Controls which capabilities are announced during server initialization,
/// enabling clients to provide appropriate UI elements and functionality
/// for Perl script editing within LSP workflows.
#[derive(Debug, Clone, Default)]
pub struct AdvertisedFeatures {
    /// Code completion for variables, functions, and keywords
    pub completion: bool,
    /// Hover information for symbols and documentation
    pub hover: bool,
    /// Go-to-definition navigation for symbols
    pub definition: bool,
    /// Find-all-references for symbol usage analysis
    pub references: bool,
    /// Document symbol outline for Perl script structure
    pub document_symbol: bool,
    /// Workspace-wide symbol search across Perl parsing files
    pub workspace_symbol: bool,
    /// Automated code actions and refactoring suggestions
    pub code_action: bool,
    /// Code lens with reference counts and actionable information
    pub code_lens: bool,
    /// Full document formatting with perltidy integration
    pub formatting: bool,
    /// Range-specific formatting for selected code sections
    pub range_formatting: bool,
    /// Symbol renaming with workspace-wide updates
    pub rename: bool,
    /// Code folding for improved Perl script navigation
    pub folding_range: bool,
    /// Smart text selection expansion for efficient editing
    pub selection_range: bool,
    /// Linked editing for synchronized symbol updates
    pub linked_editing: bool,
    /// Inline type and parameter hints for clarity
    pub inlay_hints: bool,
    /// Semantic syntax highlighting for Perl scripts
    pub semantic_tokens: bool,
    /// Call hierarchy navigation for function relationships
    pub call_hierarchy: bool,
    /// Type hierarchy for object-oriented Perl parsing
    pub type_hierarchy: bool,
    /// Pull-based diagnostic reporting for error detection
    pub diagnostic_provider: bool,
    /// Document color detection for hex codes and ANSI colors
    pub document_color: bool,
    /// Notebook document sync (didOpen/didChange/didSave/didClose)
    pub notebook_document_sync: bool,
    /// Notebook cell execution summary tracking
    pub notebook_cell_execution: bool,
    /// Signature help for function parameters
    pub signature_help: bool,
    /// Document highlight for symbol occurrences
    pub document_highlight: bool,
    /// Go-to-declaration navigation
    pub declaration: bool,
    /// Inline completion suggestions provider
    pub inline_completion: bool,
}

/// Build-time feature flags for conditional LSP capability compilation
///
/// Controls which capabilities are compiled into the LSP server binary,
/// allowing for optimized builds targeted at specific Perl parsing
/// deployment scenarios within enterprise LSP environments.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildFlags {
    /// Code completion provider compilation flag
    pub completion: bool,
    /// Hover information provider compilation flag
    pub hover: bool,
    /// Go-to-definition provider compilation flag
    pub definition: bool,
    /// Type definition navigation compilation flag
    pub type_definition: bool,
    /// Implementation finding compilation flag
    pub implementation: bool,
    /// Find-all-references provider compilation flag
    pub references: bool,
    /// Document symbol outline provider compilation flag
    pub document_symbol: bool,
    /// Workspace symbol search provider compilation flag
    pub workspace_symbol: bool,
    /// Inlay hints provider compilation flag
    pub inlay_hints: bool,
    /// Pull-based diagnostics provider compilation flag
    pub pull_diagnostics: bool,
    /// Workspace symbol resolution provider compilation flag
    pub workspace_symbol_resolve: bool,
    /// Semantic token highlighting provider compilation flag
    pub semantic_tokens: bool,
    /// Code actions provider compilation flag
    pub code_actions: bool,
    /// Command execution provider compilation flag
    pub execute_command: bool,
    /// Symbol renaming provider compilation flag
    pub rename: bool,
    /// Document links provider compilation flag
    pub document_links: bool,
    /// Smart text selection ranges provider compilation flag
    pub selection_ranges: bool,
    /// On-type formatting provider compilation flag
    pub on_type_formatting: bool,
    /// Code lens provider compilation flag
    pub code_lens: bool,
    /// Call hierarchy navigation provider compilation flag
    pub call_hierarchy: bool,
    /// Type hierarchy navigation provider compilation flag
    pub type_hierarchy: bool,
    /// Linked editing ranges provider compilation flag
    pub linked_editing: bool,
    /// Inline completion suggestions provider compilation flag
    pub inline_completion: bool,
    /// Inline values for debugging provider compilation flag
    pub inline_values: bool,
    /// Notebook document sync provider compilation flag
    pub notebook_document_sync: bool,
    /// Notebook cell execution summary tracking compilation flag
    pub notebook_cell_execution: bool,
    /// Stable symbol identifiers provider compilation flag
    pub moniker: bool,
    /// Document color provider compilation flag for color swatches in strings and comments
    pub document_color: bool,
    /// Source organize imports capability (GA-lock excludes this)
    pub source_organize_imports: bool,
    /// Document formatting provider compilation flag
    pub formatting: bool,
    /// Range formatting provider compilation flag
    pub range_formatting: bool,
    /// Folding range provider compilation flag
    pub folding_range: bool,
    /// Signature help provider compilation flag
    pub signature_help: bool,
    /// Document highlight provider compilation flag
    pub document_highlight: bool,
    /// Declaration provider compilation flag
    pub declaration: bool,
}

impl BuildFlags {
    /// Convert build flags to advertised features.
    pub fn to_advertised_features(&self) -> AdvertisedFeatures {
        AdvertisedFeatures {
            completion: self.completion,
            hover: self.hover,
            definition: self.definition,
            references: self.references,
            document_symbol: self.document_symbol,
            workspace_symbol: self.workspace_symbol,
            code_action: self.code_actions,
            code_lens: self.code_lens,
            formatting: self.formatting,
            range_formatting: self.range_formatting,
            rename: self.rename,
            folding_range: self.folding_range,
            selection_range: self.selection_ranges,
            linked_editing: self.linked_editing,
            inlay_hints: self.inlay_hints,
            semantic_tokens: self.semantic_tokens,
            call_hierarchy: self.call_hierarchy,
            type_hierarchy: self.type_hierarchy,
            diagnostic_provider: self.pull_diagnostics,
            document_color: self.document_color,
            notebook_document_sync: self.notebook_document_sync,
            notebook_cell_execution: self.notebook_cell_execution,
            signature_help: self.signature_help,
            document_highlight: self.document_highlight,
            declaration: self.declaration,
            inline_completion: self.inline_completion,
        }
    }

    /// Convert build flags to advertised feature ids used by BDD and tooling layers.
    pub fn to_feature_ids(&self) -> Vec<&'static str> {
        let mut ids = Vec::new();

        if self.completion {
            ids.push(LSP_COMPLETION);
        }
        if self.hover {
            ids.push(LSP_HOVER);
        }
        if self.definition {
            ids.push(LSP_DEFINITION);
        }
        if self.type_definition {
            ids.push(LSP_TYPE_DEFINITION);
        }
        if self.implementation {
            ids.push(LSP_IMPLEMENTATION);
        }
        if self.references {
            ids.push(LSP_REFERENCES);
        }
        if self.document_symbol {
            ids.push(LSP_DOCUMENT_SYMBOL);
        }
        if self.workspace_symbol {
            ids.push(LSP_WORKSPACE_SYMBOL);
        }
        if self.inlay_hints {
            ids.push(LSP_INLAY_HINT);
        }
        if self.pull_diagnostics {
            ids.push(LSP_PULL_DIAGNOSTICS);
        }
        if self.semantic_tokens {
            ids.push(LSP_SEMANTIC_TOKENS);
        }
        if self.code_actions {
            ids.push(LSP_CODE_ACTION);
        }
        if self.execute_command {
            ids.push(LSP_EXECUTE_COMMAND);
        }
        if self.rename {
            ids.push(LSP_RENAME);
        }
        if self.document_links {
            ids.push(LSP_DOCUMENT_LINK);
        }
        if self.selection_ranges {
            ids.push(LSP_SELECTION_RANGE);
        }
        if self.on_type_formatting {
            ids.push(LSP_ON_TYPE_FORMATTING);
        }
        if self.code_lens {
            ids.push(LSP_CODE_LENS);
        }
        if self.call_hierarchy {
            ids.push(LSP_CALL_HIERARCHY);
        }
        if self.type_hierarchy {
            ids.push(LSP_TYPE_HIERARCHY);
        }
        if self.linked_editing {
            ids.push(LSP_LINKED_EDITING_RANGE);
        }
        if self.inline_completion {
            ids.push(LSP_INLINE_COMPLETION);
        }
        if self.inline_values {
            ids.push(LSP_INLINE_VALUE);
        }
        if self.notebook_document_sync {
            ids.push(LSP_NOTEBOOK_DOCUMENT_SYNC);
        }
        if self.notebook_cell_execution {
            ids.push(LSP_NOTEBOOK_CELL_EXECUTION);
        }
        if self.moniker {
            ids.push(LSP_MONIKER);
        }
        if self.document_color {
            ids.push(LSP_DOCUMENT_COLOR);
        }
        if self.formatting {
            ids.push(LSP_FORMATTING);
        }
        if self.range_formatting {
            ids.push(LSP_RANGE_FORMATTING);
            // ranges formatting (multi-range, LSP 3.18) is gated on the same flag because
            // both features require perltidy and the handler already exists.
            ids.push(LSP_RANGES_FORMATTING);
        }
        if self.folding_range {
            ids.push(LSP_FOLDING_RANGE);
        }
        if self.signature_help {
            ids.push(LSP_SIGNATURE_HELP);
        }
        if self.document_highlight {
            ids.push(LSP_DOCUMENT_HIGHLIGHT);
        }
        if self.declaration {
            ids.push(LSP_DECLARATION);
        }

        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// Default production-ready capabilities.
    pub fn production() -> Self {
        Self {
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
        }
    }

    /// All capabilities for testing.
    pub fn all() -> Self {
        Self {
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
        }
    }

    /// Conservative GA-lock capabilities.
    pub fn ga_lock() -> Self {
        Self {
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
            inline_values: false,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AdvertisedFeatures, BuildFlags};
    use crate::features::contracts::all_features;
    use crate::features::ids::LSP_DOCUMENT_COLOR;

    #[test]
    fn feature_ids_are_stable_and_sorted() {
        let ga_lock = BuildFlags::ga_lock();
        let ids = ga_lock.to_feature_ids();

        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();

        assert_eq!(ids, sorted);
        assert!(
            sorted.windows(2).all(|window| window[0] < window[1]),
            "expected strictly increasing feature id ordering",
        );
    }

    #[test]
    fn feature_ids_are_valid_in_catalog() {
        let ids = BuildFlags::all().to_feature_ids();
        let known_ids: std::collections::HashSet<_> =
            all_features().iter().map(|feature| feature.id).collect();
        let unknown: Vec<_> = ids.iter().copied().filter(|id| !known_ids.contains(id)).collect();
        assert!(unknown.is_empty(), "non-catalog feature IDs emitted: {:?}", unknown);
    }

    #[test]
    fn document_color_uses_bdd_catalog_id() {
        let flags = BuildFlags { document_color: true, ..Default::default() };
        assert_eq!(flags.to_feature_ids(), vec![LSP_DOCUMENT_COLOR]);
    }

    #[test]
    fn production_has_expected_profile_shape() {
        let production = BuildFlags::production();
        assert!(production.completion);
        assert!(production.formatting);
        assert_eq!(
            production,
            BuildFlags {
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
                declaration: true
            }
        );
    }

    // ── ga_lock profile shape ───────────────────────────────────────

    #[test]
    fn ga_lock_has_expected_profile_shape() {
        let ga = BuildFlags::ga_lock();
        assert!(ga.completion);
        assert!(ga.hover);
        assert!(ga.definition);
        assert!(ga.formatting, "ga-lock should include formatting");
        assert!(ga.range_formatting, "ga-lock should include range_formatting");
        assert!(!ga.inline_values, "ga-lock should exclude inline_values");
    }

    // ── all() profile enables everything ────────────────────────────

    #[test]
    fn all_profile_enables_every_flag() {
        let all = BuildFlags::all();
        assert!(all.completion);
        assert!(all.hover);
        assert!(all.definition);
        assert!(all.type_definition);
        assert!(all.implementation);
        assert!(all.references);
        assert!(all.document_symbol);
        assert!(all.workspace_symbol);
        assert!(all.inlay_hints);
        assert!(all.pull_diagnostics);
        assert!(all.workspace_symbol_resolve);
        assert!(all.semantic_tokens);
        assert!(all.code_actions);
        assert!(all.execute_command);
        assert!(all.rename);
        assert!(all.document_links);
        assert!(all.selection_ranges);
        assert!(all.on_type_formatting);
        assert!(all.code_lens);
        assert!(all.call_hierarchy);
        assert!(all.type_hierarchy);
        assert!(all.linked_editing);
        assert!(all.inline_completion);
        assert!(all.inline_values);
        assert!(all.notebook_document_sync);
        assert!(all.notebook_cell_execution);
        assert!(all.moniker);
        assert!(all.document_color);
        assert!(all.source_organize_imports);
        assert!(all.formatting);
        assert!(all.range_formatting);
        assert!(all.folding_range);
        assert!(all.signature_help);
        assert!(all.document_highlight);
        assert!(all.declaration);
    }

    // ── Default is all-false ────────────────────────────────────────

    #[test]
    fn default_flags_are_all_false() {
        let default = BuildFlags::default();
        assert!(default.to_feature_ids().is_empty(), "default flags should yield no features");
    }

    // ── all() produces strictly more IDs than ga_lock() ─────────────

    #[test]
    fn all_produces_superset_of_ga_lock_ids() {
        let all_ids = BuildFlags::all().to_feature_ids();
        let ga_ids = BuildFlags::ga_lock().to_feature_ids();
        for id in &ga_ids {
            assert!(all_ids.contains(id), "'all' profile should contain ga-lock feature '{id}'");
        }
        assert!(all_ids.len() >= ga_ids.len());
    }

    // ── to_advertised_features mapping ──────────────────────────────

    #[test]
    fn to_advertised_features_maps_completion() {
        let flags = BuildFlags { completion: true, ..Default::default() };
        let adv = flags.to_advertised_features();
        assert!(adv.completion);
        assert!(!adv.hover);
    }

    #[test]
    fn to_advertised_features_maps_code_actions_to_code_action() {
        let flags = BuildFlags { code_actions: true, ..Default::default() };
        let adv = flags.to_advertised_features();
        assert!(
            adv.code_action,
            "BuildFlags.code_actions should map to AdvertisedFeatures.code_action"
        );
    }

    #[test]
    fn to_advertised_features_maps_pull_diagnostics_to_diagnostic_provider() {
        let flags = BuildFlags { pull_diagnostics: true, ..Default::default() };
        let adv = flags.to_advertised_features();
        assert!(
            adv.diagnostic_provider,
            "BuildFlags.pull_diagnostics should map to AdvertisedFeatures.diagnostic_provider"
        );
    }

    #[test]
    fn to_advertised_features_maps_selection_ranges_to_selection_range() {
        let flags = BuildFlags { selection_ranges: true, ..Default::default() };
        let adv = flags.to_advertised_features();
        assert!(
            adv.selection_range,
            "BuildFlags.selection_ranges should map to AdvertisedFeatures.selection_range"
        );
    }

    #[test]
    fn default_advertised_features_are_all_false() {
        let adv = AdvertisedFeatures::default();
        assert!(!adv.completion);
        assert!(!adv.hover);
        assert!(!adv.definition);
        assert!(!adv.references);
        assert!(!adv.document_symbol);
        assert!(!adv.workspace_symbol);
        assert!(!adv.code_action);
        assert!(!adv.code_lens);
        assert!(!adv.formatting);
        assert!(!adv.rename);
    }

    // ── Individual flag toggling produces exactly one feature ID ─────

    #[test]
    fn single_flag_produces_single_id() {
        let cases: Vec<(&str, BuildFlags)> = vec![
            ("completion", BuildFlags { completion: true, ..Default::default() }),
            ("hover", BuildFlags { hover: true, ..Default::default() }),
            ("definition", BuildFlags { definition: true, ..Default::default() }),
            ("references", BuildFlags { references: true, ..Default::default() }),
            ("rename", BuildFlags { rename: true, ..Default::default() }),
            ("formatting", BuildFlags { formatting: true, ..Default::default() }),
            ("signature_help", BuildFlags { signature_help: true, ..Default::default() }),
            ("declaration", BuildFlags { declaration: true, ..Default::default() }),
        ];
        for (label, flags) in cases {
            let ids = flags.to_feature_ids();
            assert_eq!(
                ids.len(),
                1,
                "flag '{label}' should produce exactly 1 feature id, got {ids:?}"
            );
        }
    }

    // ── Production vs all diff ──────────────────────────────────────

    #[test]
    fn production_and_all_match_on_formatting() {
        let prod = BuildFlags::production();
        let all = BuildFlags::all();
        assert!(prod.formatting);
        assert!(all.formatting);
        assert!(prod.range_formatting);
        assert!(all.range_formatting);
    }

    #[test]
    fn ga_lock_and_production_differ_on_inline_values() {
        let ga = BuildFlags::ga_lock();
        let prod = BuildFlags::production();
        assert!(!ga.inline_values, "ga-lock excludes inline_values");
        assert!(prod.inline_values, "production includes inline_values");
    }
}
