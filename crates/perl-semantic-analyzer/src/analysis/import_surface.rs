//! Canonical per-file import visibility model.
//!
//! This module centralizes extraction of visible imported bare names so
//! consumers (diagnostics, declaration lookup, completion) can share one
//! consistent view.

use perl_module::import::resolve_known_export_tag;

use crate::{Node, NodeKind, SourceLocation};

/// How a visible name became available in the current file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibleImportKind {
    /// Explicit symbol list import, e.g. `use Module qw(foo)` or `use Module 'foo'`.
    UseList,
    /// Import via export tag expansion, e.g. `use POSIX qw(:sys_wait_h)`.
    ExportTag,
    /// Constant declaration via `use constant ...`.
    UseConstant,
}

/// One visible bare-name import candidate in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleImport {
    /// Imported/visible bare symbol name.
    pub bare_name: String,
    /// Source module or package that provides the symbol.
    pub source_package: String,
    /// Import form used to make the symbol visible.
    pub kind: VisibleImportKind,
    /// Source span for the originating statement, when available.
    pub origin_span: Option<SourceLocation>,
    /// Whether symbol origin has been resolved with static certainty.
    pub is_resolved: bool,
}

/// Per-file import visibility surface.
#[derive(Debug, Clone, Default)]
pub struct ImportSurface {
    entries: Vec<VisibleImport>,
}

impl ImportSurface {
    /// Build visible import entries from an AST.
    #[must_use]
    pub fn from_ast(ast: &Node) -> Self {
        let mut surface = Self::default();
        Self::collect(ast, "main", &mut surface);
        surface
    }

    /// Returns all visible import entries.
    #[must_use]
    pub fn entries(&self) -> &[VisibleImport] {
        &self.entries
    }

    /// Returns true if a visible bare-name import exists.
    #[must_use]
    pub fn has_visible_bare_name(&self, name: &str) -> bool {
        self.entries.iter().any(|entry| entry.bare_name == name)
    }

    /// Return the first resolved source package for an imported bare name.
    #[must_use]
    pub fn first_source_package_for(&self, name: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.bare_name == name && entry.is_resolved)
            .map(|entry| entry.source_package.as_str())
    }

    fn collect(node: &Node, current_package: &str, surface: &mut Self) {
        let mut pkg = current_package;
        if let NodeKind::Package { name, .. } = &node.kind {
            pkg = name.as_str();
        }

        if let NodeKind::Use { module, args, .. } = &node.kind {
            if module == "constant" {
                Self::collect_constant_use(node, args, pkg, surface);
            } else {
                Self::collect_regular_use(node, module, args, surface);
            }
        }

        for child in node.children() {
            Self::collect(child, pkg, surface);
        }
    }

    fn collect_regular_use(node: &Node, module: &str, args: &[String], surface: &mut Self) {
        for arg in args {
            if arg.starts_with("qw") {
                for token in Self::split_qw_tokens(arg) {
                    Self::push_regular_token(node, module, token, surface);
                }
            } else {
                Self::push_regular_token(node, module, arg, surface);
            }
        }
    }

    fn push_regular_token(node: &Node, module: &str, token: &str, surface: &mut Self) {
        let symbol = Self::normalize_token(token);
        if symbol.is_empty() {
            return;
        }

        if symbol.starts_with(':') {
            if let Some(expanded) = resolve_known_export_tag(module, symbol) {
                for name in expanded {
                    surface.entries.push(VisibleImport {
                        bare_name: (*name).to_string(),
                        source_package: module.to_string(),
                        kind: VisibleImportKind::ExportTag,
                        origin_span: Some(node.location),
                        is_resolved: true,
                    });
                }
            }
            return;
        }

        if Self::is_bareword(symbol) {
            surface.entries.push(VisibleImport {
                bare_name: symbol.to_string(),
                source_package: module.to_string(),
                kind: VisibleImportKind::UseList,
                origin_span: Some(node.location),
                is_resolved: true,
            });
        }
    }

    fn collect_constant_use(node: &Node, args: &[String], package: &str, surface: &mut Self) {
        let stripped = Self::strip_constant_options(args);
        let args_text = stripped.join(" ");
        if let Some(first) = stripped.first() {
            let bare = Self::normalize_token(first);
            if Self::is_bareword(bare) {
                surface.entries.push(VisibleImport {
                    bare_name: bare.to_string(),
                    source_package: package.to_string(),
                    kind: VisibleImportKind::UseConstant,
                    origin_span: Some(node.location),
                    is_resolved: true,
                });
            }
        }

        for arg in stripped {
            if arg.starts_with("qw") {
                for name in Self::split_qw_tokens(arg) {
                    let bare = Self::normalize_token(name);
                    if Self::is_bareword(bare) {
                        surface.entries.push(VisibleImport {
                            bare_name: bare.to_string(),
                            source_package: package.to_string(),
                            kind: VisibleImportKind::UseConstant,
                            origin_span: Some(node.location),
                            is_resolved: true,
                        });
                    }
                }
            }
        }
        for bare in Self::extract_qw_names(&args_text) {
            surface.entries.push(VisibleImport {
                bare_name: bare,
                source_package: package.to_string(),
                kind: VisibleImportKind::UseConstant,
                origin_span: Some(node.location),
                is_resolved: true,
            });
        }

        for pair in stripped.windows(2) {
            if pair[1] != "=>" {
                continue;
            }
            let lhs = Self::normalize_token(&pair[0]);
            if Self::is_bareword(lhs) {
                surface.entries.push(VisibleImport {
                    bare_name: lhs.to_string(),
                    source_package: package.to_string(),
                    kind: VisibleImportKind::UseConstant,
                    origin_span: Some(node.location),
                    is_resolved: true,
                });
            }
        }
        for lhs in Self::extract_arrow_lhs_names(&args_text) {
            surface.entries.push(VisibleImport {
                bare_name: lhs,
                source_package: package.to_string(),
                kind: VisibleImportKind::UseConstant,
                origin_span: Some(node.location),
                is_resolved: true,
            });
        }
    }

    fn split_qw_tokens(raw: &str) -> impl Iterator<Item = &str> {
        raw.trim_start_matches("qw")
            .trim_start_matches(|c: char| "([{/<|!".contains(c))
            .trim_end_matches(|c: char| ")]}/|!>".contains(c))
            .split_whitespace()
    }

    fn strip_constant_options(args: &[String]) -> &[String] {
        let mut i = 0;
        while i < args.len() && args[i].starts_with('-') {
            i += 1;
        }
        if i < args.len() && args[i] == "," {
            i += 1;
        }
        &args[i..]
    }

    fn normalize_token(token: &str) -> &str {
        token
            .trim()
            .trim_matches(',')
            .trim()
            .trim_matches('\'')
            .trim_matches('"')
            .trim()
            .trim_matches('{')
            .trim_matches('}')
            .trim_matches('(')
            .trim_matches(')')
            .trim()
    }

    fn is_bareword(symbol: &str) -> bool {
        symbol.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            && symbol
                .as_bytes()
                .first()
                .is_some_and(|first| first.is_ascii_alphabetic() || *first == b'_')
    }

    fn extract_qw_names(text: &str) -> Vec<String> {
        let mut names = Vec::new();
        let bytes = text.as_bytes();
        let mut i = 0usize;
        while i + 2 <= bytes.len() {
            if bytes[i..].starts_with(b"qw") {
                let delim = bytes.get(i + 2).copied().unwrap_or_default() as char;
                let close = match delim {
                    '(' => ')',
                    '[' => ']',
                    '{' => '}',
                    '<' => '>',
                    c => c,
                };
                let start = i + 3;
                if start > text.len() {
                    break;
                }
                if let Some(end_rel) = text[start..].find(close) {
                    let end = start + end_rel;
                    for token in text[start..end].split_whitespace() {
                        let bare = Self::normalize_token(token);
                        if Self::is_bareword(bare) {
                            names.push(bare.to_string());
                        }
                    }
                    i = end + 1;
                    continue;
                }
            }
            i += 1;
        }
        names
    }

    fn extract_arrow_lhs_names(text: &str) -> Vec<String> {
        let bytes = text.as_bytes();
        let mut names = Vec::new();
        let mut i = 0usize;
        while i + 2 <= bytes.len() {
            if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
                let start = i;
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let ident = &text[start..i];
                let mut j = i;
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if bytes.get(j) == Some(&b'=') && bytes.get(j + 1) == Some(&b'>') {
                    names.push(ident.to_string());
                }
            } else {
                i += 1;
            }
        }
        names
    }
}

#[cfg(test)]
mod tests {
    use super::{ImportSurface, VisibleImportKind};
    use crate::Parser;

    fn collect(code: &str) -> Result<ImportSurface, Box<dyn std::error::Error>> {
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        Ok(ImportSurface::from_ast(&ast))
    }

    #[test]
    fn collects_qw_imports() -> Result<(), Box<dyn std::error::Error>> {
        let surface = collect("use POSIX qw(WIFEXITED);")?;
        assert!(surface.has_visible_bare_name("WIFEXITED"));
        assert!(surface.entries().iter().any(|entry| {
            entry.bare_name == "WIFEXITED" && entry.kind == VisibleImportKind::UseList
        }));
        Ok(())
    }

    #[test]
    fn collects_parenthesized_and_single_quote_imports() -> Result<(), Box<dyn std::error::Error>> {
        let surface = collect("use Carp ('croak'); use Carp 'confess';")?;
        assert!(surface.has_visible_bare_name("croak"));
        assert!(surface.has_visible_bare_name("confess"));
        Ok(())
    }

    #[test]
    fn expands_known_export_tags() -> Result<(), Box<dyn std::error::Error>> {
        let surface = collect("use POSIX qw(:sys_wait_h);")?;
        assert!(surface.has_visible_bare_name("WIFEXITED"));
        assert!(surface.entries().iter().any(|entry| {
            entry.bare_name == "WIFEXITED" && entry.kind == VisibleImportKind::ExportTag
        }));
        Ok(())
    }

    #[test]
    fn collects_use_constant_forms() -> Result<(), Box<dyn std::error::Error>> {
        let surface = collect(
            "use constant PI => 3.14; use constant { MIN => 1, MAX => 2 }; use constant qw(FOO BAR);",
        )?;
        assert!(surface.has_visible_bare_name("PI"));
        assert!(surface.has_visible_bare_name("MIN"));
        assert!(surface.has_visible_bare_name("MAX"));
        assert!(surface.has_visible_bare_name("FOO"));
        assert!(surface.has_visible_bare_name("BAR"));
        assert!(surface.entries().iter().any(|entry| {
            entry.bare_name == "PI" && entry.kind == VisibleImportKind::UseConstant
        }));
        Ok(())
    }
}
