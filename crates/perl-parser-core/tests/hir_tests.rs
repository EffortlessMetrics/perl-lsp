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
         3 SubDecl greet proto=true sig=false attrs=1 pkg=My::Module recovery=Parsed anchor=Subroutine via=name scope=<none>\n\
         4 MethodDecl run sig=false attrs=0 pkg=My::Module recovery=Parsed anchor=Method via=node scope=<none>"
    );

    for (index, item) in file.items.iter().enumerate() {
        assert_eq!(item.id.index(), index as u32);
        assert!(item.range.end >= item.range.start, "HIR item range should be ordered: {:?}", item);
        assert_eq!(item.range, item.anchor.range);
    }

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
