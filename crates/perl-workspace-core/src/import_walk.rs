//! Walk a parsed AST to extract import facts, dynamic boundaries, and package
//! inheritance.
//!
//! The rich extractors in the tree (ImportExtractor, PackageGraphExtractor)
//! live in crates the substrate may not depend on (perl-semantic-analyzer,
//! perl-workspace), so their logic is ported here against the leaf
//! `perl-parser-core` AST — resolving the tech-debt note in
//! `perl-workspace/src/semantic/workspace_import_extractor.rs`, which says its
//! correct home is a lower leaf crate exactly like this one.

use std::collections::BTreeMap;

use perl_parser_core::{Node, NodeKind};

use crate::boundary::{DynamicBoundary, DynamicBoundaryKind};
use crate::export::{ExportFact, export_kind_for};
use crate::id::FileId;
use crate::import::{
    ImportFact, ImportKind, looks_like_pragma, normalize_import_arg, split_module_version,
    strip_quotes,
};
use crate::provenance::Confidence;
use crate::range::{SourceRange, Utf8LineIndex};

/// The facts produced by one file's import walk.
pub(crate) struct ImportWalkResult {
    pub imports: Vec<ImportFact>,
    pub exports: Vec<ExportFact>,
    pub boundaries: Vec<DynamicBoundary>,
    /// Inheritance parents keyed by package name (from `use parent`/`use base`).
    pub parents_by_package: BTreeMap<String, Vec<String>>,
    /// Perl language version from a bare `use v5.x`, if seen.
    pub perl_version: Option<String>,
}

/// Walk `ast`, collecting import facts, dynamic boundaries, and inheritance.
pub(crate) fn walk_imports(
    ast: &Node,
    file_id: &FileId,
    line_index: &Utf8LineIndex,
) -> ImportWalkResult {
    let mut result = ImportWalkResult {
        imports: Vec::new(),
        exports: Vec::new(),
        boundaries: Vec::new(),
        parents_by_package: BTreeMap::new(),
        perl_version: None,
    };
    walk(ast, file_id, line_index, &mut None, &mut result);
    result
}

fn range_of(node: &Node, line_index: &Utf8LineIndex) -> SourceRange {
    let start = u32::try_from(node.location.start).unwrap_or(u32::MAX);
    let end = u32::try_from(node.location.end).unwrap_or(u32::MAX);
    line_index.source_range(start, end)
}

fn walk(
    node: &Node,
    file_id: &FileId,
    line_index: &Utf8LineIndex,
    current_package: &mut Option<String>,
    result: &mut ImportWalkResult,
) {
    match &node.kind {
        NodeKind::Package { name, block, .. } => {
            if let Some(block) = block {
                // Block-form `package Foo { ... }` scopes to the block; walk it
                // with a local package context and do not fall through (its only
                // child is the block we already handled).
                let mut inner = Some(name.clone());
                walk(block, file_id, line_index, &mut inner, result);
                return;
            }
            // Statement-form `package Foo;` sets context for later siblings.
            *current_package = Some(name.clone());
        }
        NodeKind::Use { module, args, has_filter_risk } => {
            record_use_or_no(
                node,
                file_id,
                line_index,
                current_package.as_deref(),
                ImportKind::Use,
                module,
                args,
                *has_filter_risk,
                result,
            );
        }
        NodeKind::No { module, args, .. } => {
            record_use_or_no(
                node,
                file_id,
                line_index,
                current_package.as_deref(),
                ImportKind::No,
                module,
                args,
                false,
                result,
            );
        }
        NodeKind::FunctionCall { name, args } if name == "require" => {
            record_require(node, file_id, line_index, args.first(), result);
        }
        NodeKind::Eval { block } => {
            // The parser models both `eval { ... }` and `eval "..."` as
            // `Eval { block }`; the block form wraps a `Block`, the string/expr
            // form wraps an expression. Only the latter is a dynamic boundary.
            if !matches!(block.kind, NodeKind::Block { .. }) {
                result.boundaries.push(DynamicBoundary {
                    file_id: file_id.clone(),
                    range: range_of(node, line_index),
                    kind: DynamicBoundaryKind::StringEval,
                    reason: "string `eval` runs code assembled at runtime; its effects are not statically visible".to_string(),
                    confidence: Confidence::Medium,
                });
            }
        }
        NodeKind::Typeglob { name } => {
            result.boundaries.push(DynamicBoundary {
                file_id: file_id.clone(),
                range: range_of(node, line_index),
                kind: DynamicBoundaryKind::TypeglobAssignment,
                reason: format!("typeglob `*{name}` manipulates the symbol table; installed symbols are not statically visible"),
                confidence: Confidence::Medium,
            });
        }
        NodeKind::VariableDeclaration { variable, initializer, .. } => {
            // `our @EXPORT = qw(...)` / `our @EXPORT_OK = (...)` declare the
            // package's Exporter interface.
            if let NodeKind::Variable { sigil, name } = &variable.kind {
                if sigil == "@" {
                    if let Some(kind) = export_kind_for(name) {
                        let mut symbols = Vec::new();
                        if let Some(init) = initializer {
                            collect_string_values(init, &mut symbols);
                        }
                        result.exports.push(ExportFact {
                            file_id: file_id.clone(),
                            package: current_package.clone(),
                            kind,
                            symbols,
                            range: range_of(node, line_index),
                            confidence: Confidence::High,
                        });
                    }
                }
            }
        }
        NodeKind::Block { .. } => {
            // A statement-form `package Foo;` is scoped to its enclosing lexical
            // block, so package context set inside a bare block / sub body must
            // NOT leak to siblings after the block closes. Snapshot and restore.
            let saved = current_package.clone();
            for child in node.children() {
                walk(child, file_id, line_index, current_package, result);
            }
            *current_package = saved;
            return;
        }
        _ => {}
    }

    for child in node.children() {
        walk(child, file_id, line_index, current_package, result);
    }
}

/// Collect the string values in a subtree (e.g. the elements of a `qw(...)` or
/// parenthesized list), quotes/`qw()` normalized away. Used to read the symbol
/// names out of an `@EXPORT` / `@EXPORT_OK` initializer.
fn collect_string_values(node: &Node, out: &mut Vec<String>) {
    if let NodeKind::String { value, .. } = &node.kind {
        let symbol = strip_quotes(value);
        if !symbol.is_empty() {
            out.push(symbol);
        }
    }
    for child in node.children() {
        collect_string_values(child, out);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "internal helper; args map 1:1 to the two call sites"
)]
fn record_use_or_no(
    node: &Node,
    file_id: &FileId,
    line_index: &Utf8LineIndex,
    current_package: Option<&str>,
    kind: ImportKind,
    module: &str,
    args: &[String],
    has_filter_risk: bool,
    result: &mut ImportWalkResult,
) {
    let (mod_name, version) = split_module_version(module);
    // Capture the Perl language version from a bare `use v5.x`.
    if mod_name.is_empty() && result.perl_version.is_none() {
        if let Some(v) = &version {
            result.perl_version = Some(v.clone());
        }
    }
    let imports: Vec<String> = args.iter().flat_map(|a| normalize_import_arg(a)).collect();

    // `use parent`/`use base` establish inheritance for the current package.
    if kind == ImportKind::Use && (mod_name == "parent" || mod_name == "base") {
        if let Some(pkg) = current_package {
            result
                .parents_by_package
                .entry(pkg.to_string())
                .or_default()
                .extend(imports.iter().cloned());
        }
    }

    if has_filter_risk {
        result.boundaries.push(DynamicBoundary {
            file_id: file_id.clone(),
            range: range_of(node, line_index),
            kind: DynamicBoundaryKind::ImportIntoDynamic,
            reason: format!(
                "`use {mod_name}` is a known source filter; it can rewrite subsequent source at compile time"
            ),
            confidence: Confidence::Medium,
        });
    }

    result.imports.push(ImportFact {
        file_id: file_id.clone(),
        kind,
        is_pragma: looks_like_pragma(&mod_name),
        module: mod_name,
        version,
        imports,
        range: range_of(node, line_index),
        confidence: Confidence::High,
    });
}

fn record_require(
    node: &Node,
    file_id: &FileId,
    line_index: &Utf8LineIndex,
    first_arg: Option<&Node>,
    result: &mut ImportWalkResult,
) {
    match first_arg.map(|a| &a.kind) {
        // Static require of a bareword module (`require Foo::Bar`).
        Some(NodeKind::Identifier { name }) => {
            result.imports.push(ImportFact {
                file_id: file_id.clone(),
                kind: ImportKind::Require,
                is_pragma: false,
                module: name.clone(),
                version: None,
                imports: Vec::new(),
                range: range_of(node, line_index),
                confidence: Confidence::High,
            });
        }
        // Static require of a filename (`require "foo.pl"`).
        Some(NodeKind::String { value, .. }) => {
            result.imports.push(ImportFact {
                file_id: file_id.clone(),
                kind: ImportKind::Require,
                is_pragma: false,
                module: strip_quotes(value),
                version: None,
                imports: Vec::new(),
                range: range_of(node, line_index),
                confidence: Confidence::Medium,
            });
        }
        // `require 5.010;` is a compile-time Perl-version assertion, not a module
        // load — it is neither an import nor a dynamic boundary.
        Some(NodeKind::Number { .. }) => {}
        // `require $mod` / `require $class->can(...)` etc — dynamic.
        Some(_) => {
            result.boundaries.push(DynamicBoundary {
                file_id: file_id.clone(),
                range: range_of(node, line_index),
                kind: DynamicBoundaryKind::RuntimeRequire,
                reason:
                    "runtime `require` of a computed expression loads a module not statically known"
                        .to_string(),
                confidence: Confidence::Medium,
            });
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::Digest;
    use perl_parser_core::Parser;

    fn walk(src: &str) -> (ImportWalkResult, FileId) {
        let ast = Parser::new(src).parse().unwrap();
        let idx = Utf8LineIndex::new(src);
        let fid = FileId::new("lib/M.pm", &Digest::of(src));
        let result = walk_imports(&ast, &fid, &idx);
        (result, fid)
    }

    fn boundary_kinds(result: &ImportWalkResult) -> Vec<DynamicBoundaryKind> {
        result.boundaries.iter().map(|b| b.kind).collect()
    }

    #[test]
    fn string_eval_records_string_eval_boundary() {
        let (result, fid) = walk("my $code = '1+1';\neval \"$code\";\n");
        assert_eq!(
            boundary_kinds(&result),
            vec![DynamicBoundaryKind::StringEval],
            "string eval is a single StringEval boundary; got {:?}",
            result.boundaries
        );
        assert_eq!(result.boundaries[0].file_id, fid, "boundary carries the walked file id");
    }

    #[test]
    fn block_eval_is_not_a_boundary() {
        let (result, _) = walk("eval { 1 + 1 };\n");
        assert!(
            result.boundaries.is_empty(),
            "block-form eval is statically visible, not a boundary; got {:?}",
            result.boundaries
        );
    }

    #[test]
    fn typeglob_assignment_records_boundary() {
        let (result, fid) = walk("*alias = \\&target;\n");
        assert_eq!(boundary_kinds(&result), vec![DynamicBoundaryKind::TypeglobAssignment]);
        assert_eq!(result.boundaries[0].file_id, fid);
    }

    #[test]
    fn runtime_require_of_expression_records_boundary() {
        let (result, fid) = walk("my $mod = 'Foo';\nrequire $mod;\n");
        assert_eq!(
            boundary_kinds(&result),
            vec![DynamicBoundaryKind::RuntimeRequire],
            "require of a computed expression is a RuntimeRequire boundary; got {:?}",
            result.boundaries
        );
        assert_eq!(result.boundaries[0].file_id, fid);
    }

    #[test]
    fn static_require_of_bareword_is_import_not_boundary() {
        let (result, _) = walk("require Foo::Bar;\n");
        assert!(
            result.boundaries.is_empty(),
            "a bareword require is a static import, not a boundary; got {:?}",
            result.boundaries
        );
        assert!(
            result.imports.iter().any(|i| i.module == "Foo::Bar" && i.kind == ImportKind::Require),
            "bareword require is recorded as an import; got {:?}",
            result.imports
        );
    }

    #[test]
    fn source_filter_use_records_import_into_dynamic() {
        let (result, fid) = walk("use Filter::Util::Call;\n");
        assert_eq!(
            boundary_kinds(&result),
            vec![DynamicBoundaryKind::ImportIntoDynamic],
            "a known source filter use is an ImportIntoDynamic boundary; got {:?}",
            result.boundaries
        );
        assert_eq!(result.boundaries[0].file_id, fid);
    }

    #[test]
    fn use_parent_records_inheritance_for_current_package() {
        let (result, _) = walk("package Child;\nuse parent -norequire, 'Base';\n");
        assert_eq!(
            result.parents_by_package.get("Child").map(Vec::as_slice),
            Some(["Base".to_string()].as_slice()),
            "use parent attributes Base to Child; got {:?}",
            result.parents_by_package
        );
    }

    #[test]
    fn exporter_array_records_export_symbols() {
        let (result, fid) = walk("package Foo;\nour @EXPORT = qw(alpha beta);\n");
        assert!(!result.exports.is_empty(), "@EXPORT records an export fact");
        let export = &result.exports[0];
        assert_eq!(export.symbols, vec!["alpha".to_string(), "beta".to_string()]);
        assert_eq!(export.file_id, fid, "export fact carries the walked file id");
    }
}
