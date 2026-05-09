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

pub(super) fn fix_deprecated_defined(
    provider: &CodeActionsProvider,
    diagnostic: &Diagnostic,
) -> Vec<CodeAction> {
    let diagnostic_text = &provider.source()[diagnostic.range.0..diagnostic.range.1];
    let Some(relative_start) = diagnostic_text.find("defined") else {
        return Vec::new();
    };

    let defined_start = diagnostic.range.0 + relative_start;
    let raw_arg = provider.source()[defined_start + "defined".len()..diagnostic.range.1].trim();
    let arg_text = normalize_deprecated_defined_arg(raw_arg);
    if arg_text.is_empty() {
        return Vec::new();
    }

    vec![diagnostic_action(
        diagnostic,
        format!("Replace with '{arg_text}'"),
        CodeActionKind::QuickFix,
        TextEdit { range: (defined_start, diagnostic.range.1), new_text: arg_text.to_string() },
    )]
}

fn normalize_deprecated_defined_arg(raw_arg: &str) -> &str {
    raw_arg
        .strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .map(str::trim)
        .unwrap_or(raw_arg)
}

pub(super) fn fix_native_undef_comparison(
    provider: &CodeActionsProvider,
    diagnostic: &Diagnostic,
) -> Vec<CodeAction> {
    let Some(replacement) = native_undef_comparison_replacement(
        &provider.source()[diagnostic.range.0..diagnostic.range.1],
    ) else {
        return Vec::new();
    };

    vec![diagnostic_action(
        diagnostic,
        "Use defined() check",
        CodeActionKind::QuickFix,
        TextEdit { range: diagnostic.range, new_text: replacement },
    )]
}

fn native_undef_comparison_replacement(text: &str) -> Option<String> {
    if let Some((left, right)) = text.split_once("==") {
        return native_defined_replacement(left, right, true);
    }
    if let Some((left, right)) = text.split_once("!=") {
        return native_defined_replacement(left, right, false);
    }

    None
}

fn native_defined_replacement(left: &str, right: &str, equal: bool) -> Option<String> {
    let left = left.trim();
    let right = right.trim();
    let compared = if left == "undef" {
        right
    } else if right == "undef" {
        left
    } else {
        return None;
    };
    if compared.is_empty() {
        return None;
    }

    let replacement =
        if equal { format!("!defined({compared})") } else { format!("defined({compared})") };
    Some(replacement)
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

pub(super) fn fix_two_arg_open(
    provider: &CodeActionsProvider,
    diagnostic: &Diagnostic,
) -> Vec<CodeAction> {
    let Some((range, new_text)) = two_arg_open_replacement(provider.source(), diagnostic.range)
    else {
        return Vec::new();
    };

    vec![diagnostic_action(
        diagnostic,
        "Convert to three-argument open() for safety",
        CodeActionKind::QuickFix,
        TextEdit { range, new_text },
    )]
}

fn two_arg_open_replacement(
    source: &str,
    range: (usize, usize),
) -> Option<((usize, usize), String)> {
    let snippet = source.get(range.0..range.1)?;
    if let Some(((start, end), new_text)) = parse_two_arg_open_call(snippet) {
        return Some(((range.0 + start, range.0 + end), new_text));
    }

    let start = range.0.min(source.len());
    let line_start = source[..start].rfind('\n').map_or(0, |idx| idx + 1);
    let line_end = source[start..].find('\n').map_or(source.len(), |offset| start + offset);
    let diagnostic_offset = start.saturating_sub(line_start);
    source.get(start..line_end).and_then(parse_two_arg_open_call).map(
        |((call_start, call_end), new_text)| {
            (
                (
                    line_start + diagnostic_offset + call_start,
                    line_start + diagnostic_offset + call_end,
                ),
                new_text,
            )
        },
    )
}

fn parse_two_arg_open_call(snippet: &str) -> Option<((usize, usize), String)> {
    let call_start = first_non_whitespace(snippet)?;
    let call = &snippet[call_start..];
    let after_open = call.strip_prefix("open")?;
    let next = after_open.chars().next()?;
    if !next.is_whitespace() && next != '(' {
        return None;
    }

    let body_start = call_start + "open".len();
    let body_start = body_start + first_non_whitespace(&snippet[body_start..])?;
    let body = &snippet[body_start..];

    let (args, call_end) = if body.starts_with('(') {
        let close = find_matching_parenthesis(body)?;
        if has_non_statement_trailing_text(&body[close + 1..]) {
            return None;
        }
        (&body[1..close], body_start + close + 1)
    } else {
        let args_end = bare_call_args_end(body)?;
        (&body[..args_end], body_start + args_end)
    };
    let (handle, path) = split_two_top_level_args(args)?;

    Some(((call_start, call_end), format!("open({}, '<', {})", handle.trim(), path.trim())))
}

fn first_non_whitespace(input: &str) -> Option<usize> {
    input.char_indices().find_map(|(idx, ch)| (!ch.is_whitespace()).then_some(idx))
}

fn has_non_statement_trailing_text(input: &str) -> bool {
    let trimmed = input.trim_start();
    if trimmed.is_empty() {
        return false;
    }

    let Some(after_semicolon) = trimmed.strip_prefix(';') else {
        return true;
    };

    !after_semicolon.trim().is_empty()
}

fn bare_call_args_end(input: &str) -> Option<usize> {
    if input.trim().is_empty() {
        return None;
    }

    find_statement_semicolon(input).map_or_else(
        || {
            if contains_unquoted_comment(input) { None } else { Some(input.trim_end().len()) }
        },
        |semicolon| {
            let trailing = input[semicolon + 1..].trim();
            if trailing.is_empty() && !contains_unquoted_comment(&input[..semicolon]) {
                Some(input[..semicolon].trim_end().len())
            } else {
                None
            }
        },
    )
}

fn find_statement_semicolon(input: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;

    for (idx, ch) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if let Some(quote_char) = quote {
            if ch == '\\' {
                escaped = true;
            } else if ch == quote_char {
                quote = None;
            }
            continue;
        }

        match ch {
            '\'' | '"' | '`' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ';' if depth == 0 => return Some(idx),
            _ => {}
        }
    }

    None
}

fn contains_unquoted_comment(input: &str) -> bool {
    let mut quote = None;
    let mut escaped = false;

    for ch in input.chars() {
        if escaped {
            escaped = false;
            continue;
        }

        if let Some(quote_char) = quote {
            if ch == '\\' {
                escaped = true;
            } else if ch == quote_char {
                quote = None;
            }
            continue;
        }

        match ch {
            '\'' | '"' | '`' => quote = Some(ch),
            '#' => return true,
            _ => {}
        }
    }

    false
}

fn find_matching_parenthesis(input: &str) -> Option<usize> {
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut quote = None;
    let mut escaped = false;

    for (idx, ch) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if let Some(quote_char) = quote {
            if ch == '\\' {
                escaped = true;
            } else if ch == quote_char {
                quote = None;
            }
            continue;
        }

        match ch {
            '\'' | '"' | '`' => quote = Some(ch),
            '(' => paren_depth += 1,
            '[' => bracket_depth += 1,
            '{' => brace_depth += 1,
            ')' if paren_depth == 1 && bracket_depth == 0 && brace_depth == 0 => return Some(idx),
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '}' => brace_depth = brace_depth.saturating_sub(1),
            _ => {}
        }
    }

    None
}

fn split_two_top_level_args(input: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    let mut split = None;

    for (idx, ch) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if let Some(quote_char) = quote {
            if ch == '\\' {
                escaped = true;
            } else if ch == quote_char {
                quote = None;
            }
            continue;
        }

        match ch {
            '\'' | '"' | '`' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                if split.replace(idx).is_some() {
                    return None;
                }
            }
            _ => {}
        }
    }

    let idx = split?;
    let first = &input[..idx];
    let second = &input[idx + 1..];

    if first.trim().is_empty() || second.trim().is_empty() {
        return None;
    }

    Some((first, second))
}
