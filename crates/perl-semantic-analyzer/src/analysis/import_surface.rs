//! Canonical per-file import visibility surface.

use crate::ast::{Node, NodeKind, SourceLocation};
use perl_module::import::resolve_known_export_tag;
use std::collections::HashSet;

/// Classifies how a visible import candidate was introduced.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImportKind {
    /// `use Module qw(foo bar)` and related list forms.
    UseList,
    /// `use Module ':tag'` expanded via known export-tag metadata.
    ExportTag {
        /// Export tag token (for example `:sys_wait_h`).
        tag: String,
    },
    /// `use constant NAME => ...` forms.
    UseConstant,
}

/// One visible bare-name entry in a file's import surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleImport {
    /// Imported or otherwise visible bare symbol name.
    pub bare_name: String,
    /// Source package/module that provides the visible name.
    pub source_package: String,
    /// Classification of the import mechanism.
    pub kind: ImportKind,
    /// Source span for the origin import statement, if available.
    pub origin: Option<SourceLocation>,
    /// Whether this entry is fully resolved (`true`) or only a candidate (`false`).
    pub is_resolved: bool,
}

/// Per-file canonical record of visible imported bare names.
#[derive(Debug, Clone, Default)]
pub struct ImportSurface {
    entries: Vec<VisibleImport>,
}

impl ImportSurface {
    /// Build an import surface from the AST using currently supported static cases.
    pub fn from_ast(ast: &Node) -> Self {
        fn normalize_symbol(token: &str) -> Option<&str> {
            let symbol = token.trim().trim_matches('\'').trim_matches('"').trim();
            if symbol.is_empty() || symbol == "," { None } else { Some(symbol) }
        }

        fn is_bareword(symbol: &str) -> bool {
            symbol.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                && symbol
                    .as_bytes()
                    .first()
                    .is_some_and(|first| first.is_ascii_alphabetic() || *first == b'_')
        }

        fn parse_qw_words(token: &str) -> Option<Vec<String>> {
            if !token.trim_start().starts_with("qw") {
                return None;
            }
            let content = token
                .trim_start()
                .trim_start_matches("qw")
                .trim_start_matches(|c: char| c.is_whitespace())
                .trim_start_matches(|c: char| "([{/<|!".contains(c))
                .trim_end_matches(|c: char| ")]}/|!>".contains(c));
            Some(content.split_whitespace().map(ToString::to_string).collect())
        }

        fn extract_constant_names(args: &[String]) -> Vec<String> {
            fn normalize_constant_name(token: &str) -> Option<&str> {
                let stripped = token.trim_matches(|c: char| {
                    matches!(c, '\'' | '"' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';')
                });
                if stripped.is_empty() || stripped.starts_with('-') {
                    return None;
                }
                stripped.chars().all(|c| c.is_alphanumeric() || c == '_').then_some(stripped)
            }

            let mut names = Vec::new();
            let mut seen = HashSet::new();

            let Some(first) = args.first().map(String::as_str) else {
                return names;
            };

            if let Some(words) = parse_qw_words(first) {
                for word in words {
                    if let Some(candidate) = normalize_constant_name(&word)
                        && seen.insert(candidate.to_string())
                    {
                        names.push(candidate.to_string());
                    }
                }
                return names;
            }

            let starts_hash_form = first == "{"
                || first == "+{"
                || (first == "+" && args.get(1).map(String::as_str) == Some("{"));
            if starts_hash_form {
                let mut skipped_leading_plus = false;
                let mut iter = args.iter().peekable();
                while let Some(arg) = iter.next() {
                    if arg == "+{" {
                        skipped_leading_plus = true;
                        continue;
                    }
                    if arg == "+" && !skipped_leading_plus {
                        skipped_leading_plus = true;
                        continue;
                    }
                    if arg == "{" || arg == "}" || arg == "," || arg == "=>" {
                        continue;
                    }
                    if let Some(candidate) = normalize_constant_name(arg)
                        && iter.peek().map(|s| s.as_str()) == Some("=>")
                        && seen.insert(candidate.to_string())
                    {
                        names.push(candidate.to_string());
                    }
                }
                return names;
            }

            if let Some(candidate) = normalize_constant_name(first)
                && seen.insert(candidate.to_string())
            {
                names.push(candidate.to_string());
            }

            names
        }

        fn insert_entry(
            entries: &mut Vec<VisibleImport>,
            dedupe: &mut HashSet<(String, String, ImportKind)>,
            bare_name: &str,
            source_package: &str,
            kind: ImportKind,
            origin: Option<SourceLocation>,
            is_resolved: bool,
        ) {
            let key = (bare_name.to_string(), source_package.to_string(), kind.clone());
            if dedupe.insert(key) {
                entries.push(VisibleImport {
                    bare_name: bare_name.to_string(),
                    source_package: source_package.to_string(),
                    kind,
                    origin,
                    is_resolved,
                });
            }
        }

        fn visit(
            node: &Node,
            current_package: &mut String,
            entries: &mut Vec<VisibleImport>,
            dedupe: &mut HashSet<(String, String, ImportKind)>,
        ) {
            match &node.kind {
                NodeKind::Package { name, .. } => {
                    *current_package = name.clone();
                }
                NodeKind::Use { module, args, .. } => {
                    if module == "constant" {
                        for constant_name in extract_constant_names(args) {
                            insert_entry(
                                entries,
                                dedupe,
                                &constant_name,
                                current_package,
                                ImportKind::UseConstant,
                                Some(node.location),
                                true,
                            );
                        }
                    } else {
                        for arg in args {
                            if let Some(words) = parse_qw_words(arg) {
                                for token in words {
                                    if let Some(symbol) = normalize_symbol(&token) {
                                        if symbol.starts_with(':') {
                                            if let Some(expanded) =
                                                resolve_known_export_tag(module, symbol)
                                            {
                                                for expanded_symbol in expanded {
                                                    insert_entry(
                                                        entries,
                                                        dedupe,
                                                        expanded_symbol,
                                                        module,
                                                        ImportKind::ExportTag {
                                                            tag: symbol.to_string(),
                                                        },
                                                        Some(node.location),
                                                        true,
                                                    );
                                                }
                                            }
                                        } else if is_bareword(symbol) {
                                            insert_entry(
                                                entries,
                                                dedupe,
                                                symbol,
                                                module,
                                                ImportKind::UseList,
                                                Some(node.location),
                                                true,
                                            );
                                        }
                                    }
                                }
                                continue;
                            }

                            let Some(symbol) = normalize_symbol(arg) else {
                                continue;
                            };
                            if symbol.starts_with(':') {
                                if let Some(expanded) = resolve_known_export_tag(module, symbol) {
                                    for expanded_symbol in expanded {
                                        insert_entry(
                                            entries,
                                            dedupe,
                                            expanded_symbol,
                                            module,
                                            ImportKind::ExportTag { tag: symbol.to_string() },
                                            Some(node.location),
                                            true,
                                        );
                                    }
                                }
                            } else if is_bareword(symbol) {
                                insert_entry(
                                    entries,
                                    dedupe,
                                    symbol,
                                    module,
                                    ImportKind::UseList,
                                    Some(node.location),
                                    true,
                                );
                            }
                        }
                    }
                }
                _ => {}
            }

            for child in node.children() {
                visit(child, current_package, entries, dedupe);
            }
        }

        let mut entries = Vec::new();
        let mut dedupe = HashSet::new();
        let mut current_package = "main".to_string();
        visit(ast, &mut current_package, &mut entries, &mut dedupe);

        Self { entries }
    }

    /// All visible import entries.
    pub fn entries(&self) -> &[VisibleImport] {
        &self.entries
    }

    /// Returns whether `name` is visible as a bare import.
    pub fn contains_name(&self, name: &str) -> bool {
        self.entries.iter().any(|entry| entry.bare_name == name)
    }

    /// Returns the source package for the first resolved matching entry.
    pub fn source_for_name(&self, name: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.bare_name == name && entry.is_resolved)
            .map(|entry| entry.source_package.as_str())
            .or_else(|| {
                self.entries
                    .iter()
                    .find(|entry| entry.bare_name == name)
                    .map(|entry| entry.source_package.as_str())
            })
    }

    /// Unique visible bare names, useful for completion-style candidate gathering.
    pub fn visible_names(&self) -> Vec<&str> {
        let mut names = Vec::new();
        let mut seen = HashSet::new();
        for entry in &self.entries {
            if seen.insert(entry.bare_name.as_str()) {
                names.push(entry.bare_name.as_str());
            }
        }
        names
    }
}
