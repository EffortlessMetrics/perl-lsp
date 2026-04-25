//! Uniform quote operator parsing for the Perl parser.
//!
//! This module provides consistent parsing for quote-like operators,
//! properly extracting patterns, bodies, and modifiers.

use std::borrow::Cow;

/// Extract pattern and modifiers from a regex-like token (qr, m, or bare //)
pub fn extract_regex_parts(text: &str) -> (String, String, String) {
    // Handle different prefixes
    let content = if let Some(stripped) = text.strip_prefix("qr") {
        stripped
    } else if text.starts_with('m')
        && text.len() > 1
        && text.chars().nth(1).is_some_and(|c| !c.is_alphabetic())
    {
        &text[1..]
    } else {
        text
    };

    // Get delimiter - content must be non-empty to have a delimiter
    let delimiter = match content.chars().next() {
        Some(d) => d,
        None => return (String::new(), String::new(), String::new()),
    };
    let closing = get_closing_delimiter(delimiter);

    // Extract body and modifiers
    let (body, modifiers) = extract_delimited_content(content, delimiter, closing);

    // Include delimiters in the pattern string for compatibility
    let pattern = format!("{}{}{}", delimiter, body, closing);

    (pattern, body, modifiers.to_string())
}

/// Error type for substitution operator parsing failures
#[derive(Debug, Clone, PartialEq)]
pub enum SubstitutionError {
    /// Invalid modifier character found
    InvalidModifier(char),
    /// Missing delimiter after 's'
    MissingDelimiter,
    /// Pattern is missing or empty (just `s/`)
    MissingPattern,
    /// Replacement section is missing (e.g., `s/pattern` without replacement part)
    MissingReplacement,
    /// Closing delimiter is missing after replacement (e.g., `s/pattern/replacement` without final `/`)
    MissingClosingDelimiter,
}

/// Error type for transliteration operator parsing failures
#[derive(Debug, Clone, PartialEq)]
pub enum TransliterationError {
    /// Invalid modifier character found
    InvalidModifier(char),
    /// Missing delimiter after `tr`/`y`
    MissingDelimiter,
    /// Search list section is missing
    MissingSearch,
    /// Replacement list section is missing
    MissingReplacement,
    /// Closing delimiter is missing
    MissingClosingDelimiter,
}

/// Extract pattern, replacement, and modifiers from a substitution token with strict validation
///
/// This function parses substitution operators like s/pattern/replacement/flags
/// and handles various delimiter forms including:
/// - Non-paired delimiters: s/pattern/replacement/ (same delimiter for all parts)
/// - Paired delimiters: s{pattern}{replacement} (different open/close delimiters)
///
/// Unlike `extract_substitution_parts`, this function returns an error if invalid modifiers
/// are present instead of silently filtering them.
///
/// # Errors
///
/// Returns `Err(SubstitutionError::InvalidModifier(c))` if an invalid modifier character is found.
/// Valid modifiers are: g, i, m, s, x, o, e, r
pub fn extract_substitution_parts_strict(
    text: &str,
) -> Result<(String, String, String), SubstitutionError> {
    // Skip 's' prefix
    let after_s = text.strip_prefix('s').unwrap_or(text);
    // Perl allows whitespace between 's' and its delimiter (e.g. `s { pattern } { replacement }g`)
    let content = after_s.trim_start();

    // Get delimiter - check for missing delimiter (just 's' or 's' followed by nothing)
    let delimiter = match content.chars().next() {
        Some(d) => d,
        None => return Err(SubstitutionError::MissingDelimiter),
    };
    let closing = get_closing_delimiter(delimiter);
    let is_paired = delimiter != closing;

    // Parse first body (pattern) with strict validation
    let (pattern, rest1, pattern_closed) =
        extract_delimited_content_strict(content, delimiter, closing);

    // For non-paired delimiters: if pattern wasn't closed, missing closing delimiter
    if !is_paired && !pattern_closed {
        return Err(SubstitutionError::MissingClosingDelimiter);
    }

    // For paired delimiters: if pattern wasn't closed, missing closing delimiter
    if is_paired && !pattern_closed {
        return Err(SubstitutionError::MissingClosingDelimiter);
    }

    // Parse second body (replacement)
    // For paired delimiters, the replacement may use a different delimiter than the pattern
    // e.g., s[pattern]{replacement} is valid Perl
    let (replacement, modifiers_str, replacement_closed) = if !is_paired {
        // Non-paired delimiters: must have replacement section
        if rest1.is_empty() {
            return Err(SubstitutionError::MissingReplacement);
        }

        // Parse replacement, skipping string literals so that delimiter chars
        // inside "foo/bar" or 'a/b' don't terminate the replacement early.
        let (body, rest, found_closing) = extract_unpaired_body_skip_strings(rest1, closing);
        (body, rest, found_closing)
    } else {
        // Paired delimiters
        let trimmed = rest1.trim_start();
        // For paired delimiters, check what delimiter the replacement uses
        // It may be the same as pattern or a different paired delimiter
        // e.g., s[pattern]{replacement} uses [] for pattern and {} for replacement
        if let Some(rd) = trimmed.chars().next() {
            // Check if it's a valid paired opening delimiter
            if rd == '{' || rd == '[' || rd == '(' || rd == '<' {
                let repl_closing = get_closing_delimiter(rd);
                extract_delimited_content_strict(trimmed, rd, repl_closing)
            } else {
                // Not a valid paired delimiter - malformed
                return Err(SubstitutionError::MissingReplacement);
            }
        } else {
            // No more content - missing replacement
            return Err(SubstitutionError::MissingReplacement);
        }
    };

    // For non-paired delimiters, must have found the closing delimiter for replacement
    if !is_paired && !replacement_closed {
        return Err(SubstitutionError::MissingClosingDelimiter);
    }

    // For paired delimiters, must have found the closing delimiter for replacement
    if is_paired && !replacement_closed {
        return Err(SubstitutionError::MissingClosingDelimiter);
    }

    // Validate modifiers strictly - reject if any invalid modifiers present
    let modifiers = validate_substitution_modifiers(modifiers_str)
        .map_err(SubstitutionError::InvalidModifier)?;

    Ok((pattern, replacement, modifiers))
}

/// Extract content between delimiters with strict tracking of whether closing was found.
/// Returns (content, rest, found_closing).
fn extract_delimited_content_strict(text: &str, open: char, close: char) -> (String, &str, bool) {
    let mut chars = text.char_indices();
    let is_paired = open != close;

    // Skip opening delimiter
    if let Some((_, c)) = chars.next() {
        if c != open {
            return (String::new(), text, false);
        }
    } else {
        return (String::new(), "", false);
    }

    let mut body = String::new();
    let mut depth = if is_paired { 1 } else { 0 };
    let mut escaped = false;
    let mut end_pos = text.len();
    let mut found_closing = false;

    for (i, ch) in chars {
        if escaped {
            body.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => {
                body.push(ch);
                escaped = true;
            }
            c if c == open && is_paired => {
                body.push(ch);
                depth += 1;
            }
            c if c == close => {
                if is_paired {
                    depth -= 1;
                    if depth == 0 {
                        end_pos = i + ch.len_utf8();
                        found_closing = true;
                        break;
                    }
                    body.push(ch);
                } else {
                    end_pos = i + ch.len_utf8();
                    found_closing = true;
                    break;
                }
            }
            _ => body.push(ch),
        }
    }

    (body, &text[end_pos..], found_closing)
}

/// Extract pattern, replacement, and modifiers from a substitution token
///
/// This function parses substitution operators like s/pattern/replacement/flags
/// and handles various delimiter forms including:
/// - Non-paired delimiters: s/pattern/replacement/ (same delimiter for all parts)
/// - Paired delimiters: s{pattern}{replacement} (different open/close delimiters)
///
/// For paired delimiters, properly handles nested delimiters within the pattern
/// or replacement parts. Returns (pattern, replacement, modifiers) as strings.
///
/// Note: This function silently filters invalid modifiers. For strict validation,
/// use `extract_substitution_parts_strict` instead.
pub fn extract_substitution_parts(text: &str) -> (String, String, String) {
    // Skip 's' prefix
    let content = text.strip_prefix('s').unwrap_or(text);

    // Get delimiter - content must be non-empty to have a delimiter
    let delimiter = match content.chars().next() {
        Some(d) => d,
        None => return (String::new(), String::new(), String::new()),
    };
    if delimiter.is_ascii_alphanumeric() || delimiter.is_whitespace() {
        if let Some((pattern, replacement, modifiers_str)) = split_on_last_paired_delimiter(content)
        {
            let modifiers = extract_substitution_modifiers(&modifiers_str);
            return (pattern, replacement, modifiers);
        }

        return (String::new(), String::new(), String::new());
    }
    let closing = get_closing_delimiter(delimiter);
    let is_paired = delimiter != closing;

    // Parse first body (pattern)
    let (mut pattern, rest1, pattern_closed) = if is_paired {
        extract_substitution_pattern_with_replacement_hint(content, delimiter, closing)
    } else {
        extract_delimited_content_strict(content, delimiter, closing)
    };

    // Parse second body (replacement)
    // For paired delimiters, the replacement may use a different delimiter than the pattern
    // e.g., s[pattern]{replacement} is valid Perl
    let (replacement, modifiers_str) = if !is_paired && !rest1.is_empty() {
        // Non-paired delimiters: manually parse the replacement, skipping string literals
        // so that delimiter chars inside "foo/bar" or 'a/b' don't end the replacement early.
        let (body, rest, _found) = extract_unpaired_body_skip_strings(rest1, closing);
        (body, Cow::Borrowed(rest))
    } else if !is_paired && !pattern_closed {
        if let Some((fallback_pattern, fallback_replacement, fallback_modifiers)) =
            split_unclosed_substitution_pattern(&pattern)
        {
            pattern = fallback_pattern;
            (fallback_replacement, Cow::Owned(fallback_modifiers))
        } else {
            (String::new(), Cow::Borrowed(rest1))
        }
    } else if is_paired {
        let trimmed = rest1.trim_start();
        // For paired delimiters, check what delimiter the replacement uses
        // It may be the same as pattern or a different paired delimiter
        // e.g., s[pattern]{replacement} uses [] for pattern and {} for replacement
        if let Some(rd) = starts_with_paired_delimiter(trimmed) {
            let repl_closing = get_closing_delimiter(rd);
            let (body, rest) = extract_delimited_content(trimmed, rd, repl_closing);
            (body, Cow::Borrowed(rest))
        } else {
            let (body, rest) = extract_unpaired_body(rest1, closing);
            (body, Cow::Borrowed(rest))
        }
    } else {
        (String::new(), Cow::Borrowed(rest1))
    };

    // Extract and validate only valid substitution modifiers
    let modifiers = extract_substitution_modifiers(modifiers_str.as_ref());

    (pattern, replacement, modifiers)
}

/// Extract search, replace, and modifiers from a transliteration token
pub fn extract_transliteration_parts(text: &str) -> (String, String, String) {
    // Skip 'tr' or 'y' prefix
    let after_op = if let Some(stripped) = text.strip_prefix("tr") {
        stripped
    } else if let Some(stripped) = text.strip_prefix('y') {
        stripped
    } else {
        text
    };
    let content = after_op.trim_start();

    // Get delimiter - content must be non-empty to have a delimiter
    let delimiter = match content.chars().next() {
        Some(d) => d,
        None => return (String::new(), String::new(), String::new()),
    };
    if delimiter.is_ascii_alphanumeric() || delimiter.is_whitespace() {
        return (String::new(), String::new(), String::new());
    }
    let closing = get_closing_delimiter(delimiter);
    let is_paired = delimiter != closing;

    // Parse first body (search pattern)
    let (search, rest1) = extract_delimited_content(content, delimiter, closing);

    // For paired delimiters, skip whitespace and allow any paired opening delimiter for the
    // replacement list. Perl accepts forms like tr[abc]{xyz} in addition to tr[abc][xyz].
    let rest2_owned;
    let rest2 = if is_paired {
        rest1.trim_start()
    } else {
        rest2_owned = format!("{}{}", delimiter, rest1);
        &rest2_owned
    };

    // Parse second body (replacement pattern)
    let (replacement, modifiers_str) = if !is_paired && !rest1.is_empty() {
        // Manually parse the replacement for non-paired delimiters
        let chars = rest1.char_indices();
        let mut body = String::new();
        let mut escaped = false;
        let mut end_pos = rest1.len();

        for (i, ch) in chars {
            if escaped {
                body.push(ch);
                escaped = false;
                continue;
            }

            match ch {
                '\\' => {
                    body.push(ch);
                    escaped = true;
                }
                c if c == closing => {
                    end_pos = i + ch.len_utf8();
                    break;
                }
                _ => body.push(ch),
            }
        }

        (body, &rest1[end_pos..])
    } else if is_paired {
        if let Some(repl_delimiter) = starts_with_paired_delimiter(rest2) {
            let repl_closing = get_closing_delimiter(repl_delimiter);
            extract_delimited_content(rest2, repl_delimiter, repl_closing)
        } else {
            (String::new(), rest2)
        }
    } else {
        (String::new(), rest1)
    };

    // Extract and validate only valid transliteration modifiers
    // Security fix: Apply consistent validation for all delimiter types
    let modifiers = modifiers_str
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .filter(|&c| matches!(c, 'c' | 'd' | 's' | 'r'))
        .collect();

    (search, replacement, modifiers)
}

/// Extract search, replace, and modifiers from a transliteration token with strict validation.
///
/// Supports both `tr///` and `y///` syntax, including optional whitespace between
/// the operator and delimiter (e.g. `tr /a/b/`).
///
/// # Errors
///
/// Returns `Err(TransliterationError::InvalidModifier(c))` if an invalid modifier
/// character is encountered. Valid modifiers are: `c`, `d`, `s`, `r`.
pub fn extract_transliteration_parts_strict(
    text: &str,
) -> Result<(String, String, String), TransliterationError> {
    // Skip `tr` or `y` prefix, then allow optional whitespace before delimiter.
    let after_op = if let Some(stripped) = text.strip_prefix("tr") {
        stripped
    } else if let Some(stripped) = text.strip_prefix('y') {
        stripped
    } else {
        text
    };
    let content = after_op.trim_start();

    // Get delimiter.
    let delimiter = match content.chars().next() {
        Some(d) => d,
        None => return Err(TransliterationError::MissingDelimiter),
    };
    if delimiter.is_ascii_alphanumeric() || delimiter.is_whitespace() {
        return Err(TransliterationError::MissingDelimiter);
    }
    let closing = get_closing_delimiter(delimiter);
    let is_paired = delimiter != closing;

    // Parse first body (search).
    let (search, rest1, search_closed) =
        extract_delimited_content_strict(content, delimiter, closing);
    if !search_closed {
        return Err(TransliterationError::MissingClosingDelimiter);
    }

    // Parse second body (replacement).
    let (replacement, modifiers_str, replacement_closed) = if !is_paired {
        if rest1.is_empty() {
            return Err(TransliterationError::MissingReplacement);
        }
        let (body, rest, found_closing) = extract_unpaired_body_skip_strings(rest1, closing);
        (body, rest, found_closing)
    } else {
        let trimmed = rest1.trim_start();
        if let Some(repl_delimiter) = starts_with_paired_delimiter(trimmed) {
            let repl_closing = get_closing_delimiter(repl_delimiter);
            let (body, rest, found_closing) =
                extract_delimited_content_strict(trimmed, repl_delimiter, repl_closing);
            (body, rest, found_closing)
        } else {
            return Err(TransliterationError::MissingReplacement);
        }
    };

    if !replacement_closed {
        return Err(TransliterationError::MissingClosingDelimiter);
    }

    if search.is_empty() {
        return Err(TransliterationError::MissingSearch);
    }

    // Validate transliteration modifiers strictly.
    let mut modifiers = String::new();
    for modifier in modifiers_str.chars().take_while(|c: &char| c.is_ascii_alphanumeric()) {
        if matches!(modifier, 'c' | 'd' | 's' | 'r') {
            modifiers.push(modifier);
        } else {
            return Err(TransliterationError::InvalidModifier(modifier));
        }
    }

    Ok((search, replacement, modifiers))
}

/// Get the closing delimiter for a given opening delimiter
fn get_closing_delimiter(open: char) -> char {
    match open {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '<' => '>',
        _ => open,
    }
}

fn is_paired_open(ch: char) -> bool {
    matches!(ch, '{' | '[' | '(' | '<')
}

fn starts_with_paired_delimiter(text: &str) -> Option<char> {
    let trimmed = text.trim_start();
    match trimmed.chars().next() {
        Some(ch) if is_paired_open(ch) => Some(ch),
        _ => None,
    }
}

/// Extract content between delimiters and return (content, rest)
fn extract_delimited_content(text: &str, open: char, close: char) -> (String, &str) {
    let mut chars = text.char_indices();
    let is_paired = open != close;

    // Skip opening delimiter
    if let Some((_, c)) = chars.next() {
        if c != open {
            return (String::new(), text);
        }
    } else {
        return (String::new(), "");
    }

    let mut body = String::new();
    let mut depth = if is_paired { 1 } else { 0 };
    let mut escaped = false;
    let mut end_pos = text.len();

    for (i, ch) in chars {
        if escaped {
            body.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => {
                body.push(ch);
                escaped = true;
            }
            c if c == open && is_paired => {
                body.push(ch);
                depth += 1;
            }
            c if c == close => {
                if is_paired {
                    depth -= 1;
                    if depth == 0 {
                        end_pos = i + ch.len_utf8();
                        break;
                    }
                    body.push(ch);
                } else {
                    end_pos = i + ch.len_utf8();
                    break;
                }
            }
            _ => body.push(ch),
        }
    }

    (body, &text[end_pos..])
}

fn extract_unpaired_body(text: &str, closing: char) -> (String, &str) {
    let mut body = String::new();
    let mut escaped = false;
    let mut end_pos = text.len();

    for (i, ch) in text.char_indices() {
        if escaped {
            body.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => {
                body.push(ch);
                escaped = true;
            }
            c if c == closing => {
                end_pos = i + ch.len_utf8();
                break;
            }
            _ => body.push(ch),
        }
    }

    (body, &text[end_pos..])
}

/// Lookahead helper: determine whether a `quote` char at byte `pos` in `text` is the
/// opening of a genuine inner string literal that protects `closing` delimiter chars.
///
/// Returns `Some((end_pos, true))` when:
///   - A matching closing `quote` is found on the SAME LINE (no `\n` crossed), AND
///   - The content between the two `quote` chars contains `closing`.
///   - `end_pos` is the byte offset just after the closing `quote`.
///
/// Returns `None` (or `Some((_, false))`) when:
///   - A newline or end of `text` is reached before the matching closing `quote`, OR
///   - The string content does not contain `closing`.
///
/// Stopping at newlines prevents cross-statement false positives in multiline source.
fn scan_inner_string(
    text: &str,
    pos: usize,
    quote: char,
    delimiter: char,
) -> Option<(usize, bool)> {
    let start = pos + quote.len_utf8();
    let rest = text.get(start..)?;
    let mut escaped = false;
    let mut contains_delim = false;
    let mut end_of_string = None;
    let mut local_pos = start;
    for ch in rest.chars() {
        if escaped {
            escaped = false;
            local_pos += ch.len_utf8();
            continue;
        }
        if ch == '\\' {
            escaped = true;
            local_pos += ch.len_utf8();
            continue;
        }
        // Newline terminates the scan: inner string literals don't span lines.
        if ch == '\n' {
            return None;
        }
        if ch == delimiter {
            contains_delim = true;
        }
        if ch == quote {
            end_of_string = Some(local_pos + ch.len_utf8());
            break;
        }
        local_pos += ch.len_utf8();
    }
    end_of_string.map(|end| (end, contains_delim))
}

/// Like `extract_unpaired_body` but skips over string literals (`"..."` / `'...'`)
/// so that the closing delimiter character inside a string is not mistaken for the
/// end of the replacement section.  Returns `(body, rest, found_closing)`.
///
/// Uses lookahead to determine whether a `'` or `"` is actually an inner string:
/// only enters string-skip mode when the candidate string (a) has a matching closing
/// quote on the same line AND (b) contains the closing delimiter in its content.
/// This prevents lone apostrophes (e.g. the `'` in `s/''/'/g`) from triggering
/// string-skip, which would cause replacement scanning to cross statement boundaries.
fn extract_unpaired_body_skip_strings(text: &str, closing: char) -> (String, &str, bool) {
    let mut body = String::new();
    let mut end_pos = text.len();
    let mut found_closing = false;
    let mut pos = 0usize;
    let mut escaped = false;

    while let Some(ch) = text.get(pos..).and_then(|s| s.chars().next()) {
        if escaped {
            body.push(ch);
            escaped = false;
            pos += ch.len_utf8();
            continue;
        }

        match ch {
            '\\' => {
                body.push(ch);
                escaped = true;
                pos += ch.len_utf8();
            }
            // Skip over string literals to avoid treating delimiter chars inside
            // "foo/bar" or 'a/b' as the closing delimiter of the replacement.
            //
            // Guard: only enter string-skip when lookahead confirms a matching closing
            // quote exists on the same line AND the content contains the closing delimiter.
            '"' | '\'' if ch != closing => {
                let quote = ch;
                match scan_inner_string(text, pos, quote, closing) {
                    Some((string_end, true)) => {
                        // String content contains the closing delimiter → skip the string.
                        let string_text = &text[pos..string_end];
                        body.push_str(string_text);
                        pos = string_end;
                    }
                    _ => {
                        // No closing quote on same line, or content has no delimiter:
                        // treat the opening quote as a literal character.
                        body.push(ch);
                        pos += ch.len_utf8();
                    }
                }
            }
            c if c == closing => {
                end_pos = pos + ch.len_utf8();
                found_closing = true;
                break;
            }
            _ => {
                body.push(ch);
                pos += ch.len_utf8();
            }
        }
    }

    (body, &text[end_pos..], found_closing)
}

fn extract_substitution_pattern_with_replacement_hint(
    text: &str,
    open: char,
    close: char,
) -> (String, &str, bool) {
    let mut chars = text.char_indices();

    // Skip opening delimiter
    if let Some((_, c)) = chars.next() {
        if c != open {
            return (String::new(), text, false);
        }
    } else {
        return (String::new(), "", false);
    }

    let mut body = String::new();
    let mut depth = 1usize;
    let mut escaped = false;
    let mut first_close_pos: Option<usize> = None;
    let mut first_body_len: usize = 0;

    for (i, ch) in chars {
        if escaped {
            body.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => {
                body.push(ch);
                escaped = true;
            }
            c if c == open => {
                body.push(ch);
                depth += 1;
            }
            c if c == close => {
                if depth > 1 {
                    depth -= 1;
                    body.push(ch);
                    continue;
                }

                let rest = &text[i + ch.len_utf8()..];
                if first_close_pos.is_none() {
                    first_close_pos = Some(i + ch.len_utf8());
                    first_body_len = body.len();
                }

                if starts_with_paired_delimiter(rest).is_some() {
                    return (body, rest, true);
                }

                body.push(ch);
            }
            _ => body.push(ch),
        }
    }

    if let Some(pos) = first_close_pos {
        body.truncate(first_body_len);
        return (body, &text[pos..], true);
    }

    (body, "", false)
}

fn split_unclosed_substitution_pattern(pattern: &str) -> Option<(String, String, String)> {
    let mut escaped = false;

    for (idx, ch) in pattern.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        if is_paired_open(ch) {
            let closing = get_closing_delimiter(ch);
            let (replacement, rest, found_closing) =
                extract_delimited_content_strict(&pattern[idx..], ch, closing);
            if found_closing {
                let leading = pattern[..idx].to_string();
                return Some((leading, replacement, rest.to_string()));
            }
        }
    }

    None
}

fn split_on_last_paired_delimiter(text: &str) -> Option<(String, String, String)> {
    let mut escaped = false;
    let mut candidates = Vec::new();

    for (idx, ch) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        if is_paired_open(ch) {
            candidates.push((idx, ch));
        }
    }

    for (idx, ch) in candidates.into_iter().rev() {
        let closing = get_closing_delimiter(ch);
        let (replacement, rest, found_closing) =
            extract_delimited_content_strict(&text[idx..], ch, closing);
        if found_closing {
            let leading = text[..idx].to_string();
            return Some((leading, replacement, rest.to_string()));
        }
    }

    None
}

/// Extract and validate substitution modifiers, returning only valid ones
///
/// Valid Perl substitution modifiers include:
/// - Core modifiers: g, i, m, s, x, o, e, r
/// - Charset modifiers (Perl 5.14+): a, d, l, u
/// - Additional modifiers: n (5.22+), p, c
///
/// This function provides panic-safe modifier validation for substitution operators,
/// filtering out invalid modifiers to prevent security vulnerabilities.
fn extract_substitution_modifiers(text: &str) -> String {
    text.chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .filter(|&c| {
            matches!(
                c,
                'g' | 'i'
                    | 'm'
                    | 's'
                    | 'x'
                    | 'o'
                    | 'e'
                    | 'r'
                    | 'a'
                    | 'd'
                    | 'l'
                    | 'u'
                    | 'n'
                    | 'p'
                    | 'c'
            )
        })
        .collect()
}

/// Validate substitution modifiers and return an error if any are invalid
///
/// Valid Perl substitution modifiers include:
/// - Core modifiers: g, i, m, s, x, o, e, r
/// - Charset modifiers (Perl 5.14+): a, d, l, u
/// - Additional modifiers: n (5.22+), p, c
///
/// # Arguments
///
/// * `modifiers_str` - The raw modifier string following the substitution operator
///
/// # Returns
///
/// * `Ok(String)` - The validated modifiers if all are valid
/// * `Err(char)` - The first invalid modifier character encountered
///
/// # Examples
///
/// ```ignore
/// assert!(validate_substitution_modifiers("gi").is_ok());
/// assert!(validate_substitution_modifiers("gia").is_ok());  // 'a' for ASCII mode
/// assert!(validate_substitution_modifiers("giz").is_err()); // 'z' is invalid
/// ```
pub fn validate_substitution_modifiers(modifiers_str: &str) -> Result<String, char> {
    let mut valid_modifiers = String::new();

    for c in modifiers_str.chars() {
        // Stop at non-alphabetic characters (end of modifiers)
        if !c.is_ascii_alphabetic() {
            // If it's whitespace or end of input, that's ok
            if c.is_whitespace() || c == ';' || c == '\n' || c == '\r' {
                break;
            }
            // Non-alphabetic, non-whitespace character in modifier position is invalid
            return Err(c);
        }

        // Check if it's a valid substitution modifier
        if matches!(
            c,
            'g' | 'i' | 'm' | 's' | 'x' | 'o' | 'e' | 'r' | 'a' | 'd' | 'l' | 'u' | 'n' | 'p' | 'c'
        ) {
            valid_modifiers.push(c);
        } else {
            // Invalid alphabetic modifier
            return Err(c);
        }
    }

    Ok(valid_modifiers)
}
