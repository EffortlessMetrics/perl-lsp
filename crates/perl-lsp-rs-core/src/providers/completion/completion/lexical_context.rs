/// Simple heuristic to check if position is in a string.
pub(super) fn is_in_string(source: &str, position: usize) -> bool {
    let before = &source[..position];
    let single_quotes = before.matches('\'').count();
    let double_quotes = before.matches('"').count();

    single_quotes % 2 == 1 || double_quotes % 2 == 1
}

/// Heuristic to check if position is inside a regex literal.
pub(super) fn is_in_regex(source: &str, position: usize) -> bool {
    let before = &source[..position];

    let Some(last_slash) = before.rfind('/') else {
        return false;
    };

    let pre_slash = before[..last_slash].trim_end();
    if pre_slash.ends_with("=~") || pre_slash.ends_with("!~") {
        return true;
    }

    if pre_slash_has_regex_op(pre_slash) {
        return true;
    }

    if matches!(
        pre_slash.split_ascii_whitespace().next_back(),
        Some("or") | Some("and") | Some("not")
    ) {
        return true;
    }

    if let Some(last_char) = pre_slash.chars().next_back()
        && matches!(last_char, '(' | ',' | '=' | '!' | '&' | '|' | ';' | '{' | '~')
    {
        return true;
    }

    pre_slash.is_empty()
}

/// Return true when the text immediately before a `/` is one of the explicit regex operators.
fn pre_slash_has_regex_op(pre_slash: &str) -> bool {
    let trimmed = pre_slash.trim_end();
    for op in &["qr", "m", "s", "tr", "y"] {
        if let Some(before_op) = trimmed.strip_suffix(op) {
            let boundary_ok = before_op
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_');
            if boundary_ok {
                return true;
            }
        }
    }
    false
}

/// Return true when the cursor is positioned in the flag region after a closing regex delimiter.
pub(crate) fn is_in_regex_flags(source: &str, position: usize) -> bool {
    if position == 0 || position > source.len() {
        return false;
    }
    let before = &source[..position];
    let flag_chars: &[char] = &['g', 'i', 'm', 's', 'x', 'e', 'r', 'a', 'd', 'u', 'p', 'l', 'c'];
    let without_flags = before.trim_end_matches(|c: char| flag_chars.contains(&c));
    let close_pos = without_flags.len();
    if close_pos >= 2 && without_flags.ends_with('/') && is_in_regex(source, close_pos - 1) {
        return true;
    }

    let body = without_flags.trim();
    is_multi_delim_regex_at_close(body)
}

fn is_multi_delim_regex_at_close(text: &str) -> bool {
    let (op_len, required_slashes) = if text.starts_with("tr/") || text.starts_with("y/") {
        let op = if text.starts_with("tr/") { 2 } else { 1 };
        (op, 3usize)
    } else if text.starts_with("s/") {
        (1, 3usize)
    } else if text.starts_with("m") || text.starts_with("qr") {
        return is_m_or_qr_closed(text);
    } else {
        let stripped = text
            .find("=~")
            .map(|p| text[p + 2..].trim_start())
            .or_else(|| text.find("!~").map(|p| text[p + 2..].trim_start()));
        if let Some(rhs) = stripped {
            return is_multi_delim_regex_at_close(rhs);
        }
        return false;
    };
    let body_after_op = &text[op_len..];
    let slash_count = count_unescaped_slashes(body_after_op);
    slash_count == required_slashes
}

fn is_m_or_qr_closed(text: &str) -> bool {
    let (op_len, delim_pos) =
        if text.starts_with("qr") { (2usize, 2usize) } else { (1usize, 1usize) };
    let Some(delim) = text[delim_pos..].chars().next() else {
        return false;
    };
    if delim.is_ascii_alphanumeric() || delim.is_ascii_whitespace() {
        return false;
    }
    let close_delim = matching_close_delimiter(delim);
    let body = &text[(op_len + delim.len_utf8())..];
    body.ends_with(close_delim)
}

fn matching_close_delimiter(open: char) -> char {
    match open {
        '(' => ')',
        '{' => '}',
        '[' => ']',
        '<' => '>',
        _ => open,
    }
}

fn count_unescaped_slashes(s: &str) -> usize {
    let mut count = 0usize;
    let mut escaped = false;
    for ch in s.chars() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '/' {
            count += 1;
        }
    }
    count
}

/// Simple heuristic to check if position is in a comment.
pub(super) fn is_in_comment(source: &str, position: usize) -> bool {
    let line_start = source[..position].rfind('\n').map_or(0, |p| p + 1);
    let line = &source[line_start..position];
    line.contains('#')
}
