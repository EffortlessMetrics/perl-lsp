//! Text processing utilities for LSP
//!
//! Common text processing helpers used across the LSP implementation.
//! Includes panic-free accessors for safe string processing.

pub mod uri;

use lsp_types::Position;

// =============================================================================
// Panic-free character accessors (Issue #143)
// =============================================================================

/// Safely get the first character of a string slice.
/// Returns None for empty strings instead of panicking.
#[inline]
pub fn first_char(s: &str) -> Option<char> {
    s.chars().next()
}

/// Safely get the nth character of a string slice.
/// Returns None if index is out of bounds instead of panicking.
#[inline]
pub fn nth_char(s: &str, n: usize) -> Option<char> {
    s.chars().nth(n)
}

/// Safely get the first character as a String.
/// Useful when you need the sigil character as a string.
#[inline]
pub fn first_char_string(s: &str) -> Option<String> {
    s.chars().next().map(|c| c.to_string())
}

/// Safely check if the first character satisfies a predicate.
/// Returns false for empty strings.
#[inline]
pub fn first_char_is<F: FnOnce(char) -> bool>(s: &str, predicate: F) -> bool {
    s.chars().next().is_some_and(predicate)
}

/// Safely check if the nth character satisfies a predicate.
/// Returns false if index is out of bounds.
#[inline]
pub fn nth_char_is<F: FnOnce(char) -> bool>(s: &str, n: usize, predicate: F) -> bool {
    s.chars().nth(n).is_some_and(predicate)
}

/// Convert byte offset to UTF-16 column position
///
/// LSP uses UTF-16 code units for character positions, but Rust strings use
/// UTF-8 byte offsets. This function converts a byte position within a line
/// to the corresponding UTF-16 column position.
pub fn byte_to_utf16_col(line_text: &str, byte_pos: usize) -> usize {
    let mut units = 0;
    for (i, ch) in line_text.char_indices() {
        if i >= byte_pos {
            break;
        }
        // UTF-16 encoding: chars >= U+10000 use 2 units
        units += if ch as u32 >= 0x10000 { 2 } else { 1 };
    }
    units
}

/// Convert UTF-16 column position to byte offset
pub fn byte_offset_utf16(line_text: &str, col_utf16: usize) -> usize {
    let mut units = 0;
    for (i, ch) in line_text.char_indices() {
        if units == col_utf16 {
            return i;
        }
        // UTF-16 encoding: chars >= U+10000 use 2 units (surrogate pairs)
        let add = if ch as u32 >= 0x10000 { 2 } else { 1 };
        units += add;
    }
    line_text.len()
}

/// Convert byte offset to line and column
pub fn byte_to_line_col(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 0;
    let mut col = 0;

    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    (line, col)
}

/// Helper character check for Perl identifiers
pub fn is_modchar(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b':' || b == b'_'
}

/// Extract token at cursor position (UTF-16 aware)
pub fn token_under_cursor(text: &str, line: usize, col_utf16: usize) -> Option<String> {
    let l = text.lines().nth(line)?;
    let byte_pos = byte_offset_utf16(l, col_utf16);
    let bytes = l.as_bytes();

    if byte_pos >= bytes.len() {
        return None;
    }

    // Expand to a "word" containing :: and \w
    // Also include sigils if we're on or after one
    let mut s = byte_pos;
    let mut e = byte_pos;

    // Expand left - if we hit a sigil, include it
    while s > 0 && is_modchar(bytes[s - 1]) {
        s -= 1;
    }
    if s > 0
        && (bytes[s - 1] == b'$'
            || bytes[s - 1] == b'@'
            || bytes[s - 1] == b'%'
            || bytes[s - 1] == b'&'
            || bytes[s - 1] == b'*')
    {
        s -= 1;
    }

    // Expand right
    while e < bytes.len() && is_modchar(bytes[e]) {
        e += 1;
    }

    Some(l[s..e].to_string())
}

/// Check if position is at word boundary (for accurate reference matching)
pub fn is_word_boundary(text: &[u8], pos: usize, word_len: usize) -> bool {
    // Check left boundary (before the sigil if present)
    if pos > 0 && is_modchar(text[pos - 1]) {
        return false;
    }

    // Check right boundary (after the identifier part)
    let end_pos = pos + word_len;
    if end_pos < text.len() && is_modchar(text[end_pos]) {
        return false;
    }

    true
}

/// Find matching closing parenthesis
pub fn find_matching_paren(s: &str, open_at: usize) -> Option<usize> {
    // s[open_at] must be '('; walk forwards tracking () and quotes.
    let mut i = open_at;
    let mut depth_par = 0i32;
    let mut in_s = false;
    let mut in_d = false;
    while i < s.len() {
        let b = s.as_bytes()[i];
        let prev_backslash = i > 0 && s.as_bytes()[i - 1] == b'\\';
        if in_s {
            if b == b'\'' && !prev_backslash {
                in_s = false;
            }
        } else if in_d {
            if b == b'"' && !prev_backslash {
                in_d = false;
            }
        } else {
            match b {
                b'\'' => in_s = true,
                b'"' => in_d = true,
                b'(' => depth_par += 1,
                b')' => {
                    depth_par -= 1;
                    if depth_par == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    None
}

/// Scan forward until end of statement (top-level `;`) honoring quotes/brackets.
pub fn slice_until_stmt_end(src: &str, from: usize) -> usize {
    let mut i = from;
    let mut depth_par = 0i32;
    let mut depth_brk = 0i32;
    let mut depth_brc = 0i32;
    let mut in_s = false;
    let mut in_d = false;
    while i < src.len() {
        let b = src.as_bytes()[i];
        let esc = i > 0 && src.as_bytes()[i - 1] == b'\\';
        if in_s {
            if b == b'\'' && !esc {
                in_s = false;
            }
        } else if in_d {
            if b == b'"' && !esc {
                in_d = false;
            }
        } else {
            match b {
                b'\'' => in_s = true,
                b'"' => in_d = true,
                b'(' => depth_par += 1,
                b')' => depth_par -= 1,
                b'[' => depth_brk += 1,
                b']' => depth_brk -= 1,
                b'{' => depth_brc += 1,
                b'}' => depth_brc -= 1,
                b';' if depth_par == 0 && depth_brk == 0 && depth_brc == 0 => return i,
                _ => {}
            }
        }
        i += 1;
    }
    src.len()
}

/// Top-level argument starts for a comma-separated list without surrounding parens.
pub fn arg_starts_top_level(src: &str) -> Vec<usize> {
    let mut v = Vec::new();
    let mut i = 0usize;
    while i < src.len() && src.as_bytes()[i].is_ascii_whitespace() {
        i += 1;
    }
    if i < src.len() {
        v.push(i);
    }
    let mut j = i;
    let mut depth_par = 0i32;
    let mut depth_brk = 0i32;
    let mut depth_brc = 0i32;
    let mut in_s = false;
    let mut in_d = false;
    while j < src.len() {
        let b = src.as_bytes()[j];
        let esc = j > 0 && src.as_bytes()[j - 1] == b'\\';
        if in_s {
            if b == b'\'' && !esc {
                in_s = false;
            }
        } else if in_d {
            if b == b'"' && !esc {
                in_d = false;
            }
        } else {
            match b {
                b'\'' => in_s = true,
                b'"' => in_d = true,
                b'(' => depth_par += 1,
                b')' => depth_par -= 1,
                b'[' => depth_brk += 1,
                b']' => depth_brk -= 1,
                b'{' => depth_brc += 1,
                b'}' => depth_brc -= 1,
                b',' if depth_par == 0 && depth_brk == 0 && depth_brc == 0 => {
                    let mut k = j + 1;
                    while k < src.len() && src.as_bytes()[k].is_ascii_whitespace() {
                        k += 1;
                    }
                    if k < src.len() {
                        v.push(k);
                    }
                }
                _ => {}
            }
        }
        j += 1;
    }
    v
}

/// Move the anchor inside an argument to the "interesting" token:
///  - skip leading whitespace
///  - for `my|our` args, jump to the first sigiled var (`$foo`/`@a`/`%h`)
///  - for bareword filehandles (e.g., `FH`), jump to the bareword
pub fn anchor_arg_start(body: &str, rel: usize) -> usize {
    let s = &body[rel..];
    let mut i = 0usize;
    while i < s.len() && s.as_bytes()[i].is_ascii_whitespace() {
        i += 1;
    }
    // my/our <sigiled-var>
    if s[i..].starts_with("my ") || s[i..].starts_with("our ") {
        let mut j = i + 3; // skip "my " / "our "
        while j < s.len() && s.as_bytes()[j].is_ascii_whitespace() {
            j += 1;
        }
        return rel + j;
    }
    // If next is sigiled variable, keep; else keep bareword start
    rel + i
}

/// If argument starts at `my $fh`, retarget anchor to the `$fh` (or bareword FH).
pub fn smart_arg_anchor(body: &str, rel: usize) -> usize {
    let s = &body[rel..];
    let mut i = 0usize;
    while i < s.len() && s.as_bytes()[i].is_ascii_whitespace() {
        i += 1;
    }

    // handle my/our
    for kw in ["my ", "our "] {
        if s[i..].starts_with(kw) {
            i += kw.len();
            while i < s.len() && s.as_bytes()[i].is_ascii_whitespace() {
                i += 1;
            }
            break;
        }
    }

    // valid anchors: sigils, barewords, deref braces and array/hash derefs
    // $, @, %, &, { (for @{ ... }, %{ ... }), [ (rare, but safe), or A-Za-z_ bareword
    let b = s.as_bytes().get(i).copied().unwrap_or(b' ');
    if matches!(b, b'$' | b'@' | b'%' | b'&' | b'{' | b'[') || b.is_ascii_alphabetic() || b == b'_'
    {
        return rel + i;
    }
    rel + i
}

/// Find argument starts in function call body
pub fn arg_starts_in_call_body(body: &str) -> Vec<usize> {
    // Return byte offsets (within body) where each top-level argument starts.
    let mut starts = Vec::new();
    let mut i = 0usize;
    let mut depth_par = 0i32;
    let mut depth_brk = 0i32;
    let mut depth_brc = 0i32;
    let mut in_s = false;
    let mut in_d = false;
    // First arg always starts at the first non-space
    while i < body.len() && body.as_bytes()[i].is_ascii_whitespace() {
        i += 1;
    }
    if i < body.len() {
        starts.push(i);
    }
    let mut j = i;
    while j < body.len() {
        let b = body.as_bytes()[j];
        let prev_backslash = j > 0 && body.as_bytes()[j - 1] == b'\\';
        if in_s {
            if b == b'\'' && !prev_backslash {
                in_s = false;
            }
        } else if in_d {
            if b == b'"' && !prev_backslash {
                in_d = false;
            }
        } else {
            match b {
                b'\'' => in_s = true,
                b'"' => in_d = true,
                b'(' => depth_par += 1,
                b')' => depth_par -= 1,
                b'[' => depth_brk += 1,
                b']' => depth_brk -= 1,
                b'{' => depth_brc += 1,
                b'}' => depth_brc -= 1,
                b',' if depth_par == 0 && depth_brk == 0 && depth_brc == 0 => {
                    // next arg start = first non-space after comma
                    let mut k = j + 1;
                    while k < body.len() && body.as_bytes()[k].is_ascii_whitespace() {
                        k += 1;
                    }
                    if k < body.len() {
                        starts.push(k);
                    }
                }
                _ => {}
            }
        }
        j += 1;
    }
    starts
}

/// Convert position to byte offset
pub fn pos_to_offset_bytes(text: &str, line: u32, ch: u32) -> usize {
    let mut byte = 0usize;
    for (cur, l) in text.split_inclusive('\n').enumerate() {
        if cur as u32 == line {
            return byte + (ch as usize).min(l.len());
        }
        byte += l.len();
    }
    text.len()
}

/// Slice text within range
pub fn slice_in_range(text: &str, start: (u32, u32), end: (u32, u32)) -> (usize, usize, &str) {
    let s = pos_to_offset_bytes(text, start.0, start.1);
    let e = pos_to_offset_bytes(text, end.0, end.1);
    (s, e, &text[s.min(text.len())..e.min(text.len())])
}

/// Get text around an offset position
pub fn get_text_around_offset(content: &str, offset: usize, radius: usize) -> String {
    let start = offset.saturating_sub(radius);
    let end = (offset + radius).min(content.len());
    content[start..end].to_string()
}

/// Extract module reference from text (e.g., from "use Module::Name" or "require Module::Name")
pub fn extract_module_reference(text: &str, cursor_pos: usize) -> Option<String> {
    // Look for patterns like "use Module::Name" or "require Module::Name"
    let patterns = [
        r"use\s+([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)",
        r"require\s+([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)",
    ];

    for pattern in patterns {
        let re = regex::Regex::new(pattern).ok()?;
        for cap in re.captures_iter(text) {
            if let Some(module_match) = cap.get(1) {
                let match_start = module_match.start();
                let match_end = module_match.end();

                // Check if cursor is within the module name
                if cursor_pos >= match_start && cursor_pos <= match_end {
                    return Some(module_match.as_str().to_string());
                }
            }
        }
    }

    None
}

/// Convert an LSP position to a byte offset in the text (UTF-16 aware, CRLF safe)
pub fn position_to_offset(content: &str, line: u32, character: u32) -> Option<usize> {
    let mut cur_line = 0u32;
    let mut col_utf16 = 0u32;
    let mut prev_was_cr = false;

    for (byte_idx, ch) in content.char_indices() {
        // Check if we've reached the target position
        if cur_line == line && col_utf16 == character {
            return Some(byte_idx);
        }

        // Handle line endings and character counting
        match ch {
            '\n' => {
                if !prev_was_cr {
                    // Standalone \n
                    cur_line += 1;
                    col_utf16 = 0;
                }
                // If prev_was_cr, this \n is part of CRLF and we already incremented the line
            }
            '\r' => {
                // Always increment line on \r (whether solo or part of CRLF)
                cur_line += 1;
                col_utf16 = 0;
            }
            _ => {
                // Regular character - only count UTF-16 units on target line
                if cur_line == line {
                    col_utf16 += if ch.len_utf16() == 2 { 2 } else { 1 };
                }
            }
        }

        prev_was_cr = ch == '\r';
    }

    // Handle end of file position
    if cur_line == line && col_utf16 == character {
        return Some(content.len());
    }

    // Return None if position is out of bounds
    None
}

/// Convert a byte offset to an LSP position (UTF-16 aware, CRLF safe)
pub fn offset_to_position(content: &str, offset: usize) -> Position {
    let mut line = 0u32;
    let mut col_utf16 = 0u32;
    let mut byte_pos = 0usize;
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        if byte_pos >= offset {
            break;
        }

        match ch {
            '\r' => {
                // Peek ahead to see if this is CRLF
                if chars.peek() == Some(&'\n') {
                    // This is CRLF - treat as single line ending
                    if byte_pos + 1 >= offset {
                        // Offset is at the \r - treat as end of current line
                        break;
                    }
                    // Skip both \r and \n
                    chars.next(); // consume the \n
                    line += 1;
                    col_utf16 = 0;
                    byte_pos += 2; // \r + \n
                } else {
                    // Solo \r - treat as line ending
                    line += 1;
                    col_utf16 = 0;
                    byte_pos += ch.len_utf8();
                }
            }
            '\n' => {
                // LF (could be standalone or part of CRLF, but CRLF is handled above)
                line += 1;
                col_utf16 = 0;
                byte_pos += ch.len_utf8();
            }
            _ => {
                // Regular character
                col_utf16 += if ch.len_utf16() == 2 { 2 } else { 1 };
                byte_pos += ch.len_utf8();
            }
        }
    }

    Position { line, character: col_utf16 }
}
