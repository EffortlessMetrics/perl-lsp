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
fn coderef_and_typeglob_sigils_are_classified() -> Result<()> {
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
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].kind, SymbolRefKind::CoderefReference);
    assert_eq!(refs[0].name, "handler");
    assert_eq!(refs[0].sigil.as_deref(), Some("&"));
    assert_eq!(refs[1].kind, SymbolRefKind::TypeglobReference);
    assert_eq!(refs[1].name, "slot");
    assert_eq!(refs[1].sigil.as_deref(), Some("*"));
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
fn static_method_call_reference_is_projected() -> Result<()> {
    let object = Node::new(NodeKind::Identifier { name: "My::Class".to_string() }, loc(0, 9));
    let call = Node::new(
        NodeKind::MethodCall {
            object: Box::new(object),
            method: "build".to_string(),
            args: vec![],
        },
        loc(0, 16),
    );
    let program = Node::new(NodeKind::Program { statements: vec![call] }, loc(0, 16));

    let refs = extract_symbol_refs(&program);

    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].kind, SymbolRefKind::StaticMethodCall);
    assert_eq!(refs[0].name, "build");
    assert_eq!(refs[0].qualified_name, "My::Class::build");
    assert_eq!(refs[0].package_qualifier.as_deref(), Some("My::Class"));
    assert_eq!(refs[0].sigil, None);
    Ok(())
}

#[test]
fn instance_method_call_reference_is_projected_with_receiver_refs() -> Result<()> {
    let object = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "self".to_string() },
        loc(0, 5),
    );
    let arg = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "value".to_string() },
        loc(13, 19),
    );
    let call = Node::new(
        NodeKind::MethodCall {
            object: Box::new(object),
            method: "save".to_string(),
            args: vec![arg],
        },
        loc(0, 20),
    );
    let program = Node::new(NodeKind::Program { statements: vec![call] }, loc(0, 20));

    let refs = extract_symbol_refs(&program);

    assert_eq!(refs.len(), 3);
    assert_eq!(refs[0].kind, SymbolRefKind::MethodCall);
    assert_eq!(refs[0].name, "save");
    assert_eq!(refs[0].qualified_name, "save");
    assert_eq!(refs[0].anchor_span, None);
    assert_eq!(refs[1].kind, SymbolRefKind::Variable(VarKind::Scalar));
    assert_eq!(refs[1].name, "self");
    assert_eq!(refs[2].kind, SymbolRefKind::Variable(VarKind::Scalar));
    assert_eq!(refs[2].name, "value");
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
fn sub_definition_and_sub_call_are_not_typed_as_distinct_edges() -> Result<()> {
    // Baseline for future typed reference edges: declaration sites are intentionally
    // not emitted by `extract_symbol_refs`, while call sites are emitted as
    // `SubroutineCall`.
    let decl = Node::new(
        NodeKind::Subroutine {
            name: Some("foo".to_string()),
            name_span: None,
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(7, 12))),
        },
        loc(0, 12),
    );
    let call =
        Node::new(NodeKind::FunctionCall { name: "foo".to_string(), args: vec![] }, loc(13, 18));
    let program = Node::new(NodeKind::Program { statements: vec![decl, call] }, loc(0, 18));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 1, "only call edges are represented today");
    assert_eq!(refs[0].kind, SymbolRefKind::SubroutineCall);
    assert_eq!(refs[0].name, "foo");
    Ok(())
}

#[test]
fn variable_reads_and_writes_collapse_to_variable_refs() -> Result<()> {
    // Baseline for typed edges: current API cannot distinguish read vs write.
    let lhs = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "value".to_string() },
        loc(0, 6),
    );
    let rhs = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "value".to_string() },
        loc(9, 15),
    );
    let assign = Node::new(
        NodeKind::Assignment { lhs: Box::new(lhs), rhs: Box::new(rhs), op: "=".to_string() },
        loc(0, 15),
    );
    let program = Node::new(NodeKind::Program { statements: vec![assign] }, loc(0, 15));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 2);
    assert!(refs.iter().all(|r| r.kind == SymbolRefKind::Variable(VarKind::Scalar)));
    assert!(refs.iter().all(|r| r.name == "value"));
    Ok(())
}

#[test]
fn coderef_syntax_forms_are_classified_conservatively() -> Result<()> {
    // Covers direct `&foo` plus parser-shaped `\\&foo` and `goto &foo` targets.
    // The parser consumes the ampersand in the latter forms and leaves a zero-arg
    // FunctionCall target, which must still classify as a coderef reference.
    let amp = Node::new(
        NodeKind::Variable { sigil: "&".to_string(), name: "foo".to_string() },
        loc(0, 4),
    );
    let backslash_amp_target =
        Node::new(NodeKind::FunctionCall { name: "foo".to_string(), args: vec![] }, loc(7, 10));
    let backslash_amp = Node::new(
        NodeKind::Unary { op: "\\".to_string(), operand: Box::new(backslash_amp_target) },
        loc(5, 10),
    );
    let goto_amp =
        Node::new(NodeKind::FunctionCall { name: "foo".to_string(), args: vec![] }, loc(17, 20));
    let goto = Node::new(NodeKind::Goto { target: Box::new(goto_amp) }, loc(11, 20));
    let program =
        Node::new(NodeKind::Program { statements: vec![amp, backslash_amp, goto] }, loc(0, 20));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 3);
    assert!(refs.iter().all(|r| r.kind == SymbolRefKind::CoderefReference));
    assert!(refs.iter().all(|r| r.name == "foo"));
    Ok(())
}

#[test]
fn typeglob_alias_boundary_is_classified() -> Result<()> {
    let typeglob = Node::new(NodeKind::Typeglob { name: "foo".to_string() }, loc(0, 4));
    let program = Node::new(NodeKind::Program { statements: vec![typeglob] }, loc(0, 4));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].kind, SymbolRefKind::TypeglobReference);
    assert_eq!(refs[0].name, "foo");
    assert_eq!(refs[0].sigil.as_deref(), Some("*"));
    Ok(())
}

#[test]
fn typeglob_assignment_keeps_rhs_coderef_reference() -> Result<()> {
    let lhs = Node::new(NodeKind::Typeglob { name: "alias".to_string() }, loc(0, 6));
    let rhs_target =
        Node::new(NodeKind::FunctionCall { name: "target".to_string(), args: vec![] }, loc(11, 17));
    let rhs = Node::new(
        NodeKind::Unary { op: "\\".to_string(), operand: Box::new(rhs_target) },
        loc(9, 17),
    );
    let assign = Node::new(
        NodeKind::Assignment { lhs: Box::new(lhs), rhs: Box::new(rhs), op: "=".to_string() },
        loc(0, 17),
    );
    let program = Node::new(NodeKind::Program { statements: vec![assign] }, loc(0, 17));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].kind, SymbolRefKind::TypeglobReference);
    assert_eq!(refs[0].name, "alias");
    assert_eq!(refs[1].kind, SymbolRefKind::CoderefReference);
    assert_eq!(refs[1].name, "target");
    Ok(())
}

#[test]
fn signature_parameters_are_not_emitted_as_refs() -> Result<()> {
    // `sub foo($x, $y = $default, @rest)` — $x, $y, @rest are declaration sites;
    // only $default (the default-value expression) must be emitted as a ref.
    let param_x =
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() }, loc(8, 10));
    let mandatory =
        Node::new(NodeKind::MandatoryParameter { variable: Box::new(param_x) }, loc(8, 10));

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
    let slurpy =
        Node::new(NodeKind::SlurpyParameter { variable: Box::new(param_rest) }, loc(27, 32));

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
