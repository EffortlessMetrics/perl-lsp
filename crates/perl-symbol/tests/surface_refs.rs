//! Tests for phase-1 `SymbolRef` extraction.

use perl_ast::{Node, NodeKind, SourceLocation};
use perl_symbol::VarKind;
use perl_symbol::surface::{SymbolRefKind, extract_symbol_refs};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

#[test]
fn variable_reference_is_extracted() -> Result<()> {
    let var = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "count".to_string() },
        loc(0, 6),
    );
    let expr_stmt =
        Node::new(NodeKind::ExpressionStatement { expression: Box::new(var) }, loc(0, 7));
    let program = Node::new(NodeKind::Program { statements: vec![expr_stmt] }, loc(0, 7));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].kind, SymbolRefKind::Variable(VarKind::Scalar));
    assert_eq!(refs[0].name, "count");
    assert_eq!(refs[0].qualified_name, "count");
    assert_eq!(refs[0].sigil.as_deref(), Some("$"));
    assert_eq!(refs[0].package_qualifier, None);
    Ok(())
}

#[test]
fn declaration_target_is_not_treated_as_reference() -> Result<()> {
    let decl_var =
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() }, loc(3, 5));
    let init_var =
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "y".to_string() }, loc(8, 10));
    let decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(decl_var),
            attributes: vec![],
            initializer: Some(Box::new(init_var)),
        },
        loc(0, 10),
    );
    let program = Node::new(NodeKind::Program { statements: vec![decl] }, loc(0, 10));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].name, "y");
    Ok(())
}

#[test]
fn subroutine_call_reference_is_extracted() -> Result<()> {
    let call = Node::new(
        NodeKind::FunctionCall {
            name: "greet".to_string(),
            args: vec![Node::new(NodeKind::Number { value: "1".to_string() }, loc(6, 7))],
        },
        loc(0, 8),
    );
    let program = Node::new(NodeKind::Program { statements: vec![call] }, loc(0, 8));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].kind, SymbolRefKind::SubroutineCall);
    assert_eq!(refs[0].name, "greet");
    assert_eq!(refs[0].qualified_name, "greet");
    assert_eq!(refs[0].package_qualifier, None);
    Ok(())
}

#[test]
fn package_qualified_references_are_projected() -> Result<()> {
    let call = Node::new(
        NodeKind::FunctionCall { name: "My::Pkg::run".to_string(), args: vec![] },
        loc(0, 12),
    );
    let var = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "My::Pkg::VALUE".to_string() },
        loc(13, 28),
    );
    let program = Node::new(NodeKind::Program { statements: vec![call, var] }, loc(0, 28));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 2);

    assert_eq!(refs[0].kind, SymbolRefKind::SubroutineCall);
    assert_eq!(refs[0].name, "run");
    assert_eq!(refs[0].qualified_name, "My::Pkg::run");
    assert_eq!(refs[0].package_qualifier.as_deref(), Some("My::Pkg"));

    assert_eq!(refs[1].kind, SymbolRefKind::Variable(VarKind::Scalar));
    assert_eq!(refs[1].name, "VALUE");
    assert_eq!(refs[1].qualified_name, "My::Pkg::VALUE");
    assert_eq!(refs[1].package_qualifier.as_deref(), Some("My::Pkg"));
    Ok(())
}

#[test]
fn array_last_index_sigil_is_treated_as_scalar_reference() -> Result<()> {
    // `$#array` is a valid Perl expression yielding the last index (a scalar).
    // The parser encodes it as Variable { sigil: "$#", name: "array" }.
    let var = Node::new(
        NodeKind::Variable { sigil: "$#".to_string(), name: "items".to_string() },
        loc(0, 7),
    );
    let program = Node::new(NodeKind::Program { statements: vec![var] }, loc(0, 7));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 1, "$#array should be emitted as a scalar reference");
    assert_eq!(refs[0].kind, SymbolRefKind::Variable(VarKind::Scalar));
    assert_eq!(refs[0].name, "items");
    assert_eq!(refs[0].sigil.as_deref(), Some("$#"));
    assert_eq!(refs[0].package_qualifier, None);
    Ok(())
}

#[test]
fn code_ref_and_typeglob_sigils_are_excluded_in_phase1() -> Result<()> {
    // `&` (code ref) and `*` (typeglob) are phase-1 exclusions documented in the
    // module; they must not produce SymbolRef entries.
    let code_ref = Node::new(
        NodeKind::Variable { sigil: "&".to_string(), name: "handler".to_string() },
        loc(0, 8),
    );
    let typeglob = Node::new(
        NodeKind::Variable { sigil: "*".to_string(), name: "slot".to_string() },
        loc(9, 14),
    );
    let program = Node::new(NodeKind::Program { statements: vec![code_ref, typeglob] }, loc(0, 14));

    let refs = extract_symbol_refs(&program);
    assert!(
        refs.is_empty(),
        "phase-1 should not emit refs for & or * sigil variables, got: {:?}",
        refs
    );
    Ok(())
}

#[test]
fn variable_with_attributes_wrapper_is_traversed() -> Result<()> {
    // VariableWithAttributes wraps a Variable node; the inner Variable must still
    // be discovered by the walker via for_each_child.
    let inner_var = Node::new(
        NodeKind::Variable { sigil: "@".to_string(), name: "data".to_string() },
        loc(3, 8),
    );
    let wrapped = Node::new(
        NodeKind::VariableWithAttributes {
            variable: Box::new(inner_var),
            attributes: vec!["shared".to_string()],
        },
        loc(0, 8),
    );
    let program = Node::new(NodeKind::Program { statements: vec![wrapped] }, loc(0, 8));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 1, "inner Variable inside VariableWithAttributes must be visited");
    assert_eq!(refs[0].kind, SymbolRefKind::Variable(VarKind::Array));
    assert_eq!(refs[0].name, "data");
    Ok(())
}

#[test]
fn declaration_without_initializer_emits_no_refs() -> Result<()> {
    // `my $x;` — declaration with no initializer should not emit any refs.
    let decl_var =
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() }, loc(3, 5));
    let decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(decl_var),
            attributes: vec![],
            initializer: None,
        },
        loc(0, 6),
    );
    let program = Node::new(NodeKind::Program { statements: vec![decl] }, loc(0, 6));

    let refs = extract_symbol_refs(&program);
    assert!(refs.is_empty(), "declaration with no initializer must produce no refs");
    Ok(())
}

#[test]
fn function_call_args_are_walked_for_refs() -> Result<()> {
    // Arguments to a function call are expression contexts — variables inside them
    // must be emitted as refs.  Both the call site and the arg-variable must appear.
    let arg_var =
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "n".to_string() }, loc(5, 7));
    let call = Node::new(
        NodeKind::FunctionCall { name: "print".to_string(), args: vec![arg_var] },
        loc(0, 8),
    );
    let program = Node::new(NodeKind::Program { statements: vec![call] }, loc(0, 8));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 2, "expected call ref + argument variable ref");
    assert_eq!(refs[0].kind, SymbolRefKind::SubroutineCall);
    assert_eq!(refs[0].name, "print");
    assert_eq!(refs[1].kind, SymbolRefKind::Variable(VarKind::Scalar));
    assert_eq!(refs[1].name, "n");
    Ok(())
}

#[test]
fn parser_sentinel_names_are_not_emitted_as_refs() -> Result<()> {
    // The parser uses synthetic FunctionCall names for constructs that are not real
    // subroutine calls: "->()"/\&{}\" for coderef invocations, "field" for OOP
    // Perl 5.38+ field declarations.  None of these should appear as SubroutineCall refs.
    let coderef_call =
        Node::new(NodeKind::FunctionCall { name: "->()".to_string(), args: vec![] }, loc(0, 6));
    let ampersand_deref =
        Node::new(NodeKind::FunctionCall { name: "&{}".to_string(), args: vec![] }, loc(7, 11));
    let field_var = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
        loc(19, 21),
    );
    let field_decl = Node::new(
        NodeKind::FunctionCall { name: "field".to_string(), args: vec![field_var] },
        loc(13, 21),
    );
    let program = Node::new(
        NodeKind::Program { statements: vec![coderef_call, ampersand_deref, field_decl] },
        loc(0, 21),
    );

    let refs = extract_symbol_refs(&program);
    // Only the $x variable inside the field args should appear (as a variable ref).
    // The three sentinel FunctionCall nodes must not produce SubroutineCall refs.
    let sub_refs: Vec<_> =
        refs.iter().filter(|r| r.kind == SymbolRefKind::SubroutineCall).collect();
    assert!(
        sub_refs.is_empty(),
        "sentinel FunctionCall names must not be emitted as SubroutineCall refs: {:?}",
        sub_refs,
    );
    Ok(())
}

#[test]
fn signature_parameters_are_not_emitted_as_refs() -> Result<()> {
    // `sub foo($x, $y = $default, @rest)` — $x, $y, @rest are declaration sites;
    // only $default (the default-value expression) must be emitted as a ref.
    let param_x = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
        loc(8, 10),
    );
    let mandatory = Node::new(
        NodeKind::MandatoryParameter { variable: Box::new(param_x) },
        loc(8, 10),
    );

    let param_y = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "y".to_string() },
        loc(12, 14),
    );
    let default_var = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "default".to_string() },
        loc(17, 25),
    );
    let optional = Node::new(
        NodeKind::OptionalParameter {
            variable: Box::new(param_y),
            default_value: Box::new(default_var),
        },
        loc(12, 25),
    );

    let param_rest = Node::new(
        NodeKind::Variable { sigil: "@".to_string(), name: "rest".to_string() },
        loc(27, 32),
    );
    let slurpy = Node::new(
        NodeKind::SlurpyParameter { variable: Box::new(param_rest) },
        loc(27, 32),
    );

    let sig = Node::new(
        NodeKind::Signature { parameters: vec![mandatory, optional, slurpy] },
        loc(7, 33),
    );
    let body = Node::new(NodeKind::Block { statements: vec![] }, loc(34, 36));
    let sub_node = Node::new(
        NodeKind::Subroutine {
            name: Some("foo".to_string()),
            name_span: None,
            prototype: None,
            signature: Some(Box::new(sig)),
            attributes: vec![],
            body: Box::new(body),
        },
        loc(0, 36),
    );
    let program = Node::new(NodeKind::Program { statements: vec![sub_node] }, loc(0, 36));

    let refs = extract_symbol_refs(&program);

    // Only $default (the optional-parameter default value) should appear.
    // $x, $y, @rest are declaration sites and must not be emitted.
    assert_eq!(
        refs.len(),
        1,
        "only $default should be a ref; got: {:?}",
        refs.iter().map(|r| &r.name).collect::<Vec<_>>()
    );
    assert_eq!(refs[0].kind, SymbolRefKind::Variable(VarKind::Scalar));
    assert_eq!(refs[0].name, "default");
    Ok(())
}
