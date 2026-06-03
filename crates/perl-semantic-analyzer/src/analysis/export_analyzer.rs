//! Export symbol extraction for Exporter-based Perl modules
//!
//! This module provides functionality to extract export information from Perl modules
//! that use the Exporter framework. It detects four inheritance patterns and parses
//! the `@EXPORT`, `@EXPORT_OK`, and `%EXPORT_TAGS` arrays.
//!
//! # Exporter Detection Patterns
//!
//! A module is considered an Exporter if it matches any of:
//! - `use Exporter;` (Use node with module="Exporter" and empty args — bare form)
//! - `use Exporter 'import';` (Use node with module="Exporter" and args containing "import")
//! - `use parent 'Exporter';` or `use parent qw(Exporter);` (Use node with module="parent")
//! - `use base 'Exporter';` or `use base qw(Exporter);` (Use node with module="base")
//! - `our @ISA = qw(Exporter);` (VariableDeclaration with @ISA array containing "Exporter")
//! - `@ISA = qw(Exporter);` (bare Assignment with @ISA array containing "Exporter")
//!
//! # Export Array Format
//!
//! The parser supports all Perl qw() delimiters:
//! - `@EXPORT = qw(foo bar)` — parentheses
//! - `@EXPORT = [qw(foo bar)]` — brackets
//! - `@EXPORT = qw<foo bar>` — angle brackets
//! - `@EXPORT = qw/foo bar/` — slashes
//! - `@EXPORT = qw|foo bar|` — pipes
//!
//! Both `our @EXPORT = ...` (VariableDeclaration) and bare `@EXPORT = ...` (Assignment)
//! forms are extracted.

use crate::ast::{Node, NodeKind};
use perl_semantic_facts::{AnchorId, Confidence, ExportSet, ExportTag, Provenance};
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
    /// Package name extracted from the AST's `package` declaration.
    pub module_name: Option<String>,
    /// Anchor ID derived from the first export declaration's byte span.
    pub anchor_id: Option<AnchorId>,
    /// True when the module was detected via a custom `sub import` (not Exporter).
    /// When true, export sets are empty and confidence is Low.
    pub custom_import: bool,
}

impl ExportInfo {
    /// Convert extracted Exporter data into canonical semantic export facts.
    #[must_use]
    pub fn to_export_set(&self) -> ExportSet {
        let mut default_exports: Vec<String> = self.default_export.iter().cloned().collect();
        default_exports.sort();

        let mut optional_exports: Vec<String> = self.optional_export.iter().cloned().collect();
        optional_exports.sort();

        let mut tags: Vec<ExportTag> = self
            .export_tags
            .iter()
            .map(|(name, members)| {
                let mut members = members.clone();
                members.sort();
                members.dedup();
                ExportTag { name: name.clone(), members }
            })
            .collect();
        tags.sort_by(|left, right| left.name.cmp(&right.name));

        ExportSet {
            default_exports,
            optional_exports,
            tags,
            provenance: Provenance::ImportExportInference,
            confidence: if self.custom_import { Confidence::Low } else { Confidence::High },
            module_name: self.module_name.clone(),
            anchor_id: self.anchor_id,
        }
    }
}

/// Detection method for Exporter inheritance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExporterDetector {
    /// Detected via `use Exporter;` or `use Exporter 'import';`
    UseExporterImport,
    /// Detected via `use parent 'Exporter';` or `use parent qw(Exporter ...)`
    UseParentExporter,
    /// Detected via `use base 'Exporter';` or `use base qw(Exporter ...)`
    UseBaseExporter,
    /// Detected via `our @ISA = qw(Exporter ...);` or bare `@ISA = qw(Exporter ...);`
    OurIsaExporter,
    /// Detected via a custom `sub import { ... }` (no Exporter inheritance).
    /// Export lists are unknown; confidence is Low.
    CustomImport,
}

/// Export symbol extractor for Exporter-based Perl modules.
///
/// This extractor walks the AST to:
/// 1. Detect if a module uses Exporter (via one of four patterns)
/// 2. Parse `@EXPORT`, `@EXPORT_OK`, and `%EXPORT_TAGS` assignments
pub struct ExportSymbolExtractor;

impl ExportSymbolExtractor {
    /// Extract export information from an AST.
    ///
    /// Returns `None` only when the module has neither Exporter inheritance
    /// (`use Exporter;`, `use parent 'Exporter';`, `use base 'Exporter';`,
    /// or `@ISA = qw(Exporter ...);`) nor a custom `sub import { ... }` definition.
    /// Returns `Some(ExportInfo)` with empty sets otherwise; modules whose only
    /// signal is a custom `sub import` get `custom_import: true` and
    /// `confidence: Low` on the returned info.
    pub fn extract(ast: &Node) -> Option<ExportInfo> {
        let detector = Self::detect_exporter_inheritance(ast).or_else(|| {
            if Self::detect_custom_import(ast) {
                Some(ExporterDetector::CustomImport)
            } else {
                None
            }
        })?;

        let custom_import = matches!(detector, ExporterDetector::CustomImport);

        let mut info = ExportInfo {
            // Extract the package name from the AST.
            module_name: Self::find_package_name(ast),
            // Derive anchor_id from the first export declaration's byte span.
            anchor_id: Self::find_first_export_anchor(ast),
            custom_import,
            ..Default::default()
        };

        // Walk the AST to find export array assignments
        Self::walk_and_extract_exports(ast, &detector, &mut info);

        Some(info)
    }

    /// Detect if the AST represents an Exporter-based module.
    ///
    /// Checks for four patterns:
    /// 1. `use Exporter;` or `use Exporter 'import';`
    /// 2. `use parent 'Exporter';` or `use parent qw(Exporter ...)`
    /// 3. `use base 'Exporter';` or `use base qw(Exporter ...)`
    /// 4. `our @ISA = qw(Exporter ...);` or bare `@ISA = qw(Exporter ...);`
    fn detect_exporter_inheritance(ast: &Node) -> Option<ExporterDetector> {
        Self::walk_for_exporter_detection(ast)
    }

    /// Detect if the AST contains a custom `sub import { ... }` definition.
    ///
    /// Returns `true` when the module defines its own `import` subroutine,
    /// indicating dynamic export behaviour that cannot be statically analysed.
    fn detect_custom_import(ast: &Node) -> bool {
        match &ast.kind {
            NodeKind::Subroutine { name: Some(n), .. } if n == "import" => return true,
            _ => {}
        }
        for child in ast.children() {
            if Self::detect_custom_import(child) {
                return true;
            }
        }
        false
    }

    /// Walk the AST to find the first `package` declaration and return its name.
    fn find_package_name(ast: &Node) -> Option<String> {
        match &ast.kind {
            NodeKind::Package { name, .. } => Some(name.clone()),
            _ => {
                for child in ast.children() {
                    if let Some(name) = Self::find_package_name(child) {
                        return Some(name);
                    }
                }
                None
            }
        }
    }

    /// Walk the AST to find the first `@EXPORT`, `@EXPORT_OK`, or `%EXPORT_TAGS`
    /// declaration and derive an [`AnchorId`] from its byte-offset span.
    fn find_first_export_anchor(ast: &Node) -> Option<AnchorId> {
        Self::walk_for_first_export_anchor(ast)
    }

    /// Recursive helper for [`Self::find_first_export_anchor`].
    fn walk_for_first_export_anchor(node: &Node) -> Option<AnchorId> {
        match &node.kind {
            // `our @EXPORT = ...` or `our @EXPORT_OK = ...` or `our %EXPORT_TAGS = ...`
            NodeKind::VariableDeclaration { variable, initializer: Some(_), .. } => {
                if let NodeKind::Variable { sigil, name } = &variable.kind {
                    let is_export_var = (sigil == "@" && (name == "EXPORT" || name == "EXPORT_OK"))
                        || (sigil == "%" && name == "EXPORT_TAGS");
                    if is_export_var {
                        return Some(AnchorId(node.location.start as u64));
                    }
                }
            }
            // `@EXPORT = ...` or `@EXPORT_OK = ...` or `%EXPORT_TAGS = ...` (bare)
            NodeKind::Assignment { lhs, .. } => {
                if let NodeKind::Variable { sigil, name } = &lhs.kind {
                    let is_export_var = (sigil == "@" && (name == "EXPORT" || name == "EXPORT_OK"))
                        || (sigil == "%" && name == "EXPORT_TAGS");
                    if is_export_var {
                        return Some(AnchorId(node.location.start as u64));
                    }
                }
            }
            _ => {}
        }

        for child in node.children() {
            if let Some(anchor) = Self::walk_for_first_export_anchor(child) {
                return Some(anchor);
            }
        }

        None
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
            // Pattern 2: `use parent 'Exporter';` or `use parent qw(Exporter ...)`
            //
            // The parser stores qw-lists as a single normalised string like `"qw(Exporter)"`,
            // so we must check both single-quoted strings and the qw-expanded form.
            NodeKind::Use { module, args, .. } if module == "parent" => {
                if args.iter().any(|arg| Self::arg_contains_exporter(arg)) {
                    return Some(ExporterDetector::UseParentExporter);
                }
            }
            // Pattern 3: `use base 'Exporter';` or `use base qw(Exporter ...)`
            //
            // `use base` is the older form of `use parent` and is still widely used in
            // legacy CPAN code. The same qw-normalisation applies.
            NodeKind::Use { module, args, .. } if module == "base" => {
                if args.iter().any(|arg| Self::arg_contains_exporter(arg)) {
                    return Some(ExporterDetector::UseBaseExporter);
                }
            }
            // Pattern 4a: `our @ISA = qw(Exporter ...);` (declared form)
            NodeKind::VariableDeclaration { variable, initializer: Some(init), .. } => {
                if let NodeKind::Variable { sigil, name } = &variable.kind {
                    if sigil == "@" && name == "ISA" && Self::initializer_contains_exporter(init) {
                        return Some(ExporterDetector::OurIsaExporter);
                    }
                }
            }
            // Pattern 4b: `@ISA = qw(Exporter ...);` (bare assignment without `our`)
            NodeKind::Assignment { lhs, rhs, .. } => {
                if let NodeKind::Variable { sigil, name } = &lhs.kind {
                    if sigil == "@" && name == "ISA" && Self::initializer_contains_exporter(rhs) {
                        return Some(ExporterDetector::OurIsaExporter);
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

    /// Check whether a single `Use` argument string contains `Exporter`.
    ///
    /// The parser normalises `qw(Foo Bar)` forms to the string `"qw(Foo Bar)"`.
    /// Single-quoted module names arrive as `"'Exporter'"`.
    fn arg_contains_exporter(arg: &str) -> bool {
        let arg = arg.trim();
        // Single- or double-quoted: 'Exporter' or "Exporter"
        if arg.trim_matches('\'').trim_matches('"') == "Exporter" {
            return true;
        }
        // qw(...) normalised form: "qw(Exporter)" or "qw(SomeBase Exporter OtherBase)"
        if arg.starts_with("qw") {
            // Find the content between the outer delimiters
            let open_pos = arg.find(|c: char| !c.is_alphanumeric()).unwrap_or(arg.len());
            let close = match arg[open_pos..].chars().next() {
                Some('(') => ')',
                Some('{') => '}',
                Some('[') => ']',
                Some('<') => '>',
                Some(c) => c,
                None => return false,
            };
            if let (Some(start), Some(end)) =
                (arg[open_pos..].find(|c: char| !c.is_whitespace()), arg.rfind(close))
            {
                let content = &arg[open_pos + start + 1..end];
                return content.split_whitespace().any(|w| w == "Exporter");
            }
        }
        false
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
    /// Short-circuits immediately for `CustomImport` — static export lists are
    /// not available when the module uses a custom `sub import`.
    fn walk_and_extract_exports(ast: &Node, detector: &ExporterDetector, info: &mut ExportInfo) {
        if matches!(detector, ExporterDetector::CustomImport) {
            return;
        }
        match &ast.kind {
            // `our @EXPORT = qw(...)` (declared form)
            NodeKind::VariableDeclaration { variable, initializer: Some(init), .. } => {
                if let NodeKind::Variable { sigil, name } = &variable.kind {
                    if sigil == "@" {
                        match name.as_str() {
                            "EXPORT" => {
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
                Self::walk_and_extract_exports(init, detector, info);
            }
            // `@EXPORT = qw(...)` (bare assignment without `our`)
            NodeKind::Assignment { lhs, rhs, .. } => {
                if let NodeKind::Variable { sigil, name } = &lhs.kind {
                    if sigil == "@" {
                        match name.as_str() {
                            "EXPORT" => {
                                let symbols = Self::parse_qw_array(rhs);
                                info.default_export.extend(symbols);
                            }
                            "EXPORT_OK" => {
                                let symbols = Self::parse_qw_array(rhs);
                                info.optional_export.extend(symbols);
                            }
                            _ => {}
                        }
                    } else if sigil == "%" && name == "EXPORT_TAGS" {
                        let tags = Self::parse_export_tags(rhs);
                        info.export_tags.extend(tags);
                    }
                }
                // Walk into rhs for nested assignments
                Self::walk_and_extract_exports(rhs, detector, info);
            }
            _ => {
                // Walk children
                for child in ast.children() {
                    Self::walk_and_extract_exports(child, detector, info);
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
                for (key_node, value_node) in pairs {
                    if let Some(tag_name) = Self::extract_string_value(key_node) {
                        let symbols = Self::parse_qw_array(value_node);
                        if !symbols.is_empty() {
                            tags.insert(tag_name, symbols);
                        }
                    }
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
                for (key_node, value_node) in pairs {
                    if let Some(tag_name) = Self::extract_string_value(key_node) {
                        let symbols = Self::parse_qw_array(value_node);
                        if !symbols.is_empty() {
                            tags.insert(tag_name, symbols);
                        }
                    }
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
    use crate::Parser;

    fn parse_and_extract(code: &str) -> Option<ExportInfo> {
        let mut parser = Parser::new(code);
        let ast = parser.parse().ok()?;
        ExportSymbolExtractor::extract(&ast)
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
    fn test_detect_use_parent_exporter_qw_form() {
        // `use parent qw(Exporter)` is common; the parser normalises qw to "qw(Exporter)".
        let code = r#"
package MyModule;
use parent qw(Exporter);
our @EXPORT = qw(qw_parent_func);
1;
"#;
        let info = parse_and_extract(code);
        assert!(info.is_some(), "Should detect `use parent qw(Exporter)` as Exporter-based");
        let info = info.unwrap();
        assert!(info.default_export.contains("qw_parent_func"));
    }

    #[test]
    fn test_detect_use_base_exporter() {
        // `use base` is the older equivalent of `use parent`.
        let code = r#"
package Legacy;
use base 'Exporter';
our @EXPORT = qw(legacy_func);
1;
"#;
        let info = parse_and_extract(code);
        assert!(info.is_some(), "Should detect `use base 'Exporter'` as Exporter-based");
        let info = info.unwrap();
        assert!(info.default_export.contains("legacy_func"));
    }

    #[test]
    fn test_detect_use_base_exporter_qw_form() {
        let code = r#"
package Legacy;
use base qw(Exporter SomeOtherBase);
our @EXPORT = qw(base_qw_func);
1;
"#;
        let info = parse_and_extract(code);
        assert!(info.is_some(), "Should detect `use base qw(Exporter ...)` as Exporter-based");
        let info = info.unwrap();
        assert!(info.default_export.contains("base_qw_func"));
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
    fn test_detect_bare_isa_assignment() {
        // `@ISA = qw(Exporter)` without `our` is common in older Perl code.
        let code = r#"
package OldStyle;
@ISA = qw(Exporter);
@EXPORT = qw(old_func);
1;
"#;
        let info = parse_and_extract(code);
        assert!(info.is_some(), "Should detect bare `@ISA = qw(Exporter)` form");
        let info = info.unwrap();
        assert!(
            info.default_export.contains("old_func"),
            "Should extract @EXPORT from bare assignment form"
        );
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
        // Without any Exporter inheritance pattern, the extractor must return None.
        // A bare @EXPORT without use Exporter / use parent / @ISA is not enough.
        let code = r#"
package MyModule;
our @EXPORT = qw(not_exported);
1;
"#;
        let info = parse_and_extract(code);
        assert!(
            info.is_none(),
            "Should return None when no Exporter inheritance is detected, got {:?}",
            info
        );
    }

    #[test]
    fn test_custom_import_uses_low_confidence_unknown_exports() -> Result<(), String> {
        let code = r#"
package CustomExporter;
sub import {}
our @EXPORT = qw(static_symbol);
1;
"#;
        let info = parse_and_extract(code).ok_or("Expected custom import ExportInfo")?;

        assert!(info.custom_import, "custom import modules should be marked dynamic");
        assert!(
            info.default_export.is_empty(),
            "static-looking @EXPORT should not be claimed for custom import modules"
        );

        let export_set = info.to_export_set();
        assert_eq!(export_set.confidence, Confidence::Low);
        assert!(export_set.default_exports.is_empty());
        assert!(export_set.optional_exports.is_empty());
        assert!(export_set.tags.is_empty());
        Ok(())
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
        assert!(info.is_some(), "Should detect bare `use Exporter;` as Exporter-based module");
        let info = info.unwrap();
        assert!(
            info.default_export.contains("legacy_func"),
            "Should extract @EXPORT symbols from bare use Exporter; module"
        );
    }

    #[test]
    fn test_isa_with_multiple_parents_includes_exporter() {
        // When Exporter is one of multiple @ISA entries it must still be detected.
        let code = r#"
package Multi;
our @ISA = qw(SomeBase Exporter OtherBase);
our @EXPORT = qw(multi_func);
1;
"#;
        let info = parse_and_extract(code);
        assert!(info.is_some(), "Should detect Exporter even when mixed with other @ISA parents");
        let info = info.unwrap();
        assert!(info.default_export.contains("multi_func"));
    }
    #[test]
    fn test_regression_exporter_visibility_fixture() {
        let code = r#"
package MyLib;
use Exporter 'import';
our @EXPORT = qw(foo);
our @EXPORT_OK = qw(bar baz);
our %EXPORT_TAGS = (
    all => [qw(foo bar baz)],
);
1;
"#;
        let info = parse_and_extract(code).unwrap();

        assert_eq!(info.default_export.len(), 1);
        assert!(info.default_export.contains("foo"));

        assert_eq!(info.optional_export.len(), 2);
        assert!(info.optional_export.contains("bar"));
        assert!(info.optional_export.contains("baz"));

        let all = info.export_tags.get("all").unwrap();
        assert_eq!(all, &vec!["foo".to_string(), "bar".to_string(), "baz".to_string()]);
    }

    #[test]
    fn test_regression_merges_export_assignments_across_statements() {
        let code = r#"
package MyLib;
use Exporter 'import';
our @EXPORT = qw(foo);
our @EXPORT_OK = qw(bar);
our @EXPORT_OK = qw(bar baz);
our %EXPORT_TAGS = (core => [qw(foo bar)]);
our %EXPORT_TAGS = (all => [qw(foo bar baz)]);
1;
"#;
        let info = parse_and_extract(code).unwrap();

        assert!(info.default_export.contains("foo"));
        assert!(info.optional_export.contains("bar"));
        assert!(info.optional_export.contains("baz"));
        assert_eq!(
            info.export_tags.get("core").unwrap(),
            &vec!["foo".to_string(), "bar".to_string()]
        );
        assert_eq!(
            info.export_tags.get("all").unwrap(),
            &vec!["foo".to_string(), "bar".to_string(), "baz".to_string()]
        );
    }

    #[test]
    fn test_module_name_populated_from_package_declaration() -> Result<(), String> {
        let code = r#"
package My::Utils;
use Exporter 'import';
our @EXPORT = qw(helper);
1;
"#;
        let info = parse_and_extract(code).ok_or("Expected Some(ExportInfo)")?;
        assert_eq!(
            info.module_name.as_deref(),
            Some("My::Utils"),
            "module_name should be extracted from the package declaration"
        );
        Ok(())
    }

    #[test]
    fn test_module_name_propagated_to_export_set() -> Result<(), String> {
        let code = r#"
package Data::Formatter;
use parent 'Exporter';
our @EXPORT_OK = qw(format_csv);
1;
"#;
        let info = parse_and_extract(code).ok_or("Expected Some(ExportInfo)")?;
        let export_set = info.to_export_set();
        assert_eq!(
            export_set.module_name.as_deref(),
            Some("Data::Formatter"),
            "ExportSet.module_name should carry the package name"
        );
        Ok(())
    }

    #[test]
    fn test_anchor_id_populated_from_first_export_declaration() -> Result<(), String> {
        let code = r#"
package MyLib;
use Exporter 'import';
our @EXPORT = qw(foo);
1;
"#;
        let info = parse_and_extract(code).ok_or("Expected Some(ExportInfo)")?;
        assert!(
            info.anchor_id.is_some(),
            "anchor_id should be populated from the first export declaration"
        );
        Ok(())
    }

    #[test]
    fn test_anchor_id_propagated_to_export_set() -> Result<(), String> {
        let code = r#"
package MyLib;
use Exporter 'import';
our @EXPORT_OK = qw(bar baz);
1;
"#;
        let info = parse_and_extract(code).ok_or("Expected Some(ExportInfo)")?;
        let export_set = info.to_export_set();
        assert!(
            export_set.anchor_id.is_some(),
            "ExportSet.anchor_id should carry the first export declaration anchor"
        );
        Ok(())
    }

    #[test]
    fn test_anchor_id_none_when_no_export_arrays() -> Result<(), String> {
        // Module uses Exporter but declares no export arrays — anchor_id should be None.
        let code = r#"
package EmptyExporter;
use Exporter 'import';
1;
"#;
        let info = parse_and_extract(code).ok_or("Expected Some(ExportInfo)")?;
        assert!(
            info.anchor_id.is_none(),
            "anchor_id should be None when no export arrays are declared"
        );
        Ok(())
    }

    #[test]
    fn test_module_name_and_anchor_id_with_bare_assignment() -> Result<(), String> {
        let code = r#"
package OldStyle::Lib;
@ISA = qw(Exporter);
@EXPORT = qw(old_func);
1;
"#;
        let info = parse_and_extract(code).ok_or("Expected Some(ExportInfo)")?;
        assert_eq!(
            info.module_name.as_deref(),
            Some("OldStyle::Lib"),
            "module_name should work with bare assignment style"
        );
        assert!(
            info.anchor_id.is_some(),
            "anchor_id should be populated from bare @EXPORT assignment"
        );
        Ok(())
    }

    #[test]
    fn test_export_set_completeness_with_module_and_anchor() -> Result<(), String> {
        let code = r#"
package Full::Module;
use base 'Exporter';
our @EXPORT = qw(alpha beta);
our @EXPORT_OK = qw(gamma);
our %EXPORT_TAGS = (all => [qw(alpha beta gamma)]);
1;
"#;
        let info = parse_and_extract(code).ok_or("Expected Some(ExportInfo)")?;
        let export_set = info.to_export_set();

        // Verify module_name and anchor_id are present
        assert_eq!(export_set.module_name.as_deref(), Some("Full::Module"));
        assert!(export_set.anchor_id.is_some());

        // Verify export contents are still correct
        assert_eq!(export_set.default_exports, vec!["alpha", "beta"]);
        assert_eq!(export_set.optional_exports, vec!["gamma"]);
        assert_eq!(export_set.tags.len(), 1);
        assert_eq!(export_set.tags[0].name, "all");
        assert_eq!(export_set.tags[0].members, vec!["alpha", "beta", "gamma"]);
        Ok(())
    }
}
