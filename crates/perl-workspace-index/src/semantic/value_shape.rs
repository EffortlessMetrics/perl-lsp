//! Value-shape index mapping entity IDs to lightweight type approximations.
//!
//! Maintains a [`HashMap<EntityId, ValueShape>`] index populated from the
//! [`ValueShapeInferrer`](perl_semantic_analyzer) output during workspace
//! indexing. Supports incremental add/remove keyed by the file's source URI.
//!
//! The index is consumed by [`method_candidates`](super::queries) to filter
//! candidates by receiver shape — when the receiver's `ValueShape` is
//! `Object { package, .. }` or `PackageName { package }`, the method search
//! is scoped to that package's inheritance chain.
//!
//! # Requirements
//!
//! - **Req 15.2**: Maintain a `value_shapes` index mapping `EntityId` to `ValueShape`.

use perl_semantic_facts::{EntityId, ValueShape};
use std::collections::HashMap;

/// Cross-file value-shape index.
///
/// Populated from `(EntityId, ValueShape)` pairs produced by the value-shape
/// inferrer during workspace indexing. Supports incremental updates: call
/// [`remove_shapes_for_file`](Self::remove_shapes_for_file) to purge stale
/// entries, then [`add_shapes`](Self::add_shapes) to insert fresh ones.
#[derive(Debug, Default)]
pub struct ValueShapeIndex {
    /// Entity ID → inferred value shape.
    shapes: HashMap<EntityId, ValueShape>,

    /// Tracks which file URIs contributed which entity IDs, so that
    /// [`remove_shapes_for_file`](Self::remove_shapes_for_file) can purge
    /// stale entries during incremental re-indexing.
    file_entities: HashMap<String, Vec<EntityId>>,
}

impl ValueShapeIndex {
    /// Create an empty value-shape index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a batch of `(EntityId, ValueShape)` pairs from a single file.
    ///
    /// Later entries for the same `EntityId` overwrite earlier ones (last
    /// writer wins within a file).
    pub fn add_shapes(&mut self, source_uri: &str, shapes: Vec<(EntityId, ValueShape)>) {
        let entity_ids: Vec<EntityId> = shapes.iter().map(|(id, _)| *id).collect();
        self.file_entities.insert(source_uri.to_string(), entity_ids);

        for (entity_id, shape) in shapes {
            self.shapes.insert(entity_id, shape);
        }
    }

    /// Remove all value shapes that originated from the given file URI.
    ///
    /// This is the "remove" half of incremental re-indexing: call this
    /// before [`add_shapes`](Self::add_shapes) with the updated shapes.
    pub fn remove_shapes_for_file(&mut self, source_uri: &str) {
        let entity_ids = match self.file_entities.remove(source_uri) {
            Some(ids) => ids,
            None => return,
        };

        for id in &entity_ids {
            self.shapes.remove(id);
        }
    }

    /// Look up the [`ValueShape`] for a given entity.
    pub fn get(&self, entity_id: EntityId) -> Option<&ValueShape> {
        self.shapes.get(&entity_id)
    }

    /// Return the number of entity→shape mappings in the index.
    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    /// Return `true` when the index contains no mappings.
    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }

    /// Resolve a [`ValueShape`] to a package name for method candidate
    /// filtering.
    ///
    /// Returns `Some(package)` for shapes that identify a specific package
    /// (`Object`, `PackageName`), and `None` for shapes that do not
    /// constrain the receiver (`Unknown`, `Scalar`, etc.).
    pub fn resolve_receiver_package(shape: &ValueShape) -> Option<&str> {
        match shape {
            ValueShape::Object { package, .. } => Some(package.as_str()),
            ValueShape::PackageName { package } => Some(package.as_str()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_semantic_facts::Confidence;

    // ── Construction and basic operations ──

    #[test]
    fn empty_index_has_no_shapes() -> Result<(), Box<dyn std::error::Error>> {
        let index = ValueShapeIndex::new();
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
        Ok(())
    }

    #[test]
    fn add_shapes_populates_index() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ValueShapeIndex::new();
        index.add_shapes(
            "file:///lib/Foo.pm",
            vec![
                (
                    EntityId(1),
                    ValueShape::Object { package: "Foo".to_string(), confidence: Confidence::High },
                ),
                (EntityId(2), ValueShape::Unknown),
            ],
        );

        assert_eq!(index.len(), 2);
        assert!(!index.is_empty());

        let shape1 = index.get(EntityId(1)).ok_or("expected shape for EntityId(1)")?;
        assert_eq!(
            *shape1,
            ValueShape::Object { package: "Foo".to_string(), confidence: Confidence::High }
        );

        let shape2 = index.get(EntityId(2)).ok_or("expected shape for EntityId(2)")?;
        assert_eq!(*shape2, ValueShape::Unknown);

        Ok(())
    }

    #[test]
    fn get_returns_none_for_unknown_entity() -> Result<(), Box<dyn std::error::Error>> {
        let index = ValueShapeIndex::new();
        assert!(index.get(EntityId(999)).is_none());
        Ok(())
    }

    // ── Incremental add/remove ──

    #[test]
    fn remove_shapes_for_file_clears_entries() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ValueShapeIndex::new();
        index.add_shapes("file:///lib/Foo.pm", vec![(EntityId(1), ValueShape::Scalar)]);

        assert_eq!(index.len(), 1);

        index.remove_shapes_for_file("file:///lib/Foo.pm");

        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
        assert!(index.get(EntityId(1)).is_none());
        Ok(())
    }

    #[test]
    fn remove_shapes_for_file_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ValueShapeIndex::new();
        index.add_shapes("file:///lib/Foo.pm", vec![(EntityId(1), ValueShape::Scalar)]);

        index.remove_shapes_for_file("file:///lib/Foo.pm");
        // Second remove should be a no-op.
        index.remove_shapes_for_file("file:///lib/Foo.pm");

        assert_eq!(index.len(), 0);
        Ok(())
    }

    #[test]
    fn remove_unknown_file_is_noop() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ValueShapeIndex::new();
        index.add_shapes("file:///lib/Foo.pm", vec![(EntityId(1), ValueShape::Scalar)]);

        index.remove_shapes_for_file("file:///nonexistent.pm");

        assert_eq!(index.len(), 1);
        assert!(index.get(EntityId(1)).is_some());
        Ok(())
    }

    #[test]
    fn multiple_files_coexist() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ValueShapeIndex::new();
        index.add_shapes(
            "file:///lib/Foo.pm",
            vec![(
                EntityId(1),
                ValueShape::Object { package: "Foo".to_string(), confidence: Confidence::High },
            )],
        );
        index.add_shapes(
            "file:///lib/Bar.pm",
            vec![(
                EntityId(2),
                ValueShape::Object { package: "Bar".to_string(), confidence: Confidence::Medium },
            )],
        );

        assert_eq!(index.len(), 2);

        // Remove one file — only its shapes should disappear.
        index.remove_shapes_for_file("file:///lib/Foo.pm");

        assert_eq!(index.len(), 1);
        assert!(index.get(EntityId(1)).is_none());
        assert!(index.get(EntityId(2)).is_some());
        Ok(())
    }

    #[test]
    fn incremental_reindex_replaces_shapes() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ValueShapeIndex::new();
        index.add_shapes("file:///lib/Foo.pm", vec![(EntityId(1), ValueShape::Unknown)]);

        let shape = index.get(EntityId(1)).ok_or("expected shape")?;
        assert_eq!(*shape, ValueShape::Unknown);

        // Simulate re-indexing: remove old, add updated shapes.
        index.remove_shapes_for_file("file:///lib/Foo.pm");
        index.add_shapes(
            "file:///lib/Foo.pm",
            vec![(
                EntityId(1),
                ValueShape::Object { package: "Foo".to_string(), confidence: Confidence::High },
            )],
        );

        let shape = index.get(EntityId(1)).ok_or("expected updated shape")?;
        assert_eq!(
            *shape,
            ValueShape::Object { package: "Foo".to_string(), confidence: Confidence::High }
        );
        Ok(())
    }

    #[test]
    fn last_writer_wins_within_file() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ValueShapeIndex::new();
        // Same entity ID with two different shapes — last one wins.
        index.add_shapes(
            "file:///lib/Foo.pm",
            vec![(EntityId(1), ValueShape::Unknown), (EntityId(1), ValueShape::Scalar)],
        );

        let shape = index.get(EntityId(1)).ok_or("expected shape")?;
        assert_eq!(*shape, ValueShape::Scalar);
        Ok(())
    }

    // ── resolve_receiver_package ──

    #[test]
    fn resolve_receiver_package_for_object() -> Result<(), Box<dyn std::error::Error>> {
        let shape = ValueShape::Object { package: "Foo".to_string(), confidence: Confidence::High };
        let pkg = ValueShapeIndex::resolve_receiver_package(&shape)
            .ok_or("expected package from Object shape")?;
        assert_eq!(pkg, "Foo");
        Ok(())
    }

    #[test]
    fn resolve_receiver_package_for_package_name() -> Result<(), Box<dyn std::error::Error>> {
        let shape = ValueShape::PackageName { package: "Bar".to_string() };
        let pkg = ValueShapeIndex::resolve_receiver_package(&shape)
            .ok_or("expected package from PackageName shape")?;
        assert_eq!(pkg, "Bar");
        Ok(())
    }

    #[test]
    fn resolve_receiver_package_returns_none_for_unknown() -> Result<(), Box<dyn std::error::Error>>
    {
        assert!(ValueShapeIndex::resolve_receiver_package(&ValueShape::Unknown).is_none());
        assert!(ValueShapeIndex::resolve_receiver_package(&ValueShape::Scalar).is_none());
        assert!(ValueShapeIndex::resolve_receiver_package(&ValueShape::ArrayRef).is_none());
        assert!(ValueShapeIndex::resolve_receiver_package(&ValueShape::HashRef).is_none());
        assert!(ValueShapeIndex::resolve_receiver_package(&ValueShape::CodeRef).is_none());
        Ok(())
    }

    // ── All ValueShape variants can be stored and retrieved ──

    #[test]
    fn all_shape_variants_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = ValueShapeIndex::new();
        let shapes = vec![
            (EntityId(1), ValueShape::Unknown),
            (EntityId(2), ValueShape::Scalar),
            (EntityId(3), ValueShape::ArrayRef),
            (EntityId(4), ValueShape::HashRef),
            (EntityId(5), ValueShape::CodeRef),
            (EntityId(6), ValueShape::PackageName { package: "Foo".to_string() }),
            (
                EntityId(7),
                ValueShape::Object { package: "Bar".to_string(), confidence: Confidence::Low },
            ),
        ];

        index.add_shapes("file:///lib/Test.pm", shapes.clone());

        for (id, expected) in &shapes {
            let actual = index.get(*id).ok_or(format!("expected shape for {id:?}"))?;
            assert_eq!(actual, expected);
        }
        Ok(())
    }
}
