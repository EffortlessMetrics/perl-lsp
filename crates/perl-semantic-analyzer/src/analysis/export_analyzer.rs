//! Export symbol extraction for Exporter-based Perl modules
//!
//! This module provides functionality to extract export information from Perl modules
//! that use the Exporter framework. It detects three inheritance patterns and parses
//! the `@EXPORT`, `@EXPORT_OK`, and `%EXPORT_TAGS` arrays.
//!
//! # Exporter Detection Patterns
//!
//! A module is considered an Exporter if it matches any of:
//! - `use Exporter;` (Use node with module="Exporter" and empty args — bare form)
//! - `use Exporter 'import';` (Use node with module="Exporter" and args containing "import")
//! - `use parent 'Exporter';` (Use node with module="parent" and args containing "Exporter")
//! - `our @ISA = qw(Exporter);` (VariableDeclaration with @ISA array containing "Exporter")
//!
//! # Export Array Format
//!
//! The parser supports all Perl qw() delimiters:
//! - `@EXPORT = qw(foo bar)` — parentheses
//! - `@EXPORT = [qw(foo bar)]` — brackets
//! - `@EXPORT = qw<foo bar>` — angle brackets
//! - `@EXPORT = qw/foo bar/` — slashes
//! - `@EXPORT = qw|foo bar|` — pipes

use crate::ast::{Node, NodeKind};
use std::collections::{HashMap, HashSet};

/// Information extracted from an Exporter-based module.
#[derive(Debug, Clone, Default)]
pub struct ExportInfo {
    /// Symbols exported via `@EXPORT` (default exports)
    pub default_export: HashSet<String>,
    /// Symbols exported via `@EXPORT_OK` (optional exports)
    pub optional_export: HashSet<String>,
    /// Tag-based exports via `%EXPORT_TAGS` (tag name -> symbols)
    pub export_tags: HashMap<String, Vec<String>>,
}

/// Detection method for Exporter inheritance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExporterDetector {
    /// Detected via `use Exporter 'import';`
    UseExporterImport,
    /// Detected via `use parent 'Exporter';`
    UseParentExporter,
    /// Detected via `our @ISA = qw(Exporter);`
    OurIsaExporter,
}

/// Export symbol extractor for Exporter-based Perl modules.
///
/// This extractor walks the AST to:
/// 1. Detect if a module uses Exporter (via one of three patterns)
/// 2. Parse `@EXPORT`, `@EXPORT_OK`, and `%EXPORT_TAGS` assignments
pub struct ExportSymbolExtractor;

impl ExportSymbolExtractor {
    /// Extract export information from an AST.
    ///
    /// Returns `None` if the module does not use Exporter.
    /// Returns `Some(ExportInfo)` with empty sets if the module uses Exporter
    /// but does not define any export arrays.
    pub fn extract(ast: &Node) -> Option<ExportInfo> {
        let detector = Self::detect_exporter_inheritance(ast)?;

        let mut info = ExportInfo::default();

        // Walk the AST to find export array assignments
        Self::walk_and_extract_exports(ast, &detector, &mut info);

        Some(info)
    }

    /// Detect if the AST represents an Exporter-based module.
    ///
    /// Checks for three patterns:
    /// 1. `use Exporter 'import';`
    /// 2. `use parent 'Exporter';`
    /// 3. `our @ISA = qw(Exporter);`
    fn detect_exporter_inheritance(ast: &Node) -> Option<ExporterDetector> {
        Self::walk_for_exporter_detection(ast)
    }

    /// Walk AST looking for Exporter inheritance patterns.
    fn walk_for_exporter_detection(ast: &Node) -> Option<ExporterDetector> {
        match &ast.kind {
            // Pattern 1: `use Exporter 'import';` or `use Exporter;` (no-args form)
            //
            // `use Exporter;` without 'import' is valid and extremely common in CPAN code —
            // the module is loaded but callers must invoke `Exporter::import` explicitly, or
            // rely on `@EXPORT` being populated before import time.  We treat both forms as
            // Exporter-based so that @EXPORT/@EXPORT_OK are still extracted.
            NodeKind::Use { module, args, .. } if module == "Exporter" => {
                // Accept `use Exporter;` (args empty) or `use Exporter 'import';`
                if args.is_empty()
                    || args.iter().any(|arg| {
                        let arg_stripped = arg.trim_matches('\'');
                        arg_stripped == "import" || arg == "import"
                    })
                {
                    return Some(ExporterDetector::UseExporterImport);
                }
            }
            // Pattern 2: `use parent 'Exporter';`
            NodeKind::Use { module, args, .. } if module == "parent" => {
                // Check if 'Exporter' is in the arguments (args may contain quoted strings)
                if args.iter().any(|arg| {
                    let arg_stripped = arg.trim_matches('\'');
                    arg_stripped == "Exporter" || arg == "Exporter"
                }) {
                    return Some(ExporterDetector::UseParentExporter);
                }
            }
            // Pattern 3: `our @ISA = qw(Exporter);`
            NodeKind::VariableDeclaration { variable, initializer: Some(init), .. } => {
                if let NodeKind::Variable { sigil, name } = &variable.kind {
                    if sigil == "@" && name == "ISA" {
                        // init is &Box<Node>, pass directly (auto-deref to &Node)
                        if Self::initializer_contains_exporter(init) {
                            return Some(ExporterDetector::OurIsaExporter);
                        }
                    }
                }
            }
            _ => {}
        }

        // If no pattern matched at this node, recurse into children.
        // This handles cases where Exporter inheritance is declared in nested scopes
        // or after other statements in the package body.
        for child in ast.children() {
            if let Some(detector) = Self::walk_for_exporter_detection(child) {
                return Some(detector);
            }
        }

        None
    }

    /// Check if an initializer node contains 'Exporter'.
    fn initializer_contains_exporter(init: &Node) -> bool {
        match &init.kind {
            // Array or list literal (e.g., qw(Exporter) or [qw(Exporter)])
            NodeKind::ArrayLiteral { elements } => elements.iter().any(Self::node_is_exporter),
            // For simple strings
            NodeKind::String { value, .. } => {
                let s_stripped = value.trim_matches('\'');
                s_stripped == "Exporter" || value == "Exporter"
            }
            _ => false,
        }
    }

    /// Check if a node contains 'Exporter'.
    fn node_is_exporter(node: &Node) -> bool {
        match &node.kind {
            NodeKind::String { value, .. } => {
                let s_stripped = value.trim_matches('\'');
                s_stripped == "Exporter" || value == "Exporter"
            }
            NodeKind::ArrayLiteral { elements } => elements.iter().any(Self::node_is_exporter),
            _ => false,
        }
    }

    /// Walk AST and extract export arrays.
    ///
    /// The `_detector` parameter is accepted but unused (marked with underscore prefix).
    /// It is kept in the signature for API symmetry with the detection phase and to allow
    /// future pattern-specific extraction logic without changing the interface.
    fn walk_and_extract_exports(ast: &Node, _detector: &ExporterDetector, info: &mut ExportInfo) {
        match &ast.kind {
            NodeKind::VariableDeclaration { variable, initializer: Some(init), .. } => {
                if let NodeKind::Variable { sigil, name } = &variable.kind {
                    if sigil == "@" {
                        match name.as_str() {
                            "EXPORT" => {
                                // init is &Box<Node>, auto-derefs to &Node
                                let symbols = Self::parse_qw_array(init);
                                info.default_export.extend(symbols);
                            }
                            "EXPORT_OK" => {
                                let symbols = Self::parse_qw_array(init);
                                info.optional_export.extend(symbols);
                            }
                            _ => {}
                        }
                    } else if sigil == "%" && name == "EXPORT_TAGS" {
                        let tags = Self::parse_export_tags(init);
                        info.export_tags.extend(tags);
                    }
                }

                // Continue walking for nested declarations
                Self::walk_and_extract_exports(init, _detector, info);
            }
            _ => {
                // Walk children
                for child in ast.children() {
                    Self::walk_and_extract_exports(child, _detector, info);
                }
            }
        }
    }

    /// Parse a qw() array from an initializer node.
    ///
    /// Handles all Perl qw delimiters: (), [], {}, <>, //, ||
    ///
    /// The input node can be:
    /// - An ArrayLiteral with String elements (from `qw(...)`)
    /// - An ArrayLiteral with one ArrayLiteral element (from `[qw(...)]`)
    /// - A HashLiteral (from `%EXPORT_TAGS = (...)`)
    /// - Other expression types
    fn parse_qw_array(node: &Node) -> Vec<String> {
        match &node.kind {
            // ArrayLiteral: `(1, 2, 3)` or `[1, 2, 3]` containing strings
            NodeKind::ArrayLiteral { elements } => {
                if elements.is_empty() {
                    return Vec::new();
                }
                // Check if this ArrayLiteral contains only one element which is itself an ArrayLiteral
                // This happens with `[qw(tag_a tag_b)]` where the outer [...] creates an ArrayLiteral
                // containing the result of qw()
                if elements.len() == 1 {
                    if let NodeKind::ArrayLiteral { .. } = &elements[0].kind {
                        // Recursively parse the inner array which contains the actual strings
                        return Self::parse_qw_array(&elements[0]);
                    }
                }
                // Normal case: ArrayLiteral with direct String elements
                elements
                    .iter()
                    .filter_map(|elem| {
                        // Handle String nodes from qw()
                        if let NodeKind::String { value, .. } = &elem.kind {
                            Some(value.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            // Binary expression for concatenation
            NodeKind::Binary { op, left, right } if op == "." => {
                // Handle "foo" . "bar" form (rare, but possible)
                let mut result = Vec::new();
                if let NodeKind::String { value, .. } = &left.kind {
                    result.push(value.clone());
                }
                if let NodeKind::String { value, .. } = &right.kind {
                    result.push(value.clone());
                }
                result
            }
            // Handle parenthesized expressions like `('foo', 'bar')`
            // which might be wrapped in a Block or other node types
            _ => {
                // Try walking children if this node itself isn't a qw array
                let mut symbols = Vec::new();
                for child in node.children() {
                    symbols.extend(Self::parse_qw_array(child));
                }
                symbols
            }
        }
    }

    /// Parse `%EXPORT_TAGS` hash from an initializer node.
    ///
    /// The hash format is:
    /// ```perl
    /// %EXPORT_TAGS = (
    ///     tag1 => [qw(a b c)],
    ///     tag2 => [qw(d e f)],
    /// );
    /// ```
    ///
    /// Returns a map from tag name to list of exported symbols.
    fn parse_export_tags(node: &Node) -> HashMap<String, Vec<String>> {
        let mut tags: HashMap<String, Vec<String>> = HashMap::new();

        match &node.kind {
            // HashLiteral: `{ key => value, ... }`
            NodeKind::HashLiteral { pairs } => {
                let mut i = 0;
                while i < pairs.len() {
                    let (key_node, value_node) = &pairs[i];

                    // Get the tag name from the key
                    if let Some(tag_name) = Self::extract_string_value(key_node) {
                        // The value should be an ArrayLiteral containing symbol names
                        let symbols = Self::parse_qw_array(value_node);
                        if !symbols.is_empty() {
                            tags.insert(tag_name, symbols);
                        }
                    }
                    i += 1;
                }
            }
            // If it's not a HashLiteral, try to walk children to find hash pairs
            _ => {
                Self::walk_and_extract_export_tags(node, &mut tags);
            }
        }

        tags
    }

    /// Walk a node to extract export tags.
    fn walk_and_extract_export_tags(node: &Node, tags: &mut HashMap<String, Vec<String>>) {
        match &node.kind {
            NodeKind::HashLiteral { pairs } => {
                let mut i = 0;
                while i < pairs.len() {
                    let (key_node, value_node) = &pairs[i];

                    if let Some(tag_name) = Self::extract_string_value(key_node) {
                        let symbols = Self::parse_qw_array(value_node);
                        if !symbols.is_empty() {
                            tags.insert(tag_name, symbols);
                        }
                    }
                    i += 1;
                }
            }
            _ => {
                for child in node.children() {
                    Self::walk_and_extract_export_tags(child, tags);
                }
            }
        }
    }

    /// Extract a string value from a node.
    fn extract_string_value(node: &Node) -> Option<String> {
        match &node.kind {
            NodeKind::String { value, .. } => Some(value.clone()),
            NodeKind::Identifier { name } => Some(name.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeKind;
    use crate::Parser;

    fn parse_and_extract(code: &str) -> Option<ExportInfo> {
        let mut parser = Parser::new(code);
        let ast = parser.parse().ok()?;
        ExportSymbolExtractor::extract(&ast)
    }

    #[test]
    fn debug_ast_structure() {
        let code = r#"
package MyModule;
use Exporter 'import';
our %EXPORT_TAGS = (
    tag1 => [qw(tag_a tag_b)],
);
1;
"#;
        let mut parser = Parser::new(code);
        let ast = parser.parse().expect("parse should succeed");

        // Walk and print node kinds
        fn walk(node: &crate::Node, depth: usize) {
            let indent = "  ".repeat(depth);
            eprintln!("{}{:?}", indent, node.kind.kind_name());
            // Print details for key node types
            match &node.kind {
                NodeKind::VariableDeclaration { variable, initializer, .. } => {
                    if let NodeKind::Variable { sigil, name } = &variable.kind {
                        eprintln!("{}  variable: {}{}", indent, sigil, name);
                    }
                    if let Some(init) = initializer {
                        eprintln!("{}  initializer: {:?}", indent, init.kind.kind_name());
                    }
                }
                NodeKind::HashLiteral { pairs } => {
                    eprintln!("{}  HashLiteral with {} pairs", indent, pairs.len());
                    for (i, (k, v)) in pairs.iter().enumerate() {
                        eprintln!(
                            "{}    pair[{}]: key_kind={:?}, value_kind={:?}",
                            indent,
                            i,
                            k.kind.kind_name(),
                            v.kind.kind_name()
                        );
                    }
                }
                NodeKind::ArrayLiteral { elements } => {
                    eprintln!("{}  ArrayLiteral with {} elements", indent, elements.len());
                }
                _ => {}
            }
            for child in node.children() {
                walk(child, depth + 1);
            }
        }
        walk(&ast, 0);

        // Now try extraction
        let info = ExportSymbolExtractor::extract(&ast);
        eprintln!("Export info: {:?}", info);
        assert!(info.is_some());
        let info = info.unwrap();
        eprintln!("export_tags: {:?}", info.export_tags);
        assert_eq!(info.export_tags.len(), 1);
    }

    #[test]
    fn test_detect_use_exporter_import() {
        let code = r#"
package MyUtils;
use Exporter 'import';
our @EXPORT = qw(foo bar);
1;
"#;
        let info = parse_and_extract(code);
        assert!(info.is_some(), "Should detect Exporter, got {:?}", info);
        let info = info.unwrap();
        assert!(info.default_export.contains("foo"));
        assert!(info.default_export.contains("bar"));
    }

    #[test]
    fn test_detect_use_parent_exporter() {
        let code = r#"
package MyModule;
use parent 'Exporter';
our @EXPORT = qw(default_func);
1;
"#;
        let info = parse_and_extract(code);
        assert!(info.is_some(), "Should detect parent Exporter");
        let info = info.unwrap();
        assert!(info.default_export.contains("default_func"));
    }

    #[test]
    fn test_detect_our_isa_exporter() {
        let code = r#"
package MyClass;
our @ISA = qw(Exporter);
our @EXPORT = qw(inherited_func);
1;
"#;
        let info = parse_and_extract(code);
        assert!(info.is_some(), "Should detect @ISA Exporter");
        let info = info.unwrap();
        assert!(info.default_export.contains("inherited_func"));
    }

    #[test]
    fn test_export_ok() {
        let code = r#"
package MyLib;
use Exporter 'import';
our @EXPORT_OK = qw(optional_a optional_b);
1;
"#;
        let info = parse_and_extract(code).unwrap();
        assert!(info.optional_export.contains("optional_a"));
        assert!(info.optional_export.contains("optional_b"));
    }

    #[test]
    fn test_export_tags() {
        let code = r#"
package Color;
use Exporter 'import';
our @EXPORT_OK = qw(red green blue rgb hex);
our %EXPORT_TAGS = (
    primary => [qw(red green blue)],
    formats => [qw(rgb hex)],
);
1;
"#;
        let info = parse_and_extract(code).unwrap();
        let primary = info.export_tags.get("primary");
        assert!(primary.is_some());
        let primary = primary.unwrap();
        assert!(primary.contains(&"red".to_string()));
        assert!(primary.contains(&"green".to_string()));
        assert!(primary.contains(&"blue".to_string()));

        let formats = info.export_tags.get("formats").unwrap();
        assert!(formats.contains(&"rgb".to_string()));
        assert!(formats.contains(&"hex".to_string()));
    }

    #[test]
    fn test_no_exporter_no_extraction() {
        let code = r#"
package MyModule;
our @EXPORT = qw(not_exported);
1;
"#;
        let info = parse_and_extract(code);
        // Without Exporter inheritance, no export info should be extracted
        // because we don't want false positives
        assert!(
            info.is_none() || info.as_ref().map(|i| i.default_export.is_empty()).unwrap_or(false)
        );
    }

    #[test]
    fn test_empty_export_arrays() {
        let code = r#"
package MyModule;
use Exporter 'import';
our @EXPORT = ();
our @EXPORT_OK = ();
our %EXPORT_TAGS = ();
1;
"#;
        let info = parse_and_extract(code).unwrap();
        assert!(info.default_export.is_empty());
        assert!(info.optional_export.is_empty());
        assert!(info.export_tags.is_empty());
    }

    #[test]
    fn test_multiple_arrays() {
        let code = r#"
package MyModule;
use Exporter 'import';
our @EXPORT = qw(default_a default_b);
our @EXPORT_OK = qw(optional_c optional_d);
our %EXPORT_TAGS = (
    tag1 => [qw(tag_a tag_b)],
);
1;
"#;
        let info = parse_and_extract(code).unwrap();
        assert_eq!(info.default_export.len(), 2);
        assert!(info.default_export.contains("default_a"));
        assert!(info.default_export.contains("default_b"));

        assert_eq!(info.optional_export.len(), 2);
        assert!(info.optional_export.contains("optional_c"));
        assert!(info.optional_export.contains("optional_d"));

        assert_eq!(info.export_tags.len(), 1);
    }

    #[test]
    fn test_detect_use_exporter_no_args() {
        // `use Exporter;` (no 'import' argument) is common in CPAN code and must
        // also trigger export extraction.
        let code = r#"
package MyUtils;
use Exporter;
our @EXPORT = qw(legacy_func);
1;
"#;
        let info = parse_and_extract(code);
        assert!(
            info.is_some(),
            "Should detect bare `use Exporter;` as Exporter-based module"
        );
        let info = info.unwrap();
        assert!(
            info.default_export.contains("legacy_func"),
            "Should extract @EXPORT symbols from bare use Exporter; module"
        );
    }
}
