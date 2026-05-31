//! Generate the selected LSP 3.18 conformance matrix.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use std::fs;

pub const MATRIX_PATH: &str = "docs/specs/lsp-318-conformance-matrix.md";

#[derive(Clone, Copy)]
struct MatrixRow {
    feature: &'static str,
    since_or_flag: &'static str,
    client_gate: &'static str,
    server_shape: &'static str,
    method_or_shape: &'static str,
    status: &'static str,
    proof: &'static str,
    owner: &'static str,
    priority: &'static str,
    notes: &'static str,
}

const ROWS: &[MatrixRow] = &[
    MatrixRow {
        feature: "Standard inline completion",
        since_or_flag: "LSP 3.18",
        client_gate: "`textDocument.inlineCompletion`; dynamic registration via `dynamicRegistration`",
        server_shape: "`inlineCompletionProvider` for static clients; `client/registerCapability` for dynamic clients",
        method_or_shape: "`textDocument/inlineCompletion`",
        status: "implemented+tested+documented",
        proof: "`lsp_inline_completion_registration_tests`; `lsp_ai_inline_completion_tests`; `lsp_streaming_completion_tests`; `lsp_cap_snap`",
        owner: "`crates/perl-lsp-rs/src/runtime/language/misc.rs`; `crates/perl-lsp-rs-core/src/providers/inline_completion/`",
        priority: "P0",
        notes: "Static and dynamic modes are mutually exclusive; `experimental.inlineCompletionProvider` is forbidden.",
    },
    MatrixRow {
        feature: "`selectedCompletionInfo` inline context",
        since_or_flag: "LSP 3.18",
        client_gate: "`textDocument.inlineCompletion` request context",
        server_shape: "Inline completion result items",
        method_or_shape: "`InlineCompletionContext.selectedCompletionInfo`",
        status: "implemented+tested+documented",
        proof: "`lsp_inline_completion_registration_tests`; `lsp_inline_completion_tests`",
        owner: "`crates/perl-lsp-rs/src/runtime/language/misc.rs`",
        priority: "P0",
        notes: "Returned items must use the same range and extend selected text, or return empty.",
    },
    MatrixRow {
        feature: "Object-form `StringValue` inline insert text",
        since_or_flag: "LSP 3.18",
        client_gate: "`textDocument.inlineCompletion`",
        server_shape: "`InlineCompletionItem.insertText` object form",
        method_or_shape: "`InlineCompletionItem.insertText: StringValue`",
        status: "negative-gated+documented",
        proof: "`lsp_inline_completion_registration_tests`",
        owner: "`crates/perl-lsp-rs/src/runtime/language/misc.rs`; inline-completion provider",
        priority: "P1",
        notes: "Standard inline completion currently emits plain string `insertText`; object-form `StringValue` remains unclaimed until a provider returns it with wire proof.",
    },
    MatrixRow {
        feature: "Multi-range formatting",
        since_or_flag: "LSP 3.18",
        client_gate: "range-formatting client support",
        server_shape: "`documentRangeFormattingProvider.rangesSupport`",
        method_or_shape: "`textDocument/rangesFormatting`",
        status: "implemented+tested+documented",
        proof: "`lsp_caps_contract_shapes`; `lsp_disabled_features_tests`; `lsp_formatting_e2e`; `lsp_capabilities_snapshot`; `lsp_cap_snap`",
        owner: "`crates/perl-lsp-rs/src/runtime/language/formatting.rs`; `crates/perl-lsp-rs-core/src/protocol/capabilities.rs`",
        priority: "P0",
        notes: "`documentRangesFormattingProvider` is not a valid capability and remains forbidden.",
    },
    MatrixRow {
        feature: "`workspace/textDocumentContent`",
        since_or_flag: "LSP 3.18 proposed",
        client_gate: "client can request custom scheme content",
        server_shape: "`workspace.textDocumentContent.schemes = [\"perldoc\"]`",
        method_or_shape: "`workspace/textDocumentContent` returns `{ text }`",
        status: "implemented+tested+documented",
        proof: "`lsp_text_document_content_tests`; `lsp_virtual_content_tests`; `lsp_cap_snap`",
        owner: "`crates/perl-lsp-rs/src/runtime/language/virtual_content.rs`",
        priority: "P0",
        notes: "`perldoc` is the current advertised virtual-document scheme; workspace POD output includes sorted related `perldoc://` links for simple `L<Module::Name>` references.",
    },
    MatrixRow {
        feature: "`workspace/textDocumentContent/refresh`",
        since_or_flag: "LSP 3.18 proposed",
        client_gate: "virtual content refresh support",
        server_shape: "server-originated request through bounded request IDs",
        method_or_shape: "`workspace/textDocumentContent/refresh`",
        status: "implemented+tested+documented",
        proof: "`lsp_text_document_content_tests`; `check-lsp-318-claims`",
        owner: "`crates/perl-lsp-rs/src/runtime/language/virtual_content.rs`; server request path",
        priority: "P1",
        notes: "Refresh uses the shared server request path rather than ad hoc IDs.",
    },
    MatrixRow {
        feature: "`workspace/foldingRange/refresh`",
        since_or_flag: "LSP 3.18 proposed",
        client_gate: "`workspace.foldingRange.refreshSupport`",
        server_shape: "no server capability; client-gated server request",
        method_or_shape: "`workspace/foldingRange/refresh`",
        status: "implemented+tested+documented",
        proof: "`lsp_refresh_methods_tests`; `lsp_318_negative_claims`; `check-lsp-318-claims`",
        owner: "`crates/perl-lsp-rs/src/runtime/client_requests.rs`; `crates/perl-lsp-rs/src/runtime/refresh.rs`",
        priority: "P1",
        notes: "Unsupported clients are guarded; supported clients receive a bounded server request through the shared request path.",
    },
    MatrixRow {
        feature: "Semantic tokens full/range",
        since_or_flag: "pre-3.18 surface with 3.18 honesty requirement",
        client_gate: "`textDocument.semanticTokens`",
        server_shape: "`semanticTokensProvider.full = true`; `range = true`",
        method_or_shape: "`textDocument/semanticTokens/full`; `textDocument/semanticTokens/range`",
        status: "implemented+tested+documented",
        proof: "`lsp_caps_contract_shapes`; `lsp_semantic_legend_contract_tests`; `lsp_cap_snap`",
        owner: "`crates/perl-lsp-rs-core/src/providers/semantic_tokens/`; capability builder",
        priority: "P0",
        notes: "Delta is intentionally absent until result-id state exists.",
    },
    MatrixRow {
        feature: "Semantic-token delta",
        since_or_flag: "LSP semantic tokens",
        client_gate: "`textDocument.semanticTokens.requests.full.delta`",
        server_shape: "`semanticTokensProvider.full.delta`",
        method_or_shape: "`textDocument/semanticTokens/full/delta`",
        status: "negative-gated+documented",
        proof: "`lsp_318_negative_claims`; `lsp_caps_contract_shapes`; `check-lsp-318-claims`",
        owner: "semantic token provider",
        priority: "P1",
        notes: "Do not advertise or route delta without result-id state and delta responses.",
    },
    MatrixRow {
        feature: "`SemanticTokenTypes.label` and open-set legend audit",
        since_or_flag: "LSP 3.18",
        client_gate: "semantic-token legend negotiation",
        server_shape: "`semanticTokensProvider.legend.tokenTypes`",
        method_or_shape: "token type indexes must stay within legend bounds",
        status: "implemented+tested+documented",
        proof: "`semantic_token_label_type_decodes_for_perl_labels`; `semantic_token_result_indexes_stay_within_advertised_legend_bounds`; `check-lsp-318-claims`",
        owner: "`crates/perl-lsp-rs-core/src/providers/semantic_tokens/`",
        priority: "P3",
        notes: "`label` is advertised and emitted for deterministic Perl label declarations and loop-control label references; emitted token indexes and modifier bits remain bounds-checked.",
    },
    MatrixRow {
        feature: "`Diagnostic.message` as `MarkupContent`",
        since_or_flag: "LSP 3.18",
        client_gate: "`textDocument.diagnostic.markupMessageSupport`",
        server_shape: "`Diagnostic.message` union shape",
        method_or_shape: "`textDocument/diagnostic`; `workspace/diagnostic` pull responses",
        status: "implemented+tested+documented",
        proof: "`lsp_diagnostic_enrichment_test`; `lsp_318_negative_claims`; `lsp_schema_validation`; `check-lsp-318-claims`",
        owner: "`crates/perl-lsp-rs/src/runtime/diagnostics.rs`",
        priority: "P1",
        notes: "Pull diagnostics emit MarkupContent only when `markupMessageSupport` is true; unsupported clients and publish diagnostics stay string-only.",
    },
    MatrixRow {
        feature: "`SignatureHelp.activeParameter = null`",
        since_or_flag: "LSP 3.18",
        client_gate: "signature-help client support",
        server_shape: "`SignatureHelp.activeParameter`; `SignatureInformation.activeParameter`",
        method_or_shape: "`textDocument/signatureHelp` response",
        status: "implemented+tested+documented",
        proof: "`lsp_schema_validation`; `lsp_signature_help_tests`; `check-lsp-318-claims`",
        owner: "`crates/perl-lsp-rs/src/runtime/language/hover/signature_help.rs`",
        priority: "P2",
        notes: "Schema validation accepts unsigned integer or null; runtime receipts preserve current numeric active-parameter tracking.",
    },
    MatrixRow {
        feature: "`ApplyWorkspaceEditParams.metadata`",
        since_or_flag: "LSP 3.18",
        client_gate: "`workspace.applyEdit` and `workspace.workspaceEdit.metadataSupport`",
        server_shape: "`ApplyWorkspaceEditParams.metadata` with `WorkspaceEditMetadata.isRefactoring`",
        method_or_shape: "`workspace/applyEdit` server request params",
        status: "implemented+tested+documented",
        proof: "`lsp_318_negative_claims`; `features.toml`; `check-lsp-318-claims`",
        owner: "`crates/perl-lsp-rs/src/runtime/client_requests.rs`",
        priority: "P2",
        notes: "Server-originated refactoring apply-edit requests may include `metadata.isRefactoring` only when `workspace.applyEdit` and `metadataSupport` are both true; ordinary `WorkspaceEdit` responses stay metadata-free.",
    },
    MatrixRow {
        feature: "`SnippetTextEdit` workspace edits",
        since_or_flag: "LSP 3.18",
        client_gate: "`workspace.workspaceEdit.documentChanges` and `workspace.workspaceEdit.snippetEditSupport`",
        server_shape: "`SnippetTextEdit` in `WorkspaceEdit.documentChanges`",
        method_or_shape: "`textDocument/codeAction` pragma quick fixes",
        status: "implemented+tested+documented",
        proof: "`lsp_318_negative_claims`; `features.toml`; `check-lsp-318-claims`",
        owner: "`crates/perl-lsp-rs/src/runtime/language/code_actions.rs`",
        priority: "P2",
        notes: "Supported clients receive SnippetTextEdit for current-document pragma quick fixes; unsupported clients and aggregate fix-all actions retain plain TextEdit fallback.",
    },
    MatrixRow {
        feature: "`CompletionList.itemDefaults.data`",
        since_or_flag: "LSP 3.18",
        client_gate: "`textDocument.completion.completionList.itemDefaults` contains `data`",
        server_shape: "`CompletionList.itemDefaults.data`",
        method_or_shape: "`textDocument/completion`; `completionItem/resolve`",
        status: "implemented+tested+documented",
        proof: "`lsp_completion_tests`; `lsp_318_negative_claims`; `check-lsp-318-claims`",
        owner: "`crates/perl-lsp-rs/src/runtime/language/completion.rs`",
        priority: "P2",
        notes: "Supported clients receive shared completion-list data; unsupported clients retain the current response shape.",
    },
    MatrixRow {
        feature: "`CompletionList.applyKind`",
        since_or_flag: "LSP 3.18",
        client_gate: "`textDocument.completion.completionList.applyKindSupport`",
        server_shape: "`CompletionList.applyKind`",
        method_or_shape: "`textDocument/completion`",
        status: "implemented+tested+documented",
        proof: "`lsp_completion_tests`; `lsp_318_negative_claims`; `check-lsp-318-claims`",
        owner: "`crates/perl-lsp-rs/src/runtime/language/completion.rs`",
        priority: "P2",
        notes: "Supported clients receive `applyKind.data = 2` with supported default data; unsupported clients and responses without item defaults retain the current shape.",
    },
    MatrixRow {
        feature: "`CodeAction.documentation`",
        since_or_flag: "LSP 3.18",
        client_gate: "`textDocument.codeAction.documentationSupport`",
        server_shape: "`CodeActionOptions.documentation`",
        method_or_shape: "initialize capability",
        status: "implemented+tested+documented",
        proof: "`lsp_318_negative_claims`; `check-lsp-318-claims`",
        owner: "`crates/perl-lsp-rs/src/runtime/lifecycle/capabilities.rs`",
        priority: "P2",
        notes: "Supported clients receive documentation entries for quickfix, refactor, and source.fixAll. Unsupported clients receive no documentation advertisement.",
    },
    MatrixRow {
        feature: "`CodeAction.tags` and `CodeActionTag.LLMGenerated`",
        since_or_flag: "LSP 3.18",
        client_gate: "`textDocument.codeAction.tagSupport`",
        server_shape: "`CodeAction.tags`",
        method_or_shape: "`textDocument/codeAction` responses",
        status: "negative-gated+documented",
        proof: "`lsp_318_negative_claims`; `check-lsp-318-claims`",
        owner: "`crates/perl-lsp-rs/src/runtime/lifecycle/capabilities.rs`; `crates/perl-lsp-rs/src/runtime/language/code_actions.rs`",
        priority: "P2",
        notes: "`tagSupport.valueSet` is parsed and code-action/resolve response tags are stripped unless supported; deterministic actions remain untagged. Generated-action tagging remains unclaimed until a generated-action source exists.",
    },
    MatrixRow {
        feature: "`MessageType.Debug`",
        since_or_flag: "LSP 3.18",
        client_gate: "no client gate; explicit debug message calls only",
        server_shape: "`window/logMessage`, `window/showMessage`, and `window/showMessageRequest` type `5`",
        method_or_shape: "window message notifications and requests",
        status: "implemented+tested+documented",
        proof: "`lsp_window_tests`; `lsp_318_negative_claims`; `check-lsp-318-claims`",
        owner: "`crates/perl-lsp-rs/src/runtime/window.rs`",
        priority: "P3",
        notes: "Explicit debug calls serialize type 5; normal runtime paths remain on the existing non-debug message levels.",
    },
    MatrixRow {
        feature: "`Command.tooltip`",
        since_or_flag: "LSP 3.18",
        client_gate: "no client gate; currently scoped to CodeLens command objects",
        server_shape: "`Command.tooltip` on CodeLens commands",
        method_or_shape: "`textDocument/codeLens`; `codeLens/resolve`",
        status: "implemented+tested+documented",
        proof: "`lsp_codelens_tests`; `lsp_318_negative_claims`; `check-lsp-318-claims`",
        owner: "`crates/perl-lsp-rs-core/src/providers/code_lens/`; CodeLens handlers",
        priority: "P3",
        notes: "CodeLens command tooltips are plain text; non-CodeLens command tooltips remain unclaimed and negative-gated.",
    },
    MatrixRow {
        feature: "`RelativePattern` watcher registrations",
        since_or_flag: "LSP 3.18",
        client_gate: "`workspace.didChangeWatchedFiles.relativePatternSupport`",
        server_shape: "`RelativePattern` watcher glob objects with `baseUri`",
        method_or_shape: "`client/registerCapability` for `workspace/didChangeWatchedFiles`",
        status: "implemented+tested+documented",
        proof: "`lsp_318_negative_claims`; `lsp_registration_tests`; `check-lsp-318-claims`",
        owner: "`crates/perl-lsp-rs/src/runtime/lifecycle/watchers.rs`; capability parser",
        priority: "P3",
        notes: "Capable clients receive `baseUri`/`pattern` watcher globs rooted at workspace folders; unsupported clients or invalid roots keep string glob fallback. Document-selector `RelativePattern` support remains unclaimed.",
    },
    MatrixRow {
        feature: "`CodeLens.resolveSupport.properties`",
        since_or_flag: "LSP 3.18",
        client_gate: "`textDocument.codeLens.resolveSupport.properties`",
        server_shape: "`codeLensProvider.resolveProvider` plus client property parsing",
        method_or_shape: "`codeLens/resolve`",
        status: "implemented+tested+documented",
        proof: "`lsp_codelens_tests`; `lsp_code_lens_tests`; `lsp_bdd_workflows`; `check-lsp-318-claims`",
        owner: "`crates/perl-lsp-rs/src/runtime/lifecycle/capabilities.rs`; `crates/perl-lsp-rs/src/runtime/language/misc.rs`; code-lens provider",
        priority: "P2",
        notes: "Unresolved command lenses are returned only when `command` appears in the client resolve-support properties; other clients receive eager command lenses.",
    },
    MatrixRow {
        feature: "Markdown command links and theme-icon syntax guard",
        since_or_flag: "not in current LSP 3.18 spec",
        client_gate: "n/a; `supportThemeIcons` and trusted markdown `enabledCommands` are not LSP 3.18 capabilities",
        server_shape: "markdown returned by hover/completion/signature/docs providers",
        method_or_shape: "markup content strings",
        status: "not-applicable+documented",
        proof: "`lsp_318_negative_claims`; `check-lsp-318-claims`",
        owner: "hover, completion, signature, and docs providers",
        priority: "P4",
        notes: "Representative markdown/string outputs are absence-tested for `command:` links and `$()` theme-icon syntax; do not implement these as LSP 3.18 behavior without a separate editor-specific proposal.",
    },
    MatrixRow {
        feature: "Notebook 3.18 additions",
        since_or_flag: "LSP 3.18",
        client_gate: "notebook document client capabilities",
        server_shape: "notebook diagnostic pull and notebook code-action-kind additions",
        method_or_shape: "notebook-specific methods and selectors",
        status: "not-applicable+documented",
        proof: "PLSP-SPEC-0029 explicit non-claim; feature catalog does not claim these additions",
        owner: "n/a",
        priority: "P4",
        notes: "Keep classified as not applicable unless a concrete Perl editor notebook need appears.",
    },
];

#[cfg(test)]
const CLOSED_STATUSES: &[&str] =
    &["implemented+tested+documented", "negative-gated+documented", "not-applicable+documented"];

pub fn run(check: bool) -> Result<()> {
    let root = project_root()?;
    let path = root.join(MATRIX_PATH);
    let generated = render_matrix();

    if check {
        let existing =
            fs::read_to_string(&path).with_context(|| format!("failed to read {}", MATRIX_PATH))?;
        if normalize_newlines(&existing) != generated {
            bail!("{} is stale; run `cargo xtask generate-lsp-318-matrix`", MATRIX_PATH);
        }
        println!("LSP 3.18 conformance matrix is up to date: {} rows", ROWS.len());
        return Ok(());
    }

    fs::write(&path, generated).with_context(|| format!("failed to write {}", MATRIX_PATH))?;
    println!("Wrote {} with {} rows", MATRIX_PATH, ROWS.len());
    Ok(())
}

fn render_matrix() -> String {
    let mut output = String::new();
    output.push_str("# LSP 3.18 Conformance Matrix\n\n");
    output.push_str("Status: generated\n");
    output.push_str("Owner: perl-lsp maintainers\n");
    output.push_str("Generator: `cargo xtask generate-lsp-318-matrix`\n");
    output.push_str("Check: `cargo xtask generate-lsp-318-matrix --check`\n");
    output.push_str(
        "Boundary spec: [PLSP-SPEC-0029](PLSP-SPEC-0029-lsp-318-conformance-boundary.md)\n\n",
    );
    output.push_str("This matrix is the working ledger for selected LSP 3.18 coverage. It is not a blanket full-conformance claim and does not imply release readiness. Each row classifies a surface as implemented, intentionally absent, or outside the current Perl editor substrate lane. The current closeout state has no unknown or transitional rows.\n\n");
    output.push_str("Status vocabulary:\n\n");
    output.push_str("- `implemented+tested+documented`: implemented, tested over the wire or snapshots, and documented in the current boundary.\n");
    output.push_str("- `negative-gated+documented`: intentionally unsupported or absent until a later capability-gated implementation PR.\n");
    output.push_str(
        "- `not-applicable+documented`: explicitly outside the current Perl editor substrate lane.\n\n",
    );
    output.push_str("| Feature | Since / proposed flag | Client capability gate | Server capability / advertised shape | Method / response shape | Status | Current proof | Owner module | Priority | Notes |\n");
    output.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");

    for row in ROWS {
        output.push_str("| ");
        output.push_str(&escape_cell(row.feature));
        output.push_str(" | ");
        output.push_str(&escape_cell(row.since_or_flag));
        output.push_str(" | ");
        output.push_str(&escape_cell(row.client_gate));
        output.push_str(" | ");
        output.push_str(&escape_cell(row.server_shape));
        output.push_str(" | ");
        output.push_str(&escape_cell(row.method_or_shape));
        output.push_str(" | ");
        output.push_str(&escape_cell(row.status));
        output.push_str(" | ");
        output.push_str(&escape_cell(row.proof));
        output.push_str(" | ");
        output.push_str(&escape_cell(row.owner));
        output.push_str(" | ");
        output.push_str(&escape_cell(row.priority));
        output.push_str(" | ");
        output.push_str(&escape_cell(row.notes));
        output.push_str(" |\n");
    }

    output
}

fn escape_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', "<br>")
}

fn normalize_newlines(value: &str) -> String {
    value.replace("\r\n", "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_contains_required_seed_surfaces() {
        let rendered = render_matrix();
        for surface in [
            "Standard inline completion",
            "`selectedCompletionInfo` inline context",
            "Multi-range formatting",
            "`workspace/textDocumentContent`",
            "`workspace/textDocumentContent/refresh`",
            "`workspace/foldingRange/refresh`",
            "`ApplyWorkspaceEditParams.metadata`",
            "`SnippetTextEdit` workspace edits",
            "`CompletionList.itemDefaults.data`",
            "`CompletionList.applyKind`",
            "`CodeAction.documentation`",
            "`CodeAction.tags` and `CodeActionTag.LLMGenerated`",
            "`MessageType.Debug`",
            "`Command.tooltip`",
            "`RelativePattern` watcher registrations",
            "`CodeLens.resolveSupport.properties`",
            "Markdown command links and theme-icon syntax guard",
            "Notebook 3.18 additions",
            "`SemanticTokenTypes.label` and open-set legend audit",
        ] {
            assert!(rendered.contains(surface), "matrix missing required surface {surface}");
        }
    }

    #[test]
    fn matrix_has_one_table_row_per_declared_surface() {
        let rendered = render_matrix();
        let data_rows = rendered.lines().filter(|line| line.starts_with("| ")).count() - 2;
        assert_eq!(data_rows, ROWS.len());
    }

    #[test]
    fn matrix_rows_use_only_closed_statuses() {
        for row in ROWS {
            assert!(
                CLOSED_STATUSES.contains(&row.status),
                "matrix row {} uses non-closed status {}",
                row.feature,
                row.status
            );
        }
    }

    #[test]
    fn rendered_matrix_has_no_transitional_status_vocabulary() {
        let rendered = render_matrix();
        for transitional in [
            "implemented-needs-positive-wire-test",
            "needs-capability-parser",
            "needs-compat-test",
            "planned-needs-negative-gate",
        ] {
            assert!(
                !rendered.contains(transitional),
                "rendered matrix still documents transitional status {transitional}"
            );
        }
    }

    #[test]
    fn table_cells_escape_pipe_characters() {
        assert_eq!(escape_cell("a|b"), "a\\|b");
    }
}
