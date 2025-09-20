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

/// Extract pattern, replacement, and modifiers from a substitution token
///
/// This function parses substitution operators like s/pattern/replacement/flags
/// and handles various delimiter forms including:
/// - Non-paired delimiters: s/pattern/replacement/ (same delimiter for all parts)
/// - Paired delimiters: s{pattern}{replacement} (different open/close delimiters)
///
/// For paired delimiters, properly handles nested delimiters within the pattern
/// or replacement parts. Returns (pattern, replacement, modifiers) as strings.
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

    // For paired delimiters, skip whitespace and expect new delimiter
    let rest2_owned;
    let rest2 = if is_paired {
        let trimmed = rest1.trim_start();
        // For paired delimiters like s{pattern}{replacement}, we expect another opening delimiter
        if trimmed.starts_with(delimiter) {
            // Keep the delimiter - don't strip it here since extract_delimited_content expects it
            trimmed
        } else {
            // If no second delimiter found, the replacement is empty
            ""
        }
    } else {
        rest2_owned = format!("{}{}", delimiter, rest1);
        &rest2_owned
    };

    // Parse second body (replacement)
    // For non-paired delimiters, we need special handling
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

    // Extract only alphabetic modifiers
    let modifiers = extract_modifiers(modifiers_str);

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
            // If no second delimiter found, fall back to original rest
            rest1
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

    // Extract only valid transliteration modifiers
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

/// Extract only alphabetic characters as modifiers
fn extract_modifiers(text: &str) -> String {
    text.chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .filter(|&c| matches!(c, 'g' | 'i' | 'm' | 's' | 'x' | 'o' | 'e' | 'r'))
        .collect()
}
