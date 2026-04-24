//! Per-file metadata extraction for `Exporter`-style modules.

use crate::SourceLocation;
use crate::ast::{Node, NodeKind};
use std::collections::{HashMap, HashSet};

/// A same-file subroutine that can be exported by an `Exporter` package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportedSubroutine {
    /// Subroutine name (without leading sigils).
    pub name: String,
    /// Source location of the defining `sub` declaration.
    pub location: SourceLocation,
}

/// Export metadata captured for a single package declaration in a file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PackageExportMetadata {
    /// Package name that owns these exports.
    pub package: String,
    /// Default exports declared via `@EXPORT`.
    pub exports: Vec<ExportedSubroutine>,
    /// Optional exports declared via `@EXPORT_OK`.
    pub export_ok: Vec<ExportedSubroutine>,
    /// Tag-based exports declared via `%EXPORT_TAGS`.
    pub export_tags: HashMap<String, Vec<ExportedSubroutine>>,
}

/// Export metadata extracted from one parsed source file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileExportMetadata {
    /// Package-level export metadata entries discovered in this file.
    pub packages: Vec<PackageExportMetadata>,
}

#[derive(Default)]
struct PendingPackageExports {
    uses_exporter: bool,
    export_names: Vec<String>,
    export_ok_names: Vec<String>,
    export_tag_names: HashMap<String, Vec<String>>,
    subroutines: HashMap<String, SourceLocation>,
}

pub(super) struct ExportMetadataBuilder {
    current_package: String,
    current: PendingPackageExports,
    packages: Vec<PackageExportMetadata>,
}

impl Default for ExportMetadataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ExportMetadataBuilder {
    pub(super) fn new() -> Self {
        Self {
            current_package: "main".to_string(),
            current: PendingPackageExports::default(),
            packages: Vec::new(),
        }
    }

    pub(super) fn build(mut self, root: &Node) -> FileExportMetadata {
        self.visit(root);
        self.flush_current_package();
        FileExportMetadata { packages: self.packages }
    }

    fn flush_current_package(&mut self) {
        if !self.current.uses_exporter {
            self.current = PendingPackageExports::default();
            return;
        }

        let mut seen = HashSet::new();
        let resolve_names = |names: &[String],
                             subroutines: &HashMap<String, SourceLocation>,
                             seen: &mut HashSet<String>| {
            let mut resolved = Vec::new();
            for name in names {
                if let Some(location) = subroutines.get(name)
                    && seen.insert(name.clone())
                {
                    resolved.push(ExportedSubroutine { name: name.clone(), location: *location });
                }
            }
            resolved
        };

        let exports = resolve_names(&self.current.export_names, &self.current.subroutines, &mut seen);
        let export_ok =
            resolve_names(&self.current.export_ok_names, &self.current.subroutines, &mut seen);

        let mut export_tags = HashMap::new();
        for (tag, names) in &self.current.export_tag_names {
            let mut local_seen = HashSet::new();
            let mut resolved = Vec::new();
            for name in names {
                if let Some(location) = self.current.subroutines.get(name)
                    && local_seen.insert(name.clone())
                {
                    resolved.push(ExportedSubroutine { name: name.clone(), location: *location });
                }
            }
            if !resolved.is_empty() {
                export_tags.insert(tag.clone(), resolved);
            }
        }

        if !(exports.is_empty() && export_ok.is_empty() && export_tags.is_empty()) {
            self.packages.push(PackageExportMetadata {
                package: self.current_package.clone(),
                exports,
                export_ok,
                export_tags,
            });
        }

        self.current = PendingPackageExports::default();
    }

    fn visit_statement_list(&mut self, statements: &[Node]) {
        for statement in statements {
            self.visit(statement);
        }
    }

    fn visit(&mut self, node: &Node) {
        match &node.kind {
            NodeKind::Program { statements } => self.visit_statement_list(statements),
            NodeKind::Block { statements, .. } => self.visit_statement_list(statements),
            NodeKind::Package { name, block, .. } => {
                self.flush_current_package();
                self.current_package = name.clone();
                if let Some(block) = block {
                    self.visit(block);
                }
            }
            NodeKind::Use { module, args, .. } => {
                if module == "Exporter"
                    || ((module == "parent" || module == "base")
                        && args.iter().any(|arg| parse_argument_names(arg).iter().any(|i| i == "Exporter")))
                {
                    self.current.uses_exporter = true;
                }
            }
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                if let NodeKind::Variable { sigil, name } = &variable.kind
                    && let Some(initializer) = initializer
                {
                    self.capture_export_assignment(sigil, name, initializer);
                }
            }
            NodeKind::Assignment { lhs, rhs, .. } => {
                if let NodeKind::Variable { sigil, name } = &lhs.kind {
                    self.capture_export_assignment(sigil, name, rhs);
                }
            }
            NodeKind::Subroutine { name, body, .. } => {
                if let Some(sub_name) = name {
                    self.current.subroutines.insert(sub_name.clone(), node.location);
                }
                self.visit(body);
            }
            NodeKind::ExpressionStatement { expression } => self.visit(expression),
            _ => {}
        }
    }

    fn capture_export_assignment(&mut self, sigil: &str, name: &str, rhs: &Node) {
        match (sigil, name) {
            ("@", "ISA") => {
                if let Some(items) = parse_name_list(rhs)
                    && items.iter().any(|item| item == "Exporter")
                {
                    self.current.uses_exporter = true;
                }
            }
            ("@", "EXPORT") => {
                if let Some(items) = parse_name_list(rhs) {
                    self.current.export_names.extend(items);
                }
            }
            ("@", "EXPORT_OK") => {
                if let Some(items) = parse_name_list(rhs) {
                    self.current.export_ok_names.extend(items);
                }
            }
            ("%", "EXPORT_TAGS") => {
                if let Some(tags) = parse_export_tags(rhs) {
                    for (tag, names) in tags {
                        self.current.export_tag_names.entry(tag).or_default().extend(names);
                    }
                }
            }
            _ => {}
        }
    }
}

fn parse_export_tags(node: &Node) -> Option<HashMap<String, Vec<String>>> {
    let NodeKind::HashLiteral { pairs } = &node.kind else {
        return None;
    };

    let mut tags = HashMap::new();
    for (key_node, value_node) in pairs {
        let mut key_names = parse_name_list(key_node)?;
        let tag = key_names.pop()?;
        let members = parse_name_list(value_node)?;
        tags.insert(tag, members);
    }
    Some(tags)
}

fn parse_name_list(node: &Node) -> Option<Vec<String>> {
    match &node.kind {
        NodeKind::String { value, .. } => {
            let list = parse_string_value(value);
            if list.is_empty() { None } else { Some(list) }
        }
        NodeKind::Identifier { name } => {
            let list = parse_string_value(name);
            if list.is_empty() { None } else { Some(list) }
        }
        NodeKind::ArrayLiteral { elements } => {
            let mut out = Vec::new();
            for element in elements {
                out.extend(parse_name_list(element)?);
            }
            Some(out)
        }
        _ => None,
    }
}

fn parse_string_value(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();

    if trimmed.starts_with("qw") {
        return parse_qw_list(trimmed);
    }

    normalize_name(trimmed).into_iter().collect()
}

fn parse_qw_list(raw: &str) -> Vec<String> {
    if raw.len() < 4 {
        return Vec::new();
    }

    let mut chars = raw.chars();
    let _q = chars.next();
    let _w = chars.next();
    let open = chars.next().unwrap_or(' ');
    let close = match open {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '<' => '>',
        c => c,
    };

    let Some(start) = raw.find(open) else {
        return Vec::new();
    };
    let Some(end) = raw.rfind(close) else {
        return Vec::new();
    };
    if start >= end {
        return Vec::new();
    }

    raw[start + 1..end]
        .split_whitespace()
        .filter_map(normalize_name)
        .collect()
}

fn parse_argument_names(raw: &str) -> Vec<String> {
    parse_string_value(raw)
}

fn normalize_name(value: &str) -> Option<String> {
    let name = value.trim().trim_matches('"').trim_matches('\'').trim();
    if name.is_empty() { None } else { Some(name.to_string()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    fn parse_export_metadata(source: &str) -> Result<FileExportMetadata, Box<dyn std::error::Error>> {
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;
        Ok(ExportMetadataBuilder::new().build(&ast))
    }

    #[test]
    fn captures_simple_export_array() -> Result<(), Box<dyn std::error::Error>> {
        let metadata = parse_export_metadata(
            "package Demo;\nuse Exporter 'import';\nour @EXPORT = qw(foo bar);\nsub foo {}\nsub bar {}\n1;",
        )?;

        let package = &metadata.packages[0];
        assert_eq!(package.package, "Demo");
        assert_eq!(package.exports.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), vec!["foo", "bar"]);
        Ok(())
    }

    #[test]
    fn captures_export_ok_and_ignores_missing_definitions() -> Result<(), Box<dyn std::error::Error>> {
        let metadata = parse_export_metadata(
            "package Demo;\nuse Exporter 'import';\nour @EXPORT_OK = qw(alpha missing);\nsub alpha {}\n1;",
        )?;

        let package = &metadata.packages[0];
        assert_eq!(package.export_ok.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), vec!["alpha"]);
        Ok(())
    }

    #[test]
    fn captures_export_tags_hash_literal() -> Result<(), Box<dyn std::error::Error>> {
        let metadata = parse_export_metadata(
            "package Demo;\nuse parent 'Exporter';\nour %EXPORT_TAGS = (\n  core => [qw(one two)],\n  extra => ['three'],\n);\nsub one {}\nsub two {}\nsub three {}\n1;",
        )?;

        let package = &metadata.packages[0];
        assert_eq!(
            package.export_tags["core"].iter().map(|item| item.name.as_str()).collect::<Vec<_>>(),
            vec!["one", "two"]
        );
        assert_eq!(
            package.export_tags["extra"].iter().map(|item| item.name.as_str()).collect::<Vec<_>>(),
            vec!["three"]
        );
        Ok(())
    }
}
