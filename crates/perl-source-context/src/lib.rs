//! Shared source-text context heuristics for Perl tooling.

/// Returns `true` when `position` appears inside a quoted string literal.
///
/// This uses a lightweight heuristic based on unmatched single or double quotes.
pub fn is_in_string(source: &str, position: usize) -> bool {
    let before = &source[..position];
    let single_quotes = before.matches('\'').count();
    let double_quotes = before.matches('"').count();

    single_quotes % 2 == 1 || double_quotes % 2 == 1
}

/// Returns `true` when `position` appears inside a line comment (`# ...`).
///
/// This uses a lightweight line-scoped heuristic without full Perl parsing.
pub fn is_in_comment(source: &str, position: usize) -> bool {
    let line_start = if position == 0 {
        0
    } else {
        source[..position].rfind('\n').map_or(0, |line_break| line_break + 1)
    };
    let line = &source[line_start..position];

    line.contains('#')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_string_context() {
        let src = "my $x = \"hello\";";
        let pos = src.find("hello").unwrap_or(0);
        assert!(is_in_string(src, pos));
    }

    #[test]
    fn detects_comment_context() {
        let src = "my $x = 1; # comment";
        let pos = src.find("comment").unwrap_or(0);
        assert!(is_in_comment(src, pos));
    }

    #[test]
    fn ignores_non_comment_prefix() {
        let src = "my $x = 1;\nmy $y = 2;";
        let pos = src.find("$y").unwrap_or(0);
        assert!(!is_in_comment(src, pos));
    }
}
