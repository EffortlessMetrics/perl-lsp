#![warn(missing_docs)]
//! On-type formatting provider for Perl LSP.
//!
//! Provides automatic indentation and formatting when typing trigger characters
//! (`}`, `;`, `\n`). All formatting is suppressed inside heredoc bodies to avoid
//! corrupting heredoc content.

use serde_json::Value;
use serde_json::json;

/// Computes on-type formatting edits for a Perl document based on character input.
///
/// Handles special characters (`}`, `;`, newlines) to provide automatic indentation
/// and formatting adjustments. Returns a vector of text edits to apply, or `None` if no
/// edits are needed for the given character.
///
/// # Trigger semantics
///
/// - **`}`** — Re-indents the closing brace to match the indentation of its
///   corresponding opening `{`.
/// - **`;`** — No change to indentation (the line keeps its existing indent).
/// - **`\n`** — Sets the indentation of the new line based on the previous line:
///   increases after `{`, decreases after `}`.
///
/// # Heredoc suppression
///
/// When the cursor falls inside a heredoc body, all on-type formatting is
/// suppressed to avoid corrupting heredoc content.
pub fn compute_on_type_edit(text: &str, line: u32, _col: u32, ch: char) -> Option<Vec<Value>> {
    // `str::lines()` drops a trailing empty line, but the LSP cursor can be
    // on a line that only exists because of a trailing `\n`.  We manually
    // append an empty element when the text ends with a newline to keep
    // line numbering consistent with the editor.
    let mut lines: Vec<&str> = text.lines().collect();
    if text.ends_with('\n') || text.ends_with("\r\n") {
        lines.push("");
    }

    if line as usize >= lines.len() {
        return None;
    }

    // Suppress all formatting inside heredoc bodies.
    if is_inside_heredoc(&lines, line as usize) {
        return None;
    }

    match ch {
        '}' => handle_close_brace(&lines, line),
        ';' => None, // Semicolons preserve existing indentation.
        '\n' | '\r' => handle_newline(&lines, line),
        _ => None,
    }
}

/// Handle `}` — re-indent the closing brace to match its opening `{`.
fn handle_close_brace(lines: &[&str], line: u32) -> Option<Vec<Value>> {
    let current_line = lines[line as usize];
    let current_indent = get_indentation(current_line);

    let target_indent = find_matching_brace_indent(lines, line as usize)
        .unwrap_or_else(|| current_indent.saturating_sub(2));

    if current_indent != target_indent {
        Some(vec![json!({
            "range": {
                "start": {"line": line, "character": 0},
                "end": {"line": line, "character": current_indent as u32}
            },
            "newText": " ".repeat(target_indent)
        })])
    } else {
        None
    }
}

/// Handle `\n` — set indentation of the new blank line based on the previous line.
fn handle_newline(lines: &[&str], line: u32) -> Option<Vec<Value>> {
    if line == 0 {
        return None;
    }

    let prev_line = lines[(line - 1) as usize];
    let prev_indent = get_indentation(prev_line);
    let trimmed = prev_line.trim_end();

    let indent = if trimmed.ends_with('{') {
        // Indent after opening brace.
        prev_indent + 2
    } else if trimmed.ends_with('}') {
        // Dedent after closing brace (the `}` line itself is already at the
        // correct indentation so the *next* line should match).
        prev_indent
    } else {
        prev_indent
    };

    let current_line = lines[line as usize];
    let current_indent = get_indentation(current_line);

    if current_indent == indent {
        return None;
    }

    Some(vec![json!({
        "range": {
            "start": {"line": line, "character": 0},
            "end": {"line": line, "character": current_indent as u32}
        },
        "newText": " ".repeat(indent)
    })])
}

/// Returns the number of leading space characters in `line`.
fn get_indentation(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Walk backwards from `closing_line` to find the matching `{` and return its
/// line's indentation.
///
/// Braces that appear inside single-quoted strings, double-quoted strings,
/// or comments (`#` to end-of-line) are ignored.
fn find_matching_brace_indent(lines: &[&str], closing_line: usize) -> Option<usize> {
    let mut brace_count: i32 = 1;

    for i in (0..closing_line).rev() {
        let braces = extract_significant_braces(lines[i]);
        // Process in reverse order to mirror scanning from right-to-left.
        for &brace_ch in braces.iter().rev() {
            match brace_ch {
                '}' => brace_count += 1,
                '{' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        return Some(get_indentation(lines[i]));
                    }
                }
                _ => {}
            }
        }
    }

    None
}

/// Extract braces from `line` that are *not* inside strings or comments.
///
/// Returns the braces in left-to-right order. This is a best-effort heuristic
/// that handles the most common Perl patterns:
/// - `#` comments (to end of line)
/// - Single-quoted strings (`'...'`)
/// - Double-quoted strings (`"..."`) with backslash escapes
///
/// It does not attempt to parse regex, heredocs, or multi-line strings —
/// those are handled by separate guards (e.g. `is_inside_heredoc`).
fn extract_significant_braces(line: &str) -> Vec<char> {
    let mut braces = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];
        match c {
            '#' => break, // Rest of line is a comment.
            '\'' => {
                // Skip to the closing single quote (no escape processing).
                i += 1;
                while i < len && chars[i] != '\'' {
                    i += 1;
                }
                // Skip closing quote if present.
                i += 1;
            }
            '"' => {
                // Skip to the closing double quote, respecting backslash escapes.
                i += 1;
                while i < len && chars[i] != '"' {
                    if chars[i] == '\\' {
                        i += 1; // skip escaped char
                    }
                    i += 1;
                }
                // Skip closing quote if present.
                i += 1;
            }
            '{' | '}' => {
                braces.push(c);
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    braces
}

/// Determine whether `target_line` falls inside a heredoc body.
///
/// A heredoc body starts on the line after the `<<` (or `<<~`) operator and
/// ends on the line containing the terminator.  This is a lightweight
/// heuristic that does not require a full parse tree.
fn is_inside_heredoc(lines: &[&str], target_line: usize) -> bool {
    // Track active heredoc terminators.  When we find a `<<IDENT` or
    // `<<'IDENT'` or `<<"IDENT"` or `<<~IDENT` etc., the body starts on
    // the *next* line and runs until we see the terminator on a line by
    // itself.
    let mut active_heredocs: Vec<String> = Vec::new();
    // Whether we are currently inside a heredoc body.
    let mut inside = false;

    for (line_idx, &line) in lines.iter().enumerate() {
        if line_idx > target_line {
            break;
        }

        // If we are inside a heredoc, check whether this line is the
        // terminator.
        if inside {
            if let Some(term) = active_heredocs.last() {
                let trimmed = line.trim();
                // Perl heredoc terminator: the tag alone on a line (possibly
                // with leading whitespace for <<~ heredocs, trailing ; allowed).
                let trimmed_semi = trimmed.trim_end_matches(';').trim_end();
                if trimmed_semi == term {
                    active_heredocs.pop();
                    inside = !active_heredocs.is_empty();
                    continue;
                }
            }
            if line_idx == target_line {
                return true;
            }
            continue;
        }

        // Scan this line for heredoc openers (<<TAG, <<'TAG', <<"TAG",
        // <<~TAG, <<~'TAG', <<~"TAG", <<`TAG`).
        let new_tags = find_heredoc_tags(line);
        if !new_tags.is_empty() {
            for tag in new_tags {
                active_heredocs.push(tag);
            }
            // Body starts on the next line.
            inside = true;
        }
    }

    false
}

/// Find all heredoc tags on a single line.
///
/// Returns the tag strings (without quotes) in order of appearance.
fn find_heredoc_tags(line: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i + 1 < len {
        if bytes[i] == b'<' && bytes[i + 1] == b'<' {
            i += 2;
            // Optional ~ for indented heredocs.
            if i < len && bytes[i] == b'~' {
                i += 1;
            }
            // Skip whitespace between << and tag.
            while i < len && bytes[i] == b' ' {
                i += 1;
            }
            if i >= len {
                break;
            }

            match bytes[i] {
                b'\'' | b'"' | b'`' => {
                    let quote = bytes[i];
                    i += 1;
                    let start = i;
                    while i < len && bytes[i] != quote {
                        i += 1;
                    }
                    if i > start {
                        tags.push(String::from_utf8_lossy(&bytes[start..i]).into_owned());
                    }
                    if i < len {
                        i += 1; // skip closing quote
                    }
                }
                b'\\' => {
                    // <<\TAG form
                    i += 1;
                    let start = i;
                    while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                        i += 1;
                    }
                    if i > start {
                        tags.push(String::from_utf8_lossy(&bytes[start..i]).into_owned());
                    }
                }
                b if b.is_ascii_alphabetic() || b == b'_' => {
                    let start = i;
                    while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                        i += 1;
                    }
                    tags.push(String::from_utf8_lossy(&bytes[start..i]).into_owned());
                }
                _ => {
                    // Not a valid heredoc, skip.
                }
            }
        } else {
            i += 1;
        }
    }

    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Helper: extract the "newText" from the first edit in the result.
    // ------------------------------------------------------------------
    fn first_new_text(edits: &Option<Vec<Value>>) -> Option<String> {
        edits
            .as_ref()
            .and_then(|v| v.first())
            .and_then(|e| e.get("newText"))
            .and_then(|t| t.as_str())
            .map(String::from)
    }

    fn first_edit_range(edits: &Option<Vec<Value>>) -> Option<(u64, u64, u64, u64)> {
        edits.as_ref().and_then(|v| v.first()).and_then(|e| {
            let r = e.get("range")?;
            let sl = r.pointer("/start/line")?.as_u64()?;
            let sc = r.pointer("/start/character")?.as_u64()?;
            let el = r.pointer("/end/line")?.as_u64()?;
            let ec = r.pointer("/end/character")?.as_u64()?;
            Some((sl, sc, el, ec))
        })
    }

    // ==================================================================
    //  `}` — auto-indent to match opening `{`
    // ==================================================================

    #[test]
    fn close_brace_aligns_to_opening_brace() {
        // The `}` on line 2 is indented 4 spaces, but the opening `{` is
        // on a line with 0-space indent, so `}` should be re-indented to 0.
        let text = "if ($x) {\n  print 1;\n    }";
        let edits = compute_on_type_edit(text, 2, 5, '}');
        assert!(edits.is_some());
        let new_text = first_new_text(&edits);
        assert_eq!(new_text.as_deref(), Some(""));
        // Range should replace the leading 4 spaces.
        assert_eq!(first_edit_range(&edits), Some((2, 0, 2, 4)));
    }

    #[test]
    fn close_brace_already_correct_indentation() {
        let text = "if ($x) {\n  print 1;\n}";
        let edits = compute_on_type_edit(text, 2, 1, '}');
        // Already at correct indent (0), so no edits.
        assert!(edits.is_none());
    }

    #[test]
    fn close_brace_nested_blocks() {
        let text = "sub foo {\n  if ($x) {\n    print 1;\n      }\n}";
        // Line 3: "      }" has 6 spaces, should align to "  if ($x) {" = 2 spaces.
        let edits = compute_on_type_edit(text, 3, 7, '}');
        assert!(edits.is_some());
        assert_eq!(first_new_text(&edits).as_deref(), Some("  "));
        assert_eq!(first_edit_range(&edits), Some((3, 0, 3, 6)));
    }

    #[test]
    fn close_brace_on_first_line_uses_saturating_sub() {
        // `}` on line 0 with no matching opener — falls back to saturating_sub.
        let text = "  }";
        let edits = compute_on_type_edit(text, 0, 3, '}');
        assert!(edits.is_some());
        assert_eq!(first_new_text(&edits).as_deref(), Some(""));
    }

    #[test]
    fn close_brace_skips_braces_in_strings() {
        // The `{` inside the string should not count as the matching opener.
        let text = "my $x = \"{\";\nif ($y) {\n  1;\n    }";
        let edits = compute_on_type_edit(text, 3, 5, '}');
        assert!(edits.is_some());
        // Should align to line 1 ("if ($y) {") which has indent 0.
        assert_eq!(first_new_text(&edits).as_deref(), Some(""));
    }

    #[test]
    fn close_brace_skips_braces_in_comments() {
        // The `{` in the comment should be ignored.
        let text = "# {\nif ($y) {\n  1;\n    }";
        let edits = compute_on_type_edit(text, 3, 5, '}');
        assert!(edits.is_some());
        assert_eq!(first_new_text(&edits).as_deref(), Some(""));
    }

    // ==================================================================
    //  `;` — does not change indentation
    // ==================================================================

    #[test]
    fn semicolon_does_not_change_indentation() {
        let text = "  my $x = 1;";
        let edits = compute_on_type_edit(text, 0, 12, ';');
        assert!(edits.is_none());
    }

    #[test]
    fn semicolon_at_end_of_indented_line_no_edit() {
        let text = "sub foo {\n    my $x = 1;";
        let edits = compute_on_type_edit(text, 1, 14, ';');
        assert!(edits.is_none());
    }

    #[test]
    fn semicolon_preserves_deeper_indentation() {
        let text = "        return 1;";
        let edits = compute_on_type_edit(text, 0, 17, ';');
        assert!(edits.is_none());
    }

    // ==================================================================
    //  `\n` — indent new line after `{`
    // ==================================================================

    #[test]
    fn newline_after_open_brace_indents() {
        let text = "if ($x) {\n";
        let edits = compute_on_type_edit(text, 1, 0, '\n');
        assert!(edits.is_some());
        // New line should get 2-space indent (0 + 2).
        assert_eq!(first_new_text(&edits).as_deref(), Some("  "));
    }

    #[test]
    fn newline_after_indented_open_brace() {
        let text = "  sub foo {\n";
        let edits = compute_on_type_edit(text, 1, 0, '\n');
        assert!(edits.is_some());
        // Previous line has 2-space indent, so new line gets 4.
        assert_eq!(first_new_text(&edits).as_deref(), Some("    "));
    }

    #[test]
    fn newline_after_plain_statement_keeps_indent() {
        let text = "    my $x = 1;\n";
        let edits = compute_on_type_edit(text, 1, 0, '\n');
        assert!(edits.is_some());
        assert_eq!(first_new_text(&edits).as_deref(), Some("    "));
    }

    #[test]
    fn newline_after_close_brace_keeps_its_indent() {
        let text = "  if ($x) {\n    1;\n  }\n";
        // Line 3 is after "  }", which has 2-space indent.
        let edits = compute_on_type_edit(text, 3, 0, '\n');
        assert!(edits.is_some());
        assert_eq!(first_new_text(&edits).as_deref(), Some("  "));
    }

    #[test]
    fn newline_on_first_line_returns_none() {
        let text = "\n";
        let edits = compute_on_type_edit(text, 0, 0, '\n');
        assert!(edits.is_none());
    }

    #[test]
    fn newline_replaces_existing_wrong_indent() {
        let text = "if ($x) {\n      ";
        // Line 1 has 6-space indent but should be 2.
        let edits = compute_on_type_edit(text, 1, 6, '\n');
        assert!(edits.is_some());
        assert_eq!(first_new_text(&edits).as_deref(), Some("  "));
        assert_eq!(first_edit_range(&edits), Some((1, 0, 1, 6)));
    }

    // ==================================================================
    //  Heredoc — formatting is disabled
    // ==================================================================

    #[test]
    fn heredoc_suppresses_close_brace() {
        let text = "my $x = <<END;\n}\nEND";
        let edits = compute_on_type_edit(text, 1, 1, '}');
        assert!(edits.is_none());
    }

    #[test]
    fn heredoc_suppresses_semicolon() {
        let text = "my $x = <<END;\nsome text;\nEND";
        let edits = compute_on_type_edit(text, 1, 10, ';');
        assert!(edits.is_none());
    }

    #[test]
    fn heredoc_suppresses_newline() {
        let text = "my $x = <<END;\nsome text\n\nEND";
        let edits = compute_on_type_edit(text, 2, 0, '\n');
        assert!(edits.is_none());
    }

    #[test]
    fn heredoc_single_quoted_tag() {
        let text = "my $x = <<'END';\n}\nEND";
        let edits = compute_on_type_edit(text, 1, 1, '}');
        assert!(edits.is_none());
    }

    #[test]
    fn heredoc_double_quoted_tag() {
        let text = "my $x = <<\"END\";\n}\nEND";
        let edits = compute_on_type_edit(text, 1, 1, '}');
        assert!(edits.is_none());
    }

    #[test]
    fn heredoc_indented_form() {
        let text = "my $x = <<~END;\n  }\n  END";
        let edits = compute_on_type_edit(text, 1, 3, '}');
        assert!(edits.is_none());
    }

    #[test]
    fn after_heredoc_terminator_formatting_resumes() {
        let text = "my $x = <<END;\nheredoc body\nEND\n    }";
        // Line 3 ("    }") is after the heredoc ends, so formatting should work.
        let edits = compute_on_type_edit(text, 3, 5, '}');
        // No matching opener, so falls back to saturating_sub: 4 - 2 = 2.
        assert!(edits.is_some());
        assert_eq!(first_new_text(&edits).as_deref(), Some("  "));
    }

    // ==================================================================
    //  Edge cases
    // ==================================================================

    #[test]
    fn out_of_bounds_line_is_none() {
        let edits = compute_on_type_edit("", 42, 0, '}');
        assert!(edits.is_none());
    }

    #[test]
    fn returns_none_for_unknown_trigger() {
        let text = "my $x = 1;";
        let edits = compute_on_type_edit(text, 0, 5, 'a');
        assert!(edits.is_none());
    }

    #[test]
    fn empty_document_newline() {
        let edits = compute_on_type_edit("\n", 0, 0, '\n');
        assert!(edits.is_none());
    }

    #[test]
    fn multiple_heredocs_on_one_line() {
        // Two heredocs on one line: body of first, then body of second.
        let text = "my ($a, $b) = (<<A, <<B);\nfirst\nA\nsecond\nB\nnormal;";
        // Line 1 ("first") is inside heredoc A.
        assert!(compute_on_type_edit(text, 1, 5, ';').is_none());
        // Line 3 ("second") is inside heredoc B.
        assert!(compute_on_type_edit(text, 3, 6, ';').is_none());
        // Line 5 ("normal;") is after both heredocs.
        assert!(compute_on_type_edit(text, 5, 7, ';').is_none()); // `;` is always None
    }

    // ------------------------------------------------------------------
    //  Internal helper unit tests
    // ------------------------------------------------------------------

    #[test]
    fn extract_braces_skips_strings_and_comments() {
        let braces = extract_significant_braces("my $h = { a => '{' }; # }");
        assert_eq!(braces, vec!['{', '}']);
    }

    #[test]
    fn extract_braces_handles_escaped_quotes() {
        let braces = extract_significant_braces("my $x = \"\\\"{\"; }");
        // The `{` is inside the string, the `}` is outside.
        assert_eq!(braces, vec!['}']);
    }

    #[test]
    fn find_heredoc_tags_bare() {
        let tags = find_heredoc_tags("my $x = <<EOF;");
        assert_eq!(tags, vec!["EOF"]);
    }

    #[test]
    fn find_heredoc_tags_quoted() {
        let tags = find_heredoc_tags("my $x = <<'END';");
        assert_eq!(tags, vec!["END"]);
    }

    #[test]
    fn find_heredoc_tags_tilde() {
        let tags = find_heredoc_tags("my $x = <<~HTML;");
        assert_eq!(tags, vec!["HTML"]);
    }

    #[test]
    fn is_inside_heredoc_basic() {
        let lines = vec!["my $x = <<END;", "body line", "END"];
        assert!(!is_inside_heredoc(&lines, 0));
        assert!(is_inside_heredoc(&lines, 1));
        assert!(!is_inside_heredoc(&lines, 2));
    }

    #[test]
    fn get_indentation_returns_leading_spaces() {
        assert_eq!(get_indentation("    foo"), 4);
        assert_eq!(get_indentation("foo"), 0);
        assert_eq!(get_indentation("  "), 2);
    }
}
