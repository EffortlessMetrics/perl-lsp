//! Perl modernization code actions
//!
//! Scans source text for legacy Perl patterns and suggests modern replacements.
//! Registered as `source.modernize.perl` code action kind.

use super::types::{CodeAction, CodeActionEdit, CodeActionKind};
use crate::providers::rename::TextEdit;
use perl_parser_core::SourceLocation;

/// Scan source for modernization opportunities and return code actions.
pub fn get_modernize_actions(source: &str) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    actions.extend(find_two_arg_open(source));
    actions.extend(find_deprecated_defined(source));
    actions.extend(find_legacy_require_version(source));
    actions.extend(find_missing_strict_warnings(source));
    actions.extend(find_die_in_module(source));

    actions
}

/// Detect two-argument `open` calls and suggest three-argument form.
fn find_two_arg_open(source: &str) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    for (line_idx, line) in source.lines().enumerate() {
        let trimmed = line.trim();

        if !trimmed.starts_with("open") {
            continue;
        }

        let after_open = &trimmed[4..];
        let content = if after_open.starts_with('(') {
            after_open.trim_start_matches('(').trim_end_matches(");").trim_end_matches(')').trim()
        } else if after_open.starts_with(' ') || after_open.starts_with('\t') {
            after_open.trim().trim_end_matches(';').trim()
        } else {
            continue;
        };

        let comma_count = count_commas_outside_quotes(content);

        if comma_count != 1 {
            continue;
        }

        if let Some((filehandle, mode_file)) = split_at_first_comma(content) {
            let filehandle = filehandle.trim();
            let mode_file = mode_file.trim().trim_matches('"').trim_matches('\'');

            let (mode, filename) = extract_mode_and_filename(mode_file);

            let modern_open = if filehandle.starts_with("my ") || filehandle.starts_with('$') {
                format!(
                    "open({}, \"{}\", \"{}\") or die \"Cannot open {}: $!\"",
                    filehandle, mode, filename, filename
                )
            } else {
                let lc_handle = filehandle.to_lowercase();
                format!(
                    "open(my ${}, \"{}\", \"{}\") or die \"Cannot open {}: $!\"",
                    lc_handle, mode, filename, filename
                )
            };

            let line_start = line_start_offset(source, line_idx);
            let line_end = line_start + line.len();

            actions.push(CodeAction {
                title: "Modernize: use three-arg open with error handling".to_string(),
                kind: CodeActionKind::SourceModernize,
                diagnostics: Vec::new(),
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation { start: line_start, end: line_end },
                        new_text: format!(
                            "{}{}",
                            &line[..line.len() - line.trim_start().len()],
                            modern_open
                        ),
                    }],
                },
                is_preferred: false,
            });
        }
    }

    actions
}

/// Detect `defined(@array)` and `defined(%hash)` which are deprecated since v5.22.
fn find_deprecated_defined(source: &str) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    let mut search_from = 0;
    while let Some(pos) = source[search_from..].find("defined") {
        let abs_pos = search_from + pos;

        if abs_pos > 0 {
            let prev = source.as_bytes()[abs_pos - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' {
                search_from = abs_pos + 7;
                continue;
            }
        }

        let after = &source[abs_pos + 7..];
        let after_trimmed = after.trim_start();
        let has_paren = after_trimmed.starts_with('(');
        let inner = if has_paren {
            after_trimmed.trim_start_matches('(').trim_start()
        } else {
            after_trimmed
        };

        if inner.starts_with('@') || inner.starts_with('%') {
            let sigil = if inner.starts_with('@') { '@' } else { '%' };
            let var_end = inner[1..]
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .map(|p| p + 1)
                .unwrap_or(inner.len());
            let var_name = &inner[..var_end];

            let expr_end = if has_paren {
                let paren_start = abs_pos + 7 + (after.len() - after_trimmed.len()) + 1;
                let close = source[paren_start..].find(')').map(|p| paren_start + p + 1);
                close.unwrap_or(abs_pos + 7 + var_end + 2)
            } else {
                abs_pos + 7 + (after.len() - after_trimmed.len()) + var_end
            };

            actions.push(CodeAction {
                title: format!("Modernize: remove deprecated defined({}) (since v5.22)", sigil),
                kind: CodeActionKind::SourceModernize,
                diagnostics: Vec::new(),
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation { start: abs_pos, end: expr_end },
                        new_text: var_name.to_string(),
                    }],
                },
                is_preferred: false,
            });
        }

        search_from = abs_pos + 7;
    }

    actions
}

/// Detect `require 5.006` and suggest `use v5.6`.
fn find_legacy_require_version(source: &str) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    for (line_idx, line) in source.lines().enumerate() {
        let trimmed = line.trim();

        if !trimmed.starts_with("require ") {
            continue;
        }

        let after_require = trimmed[8..].trim().trim_end_matches(';').trim();

        if !after_require.starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }

        if let Some(modern_version) = modernize_version(after_require) {
            let line_start = line_start_offset(source, line_idx);
            let line_end = line_start + line.len();
            let indent = &line[..line.len() - trimmed.len()];

            actions.push(CodeAction {
                title: format!(
                    "Modernize: use {} instead of require {}",
                    modern_version, after_require
                ),
                kind: CodeActionKind::SourceModernize,
                diagnostics: Vec::new(),
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: SourceLocation { start: line_start, end: line_end },
                        new_text: format!("{}use {};", indent, modern_version),
                    }],
                },
                is_preferred: false,
            });
        }
    }

    actions
}

/// Detect missing `use strict` / `use warnings` and suggest adding both.
fn find_missing_strict_warnings(source: &str) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    let has_strict = source.contains("use strict");
    let has_warnings = source.contains("use warnings");

    if has_strict && has_warnings {
        return actions;
    }

    let implicit_strict = [
        "use Moo",
        "use Moose",
        "use Mouse",
        "use Dancer2",
        "use Mojolicious",
        "use Catalyst",
        "use Modern::Perl",
        "use common::sense",
        "use Mojo::Base",
        "use v5.12",
        "use v5.14",
        "use v5.16",
        "use v5.18",
        "use v5.20",
        "use v5.22",
        "use v5.24",
        "use v5.26",
        "use v5.28",
        "use v5.30",
        "use v5.32",
        "use v5.34",
        "use v5.36",
        "use v5.38",
        "use v5.40",
    ];

    for pattern in &implicit_strict {
        if source.contains(pattern) {
            return actions;
        }
    }

    let insert_pos = find_pragma_insert_pos(source);

    let mut missing = Vec::new();
    if !has_strict {
        missing.push("use strict;");
    }
    if !has_warnings {
        missing.push("use warnings;");
    }

    let new_text = format!("{}\n", missing.join("\n"));

    actions.push(CodeAction {
        title: format!("Modernize: add {}", missing.join(" and ")),
        kind: CodeActionKind::SourceModernize,
        diagnostics: Vec::new(),
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: insert_pos, end: insert_pos },
                new_text,
            }],
        },
        is_preferred: false,
    });

    actions
}

/// Detect bare `die` calls in module files and suggest upgrading to `Carp::croak`.
///
/// Only fires when the source contains a `package` declaration (i.e. a module, not
/// a script). The `or die` and `|| die` idioms used for system-call error handling
/// are explicitly excluded — those are idiomatic and correct as-is.
///
/// Multi-line forms are handled: a `die` on a line by itself is also skipped when
/// the previous non-empty line ends in `or` or `||` (e.g. `open(...) or\n    die`).
fn find_die_in_module(source: &str) -> Vec<CodeAction> {
    // Only relevant in module files
    if !source.contains("package ") {
        return Vec::new();
    }

    let already_uses_carp = source.contains("use Carp");
    let mut actions = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Skip `or die` and `|| die` idioms — correct for system calls.
        // Only match these patterns in executable code (not inside strings/comments).
        if contains_or_die_idiom(line) {
            continue;
        }

        // Match bare die call at start of trimmed line (space, paren, or semicolon after "die")
        if !trimmed.starts_with("die ")
            && !trimmed.starts_with("die;")
            && !trimmed.starts_with("die(")
        {
            continue;
        }

        // Skip multi-line `or die` / `|| die` where `or` or `||` is at end of previous line
        if line_idx > 0 {
            if line_ends_with_or_operator(lines[line_idx - 1]) {
                continue;
            }
        }

        let line_start = line_start_offset(source, line_idx);
        let indent_len = line.len() - trimmed.len();
        let die_start = line_start + indent_len;
        let die_end = die_start + 3; // len("die")

        let mut changes = vec![TextEdit {
            location: SourceLocation { start: die_start, end: die_end },
            new_text: "croak".to_string(),
        }];

        if !already_uses_carp {
            let insert_pos = find_pragma_insert_pos(source);
            changes.push(TextEdit {
                location: SourceLocation { start: insert_pos, end: insert_pos },
                new_text: "use Carp qw(croak);\n".to_string(),
            });
        }

        actions.push(CodeAction {
            title: "Use Carp::croak instead of die in modules".to_string(),
            kind: CodeActionKind::SourceModernize,
            diagnostics: Vec::new(),
            edit: CodeActionEdit { changes },
            is_preferred: false,
        });
    }

    actions
}

// ---- helpers ----------------------------------------------------------------

fn strip_strings_and_comments(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for ch in line.chars() {
        if in_single {
            if ch == '\'' && !escaped {
                in_single = false;
            }
            escaped = ch == '\\' && !escaped;
            out.push(' ');
            continue;
        }

        if in_double {
            if ch == '"' && !escaped {
                in_double = false;
            }
            escaped = ch == '\\' && !escaped;
            out.push(' ');
            continue;
        }

        escaped = false;

        if ch == '#' {
            break;
        }
        if ch == '\'' {
            in_single = true;
            out.push(' ');
            continue;
        }
        if ch == '"' {
            in_double = true;
            out.push(' ');
            continue;
        }

        out.push(ch);
    }

    out
}

fn is_identifier_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

fn has_die_word_at(bytes: &[u8], idx: usize) -> bool {
    if idx + 3 > bytes.len() || &bytes[idx..idx + 3] != b"die" {
        return false;
    }

    if idx > 0 && is_identifier_char(bytes[idx - 1]) {
        return false;
    }

    if idx + 3 < bytes.len() && is_identifier_char(bytes[idx + 3]) {
        return false;
    }

    true
}

fn contains_or_die_idiom(line: &str) -> bool {
    let stripped = strip_strings_and_comments(line);
    let bytes = stripped.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // `|| die`
        if i + 2 <= bytes.len() && &bytes[i..i + 2] == b"||" {
            let mut j = i + 2;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if has_die_word_at(bytes, j) {
                return true;
            }
        }

        // `or die` with token boundaries
        if i + 2 <= bytes.len() && &bytes[i..i + 2] == b"or" {
            if (i == 0 || !is_identifier_char(bytes[i - 1]))
                && (i + 2 == bytes.len() || !is_identifier_char(bytes[i + 2]))
            {
                let mut j = i + 2;
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if has_die_word_at(bytes, j) {
                    return true;
                }
            }
        }

        i += 1;
    }

    false
}

fn line_ends_with_or_operator(line: &str) -> bool {
    let stripped = strip_strings_and_comments(line);
    matches!(stripped.split_whitespace().last(), Some("or" | "||"))
}

fn count_commas_outside_quotes(s: &str) -> usize {
    let mut count = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut prev = '\0';

    for ch in s.chars() {
        match ch {
            '\'' if !in_double && prev != '\\' => in_single = !in_single,
            '"' if !in_single && prev != '\\' => in_double = !in_double,
            ',' if !in_single && !in_double => count += 1,
            _ => {}
        }
        prev = ch;
    }

    count
}

fn split_at_first_comma(s: &str) -> Option<(&str, &str)> {
    let mut in_single = false;
    let mut in_double = false;
    let mut prev = '\0';

    for (i, ch) in s.char_indices() {
        match ch {
            '\'' if !in_double && prev != '\\' => in_single = !in_single,
            '"' if !in_single && prev != '\\' => in_double = !in_double,
            ',' if !in_single && !in_double => {
                return Some((&s[..i], &s[i + 1..]));
            }
            _ => {}
        }
        prev = ch;
    }

    None
}

fn extract_mode_and_filename(s: &str) -> (&str, &str) {
    if let Some(rest) = s.strip_prefix(">>") {
        (">>", rest)
    } else if let Some(rest) = s.strip_prefix('>') {
        (">", rest)
    } else if let Some(rest) = s.strip_prefix('<') {
        ("<", rest)
    } else if let Some(rest) = s.strip_prefix("+<") {
        ("+<", rest)
    } else if let Some(rest) = s.strip_prefix("+>") {
        ("+>", rest)
    } else {
        ("<", s)
    }
}

fn modernize_version(ver: &str) -> Option<String> {
    if ver.starts_with('v') {
        return None;
    }

    let parts: Vec<&str> = ver.split('.').collect();
    match parts.len() {
        1 => Some(format!("v{}", parts[0])),
        2 => {
            let minor = parts[1].trim_start_matches('0');
            let minor = if minor.is_empty() { "0" } else { minor };
            Some(format!("v{}.{}", parts[0], minor))
        }
        3 => Some(format!("v{}.{}.{}", parts[0], parts[1], parts[2])),
        _ => None,
    }
}

fn line_start_offset(source: &str, line_idx: usize) -> usize {
    let mut offset = 0;
    for (i, line) in source.lines().enumerate() {
        if i == line_idx {
            return offset;
        }
        offset += line.len() + 1;
    }
    offset
}

fn find_pragma_insert_pos(source: &str) -> usize {
    let mut pos = 0;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#!") || trimmed.is_empty() {
            pos += line.len() + 1;
        } else if trimmed.starts_with("package ") {
            pos += line.len() + 1;
            break;
        } else {
            break;
        }
    }

    if pos > source.len() {
        pos = source.len();
    }

    pos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_arg_open_detected() {
        let source = r#"open(FILE, ">output.txt");"#;
        let actions = get_modernize_actions(source);
        assert!(
            actions.iter().any(|a| a.title.contains("three-arg open")),
            "Expected three-arg open suggestion, got: {:?}",
            actions.iter().map(|a| &a.title).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_three_arg_open_not_flagged() {
        let source = r#"open(my $fh, ">", "output.txt");"#;
        let actions = find_two_arg_open(source);
        assert!(actions.is_empty(), "Three-arg open should not trigger");
    }

    #[test]
    fn test_deprecated_defined_array() {
        let source = "if (defined(@array)) { }";
        let actions = get_modernize_actions(source);
        assert!(
            actions.iter().any(|a| a.title.contains("deprecated defined(@")),
            "Expected deprecated defined(@) action"
        );
    }

    #[test]
    fn test_deprecated_defined_hash() {
        let source = "if (defined(%hash)) { }";
        let actions = get_modernize_actions(source);
        assert!(actions.iter().any(|a| a.title.contains("deprecated defined(%")));
    }

    #[test]
    fn test_defined_scalar_not_flagged() {
        let source = "if (defined($x)) { }";
        let actions = find_deprecated_defined(source);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_require_version_to_use() {
        let source = "require 5.006;";
        let actions = get_modernize_actions(source);
        assert!(actions.iter().any(|a| a.title.contains("use v5.6")));
    }

    #[test]
    fn test_require_version_5010() {
        let source = "require 5.010;";
        let actions = get_modernize_actions(source);
        assert!(actions.iter().any(|a| a.title.contains("use v5.10")));
    }

    #[test]
    fn test_require_module_not_flagged() {
        let source = "require Foo::Bar;";
        let actions = find_legacy_require_version(source);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_missing_strict_warnings() {
        let source = "print 'hello';";
        let actions = get_modernize_actions(source);
        assert!(actions.iter().any(|a| a.title.contains("use strict")));
    }

    #[test]
    fn test_strict_warnings_present_no_action() {
        let source = "use strict;\nuse warnings;\nprint 'hello';";
        let actions = find_missing_strict_warnings(source);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_moose_implies_strict() {
        let source = "use Moose;\nprint 'hello';";
        let actions = find_missing_strict_warnings(source);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_all_actions_have_modernize_kind() {
        let source = "require 5.006;\nopen(FILE, \">foo\");\nif (defined(@arr)) {}";
        let actions = get_modernize_actions(source);
        for action in &actions {
            assert_eq!(action.kind, CodeActionKind::SourceModernize);
        }
    }

    #[test]
    fn test_modernize_version_conversion() {
        assert_eq!(modernize_version("5.006"), Some("v5.6".to_string()));
        assert_eq!(modernize_version("5.010"), Some("v5.10".to_string()));
        assert_eq!(modernize_version("5.6.1"), Some("v5.6.1".to_string()));
        assert_eq!(modernize_version("v5.10"), None);
    }

    #[test]
    fn test_extract_mode_and_filename() {
        assert_eq!(extract_mode_and_filename(">foo"), (">", "foo"));
        assert_eq!(extract_mode_and_filename(">>log"), (">>", "log"));
        assert_eq!(extract_mode_and_filename("<input"), ("<", "input"));
        assert_eq!(extract_mode_and_filename("data.txt"), ("<", "data.txt"));
    }

    // ---- find_die_in_module tests -------------------------------------------

    #[test]
    fn test_die_in_module_suggests_croak() {
        let source = "package Foo;\nuse strict;\ndie \"Something failed\";\n";
        let actions = get_modernize_actions(source);
        assert!(
            actions.iter().any(|a| a.title.contains("croak")),
            "Expected croak suggestion in module context, got: {:?}",
            actions.iter().map(|a| &a.title).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_die_in_script_not_flagged() {
        let source = "#!/usr/bin/env perl\nuse strict;\ndie \"Something failed\";\n";
        let actions = find_die_in_module(source);
        assert!(actions.is_empty(), "die in script should not suggest croak");
    }

    #[test]
    fn test_or_die_not_flagged() {
        let source = "package Foo;\nopen(my $fh, '<', 'f') or die \"open failed: $!\";\n";
        let actions = find_die_in_module(source);
        assert!(actions.is_empty(), "or die idiom should not be flagged");
    }

    #[test]
    fn test_pipe_die_not_flagged() {
        let source = "package Foo;\nopen(my $fh, '<', 'f') || die \"open failed: $!\";\n";
        let actions = find_die_in_module(source);
        assert!(actions.is_empty(), "|| die idiom should not be flagged");
    }

    #[test]
    fn test_die_in_module_inserts_use_carp() {
        let source = "package Foo;\nuse strict;\ndie \"oops\";\n";
        let actions = find_die_in_module(source);
        assert!(!actions.is_empty(), "Expected croak action for die in module");
        let action = &actions[0];
        assert_eq!(action.edit.changes.len(), 2, "Should emit die->croak edit AND use Carp insert");
        assert!(
            action.edit.changes.iter().any(|e| e.new_text.contains("use Carp")),
            "One change should insert use Carp"
        );
    }

    #[test]
    fn test_die_in_module_already_uses_carp_no_duplicate() {
        let source = "package Foo;\nuse Carp qw(croak);\ndie \"oops\";\n";
        let actions = find_die_in_module(source);
        assert!(!actions.is_empty(), "Expected croak action even when Carp already used");
        let action = &actions[0];
        assert_eq!(
            action.edit.changes.len(),
            1,
            "Should only emit die->croak, no duplicate use Carp"
        );
    }

    #[test]
    fn test_die_in_module_action_kind_is_modernize() {
        let source = "package Foo;\ndie \"oops\";\n";
        let actions = find_die_in_module(source);
        assert!(!actions.is_empty());
        assert_eq!(actions[0].kind, CodeActionKind::SourceModernize);
    }

    #[test]
    fn test_die_with_or_die_text_in_message_still_flagged() {
        // Edge case: quoted message text containing "or die" should still be flagged.
        let source = "package Foo;\ndie \"or die trying harder\";\n";
        let actions = find_die_in_module(source);
        assert!(!actions.is_empty(), "die even with 'or die' in message gets flagged (correct)");
    }

    #[test]
    fn test_die_with_space_or_die_in_message_is_flagged() {
        // "or die" inside a quoted message should not suppress the modernization action.
        let source = "package Foo;\ndie \"message: or die trying\";\n";
        let actions = find_die_in_module(source);
        assert!(
            !actions.is_empty(),
            "quoted 'or die' text should not be treated as an or-die idiom"
        );
    }

    #[test]
    fn test_die_with_or_die_only_in_comment_is_flagged() {
        let source = "package Foo;\ndie \"message\"; # or die fallback\n";
        let actions = find_die_in_module(source);
        assert!(!actions.is_empty(), "comment text should not suppress bare die detection");
    }

    #[test]
    fn test_die_with_parens_flagged() {
        // die("msg") — parenthesised form — must be flagged like die "msg"
        let source = "package Foo;\ndie(\"Something failed\");\n";
        let actions = find_die_in_module(source);
        assert!(
            !actions.is_empty(),
            "die(\"msg\") in module context should be flagged for croak upgrade"
        );
    }

    #[test]
    fn test_multiline_or_die_not_flagged() {
        // Multi-line `or die`: `or` at end of previous line, `die` on its own line.
        // The die should NOT be flagged — it is part of an `or die` idiom.
        let source = "package Foo;\nopen(my $fh, '<', 'f') or\n    die \"open failed: $!\";\n";
        let actions = find_die_in_module(source);
        assert!(
            actions.is_empty(),
            "die on its own line after trailing `or` should not be flagged as bare die"
        );
    }

    #[test]
    fn test_multiline_pipe_die_not_flagged() {
        // Multi-line `|| die`: `||` at end of previous line, `die` on its own line.
        let source = "package Foo;\nopen(my $fh, '<', 'f') ||\n    die \"open failed: $!\";\n";
        let actions = find_die_in_module(source);
        assert!(
            actions.is_empty(),
            "die on its own line after trailing `||` should not be flagged as bare die"
        );
    }

    #[test]
    fn test_multiple_dies_produce_independent_actions() {
        // When a module has two bare dies and Carp is not yet imported, each action
        // is self-contained with its own die->croak edit and use Carp insertion.
        // LSP applies actions one at a time; after the first apply the file has
        // use Carp and the second invocation would emit only the die->croak edit.
        let source = "package Foo;\nuse strict;\ndie \"first error\";\ndie \"second error\";\n";
        let actions = find_die_in_module(source);
        assert_eq!(actions.len(), 2, "Two bare dies should produce two actions");
        // Each action is self-contained: both include a use Carp insertion
        for action in &actions {
            assert_eq!(
                action.edit.changes.len(),
                2,
                "Each action should have die->croak and use Carp insertion when Carp not present"
            );
        }
    }

    #[test]
    fn test_use_carp_heavy_treated_as_carp_present() {
        // use Carp::Heavy contains "use Carp" as a substring; already_uses_carp = true.
        // Only the die->croak edit should be emitted, no duplicate Carp insertion.
        let source = "package Foo;\nuse Carp::Heavy;\ndie \"oops\";\n";
        let actions = find_die_in_module(source);
        assert!(!actions.is_empty(), "die in module with Carp::Heavy should still suggest croak");
        assert_eq!(
            actions[0].edit.changes.len(),
            1,
            "use Carp::Heavy counts as Carp present; no new use Carp should be inserted"
        );
    }
}
