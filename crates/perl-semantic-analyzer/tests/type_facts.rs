use perl_semantic_analyzer::analysis::type_facts::{DynamicBoundary, TypeEvidence, TypeFact};
use perl_semantic_analyzer::analysis::type_inference::{PerlType, ScalarType, TypeEnvironment};
use perl_semantic_facts::Confidence;

#[test]
fn variable_fact_stores_erased_type_for_existing_api() -> Result<(), Box<dyn std::error::Error>> {
    let mut env = TypeEnvironment::new();
    let mut fact = TypeFact::from_type(PerlType::Object("MyApp::DB".to_string()));
    fact.evidence.push(TypeEvidence::ConstructorCall { package: "MyApp::DB".to_string() });

    env.set_variable_fact("db".to_string(), fact.clone());

    assert_eq!(env.get_variable("db"), Some(&PerlType::Object("MyApp::DB".to_string())));
    assert_eq!(env.get_fact_at("db"), Some(fact));
    Ok(())
}

#[test]
fn variable_fact_lookup_searches_parent_scopes() -> Result<(), Box<dyn std::error::Error>> {
    let mut parent = TypeEnvironment::new();
    let fact = TypeFact::unknown_hash();
    parent.set_variable_fact("services".to_string(), fact.clone());

    let child = TypeEnvironment::with_parent(parent);

    assert_eq!(child.get_variable("services"), Some(&fact.erased_type()));
    assert_eq!(child.get_fact_at("services"), Some(fact));
    Ok(())
}

#[test]
fn type_only_assignment_clears_stale_variable_fact() -> Result<(), Box<dyn std::error::Error>> {
    let mut env = TypeEnvironment::new();
    env.set_variable_fact(
        "value".to_string(),
        TypeFact::from_type(PerlType::Object("MyApp::DB".to_string())),
    );

    env.set_variable("value".to_string(), PerlType::Scalar(ScalarType::String));

    assert_eq!(env.get_variable("value"), Some(&PerlType::Scalar(ScalarType::String)));
    assert_eq!(env.get_fact_at("value"), None);
    Ok(())
}

#[test]
fn dynamic_fact_records_boundary_and_low_confidence() -> Result<(), Box<dyn std::error::Error>> {
    let fact = TypeFact::dynamic(DynamicBoundary::DynamicHashKey);

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Low);
    assert_eq!(fact.dynamic_boundary, Some(DynamicBoundary::DynamicHashKey));
    Ok(())
}

#[test]
fn unknown_hash_erases_to_string_keyed_hash() -> Result<(), Box<dyn std::error::Error>> {
    let fact = TypeFact::unknown_hash();

    assert_eq!(
        fact.erased_type(),
        PerlType::Hash {
            key: Box::new(PerlType::Scalar(ScalarType::String)),
            value: Box::new(PerlType::Any),
        }
    );
    assert_eq!(fact.confidence, Confidence::Low);
    Ok(())
}
