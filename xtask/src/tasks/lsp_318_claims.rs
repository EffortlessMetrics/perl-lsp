//! Validate the selected LSP 3.18 claim boundary.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use serde_json::Value;
use std::fs;
use std::path::Path;

const SPEC_PATH: &str = "docs/specs/PLSP-SPEC-0029-lsp-318-conformance-boundary.md";
const MATRIX_PATH: &str = "docs/specs/lsp-318-conformance-matrix.md";
const NEGATIVE_CLAIMS_TEST: &str = "crates/perl-lsp-rs/tests/lsp_318_negative_claims.rs";
const REGISTRATION_TEST: &str = "crates/perl-lsp-rs/tests/lsp_registration_tests.rs";
const DIAGNOSTIC_ENRICHMENT_TEST: &str =
    "crates/perl-lsp-rs/tests/lsp_diagnostic_enrichment_test.rs";
const REFRESH_METHODS_TEST: &str = "crates/perl-lsp-rs/tests/lsp_refresh_methods_tests.rs";
const SCHEMA_VALIDATION_TEST: &str = "crates/perl-lsp-rs/tests/lsp_schema_validation.rs";
const SEMANTIC_LEGEND_TEST: &str = "crates/perl-lsp-rs/tests/lsp_semantic_legend_contract_tests.rs";
const COMPLETION_TEST: &str = "crates/perl-lsp-rs/tests/lsp_completion_tests.rs";
const CODE_LENS_TEST: &str = "crates/perl-lsp-rs/tests/lsp_codelens_tests.rs";
const WINDOW_TEST: &str = "crates/perl-lsp-rs/tests/lsp_window_tests.rs";
const CLIENT_REQUESTS: &str = "crates/perl-lsp-rs/src/runtime/client_requests.rs";
const LIFECYCLE_CAPABILITIES: &str = "crates/perl-lsp-rs/src/runtime/lifecycle/capabilities.rs";
const RUNTIME_LANGUAGE_MISC: &str = "crates/perl-lsp-rs/src/runtime/language/misc.rs";
const REFACTOR_RUNTIME_RECEIPTS: &str =
    "crates/perl-lsp-rs/src/runtime/language/refactor_runtime_blocker_receipts.rs";
const LIFECYCLE_WATCHERS: &str = "crates/perl-lsp-rs/src/runtime/lifecycle/watchers.rs";
const RUNTIME_REFRESH: &str = "crates/perl-lsp-rs/src/runtime/refresh.rs";
const STATE_DOCUMENT: &str = "crates/perl-lsp-rs/src/state/document.rs";
const FEATURE_CATALOG: &str = "features.toml";

const CAPABILITY_SNAPSHOTS: &[&str] = &[
    "crates/perl-lsp-rs/tests/snapshots/all_capabilities.json",
    "crates/perl-lsp-rs/tests/snapshots/ga_lock_capabilities.json",
    "crates/perl-lsp-rs/tests/snapshots/production_capabilities.json",
];

const SPEC_MARKERS: &[RequiredMarker] = &[
    RequiredMarker { label: "claim guard command", marker: "check-lsp-318-claims" },
    RequiredMarker { label: "negative claim gates section", marker: "## Negative Claim Gates" },
    RequiredMarker {
        label: "selected-surface claim boundary",
        marker: "This spec may claim that `perl-lsp` has a documented LSP 3.18 selected-surface",
    },
    RequiredMarker {
        label: "StringValue object-form non-claim",
        marker: "object-form `StringValue` inline completion insert text",
    },
];

const MATRIX_MARKERS: &[RequiredMarker] = &[
    RequiredMarker {
        label: "matrix generator command",
        marker: "cargo xtask generate-lsp-318-matrix --check",
    },
    RequiredMarker { label: "matrix inline-completion row", marker: "Standard inline completion" },
    RequiredMarker {
        label: "matrix textDocumentContent row",
        marker: "`workspace/textDocumentContent`",
    },
    RequiredMarker {
        label: "matrix StringValue object-form row",
        marker: "Object-form `StringValue` inline insert text",
    },
    RequiredMarker {
        label: "matrix negative-gated vocabulary",
        marker: "`negative-gated+documented`",
    },
    RequiredMarker { label: "matrix notebook classification", marker: "Notebook 3.18 additions" },
];

const NEGATIVE_TEST_MARKERS: &[RequiredMarker] = &[
    RequiredMarker {
        label: "StringValue object-form negative receipt",
        marker: "inline_completion_does_not_emit_object_form_string_value",
    },
    RequiredMarker {
        label: "unsupported capability snapshot assertions",
        marker: "initialize_does_not_advertise_unimplemented_318_capabilities",
    },
    RequiredMarker {
        label: "semantic token delta negative route",
        marker: "semantic_tokens_delta_request_returns_method_not_found",
    },
    RequiredMarker {
        label: "CompletionList.applyKind gate",
        marker: "completion_response_does_not_emit_apply_kind_without_client_support",
    },
    RequiredMarker { label: "CompletionList.itemDefaults.data gate", marker: "itemDefaults" },
    RequiredMarker {
        label: "CodeAction documentation and tag gates",
        marker: "code_action_and_workspace_edit_responses_do_not_emit_optional_318_shapes",
    },
    RequiredMarker {
        label: "CodeAction.tags supported-client trust boundary",
        marker: "code_actions_do_not_emit_llm_generated_tags_for_deterministic_actions",
    },
    RequiredMarker {
        label: "CodeAction.tags resolve echo gate",
        marker: "code_action_resolve_does_not_echo_tags_without_client_support",
    },
    RequiredMarker {
        label: "CodeAction.documentation positive gate",
        marker: "code_action_documentation_advertised_when_supported",
    },
    RequiredMarker {
        label: "WorkspaceEdit metadata absence gate",
        marker: "assert_no_workspace_edit_metadata",
    },
    RequiredMarker {
        label: "ApplyWorkspaceEditParams.metadata positive gate",
        marker: "apply_workspace_edit_metadata_emitted_when_supported_for_refactor_request",
    },
    RequiredMarker {
        label: "ApplyWorkspaceEditParams.metadata negative gate",
        marker: "apply_workspace_edit_metadata_absent_without_metadata_support",
    },
    RequiredMarker { label: "SnippetTextEdit gate", marker: "snippet" },
    RequiredMarker {
        label: "Diagnostic MarkupContent gate",
        marker: "diagnostics_keep_plain_string_messages_without_markup_support",
    },
    RequiredMarker {
        label: "RelativePattern/baseUri gate",
        marker: "dynamic_file_watcher_registration_uses_string_globs_not_relative_patterns",
    },
    RequiredMarker {
        label: "workspace/foldingRange/refresh gate",
        marker: "folding_range_refresh_is_not_sent_without_client_support",
    },
    RequiredMarker {
        label: "normal runtime window messages avoid Debug",
        marker: "window_message_type_does_not_emit_debug_level",
    },
    RequiredMarker {
        label: "non-CodeLens Command.tooltip gate",
        marker: "assert_no_command_tooltip",
    },
    RequiredMarker {
        label: "trusted markdown command/theme-icon gate",
        marker: "markdown_surfaces_do_not_emit_trusted_commands_or_theme_icons_without_support",
    },
    RequiredMarker {
        label: "experimental inline-completion provider gate",
        marker: "/experimental/inlineCompletionProvider",
    },
    RequiredMarker {
        label: "non-spec ranges-formatting provider gate",
        marker: "/documentRangesFormattingProvider",
    },
];

const FEATURE_CATALOG_MARKERS: &[RequiredMarker] = &[
    RequiredMarker {
        label: "inline completion feature catalog row",
        marker: "id = \"lsp.inline_completion\"",
    },
    RequiredMarker {
        label: "multi-range formatting feature catalog row",
        marker: "id = \"lsp.ranges_formatting\"",
    },
    RequiredMarker {
        label: "diagnostic markup support feature catalog row",
        marker: "id = \"lsp.diagnostic.markup_message_support\"",
    },
    RequiredMarker {
        label: "textDocumentContent feature catalog row",
        marker: "id = \"lsp.text_document_content\"",
    },
    RequiredMarker {
        label: "textDocumentContent refresh feature catalog row",
        marker: "id = \"lsp.text_document_content_refresh\"",
    },
    RequiredMarker {
        label: "folding range refresh feature catalog row",
        marker: "id = \"lsp.folding_range_refresh\"",
    },
    RequiredMarker {
        label: "CodeLens resolveSupport.properties feature catalog row",
        marker: "id = \"lsp.code_lens_resolve_support_properties\"",
    },
    RequiredMarker {
        label: "CompletionList.itemDefaults.data feature catalog row",
        marker: "id = \"lsp.completion_list_item_defaults_data\"",
    },
    RequiredMarker {
        label: "CompletionList.applyKind feature catalog row",
        marker: "id = \"lsp.completion_list_apply_kind\"",
    },
    RequiredMarker {
        label: "CodeAction.documentation feature catalog row",
        marker: "id = \"lsp.code_action_documentation\"",
    },
    RequiredMarker {
        label: "SnippetTextEdit feature catalog row",
        marker: "id = \"lsp.workspace_edit_snippet_text_edit\"",
    },
    RequiredMarker {
        label: "ApplyWorkspaceEditParams.metadata feature catalog row",
        marker: "id = \"lsp.apply_edit_metadata\"",
    },
];

const REFRESH_METHODS_TEST_MARKERS: &[RequiredMarker] = &[RequiredMarker {
    label: "workspace/foldingRange/refresh positive receipt",
    marker: "lsp_refresh_folding_range_sent_with_client_support",
}];

const REGISTRATION_TEST_MARKERS: &[RequiredMarker] = &[
    RequiredMarker {
        label: "RelativePattern watcher positive receipt",
        marker: "relative_pattern_clients_receive_relative_file_watchers",
    },
    RequiredMarker {
        label: "RelativePattern watcher fallback receipt",
        marker: "relative_pattern_clients_fall_back_to_string_watchers_without_valid_workspace_uri",
    },
];

const DIAGNOSTIC_ENRICHMENT_TEST_MARKERS: &[RequiredMarker] = &[RequiredMarker {
    label: "Diagnostic.message MarkupContent positive receipt",
    marker: "test_markup_message_support_populates_standard_message_markup",
}];

const SCHEMA_VALIDATION_TEST_MARKERS: &[RequiredMarker] = &[
    RequiredMarker {
        label: "SignatureHelp nullable activeParameter compatibility",
        marker: "signature_help_active_parameter_accepts_lsp_318_null",
    },
    RequiredMarker {
        label: "Diagnostic.message MarkupContent schema compatibility",
        marker: "diagnostic_message_accepts_lsp_318_markup_content",
    },
];

const SEMANTIC_LEGEND_TEST_MARKERS: &[RequiredMarker] = &[
    RequiredMarker {
        label: "SemanticTokenTypes.label positive receipt",
        marker: "semantic_token_label_type_decodes_for_perl_labels",
    },
    RequiredMarker {
        label: "semantic-token legend bounds receipt",
        marker: "semantic_token_result_indexes_stay_within_advertised_legend_bounds",
    },
];

const COMPLETION_TEST_MARKERS: &[RequiredMarker] = &[
    RequiredMarker {
        label: "CompletionList.itemDefaults.data positive receipt",
        marker: "test_completion_list_item_defaults_data_emitted_when_supported",
    },
    RequiredMarker {
        label: "CompletionList.itemDefaults.data negative receipt",
        marker: "test_completion_list_item_defaults_data_absent_without_support",
    },
    RequiredMarker {
        label: "CompletionList.applyKind positive receipt",
        marker: "test_completion_list_apply_kind_emitted_when_supported",
    },
    RequiredMarker {
        label: "CompletionList.applyKind fallback receipt",
        marker: "test_completion_list_apply_kind_absent_without_item_defaults",
    },
];

const CODE_LENS_TEST_MARKERS: &[RequiredMarker] = &[
    RequiredMarker {
        label: "CodeLens eager fallback without resolve support",
        marker: "test_codelens_eager_without_resolve_support",
    },
    RequiredMarker {
        label: "CodeLens command resolve support positive gate",
        marker: "test_codelens_defers_command_when_resolve_support_allows_command",
    },
    RequiredMarker {
        label: "CodeLens command resolve support negative gate",
        marker: "test_codelens_eager_when_resolve_support_lacks_command",
    },
    RequiredMarker {
        label: "CodeLens Command.tooltip positive receipt",
        marker: "test_codelens_commands_include_lsp_318_tooltips",
    },
    RequiredMarker {
        label: "CodeLens resolve Command.tooltip positive receipt",
        marker: "test_codelens_resolve_adds_lsp_318_command_tooltip",
    },
];

const WINDOW_TEST_MARKERS: &[RequiredMarker] = &[
    RequiredMarker { label: "MessageType.Debug discriminant", marker: "MessageType::Debug, 5" },
    RequiredMarker {
        label: "MessageType.Debug positive receipt",
        marker: "lsp_window_debug_message_type_serializes_to_five",
    },
];

const CAPABILITY_ABSENCE_CHECKS: &[JsonAbsenceCheck] = &[
    JsonAbsenceCheck {
        pointer: "/documentRangesFormattingProvider",
        label: "non-spec multi-range formatting capability",
        reason: "multi-range formatting must be advertised through documentRangeFormattingProvider.rangesSupport",
    },
    JsonAbsenceCheck {
        pointer: "/experimental/inlineCompletionProvider",
        label: "experimental inline completion provider",
        reason: "inline completion is a standard top-level LSP 3.18 capability",
    },
    JsonAbsenceCheck {
        pointer: "/semanticTokensProvider/full/delta",
        label: "semantic-token delta",
        reason: "delta requires resultId state and real delta responses",
    },
    JsonAbsenceCheck {
        pointer: "/completionProvider/applyKind",
        label: "CompletionList.applyKind",
        reason: "applyKind is a CompletionList response field, not an initialize server capability",
    },
    JsonAbsenceCheck {
        pointer: "/completionProvider/itemDefaults/data",
        label: "CompletionList.itemDefaults.data",
        reason: "default data must stay absent until explicitly supported and tested",
    },
    JsonAbsenceCheck {
        pointer: "/codeActionProvider/documentation",
        label: "CodeAction.documentation",
        reason: "code action documentation is client-capability gated and must stay absent from static snapshots",
    },
    JsonAbsenceCheck {
        pointer: "/workspace/foldingRange",
        label: "workspace/foldingRange server capability",
        reason: "folding refresh is client-capability gated and must not be advertised as a server capability",
    },
];

const RAW_SNAPSHOT_PATTERNS: &[RawPatternCheck] = &[
    RawPatternCheck {
        needle: "documentRangesFormattingProvider",
        label: "non-spec documentRangesFormattingProvider snapshot",
    },
    RawPatternCheck {
        needle: "experimental.inlineCompletionProvider",
        label: "experimental inline-completion provider snapshot",
    },
    RawPatternCheck { needle: "\"delta\": true", label: "semantic-token delta snapshot" },
    RawPatternCheck { needle: "applyKind:", label: "CompletionList.applyKind snapshot" },
    RawPatternCheck { needle: "\"applyKind\"", label: "CompletionList.applyKind JSON snapshot" },
];

const FEATURE_CATALOG_FORBIDDEN_PATTERNS: &[RawPatternCheck] = &[
    RawPatternCheck {
        needle: "documentRangesFormattingProvider",
        label: "non-spec documentRangesFormattingProvider feature claim",
    },
    RawPatternCheck {
        needle: "experimental.inlineCompletionProvider",
        label: "experimental inline-completion provider feature claim",
    },
    RawPatternCheck {
        needle: "semanticTokens/full/delta",
        label: "semantic-token delta feature claim",
    },
    RawPatternCheck { needle: "CodeAction.tags", label: "CodeAction.tags feature claim" },
    RawPatternCheck { needle: "Command.tooltip", label: "Command.tooltip feature claim" },
    RawPatternCheck { needle: "RelativePattern", label: "RelativePattern feature claim" },
    RawPatternCheck { needle: "supportThemeIcons", label: "markdown theme-icon feature claim" },
    RawPatternCheck { needle: "enabledCommands", label: "trusted markdown command feature claim" },
];

#[derive(Clone, Copy)]
struct RequiredMarker {
    label: &'static str,
    marker: &'static str,
}

#[derive(Clone, Copy)]
struct JsonAbsenceCheck {
    pointer: &'static str,
    label: &'static str,
    reason: &'static str,
}

#[derive(Clone, Copy)]
struct RawPatternCheck {
    needle: &'static str,
    label: &'static str,
}

#[derive(Debug)]
struct Violation {
    rel_path: String,
    line: usize,
    label: &'static str,
    detail: String,
}

pub fn run() -> Result<()> {
    let root = project_root()?;
    let mut violations = Vec::new();

    check_required_markers(&root, SPEC_PATH, SPEC_MARKERS, &mut violations)?;
    check_required_markers(&root, MATRIX_PATH, MATRIX_MARKERS, &mut violations)?;
    check_required_markers(&root, NEGATIVE_CLAIMS_TEST, NEGATIVE_TEST_MARKERS, &mut violations)?;
    check_required_markers(
        &root,
        REFRESH_METHODS_TEST,
        REFRESH_METHODS_TEST_MARKERS,
        &mut violations,
    )?;
    check_required_markers(&root, REGISTRATION_TEST, REGISTRATION_TEST_MARKERS, &mut violations)?;
    check_required_markers(
        &root,
        DIAGNOSTIC_ENRICHMENT_TEST,
        DIAGNOSTIC_ENRICHMENT_TEST_MARKERS,
        &mut violations,
    )?;
    check_required_markers(
        &root,
        SCHEMA_VALIDATION_TEST,
        SCHEMA_VALIDATION_TEST_MARKERS,
        &mut violations,
    )?;
    let semantic_legend_markers = SEMANTIC_LEGEND_TEST_MARKERS;
    check_required_markers(&root, SEMANTIC_LEGEND_TEST, semantic_legend_markers, &mut violations)?;
    check_required_markers(&root, COMPLETION_TEST, COMPLETION_TEST_MARKERS, &mut violations)?;
    check_required_markers(&root, CODE_LENS_TEST, CODE_LENS_TEST_MARKERS, &mut violations)?;
    check_required_markers(&root, WINDOW_TEST, WINDOW_TEST_MARKERS, &mut violations)?;
    check_feature_catalog(&root, &mut violations)?;
    check_capability_snapshots(&root, &mut violations)?;
    check_folding_range_refresh_guard(&root, &mut violations)?;
    check_relative_pattern_guard(&root, &mut violations)?;
    check_code_lens_resolve_support_guard(&root, &mut violations)?;
    check_completion_item_defaults_data_guard(&root, &mut violations)?;
    check_completion_apply_kind_guard(&root, &mut violations)?;
    check_code_action_documentation_guard(&root, &mut violations)?;
    check_code_action_tag_guard(&root, &mut violations)?;
    check_snippet_text_edit_guard(&root, &mut violations)?;
    check_apply_edit_metadata_guard(&root, &mut violations)?;
    check_message_type_debug_support(&root, &mut violations)?;

    if violations.is_empty() {
        println!(
            "LSP 3.18 claim guard OK: {} capability snapshots, {} feature markers, {} negative-test markers, {} positive refresh markers, {} RelativePattern registration markers, {} diagnostic markers, {} schema markers, {} semantic legend markers, {} completion markers, {} CodeLens markers, {} window markers, {} spec markers checked",
            CAPABILITY_SNAPSHOTS.len(),
            FEATURE_CATALOG_MARKERS.len(),
            NEGATIVE_TEST_MARKERS.len(),
            REFRESH_METHODS_TEST_MARKERS.len(),
            REGISTRATION_TEST_MARKERS.len(),
            DIAGNOSTIC_ENRICHMENT_TEST_MARKERS.len(),
            SCHEMA_VALIDATION_TEST_MARKERS.len(),
            SEMANTIC_LEGEND_TEST_MARKERS.len(),
            COMPLETION_TEST_MARKERS.len(),
            CODE_LENS_TEST_MARKERS.len(),
            WINDOW_TEST_MARKERS.len(),
            SPEC_MARKERS.len()
        );
        return Ok(());
    }

    eprintln!("LSP 3.18 CLAIM GUARD VIOLATIONS:");
    eprintln!("{}", "=".repeat(72));
    for violation in &violations {
        eprintln!("  {}:{}: {}", violation.rel_path, violation.line, violation.label);
        eprintln!("    {}", violation.detail);
    }
    eprintln!("{}", "=".repeat(72));
    bail!("LSP 3.18 claim guard failed with {} violation(s)", violations.len());
}

fn check_required_markers(
    root: &Path,
    rel_path: &str,
    markers: &[RequiredMarker],
    violations: &mut Vec<Violation>,
) -> Result<()> {
    let text = read_required(root, rel_path)?;
    for marker in markers {
        if !text.contains(marker.marker) {
            violations.push(Violation {
                rel_path: rel_path.to_string(),
                line: 1,
                label: marker.label,
                detail: format!("missing required marker {:?}", marker.marker),
            });
        }
    }
    Ok(())
}

fn check_feature_catalog(root: &Path, violations: &mut Vec<Violation>) -> Result<()> {
    let text = read_required(root, FEATURE_CATALOG)?;
    for marker in FEATURE_CATALOG_MARKERS {
        if !text.contains(marker.marker) {
            violations.push(Violation {
                rel_path: FEATURE_CATALOG.to_string(),
                line: 1,
                label: marker.label,
                detail: format!("missing required LSP 3.18 catalog marker {:?}", marker.marker),
            });
        }
    }
    for pattern in FEATURE_CATALOG_FORBIDDEN_PATTERNS {
        if text.contains(pattern.needle) {
            violations.push(Violation {
                rel_path: FEATURE_CATALOG.to_string(),
                line: line_number_for(&text, pattern.needle),
                label: pattern.label,
                detail: format!(
                    "unsupported 3.18 surface must stay out of the advertised feature catalog: {:?}",
                    pattern.needle
                ),
            });
        }
    }
    Ok(())
}

fn check_capability_snapshots(root: &Path, violations: &mut Vec<Violation>) -> Result<()> {
    for rel_path in CAPABILITY_SNAPSHOTS {
        let text = read_required(root, rel_path)?;
        for pattern in RAW_SNAPSHOT_PATTERNS {
            if text.contains(pattern.needle) {
                violations.push(Violation {
                    rel_path: (*rel_path).to_string(),
                    line: line_number_for(&text, pattern.needle),
                    label: pattern.label,
                    detail: format!("forbidden snapshot text {:?}", pattern.needle),
                });
            }
        }

        let json: Value = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse capability snapshot {rel_path}"))?;
        for check in CAPABILITY_ABSENCE_CHECKS {
            if json.pointer(check.pointer).is_some() {
                violations.push(Violation {
                    rel_path: (*rel_path).to_string(),
                    line: line_number_for_pointer(&text, check.pointer),
                    label: check.label,
                    detail: check.reason.to_string(),
                });
            }
        }
    }
    Ok(())
}

fn check_folding_range_refresh_guard(root: &Path, violations: &mut Vec<Violation>) -> Result<()> {
    let client_requests = read_required(root, CLIENT_REQUESTS)?;
    let lifecycle_capabilities = read_required(root, LIFECYCLE_CAPABILITIES)?;
    let runtime_refresh = read_required(root, RUNTIME_REFRESH)?;

    require_all(
        CLIENT_REQUESTS,
        &client_requests,
        &[
            "request_folding_range_refresh",
            "folding_range_refresh_support",
            "workspace/foldingRange/refresh",
            "self.send_request(",
        ],
        "workspace/foldingRange/refresh send path",
        violations,
    );
    require_all(
        LIFECYCLE_CAPABILITIES,
        &lifecycle_capabilities,
        &["/workspace/foldingRange/refreshSupport", "folding_range_refresh_support"],
        "workspace/foldingRange/refresh capability parser",
        violations,
    );
    require_all(
        RUNTIME_REFRESH,
        &runtime_refresh,
        &[
            "refresh_folding_ranges",
            "folding_range_refresh_support",
            "request_folding_range_refresh",
        ],
        "workspace/foldingRange/refresh debounce path",
        violations,
    );

    Ok(())
}

fn check_relative_pattern_guard(root: &Path, violations: &mut Vec<Violation>) -> Result<()> {
    let state_document = read_required(root, STATE_DOCUMENT)?;
    let lifecycle_capabilities = read_required(root, LIFECYCLE_CAPABILITIES)?;
    let lifecycle_watchers = read_required(root, LIFECYCLE_WATCHERS)?;
    let registration_tests = read_required(root, REGISTRATION_TEST)?;

    require_all(
        STATE_DOCUMENT,
        &state_document,
        &["file_watcher_relative_pattern_support"],
        "RelativePattern file watcher capability storage",
        violations,
    );
    require_all(
        LIFECYCLE_CAPABILITIES,
        &lifecycle_capabilities,
        &[
            "/capabilities/workspace/didChangeWatchedFiles/relativePatternSupport",
            "file_watcher_relative_pattern_support",
        ],
        "RelativePattern file watcher capability parser",
        violations,
    );
    require_all(
        LIFECYCLE_WATCHERS,
        &lifecycle_watchers,
        &[
            "file_watcher_relative_pattern_support",
            "GlobPattern::Relative",
            "RelativePattern",
            "OneOf::Right",
            "string_file_watchers",
        ],
        "RelativePattern file watcher registration gate",
        violations,
    );
    require_all(
        REGISTRATION_TEST,
        &registration_tests,
        &[
            "relative_pattern_clients_receive_relative_file_watchers",
            "relative_pattern_clients_fall_back_to_string_watchers_without_valid_workspace_uri",
            "relativePatternSupport",
            "baseUri",
        ],
        "RelativePattern file watcher wire receipts",
        violations,
    );

    Ok(())
}

fn check_code_lens_resolve_support_guard(
    root: &Path,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    let state_document = read_required(root, STATE_DOCUMENT)?;
    let lifecycle_capabilities = read_required(root, LIFECYCLE_CAPABILITIES)?;
    let runtime_language_misc = read_required(root, RUNTIME_LANGUAGE_MISC)?;

    require_all(
        STATE_DOCUMENT,
        &state_document,
        &["code_lens_resolve_support", "HashSet<String>"],
        "CodeLens resolveSupport.properties capability storage",
        violations,
    );
    require_all(
        LIFECYCLE_CAPABILITIES,
        &lifecycle_capabilities,
        &["/textDocument/codeLens/resolveSupport/properties", "code_lens_resolve_support"],
        "CodeLens resolveSupport.properties capability parser",
        violations,
    );
    require_all(
        RUNTIME_LANGUAGE_MISC,
        &runtime_language_misc,
        &[
            "client_supports_code_lens_command_resolve",
            "properties.contains(\"command\")",
            "prepare_code_lenses_for_client",
        ],
        "CodeLens command lazy-resolution gate",
        violations,
    );

    Ok(())
}

fn check_completion_item_defaults_data_guard(
    root: &Path,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    let state_document = read_required(root, STATE_DOCUMENT)?;
    let lifecycle_capabilities = read_required(root, LIFECYCLE_CAPABILITIES)?;
    let completion = read_required(root, "crates/perl-lsp-rs/src/runtime/language/completion.rs")?;

    require_all(
        STATE_DOCUMENT,
        &state_document,
        &["completion_list_item_defaults_data_support"],
        "CompletionList.itemDefaults.data capability storage",
        violations,
    );
    require_all(
        LIFECYCLE_CAPABILITIES,
        &lifecycle_capabilities,
        &[
            "/capabilities/textDocument/completion/completionList/itemDefaults",
            "completion_list_item_defaults_data_support",
            "Some(\"data\")",
        ],
        "CompletionList.itemDefaults.data capability parser",
        violations,
    );
    require_all(
        "crates/perl-lsp-rs/src/runtime/language/completion.rs",
        &completion,
        &[
            "completion_list_default_data",
            "completion_list_response",
            "\"itemDefaults\"",
            "\"data\"",
        ],
        "CompletionList.itemDefaults.data response gate",
        violations,
    );

    Ok(())
}

fn check_completion_apply_kind_guard(root: &Path, violations: &mut Vec<Violation>) -> Result<()> {
    let state_document = read_required(root, STATE_DOCUMENT)?;
    let lifecycle_capabilities = read_required(root, LIFECYCLE_CAPABILITIES)?;
    let completion = read_required(root, "crates/perl-lsp-rs/src/runtime/language/completion.rs")?;

    require_all(
        STATE_DOCUMENT,
        &state_document,
        &["completion_list_apply_kind_support"],
        "CompletionList.applyKind capability storage",
        violations,
    );
    require_all(
        LIFECYCLE_CAPABILITIES,
        &lifecycle_capabilities,
        &[
            "/capabilities/textDocument/completion/completionList/applyKindSupport",
            "completion_list_apply_kind_support",
        ],
        "CompletionList.applyKind capability parser",
        violations,
    );
    require_all(
        "crates/perl-lsp-rs/src/runtime/language/completion.rs",
        &completion,
        &["completion_list_response", "\"applyKind\"", "\"data\": 2"],
        "CompletionList.applyKind response gate",
        violations,
    );

    Ok(())
}

fn check_code_action_documentation_guard(
    root: &Path,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    let state_document = read_required(root, STATE_DOCUMENT)?;
    let lifecycle_capabilities = read_required(root, LIFECYCLE_CAPABILITIES)?;
    let negative_claims = read_required(root, NEGATIVE_CLAIMS_TEST)?;

    require_all(
        STATE_DOCUMENT,
        &state_document,
        &["code_action_documentation_support"],
        "CodeAction.documentation capability storage",
        violations,
    );
    require_all(
        LIFECYCLE_CAPABILITIES,
        &lifecycle_capabilities,
        &[
            "/capabilities/textDocument/codeAction/documentationSupport",
            "code_action_documentation_support",
            "code_action_documentation_entries",
            "\"documentation\"",
        ],
        "CodeAction.documentation capability gate",
        violations,
    );
    require_all(
        NEGATIVE_CLAIMS_TEST,
        &negative_claims,
        &[
            "/codeActionProvider/documentation",
            "code_action_documentation_advertised_when_supported",
            "perl.explainProviderDecision",
        ],
        "CodeAction.documentation wire receipts",
        violations,
    );

    Ok(())
}

fn check_code_action_tag_guard(root: &Path, violations: &mut Vec<Violation>) -> Result<()> {
    let state_document = read_required(root, STATE_DOCUMENT)?;
    let lifecycle_capabilities = read_required(root, LIFECYCLE_CAPABILITIES)?;
    let code_actions =
        read_required(root, "crates/perl-lsp-rs/src/runtime/language/code_actions.rs")?;
    let negative_claims = read_required(root, NEGATIVE_CLAIMS_TEST)?;

    require_all(
        STATE_DOCUMENT,
        &state_document,
        &["code_action_llm_generated_tag_support"],
        "CodeAction.tags capability storage",
        violations,
    );
    require_all(
        LIFECYCLE_CAPABILITIES,
        &lifecycle_capabilities,
        &[
            "/capabilities/textDocument/codeAction/tagSupport/valueSet",
            "code_action_llm_generated_tag_support",
            "initialize_parses_code_action_llm_generated_tag_support",
        ],
        "CodeAction.tags capability parser",
        violations,
    );
    require_all(
        "crates/perl-lsp-rs/src/runtime/language/code_actions.rs",
        &code_actions,
        &[
            "CODE_ACTION_TAG_LLM_GENERATED",
            "enforce_code_action_tag_capability",
            "code_action_llm_generated_tag_support",
        ],
        "CodeAction.tags response gate",
        violations,
    );
    require_all(
        NEGATIVE_CLAIMS_TEST,
        &negative_claims,
        &[
            "tagSupport",
            "valueSet",
            "code_actions_do_not_emit_llm_generated_tags_for_deterministic_actions",
            "code_action_resolve_does_not_echo_tags_without_client_support",
        ],
        "CodeAction.tags wire receipts",
        violations,
    );

    Ok(())
}

fn check_snippet_text_edit_guard(root: &Path, violations: &mut Vec<Violation>) -> Result<()> {
    let state_document = read_required(root, STATE_DOCUMENT)?;
    let lifecycle_capabilities = read_required(root, LIFECYCLE_CAPABILITIES)?;
    let code_actions =
        read_required(root, "crates/perl-lsp-rs/src/runtime/language/code_actions.rs")?;
    let negative_claims = read_required(root, NEGATIVE_CLAIMS_TEST)?;

    require_all(
        STATE_DOCUMENT,
        &state_document,
        &["workspace_edit_document_changes_support", "workspace_edit_snippet_edit_support"],
        "SnippetTextEdit workspace-edit capability storage",
        violations,
    );
    require_all(
        LIFECYCLE_CAPABILITIES,
        &lifecycle_capabilities,
        &[
            "/capabilities/workspace/workspaceEdit/documentChanges",
            "/capabilities/workspace/workspaceEdit/snippetEditSupport",
            "workspace_edit_document_changes_support",
            "workspace_edit_snippet_edit_support",
        ],
        "SnippetTextEdit workspace-edit capability parser",
        violations,
    );
    require_all(
        "crates/perl-lsp-rs/src/runtime/language/code_actions.rs",
        &code_actions,
        &[
            "supports_workspace_snippet_text_edits",
            "convert_pragma_quickfix_edits_to_snippet_text_edits",
            "\"documentChanges\"",
            "\"snippet\"",
            "\"kind\": \"snippet\"",
        ],
        "SnippetTextEdit response gate",
        violations,
    );
    require_all(
        NEGATIVE_CLAIMS_TEST,
        &negative_claims,
        &[
            "code_action_pragmas_emit_snippet_text_edits_when_supported",
            "code_action_pragmas_require_document_changes_for_snippet_text_edits",
            "snippetEditSupport",
        ],
        "SnippetTextEdit wire receipts",
        violations,
    );

    Ok(())
}

fn check_apply_edit_metadata_guard(root: &Path, violations: &mut Vec<Violation>) -> Result<()> {
    let state_document = read_required(root, STATE_DOCUMENT)?;
    let lifecycle_capabilities = read_required(root, LIFECYCLE_CAPABILITIES)?;
    let client_requests = read_required(root, CLIENT_REQUESTS)?;
    let refactor_receipts = read_required(root, REFACTOR_RUNTIME_RECEIPTS)?;
    let negative_claims = read_required(root, NEGATIVE_CLAIMS_TEST)?;

    require_all(
        STATE_DOCUMENT,
        &state_document,
        &["workspace_apply_edit_support", "workspace_edit_metadata_support"],
        "ApplyWorkspaceEditParams.metadata capability storage",
        violations,
    );
    require_all(
        LIFECYCLE_CAPABILITIES,
        &lifecycle_capabilities,
        &[
            "/capabilities/workspace/applyEdit",
            "/capabilities/workspace/workspaceEdit/metadataSupport",
            "workspace_apply_edit_support",
            "workspace_edit_metadata_support",
            "initialize_parses_apply_edit_metadata_support",
        ],
        "ApplyWorkspaceEditParams.metadata capability parser",
        violations,
    );
    require_all(
        CLIENT_REQUESTS,
        &client_requests,
        &[
            "request_apply_workspace_edit_with_metadata",
            "request_apply_workspace_edit_with_metadata_call_presence_observer",
            "request_apply_workspace_edit_with_metadata_boundary_discriminator",
            "request_apply_workspace_edit_with_metadata_return_value_discriminator",
            "WORKSPACE_APPLY_EDIT",
            "\"metadata\"",
            "\"isRefactoring\"",
        ],
        "ApplyWorkspaceEditParams.metadata request helper",
        violations,
    );
    require_all(
        REFACTOR_RUNTIME_RECEIPTS,
        &refactor_receipts,
        &[
            "request_apply_workspace_edit_with_metadata",
            "apply_edit_requested",
            "apply_edit_request",
        ],
        "ApplyWorkspaceEditParams.metadata safe-delete apply path",
        violations,
    );
    require_all(
        NEGATIVE_CLAIMS_TEST,
        &negative_claims,
        &[
            "apply_workspace_edit_metadata_emitted_when_supported_for_refactor_request",
            "apply_workspace_edit_metadata_absent_without_metadata_support",
            "metadataSupport",
            "workspace/applyEdit",
            "/edit/metadata",
        ],
        "ApplyWorkspaceEditParams.metadata wire receipts",
        violations,
    );

    Ok(())
}

fn check_message_type_debug_support(root: &Path, violations: &mut Vec<Violation>) -> Result<()> {
    let window = read_required(root, "crates/perl-lsp-rs/src/runtime/window.rs")?;
    require_all(
        "crates/perl-lsp-rs/src/runtime/window.rs",
        &window,
        &["Debug = 5"],
        "MessageType.Debug enum support",
        violations,
    );
    Ok(())
}

fn require_all(
    rel_path: &str,
    text: &str,
    needles: &[&'static str],
    label: &'static str,
    violations: &mut Vec<Violation>,
) {
    for needle in needles {
        if !text.contains(needle) {
            violations.push(Violation {
                rel_path: rel_path.to_string(),
                line: 1,
                label,
                detail: format!("missing required guard marker {:?}", needle),
            });
        }
    }
}

fn read_required(root: &Path, rel_path: &str) -> Result<String> {
    fs::read_to_string(root.join(rel_path))
        .with_context(|| format!("failed to read required guard input {rel_path}"))
}

fn line_number_for(text: &str, needle: &str) -> usize {
    text.lines().position(|line| line.contains(needle)).map_or(1, |idx| idx + 1)
}

fn line_number_for_pointer(text: &str, pointer: &str) -> usize {
    pointer
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .map_or(1, |key| line_number_for(text, key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_number_for_reports_first_matching_line() {
        let text = "one\ntwo marker\nthree marker\n";
        assert_eq!(line_number_for(text, "marker"), 2);
    }

    #[test]
    fn line_number_for_missing_marker_falls_back_to_one() {
        assert_eq!(line_number_for("one\ntwo\n", "missing"), 1);
    }

    #[test]
    fn pointer_line_uses_last_path_segment() {
        let text = "{\n  \"semanticTokensProvider\": {\n    \"delta\": true\n  }\n}\n";
        assert_eq!(line_number_for_pointer(text, "/semanticTokensProvider/full/delta"), 3);
    }
}
