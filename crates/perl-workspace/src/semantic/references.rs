//! Typed reference index for cross-file reference lookups.
//!
//! Maintains two indexes over [`ReferenceEdge`] entries:
//!
//! - `references_by_name` — keyed by the bare or qualified symbol key, for
//!   name-based lookups (e.g. find-references by symbol name).
//! - `references_by_entity` — keyed by [`EntityId`], for entity-based lookups
//!   (e.g. find all references to a specific declaration).
//!
//! Both indexes support incremental add/remove via [`ReferenceIndex::add_file`]
//! and [`ReferenceIndex::remove_file`], keyed by the file's source URI.

use perl_semantic_facts::{EdgeKind, EntityId, FileId, OccurrenceKind, ReferenceEdge};
use std::collections::HashMap;

use crate::workspace::workspace_index::FileFactShard;

/// Cross-file reference index backed by two `HashMap`s.
///
/// Populated from [`FileFactShard`] occurrences and edges during workspace
/// indexing. Supports incremental updates: call [`remove_file`](Self::remove_file)
/// to purge stale entries, then [`add_file`](Self::add_file) to insert fresh ones.
#[derive(Debug, Default)]
pub struct ReferenceIndex {
    /// Symbol-key → reference edges. The key is the bare or qualified name
    /// carried on each [`ReferenceEdge::symbol_key`].
    references_by_name: HashMap<String, Vec<ReferenceEdge>>,

    /// Entity → reference edges. One entry per target candidate in each
    /// [`ReferenceEdge::target_candidates`].
    references_by_entity: HashMap<EntityId, Vec<ReferenceEdge>>,

    /// Tracks which file URIs have been indexed so that [`remove_file`](Self::remove_file)
    /// can efficiently purge stale entries.
    indexed_files: HashMap<String, FileId>,
}

impl ReferenceIndex {
    /// Create an empty reference index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Index all reference-like occurrences from a [`FileFactShard`].
    ///
    /// For each non-definition occurrence in the shard, a [`ReferenceEdge`] is
    /// synthesized and inserted into both lookup maps. Edge facts with kind
    /// [`EdgeKind::References`] are consulted to populate `target_candidates`.
    pub fn add_file(&mut self, shard: &FileFactShard) {
        // Record the file so remove_file can match by URI later.
        self.indexed_files.insert(shard.source_uri.clone(), shard.file_id);

        // Build a quick lookup: occurrence_id → list of target entity IDs from
        // Reference edges in the shard.
        let mut edge_targets: HashMap<u64, Vec<EntityId>> = HashMap::new();
        for edge in &shard.edges {
            if edge.kind == EdgeKind::References {
                if let Some(occ_id) = edge.via_occurrence_id {
                    edge_targets.entry(occ_id.0).or_default().push(edge.to_entity_id);
                }
            }
        }

        for occ in &shard.occurrences {
            // Skip definition occurrences — they are not references.
            if occ.kind == OccurrenceKind::Definition {
                continue;
            }

            // Build the target_candidates list from edges, falling back to the
            // occurrence's own entity_id when no edge exists.
            let target_candidates = match edge_targets.get(&occ.id.0) {
                Some(targets) => targets.clone(),
                None => occ.entity_id.into_iter().collect(),
            };

            // Derive the symbol_key from the entity canonical name when
            // available. For occurrences without a resolved entity we use the
            // anchor id as a placeholder key — callers that need name-based
            // lookup will not match these, but entity-based lookup still works.
            let symbol_key = self.derive_symbol_key(shard, occ);

            let ref_edge = ReferenceEdge::new(
                occ.id,
                occ.anchor_id,
                shard.file_id,
                symbol_key.clone(),
                target_candidates.clone(),
                occ.kind,
                occ.provenance,
                occ.confidence,
            );

            // Insert into name index.
            self.references_by_name.entry(symbol_key).or_default().push(ref_edge.clone());

            // Insert into entity index — one entry per target candidate.
            for entity_id in &target_candidates {
                self.references_by_entity.entry(*entity_id).or_default().push(ref_edge.clone());
            }
        }
    }

    /// Remove all reference entries that originated from the given file URI.
    ///
    /// This is the "remove" half of incremental re-indexing: call this before
    /// [`add_file`](Self::add_file) with the updated shard.
    pub fn remove_file(&mut self, source_uri: &str) {
        let file_id = match self.indexed_files.remove(source_uri) {
            Some(id) => id,
            None => return,
        };

        // Retain only entries from other files.
        for refs in self.references_by_name.values_mut() {
            refs.retain(|r| r.file_id != file_id);
        }
        // Remove empty buckets to keep the map tidy.
        self.references_by_name.retain(|_, v| !v.is_empty());

        for refs in self.references_by_entity.values_mut() {
            refs.retain(|r| r.file_id != file_id);
        }
        self.references_by_entity.retain(|_, v| !v.is_empty());
    }

    /// Look up all reference edges for a given symbol key (bare or qualified name).
    pub fn get_by_name(&self, symbol_key: &str) -> &[ReferenceEdge] {
        self.references_by_name.get(symbol_key).map(Vec::as_slice).unwrap_or_default()
    }

    /// Look up all reference edges targeting a given entity.
    pub fn get_by_entity(&self, entity_id: EntityId) -> &[ReferenceEdge] {
        self.references_by_entity.get(&entity_id).map(Vec::as_slice).unwrap_or_default()
    }

    /// Return the number of distinct symbol keys in the name index.
    pub fn name_count(&self) -> usize {
        self.references_by_name.len()
    }

    /// Return the number of distinct entities in the entity index.
    pub fn entity_count(&self) -> usize {
        self.references_by_entity.len()
    }

    // ── Private helpers ──

    /// Derive a symbol key for an occurrence.
    ///
    /// When the occurrence has a resolved entity_id, we look up the entity's
    /// canonical name from the shard. Otherwise we fall back to a synthetic
    /// key based on the anchor id.
    fn derive_symbol_key(
        &self,
        shard: &FileFactShard,
        occ: &perl_semantic_facts::OccurrenceFact,
    ) -> String {
        if let Some(entity_id) = occ.entity_id {
            // Try to find the entity in the same shard for its canonical name.
            if let Some(entity) = shard.entities.iter().find(|e| e.id == entity_id) {
                return entity.canonical_name.clone();
            }
        }
        // Fallback: use anchor id as a synthetic key.
        format!("__unresolved_anchor_{}", occ.anchor_id.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::facts::PRODUCER_SCHEMA_VERSION;
    use perl_semantic_facts::{
        AnchorFact, AnchorId, Confidence, EdgeFact, EdgeId, EntityFact, EntityKind, OccurrenceFact,
        OccurrenceId, Provenance, ScopeId,
    };

    /// Build a minimal `FileFactShard` with one entity, one reference occurrence,
    /// and one `References` edge linking them.
    fn sample_shard() -> FileFactShard {
        let file_id = FileId(1);
        let entity_id = EntityId(100);
        let anchor_def = AnchorId(10);
        let anchor_ref = AnchorId(20);
        let occ_id = OccurrenceId(400);

        FileFactShard {
            source_uri: "file:///lib/Foo.pm".to_string(),
            file_id,
            content_hash: 999,
            producer_schema_version: PRODUCER_SCHEMA_VERSION,
            anchors_hash: None,
            entities_hash: None,
            occurrences_hash: None,
            edges_hash: None,
            anchors: vec![
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
                    id: anchor_ref,
                    file_id,
                    span_start_byte: 50,
                    span_end_byte: 55,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
            ],
            entities: vec![EntityFact {
                id: entity_id,
                kind: EntityKind::Subroutine,
                canonical_name: "Foo::bar".to_string(),
                anchor_id: Some(anchor_def),
                scope_id: Some(ScopeId(1)),
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            occurrences: vec![OccurrenceFact {
                id: occ_id,
                kind: OccurrenceKind::Call,
                entity_id: Some(entity_id),
                anchor_id: anchor_ref,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            edges: vec![EdgeFact {
                id: EdgeId(500),
                kind: EdgeKind::References,
                from_entity_id: EntityId(0), // caller entity (not relevant here)
                to_entity_id: entity_id,
                via_occurrence_id: Some(occ_id),
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
        }
    }

    #[test]
    fn add_file_populates_name_index() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ReferenceIndex::new();
        index.add_file(&sample_shard());

        let refs = index.get_by_name("Foo::bar");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].kind, OccurrenceKind::Call);
        assert_eq!(refs[0].symbol_key, "Foo::bar");
        Ok(())
    }

    #[test]
    fn add_file_populates_entity_index() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ReferenceIndex::new();
        index.add_file(&sample_shard());

        let refs = index.get_by_entity(EntityId(100));
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].occurrence_id, OccurrenceId(400));
        Ok(())
    }

    #[test]
    fn remove_file_clears_entries() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ReferenceIndex::new();
        index.add_file(&sample_shard());

        assert_eq!(index.name_count(), 1);
        assert_eq!(index.entity_count(), 1);

        index.remove_file("file:///lib/Foo.pm");

        assert_eq!(index.name_count(), 0);
        assert_eq!(index.entity_count(), 0);
        assert!(index.get_by_name("Foo::bar").is_empty());
        assert!(index.get_by_entity(EntityId(100)).is_empty());
        Ok(())
    }

    #[test]
    fn remove_file_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ReferenceIndex::new();
        index.add_file(&sample_shard());

        index.remove_file("file:///lib/Foo.pm");
        // Second remove should be a no-op.
        index.remove_file("file:///lib/Foo.pm");

        assert_eq!(index.name_count(), 0);
        assert_eq!(index.entity_count(), 0);
        Ok(())
    }

    #[test]
    fn remove_unknown_file_is_noop() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ReferenceIndex::new();
        index.add_file(&sample_shard());

        index.remove_file("file:///nonexistent.pm");

        // Original entries should still be present.
        assert_eq!(index.name_count(), 1);
        assert_eq!(index.entity_count(), 1);
        Ok(())
    }

    #[test]
    fn definition_occurrences_are_excluded() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(2);
        let entity_id = EntityId(200);
        let anchor_id = AnchorId(30);

        let shard = FileFactShard {
            source_uri: "file:///lib/Defs.pm".to_string(),
            file_id,
            content_hash: 111,
            producer_schema_version: PRODUCER_SCHEMA_VERSION,
            anchors_hash: None,
            entities_hash: None,
            occurrences_hash: None,
            edges_hash: None,
            anchors: vec![AnchorFact {
                id: anchor_id,
                file_id,
                span_start_byte: 0,
                span_end_byte: 5,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            entities: vec![EntityFact {
                id: entity_id,
                kind: EntityKind::Subroutine,
                canonical_name: "Defs::init".to_string(),
                anchor_id: Some(anchor_id),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            occurrences: vec![OccurrenceFact {
                id: OccurrenceId(600),
                kind: OccurrenceKind::Definition,
                entity_id: Some(entity_id),
                anchor_id,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            edges: vec![],
        };

        let mut index = ReferenceIndex::new();
        index.add_file(&shard);

        // Definition occurrences should not appear in the reference index.
        assert_eq!(index.name_count(), 0);
        assert_eq!(index.entity_count(), 0);
        Ok(())
    }

    #[test]
    fn multiple_files_coexist() -> Result<(), Box<dyn std::error::Error>> {
        let shard_a = sample_shard();

        let file_id_b = FileId(2);
        let entity_id = EntityId(100); // same target entity
        let occ_id_b = OccurrenceId(700);
        let anchor_b = AnchorId(40);

        let shard_b = FileFactShard {
            source_uri: "file:///lib/Bar.pm".to_string(),
            file_id: file_id_b,
            content_hash: 888,
            producer_schema_version: PRODUCER_SCHEMA_VERSION,
            anchors_hash: None,
            entities_hash: None,
            occurrences_hash: None,
            edges_hash: None,
            anchors: vec![AnchorFact {
                id: anchor_b,
                file_id: file_id_b,
                span_start_byte: 10,
                span_end_byte: 18,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            entities: vec![EntityFact {
                id: entity_id,
                kind: EntityKind::Subroutine,
                canonical_name: "Foo::bar".to_string(),
                anchor_id: None,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            occurrences: vec![OccurrenceFact {
                id: occ_id_b,
                kind: OccurrenceKind::Call,
                entity_id: Some(entity_id),
                anchor_id: anchor_b,
                scope_id: None,
                provenance: Provenance::NameHeuristic,
                confidence: Confidence::Medium,
            }],
            edges: vec![],
        };

        let mut index = ReferenceIndex::new();
        index.add_file(&shard_a);
        index.add_file(&shard_b);

        // Both files contribute references to the same name.
        assert_eq!(index.get_by_name("Foo::bar").len(), 2);
        // Both files contribute references to the same entity.
        assert_eq!(index.get_by_entity(entity_id).len(), 2);

        // Remove one file — only its entries should disappear.
        index.remove_file("file:///lib/Foo.pm");
        assert_eq!(index.get_by_name("Foo::bar").len(), 1);
        assert_eq!(index.get_by_entity(entity_id).len(), 1);
        assert_eq!(index.get_by_name("Foo::bar")[0].file_id, file_id_b);

        Ok(())
    }

    #[test]
    fn incremental_reindex_replaces_entries() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ReferenceIndex::new();
        index.add_file(&sample_shard());

        assert_eq!(index.get_by_name("Foo::bar").len(), 1);

        // Simulate re-indexing: remove old, add updated shard.
        index.remove_file("file:///lib/Foo.pm");

        // Updated shard with a different occurrence.
        let file_id = FileId(1);
        let entity_id = EntityId(100);
        let updated_shard = FileFactShard {
            source_uri: "file:///lib/Foo.pm".to_string(),
            file_id,
            content_hash: 1000,
            producer_schema_version: PRODUCER_SCHEMA_VERSION,
            anchors_hash: None,
            entities_hash: None,
            occurrences_hash: None,
            edges_hash: None,
            anchors: vec![AnchorFact {
                id: AnchorId(50),
                file_id,
                span_start_byte: 60,
                span_end_byte: 68,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            entities: vec![EntityFact {
                id: entity_id,
                kind: EntityKind::Subroutine,
                canonical_name: "Foo::bar".to_string(),
                anchor_id: None,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            occurrences: vec![OccurrenceFact {
                id: OccurrenceId(800),
                kind: OccurrenceKind::Read,
                entity_id: Some(entity_id),
                anchor_id: AnchorId(50),
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            edges: vec![],
        };

        index.add_file(&updated_shard);

        let refs = index.get_by_name("Foo::bar");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].occurrence_id, OccurrenceId(800));
        assert_eq!(refs[0].kind, OccurrenceKind::Read);
        Ok(())
    }

    #[test]
    fn unresolved_occurrence_uses_fallback_key() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(3);
        let anchor_id = AnchorId(60);

        let shard = FileFactShard {
            source_uri: "file:///lib/Unresolved.pm".to_string(),
            file_id,
            content_hash: 222,
            producer_schema_version: PRODUCER_SCHEMA_VERSION,
            anchors_hash: None,
            entities_hash: None,
            occurrences_hash: None,
            edges_hash: None,
            anchors: vec![AnchorFact {
                id: anchor_id,
                file_id,
                span_start_byte: 0,
                span_end_byte: 8,
                scope_id: None,
                provenance: Provenance::NameHeuristic,
                confidence: Confidence::Low,
            }],
            entities: vec![],
            occurrences: vec![OccurrenceFact {
                id: OccurrenceId(900),
                kind: OccurrenceKind::Call,
                entity_id: None,
                anchor_id,
                scope_id: None,
                provenance: Provenance::NameHeuristic,
                confidence: Confidence::Low,
            }],
            edges: vec![],
        };

        let mut index = ReferenceIndex::new();
        index.add_file(&shard);

        // The fallback key should be based on the anchor id.
        let fallback_key = "__unresolved_anchor_60";
        let refs = index.get_by_name(fallback_key);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].confidence, Confidence::Low);

        // No entity-based entries since there are no target candidates.
        assert_eq!(index.entity_count(), 0);
        Ok(())
    }

    #[test]
    fn edge_targets_populate_candidates() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ReferenceIndex::new();
        index.add_file(&sample_shard());

        let refs = index.get_by_entity(EntityId(100));
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].target_candidates, vec![EntityId(100)]);
        Ok(())
    }

    #[test]
    fn multiple_edge_targets_produce_multiple_entity_entries()
    -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(4);
        let occ_id = OccurrenceId(1000);
        let anchor_id = AnchorId(70);
        let entity_a = EntityId(300);
        let entity_b = EntityId(301);

        let shard = FileFactShard {
            source_uri: "file:///lib/Ambiguous.pm".to_string(),
            file_id,
            content_hash: 333,
            producer_schema_version: PRODUCER_SCHEMA_VERSION,
            anchors_hash: None,
            entities_hash: None,
            occurrences_hash: None,
            edges_hash: None,
            anchors: vec![AnchorFact {
                id: anchor_id,
                file_id,
                span_start_byte: 0,
                span_end_byte: 5,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            entities: vec![
                EntityFact {
                    id: entity_a,
                    kind: EntityKind::Subroutine,
                    canonical_name: "ambig_func".to_string(),
                    anchor_id: None,
                    scope_id: None,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                EntityFact {
                    id: entity_b,
                    kind: EntityKind::Subroutine,
                    canonical_name: "ambig_func".to_string(),
                    anchor_id: None,
                    scope_id: None,
                    provenance: Provenance::NameHeuristic,
                    confidence: Confidence::Low,
                },
            ],
            occurrences: vec![OccurrenceFact {
                id: occ_id,
                kind: OccurrenceKind::Call,
                entity_id: Some(entity_a),
                anchor_id,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::Medium,
            }],
            edges: vec![
                EdgeFact {
                    id: EdgeId(1001),
                    kind: EdgeKind::References,
                    from_entity_id: EntityId(0),
                    to_entity_id: entity_a,
                    via_occurrence_id: Some(occ_id),
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                },
                EdgeFact {
                    id: EdgeId(1002),
                    kind: EdgeKind::References,
                    from_entity_id: EntityId(0),
                    to_entity_id: entity_b,
                    via_occurrence_id: Some(occ_id),
                    provenance: Provenance::NameHeuristic,
                    confidence: Confidence::Low,
                },
            ],
        };

        let mut index = ReferenceIndex::new();
        index.add_file(&shard);

        // Both entities should have entries in the entity index.
        let refs_a = index.get_by_entity(entity_a);
        assert_eq!(refs_a.len(), 1);
        assert_eq!(refs_a[0].target_candidates.len(), 2);

        let refs_b = index.get_by_entity(entity_b);
        assert_eq!(refs_b.len(), 1);
        assert_eq!(refs_b[0].target_candidates.len(), 2);

        // Name index should have one entry (same symbol key).
        let refs_name = index.get_by_name("ambig_func");
        assert_eq!(refs_name.len(), 1);
        assert_eq!(refs_name[0].target_candidates.len(), 2);

        Ok(())
    }
}
