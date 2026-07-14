mod cpan_test_helpers;

use perl_ast::GotoTargetForm;
use perl_parser_core::{Node, NodeKind};

fn first_goto_form(source: &str) -> Result<(GotoTargetForm, &'static str), String> {
    fn find(node: &Node) -> Option<(GotoTargetForm, &'static str)> {
        if let NodeKind::Goto { target, form } = &node.kind {
            return Some((form.clone(), target.kind.kind_name()));
        }

        node.children().into_iter().find_map(find)
    }

    let ast = cpan_test_helpers::parse(source);
    find(&ast).ok_or_else(|| format!("expected a Goto node for source:\n{source}"))
}

#[test]
fn goto_ampersand_variable_is_sub_form() -> Result<(), String> {
    let (form, target_kind) = first_goto_form(r#"goto &$dispatch;"#)?;

    assert_eq!(form, GotoTargetForm::Sub);
    assert_ne!(target_kind, "MissingExpression");
    Ok(())
}

#[test]
fn goto_ampersand_coderef_deref_is_sub_form() -> Result<(), String> {
    let (form, target_kind) = first_goto_form(r#"goto &{ $code };"#)?;

    assert_eq!(form, GotoTargetForm::Sub);
    assert_ne!(target_kind, "MissingExpression");
    Ok(())
}

#[test]
fn goto_bareword_concat_is_expr_form() -> Result<(), String> {
    let (form, target_kind) = first_goto_form(r#"goto E . $suffix;"#)?;

    assert_eq!(form, GotoTargetForm::Expr);
    assert_ne!(target_kind, "Identifier");
    Ok(())
}

#[test]
fn goto_plain_label_is_label_form() -> Result<(), String> {
    let (form, target_kind) = first_goto_form(r#"goto LABEL;"#)?;

    assert_eq!(form, GotoTargetForm::Label);
    assert_eq!(target_kind, "Identifier");
    Ok(())
}

#[test]
fn goto_function_call_is_expr_form() -> Result<(), String> {
    let (form, target_kind) = first_goto_form(r#"goto foo();"#)?;

    assert_eq!(form, GotoTargetForm::Expr);
    assert_eq!(target_kind, "FunctionCall");
    Ok(())
}

#[test]
fn goto_ampersand_named_sub_is_sub_form() -> Result<(), String> {
    let (form, target_kind) = first_goto_form(r#"goto &foo;"#)?;

    assert_eq!(form, GotoTargetForm::Sub);
    assert_ne!(target_kind, "MissingExpression");
    Ok(())
}

#[test]
fn goto_ampersand_qualified_sub_is_sub_form() -> Result<(), String> {
    let (form, target_kind) = first_goto_form(r#"goto &Pkg::bar;"#)?;

    assert_eq!(form, GotoTargetForm::Sub);
    assert_ne!(target_kind, "MissingExpression");
    Ok(())
}
