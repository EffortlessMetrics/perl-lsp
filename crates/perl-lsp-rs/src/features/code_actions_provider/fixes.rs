//! Diagnostic-to-fix mapping logic for quick-fix code actions.

use crate::features::diagnostics::Diagnostic;

use super::{CodeAction, CodeActionKind, CodeActionsProvider, TextEdit, source_utils};

fn diagnostic_action(
    diagnostic: &Diagnostic,
    title: impl Into<String>,
    kind: CodeActionKind,
    edit: TextEdit,
) -> CodeAction {
    CodeAction {
        title: title.into(),
        kind,
        edit,
        diagnostic_id: diagnostic.code.clone(),
        diagnostic_range: Some(diagnostic.range),
    }
}

pub(super) fn fix_undefined_variable(
    provider: &CodeActionsProvider,
    diagnostic: &Diagnostic,
) -> Vec<CodeAction> {
    let Some(var_name) = source_utils::extract_quoted_value(&diagnostic.message) else {
        return Vec::new();
    };
    let insert_pos = source_utils::find_declaration_position(provider, diagnostic.range.0);

    vec![
        diagnostic_action(
            diagnostic,
            format!("Declare '{}' with 'my'", var_name),
            CodeActionKind::QuickFix,
            TextEdit { range: (insert_pos, insert_pos), new_text: format!("my {};\n", var_name) },
        ),
        diagnostic_action(
            diagnostic,
            format!("Declare '{}' with 'our'", var_name),
            CodeActionKind::QuickFix,
            TextEdit { range: (insert_pos, insert_pos), new_text: format!("our {};\n", var_name) },
        ),
    ]
}

pub(super) fn fix_unused_variable(
    provider: &CodeActionsProvider,
    diagnostic: &Diagnostic,
) -> Vec<CodeAction> {
    let Some(var_name) = source_utils::extract_quoted_value(&diagnostic.message) else {
        return Vec::new();
    };
    let unused_name = source_utils::make_unused_name(&var_name);
    let mut actions = Vec::new();

    if let Some(range) =
        source_utils::find_declaration_range(provider, &var_name, diagnostic.range.0)
    {
        actions.push(diagnostic_action(
            diagnostic,
            format!("Remove unused variable '{}'", var_name),
            CodeActionKind::QuickFix,
            TextEdit { range, new_text: String::new() },
        ));
    }

    actions.push(diagnostic_action(
        diagnostic,
        format!("Rename to '{}' (mark as intentionally unused)", unused_name),
        CodeActionKind::QuickFix,
        TextEdit { range: diagnostic.range, new_text: unused_name },
    ));

    actions
}

pub(super) fn fix_assignment_in_condition(
    provider: &CodeActionsProvider,
    diagnostic: &Diagnostic,
) -> Vec<CodeAction> {
    let Some(relative_pos) = provider.source()[diagnostic.range.0..diagnostic.range.1].find('=')
    else {
        return Vec::new();
    };
    let equals_pos = diagnostic.range.0 + relative_pos;

    vec![
        diagnostic_action(
            diagnostic,
            "Change to comparison (==)",
            CodeActionKind::QuickFix,
            TextEdit { range: (equals_pos, equals_pos + 1), new_text: "==".to_string() },
        ),
        diagnostic_action(
            diagnostic,
            "Keep assignment (add parentheses)",
            CodeActionKind::QuickFix,
            TextEdit {
                range: diagnostic.range,
                new_text: format!(
                    "({})",
                    &provider.source()[diagnostic.range.0..diagnostic.range.1]
                ),
            },
        ),
    ]
}

pub(super) fn add_use_strict(diagnostic: &Diagnostic) -> Vec<CodeAction> {
    vec![diagnostic_action(
        diagnostic,
        "Add 'use strict'",
        CodeActionKind::QuickFix,
        TextEdit { range: (0, 0), new_text: "use strict;\n".to_string() },
    )]
}

pub(super) fn add_use_warnings(diagnostic: &Diagnostic) -> Vec<CodeAction> {
    vec![diagnostic_action(
        diagnostic,
        "Add 'use warnings'",
        CodeActionKind::QuickFix,
        TextEdit { range: (0, 0), new_text: "use warnings;\n".to_string() },
    )]
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
    .map(|alt_name| {
        diagnostic_action(
            diagnostic,
            format!("Rename shadowing variable to '{}'", alt_name),
            CodeActionKind::QuickFix,
            TextEdit { range: diagnostic.range, new_text: alt_name },
        )
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
        vec![diagnostic_action(
            diagnostic,
            "Remove redundant 'my'",
            CodeActionKind::QuickFix,
            TextEdit { range: (range.0, range.0 + 3), new_text: String::new() },
        )]
    } else if let Some(my_range) = find_duplicate_my_span(provider.source(), range.0) {
        vec![diagnostic_action(
            diagnostic,
            "Remove redundant 'my'",
            CodeActionKind::QuickFix,
            TextEdit { range: my_range, new_text: String::new() },
        )]
    } else {
        Vec::new()
    }
}

fn find_duplicate_my_span(source: &str, variable_start: usize) -> Option<(usize, usize)> {
    let variable_start = variable_start.min(source.len());
    let line_start = source[..variable_start].rfind('\n').map_or(0, |pos| pos + 1);
    let before_var = &source[line_start..variable_start];
    let my_offset = before_var.rfind("my ")?;

    if before_var[my_offset + 3..].chars().all(char::is_whitespace) {
        let start = line_start + my_offset;
        Some((start, start + 3))
    } else {
        None
    }
}

pub(super) fn fix_parse_error(
    provider: &CodeActionsProvider,
    diagnostic: &Diagnostic,
    error_code: &str,
) -> Vec<CodeAction> {
    let action = match error_code {
        "parse-error-missingsemicolon" => diagnostic_action(
            diagnostic,
            "Add missing semicolon",
            CodeActionKind::QuickFix,
            TextEdit {
                range: (
                    source_utils::find_line_end(provider, diagnostic.range.1),
                    source_utils::find_line_end(provider, diagnostic.range.1),
                ),
                new_text: ";".to_string(),
            },
        ),
        "parse-error-unclosedstring" => {
            let quote_char = source_utils::detect_quote_char(provider, diagnostic.range.0);
            diagnostic_action(
                diagnostic,
                format!("Add closing quote '{}'", quote_char),
                CodeActionKind::QuickFix,
                TextEdit {
                    range: (diagnostic.range.1, diagnostic.range.1),
                    new_text: quote_char.to_string(),
                },
            )
        }
        "parse-error-unclosedparen" => diagnostic_action(
            diagnostic,
            "Add closing parenthesis",
            CodeActionKind::QuickFix,
            TextEdit { range: (diagnostic.range.1, diagnostic.range.1), new_text: ")".to_string() },
        ),
        "parse-error-unclosedbrace" => diagnostic_action(
            diagnostic,
            "Add closing brace",
            CodeActionKind::QuickFix,
            TextEdit { range: (diagnostic.range.1, diagnostic.range.1), new_text: "}".to_string() },
        ),
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
        diagnostic_action(
            diagnostic,
            format!("Remove duplicate parameter '{}'", param_name),
            CodeActionKind::QuickFix,
            TextEdit { range: diagnostic.range, new_text: String::new() },
        ),
        diagnostic_action(
            diagnostic,
            format!("Rename duplicate to '{}'", new_name),
            CodeActionKind::QuickFix,
            TextEdit { range: diagnostic.range, new_text: new_name },
        ),
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
    .map(|alt_name| {
        diagnostic_action(
            diagnostic,
            format!("Rename parameter to '{}'", alt_name),
            CodeActionKind::QuickFix,
            TextEdit { range: diagnostic.range, new_text: alt_name },
        )
    })
    .collect()
}

pub(super) fn fix_unused_parameter(diagnostic: &Diagnostic) -> Vec<CodeAction> {
    let Some(param_name) = source_utils::extract_quoted_value(&diagnostic.message) else {
        return Vec::new();
    };
    let underscore_name = source_utils::make_unused_name(&param_name);

    vec![diagnostic_action(
        diagnostic,
        format!("Rename to '{}' (mark as intentionally unused)", underscore_name),
        CodeActionKind::QuickFix,
        TextEdit { range: diagnostic.range, new_text: underscore_name },
    )]
}

pub(super) fn fix_unquoted_bareword(
    provider: &CodeActionsProvider,
    diagnostic: &Diagnostic,
) -> Vec<CodeAction> {
    let Some(bareword) = source_utils::extract_quoted_value(&diagnostic.message) else {
        return Vec::new();
    };

    let mut actions = vec![
        diagnostic_action(
            diagnostic,
            format!("Quote bareword as '{}'", bareword),
            CodeActionKind::QuickFix,
            TextEdit { range: diagnostic.range, new_text: format!("'{}'", bareword) },
        ),
        diagnostic_action(
            diagnostic,
            format!("Quote bareword as \"{}\"", bareword),
            CodeActionKind::QuickFix,
            TextEdit { range: diagnostic.range, new_text: format!("\"{}\"", bareword) },
        ),
    ];

    if bareword.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
        let insert_pos = source_utils::find_declaration_position(provider, diagnostic.range.0);
        actions.push(diagnostic_action(
            diagnostic,
            format!("Declare {} as filehandle", bareword),
            CodeActionKind::QuickFix,
            TextEdit {
                range: (insert_pos, insert_pos),
                new_text: format!(
                    "open my ${}, '<', 'filename.txt' or die $!;\n",
                    bareword.to_lowercase()
                ),
            },
        ));
    }

    actions
}

pub(super) fn fix_bareword_filehandle(diagnostic: &Diagnostic) -> Vec<CodeAction> {
    let Some(handle_name) = source_utils::extract_quoted_value(&diagnostic.message) else {
        return Vec::new();
    };
    let lexical_name = format!("${}_fh", handle_name.to_lowercase());

    vec![diagnostic_action(
        diagnostic,
        format!("Replace bareword filehandle '{}' with lexical '{}'", handle_name, lexical_name),
        CodeActionKind::QuickFix,
        TextEdit { range: diagnostic.range, new_text: format!("my {lexical_name}") },
    )]
}
