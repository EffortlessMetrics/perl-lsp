use perl_semantic_analyzer::analysis::type_facts::{DynamicBoundary, ShapeFact, TypeEvidence};
use perl_semantic_analyzer::analysis::type_inference::{PerlType, TypeInferenceEngine};
use perl_semantic_analyzer::{Node, NodeKind, Parser};
use perl_semantic_facts::Confidence;

fn parse_ast(code: &str) -> Result<Node, String> {
    let mut parser = Parser::new(code);
    parser.parse().map_err(|err| format!("parse failed: {err:?}"))
}

fn method_call_named<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    if let NodeKind::MethodCall { method, .. } = &node.kind {
        if method == name {
            return Some(node);
        }
    }

    match &node.kind {
        NodeKind::Program { statements } => {
            statements.iter().find_map(|child| method_call_named(child, name))
        }
        NodeKind::ExpressionStatement { expression } => method_call_named(expression, name),
        NodeKind::VariableDeclaration { initializer, .. } => {
            initializer.as_deref().and_then(|child| method_call_named(child, name))
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            method_call_named(lhs, name).or_else(|| method_call_named(rhs, name))
        }
        NodeKind::MethodCall { object, args, .. } => method_call_named(object, name)
            .or_else(|| args.iter().find_map(|child| method_call_named(child, name))),
        NodeKind::Binary { left, right, .. } => {
            method_call_named(left, name).or_else(|| method_call_named(right, name))
        }
        NodeKind::ArrayLiteral { elements } => {
            elements.iter().find_map(|child| method_call_named(child, name))
        }
        NodeKind::HashLiteral { pairs } => pairs.iter().find_map(|(key, value)| {
            method_call_named(key, name).or_else(|| method_call_named(value, name))
        }),
        _ => None,
    }
}

fn method_receiver<'a>(node: &'a Node, name: &str) -> Result<&'a Node, String> {
    let call = method_call_named(node, name).ok_or_else(|| format!("missing {name} call"))?;
    match &call.kind {
        NodeKind::MethodCall { object, .. } => Ok(object),
        _ => Err(format!("{name} is not a method call")),
    }
}

#[test]
fn constructor_expr_fact_records_object_package() -> Result<(), String> {
    let ast = parse_ast("MyApp::DB->new();")?;
    let receiver = method_call_named(&ast, "new").ok_or_else(|| "missing new call".to_string())?;
    let mut engine = TypeInferenceEngine::new();

    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Object("MyApp::DB".to_string()));
    assert_eq!(fact.confidence, Confidence::High);
    assert!(matches!(fact.shape, Some(ShapeFact::Object(_))));
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::ConstructorCall { package } if package == "MyApp::DB")
    }));
    Ok(())
}

#[test]
fn plain_hash_literal_slot_resolves_source_derived_receiver_fact() -> Result<(), String> {
    let code = "my %services = (db => MyApp::DB->new); $services{db}->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let services =
        engine.get_fact_at("services").ok_or_else(|| "missing services fact".to_string())?;
    assert!(matches!(services.shape, Some(ShapeFact::Hash(_))));

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Object("MyApp::DB".to_string()));
    assert_eq!(fact.confidence, Confidence::High);
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::HashSlot { hash, key } if hash == "$services" && key == "db")
    }));
    Ok(())
}

#[test]
fn plain_hash_slot_assignment_updates_later_receiver_fact() -> Result<(), String> {
    let code = "my %services; $services{db} = MyApp::DB->new; $services{db}->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Object("MyApp::DB".to_string()));
    assert_eq!(fact.confidence, Confidence::High);
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::Assignment { name } if name == "services")
    }));
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::HashSlot { hash, key } if hash == "$services" && key == "db")
    }));
    Ok(())
}

#[test]
fn dynamic_plain_hash_key_fails_closed() -> Result<(), String> {
    let code = "my %services = (db => MyApp::DB->new); $services{$name}->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Low);
    assert_eq!(fact.dynamic_boundary, Some(DynamicBoundary::DynamicHashKey));
    assert!(fact.evidence.is_empty());
    Ok(())
}
