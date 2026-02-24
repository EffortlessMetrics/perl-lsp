//! Shared feature contracts for profile parsing, BDD-grid rows, and capability mapping.

use perl_lsp_feature_ids::*;
use serde::Serialize;

/// Canonical metadata for profile aliases and normalization behavior.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct FeatureProfileSpec {
    /// Canonical profile label used by CLI and runtime APIs.
    pub canonical: &'static str,
    /// Additional accepted CLI aliases for this profile.
    pub aliases: &'static [&'static str],
    /// Short human-friendly description for settings/docs tooling.
    pub description: &'static str,
}

const GA_LOCK_ALIASES: &[&str] = &["ga-lock", "ga", "ga_lock"];
const PRODUCTION_ALIASES: &[&str] = &["production", "prod"];
const ALL_ALIASES: &[&str] = &["all"];

/// Canonical profile definitions and alias map.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FeatureProfileKind {
    /// Conservative GA-lock feature profile.
    GaLock,
    /// Default production profile.
    Production,
    /// All features enabled.
    All,
}

impl FeatureProfileKind {
    /// Parse a raw profile token into canonical form.
    pub fn from_str_name(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(Self::current()),
            "ga-lock" | "ga" | "ga_lock" => Some(Self::GaLock),
            "production" | "prod" => Some(Self::Production),
            "all" => Some(Self::All),
            _ => None,
        }
    }

    /// Resolve whether the compiled binary default enables GA-lock mode.
    pub const fn current() -> Self {
        Self::from_ga_lock_enabled(cfg!(feature = "lsp-ga-lock"))
    }

    /// Resolve explicit GA-lock toggle into canonical profile.
    pub const fn from_ga_lock_enabled(ga_lock_enabled: bool) -> Self {
        if ga_lock_enabled { Self::GaLock } else { Self::Production }
    }

    /// Canonical runtime label for diagnostics and APIs.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GaLock => "ga-lock",
            Self::Production => "production",
            Self::All => "all",
        }
    }

    /// All canonical profiles.
    pub const fn all() -> &'static [Self] {
        &[Self::GaLock, Self::Production, Self::All]
    }

    /// Supported CLI tokens, including aliases and backward compatible forms.
    pub const fn supported_cli_profiles() -> &'static [&'static str] {
        const PROFILE_CLI_NAMES: &[&str] =
            &["auto", "ga-lock", "ga", "ga_lock", "prod", "production", "all"];

        PROFILE_CLI_NAMES
    }

    /// Static alias metadata for this profile.
    pub const fn aliases(self) -> &'static [&'static str] {
        match self {
            Self::GaLock => GA_LOCK_ALIASES,
            Self::Production => PRODUCTION_ALIASES,
            Self::All => ALL_ALIASES,
        }
    }
}

/// A serializable profile metadata row for tooling and interoperability.
pub const FEATURE_PROFILE_SPECS: &[FeatureProfileSpec] = &[
    FeatureProfileSpec {
        canonical: "ga-lock",
        aliases: GA_LOCK_ALIASES,
        description: "Conservative GA-lock profile for minimal runtime surface.",
    },
    FeatureProfileSpec {
        canonical: "production",
        aliases: PRODUCTION_ALIASES,
        description: "Production profile for normal runtime feature set.",
    },
    FeatureProfileSpec {
        canonical: "all",
        aliases: ALL_ALIASES,
        description: "All in-tree features enabled for snapshot and testing.",
    },
];

/// Return canonical feature profile descriptors for tooling.
pub const fn feature_profile_specs() -> &'static [FeatureProfileSpec] {
    FEATURE_PROFILE_SPECS
}

#[allow(dead_code, clippy::all)]
pub mod catalog {
    include!(concat!(env!("OUT_DIR"), "/feature_contracts.rs"));
}

use lsp_types::ServerCapabilities;

/// Human-readable BDD-oriented feature row for automation and reporting.
#[derive(Debug, Clone, Serialize)]
pub struct BddFeatureRow {
    pub id: &'static str,
    pub spec: &'static str,
    pub area: &'static str,
    pub maturity: &'static str,
    pub advertised: bool,
    pub counts_in_coverage: bool,
    pub description: &'static str,
    pub tests: &'static [&'static str],
}

pub use catalog::{
    Feature, LSP_VERSION, VERSION, advertised_features, compliance_percent, has_feature,
};

/// All discovered LSP features in canonical declaration order.
pub fn all_features() -> &'static [Feature] {
    catalog::ALL_FEATURES
}

/// Export feature rows suitable for BDD matrices and acceptance criteria tooling.
pub fn bdd_feature_rows() -> Vec<BddFeatureRow> {
    let mut rows = all_features()
        .iter()
        .map(|feature| BddFeatureRow {
            id: feature.id,
            spec: feature.spec,
            area: feature.area,
            maturity: feature.maturity,
            advertised: feature.advertised,
            counts_in_coverage: feature.counts_in_coverage,
            description: feature.description,
            tests: feature.tests,
        })
        .collect::<Vec<_>>();

    rows.sort_by(|a, b| a.area.cmp(b.area).then(a.id.cmp(b.id)));
    rows
}

/// Number of BDD rows that participate in coverage accounting.
pub fn trackable_feature_count_for_grid() -> usize {
    all_features()
        .iter()
        .filter(|feature| feature.maturity != "planned" && feature.counts_in_coverage)
        .count()
}

/// Number of advertised BDD rows that participate in coverage accounting.
pub fn advertised_trackable_feature_count_for_grid() -> usize {
    all_features()
        .iter()
        .filter(|feature| {
            feature.maturity != "planned" && feature.counts_in_coverage && feature.advertised
        })
        .count()
}

/// Compliance percentage for the BDD grid (`advertised / trackable`, rounded).
pub fn compliance_percent_for_grid() -> f32 {
    let trackable = trackable_feature_count_for_grid();
    if trackable == 0 {
        return 0.0;
    }
    let advertised = advertised_trackable_feature_count_for_grid();
    (advertised as f64 / trackable as f64 * 100.0).round() as f32
}

/// Extract feature IDs from LSP `ServerCapabilities`.
pub fn feature_ids_from_caps(c: &ServerCapabilities) -> Vec<&'static str> {
    let mut v = Vec::new();

    // Text Document Features
    if c.completion_provider.is_some() {
        v.push(LSP_COMPLETION);
    }
    if c.hover_provider.is_some() {
        v.push(LSP_HOVER);
    }
    if c.signature_help_provider.is_some() {
        v.push(LSP_SIGNATURE_HELP);
    }
    if c.definition_provider.is_some() {
        v.push(LSP_DEFINITION);
    }
    if c.declaration_provider.is_some() {
        v.push(LSP_DECLARATION);
    }
    if c.notebook_document_sync.is_some() {
        v.push(LSP_NOTEBOOK_DOCUMENT_SYNC);
    }
    if c.type_definition_provider.is_some() {
        v.push(LSP_TYPE_DEFINITION);
    }
    if c.implementation_provider.is_some() {
        v.push(LSP_IMPLEMENTATION);
    }
    if c.references_provider.is_some() {
        v.push(LSP_REFERENCES);
    }
    if c.document_highlight_provider.is_some() {
        v.push(LSP_DOCUMENT_HIGHLIGHT);
    }
    if c.document_symbol_provider.is_some() {
        v.push(LSP_DOCUMENT_SYMBOL);
    }
    if c.code_action_provider.is_some() {
        v.push(LSP_CODE_ACTION);
    }
    if c.code_lens_provider.is_some() {
        v.push(LSP_CODE_LENS);
    }
    if c.document_link_provider.is_some() {
        v.push(LSP_DOCUMENT_LINK);
    }
    if c.color_provider.is_some() {
        v.push(LSP_DOCUMENT_COLOR);
    }
    if c.document_formatting_provider.is_some() {
        v.push(LSP_FORMATTING);
    }
    if c.document_range_formatting_provider.is_some() {
        v.push(LSP_RANGE_FORMATTING);
    }
    if c.document_on_type_formatting_provider.is_some() {
        v.push(LSP_ON_TYPE_FORMATTING);
    }
    if c.rename_provider.is_some() {
        v.push(LSP_RENAME);
    }
    if c.folding_range_provider.is_some() {
        v.push(LSP_FOLDING_RANGE);
    }
    if c.selection_range_provider.is_some() {
        v.push(LSP_SELECTION_RANGE);
    }
    if c.linked_editing_range_provider.is_some() {
        v.push(LSP_LINKED_EDITING_RANGE);
    }
    if c.call_hierarchy_provider.is_some() {
        v.push(LSP_CALL_HIERARCHY);
    }
    if c.semantic_tokens_provider.is_some() {
        v.push(LSP_SEMANTIC_TOKENS);
    }
    if c.moniker_provider.is_some() {
        v.push(LSP_MONIKER);
    }
    // Note: type_hierarchy_provider doesn't exist in lsp-types 0.97
    // This would be added in newer versions of lsp-types
    if c.inline_value_provider.is_some() {
        v.push(LSP_INLINE_VALUE);
    }
    if c.inlay_hint_provider.is_some() {
        v.push(LSP_INLAY_HINT);
    }
    if c.diagnostic_provider.is_some() {
        v.push(LSP_PULL_DIAGNOSTICS);
    }

    // Workspace Features
    if c.workspace_symbol_provider.is_some() {
        v.push(LSP_WORKSPACE_SYMBOL);
    }
    if c.execute_command_provider.is_some() {
        v.push(LSP_EXECUTE_COMMAND);
    }

    // Note: Some features like workspace edit, file operations etc. are in workspace capabilities
    // which are separate from ServerCapabilities

    v.sort();
    v.dedup();
    v
}

/// Build LSP `ServerCapabilities` from feature IDs.
pub fn caps_from_feature_ids(features: &[&str]) -> ServerCapabilities {
    use lsp_types::*;

    let mut caps = ServerCapabilities::default();

    for &feature in features {
        match feature {
            LSP_COMPLETION => {
                caps.completion_provider = Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        "$".to_string(),
                        "@".to_string(),
                        "%".to_string(),
                        ">".to_string(),
                        ":".to_string(),
                    ]),
                    ..Default::default()
                });
            }
            LSP_HOVER => {
                caps.hover_provider = Some(HoverProviderCapability::Simple(true));
            }
            LSP_SIGNATURE_HELP => {
                caps.signature_help_provider = Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    ..Default::default()
                });
            }
            LSP_DEFINITION => {
                caps.definition_provider = Some(OneOf::Left(true));
            }
            LSP_DECLARATION => {
                caps.declaration_provider = Some(DeclarationCapability::Simple(true));
            }
            LSP_NOTEBOOK_DOCUMENT_SYNC => {
                caps.notebook_document_sync = Some(OneOf::Left(NotebookDocumentSyncOptions {
                    notebook_selector: vec![NotebookSelector::ByNotebook {
                        notebook: Notebook::String("jupyter-notebook".to_string()),
                        cells: Some(vec![NotebookCellSelector { language: "perl".to_string() }]),
                    }],
                    save: Some(true),
                }));
            }
            LSP_TYPE_DEFINITION => {
                caps.type_definition_provider =
                    Some(TypeDefinitionProviderCapability::Simple(true));
            }
            LSP_IMPLEMENTATION => {
                caps.implementation_provider = Some(ImplementationProviderCapability::Simple(true));
            }
            LSP_REFERENCES => {
                caps.references_provider = Some(OneOf::Left(true));
            }
            LSP_DOCUMENT_SYMBOL => {
                caps.document_symbol_provider = Some(OneOf::Left(true));
            }
            LSP_CODE_ACTION => {
                caps.code_action_provider = Some(CodeActionProviderCapability::Simple(true));
            }
            LSP_FORMATTING => {
                caps.document_formatting_provider = Some(OneOf::Left(true));
            }
            LSP_RANGE_FORMATTING => {
                caps.document_range_formatting_provider = Some(OneOf::Left(true));
            }
            LSP_RENAME => {
                caps.rename_provider = Some(OneOf::Left(true));
            }
            LSP_FOLDING_RANGE => {
                caps.folding_range_provider = Some(FoldingRangeProviderCapability::Simple(true));
            }
            LSP_SEMANTIC_TOKENS => {
                caps.semantic_tokens_provider =
                    Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::NAMESPACE,
                                    SemanticTokenType::TYPE,
                                    SemanticTokenType::CLASS,
                                    SemanticTokenType::ENUM,
                                    SemanticTokenType::INTERFACE,
                                    SemanticTokenType::STRUCT,
                                    SemanticTokenType::TYPE_PARAMETER,
                                    SemanticTokenType::PARAMETER,
                                    SemanticTokenType::VARIABLE,
                                    SemanticTokenType::PROPERTY,
                                    SemanticTokenType::ENUM_MEMBER,
                                    SemanticTokenType::EVENT,
                                    SemanticTokenType::FUNCTION,
                                    SemanticTokenType::METHOD,
                                    SemanticTokenType::MACRO,
                                    SemanticTokenType::KEYWORD,
                                    SemanticTokenType::MODIFIER,
                                    SemanticTokenType::COMMENT,
                                    SemanticTokenType::STRING,
                                    SemanticTokenType::NUMBER,
                                    SemanticTokenType::REGEXP,
                                    SemanticTokenType::OPERATOR,
                                ],
                                token_modifiers: vec![
                                    SemanticTokenModifier::DECLARATION,
                                    SemanticTokenModifier::DEFINITION,
                                    SemanticTokenModifier::READONLY,
                                    SemanticTokenModifier::STATIC,
                                    SemanticTokenModifier::DEPRECATED,
                                    SemanticTokenModifier::ABSTRACT,
                                    SemanticTokenModifier::ASYNC,
                                    SemanticTokenModifier::MODIFICATION,
                                    SemanticTokenModifier::DOCUMENTATION,
                                    SemanticTokenModifier::DEFAULT_LIBRARY,
                                ],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: Some(true),
                            ..Default::default()
                        },
                    ));
            }
            LSP_DOCUMENT_HIGHLIGHT => {
                caps.document_highlight_provider = Some(OneOf::Left(true));
            }
            LSP_CODE_LENS => {
                caps.code_lens_provider = Some(CodeLensOptions { resolve_provider: Some(true) });
            }
            LSP_DOCUMENT_LINK => {
                caps.document_link_provider = Some(DocumentLinkOptions {
                    resolve_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                });
            }
            LSP_DOCUMENT_COLOR | LSP_COLOR => {
                caps.color_provider = Some(ColorProviderCapability::Simple(true));
            }
            LSP_ON_TYPE_FORMATTING => {
                caps.document_on_type_formatting_provider = Some(DocumentOnTypeFormattingOptions {
                    first_trigger_character: ";".to_string(),
                    more_trigger_character: Some(vec!["}".to_string()]),
                });
            }
            LSP_SELECTION_RANGE => {
                caps.selection_range_provider =
                    Some(SelectionRangeProviderCapability::Simple(true));
            }
            LSP_LINKED_EDITING_RANGE => {
                caps.linked_editing_range_provider =
                    Some(LinkedEditingRangeServerCapabilities::Simple(true));
            }
            LSP_CALL_HIERARCHY => {
                caps.call_hierarchy_provider = Some(CallHierarchyServerCapability::Simple(true));
            }
            LSP_MONIKER => {
                caps.moniker_provider =
                    Some(OneOf::Right(MonikerServerCapabilities::Options(MonikerOptions {
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    })));
            }
            LSP_INLINE_VALUE => {
                caps.inline_value_provider = Some(OneOf::Right(
                    InlineValueServerCapabilities::Options(InlineValueOptions::default()),
                ));
            }
            LSP_INLAY_HINT => {
                caps.inlay_hint_provider =
                    Some(OneOf::Right(InlayHintServerCapabilities::Options(InlayHintOptions {
                        resolve_provider: Some(true),
                        ..Default::default()
                    })));
            }
            LSP_PULL_DIAGNOSTICS => {
                caps.diagnostic_provider =
                    Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
                        identifier: Some("perl-lsp".to_string()),
                        inter_file_dependencies: true,
                        workspace_diagnostics: true,
                        ..Default::default()
                    }));
            }
            LSP_WORKSPACE_SYMBOL => {
                caps.workspace_symbol_provider = Some(OneOf::Left(true));
            }
            LSP_EXECUTE_COMMAND => {
                caps.execute_command_provider = Some(ExecuteCommandOptions {
                    commands: vec!["perl.runCritic".to_string()],
                    ..Default::default()
                });
            }
            _ => {
                // Unknown feature - ignore.
            }
        }
    }

    caps
}

#[cfg(test)]
mod tests {
    use lsp_types::{ColorProviderCapability, ServerCapabilities};

    use super::{LSP_COLOR, LSP_DOCUMENT_COLOR, caps_from_feature_ids, feature_ids_from_caps};

    #[test]
    fn feature_ids_from_caps_reports_catalog_color_id() {
        let caps = ServerCapabilities {
            color_provider: Some(ColorProviderCapability::Simple(true)),
            ..Default::default()
        };

        assert_eq!(feature_ids_from_caps(&caps), vec![LSP_DOCUMENT_COLOR]);
    }

    #[test]
    fn caps_from_feature_ids_accepts_legacy_color_alias() {
        let caps = caps_from_feature_ids(&[LSP_COLOR]);
        assert!(caps.color_provider.is_some());
    }

    #[test]
    fn caps_from_feature_ids_accepts_canonical_color_id() {
        let caps = caps_from_feature_ids(&[LSP_DOCUMENT_COLOR]);
        assert!(caps.color_provider.is_some());
    }
}
