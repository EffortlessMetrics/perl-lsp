//! Canonical `FileFactShard` population from semantic fact producers.
//!
//! The [`build_canonical_fact_shard`] function assembles a [`FileFactShard`]
//! from the outputs of the canonical semantic adapters in `perl-symbol` and
//! additional semantic analysis products (imports, dynamic boundaries).
//!
//! Unlike the legacy [`WorkspaceIndex::build_fact_shard`] path which populates
//! from `WorkspaceSymbol` with `SearchFallback` provenance and zero byte spans,
//! this path produces facts with:
//!
//! - Real byte spans from the AST projection
//! - `ExactAst` provenance on adapter-produced facts
//! - Per-category hashes computed from the canonical fact vectors
//! - Preserved `ScopeId` where the producer supplies one (carried as `None`
//!   when unknown, per Req 26)

use crate::workspace::workspace_index::FileFactShard;
use perl_position_tracking::WireRange;
use perl_semantic_facts::{
    AnchorFact, AnchorId, Confidence, EdgeFact, EntityFact, FileId, ImportSpec, OccurrenceFact,
    Provenance,
};
use perl_symbol::surface::facts::{SymbolDeclSemanticFacts, SymbolRefSemanticFacts};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Build a [`FileFactShard`] from canonical semantic fact producers.
///
/// This is the canonical population path that replaces the legacy
/// `build_fact_shard` for files that have been processed through the
/// `perl-symbol` AST projection adapters.
///
/// # Arguments
///
/// * `uri` — Canonical file URI for the shard.
/// * `content_hash` — Whole-file content hash for stale-shard replacement.
/// * `decl_facts` — Declaration facts from `symbol_decls_to_semantic_facts`.
/// * `ref_facts` — Reference facts from `symbol_refs_to_semantic_facts`.
/// * `imports` — Import specifications extracted from the file.
/// * `dynamic_boundaries` — Dynamic boundary occurrences (eval, symbolic deref, etc.).
///
/// # Returns
///
/// A fully populated `FileFactShard` with real byte spans, `ExactAst`
/// provenance, and per-category hashes.
///
/// # Note on `use lib` facts
///
/// `use lib`/`no lib` path facts are stored in [`ImportExportIndex`] via
/// `add_file_use_lib` at the indexing call site (in `WorkspaceIndex::index_file`),
/// not in the returned shard.  Query via `SemanticQueries::use_lib_paths`.
pub fn build_canonical_fact_shard(
    uri: &str,
    content_hash: u64,
    decl_facts: &SymbolDeclSemanticFacts,
    ref_facts: &SymbolRefSemanticFacts,
    imports: &[ImportSpec],
    dynamic_boundaries: &[OccurrenceFact],
) -> FileFactShard {
    let file_id = hash_uri_to_file_id(uri);

    // ── Merge anchors ──
    // Declaration anchors come first, then reference anchors, then synthetic
    // anchors for import sites.
    let mut anchors: Vec<AnchorFact> = Vec::with_capacity(
        decl_facts.anchors.len()
            + ref_facts.anchors.len()
            + imports.len()
            + dynamic_boundaries.len(),
    );
    anchors.extend_from_slice(&decl_facts.anchors);
    anchors.extend_from_slice(&ref_facts.anchors);

    // Synthesize anchors for import sites that carry an anchor_id.
    // ImportSpec itself does not carry a full AnchorFact, so we create
    // placeholder anchors with zero byte spans when no span is available.
    // (Future import extractors will supply real spans.)
    for (idx, _import) in imports.iter().enumerate() {
        let anchor_id = AnchorId(import_anchor_base_id(idx));
        anchors.push(AnchorFact {
            id: anchor_id,
            file_id,
            span_start_byte: 0,
            span_end_byte: 0,
            scope_id: None,
            provenance: Provenance::ImportExportInference,
            confidence: Confidence::Medium,
        });
    }

    // ── Merge entities ──
    let entities: Vec<EntityFact> = decl_facts.entities.clone();

    // ── Merge occurrences ──
    // Reference occurrences from the adapter, plus dynamic boundary occurrences.
    let mut occurrences: Vec<OccurrenceFact> =
        Vec::with_capacity(ref_facts.occurrences.len() + dynamic_boundaries.len());
    occurrences.extend_from_slice(&ref_facts.occurrences);
    occurrences.extend_from_slice(dynamic_boundaries);

    // ── Merge edges ──
    // Defines edges from declarations, reference edges from references.
    let mut edges: Vec<EdgeFact> =
        Vec::with_capacity(decl_facts.defines_edges.len() + ref_facts.reference_edges.len());
    edges.extend_from_slice(&decl_facts.defines_edges);
    edges.extend_from_slice(&ref_facts.reference_edges);

    // ── Compute per-category hashes ──
    let anchors_hash = compute_anchors_hash(&anchors);
    let entities_hash = compute_entities_hash(&entities);
    let occurrences_hash = compute_occurrences_hash(&occurrences);
    let edges_hash = compute_edges_hash(&edges);

    FileFactShard {
        source_uri: uri.to_string(),
        file_id,
        content_hash,
        anchors_hash: Some(anchors_hash),
        entities_hash: Some(entities_hash),
        occurrences_hash: Some(occurrences_hash),
        edges_hash: Some(edges_hash),
        anchors,
        entities,
        occurrences,
        edges,
    }
}

/// Deterministic byte-span to LSP range mapping.
///
/// Uses [`perl_position_tracking::WireRange`] to convert byte offsets to
/// zero-based `(line, utf16_column)` coordinates suitable for the LSP
/// protocol.  Multi-byte UTF-8 characters (including those outside the BMP
/// that require surrogate pairs in UTF-16) are handled correctly.
///
/// AnchorFact stores byte offsets only; this conversion happens at query
/// time so that the canonical fact representation stays encoding-neutral.
///
/// Returns `None` when `span_start_byte > span_end_byte` or when
/// `span_end_byte` exceeds the source length.
pub fn byte_span_to_lsp_range(
    source: &str,
    span_start_byte: u32,
    span_end_byte: u32,
) -> Option<WireRange> {
    let start = span_start_byte as usize;
    let end = span_end_byte as usize;

    if start > end || end > source.len() {
        return None;
    }

    Some(WireRange::from_byte_offsets(source, start, end))
}

// ── Helpers ──

/// Derive a stable `FileId` from a URI string (mirrors `WorkspaceIndex::hash_uri_to_file_id`).
fn hash_uri_to_file_id(uri: &str) -> FileId {
    let mut hasher = DefaultHasher::new();
    uri.hash(&mut hasher);
    FileId(hasher.finish())
}

/// Deterministic base ID for synthetic import anchors.
fn import_anchor_base_id(index: usize) -> u64 {
    let mut hasher = DefaultHasher::new();
    "import-anchor".hash(&mut hasher);
    index.hash(&mut hasher);
    hasher.finish()
}

fn compute_anchors_hash(anchors: &[AnchorFact]) -> u64 {
    let mut h = DefaultHasher::new();
    anchors.len().hash(&mut h);
    for a in anchors {
        a.id.hash(&mut h);
        a.span_start_byte.hash(&mut h);
        a.span_end_byte.hash(&mut h);
    }
    h.finish()
}

fn compute_entities_hash(entities: &[EntityFact]) -> u64 {
    let mut h = DefaultHasher::new();
    entities.len().hash(&mut h);
    for e in entities {
        e.id.hash(&mut h);
        e.canonical_name.hash(&mut h);
    }
    h.finish()
}

fn compute_occurrences_hash(occurrences: &[OccurrenceFact]) -> u64 {
    let mut h = DefaultHasher::new();
    occurrences.len().hash(&mut h);
    for o in occurrences {
        o.id.hash(&mut h);
        o.kind.hash(&mut h);
    }
    h.finish()
}

fn compute_edges_hash(edges: &[EdgeFact]) -> u64 {
    let mut h = DefaultHasher::new();
    edges.len().hash(&mut h);
    for e in edges {
        e.id.hash(&mut h);
        e.kind.hash(&mut h);
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_semantic_facts::{
        AnchorId, Confidence, EdgeId, EdgeKind, EntityId, EntityKind, FileId, OccurrenceId,
        OccurrenceKind, Provenance, ScopeId,
    };

    fn sample_decl_facts() -> SymbolDeclSemanticFacts {
        let file_id = FileId(42);
        let anchor_id = AnchorId(100);
        let entity_id = EntityId(200);
        SymbolDeclSemanticFacts {
            anchors: vec![AnchorFact {
                id: anchor_id,
                file_id,
                span_start_byte: 10,
                span_end_byte: 20,
                scope_id: Some(ScopeId(1)),
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            entities: vec![EntityFact {
                id: entity_id,
                kind: EntityKind::Subroutine,
                canonical_name: "MyPackage::foo".to_string(),
                anchor_id: Some(anchor_id),
                scope_id: Some(ScopeId(1)),
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            defines_edges: vec![],
            unsupported: vec![],
        }
    }

    fn sample_ref_facts() -> SymbolRefSemanticFacts {
        let file_id = FileId(42);
        let anchor_id = AnchorId(300);
        let occurrence_id = OccurrenceId(400);
        SymbolRefSemanticFacts {
            anchors: vec![AnchorFact {
                id: anchor_id,
                file_id,
                span_start_byte: 50,
                span_end_byte: 55,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            occurrences: vec![OccurrenceFact {
                id: occurrence_id,
                kind: OccurrenceKind::Call,
                entity_id: Some(EntityId(200)),
                anchor_id,
                scope_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            reference_edges: vec![],
        }
    }

    #[test]
    fn canonical_shard_has_exact_ast_provenance() -> Result<(), Box<dyn std::error::Error>> {
        let shard = build_canonical_fact_shard(
            "file:///test.pl",
            12345,
            &sample_decl_facts(),
            &sample_ref_facts(),
            &[],
            &[],
        );

        // All anchors from the adapters carry ExactAst provenance.
        for anchor in &shard.anchors {
            assert_eq!(
                anchor.provenance,
                Provenance::ExactAst,
                "anchor {} should have ExactAst provenance",
                anchor.id.0
            );
        }

        // Entities carry ExactAst provenance.
        for entity in &shard.entities {
            assert_eq!(entity.provenance, Provenance::ExactAst);
        }

        Ok(())
    }

    #[test]
    fn canonical_shard_has_real_byte_spans() -> Result<(), Box<dyn std::error::Error>> {
        let shard = build_canonical_fact_shard(
            "file:///test.pl",
            12345,
            &sample_decl_facts(),
            &sample_ref_facts(),
            &[],
            &[],
        );

        // Declaration anchor has real byte spans (10..20).
        let decl_anchor =
            shard.anchors.iter().find(|a| a.id == AnchorId(100)).ok_or("missing decl anchor")?;
        assert_eq!(decl_anchor.span_start_byte, 10);
        assert_eq!(decl_anchor.span_end_byte, 20);

        // Reference anchor has real byte spans (50..55).
        let ref_anchor =
            shard.anchors.iter().find(|a| a.id == AnchorId(300)).ok_or("missing ref anchor")?;
        assert_eq!(ref_anchor.span_start_byte, 50);
        assert_eq!(ref_anchor.span_end_byte, 55);

        Ok(())
    }

    #[test]
    fn canonical_shard_preserves_scope_id() -> Result<(), Box<dyn std::error::Error>> {
        let shard = build_canonical_fact_shard(
            "file:///test.pl",
            12345,
            &sample_decl_facts(),
            &sample_ref_facts(),
            &[],
            &[],
        );

        // Declaration anchor preserves ScopeId(1).
        let decl_anchor =
            shard.anchors.iter().find(|a| a.id == AnchorId(100)).ok_or("missing decl anchor")?;
        assert_eq!(decl_anchor.scope_id, Some(ScopeId(1)));

        // Entity preserves ScopeId(1).
        let entity = shard.entities.first().ok_or("missing entity")?;
        assert_eq!(entity.scope_id, Some(ScopeId(1)));

        // Reference anchor carries scope_id as None (unknown).
        let ref_anchor =
            shard.anchors.iter().find(|a| a.id == AnchorId(300)).ok_or("missing ref anchor")?;
        assert_eq!(ref_anchor.scope_id, None);

        Ok(())
    }

    #[test]
    fn canonical_shard_computes_per_category_hashes() -> Result<(), Box<dyn std::error::Error>> {
        let shard = build_canonical_fact_shard(
            "file:///test.pl",
            12345,
            &sample_decl_facts(),
            &sample_ref_facts(),
            &[],
            &[],
        );

        // All per-category hashes are present.
        assert!(shard.anchors_hash.is_some());
        assert!(shard.entities_hash.is_some());
        assert!(shard.occurrences_hash.is_some());
        assert!(shard.edges_hash.is_some());

        // Hashes are non-zero (we have actual facts).
        assert_ne!(shard.anchors_hash, Some(0));
        assert_ne!(shard.entities_hash, Some(0));

        Ok(())
    }

    #[test]
    fn canonical_shard_includes_dynamic_boundaries() -> Result<(), Box<dyn std::error::Error>> {
        let boundary = OccurrenceFact {
            id: OccurrenceId(999),
            kind: OccurrenceKind::DynamicBoundary,
            entity_id: None,
            anchor_id: AnchorId(998),
            scope_id: None,
            provenance: Provenance::DynamicBoundary,
            confidence: Confidence::Low,
        };

        let shard = build_canonical_fact_shard(
            "file:///test.pl",
            12345,
            &sample_decl_facts(),
            &sample_ref_facts(),
            &[],
            std::slice::from_ref(&boundary),
        );

        let found = shard.occurrences.iter().any(|o| o.id == OccurrenceId(999));
        assert!(found, "dynamic boundary occurrence should be in the shard");

        Ok(())
    }

    #[test]
    fn canonical_shard_includes_import_anchors() -> Result<(), Box<dyn std::error::Error>> {
        let import = ImportSpec {
            module: "Foo::Bar".to_string(),
            kind: perl_semantic_facts::ImportKind::Use,
            symbols: perl_semantic_facts::ImportSymbols::Default,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            file_id: None,
            anchor_id: None,
            scope_id: None,
            span_start_byte: None,
        };

        let shard = build_canonical_fact_shard(
            "file:///test.pl",
            12345,
            &sample_decl_facts(),
            &sample_ref_facts(),
            &[import],
            &[],
        );

        // Should have decl anchors + ref anchors + 1 import anchor.
        assert_eq!(shard.anchors.len(), 3);

        // The import anchor should have ImportExportInference provenance.
        let import_anchor = shard
            .anchors
            .iter()
            .find(|a| a.provenance == Provenance::ImportExportInference)
            .ok_or("missing import anchor")?;
        assert_eq!(import_anchor.confidence, Confidence::Medium);

        Ok(())
    }

    #[test]
    fn canonical_shard_deterministic_hashes() -> Result<(), Box<dyn std::error::Error>> {
        let decl_facts = sample_decl_facts();
        let ref_facts = sample_ref_facts();

        let shard1 =
            build_canonical_fact_shard("file:///test.pl", 12345, &decl_facts, &ref_facts, &[], &[]);
        let shard2 =
            build_canonical_fact_shard("file:///test.pl", 12345, &decl_facts, &ref_facts, &[], &[]);

        assert_eq!(shard1.anchors_hash, shard2.anchors_hash);
        assert_eq!(shard1.entities_hash, shard2.entities_hash);
        assert_eq!(shard1.occurrences_hash, shard2.occurrences_hash);
        assert_eq!(shard1.edges_hash, shard2.edges_hash);

        Ok(())
    }

    #[test]
    fn canonical_shard_file_id_matches_uri() -> Result<(), Box<dyn std::error::Error>> {
        let shard = build_canonical_fact_shard(
            "file:///test.pl",
            12345,
            &sample_decl_facts(),
            &sample_ref_facts(),
            &[],
            &[],
        );

        let expected_file_id = hash_uri_to_file_id("file:///test.pl");
        assert_eq!(shard.file_id, expected_file_id);

        Ok(())
    }

    // ── byte_span_to_lsp_range tests ──

    #[test]
    fn byte_span_to_lsp_range_ascii_single_line() -> Result<(), Box<dyn std::error::Error>> {
        let source = "hello world";
        let range = byte_span_to_lsp_range(source, 6, 11).ok_or("expected Some")?;
        // "world" starts at col 6, ends at col 11 (all ASCII, byte == UTF-16 col)
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 6);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 11);
        Ok(())
    }

    #[test]
    fn byte_span_to_lsp_range_multiline() -> Result<(), Box<dyn std::error::Error>> {
        let source = "line0\nline1\nline2";
        // "line2" starts at byte 12, ends at byte 17
        let range = byte_span_to_lsp_range(source, 12, 17).ok_or("expected Some")?;
        assert_eq!(range.start.line, 2);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 2);
        assert_eq!(range.end.character, 5);
        Ok(())
    }

    #[test]
    fn byte_span_to_lsp_range_cross_line() -> Result<(), Box<dyn std::error::Error>> {
        let source = "ab\ncd\nef";
        // Span from 'b' (byte 1) to 'e' (byte 6)
        let range = byte_span_to_lsp_range(source, 1, 7).ok_or("expected Some")?;
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 1);
        assert_eq!(range.end.line, 2);
        assert_eq!(range.end.character, 1);
        Ok(())
    }

    #[test]
    fn byte_span_to_lsp_range_multibyte_utf8() -> Result<(), Box<dyn std::error::Error>> {
        // 'é' is 2 bytes in UTF-8 but 1 UTF-16 code unit
        let source = "aéb";
        // 'a' = byte 0, 'é' = bytes 1..3, 'b' = byte 3
        // Span covering 'b': byte 3..4
        let range = byte_span_to_lsp_range(source, 3, 4).ok_or("expected Some")?;
        assert_eq!(range.start.line, 0);
        // UTF-16 col: 'a'=1 unit, 'é'=1 unit → col 2
        assert_eq!(range.start.character, 2);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 3);
        Ok(())
    }

    #[test]
    fn byte_span_to_lsp_range_surrogate_pair() -> Result<(), Box<dyn std::error::Error>> {
        // '💖' is 4 bytes in UTF-8 and 2 UTF-16 code units (surrogate pair)
        let source = "a💖z";
        // 'a' = byte 0, '💖' = bytes 1..5, 'z' = byte 5
        // Span covering 'z': byte 5..6
        let range = byte_span_to_lsp_range(source, 5, 6).ok_or("expected Some")?;
        assert_eq!(range.start.line, 0);
        // UTF-16 col: 'a'=1 unit, '💖'=2 units → col 3
        assert_eq!(range.start.character, 3);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 4);
        Ok(())
    }

    #[test]
    fn byte_span_to_lsp_range_emoji_on_second_line() -> Result<(), Box<dyn std::error::Error>> {
        let source = "hi\n💖ok";
        // Line 0: "hi\n" (bytes 0..3)
        // Line 1: "💖ok" — '💖' bytes 3..7, 'o' byte 7, 'k' byte 8
        // Span covering "ok": bytes 7..9
        let range = byte_span_to_lsp_range(source, 7, 9).ok_or("expected Some")?;
        assert_eq!(range.start.line, 1);
        // UTF-16 col: '💖'=2 units → col 2
        assert_eq!(range.start.character, 2);
        assert_eq!(range.end.line, 1);
        assert_eq!(range.end.character, 4);
        Ok(())
    }

    #[test]
    fn byte_span_to_lsp_range_empty_span() -> Result<(), Box<dyn std::error::Error>> {
        let source = "abc";
        let range = byte_span_to_lsp_range(source, 1, 1).ok_or("expected Some")?;
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 1);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 1);
        Ok(())
    }

    #[test]
    fn byte_span_to_lsp_range_whole_file() -> Result<(), Box<dyn std::error::Error>> {
        let source = "abc\ndef";
        let len = source.len() as u32;
        let range = byte_span_to_lsp_range(source, 0, len).ok_or("expected Some")?;
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 1);
        assert_eq!(range.end.character, 3);
        Ok(())
    }

    #[test]
    fn byte_span_to_lsp_range_rejects_inverted_span() -> Result<(), Box<dyn std::error::Error>> {
        let source = "hello";
        assert!(byte_span_to_lsp_range(source, 3, 1).is_none());
        Ok(())
    }

    #[test]
    fn byte_span_to_lsp_range_rejects_out_of_bounds() -> Result<(), Box<dyn std::error::Error>> {
        let source = "hello";
        assert!(byte_span_to_lsp_range(source, 0, 10).is_none());
        Ok(())
    }

    #[test]
    fn byte_span_to_lsp_range_empty_source() -> Result<(), Box<dyn std::error::Error>> {
        let source = "";
        let range = byte_span_to_lsp_range(source, 0, 0).ok_or("expected Some")?;
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 0);
        Ok(())
    }

    #[test]
    fn byte_span_to_lsp_range_deterministic() -> Result<(), Box<dyn std::error::Error>> {
        let source = "a💖é\ntest";
        let r1 = byte_span_to_lsp_range(source, 0, 9);
        let r2 = byte_span_to_lsp_range(source, 0, 9);
        assert_eq!(r1, r2);
        Ok(())
    }

    // ── Property-based tests ──

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;
        use proptest::test_runner::Config as ProptestConfig;

        /// Character pool covering ASCII, 2-byte, 3-byte, and 4-byte UTF-8
        /// characters plus newlines.  This exercises the full range of
        /// UTF-16 encoding (BMP and surrogate-pair paths).
        const CHAR_POOL: &[char] = &[
            'a', 'b', 'z', '0', ' ', '\n', // ASCII (1 byte UTF-8, 1 UTF-16 unit)
            'é', 'ñ', 'ü', // 2-byte UTF-8, 1 UTF-16 unit
            '中', '日', '€', // 3-byte UTF-8, 1 UTF-16 unit
            '💖', '😀', '🎉', // 4-byte UTF-8, 2 UTF-16 units (surrogate pair)
        ];

        /// Strategy that builds a source string from the multi-byte character
        /// pool and returns the string together with a valid `(start, end)`
        /// byte span that sits on character boundaries.
        fn arb_source_and_span() -> impl Strategy<Value = (String, u32, u32)> {
            // Pick 1–60 characters from the pool to build the source.
            prop::collection::vec(prop::sample::select(CHAR_POOL), 1..60usize)
                .prop_flat_map(|chars| {
                    let source: String = chars.into_iter().collect();
                    // Collect all valid char-boundary byte offsets (including
                    // source.len() which is the exclusive end).
                    let boundaries: Vec<usize> =
                        (0..=source.len()).filter(|&i| source.is_char_boundary(i)).collect();
                    let max_idx = boundaries.len() - 1;
                    // Pick two indices into the boundaries vec; sort them so
                    // start <= end.
                    (Just(source), Just(boundaries), 0..=max_idx, 0..=max_idx)
                })
                .prop_map(|(source, boundaries, idx_a, idx_b)| {
                    let (lo, hi) = if idx_a <= idx_b { (idx_a, idx_b) } else { (idx_b, idx_a) };
                    (source, boundaries[lo] as u32, boundaries[hi] as u32)
                })
        }

        /// Independently compute the expected UTF-16 column for a byte
        /// offset within a source string.  This mirrors the LSP convention:
        /// column = number of UTF-16 code units from the start of the line
        /// containing the offset.
        fn expected_utf16_col(source: &str, byte_offset: usize) -> u32 {
            // Walk backwards to find the start of the line.
            let line_start = source[..byte_offset].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
            // Count UTF-16 code units from line_start to byte_offset.
            source[line_start..byte_offset].encode_utf16().count() as u32
        }

        // **Validates: Requirements 27.1, 27.3**
        //
        // Property 16: Byte-Span to LSP Range Determinism and UTF-8
        // Correctness — For any source with multi-byte UTF-8 and any valid
        // byte span, the mapping produces a deterministic result with
        // correct UTF-16 column offsets.
        proptest! {
            #![proptest_config(ProptestConfig {
                failure_persistence: None,
                ..ProptestConfig::default()
            })]

            #[test]
            fn prop_byte_span_to_lsp_range_determinism_and_utf16_correctness(
                (source, start, end) in arb_source_and_span(),
            ) {
                // ── Determinism: calling twice with the same inputs
                // produces identical output. ──
                let r1 = byte_span_to_lsp_range(&source, start, end);
                let r2 = byte_span_to_lsp_range(&source, start, end);
                prop_assert_eq!(
                    &r1, &r2,
                    "determinism violated for source={:?} span={}..{}",
                    source, start, end,
                );

                // ── Valid spans must produce Some. ──
                let range = match r1 {
                    Some(r) => r,
                    None => {
                        prop_assert!(
                            false,
                            "expected Some for valid span {}..{} in {:?} (len={})",
                            start, end, source, source.len(),
                        );
                        return Ok(());
                    }
                };

                // ── UTF-16 column correctness for start position. ──
                let expected_start_col = expected_utf16_col(&source, start as usize);
                prop_assert_eq!(
                    range.start.character,
                    expected_start_col,
                    "start UTF-16 col mismatch for source={:?} byte={}",
                    source, start,
                );

                // ── UTF-16 column correctness for end position. ──
                let expected_end_col = expected_utf16_col(&source, end as usize);
                prop_assert_eq!(
                    range.end.character,
                    expected_end_col,
                    "end UTF-16 col mismatch for source={:?} byte={}",
                    source, end,
                );

                // ── Line numbers: start line <= end line. ──
                prop_assert!(
                    range.start.line <= range.end.line,
                    "start line {} > end line {}",
                    range.start.line, range.end.line,
                );

                // ── When start == end, positions must be identical. ──
                if start == end {
                    prop_assert_eq!(
                        range.start, range.end,
                        "empty span should produce identical start/end positions",
                    );
                }
            }
        }
    }

    #[test]
    fn canonical_shard_merges_edges() -> Result<(), Box<dyn std::error::Error>> {
        let _file_id = FileId(42);
        let entity_a = EntityId(1);
        let entity_b = EntityId(2);

        let decl_facts = SymbolDeclSemanticFacts {
            anchors: vec![],
            entities: vec![],
            defines_edges: vec![EdgeFact {
                id: EdgeId(10),
                kind: EdgeKind::Defines,
                from_entity_id: entity_a,
                to_entity_id: entity_b,
                via_occurrence_id: None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            }],
            unsupported: vec![],
        };

        let ref_facts = SymbolRefSemanticFacts {
            anchors: vec![],
            occurrences: vec![],
            reference_edges: vec![EdgeFact {
                id: EdgeId(20),
                kind: EdgeKind::References,
                from_entity_id: entity_b,
                to_entity_id: entity_a,
                via_occurrence_id: None,
                provenance: Provenance::NameHeuristic,
                confidence: Confidence::Low,
            }],
        };

        let shard =
            build_canonical_fact_shard("file:///test.pl", 12345, &decl_facts, &ref_facts, &[], &[]);

        assert_eq!(shard.edges.len(), 2);
        assert!(shard.edges.iter().any(|e| e.kind == EdgeKind::Defines));
        assert!(shard.edges.iter().any(|e| e.kind == EdgeKind::References));

        Ok(())
    }
}
