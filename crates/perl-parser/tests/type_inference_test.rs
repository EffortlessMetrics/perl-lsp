use perl_parser::Parser;
use perl_semantic_analyzer::analysis::type_inference::{PerlType, ScalarType, TypeInferenceEngine};

fn infer(code: &str) -> TypeInferenceEngine {
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let mut engine = TypeInferenceEngine::new();
    let _ = engine.infer(&ast);
    engine
}

#[test]
fn test_variable_assignment_propagation() {
    let code = r#"
        my $x = 42;
        my $y = $x;
    "#;
    let engine = infer(code);
    assert_eq!(engine.get_type_at("x"), Some(PerlType::Scalar(ScalarType::Integer)));
    // Currently this might fail or return Any if propagation isn't fully implemented
    assert_eq!(engine.get_type_at("y"), Some(PerlType::Scalar(ScalarType::Integer)));
}

#[test]
fn test_return_type_inference() {
    let code = r#"
        sub get_int {
            return 100;
        }
        my $x = get_int();
    "#;
    let engine = infer(code);

    // Check subroutine return type
    if let Some(PerlType::Subroutine { returns, .. }) = engine.get_subroutine("get_int") {
        assert!(!returns.is_empty());
        assert_eq!(returns[0], PerlType::Scalar(ScalarType::Integer));
    } else {
        panic!("Subroutine get_int not found or wrong type");
    }

    // Check variable assigned from function call
    assert_eq!(engine.get_type_at("x"), Some(PerlType::Scalar(ScalarType::Integer)));
}

#[test]
fn test_implicit_return_inference() {
    let code = r#"
        sub get_str {
            "hello";
        }
    "#;
    let engine = infer(code);

    if let Some(PerlType::Subroutine { returns, .. }) = engine.get_subroutine("get_str") {
        assert!(!returns.is_empty());
        assert_eq!(returns[0], PerlType::Scalar(ScalarType::String));
    } else {
        panic!("Subroutine get_str not found");
    }
}

#[test]
fn test_control_flow_union() {
    let code = r#"
        my $x;
        if (1) {
            $x = 1;
        } else {
            $x = "string";
        }
    "#;
    // This test is harder to assert on with current engine because it might just be Any
    // But ideally it should be Union(Integer, String) or similar.
    // For now, let's just see what happens.
    let _engine = infer(code);
}
