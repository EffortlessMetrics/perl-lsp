//! BDD-style behavior specification tests for `tree-sitter-perl-rs`.
//!
//! These scenarios lock facade-level behavior from a user perspective:
//! parser ergonomics, traversal, source extraction, and resilience on malformed input.

use perl_tdd_support::{must, must_some};
use tree_sitter_perl_rs::Parser;

fn parse(source: &str) -> tree_sitter_perl_rs::Tree {
    let mut parser = Parser::new();
    must_some(parser.parse(source))
}

#[test]
fn when_parsing_valid_perl_then_tree_and_source_are_available() {
    let source = "my $x = 42;";

    let tree = parse(source);

    assert_eq!(tree.source(), source);
    assert_eq!(tree.root_node().start_byte(), 0);
}

#[test]
fn when_requesting_root_kind_then_program_is_returned() {
    let tree = parse("my $x = 42;");

    assert_eq!(tree.root_node().kind(), "Program");
}

#[test]
fn when_rendering_root_as_sexp_then_output_uses_source_file_shape() {
    let tree = parse("my $x = 42;");
    let sexp = tree.root_node().to_sexp();

    assert!(sexp.starts_with("(source_file"), "unexpected sexp: {sexp}");
}

#[test]
fn when_iterating_children_then_iterator_count_matches_indexed_access() {
    let tree = parse("my $x = 1; my $y = 2;");
    let root = tree.root_node();

    let children: Vec<_> = root.children().collect();
    assert_eq!(children.len(), root.child_count());

    if let Some(first_from_iter) = children.first() {
        let first_from_index = must_some(root.child(0));
        assert_eq!(first_from_index.kind(), first_from_iter.kind());
    }
}

#[test]
fn when_requesting_out_of_bounds_child_then_none_is_returned() {
    let tree = parse("my $x = 1;");

    assert!(tree.root_node().child(usize::MAX).is_none());
}

#[test]
fn when_extracting_utf8_text_then_root_round_trips_source_bytes() {
    let source = "my $x = 'café';";
    let tree = parse(source);

    let text = must(tree.root_node().utf8_text(source.as_bytes()));
    assert_eq!(text, source);
}

#[test]
fn when_utf8_text_uses_shorter_buffer_then_it_clamps_without_panicking() {
    let tree = parse("my $x = 42;");

    let result = tree.root_node().utf8_text(b"my");
    assert!(result.is_ok());
    assert_eq!(must(result), "my");
}

#[test]
fn when_parsing_malformed_input_then_error_tolerant_tree_is_still_produced() {
    let mut parser = Parser::new();

    let tree = parser.parse("sub { @@@@invalid{{{{");

    assert!(tree.is_some());
}

#[test]
fn when_reusing_one_parser_for_multiple_inputs_then_each_parse_still_returns_a_tree() {
    let mut parser = Parser::new();

    let first = parser.parse("package Demo;\nsub one { 1 }\n");
    let second = parser.parse("for my $item (@items) { print $item; }\n");

    assert!(first.is_some());
    assert!(second.is_some());
}

#[test]
fn when_requesting_grammar_kind_of_root_then_source_file_is_returned() {
    let tree = parse("my $x = 42;");
    assert_eq!(tree.root_node().grammar_kind(), "source_file");
}

#[test]
fn when_requesting_grammar_kind_of_subroutine_then_sub_is_returned() {
    let tree = parse("sub greet { 1 }");
    let root = tree.root_node();
    // Find the subroutine child
    let sub_node = must_some(root.children().find(|n| n.kind() == "Subroutine"));
    assert_eq!(sub_node.grammar_kind(), "sub");
}

#[test]
fn when_v3_kind_and_grammar_kind_are_both_available_then_they_differ_for_program() {
    let tree = parse("1;");
    let root = tree.root_node();
    assert_eq!(root.kind(), "Program");
    assert_eq!(root.grammar_kind(), "source_file");
    assert_ne!(root.kind(), root.grammar_kind());
}

#[test]
fn when_requesting_grammar_kind_of_variable_with_attributes_then_snake_case_fallback_is_used() {
    // NodeKind::VariableWithAttributes produces a double-paren sexp of the form
    // `((variable $ foo) (attributes :lvalue))` -- grammar_kind() must fall back
    // to snake_case of kind_name() and must NOT return the child kind "variable".
    let tree = parse("my ($foo :lvalue);");
    let root = tree.root_node();
    // Walk the tree to find the VariableWithAttributes node if present.
    fn find_var_attrs(n: tree_sitter_perl_rs::Node<'_>) -> Option<String> {
        if n.kind() == "VariableWithAttributes" {
            return Some(n.grammar_kind());
        }
        for child in n.children() {
            if let Some(gk) = find_var_attrs(child) {
                return Some(gk);
            }
        }
        None
    }
    if let Some(gk) = find_var_attrs(root) {
        assert_ne!(
            gk, "variable",
            "grammar_kind() must not return child kind for VariableWithAttributes; got {gk}"
        );
        assert_eq!(gk, "variable_with_attributes", "expected snake_case fallback; got {gk}");
    }
}

#[test]
fn when_querying_definition_overlay_at_offset_then_definition_is_returned() {
    let source = "my $value = 1;\n$value + 2;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();

    let offset = must_some(source.find("$value +"));
    let definition = must_some(overlay.definition_at_offset(offset));

    assert_eq!(definition.name, "value");
    assert_eq!(definition.start_byte, 3);
}

#[test]
fn when_querying_visible_imports_overlay_then_prior_use_statements_are_reported() {
    let source = "use strict;\nuse warnings;\nmy $x = 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();
    let offset = must_some(source.find("my $x"));

    let imports = overlay.visible_imports_at_offset(offset);
    let import_modules: Vec<_> = imports.iter().map(|import| import.module.as_str()).collect();

    assert_eq!(import_modules, vec!["strict", "warnings"]);
}

#[test]
fn when_querying_pragma_state_overlay_then_effective_state_matches_offset() {
    let source = "no strict;\nuse warnings;\nmy $x = 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();
    let offset = must_some(source.find("my $x"));

    let state = overlay.pragma_state_at_offset(offset);

    assert!(!state.strict_refs);
    assert!(state.warnings);
}

#[test]
fn when_source_starts_with_use_then_visible_imports_byte_range_is_exact_statement_span() {
    // Regression: a source that starts with `use` caused the Program root node's text
    // to be parsed as an import with statement_end_byte = source.len() rather than
    // the end of the actual `use` statement. This test locks the correct byte range.
    let source = "use strict;\nmy $x = 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();
    let offset = must_some(source.find("my $x"));

    let imports = overlay.visible_imports_at_offset(offset);

    assert_eq!(imports.len(), 1, "expected exactly one import");
    let strict_import = &imports[0];
    assert_eq!(strict_import.module, "strict");
    // statement_end_byte must be strictly less than source.len() — it points to the
    // end of the Use AST node (at or before the semicolon), not to end-of-file.
    assert!(
        strict_import.statement_end_byte < source.len(),
        "statement_end_byte ({}) must not equal source.len() ({}) — \
         this indicates the Program root node was mistakenly used as the statement span",
        strict_import.statement_end_byte,
        source.len()
    );
    assert_eq!(strict_import.statement_start_byte, 0);
}

#[test]
fn when_querying_visible_imports_then_no_module_is_excluded() {
    // `no` statements are NOT module imports — they disable pragmas.
    // visible_imports_at_offset must not include `no strict` as a visible import.
    let source = "no strict;\nuse warnings;\nmy $x = 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();
    let offset = must_some(source.find("my $x"));

    let imports = overlay.visible_imports_at_offset(offset);
    let modules: Vec<_> = imports.iter().map(|i| i.module.as_str()).collect();

    assert!(!modules.contains(&"strict"), "no strict must not appear as a visible import");
    assert!(modules.contains(&"warnings"), "use warnings must appear as a visible import");
}
