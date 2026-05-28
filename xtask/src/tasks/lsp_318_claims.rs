//! Validate the selected LSP 3.18 claim boundary.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use serde_json::Value;
use std::fs;
use std::path::Path;

const SPEC_PATH: &str = "docs/specs/PLSP-SPEC-0029-lsp-318-conformance-boundary.md";
const MATRIX_PATH: &str = "docs/specs/lsp-318-conformance-matrix.md";
const NEGATIVE_CLAIMS_TEST: &str = "crates/perl-lsp-rs/tests/lsp_318_negative_claims.rs";
const DIAGNOSTIC_ENRICHMENT_TEST: &str =
    "crates/perl-lsp-rs/tests/lsp_diagnostic_enrichment_test.rs";
const REFRESH_METHODS_TEST: &str = "crates/perl-lsp-rs/tests/lsp_refresh_methods_tests.rs";
const SCHEMA_VALIDATION_TEST: &str = "crates/perl-lsp-rs/tests/lsp_schema_validation.rs";
const CLIENT_REQUESTS: &str = "crates/perl-lsp-rs/src/runtime/client_requests.rs";
const LIFECYCLE_CAPABILITIES: &str = "crates/perl-lsp-rs/src/runtime/lifecycle/capabilities.rs";
const RUNTIME_REFRESH: &str = "crates/perl-lsp-rs/src/runtime/refresh.rs";
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
        label: "matrix negative-gated vocabulary",
        marker: "`negative-gated+documented`",
    },
    RequiredMarker { label: "matrix notebook classification", marker: "Notebook 3.18 additions" },
];

const NEGATIVE_TEST_MARKERS: &[RequiredMarker] = &[
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
    RequiredMarker { label: "WorkspaceEdit metadata gate", marker: "metadata" },
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
        label: "MessageType.Debug gate",
        marker: "window_message_type_does_not_emit_debug_level",
    },
    RequiredMarker { label: "Command.tooltip gate", marker: "assert_no_command_tooltip" },
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
];

const REFRESH_METHODS_TEST_MARKERS: &[RequiredMarker] = &[RequiredMarker {
    label: "workspace/foldingRange/refresh positive receipt",
    marker: "lsp_refresh_folding_range_sent_with_client_support",
}];

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
        reason: "applyKind must stay absent until applyKindSupport is parsed and tested",
    },
    JsonAbsenceCheck {
        pointer: "/completionProvider/itemDefaults/data",
        label: "CompletionList.itemDefaults.data",
        reason: "default data must stay absent until explicitly supported and tested",
    },
    JsonAbsenceCheck {
        pointer: "/codeActionProvider/documentation",
        label: "CodeAction.documentation",
        reason: "code action documentation must be client-capability gated before advertisement",
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
    RawPatternCheck { needle: "SnippetTextEdit", label: "SnippetTextEdit feature claim" },
    RawPatternCheck {
        needle: "CompletionList.applyKind",
        label: "CompletionList.applyKind feature claim",
    },
    RawPatternCheck {
        needle: "CompletionList.itemDefaults.data",
        label: "CompletionList.itemDefaults.data feature claim",
    },
    RawPatternCheck {
        needle: "CodeAction.documentation",
        label: "CodeAction.documentation feature claim",
    },
    RawPatternCheck { needle: "CodeAction.tags", label: "CodeAction.tags feature claim" },
    RawPatternCheck { needle: "MessageType.Debug", label: "MessageType.Debug feature claim" },
    RawPatternCheck { needle: "Command.tooltip", label: "Command.tooltip feature claim" },
    RawPatternCheck { needle: "RelativePattern", label: "RelativePattern feature claim" },
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
    check_feature_catalog(&root, &mut violations)?;
    check_capability_snapshots(&root, &mut violations)?;
    check_folding_range_refresh_guard(&root, &mut violations)?;
    check_forbidden_source_claims(&root, &mut violations)?;

    if violations.is_empty() {
        println!(
            "LSP 3.18 claim guard OK: {} capability snapshots, {} feature markers, {} negative-test markers, {} positive refresh markers, {} diagnostic markers, {} schema markers, {} spec markers checked",
            CAPABILITY_SNAPSHOTS.len(),
            FEATURE_CATALOG_MARKERS.len(),
            NEGATIVE_TEST_MARKERS.len(),
            REFRESH_METHODS_TEST_MARKERS.len(),
            DIAGNOSTIC_ENRICHMENT_TEST_MARKERS.len(),
            SCHEMA_VALIDATION_TEST_MARKERS.len(),
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

fn check_forbidden_source_claims(root: &Path, violations: &mut Vec<Violation>) -> Result<()> {
    let window = read_required(root, "crates/perl-lsp-rs/src/runtime/window.rs")?;
    for (idx, line) in window.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.contains("MessageType::Debug")
            || trimmed == "Debug,"
            || trimmed.starts_with("Debug =")
        {
            violations.push(Violation {
                rel_path: "crates/perl-lsp-rs/src/runtime/window.rs".to_string(),
                line: idx + 1,
                label: "MessageType.Debug",
                detail: "MessageType.Debug is not part of the current selected 3.18 claim"
                    .to_string(),
            });
        }
    }
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
