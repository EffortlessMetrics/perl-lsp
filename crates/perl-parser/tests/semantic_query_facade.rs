use perl_parser::semantic_query::SemanticQueryFacade;
use perl_parser::workspace::workspace_index::WorkspaceIndex;
use std::error::Error;
use url::Url;

#[test]
fn resolves_symbol_and_imports_and_pragmas() -> Result<(), Box<dyn Error>> {
    let source = r#"
use strict;
use warnings;
use parent 'Base::Pkg';
my $value = 1;
$value += 2;
"#;

    let facade = SemanticQueryFacade::from_source(source)?;

    let value_offset = source.find("$value +=").ok_or("missing usage")?;
    let resolved = facade.resolve_symbol_at(value_offset).ok_or("expected resolved symbol")?;
    assert_eq!(resolved.name, "$value");

    let imports = facade.visible_imports();
    assert!(imports.iter().any(|item| item.module == "strict"));
    assert!(imports.iter().any(|item| item.module == "warnings"));
    assert!(imports.iter().any(|item| item.module == "parent"));

    let pragma = facade.effective_pragma_state(value_offset);
    assert!(pragma.strict_vars);
    assert!(pragma.warnings);

    Ok(())
}

#[test]
fn supports_parent_chain_and_workspace_definition_lookup() -> Result<(), Box<dyn Error>> {
    let source = r#"
package Child;
use parent 'Base';
sub local_sub { return 1; }
"#;

    let uri = Url::parse("file:///tmp/semantic_query_facade.pl")?;
    let index = WorkspaceIndex::new();
    index.index_file(uri, source.to_string()).map_err(|error| format!("index failed: {error}"))?;

    let facade = SemanticQueryFacade::from_source(source)?.with_workspace_index(index);

    let chain = facade.parent_chain("Child").ok_or("missing parent chain")?;
    assert!(chain.inherited_from.iter().any(|item| item.package == "Base"));

    let location = facade.definition_location("Child::local_sub").ok_or("missing definition")?;
    assert!(location.uri.is_some());

    Ok(())
}
