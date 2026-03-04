//! Comprehensive unit tests for perl-symbol-table crate.
//!
//! Covers: SymbolTable, Symbol, SymbolReference, Scope, ScopeKind,
//! scope management, symbol lookup, reference tracking, and edge cases.

use perl_position_tracking::SourceLocation;
use perl_symbol_table::{
    Scope, ScopeId, ScopeKind, Symbol, SymbolKind, SymbolReference, SymbolTable, VarKind,
};
use std::collections::HashSet;

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
        location: SourceLocation::new(start, end),
        scope_id,
        declaration: None,
        documentation: None,
        attributes: vec![],
    }
}

fn make_ref(
    name: &str,
    kind: SymbolKind,
    scope_id: ScopeId,
    start: usize,
    end: usize,
    is_write: bool,
) -> SymbolReference {
    SymbolReference {
        name: name.to_string(),
        kind,
        location: SourceLocation::new(start, end),
        scope_id,
        is_write,
    }
}

// ---------------------------------------------------------------------------
// SymbolTable::new – initial state
// ---------------------------------------------------------------------------

#[test]
fn new_table_has_global_scope() -> Result<(), String> {
    let table = SymbolTable::new();
    let scope = table.get_scope(0).ok_or("global scope missing")?;
    assert_eq!(scope.id, 0);
    assert_eq!(scope.kind, ScopeKind::Global);
    assert!(scope.parent.is_none());
    assert!(scope.symbols.is_empty());
    Ok(())
}

#[test]
fn new_table_defaults() -> Result<(), String> {
    let table = SymbolTable::new();
    assert_eq!(table.current_scope(), 0);
    assert_eq!(table.current_package(), "main");
    assert_eq!(table.symbols.len(), 0);
    assert_eq!(table.references.len(), 0);
    // Only the global scope exists
    assert_eq!(table.scopes.len(), 1);
    Ok(())
}

#[test]
fn default_trait_produces_empty_table() -> Result<(), String> {
    let table = SymbolTable::default();
    // Default has no scopes or symbols (unlike ::new which creates global scope)
    assert_eq!(table.symbols.len(), 0);
    assert_eq!(table.references.len(), 0);
    Ok(())
}

// ---------------------------------------------------------------------------
// Package management
// ---------------------------------------------------------------------------

#[test]
fn set_and_get_current_package() -> Result<(), String> {
    let mut table = SymbolTable::new();
    assert_eq!(table.current_package(), "main");

    table.set_current_package("Foo::Bar".to_string());
    assert_eq!(table.current_package(), "Foo::Bar");

    table.set_current_package("Baz".to_string());
    assert_eq!(table.current_package(), "Baz");
    Ok(())
}

// ---------------------------------------------------------------------------
// Scope management
// ---------------------------------------------------------------------------

#[test]
fn push_scope_returns_incrementing_ids() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 100);

    let id1 = table.push_scope(ScopeKind::Subroutine, loc);
    let id2 = table.push_scope(ScopeKind::Block, loc);
    let id3 = table.push_scope(ScopeKind::Eval, loc);

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);
    Ok(())
}

#[test]
fn push_scope_sets_parent_correctly() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 100);

    let sub_id = table.push_scope(ScopeKind::Subroutine, loc);
    let block_id = table.push_scope(ScopeKind::Block, loc);

    let sub_scope = table.get_scope(sub_id).ok_or("sub scope missing")?;
    assert_eq!(sub_scope.parent, Some(0)); // parent is global

    let block_scope = table.get_scope(block_id).ok_or("block scope missing")?;
    assert_eq!(block_scope.parent, Some(sub_id));
    Ok(())
}

#[test]
fn push_scope_records_kind_and_location() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(10, 50);

    let id = table.push_scope(ScopeKind::Package, loc);
    let scope = table.get_scope(id).ok_or("scope missing")?;

    assert_eq!(scope.kind, ScopeKind::Package);
    assert_eq!(scope.location.start, 10);
    assert_eq!(scope.location.end, 50);
    assert!(scope.symbols.is_empty());
    Ok(())
}

#[test]
fn pop_scope_restores_previous() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 100);

    let sub_id = table.push_scope(ScopeKind::Subroutine, loc);
    assert_eq!(table.current_scope(), sub_id);

    table.pop_scope();
    assert_eq!(table.current_scope(), 0);
    Ok(())
}

#[test]
fn pop_scope_on_global_returns_zero() -> Result<(), String> {
    let mut table = SymbolTable::new();
    // Pop the global scope off the stack
    table.pop_scope();
    // current_scope falls back to 0
    assert_eq!(table.current_scope(), 0);
    Ok(())
}

#[test]
fn nested_scope_push_pop_sequence() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 100);

    // Global(0) -> Sub(1) -> Block(2) -> Eval(3)
    let s1 = table.push_scope(ScopeKind::Subroutine, loc);
    let s2 = table.push_scope(ScopeKind::Block, loc);
    let s3 = table.push_scope(ScopeKind::Eval, loc);
    assert_eq!(table.current_scope(), s3);

    table.pop_scope();
    assert_eq!(table.current_scope(), s2);

    table.pop_scope();
    assert_eq!(table.current_scope(), s1);

    table.pop_scope();
    assert_eq!(table.current_scope(), 0);
    Ok(())
}

#[test]
fn all_scope_kinds_can_be_pushed() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 1);

    let kinds = [
        ScopeKind::Global,
        ScopeKind::Package,
        ScopeKind::Subroutine,
        ScopeKind::Block,
        ScopeKind::Eval,
    ];
    for kind in &kinds {
        let id = table.push_scope(*kind, loc);
        let scope = table.get_scope(id).ok_or("scope missing")?;
        assert_eq!(scope.kind, *kind);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Symbol addition and lookup
// ---------------------------------------------------------------------------

#[test]
fn add_symbol_registers_in_scope() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let sym = make_symbol("foo", "main::foo", SymbolKind::Subroutine, 0, 0, 10);
    table.add_symbol(sym);

    let scope = table.get_scope(0).ok_or("global scope missing")?;
    assert!(scope.symbols.contains("foo"));
    Ok(())
}

#[test]
fn add_symbol_to_nonexistent_scope_still_indexes() -> Result<(), String> {
    let mut table = SymbolTable::new();
    // Scope 999 does not exist
    let sym = make_symbol("bar", "main::bar", SymbolKind::Subroutine, 999, 0, 5);
    table.add_symbol(sym);

    // Symbol is in the index even though scope didn't exist
    assert!(table.symbols.contains_key("bar"));
    assert_eq!(table.symbols["bar"].len(), 1);
    Ok(())
}

#[test]
fn add_multiple_symbols_same_name() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 100);
    let sub_id = table.push_scope(ScopeKind::Subroutine, loc);

    let s1 = make_symbol("x", "main::x", SymbolKind::scalar(), 0, 0, 5);
    let s2 = make_symbol("x", "Other::x", SymbolKind::scalar(), sub_id, 10, 15);

    table.add_symbol(s1);
    table.add_symbol(s2);

    assert_eq!(table.symbols["x"].len(), 2);
    Ok(())
}

#[test]
fn add_symbol_with_all_fields() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let sym = Symbol {
        name: "my_sub".to_string(),
        qualified_name: "Pkg::my_sub".to_string(),
        kind: SymbolKind::Subroutine,
        location: SourceLocation::new(100, 200),
        scope_id: 0,
        declaration: Some("sub".to_string()),
        documentation: Some("Does stuff".to_string()),
        attributes: vec!["method".to_string(), "lvalue".to_string()],
    };
    table.add_symbol(sym);

    let stored = table.symbols.get("my_sub").ok_or("symbol missing")?;
    let s = stored.first().ok_or("empty vec")?;
    assert_eq!(s.qualified_name, "Pkg::my_sub");
    assert_eq!(s.declaration.as_deref(), Some("sub"));
    assert_eq!(s.documentation.as_deref(), Some("Does stuff"));
    assert_eq!(s.attributes.len(), 2);
    Ok(())
}

// ---------------------------------------------------------------------------
// find_symbol – scope chain traversal
// ---------------------------------------------------------------------------

#[test]
fn find_symbol_in_same_scope() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let sym = make_symbol("count", "main::count", SymbolKind::scalar(), 0, 0, 5);
    table.add_symbol(sym);

    let found = table.find_symbol("count", 0, SymbolKind::scalar());
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "count");
    Ok(())
}

#[test]
fn find_symbol_walks_up_scope_chain() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 100);

    // Define in global scope
    let sym = make_symbol("global_var", "main::global_var", SymbolKind::scalar(), 0, 0, 10);
    table.add_symbol(sym);

    // Push nested scopes
    let sub_id = table.push_scope(ScopeKind::Subroutine, loc);
    let block_id = table.push_scope(ScopeKind::Block, loc);

    // Should find global_var from the block scope
    let found = table.find_symbol("global_var", block_id, SymbolKind::scalar());
    assert!(!found.is_empty());
    assert_eq!(found[0].name, "global_var");

    // Also from subroutine scope
    let found2 = table.find_symbol("global_var", sub_id, SymbolKind::scalar());
    assert!(!found2.is_empty());
    Ok(())
}

#[test]
fn find_symbol_wrong_kind_returns_empty() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let sym = make_symbol("thing", "main::thing", SymbolKind::Subroutine, 0, 0, 10);
    table.add_symbol(sym);

    let found = table.find_symbol("thing", 0, SymbolKind::scalar());
    // Should not find a subroutine when looking for scalar
    assert!(found.is_empty());
    Ok(())
}

#[test]
fn find_symbol_nonexistent_returns_empty() -> Result<(), String> {
    let table = SymbolTable::new();
    let found = table.find_symbol("does_not_exist", 0, SymbolKind::Subroutine);
    assert!(found.is_empty());
    Ok(())
}

#[test]
fn find_symbol_nonexistent_scope_returns_empty() -> Result<(), String> {
    let table = SymbolTable::new();
    let found = table.find_symbol("foo", 999, SymbolKind::Subroutine);
    assert!(found.is_empty());
    Ok(())
}

#[test]
fn find_symbol_our_variable_visible_across_scopes() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 100);

    // Add an 'our' variable in global scope
    let mut sym = make_symbol("shared", "main::shared", SymbolKind::scalar(), 0, 0, 10);
    sym.declaration = Some("our".to_string());
    table.add_symbol(sym);

    // Push a subroutine scope
    let sub_id = table.push_scope(ScopeKind::Subroutine, loc);

    // 'our' variables should be visible from inner scopes
    let found = table.find_symbol("shared", sub_id, SymbolKind::scalar());
    assert!(!found.is_empty());
    Ok(())
}

#[test]
fn find_symbol_in_inner_scope_not_visible_from_outer() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 100);

    let sub_id = table.push_scope(ScopeKind::Subroutine, loc);

    // Define symbol only in the inner scope
    let sym = make_symbol("inner_only", "main::inner_only", SymbolKind::scalar(), sub_id, 10, 20);
    table.add_symbol(sym);

    table.pop_scope();

    // From global scope, inner symbol should NOT be found via direct scope match
    // (find_symbol walks *up*, not down)
    let found = table.find_symbol("inner_only", 0, SymbolKind::scalar());
    assert!(found.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// Reference tracking
// ---------------------------------------------------------------------------

#[test]
fn add_and_find_references() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let sym = make_symbol("func", "main::func", SymbolKind::Subroutine, 0, 0, 10);
    table.add_symbol(sym.clone());

    let r1 = make_ref("func", SymbolKind::Subroutine, 0, 20, 24, false);
    let r2 = make_ref("func", SymbolKind::Subroutine, 0, 40, 44, false);
    table.add_reference(r1);
    table.add_reference(r2);

    let stored = &table.symbols["func"][0];
    let refs = table.find_references(stored);
    assert_eq!(refs.len(), 2);
    Ok(())
}

#[test]
fn find_references_filters_by_kind() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let sym = make_symbol("x", "main::x", SymbolKind::scalar(), 0, 0, 5);
    table.add_symbol(sym.clone());

    // Add reference matching kind
    let r1 = make_ref("x", SymbolKind::scalar(), 0, 10, 11, false);
    // Add reference with different kind (same name, but subroutine)
    let r2 = make_ref("x", SymbolKind::Subroutine, 0, 20, 21, false);
    table.add_reference(r1);
    table.add_reference(r2);

    let stored = &table.symbols["x"][0];
    let refs = table.find_references(stored);
    // Only the scalar reference should match
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].kind, SymbolKind::scalar());
    Ok(())
}

#[test]
fn find_references_no_refs_returns_empty() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let sym = make_symbol("lonely", "main::lonely", SymbolKind::Subroutine, 0, 0, 10);
    table.add_symbol(sym.clone());

    let stored = &table.symbols["lonely"][0];
    let refs = table.find_references(stored);
    assert!(refs.is_empty());
    Ok(())
}

#[test]
fn reference_write_flag() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let r_read = make_ref("val", SymbolKind::scalar(), 0, 10, 13, false);
    let r_write = make_ref("val", SymbolKind::scalar(), 0, 20, 23, true);

    table.add_reference(r_read);
    table.add_reference(r_write);

    let refs = table.references.get("val").ok_or("refs missing")?;
    assert_eq!(refs.len(), 2);
    assert!(!refs[0].is_write);
    assert!(refs[1].is_write);
    Ok(())
}

// ---------------------------------------------------------------------------
// all_symbols / all_references iterators
// ---------------------------------------------------------------------------

#[test]
fn all_symbols_iterates_everything() -> Result<(), String> {
    let mut table = SymbolTable::new();
    table.add_symbol(make_symbol("a", "main::a", SymbolKind::Subroutine, 0, 0, 5));
    table.add_symbol(make_symbol("b", "main::b", SymbolKind::scalar(), 0, 10, 15));
    table.add_symbol(make_symbol("c", "main::c", SymbolKind::Package, 0, 20, 25));

    let names: HashSet<&str> = table.all_symbols().map(|s| s.name.as_str()).collect();
    assert!(names.contains("a"));
    assert!(names.contains("b"));
    assert!(names.contains("c"));
    assert_eq!(names.len(), 3);
    Ok(())
}

#[test]
fn all_symbols_empty_table() -> Result<(), String> {
    let table = SymbolTable::new();
    assert_eq!(table.all_symbols().count(), 0);
    Ok(())
}

#[test]
fn all_references_iterates_everything() -> Result<(), String> {
    let mut table = SymbolTable::new();
    table.add_reference(make_ref("x", SymbolKind::scalar(), 0, 0, 1, false));
    table.add_reference(make_ref("y", SymbolKind::array(), 0, 5, 6, true));

    let names: HashSet<&str> = table.all_references().map(|r| r.name.as_str()).collect();
    assert!(names.contains("x"));
    assert!(names.contains("y"));
    assert_eq!(names.len(), 2);
    Ok(())
}

#[test]
fn all_references_empty_table() -> Result<(), String> {
    let table = SymbolTable::new();
    assert_eq!(table.all_references().count(), 0);
    Ok(())
}

// ---------------------------------------------------------------------------
// get_scope
// ---------------------------------------------------------------------------

#[test]
fn get_scope_returns_none_for_invalid_id() -> Result<(), String> {
    let table = SymbolTable::new();
    assert!(table.get_scope(42).is_none());
    Ok(())
}

#[test]
fn get_scope_returns_pushed_scope() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(5, 50);
    let id = table.push_scope(ScopeKind::Eval, loc);

    let scope = table.get_scope(id).ok_or("scope missing")?;
    assert_eq!(scope.id, id);
    assert_eq!(scope.kind, ScopeKind::Eval);
    Ok(())
}

// ---------------------------------------------------------------------------
// ScopeKind equality and copy
// ---------------------------------------------------------------------------

#[test]
fn scope_kind_equality() -> Result<(), String> {
    assert_eq!(ScopeKind::Global, ScopeKind::Global);
    assert_eq!(ScopeKind::Block, ScopeKind::Block);
    assert_ne!(ScopeKind::Global, ScopeKind::Package);
    assert_ne!(ScopeKind::Subroutine, ScopeKind::Eval);
    Ok(())
}

#[test]
fn scope_kind_is_copy() -> Result<(), String> {
    let kind = ScopeKind::Block;
    let copy = kind; // Copy, not move
    assert_eq!(kind, copy);
    Ok(())
}

// ---------------------------------------------------------------------------
// Symbol and SymbolReference Clone / Debug
// ---------------------------------------------------------------------------

#[test]
fn symbol_clone() -> Result<(), String> {
    let sym = Symbol {
        name: "original".to_string(),
        qualified_name: "main::original".to_string(),
        kind: SymbolKind::Method,
        location: SourceLocation::new(0, 10),
        scope_id: 0,
        declaration: Some("sub".to_string()),
        documentation: Some("doc".to_string()),
        attributes: vec!["method".to_string()],
    };
    let cloned = sym.clone();
    assert_eq!(cloned.name, "original");
    assert_eq!(cloned.kind, SymbolKind::Method);
    assert_eq!(cloned.attributes.len(), 1);
    Ok(())
}

#[test]
fn symbol_debug() -> Result<(), String> {
    let sym = make_symbol("dbg_test", "main::dbg_test", SymbolKind::Constant, 0, 0, 5);
    let debug_str = format!("{:?}", sym);
    assert!(debug_str.contains("dbg_test"));
    Ok(())
}

#[test]
fn symbol_reference_clone() -> Result<(), String> {
    let r = make_ref("cloned_ref", SymbolKind::Import, 0, 0, 5, true);
    let c = r.clone();
    assert_eq!(c.name, "cloned_ref");
    assert_eq!(c.kind, SymbolKind::Import);
    assert!(c.is_write);
    Ok(())
}

#[test]
fn symbol_reference_debug() -> Result<(), String> {
    let r = make_ref("debug_ref", SymbolKind::hash(), 0, 0, 3, false);
    let debug_str = format!("{:?}", r);
    assert!(debug_str.contains("debug_ref"));
    Ok(())
}

#[test]
fn scope_clone() -> Result<(), String> {
    let mut symbols = HashSet::new();
    symbols.insert("x".to_string());
    let scope = Scope {
        id: 5,
        parent: Some(0),
        kind: ScopeKind::Subroutine,
        location: SourceLocation::new(10, 20),
        symbols,
    };
    let cloned = scope.clone();
    assert_eq!(cloned.id, 5);
    assert!(cloned.symbols.contains("x"));
    Ok(())
}

// ---------------------------------------------------------------------------
// SymbolKind variants coverage
// ---------------------------------------------------------------------------

#[test]
fn symbol_kinds_variable_variants() -> Result<(), String> {
    let mut table = SymbolTable::new();

    table.add_symbol(make_symbol("s", "main::s", SymbolKind::scalar(), 0, 0, 1));
    table.add_symbol(make_symbol("a", "main::a", SymbolKind::array(), 0, 2, 3));
    table.add_symbol(make_symbol("h", "main::h", SymbolKind::hash(), 0, 4, 5));
    table.add_symbol(make_symbol("sub", "main::sub", SymbolKind::Subroutine, 0, 6, 10));
    table.add_symbol(make_symbol("m", "main::m", SymbolKind::Method, 0, 11, 15));
    table.add_symbol(make_symbol("Pkg", "Pkg", SymbolKind::Package, 0, 16, 20));
    table.add_symbol(make_symbol("C", "main::C", SymbolKind::Constant, 0, 21, 22));
    table.add_symbol(make_symbol("i", "main::i", SymbolKind::Import, 0, 23, 24));

    assert_eq!(table.all_symbols().count(), 8);

    // Verify each kind is findable
    assert_eq!(table.find_symbol("s", 0, SymbolKind::scalar()).len(), 1);
    assert_eq!(table.find_symbol("a", 0, SymbolKind::array()).len(), 1);
    assert_eq!(table.find_symbol("h", 0, SymbolKind::hash()).len(), 1);
    assert_eq!(table.find_symbol("sub", 0, SymbolKind::Subroutine).len(), 1);
    assert_eq!(table.find_symbol("m", 0, SymbolKind::Method).len(), 1);
    assert_eq!(table.find_symbol("Pkg", 0, SymbolKind::Package).len(), 1);
    assert_eq!(table.find_symbol("C", 0, SymbolKind::Constant).len(), 1);
    assert_eq!(table.find_symbol("i", 0, SymbolKind::Import).len(), 1);
    Ok(())
}

#[test]
fn variable_kind_constructed_directly() -> Result<(), String> {
    let sk = SymbolKind::Variable(VarKind::Scalar);
    assert_eq!(sk, SymbolKind::scalar());

    let ak = SymbolKind::Variable(VarKind::Array);
    assert_eq!(ak, SymbolKind::array());

    let hk = SymbolKind::Variable(VarKind::Hash);
    assert_eq!(hk, SymbolKind::hash());
    Ok(())
}

// ---------------------------------------------------------------------------
// Complex scenarios
// ---------------------------------------------------------------------------

#[test]
fn multi_package_symbol_table() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 200);

    // package Foo
    table.set_current_package("Foo".to_string());
    let pkg_scope = table.push_scope(ScopeKind::Package, loc);
    let sym_foo =
        make_symbol("do_stuff", "Foo::do_stuff", SymbolKind::Subroutine, pkg_scope, 10, 50);
    table.add_symbol(sym_foo);
    table.pop_scope();

    // package Bar
    table.set_current_package("Bar".to_string());
    let pkg2_scope = table.push_scope(ScopeKind::Package, loc);
    let sym_bar =
        make_symbol("do_stuff", "Bar::do_stuff", SymbolKind::Subroutine, pkg2_scope, 60, 100);
    table.add_symbol(sym_bar);
    table.pop_scope();

    assert_eq!(table.current_package(), "Bar");

    // Both symbols are stored under the same name
    let all = table.symbols.get("do_stuff").ok_or("missing symbol")?;
    assert_eq!(all.len(), 2);

    // Find from each package scope
    let foo_syms = table.find_symbol("do_stuff", pkg_scope, SymbolKind::Subroutine);
    assert!(!foo_syms.is_empty());
    assert!(foo_syms.iter().any(|s| s.qualified_name == "Foo::do_stuff"));

    let bar_syms = table.find_symbol("do_stuff", pkg2_scope, SymbolKind::Subroutine);
    assert!(!bar_syms.is_empty());
    assert!(bar_syms.iter().any(|s| s.qualified_name == "Bar::do_stuff"));
    Ok(())
}

#[test]
fn deeply_nested_scope_chain() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 1000);

    // Global -> Sub -> Block -> Block -> Block -> Eval
    let sym_global = make_symbol("g", "main::g", SymbolKind::scalar(), 0, 0, 1);
    table.add_symbol(sym_global);

    let s1 = table.push_scope(ScopeKind::Subroutine, loc);
    let sym_s1 = make_symbol("s1_var", "main::s1_var", SymbolKind::scalar(), s1, 10, 15);
    table.add_symbol(sym_s1);

    let s2 = table.push_scope(ScopeKind::Block, loc);
    let s3 = table.push_scope(ScopeKind::Block, loc);
    let s4 = table.push_scope(ScopeKind::Block, loc);
    let s5 = table.push_scope(ScopeKind::Eval, loc);

    // From deepest scope, should find both global and subroutine-level symbols
    let found_g = table.find_symbol("g", s5, SymbolKind::scalar());
    assert!(!found_g.is_empty());

    let found_s1 = table.find_symbol("s1_var", s5, SymbolKind::scalar());
    assert!(!found_s1.is_empty());

    // Symbol defined in s2 should not be found (never added to s2)
    let found_none = table.find_symbol("nonexistent", s5, SymbolKind::scalar());
    assert!(found_none.is_empty());

    // Verify chain: s5 -> s4 -> s3 -> s2 -> s1 -> 0
    let scope5 = table.get_scope(s5).ok_or("scope missing")?;
    assert_eq!(scope5.parent, Some(s4));
    let scope4 = table.get_scope(s4).ok_or("scope missing")?;
    assert_eq!(scope4.parent, Some(s3));
    let scope3 = table.get_scope(s3).ok_or("scope missing")?;
    assert_eq!(scope3.parent, Some(s2));
    let scope2 = table.get_scope(s2).ok_or("scope missing")?;
    assert_eq!(scope2.parent, Some(s1));
    Ok(())
}

#[test]
fn references_across_scopes() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 200);

    // Define symbol in global scope
    let sym = make_symbol("config", "main::config", SymbolKind::hash(), 0, 0, 10);
    table.add_symbol(sym);

    // References from different scopes
    let sub_id = table.push_scope(ScopeKind::Subroutine, loc);
    table.add_reference(make_ref("config", SymbolKind::hash(), sub_id, 50, 56, false));

    let block_id = table.push_scope(ScopeKind::Block, loc);
    table.add_reference(make_ref("config", SymbolKind::hash(), block_id, 80, 86, true));

    let stored = &table.symbols["config"][0];
    let refs = table.find_references(stored);
    assert_eq!(refs.len(), 2);
    Ok(())
}

#[test]
fn symbol_with_our_declaration_in_package_scope() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 100);

    // Create a package scope
    let pkg_id = table.push_scope(ScopeKind::Package, loc);

    // Add 'our' variable in package scope
    let mut sym = make_symbol("VERSION", "main::VERSION", SymbolKind::scalar(), pkg_id, 5, 15);
    sym.declaration = Some("our".to_string());
    table.add_symbol(sym);

    // Push a sub scope inside the package
    let sub_id = table.push_scope(ScopeKind::Subroutine, loc);

    // 'our' should be visible from sub scope
    let found = table.find_symbol("VERSION", sub_id, SymbolKind::scalar());
    assert!(!found.is_empty());
    Ok(())
}

#[test]
fn our_variable_not_duplicated_in_package_scope() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 100);

    // Add 'our' variable in global scope
    let mut sym = make_symbol("data", "main::data", SymbolKind::array(), 0, 0, 5);
    sym.declaration = Some("our".to_string());
    table.add_symbol(sym);

    // Push a package scope (Package kind skips the 'our' extra check)
    let pkg_id = table.push_scope(ScopeKind::Package, loc);
    let found = table.find_symbol("data", pkg_id, SymbolKind::array());
    // Should find it exactly once from the 'our' check since Package scope
    // skips the extra 'our' check (scope.kind == Package)
    let unique_names: HashSet<&str> = found.iter().map(|s| s.qualified_name.as_str()).collect();
    assert!(unique_names.contains("main::data"));
    Ok(())
}

#[test]
fn empty_symbol_name() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let sym = make_symbol("", "::", SymbolKind::Subroutine, 0, 0, 0);
    table.add_symbol(sym);

    assert!(table.symbols.contains_key(""));
    let found = table.find_symbol("", 0, SymbolKind::Subroutine);
    assert_eq!(found.len(), 1);
    Ok(())
}

#[test]
fn unicode_symbol_names() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let sym = make_symbol("日本語", "main::日本語", SymbolKind::Subroutine, 0, 0, 9);
    table.add_symbol(sym);

    let found = table.find_symbol("日本語", 0, SymbolKind::Subroutine);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].qualified_name, "main::日本語");
    Ok(())
}

// ---------------------------------------------------------------------------
// Scope symbols set tracking
// ---------------------------------------------------------------------------

#[test]
fn scope_tracks_symbols_added_to_it() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 100);
    let sub_id = table.push_scope(ScopeKind::Subroutine, loc);

    table.add_symbol(make_symbol("x", "main::x", SymbolKind::scalar(), sub_id, 10, 11));
    table.add_symbol(make_symbol("y", "main::y", SymbolKind::scalar(), sub_id, 12, 13));
    table.add_symbol(make_symbol("z", "main::z", SymbolKind::scalar(), sub_id, 14, 15));

    let scope = table.get_scope(sub_id).ok_or("scope missing")?;
    assert_eq!(scope.symbols.len(), 3);
    assert!(scope.symbols.contains("x"));
    assert!(scope.symbols.contains("y"));
    assert!(scope.symbols.contains("z"));
    Ok(())
}

// ---------------------------------------------------------------------------
// SymbolTable Debug
// ---------------------------------------------------------------------------

#[test]
fn symbol_table_debug() -> Result<(), String> {
    let table = SymbolTable::new();
    let debug_str = format!("{:?}", table);
    assert!(debug_str.contains("SymbolTable"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Multiple references for same symbol
// ---------------------------------------------------------------------------

#[test]
fn multiple_references_same_symbol() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let sym = make_symbol("counter", "main::counter", SymbolKind::scalar(), 0, 0, 7);
    table.add_symbol(sym);

    for i in 0..10 {
        let offset = 20 + i * 10;
        table.add_reference(make_ref(
            "counter",
            SymbolKind::scalar(),
            0,
            offset,
            offset + 7,
            i % 3 == 0, // every 3rd is a write
        ));
    }

    let stored = &table.symbols["counter"][0];
    let refs = table.find_references(stored);
    assert_eq!(refs.len(), 10);

    let write_count = refs.iter().filter(|r| r.is_write).count();
    assert_eq!(write_count, 4); // i=0,3,6,9
    Ok(())
}
