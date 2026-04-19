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
///
/// # POD suppression
///
/// When the cursor falls inside a POD block (`=pod`/`=head1`/etc. … `=cut`),
/// all on-type formatting is suppressed to avoid corrupting documentation.
///
/// # `indent_step`
///
/// The number of spaces to add or remove per indentation level. Corresponds
/// to the LSP client's `tabSize` option. Typical values: 2, 4.
pub fn compute_on_type_edit(
    text: &str,
    line: u32,
    _col: u32,
    ch: char,
    indent_step: usize,
) -> Option<Vec<Value>> {
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

    // Suppress all formatting inside POD blocks.
    if is_inside_pod(&lines, line as usize) {
        return None;
    }

    match ch {
        '}' => handle_close_brace(&lines, line, indent_step),
        ';' => None, // Semicolons preserve existing indentation.
        '\n' | '\r' => handle_newline(&lines, line, indent_step),
        _ => None,
    }
}

/// Handle `}` — re-indent the closing brace to match its opening `{`.
fn handle_close_brace(lines: &[&str], line: u32, indent_step: usize) -> Option<Vec<Value>> {
    let current_line = lines[line as usize];
    let current_indent = get_indentation(current_line);

    let target_indent = find_matching_brace_indent(lines, line as usize)
        .unwrap_or_else(|| current_indent.saturating_sub(indent_step));

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
fn handle_newline(lines: &[&str], line: u32, indent_step: usize) -> Option<Vec<Value>> {
    if line == 0 {
        return None;
    }

    let prev_line = lines[(line - 1) as usize];
    let prev_indent = get_indentation(prev_line);
    let trimmed = prev_line.trim_end();

    let indent = if trimmed.ends_with('{') {
        // Indent after opening brace.
        prev_indent + indent_step
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
/// comments (`#` to end-of-line), `qw{...}` blocks, or regex quantifiers
/// (`{n}`, `{n,}`, `{n,m}`) are ignored.
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
            'q' if i + 2 < len && chars[i + 1] == 'w' && chars[i + 2] == '{' => {
                // qw{...} word-list constructor — skip the entire block.
                i += 3; // skip 'q', 'w', '{'
                while i < len && chars[i] != '}' {
                    i += 1;
                }
                // Skip closing '}'.
                i += 1;
            }
            '{' => {
                // Check if this is a regex quantifier {n}, {n,}, or {n,m}.
                // Pattern: { digits [, [digits]] }
                if is_regex_quantifier(&chars, i) {
                    // Skip past the closing '}'.
                    i += 1;
                    while i < len && chars[i] != '}' {
                        i += 1;
                    }
                    // Skip the closing '}'.
                    i += 1;
                } else {
                    braces.push(c);
                    i += 1;
                }
            }
            '}' => {
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

/// Returns `true` if the `{` at position `start` in `chars` opens a regex
/// quantifier of the form `{n}`, `{n,}`, or `{n,m}` where n and m are
/// non-negative integers (digit sequences).
fn is_regex_quantifier(chars: &[char], start: usize) -> bool {
    let len = chars.len();
    let mut i = start + 1; // move past '{'

    // Must start with at least one digit.
    if i >= len || !chars[i].is_ascii_digit() {
        return false;
    }
    while i < len && chars[i].is_ascii_digit() {
        i += 1;
    }

    // Now either '}' (exact quantifier) or ',' (range quantifier).
    if i >= len {
        return false;
    }
    if chars[i] == '}' {
        return true; // {n}
    }
    if chars[i] != ',' {
        return false;
    }
    i += 1; // skip ','

    // Optional second digit sequence.
    while i < len && chars[i].is_ascii_digit() {
        i += 1;
    }

    // Must end with '}'.
    i < len && chars[i] == '}'
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

/// POD keyword prefixes that open a POD block when at column 0.
const POD_OPENERS: &[&str] = &[
    "=pod", "=head1", "=head2", "=head3", "=head4", "=over", "=begin", "=item",
];

/// Determine whether `target_line` falls inside a POD block.
///
/// A POD block starts when a line begins with one of the POD opener keywords
/// (`=pod`, `=head1`–`=head4`, `=over`, `=begin`, `=item`) at column 0.
/// The block ends when a line begins with `=cut` or `=end` at column 0
/// (trailing whitespace on the terminator line is allowed).
///
/// This is a lightweight heuristic that does not require a full parse tree,
/// modeled on `is_inside_heredoc`.
fn is_inside_pod(lines: &[&str], target_line: usize) -> bool {
    let mut inside = false;

    for (line_idx, &line) in lines.iter().enumerate() {
        if line_idx > target_line {
            break;
        }

        if inside {
            // Check for terminator: `=cut` or `=end` at column 0 (trailing
            // whitespace is allowed, matching Perl's own pod parsing rules).
            let trimmed = line.trim_end();
            let is_end =
                trimmed == "=end" || trimmed.starts_with("=end ") || trimmed.starts_with("=end\t");
            if trimmed == "=cut" || is_end {
                inside = false;
                continue;
            }
            if line_idx == target_line {
                return true;
            }
            continue;
        }

        // Fast pre-check: POD lines always start with '='.
        if line.starts_with('=') {
            for opener in POD_OPENERS {
                // Must be an exact keyword or followed by whitespace/EOL.
                let rest = &line[opener.len().min(line.len())..];
                if line.starts_with(opener)
                    && (rest.is_empty() || rest.starts_with(char::is_whitespace))
                {
                    inside = true;
                    break;
                }
            }
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

    // Internal helper unit tests — these call private functions and must stay here.

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

    #[test]
    fn extract_braces_skips_regex_quantifier_exact() {
        // {3} is a regex quantifier — neither brace should be counted.
        let braces = extract_significant_braces("my $x = /\\w{3}/;");
        assert_eq!(braces, vec![]);
    }

    #[test]
    fn extract_braces_skips_regex_quantifier_range() {
        // {2,5} is a regex quantifier — neither brace should be counted.
        let braces = extract_significant_braces("if ($str =~ /\\d{2,5}/) {");
        assert_eq!(braces, vec!['{']);
    }

    #[test]
    fn extract_braces_skips_regex_quantifier_open_range() {
        // {2,} is a regex quantifier (at least n) — neither brace counted.
        let braces = extract_significant_braces("my $x = /a{2,}/;");
        assert_eq!(braces, vec![]);
    }

    #[test]
    fn extract_braces_skips_qw_block() {
        // qw{...} — neither brace should be counted.
        let braces = extract_significant_braces("my @a = qw{foo bar baz};");
        assert_eq!(braces, vec![]);
    }

    #[test]
    fn extract_braces_qw_block_before_real_brace() {
        // qw{...} followed by a real block opener.
        let braces = extract_significant_braces("foreach my $x (qw{a b}) {");
        assert_eq!(braces, vec!['{']);
    }

    #[test]
    fn is_regex_quantifier_exact() {
        let chars: Vec<char> = "{3}".chars().collect();
        assert!(is_regex_quantifier(&chars, 0));
    }

    #[test]
    fn is_regex_quantifier_range() {
        let chars: Vec<char> = "{2,5}".chars().collect();
        assert!(is_regex_quantifier(&chars, 0));
    }

    #[test]
    fn is_regex_quantifier_open_range() {
        let chars: Vec<char> = "{2,}".chars().collect();
        assert!(is_regex_quantifier(&chars, 0));
    }

    #[test]
    fn is_regex_quantifier_rejects_hash_block() {
        // A hash/block opener like `{` followed by non-digit is NOT a quantifier.
        let chars: Vec<char> = "{ key => 1 }".chars().collect();
        assert!(!is_regex_quantifier(&chars, 0));
    }

    #[test]
    fn is_regex_quantifier_rejects_empty_braces() {
        let chars: Vec<char> = "{}".chars().collect();
        assert!(!is_regex_quantifier(&chars, 0));
    }

    #[test]
    fn extract_braces_multiple_quantifiers_on_one_line() {
        // Two quantifiers on one line with a real block opener — only the opener
        // should be pushed.  This would fail before the fix because the old code
        // pushed all six braces ({, }, {, }, {) and the reversed walk found the
        // wrong opener.
        let braces = extract_significant_braces("if (/\\w{2}.*\\d{3}/) {");
        assert_eq!(braces, vec!['{']);
    }

    #[test]
    fn is_regex_quantifier_rejects_unclosed_brace_at_eol() {
        // '{' with no following content must not panic and must return false.
        let chars: Vec<char> = "{".chars().collect();
        assert!(!is_regex_quantifier(&chars, 0));
    }

    #[test]
    fn is_regex_quantifier_rejects_digit_then_eol() {
        // '{3' with no closing '}' must not panic and must return false.
        let chars: Vec<char> = "{3".chars().collect();
        assert!(!is_regex_quantifier(&chars, 0));
    }

    #[test]
    fn extract_braces_qw_block_unterminated() {
        // Malformed qw{ with no closing '}' must not panic.
        // The opening qw{ is consumed; no brace is pushed.
        let braces = extract_significant_braces("my @x = qw{a b c");
        assert_eq!(braces, vec![]);
    }
}
