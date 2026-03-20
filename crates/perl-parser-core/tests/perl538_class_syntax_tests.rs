mod cpan_test_helpers;
use cpan_test_helpers::*;

// Perl 5.38 native class syntax — extended forms that go beyond the basic
// `class Name { ... }` already covered in field_declaration_tests.rs.

/// `class Foo :isa(Parent) { ... }` — class-level `:isa` attribute
/// specifying a parent class. The parser must consume the `:isa(Parent)`
/// before the opening `{` without producing error nodes.
#[test]
fn test_class_isa_attribute_clean_parse() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
        use v5.38;
        class Animal :isa(Creature) {
            field $name :param;
            method speak { return "..."; }
        }
    "#;
    assert_clean_parse(source);
    Ok(())
}

/// `class Foo 1.0 { ... }` — class declaration with version number.
/// Perl 5.38 allows an optional version after the class name.
#[test]
fn test_class_with_version_clean_parse() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
        use v5.38;
        class Point 1.0 {
            field $x :param = 0;
            field $y :param = 0;
        }
    "#;
    assert_clean_parse(source);
    Ok(())
}

/// `class Foo 1.0 :isa(Parent) { ... }` — version before attribute.
/// Both optional parts must be consumed before the opening brace.
#[test]
fn test_class_version_then_isa_clean_parse() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
        use v5.38;
        class Animal 1.0 :isa(Creature) {
            field $name :param;
        }
    "#;
    assert_clean_parse(source);
    Ok(())
}

/// `method` inside a class with an explicit signature
/// `method distance_to($other) { ... }`.
#[test]
fn test_method_with_signature_clean_parse() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
        use v5.38;
        class Point {
            field $x :param = 0;
            field $y :param = 0;
            method distance_to($other) {
                return sqrt(($self->{x} - $other->{x})**2);
            }
        }
    "#;
    assert_clean_parse(source);
    Ok(())
}

/// A class with multiple fields and multiple methods should parse cleanly.
#[test]
fn test_class_with_multiple_fields_and_methods_clean_parse()
-> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
        use v5.38;
        class Person {
            field $first :param;
            field $last  :param;
            field $age   :param = 0;

            method full_name { return "$first $last"; }
            method greet     { return "Hello, I am " . $self->full_name; }
        }
    "#;
    assert_clean_parse(source);
    Ok(())
}
