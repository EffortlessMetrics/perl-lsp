//! textDocument/selectionRange handler - smart selection expansion
//!
//! This module provides intelligent selection expansion that grows from
//! the narrowest syntactic element outward:
//!
//! - **Strings**: string content -> full string (with quotes) -> expression
//! - **Hash access**: key -> subscript `{key}` -> full expression `$h{key}`
//! - **Function names**: name -> signature -> full sub definition
//! - **General**: word -> trimmed line -> statement -> block -> function -> file

use lsp_types::{Position, Range, SelectionRange};
use perl_position_tracking::{offset_to_utf16_line_col, utf16_line_col_to_offset};

// ---------------------------------------------------------------------------
// Byte / position mapping helpers
// ---------------------------------------------------------------------------

fn byte_offset(text: &str, pos: Position) -> usize {
    utf16_line_col_to_offset(text, pos.line, pos.character)
}

fn make_range(text: &str, start: usize, end: usize) -> Range {
    let start = start.min(text.len());
    let end = end.min(text.len());
    let (sl, sc) = offset_to_utf16_line_col(text, start);
    let (el, ec) = offset_to_utf16_line_col(text, end);
    Range::new(Position::new(sl, sc), Position::new(el, ec))
}

// ---------------------------------------------------------------------------
// Span finders
// ---------------------------------------------------------------------------

/// Find the word (identifier/variable) span around `off`.
fn word_span(bytes: &[u8], off: usize) -> (usize, usize) {
    let safe_off = off.min(bytes.len().saturating_sub(1));
    let start = (0..=safe_off)
        .rev()
        .find(|&i| {
            i == 0
                || (!bytes[i - 1].is_ascii_alphanumeric()
                    && bytes[i - 1] != b'_'
                    && bytes[i - 1] != b'$'
                    && bytes[i - 1] != b'@'
                    && bytes[i - 1] != b'%')
        })
        .unwrap_or(off);
    let end = (off..bytes.len())
        .find(|&i| !bytes[i].is_ascii_alphanumeric() && bytes[i] != b'_')
        .unwrap_or(bytes.len());
    (start, end)
}

/// If `off` is inside a quoted string, return (content_start, content_end, full_start, full_end).
/// content excludes quote characters, full includes them.
fn string_span(text: &str, off: usize) -> Option<(usize, usize, usize, usize)> {
    let bytes = text.as_bytes();
    // Look for matching quote pairs around `off`
    for &q in b"\"'" {
        // Search backwards for opening quote
        let mut open = None;
        for i in (0..off).rev() {
            if bytes[i] == q {
                // Make sure it's not escaped
                let mut backslashes = 0usize;
                let mut j = i;
                while j > 0 && bytes[j - 1] == b'\\' {
                    backslashes += 1;
                    j -= 1;
                }
                if backslashes.is_multiple_of(2) {
                    open = Some(i);
                    break;
                }
            }
            // Stop at newline for safety (don't cross lines for non-heredoc strings)
            if bytes[i] == b'\n' {
                break;
            }
        }

        if let Some(open_pos) = open {
            // Search forwards for closing quote
            let mut i = off;
            while i < bytes.len() {
                if bytes[i] == q {
                    let mut backslashes = 0usize;
                    let mut j = i;
                    while j > 0 && bytes[j - 1] == b'\\' {
                        backslashes += 1;
                        j -= 1;
                    }
                    if backslashes.is_multiple_of(2) {
                        // Found matching close
                        let content_start = open_pos + 1;
                        let content_end = i;
                        let full_start = open_pos;
                        let full_end = i + 1;
                        return Some((content_start, content_end, full_start, full_end));
                    }
                }
                if bytes[i] == b'\n' {
                    break;
                }
                i += 1;
            }
        }
    }
    None
}

/// If `off` is inside a hash subscript `{...}`, return (key_start, key_end, subscript_start,
/// subscript_end, expr_start, expr_end).
///
/// - key: the text inside `{}`
/// - subscript: `{key}` including braces
/// - expr: `$hash{key}` including the variable
fn hash_access_span(text: &str, off: usize) -> Option<(usize, usize, usize, usize, usize, usize)> {
    let bytes = text.as_bytes();

    // Check if we're inside braces `{ ... }`
    let mut open = None;
    let mut depth = 0i32;
    for i in (0..off).rev() {
        if bytes[i] == b'}' {
            depth += 1;
        } else if bytes[i] == b'{' {
            if depth == 0 {
                open = Some(i);
                break;
            }
            depth -= 1;
        }
    }

    let open_pos = open?;

    // Check that what precedes the `{` looks like a hash variable or expression
    // (e.g. `$hash`, `$self->`, `$h`, `$hash_ref->`)
    if open_pos == 0 {
        return None;
    }
    let before = &text[..open_pos];
    let trimmed_before = before.trim_end();
    // Must end with an identifier char or `->`
    let looks_like_hash = trimmed_before.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_')
        || trimmed_before.ends_with("->");
    if !looks_like_hash {
        return None;
    }

    // Find closing brace
    let mut close = None;
    let mut depth = 0i32;
    for (i, &b) in bytes.iter().enumerate().skip(off) {
        if b == b'{' {
            depth += 1;
        } else if b == b'}' {
            if depth == 0 {
                close = Some(i);
                break;
            }
            depth -= 1;
        }
    }

    let close_pos = close?;

    let key_start = open_pos + 1;
    let key_end = close_pos;
    let subscript_start = open_pos;
    let subscript_end = close_pos + 1;

    // Walk backwards to find the start of the full expression ($hash or $self->hash)
    let mut expr_start = open_pos;
    // Skip any whitespace between variable and `{`
    while expr_start > 0 && bytes[expr_start - 1] == b' ' {
        expr_start -= 1;
    }
    // Walk back through `->` if present
    if expr_start >= 2 && &bytes[expr_start - 2..expr_start] == b"->" {
        expr_start -= 2;
        // Continue walking back through identifier
        while expr_start > 0
            && (bytes[expr_start - 1].is_ascii_alphanumeric() || bytes[expr_start - 1] == b'_')
        {
            expr_start -= 1;
        }
    }
    // Walk back through identifier chars
    while expr_start > 0
        && (bytes[expr_start - 1].is_ascii_alphanumeric() || bytes[expr_start - 1] == b'_')
    {
        expr_start -= 1;
    }
    // Include sigil ($, @, %)
    if expr_start > 0
        && (bytes[expr_start - 1] == b'$'
            || bytes[expr_start - 1] == b'@'
            || bytes[expr_start - 1] == b'%')
    {
        expr_start -= 1;
    }

    Some((key_start, key_end, subscript_start, subscript_end, expr_start, subscript_end))
}

/// If `off` is on a function name in a `sub` definition, return
/// (name_start, name_end, sig_start, sig_end, full_start, full_end).
fn sub_definition_span(
    text: &str,
    off: usize,
) -> Option<(usize, usize, Option<(usize, usize)>, usize, usize)> {
    let bytes = text.as_bytes();

    // Look backwards for `sub ` keyword
    let sub_keyword = text[..off.min(text.len())].rfind("sub ")?;

    // The name starts right after `sub `
    let name_start = sub_keyword + 4;

    // Skip whitespace
    let name_start = text[name_start..]
        .find(|c: char| !c.is_whitespace())
        .map(|i| name_start + i)
        .unwrap_or(name_start);

    // Find end of name (identifier characters)
    let mut name_end = name_start;
    while name_end < bytes.len()
        && (bytes[name_end].is_ascii_alphanumeric() || bytes[name_end] == b'_')
    {
        name_end += 1;
    }

    // Cursor must actually be on/near the name, or within the sub body
    if off < sub_keyword {
        return None;
    }

    // Find signature span (parenthesized parameter list)
    let after_name = &text[name_end..];
    let sig_span = if let Some(paren_off) = after_name.find('(') {
        let sig_start = name_end + paren_off;
        // Find matching close paren
        let mut depth = 0i32;
        let mut sig_end = sig_start;
        for (i, b) in bytes[sig_start..].iter().enumerate() {
            if *b == b'(' {
                depth += 1;
            } else if *b == b')' {
                depth -= 1;
                if depth == 0 {
                    sig_end = sig_start + i + 1;
                    break;
                }
            }
        }
        if sig_end > sig_start { Some((sig_start, sig_end)) } else { None }
    } else {
        None
    };

    // Find the full sub definition end (matching brace)
    let func_end = {
        let mut depth = 0i32;
        let mut found_brace = false;
        text[sub_keyword..]
            .char_indices()
            .find(|(_, c)| {
                if *c == '{' {
                    found_brace = true;
                    depth += 1;
                } else if *c == '}' && found_brace {
                    depth -= 1;
                    if depth == 0 {
                        return true;
                    }
                }
                false
            })
            .map(|(i, _)| sub_keyword + i + 1)
            .unwrap_or(text.len())
    };

    Some((name_start, name_end, sig_span, sub_keyword, func_end))
}

// ---------------------------------------------------------------------------
// Chain builder
// ---------------------------------------------------------------------------

/// Build a `SelectionRange` chain from a list of `(start, end)` spans.
/// Deduplicates ranges with the same LSP positions and ensures each parent
/// strictly encompasses its child.
fn build_chain(text: &str, spans: &[(usize, usize)]) -> SelectionRange {
    // Build ranges from spans, deduplicating
    let mut ranges: Vec<Range> = Vec::new();
    for &(s, e) in spans {
        let r = make_range(text, s, e);
        if ranges.last().is_none_or(|prev| *prev != r) {
            ranges.push(r);
        }
    }

    // Build nested chain from outermost to innermost
    let mut chain = SelectionRange { range: Range::default(), parent: None };
    for r in ranges.into_iter().rev() {
        chain = SelectionRange { range: r, parent: Some(Box::new(chain)) };
    }
    // The outermost `chain` is now the innermost selection; strip the dummy
    // we may have left at the tail.
    strip_default_tail(chain)
}

/// Remove the trailing dummy `Range::default()` node we may have seeded.
fn strip_default_tail(mut sel: SelectionRange) -> SelectionRange {
    if sel.parent.is_none() && sel.range == Range::default() {
        // Shouldn't happen if spans is non-empty, but safety fallback
        return sel;
    }
    if let Some(ref mut p) = sel.parent {
        if p.parent.is_none() && p.range == Range::default() {
            sel.parent = None;
        } else {
            **p = strip_default_tail(*p.clone());
        }
    }
    sel
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Generates smart selection ranges for given positions, expanding from the
/// narrowest syntactic element outward to the file scope.
///
/// The expansion chain is context-sensitive:
///
/// - **Inside a string**: string content -> full string (with quotes) ->
///   expression -> statement -> block -> function -> file
/// - **Inside a hash access**: key -> subscript `{key}` -> full expression
///   `$h{key}` -> statement -> block -> function -> file
/// - **On a function name**: name -> signature (if present) -> full sub
///   definition -> file
/// - **General**: word -> trimmed line -> full line -> statement -> block ->
///   function -> file
pub fn selection_ranges(text: &str, positions: &[Position]) -> Vec<SelectionRange> {
    positions
        .iter()
        .map(|&pos| {
            let off = byte_offset(text, pos);
            let bytes = text.as_bytes();

            let mut spans: Vec<(usize, usize)> = Vec::new();

            // 1. Word span (identifier or variable)
            let (w_start, w_end) = word_span(bytes, off);
            spans.push((w_start, w_end));

            // 2. Context-specific intermediate ranges
            //
            // String content -> full string
            if let Some((cs, ce, fs, fe)) = string_span(text, off) {
                // Insert content span before word if narrower
                if cs <= w_start && ce >= w_end && (cs != w_start || ce != w_end) {
                    spans.push((cs, ce));
                }
                spans.push((fs, fe));
            }

            // Hash access: key -> subscript -> full expression
            if let Some((ks, ke, ss, se, es, ee)) = hash_access_span(text, off) {
                // Key span
                if ks <= w_start && ke >= w_end && (ks != w_start || ke != w_end) {
                    spans.push((ks, ke));
                }
                spans.push((ss, se));
                spans.push((es, ee));
            }

            // 3. Trimmed line
            let line_start = text[..off].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = text[off..].find('\n').map(|i| off + i).unwrap_or(text.len());
            let line_text = &text[line_start..line_end];
            let trim_left = line_text.find(|c: char| !c.is_whitespace()).unwrap_or(0);
            let trim_right = line_text
                .rfind(|c: char| !c.is_whitespace())
                .map(|i| i + 1)
                .unwrap_or(line_text.len());
            spans.push((line_start + trim_left, line_start + trim_right));

            // 4. Full line
            spans.push((line_start, line_end));

            // 5. Statement (semicolon boundaries)
            let stmt_start = text[..off]
                .rfind(';')
                .map(|i| {
                    text[i + 1..]
                        .chars()
                        .position(|c| !c.is_whitespace())
                        .map(|j| i + 1 + j)
                        .unwrap_or(i + 1)
                })
                .unwrap_or(0);
            let stmt_end = text[off..]
                .find(';')
                .map(|i| off + i + 1)
                .unwrap_or_else(|| text[off..].find('\n').map(|i| off + i).unwrap_or(text.len()));
            spans.push((stmt_start, stmt_end));

            // 6. Block (brace boundaries)
            let block_start = text[..off].rfind('{').unwrap_or(0);
            let block_end = text[off..].find('}').map(|i| off + i + 1).unwrap_or(text.len());
            if block_end > block_start {
                spans.push((block_start, block_end));
            }

            // 7. Function (sub definition)
            if let Some((name_s, name_e, sig_span, full_s, full_e)) = sub_definition_span(text, off)
            {
                // If cursor is on/near the name, add name span first
                if off >= name_s && off <= name_e {
                    spans.push((name_s, name_e));
                }
                // Add signature if present
                if let Some((sig_s, sig_e)) = sig_span {
                    // Name + signature combined
                    spans.push((name_s, sig_e));
                    // Just signature
                    if off >= sig_s && off <= sig_e {
                        spans.push((sig_s, sig_e));
                    }
                }
                spans.push((full_s, full_e));
            } else {
                // Fallback: file-level
                spans.push((0, text.len()));
            }

            // 8. File scope (always outermost)
            spans.push((0, text.len()));

            // Sort spans by size (smallest first), then deduplicate
            spans.sort_by(|a, b| {
                let size_a = a.1.saturating_sub(a.0);
                let size_b = b.1.saturating_sub(b.0);
                size_a.cmp(&size_b)
            });
            spans.dedup();

            // Filter out spans that don't contain the cursor offset
            spans.retain(|&(s, e)| s <= off && e >= off);

            // Ensure strictly growing containment
            let mut filtered: Vec<(usize, usize)> = Vec::new();
            for span in &spans {
                if let Some(prev) = filtered.last() {
                    // Must be strictly larger (encompass previous)
                    if span.0 <= prev.0 && span.1 >= prev.1 && (span.0 < prev.0 || span.1 > prev.1)
                    {
                        filtered.push(*span);
                    }
                } else {
                    filtered.push(*span);
                }
            }

            if filtered.is_empty() {
                filtered.push((0, text.len()));
            }

            build_chain(text, &filtered)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: collect the chain of ranges as (start_line, start_col, end_line, end_col) tuples.
    fn chain_to_vec(sel: &SelectionRange) -> Vec<(u32, u32, u32, u32)> {
        let mut out = Vec::new();
        let mut cur = sel;
        loop {
            let r = &cur.range;
            out.push((r.start.line, r.start.character, r.end.line, r.end.character));
            if let Some(ref p) = cur.parent {
                cur = p;
            } else {
                break;
            }
        }
        out
    }

    #[test]
    fn string_content_expands_to_full_string() {
        // Cursor inside "hello" on the 'e' (offset 5 in the string content)
        let text = r#"my $x = "hello";"#;
        //           0123456789...
        // "hello" starts at byte 8 (the opening quote)
        // 'e' is at byte 10 (content: h=9, e=10)
        let pos = Position::new(0, 10);
        let results = selection_ranges(text, &[pos]);
        assert_eq!(results.len(), 1);
        let chain = chain_to_vec(&results[0]);

        // The innermost range should be the word "hello" or narrower
        // Then we should see string content, then full string with quotes
        assert!(chain.len() >= 3, "expected at least 3 levels for string, got {}", chain.len());

        // Verify ranges grow strictly
        for window in chain.windows(2) {
            let inner = window[0];
            let outer = window[1];
            assert!(
                outer.0 <= inner.0 && outer.2 >= inner.2,
                "parent ({},{})..({},{}) must encompass child ({},{})..({},{})",
                outer.0,
                outer.1,
                outer.2,
                outer.3,
                inner.0,
                inner.1,
                inner.2,
                inner.3,
            );
        }
    }

    #[test]
    fn hash_access_key_expands() {
        let text = r#"my $v = $hash{key};"#;
        //           01234567890123456789
        // 'k' of key is at byte 14
        let pos = Position::new(0, 14);
        let results = selection_ranges(text, &[pos]);
        assert_eq!(results.len(), 1);
        let chain = chain_to_vec(&results[0]);

        assert!(
            chain.len() >= 3,
            "expected at least 3 levels for hash access, got {}",
            chain.len()
        );

        // Verify ranges grow strictly
        for window in chain.windows(2) {
            let inner = window[0];
            let outer = window[1];
            assert!(outer.0 <= inner.0 && outer.2 >= inner.2, "parent must encompass child");
        }
    }

    #[test]
    fn function_name_expands_to_full_sub() {
        let text = "sub greet ($name) {\n    print $name;\n}\n";
        // 'greet' starts at byte 4
        let pos = Position::new(0, 5); // on the 'r' of greet
        let results = selection_ranges(text, &[pos]);
        assert_eq!(results.len(), 1);
        let chain = chain_to_vec(&results[0]);

        assert!(
            chain.len() >= 2,
            "expected at least 2 levels for function name, got {}",
            chain.len()
        );

        // Last range should be the full file
        let last = &chain[chain.len() - 1];
        assert_eq!(last.0, 0, "outermost should start at line 0");
    }

    #[test]
    fn empty_text_returns_zero_range() {
        let text = "";
        let pos = Position::new(0, 0);
        let results = selection_ranges(text, &[pos]);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn multiple_positions() {
        let text = "my $x = 1;\nmy $y = 2;\n";
        let positions = vec![Position::new(0, 3), Position::new(1, 3)];
        let results = selection_ranges(text, &positions);
        assert_eq!(results.len(), 2);
    }
}

/// Selection range provider wrapper.
///
/// Wraps the `selection_ranges` function in a conventional provider interface.
#[derive(Debug, Default)]
pub struct SelectionRangeProvider;

impl SelectionRangeProvider {
    /// Create a new selection range provider.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Compute selection ranges for the given positions.
    #[must_use]
    pub fn get_selection_ranges(&self, text: &str, positions: &[Position]) -> Vec<SelectionRange> {
        selection_ranges(text, positions)
    }
}
