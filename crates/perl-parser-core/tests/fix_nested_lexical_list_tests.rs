mod cpan_test_helpers;

use cpan_test_helpers::*;
use perl_parser_core::hir::{HirKind, lower_ast};

#[test]
fn perltidy_nested_optional_arg_list_clean_parse() {
    let source = r#"
sub write_blank_code_line {
    my ( $self, ($forced) ) = @_;
}
"#;

    assert_clean_parse(source);
}

#[test]
fn nested_list_entries_lower_as_decl_bindings() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parse(
        r#"
sub write_blank_code_line {
    my ( $self, ($forced) ) = @_;
}
"#,
    );
    let hir = lower_ast(&ast);

    let Some(variables) = hir.items.iter().find_map(|item| match &item.kind {
        HirKind::VariableDecl(decl) if decl.is_list => Some(&decl.variables),
        _ => None,
    }) else {
        return Err("expected list variable declaration in HIR".into());
    };

    let names = variables
        .iter()
        .map(|binding| format!("{}{}", binding.sigil, binding.name))
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["$self", "$forced"]);

    let nested_refs =
        hir.scope_graph.references.iter().filter(|reference| reference.name == "forced").count();
    assert_eq!(nested_refs, 0, "nested declaration binding should not be recorded as a reference");

    Ok(())
}
