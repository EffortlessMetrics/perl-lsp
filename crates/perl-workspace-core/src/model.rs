//! The project model: the assembled set of facts for a workspace.

use serde::{Deserialize, Serialize};

use crate::boundary::DynamicBoundary;
use crate::dist::DistMetadataFacts;
use crate::effects::CompileEffectFacts;
pub use crate::error::ModelLimitation;
use crate::export::ExportFact;
use crate::fact_classes::FactClasses;
use crate::file::FileRecord;
use crate::id::FileId;
use crate::import::ImportFact;
use crate::package::PackageRecord;
use crate::pod::PodFact;
use crate::relation::RelationFact;
use crate::symbol::SymbolRecord;
use crate::test::TestFact;

/// The deterministic set of facts derived from a workspace.
///
/// Facts are stored as flat, deterministically-ordered vectors. Consumers query
/// by [`FileId`] via the helper methods; the ordering is stable across builds
/// of identical input, so serialized models diff cleanly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectModel {
    /// The repo-relative root that was scanned (forward-slash).
    pub root: String,
    /// Which fact classes were requested.
    pub requested: FactClasses,
    /// File facts, ordered by relative path.
    pub files: Vec<FileRecord>,
    /// Package facts, ordered by (file, declaration byte).
    pub packages: Vec<PackageRecord>,
    /// Symbol facts, ordered by (file, declaration byte).
    pub symbols: Vec<SymbolRecord>,
    /// Import facts (`use`/`no`/`require`), ordered by (file, byte).
    pub imports: Vec<ImportFact>,
    /// Export facts (`@EXPORT`/`@EXPORT_OK`), ordered by (file, byte).
    pub exports: Vec<ExportFact>,
    /// Compile-time effect facts, one per parsed file, ordered by file.
    pub compile_effects: Vec<CompileEffectFacts>,
    /// Distribution-metadata facts (from `META.json` / `cpanfile`), ordered by file.
    pub dist_metadata: Vec<DistMetadataFacts>,
    /// Test-file facts (framework + assertion counts), ordered by file.
    pub tests: Vec<TestFact>,
    /// POD documentation facts, ordered by file.
    pub pod: Vec<PodFact>,
    /// Relation edges (inherits/uses/tests), ordered by (kind, source, target).
    pub relations: Vec<RelationFact>,
    /// Dynamic boundaries, ordered by (file, byte).
    pub dynamic_boundaries: Vec<DynamicBoundary>,
    /// Things the model could not fully determine.
    pub limitations: Vec<ModelLimitation>,
}

impl ProjectModel {
    /// An empty model for a root, with the given requested classes.
    #[must_use]
    pub fn empty(root: impl Into<String>, requested: FactClasses) -> Self {
        Self {
            root: root.into(),
            requested,
            files: Vec::new(),
            packages: Vec::new(),
            symbols: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            compile_effects: Vec::new(),
            dist_metadata: Vec::new(),
            tests: Vec::new(),
            pod: Vec::new(),
            relations: Vec::new(),
            dynamic_boundaries: Vec::new(),
            limitations: Vec::new(),
        }
    }

    /// The file record for a given id, if present.
    #[must_use]
    pub fn file(&self, file_id: &FileId) -> Option<&FileRecord> {
        self.files.iter().find(|f| &f.file_id == file_id)
    }

    /// The file record for a repo-relative path, if present.
    #[must_use]
    pub fn file_by_path(&self, relative_path: &str) -> Option<&FileRecord> {
        self.files.iter().find(|f| f.relative_path == relative_path)
    }

    /// All packages declared in a file.
    #[must_use]
    pub fn packages_in_file(&self, file_id: &FileId) -> Vec<&PackageRecord> {
        self.packages.iter().filter(|p| &p.file_id == file_id).collect()
    }

    /// All symbols declared in a file.
    #[must_use]
    pub fn symbols_in_file(&self, file_id: &FileId) -> Vec<&SymbolRecord> {
        self.symbols.iter().filter(|s| &s.file_id == file_id).collect()
    }

    /// All dynamic boundaries recorded for a file.
    #[must_use]
    pub fn boundaries_in_file(&self, file_id: &FileId) -> Vec<&DynamicBoundary> {
        self.dynamic_boundaries.iter().filter(|b| &b.file_id == file_id).collect()
    }

    /// All import facts recorded for a file.
    #[must_use]
    pub fn imports_in_file(&self, file_id: &FileId) -> Vec<&ImportFact> {
        self.imports.iter().filter(|i| &i.file_id == file_id).collect()
    }

    /// All export facts recorded for a file.
    #[must_use]
    pub fn exports_in_file(&self, file_id: &FileId) -> Vec<&ExportFact> {
        self.exports.iter().filter(|e| &e.file_id == file_id).collect()
    }

    /// The compile-effect facts for a file, if computed.
    #[must_use]
    pub fn compile_effects_for_file(&self, file_id: &FileId) -> Option<&CompileEffectFacts> {
        self.compile_effects.iter().find(|e| &e.file_id == file_id)
    }

    /// All declared prerequisites across every metadata file in the model.
    #[must_use]
    pub fn all_prereqs(&self) -> Vec<&crate::dist::Prereq> {
        self.dist_metadata.iter().flat_map(|d| d.prereqs.iter()).collect()
    }

    /// Total number of facts across all classes — a quick health signal.
    #[must_use]
    pub fn fact_count(&self) -> usize {
        self.files.len()
            + self.packages.len()
            + self.symbols.len()
            + self.imports.len()
            + self.exports.len()
            + self.compile_effects.len()
            + self.dist_metadata.len()
            + self.tests.len()
            + self.pod.len()
            + self.relations.len()
            + self.dynamic_boundaries.len()
    }

    /// Sort every fact vector into its canonical, build-stable order.
    ///
    /// The builder calls this before returning, so identical source always
    /// yields an identical (and cleanly diffable) model.
    pub fn sort_for_determinism(&mut self) {
        self.files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        self.packages.sort_by(|a, b| {
            (a.file_id.as_str(), a.declaration_range.start_byte)
                .cmp(&(b.file_id.as_str(), b.declaration_range.start_byte))
        });
        self.symbols.sort_by(|a, b| {
            (a.file_id.as_str(), a.declaration_range.start_byte, a.kind.tag()).cmp(&(
                b.file_id.as_str(),
                b.declaration_range.start_byte,
                b.kind.tag(),
            ))
        });
        self.imports.sort_by(|a, b| {
            (a.file_id.as_str(), a.range.start_byte).cmp(&(b.file_id.as_str(), b.range.start_byte))
        });
        self.exports.sort_by(|a, b| {
            (a.file_id.as_str(), a.range.start_byte).cmp(&(b.file_id.as_str(), b.range.start_byte))
        });
        self.compile_effects.sort_by(|a, b| a.file_id.as_str().cmp(b.file_id.as_str()));
        self.dist_metadata.sort_by(|a, b| a.file_id.as_str().cmp(b.file_id.as_str()));
        self.tests.sort_by(|a, b| a.file_id.as_str().cmp(b.file_id.as_str()));
        self.pod.sort_by(|a, b| a.file_id.as_str().cmp(b.file_id.as_str()));
        self.relations.sort_by(|a, b| {
            (a.kind.tag(), &a.source, &a.target).cmp(&(b.kind.tag(), &b.source, &b.target))
        });
        self.dynamic_boundaries.sort_by(|a, b| {
            (a.file_id.as_str(), a.range.start_byte).cmp(&(b.file_id.as_str(), b.range.start_byte))
        });
        self.limitations.sort_by(|a, b| a.id.cmp(&b.id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::Digest;

    #[test]
    fn empty_model_has_no_facts() {
        let model = ProjectModel::empty("lib", FactClasses::all());
        assert_eq!(model.fact_count(), 0);
        assert!(model.file_by_path("lib/App.pm").is_none());
    }

    #[test]
    fn queries_filter_by_file() {
        let mut model = ProjectModel::empty(".", FactClasses::all());
        let fa = FileId::new("lib/A.pm", &Digest::of("a"));
        let fb = FileId::new("lib/B.pm", &Digest::of("b"));
        model.files.push(FileRecord {
            file_id: fa.clone(),
            relative_path: "lib/A.pm".to_string(),
            role: crate::file::FileRole::Lib,
            digest: Digest::of("a"),
            parse_status: crate::file::ParseStatus::Clean,
        });
        model.files.push(FileRecord {
            file_id: fb.clone(),
            relative_path: "lib/B.pm".to_string(),
            role: crate::file::FileRole::Lib,
            digest: Digest::of("b"),
            parse_status: crate::file::ParseStatus::Clean,
        });
        assert!(model.file(&fa).is_some());
        assert_eq!(model.file_by_path("lib/B.pm").unwrap().file_id, fb);
        assert_eq!(model.packages_in_file(&fa).len(), 0);
    }
}
