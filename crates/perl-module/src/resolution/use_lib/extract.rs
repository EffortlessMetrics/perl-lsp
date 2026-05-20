//! Extract path arguments from `use lib` and `no lib` pragma arguments.

use super::UseLibPath;

pub(super) fn extract_paths_from_args(args: &str, out: &mut Vec<UseLibPath>) {
    let args = args.trim_end_matches(';').trim();

    if let Some(rest) = args.strip_prefix("qw") {
        extract_qw_paths(rest.trim_start(), out);
        return;
    }

    if let Some(inner) = strip_parens(args) {
        extract_quoted_list(inner, out);
        return;
    }

    extract_quoted_list(args, out);
}

fn extract_qw_paths(rest: &str, out: &mut Vec<UseLibPath>) {
    let (open, close) = match rest.chars().next() {
        Some('(') => ('(', ')'),
        Some('/') => ('/', '/'),
        Some('{') => ('{', '}'),
        Some('[') => ('[', ']'),
        Some('<') => ('<', '>'),
        Some('!') => ('!', '!'),
        _ => return,
    };

    let inner = &rest[open.len_utf8()..];
    let end = inner.find(close).unwrap_or(inner.len());
    let content = &inner[..end];

    for word in content.split_whitespace() {
        out.push(UseLibPath { path: word.to_string(), from_findbin: false });
    }
}

fn strip_parens(s: &str) -> Option<&str> {
    let s = s.trim();
    let inner = s.strip_prefix('(')?;
    let inner = inner.trim_end().strip_suffix(')')?;
    Some(inner)
}

fn extract_quoted_list(s: &str, out: &mut Vec<UseLibPath>) {
    let mut remaining = s.trim();

    while !remaining.is_empty() {
        remaining = remaining.trim_start_matches(|c: char| c == ',' || c.is_whitespace());
        if remaining.is_empty() {
            break;
        }

        // Skip Perl line comments: # ... <newline>
        if remaining.starts_with('#') {
            remaining = match remaining.find('\n') {
                Some(nl) => &remaining[nl + 1..],
                None => "",
            };
            continue;
        }

        if let Some((path, from_findbin, rest)) = extract_one_quoted(remaining) {
            out.push(UseLibPath { path, from_findbin });
            remaining = rest.trim_start_matches(|c: char| c == ',' || c.is_whitespace());
        } else {
            break;
        }
    }
}

fn extract_one_quoted(s: &str) -> Option<(String, bool, &str)> {
    let s = s.trim();
    let quote = match s.chars().next()? {
        '\'' => '\'',
        '"' => '"',
        _ => return None,
    };

    let inner = &s[1..];
    let end = find_unescaped_quote(inner, quote)?;
    let content = &inner[..end];
    let rest = &inner[end + quote.len_utf8()..];

    let (path, from_findbin) = resolve_findbin_in_string(content);
    Some((path, from_findbin, rest))
}

fn find_unescaped_quote(s: &str, quote: char) -> Option<usize> {
    let mut escaped = false;

    for (idx, ch) in s.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        if ch == quote {
            return Some(idx);
        }
    }

    None
}

fn resolve_findbin_in_string(s: &str) -> (String, bool) {
    // Fully-qualified FindBin variables — no word-boundary ambiguity because `::` terminates
    // the name and braced forms are unambiguous.
    let qualified_vars =
        ["$FindBin::Bin", "$FindBin::RealBin", "${FindBin::Bin}", "${FindBin::RealBin}"];

    for var in &qualified_vars {
        if let Some(rest) = s.strip_prefix(var) {
            let path = rest.strip_prefix('/').unwrap_or(rest);
            if path.is_empty() {
                return (".".to_string(), true);
            }
            return (path.to_string(), true);
        }
    }

    // Short exported forms: `$Bin`, `$RealBin`, `${Bin}`, `${RealBin}`.
    // Braced forms (`${Bin}`) are always unambiguous.  Bare forms (`$Bin`,
    // `$RealBin`) must be followed by `/`, end-of-string, or a non-identifier
    // character to avoid false-positives on variables like `$BinDir` or
    // `$RealBinPath`.
    let bare_short = ["$Bin", "$RealBin"];
    let braced_short = ["${Bin}", "${RealBin}"];

    for var in &bare_short {
        if let Some(rest) = s.strip_prefix(var) {
            // Word-boundary check: the character after the variable name must
            // not be a Perl identifier character (letter, digit, or `_`).
            // This prevents `$BinDir` or `$RealBinPath` from matching `$Bin`/`$RealBin`.
            let next = rest.chars().next();
            if next.is_none() || next.is_some_and(|c| !c.is_alphanumeric() && c != '_') {
                let path = rest.strip_prefix('/').unwrap_or(rest);
                if path.is_empty() {
                    return (".".to_string(), true);
                }
                return (path.to_string(), true);
            }
        }
    }

    for var in &braced_short {
        if let Some(rest) = s.strip_prefix(var) {
            let path = rest.strip_prefix('/').unwrap_or(rest);
            if path.is_empty() {
                return (".".to_string(), true);
            }
            return (path.to_string(), true);
        }
    }

    (s.to_string(), false)
}
