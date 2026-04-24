//! Visible import surface extraction for a single Perl file.
//!
//! Consolidates import visibility into a single canonical record so downstream
//! consumers (bareword resolution, diagnostics, completion seeding) can use one
//! source of truth.

use crate::ast::{Node, NodeKind, SourceLocation};
use rustc_hash::FxHashMap;

/// Canonical import origin kind for visible bare names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportKind {
    /// `use Module qw(foo bar)` / `use Module 'foo'` / `use Module ('foo')`
    UseList,
    /// `use Module qw(:tag)` expanded through known tag tables.
    ExportTag {
        /// Original export-tag token (for example `:sys_wait_h`).
        tag: String,
    },
    /// `use constant NAME => ...` or `use constant { NAME => ... }`.
    UseConstant,
}

/// One visible bare name introduced by imports/constants in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleImport {
    /// Bare symbol name visible in the file.
    pub bare_name: String,
    /// Source module/package that contributes the name.
    pub source_package: String,
    /// Import form that introduced the visible name.
    pub kind: ImportKind,
    /// Source location of the originating statement, if available.
    pub origin: Option<SourceLocation>,
    /// Whether this entry is fully resolved (`true`) versus a candidate (`false`).
    pub is_resolved: bool,
}

/// Per-file visible import record.
#[derive(Debug, Clone, Default)]
pub struct ImportSurface {
    entries: Vec<VisibleImport>,
    by_name: FxHashMap<String, Vec<usize>>,
}

impl ImportSurface {
    /// Build the visible import surface for a parsed file AST.
    pub fn from_ast(ast: &Node) -> Self {
        fn visit(node: &Node, current_package: &str, surface: &mut ImportSurface) {
            let mut package_for_children = current_package.to_string();

            match &node.kind {
                NodeKind::Package { name, .. } => {
                    package_for_children = name.clone();
                }
                NodeKind::Use { module, args, .. } => {
                    if module == "constant" {
                        for name in extract_constant_names_from_use_args(args) {
                            surface.push(VisibleImport {
                                bare_name: name,
                                source_package: current_package.to_string(),
                                kind: ImportKind::UseConstant,
                                origin: Some(node.location),
                                is_resolved: true,
                            });
                        }
                    } else {
                        for arg in args {
                            let normalized_arg = normalize_symbol_name(arg);
                            if normalized_arg.is_empty() {
                                continue;
                            }

                            if normalized_arg.starts_with("qw") {
                                for token in qw_tokens(&normalized_arg) {
                                    surface.push_use_token(module, token, node.location);
                                }
                                continue;
                            }

                            surface.push_use_token(module, &normalized_arg, node.location);
                        }
                    }
                }
                _ => {}
            }

            for child in node.children() {
                visit(child, &package_for_children, surface);
            }
        }

        let mut surface = Self::default();
        visit(ast, "main", &mut surface);
        surface
    }

    /// Immutable entries in insertion order.
    pub fn entries(&self) -> &[VisibleImport] {
        &self.entries
    }

    /// Returns true if the bare name is visible through any import entry.
    pub fn contains_bare_name(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    /// Returns visible entries for a given bare symbol name.
    pub fn visible_entries_for(&self, name: &str) -> impl Iterator<Item = &VisibleImport> {
        self.by_name
            .get(name)
            .into_iter()
            .flat_map(|indices| indices.iter().filter_map(|index| self.entries.get(*index)))
    }

    /// Find first resolved source package for a bare symbol, if present.
    pub fn resolved_source_for(&self, name: &str) -> Option<&str> {
        self.visible_entries_for(name)
            .find(|entry| entry.is_resolved)
            .map(|entry| entry.source_package.as_str())
    }

    fn push_use_token(&mut self, module: &str, token: &str, origin: SourceLocation) {
        if token.starts_with(':') {
            if let Some(expanded) = resolve_known_export_tag(module, token) {
                for symbol in expanded {
                    self.push(VisibleImport {
                        bare_name: symbol.to_string(),
                        source_package: module.to_string(),
                        kind: ImportKind::ExportTag { tag: token.to_string() },
                        origin: Some(origin),
                        is_resolved: true,
                    });
                }
            }
            return;
        }

        if is_bareword(token) {
            self.push(VisibleImport {
                bare_name: token.to_string(),
                source_package: module.to_string(),
                kind: ImportKind::UseList,
                origin: Some(origin),
                is_resolved: true,
            });
        }
    }

    fn push(&mut self, entry: VisibleImport) {
        let index = self.entries.len();
        self.by_name.entry(entry.bare_name.clone()).or_default().push(index);
        self.entries.push(entry);
    }
}

/// Resolve known export-tag members for core/common modules.
pub fn resolve_known_export_tag(module: &str, tag: &str) -> Option<&'static [&'static str]> {
    match (module, tag) {
        ("POSIX", ":sys_wait_h") => {
            Some(&["WEXITSTATUS", "WIFEXITED", "WIFSIGNALED", "WIFSTOPPED", "WTERMSIG"])
        }
        ("POSIX", ":fcntl_h") => Some(&["F_GETFD", "F_SETFD", "F_GETFL", "F_SETFL", "FD_CLOEXEC"]),
        ("POSIX", ":termios_h") => {
            Some(&["B9600", "B19200", "B38400", "TCSANOW", "TCSADRAIN", "TCSAFLUSH"])
        }
        ("File::Find", ":find") => Some(&["find", "finddepth"]),
        ("Fcntl", ":seek") => Some(&["SEEK_SET", "SEEK_CUR", "SEEK_END"]),
        ("Fcntl", ":lock") => Some(&["LOCK_SH", "LOCK_EX", "LOCK_NB", "LOCK_UN"]),
        ("Encode", ":fallback") => Some(&[
            "FB_DEFAULT",
            "FB_CROAK",
            "FB_QUIET",
            "FB_WARN",
            "FB_PERLQQ",
            "FB_HTMLCREF",
            "FB_XMLCREF",
        ]),
        _ => None,
    }
}

fn qw_tokens(arg: &str) -> impl Iterator<Item = &str> {
    arg.trim_start_matches("qw")
        .trim_start_matches(|c: char| "([{/<|!".contains(c))
        .trim_end_matches(|c: char| ")]}/|!>".contains(c))
        .split_whitespace()
}

fn normalize_symbol_name(raw: &str) -> String {
    raw.trim().trim_matches('\'').trim_matches('"').trim().to_string()
}

fn is_bareword(symbol: &str) -> bool {
    symbol.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && symbol
            .as_bytes()
            .first()
            .is_some_and(|first| first.is_ascii_alphabetic() || *first == b'_')
}

fn extract_constant_names_from_use_args(args: &[String]) -> Vec<String> {
    fn normalize_constant_name(token: &str) -> Option<&str> {
        let trimmed = token.trim().trim_matches('\'').trim_matches('"').trim();
        let candidate = trimmed
            .trim_start_matches(|c: char| c == '{' || c == '(')
            .trim_end_matches(|c: char| c == '}' || c == ')' || c == ',');

        if candidate.is_empty() || candidate == "=>" {
            return None;
        }
        if !candidate.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == ':') {
            return None;
        }
        candidate.chars().next().filter(|ch| ch.is_ascii_alphabetic() || *ch == '_')?;
        Some(candidate)
    }

    fn push_unique(
        names: &mut Vec<String>,
        seen: &mut std::collections::HashSet<String>,
        candidate: &str,
    ) {
        if seen.insert(candidate.to_string()) {
            names.push(candidate.to_string());
        }
    }

    let mut names = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for arg in args {
        if arg.starts_with('{') {
            let mut expect_key = true;
            for word in
                arg.split(|ch: char| ch.is_whitespace() || [',', '{', '}', '(', ')'].contains(&ch))
            {
                if word.is_empty() {
                    continue;
                }
                if word == "=>" {
                    expect_key = true;
                    continue;
                }
                if expect_key {
                    if let Some(candidate) = normalize_constant_name(word) {
                        push_unique(&mut names, &mut seen, candidate);
                    }
                    expect_key = false;
                }
            }
            continue;
        }

        if arg.starts_with("qw") {
            for word in qw_tokens(arg) {
                if let Some(candidate) = normalize_constant_name(word) {
                    push_unique(&mut names, &mut seen, candidate);
                }
            }
            continue;
        }

        if let Some(candidate) = normalize_constant_name(arg)
            && candidate != "constant"
            && !candidate.contains("::")
            && !candidate.chars().all(char::is_numeric)
        {
            push_unique(&mut names, &mut seen, candidate);
            continue;
        }
    }

    if names.is_empty() {
        let Some(first) = args.first() else {
            return names;
        };
        if let Some(candidate) = normalize_constant_name(first) {
            push_unique(&mut names, &mut seen, candidate);
        }
    }

    names
}

#[cfg(test)]
mod tests {
    use super::{ImportKind, ImportSurface};
    use crate::Parser;

    #[test]
    fn collects_use_forms_tags_and_constants() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"
use List::Util qw(sum);
use POSIX ('WIFEXITED');
use POSIX 'WTERMSIG';
use POSIX qw(:sys_wait_h);
use constant PI => 3.14;
"#;
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;

        let surface = ImportSurface::from_ast(&ast);

        assert!(surface.contains_bare_name("sum"));
        assert!(surface.contains_bare_name("WIFEXITED"));
        assert!(surface.contains_bare_name("WTERMSIG"));
        assert!(surface.contains_bare_name("WEXITSTATUS"));
        assert!(surface.contains_bare_name("PI"));

        let pi = surface.visible_entries_for("PI").next().ok_or("PI should be collected")?;
        assert_eq!(pi.source_package, "main");
        assert!(matches!(pi.kind, ImportKind::UseConstant));
        assert!(pi.is_resolved);
        Ok(())
    }
}
