//! Cross-file workspace indexing for Perl symbols
//!
//! This module provides efficient indexing of symbols across all files in a workspace,
//! enabling fast cross-file navigation, references, and refactoring.

use crate::analysis::import_extractor::ImportExtractor;
use crate::symbol::{SymbolKind, SymbolTable};
use crate::{Node, NodeKind, Parser};
use perl_semantic_facts::FileId;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Symbol kinds for cross-file indexing during Index/Navigate workflows.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SymKind {
    /// Variable symbol ($, @, or % sigil)
    Var,
    /// Subroutine definition (sub foo)
    Sub,
    /// Package declaration (package Foo)
    Pack,
}

/// A normalized symbol key for cross-file lookups in Index/Navigate workflows.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SymbolKey {
    /// Package name containing this symbol
    pub pkg: Arc<str>,
    /// Bare name without sigil prefix
    pub name: Arc<str>,
    /// Variable sigil ($, @, or %) if applicable
    pub sigil: Option<char>,
    /// Kind of symbol (variable, subroutine, package)
    pub kind: SymKind,
}

/// A symbol definition in the workspace
#[derive(Clone, Debug)]
pub struct SymbolDef {
    /// The name of the symbol
    pub name: String,
    /// The kind of symbol (function, variable, package, etc.)
    pub kind: SymbolKind,
    /// The URI of the file containing this symbol
    pub uri: String,
    /// Start byte offset in the file
    pub start: usize,
    /// End byte offset in the file
    pub end: usize,
}

/// Workspace-wide index for fast symbol lookups
#[derive(Default)]
pub struct WorkspaceIndex {
    /// Index from symbol name to all its definitions
    by_name: HashMap<String, Vec<SymbolDef>>,
    /// Index from URI to all symbol names in that file (for fast removal)
    by_uri: HashMap<String, HashSet<String>>,
    /// Import/module dependencies by URI.
    imports_by_uri: RwLock<HashMap<String, HashSet<String>>>,
}

impl WorkspaceIndex {
    /// Create a new empty workspace index
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the index with symbols from a document
    pub fn update_from_document(&mut self, uri: &str, content: &str, symtab: &SymbolTable) {
        // Remove old symbols from this file
        self.remove_document(uri);

        // Track all symbol names in this file
        let mut names_in_file = HashSet::new();

        // Add all symbols from the symbol table
        for symbols in symtab.symbols.values() {
            for symbol in symbols {
                let name = symbol.name.clone();
                names_in_file.insert(name.clone());

                let def = SymbolDef {
                    name: symbol.name.clone(),
                    kind: symbol.kind,
                    uri: uri.to_string(),
                    start: symbol.location.start,
                    end: symbol.location.end,
                };

                self.by_name.entry(name).or_default().push(def);
            }
        }

        // Track which names are in this file
        self.by_uri.insert(uri.to_string(), names_in_file);

        if !content.is_empty()
            && let Ok(dependencies) = Self::extract_dependencies(content)
        {
            self.set_file_dependencies(uri, dependencies);
        }
    }

    /// Remove all symbols from a document
    pub fn remove_document(&mut self, uri: &str) {
        if let Some(names) = self.by_uri.remove(uri) {
            for name in names {
                if let Some(defs) = self.by_name.get_mut(&name) {
                    defs.retain(|d| d.uri != uri);
                    if defs.is_empty() {
                        self.by_name.remove(&name);
                    }
                }
            }
        }
        self.remove_file_dependencies(uri);
    }

    /// Index import dependencies from raw file contents.
    pub fn index_file_str(&self, uri: &str, content: &str) -> Result<(), String> {
        let dependencies = Self::extract_dependencies(content)?;
        let mut imports = self
            .imports_by_uri
            .write()
            .map_err(|_| "workspace import index lock poisoned".to_string())?;
        imports.insert(uri.to_string(), dependencies);
        Ok(())
    }

    /// Return modules imported or required by a file.
    pub fn file_dependencies(&self, uri: &str) -> HashSet<String> {
        let Ok(imports) = self.imports_by_uri.read() else {
            return HashSet::new();
        };
        imports.get(uri).cloned().unwrap_or_default()
    }

    fn set_file_dependencies(&self, uri: &str, dependencies: HashSet<String>) {
        if let Ok(mut imports) = self.imports_by_uri.write() {
            imports.insert(uri.to_string(), dependencies);
        }
    }

    fn remove_file_dependencies(&self, uri: &str) {
        if let Ok(mut imports) = self.imports_by_uri.write() {
            imports.remove(uri);
        }
    }

    fn extract_dependencies(content: &str) -> Result<HashSet<String>, String> {
        let mut parser = Parser::new(content);
        let ast = parser.parse().map_err(|err| format!("Parse error: {err}"))?;
        let mut dependencies: HashSet<String> = ImportExtractor::extract(&ast, FileId(0))
            .into_iter()
            .filter_map(|spec| {
                if spec.module.is_empty() || matches!(spec.module.as_str(), "parent" | "base") {
                    None
                } else {
                    Some(spec.module)
                }
            })
            .collect();

        Self::collect_parent_dependencies(&ast, &mut dependencies);
        Ok(dependencies)
    }

    fn collect_parent_dependencies(node: &Node, dependencies: &mut HashSet<String>) {
        if let NodeKind::Use { module, args, .. } = &node.kind
            && matches!(module.as_str(), "parent" | "base")
        {
            for name in Self::parent_names_from_args(args) {
                dependencies.insert(name);
            }
        }

        for child in node.children() {
            Self::collect_parent_dependencies(child, dependencies);
        }
    }

    fn parent_names_from_args(args: &[String]) -> Vec<String> {
        args.iter()
            .flat_map(|arg| Self::expand_parent_arg(arg))
            .filter(|name| !name.starts_with('-'))
            .collect()
    }

    fn expand_parent_arg(arg: &str) -> Vec<String> {
        let trimmed = arg.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }

        if let Some(content) = Self::parse_qw_content(trimmed) {
            return content.split_whitespace().map(str::to_string).collect();
        }

        let unquoted = trimmed.trim_matches('\'').trim_matches('"').trim();
        if unquoted.is_empty() { Vec::new() } else { vec![unquoted.to_string()] }
    }

    pub(crate) fn parse_qw_content(arg: &str) -> Option<&str> {
        // Perl allows whitespace between `qw` and its opening delimiter
        // (e.g. `qw [Foo::Bar]`); trim it so the delimiter is read correctly
        // rather than treating the space as the delimiter.
        let rest = arg.strip_prefix("qw")?.trim_start();
        let mut chars = rest.chars();
        let open = chars.next()?;
        let close = match open {
            '(' => ')',
            '{' => '}',
            '[' => ']',
            '<' => '>',
            delimiter => delimiter,
        };
        let start = open.len_utf8();
        let end = rest.rfind(close)?;
        // Guard the bounds explicitly: constructing `&rest[start..end]` when
        // `end < start` would panic (e.g. a bareword like `qwfoo` where the
        // delimiter char is also the first content char). Return `None` first.
        if end < start {
            return None;
        }
        Some(&rest[start..end])
    }

    /// Find all definitions of a symbol by name
    pub fn find_defs(&self, name: &str) -> &[SymbolDef] {
        static EMPTY: Vec<SymbolDef> = Vec::new();
        self.by_name.get(name).map(|v| v.as_slice()).unwrap_or(&EMPTY[..])
    }

    /// Find all references to a symbol (simplified version)
    /// In a full implementation, this would analyze usage sites
    pub fn find_refs(&self, name: &str) -> Vec<SymbolDef> {
        // For now, return all definitions as references
        // A full implementation would scan all files for usage sites
        self.find_defs(name).to_vec()
    }

    /// Get all symbols in the workspace matching a query
    pub fn search_symbols(&self, query: &str) -> Vec<SymbolDef> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for (name, defs) in &self.by_name {
            if name.to_lowercase().contains(&query_lower) {
                results.extend(defs.clone());
            }
        }

        results
    }

    /// Get the total number of indexed symbols
    pub fn symbol_count(&self) -> usize {
        self.by_name.values().map(|v| v.len()).sum()
    }

    /// Get the number of indexed files
    pub fn file_count(&self) -> usize {
        self.by_uri.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SourceLocation;
    use crate::symbol::Symbol;

    /// Regression: `use parent qw [..]` with whitespace before the delimiter
    /// must extract each base module as a dependency. Previously the leading
    /// space was treated as the `qw` delimiter, so `parse_qw_content` returned
    /// garbage and the whole `qw [..]` token was recorded as a single bogus
    /// dependency. See `parse_qw_content`.
    #[test]
    fn parent_qw_space_before_delimiter_extracts_each_base() -> Result<(), String> {
        let index = WorkspaceIndex::new();
        index.index_file_str("file:///c.pl", "use parent qw [Foo::Base Bar::Base];\n1;\n")?;
        let deps = index.file_dependencies("file:///c.pl");
        assert!(deps.contains("Foo::Base"), "deps: {deps:?}");
        assert!(deps.contains("Bar::Base"), "deps: {deps:?}");
        assert!(
            !deps.iter().any(|d| d.contains("qw")),
            "no bogus qw-prefixed dependency should be recorded, got {deps:?}"
        );
        Ok(())
    }

    /// `parse_qw_content` unit coverage: whitespace before the delimiter is
    /// tolerated; compact form still works.
    /// Covers space, tab, and newline — all `trim_start` whitespace variants.
    #[test]
    fn parse_qw_content_tolerates_leading_space() {
        // Space before delimiter.
        assert_eq!(WorkspaceIndex::parse_qw_content("qw [a b]"), Some("a b"));
        assert_eq!(WorkspaceIndex::parse_qw_content("qw(a b)"), Some("a b"));
        assert_eq!(WorkspaceIndex::parse_qw_content("qw/a b/"), Some("a b"));
        // Tab before delimiter.
        assert_eq!(WorkspaceIndex::parse_qw_content("qw\t[a b]"), Some("a b"));
        assert_eq!(WorkspaceIndex::parse_qw_content("qw\t(a b)"), Some("a b"));
        // Newline before delimiter.
        assert_eq!(WorkspaceIndex::parse_qw_content("qw\n[a b]"), Some("a b"));
        // Multiple mixed whitespace.
        assert_eq!(WorkspaceIndex::parse_qw_content("qw  \t [a b]"), Some("a b"));
        // A bareword directly after qw is not a valid delimiter.
        assert_eq!(WorkspaceIndex::parse_qw_content("qwfoo"), None);
    }

    /// Direct boundary test for the `end < start` guard in `parse_qw_content`.
    ///
    /// `rfind(close)` can land at index 0 when the opening character is also
    /// the only occurrence of `close` in `rest` (e.g. `qwf` where `f` is both
    /// open and close for the symmetric-delimiter arm).  In that case
    /// `end (= 0) < start (= open.len_utf8() = 1)`, and the guard must return
    /// `None` without constructing the invalid slice `&rest[1..0]`.
    ///
    /// This test is a RIPR seam anchor: it asserts the exact discriminating
    /// input that sits on the `end < start` boundary so that any mutation
    /// removing that guard is caught immediately.
    #[test]
    fn parse_qw_content_end_lt_start_guard_fires() {
        // "qwf" → rest = "f", open = 'f', close = 'f' (symmetric),
        // start = 1, rfind('f') = 0 → end = 0 < start = 1 → None.
        assert_eq!(
            WorkspaceIndex::parse_qw_content("qwf"),
            None,
            "end < start boundary: single-char symmetric delimiter must return None, not panic"
        );
        // Longer bareword: same logic, rfind finds the *last* occurrence of
        // 'f' at index 2, which is still < start = 1? No — "qwfoo":
        // rest = "foo", open = 'f', close = 'f', rfind('f') in "foo" = 0
        // → end = 0 < start = 1 → None.
        assert_eq!(
            WorkspaceIndex::parse_qw_content("qwfoo"),
            None,
            "end < start boundary: bareword after qw must return None"
        );
    }

    #[test]
    fn test_workspace_index() {
        let mut index = WorkspaceIndex::new();

        // Create a mock symbol table
        let mut symtab = SymbolTable::new();

        // Add a symbol to the symbol table
        let symbol = Symbol {
            name: "test_func".to_string(),
            qualified_name: "main::test_func".to_string(),
            kind: SymbolKind::Subroutine,
            location: SourceLocation { start: 0, end: 10 },
            scope_id: 0,
            declaration: Some("sub".to_string()),
            documentation: None,
            attributes: Vec::new(),
        };

        symtab.symbols.entry("test_func".to_string()).or_default().push(symbol);

        // Add document to index
        index.update_from_document("file:///test.pl", "", &symtab);

        // Find definitions
        let defs = index.find_defs("test_func");
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "test_func");
        assert_eq!(defs[0].uri, "file:///test.pl");

        // Remove document
        index.remove_document("file:///test.pl");
        assert_eq!(index.find_defs("test_func").len(), 0);
    }
}
