//! Error-to-action mapping for common Perl mistakes
//!
//! Maps generic `parse-error` and `syntax-error` diagnostics to actionable
//! quick-fix code actions by analyzing the diagnostic message text and
//! surrounding source context.
//!
//! This bridges the gap between classified parse errors (which already have
//! specific `parse-error-*` codes handled by [`crate::quick_fixes`]) and
//! generic error diagnostics that lack a fine-grained code.

use crate::types::{CodeAction, CodeActionEdit, CodeActionKind, QuickFixDiagnostic};
use perl_lsp_rename::TextEdit;
use perl_parser_core::SourceLocation;

/// Common Perl builtin misspellings mapped to the correct spelling.
///
/// Each entry is `(typo, correct_builtin)`.
const BUILTIN_TYPOS: &[(&str, &str)] = &[
    // print family
    ("pritn", "print"),
    ("pirnt", "print"),
    ("prnt", "print"),
    ("pring", "print"),
    ("prtin", "print"),
    ("rpint", "print"),
    ("prnit", "print"),
    ("println", "print"),
    ("pintrf", "printf"),
    ("pintf", "printf"),
    // say
    ("sya", "say"),
    ("asy", "say"),
    // chomp/chop
    ("chom", "chomp"),
    ("chmp", "chomp"),
    ("chompp", "chomp"),
    ("comp", "chomp"),
    ("choop", "chop"),
    ("chp", "chop"),
    // push/pop/shift/unshift
    ("psuh", "push"),
    ("psh", "push"),
    ("puhs", "push"),
    ("poop", "pop"),
    ("shfit", "shift"),
    ("shif", "shift"),
    ("sihft", "shift"),
    ("unshfit", "unshift"),
    ("unshif", "unshift"),
    // split/join
    ("spilt", "split"),
    ("splti", "split"),
    ("slpit", "split"),
    ("splt", "split"),
    ("jion", "join"),
    ("joing", "join"),
    // sort/reverse/map/grep
    ("srot", "sort"),
    ("srt", "sort"),
    ("sor", "sort"),
    ("reveres", "reverse"),
    ("reveser", "reverse"),
    ("revsere", "reverse"),
    ("mpap", "map"),
    ("mpa", "map"),
    ("gep", "grep"),
    ("grp", "grep"),
    ("gerp", "grep"),
    // open/close
    ("opne", "open"),
    ("oepn", "open"),
    ("cloze", "close"),
    ("clsoe", "close"),
    ("colse", "close"),
    // defined/exists/delete
    ("defind", "defined"),
    ("defineed", "defined"),
    ("deffined", "defined"),
    ("exsits", "exists"),
    ("exsts", "exists"),
    ("exisst", "exists"),
    ("delte", "delete"),
    ("deleet", "delete"),
    ("dleete", "delete"),
    // die/warn
    ("dei", "die"),
    ("ide", "die"),
    ("wanr", "warn"),
    ("warrn", "warn"),
    ("wrn", "warn"),
    // length/substr/index
    ("legnth", "length"),
    ("lenght", "length"),
    ("lentgh", "length"),
    ("lengh", "length"),
    ("subtr", "substr"),
    ("subsrt", "substr"),
    ("sbstr", "substr"),
    ("idnex", "index"),
    ("indx", "index"),
    // keys/values/each
    ("kyes", "keys"),
    ("kesy", "keys"),
    ("valeus", "values"),
    ("vlaues", "values"),
    ("eahc", "each"),
    ("aech", "each"),
    // require/use
    ("requrie", "require"),
    ("reuqire", "require"),
    ("requre", "require"),
    // declarations
    ("myu", "my"),
    ("oure", "our"),
    ("locla", "local"),
    ("lcaol", "local"),
    ("loacl", "local"),
    ("sbu", "sub"),
    ("reutrn", "return"),
    ("retrun", "return"),
    ("retunr", "return"),
    ("rteurn", "return"),
    // control flow
    ("whiel", "while"),
    ("whle", "while"),
    ("untl", "until"),
    ("untli", "until"),
    ("forech", "foreach"),
    ("foreahc", "foreach"),
    ("freack", "foreach"),
    ("unles", "unless"),
    ("unlses", "unless"),
    ("els", "else"),
    ("eslif", "elsif"),
    ("elseif", "elsif"),
    ("eleif", "elsif"),
];

/// Generate code actions for a generic `parse-error` or `syntax-error` diagnostic.
///
/// Analyzes the diagnostic message text to determine the likely error type and
/// suggests appropriate fixes such as adding a missing semicolon or closing
/// a delimiter.
pub fn actions_for_error_diagnostic(
    source: &str,
    diagnostic: &QuickFixDiagnostic,
) -> Vec<CodeAction> {
    let msg = diagnostic.message.to_lowercase();
    let mut actions = Vec::new();

    // Missing semicolon heuristic
    if (msg.contains("semicolon") || msg.contains("missing ;") || msg.contains("expected ;"))
        && let Some(action) = suggest_add_semicolon(source, diagnostic)
    {
        actions.push(action);
    }

    // Unclosed string heuristic
    if msg.contains("unclosed")
        && (msg.contains("string") || msg.contains("quote"))
        && let Some(action) = suggest_close_string(source, diagnostic)
    {
        actions.push(action);
    }

    // Unclosed paren/bracket/brace
    if (msg.contains("unclosed") || msg.contains("unmatched") || msg.contains("missing"))
        && let Some(action) = suggest_close_delimiter(source, diagnostic, &msg)
    {
        actions.push(action);
    }

    // EOF / unexpected end
    if msg.contains("unexpected end") || msg.contains("eof") || msg.contains("end of file") {
        actions.extend(suggest_eof_fix(source, diagnostic));
    }

    actions
}

/// Generate "Did you mean...?" code actions for builtin typos found in the
/// diagnostic message or source context.
pub fn actions_for_builtin_typo(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let candidates = extract_identifier_candidates(source, diagnostic);
    let mut actions = Vec::new();

    for candidate in &candidates {
        if let Some((_typo, builtin)) = find_builtin_match(candidate) {
            // Find position of the candidate (original case) in source near the diagnostic range
            let search_start = diagnostic.range.0.saturating_sub(50);
            let search_end = (diagnostic.range.1 + 50).min(source.len());
            let search_region = &source[search_start..search_end];

            if let Some(offset) = search_region.find(candidate.as_str()) {
                let abs_start = search_start + offset;
                let abs_end = abs_start + candidate.len();

                actions.push(CodeAction {
                    title: format!("Did you mean '{}'?", builtin),
                    kind: CodeActionKind::QuickFix,
                    diagnostics: vec!["parse-error".to_string()],
                    edit: CodeActionEdit {
                        changes: vec![TextEdit {
                            location: SourceLocation { start: abs_start, end: abs_end },
                            new_text: builtin.to_string(),
                        }],
                    },
                    is_preferred: true,
                });
                break; // One typo fix per diagnostic
            }
        }
    }

    actions
}

/// Suggest adding a missing semicolon at the end of the statement.
fn suggest_add_semicolon(source: &str, diagnostic: &QuickFixDiagnostic) -> Option<CodeAction> {
    let line_end = source[diagnostic.range.0..]
        .find('\n')
        .map(|p| diagnostic.range.0 + p)
        .unwrap_or(source.len());

    // Trim trailing whitespace to find statement end
    let mut end_pos = line_end;
    while end_pos > diagnostic.range.0
        && source.as_bytes().get(end_pos - 1).copied().is_some_and(|b| b.is_ascii_whitespace())
    {
        end_pos -= 1;
    }

    // Don't add semicolon if there's already one
    if end_pos > 0 && source.as_bytes().get(end_pos - 1).copied() == Some(b';') {
        return None;
    }

    Some(CodeAction {
        title: "Add missing semicolon".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec!["parse-error".to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: end_pos, end: end_pos },
                new_text: ";".to_string(),
            }],
        },
        is_preferred: true,
    })
}

/// Suggest closing an unclosed string literal.
fn suggest_close_string(source: &str, diagnostic: &QuickFixDiagnostic) -> Option<CodeAction> {
    let quote = detect_open_quote(source, diagnostic.range.0);
    let close_char = match quote {
        Some('\'') => '\'',
        Some('"') => '"',
        Some('`') => '`',
        _ => '"', // default
    };

    Some(CodeAction {
        title: format!("Add closing {}", close_char),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec!["parse-error".to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: diagnostic.range.1, end: diagnostic.range.1 },
                new_text: close_char.to_string(),
            }],
        },
        is_preferred: true,
    })
}

/// Suggest closing a delimiter (paren, bracket, brace).
fn suggest_close_delimiter(
    source: &str,
    diagnostic: &QuickFixDiagnostic,
    msg: &str,
) -> Option<CodeAction> {
    let (title, close) = if msg.contains("parenthes") || msg.contains("paren") {
        ("Add closing parenthesis", ")")
    } else if msg.contains("bracket") {
        ("Add closing bracket", "]")
    } else if msg.contains("brace") || msg.contains("block") {
        ("Add closing brace", "}")
    } else {
        return None;
    };

    // Don't duplicate if the exact closing char is already at the insert point
    if diagnostic.range.1 < source.len()
        && source.as_bytes().get(diagnostic.range.1).copied() == Some(close.as_bytes()[0])
    {
        return None;
    }

    Some(CodeAction {
        title: title.to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: vec!["parse-error".to_string()],
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: diagnostic.range.1, end: diagnostic.range.1 },
                new_text: close.to_string(),
            }],
        },
        is_preferred: true,
    })
}

/// Suggest fixes for unexpected EOF errors.
fn suggest_eof_fix(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<CodeAction> {
    let mut actions = Vec::new();
    let msg = diagnostic.message.to_lowercase();

    // Check for unclosed heredoc
    if msg.contains("heredoc")
        && let Some(marker) = find_heredoc_marker(source, source.len())
    {
        actions.push(CodeAction {
            title: format!("Add heredoc terminator '{}'", marker),
            kind: CodeActionKind::QuickFix,
            diagnostics: vec!["parse-error".to_string()],
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start: source.len(), end: source.len() },
                    new_text: format!("\n{}\n", marker),
                }],
            },
            is_preferred: true,
        });
    }

    // Generic EOF -- suggest semicolon
    if actions.is_empty() {
        // Trim trailing whitespace from source end
        let mut end = source.len();
        while end > 0
            && source.as_bytes().get(end - 1).copied().is_some_and(|b| b.is_ascii_whitespace())
        {
            end -= 1;
        }

        if end > 0 && source.as_bytes().get(end - 1).copied() != Some(b';') {
            actions.push(CodeAction {
                title: "Add missing semicolon before end of file".to_string(),
                kind: CodeActionKind::QuickFix,
                diagnostics: vec!["parse-error".to_string()],
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation { start: end, end },
                        new_text: ";".to_string(),
                    }],
                },
                is_preferred: false,
            });
        }
    }

    actions
}

/// Detect which quote character opened an unclosed string near `pos`.
fn detect_open_quote(source: &str, pos: usize) -> Option<char> {
    let search_start = pos.saturating_sub(200);
    let region = &source[search_start..pos.min(source.len())];

    // Walk backwards from the diagnostic looking for an unmatched quote
    let mut single_count = 0i32;
    let mut double_count = 0i32;

    for ch in region.chars().rev() {
        match ch {
            '\'' => single_count += 1,
            '"' => double_count += 1,
            _ => {}
        }
    }

    // Odd count means unclosed
    if single_count % 2 != 0 {
        Some('\'')
    } else if double_count % 2 != 0 {
        Some('"')
    } else {
        None
    }
}

/// Try to find the heredoc marker name from source preceding `pos`.
fn find_heredoc_marker(source: &str, pos: usize) -> Option<String> {
    let search_start = pos.saturating_sub(500);
    let region = &source[search_start..pos.min(source.len())];

    // Look for <<MARKER, <<"MARKER", <<'MARKER', <<~MARKER patterns
    let mut idx = 0;
    let bytes = region.as_bytes();
    let mut last_marker = None;

    while idx + 2 < bytes.len() {
        if bytes[idx] == b'<' && bytes[idx + 1] == b'<' {
            idx += 2;
            // skip optional ~
            if idx < bytes.len() && bytes[idx] == b'~' {
                idx += 1;
            }
            // skip optional quote
            if idx < bytes.len()
                && (bytes[idx] == b'"' || bytes[idx] == b'\'' || bytes[idx] == b'\\')
            {
                idx += 1;
            }
            // collect marker name
            let marker_start = idx;
            while idx < bytes.len() && (bytes[idx].is_ascii_alphanumeric() || bytes[idx] == b'_') {
                idx += 1;
            }
            if idx > marker_start {
                let marker = String::from_utf8_lossy(&bytes[marker_start..idx]).to_string();
                last_marker = Some(marker);
            }
        } else {
            idx += 1;
        }
    }

    last_marker
}

/// Extract identifier-like tokens near the diagnostic range.
fn extract_identifier_candidates(source: &str, diagnostic: &QuickFixDiagnostic) -> Vec<String> {
    let search_start = diagnostic.range.0.saturating_sub(50);
    let search_end = (diagnostic.range.1 + 50).min(source.len());
    let region = &source[search_start..search_end];

    let mut candidates = Vec::new();
    let mut current = String::new();

    for ch in region.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            current.push(ch);
        } else {
            if current.len() >= 2 {
                candidates.push(current.clone());
            }
            current.clear();
        }
    }
    if current.len() >= 2 {
        candidates.push(current);
    }

    candidates
}

/// Find a builtin match for a candidate identifier.
fn find_builtin_match(candidate: &str) -> Option<(&'static str, &'static str)> {
    let lower = candidate.to_lowercase();
    for &(typo, builtin) in BUILTIN_TYPOS {
        if lower == typo && lower != builtin {
            return Some((typo, builtin));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_qf(start: usize, end: usize, msg: &str) -> QuickFixDiagnostic {
        QuickFixDiagnostic {
            range: (start, end),
            message: msg.to_string(),
            code: Some("parse-error".to_string()),
        }
    }

    // --- Semicolon tests ---

    #[test]
    fn test_missing_semicolon_from_message() {
        let source = "my $x = 1\nmy $y = 2;";
        let diag = make_qf(0, 9, "Missing semicolon");
        let actions = actions_for_error_diagnostic(source, &diag);
        assert!(
            actions.iter().any(|a| a.title.contains("semicolon")),
            "Expected semicolon action, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_no_duplicate_semicolon() {
        let source = "my $x = 1;";
        let diag = make_qf(0, 10, "Missing semicolon");
        let actions = actions_for_error_diagnostic(source, &diag);
        let semicolon_actions: Vec<_> =
            actions.iter().filter(|a| a.title.contains("semicolon")).collect();
        assert!(
            semicolon_actions.is_empty(),
            "Should not suggest duplicate semicolon, got: {:?}",
            semicolon_actions
        );
    }

    // --- Unclosed string tests ---

    #[test]
    fn test_unclosed_double_quote() {
        let source = r#"my $x = "hello"#;
        let diag = make_qf(8, 14, "Unclosed string");
        let actions = actions_for_error_diagnostic(source, &diag);
        assert!(
            actions.iter().any(|a| a.title.contains("closing")),
            "Expected closing quote action, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_unclosed_single_quote() {
        let source = "my $x = 'hello";
        let diag = make_qf(8, 14, "Unclosed string");
        let actions = actions_for_error_diagnostic(source, &diag);
        assert!(
            actions.iter().any(|a| a.title.contains("closing")),
            "Expected closing quote action, got: {:?}",
            actions
        );
    }

    // --- Delimiter tests ---

    #[test]
    fn test_unclosed_parenthesis() {
        let source = "print(1 + 2";
        let diag = make_qf(5, 11, "Unclosed parenthesis");
        let actions = actions_for_error_diagnostic(source, &diag);
        assert!(
            actions.iter().any(|a| a.title.contains("parenthesis")),
            "Expected close paren action, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_unclosed_bracket() {
        let source = "my @a = [1, 2";
        let diag = make_qf(8, 13, "Unclosed bracket");
        let actions = actions_for_error_diagnostic(source, &diag);
        assert!(
            actions.iter().any(|a| a.title.contains("bracket")),
            "Expected close bracket action, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_unclosed_brace() {
        let source = "if (1) {";
        let diag = make_qf(7, 8, "Unclosed brace");
        let actions = actions_for_error_diagnostic(source, &diag);
        assert!(
            actions.iter().any(|a| a.title.contains("brace")),
            "Expected close brace action, got: {:?}",
            actions
        );
    }

    // --- EOF tests ---

    #[test]
    fn test_unexpected_eof_adds_semicolon() {
        let source = "my $x = 1";
        let diag = make_qf(0, 9, "Unexpected end of file");
        let actions = actions_for_error_diagnostic(source, &diag);
        assert!(
            actions.iter().any(|a| a.title.contains("semicolon")),
            "Expected semicolon action for EOF, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_eof_heredoc() {
        let source = "my $text = <<EOF;\nsome text\n";
        let diag = make_qf(0, source.len(), "Unexpected end of file, unclosed heredoc");
        let actions = actions_for_error_diagnostic(source, &diag);
        assert!(
            actions.iter().any(|a| a.title.contains("EOF")),
            "Expected heredoc terminator action, got: {:?}",
            actions
        );
    }

    // --- Builtin typo tests ---

    #[test]
    fn test_builtin_typo_pritn() {
        let source = "pritn \"hello\";";
        let diag = make_qf(0, 5, "Bareword 'pritn' not allowed");
        let actions = actions_for_builtin_typo(source, &diag);
        assert!(
            actions.iter().any(|a| a.title.contains("print")),
            "Expected 'Did you mean print?' action, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_builtin_typo_chom() {
        let source = "chom($line);";
        let diag = make_qf(0, 4, "Bareword 'chom' not allowed");
        let actions = actions_for_builtin_typo(source, &diag);
        assert!(
            actions.iter().any(|a| a.title.contains("chomp")),
            "Expected 'Did you mean chomp?' action, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_builtin_typo_case_insensitive() {
        let source = "PRITN \"hello\";";
        let diag = make_qf(0, 5, "Bareword 'PRITN' not allowed");
        let actions = actions_for_builtin_typo(source, &diag);
        assert!(
            actions.iter().any(|a| a.title.contains("print")),
            "Expected case-insensitive typo match, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_no_typo_for_correct_builtin() {
        let source = "print \"hello\";";
        let diag = make_qf(0, 5, "Something about print");
        let actions = actions_for_builtin_typo(source, &diag);
        assert!(
            actions.is_empty(),
            "Should not suggest typo fix for correct builtin, got: {:?}",
            actions
        );
    }

    #[test]
    fn test_builtin_typo_spilt() {
        let source = "my @parts = spilt(/,/, $str);";
        let diag = make_qf(12, 17, "Bareword 'spilt' not allowed");
        let actions = actions_for_builtin_typo(source, &diag);
        assert!(
            actions.iter().any(|a| a.title.contains("split")),
            "Expected 'Did you mean split?' action, got: {:?}",
            actions
        );
    }

    // --- Edge case tests ---

    #[test]
    fn test_empty_source() {
        let source = "";
        let diag = make_qf(0, 0, "Unexpected end of file");
        let actions = actions_for_error_diagnostic(source, &diag);
        // Should not panic on empty source
        assert!(actions.is_empty() || !actions.is_empty());
    }

    #[test]
    fn test_empty_source_typo() {
        let source = "";
        let diag = make_qf(0, 0, "Error");
        let actions = actions_for_builtin_typo(source, &diag);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_elseif_typo() {
        let source = "if (1) { } elseif (2) { }";
        let diag = make_qf(11, 17, "Bareword 'elseif' not allowed");
        let actions = actions_for_builtin_typo(source, &diag);
        assert!(
            actions.iter().any(|a| a.title.contains("elsif")),
            "Expected 'Did you mean elsif?' action, got: {:?}",
            actions
        );
    }
}
