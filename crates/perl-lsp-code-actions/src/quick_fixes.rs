//! Quick fixes for diagnostic issues
//!
//! Provides automated fixes for common Perl issues driven by diagnostic codes.

use crate::types::{CodeAction, CodeActionEdit, CodeActionKind, QuickFixDiagnostic};
use perl_ast_utils::{find_declaration_position, get_indent_at};
use perl_diagnostics_codes::DiagnosticCode;
use perl_lsp_rename::TextEdit;
use perl_parser_core::SourceLocation;

/// Fix undefined variable by declaring it
pub fn fix_undefined_variable(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Extract variable name from diagnostic message
    if let Some(var_name) = diagnostic.message.split('\'').nth(1) {
        // Find the best place to insert declaration
        let insert_pos = find_declaration_position(source, diagnostic.range.0);

        // Add 'my' declaration
        actions.push(CodeAction {
            title: format!("Declare '{}' with 'my'", var_name),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::UndefinedVariable.as_str().to_string()],
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
            diagnostics: vec![DiagnosticCode::UndefinedVariable.as_str().to_string()],
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
pub fn fix_unused_variable(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
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
        diagnostics: vec![DiagnosticCode::UnusedVariable.as_str().to_string()],
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
            diagnostics: vec![DiagnosticCode::UnusedVariable.as_str().to_string()],
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
pub fn fix_assignment_in_condition(
    source: &str,
    diagnostic: &QuickFixDiagnostic,
) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Change = to ==
    let assignment_pos =
        source[diagnostic.range.0..diagnostic.range.1].find('=').map(|p| diagnostic.range.0 + p);

    if let Some(pos) = assignment_pos {
        actions.push(CodeAction {
            title: "Change to comparison (==)".to_string(),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::AssignmentInCondition.as_str().to_string()],
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
            diagnostics: vec![DiagnosticCode::AssignmentInCondition.as_str().to_string()],
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
        diagnostics: vec![DiagnosticCode::MissingStrict.as_str().to_string()],
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
        diagnostics: vec![DiagnosticCode::MissingWarnings.as_str().to_string()],
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
pub fn fix_deprecated_defined(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
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
            diagnostics: vec![DiagnosticCode::DeprecatedDefined.as_str().to_string()],
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
pub fn fix_numeric_undef(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Add defined check
    actions.push(CodeAction {
        title: "Add defined check".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::NumericComparisonWithUndef.as_str().to_string()],
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
            diagnostics: vec![DiagnosticCode::NumericComparisonWithUndef.as_str().to_string()],
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
pub fn fix_bareword(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Extract bareword text from the source at the diagnostic range
    let bareword = &source[diagnostic.range.0..diagnostic.range.1];

    // Check if bareword is all uppercase (filehandle convention)
    let is_uppercase = bareword.chars().all(|c| c.is_ascii_uppercase() || c == '_');

    // Action 1: Quote with single quotes
    actions.push(CodeAction {
        title: format!("Quote '{}' with single quotes", bareword),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::UnquotedBareword.as_str().to_string()],
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
        diagnostics: vec![DiagnosticCode::UnquotedBareword.as_str().to_string()],
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
            diagnostics: vec![DiagnosticCode::UnquotedBareword.as_str().to_string()],
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
pub fn fix_parse_error(
    source: &str,
    diagnostic: &QuickFixDiagnostic,
    code: &str,
) -> Vec<CodeAction> {
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
        "PL001" | "PL002"
            if diagnostic.message.to_ascii_lowercase().contains("missing semicolon") =>
        {
            // PL001/PL002 are general parse error codes. When the message indicates a missing
            // semicolon, apply the same fix — but skip heredoc contexts where insertion is wrong.
            let at_heredoc = source[diagnostic.range.0..].get(..2).is_some_and(|s| s == "<<");
            if !at_heredoc {
                let line_end = source[diagnostic.range.0..]
                    .find('\n')
                    .map(|p| diagnostic.range.0 + p)
                    .unwrap_or(source.len());

                // Insert before trailing whitespace
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
pub fn fix_unused_parameter(diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    if let Some(param_name) = diagnostic.message.split('\'').nth(1) {
        // Add underscore prefix
        actions.push(CodeAction {
            title: format!("Rename to '_{}'", param_name),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::UnusedParameter.as_str().to_string()],
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

/// Suggest portable shebang line
///
/// Detects hardcoded perl paths in shebang lines (e.g., `#!/usr/bin/perl`,
/// `#!/usr/local/bin/perl`) and suggests replacing with `#!/usr/bin/env perl`
/// for better portability across systems.
///
/// Only triggers on the first line of the file when it starts with `#!` and
/// contains a path to perl that is not already using `env`.
pub fn fix_hardcoded_shebang(source: &str) -> Vec<CodeAction> {
    let first_line = match source.lines().next() {
        Some(line) => line,
        None => return Vec::new(),
    };

    // Must be a shebang line
    if !first_line.starts_with("#!") {
        return Vec::new();
    }

    // Already portable
    if first_line.contains("/env ") || first_line.contains("/env\t") {
        return Vec::new();
    }

    // Must reference perl
    if !first_line.contains("perl") {
        return Vec::new();
    }

    // Extract any flags after the perl path (e.g., -w, -T)
    let flags = extract_shebang_flags(first_line);
    let new_shebang = if flags.is_empty() {
        "#!/usr/bin/env perl".to_string()
    } else {
        format!("#!/usr/bin/env perl {}", flags)
    };

    vec![CodeAction {
        title: "Use portable shebang (#!/usr/bin/env perl)".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec!["hardcoded-shebang".to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: 0, end: first_line.len() },
                new_text: new_shebang,
            }],
        },
        is_preferred: true,
    }]
}

/// Extract flags from a shebang line (e.g., `-w` from `#!/usr/bin/perl -w`)
fn extract_shebang_flags(shebang_line: &str) -> String {
    // Find "perl" in the line, then grab everything after it
    if let Some(perl_pos) = shebang_line.find("perl") {
        let after_perl = &shebang_line[perl_pos + 4..];
        let trimmed = after_perl.trim();
        if trimmed.is_empty() { String::new() } else { trimmed.to_string() }
    } else {
        String::new()
    }
}

/// Fix variable shadowing by suggesting rename
pub fn fix_variable_shadowing(diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
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
                diagnostics: vec![DiagnosticCode::VariableShadowing.as_str().to_string()],
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

/// Fix bareword filehandle by replacing with lexical filehandle
///
/// Bareword filehandles (e.g., `open FILE, ...`) are a common Perl anti-pattern.
/// This fix suggests replacing the bareword with a lexical variable (`my $fh`).
pub fn fix_bareword_filehandle(diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    // Extract filehandle name from message, e.g. "Bareword filehandle 'FILE'"
    let fh_name = diagnostic.message.split('\'').nth(1).unwrap_or("FH");
    // Derive a lowercase lexical name: FILE -> $file_fh, LOGFILE -> $logfile_fh
    let lexical_name = format!("${}_fh", fh_name.to_lowercase());

    vec![CodeAction {
        title: format!("Replace bareword filehandle '{}' with lexical '{}'", fh_name, lexical_name),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::BarewordFilehandle.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                new_text: format!("my {}", lexical_name),
            }],
        },
        is_preferred: true,
    }]
}

/// Fix missing package declaration by inserting `package main;` at the top
///
/// When a Perl file has no `package` declaration (PL200), the default package
/// is `main`. This fix makes that intent explicit by inserting `package main;`
/// at the top of the file.
pub fn fix_missing_package_declaration(source: &str) -> Vec<CodeAction> {
    // Insert after shebang if present, otherwise at top
    let insert_pos =
        if source.starts_with("#!") { source.find('\n').map(|p| p + 1).unwrap_or(0) } else { 0 };

    vec![CodeAction {
        title: "Add 'package main;' declaration".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::MissingPackageDeclaration.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: insert_pos, end: insert_pos },
                new_text: "package main;\n".to_string(),
            }],
        },
        is_preferred: true,
    }]
}

/// Fix variable redeclaration by removing the duplicate `my` keyword
///
/// When a variable is declared twice in the same scope (PL105), the fix
/// is to remove the `my` keyword from the second declaration, turning it
/// into a plain assignment.
pub fn fix_variable_redeclaration(
    source: &str,
    diagnostic: &QuickFixDiagnostic,
) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    if let Some((abs_my_start, abs_my_end)) = find_duplicate_my_span(source, diagnostic) {
        // Remove only the duplicate declarator and keep the assignment/value intact.

        actions.push(CodeAction {
            title: "Remove duplicate 'my' declaration".to_string(),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::VariableRedeclaration.as_str().to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: abs_my_start, end: abs_my_end },
                    new_text: String::new(),
                }],
            },
            is_preferred: true,
        });
    }

    actions
}

fn find_duplicate_my_span(source: &str, diagnostic: &QuickFixDiagnostic) -> Option<(usize, usize)> {
    let variable_start = diagnostic.range.0.min(source.len());
    let line_start = source[..variable_start].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
    let before_var = &source[line_start..variable_start];
    let my_offset = before_var.rfind("my ")?;

    if before_var[my_offset + 3..].chars().all(char::is_whitespace) {
        let start = line_start + my_offset;
        return Some((start, start + 3));
    }

    None
}

/// Fix misspelled pragma by replacing with the correctly spelled name
///
/// The MisspelledPragma diagnostic (PL111) message has the format:
/// `"Did you mean 'use <correct>;'? '<typo>' is not a known pragma"`
/// This fix extracts the correct name and replaces the entire `use <typo>` statement.
pub fn fix_misspelled_pragma(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Parse correct pragma from message: "Did you mean 'use <correct>;'?"
    let msg = &diagnostic.message;
    if let Some(after_use) = msg.strip_prefix("Did you mean 'use ")
        && let Some(correct_name) = after_use.split(';').next()
    {
        let correct_pragma = correct_name.trim();
        actions.push(CodeAction {
            title: format!("Fix pragma spelling: 'use {};'", correct_pragma),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec![DiagnosticCode::MisspelledPragma.as_str().to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                    new_text: format!("use {};", correct_pragma),
                }],
            },
            is_preferred: true,
        });
    }

    // Unused parameter suppression: source is used indirectly through the
    // diagnostic message which was produced from the same source text.
    let _ = source;

    actions
}

/// Fix unreachable code by removing the unreachable statement
///
/// PL406 fires when a statement follows an unconditional exit (return, die, exit).
/// The fix removes the entire line containing the unreachable statement.
pub fn fix_unreachable_code(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    // Find the full line containing the unreachable statement
    let line_start = source[..diagnostic.range.0].rfind('\n').map(|p| p + 1).unwrap_or(0);
    let line_end = source[diagnostic.range.1..]
        .find('\n')
        .map(|p| diagnostic.range.1 + p + 1)
        .unwrap_or(source.len());

    vec![CodeAction {
        title: "Remove unreachable code".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::UnreachableCode.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: line_start, end: line_end },
                new_text: String::new(),
            }],
        },
        is_preferred: true,
    }]
}

/// Fix duplicate subroutine by suggesting rename of the second definition
///
/// PL300 fires when a subroutine is defined more than once. The fix renames the
/// second definition to avoid the conflict, preserving both implementations.
pub fn fix_duplicate_subroutine(diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Extract sub name from message: "Subroutine 'foo' is defined more than once..."
    let sub_name = diagnostic.message.split('\'').nth(1).unwrap_or("sub");

    actions.push(CodeAction {
        title: format!("Rename duplicate subroutine '{}' to '{}_2'", sub_name, sub_name),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::DuplicateSubroutine.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                new_text: format!("{}_2", sub_name),
            }],
        },
        is_preferred: true,
    });

    actions
}

/// Fix missing return statement by adding an explicit `return` before the closing brace
///
/// PL301 fires when a subroutine has no explicit return statement. The diagnostic
/// range covers the subroutine body. This inserts `return;` at the end of the range.
pub fn fix_missing_return(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    // Find indentation to match surrounding code style
    let insert_pos = diagnostic.range.1.min(source.len());
    let indent = get_indent_at(source, insert_pos.saturating_sub(1));

    vec![CodeAction {
        title: "Add explicit 'return' statement".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::MissingReturn.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: insert_pos, end: insert_pos },
                new_text: format!("{}return;\n", indent),
            }],
        },
        is_preferred: true,
    }]
}

/// Suggest upgrading two-argument open() to three-argument form
///
/// Two-argument `open($fh, $filename)` is unsafe because `$filename` can
/// contain shell metacharacters. The three-argument form separates the mode
/// from the filename, e.g. `open(my $fh, '<', $filename)`.
pub fn fix_two_arg_open(diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    vec![CodeAction {
        title: "Convert to three-argument open() for safety".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec![DiagnosticCode::TwoArgOpen.as_str().to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: diagnostic.range.0, end: diagnostic.range.1 },
                new_text: "open(my $fh, '<', $filename)".to_string(),
            }],
        },
        is_preferred: true,
    }]
}
