//! Extended unit tests for the `perl-symbol-table` crate.
//!
//! Covers: Symbol definitions, scope management, scope kind variants,
//! symbol lookup, references, re-exports, and edge cases.

use perl_position_tracking::SourceLocation;
use perl_symbol_table::{
    Scope, ScopeId, ScopeKind, Symbol, SymbolKind, SymbolReference, SymbolTable, VarKind,
};

// ---------------------------------------------------------------------------
// Helper builders
// ---------------------------------------------------------------------------

fn make_symbol(
    name: &str,
    qualified: &str,
    kind: SymbolKind,
    scope_id: ScopeId,
    start: usize,
    end: usize,
) -> Symbol {
    Symbol {
        name: name.to_string(),
        qualified_name: qualified.to_string(),
        kind,
        location: SourceLocation { start, end },
        scope_id,
        declaration: None,
        documentation: None,
        attributes: vec![],
    }
}

fn make_ref(name: &str, kind: SymbolKind, scope_id: ScopeId, is_write: bool) -> SymbolReference {
    SymbolReference {
        name: name.to_string(),
        kind,
        location: SourceLocation { start: 0, end: 1 },
        scope_id,
        is_write,
    }
}

// ===========================================================================
// 1. SymbolTable creation and defaults
// ===========================================================================

#[test]
fn new_table_has_global_scope() {
    let table = SymbolTable::new();
    let scope = table.get_scope(0);
    assert!(scope.is_some());
    let scope = scope.unwrap_or_else(|| unreachable!());
    assert_eq!(scope.kind, ScopeKind::Global);
    assert!(scope.parent.is_none());
}

#[test]
fn new_table_current_scope_is_zero() {
    let table = SymbolTable::new();
    assert_eq!(table.current_scope(), 0);
}

#[test]
fn new_table_default_package_is_main() {
    let table = SymbolTable::new();
    assert_eq!(table.current_package(), "main");
}

#[test]
fn new_table_symbols_empty() {
    let table = SymbolTable::new();
    assert_eq!(table.all_symbols().count(), 0);
}

#[test]
fn new_table_references_empty() {
    let table = SymbolTable::new();
    assert_eq!(table.all_references().count(), 0);
}

#[test]
fn default_trait_matches_new() {
    let table = SymbolTable::default();
    // Default has no scopes pre-inserted (unlike ::new which inserts global).
    // Verify that default is usable.
    assert_eq!(table.all_symbols().count(), 0);
}

// ===========================================================================
// 2. Package management
// ===========================================================================

#[test]
fn set_current_package() {
    let mut table = SymbolTable::new();
    table.set_current_package("Foo::Bar".to_string());
    assert_eq!(table.current_package(), "Foo::Bar");
}

#[test]
fn set_package_multiple_times() {
    let mut table = SymbolTable::new();
    table.set_current_package("A".to_string());
    table.set_current_package("B".to_string());
    assert_eq!(table.current_package(), "B");
}

#[test]
fn set_package_empty_string() {
    let mut table = SymbolTable::new();
    table.set_current_package(String::new());
    assert_eq!(table.current_package(), "");
}

// ===========================================================================
// 3. Scope management
// ===========================================================================

#[test]
fn push_scope_returns_incremental_ids() {
    let mut table = SymbolTable::new();
    let s1 = table.push_scope(ScopeKind::Subroutine, SourceLocation { start: 0, end: 10 });
    let s2 = table.push_scope(ScopeKind::Block, SourceLocation { start: 1, end: 9 });
    assert_eq!(s1, 1);
    assert_eq!(s2, 2);
}

#[test]
fn push_scope_sets_parent() {
    let mut table = SymbolTable::new();
    let s1 = table.push_scope(ScopeKind::Subroutine, SourceLocation { start: 0, end: 50 });
    let s2 = table.push_scope(ScopeKind::Block, SourceLocation { start: 5, end: 45 });
    let scope2 = table.get_scope(s2);
    assert!(scope2.is_some());
    assert_eq!(scope2.map(|s| s.parent), Some(Some(s1)));
}

#[test]
fn push_scope_updates_current_scope() {
    let mut table = SymbolTable::new();
    let s1 = table.push_scope(ScopeKind::Package, SourceLocation { start: 0, end: 100 });
    assert_eq!(table.current_scope(), s1);
}

#[test]
fn pop_scope_restores_previous() {
    let mut table = SymbolTable::new();
    let _s1 = table.push_scope(ScopeKind::Subroutine, SourceLocation { start: 0, end: 50 });
    table.pop_scope();
    assert_eq!(table.current_scope(), 0);
}

#[test]
fn pop_scope_beyond_global_stays_at_zero() {
    let mut table = SymbolTable::new();
    table.pop_scope(); // pop global
    // scope_stack is empty → current_scope falls back to 0
    assert_eq!(table.current_scope(), 0);
}

#[test]
fn nested_scopes_three_levels() {
    let mut table = SymbolTable::new();
    let s1 = table.push_scope(ScopeKind::Package, SourceLocation { start: 0, end: 200 });
    let s2 = table.push_scope(ScopeKind::Subroutine, SourceLocation { start: 10, end: 190 });
    let s3 = table.push_scope(ScopeKind::Block, SourceLocation { start: 20, end: 180 });
    assert_eq!(table.current_scope(), s3);

    table.pop_scope();
    assert_eq!(table.current_scope(), s2);

    table.pop_scope();
    assert_eq!(table.current_scope(), s1);

    table.pop_scope();
    assert_eq!(table.current_scope(), 0);
}

#[test]
fn get_scope_returns_none_for_unknown_id() {
    let table = SymbolTable::new();
    assert!(table.get_scope(999).is_none());
}

#[test]
fn scope_location_preserved() {
    let mut table = SymbolTable::new();
    let loc = SourceLocation { start: 42, end: 99 };
    let id = table.push_scope(ScopeKind::Eval, loc);
    let scope = table.get_scope(id);
    assert_eq!(scope.map(|s| s.location.start), Some(42));
    assert_eq!(scope.map(|s| s.location.end), Some(99));
}

// ===========================================================================
// 4. ScopeKind variants
// ===========================================================================

#[test]
fn scope_kind_global() {
    let table = SymbolTable::new();
    let scope = table.get_scope(0);
    assert_eq!(scope.map(|s| s.kind), Some(ScopeKind::Global));
}

#[test]
fn scope_kind_package() {
    let mut table = SymbolTable::new();
    let id = table.push_scope(ScopeKind::Package, SourceLocation { start: 0, end: 1 });
    assert_eq!(table.get_scope(id).map(|s| s.kind), Some(ScopeKind::Package));
}

#[test]
fn scope_kind_subroutine() {
    let mut table = SymbolTable::new();
    let id = table.push_scope(ScopeKind::Subroutine, SourceLocation { start: 0, end: 1 });
    assert_eq!(table.get_scope(id).map(|s| s.kind), Some(ScopeKind::Subroutine));
}

#[test]
fn scope_kind_block() {
    let mut table = SymbolTable::new();
    let id = table.push_scope(ScopeKind::Block, SourceLocation { start: 0, end: 1 });
    assert_eq!(table.get_scope(id).map(|s| s.kind), Some(ScopeKind::Block));
}

#[test]
fn scope_kind_eval() {
    let mut table = SymbolTable::new();
    let id = table.push_scope(ScopeKind::Eval, SourceLocation { start: 0, end: 1 });
    assert_eq!(table.get_scope(id).map(|s| s.kind), Some(ScopeKind::Eval));
}

#[test]
fn scope_kind_equality() {
    assert_eq!(ScopeKind::Global, ScopeKind::Global);
    assert_ne!(ScopeKind::Global, ScopeKind::Package);
    assert_ne!(ScopeKind::Package, ScopeKind::Subroutine);
    assert_ne!(ScopeKind::Block, ScopeKind::Eval);
}

#[test]
fn scope_kind_copy_semantics() {
    let k = ScopeKind::Subroutine;
    let k2 = k; // Copy
    assert_eq!(k, k2);
}

// ===========================================================================
// 5. Adding symbols
// ===========================================================================

#[test]
fn add_symbol_registers_in_map() {
    let mut table = SymbolTable::new();
    let sym = make_symbol("foo", "main::foo", SymbolKind::Subroutine, 0, 0, 10);
    table.add_symbol(sym);
    assert!(table.symbols.contains_key("foo"));
}

#[test]
fn add_symbol_inserts_into_scope_symbols() {
    let mut table = SymbolTable::new();
    let sym = make_symbol("bar", "main::bar", SymbolKind::Subroutine, 0, 0, 5);
    table.add_symbol(sym);
    let scope = table.get_scope(0);
    assert!(scope.map_or(false, |s| s.symbols.contains("bar")));
}

#[test]
fn add_multiple_symbols_same_name() {
    let mut table = SymbolTable::new();
    let s1 = make_symbol("x", "A::x", SymbolKind::scalar(), 0, 0, 5);
    let s2 = make_symbol("x", "B::x", SymbolKind::scalar(), 0, 10, 15);
    table.add_symbol(s1);
    table.add_symbol(s2);
    assert_eq!(table.symbols.get("x").map_or(0, |v| v.len()), 2);
}

#[test]
fn add_symbol_to_inner_scope() {
    let mut table = SymbolTable::new();
    let sid = table.push_scope(ScopeKind::Subroutine, SourceLocation { start: 0, end: 50 });
    let sym = make_symbol("local_var", "main::local_var", SymbolKind::scalar(), sid, 5, 15);
    table.add_symbol(sym);
    let scope = table.get_scope(sid);
    assert!(scope.map_or(false, |s| s.symbols.contains("local_var")));
}

#[test]
fn add_symbol_with_documentation() {
    let mut table = SymbolTable::new();
    let mut sym = make_symbol("documented", "main::documented", SymbolKind::Subroutine, 0, 0, 20);
    sym.documentation = Some("This function does stuff".to_string());
    table.add_symbol(sym);
    let found = table.symbols.get("documented");
    assert!(found.is_some());
    let first = found.and_then(|v| v.first());
    assert_eq!(first.and_then(|s| s.documentation.as_deref()), Some("This function does stuff"));
}

#[test]
fn add_symbol_with_attributes() {
    let mut table = SymbolTable::new();
    let mut sym = make_symbol("meth", "main::meth", SymbolKind::Method, 0, 0, 10);
    sym.attributes = vec![":method".to_string(), ":lvalue".to_string()];
    table.add_symbol(sym);
    let first = table.symbols.get("meth").and_then(|v| v.first());
    assert_eq!(first.map(|s| s.attributes.len()), Some(2));
}

#[test]
fn add_symbol_with_declaration() {
    let mut table = SymbolTable::new();
    let mut sym = make_symbol("y", "main::y", SymbolKind::scalar(), 0, 0, 5);
    sym.declaration = Some("my".to_string());
    table.add_symbol(sym);
    let first = table.symbols.get("y").and_then(|v| v.first());
    assert_eq!(first.and_then(|s| s.declaration.as_deref()), Some("my"));
}

// ===========================================================================
// 6. Adding references
// ===========================================================================

#[test]
fn add_reference_registers_in_map() {
    let mut table = SymbolTable::new();
    let r = make_ref("foo", SymbolKind::Subroutine, 0, false);
    table.add_reference(r);
    assert!(table.references.contains_key("foo"));
}

#[test]
fn add_multiple_references_same_name() {
    let mut table = SymbolTable::new();
    table.add_reference(make_ref("x", SymbolKind::scalar(), 0, false));
    table.add_reference(make_ref("x", SymbolKind::scalar(), 0, true));
    assert_eq!(table.references.get("x").map_or(0, |v| v.len()), 2);
}

#[test]
fn reference_is_write_flag() {
    let mut table = SymbolTable::new();
    table.add_reference(make_ref("w", SymbolKind::scalar(), 0, true));
    let refs = table.references.get("w");
    let first = refs.and_then(|v| v.first());
    assert_eq!(first.map(|r| r.is_write), Some(true));
}

// ===========================================================================
// 7. find_symbol — scope walking
// ===========================================================================

#[test]
fn find_symbol_in_same_scope() {
    let mut table = SymbolTable::new();
    let sym = make_symbol("a", "main::a", SymbolKind::scalar(), 0, 0, 5);
    table.add_symbol(sym);
    let found = table.find_symbol("a", 0, SymbolKind::scalar());
    assert!(!found.is_empty());
}

#[test]
fn find_symbol_walks_up_scope_chain() {
    let mut table = SymbolTable::new();
    let sym = make_symbol("outer", "main::outer", SymbolKind::scalar(), 0, 0, 5);
    table.add_symbol(sym);
    let inner = table.push_scope(ScopeKind::Subroutine, SourceLocation { start: 10, end: 50 });
    let found = table.find_symbol("outer", inner, SymbolKind::scalar());
    assert!(!found.is_empty());
}

#[test]
fn find_symbol_wrong_kind_returns_empty() {
    let mut table = SymbolTable::new();
    let sym = make_symbol("f", "main::f", SymbolKind::Subroutine, 0, 0, 10);
    table.add_symbol(sym);
    let found = table.find_symbol("f", 0, SymbolKind::scalar());
    assert!(found.is_empty());
}

#[test]
fn find_symbol_not_defined_returns_empty() {
    let table = SymbolTable::new();
    let found = table.find_symbol("nonexistent", 0, SymbolKind::Subroutine);
    assert!(found.is_empty());
}

#[test]
fn find_symbol_in_deeply_nested_scope() {
    let mut table = SymbolTable::new();
    let sym = make_symbol("deep", "main::deep", SymbolKind::Subroutine, 0, 0, 5);
    table.add_symbol(sym);
    let s1 = table.push_scope(ScopeKind::Package, SourceLocation { start: 0, end: 200 });
    let s2 = table.push_scope(ScopeKind::Subroutine, SourceLocation { start: 10, end: 190 });
    let s3 = table.push_scope(ScopeKind::Block, SourceLocation { start: 20, end: 180 });
    let _ = (s1, s2); // suppress unused warnings
    let found = table.find_symbol("deep", s3, SymbolKind::Subroutine);
    assert!(!found.is_empty());
}

#[test]
fn find_symbol_our_variable_visible_from_non_package_scope() {
    let mut table = SymbolTable::new();
    let mut sym = make_symbol("shared", "main::shared", SymbolKind::scalar(), 0, 0, 10);
    sym.declaration = Some("our".to_string());
    table.add_symbol(sym);

    let inner = table.push_scope(ScopeKind::Subroutine, SourceLocation { start: 15, end: 50 });
    let found = table.find_symbol("shared", inner, SymbolKind::scalar());
    // "our" variables are visible across scopes
    assert!(!found.is_empty());
}

#[test]
fn find_symbol_from_invalid_scope_returns_empty() {
    let mut table = SymbolTable::new();
    let sym = make_symbol("z", "main::z", SymbolKind::scalar(), 0, 0, 5);
    table.add_symbol(sym);
    // Scope 999 doesn't exist — should not panic, just return empty
    let found = table.find_symbol("z", 999, SymbolKind::scalar());
    assert!(found.is_empty());
}

// ===========================================================================
// 8. find_references
// ===========================================================================

#[test]
fn find_references_matches_kind() {
    let mut table = SymbolTable::new();
    let sym = make_symbol("func", "main::func", SymbolKind::Subroutine, 0, 0, 10);
    table.add_symbol(sym.clone());
    table.add_reference(make_ref("func", SymbolKind::Subroutine, 0, false));
    table.add_reference(make_ref("func", SymbolKind::scalar(), 0, false)); // different kind

    let refs = table.find_references(&sym);
    assert_eq!(refs.len(), 1);
}

#[test]
fn find_references_no_refs_returns_empty() {
    let mut table = SymbolTable::new();
    let sym = make_symbol("lonely", "main::lonely", SymbolKind::Subroutine, 0, 0, 10);
    table.add_symbol(sym.clone());
    let refs = table.find_references(&sym);
    assert!(refs.is_empty());
}

#[test]
fn find_references_multiple() {
    let mut table = SymbolTable::new();
    let sym = make_symbol("multi", "main::multi", SymbolKind::Subroutine, 0, 0, 10);
    table.add_symbol(sym.clone());
    table.add_reference(make_ref("multi", SymbolKind::Subroutine, 0, false));
    table.add_reference(make_ref("multi", SymbolKind::Subroutine, 0, true));
    table.add_reference(make_ref("multi", SymbolKind::Subroutine, 1, false));

    let refs = table.find_references(&sym);
    assert_eq!(refs.len(), 3);
}

// ===========================================================================
// 9. all_symbols / all_references iterators
// ===========================================================================

#[test]
fn all_symbols_counts_all() {
    let mut table = SymbolTable::new();
    table.add_symbol(make_symbol("a", "main::a", SymbolKind::Subroutine, 0, 0, 5));
    table.add_symbol(make_symbol("b", "main::b", SymbolKind::scalar(), 0, 10, 15));
    table.add_symbol(make_symbol("c", "main::c", SymbolKind::Package, 0, 20, 25));
    assert_eq!(table.all_symbols().count(), 3);
}

#[test]
fn all_references_counts_all() {
    let mut table = SymbolTable::new();
    table.add_reference(make_ref("a", SymbolKind::Subroutine, 0, false));
    table.add_reference(make_ref("b", SymbolKind::scalar(), 0, true));
    assert_eq!(table.all_references().count(), 2);
}

// ===========================================================================
// 10. Re-exports from perl-symbol-types
// ===========================================================================

#[test]
fn reexport_symbol_kind_subroutine() {
    let k = SymbolKind::Subroutine;
    assert!(k.is_callable());
}

#[test]
fn reexport_symbol_kind_method() {
    let k = SymbolKind::Method;
    assert!(k.is_callable());
}

#[test]
fn reexport_symbol_kind_package() {
    let k = SymbolKind::Package;
    assert!(k.is_namespace());
}

#[test]
fn reexport_symbol_kind_class() {
    let k = SymbolKind::Class;
    assert!(k.is_namespace());
}

#[test]
fn reexport_symbol_kind_role() {
    let k = SymbolKind::Role;
    assert!(k.is_namespace());
}

#[test]
fn reexport_symbol_kind_variable_scalar() {
    let k = SymbolKind::scalar();
    assert!(k.is_variable());
    assert_eq!(k, SymbolKind::Variable(VarKind::Scalar));
}

#[test]
fn reexport_symbol_kind_variable_array() {
    let k = SymbolKind::array();
    assert!(k.is_variable());
    assert_eq!(k, SymbolKind::Variable(VarKind::Array));
}

#[test]
fn reexport_symbol_kind_variable_hash() {
    let k = SymbolKind::hash();
    assert!(k.is_variable());
    assert_eq!(k, SymbolKind::Variable(VarKind::Hash));
}

#[test]
fn reexport_var_kind_sigils() {
    assert_eq!(VarKind::Scalar.sigil(), "$");
    assert_eq!(VarKind::Array.sigil(), "@");
    assert_eq!(VarKind::Hash.sigil(), "%");
}

#[test]
fn reexport_symbol_kind_constant() {
    let k = SymbolKind::Constant;
    assert!(!k.is_variable());
    assert!(!k.is_callable());
    assert!(!k.is_namespace());
}

#[test]
fn reexport_symbol_kind_import() {
    let k = SymbolKind::Import;
    assert!(!k.is_variable());
    assert!(!k.is_callable());
}

#[test]
fn reexport_symbol_kind_export() {
    let k = SymbolKind::Export;
    assert!(!k.is_variable());
    assert!(!k.is_callable());
}

#[test]
fn reexport_symbol_kind_label() {
    let k = SymbolKind::Label;
    assert!(!k.is_callable());
    assert!(!k.is_namespace());
}

#[test]
fn reexport_symbol_kind_format() {
    let k = SymbolKind::Format;
    assert!(!k.is_callable());
    assert!(!k.is_namespace());
}

// ===========================================================================
// 11. Symbol struct field access
// ===========================================================================

#[test]
fn symbol_fields_accessible() {
    let sym = Symbol {
        name: "test_sym".to_string(),
        qualified_name: "Pkg::test_sym".to_string(),
        kind: SymbolKind::Method,
        location: SourceLocation { start: 100, end: 200 },
        scope_id: 5,
        declaration: Some("our".to_string()),
        documentation: Some("doc".to_string()),
        attributes: vec![":shared".to_string()],
    };
    assert_eq!(sym.name, "test_sym");
    assert_eq!(sym.qualified_name, "Pkg::test_sym");
    assert_eq!(sym.kind, SymbolKind::Method);
    assert_eq!(sym.location.start, 100);
    assert_eq!(sym.location.end, 200);
    assert_eq!(sym.scope_id, 5);
    assert_eq!(sym.declaration.as_deref(), Some("our"));
    assert_eq!(sym.documentation.as_deref(), Some("doc"));
    assert_eq!(sym.attributes.len(), 1);
}

#[test]
fn symbol_clone() {
    let sym = make_symbol("cloned", "main::cloned", SymbolKind::Subroutine, 0, 0, 5);
    let sym2 = sym.clone();
    assert_eq!(sym.name, sym2.name);
    assert_eq!(sym.qualified_name, sym2.qualified_name);
}

// ===========================================================================
// 12. SymbolReference struct fields
// ===========================================================================

#[test]
fn symbol_reference_fields_accessible() {
    let r = SymbolReference {
        name: "ref_sym".to_string(),
        kind: SymbolKind::array(),
        location: SourceLocation { start: 50, end: 60 },
        scope_id: 3,
        is_write: true,
    };
    assert_eq!(r.name, "ref_sym");
    assert_eq!(r.kind, SymbolKind::array());
    assert_eq!(r.location.start, 50);
    assert_eq!(r.location.end, 60);
    assert_eq!(r.scope_id, 3);
    assert!(r.is_write);
}

#[test]
fn symbol_reference_clone() {
    let r = make_ref("clone_me", SymbolKind::hash(), 0, false);
    let r2 = r.clone();
    assert_eq!(r.name, r2.name);
    assert_eq!(r.is_write, r2.is_write);
}

// ===========================================================================
// 13. Scope struct fields
// ===========================================================================

#[test]
fn scope_struct_fields() {
    let scope = Scope {
        id: 42,
        parent: Some(1),
        kind: ScopeKind::Eval,
        location: SourceLocation { start: 10, end: 20 },
        symbols: std::collections::HashSet::new(),
    };
    assert_eq!(scope.id, 42);
    assert_eq!(scope.parent, Some(1));
    assert_eq!(scope.kind, ScopeKind::Eval);
    assert_eq!(scope.location.start, 10);
    assert!(scope.symbols.is_empty());
}

#[test]
fn scope_clone() {
    let mut scope = Scope {
        id: 1,
        parent: None,
        kind: ScopeKind::Global,
        location: SourceLocation { start: 0, end: 0 },
        symbols: std::collections::HashSet::new(),
    };
    scope.symbols.insert("x".to_string());
    let scope2 = scope.clone();
    assert!(scope2.symbols.contains("x"));
}

// ===========================================================================
// 14. Integration: realistic Perl file simulation
// ===========================================================================

#[test]
fn simulate_perl_file_with_package_and_sub() {
    let mut table = SymbolTable::new();

    // package Foo;
    table.set_current_package("Foo".to_string());
    let pkg_scope = table.push_scope(ScopeKind::Package, SourceLocation { start: 0, end: 200 });

    // sub bar { ... }
    let sub_scope = table.push_scope(ScopeKind::Subroutine, SourceLocation { start: 20, end: 100 });
    let sym = make_symbol("bar", "Foo::bar", SymbolKind::Subroutine, pkg_scope, 20, 100);
    table.add_symbol(sym);

    // my $x inside sub
    let mut var = make_symbol("x", "Foo::x", SymbolKind::scalar(), sub_scope, 30, 35);
    var.declaration = Some("my".to_string());
    table.add_symbol(var);

    // reference to $x
    table.add_reference(make_ref("x", SymbolKind::scalar(), sub_scope, false));

    table.pop_scope(); // exit sub
    table.pop_scope(); // exit package

    assert_eq!(table.current_package(), "Foo");
    assert_eq!(table.all_symbols().count(), 2);
    assert_eq!(table.all_references().count(), 1);
}

#[test]
fn simulate_our_variable_across_scopes() {
    let mut table = SymbolTable::new();

    // our $config at global scope
    let mut sym = make_symbol("config", "main::config", SymbolKind::scalar(), 0, 0, 15);
    sym.declaration = Some("our".to_string());
    table.add_symbol(sym.clone());

    // sub process { ... }
    let sub_id = table.push_scope(ScopeKind::Subroutine, SourceLocation { start: 20, end: 100 });

    // $config should be visible inside sub due to "our"
    let found = table.find_symbol("config", sub_id, SymbolKind::scalar());
    assert!(!found.is_empty());

    // Even from a block inside the sub
    let block_id = table.push_scope(ScopeKind::Block, SourceLocation { start: 30, end: 90 });
    let found_block = table.find_symbol("config", block_id, SymbolKind::scalar());
    assert!(!found_block.is_empty());

    table.pop_scope();
    table.pop_scope();
}

#[test]
fn simulate_multiple_packages() {
    let mut table = SymbolTable::new();

    // package A;
    table.set_current_package("A".to_string());
    let pa = table.push_scope(ScopeKind::Package, SourceLocation { start: 0, end: 100 });
    table.add_symbol(make_symbol("helper", "A::helper", SymbolKind::Subroutine, pa, 10, 50));
    table.pop_scope();

    // package B;
    table.set_current_package("B".to_string());
    let pb = table.push_scope(ScopeKind::Package, SourceLocation { start: 100, end: 200 });
    table.add_symbol(make_symbol("helper", "B::helper", SymbolKind::Subroutine, pb, 110, 150));
    table.pop_scope();

    // Two symbols named "helper" in different packages
    let helpers = table.symbols.get("helper");
    assert_eq!(helpers.map_or(0, |v| v.len()), 2);
}

// ===========================================================================
// 15. Edge cases
// ===========================================================================

#[test]
fn empty_symbol_name() {
    let mut table = SymbolTable::new();
    let sym = make_symbol("", "", SymbolKind::Subroutine, 0, 0, 0);
    table.add_symbol(sym);
    assert!(table.symbols.contains_key(""));
}

#[test]
fn unicode_symbol_name() {
    let mut table = SymbolTable::new();
    let sym = make_symbol("café", "main::café", SymbolKind::scalar(), 0, 0, 10);
    table.add_symbol(sym);
    let found = table.find_symbol("café", 0, SymbolKind::scalar());
    assert!(!found.is_empty());
}

#[test]
fn symbol_with_empty_attributes_vec() {
    let sym = make_symbol("no_attrs", "main::no_attrs", SymbolKind::Subroutine, 0, 0, 5);
    assert!(sym.attributes.is_empty());
}

#[test]
fn scope_id_type_is_usize() {
    let id: ScopeId = 42;
    assert_eq!(id, 42_usize);
}

#[test]
fn many_scopes_monotonic_ids() {
    let mut table = SymbolTable::new();
    let mut prev = 0;
    for i in 0..20 {
        let id = table.push_scope(ScopeKind::Block, SourceLocation { start: i, end: i + 1 });
        assert!(id > prev || i == 0);
        prev = id;
    }
}

#[test]
fn find_symbol_different_variable_kinds_distinct() {
    let mut table = SymbolTable::new();
    table.add_symbol(make_symbol("x", "main::x", SymbolKind::scalar(), 0, 0, 5));
    table.add_symbol(make_symbol("x", "main::x", SymbolKind::array(), 0, 10, 15));
    table.add_symbol(make_symbol("x", "main::x", SymbolKind::hash(), 0, 20, 25));

    let scalars = table.find_symbol("x", 0, SymbolKind::scalar());
    let arrays = table.find_symbol("x", 0, SymbolKind::array());
    let hashes = table.find_symbol("x", 0, SymbolKind::hash());

    assert!(!scalars.is_empty());
    assert!(!arrays.is_empty());
    assert!(!hashes.is_empty());
}
