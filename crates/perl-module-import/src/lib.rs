//! Single-line Perl import head parsing.
//!
//! This crate provides one narrow responsibility: parse a single source line
//! that starts with `use` or `require` and return the first import token with
//! stable byte offsets.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

/// Classifies the import statement form for a parsed line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleImportKind {
    /// `use Module::Name;`
    Use,
    /// `require Module::Name;`
    Require,
    /// `use parent ...`
    UseParent,
    /// `use base ...`
    UseBase,
}

/// Parsed leading import token from a `use`/`require` line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleImportHead<'a> {
    /// Parsed statement kind.
    pub kind: ModuleImportKind,
    /// First token after `use` or `require`.
    pub token: &'a str,
    /// Inclusive byte start offset of `token` in the full line.
    pub token_start: usize,
    /// Exclusive byte end offset of `token` in the full line.
    pub token_end: usize,
}

/// Parse the leading import token of a single Perl source line.
///
/// Returns [`None`] when the line does not start with `use` or `require`
/// (after leading whitespace) or when no token is present after the keyword.
///
/// # Examples
///
/// ```
/// use perl_module_import::{ModuleImportKind, parse_module_import_head};
///
/// let parsed = parse_module_import_head("use Foo::Bar;");
/// assert_eq!(parsed.map(|head| head.kind), Some(ModuleImportKind::Use));
/// assert_eq!(parsed.map(|head| head.token), Some("Foo::Bar"));
///
/// let parsed = parse_module_import_head("use parent 'Foo::Bar';");
/// assert_eq!(parsed.map(|head| head.kind), Some(ModuleImportKind::UseParent));
/// assert_eq!(parsed.map(|head| head.token), Some("parent"));
/// ```
#[must_use]
pub fn parse_module_import_head(line: &str) -> Option<ModuleImportHead<'_>> {
    if let Some((token, token_start, token_end)) = parse_statement_head(line, "use") {
        let kind = match token {
            "parent" => ModuleImportKind::UseParent,
            "base" => ModuleImportKind::UseBase,
            _ => ModuleImportKind::Use,
        };
        return Some(ModuleImportHead { kind, token, token_start, token_end });
    }

    if let Some((token, token_start, token_end)) = parse_statement_head(line, "require") {
        return Some(ModuleImportHead {
            kind: ModuleImportKind::Require,
            token,
            token_start,
            token_end,
        });
    }

    None
}

fn parse_statement_head<'a>(line: &'a str, keyword: &str) -> Option<(&'a str, usize, usize)> {
    let trimmed = line.trim_start();
    let leading = line.len().saturating_sub(trimmed.len());

    let rest = trimmed.strip_prefix(keyword)?;
    if !rest.chars().next().is_some_and(char::is_whitespace) {
        return None;
    }

    let (token, token_rel_start, token_rel_end) = first_token_with_range(rest)?;
    let token_start = leading + keyword.len() + token_rel_start;
    let token_end = leading + keyword.len() + token_rel_end;

    Some((token, token_start, token_end))
}

fn first_token_with_range(input: &str) -> Option<(&str, usize, usize)> {
    let mut token_start = None;

    for (idx, ch) in input.char_indices() {
        match token_start {
            None => {
                if is_token_delimiter(ch) {
                    continue;
                }
                token_start = Some(idx);
            }
            Some(start) => {
                if is_token_delimiter(ch) {
                    if start == idx {
                        return None;
                    }
                    return Some((&input[start..idx], start, idx));
                }
            }
        }
    }

    if let Some(start) = token_start {
        if start < input.len() { Some((&input[start..], start, input.len())) } else { None }
    } else {
        None
    }
}

fn is_token_delimiter(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, ';' | '(' | ')')
}

#[cfg(test)]
mod tests {
    use super::{ModuleImportKind, parse_module_import_head};

    #[test]
    fn parses_use_statement_head() {
        let parsed = parse_module_import_head("use Foo::Bar;");

        assert!(parsed.is_some());
        if let Some(head) = parsed {
            assert_eq!(head.kind, ModuleImportKind::Use);
            assert_eq!(head.token, "Foo::Bar");
            assert_eq!(head.token_start, 4);
            assert_eq!(head.token_end, 12);
        }
    }

    #[test]
    fn parses_require_statement_head() {
        let parsed = parse_module_import_head("  require Foo::Bar;");

        assert!(parsed.is_some());
        if let Some(head) = parsed {
            assert_eq!(head.kind, ModuleImportKind::Require);
            assert_eq!(head.token, "Foo::Bar");
            assert_eq!(head.token_start, 10);
            assert_eq!(head.token_end, 18);
        }
    }

    #[test]
    fn classifies_parent_and_base_specializations() {
        let parent = parse_module_import_head("use parent qw(Foo::Bar);");
        let base = parse_module_import_head("use base 'Foo::Bar';");

        assert!(parent.is_some());
        if let Some(head) = parent {
            assert_eq!(head.kind, ModuleImportKind::UseParent);
            assert_eq!(head.token, "parent");
        }

        assert!(base.is_some());
        if let Some(head) = base {
            assert_eq!(head.kind, ModuleImportKind::UseBase);
            assert_eq!(head.token, "base");
        }
    }

    #[test]
    fn rejects_non_keyword_boundaries() {
        assert!(parse_module_import_head("user Foo::Bar;").is_none());
        assert!(parse_module_import_head("required Foo::Bar;").is_none());
    }

    #[test]
    fn rejects_missing_tokens() {
        assert!(parse_module_import_head("use ;").is_none());
        assert!(parse_module_import_head("require").is_none());
    }
}
