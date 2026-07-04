//! Import facts: `use` / `no` / `require` and what they bring in.

use serde::{Deserialize, Serialize};

use crate::id::FileId;
use crate::provenance::Confidence;
use crate::range::SourceRange;

/// Which import form a fact came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportKind {
    /// `use Foo ...;`
    Use,
    /// `no Foo ...;`
    No,
    /// `require Foo;` / `require "foo.pl";` (a static, bareword/string require).
    Require,
}

/// A single import statement fact.
///
/// Runtime `require $expr` and `eval "..."` are **not** imports — they are
/// recorded as [`DynamicBoundary`](crate::boundary::DynamicBoundary)s instead,
/// because static analysis cannot see what they load.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportFact {
    /// The file containing the statement.
    pub file_id: FileId,
    /// Which import form.
    pub kind: ImportKind,
    /// Module or pragma name (e.g. `strict`, `Foo::Bar`). Empty for a bare
    /// version requirement like `use v5.38`.
    pub module: String,
    /// A version requirement attached to the statement, if any (`use Foo 1.23`,
    /// `use v5.38`).
    pub version: Option<String>,
    /// Requested import symbols / arguments, with `qw(...)` and quotes
    /// normalized away (e.g. `use POSIX qw(floor ceil)` → `["floor", "ceil"]`).
    pub imports: Vec<String>,
    /// Whether the module name looks like a pragma (lowercase first segment:
    /// `strict`, `warnings`, `feature`, `parent`, `lib`, …).
    pub is_pragma: bool,
    /// Span of the statement.
    pub range: SourceRange,
    /// Confidence in the fact.
    pub confidence: Confidence,
}

/// Split a `NodeKind::Use` module string into `(module_name, version)`.
///
/// The parser appends a numeric version onto the module field, so
/// `use Foo 1.23` arrives as `module == "Foo 1.23"`; and a bare version use
/// (`use v5.38` / `use 5.036`) puts the whole version in the module field with
/// no name. This untangles both.
#[must_use]
pub fn split_module_version(module: &str) -> (String, Option<String>) {
    let mut parts = module.split_whitespace();
    let first = parts.next().unwrap_or("");
    if is_version_token(first) {
        // Bare version requirement: `use v5.38;` — no module name.
        return (String::new(), Some(first.to_string()));
    }
    let version = parts.next().filter(|v| is_version_token(v)).map(str::to_string);
    (first.to_string(), version)
}

/// True if a token looks like a Perl version (`v5.38`, `5.036`, `1.23`).
#[must_use]
pub fn is_version_token(token: &str) -> bool {
    let digits = token.strip_prefix('v').unwrap_or(token);
    digits.starts_with(|c: char| c.is_ascii_digit())
}

/// Normalize a raw `use`/`no` argument token into zero or more import names.
///
/// Handles `qw(a b)` / `qw[a b]` / `qw{a b}` / `qw<a b>` expansion, strips
/// surrounding quotes, and skips flag args like `-norequire`.
#[must_use]
pub fn normalize_import_arg(arg: &str) -> Vec<String> {
    let arg = arg.trim();
    // qw(...) and its delimiter variants — but only when `qw` is immediately
    // followed by a real delimiter, so a bareword like `qword` or `qwerty` is
    // NOT mistaken for a quote-word list.
    if let Some(rest) = arg.strip_prefix("qw") {
        if rest.starts_with(|c: char| !c.is_alphanumeric() && c != '_') {
            let inner = rest
                .trim_start_matches(['(', '[', '{', '<', '/', '!', '|'])
                .trim_end_matches([')', ']', '}', '>', '/', '!', '|']);
            return inner.split_whitespace().map(str::to_string).collect();
        }
        // Otherwise fall through: `qword` is an ordinary import symbol.
    }
    // Flags like `-norequire` are not imported symbols.
    if arg.starts_with('-') {
        return Vec::new();
    }
    let cleaned = strip_quotes(arg);
    if cleaned.is_empty() { Vec::new() } else { vec![cleaned] }
}

/// Strip one layer of matching surrounding quotes from a token.
#[must_use]
pub fn strip_quotes(token: &str) -> String {
    let token = token.trim();
    for (open, close) in [('\'', '\''), ('"', '"')] {
        if token.len() >= 2 && token.starts_with(open) && token.ends_with(close) {
            return token[1..token.len() - 1].to_string();
        }
    }
    token.to_string()
}

/// True when a module name looks like a pragma (lowercase first segment).
#[must_use]
pub fn looks_like_pragma(module: &str) -> bool {
    module.chars().next().is_some_and(|c| c.is_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_trailing_version() {
        assert_eq!(split_module_version("Foo 1.23"), ("Foo".to_string(), Some("1.23".to_string())));
        assert_eq!(split_module_version("Foo::Bar"), ("Foo::Bar".to_string(), None));
    }

    #[test]
    fn splits_bare_version_use() {
        assert_eq!(split_module_version("v5.38"), (String::new(), Some("v5.38".to_string())));
        assert_eq!(split_module_version("5.036"), (String::new(), Some("5.036".to_string())));
    }

    #[test]
    fn normalizes_qw_and_quotes() {
        assert_eq!(normalize_import_arg("qw(floor ceil)"), vec!["floor", "ceil"]);
        assert_eq!(normalize_import_arg("'Base'"), vec!["Base"]);
        assert_eq!(normalize_import_arg("\"say\""), vec!["say"]);
        assert!(normalize_import_arg("-norequire").is_empty());
        assert_eq!(normalize_import_arg("say"), vec!["say"]);
    }

    #[test]
    fn qw_prefix_only_triggers_on_a_real_delimiter() {
        // Regression: a bareword symbol beginning with `qw` must not be treated
        // as a quote-word list and have its first two letters stripped.
        assert_eq!(normalize_import_arg("qword"), vec!["qword"]);
        assert_eq!(normalize_import_arg("qwerty"), vec!["qwerty"]);
        assert_eq!(normalize_import_arg("qw_helper"), vec!["qw_helper"]);
        // But a genuine qw list still expands.
        assert_eq!(normalize_import_arg("qw(a b)"), vec!["a", "b"]);
    }

    #[test]
    fn pragma_detection() {
        assert!(looks_like_pragma("strict"));
        assert!(looks_like_pragma("warnings"));
        assert!(looks_like_pragma("parent"));
        assert!(!looks_like_pragma("Foo::Bar"));
        assert!(!looks_like_pragma("Moose"));
    }

    #[test]
    fn version_token_recognition() {
        assert!(is_version_token("v5.38"));
        assert!(is_version_token("5.036"));
        assert!(is_version_token("1.23"));
        assert!(!is_version_token("Foo"));
        assert!(!is_version_token("strict"));
    }
}
