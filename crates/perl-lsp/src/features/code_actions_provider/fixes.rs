use crate::features::diagnostics::Diagnostic;

use super::{CodeAction, CodeActionKind, CodeActionsProvider, TextEdit, source_utils};

pub(super) fn fix_undefined_variable(
    provider: &CodeActionsProvider,
    diagnostic: &Diagnostic,
) -> Vec<CodeAction> {
    let Some(var_name) = source_utils::extract_quoted_value(&diagnostic.message) else {
        return Vec::new();
    };
    let insert_pos = source_utils::find_declaration_position(provider, diagnostic.range.0);

    vec![
        CodeAction {
            title: format!("Declare '{}' with 'my'", var_name),
            kind: CodeActionKind::QuickFix,
            edit: TextEdit {
                range: (insert_pos, insert_pos),
                new_text: format!("my {};\n", var_name),
            },
            diagnostic_id: diagnostic.code.clone(),
        },
        CodeAction {
            title: format!("Declare '{}' with 'our'", var_name),
            kind: CodeActionKind::QuickFix,
            edit: TextEdit {
                range: (insert_pos, insert_pos),
                new_text: format!("our {};\n", var_name),
            },
            diagnostic_id: diagnostic.code.clone(),
        },
    ]
}

pub(super) fn fix_unused_variable(
    provider: &CodeActionsProvider,
    diagnostic: &Diagnostic,
) -> Vec<CodeAction> {
    let Some(var_name) = source_utils::extract_quoted_value(&diagnostic.message) else {
        return Vec::new();
    };

    vec![
        CodeAction {
            title: format!("Remove unused variable '{}'", var_name),
            kind: CodeActionKind::QuickFix,
            edit: TextEdit {
                range: source_utils::find_declaration_range(
                    provider,
                    &var_name,
                    diagnostic.range.0,
                ),
                new_text: String::new(),
            },
            diagnostic_id: diagnostic.code.clone(),
        },
        CodeAction {
            title: format!(
                "Rename to '{}' (mark as intentionally unused)",
                source_utils::make_unused_name(&var_name)
            ),
            kind: CodeActionKind::QuickFix,
            edit: TextEdit {
                range: diagnostic.range,
                new_text: source_utils::make_unused_name(&var_name),
            },
            diagnostic_id: diagnostic.code.clone(),
        },
    ]
}

pub(super) fn fix_variable_shadowing(diagnostic: &Diagnostic) -> Vec<CodeAction> {
    let Some(var_name) = source_utils::extract_quoted_value(&diagnostic.message) else {
        return Vec::new();
    };
    let (sigil, base_name) = source_utils::split_sigil(&var_name);

    [
        format!("{}inner_{}", sigil, base_name),
        format!("{}local_{}", sigil, base_name),
        format!("{}{}_2", sigil, base_name),
    ]
    .into_iter()
    .map(|alt_name| CodeAction {
        title: format!("Rename shadowing variable to '{}'", alt_name),
        kind: CodeActionKind::QuickFix,
        edit: TextEdit { range: diagnostic.range, new_text: alt_name },
        diagnostic_id: diagnostic.code.clone(),
    })
    .collect()
}

pub(super) fn fix_variable_redeclaration(
    provider: &CodeActionsProvider,
    diagnostic: &Diagnostic,
) -> Vec<CodeAction> {
    let range = diagnostic.range;
    let text = &provider.source()[range.0..range.1];

    if text.starts_with("my ") {
        vec![CodeAction {
            title: "Remove redundant 'my'".to_string(),
            kind: CodeActionKind::QuickFix,
            edit: TextEdit { range: (range.0, range.0 + 3), new_text: String::new() },
            diagnostic_id: diagnostic.code.clone(),
        }]
    } else {
        Vec::new()
    }
}

pub(super) fn fix_parse_error(
    provider: &CodeActionsProvider,
    diagnostic: &Diagnostic,
    error_code: &str,
) -> Vec<CodeAction> {
    let action = match error_code {
        "parse-error-missingsemicolon" => CodeAction {
            title: "Add missing semicolon".to_string(),
            kind: CodeActionKind::QuickFix,
            edit: TextEdit {
                range: (
                    source_utils::find_line_end(provider, diagnostic.range.1),
                    source_utils::find_line_end(provider, diagnostic.range.1),
                ),
                new_text: ";".to_string(),
            },
            diagnostic_id: diagnostic.code.clone(),
        },
        "parse-error-unclosedstring" => {
            let quote_char = source_utils::detect_quote_char(provider, diagnostic.range.0);
            CodeAction {
                title: format!("Add closing quote '{}'", quote_char),
                kind: CodeActionKind::QuickFix,
                edit: TextEdit {
                    range: (diagnostic.range.1, diagnostic.range.1),
                    new_text: quote_char.to_string(),
                },
                diagnostic_id: diagnostic.code.clone(),
            }
        }
        "parse-error-unclosedparen" => CodeAction {
            title: "Add closing parenthesis".to_string(),
            kind: CodeActionKind::QuickFix,
            edit: TextEdit {
                range: (diagnostic.range.1, diagnostic.range.1),
                new_text: ")".to_string(),
            },
            diagnostic_id: diagnostic.code.clone(),
        },
        "parse-error-unclosedbrace" => CodeAction {
            title: "Add closing brace".to_string(),
            kind: CodeActionKind::QuickFix,
            edit: TextEdit {
                range: (diagnostic.range.1, diagnostic.range.1),
                new_text: "}".to_string(),
            },
            diagnostic_id: diagnostic.code.clone(),
        },
        _ => return Vec::new(),
    };

    vec![action]
}

pub(super) fn fix_duplicate_parameter(diagnostic: &Diagnostic) -> Vec<CodeAction> {
    let Some(param_name) = source_utils::extract_quoted_value(&diagnostic.message) else {
        return Vec::new();
    };
    let (sigil, base_name) = source_utils::split_sigil(&param_name);
    let new_name = format!("{}{}_2", sigil, base_name);

    vec![
        CodeAction {
            title: format!("Remove duplicate parameter '{}'", param_name),
            kind: CodeActionKind::QuickFix,
            edit: TextEdit { range: diagnostic.range, new_text: String::new() },
            diagnostic_id: diagnostic.code.clone(),
        },
        CodeAction {
            title: format!("Rename duplicate to '{}'", new_name),
            kind: CodeActionKind::QuickFix,
            edit: TextEdit { range: diagnostic.range, new_text: new_name },
            diagnostic_id: diagnostic.code.clone(),
        },
    ]
}

pub(super) fn fix_parameter_shadowing(diagnostic: &Diagnostic) -> Vec<CodeAction> {
    let Some(param_name) = source_utils::extract_quoted_value(&diagnostic.message) else {
        return Vec::new();
    };
    let (sigil, base_name) = source_utils::split_sigil(&param_name);

    [
        format!("{}p_{}", sigil, base_name),
        format!("{}{}_param", sigil, base_name),
        format!("{}{}_arg", sigil, base_name),
    ]
    .into_iter()
    .map(|alt_name| CodeAction {
        title: format!("Rename parameter to '{}'", alt_name),
        kind: CodeActionKind::QuickFix,
        edit: TextEdit { range: diagnostic.range, new_text: alt_name },
        diagnostic_id: diagnostic.code.clone(),
    })
    .collect()
}

pub(super) fn fix_unused_parameter(diagnostic: &Diagnostic) -> Vec<CodeAction> {
    let Some(param_name) = source_utils::extract_quoted_value(&diagnostic.message) else {
        return Vec::new();
    };
    let underscore_name = source_utils::make_unused_name(&param_name);

    vec![
        CodeAction {
            title: format!("Rename to '{}' (mark as intentionally unused)", underscore_name),
            kind: CodeActionKind::QuickFix,
            edit: TextEdit { range: diagnostic.range, new_text: underscore_name },
            diagnostic_id: diagnostic.code.clone(),
        },
        CodeAction {
            title: "Add comment explaining unused parameter".to_string(),
            kind: CodeActionKind::QuickFix,
            edit: TextEdit {
                range: (diagnostic.range.0, diagnostic.range.0),
                new_text: "# unused ".to_string(),
            },
            diagnostic_id: diagnostic.code.clone(),
        },
    ]
}

pub(super) fn fix_unquoted_bareword(
    provider: &CodeActionsProvider,
    diagnostic: &Diagnostic,
) -> Vec<CodeAction> {
    let Some(bareword) = source_utils::extract_quoted_value(&diagnostic.message) else {
        return Vec::new();
    };

    let mut actions = vec![
        CodeAction {
            title: format!("Quote bareword as '{}'", bareword),
            kind: CodeActionKind::QuickFix,
            edit: TextEdit { range: diagnostic.range, new_text: format!("'{}'", bareword) },
            diagnostic_id: diagnostic.code.clone(),
        },
        CodeAction {
            title: format!("Quote bareword as \"{}\"", bareword),
            kind: CodeActionKind::QuickFix,
            edit: TextEdit { range: diagnostic.range, new_text: format!("\"{}\"", bareword) },
            diagnostic_id: diagnostic.code.clone(),
        },
    ];

    if bareword.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
        let insert_pos = source_utils::find_declaration_position(provider, diagnostic.range.0);
        actions.push(CodeAction {
            title: format!("Declare {} as filehandle", bareword),
            kind: CodeActionKind::QuickFix,
            edit: TextEdit {
                range: (insert_pos, insert_pos),
                new_text: format!(
                    "open my ${}, '<', 'filename.txt' or die $!;\n",
                    bareword.to_lowercase()
                ),
            },
            diagnostic_id: diagnostic.code.clone(),
        });
    }

    actions
}
