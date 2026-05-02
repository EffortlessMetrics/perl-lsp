//! Semantic query facade for workspace-level semantic lookups.
//!
//! Defines the [`SemanticQueries`] trait — the single entry point that all LSP
//! providers use for semantic lookups — and [`WorkspaceSemanticQueries`], the
//! concrete implementation that delegates to the underlying semantic indexes.
//!
//! # Design
//!
//! Providers call trait methods on `SemanticQueries` without knowing the
//! internal index structure. The workspace owns a `WorkspaceSemanticQueries`
//! instance that holds references to:
//!
//! - [`ReferenceIndex`] — typed reference lookups by name or entity.
//! - [`ImportExportIndex`] — import/export resolution for visibility.
//! - Per-file [`FileFactShard`] data for anchor/entity lookups.
//!
//! # Requirements
//!
//! - **Req 8.1**: `symbol_at` returns entity + occurrence at a file position.
//! - **Req 8.2**: `definitions` returns ranked `DefinitionCandidate` lists.
//! - **Req 8.3**: `references` returns typed `OccurrenceFact` lists.
//! - **Req 8.4**: `visible_symbols_at` returns `VisibleSymbol` lists.
//! - **Req 8.5**: `method_candidates` returns method candidates.
//! - **Req 8.6**: `rename_plan` returns a conservative rename plan.
//! - **Req 8.7**: `safe_delete_plan` returns a conservative safe-delete plan.
//! - **Req 5.4**: Definitions sorted by `DefinitionRank`.
//! - **Req 5.5**: Same-rank candidates sorted deterministically by URI + position.
//! - **Req 5.6**: Empty list (not error) when no candidates found.

use perl_semantic_facts::{
    AnchorId, DefinitionCandidate, DefinitionRank, DefinitionRankReason, EntityFact, EntityId,
    EntityKind, FileId, OccurrenceFact, OccurrenceKind, PlanBlocker, PlanBlockerReason,
    PlanWarning, PlannedEdit, PlannedEditCategory, RenamePlan, SafeDeletePlan, ScopeId, ValueShape,
    VisibleSymbol,
};

use super::imports::ImportExportIndex;
use super::package_graph::PackageGraphIndex;
use super::references::ReferenceIndex;
use super::value_shape::ValueShapeIndex;
use super::visibility;
use crate::workspace::workspace_index::FileFactShard;

// ── QueryContext ──

/// Context for definition queries: file, scope, and byte offset.
///
/// Passed to [`SemanticQueries::definitions`] so the facade can rank
/// candidates relative to the query point (same-package, explicit-import,
/// etc.).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryContext {
    /// File containing the query point.
    pub file_id: FileId,
    /// Scope enclosing the query point, when known.
    pub scope_id: Option<ScopeId>,
    /// Byte offset of the query point within the file, when known.
    pub byte_offset: Option<u32>,
}

impl QueryContext {
    /// Create a new `QueryContext`.
    pub fn new(file_id: FileId, scope_id: Option<ScopeId>, byte_offset: Option<u32>) -> Self {
        Self { file_id, scope_id, byte_offset }
    }
}

// ── SemanticQueries trait ──

/// Workspace-level semantic query facade.
///
/// All LSP providers consume this trait rather than accessing the internal
/// index structures directly. This decouples providers from the indexing
/// implementation and enables shadow-compare testing.
pub trait SemanticQueries {
    /// Return the entity and occurrence at a given file position.
    ///
    /// Looks up the anchor that encloses `byte_offset`, then returns the
    /// associated entity and occurrence facts.
    fn symbol_at(&self, file_id: FileId, byte_offset: u32) -> Option<(EntityFact, OccurrenceFact)>;

    /// Return ranked definition candidates for a symbol.
    ///
    /// Candidates are sorted by [`DefinitionRank`] (best first), then
    /// deterministically by file URI and source position within the same
    /// rank. Returns an empty list when no candidates are found.
    fn definitions(&self, symbol: &str, context: &QueryContext) -> Vec<DefinitionCandidate>;

    /// Return typed occurrence references for an entity.
    ///
    /// Returns all non-definition occurrences that reference the given
    /// entity, preserving occurrence kind classification.
    fn references(&self, entity_id: EntityId) -> Vec<OccurrenceFact>;

    /// Return symbols visible at a given file position and scope.
    fn visible_symbols_at(
        &self,
        file_id: FileId,
        byte_offset: u32,
        scope_id: Option<ScopeId>,
    ) -> Vec<VisibleSymbol>;

    /// Return method candidates for a receiver type and method name.
    ///
    /// Stubbed to return empty results until package graph and value-shape
    /// indexes are implemented.
    fn method_candidates(
        &self,
        receiver_package: &str,
        method_name: &str,
    ) -> Vec<DefinitionCandidate>;

    /// Return a conservative rename plan.
    ///
    /// Returns a [`RenamePlan`] with affected occurrences classified by
    /// category (Definition, Reference, ImportList, ExportList) and any
    /// blockers (dynamic boundaries, cross-module exports, generated members).
    fn rename_plan(&self, entity_id: EntityId, new_name: &str) -> RenamePlan;

    /// Return a conservative safe-delete plan.
    ///
    /// Returns a [`SafeDeletePlan`] with blockers when the symbol has
    /// remaining references, is exported, is imported by another file,
    /// or is a generated member without a generator-specific delete plan.
    fn safe_delete_plan(&self, entity_id: EntityId) -> SafeDeletePlan;
}

// ── WorkspaceSemanticQueries ──

/// Concrete implementation of [`SemanticQueries`] backed by workspace indexes.
///
/// Holds references to the semantic indexes and per-file fact shards,
/// delegating each query method to the appropriate index.
pub struct WorkspaceSemanticQueries<'a> {
    /// Typed reference index for cross-file reference lookups.
    reference_index: &'a ReferenceIndex,
    /// Import/export index for visibility resolution.
    import_export_index: &'a ImportExportIndex,
    /// Per-file fact shards keyed by normalized URI.
    fact_shards: &'a std::collections::HashMap<String, FileFactShard>,
    /// Package graph index for inheritance and role-composition traversal.
    package_graph: Option<&'a PackageGraphIndex>,
    /// Value-shape index for receiver-shape filtering in method candidates.
    value_shape_index: Option<&'a ValueShapeIndex>,
}

impl<'a> WorkspaceSemanticQueries<'a> {
    /// Create a new `WorkspaceSemanticQueries` facade.
    pub fn new(
        reference_index: &'a ReferenceIndex,
        import_export_index: &'a ImportExportIndex,
        fact_shards: &'a std::collections::HashMap<String, FileFactShard>,
    ) -> Self {
        Self {
            reference_index,
            import_export_index,
            fact_shards,
            package_graph: None,
            value_shape_index: None,
        }
    }

    /// Create a new `WorkspaceSemanticQueries` facade with a package graph.
    pub fn with_package_graph(
        reference_index: &'a ReferenceIndex,
        import_export_index: &'a ImportExportIndex,
        fact_shards: &'a std::collections::HashMap<String, FileFactShard>,
        package_graph: &'a PackageGraphIndex,
    ) -> Self {
        Self {
            reference_index,
            import_export_index,
            fact_shards,
            package_graph: Some(package_graph),
            value_shape_index: None,
        }
    }

    /// Create a new `WorkspaceSemanticQueries` facade with a package graph
    /// and a value-shape index for receiver-shape filtering.
    pub fn with_package_graph_and_shapes(
        reference_index: &'a ReferenceIndex,
        import_export_index: &'a ImportExportIndex,
        fact_shards: &'a std::collections::HashMap<String, FileFactShard>,
        package_graph: &'a PackageGraphIndex,
        value_shape_index: &'a ValueShapeIndex,
    ) -> Self {
        Self {
            reference_index,
            import_export_index,
            fact_shards,
            package_graph: Some(package_graph),
            value_shape_index: Some(value_shape_index),
        }
    }

    /// Find the [`FileFactShard`] for a given `FileId`.
    fn shard_for_file(&self, file_id: FileId) -> Option<&FileFactShard> {
        self.fact_shards.values().find(|s| s.file_id == file_id)
    }

    /// Sort definition candidates by rank, then deterministically by URI and
    /// source position within the same rank.
    fn sort_candidates(&self, candidates: &mut [DefinitionCandidate]) {
        candidates.sort_by(|a, b| {
            // Primary: sort by DefinitionRank (ExactQualified < ... < Heuristic).
            a.rank.cmp(&b.rank).then_with(|| {
                // Secondary: deterministic tie-break by anchor location.
                // Look up the anchor's file URI and byte offset for each candidate.
                let a_loc = self.anchor_location(a.anchor_id);
                let b_loc = self.anchor_location(b.anchor_id);
                a_loc.cmp(&b_loc)
            })
        });
    }

    /// Return `(source_uri, span_start_byte)` for an anchor, used for
    /// deterministic sorting. Returns a fallback tuple when the anchor
    /// cannot be found.
    fn anchor_location(&self, anchor_id: AnchorId) -> (String, u32) {
        for shard in self.fact_shards.values() {
            if let Some(anchor) = shard.anchors.iter().find(|a| a.id == anchor_id) {
                return (shard.source_uri.clone(), anchor.span_start_byte);
            }
        }
        // Fallback: unknown anchor sorts last.
        (String::new(), u32::MAX)
    }
}

impl<'a> SemanticQueries for WorkspaceSemanticQueries<'a> {
    fn symbol_at(&self, file_id: FileId, byte_offset: u32) -> Option<(EntityFact, OccurrenceFact)> {
        let shard = self.shard_for_file(file_id)?;

        // Find the anchor that encloses the byte offset.
        let anchor = shard.anchors.iter().find(|a| {
            a.file_id == file_id
                && a.span_start_byte <= byte_offset
                && byte_offset < a.span_end_byte
        })?;

        // Find an occurrence at this anchor.
        let occurrence = shard.occurrences.iter().find(|o| o.anchor_id == anchor.id)?;

        // Resolve the entity from the occurrence's entity_id.
        let entity_id = occurrence.entity_id?;
        let entity = shard.entities.iter().find(|e| e.id == entity_id)?;

        Some((entity.clone(), occurrence.clone()))
    }

    fn definitions(&self, symbol: &str, _context: &QueryContext) -> Vec<DefinitionCandidate> {
        let mut candidates = Vec::new();

        // Search all shards for entities whose canonical name matches the
        // symbol (qualified or bare name match).
        for shard in self.fact_shards.values() {
            for entity in &shard.entities {
                let matches =
                    entity.canonical_name == symbol || bare_name(&entity.canonical_name) == symbol;

                if !matches {
                    continue;
                }

                // Only definition-like entities produce candidates.
                if !is_definition_kind(entity.kind) {
                    continue;
                }

                let anchor_id = match entity.anchor_id {
                    Some(id) => id,
                    None => continue,
                };

                let rank = rank_for_entity(entity, symbol);
                let rank_reason = rank_reason_for(rank);

                let package = extract_package(&entity.canonical_name);
                let display = bare_name(&entity.canonical_name);

                candidates.push(DefinitionCandidate::new(
                    entity.id,
                    anchor_id,
                    entity.canonical_name.clone(),
                    display,
                    package,
                    entity.kind,
                    entity.provenance,
                    entity.confidence,
                    rank,
                    rank_reason,
                ));
            }
        }

        self.sort_candidates(&mut candidates);
        candidates
    }

    fn references(&self, entity_id: EntityId) -> Vec<OccurrenceFact> {
        let ref_edges = self.reference_index.get_by_entity(entity_id);

        let mut results = Vec::with_capacity(ref_edges.len());
        for edge in ref_edges {
            // Reconstruct an OccurrenceFact from the ReferenceEdge data.
            results.push(OccurrenceFact {
                id: edge.occurrence_id,
                kind: edge.kind,
                entity_id: edge.target_candidates.first().copied(),
                anchor_id: edge.anchor_id,
                scope_id: None,
                provenance: edge.provenance,
                confidence: edge.confidence,
            });
        }

        results
    }

    fn visible_symbols_at(
        &self,
        file_id: FileId,
        byte_offset: u32,
        scope_id: Option<ScopeId>,
    ) -> Vec<VisibleSymbol> {
        match self.shard_for_file(file_id) {
            Some(shard) => visibility::visible_symbols_at(
                file_id,
                byte_offset,
                scope_id,
                shard,
                self.import_export_index,
            ),
            None => Vec::new(),
        }
    }

    fn method_candidates(
        &self,
        receiver_package: &str,
        method_name: &str,
    ) -> Vec<DefinitionCandidate> {
        let graph = match self.package_graph {
            Some(g) => g,
            None => return Vec::new(),
        };

        // If the receiver package is not known in the graph, return empty
        // (conservative for unknown receiver shapes).
        if graph.get_node(receiver_package).is_none() {
            // Still search fact shards directly — the package may have
            // entities even without graph edges.
            let mut candidates = self.find_method_entities(receiver_package, method_name);
            self.sort_candidates(&mut candidates);
            return candidates;
        }

        // Collect all packages to search: the receiver itself, its ancestors,
        // and its composed roles (and ancestors' composed roles).
        let mut packages_to_search = vec![receiver_package.to_string()];

        // Add composed roles of the receiver.
        let receiver_roles = graph.composed_roles(receiver_package);
        packages_to_search.extend(receiver_roles);

        // Add ancestors (inheritance chain).
        let ancestor_result = graph.ancestors(receiver_package);
        for ancestor in &ancestor_result.ancestors {
            packages_to_search.push(ancestor.clone());
            // Also add composed roles of each ancestor.
            let ancestor_roles = graph.composed_roles(ancestor);
            packages_to_search.extend(ancestor_roles);
        }

        // Deduplicate while preserving order (receiver first, then MRO order).
        let mut seen = std::collections::HashSet::new();
        packages_to_search.retain(|pkg| seen.insert(pkg.clone()));

        // Collect method candidates from all packages in the chain.
        let mut candidates = Vec::new();
        for pkg in &packages_to_search {
            candidates.extend(self.find_method_entities(pkg, method_name));
        }

        self.sort_candidates(&mut candidates);
        candidates
    }

    fn rename_plan(&self, entity_id: EntityId, new_name: &str) -> RenamePlan {
        let old_name = self.find_entity_name(entity_id).unwrap_or_default();
        let entity_info = self.find_entity(entity_id);

        let mut edits = Vec::new();
        let mut blockers = Vec::new();
        let mut warnings = Vec::new();

        // ── Block on generated-member entities ──
        // Generated members (Moo/Moose accessors) cannot be safely renamed
        // without a generator-specific edit plan (Req 17.6).
        if let Some(ref info) = entity_info {
            if info.kind == EntityKind::GeneratedMember {
                blockers.push(PlanBlocker::new(
                    PlanBlockerReason::GeneratedMember,
                    info.anchor_id,
                    "Cannot rename generated member without a generator-specific edit plan."
                        .to_string(),
                ));
                return RenamePlan::new(
                    entity_id,
                    old_name,
                    new_name.to_string(),
                    edits,
                    blockers,
                    warnings,
                );
            }
        }

        // ── Collect definition occurrences ──
        let bare = bare_name(&old_name);
        for shard in self.fact_shards.values() {
            for occ in &shard.occurrences {
                if occ.entity_id != Some(entity_id) {
                    continue;
                }

                let category = classify_occurrence(occ.kind);

                // Dynamic boundary references → add blocker (Req 16.2).
                if occ.kind == OccurrenceKind::DynamicBoundary {
                    blockers.push(PlanBlocker::new(
                        PlanBlockerReason::DynamicBoundary,
                        Some(occ.anchor_id),
                        "Reference crosses a dynamic boundary (string eval, symbolic deref, or AUTOLOAD).".to_string(),
                    ));
                    continue;
                }

                match category {
                    Some(cat) => {
                        edits.push(PlannedEdit::new(
                            occ.anchor_id,
                            shard.file_id,
                            cat,
                            bare.clone(),
                            new_name.to_string(),
                        ));
                    }
                    None => {
                        // Unclassified occurrence → block rather than silently
                        // omitting (Req 16.7).
                        blockers.push(PlanBlocker::new(
                            PlanBlockerReason::UnclassifiedOccurrence,
                            Some(occ.anchor_id),
                            format!(
                                "Occurrence kind {:?} could not be classified into a rename edit category.",
                                occ.kind
                            ),
                        ));
                    }
                }
            }
        }

        // ── Collect reference occurrences from the reference index ──
        let ref_edges = self.reference_index.get_by_entity(entity_id);
        for edge in ref_edges {
            if edge.kind == OccurrenceKind::DynamicBoundary {
                blockers.push(PlanBlocker::new(
                    PlanBlockerReason::DynamicBoundary,
                    Some(edge.anchor_id),
                    "Reference crosses a dynamic boundary (string eval, symbolic deref, or AUTOLOAD).".to_string(),
                ));
                continue;
            }

            let category = classify_occurrence(edge.kind);
            match category {
                Some(cat) => {
                    edits.push(PlannedEdit::new(
                        edge.anchor_id,
                        edge.file_id,
                        cat,
                        bare.clone(),
                        new_name.to_string(),
                    ));
                }
                None => {
                    blockers.push(PlanBlocker::new(
                        PlanBlockerReason::UnclassifiedOccurrence,
                        Some(edge.anchor_id),
                        format!(
                            "Reference edge kind {:?} could not be classified into a rename edit category.",
                            edge.kind
                        ),
                    ));
                }
            }
        }

        // ── Cross-module export check (Req 16.3) ──
        // If the symbol is exported and referenced from other modules,
        // add a CrossModuleExport blocker.
        if let Some(exporting_module) = self.import_export_index.find_exporting_module(&bare) {
            // Check if any other file imports this symbol.
            let entity_file_id = entity_info.as_ref().and_then(|e| {
                e.anchor_id.and_then(|aid| {
                    self.fact_shards
                        .values()
                        .find_map(|s| s.anchors.iter().find(|a| a.id == aid).map(|_| s.file_id))
                })
            });

            let has_cross_module_refs = entity_file_id
                .map(|fid| self.import_export_index.is_imported_by_other_file(&bare, fid))
                .unwrap_or(false);

            if has_cross_module_refs {
                blockers.push(PlanBlocker::new(
                    PlanBlockerReason::CrossModuleExport,
                    None,
                    format!(
                        "Symbol '{}' is exported by module '{}' and imported by other files.",
                        bare, exporting_module
                    ),
                ));
            } else {
                // Exported but not imported — warn rather than block.
                warnings.push(PlanWarning::new(
                    format!(
                        "Symbol '{}' is listed in the export set of module '{}'.",
                        bare, exporting_module
                    ),
                    None,
                ));
            }
        }

        // Deduplicate edits by anchor_id (an occurrence may appear in both
        // the shard scan and the reference index).
        edits.sort_by_key(|e| (e.file_id, e.anchor_id));
        edits.dedup_by_key(|e| e.anchor_id);

        RenamePlan::new(entity_id, old_name, new_name.to_string(), edits, blockers, warnings)
    }

    fn safe_delete_plan(&self, entity_id: EntityId) -> SafeDeletePlan {
        let name = self.find_entity_name(entity_id).unwrap_or_default();
        let entity_info = self.find_entity(entity_id);

        let mut blockers = Vec::new();
        let mut warnings = Vec::new();

        // ── Block on generated-member entities (Req 17.7) ──
        // Generated members (Moo/Moose accessors) cannot be safely deleted
        // without a generator-specific delete plan.
        if let Some(ref info) = entity_info {
            if info.kind == EntityKind::GeneratedMember {
                blockers.push(PlanBlocker::new(
                    PlanBlockerReason::GeneratedMember,
                    info.anchor_id,
                    "Cannot delete generated member without a generator-specific delete plan."
                        .to_string(),
                ));
                return SafeDeletePlan::new(entity_id, name, blockers, warnings);
            }
        }

        let bare = bare_name(&name);

        // ── Check for remaining references (Req 17.2) ──
        // If the symbol has references in the workspace, block deletion.
        let ref_edges = self.reference_index.get_by_entity(entity_id);
        if !ref_edges.is_empty() {
            blockers.push(PlanBlocker::new(
                PlanBlockerReason::ReferencesExist,
                None,
                format!(
                    "Symbol '{}' still has {} reference(s) in the workspace.",
                    bare,
                    ref_edges.len()
                ),
            ));
        }

        // Also check occurrences in fact shards for non-definition references
        // that may not be in the reference index.
        let shard_ref_count: usize = self
            .fact_shards
            .values()
            .flat_map(|s| s.occurrences.iter())
            .filter(|occ| {
                occ.entity_id == Some(entity_id) && occ.kind != OccurrenceKind::Definition
            })
            .count();

        if ref_edges.is_empty() && shard_ref_count > 0 {
            blockers.push(PlanBlocker::new(
                PlanBlockerReason::ReferencesExist,
                None,
                format!(
                    "Symbol '{}' still has {} reference(s) in fact shards.",
                    bare, shard_ref_count
                ),
            ));
        }

        // ── Check if symbol is in an ExportSet (Req 17.3) ──
        if let Some(exporting_module) = self.import_export_index.find_exporting_module(&bare) {
            blockers.push(PlanBlocker::new(
                PlanBlockerReason::ExportedSymbol,
                None,
                format!(
                    "Symbol '{}' is listed in the export set of module '{}'.",
                    bare, exporting_module
                ),
            ));
        }

        // ── Check if symbol is imported by another file (Req 17.4) ──
        let entity_file_id = entity_info.as_ref().and_then(|e| {
            e.anchor_id.and_then(|aid| {
                self.fact_shards
                    .values()
                    .find_map(|s| s.anchors.iter().find(|a| a.id == aid).map(|_| s.file_id))
            })
        });

        let is_imported = entity_file_id
            .map(|fid| self.import_export_index.is_imported_by_other_file(&bare, fid))
            .unwrap_or(false);

        if is_imported {
            blockers.push(PlanBlocker::new(
                PlanBlockerReason::ImportedSymbol,
                None,
                format!("Symbol '{}' is imported by another file.", bare),
            ));
        }

        // ── Add a warning when no blockers found ──
        if blockers.is_empty() {
            warnings
                .push(PlanWarning::new(format!("Symbol '{}' appears safe to delete.", bare), None));
        }

        SafeDeletePlan::new(entity_id, name, blockers, warnings)
    }
}

impl<'a> WorkspaceSemanticQueries<'a> {
    /// Look up an entity's canonical name across all shards.
    fn find_entity_name(&self, entity_id: EntityId) -> Option<String> {
        for shard in self.fact_shards.values() {
            if let Some(entity) = shard.entities.iter().find(|e| e.id == entity_id) {
                return Some(entity.canonical_name.clone());
            }
        }
        None
    }

    /// Look up an entity fact across all shards.
    fn find_entity(&self, entity_id: EntityId) -> Option<EntityFact> {
        for shard in self.fact_shards.values() {
            if let Some(entity) = shard.entities.iter().find(|e| e.id == entity_id) {
                return Some(entity.clone());
            }
        }
        None
    }

    /// Return method candidates filtered by a receiver's [`ValueShape`].
    ///
    /// When the shape resolves to a known package (`Object` or
    /// `PackageName`), delegates to [`method_candidates`](SemanticQueries::method_candidates)
    /// with that package. For shapes that do not identify a package
    /// (`Unknown`, `Scalar`, etc.), returns an empty list (conservative).
    pub fn method_candidates_by_shape(
        &self,
        receiver_shape: &ValueShape,
        method_name: &str,
    ) -> Vec<DefinitionCandidate> {
        match ValueShapeIndex::resolve_receiver_package(receiver_shape) {
            Some(package) => self.method_candidates(package, method_name),
            None => Vec::new(),
        }
    }

    /// Return method candidates for a receiver identified by entity ID.
    ///
    /// Looks up the entity's [`ValueShape`] in the value-shape index, then
    /// delegates to [`method_candidates_by_shape`](Self::method_candidates_by_shape).
    /// Returns an empty list when the entity has no known shape or the
    /// value-shape index is not available.
    pub fn method_candidates_for_entity(
        &self,
        entity_id: EntityId,
        method_name: &str,
    ) -> Vec<DefinitionCandidate> {
        let vs_index = match self.value_shape_index {
            Some(idx) => idx,
            None => return Vec::new(),
        };

        match vs_index.get(entity_id) {
            Some(shape) => self.method_candidates_by_shape(shape, method_name),
            None => Vec::new(),
        }
    }

    /// Find method/subroutine/generated-member entities in a package that
    /// match the given method name.
    ///
    /// Searches all fact shards for entities whose canonical name is
    /// `package::method_name` and whose kind is Method, Subroutine, or
    /// GeneratedMember.
    fn find_method_entities(&self, package: &str, method_name: &str) -> Vec<DefinitionCandidate> {
        let qualified = format!("{package}::{method_name}");
        let mut candidates = Vec::new();

        for shard in self.fact_shards.values() {
            for entity in &shard.entities {
                // Match by qualified name.
                if entity.canonical_name != qualified {
                    continue;
                }

                // Only method-like entities are candidates.
                if !matches!(
                    entity.kind,
                    EntityKind::Method | EntityKind::Subroutine | EntityKind::GeneratedMember
                ) {
                    continue;
                }

                let anchor_id = match entity.anchor_id {
                    Some(id) => id,
                    None => continue,
                };

                let rank = DefinitionRank::ExactQualified;
                let rank_reason = DefinitionRankReason::ExactQualifiedName;

                candidates.push(DefinitionCandidate::new(
                    entity.id,
                    anchor_id,
                    entity.canonical_name.clone(),
                    method_name.to_string(),
                    Some(package.to_string()),
                    entity.kind,
                    entity.provenance,
                    entity.confidence,
                    rank,
                    rank_reason,
                ));
            }
        }

        candidates
    }
}

// ── Private helpers ──

/// Extract the bare name from a potentially qualified name.
///
/// `"Foo::Bar::baz"` → `"baz"`, `"baz"` → `"baz"`.
fn bare_name(qualified: &str) -> String {
    match qualified.rsplit_once("::") {
        Some((_, bare)) => bare.to_string(),
        None => qualified.to_string(),
    }
}

/// Extract the package prefix from a qualified name.
///
/// `"Foo::Bar::baz"` → `Some("Foo::Bar")`, `"baz"` → `None`.
fn extract_package(qualified: &str) -> Option<String> {
    qualified.rsplit_once("::").map(|(pkg, _)| pkg.to_string())
}

/// Determine whether an entity kind represents a definition that should
/// appear in definition candidate lists.
fn is_definition_kind(kind: EntityKind) -> bool {
    matches!(
        kind,
        EntityKind::Subroutine
            | EntityKind::Method
            | EntityKind::Variable
            | EntityKind::Constant
            | EntityKind::Package
            | EntityKind::Class
            | EntityKind::Role
            | EntityKind::Module
            | EntityKind::Field
            | EntityKind::GeneratedMember
    )
}

/// Assign a coarse rank to an entity based on name match quality.
fn rank_for_entity(entity: &EntityFact, symbol: &str) -> DefinitionRank {
    if entity.canonical_name == symbol && symbol.contains("::") {
        DefinitionRank::ExactQualified
    } else if entity.canonical_name == symbol {
        // Bare name exact match — treat as same-package.
        DefinitionRank::SamePackage
    } else if bare_name(&entity.canonical_name) == symbol {
        // Bare name matches but qualified name differs — workspace candidate.
        DefinitionRank::WorkspaceCandidate
    } else {
        DefinitionRank::Heuristic
    }
}

/// Produce a structured rank reason from a rank tier.
fn rank_reason_for(rank: DefinitionRank) -> DefinitionRankReason {
    match rank {
        DefinitionRank::ExactQualified => DefinitionRankReason::ExactQualifiedName,
        DefinitionRank::SamePackage => DefinitionRankReason::SamePackage,
        DefinitionRank::ExplicitImport => {
            DefinitionRankReason::ExplicitImport { module: String::new() }
        }
        DefinitionRank::DefaultExport => {
            DefinitionRankReason::DefaultExport { module: String::new() }
        }
        DefinitionRank::WorkspaceCandidate => DefinitionRankReason::WorkspaceSymbol,
        DefinitionRank::Heuristic => DefinitionRankReason::HeuristicNameMatch,
        // DefinitionRank is #[non_exhaustive]; future variants get heuristic.
        _ => DefinitionRankReason::HeuristicNameMatch,
    }
}

/// Classify an [`OccurrenceKind`] into a [`PlannedEditCategory`] for rename.
///
/// Returns `None` for occurrence kinds that cannot be mapped to a rename
/// edit category (e.g. `DynamicBoundary` is handled separately as a blocker).
fn classify_occurrence(kind: OccurrenceKind) -> Option<PlannedEditCategory> {
    match kind {
        OccurrenceKind::Definition => Some(PlannedEditCategory::Definition),
        OccurrenceKind::Import => Some(PlannedEditCategory::ImportList),
        OccurrenceKind::Export => Some(PlannedEditCategory::ExportList),
        OccurrenceKind::Reference
        | OccurrenceKind::Read
        | OccurrenceKind::Write
        | OccurrenceKind::Call
        | OccurrenceKind::MethodCall
        | OccurrenceKind::StaticMethodCall
        | OccurrenceKind::GeneratedUse => Some(PlannedEditCategory::Reference),
        // Inheritance and role-composition are structural edges, not rename
        // targets — warn rather than silently omitting.
        OccurrenceKind::Inheritance | OccurrenceKind::RoleComposition => None,
        // DynamicBoundary is handled as a blocker before this function is called.
        OccurrenceKind::DynamicBoundary => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_semantic_facts::{
        AnchorFact, AnchorId, Confidence, EdgeFact, EdgeId, EdgeKind, EntityFact, EntityId,
        EntityKind, FileId, OccurrenceFact, OccurrenceId, OccurrenceKind, PackageEdge,
        PackageEdgeKind, PlanBlockerReason, PlannedEditCategory, Provenance, ScopeId,
    };
    use std::collections::HashMap;

    // ── Test helpers ──

    fn make_shard(
        uri: &str,
        file_id: FileId,
        anchors: Vec<AnchorFact>,
        entities: Vec<EntityFact>,
        occurrences: Vec<OccurrenceFact>,
        edges: Vec<EdgeFact>,
    ) -> FileFactShard {
        FileFactShard {
            source_uri: uri.to_string(),
            file_id,
            content_hash: 0,
            anchors_hash: None,
            entities_hash: None,
            occurrences_hash: None,
            edges_hash: None,
            anchors,
            entities,
            occurrences,
            edges,
        }
    }

    fn simple_shard() -> (FileId, FileFactShard) {
        let file_id = FileId(1);
        let anchor_def = AnchorId(10);
        let anchor_ref = AnchorId(20);
        let entity_id = EntityId(100);

        let shard = make_shard(
            "file:///lib/Foo.pm",
            file_id,
            vec![
                AnchorFact {
                    id: anchor_def,
                    file_id,
                    span_start_byte: 0,
                    span_end_byte: 15,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                AnchorFact {
                    id: anchor_ref,
                    file_id,
                    span_start_byte: 50,
                    span_end_byte: 58,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
            ],
            vec![EntityFact {
                id: entity_id,
                kind: EntityKind::Subroutine,
                canonical_name: "Foo::bar".to_string(),
                anchor_id: Some(anchor_def),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![
                OccurrenceFact {
                    id: OccurrenceId(200),
                    kind: OccurrenceKind::Definition,
                    entity_id: Some(entity_id),
                    anchor_id: anchor_def,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                OccurrenceFact {
                    id: OccurrenceId(201),
                    kind: OccurrenceKind::Call,
                    entity_id: Some(entity_id),
                    anchor_id: anchor_ref,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
            ],
            vec![EdgeFact {
                id: EdgeId(300),
                kind: EdgeKind::References,
                from_entity_id: EntityId(0),
                to_entity_id: entity_id,
                via_occurrence_id: Some(OccurrenceId(201)),
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
        );

        (file_id, shard)
    }

    fn build_queries<'a>(
        ref_index: &'a ReferenceIndex,
        ie_index: &'a ImportExportIndex,
        shards: &'a HashMap<String, FileFactShard>,
    ) -> WorkspaceSemanticQueries<'a> {
        WorkspaceSemanticQueries::new(ref_index, ie_index, shards)
    }

    // ── QueryContext tests ──

    #[test]
    fn query_context_new_sets_fields() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = QueryContext::new(FileId(1), Some(ScopeId(2)), Some(42));
        assert_eq!(ctx.file_id, FileId(1));
        assert_eq!(ctx.scope_id, Some(ScopeId(2)));
        assert_eq!(ctx.byte_offset, Some(42));
        Ok(())
    }

    #[test]
    fn query_context_with_none_fields() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = QueryContext::new(FileId(5), None, None);
        assert_eq!(ctx.file_id, FileId(5));
        assert_eq!(ctx.scope_id, None);
        assert_eq!(ctx.byte_offset, None);
        Ok(())
    }

    // ── symbol_at tests ──

    #[test]
    fn symbol_at_returns_entity_and_occurrence() -> Result<(), Box<dyn std::error::Error>> {
        let (file_id, shard) = simple_shard();
        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        // Query at byte offset 5, which falls within the definition anchor (0..15).
        let result = queries.symbol_at(file_id, 5);
        assert!(result.is_some(), "should find symbol at offset 5");

        let (entity, occ) = result.ok_or("expected symbol_at result")?;
        assert_eq!(entity.id, EntityId(100));
        assert_eq!(entity.canonical_name, "Foo::bar");
        assert_eq!(occ.kind, OccurrenceKind::Definition);
        Ok(())
    }

    #[test]
    fn symbol_at_returns_none_for_empty_position() -> Result<(), Box<dyn std::error::Error>> {
        let (file_id, shard) = simple_shard();
        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        // Query at byte offset 30, which is between the two anchors.
        let result = queries.symbol_at(file_id, 30);
        assert!(result.is_none(), "should not find symbol at offset 30");
        Ok(())
    }

    #[test]
    fn symbol_at_returns_none_for_unknown_file() -> Result<(), Box<dyn std::error::Error>> {
        let shards = HashMap::new();
        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let result = queries.symbol_at(FileId(999), 0);
        assert!(result.is_none(), "should return None for unknown file");
        Ok(())
    }

    // ── definitions tests ──

    #[test]
    fn definitions_finds_by_qualified_name() -> Result<(), Box<dyn std::error::Error>> {
        let (file_id, shard) = simple_shard();
        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let ctx = QueryContext::new(file_id, None, None);
        let candidates = queries.definitions("Foo::bar", &ctx);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].entity_id, EntityId(100));
        assert_eq!(candidates[0].canonical_name, "Foo::bar");
        assert_eq!(candidates[0].rank, DefinitionRank::ExactQualified);
        Ok(())
    }

    #[test]
    fn definitions_finds_by_bare_name() -> Result<(), Box<dyn std::error::Error>> {
        let (file_id, shard) = simple_shard();
        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let ctx = QueryContext::new(file_id, None, None);
        let candidates = queries.definitions("bar", &ctx);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].entity_id, EntityId(100));
        assert_eq!(candidates[0].display_name, "bar");
        assert_eq!(candidates[0].rank, DefinitionRank::WorkspaceCandidate);
        Ok(())
    }

    #[test]
    fn definitions_returns_empty_for_unknown_symbol() -> Result<(), Box<dyn std::error::Error>> {
        let (file_id, shard) = simple_shard();
        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let ctx = QueryContext::new(file_id, None, None);
        let candidates = queries.definitions("nonexistent", &ctx);

        assert!(candidates.is_empty(), "should return empty list for unknown symbol");
        Ok(())
    }

    #[test]
    fn definitions_sorted_by_rank() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = make_shard(
            "file:///lib/Multi.pm",
            file_id,
            vec![
                AnchorFact {
                    id: AnchorId(10),
                    file_id,
                    span_start_byte: 0,
                    span_end_byte: 10,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                AnchorFact {
                    id: AnchorId(20),
                    file_id,
                    span_start_byte: 20,
                    span_end_byte: 30,
                    scope_id: None,
                    provenance: Provenance::NameHeuristic,
                    confidence: Confidence::Low,
                },
            ],
            vec![
                EntityFact {
                    id: EntityId(1),
                    kind: EntityKind::Subroutine,
                    canonical_name: "Other::process".to_string(),
                    anchor_id: Some(AnchorId(20)),
                    scope_id: None,
                    provenance: Provenance::NameHeuristic,
                    confidence: Confidence::Low,
                },
                EntityFact {
                    id: EntityId(2),
                    kind: EntityKind::Subroutine,
                    canonical_name: "Multi::process".to_string(),
                    anchor_id: Some(AnchorId(10)),
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
            ],
            vec![],
            vec![],
        );

        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let ctx = QueryContext::new(file_id, None, None);
        // Search by bare name "process" — both entities match.
        let candidates = queries.definitions("process", &ctx);

        assert_eq!(candidates.len(), 2);
        // Both should be WorkspaceCandidate rank (bare name match).
        // Within same rank, sorted by URI then position.
        assert!(candidates[0].rank <= candidates[1].rank, "candidates should be sorted by rank");
        Ok(())
    }

    #[test]
    fn definitions_deterministic_within_same_rank() -> Result<(), Box<dyn std::error::Error>> {
        let file_a = FileId(1);
        let file_b = FileId(2);

        let shard_a = make_shard(
            "file:///lib/A.pm",
            file_a,
            vec![AnchorFact {
                id: AnchorId(10),
                file_id: file_a,
                span_start_byte: 100,
                span_end_byte: 110,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![EntityFact {
                id: EntityId(1),
                kind: EntityKind::Subroutine,
                canonical_name: "A::helper".to_string(),
                anchor_id: Some(AnchorId(10)),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![],
            vec![],
        );

        let shard_b = make_shard(
            "file:///lib/B.pm",
            file_b,
            vec![AnchorFact {
                id: AnchorId(20),
                file_id: file_b,
                span_start_byte: 50,
                span_end_byte: 60,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![EntityFact {
                id: EntityId(2),
                kind: EntityKind::Subroutine,
                canonical_name: "B::helper".to_string(),
                anchor_id: Some(AnchorId(20)),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![],
            vec![],
        );

        let mut shards = HashMap::new();
        shards.insert(shard_a.source_uri.clone(), shard_a);
        shards.insert(shard_b.source_uri.clone(), shard_b);

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let ctx = QueryContext::new(file_a, None, None);
        let candidates = queries.definitions("helper", &ctx);

        assert_eq!(candidates.len(), 2);
        // Both are WorkspaceCandidate rank. Within same rank, sorted by URI.
        // "file:///lib/A.pm" < "file:///lib/B.pm"
        assert_eq!(candidates[0].canonical_name, "A::helper");
        assert_eq!(candidates[1].canonical_name, "B::helper");
        Ok(())
    }

    // ── references tests ──

    #[test]
    fn references_returns_occurrences_for_entity() -> Result<(), Box<dyn std::error::Error>> {
        let (_file_id, shard) = simple_shard();
        let mut ref_index = ReferenceIndex::new();
        ref_index.add_file(&shard);

        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let refs = queries.references(EntityId(100));
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].kind, OccurrenceKind::Call);
        assert_eq!(refs[0].anchor_id, AnchorId(20));
        Ok(())
    }

    #[test]
    fn references_returns_empty_for_unknown_entity() -> Result<(), Box<dyn std::error::Error>> {
        let shards = HashMap::new();
        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let refs = queries.references(EntityId(999));
        assert!(refs.is_empty(), "should return empty for unknown entity");
        Ok(())
    }

    // ── visible_symbols_at tests ──

    #[test]
    fn visible_symbols_at_delegates_to_visibility() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = make_shard(
            "file:///lib/Main.pm",
            file_id,
            vec![AnchorFact {
                id: AnchorId(10),
                file_id,
                span_start_byte: 0,
                span_end_byte: 20,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![EntityFact {
                id: EntityId(100),
                kind: EntityKind::Subroutine,
                canonical_name: "Main::do_stuff".to_string(),
                anchor_id: Some(AnchorId(10)),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![],
            vec![],
        );

        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let symbols = queries.visible_symbols_at(file_id, 50, None);
        let sub_sym = symbols.iter().find(|s| s.name == "do_stuff");
        assert!(sub_sym.is_some(), "subroutine should be visible");
        Ok(())
    }

    #[test]
    fn visible_symbols_at_returns_empty_for_unknown_file() -> Result<(), Box<dyn std::error::Error>>
    {
        let shards = HashMap::new();
        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let symbols = queries.visible_symbols_at(FileId(999), 0, None);
        assert!(symbols.is_empty(), "should return empty for unknown file");
        Ok(())
    }

    // ── method_candidates tests ──

    #[test]
    fn method_candidates_returns_empty_without_package_graph()
    -> Result<(), Box<dyn std::error::Error>> {
        let shards = HashMap::new();
        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let candidates = queries.method_candidates("Foo", "new");
        assert!(candidates.is_empty(), "should return empty without package graph");
        Ok(())
    }

    #[test]
    fn method_candidates_finds_method_in_receiver_package() -> Result<(), Box<dyn std::error::Error>>
    {
        let file_id = FileId(1);
        let shard = make_shard(
            "file:///lib/Dog.pm",
            file_id,
            vec![AnchorFact {
                id: AnchorId(10),
                file_id,
                span_start_byte: 0,
                span_end_byte: 15,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![EntityFact {
                id: EntityId(100),
                kind: EntityKind::Method,
                canonical_name: "Dog::bark".to_string(),
                anchor_id: Some(AnchorId(10)),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![],
            vec![],
        );

        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let mut pkg_graph = PackageGraphIndex::new();
        pkg_graph.add_edges(
            "file:///lib/Dog.pm",
            file_id,
            vec![PackageEdge::new(
                "Dog".to_string(),
                "Animal".to_string(),
                PackageEdgeKind::Inherits,
                Some(AnchorId(1)),
                Provenance::ExactAst,
                Confidence::High,
            )],
        );

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = WorkspaceSemanticQueries::with_package_graph(
            &ref_index, &ie_index, &shards, &pkg_graph,
        );

        let candidates = queries.method_candidates("Dog", "bark");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].canonical_name, "Dog::bark");
        assert_eq!(candidates[0].kind, EntityKind::Method);
        Ok(())
    }

    #[test]
    fn method_candidates_finds_inherited_method() -> Result<(), Box<dyn std::error::Error>> {
        let file_child = FileId(1);
        let file_parent = FileId(2);

        let shard_child =
            make_shard("file:///lib/Child.pm", file_child, vec![], vec![], vec![], vec![]);

        let shard_parent = make_shard(
            "file:///lib/Parent.pm",
            file_parent,
            vec![AnchorFact {
                id: AnchorId(20),
                file_id: file_parent,
                span_start_byte: 0,
                span_end_byte: 15,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![EntityFact {
                id: EntityId(200),
                kind: EntityKind::Method,
                canonical_name: "Parent::greet".to_string(),
                anchor_id: Some(AnchorId(20)),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![],
            vec![],
        );

        let mut shards = HashMap::new();
        shards.insert(shard_child.source_uri.clone(), shard_child);
        shards.insert(shard_parent.source_uri.clone(), shard_parent);

        let mut pkg_graph = PackageGraphIndex::new();
        pkg_graph.add_edges(
            "file:///lib/Child.pm",
            file_child,
            vec![PackageEdge::new(
                "Child".to_string(),
                "Parent".to_string(),
                PackageEdgeKind::Inherits,
                Some(AnchorId(1)),
                Provenance::ExactAst,
                Confidence::High,
            )],
        );

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = WorkspaceSemanticQueries::with_package_graph(
            &ref_index, &ie_index, &shards, &pkg_graph,
        );

        let candidates = queries.method_candidates("Child", "greet");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].canonical_name, "Parent::greet");
        Ok(())
    }

    #[test]
    fn method_candidates_finds_role_composed_method() -> Result<(), Box<dyn std::error::Error>> {
        let file_class = FileId(1);
        let file_role = FileId(2);

        let shard_class =
            make_shard("file:///lib/MyClass.pm", file_class, vec![], vec![], vec![], vec![]);

        let shard_role = make_shard(
            "file:///lib/Printable.pm",
            file_role,
            vec![AnchorFact {
                id: AnchorId(30),
                file_id: file_role,
                span_start_byte: 0,
                span_end_byte: 20,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![EntityFact {
                id: EntityId(300),
                kind: EntityKind::Method,
                canonical_name: "Printable::to_string".to_string(),
                anchor_id: Some(AnchorId(30)),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![],
            vec![],
        );

        let mut shards = HashMap::new();
        shards.insert(shard_class.source_uri.clone(), shard_class);
        shards.insert(shard_role.source_uri.clone(), shard_role);

        let mut pkg_graph = PackageGraphIndex::new();
        pkg_graph.add_edges(
            "file:///lib/MyClass.pm",
            file_class,
            vec![PackageEdge::new(
                "MyClass".to_string(),
                "Printable".to_string(),
                PackageEdgeKind::ComposesRole,
                Some(AnchorId(1)),
                Provenance::ExactAst,
                Confidence::High,
            )],
        );

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = WorkspaceSemanticQueries::with_package_graph(
            &ref_index, &ie_index, &shards, &pkg_graph,
        );

        let candidates = queries.method_candidates("MyClass", "to_string");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].canonical_name, "Printable::to_string");
        Ok(())
    }

    #[test]
    fn method_candidates_includes_generated_members() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let shard = make_shard(
            "file:///lib/Person.pm",
            file_id,
            vec![AnchorFact {
                id: AnchorId(10),
                file_id,
                span_start_byte: 0,
                span_end_byte: 15,
                scope_id: None,
                provenance: Provenance::FrameworkSynthesis,
                confidence: Confidence::Medium,
            }],
            vec![EntityFact {
                id: EntityId(100),
                kind: EntityKind::GeneratedMember,
                canonical_name: "Person::name".to_string(),
                anchor_id: Some(AnchorId(10)),
                scope_id: None,
                provenance: Provenance::FrameworkSynthesis,
                confidence: Confidence::Medium,
            }],
            vec![],
            vec![],
        );

        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let mut pkg_graph = PackageGraphIndex::new();
        pkg_graph.add_edges(
            "file:///lib/Person.pm",
            file_id,
            vec![PackageEdge::new(
                "Person".to_string(),
                "Moo::Object".to_string(),
                PackageEdgeKind::Inherits,
                Some(AnchorId(1)),
                Provenance::ExactAst,
                Confidence::High,
            )],
        );

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = WorkspaceSemanticQueries::with_package_graph(
            &ref_index, &ie_index, &shards, &pkg_graph,
        );

        let candidates = queries.method_candidates("Person", "name");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].kind, EntityKind::GeneratedMember);
        assert_eq!(candidates[0].canonical_name, "Person::name");
        Ok(())
    }

    #[test]
    fn method_candidates_returns_empty_for_unknown_package()
    -> Result<(), Box<dyn std::error::Error>> {
        let shards = HashMap::new();
        let mut pkg_graph = PackageGraphIndex::new();
        // Add some unrelated package so the graph is non-empty.
        pkg_graph.add_edges(
            "file:///lib/Other.pm",
            FileId(1),
            vec![PackageEdge::new(
                "Other".to_string(),
                "Base".to_string(),
                PackageEdgeKind::Inherits,
                None,
                Provenance::ExactAst,
                Confidence::High,
            )],
        );

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = WorkspaceSemanticQueries::with_package_graph(
            &ref_index, &ie_index, &shards, &pkg_graph,
        );

        let candidates = queries.method_candidates("Unknown", "foo");
        assert!(candidates.is_empty(), "should return empty for unknown receiver package");
        Ok(())
    }

    #[test]
    fn method_candidates_traverses_deep_inheritance() -> Result<(), Box<dyn std::error::Error>> {
        let file_a = FileId(1);
        let file_b = FileId(2);
        let file_c = FileId(3);

        // C inherits B, B inherits A. Method defined on A.
        let shard_a = make_shard(
            "file:///lib/A.pm",
            file_a,
            vec![AnchorFact {
                id: AnchorId(10),
                file_id: file_a,
                span_start_byte: 0,
                span_end_byte: 10,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![EntityFact {
                id: EntityId(100),
                kind: EntityKind::Subroutine,
                canonical_name: "A::init".to_string(),
                anchor_id: Some(AnchorId(10)),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![],
            vec![],
        );

        let shard_b = make_shard("file:///lib/B.pm", file_b, vec![], vec![], vec![], vec![]);
        let shard_c = make_shard("file:///lib/C.pm", file_c, vec![], vec![], vec![], vec![]);

        let mut shards = HashMap::new();
        shards.insert(shard_a.source_uri.clone(), shard_a);
        shards.insert(shard_b.source_uri.clone(), shard_b);
        shards.insert(shard_c.source_uri.clone(), shard_c);

        let mut pkg_graph = PackageGraphIndex::new();
        pkg_graph.add_edges(
            "file:///lib/C.pm",
            file_c,
            vec![PackageEdge::new(
                "C".to_string(),
                "B".to_string(),
                PackageEdgeKind::Inherits,
                Some(AnchorId(1)),
                Provenance::ExactAst,
                Confidence::High,
            )],
        );
        pkg_graph.add_edges(
            "file:///lib/B.pm",
            file_b,
            vec![PackageEdge::new(
                "B".to_string(),
                "A".to_string(),
                PackageEdgeKind::Inherits,
                Some(AnchorId(2)),
                Provenance::ExactAst,
                Confidence::High,
            )],
        );

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = WorkspaceSemanticQueries::with_package_graph(
            &ref_index, &ie_index, &shards, &pkg_graph,
        );

        let candidates = queries.method_candidates("C", "init");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].canonical_name, "A::init");
        Ok(())
    }

    #[test]
    fn method_candidates_sorted_by_rank() -> Result<(), Box<dyn std::error::Error>> {
        let file_child = FileId(1);
        let file_parent = FileId(2);

        // Both Child and Parent define "process" — Child's should come first
        // since both are ExactQualified but Child's URI sorts before Parent's.
        let shard_child = make_shard(
            "file:///lib/Child.pm",
            file_child,
            vec![AnchorFact {
                id: AnchorId(10),
                file_id: file_child,
                span_start_byte: 0,
                span_end_byte: 15,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![EntityFact {
                id: EntityId(100),
                kind: EntityKind::Method,
                canonical_name: "Child::process".to_string(),
                anchor_id: Some(AnchorId(10)),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![],
            vec![],
        );

        let shard_parent = make_shard(
            "file:///lib/Parent.pm",
            file_parent,
            vec![AnchorFact {
                id: AnchorId(20),
                file_id: file_parent,
                span_start_byte: 0,
                span_end_byte: 15,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![EntityFact {
                id: EntityId(200),
                kind: EntityKind::Method,
                canonical_name: "Parent::process".to_string(),
                anchor_id: Some(AnchorId(20)),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![],
            vec![],
        );

        let mut shards = HashMap::new();
        shards.insert(shard_child.source_uri.clone(), shard_child);
        shards.insert(shard_parent.source_uri.clone(), shard_parent);

        let mut pkg_graph = PackageGraphIndex::new();
        pkg_graph.add_edges(
            "file:///lib/Child.pm",
            file_child,
            vec![PackageEdge::new(
                "Child".to_string(),
                "Parent".to_string(),
                PackageEdgeKind::Inherits,
                Some(AnchorId(1)),
                Provenance::ExactAst,
                Confidence::High,
            )],
        );

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = WorkspaceSemanticQueries::with_package_graph(
            &ref_index, &ie_index, &shards, &pkg_graph,
        );

        let candidates = queries.method_candidates("Child", "process");
        assert_eq!(candidates.len(), 2);
        // Both are ExactQualified; sorted by URI: Child.pm < Parent.pm.
        assert_eq!(candidates[0].canonical_name, "Child::process");
        assert_eq!(candidates[1].canonical_name, "Parent::process");
        Ok(())
    }

    // ── rename_plan tests ──

    #[test]
    fn rename_plan_returns_edits_for_known_entity() -> Result<(), Box<dyn std::error::Error>> {
        let (_, shard) = simple_shard();
        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let plan = queries.rename_plan(EntityId(100), "baz");
        assert_eq!(plan.entity_id, EntityId(100));
        assert_eq!(plan.old_name, "Foo::bar");
        assert_eq!(plan.new_name, "baz");
        // Should have edits for the definition and call occurrences.
        assert!(!plan.edits.is_empty(), "should have planned edits");
        // Definition occurrence should be classified as Definition.
        let def_edit = plan.edits.iter().find(|e| e.category == PlannedEditCategory::Definition);
        assert!(def_edit.is_some(), "should have a definition edit");
        // Call occurrence should be classified as Reference.
        let ref_edit = plan.edits.iter().find(|e| e.category == PlannedEditCategory::Reference);
        assert!(ref_edit.is_some(), "should have a reference edit");
        Ok(())
    }

    #[test]
    fn rename_plan_unknown_entity_returns_empty_plan() -> Result<(), Box<dyn std::error::Error>> {
        let shards = HashMap::new();
        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let plan = queries.rename_plan(EntityId(999), "new_name");
        assert_eq!(plan.old_name, "");
        assert!(plan.edits.is_empty());
        assert!(plan.blockers.is_empty());
        Ok(())
    }

    #[test]
    fn rename_plan_blocks_on_dynamic_boundary() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let entity_id = EntityId(100);
        let anchor_def = AnchorId(10);
        let anchor_dyn = AnchorId(20);

        let shard = make_shard(
            "file:///lib/Dyn.pm",
            file_id,
            vec![
                AnchorFact {
                    id: anchor_def,
                    file_id,
                    span_start_byte: 0,
                    span_end_byte: 10,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                AnchorFact {
                    id: anchor_dyn,
                    file_id,
                    span_start_byte: 50,
                    span_end_byte: 60,
                    scope_id: None,
                    provenance: Provenance::DynamicBoundary,
                    confidence: Confidence::Low,
                },
            ],
            vec![EntityFact {
                id: entity_id,
                kind: EntityKind::Subroutine,
                canonical_name: "Dyn::eval_sub".to_string(),
                anchor_id: Some(anchor_def),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![
                OccurrenceFact {
                    id: OccurrenceId(200),
                    kind: OccurrenceKind::Definition,
                    entity_id: Some(entity_id),
                    anchor_id: anchor_def,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                OccurrenceFact {
                    id: OccurrenceId(201),
                    kind: OccurrenceKind::DynamicBoundary,
                    entity_id: Some(entity_id),
                    anchor_id: anchor_dyn,
                    scope_id: None,
                    provenance: Provenance::DynamicBoundary,
                    confidence: Confidence::Low,
                },
            ],
            vec![],
        );

        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let plan = queries.rename_plan(entity_id, "safe_sub");
        let dyn_blockers: Vec<_> = plan
            .blockers
            .iter()
            .filter(|b| b.reason == PlanBlockerReason::DynamicBoundary)
            .collect();
        assert!(!dyn_blockers.is_empty(), "should have DynamicBoundary blocker");
        Ok(())
    }

    #[test]
    fn rename_plan_blocks_on_generated_member() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let entity_id = EntityId(100);
        let anchor_id = AnchorId(10);

        let shard = make_shard(
            "file:///lib/Gen.pm",
            file_id,
            vec![AnchorFact {
                id: anchor_id,
                file_id,
                span_start_byte: 0,
                span_end_byte: 10,
                scope_id: None,
                provenance: Provenance::FrameworkSynthesis,
                confidence: Confidence::Medium,
            }],
            vec![EntityFact {
                id: entity_id,
                kind: EntityKind::GeneratedMember,
                canonical_name: "Gen::name".to_string(),
                anchor_id: Some(anchor_id),
                scope_id: None,
                provenance: Provenance::FrameworkSynthesis,
                confidence: Confidence::Medium,
            }],
            vec![],
            vec![],
        );

        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let plan = queries.rename_plan(entity_id, "title");
        let gen_blockers: Vec<_> = plan
            .blockers
            .iter()
            .filter(|b| b.reason == PlanBlockerReason::GeneratedMember)
            .collect();
        assert!(!gen_blockers.is_empty(), "should have GeneratedMember blocker");
        // Should return early with no edits.
        assert!(plan.edits.is_empty(), "generated member rename should have no edits");
        Ok(())
    }

    #[test]
    fn rename_plan_blocks_on_cross_module_export() -> Result<(), Box<dyn std::error::Error>> {
        use perl_semantic_facts::{ExportSet, ImportKind, ImportSpec, ImportSymbols};

        let file_def = FileId(1);
        let file_importer = FileId(2);
        let entity_id = EntityId(100);
        let anchor_def = AnchorId(10);

        let shard_def = make_shard(
            "file:///lib/Exporter.pm",
            file_def,
            vec![AnchorFact {
                id: anchor_def,
                file_id: file_def,
                span_start_byte: 0,
                span_end_byte: 10,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![EntityFact {
                id: entity_id,
                kind: EntityKind::Subroutine,
                canonical_name: "MyExporter::helper".to_string(),
                anchor_id: Some(anchor_def),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![OccurrenceFact {
                id: OccurrenceId(200),
                kind: OccurrenceKind::Definition,
                entity_id: Some(entity_id),
                anchor_id: anchor_def,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![],
        );

        let shard_importer =
            make_shard("file:///lib/Consumer.pm", file_importer, vec![], vec![], vec![], vec![]);

        let mut shards = HashMap::new();
        shards.insert(shard_def.source_uri.clone(), shard_def);
        shards.insert(shard_importer.source_uri.clone(), shard_importer);

        let ref_index = ReferenceIndex::new();
        let mut ie_index = ImportExportIndex::new();

        // Register the export.
        ie_index.add_module_exports(
            "file:///lib/Exporter.pm",
            "MyExporter",
            ExportSet {
                default_exports: vec!["helper".to_string()],
                optional_exports: vec![],
                tags: vec![],
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
                module_name: Some("MyExporter".to_string()),
                anchor_id: None,
            },
        );

        // Register an import from another file.
        ie_index.add_file_imports(
            "file:///lib/Consumer.pm",
            file_importer,
            vec![ImportSpec {
                module: "MyExporter".to_string(),
                kind: ImportKind::UseExplicitList,
                symbols: ImportSymbols::Explicit(vec!["helper".to_string()]),
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
                file_id: Some(file_importer),
                anchor_id: None,
                scope_id: None,
            }],
        );

        let queries = build_queries(&ref_index, &ie_index, &shards);

        let plan = queries.rename_plan(entity_id, "new_helper");
        let export_blockers: Vec<_> = plan
            .blockers
            .iter()
            .filter(|b| b.reason == PlanBlockerReason::CrossModuleExport)
            .collect();
        assert!(!export_blockers.is_empty(), "should have CrossModuleExport blocker");
        Ok(())
    }

    #[test]
    fn rename_plan_classifies_import_and_export_occurrences()
    -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let entity_id = EntityId(100);
        let anchor_def = AnchorId(10);
        let anchor_import = AnchorId(20);
        let anchor_export = AnchorId(30);

        let shard = make_shard(
            "file:///lib/Classify.pm",
            file_id,
            vec![
                AnchorFact {
                    id: anchor_def,
                    file_id,
                    span_start_byte: 0,
                    span_end_byte: 10,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                AnchorFact {
                    id: anchor_import,
                    file_id,
                    span_start_byte: 20,
                    span_end_byte: 30,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                AnchorFact {
                    id: anchor_export,
                    file_id,
                    span_start_byte: 40,
                    span_end_byte: 50,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
            ],
            vec![EntityFact {
                id: entity_id,
                kind: EntityKind::Subroutine,
                canonical_name: "Classify::func".to_string(),
                anchor_id: Some(anchor_def),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![
                OccurrenceFact {
                    id: OccurrenceId(200),
                    kind: OccurrenceKind::Definition,
                    entity_id: Some(entity_id),
                    anchor_id: anchor_def,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                OccurrenceFact {
                    id: OccurrenceId(201),
                    kind: OccurrenceKind::Import,
                    entity_id: Some(entity_id),
                    anchor_id: anchor_import,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                OccurrenceFact {
                    id: OccurrenceId(202),
                    kind: OccurrenceKind::Export,
                    entity_id: Some(entity_id),
                    anchor_id: anchor_export,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
            ],
            vec![],
        );

        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let plan = queries.rename_plan(entity_id, "new_func");

        let def_edits: Vec<_> =
            plan.edits.iter().filter(|e| e.category == PlannedEditCategory::Definition).collect();
        let import_edits: Vec<_> =
            plan.edits.iter().filter(|e| e.category == PlannedEditCategory::ImportList).collect();
        let export_edits: Vec<_> =
            plan.edits.iter().filter(|e| e.category == PlannedEditCategory::ExportList).collect();

        assert_eq!(def_edits.len(), 1, "should have one definition edit");
        assert_eq!(import_edits.len(), 1, "should have one import list edit");
        assert_eq!(export_edits.len(), 1, "should have one export list edit");
        Ok(())
    }

    // ── safe_delete_plan tests ──

    #[test]
    fn safe_delete_plan_blocks_on_references() -> Result<(), Box<dyn std::error::Error>> {
        let (_, shard) = simple_shard();
        let mut ref_index = ReferenceIndex::new();
        ref_index.add_file(&shard);

        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let plan = queries.safe_delete_plan(EntityId(100));
        assert_eq!(plan.entity_id, EntityId(100));
        assert_eq!(plan.name, "Foo::bar");
        let ref_blockers: Vec<_> = plan
            .blockers
            .iter()
            .filter(|b| b.reason == PlanBlockerReason::ReferencesExist)
            .collect();
        assert!(!ref_blockers.is_empty(), "should have ReferencesExist blocker");
        Ok(())
    }

    #[test]
    fn safe_delete_plan_blocks_on_shard_references() -> Result<(), Box<dyn std::error::Error>> {
        // Entity has a Call occurrence in the shard but no reference index entries.
        let (_, shard) = simple_shard();
        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let plan = queries.safe_delete_plan(EntityId(100));
        let ref_blockers: Vec<_> = plan
            .blockers
            .iter()
            .filter(|b| b.reason == PlanBlockerReason::ReferencesExist)
            .collect();
        assert!(
            !ref_blockers.is_empty(),
            "should have ReferencesExist blocker from shard occurrences"
        );
        Ok(())
    }

    #[test]
    fn safe_delete_plan_blocks_on_exported_symbol() -> Result<(), Box<dyn std::error::Error>> {
        use perl_semantic_facts::ExportSet;

        let file_id = FileId(1);
        let entity_id = EntityId(100);
        let anchor_id = AnchorId(10);

        let shard = make_shard(
            "file:///lib/Exp.pm",
            file_id,
            vec![AnchorFact {
                id: anchor_id,
                file_id,
                span_start_byte: 0,
                span_end_byte: 10,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![EntityFact {
                id: entity_id,
                kind: EntityKind::Subroutine,
                canonical_name: "Exp::helper".to_string(),
                anchor_id: Some(anchor_id),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![OccurrenceFact {
                id: OccurrenceId(200),
                kind: OccurrenceKind::Definition,
                entity_id: Some(entity_id),
                anchor_id,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![],
        );

        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let ref_index = ReferenceIndex::new();
        let mut ie_index = ImportExportIndex::new();
        ie_index.add_module_exports(
            "file:///lib/Exp.pm",
            "Exp",
            ExportSet {
                default_exports: vec!["helper".to_string()],
                optional_exports: vec![],
                tags: vec![],
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
                module_name: Some("Exp".to_string()),
                anchor_id: None,
            },
        );

        let queries = build_queries(&ref_index, &ie_index, &shards);

        let plan = queries.safe_delete_plan(entity_id);
        let export_blockers: Vec<_> = plan
            .blockers
            .iter()
            .filter(|b| b.reason == PlanBlockerReason::ExportedSymbol)
            .collect();
        assert!(!export_blockers.is_empty(), "should have ExportedSymbol blocker");
        Ok(())
    }

    #[test]
    fn safe_delete_plan_blocks_on_imported_symbol() -> Result<(), Box<dyn std::error::Error>> {
        use perl_semantic_facts::{ExportSet, ImportKind, ImportSpec, ImportSymbols};

        let file_def = FileId(1);
        let file_importer = FileId(2);
        let entity_id = EntityId(100);
        let anchor_def = AnchorId(10);

        let shard_def = make_shard(
            "file:///lib/Provider.pm",
            file_def,
            vec![AnchorFact {
                id: anchor_def,
                file_id: file_def,
                span_start_byte: 0,
                span_end_byte: 10,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![EntityFact {
                id: entity_id,
                kind: EntityKind::Subroutine,
                canonical_name: "Provider::util".to_string(),
                anchor_id: Some(anchor_def),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![OccurrenceFact {
                id: OccurrenceId(200),
                kind: OccurrenceKind::Definition,
                entity_id: Some(entity_id),
                anchor_id: anchor_def,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![],
        );

        let shard_importer =
            make_shard("file:///lib/Consumer.pm", file_importer, vec![], vec![], vec![], vec![]);

        let mut shards = HashMap::new();
        shards.insert(shard_def.source_uri.clone(), shard_def);
        shards.insert(shard_importer.source_uri.clone(), shard_importer);

        let ref_index = ReferenceIndex::new();
        let mut ie_index = ImportExportIndex::new();

        // Register the export.
        ie_index.add_module_exports(
            "file:///lib/Provider.pm",
            "Provider",
            ExportSet {
                default_exports: vec!["util".to_string()],
                optional_exports: vec![],
                tags: vec![],
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
                module_name: Some("Provider".to_string()),
                anchor_id: None,
            },
        );

        // Register an import from another file.
        ie_index.add_file_imports(
            "file:///lib/Consumer.pm",
            file_importer,
            vec![ImportSpec {
                module: "Provider".to_string(),
                kind: ImportKind::UseExplicitList,
                symbols: ImportSymbols::Explicit(vec!["util".to_string()]),
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
                file_id: Some(file_importer),
                anchor_id: None,
                scope_id: None,
            }],
        );

        let queries = build_queries(&ref_index, &ie_index, &shards);

        let plan = queries.safe_delete_plan(entity_id);
        let import_blockers: Vec<_> = plan
            .blockers
            .iter()
            .filter(|b| b.reason == PlanBlockerReason::ImportedSymbol)
            .collect();
        assert!(!import_blockers.is_empty(), "should have ImportedSymbol blocker");
        Ok(())
    }

    #[test]
    fn safe_delete_plan_blocks_on_generated_member() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let entity_id = EntityId(100);
        let anchor_id = AnchorId(10);

        let shard = make_shard(
            "file:///lib/Gen.pm",
            file_id,
            vec![AnchorFact {
                id: anchor_id,
                file_id,
                span_start_byte: 0,
                span_end_byte: 10,
                scope_id: None,
                provenance: Provenance::FrameworkSynthesis,
                confidence: Confidence::Medium,
            }],
            vec![EntityFact {
                id: entity_id,
                kind: EntityKind::GeneratedMember,
                canonical_name: "Gen::name".to_string(),
                anchor_id: Some(anchor_id),
                scope_id: None,
                provenance: Provenance::FrameworkSynthesis,
                confidence: Confidence::Medium,
            }],
            vec![],
            vec![],
        );

        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let plan = queries.safe_delete_plan(entity_id);
        let gen_blockers: Vec<_> = plan
            .blockers
            .iter()
            .filter(|b| b.reason == PlanBlockerReason::GeneratedMember)
            .collect();
        assert!(!gen_blockers.is_empty(), "should have GeneratedMember blocker");
        Ok(())
    }

    #[test]
    fn safe_delete_plan_no_blockers_when_unreferenced() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let entity_id = EntityId(100);
        let anchor_id = AnchorId(10);

        // Entity with only a definition occurrence (no references).
        let shard = make_shard(
            "file:///lib/Unused.pm",
            file_id,
            vec![AnchorFact {
                id: anchor_id,
                file_id,
                span_start_byte: 0,
                span_end_byte: 10,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![EntityFact {
                id: entity_id,
                kind: EntityKind::Subroutine,
                canonical_name: "Unused::dead_code".to_string(),
                anchor_id: Some(anchor_id),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![OccurrenceFact {
                id: OccurrenceId(200),
                kind: OccurrenceKind::Definition,
                entity_id: Some(entity_id),
                anchor_id,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            vec![],
        );

        let mut shards = HashMap::new();
        shards.insert(shard.source_uri.clone(), shard);

        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let plan = queries.safe_delete_plan(entity_id);
        assert!(plan.blockers.is_empty(), "unreferenced symbol should have no blockers");
        assert!(!plan.warnings.is_empty(), "should have a safety warning");
        Ok(())
    }

    #[test]
    fn safe_delete_plan_unknown_entity_returns_empty_plan() -> Result<(), Box<dyn std::error::Error>>
    {
        let shards = HashMap::new();
        let ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();
        let queries = build_queries(&ref_index, &ie_index, &shards);

        let plan = queries.safe_delete_plan(EntityId(999));
        assert_eq!(plan.name, "");
        assert!(plan.blockers.is_empty());
        Ok(())
    }

    // ── Helper function tests ──

    #[test]
    fn bare_name_extracts_last_segment() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(bare_name("Foo::Bar::baz"), "baz");
        assert_eq!(bare_name("baz"), "baz");
        assert_eq!(bare_name("A::b"), "b");
        Ok(())
    }

    #[test]
    fn extract_package_returns_prefix() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(extract_package("Foo::Bar::baz"), Some("Foo::Bar".to_string()));
        assert_eq!(extract_package("baz"), None);
        assert_eq!(extract_package("A::b"), Some("A".to_string()));
        Ok(())
    }

    #[test]
    fn is_definition_kind_covers_expected_kinds() -> Result<(), Box<dyn std::error::Error>> {
        assert!(is_definition_kind(EntityKind::Subroutine));
        assert!(is_definition_kind(EntityKind::Method));
        assert!(is_definition_kind(EntityKind::Variable));
        assert!(is_definition_kind(EntityKind::Constant));
        assert!(is_definition_kind(EntityKind::Package));
        assert!(is_definition_kind(EntityKind::Class));
        assert!(is_definition_kind(EntityKind::Role));
        assert!(is_definition_kind(EntityKind::Module));
        assert!(is_definition_kind(EntityKind::Field));
        assert!(is_definition_kind(EntityKind::GeneratedMember));
        // Non-definition kinds:
        assert!(!is_definition_kind(EntityKind::Label));
        assert!(!is_definition_kind(EntityKind::Format));
        assert!(!is_definition_kind(EntityKind::ExternalSymbol));
        assert!(!is_definition_kind(EntityKind::Unknown));
        Ok(())
    }

    #[test]
    fn rank_for_entity_qualified_match() -> Result<(), Box<dyn std::error::Error>> {
        let entity = EntityFact {
            id: EntityId(1),
            kind: EntityKind::Subroutine,
            canonical_name: "Foo::bar".to_string(),
            anchor_id: Some(AnchorId(1)),
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        };
        assert_eq!(rank_for_entity(&entity, "Foo::bar"), DefinitionRank::ExactQualified);
        Ok(())
    }

    #[test]
    fn rank_for_entity_bare_exact_match() -> Result<(), Box<dyn std::error::Error>> {
        let entity = EntityFact {
            id: EntityId(1),
            kind: EntityKind::Subroutine,
            canonical_name: "bar".to_string(),
            anchor_id: Some(AnchorId(1)),
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        };
        assert_eq!(rank_for_entity(&entity, "bar"), DefinitionRank::SamePackage);
        Ok(())
    }

    #[test]
    fn rank_for_entity_bare_name_workspace_candidate() -> Result<(), Box<dyn std::error::Error>> {
        let entity = EntityFact {
            id: EntityId(1),
            kind: EntityKind::Subroutine,
            canonical_name: "Foo::bar".to_string(),
            anchor_id: Some(AnchorId(1)),
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        };
        assert_eq!(rank_for_entity(&entity, "bar"), DefinitionRank::WorkspaceCandidate);
        Ok(())
    }

    // ── Property-based tests ──

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;
        use proptest::test_runner::Config as ProptestConfig;

        /// Strategy to generate a random `DefinitionRank`.
        fn arb_definition_rank() -> impl Strategy<Value = DefinitionRank> {
            prop_oneof![
                Just(DefinitionRank::ExactQualified),
                Just(DefinitionRank::SamePackage),
                Just(DefinitionRank::ExplicitImport),
                Just(DefinitionRank::DefaultExport),
                Just(DefinitionRank::WorkspaceCandidate),
                Just(DefinitionRank::Heuristic),
            ]
        }

        /// Strategy to generate a random file URI from a small pool so that
        /// same-rank tie-breaking by URI is exercised.
        fn arb_file_uri() -> impl Strategy<Value = String> {
            prop_oneof![
                Just("file:///lib/Alpha.pm".to_string()),
                Just("file:///lib/Beta.pm".to_string()),
                Just("file:///lib/Gamma.pm".to_string()),
                Just("file:///lib/Delta.pm".to_string()),
            ]
        }

        /// Strategy to generate a random byte position for an anchor.
        fn arb_span_start() -> impl Strategy<Value = u32> {
            0u32..10_000u32
        }

        /// A generated candidate descriptor before it is turned into real
        /// fact-shard data.
        #[derive(Debug, Clone)]
        struct CandidateSpec {
            rank: DefinitionRank,
            file_uri: String,
            span_start: u32,
        }

        fn arb_candidate_spec() -> impl Strategy<Value = CandidateSpec> {
            (arb_definition_rank(), arb_file_uri(), arb_span_start()).prop_map(
                |(rank, file_uri, span_start)| CandidateSpec { rank, file_uri, span_start },
            )
        }

        /// Build fact shards and candidates from a list of specs.
        ///
        /// Each spec becomes one entity + anchor in the appropriate shard,
        /// and one `DefinitionCandidate` in the returned list.
        fn build_test_data(
            specs: &[CandidateSpec],
        ) -> (HashMap<String, FileFactShard>, Vec<DefinitionCandidate>) {
            // Group specs by file URI so we build one shard per URI.
            let mut uri_to_file_id: HashMap<String, FileId> = HashMap::new();
            let mut next_file_id = 1u64;
            let mut shard_map: HashMap<String, FileFactShard> = HashMap::new();

            let mut candidates = Vec::with_capacity(specs.len());

            for (idx, spec) in specs.iter().enumerate() {
                let file_id = *uri_to_file_id.entry(spec.file_uri.clone()).or_insert_with(|| {
                    let id = FileId(next_file_id);
                    next_file_id += 1;
                    id
                });

                let anchor_id = AnchorId(idx as u64 + 1);
                let entity_id = EntityId(idx as u64 + 1);

                let anchor = AnchorFact {
                    id: anchor_id,
                    file_id,
                    span_start_byte: spec.span_start,
                    span_end_byte: spec.span_start.saturating_add(10),
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                };

                let entity = EntityFact {
                    id: entity_id,
                    kind: EntityKind::Subroutine,
                    canonical_name: format!("Pkg{}::sub_{}", idx, idx),
                    anchor_id: Some(anchor_id),
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                };

                let shard =
                    shard_map.entry(spec.file_uri.clone()).or_insert_with(|| FileFactShard {
                        source_uri: spec.file_uri.clone(),
                        file_id,
                        content_hash: 0,
                        anchors_hash: None,
                        entities_hash: None,
                        occurrences_hash: None,
                        edges_hash: None,
                        anchors: Vec::new(),
                        entities: Vec::new(),
                        occurrences: Vec::new(),
                        edges: Vec::new(),
                    });

                shard.anchors.push(anchor);
                shard.entities.push(entity);

                let rank_reason = match spec.rank {
                    DefinitionRank::ExactQualified => DefinitionRankReason::ExactQualifiedName,
                    DefinitionRank::SamePackage => DefinitionRankReason::SamePackage,
                    DefinitionRank::ExplicitImport => {
                        DefinitionRankReason::ExplicitImport { module: String::new() }
                    }
                    DefinitionRank::DefaultExport => {
                        DefinitionRankReason::DefaultExport { module: String::new() }
                    }
                    DefinitionRank::WorkspaceCandidate => DefinitionRankReason::WorkspaceSymbol,
                    DefinitionRank::Heuristic => DefinitionRankReason::HeuristicNameMatch,
                    _ => DefinitionRankReason::HeuristicNameMatch,
                };

                candidates.push(DefinitionCandidate::new(
                    entity_id,
                    anchor_id,
                    format!("Pkg{}::sub_{}", idx, idx),
                    format!("sub_{}", idx),
                    Some(format!("Pkg{}", idx)),
                    EntityKind::Subroutine,
                    Provenance::ExactAst,
                    Confidence::High,
                    spec.rank,
                    rank_reason,
                ));
            }

            (shard_map, candidates)
        }

        // **Validates: Requirements 5.4, 5.5**
        //
        // Property 7: Definition Candidate Sorting Invariant — Returned
        // candidates are sorted by DefinitionRank (ExactQualified first,
        // Heuristic last), and within same rank, sorted deterministically
        // by file URI then source position.
        proptest! {
            #![proptest_config(ProptestConfig {
                failure_persistence: None,
                ..ProptestConfig::default()
            })]

            #[test]
            fn prop_definition_candidate_sorting_invariant(
                specs in prop::collection::vec(arb_candidate_spec(), 0..30),
            ) {
                let (shard_map, mut candidates) = build_test_data(&specs);

                let ref_index = ReferenceIndex::new();
                let ie_index = ImportExportIndex::new();
                let queries = WorkspaceSemanticQueries::new(
                    &ref_index,
                    &ie_index,
                    &shard_map,
                );

                queries.sort_candidates(&mut candidates);

                // Verify the sorting invariant over every consecutive pair.
                for pair in candidates.windows(2) {
                    let a = &pair[0];
                    let b = &pair[1];

                    // Primary: rank must be non-decreasing.
                    prop_assert!(
                        a.rank <= b.rank,
                        "rank ordering violated: {:?} should come before {:?}",
                        a.rank,
                        b.rank,
                    );

                    // Secondary: within the same rank, tie-break by
                    // (source_uri, span_start_byte).
                    if a.rank == b.rank {
                        let a_loc = queries.anchor_location(a.anchor_id);
                        let b_loc = queries.anchor_location(b.anchor_id);
                        prop_assert!(
                            a_loc <= b_loc,
                            "same-rank tie-break violated: ({:?}) should come before ({:?})",
                            a_loc,
                            b_loc,
                        );
                    }
                }
            }
        }

        // ── Property 13 helpers ──

        /// Descriptor for a generated rename-plan scenario.
        #[derive(Debug, Clone)]
        struct RenamePlanScenario {
            /// Whether the target entity has a DynamicBoundary occurrence in
            /// its shard.
            has_dynamic_boundary: bool,
            /// Whether the target entity is exported and cross-module
            /// referenced (imported by another file).
            has_cross_module_export: bool,
            /// Number of normal (non-dynamic) reference occurrences to
            /// include alongside the target entity.
            normal_ref_count: usize,
            /// Bare symbol name used for the entity.
            bare_name: String,
        }

        /// Strategy to generate a valid Perl-like bare symbol name.
        fn arb_bare_symbol() -> impl Strategy<Value = String> {
            "[a-z][a-z0-9_]{0,12}".prop_filter("non-empty", |s| !s.is_empty())
        }

        fn arb_rename_plan_scenario() -> impl Strategy<Value = RenamePlanScenario> {
            (any::<bool>(), any::<bool>(), 0usize..5, arb_bare_symbol()).prop_map(
                |(has_dynamic_boundary, has_cross_module_export, normal_ref_count, bare_name)| {
                    RenamePlanScenario {
                        has_dynamic_boundary,
                        has_cross_module_export,
                        normal_ref_count,
                        bare_name,
                    }
                },
            )
        }

        /// Build the full test fixture (shards, reference index,
        /// import/export index) from a `RenamePlanScenario`.
        ///
        /// Returns the entity id of the target so the caller can invoke
        /// `rename_plan`.
        fn build_rename_scenario(
            scenario: &RenamePlanScenario,
        ) -> (EntityId, HashMap<String, FileFactShard>, ReferenceIndex, ImportExportIndex) {
            use perl_semantic_facts::{ExportSet, ImportKind, ImportSpec, ImportSymbols};

            let file_def = FileId(1);
            let entity_id = EntityId(100);
            let anchor_def = AnchorId(10);

            let module_name = format!("Pkg::{}", scenario.bare_name);

            // ── Build anchors and occurrences for the definition shard ──
            let mut anchors = vec![AnchorFact {
                id: anchor_def,
                file_id: file_def,
                span_start_byte: 0,
                span_end_byte: 10,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }];

            let mut occurrences = vec![OccurrenceFact {
                id: OccurrenceId(200),
                kind: OccurrenceKind::Definition,
                entity_id: Some(entity_id),
                anchor_id: anchor_def,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }];

            let mut next_anchor = 20u64;
            let mut next_occ = 201u64;

            // Add normal reference occurrences.
            for i in 0..scenario.normal_ref_count {
                let aid = AnchorId(next_anchor);
                next_anchor += 1;
                let oid = OccurrenceId(next_occ);
                next_occ += 1;

                let start = 100 + (i as u32) * 20;
                anchors.push(AnchorFact {
                    id: aid,
                    file_id: file_def,
                    span_start_byte: start,
                    span_end_byte: start + 10,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                });
                occurrences.push(OccurrenceFact {
                    id: oid,
                    kind: OccurrenceKind::Call,
                    entity_id: Some(entity_id),
                    anchor_id: aid,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                });
            }

            // Optionally add a DynamicBoundary occurrence.
            if scenario.has_dynamic_boundary {
                let aid = AnchorId(next_anchor);
                next_anchor += 1;
                let oid = OccurrenceId(next_occ);
                #[allow(unused_assignments)] // next_occ not read after this
                {
                    next_occ += 1;
                }

                let start = 500u32;
                anchors.push(AnchorFact {
                    id: aid,
                    file_id: file_def,
                    span_start_byte: start,
                    span_end_byte: start + 10,
                    scope_id: None,
                    provenance: Provenance::DynamicBoundary,
                    confidence: Confidence::Low,
                });
                occurrences.push(OccurrenceFact {
                    id: oid,
                    kind: OccurrenceKind::DynamicBoundary,
                    entity_id: Some(entity_id),
                    anchor_id: aid,
                    scope_id: None,
                    provenance: Provenance::DynamicBoundary,
                    confidence: Confidence::Low,
                });
            }

            let _ = next_anchor; // suppress unused warning

            let entities = vec![EntityFact {
                id: entity_id,
                kind: EntityKind::Subroutine,
                canonical_name: module_name.clone(),
                anchor_id: Some(anchor_def),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }];

            let shard_def = FileFactShard {
                source_uri: "file:///lib/Pkg.pm".to_string(),
                file_id: file_def,
                content_hash: 0,
                anchors_hash: None,
                entities_hash: None,
                occurrences_hash: None,
                edges_hash: None,
                anchors,
                entities,
                occurrences,
                edges: Vec::new(),
            };

            let mut shards = HashMap::new();
            shards.insert(shard_def.source_uri.clone(), shard_def);

            let ref_index = ReferenceIndex::new();
            let mut ie_index = ImportExportIndex::new();

            // Optionally set up cross-module export + import from another
            // file.
            if scenario.has_cross_module_export {
                let file_importer = FileId(2);

                let shard_importer = FileFactShard {
                    source_uri: "file:///lib/Consumer.pm".to_string(),
                    file_id: file_importer,
                    content_hash: 0,
                    anchors_hash: None,
                    entities_hash: None,
                    occurrences_hash: None,
                    edges_hash: None,
                    anchors: Vec::new(),
                    entities: Vec::new(),
                    occurrences: Vec::new(),
                    edges: Vec::new(),
                };
                shards.insert(shard_importer.source_uri.clone(), shard_importer);

                ie_index.add_module_exports(
                    "file:///lib/Pkg.pm",
                    "Pkg",
                    ExportSet {
                        default_exports: vec![scenario.bare_name.clone()],
                        optional_exports: vec![],
                        tags: vec![],
                        provenance: Provenance::ExactAst,
                        confidence: Confidence::High,
                        module_name: Some("Pkg".to_string()),
                        anchor_id: None,
                    },
                );

                ie_index.add_file_imports(
                    "file:///lib/Consumer.pm",
                    file_importer,
                    vec![ImportSpec {
                        module: "Pkg".to_string(),
                        kind: ImportKind::UseExplicitList,
                        symbols: ImportSymbols::Explicit(vec![scenario.bare_name.clone()]),
                        provenance: Provenance::ExactAst,
                        confidence: Confidence::High,
                        file_id: Some(file_importer),
                        anchor_id: None,
                        scope_id: None,
                    }],
                );
            }

            (entity_id, shards, ref_index, ie_index)
        }

        // **Validates: Requirements 16.2, 16.3**
        //
        // Property 13: Rename Plan Safety — Dynamic and Export Blockers
        //
        // Any rename plan where the target entity has references crossing a
        // dynamic boundary SHALL contain a PlanBlocker with reason
        // DynamicBoundary.
        //
        // Any rename plan where the target entity is exported and
        // cross-module referenced SHALL contain a PlanBlocker with reason
        // CrossModuleExport.
        proptest! {
            #![proptest_config(ProptestConfig {
                failure_persistence: None,
                ..ProptestConfig::default()
            })]

            #[test]
            fn prop_rename_plan_dynamic_boundary_blocker(
                scenario in arb_rename_plan_scenario().prop_filter(
                    "needs dynamic boundary",
                    |s| s.has_dynamic_boundary,
                ),
            ) {
                let (entity_id, shards, ref_index, ie_index) =
                    build_rename_scenario(&scenario);
                let queries = WorkspaceSemanticQueries::new(
                    &ref_index,
                    &ie_index,
                    &shards,
                );

                let plan = queries.rename_plan(entity_id, "new_name");

                let has_dyn_blocker = plan
                    .blockers
                    .iter()
                    .any(|b| b.reason == PlanBlockerReason::DynamicBoundary);

                prop_assert!(
                    has_dyn_blocker,
                    "rename plan for entity with dynamic boundary reference \
                     must contain a DynamicBoundary blocker, but blockers were: {:?}",
                    plan.blockers,
                );
            }

            #[test]
            fn prop_rename_plan_cross_module_export_blocker(
                scenario in arb_rename_plan_scenario().prop_filter(
                    "needs cross-module export",
                    |s| s.has_cross_module_export,
                ),
            ) {
                let (entity_id, shards, ref_index, ie_index) =
                    build_rename_scenario(&scenario);
                let queries = WorkspaceSemanticQueries::new(
                    &ref_index,
                    &ie_index,
                    &shards,
                );

                let plan = queries.rename_plan(entity_id, "new_name");

                let has_export_blocker = plan
                    .blockers
                    .iter()
                    .any(|b| b.reason == PlanBlockerReason::CrossModuleExport);

                prop_assert!(
                    has_export_blocker,
                    "rename plan for exported + cross-module-referenced entity \
                     must contain a CrossModuleExport blocker, but blockers were: {:?}",
                    plan.blockers,
                );
            }
        }

        // ── Property 14: Rename Plan Occurrence Classification ──

        /// An occurrence kind that the rename plan can classify into a
        /// `PlannedEditCategory`.  We exclude `DynamicBoundary`,
        /// `Inheritance`, and `RoleComposition` because those are handled
        /// as blockers/warnings rather than edits.
        #[derive(Debug, Clone, Copy)]
        struct ClassifiableOccurrence {
            kind: OccurrenceKind,
            expected_category: PlannedEditCategory,
        }

        /// Strategy to generate a classifiable occurrence kind together
        /// with its expected `PlannedEditCategory`.
        fn arb_classifiable_occurrence() -> impl Strategy<Value = ClassifiableOccurrence> {
            prop_oneof![
                Just(ClassifiableOccurrence {
                    kind: OccurrenceKind::Definition,
                    expected_category: PlannedEditCategory::Definition,
                }),
                Just(ClassifiableOccurrence {
                    kind: OccurrenceKind::Import,
                    expected_category: PlannedEditCategory::ImportList,
                }),
                Just(ClassifiableOccurrence {
                    kind: OccurrenceKind::Export,
                    expected_category: PlannedEditCategory::ExportList,
                }),
                Just(ClassifiableOccurrence {
                    kind: OccurrenceKind::Reference,
                    expected_category: PlannedEditCategory::Reference,
                }),
                Just(ClassifiableOccurrence {
                    kind: OccurrenceKind::Read,
                    expected_category: PlannedEditCategory::Reference,
                }),
                Just(ClassifiableOccurrence {
                    kind: OccurrenceKind::Write,
                    expected_category: PlannedEditCategory::Reference,
                }),
                Just(ClassifiableOccurrence {
                    kind: OccurrenceKind::Call,
                    expected_category: PlannedEditCategory::Reference,
                }),
                Just(ClassifiableOccurrence {
                    kind: OccurrenceKind::MethodCall,
                    expected_category: PlannedEditCategory::Reference,
                }),
                Just(ClassifiableOccurrence {
                    kind: OccurrenceKind::StaticMethodCall,
                    expected_category: PlannedEditCategory::Reference,
                }),
                Just(ClassifiableOccurrence {
                    kind: OccurrenceKind::GeneratedUse,
                    expected_category: PlannedEditCategory::Reference,
                }),
            ]
        }

        /// Scenario for Property 14: a rename plan with a mix of
        /// classifiable occurrence kinds.
        #[derive(Debug, Clone)]
        struct OccurrenceClassificationScenario {
            /// The classifiable occurrences to include in the shard.
            occurrences: Vec<ClassifiableOccurrence>,
            /// Bare symbol name for the entity.
            bare_name: String,
        }

        fn arb_occurrence_classification_scenario()
        -> impl Strategy<Value = OccurrenceClassificationScenario> {
            (proptest::collection::vec(arb_classifiable_occurrence(), 1..8), arb_bare_symbol())
                .prop_map(|(occurrences, bare_name)| OccurrenceClassificationScenario {
                    occurrences,
                    bare_name,
                })
        }

        /// Build the test fixture for an `OccurrenceClassificationScenario`.
        ///
        /// Returns the entity id and the expected category for each
        /// occurrence (in shard order) so the caller can verify the plan.
        fn build_classification_scenario(
            scenario: &OccurrenceClassificationScenario,
        ) -> (
            EntityId,
            HashMap<String, FileFactShard>,
            ReferenceIndex,
            ImportExportIndex,
            Vec<(AnchorId, PlannedEditCategory)>,
        ) {
            let file_id = FileId(1);
            let entity_id = EntityId(100);
            let anchor_def = AnchorId(10);

            let module_name = format!("Pkg::{}", scenario.bare_name);

            // Always include a definition anchor for the entity itself.
            let mut anchors = vec![AnchorFact {
                id: anchor_def,
                file_id,
                span_start_byte: 0,
                span_end_byte: 10,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }];

            let mut occurrences_vec = Vec::new();
            let mut expected: Vec<(AnchorId, PlannedEditCategory)> = Vec::new();

            let mut next_anchor = 20u64;
            let mut next_occ = 200u64;

            for (i, co) in scenario.occurrences.iter().enumerate() {
                let aid = AnchorId(next_anchor);
                next_anchor += 1;
                let oid = OccurrenceId(next_occ);
                next_occ += 1;

                let start = 100 + (i as u32) * 20;
                anchors.push(AnchorFact {
                    id: aid,
                    file_id,
                    span_start_byte: start,
                    span_end_byte: start + 10,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                });
                occurrences_vec.push(OccurrenceFact {
                    id: oid,
                    kind: co.kind,
                    entity_id: Some(entity_id),
                    anchor_id: aid,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                });
                expected.push((aid, co.expected_category));
            }

            let entities = vec![EntityFact {
                id: entity_id,
                kind: EntityKind::Subroutine,
                canonical_name: module_name,
                anchor_id: Some(anchor_def),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }];

            let shard = FileFactShard {
                source_uri: "file:///lib/Pkg.pm".to_string(),
                file_id,
                content_hash: 0,
                anchors_hash: None,
                entities_hash: None,
                occurrences_hash: None,
                edges_hash: None,
                anchors,
                entities,
                occurrences: occurrences_vec,
                edges: Vec::new(),
            };

            let mut shards = HashMap::new();
            shards.insert(shard.source_uri.clone(), shard);

            let ref_index = ReferenceIndex::new();
            let ie_index = ImportExportIndex::new();

            (entity_id, shards, ref_index, ie_index, expected)
        }

        // **Validates: Requirements 16.6**
        //
        // Property 14: Rename Plan Occurrence Classification
        //
        // Import occurrences SHALL be classified as ImportList, export
        // occurrences as ExportList, definition occurrences as Definition,
        // and reference occurrences as Reference.  No occurrence SHALL be
        // left unclassified.
        proptest! {
            #![proptest_config(ProptestConfig {
                failure_persistence: None,
                ..ProptestConfig::default()
            })]

            #[test]
            fn prop_rename_plan_occurrence_classification(
                scenario in arb_occurrence_classification_scenario(),
            ) {
                let (entity_id, shards, ref_index, ie_index, expected) =
                    build_classification_scenario(&scenario);
                let queries = WorkspaceSemanticQueries::new(
                    &ref_index,
                    &ie_index,
                    &shards,
                );

                let plan = queries.rename_plan(entity_id, "new_name");

                // No occurrence should be left unclassified.
                let unclassified_blockers: Vec<_> = plan
                    .blockers
                    .iter()
                    .filter(|b| b.reason == PlanBlockerReason::UnclassifiedOccurrence)
                    .collect();
                prop_assert!(
                    unclassified_blockers.is_empty(),
                    "no occurrence should be left unclassified, but found: {:?}",
                    unclassified_blockers,
                );

                // Every expected occurrence should appear in the plan edits
                // with the correct category.
                for (anchor_id, expected_cat) in &expected {
                    let matching_edit = plan
                        .edits
                        .iter()
                        .find(|e| e.anchor_id == *anchor_id);
                    prop_assert!(
                        matching_edit.is_some(),
                        "expected an edit for anchor {:?} with category {:?}, \
                         but no matching edit found in plan. edits: {:?}",
                        anchor_id,
                        expected_cat,
                        plan.edits,
                    );
                    let edit = matching_edit.expect("checked above");
                    prop_assert_eq!(
                        edit.category,
                        *expected_cat,
                        "edit for anchor {:?} should have category {:?} but had {:?}",
                        anchor_id,
                        expected_cat,
                        edit.category,
                    );
                }
            }
        }
    }
}

/// Latency benchmark tests for semantic queries on a synthetic 1000-file workspace.
///
/// These tests measure p95 latency of each `SemanticQueries` method against
/// the target thresholds from Requirement 19:
///
/// - `symbol_at`: 5ms p95 (Req 19.1)
/// - `definitions`: 10ms p95 (Req 19.2)
/// - `references`: 20ms p95 (Req 19.3)
/// - `visible_symbols_at`: 15ms p95 (Req 19.4)
///
/// The benchmarks also wire latency measurements into scorecard reporting
/// (Req 19.5, 11.7) and flag any threshold violations.
#[cfg(test)]
mod latency_benchmarks {
    use super::*;
    use crate::semantic::imports::ImportExportIndex;
    use crate::semantic::references::ReferenceIndex;
    use crate::semantic::scorecard::{
        LatencyThresholds, Scorecard, ScorecardMode, build_latency_measurement,
    };
    use crate::workspace::workspace_index::FileFactShard;
    use perl_semantic_facts::{
        AnchorFact, AnchorId, Confidence, EdgeFact, EdgeId, EdgeKind, EntityFact, EntityId,
        EntityKind, OccurrenceFact, OccurrenceId, OccurrenceKind, Provenance,
    };
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    /// Number of synthetic files in the benchmark workspace.
    const FILE_COUNT: usize = 1000;

    /// Number of latency samples to collect per query method.
    const SAMPLE_COUNT: usize = 100;

    /// Generate a synthetic workspace with `FILE_COUNT` files, each containing
    /// entities, occurrences, anchors, and edges. Returns the fact shards map,
    /// a populated reference index, and an import/export index.
    fn build_synthetic_workspace()
    -> (HashMap<String, FileFactShard>, ReferenceIndex, ImportExportIndex) {
        let mut shards = HashMap::new();
        let mut ref_index = ReferenceIndex::new();
        let ie_index = ImportExportIndex::new();

        for i in 0..FILE_COUNT {
            let file_id = FileId(i as u64);
            let uri = format!("file:///lib/Gen/Module{}.pm", i);

            // Each file has 5 entities with anchors, occurrences, and edges.
            let entity_base = (i as u64) * 100;
            let anchor_base = (i as u64) * 100;
            let occ_base = (i as u64) * 100;
            let edge_base = (i as u64) * 100;

            let mut anchors = Vec::new();
            let mut entities = Vec::new();
            let mut occurrences = Vec::new();
            let mut edges = Vec::new();

            for j in 0..5u64 {
                let entity_id = EntityId(entity_base + j);
                let anchor_def = AnchorId(anchor_base + j * 2);
                let anchor_ref = AnchorId(anchor_base + j * 2 + 1);

                let start_def = (j as u32) * 100;
                let start_ref = (j as u32) * 100 + 50;

                anchors.push(AnchorFact {
                    id: anchor_def,
                    file_id,
                    span_start_byte: start_def,
                    span_end_byte: start_def + 20,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                });
                anchors.push(AnchorFact {
                    id: anchor_ref,
                    file_id,
                    span_start_byte: start_ref,
                    span_end_byte: start_ref + 15,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                });

                let canonical_name = format!("Gen::Module{}::method_{}", i, j);
                entities.push(EntityFact {
                    id: entity_id,
                    kind: EntityKind::Subroutine,
                    canonical_name: canonical_name.clone(),
                    anchor_id: Some(anchor_def),
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                });

                occurrences.push(OccurrenceFact {
                    id: OccurrenceId(occ_base + j * 2),
                    kind: OccurrenceKind::Definition,
                    entity_id: Some(entity_id),
                    anchor_id: anchor_def,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                });
                occurrences.push(OccurrenceFact {
                    id: OccurrenceId(occ_base + j * 2 + 1),
                    kind: OccurrenceKind::Call,
                    entity_id: Some(entity_id),
                    anchor_id: anchor_ref,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                });

                edges.push(EdgeFact {
                    id: EdgeId(edge_base + j),
                    kind: EdgeKind::References,
                    from_entity_id: EntityId(0),
                    to_entity_id: entity_id,
                    via_occurrence_id: Some(OccurrenceId(occ_base + j * 2 + 1)),
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                });
            }

            let shard = FileFactShard {
                source_uri: uri.clone(),
                file_id,
                content_hash: i as u64,
                anchors_hash: None,
                entities_hash: None,
                occurrences_hash: None,
                edges_hash: None,
                anchors,
                entities,
                occurrences,
                edges,
            };

            // Populate the reference index from the shard.
            ref_index.add_file(&shard);

            shards.insert(uri, shard);
        }

        (shards, ref_index, ie_index)
    }

    /// Measure latency of `symbol_at` across `SAMPLE_COUNT` iterations.
    fn measure_symbol_at(queries: &WorkspaceSemanticQueries<'_>) -> Vec<Duration> {
        let file_id = FileId(500);
        let byte_offset = 10; // Within the first anchor's span (0..20).
        let mut samples = Vec::with_capacity(SAMPLE_COUNT);
        for _ in 0..SAMPLE_COUNT {
            let start = Instant::now();
            let _ = std::hint::black_box(queries.symbol_at(file_id, byte_offset));
            samples.push(start.elapsed());
        }
        samples
    }

    /// Measure latency of `definitions` across `SAMPLE_COUNT` iterations.
    fn measure_definitions(queries: &WorkspaceSemanticQueries<'_>) -> Vec<Duration> {
        let ctx = QueryContext::new(FileId(500), None, Some(10));
        let mut samples = Vec::with_capacity(SAMPLE_COUNT);
        for _ in 0..SAMPLE_COUNT {
            let start = Instant::now();
            let _ = std::hint::black_box(queries.definitions("Gen::Module500::method_2", &ctx));
            samples.push(start.elapsed());
        }
        samples
    }

    /// Measure latency of `references` across `SAMPLE_COUNT` iterations.
    fn measure_references(queries: &WorkspaceSemanticQueries<'_>) -> Vec<Duration> {
        let entity_id = EntityId(500 * 100 + 2); // Module500, method_2
        let mut samples = Vec::with_capacity(SAMPLE_COUNT);
        for _ in 0..SAMPLE_COUNT {
            let start = Instant::now();
            let _ = std::hint::black_box(queries.references(entity_id));
            samples.push(start.elapsed());
        }
        samples
    }

    /// Measure latency of `visible_symbols_at` across `SAMPLE_COUNT` iterations.
    fn measure_visible_symbols_at(queries: &WorkspaceSemanticQueries<'_>) -> Vec<Duration> {
        let file_id = FileId(500);
        let byte_offset = 10;
        let mut samples = Vec::with_capacity(SAMPLE_COUNT);
        for _ in 0..SAMPLE_COUNT {
            let start = Instant::now();
            let _ = std::hint::black_box(queries.visible_symbols_at(file_id, byte_offset, None));
            samples.push(start.elapsed());
        }
        samples
    }

    // ── Benchmark tests ──

    #[test]
    fn benchmark_symbol_at_latency() -> Result<(), Box<dyn std::error::Error>> {
        let (shards, ref_index, ie_index) = build_synthetic_workspace();
        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

        let mut samples = measure_symbol_at(&queries);
        let measurement = build_latency_measurement(
            "symbol_at",
            &mut samples,
            LatencyThresholds::SYMBOL_AT_MICROS,
        );

        // Log the measurement for visibility.
        eprintln!(
            "symbol_at: p95={} µs, threshold={} µs, exceeded={}",
            measurement.p95_micros, measurement.threshold_micros, measurement.exceeded
        );

        // The test verifies the measurement was collected, not that it passes
        // the threshold (CI environments vary). Threshold violations are
        // flagged in the scorecard report.
        assert_eq!(measurement.sample_count, SAMPLE_COUNT);
        assert_eq!(measurement.query_name, "symbol_at");
        Ok(())
    }

    #[test]
    fn benchmark_definitions_latency() -> Result<(), Box<dyn std::error::Error>> {
        let (shards, ref_index, ie_index) = build_synthetic_workspace();
        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

        let mut samples = measure_definitions(&queries);
        let measurement = build_latency_measurement(
            "definitions",
            &mut samples,
            LatencyThresholds::DEFINITIONS_MICROS,
        );

        eprintln!(
            "definitions: p95={} µs, threshold={} µs, exceeded={}",
            measurement.p95_micros, measurement.threshold_micros, measurement.exceeded
        );

        assert_eq!(measurement.sample_count, SAMPLE_COUNT);
        assert_eq!(measurement.query_name, "definitions");
        Ok(())
    }

    #[test]
    fn benchmark_references_latency() -> Result<(), Box<dyn std::error::Error>> {
        let (shards, ref_index, ie_index) = build_synthetic_workspace();
        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

        let mut samples = measure_references(&queries);
        let measurement = build_latency_measurement(
            "references",
            &mut samples,
            LatencyThresholds::REFERENCES_MICROS,
        );

        eprintln!(
            "references: p95={} µs, threshold={} µs, exceeded={}",
            measurement.p95_micros, measurement.threshold_micros, measurement.exceeded
        );

        assert_eq!(measurement.sample_count, SAMPLE_COUNT);
        assert_eq!(measurement.query_name, "references");
        Ok(())
    }

    #[test]
    fn benchmark_visible_symbols_at_latency() -> Result<(), Box<dyn std::error::Error>> {
        let (shards, ref_index, ie_index) = build_synthetic_workspace();
        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

        let mut samples = measure_visible_symbols_at(&queries);
        let measurement = build_latency_measurement(
            "visible_symbols_at",
            &mut samples,
            LatencyThresholds::VISIBLE_SYMBOLS_AT_MICROS,
        );

        eprintln!(
            "visible_symbols_at: p95={} µs, threshold={} µs, exceeded={}",
            measurement.p95_micros, measurement.threshold_micros, measurement.exceeded
        );

        assert_eq!(measurement.sample_count, SAMPLE_COUNT);
        assert_eq!(measurement.query_name, "visible_symbols_at");
        Ok(())
    }

    /// End-to-end test: build a 1000-file workspace, measure all four query
    /// latencies, wire them into a scorecard, and verify the report contains
    /// latency data with threshold violation flags.
    ///
    /// Validates: Requirements 19.1, 19.2, 19.3, 19.4, 19.5, 11.7
    #[test]
    fn scorecard_latency_integration() -> Result<(), Box<dyn std::error::Error>> {
        let (shards, ref_index, ie_index) = build_synthetic_workspace();
        let queries = WorkspaceSemanticQueries::new(&ref_index, &ie_index, &shards);

        // Collect latency samples for all four query methods.
        let mut symbol_at_samples = measure_symbol_at(&queries);
        let mut definitions_samples = measure_definitions(&queries);
        let mut references_samples = measure_references(&queries);
        let mut visible_symbols_samples = measure_visible_symbols_at(&queries);

        // Build latency measurements.
        let measurements = vec![
            build_latency_measurement(
                "symbol_at",
                &mut symbol_at_samples,
                LatencyThresholds::SYMBOL_AT_MICROS,
            ),
            build_latency_measurement(
                "definitions",
                &mut definitions_samples,
                LatencyThresholds::DEFINITIONS_MICROS,
            ),
            build_latency_measurement(
                "references",
                &mut references_samples,
                LatencyThresholds::REFERENCES_MICROS,
            ),
            build_latency_measurement(
                "visible_symbols_at",
                &mut visible_symbols_samples,
                LatencyThresholds::VISIBLE_SYMBOLS_AT_MICROS,
            ),
        ];

        // Wire into scorecard.
        let mut scorecard = Scorecard::new(ScorecardMode::Check);
        scorecard.add_latencies(measurements);

        let report = scorecard.report();

        // Verify all four measurements are present.
        assert_eq!(report.latency.len(), 4);
        assert!(report.latency.contains_key("symbol_at"));
        assert!(report.latency.contains_key("definitions"));
        assert!(report.latency.contains_key("references"));
        assert!(report.latency.contains_key("visible_symbols_at"));

        // Verify each measurement has the correct sample count and threshold.
        for (name, m) in &report.latency {
            assert_eq!(m.sample_count, SAMPLE_COUNT, "sample count for {}", name);
            let expected_threshold = LatencyThresholds::for_query(name)
                .ok_or_else(|| format!("unknown query: {}", name))?;
            assert_eq!(m.threshold_micros, expected_threshold, "threshold for {}", name);
        }

        // Verify violations are flagged correctly.
        for violation in &report.latency_violations {
            let m = report
                .latency
                .get(&violation.query_name)
                .ok_or_else(|| format!("violation for unknown query: {}", violation.query_name))?;
            assert!(m.exceeded, "violation query {} should be exceeded", violation.query_name);
            assert_eq!(violation.p95_micros, m.p95_micros);
            assert_eq!(violation.threshold_micros, m.threshold_micros);
        }

        // Log summary for visibility.
        eprintln!("=== Scorecard Latency Report ===");
        for (name, m) in &report.latency {
            eprintln!(
                "  {}: p95={} µs (threshold={} µs) {}",
                name,
                m.p95_micros,
                m.threshold_micros,
                if m.exceeded { "⚠ EXCEEDED" } else { "✓" }
            );
        }
        if report.latency_violations.is_empty() {
            eprintln!("  No threshold violations.");
        } else {
            eprintln!("  {} threshold violation(s) flagged.", report.latency_violations.len());
        }

        Ok(())
    }
}
