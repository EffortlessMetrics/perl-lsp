//! Quick fixes for diagnostic issues
//!
//! Provides automated fixes for common Perl issues driven by diagnostic codes.

use super::ast_utils::{find_declaration_position, get_indent_at};
use super::types::{CodeAction, CodeActionEdit, CodeActionKind};
use crate::ide::lsp_compat::diagnostics::Diagnostic;
use crate::ide::lsp_compat::rename::TextEdit;
use perl_parser_core::SourceLocation;

/// Fix undefined variable by declaring it
pub fn fix_undefined_variable(source: &str, diagnostic: &Diagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Extract variable name from diagnostic message
    if let Some(var_name) = diagnostic.message.split('\'').nth(1) {
        // Find the best place to insert declaration
        let insert_pos = find_declaration_position(source, diagnostic.range.0);

        // Add 'my' declaration
        actions.push(CodeAction {
            title: format!("Declare '{}' with 'my'", var_name),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec!["undefined-variable".to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: insert_pos, end: insert_pos },
                    new_text: format!("my {};\n", var_name),
                }],
            },
            is_preferred: true,
        });

        // Add 'our' declaration
        actions.push(CodeAction {
            title: format!("Declare '{}' with 'our'", var_name),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec!["undefined-variable".to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: insert_pos, end: insert_pos },
                    new_text: format!("our {};\n", var_name),
                }],
            },
            is_preferred: false,
        });
    }

    actions
}

/// Fix unused variable by removing it
pub fn fix_unused_variable(source: &str, diagnostic: &Diagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Find the declaration line
    let line_start = source[..diagnostic.range.0].rfind('\n').map(|p| p + 1).unwrap_or(0);
    let line_end = source[diagnostic.range.1..]
        .find('\n')
        .map(|p| diagnostic.range.1 + p)
        .unwrap_or(source.len());

    actions.push(CodeAction {
        title: "Remove unused variable".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec!["unused-variable".to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: line_start, end: line_end + 1 },
                new_text: String::new(),
            }],
        },
        is_preferred: true,
    });

    // Add underscore prefix to mark as intentionally unused
    if let Some(var_name) = diagnostic.message.split('\'').nth(1) {
        actions.push(CodeAction {
            title: format!("Rename to '_{}'", var_name),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec!["unused-variable".to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                    new_text: format!("_{}", var_name),
                }],
            },
            is_preferred: false,
        });
    }

    actions
}

/// Fix assignment in condition
pub fn fix_assignment_in_condition(source: &str, diagnostic: &Diagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Change = to ==
    let assignment_pos =
        source[diagnostic.range.0..diagnostic.range.1].find('=').map(|p| diagnostic.range.0 + p);

    if let Some(pos) = assignment_pos {
        actions.push(CodeAction {
            title: "Change to comparison (==)".to_string(),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec!["assignment-in-condition".to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: pos, end: pos + 1 },
                    new_text: "==".to_string(),
                }],
            },
            is_preferred: true,
        });

        // Wrap in parentheses to make intention clear
        actions.push(CodeAction {
            title: "Keep assignment (add parentheses)".to_string(),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec!["assignment-in-condition".to_string()],
            edit: CodeActionEdit {
                changes: vec![
                    TextEdit {
                        location: SourceLocation {
                            start: diagnostic.range.0,
                            end: diagnostic.range.0,
                        },
                        new_text: "(".to_string(),
                    },
                    TextEdit {
                        location: SourceLocation {
                            start: diagnostic.range.1,
                            end: diagnostic.range.1,
                        },
                        new_text: ")".to_string(),
                    },
                ],
            },
            is_preferred: false,
        });
    }

    actions
}

/// Add 'use strict' pragma
pub fn add_use_strict() -> Vec<CodeAction> {
    vec![CodeAction {
        title: "Add 'use strict'".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec!["missing-strict".to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: 0, end: 0 },
                new_text: "use strict;\n".to_string(),
            }],
        },
        is_preferred: true,
    }]
}

/// Add 'use warnings' pragma
pub fn add_use_warnings() -> Vec<CodeAction> {
    vec![CodeAction {
        title: "Add 'use warnings'".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec!["missing-warnings".to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: 0, end: 0 },
                new_text: "use warnings;\n".to_string(),
            }],
        },
        is_preferred: true,
    }]
}

/// Fix deprecated 'defined @array' or 'defined %hash'
pub fn fix_deprecated_defined(source: &str, diagnostic: &Diagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Extract the array/hash from the diagnostic
    if let Some(start) = source[diagnostic.range.0..diagnostic.range.1].find("defined") {
        let defined_start = diagnostic.range.0 + start;
        let arg_start = defined_start + 7; // "defined".len()

        // Find the argument
        let arg_text = &source[arg_start..diagnostic.range.1].trim();

        actions.push(CodeAction {
            title: format!("Replace with 'if ({})'", arg_text),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec!["deprecated-defined".to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: defined_start, end: diagnostic.range.1 },
                    new_text: arg_text.to_string(),
                }],
            },
            is_preferred: true,
        });
    }

    actions
}

/// Fix numeric comparison with undef
pub fn fix_numeric_undef(source: &str, diagnostic: &Diagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Add defined check
    actions.push(CodeAction {
        title: "Add defined check".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec!["numeric-undef".to_string()],
        edit: CodeActionEdit {
            changes: vec![
                TextEdit {
                    location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.0 },
                    new_text: "defined(".to_string(),
                },
                TextEdit {
                    location: SourceLocation { start: diagnostic.range.1, end: diagnostic.range.1 },
                    new_text: ")".to_string(),
                },
            ],
        },
        is_preferred: true,
    });

    // Use // operator
    if source[diagnostic.range.0..diagnostic.range.1].contains("==") {
        actions.push(CodeAction {
            title: "Use defined-or operator (//)".to_string(),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec!["numeric-undef".to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                    new_text: "// 0".to_string(), // Default to 0
                }],
            },
            is_preferred: false,
        });
    }

    actions
}

/// Fix unquoted bareword by quoting or declaring as filehandle
///
/// Provides three options for fixing bareword issues under strict mode:
/// 1. Quote with single quotes - wraps bareword in single quotes
/// 2. Quote with double quotes - wraps bareword in double quotes
/// 3. Declare as filehandle - for uppercase barewords, adds filehandle declaration
pub fn fix_bareword(source: &str, diagnostic: &Diagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Extract bareword text from the source at the diagnostic range
    let bareword = &source[diagnostic.range.0..diagnostic.range.1];

    // Check if bareword is all uppercase (filehandle convention)
    let is_uppercase = bareword.chars().all(|c| c.is_ascii_uppercase() || c == '_');

    // Action 1: Quote with single quotes
    actions.push(CodeAction {
        title: format!("Quote '{}' with single quotes", bareword),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec!["unquoted-bareword".to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                new_text: format!("'{}'", bareword),
            }],
        },
        is_preferred: true,
    });

    // Action 2: Quote with double quotes
    actions.push(CodeAction {
        title: format!("Quote '{}' with double quotes", bareword),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec!["unquoted-bareword".to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                new_text: format!("\"{}\"", bareword),
            }],
        },
        is_preferred: false,
    });

    // Action 3: Declare as filehandle (only for uppercase barewords)
    if is_uppercase {
        // Find the best position to insert a filehandle declaration
        let insert_pos = find_declaration_position(source, diagnostic.range.0);
        let indent = get_indent_at(source, insert_pos);

        actions.push(CodeAction {
            title: format!("Declare '{}' as filehandle", bareword),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec!["unquoted-bareword".to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: insert_pos, end: insert_pos },
                    new_text: format!("{}open my ${};\n", indent, bareword),
                }],
            },
            is_preferred: false,
        });
    }

    actions
}

/// Fix parse errors with automated corrections
pub fn fix_parse_error(source: &str, diagnostic: &Diagnostic, code: &str) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    match code {
        "parse-error-missingsemicolon" => {
            // Add semicolon at the end
            let line_end = source[diagnostic.range.0..]
                .find('\n')
                .map(|p| diagnostic.range.0 + p)
                .unwrap_or(source.len());

            // Find the actual end of the statement (before any trailing whitespace)
            let mut end_pos = line_end;
            while end_pos > diagnostic.range.0
                && source.as_bytes()[end_pos - 1].is_ascii_whitespace()
            {
                end_pos -= 1;
            }

            actions.push(CodeAction {
                title: "Add missing semicolon".to_string(),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec![code.to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation { start: end_pos, end: end_pos },
                        new_text: ";".to_string(),
                    }],
                },
                is_preferred: true,
            });
        }
        "parse-error-unclosedstring" => {
            // Add closing quote
            actions.push(CodeAction {
                title: "Add closing quote".to_string(),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec![code.to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation {
                            start: diagnostic.range.1,
                            end: diagnostic.range.1,
                        },
                        new_text: "\"".to_string(),
                    }],
                },
                is_preferred: true,
            });
        }
        "parse-error-unclosedparenthesis" => {
            actions.push(CodeAction {
                title: "Add closing parenthesis".to_string(),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec![code.to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation {
                            start: diagnostic.range.1,
                            end: diagnostic.range.1,
                        },
                        new_text: ")".to_string(),
                    }],
                },
                is_preferred: true,
            });
        }
        "parse-error-unclosedbracket" => {
            actions.push(CodeAction {
                title: "Add closing bracket".to_string(),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec![code.to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation {
                            start: diagnostic.range.1,
                            end: diagnostic.range.1,
                        },
                        new_text: "]".to_string(),
                    }],
                },
                is_preferred: true,
            });
        }
        "parse-error-unclosedbrace" | "parse-error-unclosedblock" => {
            actions.push(CodeAction {
                title: "Add closing brace".to_string(),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec![code.to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation {
                            start: diagnostic.range.1,
                            end: diagnostic.range.1,
                        },
                        new_text: "}".to_string(),
                    }],
                },
                is_preferred: true,
            });
        }
        _ => {}
    }

    actions
}

/// Fix unused parameter by adding underscore prefix
pub fn fix_unused_parameter(diagnostic: &Diagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    if let Some(param_name) = diagnostic.message.split('\'').nth(1) {
        // Add underscore prefix
        actions.push(CodeAction {
            title: format!("Rename to '_{}'", param_name),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec!["unused-parameter".to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                    new_text: format!("_{}", param_name),
                }],
            },
            is_preferred: true,
        });
    }

    actions
}

/// Fix variable shadowing by suggesting rename
pub fn fix_variable_shadowing(diagnostic: &Diagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    if let Some(var_name) = diagnostic.message.split('\'').nth(1) {
        // Remove sigil for the base name
        let base_name =
            var_name.trim_start_matches('$').trim_start_matches('@').trim_start_matches('%');

        // Suggest alternative names
        let suggestions = vec![
            format!("{}_inner", base_name),
            format!("{}_local", base_name),
            format!("my_{}", base_name),
        ];

        for suggestion in suggestions {
            let new_name = if var_name.starts_with('$') {
                format!("${}", suggestion)
            } else if var_name.starts_with('@') {
                format!("@{}", suggestion)
            } else if var_name.starts_with('%') {
                format!("%{}", suggestion)
            } else {
                suggestion.clone()
            };

            actions.push(CodeAction {
                title: format!("Rename to '{}'", new_name),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec!["variable-shadowing".to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation {
                            start: diagnostic.range.0,
                            end: diagnostic.range.1,
                        },
                        new_text: new_name,
                    }],
                },
                is_preferred: false,
            });
        }
    }

    actions
}
