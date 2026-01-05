/// Uniform quote operator parsing for the parser
///
/// This module provides consistent parsing for quote-like operators,
/// properly extracting patterns, bodies, and modifiers.
///
/// Extract pattern and modifiers from a regex-like token (qr, m, or bare //)
pub fn extract_regex_parts(text: &str) -> (String, String) {
    // Handle different prefixes
    let content = if let Some(stripped) = text.strip_prefix("qr") {
        stripped
    } else if text.starts_with('m')
        && text.len() > 1
        && !text.chars().nth(1).unwrap().is_alphabetic()
    {
        &text[1..]
    } else {
        text
    };

    if content.is_empty() {
        return (String::new(), String::new());
    }

    // Get delimiter
    let delimiter = content.chars().next().unwrap();
    let closing = get_closing_delimiter(delimiter);

    // Extract body and modifiers
    let (body, modifiers) = extract_delimited_content(content, delimiter, closing);

    // Include delimiters in the pattern string for compatibility
    let pattern = format!("{}{}{}", delimiter, body, closing);

    (pattern, modifiers.to_string())
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
    let content = text.strip_prefix('s').unwrap_or(text);

    // Check for missing delimiter (just 's' or 's' followed by nothing)
    if content.is_empty() {
        return Err(SubstitutionError::MissingDelimiter);
    }

    let delimiter = content.chars().next().unwrap();
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

        // Manually parse the replacement
        let chars = rest1.char_indices();
        let mut body = String::new();
        let mut escaped = false;
        let mut end_pos = rest1.len();
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
                c if c == closing => {
                    end_pos = i + ch.len_utf8();
                    found_closing = true;
                    break;
                }
                _ => body.push(ch),
            }
        }

        (body, &rest1[end_pos..], found_closing)
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

    if content.is_empty() {
        return (String::new(), String::new(), String::new());
    }

    let delimiter = content.chars().next().unwrap();
    let closing = get_closing_delimiter(delimiter);
    let is_paired = delimiter != closing;

    // Parse first body (pattern)
    let (pattern, rest1) = extract_delimited_content(content, delimiter, closing);

    // Parse second body (replacement)
    // For paired delimiters, the replacement may use a different delimiter than the pattern
    // e.g., s[pattern]{replacement} is valid Perl
    let (replacement, modifiers_str) = if !is_paired && !rest1.is_empty() {
        // Non-paired delimiters: manually parse the replacement
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
        let trimmed = rest1.trim_start();
        // For paired delimiters, check what delimiter the replacement uses
        // It may be the same as pattern or a different paired delimiter
        // e.g., s[pattern]{replacement} uses [] for pattern and {} for replacement
        if let Some(rd) = trimmed.chars().next() {
            // Check if it's a valid paired opening delimiter
            if rd == '{' || rd == '[' || rd == '(' || rd == '<' {
                let repl_closing = get_closing_delimiter(rd);
                extract_delimited_content(trimmed, rd, repl_closing)
            } else {
                // Not a valid paired delimiter - malformed, return empty replacement
                (String::new(), trimmed)
            }
        } else {
            // No more content - empty replacement
            (String::new(), "")
        }
    } else {
        (String::new(), rest1)
    };

    // Extract and validate only valid substitution modifiers
    let modifiers = extract_substitution_modifiers(modifiers_str);

    (pattern, replacement, modifiers)
}

/// Extract search, replace, and modifiers from a transliteration token
pub fn extract_transliteration_parts(text: &str) -> (String, String, String) {
    // Skip 'tr' or 'y' prefix
    let content = if let Some(stripped) = text.strip_prefix("tr") {
        stripped
    } else if let Some(stripped) = text.strip_prefix('y') {
        stripped
    } else {
        text
    };

    if content.is_empty() {
        return (String::new(), String::new(), String::new());
    }

    let delimiter = content.chars().next().unwrap();
    let closing = get_closing_delimiter(delimiter);
    let is_paired = delimiter != closing;

    // Parse first body (search pattern)
    let (search, rest1) = extract_delimited_content(content, delimiter, closing);

    // For paired delimiters, skip whitespace and expect new delimiter
    let rest2_owned;
    let rest2 = if is_paired {
        let trimmed = rest1.trim_start();
        // For paired delimiters like tr{search}{replace}, we expect another opening delimiter
        if trimmed.starts_with(delimiter) {
            // Keep the delimiter - don't strip it since extract_delimited_content expects it
            trimmed
        } else {
            // If no second delimiter found, the replacement is empty
            ""
        }
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
        extract_delimited_content(rest2, delimiter, closing)
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
        .filter(|&c| matches!(c, 'g' | 'i' | 'm' | 's' | 'x' | 'o' | 'e' | 'r' | 'a' | 'd' | 'l' | 'u' | 'n' | 'p' | 'c'))
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
        if matches!(c, 'g' | 'i' | 'm' | 's' | 'x' | 'o' | 'e' | 'r' | 'a' | 'd' | 'l' | 'u' | 'n' | 'p' | 'c') {
            valid_modifiers.push(c);
        } else {
            // Invalid alphabetic modifier
            return Err(c);
        }
    }

    Ok(valid_modifiers)
}
