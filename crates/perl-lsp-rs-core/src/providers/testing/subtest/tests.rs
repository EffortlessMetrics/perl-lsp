//! Tests for Test2 subtest discovery.

use super::*;

fn discover(source: &str) -> Vec<DiscoveredSubtest> {
    let mut parser = perl_parser::Parser::new(source);
    let ast = parser.parse().expect("source parses");
    discover_subtests(&ast, source)
}

#[test]
fn discovers_top_level_named_subtests() {
    let source = "use Test2::V0;\n\
        subtest 'user lookup' => sub {\n\
            ok(1, 'found');\n\
        };\n\
        subtest 'email' => sub {\n\
            is(1, 1, 'matches');\n\
        };\n\
        done_testing;\n";
    let subtests = discover(source);
    assert_eq!(subtests.len(), 2);
    assert_eq!(subtests[0].name, SubtestName::Named("user lookup".to_string()));
    assert_eq!(subtests[1].name, SubtestName::Named("email".to_string()));
    assert!(subtests[0].children.is_empty());
}

#[test]
fn discovers_nested_subtests_as_a_tree() {
    let source = "subtest 'outer' => sub {\n\
            ok(1);\n\
            subtest 'inner' => sub {\n\
                subtest 'deepest' => sub { ok(1); };\n\
            };\n\
        };\n";
    let subtests = discover(source);
    assert_eq!(subtests.len(), 1, "only the outer subtest is top-level");
    let outer = &subtests[0];
    assert_eq!(outer.name, SubtestName::Named("outer".to_string()));
    assert_eq!(outer.children.len(), 1);
    let inner = &outer.children[0];
    assert_eq!(inner.name, SubtestName::Named("inner".to_string()));
    assert_eq!(inner.children.len(), 1);
    assert_eq!(inner.children[0].name, SubtestName::Named("deepest".to_string()));
}

#[test]
fn double_quoted_names_are_unquoted() {
    let source = "subtest \"double quoted\" => sub { ok(1); };\n";
    let subtests = discover(source);
    assert_eq!(subtests.len(), 1);
    assert_eq!(subtests[0].name, SubtestName::Named("double quoted".to_string()));
}

#[test]
fn dynamic_name_is_not_guessed() {
    let source = "my $name = 'x';\nsubtest $name => sub { ok(1); };\n";
    let subtests = discover(source);
    assert_eq!(subtests.len(), 1);
    assert_eq!(subtests[0].name, SubtestName::Dynamic);
    assert_eq!(subtests[0].name.label(), "subtest (dynamic)");
    assert!(subtests[0].name.as_static().is_none());
}

#[test]
fn interpolated_name_with_variable_is_dynamic() {
    let source = "my $i = 1;\nsubtest \"case $i\" => sub { ok(1); };\n";
    let subtests = discover(source);
    assert_eq!(subtests.len(), 1);
    assert_eq!(subtests[0].name, SubtestName::Dynamic);
}

#[test]
fn no_subtests_yields_empty() {
    let source = "use Test2::V0;\nok(1);\nis(2, 2);\ndone_testing;\n";
    assert!(discover(source).is_empty());
}

#[test]
fn subtest_inside_conditional_is_found() {
    let source = "if ($ENV{EXTRA}) {\n\
            subtest 'conditional' => sub { ok(1); };\n\
        }\n";
    let subtests = discover(source);
    assert_eq!(subtests.len(), 1);
    assert_eq!(subtests[0].name, SubtestName::Named("conditional".to_string()));
}

#[test]
fn document_symbols_mirror_the_tree() {
    let source = "subtest 'outer' => sub {\n\
            subtest 'inner' => sub { ok(1); };\n\
        };\n";
    let subtests = discover(source);
    let symbols = subtest_document_symbols(&subtests);
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "outer");
    assert_eq!(symbols[0].detail, "subtest");
    assert_eq!(symbols[0].children.len(), 1);
    assert_eq!(symbols[0].children[0].name, "inner");
    // The selection range (name arg) is inside the full range.
    assert!(
        symbols[0].selection_range.start.to_byte_offset(source)
            >= symbols[0].range.start.to_byte_offset(source)
    );
}

#[test]
fn nearest_subtest_resolves_innermost_at_cursor() {
    // Lines (0-based):
    // 0: subtest 'outer' => sub {
    // 1:     ok(1);
    // 2:     subtest 'inner' => sub {
    // 3:         ok(2);
    // 4:     };
    // 5: };
    let source = "subtest 'outer' => sub {\n    ok(1);\n    subtest 'inner' => sub {\n        ok(2);\n    };\n};\n";
    let subtests = discover(source);

    // Cursor on line 3 (inside inner) resolves to the inner subtest.
    let inner = nearest_subtest_at_line(&subtests, 3).expect("cursor is inside a subtest");
    assert_eq!(inner.name, SubtestName::Named("inner".to_string()));

    // Cursor on line 1 (inside outer, before inner) resolves to outer.
    let outer = nearest_subtest_at_line(&subtests, 1).expect("cursor is inside a subtest");
    assert_eq!(outer.name, SubtestName::Named("outer".to_string()));
}

#[test]
fn nearest_subtest_returns_none_outside_any_subtest() {
    let source = "use Test2::V0;\nok(1);\nsubtest 'x' => sub { ok(1); };\ndone_testing;\n";
    let subtests = discover(source);
    // Line 1 is the bare `ok(1);` — not inside any subtest.
    assert!(nearest_subtest_at_line(&subtests, 1).is_none());
}

#[test]
fn buffered_and_streamed_variants_are_discovered() {
    let source = "subtest_buffered 'buf' => sub { ok(1); };\n\
        subtest_streamed 'str' => sub { ok(1); };\n";
    let subtests = discover(source);
    assert_eq!(subtests.len(), 2);
    assert_eq!(subtests[0].name, SubtestName::Named("buf".to_string()));
    assert_eq!(subtests[1].name, SubtestName::Named("str".to_string()));
}
