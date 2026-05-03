//! Literal-eval sub extractor for dynamic boundary evidence.
//!
//! Recognizes `eval "sub NAME { ... }"` patterns in an AST and emits an
//! [`OccurrenceFact`] with `kind = OccurrenceKind::DynamicBoundary` keyed to
//! the sub name `NAME`.
//!
//! # Scope
//!
//! Only literal string evals whose string value textually contains `sub NAME`
//! are recognized. Non-literal evals (e.g. `eval $code`) are out of scope —
//! the module name is not statically known and no evidence is emitted.
//!
//! # Requirements
//!
//! - **Req 7.5a**: Emit `DynamicBoundary` evidence for `eval "sub NAME { ... }"`
//!   so that `dynamic_callable_may_be_visible_at` can suppress the
//!   `UnquotedBareword` diagnostic for `NAME` at later call sites in the
//!   same file.

use crate::workspace::workspace_index::FileFactShard;
use crate::ast::{Node, NodeKind};
use perl_semantic_facts::{
    AnchorFact, AnchorId, Confidence, EntityFact, EntityId, EntityKind, FileId, OccurrenceFact,
    OccurrenceId, OccurrenceKind, Provenance,
};

/// Walk an AST and return `(EntityFact, AnchorFact, OccurrenceFact)` triples
/// for each `eval "sub NAME { ... }"` pattern found.
///
/// The returned facts should be merged into the file's [`FileFactShard`] by
/// the caller so that `dynamic_callable_may_be_visible_at` can find them.
///
/// # Algorithm
///
/// 1. Recursively walk every node.
/// 2. For each `NodeKind::Eval { block }` where `block` is a
///    `NodeKind::String { value, .. }` (a literal string eval), extract
///    all sub names that appear as `sub NAME` in `value`.
/// 3. For each name found, emit a triple with `Confidence::Low` and
///    `Provenance::DynamicBoundary`.
///
/// # ID generation
///
/// IDs are derived from a stable hash of `(file_id, node_start_byte, name)`
/// to avoid collisions across multiple eval strings in the same file.
pub fn extract_eval_sub_boundaries(
    ast: &Node,
    file_id: FileId,
) -> Vec<(EntityFact, AnchorFact, OccurrenceFact)> {
    let mut out = Vec::new();
    walk(ast, file_id, &mut out);
    out
}

fn walk(node: &Node, file_id: FileId, out: &mut Vec<(EntityFact, AnchorFact, OccurrenceFact)>) {
    if let NodeKind::Eval { block } = &node.kind {
        // Only literal string evals produce evidence.
        if let NodeKind::String { value, .. } = &block.kind {
            extract_from_eval_string(value, node.location.start, file_id, out);
        }
        // Recurse into the block for nested evals.
        walk(block, file_id, out);
        return;
    }

    for child in node.children() {
        walk(child, file_id, out);
    }
}

/// Parse `eval_string` for `sub NAME` patterns and emit triples.
///
/// Handles simple identifiers: `sub foo_bar`, `sub _helper123`, etc.
/// Does NOT handle `sub { ... }` anonymous subs (no name to extract).
fn extract_from_eval_string(
    eval_string: &str,
    node_start_byte: usize,
    file_id: FileId,
    out: &mut Vec<(EntityFact, AnchorFact, OccurrenceFact)>,
) {
    // Strip surrounding quotes if present (the parser may or may not include them).
    let content = eval_string
        .trim_start_matches('"')
        .trim_end_matches('"')
        .trim_start_matches('\'')
        .trim_end_matches('\'');

    // Scan for `sub IDENTIFIER` patterns in the string content.
    let mut search = content;
    let mut byte_pos = 0usize;
    while !search.is_empty() {
        // Find the next `sub ` keyword.
        let Some(sub_pos) = find_sub_keyword(search) else {
            break;
        };

        byte_pos += sub_pos;
        let after_sub = &search[sub_pos + 3..]; // skip "sub"
        byte_pos += 3;

        // Skip whitespace between `sub` and the name.
        let ws_len = after_sub.len() - after_sub.trim_start_matches(|c: char| c.is_ascii_whitespace()).len();
        let after_ws = &after_sub[ws_len..];
        byte_pos += ws_len;

        // Extract the identifier name.
        let name_len = after_ws
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .unwrap_or(after_ws.len());

        if name_len > 0 {
            let name = &after_ws[..name_len];
            // Validate: must start with a letter or underscore.
            if name.as_bytes().first().is_some_and(|&b| b.is_ascii_alphabetic() || b == b'_') {
                emit_triple(name, node_start_byte, file_id, out);
            }
        }

        // Advance past the name to continue scanning.
        let advance = sub_pos + 3 + ws_len + name_len.max(1);
        if advance >= search.len() {
            break;
        }
        search = &search[advance..];
        byte_pos = byte_pos.saturating_sub(advance - advance); // reset relative tracking
    }
}

/// Find the byte offset of the next `sub` keyword in `text` that is preceded
/// by a word boundary (not part of a longer identifier like `suburb`).
fn find_sub_keyword(text: &str) -> Option<usize> {
    let mut start = 0;
    while start < text.len() {
        let Some(pos) = text[start..].find("sub") else {
            return None;
        };
        let abs_pos = start + pos;

        // Check left boundary: must be at start or preceded by non-word char.
        let left_ok = abs_pos == 0
            || !text.as_bytes()[abs_pos - 1].is_ascii_alphanumeric()
                && text.as_bytes()[abs_pos - 1] != b'_';

        // Check right boundary: must be followed by whitespace or end.
        let right_byte = text.as_bytes().get(abs_pos + 3).copied();
        let right_ok = right_byte.map(|b| b.is_ascii_whitespace()).unwrap_or(true);

        if left_ok && right_ok {
            return Some(abs_pos);
        }

        start = abs_pos + 3;
    }
    None
}

/// Emit a `(EntityFact, AnchorFact, OccurrenceFact)` triple for a named sub
/// found in an eval string.
fn emit_triple(
    name: &str,
    node_start_byte: usize,
    file_id: FileId,
    out: &mut Vec<(EntityFact, AnchorFact, OccurrenceFact)>,
) {
    // Stable ID derivation: hash (file_id, node_start_byte, name).
    let base_id = stable_id(file_id.0, node_start_byte as u64, name);

    let entity_id = EntityId(base_id);
    let anchor_id = AnchorId(base_id + 1);
    let occurrence_id = OccurrenceId(base_id + 2);

    let entity = EntityFact {
        id: entity_id,
        canonical_name: name.to_string(),
        kind: EntityKind::Subroutine,
        anchor_id: Some(anchor_id),
        scope_id: None,
        provenance: Provenance::DynamicBoundary,
        confidence: Confidence::Low,
    };

    let anchor = AnchorFact {
        id: anchor_id,
        file_id,
        // Use the eval node's span — the sub lives within the eval string.
        span_start_byte: node_start_byte as u32,
        span_end_byte: (node_start_byte + 1).max(node_start_byte + name.len()) as u32,
        scope_id: None,
        provenance: Provenance::DynamicBoundary,
        confidence: Confidence::Low,
    };

    let occurrence = OccurrenceFact {
        id: occurrence_id,
        kind: OccurrenceKind::DynamicBoundary,
        entity_id: Some(entity_id),
        anchor_id,
        scope_id: None,
        provenance: Provenance::DynamicBoundary,
        confidence: Confidence::Low,
    };

    out.push((entity, anchor, occurrence));
}

/// Compute a stable u64 ID from (file_id, node_start, name) using FNV-1a.
fn stable_id(file_id: u64, node_start: u64, name: &str) -> u64 {
    // FNV-1a 64-bit hash.
    const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
    const FNV_PRIME: u64 = 1_099_511_628_211;

    let mut hash = FNV_OFFSET;
    for &byte in &file_id.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    for &byte in &node_start.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    for &byte in name.as_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    // Reserve 3 IDs per triple (entity, anchor, occurrence).
    // Shift left by 3 bits so base_id, base_id+1, base_id+2 are in a cluster.
    // Use a high-base offset (0xE_0000_0000) to avoid collisions with symbol
    // adapter IDs which start from lower values.
    0xE_0000_0000_u64.wrapping_add(hash.wrapping_shl(3))
}

/// Merge eval-sub boundary triples into a [`FileFactShard`].
///
/// Appends the new entities, anchors, and occurrences from
/// [`extract_eval_sub_boundaries`] into the shard's respective vectors.
/// Callers must re-hash the shard after merging if they rely on the
/// per-category hashes (the hash recomputation is done in
/// `build_canonical_fact_shard_for_ast`).
pub fn merge_eval_sub_boundaries_into_shard(
    shard: &mut FileFactShard,
    triples: Vec<(EntityFact, AnchorFact, OccurrenceFact)>,
) {
    for (entity, anchor, occurrence) in triples {
        shard.entities.push(entity);
        shard.anchors.push(anchor);
        shard.occurrences.push(occurrence);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_semantic_facts::FileId;

    // ── Unit tests for find_sub_keyword ──

    #[test]
    fn find_sub_keyword_basic() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(find_sub_keyword("sub foo { 1 }"), Some(0));
        assert_eq!(find_sub_keyword("  sub bar { }"), Some(2));
        // The FIRST `sub` in the string is at position 3 ("no sub here").
        assert_eq!(find_sub_keyword("no sub here really sub baz"), Some(3));
        Ok(())
    }

    #[test]
    fn find_sub_keyword_rejects_suburb() -> Result<(), Box<dyn std::error::Error>> {
        // "suburb" contains "sub" but as part of a word — must not match.
        assert_eq!(find_sub_keyword("suburb"), None);
        // "subsub" also should not match as a keyword.
        // Note: "sub sub" should match the second one.
        assert_eq!(find_sub_keyword("sub sub foo"), Some(0));
        Ok(())
    }

    #[test]
    fn find_sub_keyword_none_when_absent() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(find_sub_keyword("hello world"), None);
        assert_eq!(find_sub_keyword(""), None);
        Ok(())
    }

    // ── Unit tests for extract_eval_sub_boundaries ──

    fn parse_and_extract(code: &str, file_id: FileId) -> Vec<(EntityFact, AnchorFact, OccurrenceFact)> {
        let mut parser = crate::Parser::new(code);
        let ast = match parser.parse() {
            Ok(a) => a,
            Err(_) => return vec![],
        };
        extract_eval_sub_boundaries(&ast, file_id)
    }

    #[test]
    fn extracts_single_sub_from_eval_string() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(1);
        let triples =
            parse_and_extract(r#"eval "sub generated_from_string { 1 }";"#, file_id);

        assert_eq!(triples.len(), 1, "should extract exactly one sub");
        let (entity, _anchor, occurrence) = &triples[0];
        assert_eq!(entity.canonical_name, "generated_from_string");
        assert_eq!(entity.kind, EntityKind::Subroutine);
        assert_eq!(entity.provenance, Provenance::DynamicBoundary);
        assert_eq!(entity.confidence, Confidence::Low);
        assert_eq!(occurrence.kind, OccurrenceKind::DynamicBoundary);
        assert_eq!(occurrence.entity_id, Some(entity.id));
        Ok(())
    }

    #[test]
    fn extracts_multiple_subs_from_eval_string() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(2);
        let triples = parse_and_extract(
            r#"eval "sub foo { 1 } sub bar { 2 }";"#,
            file_id,
        );

        assert_eq!(triples.len(), 2, "should extract two subs");
        let names: Vec<&str> = triples.iter().map(|(e, _, _)| e.canonical_name.as_str()).collect();
        assert!(names.contains(&"foo"), "should include 'foo'");
        assert!(names.contains(&"bar"), "should include 'bar'");
        Ok(())
    }

    #[test]
    fn non_literal_eval_does_not_produce_evidence() -> Result<(), Box<dyn std::error::Error>> {
        // `eval $code` — non-literal, must not emit evidence.
        let file_id = FileId(3);
        let triples = parse_and_extract(r#"eval $code;"#, file_id);
        assert!(triples.is_empty(), "non-literal eval must not produce evidence");
        Ok(())
    }

    #[test]
    fn eval_block_does_not_produce_evidence() -> Result<(), Box<dyn std::error::Error>> {
        // `eval { ... }` — block eval, must not emit evidence.
        let file_id = FileId(4);
        let triples = parse_and_extract(r#"eval { die "oops" };"#, file_id);
        assert!(triples.is_empty(), "block eval must not produce evidence");
        Ok(())
    }

    #[test]
    fn anonymous_sub_in_eval_does_not_produce_named_evidence()
    -> Result<(), Box<dyn std::error::Error>> {
        // `eval "sub { 1 }"` — anonymous sub, no name to extract.
        let file_id = FileId(5);
        let triples = parse_and_extract(r#"eval "sub { 1 }";"#, file_id);
        assert!(
            triples.is_empty(),
            "anonymous sub in eval must not produce named evidence"
        );
        Ok(())
    }

    #[test]
    fn stable_id_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
        let id1 = stable_id(1, 42, "foo");
        let id2 = stable_id(1, 42, "foo");
        assert_eq!(id1, id2, "stable_id must be deterministic");

        let id3 = stable_id(1, 42, "bar");
        assert_ne!(id1, id3, "different names must produce different IDs");
        Ok(())
    }
}
