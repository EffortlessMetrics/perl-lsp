//! Import/export index for cross-file import and export lookups.
//!
//! Maintains two indexes:
//!
//! - `imports_by_file` — keyed by [`FileId`], maps each file to its list of
//!   [`ImportSpec`] entries extracted from `use`/`require` statements.
//! - `exports_by_module` — keyed by module name (`String`), maps each module
//!   to its [`ExportSet`] describing `@EXPORT`, `@EXPORT_OK`, and `%EXPORT_TAGS`.
//!
//! Both indexes support incremental add/remove for file re-indexing:
//! call [`ImportExportIndex::remove_file_imports`] /
//! [`ImportExportIndex::remove_module_exports`] to purge stale entries,
//! then [`ImportExportIndex::add_file_imports`] /
//! [`ImportExportIndex::add_module_exports`] to insert fresh ones.

use perl_semantic_facts::{ExportSet, FileId, ImportSpec, UseLibFact};
use std::collections::HashMap;

/// Cross-file import/export index backed by two `HashMap`s.
///
/// Populated from [`ImportSpec`] and [`ExportSet`] data during workspace
/// indexing. Supports incremental updates: call the `remove_*` methods
/// to purge stale entries, then the `add_*` methods to insert fresh ones.
#[derive(Debug, Default)]
pub struct ImportExportIndex {
    /// File → import specs. Each file's `use`/`require` statements are
    /// collected into a `Vec<ImportSpec>`.
    imports_by_file: HashMap<FileId, Vec<ImportSpec>>,

    /// Module name → export set. Each module's `@EXPORT`, `@EXPORT_OK`,
    /// and `%EXPORT_TAGS` declarations are collected into an [`ExportSet`].
    exports_by_module: HashMap<String, ExportSet>,

    /// Reverse mapping from file URI to `FileId` so that
    /// [`remove_file_imports`](Self::remove_file_imports) can look up the
    /// `FileId` from a URI string.
    file_uri_to_id: HashMap<String, FileId>,

    /// Reverse mapping from module name to the source URI that provided
    /// the export set, enabling removal by URI.
    module_to_source_uri: HashMap<String, String>,

    /// File → `use lib`/`no lib` path facts, in source order.
    use_lib_by_file: HashMap<FileId, Vec<UseLibFact>>,

    /// Reverse mapping from file URI to `FileId` for use-lib entries,
    /// enabling [`remove_file_use_lib`](Self::remove_file_use_lib) by URI.
    use_lib_uri_to_id: HashMap<String, FileId>,
}

impl ImportExportIndex {
    /// Create an empty import/export index.
    pub fn new() -> Self {
        Self::default()
    }

    // ── Import methods ──

    /// Index all import specs for a file.
    ///
    /// The `source_uri` is stored so that [`remove_file_imports`](Self::remove_file_imports)
    /// can locate the correct `FileId` by URI.
    pub fn add_file_imports(
        &mut self,
        source_uri: &str,
        file_id: FileId,
        imports: Vec<ImportSpec>,
    ) {
        self.file_uri_to_id.insert(source_uri.to_string(), file_id);
        self.imports_by_file.insert(file_id, imports);
    }

    /// Remove all import entries that originated from the given file URI.
    ///
    /// This is the "remove" half of incremental re-indexing for imports.
    pub fn remove_file_imports(&mut self, source_uri: &str) {
        let file_id = match self.file_uri_to_id.remove(source_uri) {
            Some(id) => id,
            None => return,
        };
        self.imports_by_file.remove(&file_id);
    }

    /// Look up all import specs for a given file.
    pub fn get_imports_for_file(&self, file_id: FileId) -> &[ImportSpec] {
        self.imports_by_file.get(&file_id).map(Vec::as_slice).unwrap_or_default()
    }

    // ── UseLib methods ──

    /// Index all `use lib`/`no lib` path facts for a file.
    ///
    /// The `source_uri` is stored so that
    /// [`remove_file_use_lib`](Self::remove_file_use_lib) can locate the
    /// correct `FileId` by URI.
    pub fn add_file_use_lib(&mut self, source_uri: &str, file_id: FileId, facts: Vec<UseLibFact>) {
        self.use_lib_uri_to_id.insert(source_uri.to_string(), file_id);
        self.use_lib_by_file.insert(file_id, facts);
    }

    /// Remove all `use lib`/`no lib` entries that originated from the given file URI.
    ///
    /// This is the "remove" half of incremental re-indexing for use-lib facts.
    pub fn remove_file_use_lib(&mut self, source_uri: &str) {
        let file_id = match self.use_lib_uri_to_id.remove(source_uri) {
            Some(id) => id,
            None => return,
        };
        self.use_lib_by_file.remove(&file_id);
    }

    /// Look up all `use lib`/`no lib` path facts for a given file, in source order.
    pub fn get_use_lib_for_file(&self, file_id: FileId) -> &[UseLibFact] {
        self.use_lib_by_file.get(&file_id).map(Vec::as_slice).unwrap_or_default()
    }

    // ── Export methods ──

    /// Index the export set for a module.
    ///
    /// The `source_uri` is stored so that [`remove_module_exports`](Self::remove_module_exports)
    /// can locate the correct module name by URI.
    pub fn add_module_exports(&mut self, source_uri: &str, module_name: &str, exports: ExportSet) {
        self.module_to_source_uri.insert(module_name.to_string(), source_uri.to_string());
        self.exports_by_module.insert(module_name.to_string(), exports);
    }

    /// Remove the export set that originated from the given file URI.
    ///
    /// This is the "remove" half of incremental re-indexing for exports.
    pub fn remove_module_exports(&mut self, source_uri: &str) {
        // Find which module name(s) came from this URI and remove them.
        let modules_to_remove: Vec<String> = self
            .module_to_source_uri
            .iter()
            .filter(|(_, uri)| uri.as_str() == source_uri)
            .map(|(module, _)| module.clone())
            .collect();

        for module in &modules_to_remove {
            self.exports_by_module.remove(module);
            self.module_to_source_uri.remove(module);
        }
    }

    /// Look up the export set for a given module name.
    pub fn get_exports_for_module(&self, module_name: &str) -> Option<&ExportSet> {
        self.exports_by_module.get(module_name)
    }

    // ── Diagnostic / count methods ──

    /// Return the number of files with indexed imports.
    pub fn import_file_count(&self) -> usize {
        self.imports_by_file.len()
    }

    /// Return the number of modules with indexed exports.
    pub fn export_module_count(&self) -> usize {
        self.exports_by_module.len()
    }

    /// Check whether a bare symbol name appears in any module's export set.
    ///
    /// Returns the module name if the symbol is found in `@EXPORT` or
    /// `@EXPORT_OK` of any indexed module.
    pub fn find_exporting_module(&self, symbol_name: &str) -> Option<&str> {
        for (module, exports) in &self.exports_by_module {
            if exports.default_exports.iter().any(|s| s == symbol_name)
                || exports.optional_exports.iter().any(|s| s == symbol_name)
            {
                return Some(module.as_str());
            }
        }
        None
    }

    /// Check whether a bare symbol name is explicitly imported by any file
    /// other than the given `exclude_file_id`.
    ///
    /// Returns `true` if any other file has an `ImportSpec` whose explicit
    /// symbol list contains the given name.
    pub fn is_imported_by_other_file(&self, symbol_name: &str, exclude_file_id: FileId) -> bool {
        for (&file_id, imports) in &self.imports_by_file {
            if file_id == exclude_file_id {
                continue;
            }
            for spec in imports {
                if import_spec_names_symbol(spec, symbol_name) {
                    return true;
                }
            }
        }
        false
    }

    /// Iterate over all `(FileId, &[ImportSpec])` pairs in the index.
    pub fn all_imports(&self) -> impl Iterator<Item = (FileId, &[ImportSpec])> {
        self.imports_by_file.iter().map(|(&fid, specs)| (fid, specs.as_slice()))
    }
}

/// Check whether an `ImportSpec` explicitly names a given symbol.
fn import_spec_names_symbol(spec: &ImportSpec, symbol_name: &str) -> bool {
    match &spec.symbols {
        perl_semantic_facts::ImportSymbols::Explicit(names) => {
            names.iter().any(|n| n == symbol_name)
        }
        perl_semantic_facts::ImportSymbols::Mixed { names, .. } => {
            names.iter().any(|n| n == symbol_name)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_semantic_facts::{
        AnchorId, Confidence, ExportTag, ImportKind, ImportSymbols, Provenance, ScopeId,
    };

    /// Helper: build a sample `ImportSpec` for `use Foo qw(bar baz)`.
    fn sample_import_explicit() -> ImportSpec {
        ImportSpec {
            module: "Foo".to_string(),
            kind: ImportKind::UseExplicitList,
            symbols: ImportSymbols::Explicit(vec!["bar".to_string(), "baz".to_string()]),
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            file_id: Some(FileId(1)),
            anchor_id: Some(AnchorId(10)),
            scope_id: Some(ScopeId(1)),
            span_start_byte: None,
        }
    }

    /// Helper: build a sample `ImportSpec` for `use Bar ()`.
    fn sample_import_empty() -> ImportSpec {
        ImportSpec {
            module: "Bar".to_string(),
            kind: ImportKind::UseEmpty,
            symbols: ImportSymbols::None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            file_id: Some(FileId(1)),
            anchor_id: Some(AnchorId(11)),
            scope_id: None,
            span_start_byte: None,
        }
    }

    /// Helper: build a sample `ExportSet` for module `Foo`.
    fn sample_export_set() -> ExportSet {
        ExportSet {
            default_exports: vec!["bar".to_string()],
            optional_exports: vec!["baz".to_string(), "qux".to_string()],
            tags: vec![ExportTag {
                name: "all".to_string(),
                members: vec!["bar".to_string(), "baz".to_string(), "qux".to_string()],
            }],
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            module_name: Some("Foo".to_string()),
            anchor_id: Some(AnchorId(20)),
        }
    }

    #[test]
    fn add_and_get_file_imports() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ImportExportIndex::new();
        let file_id = FileId(1);
        let imports = vec![sample_import_explicit(), sample_import_empty()];

        index.add_file_imports("file:///lib/Main.pm", file_id, imports);

        let result = index.get_imports_for_file(file_id);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].module, "Foo");
        assert_eq!(result[1].module, "Bar");
        Ok(())
    }

    #[test]
    fn get_imports_for_unknown_file_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
        let index = ImportExportIndex::new();
        let result = index.get_imports_for_file(FileId(999));
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn remove_file_imports_clears_entries() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ImportExportIndex::new();
        let file_id = FileId(1);
        index.add_file_imports("file:///lib/Main.pm", file_id, vec![sample_import_explicit()]);

        assert_eq!(index.import_file_count(), 1);

        index.remove_file_imports("file:///lib/Main.pm");

        assert_eq!(index.import_file_count(), 0);
        assert!(index.get_imports_for_file(file_id).is_empty());
        Ok(())
    }

    #[test]
    fn remove_file_imports_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ImportExportIndex::new();
        let file_id = FileId(1);
        index.add_file_imports("file:///lib/Main.pm", file_id, vec![sample_import_explicit()]);

        index.remove_file_imports("file:///lib/Main.pm");
        // Second remove should be a no-op.
        index.remove_file_imports("file:///lib/Main.pm");

        assert_eq!(index.import_file_count(), 0);
        Ok(())
    }

    #[test]
    fn remove_unknown_file_imports_is_noop() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ImportExportIndex::new();
        index.add_file_imports("file:///lib/Main.pm", FileId(1), vec![sample_import_explicit()]);

        index.remove_file_imports("file:///nonexistent.pm");

        // Original entries should still be present.
        assert_eq!(index.import_file_count(), 1);
        Ok(())
    }

    #[test]
    fn add_and_get_module_exports() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ImportExportIndex::new();
        index.add_module_exports("file:///lib/Foo.pm", "Foo", sample_export_set());

        let result = index.get_exports_for_module("Foo");
        assert!(result.is_some());
        let exports = result.ok_or("expected export set")?;
        assert_eq!(exports.default_exports, vec!["bar"]);
        assert_eq!(exports.optional_exports, vec!["baz", "qux"]);
        assert_eq!(exports.tags.len(), 1);
        assert_eq!(exports.tags[0].name, "all");
        Ok(())
    }

    #[test]
    fn get_exports_for_unknown_module_returns_none() -> Result<(), Box<dyn std::error::Error>> {
        let index = ImportExportIndex::new();
        assert!(index.get_exports_for_module("Unknown").is_none());
        Ok(())
    }

    #[test]
    fn remove_module_exports_clears_entries() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ImportExportIndex::new();
        index.add_module_exports("file:///lib/Foo.pm", "Foo", sample_export_set());

        assert_eq!(index.export_module_count(), 1);

        index.remove_module_exports("file:///lib/Foo.pm");

        assert_eq!(index.export_module_count(), 0);
        assert!(index.get_exports_for_module("Foo").is_none());
        Ok(())
    }

    #[test]
    fn remove_module_exports_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ImportExportIndex::new();
        index.add_module_exports("file:///lib/Foo.pm", "Foo", sample_export_set());

        index.remove_module_exports("file:///lib/Foo.pm");
        // Second remove should be a no-op.
        index.remove_module_exports("file:///lib/Foo.pm");

        assert_eq!(index.export_module_count(), 0);
        Ok(())
    }

    #[test]
    fn remove_unknown_module_exports_is_noop() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ImportExportIndex::new();
        index.add_module_exports("file:///lib/Foo.pm", "Foo", sample_export_set());

        index.remove_module_exports("file:///nonexistent.pm");

        // Original entries should still be present.
        assert_eq!(index.export_module_count(), 1);
        Ok(())
    }

    #[test]
    fn multiple_files_coexist_in_import_index() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ImportExportIndex::new();

        let file_a = FileId(1);
        let file_b = FileId(2);

        index.add_file_imports("file:///lib/A.pm", file_a, vec![sample_import_explicit()]);
        index.add_file_imports("file:///lib/B.pm", file_b, vec![sample_import_empty()]);

        assert_eq!(index.import_file_count(), 2);
        assert_eq!(index.get_imports_for_file(file_a).len(), 1);
        assert_eq!(index.get_imports_for_file(file_b).len(), 1);

        // Remove one file — only its entries should disappear.
        index.remove_file_imports("file:///lib/A.pm");

        assert_eq!(index.import_file_count(), 1);
        assert!(index.get_imports_for_file(file_a).is_empty());
        assert_eq!(index.get_imports_for_file(file_b).len(), 1);
        Ok(())
    }

    #[test]
    fn multiple_modules_coexist_in_export_index() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ImportExportIndex::new();

        let export_foo = sample_export_set();
        let export_bar = ExportSet {
            default_exports: vec!["init".to_string()],
            optional_exports: vec![],
            tags: vec![],
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            module_name: Some("Bar".to_string()),
            anchor_id: None,
        };

        index.add_module_exports("file:///lib/Foo.pm", "Foo", export_foo);
        index.add_module_exports("file:///lib/Bar.pm", "Bar", export_bar);

        assert_eq!(index.export_module_count(), 2);
        assert!(index.get_exports_for_module("Foo").is_some());
        assert!(index.get_exports_for_module("Bar").is_some());

        // Remove one module — only its entries should disappear.
        index.remove_module_exports("file:///lib/Foo.pm");

        assert_eq!(index.export_module_count(), 1);
        assert!(index.get_exports_for_module("Foo").is_none());
        assert!(index.get_exports_for_module("Bar").is_some());
        Ok(())
    }

    #[test]
    fn incremental_reindex_replaces_imports() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ImportExportIndex::new();
        let file_id = FileId(1);

        index.add_file_imports("file:///lib/Main.pm", file_id, vec![sample_import_explicit()]);
        assert_eq!(index.get_imports_for_file(file_id).len(), 1);
        assert_eq!(index.get_imports_for_file(file_id)[0].module, "Foo");

        // Simulate re-indexing: remove old, add updated imports.
        index.remove_file_imports("file:///lib/Main.pm");

        let updated_import = ImportSpec {
            module: "Baz".to_string(),
            kind: ImportKind::Use,
            symbols: ImportSymbols::Default,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            file_id: Some(file_id),
            anchor_id: Some(AnchorId(30)),
            scope_id: None,
            span_start_byte: None,
        };
        index.add_file_imports("file:///lib/Main.pm", file_id, vec![updated_import]);

        let result = index.get_imports_for_file(file_id);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].module, "Baz");
        Ok(())
    }

    #[test]
    fn incremental_reindex_replaces_exports() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ImportExportIndex::new();

        index.add_module_exports("file:///lib/Foo.pm", "Foo", sample_export_set());

        let original = index.get_exports_for_module("Foo");
        assert!(original.is_some());
        let orig = original.ok_or("expected export set")?;
        assert_eq!(orig.default_exports, vec!["bar"]);

        // Simulate re-indexing: remove old, add updated exports.
        index.remove_module_exports("file:///lib/Foo.pm");

        let updated_exports = ExportSet {
            default_exports: vec!["new_func".to_string()],
            optional_exports: vec![],
            tags: vec![],
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            module_name: Some("Foo".to_string()),
            anchor_id: Some(AnchorId(40)),
        };
        index.add_module_exports("file:///lib/Foo.pm", "Foo", updated_exports);

        let result = index.get_exports_for_module("Foo");
        assert!(result.is_some());
        let exports = result.ok_or("expected export set")?;
        assert_eq!(exports.default_exports, vec!["new_func"]);
        Ok(())
    }

    #[test]
    fn import_spec_fields_are_preserved() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ImportExportIndex::new();
        let file_id = FileId(42);

        let import = ImportSpec {
            module: "My::Module".to_string(),
            kind: ImportKind::UseTag,
            symbols: ImportSymbols::Tags(vec!["all".to_string()]),
            provenance: Provenance::ImportExportInference,
            confidence: Confidence::Medium,
            file_id: Some(file_id),
            anchor_id: Some(AnchorId(99)),
            scope_id: Some(ScopeId(5)),
            span_start_byte: None,
        };

        index.add_file_imports("file:///lib/My/Module.pm", file_id, vec![import]);

        let result = index.get_imports_for_file(file_id);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].module, "My::Module");
        assert_eq!(result[0].kind, ImportKind::UseTag);
        assert_eq!(result[0].symbols, ImportSymbols::Tags(vec!["all".to_string()]));
        assert_eq!(result[0].provenance, Provenance::ImportExportInference);
        assert_eq!(result[0].confidence, Confidence::Medium);
        assert_eq!(result[0].file_id, Some(file_id));
        assert_eq!(result[0].anchor_id, Some(AnchorId(99)));
        assert_eq!(result[0].scope_id, Some(ScopeId(5)));
        Ok(())
    }
}
