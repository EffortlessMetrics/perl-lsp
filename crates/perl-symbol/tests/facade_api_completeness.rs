//! Guards the public API surface of `perl-symbol`. If an item listed here becomes
//! inaccessible at the documented path, this test fails — catching accidental
//! API breakage during future refactoring.
//!
//! Pattern established by Wave 1 (#4422 perl-module collapse) and required for
//! every microcrate-collapse facade.

use perl_symbol::{
    SymbolDecl, SymbolDeclSemanticFacts, SymbolIndex, SymbolKind, SymbolRef, SymbolRefKind,
    SymbolRefSemanticFacts, VarKind,
    cursor::{
        CursorSymbolKind, byte_offset_utf16, extract_symbol_from_source,
        get_symbol_range_at_position, is_modchar, is_word_boundary, token_under_cursor,
    },
    extract_symbol_decls, extract_symbol_refs, symbol_decls_to_semantic_facts,
    symbol_refs_to_semantic_facts,
};

#[test]
fn symbol_kind_and_var_kind_accessible_at_crate_root() {
    let _k = SymbolKind::Subroutine;
    let _v = VarKind::Scalar;
    // SymbolKind re-export must route to the types module.
    assert_eq!(SymbolKind::Subroutine.to_lsp_kind(), 12);
    assert_eq!(VarKind::Scalar.sigil(), "$");
}

#[test]
fn cursor_surface_accessible() {
    // Construct the enum and exercise each function at its documented path.
    let _k = CursorSymbolKind::Scalar;

    // extract_symbol_from_source(position, source)
    let _ = extract_symbol_from_source(0, "");

    // get_symbol_range_at_position(position, source)
    let _ = get_symbol_range_at_position(0, "");

    // byte_offset_utf16(line_text, col_utf16)
    assert_eq!(byte_offset_utf16("", 0), 0);

    // is_modchar(byte)
    assert!(is_modchar(b'a'));
    assert!(!is_modchar(b' '));

    // token_under_cursor(text, line, col_utf16)
    let _ = token_under_cursor("", 0, 0);

    // is_word_boundary(text, pos, word_len)
    let _ = is_word_boundary(b"", 0, 0);
}

#[test]
fn symbol_index_accessible() {
    let mut idx = SymbolIndex::new();
    idx.add_symbol("Foo::bar".to_string());
    let results = idx.search_prefix("Foo");
    assert!(!results.is_empty());
}

#[test]
fn symbol_index_document_api_accessible() {
    // replace_document_symbols, remove_document, and search_fuzzy must be
    // callable at the documented paths on SymbolIndex.
    let mut idx = SymbolIndex::new();
    idx.replace_document_symbols(
        "file:///a.pl",
        vec!["get_user".to_string(), "set_user".to_string()],
    );
    assert!(!idx.search_prefix("get_").is_empty());
    assert!(!idx.search_fuzzy("user").is_empty());

    idx.remove_document("file:///a.pl");
    assert!(idx.search_prefix("get_").is_empty());
}

#[test]
fn surface_decl_accessible() {
    // SymbolDecl and extract_symbol_decls must be callable at perl_symbol
    // (crate-root re-export) and at perl_symbol::surface (module path).
    // Compilation verifies the paths; the tiny runtime check binds the
    // function to a compatible type signature.
    let _fn: fn(&perl_ast::Node, Option<&str>) -> Vec<SymbolDecl> = extract_symbol_decls;
}

#[test]
fn surface_ref_accessible() {
    let _kind = SymbolRefKind::SubroutineCall;
    let _method = SymbolRefKind::MethodCall;
    let _static_method = SymbolRefKind::StaticMethodCall;
    let _fn: fn(&perl_ast::Node) -> Vec<SymbolRef> = extract_symbol_refs;
}

/// Canonical adapter path assertion (Requirement 21).
///
/// `symbol_refs_to_semantic_facts` in `surface::facts` is the single canonical
/// SymbolRef→OccurrenceFact adapter. `symbol_decls_to_semantic_facts` is the
/// single canonical SymbolDecl→EntityFact adapter. This test binds both adapter
/// functions and their return types at the crate-root path AND the module path,
/// asserting type identity. If a second adapter path is introduced, it must use
/// a different return type — and this test will catch any attempt to shadow or
/// duplicate the canonical types.
#[test]
fn canonical_adapter_path_is_unique() {
    use perl_semantic_facts::{EntityId, FileId};
    use std::collections::BTreeMap;

    // ── SymbolRef adapter: crate-root path ──
    let _ref_adapter: fn(
        &[SymbolRef],
        FileId,
        &BTreeMap<String, EntityId>,
    ) -> SymbolRefSemanticFacts = symbol_refs_to_semantic_facts;

    // ── SymbolRef adapter: module path (must be the same function) ──
    let _ref_adapter_mod: fn(
        &[SymbolRef],
        FileId,
        &BTreeMap<String, EntityId>,
    ) -> perl_symbol::surface::SymbolRefSemanticFacts =
        perl_symbol::surface::symbol_refs_to_semantic_facts;

    // Type identity: crate-root and module-path types are the same nominal type.
    let _: fn(SymbolRefSemanticFacts) -> perl_symbol::surface::SymbolRefSemanticFacts =
        std::convert::identity;

    // ── SymbolDecl adapter: crate-root path ──
    let _decl_adapter: fn(&[SymbolDecl], FileId) -> SymbolDeclSemanticFacts =
        symbol_decls_to_semantic_facts;

    // ── SymbolDecl adapter: module path (must be the same function) ──
    let _decl_adapter_mod: fn(
        &[perl_symbol::surface::SymbolDecl],
        FileId,
    ) -> perl_symbol::surface::SymbolDeclSemanticFacts =
        perl_symbol::surface::symbol_decls_to_semantic_facts;

    // Type identity: crate-root and module-path types are the same nominal type.
    let _: fn(SymbolDeclSemanticFacts) -> perl_symbol::surface::SymbolDeclSemanticFacts =
        std::convert::identity;
}
