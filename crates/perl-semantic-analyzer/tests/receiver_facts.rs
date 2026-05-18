use perl_semantic_analyzer::analysis::receiver_facts::ReceiverFact;
use perl_semantic_analyzer::analysis::type_facts::{DynamicBoundary, TypeEvidence};
use perl_semantic_analyzer::analysis::type_inference::{TypeEnvironment, TypeInferenceEngine};
use perl_semantic_analyzer::{Node, NodeKind, Parser};
use perl_semantic_facts::Confidence;
use perl_tdd_support::{must, must_some};

fn receiver_fact_for_connect(code: &str) -> ReceiverFact {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let mut engine = TypeInferenceEngine::new();
    let mut env = TypeEnvironment::new();
    engine.infer_expr_fact(&ast, &mut env);
    let object = must_some(find_method_object(&ast, "connect"));
    engine.receiver_fact_for_method_call(object, &mut env)
}

fn find_method_object<'a>(node: &'a Node, method: &str) -> Option<&'a Node> {
    if let NodeKind::MethodCall { object, method: found, .. } = &node.kind {
        if found == method {
            return Some(object);
        }
    }

    for child in children(node) {
        if let Some(found) = find_method_object(child, method) {
            return Some(found);
        }
    }

    None
}

fn children(node: &Node) -> Vec<&Node> {
    match &node.kind {
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            statements.iter().collect()
        }
        NodeKind::ExpressionStatement { expression } => vec![expression],
        NodeKind::VariableDeclaration { variable, initializer, .. } => {
            let mut children = vec![variable.as_ref()];
            if let Some(initializer) = initializer {
                children.push(initializer);
            }
            children
        }
        NodeKind::Assignment { lhs, rhs, .. } | NodeKind::Binary { left: lhs, right: rhs, .. } => {
            vec![lhs, rhs]
        }
        NodeKind::MethodCall { object, args, .. } => {
            let mut children = vec![object.as_ref()];
            children.extend(args.iter());
            children
        }
        NodeKind::FunctionCall { args, .. } | NodeKind::ArrayLiteral { elements: args } => {
            args.iter().collect()
        }
        NodeKind::HashLiteral { pairs } => {
            pairs.iter().flat_map(|(key, value)| [key, value]).collect()
        }
        _ => Vec::new(),
    }
}

fn has_constructor_evidence(fact: &ReceiverFact, package: &str) -> bool {
    fact.fact.evidence.iter().any(|evidence| {
        matches!(evidence, TypeEvidence::ConstructorCall { package: found } if found == package)
    })
}

fn has_hash_slot_evidence(fact: &ReceiverFact, hash: &str, key: &str) -> bool {
    fact.fact.evidence.iter().any(|evidence| {
        matches!(
            evidence,
            TypeEvidence::HashSlot { hash: found_hash, key: found_key }
                if found_hash == hash && found_key == key
        )
    })
}

#[test]
fn hash_literal_slot_resolves_constructor_receiver_fact() {
    let fact = receiver_fact_for_connect(
        r#"
package MyApp::DB;
sub connect {}

package main;
my %services = (
    db => MyApp::DB->new,
);

$services{db}->connect;
"#,
    );

    assert_eq!(fact.package.as_deref(), Some("MyApp::DB"));
    assert_eq!(fact.fact.confidence, Confidence::High);
    assert!(has_hash_slot_evidence(&fact, "services", "db"));
    assert!(has_constructor_evidence(&fact, "MyApp::DB"));
}

#[test]
fn hash_slot_assignment_resolves_constructor_receiver_fact() {
    let fact = receiver_fact_for_connect(
        r#"
package MyApp::DB;
sub connect {}

package main;
my %services;
$services{db} = MyApp::DB->new;
$services{db}->connect;
"#,
    );

    assert_eq!(fact.package.as_deref(), Some("MyApp::DB"));
    assert_eq!(fact.fact.confidence, Confidence::High);
    assert!(has_hash_slot_evidence(&fact, "services", "db"));
    assert!(has_constructor_evidence(&fact, "MyApp::DB"));
}

#[test]
fn hashref_literal_slot_resolves_constructor_receiver_fact() {
    let fact = receiver_fact_for_connect(
        r#"
package MyApp::DB;
sub connect {}

package main;
my $services = {
    db => MyApp::DB->new,
};

$services->{db}->connect;
"#,
    );

    assert_eq!(fact.package.as_deref(), Some("MyApp::DB"));
    assert_eq!(fact.fact.confidence, Confidence::High);
    assert!(has_constructor_evidence(&fact, "MyApp::DB"));
}

#[test]
fn dynamic_hash_key_fails_closed() {
    let fact = receiver_fact_for_connect(
        r#"
package MyApp::DB;
sub connect {}

package main;
my %services = (
    db => MyApp::DB->new,
);
my $name = 'db';

$services{$name}->connect;
"#,
    );

    assert_eq!(fact.package, None);
    assert_eq!(fact.fact.dynamic_boundary, Some(DynamicBoundary::DynamicHashKey));
}
