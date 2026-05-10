mod cpan_test_helpers;

use cpan_test_helpers::*;
use perl_parser_core::NodeKind;

#[test]
fn object_pad_adjust_block_parses_cleanly() -> Result<(), String> {
    let source = r#"
use Object::Pad;

class Config {
    ADJUST {
        my $tmp = 1;
    }
}
"#;

    assert_clean_parse(source);

    let ast = parse(source);
    let NodeKind::Program { statements } = &ast.kind else {
        return Err(format!("expected program node, got {}", ast.kind.kind_name()));
    };

    let Some(class_stmt) =
        statements.iter().find(|statement| matches!(statement.kind, NodeKind::Class { .. }))
    else {
        return Err("expected Object::Pad class statement".to_string());
    };

    let NodeKind::Class { body, .. } = &class_stmt.kind else {
        return Err(format!("expected class node, got {}", class_stmt.kind.kind_name()));
    };

    let NodeKind::Block { statements } = &body.kind else {
        return Err(format!("expected class body block, got {}", body.kind.kind_name()));
    };

    let Some(adjust_stmt) = statements.first() else {
        return Err("expected ADJUST block statement".to_string());
    };
    let NodeKind::Method { name, signature, attributes, .. } = &adjust_stmt.kind else {
        return Err(format!(
            "expected ADJUST block to parse as a method-like node, got {}",
            adjust_stmt.kind.kind_name()
        ));
    };

    assert_eq!(name, "ADJUST");
    assert!(signature.is_none(), "ADJUST blocks should not carry method signatures");
    assert!(attributes.is_empty(), "ADJUST blocks should not synthesize attributes");
    Ok(())
}
