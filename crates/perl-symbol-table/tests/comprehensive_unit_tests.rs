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

// ===========================================================================
// Additional comprehensive tests
// ===========================================================================

// ---------------------------------------------------------------------------
// Symbol shadowing in nested scopes
// ---------------------------------------------------------------------------

#[test]
fn find_symbol_shadowing_inner_scope_hides_outer() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 200);

    // Define $x in global scope
    let outer = make_symbol("x", "main::x", SymbolKind::scalar(), 0, 0, 5);
    table.add_symbol(outer);

    // Push a sub scope and define $x again
    let sub_id = table.push_scope(ScopeKind::Subroutine, loc);
    let inner = make_symbol("x", "main::x", SymbolKind::scalar(), sub_id, 20, 25);
    table.add_symbol(inner);

    // From sub scope, find_symbol should return the inner one first
    let found = table.find_symbol("x", sub_id, SymbolKind::scalar());
    // Both may be returned (inner first via scope walk), but inner scope match is present
    assert!(found.iter().any(|s| s.scope_id == sub_id));
    Ok(())
}

#[test]
fn find_symbol_shadowing_does_not_affect_outer_lookup() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 200);

    let outer = make_symbol("x", "main::x", SymbolKind::scalar(), 0, 0, 5);
    table.add_symbol(outer);

    let sub_id = table.push_scope(ScopeKind::Subroutine, loc);
    let inner = make_symbol("x", "main::x", SymbolKind::scalar(), sub_id, 20, 25);
    table.add_symbol(inner);

    // From global scope, only the global definition should be found
    let found = table.find_symbol("x", 0, SymbolKind::scalar());
    assert!(found.iter().all(|s| s.scope_id == 0));
    Ok(())
}

// ---------------------------------------------------------------------------
// Different symbol kinds with the same name
// ---------------------------------------------------------------------------

#[test]
fn same_name_different_kinds_are_distinguishable() -> Result<(), String> {
    let mut table = SymbolTable::new();

    // sub foo and $foo in same scope
    table.add_symbol(make_symbol("foo", "main::foo", SymbolKind::Subroutine, 0, 0, 10));
    table.add_symbol(make_symbol("foo", "main::foo", SymbolKind::scalar(), 0, 15, 20));

    let subs = table.find_symbol("foo", 0, SymbolKind::Subroutine);
    let scalars = table.find_symbol("foo", 0, SymbolKind::scalar());

    assert_eq!(subs.len(), 1);
    assert_eq!(scalars.len(), 1);
    assert_eq!(subs[0].kind, SymbolKind::Subroutine);
    assert_eq!(scalars[0].kind, SymbolKind::scalar());
    Ok(())
}

#[test]
fn same_name_scalar_array_hash() -> Result<(), String> {
    let mut table = SymbolTable::new();

    // Perl allows $x, @x, %x simultaneously
    table.add_symbol(make_symbol("x", "main::x", SymbolKind::scalar(), 0, 0, 2));
    table.add_symbol(make_symbol("x", "main::x", SymbolKind::array(), 0, 5, 7));
    table.add_symbol(make_symbol("x", "main::x", SymbolKind::hash(), 0, 10, 12));

    assert_eq!(table.find_symbol("x", 0, SymbolKind::scalar()).len(), 1);
    assert_eq!(table.find_symbol("x", 0, SymbolKind::array()).len(), 1);
    assert_eq!(table.find_symbol("x", 0, SymbolKind::hash()).len(), 1);
    Ok(())
}

// ---------------------------------------------------------------------------
// Sibling scopes isolation
// ---------------------------------------------------------------------------

#[test]
fn sibling_scopes_cannot_see_each_others_symbols() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 200);

    // Sub A
    let sub_a = table.push_scope(ScopeKind::Subroutine, loc);
    let sym_a = make_symbol("local_a", "main::local_a", SymbolKind::scalar(), sub_a, 10, 17);
    table.add_symbol(sym_a);
    table.pop_scope();

    // Sub B (sibling)
    let sub_b = table.push_scope(ScopeKind::Subroutine, loc);
    let sym_b = make_symbol("local_b", "main::local_b", SymbolKind::scalar(), sub_b, 50, 57);
    table.add_symbol(sym_b);
    table.pop_scope();

    // Sub A cannot see Sub B's symbols
    let found = table.find_symbol("local_b", sub_a, SymbolKind::scalar());
    assert!(found.is_empty());

    // Sub B cannot see Sub A's symbols
    let found = table.find_symbol("local_a", sub_b, SymbolKind::scalar());
    assert!(found.is_empty());
    Ok(())
}

#[test]
fn sibling_blocks_isolated() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 200);

    let sub_id = table.push_scope(ScopeKind::Subroutine, loc);

    // Block 1
    let blk1 = table.push_scope(ScopeKind::Block, loc);
    table.add_symbol(make_symbol("tmp", "main::tmp", SymbolKind::scalar(), blk1, 10, 13));
    table.pop_scope();

    // Block 2 (sibling of Block 1, both children of sub)
    let blk2 = table.push_scope(ScopeKind::Block, loc);

    // Block 2 cannot see Block 1's symbols
    let found = table.find_symbol("tmp", blk2, SymbolKind::scalar());
    assert!(found.is_empty());

    // But Block 2 can see the parent sub scope
    table.add_symbol(make_symbol("sub_var", "main::sub_var", SymbolKind::scalar(), sub_id, 5, 12));
    let found = table.find_symbol("sub_var", blk2, SymbolKind::scalar());
    assert!(!found.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// Declaration type variants
// ---------------------------------------------------------------------------

#[test]
fn declaration_my_local_state_variants() -> Result<(), String> {
    let mut table = SymbolTable::new();

    let mut sym_my = make_symbol("a", "main::a", SymbolKind::scalar(), 0, 0, 2);
    sym_my.declaration = Some("my".to_string());
    table.add_symbol(sym_my);

    let mut sym_local = make_symbol("b", "main::b", SymbolKind::scalar(), 0, 5, 7);
    sym_local.declaration = Some("local".to_string());
    table.add_symbol(sym_local);

    let mut sym_state = make_symbol("c", "main::c", SymbolKind::scalar(), 0, 10, 12);
    sym_state.declaration = Some("state".to_string());
    table.add_symbol(sym_state);

    let mut sym_our = make_symbol("d", "main::d", SymbolKind::scalar(), 0, 15, 17);
    sym_our.declaration = Some("our".to_string());
    table.add_symbol(sym_our);

    if let Some(syms) = table.symbols.get("a") {
        assert_eq!(syms[0].declaration.as_deref(), Some("my"));
    }
    if let Some(syms) = table.symbols.get("b") {
        assert_eq!(syms[0].declaration.as_deref(), Some("local"));
    }
    if let Some(syms) = table.symbols.get("c") {
        assert_eq!(syms[0].declaration.as_deref(), Some("state"));
    }
    if let Some(syms) = table.symbols.get("d") {
        assert_eq!(syms[0].declaration.as_deref(), Some("our"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Our variable visibility rules
// ---------------------------------------------------------------------------

#[test]
fn our_variable_visible_from_deeply_nested_scope() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 500);

    let mut sym = make_symbol("GLOBAL", "main::GLOBAL", SymbolKind::scalar(), 0, 0, 6);
    sym.declaration = Some("our".to_string());
    table.add_symbol(sym);

    // Nest 5 levels deep
    let _s1 = table.push_scope(ScopeKind::Subroutine, loc);
    let _s2 = table.push_scope(ScopeKind::Block, loc);
    let _s3 = table.push_scope(ScopeKind::Block, loc);
    let _s4 = table.push_scope(ScopeKind::Block, loc);
    let s5 = table.push_scope(ScopeKind::Eval, loc);

    let found = table.find_symbol("GLOBAL", s5, SymbolKind::scalar());
    assert!(!found.is_empty());
    Ok(())
}

#[test]
fn our_variable_not_found_when_kind_mismatches() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 100);

    let mut sym = make_symbol("data", "main::data", SymbolKind::scalar(), 0, 0, 5);
    sym.declaration = Some("our".to_string());
    table.add_symbol(sym);

    let sub_id = table.push_scope(ScopeKind::Subroutine, loc);
    // Looking for array kind should not find scalar our variable
    let found = table.find_symbol("data", sub_id, SymbolKind::array());
    assert!(found.is_empty());
    Ok(())
}

#[test]
fn my_variable_not_treated_as_our() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 100);

    // 'my' variable in global scope
    let mut sym = make_symbol("private", "main::private", SymbolKind::scalar(), 0, 0, 7);
    sym.declaration = Some("my".to_string());
    table.add_symbol(sym);

    // Push a package scope (skips extra our check)
    let pkg_id = table.push_scope(ScopeKind::Package, loc);

    // 'my' variable is still visible via scope chain walk (global is parent of package)
    let found = table.find_symbol("private", pkg_id, SymbolKind::scalar());
    // It's found via the normal scope chain, but not the 'our' special path
    assert!(!found.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// Scope ID uniqueness and monotonicity
// ---------------------------------------------------------------------------

#[test]
fn scope_ids_are_unique_and_monotonic() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 1);
    let mut ids = Vec::new();

    for _ in 0..20 {
        ids.push(table.push_scope(ScopeKind::Block, loc));
    }

    // All IDs should be unique
    let unique: HashSet<ScopeId> = ids.iter().copied().collect();
    assert_eq!(unique.len(), ids.len());

    // IDs should be monotonically increasing
    for window in ids.windows(2) {
        assert!(window[0] < window[1]);
    }
    Ok(())
}

#[test]
fn scope_ids_continue_after_pop() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 1);

    let id1 = table.push_scope(ScopeKind::Block, loc);
    table.pop_scope();
    let id2 = table.push_scope(ScopeKind::Block, loc);
    table.pop_scope();
    let id3 = table.push_scope(ScopeKind::Block, loc);

    // IDs keep incrementing even after pops
    assert!(id1 < id2);
    assert!(id2 < id3);
    Ok(())
}

// ---------------------------------------------------------------------------
// Reference tracking edge cases
// ---------------------------------------------------------------------------

#[test]
fn references_with_different_scopes_tracked_together() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 200);

    let sym = make_symbol("shared", "main::shared", SymbolKind::Subroutine, 0, 0, 6);
    table.add_symbol(sym);

    let sub1 = table.push_scope(ScopeKind::Subroutine, loc);
    table.add_reference(make_ref("shared", SymbolKind::Subroutine, sub1, 20, 26, false));
    table.pop_scope();

    let sub2 = table.push_scope(ScopeKind::Subroutine, loc);
    table.add_reference(make_ref("shared", SymbolKind::Subroutine, sub2, 50, 56, false));
    table.pop_scope();

    let stored = &table.symbols["shared"][0];
    let refs = table.find_references(stored);
    assert_eq!(refs.len(), 2);
    // References from different scopes
    let scope_ids: HashSet<ScopeId> = refs.iter().map(|r| r.scope_id).collect();
    assert_eq!(scope_ids.len(), 2);
    Ok(())
}

#[test]
fn reference_preserves_location() -> Result<(), String> {
    let mut table = SymbolTable::new();
    table.add_reference(make_ref("var", SymbolKind::scalar(), 0, 42, 45, false));

    if let Some(refs) = table.references.get("var") {
        assert_eq!(refs[0].location.start, 42);
        assert_eq!(refs[0].location.end, 45);
    }
    Ok(())
}

#[test]
fn reference_ordering_preserved() -> Result<(), String> {
    let mut table = SymbolTable::new();

    // Add references in specific order
    for i in 0..5 {
        let offset = i * 10;
        table.add_reference(make_ref("seq", SymbolKind::scalar(), 0, offset, offset + 3, false));
    }

    if let Some(refs) = table.references.get("seq") {
        // Insertion order is preserved
        for (i, r) in refs.iter().enumerate() {
            assert_eq!(r.location.start, i * 10);
        }
    }
    Ok(())
}

#[test]
fn find_references_with_no_matching_name() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let sym = make_symbol("alpha", "main::alpha", SymbolKind::Subroutine, 0, 0, 5);
    table.add_symbol(sym);

    // Add reference for a different name
    table.add_reference(make_ref("beta", SymbolKind::Subroutine, 0, 10, 14, false));

    let stored = &table.symbols["alpha"][0];
    let refs = table.find_references(stored);
    assert!(refs.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// All remaining SymbolKind variants
// ---------------------------------------------------------------------------

#[test]
fn symbol_kind_export() -> Result<(), String> {
    let mut table = SymbolTable::new();
    table.add_symbol(make_symbol("func", "Exporter::func", SymbolKind::Export, 0, 0, 4));
    let found = table.find_symbol("func", 0, SymbolKind::Export);
    assert_eq!(found.len(), 1);
    Ok(())
}

#[test]
fn symbol_kind_label() -> Result<(), String> {
    let mut table = SymbolTable::new();
    table.add_symbol(make_symbol("OUTER", "main::OUTER", SymbolKind::Label, 0, 0, 5));
    let found = table.find_symbol("OUTER", 0, SymbolKind::Label);
    assert_eq!(found.len(), 1);
    Ok(())
}

#[test]
fn symbol_kind_format() -> Result<(), String> {
    let mut table = SymbolTable::new();
    table.add_symbol(make_symbol("STDOUT", "main::STDOUT", SymbolKind::Format, 0, 0, 6));
    let found = table.find_symbol("STDOUT", 0, SymbolKind::Format);
    assert_eq!(found.len(), 1);
    Ok(())
}

#[test]
fn symbol_kind_class() -> Result<(), String> {
    let mut table = SymbolTable::new();
    table.add_symbol(make_symbol("MyClass", "MyClass", SymbolKind::Class, 0, 0, 7));
    let found = table.find_symbol("MyClass", 0, SymbolKind::Class);
    assert_eq!(found.len(), 1);
    Ok(())
}

#[test]
fn symbol_kind_role() -> Result<(), String> {
    let mut table = SymbolTable::new();
    table.add_symbol(make_symbol("Printable", "Printable", SymbolKind::Role, 0, 0, 9));
    let found = table.find_symbol("Printable", 0, SymbolKind::Role);
    assert_eq!(found.len(), 1);
    Ok(())
}

// ---------------------------------------------------------------------------
// Attributes
// ---------------------------------------------------------------------------

#[test]
fn symbol_with_multiple_attributes() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let sym = Symbol {
        name: "handler".to_string(),
        qualified_name: "main::handler".to_string(),
        kind: SymbolKind::Subroutine,
        location: SourceLocation::new(0, 50),
        scope_id: 0,
        declaration: Some("sub".to_string()),
        documentation: None,
        attributes: vec!["method".to_string(), "lvalue".to_string(), "shared".to_string()],
    };
    table.add_symbol(sym);

    if let Some(syms) = table.symbols.get("handler") {
        let s = &syms[0];
        assert_eq!(s.attributes.len(), 3);
        assert!(s.attributes.contains(&"method".to_string()));
        assert!(s.attributes.contains(&"lvalue".to_string()));
        assert!(s.attributes.contains(&"shared".to_string()));
    }
    Ok(())
}

#[test]
fn symbol_with_empty_attributes() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let sym = make_symbol("plain", "main::plain", SymbolKind::Subroutine, 0, 0, 5);
    table.add_symbol(sym);

    if let Some(syms) = table.symbols.get("plain") {
        assert!(syms[0].attributes.is_empty());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Documentation field
// ---------------------------------------------------------------------------

#[test]
fn symbol_documentation_preserved() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let sym = Symbol {
        name: "documented".to_string(),
        qualified_name: "main::documented".to_string(),
        kind: SymbolKind::Subroutine,
        location: SourceLocation::new(0, 20),
        scope_id: 0,
        declaration: None,
        documentation: Some("=head1 DESCRIPTION\n\nA well-documented function.".to_string()),
        attributes: vec![],
    };
    table.add_symbol(sym);

    if let Some(syms) = table.symbols.get("documented") {
        assert!(syms[0].documentation.as_deref().unwrap_or("").contains("well-documented"));
    }
    Ok(())
}

#[test]
fn symbol_no_documentation() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let sym = make_symbol("undoc", "main::undoc", SymbolKind::Subroutine, 0, 0, 5);
    table.add_symbol(sym);

    if let Some(syms) = table.symbols.get("undoc") {
        assert!(syms[0].documentation.is_none());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Package switching scenarios
// ---------------------------------------------------------------------------

#[test]
fn package_switching_back_and_forth() -> Result<(), String> {
    let mut table = SymbolTable::new();

    table.set_current_package("Foo".to_string());
    assert_eq!(table.current_package(), "Foo");

    table.set_current_package("Bar".to_string());
    assert_eq!(table.current_package(), "Bar");

    table.set_current_package("Foo".to_string());
    assert_eq!(table.current_package(), "Foo");

    table.set_current_package("main".to_string());
    assert_eq!(table.current_package(), "main");
    Ok(())
}

#[test]
fn package_name_with_deep_nesting() -> Result<(), String> {
    let mut table = SymbolTable::new();
    table.set_current_package("A::B::C::D::E".to_string());
    assert_eq!(table.current_package(), "A::B::C::D::E");
    Ok(())
}

#[test]
fn package_name_empty_string() -> Result<(), String> {
    let mut table = SymbolTable::new();
    table.set_current_package(String::new());
    assert_eq!(table.current_package(), "");
    Ok(())
}

// ---------------------------------------------------------------------------
// Scope parent chain verification
// ---------------------------------------------------------------------------

#[test]
fn scope_parent_chain_three_levels() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 100);

    let pkg = table.push_scope(ScopeKind::Package, loc);
    let sub = table.push_scope(ScopeKind::Subroutine, loc);
    let blk = table.push_scope(ScopeKind::Block, loc);

    // Verify complete chain: blk -> sub -> pkg -> 0(global)
    let blk_scope = table.get_scope(blk).ok_or("block scope missing")?;
    assert_eq!(blk_scope.parent, Some(sub));

    let sub_scope = table.get_scope(sub).ok_or("sub scope missing")?;
    assert_eq!(sub_scope.parent, Some(pkg));

    let pkg_scope = table.get_scope(pkg).ok_or("pkg scope missing")?;
    assert_eq!(pkg_scope.parent, Some(0));

    let global = table.get_scope(0).ok_or("global scope missing")?;
    assert!(global.parent.is_none());
    Ok(())
}

// ---------------------------------------------------------------------------
// Scope with multiple push/pop cycles
// ---------------------------------------------------------------------------

#[test]
fn multiple_push_pop_cycles_state_consistent() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 100);

    for _ in 0..10 {
        let id = table.push_scope(ScopeKind::Block, loc);
        assert_eq!(table.current_scope(), id);
        table.pop_scope();
        assert_eq!(table.current_scope(), 0);
    }
    Ok(())
}

#[test]
fn push_pop_does_not_remove_scopes_from_table() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 100);

    let id = table.push_scope(ScopeKind::Subroutine, loc);
    table.pop_scope();

    // Scope still exists in the table even after pop
    assert!(table.get_scope(id).is_some());
    Ok(())
}

// ---------------------------------------------------------------------------
// Symbols survive scope pop
// ---------------------------------------------------------------------------

#[test]
fn symbols_persist_after_scope_pop() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 100);

    let sub_id = table.push_scope(ScopeKind::Subroutine, loc);
    table.add_symbol(make_symbol("temp", "main::temp", SymbolKind::scalar(), sub_id, 10, 14));
    table.pop_scope();

    // Symbol is still in the table
    assert!(table.symbols.contains_key("temp"));
    // And can still be found from the scope it was defined in
    let found = table.find_symbol("temp", sub_id, SymbolKind::scalar());
    assert_eq!(found.len(), 1);
    Ok(())
}

// ---------------------------------------------------------------------------
// Default trait behavior
// ---------------------------------------------------------------------------

#[test]
fn default_table_has_no_scopes() -> Result<(), String> {
    let table = SymbolTable::default();
    assert!(table.scopes.is_empty());
    assert!(table.symbols.is_empty());
    assert!(table.references.is_empty());
    Ok(())
}

#[test]
fn default_table_current_scope_fallback() -> Result<(), String> {
    let table = SymbolTable::default();
    // With empty scope stack, falls back to 0
    assert_eq!(table.current_scope(), 0);
    Ok(())
}

// ---------------------------------------------------------------------------
// Stress / scale tests
// ---------------------------------------------------------------------------

#[test]
fn many_symbols_in_single_scope() -> Result<(), String> {
    let mut table = SymbolTable::new();

    for i in 0..100 {
        let name = format!("var_{}", i);
        let qname = format!("main::var_{}", i);
        table.add_symbol(make_symbol(&name, &qname, SymbolKind::scalar(), 0, i * 5, i * 5 + 4));
    }

    assert_eq!(table.all_symbols().count(), 100);

    // Spot check a few
    let found = table.find_symbol("var_0", 0, SymbolKind::scalar());
    assert_eq!(found.len(), 1);
    let found = table.find_symbol("var_99", 0, SymbolKind::scalar());
    assert_eq!(found.len(), 1);
    Ok(())
}

#[test]
fn many_scopes_with_symbols() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 10000);

    for i in 0..50 {
        let scope_id = table.push_scope(ScopeKind::Subroutine, loc);
        let name = format!("fn_{}", i);
        let qname = format!("main::fn_{}", i);
        table.add_symbol(make_symbol(
            &name,
            &qname,
            SymbolKind::Subroutine,
            scope_id,
            i * 100,
            i * 100 + 50,
        ));
        table.pop_scope();
    }

    assert_eq!(table.all_symbols().count(), 50);
    // 50 sub scopes + 1 global
    assert_eq!(table.scopes.len(), 51);
    Ok(())
}

#[test]
fn many_references_for_single_symbol() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let sym = make_symbol("hot", "main::hot", SymbolKind::Subroutine, 0, 0, 3);
    table.add_symbol(sym);

    for i in 0..200 {
        let offset = 10 + i * 5;
        table.add_reference(make_ref("hot", SymbolKind::Subroutine, 0, offset, offset + 3, false));
    }

    let stored = &table.symbols["hot"][0];
    let refs = table.find_references(stored);
    assert_eq!(refs.len(), 200);
    Ok(())
}

// ---------------------------------------------------------------------------
// Scope location tracking
// ---------------------------------------------------------------------------

#[test]
fn scope_location_matches_push_argument() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(42, 128);
    let id = table.push_scope(ScopeKind::Subroutine, loc);

    let scope = table.get_scope(id).ok_or("scope missing")?;
    assert_eq!(scope.location.start, 42);
    assert_eq!(scope.location.end, 128);
    Ok(())
}

#[test]
fn global_scope_has_zero_location() -> Result<(), String> {
    let table = SymbolTable::new();
    let global = table.get_scope(0).ok_or("global scope missing")?;
    assert_eq!(global.location.start, 0);
    assert_eq!(global.location.end, 0);
    Ok(())
}

// ---------------------------------------------------------------------------
// SymbolKind predicate methods
// ---------------------------------------------------------------------------

#[test]
fn symbol_kind_is_variable() -> Result<(), String> {
    assert!(SymbolKind::scalar().is_variable());
    assert!(SymbolKind::array().is_variable());
    assert!(SymbolKind::hash().is_variable());
    assert!(!SymbolKind::Subroutine.is_variable());
    assert!(!SymbolKind::Package.is_variable());
    Ok(())
}

#[test]
fn symbol_kind_is_callable() -> Result<(), String> {
    assert!(SymbolKind::Subroutine.is_callable());
    assert!(SymbolKind::Method.is_callable());
    assert!(!SymbolKind::scalar().is_callable());
    assert!(!SymbolKind::Package.is_callable());
    assert!(!SymbolKind::Constant.is_callable());
    Ok(())
}

#[test]
fn symbol_kind_is_namespace() -> Result<(), String> {
    assert!(SymbolKind::Package.is_namespace());
    assert!(SymbolKind::Class.is_namespace());
    assert!(SymbolKind::Role.is_namespace());
    assert!(!SymbolKind::Subroutine.is_namespace());
    assert!(!SymbolKind::scalar().is_namespace());
    Ok(())
}

// ---------------------------------------------------------------------------
// VarKind sigil
// ---------------------------------------------------------------------------

#[test]
fn var_kind_sigil() -> Result<(), String> {
    assert_eq!(VarKind::Scalar.sigil(), "$");
    assert_eq!(VarKind::Array.sigil(), "@");
    assert_eq!(VarKind::Hash.sigil(), "%");
    Ok(())
}

#[test]
fn symbol_kind_sigil() -> Result<(), String> {
    assert_eq!(SymbolKind::scalar().sigil(), Some("$"));
    assert_eq!(SymbolKind::array().sigil(), Some("@"));
    assert_eq!(SymbolKind::hash().sigil(), Some("%"));
    assert_eq!(SymbolKind::Subroutine.sigil(), None);
    Ok(())
}

// ---------------------------------------------------------------------------
// find_symbol edge cases
// ---------------------------------------------------------------------------

#[test]
fn find_symbol_from_scope_with_no_parent_link() -> Result<(), String> {
    // Default table has no scopes, so a lookup from scope 0 finds nothing
    let table = SymbolTable::default();
    let found = table.find_symbol("anything", 0, SymbolKind::Subroutine);
    assert!(found.is_empty());
    Ok(())
}

#[test]
fn find_symbol_multiple_our_in_nested_non_package_scopes() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 200);

    // Two 'our' variables with same name in different scopes
    let mut sym1 = make_symbol("shared", "main::shared", SymbolKind::scalar(), 0, 0, 6);
    sym1.declaration = Some("our".to_string());
    table.add_symbol(sym1);

    let sub_id = table.push_scope(ScopeKind::Subroutine, loc);
    let block_id = table.push_scope(ScopeKind::Block, loc);

    // From block scope, 'our' variable should be found through the special our check
    let found = table.find_symbol("shared", block_id, SymbolKind::scalar());
    assert!(!found.is_empty());

    // From sub scope too
    let found2 = table.find_symbol("shared", sub_id, SymbolKind::scalar());
    assert!(!found2.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// Interleaved scope operations with symbols
// ---------------------------------------------------------------------------

#[test]
fn interleaved_scope_and_symbol_operations() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 500);

    // Add global symbol
    table.add_symbol(make_symbol("g", "main::g", SymbolKind::scalar(), 0, 0, 1));

    // Enter sub, add symbol, enter block, add symbol
    let sub_id = table.push_scope(ScopeKind::Subroutine, loc);
    table.add_symbol(make_symbol("s", "main::s", SymbolKind::scalar(), sub_id, 10, 11));

    let blk_id = table.push_scope(ScopeKind::Block, loc);
    table.add_symbol(make_symbol("b", "main::b", SymbolKind::scalar(), blk_id, 20, 21));

    // From block: can see g, s, b
    assert!(!table.find_symbol("g", blk_id, SymbolKind::scalar()).is_empty());
    assert!(!table.find_symbol("s", blk_id, SymbolKind::scalar()).is_empty());
    assert!(!table.find_symbol("b", blk_id, SymbolKind::scalar()).is_empty());

    // From sub: can see g, s but not b
    assert!(!table.find_symbol("g", sub_id, SymbolKind::scalar()).is_empty());
    assert!(!table.find_symbol("s", sub_id, SymbolKind::scalar()).is_empty());
    assert!(table.find_symbol("b", sub_id, SymbolKind::scalar()).is_empty());

    // From global: can only see g
    assert!(!table.find_symbol("g", 0, SymbolKind::scalar()).is_empty());
    assert!(table.find_symbol("s", 0, SymbolKind::scalar()).is_empty());
    assert!(table.find_symbol("b", 0, SymbolKind::scalar()).is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// Symbol qualified_name tracking
// ---------------------------------------------------------------------------

#[test]
fn qualified_names_preserved_for_multiple_packages() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 300);

    // Three packages define 'new'
    for pkg in &["Foo", "Bar", "Baz"] {
        let scope_id = table.push_scope(ScopeKind::Package, loc);
        let qname = format!("{}::new", pkg);
        table.add_symbol(make_symbol("new", &qname, SymbolKind::Subroutine, scope_id, 0, 3));
        table.pop_scope();
    }

    if let Some(syms) = table.symbols.get("new") {
        assert_eq!(syms.len(), 3);
        let qnames: HashSet<&str> = syms.iter().map(|s| s.qualified_name.as_str()).collect();
        assert!(qnames.contains("Foo::new"));
        assert!(qnames.contains("Bar::new"));
        assert!(qnames.contains("Baz::new"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// References with write vs read patterns
// ---------------------------------------------------------------------------

#[test]
fn all_read_references() -> Result<(), String> {
    let mut table = SymbolTable::new();

    for i in 0..5 {
        table.add_reference(make_ref("val", SymbolKind::scalar(), 0, i * 10, i * 10 + 3, false));
    }

    if let Some(refs) = table.references.get("val") {
        assert!(refs.iter().all(|r| !r.is_write));
    }
    Ok(())
}

#[test]
fn all_write_references() -> Result<(), String> {
    let mut table = SymbolTable::new();

    for i in 0..5 {
        table.add_reference(make_ref("val", SymbolKind::scalar(), 0, i * 10, i * 10 + 3, true));
    }

    if let Some(refs) = table.references.get("val") {
        assert!(refs.iter().all(|r| r.is_write));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Scope symbols set: adding same symbol name twice
// ---------------------------------------------------------------------------

#[test]
fn scope_symbols_set_deduplicates() -> Result<(), String> {
    let mut table = SymbolTable::new();

    // Add two symbols named "x" to scope 0 (HashSet deduplicates in scope.symbols)
    table.add_symbol(make_symbol("x", "main::x", SymbolKind::scalar(), 0, 0, 1));
    table.add_symbol(make_symbol("x", "main::x", SymbolKind::array(), 0, 5, 6));

    let scope = table.get_scope(0).ok_or("global scope missing")?;
    // HashSet contains "x" once even though two symbols have that name
    assert_eq!(scope.symbols.len(), 1);
    assert!(scope.symbols.contains("x"));

    // But the symbols vec has both entries
    assert_eq!(table.symbols["x"].len(), 2);
    Ok(())
}

// ---------------------------------------------------------------------------
// Eval scope specifics
// ---------------------------------------------------------------------------

#[test]
fn eval_scope_inherits_parent_symbols() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 200);

    let sub_id = table.push_scope(ScopeKind::Subroutine, loc);
    table.add_symbol(make_symbol(
        "local_var",
        "main::local_var",
        SymbolKind::scalar(),
        sub_id,
        10,
        19,
    ));

    let eval_id = table.push_scope(ScopeKind::Eval, loc);

    // Eval scope should see parent's symbols
    let found = table.find_symbol("local_var", eval_id, SymbolKind::scalar());
    assert!(!found.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// SymbolKind equality semantics
// ---------------------------------------------------------------------------

#[test]
fn variable_kinds_not_equal_to_non_variable() -> Result<(), String> {
    assert_ne!(SymbolKind::scalar(), SymbolKind::Subroutine);
    assert_ne!(SymbolKind::array(), SymbolKind::Method);
    assert_ne!(SymbolKind::hash(), SymbolKind::Package);
    Ok(())
}

#[test]
fn different_variable_kinds_not_equal() -> Result<(), String> {
    assert_ne!(SymbolKind::scalar(), SymbolKind::array());
    assert_ne!(SymbolKind::array(), SymbolKind::hash());
    assert_ne!(SymbolKind::scalar(), SymbolKind::hash());
    Ok(())
}

// ---------------------------------------------------------------------------
// Symbol location tracking
// ---------------------------------------------------------------------------

#[test]
fn symbol_location_matches_construction() -> Result<(), String> {
    let sym = make_symbol("loc_test", "main::loc_test", SymbolKind::Subroutine, 0, 42, 99);
    assert_eq!(sym.location.start, 42);
    assert_eq!(sym.location.end, 99);
    assert_eq!(sym.location.len(), 57);
    assert!(!sym.location.is_empty());
    Ok(())
}

#[test]
fn symbol_zero_length_location() -> Result<(), String> {
    let sym = make_symbol("zero", "main::zero", SymbolKind::Label, 0, 10, 10);
    assert_eq!(sym.location.len(), 0);
    assert!(sym.location.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// Complex realistic scenario
// ---------------------------------------------------------------------------

#[test]
fn realistic_perl_module_scenario() -> Result<(), String> {
    let mut table = SymbolTable::new();
    let loc = SourceLocation::new(0, 1000);

    // package MyApp::Controller
    table.set_current_package("MyApp::Controller".to_string());
    let pkg_scope = table.push_scope(ScopeKind::Package, loc);

    // our $VERSION = '1.0';
    let mut version_sym = make_symbol(
        "VERSION",
        "MyApp::Controller::VERSION",
        SymbolKind::scalar(),
        pkg_scope,
        10,
        25,
    );
    version_sym.declaration = Some("our".to_string());
    table.add_symbol(version_sym);

    // sub new { ... }
    let new_scope = table.push_scope(ScopeKind::Subroutine, SourceLocation::new(30, 100));
    let new_sym =
        make_symbol("new", "MyApp::Controller::new", SymbolKind::Method, pkg_scope, 30, 33);
    table.add_symbol(new_sym);

    // my $self = bless {}, $class;
    let self_sym =
        make_symbol("self", "MyApp::Controller::self", SymbolKind::scalar(), new_scope, 40, 44);
    table.add_symbol(self_sym);

    // Reference to $self
    table.add_reference(make_ref("self", SymbolKind::scalar(), new_scope, 60, 64, false));

    table.pop_scope(); // end sub new

    // sub handle_request { ... }
    let handle_scope = table.push_scope(ScopeKind::Subroutine, SourceLocation::new(110, 300));
    let handle_sym = make_symbol(
        "handle_request",
        "MyApp::Controller::handle_request",
        SymbolKind::Method,
        pkg_scope,
        110,
        124,
    );
    table.add_symbol(handle_sym);

    // my ($self, $request) = @_;
    table.add_symbol(make_symbol(
        "self",
        "MyApp::Controller::self",
        SymbolKind::scalar(),
        handle_scope,
        130,
        134,
    ));
    table.add_symbol(make_symbol(
        "request",
        "MyApp::Controller::request",
        SymbolKind::scalar(),
        handle_scope,
        136,
        143,
    ));

    // if block
    let if_scope = table.push_scope(ScopeKind::Block, SourceLocation::new(150, 250));
    table.add_symbol(make_symbol(
        "response",
        "MyApp::Controller::response",
        SymbolKind::scalar(),
        if_scope,
        160,
        168,
    ));

    // References
    table.add_reference(make_ref("self", SymbolKind::scalar(), if_scope, 170, 174, false));
    table.add_reference(make_ref("request", SymbolKind::scalar(), if_scope, 180, 187, false));
    table.add_reference(make_ref("VERSION", SymbolKind::scalar(), if_scope, 190, 197, false));

    table.pop_scope(); // end if
    table.pop_scope(); // end sub handle_request
    table.pop_scope(); // end package

    // Verify: from if_scope, can see self, request, VERSION
    assert!(!table.find_symbol("self", if_scope, SymbolKind::scalar()).is_empty());
    assert!(!table.find_symbol("request", if_scope, SymbolKind::scalar()).is_empty());
    // VERSION is 'our', visible from non-package scopes
    assert!(!table.find_symbol("VERSION", if_scope, SymbolKind::scalar()).is_empty());

    // response is only visible from if_scope, not handle_scope
    assert!(!table.find_symbol("response", if_scope, SymbolKind::scalar()).is_empty());
    assert!(table.find_symbol("response", handle_scope, SymbolKind::scalar()).is_empty());

    // Verify references
    if let Some(self_syms) = table.symbols.get("self") {
        // There are 2 "self" symbols (one per sub)
        assert_eq!(self_syms.len(), 2);
    }

    assert_eq!(table.current_package(), "MyApp::Controller");
    Ok(())
}

// ---------------------------------------------------------------------------
// SymbolKind to_lsp_kind
// ---------------------------------------------------------------------------

#[test]
fn symbol_kind_to_lsp_kind_nonzero() -> Result<(), String> {
    // All SymbolKind variants should produce non-zero LSP kind values
    let kinds = [
        SymbolKind::Package,
        SymbolKind::Class,
        SymbolKind::Role,
        SymbolKind::Subroutine,
        SymbolKind::Method,
        SymbolKind::scalar(),
        SymbolKind::array(),
        SymbolKind::hash(),
        SymbolKind::Constant,
        SymbolKind::Import,
        SymbolKind::Export,
        SymbolKind::Label,
        SymbolKind::Format,
    ];
    for kind in &kinds {
        assert!(kind.to_lsp_kind() > 0, "to_lsp_kind should be > 0 for {:?}", kind);
        assert!(
            kind.to_lsp_kind_document_symbol() > 0,
            "to_lsp_kind_document_symbol should be > 0 for {:?}",
            kind
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Scope kind Debug trait
// ---------------------------------------------------------------------------

#[test]
fn scope_kind_debug_output() -> Result<(), String> {
    let kinds = [
        ScopeKind::Global,
        ScopeKind::Package,
        ScopeKind::Subroutine,
        ScopeKind::Block,
        ScopeKind::Eval,
    ];
    for kind in &kinds {
        let dbg = format!("{:?}", kind);
        assert!(!dbg.is_empty());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Scope Debug trait
// ---------------------------------------------------------------------------

#[test]
fn scope_debug_contains_id() -> Result<(), String> {
    let scope = Scope {
        id: 42,
        parent: Some(1),
        kind: ScopeKind::Subroutine,
        location: SourceLocation::new(0, 100),
        symbols: HashSet::new(),
    };
    let dbg = format!("{:?}", scope);
    assert!(dbg.contains("42"));
    assert!(dbg.contains("Subroutine"));
    Ok(())
}

// ---------------------------------------------------------------------------
// SymbolReference Debug and fields
// ---------------------------------------------------------------------------

#[test]
fn symbol_reference_fields_accessible() -> Result<(), String> {
    let r = SymbolReference {
        name: "test_ref".to_string(),
        kind: SymbolKind::Method,
        location: SourceLocation::new(100, 108),
        scope_id: 5,
        is_write: true,
    };

    assert_eq!(r.name, "test_ref");
    assert_eq!(r.kind, SymbolKind::Method);
    assert_eq!(r.location.start, 100);
    assert_eq!(r.location.end, 108);
    assert_eq!(r.scope_id, 5);
    assert!(r.is_write);
    Ok(())
}
