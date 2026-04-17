//! Code action handlers
//!
//! Handles textDocument/codeAction and codeAction/resolve requests.
//! Provides quick fixes, refactoring actions, and source actions.

use super::super::*;
use crate::protocol::{req_range, req_uri};

fn requested_code_action_kinds(params: &Value) -> Vec<&str> {
    params
        .get("context")
        .and_then(|context| context.get("only"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn code_action_kind_matches_filter(kind: &str, requested_kind: &str) -> bool {
    requested_kind.is_empty()
        || kind == requested_kind
        || kind.strip_prefix(requested_kind).is_some_and(|suffix| suffix.starts_with('.'))
}

fn retain_requested_code_action_kinds(code_actions: &mut Vec<Value>, requested_kinds: &[&str]) {
    if requested_kinds.is_empty() {
        return;
    }

    code_actions.retain(|action| {
        action.get("kind").and_then(Value::as_str).is_some_and(|kind| {
            requested_kinds
                .iter()
                .any(|requested_kind| code_action_kind_matches_filter(kind, requested_kind))
        })
    });
}

fn display_diagnostic_message(diagnostic: &crate::features::diagnostics::Diagnostic) -> String {
    match &diagnostic.suggestion {
        Some(suggestion) => format!("{}\nSuggestion: {}", diagnostic.message, suggestion),
        None => diagnostic.message.clone(),
    }
}

/// Byte-offset-agnostic representation of an LSP range used only for
/// conflict detection in [`build_source_fix_all`]. Two edits conflict when
/// their ranges overlap in character space, which prevents stacking edits
/// that would corrupt the text when applied together.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct FixAllRange {
    start_line: u64,
    start_char: u64,
    end_line: u64,
    end_char: u64,
}

impl FixAllRange {
    fn from_json(range: &Value) -> Option<Self> {
        let start = range.get("start")?;
        let end = range.get("end")?;
        Some(Self {
            start_line: start.get("line")?.as_u64()?,
            start_char: start.get("character")?.as_u64()?,
            end_line: end.get("line")?.as_u64()?,
            end_char: end.get("character")?.as_u64()?,
        })
    }

    /// Two ranges overlap when they cover common text. Zero-width insertions
    /// at the same position are allowed to stack — LSP clients apply edits
    /// in the order provided, so distinct insertions (for example `use
    /// strict;` and `use warnings;` both at line 0) compose cleanly. The
    /// caller still dedupes exact-match insertions in
    /// [`build_source_fix_all`], so this only returns `true` when applying
    /// both edits together would actually corrupt the text.
    fn overlaps(&self, other: &Self) -> bool {
        let (a, b) = if self <= other { (self, other) } else { (other, self) };
        let a_is_insertion = a.start_line == a.end_line && a.start_char == a.end_char;
        let b_is_insertion = b.start_line == b.end_line && b.start_char == b.end_char;
        if a_is_insertion || b_is_insertion {
            return false;
        }
        // a.start <= b.start; they overlap iff b.start < a.end.
        (b.start_line, b.start_char) < (a.end_line, a.end_char)
    }
}

/// Build the aggregate `source.fixAll` action from the code actions already
/// collected for the current request. Aggregation rules:
///
/// * Only actions with `kind == "quickfix"` are considered — refactorings and
///   source actions may require user input and are excluded per the LSP
///   `source.fixAll` contract (no unsafe fixes, no user choices).
/// * Each action must carry a `WorkspaceEdit` with a `changes` map keyed by
///   the document URI. Command-only quick fixes are skipped because they
///   cannot be applied together without a round-trip.
/// * Overlapping edits are resolved by "first wins" — the first edit in the
///   iteration order keeps its slot, and any later edit whose range overlaps
///   an already-accepted range is dropped. Zero-width insertions (for example
///   `use strict;\n` and `use warnings;\n` both at position 0) are **never**
///   considered overlapping via the range check — they are allowed to stack.
///   Exact-duplicate `(range, newText)` pairs are deduped by a separate hash
///   set, so two providers that independently suggest the same pragma each
///   produce only one edit in the aggregate.
/// * The aggregate is only emitted when at least two distinct edits survive
///   conflict resolution — a single quick fix is already the preferred action
///   and does not need a wrapper.
///
/// The resulting action lists every diagnostic associated with the accepted
/// source actions so the client can clear them together once the aggregate
/// is applied.
fn build_source_fix_all(code_actions: &[Value], uri: &str) -> Option<Value> {
    let mut accepted_ranges: Vec<FixAllRange> = Vec::new();
    let mut merged_edits: Vec<Value> = Vec::new();
    let mut merged_diagnostics: Vec<Value> = Vec::new();
    let mut seen_diagnostic_keys: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    let mut seen_edit_keys: std::collections::HashSet<String> = std::collections::HashSet::new();

    for action in code_actions {
        if action.get("kind").and_then(Value::as_str) != Some("quickfix") {
            continue;
        }

        // Skip command-only quick fixes — they can't be merged into a single
        // WorkspaceEdit.
        let Some(changes) = action.pointer("/edit/changes").and_then(Value::as_object) else {
            continue;
        };
        let Some(edits) = changes.get(uri).and_then(Value::as_array) else {
            continue;
        };

        // Compute the ranges for this action so we can reject the whole action
        // atomically if any of its edits conflict with an already-accepted
        // edit. Atomicity matters because a code action's edits are designed
        // to be applied together.
        let action_ranges: Vec<FixAllRange> = edits
            .iter()
            .filter_map(|edit| edit.get("range").and_then(FixAllRange::from_json))
            .collect();

        if action_ranges.len() != edits.len() {
            // At least one edit had a malformed range — bail on this action.
            continue;
        }

        if action_ranges.iter().any(|r| accepted_ranges.iter().any(|a| a.overlaps(r))) {
            continue;
        }

        // Accept the action: record its ranges and copy its edits verbatim,
        // deduping identical (range, newText) pairs so two providers that
        // both add `use strict;\n` at offset 0 produce a single edit.
        accepted_ranges.extend(action_ranges.iter().copied());
        for edit in edits {
            let new_text = edit.get("newText").and_then(Value::as_str).unwrap_or_default();
            let range_repr = edit.get("range").map(|r| r.to_string()).unwrap_or_default();
            let key = format!("{range_repr}|{new_text}");
            if seen_edit_keys.insert(key) {
                merged_edits.push(edit.clone());
            }
        }

        if let Some(diags) = action.get("diagnostics").and_then(Value::as_array) {
            for diag in diags {
                // Dedupe diagnostics by (code, range) so the same upstream
                // finding isn't surfaced twice in the aggregate action.
                let code = diag.get("code").and_then(Value::as_str).unwrap_or_default().to_string();
                let range_key = diag.get("range").map(|r| r.to_string()).unwrap_or_default();
                let key = format!("{code}|{range_key}");
                if seen_diagnostic_keys.insert(key) {
                    merged_diagnostics.push(diag.clone());
                }
            }
        }
    }

    // Only emit the aggregate when it actually aggregates something.
    if merged_edits.len() < 2 {
        return None;
    }

    let mut changes = serde_json::Map::new();
    changes.insert(uri.to_string(), Value::Array(merged_edits));

    let mut action = json!({
        "title": "Fix all auto-fixable issues",
        "kind": "source.fixAll",
        "isPreferred": true,
        "edit": {
            "changes": Value::Object(changes),
        },
    });

    if !merged_diagnostics.is_empty() {
        if let Some(object) = action.as_object_mut() {
            object.insert("diagnostics".to_string(), Value::Array(merged_diagnostics));
        }
    }

    Some(action)
}

impl LspServer {
    /// Handle textDocument/codeAction request
    pub(crate) fn handle_code_action(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let params = match params {
            Some(p) => p,
            None => return Ok(Some(json!([]))),
        };

        let uri = req_uri(&params)?;
        let ((start_line, start_char), (end_line, end_char)) = req_range(&params)?;
        let requested_kinds = requested_code_action_kinds(&params);

        let documents = self.documents_guard();
        let doc = match self.get_document(&documents, uri) {
            Some(d) => d,
            None => return Ok(Some(json!([]))),
        };

        if let Some(ast) = &doc.ast {
            let start_offset = self.pos16_to_offset(doc, start_line, start_char);
            let end_offset = self.pos16_to_offset(doc, end_line, end_char);

            // Get diagnostics from the document
            let diag_provider = DiagnosticsProvider::new(ast, doc.text.clone());
            let mut diagnostics =
                diag_provider.get_diagnostics(ast, &doc.parse_errors, &doc.text, None);
            diagnostics.extend(self.context_diagnostics_for_code_actions(&params, doc));

            // Get code actions from both providers
            let mut code_actions: Vec<Value> = Vec::new();

            // Add Perl::Critic quick fixes
            let builtin_analyzer = BuiltInAnalyzer::new();
            let violations = builtin_analyzer.analyze(ast, &doc.text);
            for violation in &violations {
                if let Some(quick_fix) = builtin_analyzer.get_quick_fix(violation, &doc.text) {
                    let mut changes = HashMap::new();
                    let (start_line, start_char) =
                        self.offset_to_pos16(doc, violation.range.start.byte);
                    let (end_line, end_char) = self.offset_to_pos16(doc, violation.range.end.byte);

                    changes.insert(
                        uri.to_string(),
                        vec![json!({
                            "range": {
                                "start": {"line": start_line, "character": start_char},
                                "end": {"line": end_line, "character": end_char},
                            },
                            "newText": quick_fix.edit.new_text,
                        })],
                    );

                    code_actions.push(json!({
                        "title": quick_fix.title,
                        "kind": "quickfix",
                        "diagnostics": [{
                            "range": {
                                "start": {"line": start_line, "character": start_char},
                                "end": {"line": end_line, "character": end_char},
                            },
                            "severity": match violation.severity {
                                crate::perl_critic::Severity::Gentle => 1, // Error
                                crate::perl_critic::Severity::Stern |
                                crate::perl_critic::Severity::Harsh => 2, // Warning
                                crate::perl_critic::Severity::Cruel => 3, // Information
                                crate::perl_critic::Severity::Brutal => 4, // Hint
                            },
                            "code": violation.policy.clone(),
                            "source": "Perl::Critic",
                            "message": violation.description.clone()
                        }],
                        "edit": {
                            "changes": changes,
                        },
                    }));
                }
            }

            // Get quick-fixes from the V2 provider (diagnostic-based)
            let provider_v2 = CodeActionsProviderV2::new(doc.text.clone());
            let quick_fixes =
                provider_v2.get_code_actions((start_offset, end_offset), &diagnostics);

            for action in quick_fixes {
                let mut changes = HashMap::new();
                let (start_line, start_char) = self.offset_to_pos16(doc, action.edit.range.0);
                let (end_line, end_char) = self.offset_to_pos16(doc, action.edit.range.1);

                let edits = vec![json!({
                    "range": {
                        "start": {"line": start_line, "character": start_char},
                        "end": {"line": end_line, "character": end_char},
                    },
                    "newText": action.edit.new_text,
                })];
                changes.insert(uri.to_string(), edits);

                let associated_diagnostics: Vec<Value> = action
                    .diagnostic_id
                    .as_deref()
                    .zip(action.diagnostic_range)
                    .into_iter()
                    .filter_map(|(code, range)| {
                        diagnostics.iter().find(|diagnostic| {
                            diagnostic.code.as_deref() == Some(code) && diagnostic.range == range
                        })
                    })
                    .map(|diagnostic| {
                        let (diag_start_line, diag_start_char) =
                            self.offset_to_pos16(doc, diagnostic.range.0);
                        let (diag_end_line, diag_end_char) =
                            self.offset_to_pos16(doc, diagnostic.range.1);

                        json!({
                            "range": {
                                "start": {"line": diag_start_line, "character": diag_start_char},
                                "end": {"line": diag_end_line, "character": diag_end_char},
                            },
                            "severity": match diagnostic.severity {
                                crate::features::diagnostics::DiagnosticSeverity::Error => 1,
                                crate::features::diagnostics::DiagnosticSeverity::Warning => 2,
                                crate::features::diagnostics::DiagnosticSeverity::Information => 3,
                                crate::features::diagnostics::DiagnosticSeverity::Hint => 4,
                            },
                            "code": diagnostic.code.clone(),
                            "source": "perl-lsp",
                            "message": display_diagnostic_message(diagnostic),
                        })
                    })
                    .collect();

                let mut action_json = json!({
                    "title": action.title,
                    "kind": match action.kind {
                        InternalCodeActionKindV2::QuickFix => "quickfix",
                        InternalCodeActionKindV2::Refactor => "refactor",
                        InternalCodeActionKindV2::RefactorExtract => "refactor.extract",
                        InternalCodeActionKindV2::RefactorInline => "refactor.inline",
                        InternalCodeActionKindV2::RefactorRewrite => "refactor.rewrite",
                    },
                    "edit": {
                        "changes": changes,
                    },
                });

                if let Some(action_object) = action_json.as_object_mut() {
                    if !associated_diagnostics.is_empty() {
                        action_object.insert(
                            "diagnostics".to_string(),
                            Value::Array(associated_diagnostics),
                        );
                    }
                }

                code_actions.push(action_json);
            }

            // Get refactorings from the original provider (AST-based)
            let provider = CodeActionsProvider::new(doc.text.clone());
            let actions = provider.get_code_actions(ast, (start_offset, end_offset), &diagnostics);

            for action in actions {
                let mut changes = HashMap::new();
                let edits: Vec<Value> = action
                    .edit
                    .changes
                    .into_iter()
                    .map(|edit| {
                        let (start_line, start_char) =
                            self.offset_to_pos16(doc, edit.location.start);
                        let (end_line, end_char) = self.offset_to_pos16(doc, edit.location.end);
                        json!({
                            "range": {
                                "start": {"line": start_line, "character": start_char},
                                "end": {"line": end_line, "character": end_char},
                            },
                            "newText": edit.new_text,
                        })
                    })
                    .collect();
                changes.insert(uri.to_string(), edits);

                code_actions.push(json!({
                    "title": action.title,
                    "kind": match action.kind {
                        InternalCodeActionKind::QuickFix => "quickfix",
                        InternalCodeActionKind::Refactor => "refactor",
                        InternalCodeActionKind::RefactorExtract => "refactor.extract",
                        InternalCodeActionKind::RefactorInline => "refactor.inline",
                        InternalCodeActionKind::RefactorRewrite => "refactor.rewrite",
                        InternalCodeActionKind::Source => "source",
                        InternalCodeActionKind::SourceOrganizeImports => "source.organizeImports",
                        InternalCodeActionKind::SourceFixAll => "source.fixAll",
                        InternalCodeActionKind::SourceModernize => "source.modernize",
                    },
                    "edit": {
                        "changes": changes,
                    },
                }));
            }

            // Get enhanced refactorings (extract variable, convert loops, etc.)
            let enhanced_provider = EnhancedCodeActionsProvider::new(doc.text.clone());
            let enhanced_actions =
                enhanced_provider.get_enhanced_refactoring_actions(ast, (start_offset, end_offset));

            // Add test generation actions
            let test_generator = TestGenerator::new("Test::More");
            let subroutines = test_generator.find_subroutines(ast);

            for action in enhanced_actions {
                let mut changes = HashMap::new();
                let edits: Vec<Value> = action
                    .edit
                    .changes
                    .into_iter()
                    .map(|edit| {
                        let (start_line, start_char) =
                            self.offset_to_pos16(doc, edit.location.start);
                        let (end_line, end_char) = self.offset_to_pos16(doc, edit.location.end);
                        json!({
                            "range": {
                                "start": {"line": start_line, "character": start_char},
                                "end": {"line": end_line, "character": end_char},
                            },
                            "newText": edit.new_text,
                        })
                    })
                    .collect();
                changes.insert(uri.to_string(), edits);

                code_actions.push(json!({
                    "title": action.title,
                    "kind": match action.kind {
                        InternalCodeActionKind::QuickFix => "quickfix",
                        InternalCodeActionKind::Refactor => "refactor",
                        InternalCodeActionKind::RefactorExtract => "refactor.extract",
                        InternalCodeActionKind::RefactorInline => "refactor.inline",
                        InternalCodeActionKind::RefactorRewrite => "refactor.rewrite",
                        InternalCodeActionKind::Source => "source",
                        InternalCodeActionKind::SourceOrganizeImports => "source.organizeImports",
                        InternalCodeActionKind::SourceFixAll => "source.fixAll",
                        InternalCodeActionKind::SourceModernize => "source.modernize",
                    },
                    "edit": {
                        "changes": changes,
                    },
                }));
            }

            // Add test generation actions for subroutines in range
            for sub_info in subroutines {
                // Check if cursor is near this subroutine
                let test_code = test_generator.generate_test(&sub_info.name, sub_info.param_count);
                code_actions.push(json!({
                    "title": format!("Generate test for '{}'", sub_info.name),
                    "kind": "source",
                    "command": {
                        "title": "Generate test",
                        "command": "perl.generateTest",
                        "arguments": [json!({
                            "uri": uri,
                            "name": sub_info.name,
                            "test": test_code
                        })]
                    }
                }));
            }

            // Add missing pragma actions (use strict / use warnings) when applicable
            let mut pragma_actions =
                crate::code_actions_pragmas::missing_pragmas_actions(uri, &doc.text);
            for action in &mut pragma_actions {
                let data_info = (
                    action
                        .get("data")
                        .and_then(|d| d.get("uri"))
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string()),
                    action.get("data").and_then(|d| d.get("insertAt")).and_then(|n| n.as_u64()),
                    action
                        .get("data")
                        .and_then(|d| d.get("text"))
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string()),
                );

                if let (Some(u), Some(off), Some(txt)) = data_info {
                    if let Some(obj) = action.as_object_mut() {
                        let edit_range = if off as usize >= doc.text.len() {
                            let end = self.get_document_end_position(&doc.text);
                            json!({"start": end.clone(), "end": end })
                        } else {
                            let (line, col) = self.offset_to_pos16(doc, off as usize);
                            json!({
                                "start": {"line": line, "character": col},
                                "end": {"line": line, "character": col}
                            })
                        };

                        obj.insert(
                            "edit".into(),
                            json!({
                                "changes": {
                                    u: [{
                                        "range": edit_range,
                                        "newText": txt
                                    }]
                                }
                            }),
                        );
                        obj.remove("data");
                    }
                }
            }
            code_actions.extend(pragma_actions);

            // Aggregate all quick fixes collected so far into a single
            // `source.fixAll` action (LSP 3.17) when there are two or more
            // distinct edits. Editors use this to apply every safe fix with
            // one keystroke.
            if let Some(fix_all) = build_source_fix_all(&code_actions, uri) {
                code_actions.push(fix_all);
            }

            retain_requested_code_action_kinds(&mut code_actions, &requested_kinds);
            Ok(Some(json!(code_actions)))
        } else {
            // No AST (parse error), but we can still offer some actions
            let mut code_actions: Vec<Value> = Vec::new();

            // Check if source lacks strict/warnings
            if !doc.text.contains("use strict") || !doc.text.contains("use warnings") {
                let mut changes = HashMap::new();
                // Find first non-shebang line
                let insert_pos = if doc.text.starts_with("#!") {
                    doc.text.find('\n').map(|p| p + 1).unwrap_or(0)
                } else {
                    0
                };

                let new_text =
                    if !doc.text.contains("use strict") && !doc.text.contains("use warnings") {
                        "use strict;\nuse warnings;\n\n"
                    } else if !doc.text.contains("use strict") {
                        "use strict;\n"
                    } else {
                        "use warnings;\n"
                    };

                let (line, char) = self.offset_to_pos16(doc, insert_pos);
                changes.insert(
                    uri.to_string(),
                    vec![json!({
                        "range": {
                            "start": {"line": line, "character": char},
                            "end": {"line": line, "character": char},
                        },
                        "newText": new_text,
                    })],
                );

                code_actions.push(json!({
                    "title": "Add 'use strict' and 'use warnings'",
                    "kind": "quickfix",
                    "edit": {
                        "changes": changes,
                    },
                }));
            }

            // Always offer debug actions for files with issues
            code_actions.push(json!({
                "title": "Add debug print",
                "kind": "refactor.rewrite",
                "command": {
                    "title": "Add debug print",
                    "command": "perl.addDebugPrint",
                    "arguments": [json!({ "uri": uri })]
                }
            }));

            // Check for global variables that could use 'my' declarations
            let global_var_pattern = regex::Regex::new(r"(?m)^(\$|\@|\%)[a-zA-Z_]\w*\s*=").ok();
            if let Some(re) = global_var_pattern {
                if re.is_match(&doc.text) {
                    code_actions.push(json!({
                        "title": "Convert globals to 'my' declarations",
                        "kind": "refactor.rewrite",
                        "command": {
                            "title": "Convert to my declarations",
                            "command": "perl.convertToMyDeclarations",
                            "arguments": [json!({ "uri": uri })]
                        }
                    }));
                }
            }

            retain_requested_code_action_kinds(&mut code_actions, &requested_kinds);
            Ok(Some(json!(code_actions)))
        }
    }

    /// Handle textDocument/codeAction request for pragmas
    #[allow(dead_code)] // Used in tests
    pub(crate) fn handle_code_actions_pragmas(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(p) = params {
            if let Some(uri) = p["textDocument"]["uri"].as_str() {
                let documents = self.documents_guard();
                if let Some(doc) = documents.get(uri) {
                    let mut actions =
                        crate::code_actions_pragmas::missing_pragmas_actions(uri, &doc.text);

                    // Fill in edits with proper ranges
                    for a in &mut actions {
                        let data_info = (
                            a.get("data")
                                .and_then(|d| d.get("uri"))
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string()),
                            a.get("data").and_then(|d| d.get("insertAt")).and_then(|n| n.as_u64()),
                            a.get("data")
                                .and_then(|d| d.get("text"))
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string()),
                        );

                        if let (Some(u), Some(off), Some(txt)) = data_info {
                            if let Some(obj) = a.as_object_mut() {
                                let edit_range = if off as usize >= doc.text.len() {
                                    let end = self.get_document_end_position(&doc.text);
                                    json!({"start": end.clone(), "end": end })
                                } else {
                                    let (line, col) = self.offset_to_pos16(doc, off as usize);
                                    json!({
                                        "start": {"line": line, "character": col},
                                        "end": {"line": line, "character": col}
                                    })
                                };

                                obj.insert(
                                    "edit".into(),
                                    json!({
                                        "changes": {
                                            u: [{
                                                "range": edit_range,
                                                "newText": txt
                                            }]
                                        }
                                    }),
                                );
                                obj.remove("data");
                            }
                        }
                    }
                    return Ok(Some(json!(actions)));
                }
            }
        }
        Ok(Some(json!([])))
    }

    /// Handle codeAction/resolve request
    pub(crate) fn handle_code_action_resolve(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(mut action) = params {
            // The action should already have minimal information
            // We now need to compute the actual edits

            if let Some(kind) = action.get("kind").and_then(|k| k.as_str()) {
                if kind == "quickfix" {
                    // For quickfix actions, compute the workspace edit now
                    if let Some(data) = action.get("data") {
                        if let Some(uri) = data.get("uri").and_then(|u| u.as_str()) {
                            let documents = self.documents_guard();
                            if self.get_document(&documents, uri).is_some() {
                                // Example: Add "use strict;" at the beginning
                                if let Some(pragma) = data.get("pragma").and_then(|p| p.as_str()) {
                                    let text = format!("{}\n", pragma);
                                    let edit = json!({
                                        "changes": {
                                            uri: [{
                                                "range": {
                                                    "start": {"line": 0, "character": 0},
                                                    "end": {"line": 0, "character": 0}
                                                },
                                                "newText": text
                                            }]
                                        }
                                    });

                                    if let Some(obj) = action.as_object_mut() {
                                        obj.insert("edit".to_string(), edit);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Ok(Some(action))
        } else {
            Ok(None)
        }
    }
}

impl LspServer {
    fn context_diagnostics_for_code_actions(
        &self,
        params: &Value,
        doc: &crate::runtime::DocumentState,
    ) -> Vec<crate::features::diagnostics::Diagnostic> {
        params
            .get("context")
            .and_then(|ctx| ctx.get("diagnostics"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|diag| {
                let range = diag.get("range")?;
                let start_line = range.get("start")?.get("line")?.as_u64()? as u32;
                let start_char = range.get("start")?.get("character")?.as_u64()? as u32;
                let end_line = range.get("end")?.get("line")?.as_u64()? as u32;
                let end_char = range.get("end")?.get("character")?.as_u64()? as u32;
                let code = diag.get("code").and_then(Value::as_str)?.to_string();
                let message = diag.get("message").and_then(Value::as_str)?.to_string();

                let severity = match diag.get("severity").and_then(Value::as_u64) {
                    Some(1) => crate::features::diagnostics::DiagnosticSeverity::Error,
                    Some(2) => crate::features::diagnostics::DiagnosticSeverity::Warning,
                    Some(3) => crate::features::diagnostics::DiagnosticSeverity::Information,
                    Some(4) => crate::features::diagnostics::DiagnosticSeverity::Hint,
                    _ => crate::features::diagnostics::DiagnosticSeverity::Warning,
                };

                Some(crate::features::diagnostics::Diagnostic {
                    range: (
                        self.pos16_to_offset(doc, start_line, start_char),
                        self.pos16_to_offset(doc, end_line, end_char),
                    ),
                    severity,
                    code: Some(code),
                    message,
                    suggestion: None,
                    related_information: Vec::new(),
                    tags: Vec::new(),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_action_kind_filter_matches_subkinds() {
        assert!(code_action_kind_matches_filter("refactor.rewrite", "refactor"));
        assert!(code_action_kind_matches_filter("source.organizeImports", "source"));
        assert!(code_action_kind_matches_filter("quickfix", "quickfix"));
        assert!(!code_action_kind_matches_filter("quickfix", "refactor"));
        assert!(!code_action_kind_matches_filter("refactor.rewrite.extra", "refactor.inline"));
    }

    #[test]
    fn retain_requested_code_action_kinds_filters_unrequested_actions() {
        let mut actions = vec![
            json!({"title": "quickfix", "kind": "quickfix"}),
            json!({"title": "rewrite", "kind": "refactor.rewrite"}),
            json!({"title": "organize", "kind": "source.organizeImports"}),
        ];

        retain_requested_code_action_kinds(&mut actions, &["refactor"]);

        let remaining_kinds: Vec<&str> =
            actions.iter().filter_map(|action| action["kind"].as_str()).collect();
        assert_eq!(remaining_kinds, vec!["refactor.rewrite"]);
    }

    fn open_test_document(server: &LspServer, uri: &str, text: &str) {
        let result = server.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text,
            }
        })));
        assert!(result.is_ok(), "didOpen failed: {result:?}");
    }

    #[test]
    fn code_action_runtime_offers_missing_pragmas() {
        let server = LspServer::new();
        let uri = "file:///test.pl";
        let text = "print 'hello';\n";
        open_test_document(&server, uri, text);

        let response = server.handle_code_action(Some(json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 5 }
            },
            "context": { "diagnostics": [] }
        })));

        let actions =
            response.ok().flatten().and_then(|v| v.as_array().cloned()).unwrap_or_default();

        assert!(
            actions.iter().any(|a| a["title"].as_str().unwrap_or("").contains("use strict")),
            "expected missing pragma action, got: {actions:?}"
        );
    }

    /// Build a minimal quickfix action for use in unit tests.  The action has
    /// exactly one edit on the supplied single-line range and a single
    /// associated diagnostic so we can verify diagnostic propagation.
    fn make_quickfix(
        uri: &str,
        line: u64,
        start_char: u64,
        end_char: u64,
        new_text: &str,
        title: &str,
        diag_code: Option<&str>,
    ) -> Value {
        let mut action = json!({
            "title": title,
            "kind": "quickfix",
            "edit": {
                "changes": {
                    uri: [{
                        "range": {
                            "start": {"line": line, "character": start_char},
                            "end": {"line": line, "character": end_char},
                        },
                        "newText": new_text,
                    }]
                }
            }
        });

        if let Some(code) = diag_code {
            if let Some(object) = action.as_object_mut() {
                object.insert(
                    "diagnostics".to_string(),
                    json!([{
                        "range": {
                            "start": {"line": line, "character": start_char},
                            "end": {"line": line, "character": end_char},
                        },
                        "code": code,
                        "message": format!("Diagnostic for {code}"),
                        "source": "perl-lsp",
                        "severity": 2,
                    }]),
                );
            }
        }

        action
    }

    #[test]
    fn fix_all_range_overlap_detects_contained_ranges() {
        let outer = FixAllRange { start_line: 0, start_char: 0, end_line: 0, end_char: 20 };
        let inner = FixAllRange { start_line: 0, start_char: 5, end_line: 0, end_char: 10 };
        assert!(outer.overlaps(&inner));
        assert!(inner.overlaps(&outer));
    }

    #[test]
    fn fix_all_range_overlap_excludes_adjacent_ranges() {
        let left = FixAllRange { start_line: 0, start_char: 0, end_line: 0, end_char: 5 };
        let right = FixAllRange { start_line: 0, start_char: 5, end_line: 0, end_char: 10 };
        assert!(!left.overlaps(&right));
        assert!(!right.overlaps(&left));
    }

    #[test]
    fn fix_all_range_overlap_allows_stacked_insertions() {
        // Two distinct fixes can both insert at position (0,0) — for
        // example `use strict;\n` + `use warnings;\n`. LSP clients apply
        // them in sequence and they compose cleanly.  [`build_source_fix_all`]
        // dedupes exact-match edits separately.
        let first = FixAllRange { start_line: 0, start_char: 0, end_line: 0, end_char: 0 };
        let second = FixAllRange { start_line: 0, start_char: 0, end_line: 0, end_char: 0 };
        assert!(!first.overlaps(&second));
    }

    #[test]
    fn fix_all_range_overlap_allows_insertion_outside_range() {
        let insertion = FixAllRange { start_line: 5, start_char: 0, end_line: 5, end_char: 0 };
        let unrelated = FixAllRange { start_line: 0, start_char: 0, end_line: 0, end_char: 5 };
        assert!(!insertion.overlaps(&unrelated));
        assert!(!unrelated.overlaps(&insertion));
    }

    #[test]
    fn fix_all_range_overlap_detects_multiline_overlap() {
        // A multi-line replacement (lines 1-3) and an edit on line 2 must conflict.
        let multiline = FixAllRange { start_line: 1, start_char: 0, end_line: 3, end_char: 0 };
        let inner = FixAllRange { start_line: 2, start_char: 0, end_line: 2, end_char: 10 };
        assert!(multiline.overlaps(&inner));
        assert!(inner.overlaps(&multiline));
    }

    #[test]
    fn fix_all_range_overlap_insertion_inside_real_range_allowed() {
        // Design note: overlaps() returns false when either side is a zero-width
        // insertion, even if the insertion falls inside a replacement range.
        // The assumption is that quick fixes operate on original document positions
        // per the LSP spec ("all text edit ranges refer to positions in the original
        // document"), so an insertion at (0,5) and a replacement of (0,0)-(0,10) are
        // independently valid.  This test documents the current behaviour so it is
        // visible to future maintainers.
        let replacement = FixAllRange { start_line: 0, start_char: 0, end_line: 0, end_char: 10 };
        let insertion = FixAllRange { start_line: 0, start_char: 5, end_line: 0, end_char: 5 };
        assert!(!replacement.overlaps(&insertion));
        assert!(!insertion.overlaps(&replacement));
    }

    #[test]
    fn build_source_fix_all_aggregates_multiple_quickfixes() {
        let uri = "file:///aggregate.pl";
        let actions = vec![
            make_quickfix(uri, 1, 4, 10, "fix-a", "Fix A", Some("PL100")),
            make_quickfix(uri, 2, 0, 4, "fix-b", "Fix B", Some("PL101")),
            make_quickfix(uri, 3, 2, 5, "fix-c", "Fix C", Some("PL102")),
        ];

        let aggregate = build_source_fix_all(&actions, uri).expect("aggregate present");
        assert_eq!(aggregate["title"], "Fix all auto-fixable issues");
        assert_eq!(aggregate["kind"], "source.fixAll");
        assert_eq!(aggregate["isPreferred"], true);

        let edits = aggregate["edit"]["changes"][uri].as_array().expect("aggregate has edits");
        assert_eq!(edits.len(), 3, "three non-conflicting edits were kept");

        let diagnostics = aggregate["diagnostics"].as_array().expect("diagnostics array");
        assert_eq!(diagnostics.len(), 3, "one diagnostic per quickfix");
    }

    #[test]
    fn build_source_fix_all_returns_none_for_single_quickfix() {
        let uri = "file:///single.pl";
        let actions = vec![make_quickfix(uri, 0, 0, 3, "fix", "Fix solo", Some("PL100"))];
        assert!(build_source_fix_all(&actions, uri).is_none());
    }

    #[test]
    fn build_source_fix_all_returns_none_without_any_quickfix() {
        let uri = "file:///none.pl";
        let actions = vec![
            json!({
                "title": "Extract variable",
                "kind": "refactor.extract",
                "edit": { "changes": { uri: [] } }
            }),
            json!({
                "title": "Organize imports",
                "kind": "source.organizeImports",
                "edit": { "changes": { uri: [] } }
            }),
        ];
        assert!(build_source_fix_all(&actions, uri).is_none());
    }

    #[test]
    fn build_source_fix_all_ignores_non_quickfix_kinds() {
        let uri = "file:///mixed.pl";
        let actions = vec![
            make_quickfix(uri, 0, 0, 3, "fix-a", "Fix A", Some("PL100")),
            make_quickfix(uri, 1, 0, 3, "fix-b", "Fix B", Some("PL101")),
            json!({
                "title": "Extract variable",
                "kind": "refactor.extract",
                "edit": {
                    "changes": {
                        uri: [{
                            "range": {
                                "start": {"line": 2, "character": 0},
                                "end": {"line": 2, "character": 5},
                            },
                            "newText": "extracted",
                        }]
                    }
                }
            }),
        ];
        let aggregate = build_source_fix_all(&actions, uri).expect("aggregate present");
        let edits = aggregate["edit"]["changes"][uri].as_array().expect("aggregate has edits");
        assert_eq!(edits.len(), 2, "refactor action must not be merged into fixAll");
    }

    #[test]
    fn build_source_fix_all_rejects_overlapping_edits() {
        let uri = "file:///overlap.pl";
        // Second action overlaps the first — must be skipped atomically.
        let actions = vec![
            make_quickfix(uri, 0, 0, 10, "first", "Fix A", Some("PL100")),
            make_quickfix(uri, 0, 5, 15, "second", "Fix B", Some("PL101")),
            make_quickfix(uri, 2, 0, 3, "third", "Fix C", Some("PL102")),
        ];
        let aggregate = build_source_fix_all(&actions, uri).expect("aggregate present");
        let edits = aggregate["edit"]["changes"][uri].as_array().expect("aggregate has edits");
        assert_eq!(edits.len(), 2);
        let new_texts: Vec<&str> =
            edits.iter().filter_map(|edit| edit["newText"].as_str()).collect();
        assert_eq!(new_texts, vec!["first", "third"]);
    }

    #[test]
    fn build_source_fix_all_skips_command_only_quickfixes() {
        let uri = "file:///command.pl";
        let actions = vec![
            make_quickfix(uri, 0, 0, 3, "fix-a", "Fix A", Some("PL100")),
            make_quickfix(uri, 1, 0, 3, "fix-b", "Fix B", Some("PL101")),
            // Command-only action: no `edit.changes` — cannot merge.
            json!({
                "title": "Run external fixer",
                "kind": "quickfix",
                "command": {"command": "perl.runFixer", "title": "Run"}
            }),
        ];
        let aggregate = build_source_fix_all(&actions, uri).expect("aggregate present");
        let edits = aggregate["edit"]["changes"][uri].as_array().expect("aggregate has edits");
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn build_source_fix_all_deduplicates_identical_edits() {
        // Two providers may both emit a quick fix that inserts exactly the
        // same text at the same position (e.g. "Add 'use strict;'" at 0,0).
        // The aggregate must only include that edit once.
        let uri = "file:///identical.pl";
        let actions = vec![
            make_quickfix(uri, 0, 0, 0, "use strict;\n", "Add strict (A)", Some("PL100")),
            make_quickfix(uri, 0, 0, 0, "use strict;\n", "Add strict (B)", Some("PL100")),
            make_quickfix(uri, 0, 0, 0, "use warnings;\n", "Add warnings", Some("PL101")),
        ];
        let aggregate = build_source_fix_all(&actions, uri).expect("aggregate present");
        let edits = aggregate["edit"]["changes"][uri].as_array().expect("aggregate has edits");
        assert_eq!(edits.len(), 2, "identical edits must dedupe: {edits:#?}");
        let texts: Vec<&str> = edits.iter().filter_map(|edit| edit["newText"].as_str()).collect();
        assert!(texts.contains(&"use strict;\n"));
        assert!(texts.contains(&"use warnings;\n"));
    }

    #[test]
    fn build_source_fix_all_deduplicates_diagnostics() {
        let uri = "file:///dupes.pl";
        let actions = vec![
            make_quickfix(uri, 0, 0, 3, "a", "Fix A", Some("PL100")),
            make_quickfix(uri, 1, 0, 3, "b", "Fix B1", Some("PL200")),
            // Same diagnostic code and range as Fix B1: its diagnostic entry
            // should not be listed twice on the aggregate.
            {
                let mut action = make_quickfix(uri, 2, 0, 3, "b2", "Fix B2", Some("PL200"));
                if let Some(object) = action.as_object_mut() {
                    object.insert(
                        "diagnostics".to_string(),
                        json!([{
                            "range": {
                                "start": {"line": 1, "character": 0},
                                "end": {"line": 1, "character": 3},
                            },
                            "code": "PL200",
                            "message": "Duplicate diagnostic",
                            "source": "perl-lsp",
                            "severity": 2,
                        }]),
                    );
                }
                action
            },
        ];
        let aggregate = build_source_fix_all(&actions, uri).expect("aggregate present");
        let diagnostics = aggregate["diagnostics"].as_array().expect("diagnostics array");
        let codes: Vec<&str> = diagnostics.iter().filter_map(|d| d["code"].as_str()).collect();
        assert_eq!(codes, vec!["PL100", "PL200"]);
    }

    #[test]
    fn build_source_fix_all_ignores_other_uris() {
        let uri = "file:///target.pl";
        let other_uri = "file:///other.pl";
        let actions = vec![
            make_quickfix(uri, 0, 0, 3, "a", "Fix A", Some("PL100")),
            make_quickfix(uri, 1, 0, 3, "b", "Fix B", Some("PL101")),
            make_quickfix(other_uri, 0, 0, 3, "c", "Cross-file", Some("PL102")),
        ];
        let aggregate = build_source_fix_all(&actions, uri).expect("aggregate present");
        let edits = aggregate["edit"]["changes"][uri].as_array().expect("aggregate has edits");
        // Only the two edits targeting `uri` are merged; the cross-file edit
        // is skipped because it has no `changes[uri]` entry.
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn code_action_runtime_offers_extract_variable() {
        let server = LspServer::new();
        let uri = "file:///test.pl";
        let text = r#"
my $str = "hello";
my $result = length($str) + 10;
print $result;
"#;
        open_test_document(&server, uri, text);

        let response = server.handle_code_action(Some(json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 2, "character": 13 },
                "end": { "line": 2, "character": 25 }
            },
            "context": { "diagnostics": [] }
        })));

        let actions =
            response.ok().flatten().and_then(|v| v.as_array().cloned()).unwrap_or_default();

        assert!(
            actions.iter().any(|a| {
                let title = a["title"].as_str().unwrap_or("");
                title.contains("Extract") && title.contains("variable")
            }),
            "expected extract-variable action, got: {actions:?}"
        );
    }

    #[test]
    fn code_action_runtime_emits_source_fix_all_when_multiple_quick_fixes_exist() {
        // A script with no pragmas and an undefined variable triggers at
        // least two quick fixes (add `use strict`, add `use warnings`).
        // The runtime should bundle them into a `source.fixAll` action in
        // addition to the individual quickfix actions.
        let server = LspServer::new();
        let uri = "file:///fix_all.pl";
        let text = "print $undefined;\n";
        open_test_document(&server, uri, text);

        let response = server.handle_code_action(Some(json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 0 }
            },
            "context": { "diagnostics": [] }
        })));

        let actions =
            response.ok().flatten().and_then(|v| v.as_array().cloned()).unwrap_or_default();

        let fix_all: Vec<&Value> =
            actions.iter().filter(|a| a["kind"].as_str() == Some("source.fixAll")).collect();

        assert_eq!(
            fix_all.len(),
            1,
            "exactly one source.fixAll aggregate should be emitted, got: {actions:#?}"
        );

        let aggregate = fix_all[0];
        assert_eq!(aggregate["title"], "Fix all auto-fixable issues");
        assert_eq!(aggregate["isPreferred"], true);

        let edits = aggregate["edit"]["changes"][uri].as_array().expect("aggregate has edits");
        assert!(edits.len() >= 2, "aggregate must combine at least two edits, got {edits:#?}");
    }

    #[test]
    fn code_action_runtime_skips_source_fix_all_for_clean_file() {
        // A file with `use strict` and `use warnings` already should not
        // produce enough quick fixes to justify an aggregate action.
        let server = LspServer::new();
        let uri = "file:///clean.pl";
        let text = "use strict;\nuse warnings;\nmy $x = 1;\nprint $x;\n";
        open_test_document(&server, uri, text);

        let response = server.handle_code_action(Some(json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 0 }
            },
            "context": { "diagnostics": [] }
        })));

        let actions =
            response.ok().flatten().and_then(|v| v.as_array().cloned()).unwrap_or_default();

        let fix_all_count =
            actions.iter().filter(|a| a["kind"].as_str() == Some("source.fixAll")).count();
        assert_eq!(
            fix_all_count, 0,
            "no source.fixAll should be emitted for a file with no quick fixes: {actions:#?}"
        );
    }

    #[test]
    fn code_action_runtime_returns_only_source_fix_all_when_filtered() {
        // When the client filters to `only: ["source.fixAll"]`, the runtime
        // must still run the full pipeline and then prune — verifying the
        // aggregator runs before `retain_requested_code_action_kinds`.
        let server = LspServer::new();
        let uri = "file:///filter.pl";
        let text = "print $undefined;\n";
        open_test_document(&server, uri, text);

        let response = server.handle_code_action(Some(json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 0 }
            },
            "context": { "diagnostics": [], "only": ["source.fixAll"] }
        })));

        let actions =
            response.ok().flatten().and_then(|v| v.as_array().cloned()).unwrap_or_default();

        assert!(!actions.is_empty(), "filtered request should still yield the aggregate");
        for action in &actions {
            assert_eq!(
                action["kind"].as_str(),
                Some("source.fixAll"),
                "filter must retain only source.fixAll: {action:#?}"
            );
        }
    }
}
