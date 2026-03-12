use perl_parser::{Node, NodeKind, Parser};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn has_arrow_coderef_call(node: &Node) -> bool {
    if let NodeKind::FunctionCall { name, .. } = &node.kind {
        if name == "->()" {
            return true;
        }
    }

    node.children().iter().any(|child| has_arrow_coderef_call(child))
}

#[test]
fn parses_arrow_coderef_call_with_empty_args() -> TestResult {
    let mut parser = Parser::new("my $result = $code->();");
    let ast = parser.parse()?;

    assert!(
        has_arrow_coderef_call(&ast),
        "Expected a FunctionCall node named ->() for coderef invocation",
    );

    Ok(())
}

#[test]
fn parses_arrow_coderef_call_with_args() -> TestResult {
    let mut parser = Parser::new("my $result = $code->($x, 42);");
    let ast = parser.parse()?;

    assert!(
        has_arrow_coderef_call(&ast),
        "Expected a FunctionCall node named ->() for coderef invocation",
    );

    Ok(())
}
