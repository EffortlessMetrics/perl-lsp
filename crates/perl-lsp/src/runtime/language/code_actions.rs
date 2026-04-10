//! Code action handlers
//!
//! Handles textDocument/codeAction and codeAction/resolve requests.
//! Provides quick fixes, refactoring actions, and source actions.

use super::super::*;
use crate::protocol::{req_range, req_uri};

fn pragma_insert_byte(text: &str) -> usize {
    if let Some(pos) = text.find("package ")
        && let Some(newline) = text[pos..].find('\n')
    {
        return pos + newline + 1;
    }
    0
}

fn external_perlcritic_quick_fixes(params: &Value, uri: &str, doc_text: &str) -> Vec<Value> {
    let mut actions = Vec::new();
    let diagnostics = params
        .get("context")
        .and_then(|context| context.get("diagnostics"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if diagnostics.is_empty() {
        return actions;
    }

    let insert_at = pragma_insert_byte(doc_text);
    let (insert_line, insert_char) = {
        let prefix = &doc_text[..insert_at.min(doc_text.len())];
        let line = prefix.bytes().filter(|b| *b == b'\n').count() as u32;
        (line, 0_u32)
    };

    let has_strict = doc_text.contains("use strict;");
    let has_warnings = doc_text.contains("use warnings;");

    for diag in diagnostics {
        let Some(code) = diag.get("code").and_then(Value::as_str) else {
            continue;
        };
        if code == "TestingAndDebugging::RequireUseStrict" && !has_strict {
            actions.push(json!({
                "title": "Add use strict; (Perl::Critic)",
                "kind": "quickfix",
                "diagnostics": [diag.clone()],
                "edit": {
                    "changes": {
                        uri: [{
                            "range": {
                                "start": { "line": insert_line, "character": insert_char },
                                "end": { "line": insert_line, "character": insert_char }
                            },
                            "newText": "use strict;\n"
                        }]
                    }
                }
            }));
        } else if code == "TestingAndDebugging::RequireUseWarnings" && !has_warnings {
            actions.push(json!({
                "title": "Add use warnings; (Perl::Critic)",
                "kind": "quickfix",
                "diagnostics": [diag.clone()],
                "edit": {
                    "changes": {
                        uri: [{
                            "range": {
                                "start": { "line": insert_line, "character": insert_char },
                                "end": { "line": insert_line, "character": insert_char }
                            },
                            "newText": "use warnings;\n"
                        }]
                    }
                }
            }));
        }
    }

    actions
}

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
            let diagnostics =
                diag_provider.get_diagnostics(ast, &doc.parse_errors, &doc.text, None);

            // Get code actions from both providers
            let mut code_actions: Vec<Value> = Vec::new();
            code_actions.extend(external_perlcritic_quick_fixes(&params, uri, &doc.text));

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
}
