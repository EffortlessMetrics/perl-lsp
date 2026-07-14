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
//! # Placement note — circular dependency debt
//!
//! This extractor lives in `perl-workspace` rather than `perl-semantic-analyzer`
//! because of a circular dependency: `perl-semantic-analyzer/Cargo.toml` declares
//! `perl-workspace` as a dependency (for workspace indexing), so moving any
//! producer into `perl-semantic-analyzer` would create a cycle.
//!
//! This is **temporary architectural debt**. The correct long-term placement is
//! `perl-semantic-analyzer`, which owns the semantic production layer.
//! The blocker is the current `perl-semantic-analyzer → perl-workspace` dep arc.
//!
//! **Follow-up**: invert or remove the `perl-semantic-analyzer → perl-workspace`
//! dependency (possibly by introducing a `perl-workspace-types` leaf crate for
//! the fact types), then move this extractor to `perl-semantic-analyzer`.
//! Track this as a follow-up issue after the dynamic-boundary suppression PRs merge.
//!
//! # Requirements
//!
//! - **Req 7.5a**: Emit `DynamicBoundary` evidence for `eval "sub NAME { ... }"`
//!   so that `dynamic_callable_may_be_visible_at` can suppress the
//!   `UnquotedBareword` diagnostic for `NAME` at later call sites in the
//!   same file.

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
            extract_from_eval_string(value, node.location.start, node.location.end, file_id, out);
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
/// Handles plausible Perl sub declarations of the form:
/// - `sub NAME {`   — named sub with body
/// - `sub NAME ;`   — forward declaration
/// - `sub NAME (`   — named sub with prototype/signature
///
/// Also recognises `package NAME;` declarations within the eval string and
/// attributes subsequently-declared subs to that package.  For example,
/// `eval "package Foo; sub bar { 1 }"` emits a triple with
/// `canonical_name = "Foo::bar"`.  Multiple package switches within one
/// eval string are supported.
///
/// Does NOT match:
/// - `sub { ... }` — anonymous sub (no name)
/// - `sub $name { ... }` — interpolated name (sigil-prefixed)
/// - `sub NAME` followed by arbitrary text (conservative: reject if no
///   plausible Perl delimiter follows)
///
/// This conservative approach avoids false positives from strings that
/// contain the word `sub` in prose (e.g. `"no sub here really"`).
fn extract_from_eval_string(
    eval_string: &str,
    node_start_byte: usize,
    node_end_byte: usize,
    file_id: FileId,
    out: &mut Vec<(EntityFact, AnchorFact, OccurrenceFact)>,
) {
    // Strip surrounding quotes if present (the parser may or may not include them).
    let content = eval_string
        .trim_start_matches('"')
        .trim_end_matches('"')
        .trim_start_matches('\'')
        .trim_end_matches('\'');

    // Pre-scan for `package NAME;` declarations in the entire eval string.
    // `package_decls` is a list of (decl_end_offset, package_name) pairs, sorted
    // by position.  `decl_end_offset` is the absolute byte offset in `content`
    // just after the package name (before the `;` or `{`), so we can determine
    // which package was active at any given `sub` position via a simple filter.
    let package_decls = find_package_declarations(content);

    // Scan for `sub IDENTIFIER` patterns in the string content.
    let mut search = content;
    let mut search_base = 0usize; // absolute offset of `search`'s start within `content`

    while !search.is_empty() {
        // Find the next `sub ` keyword.
        let Some(sub_pos) = find_sub_keyword(search) else {
            break;
        };

        // Absolute position of this `sub` keyword within `content`.
        let abs_sub_pos = search_base + sub_pos;

        let after_sub = &search[sub_pos + 3..]; // skip "sub"

        // Skip whitespace between `sub` and the name.
        let ws_len =
            after_sub.len() - after_sub.trim_start_matches(|c: char| c.is_ascii_whitespace()).len();
        let after_ws = &after_sub[ws_len..];

        // Reject: anonymous sub (`sub {`) or sigil-prefixed (`sub $name`).
        if after_ws.starts_with('{') || after_ws.starts_with(['$', '@', '%', '&', '*']) {
            let advance = sub_pos + 3 + ws_len.max(1);
            if advance >= search.len() {
                break;
            }
            search = &search[advance..];
            search_base += advance;
            continue;
        }

        // Extract the identifier name.
        let name_len = after_ws
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .unwrap_or(after_ws.len());

        if name_len > 0 {
            let name = &after_ws[..name_len];
            // Validate: must start with a letter or underscore (not a digit).
            if name.as_bytes().first().is_some_and(|&b| b.is_ascii_alphabetic() || b == b'_') {
                // Validate: what follows the name must look like a Perl sub declaration.
                // Accept: `{`, `;`, `(` (optionally preceded by whitespace).
                // - `sub NAME {`   — named sub with body
                // - `sub NAME ;`   — forward declaration
                // - `sub NAME (`   — named sub with prototype or signature
                // Reject everything else, including bare `sub NAME` at end-of-string
                // (ambiguous — could be prose containing the word "sub").
                let after_name = after_ws[name_len..].trim_start();
                let plausible = after_name.starts_with('{')
                    || after_name.starts_with(';')
                    || after_name.starts_with('(');
                if plausible {
                    // Determine the active package at this `sub` position.
                    // The active package is the one whose declaration ended most
                    // recently before (or at) this `sub` keyword.
                    let active_package = package_decls
                        .iter()
                        .rfind(|(end, _)| *end <= abs_sub_pos)
                        .map(|(_, pkg)| pkg.as_str());

                    emit_triple(name, active_package, node_start_byte, node_end_byte, file_id, out);
                }
            }
        }

        // Advance past the name to continue scanning.
        let advance = sub_pos + 3 + ws_len + name_len.max(1);
        if advance >= search.len() {
            break;
        }
        search = &search[advance..];
        search_base += advance;
    }
}

/// Find the byte offset of the next `package` keyword in `text` that is
/// preceded by a word boundary (not part of a longer identifier).
///
/// `package` must be followed by ASCII whitespace (not end-of-string), since
/// bare `package;` (unnamed package) is not a named package declaration.
fn find_package_keyword(text: &str) -> Option<usize> {
    let mut start = 0;
    while start < text.len() {
        let pos = text[start..].find("package")?;
        let abs_pos = start + pos;

        // Left boundary: at start or preceded by a non-word character.
        let left_ok = abs_pos == 0 || {
            let b = text.as_bytes()[abs_pos - 1];
            !b.is_ascii_alphanumeric() && b != b'_'
        };

        // Right boundary: must be followed by ASCII whitespace (not end-of-string).
        let right_byte = text.as_bytes().get(abs_pos + 7).copied();
        let right_ok = right_byte.is_some_and(|b| b.is_ascii_whitespace());

        if left_ok && right_ok {
            return Some(abs_pos);
        }

        start = abs_pos + 7;
    }
    None
}

/// Return true if `name` is a syntactically valid Perl package name.
///
/// Valid: `Foo`, `Foo::Bar`, `Foo::Bar::Baz`, `_Private`.
/// Invalid: `::Foo`, `Foo::`, `Foo:::Bar`.
fn is_valid_package_name(name: &str) -> bool {
    !name.starts_with(':')
        && !name.ends_with(':')
        && !name.contains(":::")
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
}

/// Pre-scan `content` for `package NAME;` (or `package NAME {`) declarations.
///
/// Returns a list of `(decl_end_offset, package_name)` pairs in order of
/// appearance, where `decl_end_offset` is the absolute byte offset in
/// `content` just after the package name (before the terminating `;` or `{`).
///
/// This offset is used in [`extract_from_eval_string`] to determine which
/// package was active at a given `sub` declaration position: the active
/// package is the last entry whose `decl_end_offset` is ≤ the `sub` position.
///
/// # Limitations
///
/// Block-scoped packages (`package Foo { ... }`) are detected as activating
/// `Foo` but the scope end (closing `}`) is not tracked.  In practice, eval
/// strings almost always use the semicolon form.
fn find_package_declarations(content: &str) -> Vec<(usize, String)> {
    let mut packages = Vec::new();
    let mut search = content;
    let mut base_offset = 0usize;

    while !search.is_empty() {
        let Some(pkg_pos) = find_package_keyword(search) else {
            break;
        };

        let after_pkg = &search[pkg_pos + 7..]; // skip "package"
        let abs_pkg_end = base_offset + pkg_pos + 7;

        // Skip whitespace between `package` and the name.
        let ws_len =
            after_pkg.len() - after_pkg.trim_start_matches(|c: char| c.is_ascii_whitespace()).len();
        let after_ws = &after_pkg[ws_len..];

        // Extract the package name: alphanumeric, `_`, and `::`.
        let name_len = after_ws
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != ':')
            .unwrap_or(after_ws.len());

        if name_len > 0 {
            let name = &after_ws[..name_len];
            // Must start with a letter or underscore (not a digit or colon).
            let valid_start =
                name.as_bytes().first().is_some_and(|&b| b.is_ascii_alphabetic() || b == b'_');
            if valid_start && is_valid_package_name(name) {
                // The token following the name (after optional whitespace) must be
                // `;` (statement form) or `{` (block form).
                let after_name = after_ws[name_len..].trim_start();
                if after_name.starts_with(';') || after_name.starts_with('{') {
                    // `decl_end` is just after the name, before the `;` or `{`.
                    let decl_end = abs_pkg_end + ws_len + name_len;
                    packages.push((decl_end, name.to_string()));
                }
            }
        }

        // Advance past this `package` occurrence.
        let advance = pkg_pos + 7 + ws_len + name_len.max(1);
        if advance >= search.len() {
            break;
        }
        search = &search[advance..];
        base_offset += advance;
    }

    packages
}

/// Find the byte offset of the next `sub` keyword in `text` that is preceded
/// by a word boundary (not part of a longer identifier like `suburb`).
fn find_sub_keyword(text: &str) -> Option<usize> {
    let mut start = 0;
    while start < text.len() {
        let pos = text[start..].find("sub")?;
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
///
/// `node_start_byte` and `node_end_byte` are from the enclosing `Eval` AST
/// node's `location.start` and `location.end` — these are the real source
/// positions of the eval expression, used directly as the anchor span.
///
/// When `package` is `Some("Foo")`, the entity's `canonical_name` is emitted
/// as `"Foo::name"` (package-qualified), which is how the WorkspaceIndex and
/// dual-indexing machinery will find it under both `Foo::name` and bare `name`.
fn emit_triple(
    name: &str,
    package: Option<&str>,
    node_start_byte: usize,
    node_end_byte: usize,
    file_id: FileId,
    out: &mut Vec<(EntityFact, AnchorFact, OccurrenceFact)>,
) {
    // Build the qualified canonical name when a package is active.
    let canonical_name = match package {
        Some(pkg) => format!("{pkg}::{name}"),
        None => name.to_string(),
    };

    // Stable ID derivation: hash (file_id, node_start_byte, canonical_name).
    // Including the full qualified name prevents collisions between Foo::bar and
    // Bar::bar emitted from different package contexts within the same eval node.
    let base_id = stable_id(file_id.0, node_start_byte as u64, &canonical_name);

    let entity_id = EntityId(base_id);
    let anchor_id = AnchorId(base_id + 1);
    let occurrence_id = OccurrenceId(base_id + 2);

    let entity = EntityFact {
        id: entity_id,
        canonical_name,
        kind: EntityKind::Subroutine,
        anchor_id: Some(anchor_id),
        scope_id: None,
        provenance: Provenance::DynamicBoundary,
        confidence: Confidence::Low,
    };

    // Use the real AST span from the enclosing eval node.
    // node_end_byte comes from node.location.end, which is the source position
    // of the end of the entire eval expression (including closing quote/paren).
    let span_end =
        if node_end_byte > node_start_byte { node_end_byte } else { node_start_byte + 1 };
    let anchor = AnchorFact {
        id: anchor_id,
        file_id,
        span_start_byte: node_start_byte as u32,
        span_end_byte: span_end as u32,
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

    fn parse_and_extract(
        code: &str,
        file_id: FileId,
    ) -> Vec<(EntityFact, AnchorFact, OccurrenceFact)> {
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
        let triples = parse_and_extract(r#"eval "sub generated_from_string { 1 }";"#, file_id);

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
        let triples = parse_and_extract(r#"eval "sub foo { 1 } sub bar { 2 }";"#, file_id);

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
        assert!(triples.is_empty(), "anonymous sub in eval must not produce named evidence");
        Ok(())
    }

    #[test]
    fn prose_sub_in_eval_does_not_produce_evidence() -> Result<(), Box<dyn std::error::Error>> {
        // A string that contains the word "sub" in prose should not produce evidence.
        // "no sub here really sub baz" has no Perl declaration delimiters after the name.
        let file_id = FileId(6);
        // Parse as a Perl string literal rather than through the parser to test
        // the extractor function directly.
        let triples = {
            let mut out = Vec::new();
            extract_from_eval_string("no sub here really sub baz", 0, 26, file_id, &mut out);
            out
        };
        assert!(
            triples.is_empty(),
            "prose containing 'sub' without delimiters must not produce evidence, got: {:?}",
            triples.iter().map(|(e, _, _)| e.canonical_name.as_str()).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn sub_with_semicolon_delimiter_is_accepted() -> Result<(), Box<dyn std::error::Error>> {
        // Forward declaration: `sub foo;`
        let file_id = FileId(7);
        let triples = {
            let mut out = Vec::new();
            extract_from_eval_string("sub forward_decl;", 0, 18, file_id, &mut out);
            out
        };
        assert_eq!(triples.len(), 1, "sub NAME; (forward decl) should produce evidence");
        assert_eq!(triples[0].0.canonical_name, "forward_decl");
        Ok(())
    }

    #[test]
    fn sub_with_prototype_is_accepted() -> Result<(), Box<dyn std::error::Error>> {
        // Named sub with prototype: `sub proto_sub ($$) { }`
        let file_id = FileId(8);
        let triples = {
            let mut out = Vec::new();
            extract_from_eval_string("sub proto_sub ($$) { 1 }", 0, 24, file_id, &mut out);
            out
        };
        assert_eq!(triples.len(), 1, "sub NAME (proto) should produce evidence");
        assert_eq!(triples[0].0.canonical_name, "proto_sub");
        Ok(())
    }

    #[test]
    fn interpolated_name_sub_does_not_produce_evidence() -> Result<(), Box<dyn std::error::Error>> {
        // `sub $name { ... }` — dynamic name, cannot be extracted.
        let file_id = FileId(9);
        let triples = {
            let mut out = Vec::new();
            extract_from_eval_string("sub $dynamic_name { 1 }", 0, 23, file_id, &mut out);
            out
        };
        assert!(triples.is_empty(), "sub with sigil-prefixed name must not produce evidence");
        Ok(())
    }

    #[test]
    fn whitespace_variants_between_keyword_and_name_are_accepted()
    -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(10);
        let triples = {
            let mut out = Vec::new();
            extract_from_eval_string(
                "sub\twith_tab { 1 } sub\nwith_newline; sub\r\nwith_crlf () { 1 }",
                4,
                67,
                file_id,
                &mut out,
            );
            out
        };

        let names: Vec<&str> =
            triples.iter().map(|(entity, _, _)| entity.canonical_name.as_str()).collect();
        assert_eq!(names, vec!["with_tab", "with_newline", "with_crlf"]);
        Ok(())
    }

    #[test]
    fn invalid_candidate_does_not_stop_later_valid_declaration()
    -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(11);
        let triples = {
            let mut out = Vec::new();
            extract_from_eval_string(
                "sub 123_not_a_name { 1 } sub $dynamic { 2 } sub valid_after_invalid { 3 }",
                0,
                73,
                file_id,
                &mut out,
            );
            out
        };

        assert_eq!(triples.len(), 1, "only the later static declaration should match");
        assert_eq!(triples[0].0.canonical_name, "valid_after_invalid");
        Ok(())
    }

    #[test]
    fn keyword_boundaries_reject_identifier_fragments() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(find_sub_keyword("gsub name { 1 }"), None);
        assert_eq!(find_sub_keyword("prefix_sub name { 1 }"), None);
        assert_eq!(find_sub_keyword("subroutine name { 1 }"), None);
        assert_eq!(find_sub_keyword("sub:name { 1 }"), None);
        Ok(())
    }

    #[test]
    fn unsupported_declaration_shapes_are_ignored() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(12);
        let triples = {
            let mut out = Vec::new();
            extract_from_eval_string(
                "sub bare_name sub namespaced::method { 1 } sub hyphen-name { 2 } sub _ok { 3 }",
                0,
                78,
                file_id,
                &mut out,
            );
            out
        };

        assert_eq!(triples.len(), 1, "only the simple identifier declaration is supported");
        assert_eq!(triples[0].0.canonical_name, "_ok");
        Ok(())
    }

    #[test]
    fn emit_triple_normalizes_empty_or_reversed_spans() -> Result<(), Box<dyn std::error::Error>> {
        let mut out = Vec::new();
        emit_triple("span_edge", None, 99, 42, FileId(13), &mut out);

        assert_eq!(out.len(), 1);
        let (_entity, anchor, occurrence) = &out[0];
        assert_eq!(anchor.span_start_byte, 99);
        assert_eq!(anchor.span_end_byte, 100);
        assert_eq!(occurrence.anchor_id, anchor.id);
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

    // ── Unit tests for package-context attribution ──

    #[test]
    fn find_package_keyword_basic() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(find_package_keyword("package Foo;"), Some(0));
        assert_eq!(find_package_keyword("  package Foo;"), Some(2));
        assert_eq!(find_package_keyword("sub x; package Bar;"), Some(7));
        Ok(())
    }

    #[test]
    fn find_package_keyword_requires_whitespace_after() -> Result<(), Box<dyn std::error::Error>> {
        // `package;` is valid Perl (unnamed) but not a named package decl — must not match.
        assert_eq!(find_package_keyword("package;"), None);
        // `packages` is an identifier — must not match.
        assert_eq!(find_package_keyword("packages Foo;"), None);
        // `unpackaged` — must not match.
        assert_eq!(find_package_keyword("unpackaged Foo;"), None);
        Ok(())
    }

    #[test]
    fn is_valid_package_name_accepts_valid_names() -> Result<(), Box<dyn std::error::Error>> {
        assert!(is_valid_package_name("Foo"));
        assert!(is_valid_package_name("Foo::Bar"));
        assert!(is_valid_package_name("Foo::Bar::Baz"));
        assert!(is_valid_package_name("_Private"));
        assert!(is_valid_package_name("My::Module::Utils"));
        Ok(())
    }

    #[test]
    fn is_valid_package_name_rejects_invalid_names() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!is_valid_package_name("::Foo"));
        assert!(!is_valid_package_name("Foo::"));
        assert!(!is_valid_package_name("Foo:::Bar"));
        assert!(!is_valid_package_name("Foo::Bar::"));
        Ok(())
    }

    #[test]
    fn find_package_declarations_basic() -> Result<(), Box<dyn std::error::Error>> {
        let decls = find_package_declarations("package Foo; sub bar { }");
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].1, "Foo");
        // decl_end is just after "Foo" (before ";"), i.e. offset 11
        // "package Foo" — 'p' at 0, 'F' at 8, 'o' at 10, decl_end = 11
        assert_eq!(decls[0].0, 11, "decl_end should be just after the package name");
        Ok(())
    }

    #[test]
    fn find_package_declarations_multiple_packages() -> Result<(), Box<dyn std::error::Error>> {
        let decls = find_package_declarations("package A; sub x { } package B; sub y { }");
        assert_eq!(decls.len(), 2);
        assert_eq!(decls[0].1, "A");
        assert_eq!(decls[1].1, "B");
        Ok(())
    }

    #[test]
    fn find_package_declarations_multipart_name() -> Result<(), Box<dyn std::error::Error>> {
        let decls = find_package_declarations("package Foo::Bar; sub baz { }");
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].1, "Foo::Bar");
        Ok(())
    }

    #[test]
    fn find_package_declarations_empty_content() -> Result<(), Box<dyn std::error::Error>> {
        assert!(find_package_declarations("sub foo { }").is_empty());
        assert!(find_package_declarations("").is_empty());
        Ok(())
    }

    // ── Integration: package-qualified extraction ──

    #[test]
    fn sub_in_package_context_is_qualified() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(20);
        let triples = {
            let mut out = Vec::new();
            extract_from_eval_string("package Foo; sub bar { 1 }", 0, 26, file_id, &mut out);
            out
        };

        assert_eq!(triples.len(), 1, "should extract one sub");
        assert_eq!(
            triples[0].0.canonical_name, "Foo::bar",
            "sub in package context must be package-qualified"
        );
        assert_eq!(triples[0].0.kind, EntityKind::Subroutine);
        assert_eq!(triples[0].0.provenance, Provenance::DynamicBoundary);
        Ok(())
    }

    #[test]
    fn multiple_subs_same_package() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(21);
        let triples = {
            let mut out = Vec::new();
            extract_from_eval_string(
                "package Foo; sub alpha { 1 } sub beta { 2 }",
                0,
                44,
                file_id,
                &mut out,
            );
            out
        };

        assert_eq!(triples.len(), 2);
        let names: Vec<&str> = triples.iter().map(|(e, _, _)| e.canonical_name.as_str()).collect();
        assert!(names.contains(&"Foo::alpha"), "alpha must be Foo-qualified");
        assert!(names.contains(&"Foo::beta"), "beta must be Foo-qualified");
        Ok(())
    }

    #[test]
    fn multiple_packages_in_one_eval() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(22);
        let triples = {
            let mut out = Vec::new();
            extract_from_eval_string(
                "package A; sub x { } package B; sub y { }",
                0,
                42,
                file_id,
                &mut out,
            );
            out
        };

        assert_eq!(triples.len(), 2);
        let names: Vec<&str> = triples.iter().map(|(e, _, _)| e.canonical_name.as_str()).collect();
        assert!(names.contains(&"A::x"), "x must be attributed to package A");
        assert!(names.contains(&"B::y"), "y must be attributed to package B");
        Ok(())
    }

    #[test]
    fn sub_before_any_package_remains_unscoped() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(23);
        let triples = {
            let mut out = Vec::new();
            extract_from_eval_string(
                "sub before { 1 } package Foo; sub after { 2 }",
                0,
                46,
                file_id,
                &mut out,
            );
            out
        };

        assert_eq!(triples.len(), 2);
        let names: Vec<&str> = triples.iter().map(|(e, _, _)| e.canonical_name.as_str()).collect();
        assert!(names.contains(&"before"), "sub before any package must be unscoped");
        assert!(names.contains(&"Foo::after"), "sub after package Foo must be qualified");
        Ok(())
    }

    #[test]
    fn multipart_package_name_qualifies_sub() -> Result<(), Box<dyn std::error::Error>> {
        let file_id = FileId(24);
        let triples = {
            let mut out = Vec::new();
            extract_from_eval_string("package Foo::Bar; sub baz { 1 }", 0, 31, file_id, &mut out);
            out
        };

        assert_eq!(triples.len(), 1);
        assert_eq!(triples[0].0.canonical_name, "Foo::Bar::baz");
        Ok(())
    }

    #[test]
    fn package_context_via_ast_parse() -> Result<(), Box<dyn std::error::Error>> {
        // End-to-end: parse Perl source, extract from eval, verify package qualification.
        let file_id = FileId(25);
        let triples = parse_and_extract(r#"eval "package Dynamic; sub generated { 1 }";"#, file_id);

        assert_eq!(triples.len(), 1, "should extract one sub");
        assert_eq!(triples[0].0.canonical_name, "Dynamic::generated");
        Ok(())
    }

    #[test]
    fn package_switch_mid_eval_via_ast_parse() -> Result<(), Box<dyn std::error::Error>> {
        // Call-observation test: exercises the real production entry point
        // (extract_eval_sub_boundaries -> walk -> extract_from_eval_string) with
        // a mid-eval package switch, rather than calling extract_from_eval_string
        // directly with a synthetic string. Covers the package-active path
        // through the actual AST-driven `Eval` node.
        let file_id = FileId(28);
        let triples = parse_and_extract(
            r#"eval "package A; sub make_a { 1 } package B; sub make_b { 2 }";"#,
            file_id,
        );

        assert_eq!(triples.len(), 2, "should extract two subs from two packages");
        let names: Vec<&str> = triples.iter().map(|(e, _, _)| e.canonical_name.as_str()).collect();
        assert_eq!(
            names,
            vec!["A::make_a", "B::make_b"],
            "each sub must be attributed to its own active package, in source order"
        );
        Ok(())
    }

    #[test]
    fn sub_before_package_remains_unscoped_via_ast_parse() -> Result<(), Box<dyn std::error::Error>>
    {
        // Call-observation test for the unscoped-before-package path: no `package`
        // keyword precedes the `sub`, so the active_package lookup (rfind over an
        // empty/irrelevant package_decls list) must yield None and the canonical
        // name must remain the bare sub name.
        let file_id = FileId(29);
        let triples = parse_and_extract(r#"eval "sub bare_helper { 42 }";"#, file_id);

        assert_eq!(triples.len(), 1, "should extract exactly one sub");
        assert_eq!(
            triples[0].0.canonical_name, "bare_helper",
            "sub with no preceding package declaration must be unscoped"
        );
        Ok(())
    }

    #[test]
    fn three_packages_middle_sub_uses_nearest_preceding_package()
    -> Result<(), Box<dyn std::error::Error>> {
        // Direct test of the multi-package rfind path in extract_from_eval_string:
        // with three package declarations, a sub positioned between the 2nd and
        // 3rd package must attribute to the 2nd (nearest preceding), not the 3rd
        // (which comes later in the string and must be skipped by `rfind`'s
        // predicate, not blindly picked as "the last package in the list").
        let file_id = FileId(30);
        let triples = {
            let mut out = Vec::new();
            extract_from_eval_string(
                "package A; sub a { 1 } package B; sub b { 2 } package C; sub c { 3 }",
                0,
                70,
                file_id,
                &mut out,
            );
            out
        };

        assert_eq!(triples.len(), 3, "should extract three subs");
        let names: Vec<&str> = triples.iter().map(|(e, _, _)| e.canonical_name.as_str()).collect();
        assert_eq!(
            names,
            vec!["A::a", "B::b", "C::c"],
            "each sub must attribute to the nearest preceding package, not a later one"
        );
        Ok(())
    }

    #[test]
    fn three_packages_middle_sub_via_ast_parse() -> Result<(), Box<dyn std::error::Error>> {
        // Same scenario as above but via the real production entry point
        // (extract_eval_sub_boundaries -> walk), to confirm the rfind-skip
        // behavior holds through the full AST-driven call path, not just the
        // directly-invoked helper.
        let file_id = FileId(31);
        let triples = parse_and_extract(
            r#"eval "package A; sub a { 1 } package B; sub b { 2 } package C; sub c { 3 }";"#,
            file_id,
        );

        assert_eq!(triples.len(), 3, "should extract three subs");
        let names: Vec<&str> = triples.iter().map(|(e, _, _)| e.canonical_name.as_str()).collect();
        assert_eq!(
            names,
            vec!["A::a", "B::b", "C::c"],
            "middle sub must attribute to its nearest preceding package via the real AST path"
        );
        Ok(())
    }

    #[test]
    fn distinct_packages_via_ast_parse_produce_distinct_ids()
    -> Result<(), Box<dyn std::error::Error>> {
        // Call-observation test for the id-hash-includes-qualified-name path:
        // two subs with the *same* bare name but different active packages,
        // reached through the real AST/eval-string pipeline, must resolve to
        // distinct canonical names and distinct entity IDs (stable_id folds the
        // qualified name into the hash, so `Foo::bar` and `Bar::bar` differ).
        let file_id = FileId(32);
        let triples = parse_and_extract(
            r#"eval "package Foo; sub bar { 1 } package Bar; sub bar { 2 }";"#,
            file_id,
        );

        assert_eq!(triples.len(), 2, "should extract two subs");
        assert_eq!(triples[0].0.canonical_name, "Foo::bar");
        assert_eq!(triples[1].0.canonical_name, "Bar::bar");
        assert_ne!(
            triples[0].0.id, triples[1].0.id,
            "Foo::bar and Bar::bar reached via the real AST path must have distinct entity IDs"
        );
        Ok(())
    }

    #[test]
    fn package_qualified_ids_differ_from_unqualified() -> Result<(), Box<dyn std::error::Error>> {
        // Foo::bar and bar must produce different IDs even at the same node position.
        let mut out_qualified = Vec::new();
        emit_triple("bar", Some("Foo"), 0, 10, FileId(26), &mut out_qualified);

        let mut out_bare = Vec::new();
        emit_triple("bar", None, 0, 10, FileId(26), &mut out_bare);

        assert_ne!(
            out_qualified[0].0.id, out_bare[0].0.id,
            "Foo::bar and bare bar must have distinct entity IDs"
        );
        Ok(())
    }

    #[test]
    fn two_packages_in_same_eval_have_distinct_ids() -> Result<(), Box<dyn std::error::Error>> {
        // Foo::bar and Bar::bar at the same eval node position must not collide.
        let mut out_foo = Vec::new();
        emit_triple("bar", Some("Foo"), 0, 10, FileId(27), &mut out_foo);

        let mut out_bar_pkg = Vec::new();
        emit_triple("bar", Some("Bar"), 0, 10, FileId(27), &mut out_bar_pkg);

        assert_ne!(
            out_foo[0].0.id, out_bar_pkg[0].0.id,
            "Foo::bar and Bar::bar must have distinct entity IDs"
        );
        Ok(())
    }
}
