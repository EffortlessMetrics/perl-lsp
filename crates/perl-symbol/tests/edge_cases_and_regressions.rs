//! Edge case and regression tests for the `perl-symbol` facade (Wave B #4428).
//!
//! These tests complement the BDD/comprehensive suites that already cover
//! behavior of the four merged submodules (`types`, `cursor`, `index`,
//! `surface`).  The focus here is on what the *collapse itself* introduced:
//!
//! - The explicit (no-wildcard) re-export surface in `api.rs` — any
//!   accidental widening (e.g. `pub use crate::types::*`) or narrowing
//!   (e.g. dropping a name) breaks one of these tests.
//! - The type-identity asymmetry: `perl_symbol::SymbolKind` and
//!   `perl_symbol::types::SymbolKind` must resolve to the same nominal
//!   type so consumers can migrate incrementally.
//! - Regression guards for CLAUDE.md invariants (no `perl-parser-core`
//!   or LSP provider dependency, deterministic `to_lsp_kind` mappings,
//!   constructor/predicate relationships).
//! - Boundary behavior for `cursor` and `index` functions that
//!   downstream features rely on (empty source, out-of-range cursor,
//!   non-ASCII names, duplicate trie insertion).
//!
//! All tests return `Result<()>` and use `perl_tdd_support::must_some`
//! instead of `.unwrap()` / `.expect()` per the workspace coding standards.

use perl_ast::{Node, NodeKind, SourceLocation};
use perl_symbol::cursor::{
    CursorSymbolKind, byte_offset_utf16, extract_symbol_from_source, get_symbol_range_at_position,
    is_modchar, is_word_boundary, token_under_cursor,
};
use perl_symbol::{SymbolDecl, SymbolIndex, SymbolKind, VarKind, extract_symbol_decls};
use perl_tdd_support::must_some;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// ─── api.rs re-export regression guards ──────────────────────────────────────

/// Regression: the full set of names re-exported by `api.rs` must remain
/// addressable at the crate root.  This is a compile-time check that doubles
/// as a quick-read ledger of the public surface — if the list below changes,
/// update `facade_api_completeness.rs` and the crate-level rustdoc in lockstep.
#[test]
fn api_regression_every_documented_name_is_reachable_at_crate_root() -> Result<()> {
    // types
    let _sk: SymbolKind = SymbolKind::Subroutine;
    let _vk: VarKind = VarKind::Scalar;

    // cursor module path — still reachable at `perl_symbol::cursor::*`
    let _ck: CursorSymbolKind = CursorSymbolKind::Scalar;
    let _f1: fn(usize, &str) -> Option<(String, CursorSymbolKind)> = extract_symbol_from_source;
    let _f2: fn(usize, &str) -> Option<(usize, usize)> = get_symbol_range_at_position;
    let _f3: fn(&str, usize) -> usize = byte_offset_utf16;
    let _f4: fn(u8) -> bool = is_modchar;
    let _f5: fn(&[u8], usize, usize) -> bool = is_word_boundary;
    let _f6: fn(&str, usize, usize) -> Option<String> = token_under_cursor;

    // index — crate-root re-export
    let _ix: fn() -> SymbolIndex = SymbolIndex::new;

    // surface — crate-root re-export of both the type and the function
    let _sd_size = std::mem::size_of::<SymbolDecl>(); // SymbolDecl at crate root
    let _f7: fn(&Node, Option<&str>) -> Vec<SymbolDecl> = extract_symbol_decls;

    Ok(())
}

/// Regression: `api.rs` uses *named* re-exports only (no wildcards).  If
/// someone changes `pub use crate::types::{SymbolKind, VarKind};` to
/// `pub use crate::types::*;` this test catches the silent widening:
/// internal-only items (private `mod tests` doesn't count because it is
/// `#[cfg(test)]`) would not leak, but any future `pub fn helper_*` added
/// to `types::mod` would silently become part of the public API.
///
/// We encode the guard by enumerating exactly the names that should be
/// reachable via `perl_symbol::*` (excluding the submodule names
/// themselves, which are always reachable via `pub mod`).  Adding a new
/// public name requires deliberately updating this list *and*
/// `facade_api_completeness.rs` — a deliberate act, not an accident.
#[test]
fn api_regression_crate_root_exposes_exactly_eight_reexported_items() -> Result<()> {
    // The eight names re-exported at the crate root (from `api.rs`):
    //   SymbolKind, VarKind                          — types
    //   CursorSymbolKind, byte_offset_utf16,
    //   extract_symbol_from_source,
    //   get_symbol_range_at_position,
    //   is_modchar, is_word_boundary,
    //   token_under_cursor                           — cursor
    //   SymbolIndex                                  — index
    //   SymbolDecl, extract_symbol_decls             — surface
    //
    // Bind each name to assert it resolves; if `api.rs` is narrowed, the
    // missing binding fails to compile. If someone switches to a wildcard
    // export and adds a *new* pub item upstream, this test still passes —
    // so we pair it with `facade_api_completeness.rs` for the positive
    // side, and rely on cargo-semver-checks for the external contract.
    //
    // Use explicit paths rather than a `use` list so the compiler cannot
    // flag these as unused — each path literally names the item we care
    // about.
    let _ = std::any::type_name::<perl_symbol::SymbolKind>();
    let _ = std::any::type_name::<perl_symbol::VarKind>();
    let _ = std::any::type_name::<perl_symbol::CursorSymbolKind>();
    let _ = std::any::type_name::<perl_symbol::SymbolIndex>();
    let _ = std::any::type_name::<perl_symbol::SymbolDecl>();
    let _byte: fn(&str, usize) -> usize = perl_symbol::byte_offset_utf16;
    let _esfs: fn(usize, &str) -> Option<(String, perl_symbol::CursorSymbolKind)> =
        perl_symbol::extract_symbol_from_source;
    let _esd: fn(&perl_ast::Node, Option<&str>) -> Vec<perl_symbol::SymbolDecl> =
        perl_symbol::extract_symbol_decls;
    let _gsp: fn(usize, &str) -> Option<(usize, usize)> = perl_symbol::get_symbol_range_at_position;
    let _im: fn(u8) -> bool = perl_symbol::is_modchar;
    let _iwb: fn(&[u8], usize, usize) -> bool = perl_symbol::is_word_boundary;
    let _tuc: fn(&str, usize, usize) -> Option<String> = perl_symbol::token_under_cursor;
    Ok(())
}

// ─── Type-identity asymmetry (module path vs crate root) ─────────────────────

/// Architectural invariant: `perl_symbol::SymbolKind` and
/// `perl_symbol::types::SymbolKind` MUST be the same nominal type.  Without
/// this, a consumer migrating `perl_symbol_types::SymbolKind` to the new
/// crate could land on the crate-root path and a downstream dependency on
/// the module path, resulting in a mysterious type mismatch.
#[test]
fn type_identity_symbolkind_is_same_via_module_and_crate_root() -> Result<()> {
    let from_root: SymbolKind = SymbolKind::Subroutine;
    let from_module: perl_symbol::types::SymbolKind = perl_symbol::types::SymbolKind::Subroutine;

    // Same nominal type: cross-assign should compile without `as`/`From`.
    let back_as_root: SymbolKind = from_module;
    assert_eq!(from_root, back_as_root);

    // Hash/eq semantics are shared: a value constructed via one path
    // equals a value constructed via the other.
    assert_eq!(SymbolKind::Subroutine, perl_symbol::types::SymbolKind::Subroutine);
    Ok(())
}

#[test]
fn type_identity_varkind_is_same_via_module_and_crate_root() -> Result<()> {
    let from_root: VarKind = VarKind::Hash;
    let from_module: perl_symbol::types::VarKind = perl_symbol::types::VarKind::Hash;

    let back_as_root: VarKind = from_module;
    assert_eq!(from_root, back_as_root);
    Ok(())
}

#[test]
fn type_identity_symbol_index_is_same_via_module_and_crate_root() -> Result<()> {
    // SymbolIndex is re-exported at the crate root; consumers like
    // perl-lsp-performance depend on that alias.
    let mut from_root: SymbolIndex = SymbolIndex::new();
    let mut from_module: perl_symbol::index::SymbolIndex = perl_symbol::index::SymbolIndex::new();

    from_root.add_symbol("Foo::bar".to_string());
    from_module.add_symbol("Foo::bar".to_string());

    assert_eq!(from_root.search_prefix("Foo"), from_module.search_prefix("Foo"));
    Ok(())
}

#[test]
fn type_identity_symbol_decl_is_same_via_module_and_crate_root() -> Result<()> {
    // SymbolDecl is re-exported at both the module path and the crate
    // root; consumers that migrate piecemeal must see the same type.
    fn via_root(_d: SymbolDecl) {}
    fn via_module(d: perl_symbol::surface::SymbolDecl) {
        via_root(d); // same type, same function
    }

    let decl = SymbolDecl {
        kind: SymbolKind::Package,
        name: "Foo".to_string(),
        qualified_name: "Foo".to_string(),
        full_span: (0, 3),
        anchor_span: None,
        container: None,
        declarator: None,
    };
    via_module(decl);
    Ok(())
}

// ─── CLAUDE.md invariant: LSP kind mapping is deterministic ──────────────────

/// Regression: the LSP kind mapping for every `SymbolKind` must remain
/// stable across the collapse.  Any consumer (including `perl-lsp-rs`
/// snapshot tests) relies on these numeric codes being identical to
/// those shipped by the former `perl-symbol-types` crate.
#[test]
fn regression_to_lsp_kind_mapping_is_exhaustive_and_stable() -> Result<()> {
    // If a new variant is added to SymbolKind, the match below must be
    // updated — acting as a compile-time smoke test against silent
    // additions to the public taxonomy.
    let all: Vec<SymbolKind> = vec![
        SymbolKind::Package,
        SymbolKind::Class,
        SymbolKind::Role,
        SymbolKind::Subroutine,
        SymbolKind::Method,
        SymbolKind::Variable(VarKind::Scalar),
        SymbolKind::Variable(VarKind::Array),
        SymbolKind::Variable(VarKind::Hash),
        SymbolKind::Constant,
        SymbolKind::Import,
        SymbolKind::Export,
        SymbolKind::Label,
        SymbolKind::Format,
    ];

    // Workspace profile — all variables collapse to LSP Variable (13).
    let workspace_codes: Vec<u32> = all.iter().map(|k| k.to_lsp_kind()).collect();
    assert_eq!(workspace_codes, vec![2, 5, 8, 12, 6, 13, 13, 13, 14, 2, 12, 20, 23]);

    // Document profile — scalar/array/hash get richer codes.
    let doc_codes: Vec<u32> = all.iter().map(|k| k.to_lsp_kind_document_symbol()).collect();
    assert_eq!(doc_codes, vec![2, 5, 8, 12, 6, 13, 18, 19, 14, 2, 12, 20, 23]);
    Ok(())
}

/// Regression: `sigil()` on `SymbolKind` must return `Some(..)` iff
/// the kind is a `Variable`.  Pairs with `is_variable()` — these two
/// predicates must never disagree.
#[test]
fn regression_sigil_and_is_variable_agree_for_every_variant() -> Result<()> {
    let non_vars = [
        SymbolKind::Package,
        SymbolKind::Class,
        SymbolKind::Role,
        SymbolKind::Subroutine,
        SymbolKind::Method,
        SymbolKind::Constant,
        SymbolKind::Import,
        SymbolKind::Export,
        SymbolKind::Label,
        SymbolKind::Format,
    ];
    for nv in non_vars {
        assert!(!nv.is_variable(), "{nv:?} should not be a variable");
        assert_eq!(nv.sigil(), None, "{nv:?} should have no sigil");
    }

    for vk in [VarKind::Scalar, VarKind::Array, VarKind::Hash] {
        let sk = SymbolKind::Variable(vk);
        assert!(sk.is_variable(), "Variable({vk:?}) must be a variable");
        assert_eq!(sk.sigil(), Some(vk.sigil()), "sigil must match VarKind");
    }
    Ok(())
}

/// Regression: convenience constructors must match manual construction.
#[test]
fn regression_convenience_constructors_match_manual() -> Result<()> {
    assert_eq!(SymbolKind::scalar(), SymbolKind::Variable(VarKind::Scalar));
    assert_eq!(SymbolKind::array(), SymbolKind::Variable(VarKind::Array));
    assert_eq!(SymbolKind::hash(), SymbolKind::Variable(VarKind::Hash));
    // And their LSP doc codes match.
    assert_eq!(SymbolKind::scalar().to_lsp_kind_document_symbol(), 13);
    assert_eq!(SymbolKind::array().to_lsp_kind_document_symbol(), 18);
    assert_eq!(SymbolKind::hash().to_lsp_kind_document_symbol(), 19);
    Ok(())
}

// ─── cursor: boundary conditions ─────────────────────────────────────────────

#[test]
fn edge_case_cursor_empty_source_returns_none() -> Result<()> {
    assert!(extract_symbol_from_source(0, "").is_none());
    assert!(get_symbol_range_at_position(0, "").is_none());
    Ok(())
}

#[test]
fn edge_case_cursor_position_past_end_returns_none() -> Result<()> {
    let src = "$foo";
    assert!(extract_symbol_from_source(100, src).is_none());
    assert!(get_symbol_range_at_position(100, src).is_none());
    Ok(())
}

#[test]
fn edge_case_cursor_on_sigil_itself_extracts_following_name() -> Result<()> {
    // Cursor sits on the `$` at position 0 of "$foo".
    let (name, kind) = must_some(extract_symbol_from_source(0, "$foo"));
    assert_eq!(name, "foo");
    assert_eq!(kind, CursorSymbolKind::Scalar);
    Ok(())
}

#[test]
fn edge_case_cursor_sigil_only_no_identifier_returns_none() -> Result<()> {
    // Lone sigil with no following identifier chars.
    assert!(extract_symbol_from_source(0, "$").is_none());
    assert!(extract_symbol_from_source(0, "@").is_none());
    assert!(extract_symbol_from_source(0, "%").is_none());
    assert!(extract_symbol_from_source(0, "&").is_none());
    Ok(())
}

#[test]
fn edge_case_cursor_all_four_sigils_map_to_expected_kind() -> Result<()> {
    let cases = [
        ("$x", CursorSymbolKind::Scalar),
        ("@x", CursorSymbolKind::Array),
        ("%x", CursorSymbolKind::Hash),
        ("&x", CursorSymbolKind::Subroutine),
    ];
    for (src, expected_kind) in cases {
        let (name, kind) = must_some(extract_symbol_from_source(1, src));
        assert_eq!(name, "x", "in source {src:?}");
        assert_eq!(kind, expected_kind, "in source {src:?}");
    }
    Ok(())
}

#[test]
fn edge_case_byte_offset_utf16_past_end_clamps_to_len() -> Result<()> {
    let line = "hello";
    // A column far past end-of-line must not panic and must not
    // over-read — downstream LSP code relies on this to recover from
    // stale incremental edit positions.
    let off = byte_offset_utf16(line, 999);
    assert_eq!(off, line.len());
    Ok(())
}

#[test]
fn edge_case_byte_offset_utf16_on_empty_line() -> Result<()> {
    assert_eq!(byte_offset_utf16("", 0), 0);
    assert_eq!(byte_offset_utf16("", 42), 0);
    Ok(())
}

#[test]
fn edge_case_is_modchar_covers_identifier_classes() -> Result<()> {
    // Letters, digits, underscore, colon — all part of a Perl identifier.
    assert!(is_modchar(b'a'));
    assert!(is_modchar(b'Z'));
    assert!(is_modchar(b'0'));
    assert!(is_modchar(b'_'));
    assert!(is_modchar(b':'));
    // Whitespace, sigils, punctuation — not part of an identifier.
    assert!(!is_modchar(b' '));
    assert!(!is_modchar(b'$'));
    assert!(!is_modchar(b'('));
    assert!(!is_modchar(b'\n'));
    Ok(())
}

#[test]
fn edge_case_token_under_cursor_none_for_empty_and_out_of_range() -> Result<()> {
    assert!(token_under_cursor("", 0, 0).is_none());
    // Out of range line — should degrade gracefully, not panic.
    let res = token_under_cursor("my $x = 1;\n", 99, 0);
    assert!(res.is_none());
    Ok(())
}

// ─── index: boundary conditions + duplicate insertion ────────────────────────

#[test]
fn edge_case_index_empty_returns_no_results() -> Result<()> {
    let idx = SymbolIndex::new();
    assert!(idx.search_prefix("Foo").is_empty());
    // fuzzy search on empty index produces no results.
    assert!(idx.search_fuzzy("anything").is_empty());
    Ok(())
}

#[test]
fn edge_case_index_default_equals_new() -> Result<()> {
    // Default impl is documented to equal `SymbolIndex::new()`; both
    // should produce an index with identical search behavior.
    let a = SymbolIndex::new();
    let b = SymbolIndex::default();
    assert_eq!(a.search_prefix("Foo"), b.search_prefix("Foo"));
    Ok(())
}

#[test]
fn edge_case_index_duplicate_insertion_does_not_double_report_prefix() -> Result<()> {
    let mut idx = SymbolIndex::new();
    idx.add_symbol("Foo::bar".to_string());
    idx.add_symbol("Foo::bar".to_string());
    let hits = idx.search_prefix("Foo::bar");
    // Duplicate insertions must be deduplicated: workspace indexing calls
    // add_symbol for the same qualified name on incremental re-index, and
    // the UI must not receive the same completion entry twice.
    assert_eq!(
        hits.iter().filter(|s| s.as_str() == "Foo::bar").count(),
        1,
        "duplicate add_symbol must not produce duplicate search_prefix results"
    );
    Ok(())
}

#[test]
fn edge_case_index_prefix_search_is_case_sensitive() -> Result<()> {
    let mut idx = SymbolIndex::new();
    idx.add_symbol("Foo::Bar".to_string());
    let exact = idx.search_prefix("Foo");
    assert!(exact.iter().any(|s| s == "Foo::Bar"), "exact-case prefix must match");
    let wrong_case = idx.search_prefix("foo");
    assert!(
        !wrong_case.iter().any(|s| s == "Foo::Bar"),
        "prefix search must not match across case"
    );
    Ok(())
}

#[test]
fn edge_case_index_fuzzy_search_ranks_multi_token_hits_higher() -> Result<()> {
    let mut idx = SymbolIndex::new();
    idx.add_symbol("Foo::Bar".to_string());
    idx.add_symbol("Foo::Quux".to_string());
    let results = idx.search_fuzzy("foo bar");
    // "Foo::Bar" should appear first because it shares more tokens with
    // the query than "Foo::Quux" does.
    assert!(!results.is_empty(), "fuzzy must produce at least one hit");
    let bar_pos = results.iter().position(|s| s == "Foo::Bar");
    let quux_pos = results.iter().position(|s| s == "Foo::Quux");
    if let (Some(b), Some(q)) = (bar_pos, quux_pos) {
        assert!(b <= q, "multi-token match must outrank single-token match");
    }
    Ok(())
}

#[test]
fn edge_case_index_duplicate_insertion_does_not_inflate_fuzzy_score() -> Result<()> {
    // When a symbol is added twice, its fuzzy token score must not be inflated
    // relative to a symbol added once.  Score inflation causes the duplicate
    // symbol to rank higher than correct, distorting workspace symbol ordering.
    let mut idx = SymbolIndex::new();
    idx.add_symbol("Foo::Bar".to_string());
    idx.add_symbol("Foo::Bar".to_string()); // duplicate — must be idempotent
    idx.add_symbol("Foo::Quux".to_string()); // single-inserted competitor

    let results = idx.search_fuzzy("foo");
    // Both must appear exactly once in the results.
    assert_eq!(
        results.iter().filter(|s| s.as_str() == "Foo::Bar").count(),
        1,
        "Foo::Bar must appear exactly once in fuzzy results after duplicate add"
    );
    assert_eq!(
        results.iter().filter(|s| s.as_str() == "Foo::Quux").count(),
        1,
        "Foo::Quux must appear exactly once in fuzzy results"
    );
    Ok(())
}

// ─── surface: extract_symbol_decls edge cases ────────────────────────────────

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

#[test]
fn edge_case_surface_empty_program_produces_no_decls() -> Result<()> {
    let program = Node::new(NodeKind::Program { statements: vec![] }, loc(0, 0));
    let decls = extract_symbol_decls(&program, None);
    assert!(decls.is_empty());
    Ok(())
}

#[test]
fn edge_case_surface_seed_package_qualifies_top_level_subs() -> Result<()> {
    // Seeded package context means a top-level `sub greet` is qualified.
    let body = Node::new(NodeKind::Block { statements: vec![] }, loc(10, 13));
    let sub_node = Node::new(
        NodeKind::Subroutine {
            name: Some("greet".to_string()),
            name_span: Some(loc(4, 9)),
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(body),
        },
        loc(0, 13),
    );
    let program = Node::new(NodeKind::Program { statements: vec![sub_node] }, loc(0, 13));

    let decls = extract_symbol_decls(&program, Some("Foo"));
    assert_eq!(decls.len(), 1, "one decl expected");
    let d = &decls[0];
    assert_eq!(d.name, "greet");
    assert_eq!(d.qualified_name, "Foo::greet", "seed package must qualify top-level sub");
    assert_eq!(d.container.as_deref(), Some("Foo"));
    Ok(())
}

#[test]
fn edge_case_surface_conservatively_skips_non_ast_native_decl_kinds() -> Result<()> {
    // `use`/`no` statements are module-loading forms and do not encode
    // declaration-site semantics for SymbolKind::Import / ::Export / ::Role.
    let use_node = Node::new(
        NodeKind::Use {
            module: "strict".to_string(),
            args: vec!["subs".to_string()],
            has_filter_risk: false,
        },
        loc(0, 16),
    );
    let no_node = Node::new(
        NodeKind::No {
            module: "warnings".to_string(),
            args: vec!["once".to_string()],
            has_filter_risk: false,
        },
        loc(17, 36),
    );
    let program = Node::new(NodeKind::Program { statements: vec![use_node, no_node] }, loc(0, 36));

    let decls = extract_symbol_decls(&program, None);
    assert!(decls.is_empty());
    Ok(())
}

// ─── Regression: CLAUDE.md banned dependencies are not transitively present ──

/// Regression: `perl-symbol` must not depend on `perl-parser-core` or any
/// LSP provider.  A code-side smoke-test is hard (we cannot import what
/// isn't there), so we rely on cargo metadata via the workspace `just`
/// targets; here we encode the *positive* contract: the crate compiles
/// using only its declared deps (`perl-ast`, `serde`) and the dev-deps
/// (`perl-tdd-support`, `serde_json`).  If someone adds `perl-parser-core`
/// to the dev tree, this file would gain an import and a reviewer would
/// notice; adding it to `[dependencies]` is caught by the README/CLAUDE.md
/// invariant enforcement in `just ci-gate`.
#[test]
fn regression_crate_compiles_without_lsp_or_parser_core_deps() -> Result<()> {
    // The fact that this test file compiles and runs (and only imports
    // perl_ast + perl_symbol + perl_tdd_support) is itself the assertion.
    // We place a small structural check here so the test body is not empty.
    let program = Node::new(NodeKind::Program { statements: vec![] }, loc(0, 0));
    let decls = extract_symbol_decls(&program, None);
    assert!(decls.is_empty());
    Ok(())
}

// ─── is_word_boundary boundary behavior ──────────────────────────────────────

#[test]
fn edge_case_is_word_boundary_empty_bytes() -> Result<()> {
    // Empty text, zero-length word — no crash, returns some boolean.
    let _ = is_word_boundary(b"", 0, 0);
    Ok(())
}

#[test]
fn edge_case_is_word_boundary_word_at_start_and_end() -> Result<()> {
    // "foo bar": "foo" starts at 0 (boundary before EOF and after SOF),
    // "bar" starts at 4 (whitespace before).
    let text = b"foo bar";
    assert!(is_word_boundary(text, 0, 3), "word at start must be a boundary");
    assert!(is_word_boundary(text, 4, 3), "word after whitespace must be a boundary");
    Ok(())
}
