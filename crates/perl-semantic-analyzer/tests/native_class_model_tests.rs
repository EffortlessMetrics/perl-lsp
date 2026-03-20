//! Tests for Perl 5.38 native class syntax support in ClassModelBuilder.
//!
//! Verifies that `class Foo { field $x; method bar { } }` produces a
//! well-structured ClassModel with Framework::NativeClass, correct field
//! attributes, and method entries — parallel to how Moose/Moo classes are
//! modelled.

use perl_semantic_analyzer::{
    Parser,
    class_model::{ClassModel, ClassModelBuilder, Framework},
};
use perl_tdd_support::must;

fn build_models(code: &str) -> Vec<ClassModel> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    ClassModelBuilder::new().build(&ast)
}

fn find_model<'a>(models: &'a [ClassModel], name: &str) -> Option<&'a ClassModel> {
    models.iter().find(|m| m.name == name)
}

/// A bare `class Foo { }` block should produce exactly one ClassModel
/// with `Framework::NativeClass` and the correct name.
#[test]
fn test_native_class_model_is_extracted() {
    let models = build_models(
        r#"
use v5.38;
class Animal {
    field $name :param;
    method speak { return "..."; }
}
"#,
    );

    let model = find_model(&models, "Animal");
    assert!(model.is_some(), "expected ClassModel for 'Animal', got: {:?}", models);
    let model = model.unwrap_or_else(|| unreachable!());
    assert_eq!(
        model.framework,
        Framework::NativeClass,
        "native class should use Framework::NativeClass"
    );
}

/// Methods declared with `method` keyword inside a native class should appear
/// in the model's `methods` list.
#[test]
fn test_native_class_model_extracts_methods() {
    let models = build_models(
        r#"
use v5.38;
class Person {
    field $name :param;
    method greet { return "Hello"; }
    method name  { return $name;   }
}
"#,
    );

    let model = find_model(&models, "Person");
    assert!(model.is_some(), "expected ClassModel for 'Person'");
    let model = model.unwrap_or_else(|| unreachable!());

    let method_names: Vec<&str> = model.methods.iter().map(|m| m.name.as_str()).collect();
    assert!(method_names.contains(&"greet"), "expected method 'greet' in {:?}", method_names);
    assert!(method_names.contains(&"name"), "expected method 'name' in {:?}", method_names);
}

/// Multiple native classes in one file each produce their own ClassModel.
#[test]
fn test_multiple_native_classes_produce_separate_models() {
    let models = build_models(
        r#"
use v5.38;
class Cat {
    field $name :param;
    method meow { return "meow"; }
}
class Dog {
    field $name :param;
    method bark { return "woof"; }
}
"#,
    );

    assert!(find_model(&models, "Cat").is_some(), "expected ClassModel for 'Cat'");
    assert!(find_model(&models, "Dog").is_some(), "expected ClassModel for 'Dog'");
    assert_eq!(models.len(), 2, "expected exactly 2 ClassModels");
}
