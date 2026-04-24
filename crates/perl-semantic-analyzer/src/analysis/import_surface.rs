//! Canonical per-file visible import collection.

use std::collections::{HashMap, HashSet};

use perl_module::import::resolve_known_export_tag;

use crate::{Node, NodeKind, SourceLocation};

/// Kind of import visibility entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisibleImportKind {
    /// Explicit symbol imported from a `use Module ...` list.
    UseList,
    /// Symbol materialized from an export-tag expansion (for example `:sys_wait_h`).
    ExportTag,
    /// Constant introduced by `use constant ...` forms.
    UseConstant,
}

/// A visible bare-name import in one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleImport {
    /// Bare symbol name visible in the current file.
    pub bare_name: String,
    /// Source module or package associated with the visibility.
    pub source: String,
    /// Import record kind.
    pub kind: VisibleImportKind,
    /// Location of the originating statement when available.
    pub origin: Option<SourceLocation>,
    /// Whether the entry is fully resolved (`true`) or a candidate (`false`).
    pub is_resolved: bool,
}

/// Canonical set of visible imported bare names for a file.
#[derive(Debug, Clone, Default)]
pub struct ImportSurface {
    entries: Vec<VisibleImport>,
    by_name: HashMap<String, Vec<usize>>,
}

impl ImportSurface {
    /// Build a visible import surface by scanning a file AST.
    pub fn from_ast(ast: &Node) -> Self {
        let mut collector = Collector {
            entries: Vec::new(),
            seen: HashSet::new(),
            current_package: "main".to_string(),
        };
        collector.visit(ast);
        Self::from_entries(collector.entries)
    }

    fn from_entries(entries: Vec<VisibleImport>) -> Self {
        let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, entry) in entries.iter().enumerate() {
            by_name.entry(entry.bare_name.clone()).or_default().push(idx);
        }
        Self { entries, by_name }
    }

    /// Returns all visible import entries.
    pub fn entries(&self) -> &[VisibleImport] {
        &self.entries
    }

    /// Returns true if `name` is a visible imported bare name.
    pub fn has_visible_name(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    /// Returns the first resolved non-constant source module for `name`.
    pub fn first_resolved_symbol_source(&self, name: &str) -> Option<&str> {
        let indexes = self.by_name.get(name)?;
        indexes
            .iter()
            .filter_map(|idx| self.entries.get(*idx))
            .find(|entry| {
                entry.is_resolved && !matches!(entry.kind, VisibleImportKind::UseConstant)
            })
            .map(|entry| entry.source.as_str())
    }
}

struct Collector {
    entries: Vec<VisibleImport>,
    seen: HashSet<(String, String, VisibleImportKind)>,
    current_package: String,
}

impl Collector {
    fn visit(&mut self, node: &Node) {
        if let NodeKind::Package { name, .. } = &node.kind {
            self.current_package = name.clone();
        }

        if let NodeKind::Use { module, args, .. } = &node.kind {
            if module == "constant" {
                for constant_name in parse_constant_names(args) {
                    self.push_entry(VisibleImport {
                        bare_name: constant_name,
                        source: self.current_package.clone(),
                        kind: VisibleImportKind::UseConstant,
                        origin: Some(node.location),
                        is_resolved: true,
                    });
                }
            } else {
                for arg in args {
                    if arg.starts_with("qw") {
                        for token in parse_qw_tokens(arg) {
                            self.push_use_token(module, token, node.location);
                        }
                    } else {
                        self.push_use_token(module, arg, node.location);
                    }
                }
            }
        }

        for child in node.children() {
            self.visit(child);
        }
    }

    fn push_entry(&mut self, entry: VisibleImport) {
        let key = (entry.bare_name.clone(), entry.source.clone(), entry.kind);
        if self.seen.insert(key) {
            self.entries.push(entry);
        }
    }

    fn push_use_token(&mut self, module: &str, token: &str, origin: SourceLocation) {
        let symbol = normalize_symbol(token);
        if symbol.is_empty() || symbol == "," {
            return;
        }

        if symbol.starts_with(':') {
            if let Some(expanded) = resolve_known_export_tag(module, symbol) {
                for name in expanded {
                    self.push_entry(VisibleImport {
                        bare_name: (*name).to_string(),
                        source: module.to_string(),
                        kind: VisibleImportKind::ExportTag,
                        origin: Some(origin),
                        is_resolved: true,
                    });
                }
            }
            return;
        }

        if is_bareword(symbol) {
            self.push_entry(VisibleImport {
                bare_name: symbol.to_string(),
                source: module.to_string(),
                kind: VisibleImportKind::UseList,
                origin: Some(origin),
                is_resolved: true,
            });
        }
    }
}

fn normalize_symbol(token: &str) -> &str {
    token.trim().trim_matches('\'').trim_matches('"').trim()
}

fn is_bareword(symbol: &str) -> bool {
    symbol.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && symbol
            .as_bytes()
            .first()
            .is_some_and(|first| first.is_ascii_alphabetic() || *first == b'_')
}

fn parse_qw_tokens(arg: &str) -> impl Iterator<Item = &str> {
    arg.trim_start_matches("qw")
        .trim_start_matches(|c: char| "([{/<|!".contains(c))
        .trim_end_matches(|c: char| ")]}/|!>".contains(c))
        .split_whitespace()
}

fn parse_constant_names(args: &[String]) -> Vec<String> {
    fn push_unique(names: &mut Vec<String>, seen: &mut HashSet<String>, token: &str) {
        if is_bareword(token) && seen.insert(token.to_string()) {
            names.push(token.to_string());
        }
    }

    let mut names = Vec::new();
    let mut seen = HashSet::new();

    let tokens: Vec<&str> = args.iter().map(String::as_str).collect();
    let mut start = 0usize;
    while matches!(tokens.get(start), Some(token) if token.starts_with('-') || *token == ",") {
        start += 1;
    }

    if let Some(first) = tokens.get(start) {
        if first.starts_with("qw") {
            for token in parse_qw_tokens(first) {
                let normalized = normalize_symbol(token);
                push_unique(&mut names, &mut seen, normalized);
            }
            return names;
        }
    }

    let mut i = start;
    while i < tokens.len() {
        let token = normalize_symbol(tokens[i]);
        if token.is_empty() {
            i += 1;
            continue;
        }

        if token == "{" || token == "+{" || token == "+" || token == "}" {
            i += 1;
            continue;
        }

        if is_bareword(token) {
            push_unique(&mut names, &mut seen, token);
            if i + 1 < tokens.len() && tokens[i + 1].trim() == "=>" {
                i += 2;
                continue;
            }
        }

        i += 1;
    }

    names
}

#[cfg(test)]
mod tests {
    use crate::Parser;

    use super::{ImportSurface, VisibleImportKind};

    fn import_surface(code: &str) -> Result<ImportSurface, Box<dyn std::error::Error>> {
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        Ok(ImportSurface::from_ast(&ast))
    }

    #[test]
    fn collects_qw_parens_and_single_quote_imports() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"
use List::Util qw(sum);
use Carp ('croak');
use Cwd 'getcwd';
"#;
        let surface = import_surface(code)?;

        assert!(surface.has_visible_name("sum"));
        assert_eq!(surface.first_resolved_symbol_source("sum"), Some("List::Util"));
        assert_eq!(surface.first_resolved_symbol_source("croak"), Some("Carp"));
        assert_eq!(surface.first_resolved_symbol_source("getcwd"), Some("Cwd"));
        Ok(())
    }

    #[test]
    fn expands_known_export_tags() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use POSIX qw(:sys_wait_h);";
        let surface = import_surface(code)?;

        assert!(surface.has_visible_name("WIFEXITED"));
        let tag_entry = surface
            .entries()
            .iter()
            .find(|entry| entry.bare_name == "WIFEXITED")
            .ok_or("WIFEXITED should be visible via :sys_wait_h")?;
        assert_eq!(tag_entry.kind, VisibleImportKind::ExportTag);
        assert!(tag_entry.is_resolved);
        Ok(())
    }

    #[test]
    fn collects_supported_use_constant_forms() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"
use constant PI => 3.14;
use constant ('TAU', 6.28);
use constant qw(FOO BAR);
use constant { MIN => 1, MAX => 2 };
"#;
        let surface = import_surface(code)?;

        for constant in ["PI", "TAU", "FOO", "BAR", "MIN", "MAX"] {
            let entry = surface
                .entries()
                .iter()
                .find(|entry| entry.bare_name == constant)
                .ok_or("constant should be visible")?;
            assert_eq!(entry.kind, VisibleImportKind::UseConstant);
            assert!(entry.is_resolved);
            assert_eq!(entry.source, "main");
        }
        Ok(())
    }
}
