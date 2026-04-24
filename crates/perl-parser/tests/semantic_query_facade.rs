use perl_parser::SemanticQueryFacade;

#[test]
fn facade_resolves_symbol_definition_and_pragmas() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict;
use warnings;
use Foo::Bar;

package Base;
sub greet { return 1; }

package Child;
use parent 'Base';
sub call {
    my $value = 41;
    return $value;
}
"#;

    let facade = SemanticQueryFacade::build("file:///workspace/lib/Child.pm", source)?;

    let return_offset = source
        .find("return $value")
        .ok_or("expected return statement in test source")?
        + "return $".len();
    let resolved = facade
        .resolve_symbol_at(return_offset)
        .ok_or("expected resolved symbol at variable usage")?;

    assert_eq!(resolved.name, "value");
    assert_eq!(resolved.definition.uri, "file:///workspace/lib/Child.pm");

    let pragma = facade.effective_pragma_state(return_offset);
    assert!(pragma.strict_vars);
    assert!(pragma.strict_subs);
    assert!(pragma.strict_refs);
    assert!(pragma.warnings);

    Ok(())
}

#[test]
fn facade_reports_visible_imports_and_parent_origin() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict;
use Foo::Bar;

package Base;
sub greet { return 1; }

package Child;
use parent 'Base';
sub call { return shift->greet(); }
"#;

    let facade = SemanticQueryFacade::build("file:///workspace/lib/Child.pm", source)?;

    let imports = facade.visible_imports();
    assert!(imports.iter().any(|item| item.module == "Foo::Bar"));

    let chain = facade
        .parent_chain("Child", Some("greet"))
        .ok_or("expected parent chain for Child")?;

    assert_eq!(chain.parents.first().map(String::as_str), Some("Base"));
    assert_eq!(chain.inherited_origin.as_deref(), Some("Base"));

    Ok(())
}
