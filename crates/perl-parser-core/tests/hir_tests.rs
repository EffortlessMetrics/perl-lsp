use perl_parser_core::hir::{HirFile, HirKind, RecoveryConfidence, lower_ast};
use perl_parser_core::{Node, NodeKind, Parser, SourceLocation};

fn lower_source(source: &str) -> HirFile {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    lower_ast(&output.ast)
}

fn render_hir(file: &HirFile) -> String {
    file.items.iter().map(render_item).collect::<Vec<_>>().join("\n")
}

fn render_item(item: &perl_parser_core::hir::HirItem) -> String {
    let package = item.package_context.as_deref().unwrap_or("<none>");
    let scope =
        item.scope_context.map(|id| id.index().to_string()).unwrap_or_else(|| "<none>".to_string());
    let name_anchor = if item.anchor.name_range.is_some() { "name" } else { "node" };
    let kind = match &item.kind {
        HirKind::PackageDecl(decl) => format!("PackageDecl {}", decl.name),
        HirKind::SubDecl(decl) => {
            let name = decl.name.as_deref().unwrap_or("<anonymous>");
            format!(
                "SubDecl {name} proto={} sig={} attrs={}",
                decl.has_prototype, decl.has_signature, decl.attribute_count
            )
        }
        HirKind::MethodDecl(decl) => format!(
            "MethodDecl {} sig={} attrs={}",
            decl.name, decl.has_signature, decl.attribute_count
        ),
        HirKind::UseDecl(decl) => format!(
            "UseDecl {} args={} filter={}",
            decl.module,
            decl.args.join(","),
            decl.has_filter_risk
        ),
        HirKind::RequireDecl(decl) => {
            let target = decl.target.as_deref().unwrap_or("<dynamic>");
            format!("RequireDecl {target} args={}", decl.arg_count)
        }
        HirKind::VariableDecl(decl) => {
            let variables = decl
                .variables
                .iter()
                .map(|variable| format!("{}{}", variable.sigil, variable.name))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "VariableDecl {} vars={} attrs={} init={} list={}",
                decl.declarator,
                variables,
                decl.attribute_count,
                decl.has_initializer,
                decl.is_list
            )
        }
        HirKind::CallExpr(expr) => {
            format!("CallExpr {} args={} form={:?}", expr.name, expr.arg_count, expr.form)
        }
        HirKind::MethodCallExpr(expr) => format!(
            "MethodCallExpr {} args={} object={}",
            expr.method, expr.arg_count, expr.object_kind
        ),
        HirKind::IndirectCallExpr(expr) => format!(
            "IndirectCallExpr {} args={} object={}",
            expr.method, expr.arg_count, expr.object_kind
        ),
        HirKind::BarewordExpr(expr) => format!("BarewordExpr {}", expr.name),
        HirKind::LiteralExpr(expr) => {
            let value = expr.value.as_deref().unwrap_or("<none>");
            let interpolated = expr
                .interpolated
                .map(|value| value.to_string())
                .unwrap_or_else(|| "<none>".to_string());
            let element_count = expr
                .element_count
                .map(|value| value.to_string())
                .unwrap_or_else(|| "<none>".to_string());
            let pair_count = expr
                .pair_count
                .map(|value| value.to_string())
                .unwrap_or_else(|| "<none>".to_string());
            format!(
                "LiteralExpr {:?} value={} interp={} elements={} pairs={}",
                expr.kind, value, interpolated, element_count, pair_count
            )
        }
        HirKind::BlockShell(shell) => format!("BlockShell statements={}", shell.statement_count),
        HirKind::DynamicBoundary(boundary) => {
            format!("DynamicBoundary {:?} reason={}", boundary.kind, boundary.reason)
        }
        _ => "UnknownFutureHirKind".to_string(),
    };
    format!(
        "{} {kind} pkg={package} recovery={:?} anchor={} via={name_anchor} scope={scope}",
        item.id.index(),
        item.recovery_confidence,
        item.anchor.node_kind
    )
}

#[test]
fn hir_lowers_first_slice_constructs_with_stable_metadata() -> Result<(), Box<dyn std::error::Error>>
{
    let file = lower_source(
        "package My::Module;\n\
         use List::Util qw(sum);\n\
         require Other::Module;\n\
         sub greet ($) :lvalue { 1; }\n\
         method run { 1; }\n",
    );

    assert_eq!(
        render_hir(&file),
        "0 PackageDecl My::Module pkg=My::Module recovery=Parsed anchor=Package via=name scope=<none>\n\
         1 UseDecl List::Util args=qw(sum) filter=false pkg=My::Module recovery=Parsed anchor=Use via=node scope=<none>\n\
         2 RequireDecl Other::Module args=1 pkg=My::Module recovery=Parsed anchor=FunctionCall via=node scope=<none>\n\
         3 BarewordExpr Other::Module pkg=My::Module recovery=Parsed anchor=Identifier via=name scope=<none>\n\
         4 SubDecl greet proto=true sig=false attrs=1 pkg=My::Module recovery=Parsed anchor=Subroutine via=name scope=<none>\n\
         5 BlockShell statements=1 pkg=My::Module recovery=Parsed anchor=Block via=node scope=<none>\n\
         6 LiteralExpr Number value=1 interp=<none> elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=Number via=node scope=<none>\n\
         7 MethodDecl run sig=false attrs=0 pkg=My::Module recovery=Parsed anchor=Method via=node scope=<none>\n\
         8 BlockShell statements=1 pkg=My::Module recovery=Parsed anchor=Block via=node scope=<none>\n\
         9 LiteralExpr Number value=1 interp=<none> elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=Number via=node scope=<none>"
    );

    for (index, item) in file.items.iter().enumerate() {
        assert_eq!(item.id.index(), index as u32);
        assert!(item.range.end >= item.range.start, "HIR item range should be ordered: {:?}", item);
        assert_eq!(item.range, item.anchor.range);
    }

    Ok(())
}

#[test]
fn hir_lowers_variable_declarations_without_scope_cutover() -> Result<(), Box<dyn std::error::Error>>
{
    let file = lower_source(
        "package My::Module;\n\
         my $scalar = 1;\n\
         our @EXPORT_OK;\n\
         state %cache;\n\
         local $temp = undef;\n\
         local *FH;\n\
         my ($first, undef, @rest) = @_;\n\
         open(my $fh, '<', $path);\n",
    );

    assert_eq!(
        render_hir(&file),
        "0 PackageDecl My::Module pkg=My::Module recovery=Parsed anchor=Package via=name scope=<none>\n\
         1 VariableDecl my vars=$scalar attrs=0 init=true list=false pkg=My::Module recovery=Parsed anchor=VariableDeclaration via=name scope=<none>\n\
         2 LiteralExpr Number value=1 interp=<none> elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=Number via=node scope=<none>\n\
         3 VariableDecl our vars=@EXPORT_OK attrs=0 init=false list=false pkg=My::Module recovery=Parsed anchor=VariableDeclaration via=name scope=<none>\n\
         4 VariableDecl state vars=%cache attrs=0 init=false list=false pkg=My::Module recovery=Parsed anchor=VariableDeclaration via=name scope=<none>\n\
         5 VariableDecl local vars=$temp attrs=0 init=true list=false pkg=My::Module recovery=Parsed anchor=VariableDeclaration via=name scope=<none>\n\
         6 LiteralExpr Undef value=<none> interp=<none> elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=Undef via=node scope=<none>\n\
         7 VariableDecl local vars=*FH attrs=0 init=false list=false pkg=My::Module recovery=Parsed anchor=VariableDeclaration via=name scope=<none>\n\
         8 VariableDecl my vars=$first,@rest attrs=0 init=true list=true pkg=My::Module recovery=Parsed anchor=VariableListDeclaration via=node scope=<none>\n\
         9 LiteralExpr Undef value=<none> interp=<none> elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=Undef via=node scope=<none>\n\
         10 CallExpr open args=1 form=NamedFunction pkg=My::Module recovery=Parsed anchor=FunctionCall via=node scope=<none>\n\
         11 LiteralExpr Array value=<none> interp=<none> elements=3 pairs=<none> pkg=My::Module recovery=Parsed anchor=ArrayLiteral via=node scope=<none>\n\
         12 VariableDecl my vars=$fh attrs=0 init=false list=false pkg=My::Module recovery=Parsed anchor=VariableDeclaration via=name scope=<none>\n\
         13 LiteralExpr String value='<' interp=false elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=String via=node scope=<none>"
    );

    assert!(file.items.iter().all(|item| item.scope_context.is_none()));

    Ok(())
}

#[test]
fn hir_lowers_expression_shells_without_provider_cutover() -> Result<(), Box<dyn std::error::Error>>
{
    let file = lower_source(
        "package My::Module;\n\
         helper($value, 42, \"x\");\n\
         $self->method($value);\n\
         new Widget 1;\n\
         $callback->(42);\n\
         grep { $_->ok } @items;\n\
         eval $source;\n\
         do $file;\n\
         my $settings = { foo => 1, bar => \"x\" };\n",
    );

    assert_eq!(
        render_hir(&file),
        "0 PackageDecl My::Module pkg=My::Module recovery=Parsed anchor=Package via=name scope=<none>\n\
         1 CallExpr helper args=3 form=NamedFunction pkg=My::Module recovery=Parsed anchor=FunctionCall via=node scope=<none>\n\
         2 LiteralExpr Number value=42 interp=<none> elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=Number via=node scope=<none>\n\
         3 LiteralExpr String value=\"x\" interp=true elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=String via=node scope=<none>\n\
         4 MethodCallExpr method args=1 object=Variable pkg=My::Module recovery=Parsed anchor=MethodCall via=node scope=<none>\n\
         5 IndirectCallExpr new args=1 object=Identifier pkg=My::Module recovery=Parsed anchor=IndirectCall via=node scope=<none>\n\
         6 BarewordExpr Widget pkg=My::Module recovery=Parsed anchor=Identifier via=name scope=<none>\n\
         7 LiteralExpr Number value=1 interp=<none> elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=Number via=node scope=<none>\n\
         8 DynamicBoundary CoderefCall reason=coderef or dynamic callee invoked through ->() pkg=My::Module recovery=Parsed anchor=FunctionCall via=node scope=<none>\n\
         9 CallExpr ->() args=1 form=Coderef pkg=My::Module recovery=Parsed anchor=FunctionCall via=node scope=<none>\n\
         10 LiteralExpr Number value=42 interp=<none> elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=Number via=node scope=<none>\n\
         11 CallExpr grep args=2 form=NamedFunction pkg=My::Module recovery=Parsed anchor=FunctionCall via=node scope=<none>\n\
         12 BlockShell statements=1 pkg=My::Module recovery=Parsed anchor=Block via=node scope=<none>\n\
         13 MethodCallExpr ok args=0 object=Variable pkg=My::Module recovery=Parsed anchor=MethodCall via=node scope=<none>\n\
         14 DynamicBoundary EvalExpression reason=eval body is an expression rather than a parsed block pkg=My::Module recovery=Parsed anchor=Eval via=node scope=<none>\n\
         15 DynamicBoundary DoExpression reason=do body is an expression rather than a parsed block pkg=My::Module recovery=Parsed anchor=Do via=node scope=<none>\n\
         16 VariableDecl my vars=$settings attrs=0 init=true list=false pkg=My::Module recovery=Parsed anchor=VariableDeclaration via=name scope=<none>\n\
         17 LiteralExpr Hash value=<none> interp=<none> elements=<none> pairs=2 pkg=My::Module recovery=Parsed anchor=HashLiteral via=node scope=<none>\n\
         18 LiteralExpr String value=foo interp=false elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=String via=node scope=<none>\n\
         19 LiteralExpr Number value=1 interp=<none> elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=Number via=node scope=<none>\n\
         20 LiteralExpr String value=bar interp=false elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=String via=node scope=<none>\n\
         21 LiteralExpr String value=\"x\" interp=true elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=String via=node scope=<none>"
    );

    assert!(file.items.iter().all(|item| item.scope_context.is_none()));

    Ok(())
}

#[test]
fn hir_marks_items_lowered_from_error_partials_as_recovered()
-> Result<(), Box<dyn std::error::Error>> {
    let loc = SourceLocation { start: 0, end: 12 };
    let partial = Node::new(
        NodeKind::Subroutine {
            name: Some("broken".to_string()),
            name_span: Some(SourceLocation { start: 4, end: 10 }),
            prototype: None,
            signature: None,
            attributes: Vec::new(),
            body: Box::new(Node::new(
                NodeKind::MissingBlock,
                SourceLocation { start: 11, end: 11 },
            )),
        },
        loc,
    );
    let ast = Node::new(
        NodeKind::Program {
            statements: vec![Node::new(
                NodeKind::Error {
                    message: "missing block".to_string(),
                    expected: Vec::new(),
                    found: None,
                    partial: Some(Box::new(partial)),
                },
                loc,
            )],
        },
        loc,
    );

    let file = lower_ast(&ast);
    let Some(item) = file.items.first() else {
        return Err("expected recovered HIR item".into());
    };

    assert_eq!(item.recovery_confidence, RecoveryConfidence::Recovered);
    assert_eq!(
        render_hir(&file),
        "0 SubDecl broken proto=false sig=false attrs=0 pkg=<none> recovery=Recovered anchor=Subroutine via=name scope=<none>"
    );

    Ok(())
}
