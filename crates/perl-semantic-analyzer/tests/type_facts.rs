use perl_semantic_analyzer::analysis::type_facts::{HashShape, ShapeFact, TypeFact};
use perl_semantic_analyzer::analysis::type_inference::{PerlType, ScalarType, TypeEnvironment};
use perl_semantic_facts::Confidence;
use std::collections::BTreeMap;

#[test]
fn type_fact_erases_to_existing_perl_type() -> Result<(), Box<dyn std::error::Error>> {
    let fact = TypeFact::from_erased_type(PerlType::Scalar(ScalarType::String));

    assert_eq!(fact.erased_type(), PerlType::Scalar(ScalarType::String));
    assert_eq!(fact.confidence, Confidence::Low);
    Ok(())
}

#[test]
fn type_environment_stores_rich_fact_and_erased_type() -> Result<(), Box<dyn std::error::Error>> {
    let mut env = TypeEnvironment::new();
    let fact = TypeFact {
        ty: PerlType::Hash {
            key: Box::new(PerlType::Scalar(ScalarType::String)),
            value: Box::new(PerlType::Any),
        },
        confidence: Confidence::High,
        evidence: Vec::new(),
        dynamic_boundary: None,
        shape: Some(ShapeFact::Hash(HashShape {
            slots: BTreeMap::new(),
            fallback_value: Some(Box::new(TypeFact::unknown())),
        })),
    };

    env.set_variable_fact("services".to_string(), fact.clone());

    assert_eq!(env.get_variable("services"), Some(&fact.ty));
    assert_eq!(env.get_variable_fact("services"), Some(&fact));
    Ok(())
}

#[test]
fn type_environment_finds_parent_fact() -> Result<(), Box<dyn std::error::Error>> {
    let mut parent = TypeEnvironment::new();
    let fact = TypeFact::from_erased_type(PerlType::Object("MyApp::DB".to_string()));
    parent.set_variable_fact("db".to_string(), fact.clone());

    let child = TypeEnvironment::with_parent(parent);

    assert_eq!(child.get_variable_fact("db"), Some(&fact));
    Ok(())
}
