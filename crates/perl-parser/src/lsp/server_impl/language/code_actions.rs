//! Code action handlers
//!
//! Handles textDocument/codeAction and codeAction/resolve requests.
//! Provides quick fixes, refactoring actions, and source actions.

use super::super::*;

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

        let uri = params["textDocument"]["uri"].as_str().ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Missing textDocument.uri".into(),
            data: None,
        })?;
        let start_line = params["range"]["start"]["line"].as_u64().unwrap_or(0) as u32;
        let start_char = params["range"]["start"]["character"].as_u64().unwrap_or(0) as u32;
        let end_line = params["range"]["end"]["line"].as_u64().unwrap_or(0) as u32;
        let end_char = params["range"]["end"]["character"].as_u64().unwrap_or(0) as u32;

        let documents = self.documents.lock().unwrap_or_else(|e| e.into_inner());
        let doc = match self.get_document(&documents, uri) {
            Some(d) => d,
            None => return Ok(Some(json!([]))),
        };

        if let Some(ast) = &doc.ast {
            let start_offset = self.pos16_to_offset(doc, start_line, start_char);
            let end_offset = self.pos16_to_offset(doc, end_line, end_char);

            // Get diagnostics from the document
            let diag_provider = DiagnosticsProvider::new(ast, doc.text.clone());
            let diagnostics = diag_provider.get_diagnostics(ast, &doc.parse_errors, &doc.text);

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
                                crate::perl_critic::Severity::Brutal |
                                crate::perl_critic::Severity::Cruel => 1, // Error
                                crate::perl_critic::Severity::Harsh => 2, // Warning
                                _ => 3, // Information
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

                code_actions.push(json!({
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
                }));
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

            // Always offer generic debug actions when there are diagnostics
            if !diagnostics.is_empty() {
                // Add debug print action
                code_actions.push(json!({
                    "title": "Add debug print",
                    "kind": "refactor.rewrite",
                    "command": {
                        "title": "Add debug print",
                        "command": "perl.addDebugPrint",
                        "arguments": [json!({ "uri": uri, "range": {
                            "start": {"line": start_line, "character": start_char},
                            "end": {"line": end_line, "character": end_char}
                        }})]
                    }
                }));

                // Extract variable action
                code_actions.push(json!({
                    "title": "Extract variable",
                    "kind": "refactor.extract",
                    "command": {
                        "title": "Extract variable",
                        "command": "perl.extractVariable",
                        "arguments": [json!({ "uri": uri, "range": {
                            "start": {"line": start_line, "character": start_char},
                            "end": {"line": end_line, "character": end_char}
                        }})]
                    }
                }));
            }

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
                let documents = self.documents.lock().unwrap_or_else(|e| e.into_inner());
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
                            let documents =
                                self.documents.lock().unwrap_or_else(|e| e.into_inner());
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
