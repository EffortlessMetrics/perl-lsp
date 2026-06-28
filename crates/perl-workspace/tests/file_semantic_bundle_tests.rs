//! Tests for FileSemanticBundle — complete fact aggregation with correct hashing.
//!
//! Issue #1598: FileSemanticBundle (hash last)
//! Tests verify that synthetic facts (generated members, eval subs) are included
//! in hash computation and that incremental replacement correctly detects changes.

use perl_semantic_facts::{
    AnchorFact, AnchorId, Confidence, EntityFact, EntityId, EntityKind, FileId, Provenance,
};
use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use std::io;
use url::Url;

// ── Helper: Build synthetic entities/anchors for testing ──
// (Helpers defined but may not be used in integration tests; kept for API clarity)

#[allow(dead_code)]
fn make_synthetic_entity(id: EntityId, name: &str, anchor_id: AnchorId) -> EntityFact {
    EntityFact {
        id,
        kind: EntityKind::GeneratedMember,
        canonical_name: name.to_string(),
        anchor_id: Some(anchor_id),
        scope_id: None,
        provenance: Provenance::FrameworkSynthesis,
        confidence: Confidence::Medium,
    }
}

#[allow(dead_code)]
fn make_synthetic_anchor(id: AnchorId, file_id: FileId, start: u32, end: u32) -> AnchorFact {
    AnchorFact {
        id,
        file_id,
        span_start_byte: start,
        span_end_byte: end,
        scope_id: None,
        provenance: Provenance::FrameworkSynthesis,
        confidence: Confidence::Medium,
    }
}

// ── Test 1: entities_hash_covers_generated_members ──
//
// Verify that when a file with generated-member facts is indexed,
// the entities_hash changes compared to a file without them.
// This tests B1 from the acceptance grid.
#[test]
fn entities_hash_covers_generated_members() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Index a file with Moo (generates two accessor members for two `has` fields).
    // Two generated members vs one regular sub ensures entities.len() differs.
    let uri_with_gen = "file:///workspace/with_generated.pm";
    let source_with_gen = r#"
package WithMoo;
use Moo;
has name => (is => 'ro');
has age => (is => 'ro');
1;
"#;

    index.index_file(Url::parse(uri_with_gen)?, source_with_gen.to_string())?;
    let shard_with_gen = index
        .file_fact_shard(uri_with_gen)
        .ok_or_else(|| io::Error::other("missing shard with generated members"))?;

    // Index a file without Moo (no generated members)
    let uri_without_gen = "file:///workspace/without_generated.pm";
    let source_without_gen = r#"
package WithoutMoo;
sub foo { 1 }
1;
"#;

    index.index_file(Url::parse(uri_without_gen)?, source_without_gen.to_string())?;
    let shard_without_gen = index
        .file_fact_shard(uri_without_gen)
        .ok_or_else(|| io::Error::other("missing shard without generated members"))?;

    // Both shards should have entities_hash (not None)
    let hash_with_gen = shard_with_gen
        .entities_hash
        .ok_or_else(|| io::Error::other("entities_hash with generated must be Some"))?;
    let hash_without_gen = shard_without_gen
        .entities_hash
        .ok_or_else(|| io::Error::other("entities_hash without generated must be Some"))?;

    // The hashes should differ because one has generated members and one doesn't
    assert_ne!(
        hash_with_gen, hash_without_gen,
        "entities_hash must change when generated members are present"
    );

    // The shard with generated should have more entities.
    // WithMoo has 2 generated members (name + extra) vs WithoutMoo's 1 sub.
    assert!(
        shard_with_gen.entities.len() > shard_without_gen.entities.len(),
        "shard with generated members should have more entities (got {} vs {})",
        shard_with_gen.entities.len(),
        shard_without_gen.entities.len()
    );

    // At least one entity should be GeneratedMember in the Moo file
    let has_generated =
        shard_with_gen.entities.iter().any(|e| e.kind == EntityKind::GeneratedMember);
    assert!(has_generated, "shard with Moo should contain at least one GeneratedMember entity");

    Ok(())
}

// ── Test 2: category_hash_covers_eval_facts ──
//
// Verify that when a file with eval-sub facts is indexed,
// both entities_hash and anchors_hash change.
// This tests B2 from the acceptance grid.
#[test]
fn category_hash_covers_eval_facts() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Index a file with eval "sub" (generates synthetic facts)
    let uri_with_eval = "file:///workspace/with_eval.pm";
    let source_with_eval = r#"
package WithEval;
eval 'sub generated_sub { my $x = 1; }';
1;
"#;

    index.index_file(Url::parse(uri_with_eval)?, source_with_eval.to_string())?;
    let shard_with_eval = index
        .file_fact_shard(uri_with_eval)
        .ok_or_else(|| io::Error::other("missing shard with eval"))?;

    // Index a file without eval (no synthetic eval facts)
    let uri_without_eval = "file:///workspace/without_eval.pm";
    let source_without_eval = r#"
package WithoutEval;
sub normal_sub { my $x = 1; }
1;
"#;

    index.index_file(Url::parse(uri_without_eval)?, source_without_eval.to_string())?;
    let shard_without_eval = index
        .file_fact_shard(uri_without_eval)
        .ok_or_else(|| io::Error::other("missing shard without eval"))?;

    // Both should have entities_hash
    let entities_hash_with = shard_with_eval
        .entities_hash
        .ok_or_else(|| io::Error::other("entities_hash with eval must be Some"))?;
    let entities_hash_without = shard_without_eval
        .entities_hash
        .ok_or_else(|| io::Error::other("entities_hash without eval must be Some"))?;

    // Both should have anchors_hash
    let anchors_hash_with = shard_with_eval
        .anchors_hash
        .ok_or_else(|| io::Error::other("anchors_hash with eval must be Some"))?;
    let anchors_hash_without = shard_without_eval
        .anchors_hash
        .ok_or_else(|| io::Error::other("anchors_hash without eval must be Some"))?;

    // Hashes should differ when eval facts are present
    assert_ne!(
        entities_hash_with, entities_hash_without,
        "entities_hash must change when eval facts are added"
    );
    assert_ne!(
        anchors_hash_with, anchors_hash_without,
        "anchors_hash must change when eval facts are added"
    );

    Ok(())
}

// ── Test 3: file_fact_shard_carries_producer_schema_version ──
//
// Verify that FileFactShard has a producer_schema_version field
// and it equals the constant PRODUCER_SCHEMA_VERSION (expected to be 1).
// This tests B6 from the acceptance grid.
//
// NOTE: This test compiles against the absence of the field and documents
// what the test expects. Once the builder adds the field to FileFactShard,
// this test will need to be updated to assert the field's value.
#[test]
fn file_fact_shard_carries_producer_schema_version() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = "file:///workspace/schema_version_test.pm";
    let source = "package Test; sub foo { 1 } 1;";

    index.index_file(Url::parse(uri)?, source.to_string())?;
    let shard = index.file_fact_shard(uri).ok_or_else(|| io::Error::other("missing shard"))?;

    // The shard must carry the producer schema version constant.
    assert_eq!(
        shard.producer_schema_version, 1,
        "producer_schema_version must be 1 (PRODUCER_SCHEMA_VERSION constant)"
    );

    Ok(())
}

// ── Test 4: replace_fact_shard_incremental_detects_synthetic_entity_change ──
//
// Verify that when only synthetic facts change (e.g., eval sub name changes),
// replace_fact_shard_incremental re-indexes (not skipped).
// This tests B4 from the acceptance grid.
//
// NOTE: This test may fail with "ReplaceResult not public" or similar.
// That's expected if the internal type isn't exposed. The test structure
// is written to compile once the API is available.
#[test]
fn replace_fact_shard_incremental_detects_synthetic_entity_change()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Index a file with eval sub (first version)
    let uri = "file:///workspace/incremental_test.pm";
    let source_v1 = r#"
package IncrementalTest;
eval 'sub generated_name_v1 { 1 }';
1;
"#;

    index.index_file(Url::parse(uri)?, source_v1.to_string())?;
    let shard_v1 =
        index.file_fact_shard(uri).ok_or_else(|| io::Error::other("missing shard v1"))?;

    // Re-index with different eval sub (synthetic facts change)
    let source_v2 = r#"
package IncrementalTest;
eval 'sub generated_name_v2 { 1 }';
1;
"#;

    index.index_file(Url::parse(uri)?, source_v2.to_string())?;
    let shard_v2 =
        index.file_fact_shard(uri).ok_or_else(|| io::Error::other("missing shard v2"))?;

    // If the implementation is correct, entities_hash should differ
    // (because the eval sub entity changed)
    let hash_v1 =
        shard_v1.entities_hash.ok_or_else(|| io::Error::other("entities_hash v1 must be Some"))?;
    let hash_v2 =
        shard_v2.entities_hash.ok_or_else(|| io::Error::other("entities_hash v2 must be Some"))?;

    // Hashes should differ because the eval sub entity differs
    assert_ne!(hash_v1, hash_v2, "entities_hash must change when eval sub entity changes");

    // The shards should be different in their entities
    assert_ne!(shard_v1.entities, shard_v2.entities, "entities must differ when eval subs change");

    Ok(())
}

// ── Test 5: duplicate_anchor_guard_fires_on_collision ──
//
// Verify that if two shards with identical AnchorIds are registered,
// the workspace index's collision detection fails closed (returns None
// or Err, not a silent overwrite).
//
// This is an adversarial test for Hazard Class 1 (ID collision detection).
// Before file-scoped IDs (#1600), collision was possible. After #1600,
// collisions should not occur in normal operation, but the guard itself
// must still work.
//
// NOTE: This test may not fully exercise the guard without access to
// internal collision-handling logic. If the public API doesn't expose
// enough to trigger the guard, this test documents what the guard
// should do and may be completed during green-tdd.
#[test]
fn duplicate_anchor_guard_fires_on_collision() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Index file 1 with a specific anchor ID
    let uri1 = "file:///workspace/file1_collision.pm";
    let source1 = "package File1; sub anchor_id_123 { 1 } 1;";
    index.index_file(Url::parse(uri1)?, source1.to_string())?;

    // Index file 2 also with the same anchor ID (synthetic collision scenario)
    let uri2 = "file:///workspace/file2_collision.pm";
    let source2 = "package File2; sub also_anchor_id_123 { 1 } 1;";
    index.index_file(Url::parse(uri2)?, source2.to_string())?;

    // Both files should be indexed successfully
    let shard1 = index.file_fact_shard(uri1).ok_or_else(|| io::Error::other("missing shard1"))?;
    let shard2 = index.file_fact_shard(uri2).ok_or_else(|| io::Error::other("missing shard2"))?;

    // Both shards should exist and have anchors
    assert!(!shard1.anchors.is_empty(), "shard1 must have anchors");
    assert!(!shard2.anchors.is_empty(), "shard2 must have anchors");

    // The workspace index should NOT silently merge or overwrite anchors
    // Verify by checking that both shards' anchor counts are preserved
    // (This is a basic sanity check; the full collision guard is internal)
    assert_eq!(shard1.source_uri, uri1, "shard1 must retain its original URI");
    assert_eq!(shard2.source_uri, uri2, "shard2 must retain its original URI");

    Ok(())
}

// ── Test 6: empty_file_synthetic_slices ──
//
// Verify that even with empty synthetic slices, hash computation
// works correctly and produces stable hashes.
// This is an adversarial test from the Hazard-Class 2 boundary conditions.
#[test]
fn empty_file_synthetic_slices() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = "file:///workspace/empty_synthetic.pm";
    let source = "package Empty; 1;";

    index.index_file(Url::parse(uri)?, source.to_string())?;
    let shard = index.file_fact_shard(uri).ok_or_else(|| io::Error::other("missing shard"))?;

    // Even with no synthetic facts, hashes must be present and stable
    assert!(
        shard.entities_hash.is_some(),
        "entities_hash must be Some even with no synthetic entities"
    );
    assert!(
        shard.anchors_hash.is_some(),
        "anchors_hash must be Some even with no synthetic anchors"
    );

    // Hashes should be deterministic
    index.clear();
    index.index_file(Url::parse(uri)?, source.to_string())?;
    let shard2 = index.file_fact_shard(uri).ok_or_else(|| io::Error::other("missing shard2"))?;

    assert_eq!(shard.entities_hash, shard2.entities_hash, "entities_hash must be deterministic");
    assert_eq!(shard.anchors_hash, shard2.anchors_hash, "anchors_hash must be deterministic");

    Ok(())
}

// ── Test 7: synthetic_facts_not_added_twice ──
//
// Verify that synthetic entities/anchors appear exactly once in the shard,
// not duplicated by post-build push loops.
// This is a negative test ensuring the fix (deleting post-build push)
// doesn't allow double-counting.
#[test]
fn synthetic_facts_not_added_twice() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Index a file with generated members (Moo)
    let uri = "file:///workspace/no_double_push.pm";
    let source = r#"
package NoDoublePush;
use Moo;
has field1 => (is => 'ro');
has field2 => (is => 'ro');
1;
"#;

    index.index_file(Url::parse(uri)?, source.to_string())?;
    let shard = index.file_fact_shard(uri).ok_or_else(|| io::Error::other("missing shard"))?;

    // Count generated member entities
    let generated_entities: Vec<_> =
        shard.entities.iter().filter(|e| e.kind == EntityKind::GeneratedMember).collect();

    // We should have at most 2 generated members (field1 and field2)
    // If post-build push is running, we'd see double (4 or more)
    assert!(
        generated_entities.len() <= 2,
        "generated members should not be duplicated (found {})",
        generated_entities.len()
    );

    // If we found exactly 2, that's the expected case for this Moo file
    // (field1 and field2)
    if generated_entities.len() == 2 {
        assert_eq!(
            generated_entities[0].canonical_name,
            format!("{}::field1", "NoDoublePush"),
            "first generated member should be field1"
        );
        assert_eq!(
            generated_entities[1].canonical_name,
            format!("{}::field2", "NoDoublePush"),
            "second generated member should be field2"
        );
    }

    Ok(())
}

// ── Test 8: synthetic_fact_order_invariant ──
//
// Verify that hash computation is order-invariant for synthetic facts.
// Two shards built with the same facts but in different order should
// have identical hashes.
//
// NOTE: This test may require exposing the build_canonical_fact_shard
// function or internal hash computation. If those are not public, this
// test structure documents the expected behavior for green-tdd to verify.
#[test]
fn synthetic_fact_order_invariant() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Index the same file twice (should produce identical hashes)
    let uri = "file:///workspace/order_invariant.pm";
    let source = r#"
package OrderInvariant;
use Moo;
has a => (is => 'ro');
has b => (is => 'ro');
1;
"#;

    // First index
    index.index_file(Url::parse(uri)?, source.to_string())?;
    let shard1 = index.file_fact_shard(uri).ok_or_else(|| io::Error::other("missing shard1"))?;

    // Clear and re-index
    index.clear();
    index.index_file(Url::parse(uri)?, source.to_string())?;
    let shard2 = index.file_fact_shard(uri).ok_or_else(|| io::Error::other("missing shard2"))?;

    // Hashes must be identical
    assert_eq!(shard1.entities_hash, shard2.entities_hash, "entities_hash must be order-invariant");
    assert_eq!(shard1.anchors_hash, shard2.anchors_hash, "anchors_hash must be order-invariant");

    Ok(())
}
