//! Visibility resolution for symbols at a given query point.
//!
//! Resolves which symbols are visible at a `(file_id, byte_offset, scope_id)`
//! query point by consulting:
//!
//! 1. The file's own entities (local lexical/package scope)
//! 2. The file's import specs (from [`ImportExportIndex`])
//! 3. The workspace's export sets (from [`ImportExportIndex`])
//!
//! For each import in the file:
//!
//! - `UseExplicitList` with `Explicit(names)` → make those names visible
//!   with source [`VisibleSymbolSource::ExplicitImport`]
//! - `UseEmpty` → suppress all defaults (no symbols visible from this module)
//! - `Use` (bare) → make the module's `@EXPORT` (default_exports) visible
//!   with source [`VisibleSymbolSource::DefaultExport`]
//! - `UseTag` with `Tags(tags)` → make all symbols in those tags visible
//!   with source [`VisibleSymbolSource::ExportTag`]

use perl_semantic_facts::{
    EntityKind, FileId, ImportKind, ImportSpec, ImportSymbols, ScopeId, VisibleSymbol,
    VisibleSymbolContext, VisibleSymbolSource,
};

use super::imports::ImportExportIndex;
use crate::workspace::workspace_index::FileFactShard;

/// Compute the list of symbols visible at a given query point.
///
/// Visibility is resolved from multiple sources in priority order:
///
/// 1. **Local lexical scope** — entities whose scope matches `scope_id` and
///    whose byte span encloses `byte_offset`.
/// 2. **Local package scope** — file-level subroutines, constants, and
///    variables visible throughout the file.
/// 3. **Explicit imports** — `use Foo qw(a b)` makes `a` and `b` visible.
/// 4. **Default exports** — bare `use Foo` makes `@EXPORT` symbols visible.
/// 5. **Export tags** — `use Foo ':tag'` makes tag members visible.
/// 6. **Constants** — `use constant` declarations.
/// 7. **Generated members** — framework-synthesized accessors (Moo/Moose).
pub fn visible_symbols_at(
    file_id: FileId,
    byte_offset: u32,
    scope_id: Option<ScopeId>,
    shard: &FileFactShard,
    import_export_index: &ImportExportIndex,
) -> Vec<VisibleSymbol> {
    let mut result: Vec<VisibleSymbol> = Vec::new();

    // ── 1. Local lexical scope ──
    collect_local_lexical(&mut result, shard, byte_offset, scope_id);

    // ── 2. Local package scope ──
    collect_local_package(&mut result, shard);

    // ── 3–5. Import-derived visibility ──
    collect_import_visibility(&mut result, file_id, byte_offset, import_export_index);

    // ── 6. Constants (use constant) ──
    collect_constants(&mut result, shard);

    // ── 7. Generated members ──
    collect_generated_members(&mut result, shard);

    result
}

// ── Private helpers ──

/// Collect entities from the local lexical scope.
///
/// An entity is considered lexically visible when:
/// - It has a matching `scope_id` (if the query provides one), AND
/// - Its anchor span starts at or before `byte_offset`.
///
/// Variables are the primary lexical-scope entities.
fn collect_local_lexical(
    result: &mut Vec<VisibleSymbol>,
    shard: &FileFactShard,
    byte_offset: u32,
    scope_id: Option<ScopeId>,
) {
    for entity in &shard.entities {
        // Only variables are lexically scoped in Perl.
        if entity.kind != EntityKind::Variable {
            continue;
        }

        // If we have a scope_id query, require a matching scope.
        if let Some(query_scope) = scope_id {
            match entity.scope_id {
                Some(entity_scope) if entity_scope == query_scope => {}
                Some(_) => continue,
                // Entities without a scope are file-level; include them.
                None => {}
            }
        }

        // The entity must be declared at or before the query offset.
        let declared_before_offset = entity
            .anchor_id
            .and_then(|aid| shard.anchors.iter().find(|a| a.id == aid))
            .map(|anchor| anchor.span_start_byte <= byte_offset)
            .unwrap_or(false);

        if !declared_before_offset {
            continue;
        }

        result.push(VisibleSymbol {
            name: bare_name(&entity.canonical_name),
            entity_id: Some(entity.id),
            source: VisibleSymbolSource::LocalLexical,
            confidence: entity.confidence,
            context: None,
        });
    }
}

/// Collect file-level subroutines and package-scoped entities.
///
/// Subroutines, methods, and package declarations are visible throughout
/// the file regardless of byte offset.
fn collect_local_package(result: &mut Vec<VisibleSymbol>, shard: &FileFactShard) {
    for entity in &shard.entities {
        match entity.kind {
            EntityKind::Subroutine | EntityKind::Method | EntityKind::Package => {}
            _ => continue,
        }

        // Skip entities that are definition occurrences from other files
        // (they would not be local).

        result.push(VisibleSymbol {
            name: bare_name(&entity.canonical_name),
            entity_id: Some(entity.id),
            source: VisibleSymbolSource::LocalPackage,
            confidence: entity.confidence,
            context: None,
        });
    }
}

/// Collect symbols made visible through import statements.
///
/// Walks the file's import specs and resolves each against the workspace
/// export index:
///
/// - `UseExplicitList` + `Explicit(names)` → ExplicitImport
/// - `UseEmpty` → suppress defaults (skip)
/// - `Use` (bare) → DefaultExport from `@EXPORT`
/// - `UseTag` + `Tags(tags)` → ExportTag from `%EXPORT_TAGS`
/// - `RequireThenImport` + `Explicit(names)` / `Mixed { names, .. }` → ExplicitImport
fn collect_import_visibility(
    result: &mut Vec<VisibleSymbol>,
    file_id: FileId,
    byte_offset: u32,
    import_export_index: &ImportExportIndex,
) {
    let imports = import_export_index.get_imports_for_file(file_id);

    for import in imports {
        if import.span_start_byte.is_some_and(|start| start > byte_offset) {
            continue;
        }

        let context = VisibleSymbolContext::new(
            Some(import.module.clone()),
            import.anchor_id,
            import_export_index.get_exports_for_module(&import.module).and_then(|es| es.anchor_id),
        );

        match import.kind {
            ImportKind::UseExplicitList => {
                // `use Foo qw(a b)` → make explicit names visible.
                collect_explicit_import_names(result, import, &context);
                collect_import_tags(result, import, import_export_index, &context);
            }

            ImportKind::UseEmpty => {
                // `use Foo ()` → suppress all defaults. No symbols visible.
            }

            ImportKind::Use => {
                // Bare `use Foo` → make @EXPORT (default_exports) visible.
                if let Some(export_set) = import_export_index.get_exports_for_module(&import.module)
                {
                    for name in &export_set.default_exports {
                        result.push(VisibleSymbol {
                            name: name.clone(),
                            entity_id: None,
                            source: VisibleSymbolSource::DefaultExport,
                            confidence: export_set.confidence,
                            context: Some(context.clone()),
                        });
                    }
                }
            }

            ImportKind::UseTag => {
                // `use Foo ':tag'` → make tag members visible.
                collect_import_tags(result, import, import_export_index, &context);
            }

            ImportKind::RequireThenImport => {
                // `require Foo; Foo->import(qw(x y))` → explicit names.
                collect_explicit_import_names(result, import, &context);
                collect_import_tags(result, import, import_export_index, &context);
            }

            ImportKind::UseConstant | ImportKind::Require | ImportKind::DynamicRequire => {
                // UseConstant is handled separately in collect_constants.
                // Require without import and DynamicRequire do not make
                // symbols visible in the importing file's namespace.
            }

            ImportKind::ManualImport => {
                // `Class->import(@dynamic)` — the symbol list is not statically
                // known (ImportSymbols::Dynamic), so no specific symbols can be
                // added to visibility here.  The dynamic_callable_may_be_visible_at
                // query handles suppression at diagnostic time.
            }
        }
    }
}

fn collect_explicit_import_names(
    result: &mut Vec<VisibleSymbol>,
    import: &ImportSpec,
    context: &VisibleSymbolContext,
) {
    let names = match &import.symbols {
        ImportSymbols::Explicit(names) => names.as_slice(),
        ImportSymbols::Mixed { names, .. } => names.as_slice(),
        _ => return,
    };

    for name in names {
        result.push(VisibleSymbol {
            name: name.clone(),
            entity_id: None,
            source: VisibleSymbolSource::ExplicitImport,
            confidence: import.confidence,
            context: Some(context.clone()),
        });
    }
}

fn collect_import_tags(
    result: &mut Vec<VisibleSymbol>,
    import: &ImportSpec,
    import_export_index: &ImportExportIndex,
    context: &VisibleSymbolContext,
) {
    let tag_names = match &import.symbols {
        ImportSymbols::Tags(tags) => tags.as_slice(),
        ImportSymbols::Mixed { tags, .. } => tags.as_slice(),
        _ => return,
    };

    let Some(export_set) = import_export_index.get_exports_for_module(&import.module) else {
        return;
    };

    for tag_name in tag_names {
        for tag in &export_set.tags {
            if tag.name != *tag_name {
                continue;
            }
            for member in &tag.members {
                result.push(VisibleSymbol {
                    name: member.clone(),
                    entity_id: None,
                    source: VisibleSymbolSource::ExportTag,
                    confidence: export_set.confidence,
                    context: Some(context.clone()),
                });
            }
        }
    }
}

/// Collect constants declared via `use constant`.
///
/// Constants are entities with kind `Constant` and occurrence kind
/// `Definition` in the shard.
fn collect_constants(result: &mut Vec<VisibleSymbol>, shard: &FileFactShard) {
    for entity in &shard.entities {
        if entity.kind != EntityKind::Constant {
            continue;
        }

        result.push(VisibleSymbol {
            name: bare_name(&entity.canonical_name),
            entity_id: Some(entity.id),
            source: VisibleSymbolSource::Constant,
            confidence: entity.confidence,
            context: None,
        });
    }
}

/// Collect generated members (Moo/Moose accessors, etc.).
fn collect_generated_members(result: &mut Vec<VisibleSymbol>, shard: &FileFactShard) {
    for entity in &shard.entities {
        if entity.kind != EntityKind::GeneratedMember {
            continue;
        }

        result.push(VisibleSymbol {
            name: bare_name(&entity.canonical_name),
            entity_id: Some(entity.id),
            source: VisibleSymbolSource::Generated,
            confidence: entity.confidence,
            context: None,
        });
    }
}

/// Extract the bare name from a potentially qualified name.
///
/// `"Foo::Bar::baz"` → `"baz"`, `"baz"` → `"baz"`.
fn bare_name(qualified: &str) -> String {
    let normalized = qualified.trim_end_matches("::");
    if normalized.is_empty() {
        return qualified.to_string();
    }

    match normalized.rsplit_once("::") {
        Some((_, bare)) => bare.to_string(),
        None => normalized.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::{Parser, hir::lower_ast};
    use perl_semantic_facts::{
        AnchorFact, AnchorId, Confidence, EntityFact, EntityId, EntityKind, ExportSet, ExportTag,
        ImportKind, ImportSpec, ImportSymbols, Provenance, ScopeId,
    };

    /// Build a minimal shard with one subroutine entity.
    fn shard_with_sub(file_id: FileId) -> FileFactShard {
        FileFactShard {
            source_uri: "file:///lib/Main.pm".to_string(),
            file_id,
            content_hash: 1,
            anchors_hash: None,
            entities_hash: None,
            occurrences_hash: None,
            edges_hash: None,
            anchors: vec![AnchorFact {
                id: AnchorId(10),
                file_id,
                span_start_byte: 0,
                span_end_byte: 20,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            entities: vec![EntityFact {
                id: EntityId(100),
                kind: EntityKind::Subroutine,
                canonical_name: "Main::do_stuff".to_string(),
                anchor_id: Some(AnchorId(10)),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            occurrences: vec![],
            edges: vec![],
        }
    }

    /// Build a shard with a lexical variable.
    fn shard_with_variable(file_id: FileId) -> FileFactShard {
        FileFactShard {
            source_uri: "file:///lib/Main.pm".to_string(),
            file_id,
            content_hash: 2,
            anchors_hash: None,
            entities_hash: None,
            occurrences_hash: None,
            edges_hash: None,
            anchors: vec![AnchorFact {
                id: AnchorId(20),
                file_id,
                span_start_byte: 5,
                span_end_byte: 10,
                scope_id: Some(ScopeId(1)),
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            entities: vec![EntityFact {
                id: EntityId(200),
                kind: EntityKind::Variable,
                canonical_name: "$count".to_string(),
                anchor_id: Some(AnchorId(20)),
                scope_id: Some(ScopeId(1)),
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            occurrences: vec![],
            edges: vec![],
        }
    }

    /// Build a shard with a constant entity.
    fn shard_with_constant(file_id: FileId) -> FileFactShard {
        FileFactShard {
            source_uri: "file:///lib/Main.pm".to_string(),
            file_id,
            content_hash: 3,
            anchors_hash: None,
            entities_hash: None,
            occurrences_hash: None,
            edges_hash: None,
            anchors: vec![AnchorFact {
                id: AnchorId(30),
                file_id,
                span_start_byte: 0,
                span_end_byte: 15,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            entities: vec![EntityFact {
                id: EntityId(300),
                kind: EntityKind::Constant,
                canonical_name: "MAX_SIZE".to_string(),
                anchor_id: Some(AnchorId(30)),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            occurrences: vec![],
            edges: vec![],
        }
    }

    /// Build a shard with a generated member entity.
    fn shard_with_generated(file_id: FileId) -> FileFactShard {
        FileFactShard {
            source_uri: "file:///lib/Main.pm".to_string(),
            file_id,
            content_hash: 4,
            anchors_hash: None,
            entities_hash: None,
            occurrences_hash: None,
            edges_hash: None,
            anchors: vec![AnchorFact {
                id: AnchorId(40),
                file_id,
                span_start_byte: 0,
                span_end_byte: 20,
                scope_id: None,
                provenance: Provenance::FrameworkSynthesis,
                confidence: Confidence::Medium,
            }],
            entities: vec![EntityFact {
                id: EntityId(400),
                kind: EntityKind::GeneratedMember,
                canonical_name: "MyClass::name".to_string(),
                anchor_id: Some(AnchorId(40)),
                scope_id: None,
                provenance: Provenance::FrameworkSynthesis,
                confidence: Confidence::Medium,
            }],
            occurrences: vec![],
            edges: vec![],
        }
    }

    fn empty_shard(file_id: FileId) -> FileFactShard {
        FileFactShard {
            source_uri: "file:///lib/Main.pm".to_string(),
            file_id,
            content_hash: 0,
            anchors_hash: None,
            entities_hash: None,
            occurrences_hash: None,
            edges_hash: None,
            anchors: vec![],
            entities: vec![],
            occurrences: vec![],
            edges: vec![],
        }
    }

    fn sample_export_set() -> ExportSet {
        ExportSet {
            default_exports: vec!["foo".to_string(), "bar".to_string()],
            optional_exports: vec!["baz".to_string()],
            tags: vec![ExportTag {
                name: "all".to_string(),
                members: vec!["foo".to_string(), "bar".to_string(), "baz".to_string()],
            }],
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            module_name: Some("Foo".to_string()),
            anchor_id: Some(AnchorId(500)),
        }
    }

    fn lower_hir_source(source: &str) -> perl_parser_core::hir::HirFile {
        let mut parser = Parser::new(source);
        let output = parser.parse_with_recovery();
        lower_ast(&output.ast)
    }

    fn visible_symbol<'a>(
        symbols: &'a [VisibleSymbol],
        name: &str,
        source: VisibleSymbolSource,
    ) -> Result<&'a VisibleSymbol, Box<dyn std::error::Error>> {
        symbols
            .iter()
            .find(|symbol| symbol.name == name && symbol.source == source)
            .ok_or_else(|| format!("expected visible symbol {name} from {source:?}").into())
    }

    fn visible_names(symbols: &[VisibleSymbol]) -> Vec<&str> {
        symbols.iter().map(|symbol| symbol.name.as_str()).collect()
    }

    #[test]
    fn canonical_hir_import_export_facts_drive_ordered_visible_symbols()
    -> Result<(), Box<dyn std::error::Error>> {
        let exporter = lower_hir_source(
            "package HIR::Exporter;\n\
             our @EXPORT = qw(defaulted);\n\
             our @EXPORT_OK = qw(optional);\n\
             our %EXPORT_TAGS = (all => [qw(defaulted optional tagged)]);\n",
        );
        let importer_source = "package HIR::Consumer;\n\
             before_import();\n\
             use HIR::Exporter qw(optional :all);\n\
             optional();\n\
             defaulted();\n\
             tagged();\n";
        let importer = lower_hir_source(importer_source);
        let importer_file_id = FileId(91);
        let shard = empty_shard(importer_file_id);
        let mut index = ImportExportIndex::new();

        let export_sets = exporter.stash_graph.export_sets();
        let export_set = export_sets
            .iter()
            .find(|set| set.module_name.as_deref() == Some("HIR::Exporter"))
            .ok_or("expected HIR::Exporter export set")?
            .clone();
        assert_eq!(export_set.provenance, Provenance::ExactAst);
        assert_eq!(export_set.confidence, Confidence::High);
        assert!(export_set.anchor_id.is_some(), "HIR export facts should preserve source anchor");
        index.add_module_exports("file:///lib/HIR/Exporter.pm", "HIR::Exporter", export_set);

        let import_specs = importer.compile_environment.import_specs(importer_file_id);
        let import = import_specs
            .iter()
            .find(|spec| spec.module == "HIR::Exporter")
            .ok_or("expected HIR::Exporter import spec")?;
        assert_eq!(
            import.symbols,
            ImportSymbols::Mixed {
                tags: vec!["all".to_string()],
                names: vec!["optional".to_string()],
            }
        );
        assert!(import.anchor_id.is_some(), "HIR import facts should preserve source anchor");
        assert!(import.span_start_byte.is_some(), "HIR import facts should preserve order anchor");
        index.add_file_imports("file:///lib/HIR/Consumer.pm", importer_file_id, import_specs);

        let import_start = importer_source
            .find("use HIR::Exporter")
            .ok_or("expected import directive in fixture")? as u32;
        let before_import = visible_symbols_at(
            importer_file_id,
            import_start.saturating_sub(1),
            None,
            &shard,
            &index,
        );
        let before_names = visible_names(&before_import);
        assert!(
            !before_names.contains(&"optional")
                && !before_names.contains(&"defaulted")
                && !before_names.contains(&"tagged"),
            "imports must not be visible before the HIR import directive: {before_names:?}"
        );

        let after_import =
            importer_source.find("tagged();").ok_or("expected post-import call in fixture")? as u32;
        let symbols = visible_symbols_at(importer_file_id, after_import, None, &shard, &index);
        let optional = visible_symbol(&symbols, "optional", VisibleSymbolSource::ExplicitImport)?;
        let optional_context = optional.context.as_ref().ok_or("expected optional context")?;
        assert_eq!(optional_context.source_module.as_deref(), Some("HIR::Exporter"));
        assert!(optional_context.source_import_anchor_id.is_some());

        let defaulted = visible_symbol(&symbols, "defaulted", VisibleSymbolSource::ExportTag)?;
        let defaulted_context = defaulted.context.as_ref().ok_or("expected defaulted context")?;
        assert_eq!(defaulted_context.source_module.as_deref(), Some("HIR::Exporter"));
        assert!(defaulted_context.source_export_anchor_id.is_some());

        let tagged = visible_symbol(&symbols, "tagged", VisibleSymbolSource::ExportTag)?;
        let tagged_context = tagged.context.as_ref().ok_or("expected tagged context")?;
        assert_eq!(tagged_context.source_module.as_deref(), Some("HIR::Exporter"));
        assert!(tagged_context.source_import_anchor_id.is_some());

        Ok(())
    }

    #[test]
    fn canonical_hir_empty_import_does_not_invent_default_visibility()
    -> Result<(), Box<dyn std::error::Error>> {
        let exporter = lower_hir_source(
            "package HIR::Empty;\n\
             our @EXPORT = qw(defaulted);\n",
        );
        let importer_source = "package HIR::Consumer;\n\
             use HIR::Empty ();\n\
             defaulted();\n";
        let importer = lower_hir_source(importer_source);
        let importer_file_id = FileId(92);
        let shard = empty_shard(importer_file_id);
        let mut index = ImportExportIndex::new();

        let export_set = exporter
            .stash_graph
            .export_sets()
            .into_iter()
            .find(|set| set.module_name.as_deref() == Some("HIR::Empty"))
            .ok_or("expected HIR::Empty export set")?;
        index.add_module_exports("file:///lib/HIR/Empty.pm", "HIR::Empty", export_set);

        let import_specs = importer.compile_environment.import_specs(importer_file_id);
        let import = import_specs
            .iter()
            .find(|spec| spec.module == "HIR::Empty")
            .ok_or("expected HIR::Empty import spec")?;
        assert_eq!(import.kind, ImportKind::UseEmpty);
        assert_eq!(import.symbols, ImportSymbols::None);
        index.add_file_imports("file:///lib/HIR/Consumer.pm", importer_file_id, import_specs);

        let after_import = importer_source
            .find("defaulted();")
            .ok_or("expected post-import call in fixture")? as u32;
        let symbols = visible_symbols_at(importer_file_id, after_import, None, &shard, &index);
        let names = visible_names(&symbols);
        assert!(!names.contains(&"defaulted"), "empty import should suppress defaults: {names:?}");
        Ok(())
    }

    #[test]
    fn canonical_hir_dynamic_import_and_export_boundaries_fail_closed()
    -> Result<(), Box<dyn std::error::Error>> {
        let exporter = lower_hir_source(
            "package HIR::Dynamic;\n\
             our @EXPORT = @runtime;\n",
        );
        let importer_source = "package HIR::Consumer;\n\
             use HIR::Dynamic @names;\n\
             require $runtime;\n\
             runtime_symbol();\n";
        let importer = lower_hir_source(importer_source);
        let importer_file_id = FileId(93);
        let shard = empty_shard(importer_file_id);
        let mut index = ImportExportIndex::new();

        assert!(
            exporter.stash_graph.export_sets().is_empty(),
            "dynamic export declarations must not produce exact ExportSet facts"
        );
        assert!(
            exporter
                .stash_graph
                .dynamic_boundaries
                .iter()
                .any(|boundary| boundary.reason.contains("non-static")),
            "dynamic export declarations should leave a dynamic-boundary fact"
        );

        let import_specs = importer.compile_environment.import_specs(importer_file_id);
        assert!(
            import_specs.iter().any(|spec| {
                spec.module == "HIR::Dynamic"
                    && spec.symbols == ImportSymbols::Dynamic
                    && spec.provenance == Provenance::DynamicBoundary
            }),
            "dynamic import args should become a dynamic ImportSpec"
        );
        assert!(
            import_specs.iter().any(|spec| spec.kind == ImportKind::DynamicRequire),
            "dynamic require should become a DynamicRequire ImportSpec"
        );
        index.add_file_imports("file:///lib/HIR/Consumer.pm", importer_file_id, import_specs);

        let after_import = importer_source
            .find("runtime_symbol();")
            .ok_or("expected post-import call in fixture")? as u32;
        let symbols = visible_symbols_at(importer_file_id, after_import, None, &shard, &index);
        let names = visible_names(&symbols);
        assert!(
            !names.contains(&"runtime_symbol"),
            "dynamic import/export facts must fail closed instead of inventing visibility: {names:?}"
        );
        Ok(())
    }

    // ── Local package scope ──

    #[test]
    fn local_subroutine_is_visible() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = shard_with_sub(file_id);
        let index = ImportExportIndex::new();

        let symbols = visible_symbols_at(file_id, 50, None, &shard, &index);

        let sub_sym = symbols.iter().find(|s| s.name == "do_stuff");
        assert!(sub_sym.is_some(), "subroutine should be visible");
        let sym = sub_sym.ok_or("expected visible symbol")?;
        assert_eq!(sym.source, VisibleSymbolSource::LocalPackage);
        assert!(sym.context.is_none());
        Ok(())
    }

    // ── Local lexical scope ──

    #[test]
    fn lexical_variable_visible_in_same_scope() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = shard_with_variable(file_id);
        let index = ImportExportIndex::new();

        // Query at offset 15, in scope 1 — variable declared at offset 5.
        let symbols = visible_symbols_at(file_id, 15, Some(ScopeId(1)), &shard, &index);

        let var_sym = symbols.iter().find(|s| s.name == "$count");
        assert!(var_sym.is_some(), "variable should be visible in same scope");
        let sym = var_sym.ok_or("expected visible symbol")?;
        assert_eq!(sym.source, VisibleSymbolSource::LocalLexical);
        Ok(())
    }

    #[test]
    fn lexical_variable_not_visible_in_different_scope() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = shard_with_variable(file_id);
        let index = ImportExportIndex::new();

        // Query in scope 2 — variable is in scope 1.
        let symbols = visible_symbols_at(file_id, 15, Some(ScopeId(2)), &shard, &index);

        let var_sym = symbols.iter().find(|s| s.name == "$count");
        assert!(var_sym.is_none(), "variable should not be visible in different scope");
        Ok(())
    }

    #[test]
    fn lexical_variable_not_visible_before_declaration() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = shard_with_variable(file_id);
        let index = ImportExportIndex::new();

        // Query at offset 2, before the variable declared at offset 5.
        let symbols = visible_symbols_at(file_id, 2, Some(ScopeId(1)), &shard, &index);

        let var_sym = symbols.iter().find(|s| s.name == "$count");
        assert!(var_sym.is_none(), "variable should not be visible before declaration");
        Ok(())
    }

    // ── Explicit imports ──

    #[test]
    fn use_explicit_list_makes_names_visible() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = empty_shard(file_id);
        let mut index = ImportExportIndex::new();

        let import = ImportSpec {
            module: "Foo".to_string(),
            kind: ImportKind::UseExplicitList,
            symbols: ImportSymbols::Explicit(vec!["alpha".to_string(), "beta".to_string()]),
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            file_id: Some(file_id),
            anchor_id: Some(AnchorId(100)),
            scope_id: None,
            span_start_byte: None,
        };
        index.add_file_imports("file:///lib/Main.pm", file_id, vec![import]);

        let symbols = visible_symbols_at(file_id, 0, None, &shard, &index);

        let alpha = symbols.iter().find(|s| s.name == "alpha");
        let beta = symbols.iter().find(|s| s.name == "beta");
        assert!(alpha.is_some(), "alpha should be visible");
        assert!(beta.is_some(), "beta should be visible");

        let a = alpha.ok_or("expected alpha")?;
        assert_eq!(a.source, VisibleSymbolSource::ExplicitImport);
        let ctx = a.context.as_ref().ok_or("expected context")?;
        assert_eq!(ctx.source_module.as_deref(), Some("Foo"));
        Ok(())
    }

    // ── Empty import suppresses defaults ──

    #[test]
    fn use_empty_suppresses_defaults() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = empty_shard(file_id);
        let mut index = ImportExportIndex::new();

        // Register exports for Foo.
        index.add_module_exports("file:///lib/Foo.pm", "Foo", sample_export_set());

        // Import with empty list: `use Foo ()`
        let import = ImportSpec {
            module: "Foo".to_string(),
            kind: ImportKind::UseEmpty,
            symbols: ImportSymbols::None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            file_id: Some(file_id),
            anchor_id: Some(AnchorId(101)),
            scope_id: None,
            span_start_byte: None,
        };
        index.add_file_imports("file:///lib/Main.pm", file_id, vec![import]);

        let symbols = visible_symbols_at(file_id, 0, None, &shard, &index);

        // No symbols from Foo should be visible.
        let from_foo: Vec<_> = symbols
            .iter()
            .filter(|s| {
                s.context
                    .as_ref()
                    .and_then(|c| c.source_module.as_deref())
                    .map(|m| m == "Foo")
                    .unwrap_or(false)
            })
            .collect();
        assert!(from_foo.is_empty(), "use Foo () should suppress all defaults");
        Ok(())
    }

    // ── Bare use → default exports ──

    #[test]
    fn bare_use_makes_default_exports_visible() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = empty_shard(file_id);
        let mut index = ImportExportIndex::new();

        index.add_module_exports("file:///lib/Foo.pm", "Foo", sample_export_set());

        let import = ImportSpec {
            module: "Foo".to_string(),
            kind: ImportKind::Use,
            symbols: ImportSymbols::Default,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            file_id: Some(file_id),
            anchor_id: Some(AnchorId(102)),
            scope_id: None,
            span_start_byte: None,
        };
        index.add_file_imports("file:///lib/Main.pm", file_id, vec![import]);

        let symbols = visible_symbols_at(file_id, 0, None, &shard, &index);

        let foo_sym = symbols.iter().find(|s| s.name == "foo");
        let bar_sym = symbols.iter().find(|s| s.name == "bar");
        assert!(foo_sym.is_some(), "foo should be visible via default export");
        assert!(bar_sym.is_some(), "bar should be visible via default export");

        let f = foo_sym.ok_or("expected foo")?;
        assert_eq!(f.source, VisibleSymbolSource::DefaultExport);
        let ctx = f.context.as_ref().ok_or("expected context")?;
        assert_eq!(ctx.source_module.as_deref(), Some("Foo"));
        assert_eq!(ctx.source_export_anchor_id, Some(AnchorId(500)));

        // baz is in optional_exports only, not default — should NOT be visible.
        let baz_sym = symbols.iter().find(|s| s.name == "baz");
        assert!(baz_sym.is_none(), "baz should not be visible via bare use");
        Ok(())
    }

    // ── Tag import ──

    #[test]
    fn use_tag_makes_tag_members_visible() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = empty_shard(file_id);
        let mut index = ImportExportIndex::new();

        index.add_module_exports("file:///lib/Foo.pm", "Foo", sample_export_set());

        let import = ImportSpec {
            module: "Foo".to_string(),
            kind: ImportKind::UseTag,
            symbols: ImportSymbols::Tags(vec!["all".to_string()]),
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            file_id: Some(file_id),
            anchor_id: Some(AnchorId(103)),
            scope_id: None,
            span_start_byte: None,
        };
        index.add_file_imports("file:///lib/Main.pm", file_id, vec![import]);

        let symbols = visible_symbols_at(file_id, 0, None, &shard, &index);

        let foo_sym = symbols.iter().find(|s| s.name == "foo");
        let bar_sym = symbols.iter().find(|s| s.name == "bar");
        let baz_sym = symbols.iter().find(|s| s.name == "baz");
        assert!(foo_sym.is_some(), "foo should be visible via :all tag");
        assert!(bar_sym.is_some(), "bar should be visible via :all tag");
        assert!(baz_sym.is_some(), "baz should be visible via :all tag");

        let f = foo_sym.ok_or("expected foo")?;
        assert_eq!(f.source, VisibleSymbolSource::ExportTag);
        let ctx = f.context.as_ref().ok_or("expected context")?;
        assert_eq!(ctx.source_module.as_deref(), Some("Foo"));
        Ok(())
    }

    // ── Constants ──

    #[test]
    fn constants_are_visible() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = shard_with_constant(file_id);
        let index = ImportExportIndex::new();

        let symbols = visible_symbols_at(file_id, 50, None, &shard, &index);

        let const_sym = symbols.iter().find(|s| s.name == "MAX_SIZE");
        assert!(const_sym.is_some(), "constant should be visible");
        let sym = const_sym.ok_or("expected constant")?;
        assert_eq!(sym.source, VisibleSymbolSource::Constant);
        Ok(())
    }

    // ── Generated members ──

    #[test]
    fn generated_members_are_visible() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = shard_with_generated(file_id);
        let index = ImportExportIndex::new();

        let symbols = visible_symbols_at(file_id, 50, None, &shard, &index);

        let gen_sym = symbols.iter().find(|s| s.name == "name");
        assert!(gen_sym.is_some(), "generated member should be visible");
        let sym = gen_sym.ok_or("expected generated member")?;
        assert_eq!(sym.source, VisibleSymbolSource::Generated);
        assert_eq!(sym.confidence, Confidence::Medium);
        Ok(())
    }

    // ── RequireThenImport ──

    #[test]
    fn require_then_import_makes_explicit_names_visible() -> Result<(), Box<dyn std::error::Error>>
    {
        let file_id = FileId(1);
        let shard = empty_shard(file_id);
        let mut index = ImportExportIndex::new();

        let import = ImportSpec {
            module: "Bar".to_string(),
            kind: ImportKind::RequireThenImport,
            symbols: ImportSymbols::Explicit(vec!["x".to_string(), "y".to_string()]),
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            file_id: Some(file_id),
            anchor_id: Some(AnchorId(104)),
            scope_id: None,
            span_start_byte: None,
        };
        index.add_file_imports("file:///lib/Main.pm", file_id, vec![import]);

        let symbols = visible_symbols_at(file_id, 0, None, &shard, &index);

        let x_sym = symbols.iter().find(|s| s.name == "x");
        let y_sym = symbols.iter().find(|s| s.name == "y");
        assert!(x_sym.is_some(), "x should be visible via require+import");
        assert!(y_sym.is_some(), "y should be visible via require+import");

        let x = x_sym.ok_or("expected x")?;
        assert_eq!(x.source, VisibleSymbolSource::ExplicitImport);
        Ok(())
    }

    #[test]
    fn require_then_import_mixed_symbols_include_names_and_tags()
    -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = empty_shard(file_id);
        let mut index = ImportExportIndex::new();

        let export_set = ExportSet {
            module_name: Some("Bar".to_string()),
            default_exports: Vec::new(),
            optional_exports: Vec::new(),
            tags: vec![ExportTag { name: "all".to_string(), members: vec!["tagged".to_string()] }],
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            anchor_id: Some(AnchorId(201)),
        };
        index.add_module_exports("file:///lib/Bar.pm", "Bar", export_set);

        let import = ImportSpec {
            module: "Bar".to_string(),
            kind: ImportKind::RequireThenImport,
            symbols: ImportSymbols::Mixed {
                names: vec!["named".to_string()],
                tags: vec!["all".to_string()],
            },
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            file_id: Some(file_id),
            anchor_id: Some(AnchorId(104)),
            scope_id: None,
            span_start_byte: None,
        };
        index.add_file_imports("file:///lib/Main.pm", file_id, vec![import]);

        let symbols = visible_symbols_at(file_id, 0, None, &shard, &index);

        assert!(
            symbols.iter().any(|symbol| symbol.name == "named"
                && symbol.source == VisibleSymbolSource::ExplicitImport),
            "mixed require+import should expose explicit names"
        );
        assert!(
            symbols
                .iter()
                .any(|symbol| symbol.name == "tagged"
                    && symbol.source == VisibleSymbolSource::ExportTag),
            "mixed require+import should expose tag members"
        );
        Ok(())
    }

    #[test]
    fn require_then_import_dynamic_symbols_do_not_invent_visibility()
    -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = empty_shard(file_id);
        let mut index = ImportExportIndex::new();

        let import = ImportSpec {
            module: "Bar".to_string(),
            kind: ImportKind::RequireThenImport,
            symbols: ImportSymbols::Dynamic,
            provenance: Provenance::ExactAst,
            confidence: Confidence::Low,
            file_id: Some(file_id),
            anchor_id: Some(AnchorId(104)),
            scope_id: None,
            span_start_byte: None,
        };
        index.add_file_imports("file:///lib/Main.pm", file_id, vec![import]);

        let symbols = visible_symbols_at(file_id, 0, None, &shard, &index);

        assert!(symbols.is_empty(), "dynamic import list should not invent exact symbols");
        Ok(())
    }

    // ── No imports, no exports ──

    #[test]
    fn bare_use_with_no_exports_produces_nothing() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = empty_shard(file_id);
        let mut index = ImportExportIndex::new();

        // Bare use of a module with no registered exports.
        let import = ImportSpec {
            module: "Unknown".to_string(),
            kind: ImportKind::Use,
            symbols: ImportSymbols::Default,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            file_id: Some(file_id),
            anchor_id: Some(AnchorId(105)),
            scope_id: None,
            span_start_byte: None,
        };
        index.add_file_imports("file:///lib/Main.pm", file_id, vec![import]);

        let symbols = visible_symbols_at(file_id, 0, None, &shard, &index);
        assert!(symbols.is_empty(), "no symbols should be visible from unknown module");
        Ok(())
    }

    // ── Tag import with non-existent tag ──

    #[test]
    fn use_tag_with_nonexistent_tag_produces_nothing() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = empty_shard(file_id);
        let mut index = ImportExportIndex::new();

        index.add_module_exports("file:///lib/Foo.pm", "Foo", sample_export_set());

        let import = ImportSpec {
            module: "Foo".to_string(),
            kind: ImportKind::UseTag,
            symbols: ImportSymbols::Tags(vec!["nonexistent".to_string()]),
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            file_id: Some(file_id),
            anchor_id: Some(AnchorId(106)),
            scope_id: None,
            span_start_byte: None,
        };
        index.add_file_imports("file:///lib/Main.pm", file_id, vec![import]);

        let symbols = visible_symbols_at(file_id, 0, None, &shard, &index);
        assert!(symbols.is_empty(), "nonexistent tag should produce no symbols");
        Ok(())
    }

    // ── Bare name extraction ──

    #[test]
    fn bare_name_extracts_last_component() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(bare_name("Foo::Bar::baz"), "baz");
        assert_eq!(bare_name("baz"), "baz");
        assert_eq!(bare_name("A::B"), "B");
        assert_eq!(bare_name("Foo::"), "Foo");
        assert_eq!(bare_name("Foo::Bar::"), "Bar");
        assert_eq!(bare_name("::"), "::");
        Ok(())
    }

    // ── Combined visibility ──

    #[test]
    fn combined_local_and_import_visibility() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = shard_with_sub(file_id);
        let mut index = ImportExportIndex::new();

        index.add_module_exports("file:///lib/Foo.pm", "Foo", sample_export_set());

        let import = ImportSpec {
            module: "Foo".to_string(),
            kind: ImportKind::UseExplicitList,
            symbols: ImportSymbols::Explicit(vec!["alpha".to_string()]),
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            file_id: Some(file_id),
            anchor_id: Some(AnchorId(107)),
            scope_id: None,
            span_start_byte: None,
        };
        index.add_file_imports("file:///lib/Main.pm", file_id, vec![import]);

        let symbols = visible_symbols_at(file_id, 50, None, &shard, &index);

        // Local sub should be visible.
        assert!(symbols.iter().any(|s| s.name == "do_stuff"));
        // Imported symbol should be visible.
        assert!(symbols.iter().any(|s| s.name == "alpha"));
        Ok(())
    }

    // ── Context metadata ──

    // ── Property-based tests ──

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;
        use proptest::test_runner::Config as ProptestConfig;

        /// Strategy to generate a valid Perl-like symbol name (1–20 lowercase
        /// ASCII chars).
        fn arb_symbol_name() -> impl Strategy<Value = String> {
            "[a-z_][a-z_0-9]{0,19}".prop_filter("non-empty", |s| !s.is_empty())
        }

        /// Strategy to generate a Perl-like module name (e.g. "Foo", "Bar").
        fn arb_module_name() -> impl Strategy<Value = String> {
            "[A-Z][a-z]{1,9}".prop_filter("non-empty", |s| !s.is_empty())
        }

        /// Strategy to generate a non-empty set of distinct symbol names.
        fn arb_symbol_set(min: usize, max: usize) -> impl Strategy<Value = Vec<String>> {
            prop::collection::hash_set(arb_symbol_name(), min..=max)
                .prop_map(|s| s.into_iter().collect::<Vec<_>>())
        }

        /// Strategy to generate a tag name (e.g. "all", "utils").
        fn arb_tag_name() -> impl Strategy<Value = String> {
            "[a-z]{2,8}"
        }

        // **Validates: Requirements 12.1**
        //
        // Property 9 (explicit import): `use Foo qw(a b)` makes a,b visible
        // with source ExplicitImport.
        proptest! {
            #![proptest_config(ProptestConfig {
                failure_persistence: None,
                ..ProptestConfig::default()
            })]

            #[test]
            fn prop_explicit_import_source_attribution(
                module_name in arb_module_name(),
                symbols in arb_symbol_set(1, 8),
            ) {
                let file_id = FileId(1);
                let shard = empty_shard(file_id);
                let mut index = ImportExportIndex::new();

                let import = ImportSpec {
                    module: module_name.clone(),
                    kind: ImportKind::UseExplicitList,
                    symbols: ImportSymbols::Explicit(symbols.clone()),
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                    file_id: Some(file_id),
                    anchor_id: Some(AnchorId(100)),
                    scope_id: None,
                                span_start_byte: None,
                };
                index.add_file_imports("file:///lib/Main.pm", file_id, vec![import]);

                let visible = visible_symbols_at(file_id, 0, None, &shard, &index);

                // Every explicitly imported symbol must appear with ExplicitImport source.
                for sym_name in &symbols {
                    if let Some(vs) = visible.iter().find(|v| v.name == *sym_name) {
                        prop_assert_eq!(
                            &vs.source,
                            &VisibleSymbolSource::ExplicitImport,
                            "explicit import '{}' should have ExplicitImport source",
                            sym_name
                        );
                        // Context should carry the source module.
                        if let Some(ctx) = vs.context.as_ref() {
                            prop_assert_eq!(
                                ctx.source_module.as_deref(),
                                Some(module_name.as_str()),
                                "context source_module should match import module"
                            );
                        } else {
                            prop_assert!(false, "explicit import should have context");
                        }
                    } else {
                        prop_assert!(
                            false,
                            "explicit import '{}' should be visible",
                            sym_name
                        );
                    }
                }
            }
        }

        // **Validates: Requirements 12.3**
        //
        // Property 9 (default export): Bare `use Foo` makes @EXPORT symbols
        // visible with source DefaultExport.
        proptest! {
            #![proptest_config(ProptestConfig {
                failure_persistence: None,
                ..ProptestConfig::default()
            })]

            #[test]
            fn prop_default_export_source_attribution(
                module_name in arb_module_name(),
                default_exports in arb_symbol_set(1, 8),
                optional_exports in arb_symbol_set(0, 4),
            ) {
                let file_id = FileId(1);
                let shard = empty_shard(file_id);
                let mut index = ImportExportIndex::new();

                let export_set = ExportSet {
                    default_exports: default_exports.clone(),
                    optional_exports: optional_exports.clone(),
                    tags: vec![],
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                    module_name: Some(module_name.clone()),
                    anchor_id: Some(AnchorId(500)),
                };
                index.add_module_exports(
                    &format!("file:///lib/{module_name}.pm"),
                    &module_name,
                    export_set,
                );

                let import = ImportSpec {
                    module: module_name.clone(),
                    kind: ImportKind::Use,
                    symbols: ImportSymbols::Default,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                    file_id: Some(file_id),
                    anchor_id: Some(AnchorId(100)),
                    scope_id: None,
                                span_start_byte: None,
                };
                index.add_file_imports("file:///lib/Main.pm", file_id, vec![import]);

                let visible = visible_symbols_at(file_id, 0, None, &shard, &index);

                // Every default export must appear with DefaultExport source.
                for sym_name in &default_exports {
                    if let Some(vs) = visible.iter().find(|v| v.name == *sym_name) {
                        prop_assert_eq!(
                            &vs.source,
                            &VisibleSymbolSource::DefaultExport,
                            "default export '{}' should have DefaultExport source",
                            sym_name
                        );
                        if let Some(ctx) = vs.context.as_ref() {
                            prop_assert_eq!(
                                ctx.source_module.as_deref(),
                                Some(module_name.as_str()),
                                "context source_module should match export module"
                            );
                        } else {
                            prop_assert!(false, "default export should have context");
                        }
                    } else {
                        prop_assert!(
                            false,
                            "default export '{}' should be visible",
                            sym_name
                        );
                    }
                }

                // Optional exports that are NOT in default_exports should NOT
                // be visible via bare `use`.
                for sym_name in &optional_exports {
                    if !default_exports.contains(sym_name) {
                        let found = visible.iter().find(|v| v.name == *sym_name);
                        prop_assert!(
                            found.is_none(),
                            "optional-only export '{}' should NOT be visible via bare use",
                            sym_name
                        );
                    }
                }
            }
        }

        // **Validates: Requirements 12.4**
        //
        // Property 9 (tag export): `use Foo ':tag'` makes all symbols in the
        // named tag visible with source ExportTag.
        proptest! {
            #![proptest_config(ProptestConfig {
                failure_persistence: None,
                ..ProptestConfig::default()
            })]

            #[test]
            fn prop_tag_export_source_attribution(
                module_name in arb_module_name(),
                tag_name in arb_tag_name(),
                tag_members in arb_symbol_set(1, 8),
            ) {
                let file_id = FileId(1);
                let shard = empty_shard(file_id);
                let mut index = ImportExportIndex::new();

                let export_set = ExportSet {
                    default_exports: vec![],
                    optional_exports: vec![],
                    tags: vec![ExportTag {
                        name: tag_name.clone(),
                        members: tag_members.clone(),
                    }],
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                    module_name: Some(module_name.clone()),
                    anchor_id: Some(AnchorId(500)),
                };
                index.add_module_exports(
                    &format!("file:///lib/{module_name}.pm"),
                    &module_name,
                    export_set,
                );

                let import = ImportSpec {
                    module: module_name.clone(),
                    kind: ImportKind::UseTag,
                    symbols: ImportSymbols::Tags(vec![tag_name.clone()]),
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                    file_id: Some(file_id),
                    anchor_id: Some(AnchorId(100)),
                    scope_id: None,
                                span_start_byte: None,
                };
                index.add_file_imports("file:///lib/Main.pm", file_id, vec![import]);

                let visible = visible_symbols_at(file_id, 0, None, &shard, &index);

                // Every tag member must appear with ExportTag source.
                for sym_name in &tag_members {
                    if let Some(vs) = visible.iter().find(|v| v.name == *sym_name) {
                        prop_assert_eq!(
                            &vs.source,
                            &VisibleSymbolSource::ExportTag,
                            "tag member '{}' should have ExportTag source",
                            sym_name
                        );
                        if let Some(ctx) = vs.context.as_ref() {
                            prop_assert_eq!(
                                ctx.source_module.as_deref(),
                                Some(module_name.as_str()),
                                "context source_module should match export module"
                            );
                        } else {
                            prop_assert!(false, "tag export should have context");
                        }
                    } else {
                        prop_assert!(
                            false,
                            "tag member '{}' should be visible via :{} tag",
                            sym_name,
                            tag_name
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn visible_symbol_context_carries_export_anchor() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = empty_shard(file_id);
        let mut index = ImportExportIndex::new();

        index.add_module_exports("file:///lib/Foo.pm", "Foo", sample_export_set());

        let import = ImportSpec {
            module: "Foo".to_string(),
            kind: ImportKind::Use,
            symbols: ImportSymbols::Default,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            file_id: Some(file_id),
            anchor_id: Some(AnchorId(108)),
            scope_id: None,
            span_start_byte: None,
        };
        index.add_file_imports("file:///lib/Main.pm", file_id, vec![import]);

        let symbols = visible_symbols_at(file_id, 0, None, &shard, &index);
        let foo_sym = symbols.iter().find(|s| s.name == "foo").ok_or("expected foo")?;

        let ctx = foo_sym.context.as_ref().ok_or("expected context")?;
        assert_eq!(ctx.source_module.as_deref(), Some("Foo"));
        assert_eq!(ctx.source_import_anchor_id, Some(AnchorId(108)));
        assert_eq!(ctx.source_export_anchor_id, Some(AnchorId(500)));
        Ok(())
    }
}
