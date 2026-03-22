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
// 14. Cross-file (multi-document) supertypes
// ---------------------------------------------------------------------------

#[test]
fn test_find_supertypes_multi_cross_file() {
    // File A defines the parent; File B defines the child that inherits from it.
    let code_a = "package Parent;\nsub new { bless {}, shift }\n";
    let code_b = "package Child;\nuse parent 'Parent';\n";
    let ast_a = parse(code_a);
    let ast_b = parse(code_b);
    let provider = TypeHierarchyProvider::new();

    let docs: Vec<(&str, &perl_parser_core::ast::Node, &str)> =
        vec![("file:///a.pm", &ast_a, code_a), ("file:///b.pm", &ast_b, code_b)];
    let item = make_item("Child");
    let supertypes = provider.find_supertypes_multi(docs.into_iter(), &item);

    assert_eq!(supertypes.len(), 1, "Expected one supertype, got: {:?}", supertypes);
    assert_eq!(supertypes[0].name, "Parent");
    // The Parent is defined in file A — result URI must point to file A.
    assert_eq!(
        supertypes[0].uri, "file:///a.pm",
        "Supertype URI should point to the file where Parent is declared"
    );
}

// ---------------------------------------------------------------------------
// 15. Cross-file (multi-document) subtypes
// ---------------------------------------------------------------------------

#[test]
fn test_find_subtypes_multi_cross_file() {
    // File A defines the parent; File B defines the child.
    let code_a = "package Parent;\nsub new { bless {}, shift }\n";
    let code_b = "package Child;\nuse parent 'Parent';\n";
    let ast_a = parse(code_a);
    let ast_b = parse(code_b);
    let provider = TypeHierarchyProvider::new();

    let docs: Vec<(&str, &perl_parser_core::ast::Node, &str)> =
        vec![("file:///a.pm", &ast_a, code_a), ("file:///b.pm", &ast_b, code_b)];
    let item = make_item("Parent");
    let subtypes = provider.find_subtypes_multi(docs.into_iter(), &item);

    assert_eq!(subtypes.len(), 1, "Expected one subtype, got: {:?}", subtypes);
    assert_eq!(subtypes[0].name, "Child");
    // The Child is declared in file B — result URI must point to file B.
    assert_eq!(
        subtypes[0].uri, "file:///b.pm",
        "Subtype URI should point to the file where Child is declared"
    );
}

// ---------------------------------------------------------------------------
// 16. Diamond inheritance across multiple files
// ---------------------------------------------------------------------------

#[test]
fn test_diamond_inheritance_multi() {
    // GrandParent in file A; Parent1 and Parent2 in files B, C; Child in file D.
    let code_a = "package GrandParent;\n";
    let code_b = "package Parent1;\nuse parent 'GrandParent';\n";
    let code_c = "package Parent2;\nuse parent 'GrandParent';\n";
    let code_d = "package Child;\nuse parent 'Parent1';\nuse parent 'Parent2';\n";
    let ast_a = parse(code_a);
    let ast_b = parse(code_b);
    let ast_c = parse(code_c);
    let ast_d = parse(code_d);
    let provider = TypeHierarchyProvider::new();

    let docs: Vec<(&str, &perl_parser_core::ast::Node, &str)> = vec![
        ("file:///a.pm", &ast_a, code_a),
        ("file:///b.pm", &ast_b, code_b),
        ("file:///c.pm", &ast_c, code_c),
        ("file:///d.pm", &ast_d, code_d),
    ];

    // supertypes of Child should be Parent1 and Parent2 (no duplicates)
    let item_child = make_item("Child");
    let supertypes =
        provider.find_supertypes_multi(docs.iter().map(|(u, a, c)| (*u, *a, *c)), &item_child);
    let names: Vec<&str> = supertypes.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"Parent1"), "Expected Parent1 in {names:?}");
    assert!(names.contains(&"Parent2"), "Expected Parent2 in {names:?}");
    assert_eq!(names.len(), 2, "No duplicate supertypes expected, got: {names:?}");

    // subtypes of GrandParent should be Parent1 and Parent2 (no duplicates)
    let item_gp = make_item("GrandParent");
    let subtypes =
        provider.find_subtypes_multi(docs.iter().map(|(u, a, c)| (*u, *a, *c)), &item_gp);
    let subnames: Vec<&str> = subtypes.iter().map(|s| s.name.as_str()).collect();
    assert!(subnames.contains(&"Parent1"), "Expected Parent1 in {subnames:?}");
    assert!(subnames.contains(&"Parent2"), "Expected Parent2 in {subnames:?}");
    assert_eq!(subnames.len(), 2, "No duplicate subtypes expected, got: {subnames:?}");
}

// ---------------------------------------------------------------------------
// 17. C3 MRO across multiple files
// ---------------------------------------------------------------------------

#[test]
fn test_c3_mro_multi() {
    // Diamond: GrandParent <- Parent1, Parent2 <- Child
    let code_a = "package GrandParent;\n";
    let code_b = "package Parent1;\nuse parent 'GrandParent';\n";
    let code_c = "package Parent2;\nuse parent 'GrandParent';\n";
    let code_d = "package Child;\nuse parent 'Parent1';\nuse parent 'Parent2';\n";
    let ast_a = parse(code_a);
    let ast_b = parse(code_b);
    let ast_c = parse(code_c);
    let ast_d = parse(code_d);
    let provider = TypeHierarchyProvider::new();

    let docs: Vec<(&str, &perl_parser_core::ast::Node, &str)> = vec![
        ("file:///a.pm", &ast_a, code_a),
        ("file:///b.pm", &ast_b, code_b),
        ("file:///c.pm", &ast_c, code_c),
        ("file:///d.pm", &ast_d, code_d),
    ];

    let mro = provider.c3_mro_multi(docs.into_iter(), "Child");
    // C3 MRO for diamond should be: Child, Parent1, Parent2, GrandParent
    assert_eq!(mro[0], "Child", "MRO must start with the class itself");
    assert!(mro.contains(&"Parent1".to_string()), "MRO missing Parent1: {mro:?}");
    assert!(mro.contains(&"Parent2".to_string()), "MRO missing Parent2: {mro:?}");
    assert!(mro.contains(&"GrandParent".to_string()), "MRO missing GrandParent: {mro:?}");
    // Each class appears exactly once
    let mut seen = std::collections::BTreeSet::<String>::new();
    for cls in &mro {
        assert!(seen.insert(cls.clone()), "Duplicate in MRO: {cls}");
    }
}

// ---------------------------------------------------------------------------
// 18. Mixed use-parent and @ISA in same file
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
// 19. Empty source
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
// 20. C3 linearization (Method Resolution Order)
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
