//! String utility functions for common patterns

/// Strip enclosing delimiters from a string
pub fn strip_enclosing(s: &str, left: char, right: char) -> Option<&str> {
    s.strip_prefix(left).and_then(|t| t.strip_suffix(right))
}

/// Remove quotes if the string is quoted (either single or double)
pub fn unquote_if_quoted(s: &str) -> Option<&str> {
    strip_enclosing(s, '"', '"').or_else(|| strip_enclosing(s, '\'', '\''))
}

/// Strip any of the given delimiter pairs from a string
pub fn strip_any<'a>(s: &'a str, pairs: &[(char, char)]) -> Option<&'a str> {
    for (l, r) in pairs {
        if let Some(inner) = strip_enclosing(s, *l, *r) {
            return Some(inner);
        }
    }
    None
}

/// Check if a string is enclosed by the given delimiters
pub fn is_enclosed(s: &str, left: char, right: char) -> bool {
    s.starts_with(left) && s.ends_with(right) && s.len() >= 2
}
