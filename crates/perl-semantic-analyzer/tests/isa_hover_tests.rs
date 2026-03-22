//! Tests for @ISA / use parent inheritance detection and same-file hover resolution.
//!
//! Phase 2 of issue #1661: SemanticAnalyzer holds class_models and can resolve
//! inherited method hover info within a single file.

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::class_model::{ClassModel, ClassModelBuilder, Framework};
use perl_semantic_analyzer::analysis::semantic::SemanticAnalyzer;
use perl_tdd_support::{must, must_some};

fn build_models(code: &str) -> Vec<ClassModel> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    ClassModelBuilder::new().build(&ast)
}

fn find_model<'a>(models: &'a [ClassModel], name: &str) -> Option<&'a ClassModel> {
    models.iter().find(|m| m.name == name)
}

// ---------------------------------------------------------------------------
// Phase 1 regression: class_models field accessible on SemanticAnalyzer
// ---------------------------------------------------------------------------

#[test]
fn semantic_analyzer_exposes_class_models() {
    let code = r#"
package Animal;
sub speak { "generic" }

package Dog;
use parent 'Animal';
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);
    // After Phase 2, SemanticAnalyzer must hold class_models
    assert!(
        !analyzer.class_models.is_empty(),
        "SemanticAnalyzer should expose class_models built from AST"
    );
    let dog_model = analyzer.class_models.iter().find(|m| m.name == "Dog");
    assert!(dog_model.is_some(), "Dog class model should be present");
    let dog_model = must_some(dog_model);
    assert!(dog_model.parents.contains(&"Animal".to_string()), "Dog should list Animal as parent");
}

// ---------------------------------------------------------------------------
// Phase 2: same-file inherited method hover resolution via resolve_inherited_method_hover
// ---------------------------------------------------------------------------

#[test]
fn resolve_inherited_method_hover_same_file() {
    let code = r#"
package Animal;
sub speak { "generic" }

package Dog;
use parent 'Animal';
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    // Dog does not define speak directly; it should resolve via Animal
    let hover = analyzer.resolve_inherited_method_hover("Dog", "speak");
    assert!(hover.is_some(), "should resolve inherited method 'speak' via same-file parent chain");
    let hover = must_some(hover);
    assert!(
        hover.signature.contains("speak"),
        "hover signature should mention 'speak', got: {}",
        hover.signature
    );
    assert!(
        hover.details.iter().any(|d| d.contains("Animal")),
        "details should mention Animal as the source class"
    );
}

#[test]
fn resolve_method_defined_in_receiver_directly() {
    let code = r#"
package Animal;
sub speak { "generic" }
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    // Animal defines speak directly — resolver should find it
    let hover = analyzer.resolve_inherited_method_hover("Animal", "speak");
    assert!(hover.is_some(), "should resolve method 'speak' on Animal directly");
}

#[test]
fn resolve_unknown_method_returns_none() {
    let code = r#"
package Animal;
sub speak { "generic" }
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    // nonexistent method should return None
    let hover = analyzer.resolve_inherited_method_hover("Animal", "fly");
    assert!(hover.is_none(), "unknown method should return None");
}

// ---------------------------------------------------------------------------
// Class model builder: multi-parent use parent
// ---------------------------------------------------------------------------

#[test]
fn class_model_use_parent_multiple_parents() {
    let code = "package Child; use parent qw(Base1 Base2); 1;";
    let models = build_models(code);
    let model = must_some(find_model(&models, "Child"));
    assert_eq!(model.framework, Framework::PlainOO);
    assert!(model.parents.contains(&"Base1".to_string()));
    assert!(model.parents.contains(&"Base2".to_string()));
    assert_eq!(model.parents.len(), 2, "exactly 2 parents");
}

#[test]
fn class_model_isa_assignment() {
    let code = "package Child; our @ISA = qw(Parent); sub greet { } 1;";
    let models = build_models(code);
    let model = must_some(find_model(&models, "Child"));
    assert!(
        model.parents.contains(&"Parent".to_string()),
        "should detect Parent from @ISA assignment"
    );
}
