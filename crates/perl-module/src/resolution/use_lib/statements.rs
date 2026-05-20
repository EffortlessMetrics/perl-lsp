//! Statement splitting and `lib` pragma prefix recognition.

/// Split Perl source into semicolon-terminated statements without treating
/// semicolons inside simple quoted strings or line comments as terminators.
pub(super) fn split_perl_statements(source: &str) -> Vec<&str> {
    let mut statements = Vec::new();
    let mut start = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    // Whether any non-whitespace, non-comment content has appeared in the
    // current statement since `start`.  When false and we hit a comment, we
    // can safely advance `start` past the comment so it doesn't pollute the
    // next statement slice.
    let mut has_content = false;

    let chars: Vec<(usize, char)> = source.char_indices().collect();
    let mut i = 0;

    while i < chars.len() {
        let (idx, ch) = chars[i];

        if escaped {
            escaped = false;
            i += 1;
            continue;
        }

        if ch == '\\' && (in_single || in_double) {
            escaped = true;
            i += 1;
            continue;
        }

        if ch == '\'' && !in_double {
            in_single = !in_single;
            has_content = true;
            i += 1;
            continue;
        }

        if ch == '"' && !in_single {
            in_double = !in_double;
            has_content = true;
            i += 1;
            continue;
        }

        // Skip Perl line comments: # ... <newline>
        // A `#` is only a comment when outside of any string literal.
        if ch == '#' && !in_single && !in_double {
            // Skip to end of line (or end of source).
            let comment_end = match source[idx..].find('\n') {
                Some(nl_offset) => idx + nl_offset + 1,
                None => source.len(),
            };
            // If no statement content has been seen yet, advance `start` past
            // the comment so the comment text is not included in the next slice.
            if !has_content {
                start = comment_end;
            }
            // Skip the iterator past the comment.
            while i < chars.len() && chars[i].0 < comment_end {
                i += 1;
            }
            continue;
        }

        if ch == ';' && !in_single && !in_double {
            let end = idx + ch.len_utf8();
            statements.push(&source[start..end]);
            start = end;
            has_content = false;
        } else if !ch.is_whitespace() {
            has_content = true;
        }

        i += 1;
    }

    if start < source.len() {
        statements.push(&source[start..]);
    }

    statements
}

pub(super) fn strip_use_lib_prefix(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix("use")?;
    if !rest.starts_with(|c: char| c.is_whitespace()) {
        return None;
    }
    let rest = rest.trim_start();
    let rest = rest.strip_prefix("lib")?;
    if !rest.starts_with(|c: char| c.is_whitespace() || c == '(' || c == ';') {
        return None;
    }
    Some(rest.trim_start())
}

pub(super) fn strip_no_lib_prefix(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix("no")?;
    if !rest.starts_with(|c: char| c.is_whitespace()) {
        return None;
    }
    let rest = rest.trim_start();
    let rest = rest.strip_prefix("lib")?;
    if !rest.starts_with(|c: char| c.is_whitespace() || c == '(' || c == ';') {
        return None;
    }
    Some(rest.trim_start())
}
