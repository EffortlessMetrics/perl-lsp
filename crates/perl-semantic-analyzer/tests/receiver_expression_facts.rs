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

fn object_shape_package(
    fact: &perl_semantic_analyzer::analysis::type_facts::TypeFact,
) -> Result<&str, String> {
    let shape = fact.shape.as_ref().ok_or_else(|| "missing object shape".to_string())?;
    match shape {
        ShapeFact::Object(object) => Ok(object.package.as_str()),
        _ => Err("fact should carry object shape".to_string()),
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
fn hashref_literal_slot_resolves_source_derived_receiver_fact() -> Result<(), String> {
    let code = "my $services = { db => MyApp::DB->new }; $services->{db}->connect;";
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
        matches!(
            evidence,
            TypeEvidence::HashRefSlot { base, key } if base == "$services" && key == "db"
        )
    }));
    Ok(())
}

#[test]
fn hashref_slot_assignment_updates_later_receiver_fact() -> Result<(), String> {
    let code = "my $services = {}; $services->{db} = MyApp::DB->new; $services->{db}->connect;";
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
        matches!(
            evidence,
            TypeEvidence::HashRefSlot { base, key } if base == "$services" && key == "db"
        )
    }));
    Ok(())
}

#[test]
fn bless_hash_field_resolves_medium_confidence_receiver_fact() -> Result<(), String> {
    let code = "my $self = bless { db => MyApp::DB->new }, 'MyApp::Service'; $self->{db}->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let self_fact = engine.get_fact_at("self").ok_or_else(|| "missing self fact".to_string())?;
    assert_eq!(self_fact.ty, PerlType::Object("MyApp::Service".to_string()));
    assert_eq!(self_fact.confidence, Confidence::Medium);
    let ShapeFact::Object(shape) =
        self_fact.shape.as_ref().ok_or_else(|| "missing object shape".to_string())?
    else {
        return Err("self fact should carry object shape".to_string());
    };
    assert!(shape.fields.contains_key("db"));
    assert!(self_fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::BlessLiteral { package } if package == "MyApp::Service")
    }));

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Object("MyApp::DB".to_string()));
    assert_eq!(fact.confidence, Confidence::Medium);
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::ConstructorCall { package } if package == "MyApp::DB")
    }));
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::BlessLiteral { package } if package == "MyApp::Service")
    }));
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::HashRefSlot { base, key } if base == "$self" && key == "db")
    }));
    Ok(())
}

#[test]
fn dynamic_bless_class_does_not_expose_exact_object_field_fact() -> Result<(), String> {
    let code = "my $class = 'MyApp::Service'; my $self = bless { db => MyApp::DB->new }, $class; $self->{db}->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let self_fact = engine.get_fact_at("self").ok_or_else(|| "missing self fact".to_string())?;
    assert_eq!(self_fact.ty, PerlType::Any);
    assert_eq!(self_fact.confidence, Confidence::Low);
    assert_eq!(self_fact.dynamic_boundary, Some(DynamicBoundary::DynamicBlessClass));

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Low);
    assert_eq!(fact.dynamic_boundary, Some(DynamicBoundary::DynamicBlessClass));
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::HashRefSlot { base, key } if base == "$self" && key == "db")
    }));
    Ok(())
}

#[test]
fn moo_accessor_return_records_medium_confidence_object_shape() -> Result<(), String> {
    let code = "package MyApp::Service; use Moo; has db => (is => 'ro', isa => 'MyApp::DB'); my $service = MyApp::Service->new; $service->db->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Medium);
    assert_eq!(object_shape_package(&fact)?, "MyApp::DB");
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::MooseIsa { attr, isa } if attr == "db" && isa == "MyApp::DB")
    }));
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::AccessorReturn { method, field } if method == "db" && field == "db")
    }));
    Ok(())
}

#[test]
fn self_constructor_framework_accessor_records_medium_confidence_object_shape() -> Result<(), String>
{
    let code = "package MyApp::Service; use Moo; has db => (is => 'ro', isa => 'MyApp::DB'); my $self = MyApp::Service->new; $self->db->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let self_fact = engine.get_fact_at("self").ok_or_else(|| "missing self fact".to_string())?;
    assert_eq!(self_fact.ty, PerlType::Object("MyApp::Service".to_string()));
    assert_eq!(self_fact.confidence, Confidence::High);
    assert!(self_fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::ConstructorCall { package } if package == "MyApp::Service")
    }));
    assert!(self_fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::VariableInitializer { name } if name == "self")
    }));

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Medium);
    assert_eq!(object_shape_package(&fact)?, "MyApp::DB");
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::MooseIsa { attr, isa } if attr == "db" && isa == "MyApp::DB")
    }));
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::AccessorReturn { method, field } if method == "db" && field == "db")
    }));
    Ok(())
}

#[test]
fn dynamic_moo_accessor_isa_stays_non_exact() -> Result<(), String> {
    let code = "package MyApp::Service; use Moo; my $type = 'MyApp::DB'; has db => (is => 'ro', isa => $type); my $service = MyApp::Service->new; $service->db->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Low);
    assert!(fact.shape.is_none());
    assert!(fact.evidence.is_empty());
    Ok(())
}

#[test]
fn parametrized_moo_accessor_isa_stays_non_exact() -> Result<(), String> {
    let code = "package MyApp::Service; use Moo; has dbs => (is => 'ro', isa => 'ArrayRef[MyApp::DB]'); my $service = MyApp::Service->new; $service->dbs->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Low);
    assert!(fact.shape.is_none());
    assert!(fact.evidence.is_empty());
    Ok(())
}

#[test]
fn method_return_constructor_records_medium_confidence_object_shape() -> Result<(), String> {
    let code = "package MyApp::Service; sub db { return MyApp::DB->new; } my $service = MyApp::Service->new; $service->db->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Medium);
    assert_eq!(object_shape_package(&fact)?, "MyApp::DB");
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::MethodReturn { method, package } if method == "db" && package == "MyApp::DB")
    }));
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::ConstructorCall { package } if package == "MyApp::DB")
    }));
    Ok(())
}

#[test]
fn implicit_method_return_constructor_records_object_shape() -> Result<(), String> {
    let code = "package MyApp::Service; sub cache { MyApp::Cache->new; } my $service = MyApp::Service->new; $service->cache->get;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let receiver = method_receiver(&ast, "get")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Medium);
    assert_eq!(object_shape_package(&fact)?, "MyApp::Cache");
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::MethodReturn { method, package } if method == "cache" && package == "MyApp::Cache")
    }));
    Ok(())
}

#[test]
fn method_return_local_constructor_variable_records_object_shape() -> Result<(), String> {
    let code = "package MyApp::Service; sub db { my $db = MyApp::DB->new; return $db; } my $service = MyApp::Service->new; $service->db->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Medium);
    assert_eq!(object_shape_package(&fact)?, "MyApp::DB");
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::MethodReturn { method, package } if method == "db" && package == "MyApp::DB")
    }));
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::VariableInitializer { name } if name == "db")
    }));
    Ok(())
}

#[test]
fn method_return_lexical_assignment_records_assignment_evidence() -> Result<(), String> {
    let code = "package MyApp::Service; sub db { my $db; $db = MyApp::DB->new; return $db; } my $service = MyApp::Service->new; $service->db->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Medium);
    assert_eq!(object_shape_package(&fact)?, "MyApp::DB");
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::MethodReturn { method, package } if method == "db" && package == "MyApp::DB")
    }));
    assert!(
        fact.evidence.iter().any(|evidence| {
            matches!(evidence, TypeEvidence::Assignment { name } if name == "db")
        })
    );
    Ok(())
}

#[test]
fn method_return_constructor_accessor_chain_records_object_shape() -> Result<(), String> {
    let code = "package MyApp::Container; use Moo; has db => (is => 'ro', isa => 'MyApp::DB'); package MyApp::Service; sub db { return MyApp::Container->new->db; } my $service = MyApp::Service->new; $service->db->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Medium);
    assert_eq!(object_shape_package(&fact)?, "MyApp::DB");
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::MethodReturn { method, package } if method == "db" && package == "MyApp::DB")
    }));
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::ConstructorCall { package } if package == "MyApp::Container")
    }));
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::MooseIsa { attr, isa } if attr == "db" && isa == "MyApp::DB")
    }));
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::AccessorReturn { method, field } if method == "db" && field == "db")
    }));
    Ok(())
}

#[test]
fn method_return_local_accessor_chain_variable_records_object_shape() -> Result<(), String> {
    let code = "package MyApp::Container; use Moo; has db => (is => 'ro', isa => 'MyApp::DB'); package MyApp::Service; sub db { my $db = MyApp::Container->new->db; return $db; } my $service = MyApp::Service->new; $service->db->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Medium);
    assert_eq!(object_shape_package(&fact)?, "MyApp::DB");
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::MethodReturn { method, package } if method == "db" && package == "MyApp::DB")
    }));
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::ConstructorCall { package } if package == "MyApp::Container")
    }));
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::VariableInitializer { name } if name == "db")
    }));
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::AccessorReturn { method, field } if method == "db" && field == "db")
    }));
    Ok(())
}

#[test]
fn method_return_assigned_accessor_chain_variable_records_assignment() -> Result<(), String> {
    let code = "package MyApp::Container; use Moo; has db => (is => 'ro', isa => 'MyApp::DB'); package MyApp::Service; sub db { my $db; $db = MyApp::Container->new->db; return $db; } my $service = MyApp::Service->new; $service->db->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Medium);
    assert_eq!(object_shape_package(&fact)?, "MyApp::DB");
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::MethodReturn { method, package } if method == "db" && package == "MyApp::DB")
    }));
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::ConstructorCall { package } if package == "MyApp::Container")
    }));
    assert!(
        fact.evidence.iter().any(|evidence| {
            matches!(evidence, TypeEvidence::Assignment { name } if name == "db")
        })
    );
    assert!(fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::AccessorReturn { method, field } if method == "db" && field == "db")
    }));
    Ok(())
}

#[test]
fn dynamic_local_accessor_chain_variable_stays_non_exact() -> Result<(), String> {
    let code = "package MyApp::Container; use Moo; has db => (is => 'ro', isa => 'MyApp::DB'); package MyApp::Service; sub db { my $db = $container->db; return $db; } my $service = MyApp::Service->new; $service->db->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Low);
    assert!(fact.shape.is_none());
    assert!(fact.evidence.is_empty());
    Ok(())
}

#[test]
fn dynamic_method_return_accessor_chain_stays_non_exact() -> Result<(), String> {
    let code = "package MyApp::Container; use Moo; has db => (is => 'ro', isa => 'MyApp::DB'); package MyApp::Service; sub db { return $container->db; } my $service = MyApp::Service->new; $service->db->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Low);
    assert!(fact.shape.is_none());
    assert!(fact.evidence.is_empty());
    Ok(())
}

#[test]
fn bare_assigned_method_return_variable_stays_non_exact() -> Result<(), String> {
    let code = "package MyApp::Service; sub db { $db = MyApp::DB->new; return $db; } my $service = MyApp::Service->new; $service->db->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Low);
    assert!(fact.shape.is_none());
    assert!(fact.evidence.is_empty());
    Ok(())
}

#[test]
fn dynamic_reassigned_method_return_variable_stays_non_exact() -> Result<(), String> {
    let code = "package MyApp::Service; sub db { my $db = MyApp::DB->new; $db = $class->new; return $db; } my $service = MyApp::Service->new; $service->db->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Low);
    assert!(fact.shape.is_none());
    assert!(fact.evidence.is_empty());
    Ok(())
}

#[test]
fn conditional_reassigned_method_return_variable_stays_non_exact() -> Result<(), String> {
    let code = "package MyApp::Service; sub db { my $db = MyApp::DB->new; if ($flag) { $db = $class->new; } return $db; } my $service = MyApp::Service->new; $service->db->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Low);
    assert!(fact.shape.is_none());
    assert!(fact.evidence.is_empty());
    Ok(())
}

#[test]
fn dynamic_method_return_constructor_stays_non_exact() -> Result<(), String> {
    let code = "package MyApp::Service; sub db { return $class->new; } my $service = MyApp::Service->new; $service->db->connect;";
    let ast = parse_ast(code)?;
    let mut engine = TypeInferenceEngine::new();

    engine.infer(&ast).map_err(|err| format!("inference failed: {err:?}"))?;

    let receiver = method_receiver(&ast, "connect")?;
    let fact = engine.infer_expr_fact(receiver);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Low);
    assert!(fact.shape.is_none());
    assert!(fact.evidence.is_empty());
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
