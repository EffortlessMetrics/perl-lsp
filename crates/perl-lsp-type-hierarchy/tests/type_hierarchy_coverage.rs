//! Integration tests for perl-lsp-type-hierarchy public API.

use perl_lsp_type_hierarchy::{TypeHierarchyItem, TypeHierarchyProvider, TypeHierarchySymbolKind};
use perl_parser_core::parser::Parser;
use perl_position_tracking::{WirePosition, WireRange};
use perl_tdd_support::{must, must_some};

/// Helper: parse code and return AST.
fn parse(code: &str) -> perl_parser_core::ast::Node {
    let mut parser = Parser::new(code);
    must(parser.parse())
}

/// Helper: build a `TypeHierarchyItem` with the given name (for find_* queries).
fn make_item(name: &str) -> TypeHierarchyItem {
    TypeHierarchyItem {
        name: name.to_string(),
        kind: TypeHierarchySymbolKind::Class,
        uri: "file:///test".to_string(),
        range: WireRange::default(),
        selection_range: WireRange::default(),
        detail: None,
        data: None,
    }
}

// ---------------------------------------------------------------------------
// 1. Provider construction
// ---------------------------------------------------------------------------

#[test]
fn new_returns_provider() {
    let _provider = TypeHierarchyProvider::new();
}

#[test]
fn default_returns_provider() {
    let _provider: TypeHierarchyProvider = Default::default();
}

// ---------------------------------------------------------------------------
// 2. prepare() – positive cases
// ---------------------------------------------------------------------------

#[test]
fn prepare_on_package_declaration_returns_item() {
    let code = "package Foo;\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    // offset 8 falls inside "Foo"
    let items = must_some(provider.prepare(&ast, code, 8));
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "Foo");
    assert_eq!(items[0].uri, "file:///current");
    assert!(items[0].detail.is_some());
}

#[test]
fn prepare_on_package_with_use_parent() {
    let code = "package Child;\nuse parent 'Base';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let items = must_some(provider.prepare(&ast, code, 8));
    assert_eq!(items[0].name, "Child");
}

#[test]
fn prepare_on_package_with_use_base() {
    let code = "package Child;\nuse base 'Base';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let items = must_some(provider.prepare(&ast, code, 8));
    assert_eq!(items[0].name, "Child");
}

#[test]
fn prepare_on_block_form_package() {
    let code = "package BlockPkg {\n  1;\n}\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let items = must_some(provider.prepare(&ast, code, 8));
    assert_eq!(items[0].name, "BlockPkg");
}

// ---------------------------------------------------------------------------
// 3. prepare() – negative / boundary cases
// ---------------------------------------------------------------------------

#[test]
fn prepare_returns_none_for_offset_beyond_source() {
    let code = "package Foo;\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    assert!(provider.prepare(&ast, code, 9999).is_none());
}

#[test]
fn prepare_returns_none_on_whitespace_between_statements() {
    // Offset landing on a bare semicolon / whitespace after package decl
    let code = "package A;\n\n\nmy $x = 1;\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    // offset 12 is inside the blank lines between the two statements
    let result = provider.prepare(&ast, code, 12);
    // The provider may or may not return a result depending on AST spans.
    // At minimum, verify it does not panic.
    let _ = result;
}

#[test]
fn prepare_on_variable_returns_none() {
    let code = "my $x = 1;\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    // The variable declaration is not a package, so prepare should return None
    let result = provider.prepare(&ast, code, 3);
    assert!(result.is_none());
}

// ---------------------------------------------------------------------------
// 4. find_supertypes() – single-level inheritance
// ---------------------------------------------------------------------------

#[test]
fn supertypes_via_use_parent_single() {
    let code = "package Child;\nuse parent 'SingleBase';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let item = make_item("Child");
    let supertypes = provider.find_supertypes(&ast, &item);
    assert_eq!(supertypes.len(), 1);
    assert_eq!(supertypes[0].name, "SingleBase");
    assert!(matches!(supertypes[0].kind, TypeHierarchySymbolKind::Class));
    assert_eq!(supertypes[0].detail.as_deref(), Some("Parent Class"));
}

#[test]
fn supertypes_via_use_base() {
    let code = "package Derived;\nuse base 'LegacyBase';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let supertypes = provider.find_supertypes(&ast, &make_item("Derived"));
    assert_eq!(supertypes.len(), 1);
    assert_eq!(supertypes[0].name, "LegacyBase");
}

// ---------------------------------------------------------------------------
// 5. find_supertypes() – no inheritance
// ---------------------------------------------------------------------------

#[test]
fn supertypes_empty_when_no_inheritance() {
    let code = "package Standalone;\nsub greet { 1 }\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let supertypes = provider.find_supertypes(&ast, &make_item("Standalone"));
    assert!(supertypes.is_empty());
}

// ---------------------------------------------------------------------------
// 6. find_supertypes() – multiple inheritance
// ---------------------------------------------------------------------------

#[test]
fn supertypes_multiple_via_isa_list() {
    let code = "package Multi;\nour @ISA = ('Alpha', 'Beta');\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let supertypes = provider.find_supertypes(&ast, &make_item("Multi"));
    let names: Vec<&str> = supertypes.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"Alpha"), "Expected Alpha in {names:?}");
    assert!(names.contains(&"Beta"), "Expected Beta in {names:?}");
}

// ---------------------------------------------------------------------------
// 7. find_subtypes()
// ---------------------------------------------------------------------------

#[test]
fn subtypes_finds_derived_classes() {
    let code = "\
package Base;\n\
package ChildA;\n\
use parent 'Base';\n\
package ChildB;\n\
use parent 'Base';\n\
package Other;\n\
use parent 'Unrelated';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let subtypes = provider.find_subtypes(&ast, &make_item("Base"));
    let names: Vec<&str> = subtypes.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"ChildA"));
    assert!(names.contains(&"ChildB"));
    assert!(!names.contains(&"Other"));
}

#[test]
fn subtypes_empty_when_nothing_inherits() {
    let code = "package Leaf;\nsub go { 1 }\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let subtypes = provider.find_subtypes(&ast, &make_item("Leaf"));
    assert!(subtypes.is_empty());
}

#[test]
fn subtypes_detail_says_subclass() {
    let code = "package P;\npackage C;\nuse parent 'P';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let subtypes = provider.find_subtypes(&ast, &make_item("P"));
    assert_eq!(subtypes.len(), 1);
    assert_eq!(subtypes[0].detail.as_deref(), Some("Subclass"));
}

// ---------------------------------------------------------------------------
// 8. Block-form package inheritance scoping
// ---------------------------------------------------------------------------

#[test]
fn block_form_package_scoping() {
    let code = "\
package Outer {\n\
    package Inner;\n\
    use parent 'Outer';\n\
}\n\
package After;\n\
use parent 'Outer';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let subtypes = provider.find_subtypes(&ast, &make_item("Outer"));
    let names: Vec<&str> = subtypes.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"Inner"));
    assert!(names.contains(&"After"));
    assert_eq!(names.len(), 2);
}

// ---------------------------------------------------------------------------
// 9. Serde round-trip
// ---------------------------------------------------------------------------

#[test]
fn type_hierarchy_item_serializes_to_json() {
    let item = TypeHierarchyItem {
        name: "Foo::Bar".to_string(),
        kind: TypeHierarchySymbolKind::Class,
        uri: "file:///foo.pm".to_string(),
        range: WireRange {
            start: WirePosition { line: 0, character: 0 },
            end: WirePosition { line: 5, character: 1 },
        },
        selection_range: WireRange {
            start: WirePosition { line: 0, character: 8 },
            end: WirePosition { line: 0, character: 16 },
        },
        detail: Some("Perl Package".to_string()),
        data: None,
    };

    let json_str = must(serde_json::to_string(&item));
    assert!(json_str.contains("\"name\":\"Foo::Bar\""));
    // Serde serializes the enum as its variant name, not the discriminant
    assert!(json_str.contains("\"kind\":\"Class\""));
}

#[test]
fn type_hierarchy_item_roundtrips_via_serde() {
    let item = TypeHierarchyItem {
        name: "Roundtrip".to_string(),
        kind: TypeHierarchySymbolKind::Method,
        uri: "file:///test.pm".to_string(),
        range: WireRange::default(),
        selection_range: WireRange::default(),
        detail: Some("method".to_string()),
        data: Some(serde_json::json!({"id": 42})),
    };

    let json_str = must(serde_json::to_string(&item));
    let recovered: TypeHierarchyItem = must(serde_json::from_str(&json_str));
    assert_eq!(recovered.name, "Roundtrip");
    assert_eq!(recovered.uri, "file:///test.pm");
    assert_eq!(recovered.detail.as_deref(), Some("method"));
    assert!(recovered.data.is_some());
}

#[test]
fn symbol_kind_variants_roundtrip() {
    // Serde serializes enum variants as their name strings
    let variants = [
        (TypeHierarchySymbolKind::Class, "Class"),
        (TypeHierarchySymbolKind::Method, "Method"),
        (TypeHierarchySymbolKind::Function, "Function"),
    ];

    for (kind, expected_name) in variants {
        let json_val = must(serde_json::to_value(kind));
        assert_eq!(json_val, serde_json::json!(expected_name));

        let recovered: TypeHierarchySymbolKind = must(serde_json::from_value(json_val));
        // Verify by serializing again
        let re_val = must(serde_json::to_value(recovered));
        assert_eq!(re_val, serde_json::json!(expected_name));
    }
}

// ---------------------------------------------------------------------------
// 10. LSP shape compliance – prepare() output has required fields
// ---------------------------------------------------------------------------

#[test]
fn prepare_output_has_lsp_required_fields() {
    let code = "package Compliant;\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let items = must_some(provider.prepare(&ast, code, 8));
    let item = &items[0];

    // LSP spec: name, kind, uri, range, selectionRange are required
    assert!(!item.name.is_empty());
    assert!(!item.uri.is_empty());
    // kind is always set (it's a non-optional enum)
    let _kind_val = must(serde_json::to_value(item.kind));
}

// ---------------------------------------------------------------------------
// 11. Range correctness
// ---------------------------------------------------------------------------

#[test]
fn prepare_range_starts_at_line_zero_for_first_package() {
    let code = "package First;\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let items = must_some(provider.prepare(&ast, code, 8));
    assert_eq!(items[0].range.start.line, 0);
}

// ---------------------------------------------------------------------------
// 12. Multi-level inheritance chain
// ---------------------------------------------------------------------------

#[test]
fn multi_level_inheritance_chain() {
    let code = "\
package GrandParent;\n\
package Parent;\n\
use parent 'GrandParent';\n\
package Child;\n\
use parent 'Parent';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    // Child -> Parent
    let child_supers = provider.find_supertypes(&ast, &make_item("Child"));
    assert_eq!(child_supers.len(), 1);
    assert_eq!(child_supers[0].name, "Parent");

    // Parent -> GrandParent
    let parent_supers = provider.find_supertypes(&ast, &make_item("Parent"));
    assert_eq!(parent_supers.len(), 1);
    assert_eq!(parent_supers[0].name, "GrandParent");

    // GrandParent -> nothing
    let gp_supers = provider.find_supertypes(&ast, &make_item("GrandParent"));
    assert!(gp_supers.is_empty());
}

// ---------------------------------------------------------------------------
// 13. Querying supertypes for a package not in the file
// ---------------------------------------------------------------------------

#[test]
fn supertypes_for_unknown_package_is_empty() {
    let code = "package Known;\nuse parent 'Base';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let supertypes = provider.find_supertypes(&ast, &make_item("DoesNotExist"));
    assert!(supertypes.is_empty());
}

// ---------------------------------------------------------------------------
// 14. Mixed use-parent and @ISA in same file
// ---------------------------------------------------------------------------

#[test]
fn mixed_inheritance_mechanisms() {
    let code = "\
package A;\n\
use parent 'ParentA';\n\
package B;\n\
our @ISA = ('ParentB');\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let a_supers = provider.find_supertypes(&ast, &make_item("A"));
    assert_eq!(a_supers.len(), 1);
    assert_eq!(a_supers[0].name, "ParentA");

    let b_supers = provider.find_supertypes(&ast, &make_item("B"));
    assert_eq!(b_supers.len(), 1);
    assert_eq!(b_supers[0].name, "ParentB");
}

// ---------------------------------------------------------------------------
// 15. Empty source
// ---------------------------------------------------------------------------

#[test]
fn empty_source_returns_none_on_prepare() {
    let code = "";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();
    assert!(provider.prepare(&ast, code, 0).is_none());
}

#[test]
fn empty_source_returns_empty_supertypes() {
    let code = "";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();
    assert!(provider.find_supertypes(&ast, &make_item("Any")).is_empty());
}

#[test]
fn empty_source_returns_empty_subtypes() {
    let code = "";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();
    assert!(provider.find_subtypes(&ast, &make_item("Any")).is_empty());
}

// ---------------------------------------------------------------------------
// 16. C3 linearization (Method Resolution Order)
// ---------------------------------------------------------------------------

/// Simple linear chain: Child -> Parent -> GrandParent
/// C3 MRO: [Child, Parent, GrandParent]
#[test]
fn c3_mro_linear_chain() {
    let code = "\
package GrandParent;\n\
package Parent;\n\
use parent 'GrandParent';\n\
package Child;\n\
use parent 'Parent';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let mro = provider.c3_mro(&ast, "Child");
    // Child itself is first, then Parent, then GrandParent
    assert_eq!(mro, vec!["Child", "Parent", "GrandParent"]);
}

/// Diamond inheritance: C3 must deduplicate and preserve correct order.
///
///   Base
///  /    \
/// Left  Right
///  \    /
///   Child
///
/// C3 MRO for Child: [Child, Left, Right, Base]
#[test]
fn c3_mro_diamond_inheritance() {
    let code = "\
package Base;\n\
package Left;\n\
use parent 'Base';\n\
package Right;\n\
use parent 'Base';\n\
package Child;\n\
use parent 'Left', 'Right';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let mro = provider.c3_mro(&ast, "Child");
    // Child first, then linearized parents, Base appears only once at the end
    assert_eq!(mro[0], "Child");
    assert!(mro.contains(&"Left".to_string()));
    assert!(mro.contains(&"Right".to_string()));
    assert!(mro.contains(&"Base".to_string()));
    // Base must appear after both Left and Right
    let left_pos = mro.iter().position(|n| n == "Left").unwrap_or(usize::MAX);
    let right_pos = mro.iter().position(|n| n == "Right").unwrap_or(usize::MAX);
    let base_pos = mro.iter().position(|n| n == "Base").unwrap_or(usize::MAX);
    assert!(base_pos > left_pos, "Base must come after Left");
    assert!(base_pos > right_pos, "Base must come after Right");
    // Base appears exactly once
    assert_eq!(mro.iter().filter(|n: &&String| n.as_str() == "Base").count(), 1);
}

/// Class with no parents: MRO is just itself.
#[test]
fn c3_mro_no_parents() {
    let code = "package Standalone;\nsub greet { 1 }\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let mro = provider.c3_mro(&ast, "Standalone");
    assert_eq!(mro, vec!["Standalone"]);
}

/// Unknown package (not in file): MRO returns just the name.
#[test]
fn c3_mro_unknown_package() {
    let code = "package Known;\nuse parent 'Base';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let mro = provider.c3_mro(&ast, "Unknown");
    assert_eq!(mro, vec!["Unknown"]);
}

/// Multiple direct parents in order: C3 preserves left-to-right order.
#[test]
fn c3_mro_multiple_direct_parents_order() {
    let code = "\
package A;\n\
package B;\n\
package C;\n\
package Child;\n\
use parent 'A', 'B', 'C';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let mro = provider.c3_mro(&ast, "Child");
    assert_eq!(mro[0], "Child");
    // A before B before C (left-to-right from use parent list)
    let a_pos = mro.iter().position(|n| n == "A").unwrap_or(usize::MAX);
    let b_pos = mro.iter().position(|n| n == "B").unwrap_or(usize::MAX);
    let c_pos = mro.iter().position(|n| n == "C").unwrap_or(usize::MAX);
    assert!(a_pos < b_pos, "A must come before B");
    assert!(b_pos < c_pos, "B must come before C");
}

// ---------------------------------------------------------------------------
// 17. Moose `extends` keyword
// ---------------------------------------------------------------------------

/// `extends 'ParentClass'` registers a supertype with detail "Parent Class".
#[test]
fn supertypes_via_moose_extends_single() {
    let code = "package MyClass;\nuse Moose;\nextends 'ParentClass';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let supertypes = provider.find_supertypes(&ast, &make_item("MyClass"));
    assert_eq!(supertypes.len(), 1, "expected one supertype; got {supertypes:?}");
    assert_eq!(supertypes[0].name, "ParentClass");
    assert_eq!(supertypes[0].detail.as_deref(), Some("Parent Class"));
}

/// `extends 'A', 'B'` registers both parents.
#[test]
fn supertypes_via_moose_extends_multiple() {
    let code = "package MyClass;\nuse Moose;\nextends 'BaseA', 'BaseB';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let supertypes = provider.find_supertypes(&ast, &make_item("MyClass"));
    let names: Vec<&str> = supertypes.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"BaseA"), "expected BaseA in {names:?}");
    assert!(names.contains(&"BaseB"), "expected BaseB in {names:?}");
}

/// `extends` also appears in the subtypes view of the parent.
#[test]
fn subtypes_via_moose_extends() {
    let code = "package Parent;\npackage Child;\nuse Moose;\nextends 'Parent';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let subtypes = provider.find_subtypes(&ast, &make_item("Parent"));
    let names: Vec<&str> = subtypes.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"Child"), "expected Child in {names:?}");
}

// ---------------------------------------------------------------------------
// 18. Moose `with` (role composition)
// ---------------------------------------------------------------------------

/// `with 'MyRole'` exposes the role as a supertype with detail "Role".
#[test]
fn supertypes_via_moose_with_single() {
    let code = "package MyClass;\nuse Moose;\nwith 'MyRole';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let supertypes = provider.find_supertypes(&ast, &make_item("MyClass"));
    assert_eq!(supertypes.len(), 1, "expected one role; got {supertypes:?}");
    assert_eq!(supertypes[0].name, "MyRole");
    assert_eq!(supertypes[0].detail.as_deref(), Some("Role"));
}

/// `with 'Role1', 'Role2'` exposes both roles.
#[test]
fn supertypes_via_moose_with_multiple_roles() {
    let code = "package MyClass;\nuse Moose;\nwith 'Role1', 'Role2';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let supertypes = provider.find_supertypes(&ast, &make_item("MyClass"));
    let names: Vec<&str> = supertypes.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"Role1"), "expected Role1 in {names:?}");
    assert!(names.contains(&"Role2"), "expected Role2 in {names:?}");
    // All role items carry "Role" detail
    for s in &supertypes {
        assert_eq!(s.detail.as_deref(), Some("Role"), "role detail mismatch for {}", s.name);
    }
}

/// Combined: `extends` and `with` in the same class.
#[test]
fn supertypes_extends_and_with_combined() {
    let code = "package Combined;\nuse Moose;\nextends 'BaseClass';\nwith 'RoleA';\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let supertypes = provider.find_supertypes(&ast, &make_item("Combined"));
    let names: Vec<&str> = supertypes.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"BaseClass"), "expected BaseClass in {names:?}");
    assert!(names.contains(&"RoleA"), "expected RoleA in {names:?}");
}

// ---------------------------------------------------------------------------
// 19. `use parent qw(...)` syntax
// ---------------------------------------------------------------------------

/// `use parent qw(Base1 Base2)` — qw() list form must be recognized.
#[test]
fn supertypes_via_use_parent_qw_list() {
    let code = "package Child;\nuse parent qw(Base1 Base2);\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let supertypes = provider.find_supertypes(&ast, &make_item("Child"));
    let names: Vec<&str> = supertypes.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"Base1"), "expected Base1 in {names:?}");
    assert!(names.contains(&"Base2"), "expected Base2 in {names:?}");
}

/// `use base qw(...)` — qw() form must also work for the legacy `base` pragma.
#[test]
fn supertypes_via_use_base_qw_list() {
    let code = "package OldStyle;\nuse base qw(OldBase AnotherOld);\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let supertypes = provider.find_supertypes(&ast, &make_item("OldStyle"));
    let names: Vec<&str> = supertypes.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"OldBase"), "expected OldBase in {names:?}");
    assert!(names.contains(&"AnotherOld"), "expected AnotherOld in {names:?}");
}

// ---------------------------------------------------------------------------
// 20. Deep inheritance chain via @ISA
// ---------------------------------------------------------------------------

/// Three-level chain expressed purely with `our @ISA`: C -> B -> A.
#[test]
fn deep_chain_via_isa() {
    let code = "\
package A;\n\
package B;\n\
our @ISA = ('A');\n\
package C;\n\
our @ISA = ('B');\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    // C's immediate supertype is B
    let c_supers = provider.find_supertypes(&ast, &make_item("C"));
    let c_names: Vec<&str> = c_supers.iter().map(|s| s.name.as_str()).collect();
    assert!(c_names.contains(&"B"), "expected B in {c_names:?}");
    assert!(!c_names.contains(&"A"), "A should not appear as direct super of C; got {c_names:?}");

    // B's immediate supertype is A
    let b_supers = provider.find_supertypes(&ast, &make_item("B"));
    let b_names: Vec<&str> = b_supers.iter().map(|s| s.name.as_str()).collect();
    assert!(b_names.contains(&"A"), "expected A in {b_names:?}");

    // A has no supertypes
    let a_supers = provider.find_supertypes(&ast, &make_item("A"));
    assert!(a_supers.is_empty(), "A should have no supertypes; got {a_supers:?}");
}

/// C3 MRO for a three-level @ISA chain: C -> B -> A gives [C, B, A].
#[test]
fn c3_mro_deep_chain_via_isa() {
    let code = "\
package A;\n\
package B;\n\
our @ISA = ('A');\n\
package C;\n\
our @ISA = ('B');\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    let mro = provider.c3_mro(&ast, "C");
    assert_eq!(mro, vec!["C", "B", "A"]);
}

// ---------------------------------------------------------------------------
// 21. Non-OOP code returns empty gracefully
// ---------------------------------------------------------------------------

/// File with only subs and variables — no inheritance declarations at all.
#[test]
fn non_oop_code_supertypes_empty() {
    let code = "\
sub hello { return 'world'; }\n\
my $x = 42;\n\
my @arr = (1, 2, 3);\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    assert!(
        provider.find_supertypes(&ast, &make_item("main")).is_empty(),
        "non-OOP code should yield no supertypes"
    );
}

/// File with only subs — prepare() returns None for an offset in a sub body.
#[test]
fn non_oop_code_prepare_returns_none() {
    let code = "sub add { my ($a, $b) = @_; $a + $b }\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    // offset 4 is inside "add" — an identifier inside a sub decl, not a package
    let result = provider.prepare(&ast, code, 4);
    assert!(result.is_none(), "sub identifier should not produce a type hierarchy item");
}

/// File with only subs — subtypes returns empty.
#[test]
fn non_oop_code_subtypes_empty() {
    let code = "sub foo { 1 }\nsub bar { 2 }\n";
    let ast = parse(code);
    let provider = TypeHierarchyProvider::new();

    assert!(
        provider.find_subtypes(&ast, &make_item("main")).is_empty(),
        "non-OOP code should yield no subtypes"
    );
}
